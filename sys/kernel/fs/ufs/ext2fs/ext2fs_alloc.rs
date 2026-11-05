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
 *	@(#)ffs_alloc.c	8.11 (Berkeley) 10/27/94
 *  Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! ext2fs block and inode allocation: blocks (`ext2fs_alloc`, with the preferred position
//! `ext2fs_blkpref`), inodes (`ext2fs_inode_alloc`, directories spread by
//! `ext2fs_dirpref`), the cylinder group search (`ext2fs_hashalloc`, `ext2fs_alloccg`,
//! `ext2fs_nodealloccg`, `ext2fs_mapsearch`) and freeing (`ext2fs_blkfree`,
//! `ext2fs_inode_free`).
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_alloc.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_alloc.rs@3c62ede68802
//!
//! ## Deviations
//! - `ext2gennumber` is the atomic [`EXT2GENNUMBER`].
//! - The bitmaps are the buffer's bytes (`char *bbp` in C), searched with `isclr`/`setbit`
//!   and `skpc` over slices.
//! - `ext2fs_blkpref` takes the block pointers `bap` as the bytes they are stored in (the
//!   inode's `e2di_blocks` or an indirect block), little-endian (`letoh32`).
//! - `ext2fs_inode_alloc` returns the new vnode (the C's `*vpp`).
//! - A missing credential panics with or without `DIAGNOSTIC` (the C dereferences `NOCRED`
//!   without it).
//! - `uprintf`'s message goes to the current thread's process's terminal, and nowhere when no
//!   thread runs (`kernel::tty::uprintf`, as `ffs_alloc`'s).
//!
//! ## Redesign
//! - LZ's lent bitmap buffers (`&'static Buf`, their bytes reached through an `unsafe`
//!   `BufLegacy::data` at four sites) -> the busy buffers' tokens of the kernel's cache
//!   (`bread` gives a `BufRef`, `bdwrite`/`brelse` consume it), the bitmap a borrow of the
//!   token's bytes until the write; the group descriptors are the in-core super-block's
//!   members, changed one at a time (`Ext2Superblock::set_gd_nbfree`, ...) where LZ copied a
//!   descriptor in and out of a closure. The functions are generic over the thread glue, as
//!   the kernel's inode; `ext2fs_alloc` returns the block (the C's `*bnp`) as `ffs_alloc`.

use core::sync::atomic::{AtomicU64, Ordering};

use hal::sys::errno::Errno;
use hal::sys::select::{NBBY, howmany};
use hal::sys::types::{Daddr, Ino, Mode, Uid};

use super::ext2fs::{Ext2Superblock, dtog, dtogd, freespace, fsbtodb, ino_to_cg};
use super::ext2fs_dinode::EXT2_FIRSTINO;
use crate::assert::kpanic;
use crate::fs::bio::{bdwrite, bread, brelse, btodb};
use crate::fs::ufs::ffs::fs::{clrbit, isclr, setbit};
use crate::fs::ufs::ufs::dinode::{IFDIR, IFMT, Ufsino};
use crate::fs::ufs::ufs::inode::{IN_CHANGE, IN_UPDATE, Inode, vtoi};
use crate::fs::vnode::Vnode;
use crate::fs::vops::{Cred, VFS_VGET};
use crate::log::LOG_ERR;
use crate::prf::{log, printf};
use crate::proc::ProcGlue;
use crate::tc::gettime;
use libkern::skpc;

/// The allocator `ext2fs_hashalloc` runs in each cylinder group (`ext2fs_alloccg` or
/// `ext2fs_nodealloccg`).
type Allocator<G> = fn(&Inode<G>, i32, u32, i32) -> u32;

/// `ext2gennumber`: the last generation number handed to an inode.
pub static EXT2GENNUMBER: AtomicU64 = AtomicU64::new(0);

/// The uid of a credential the allocator is handed (`NOCRED` is a caller's bug, which
/// `DIAGNOSTIC` catches as "missing credential").
fn cred_uid(cred: Cred<'_>, func: &str) -> Uid {
    match cred.ucred() {
        Some(c) => c.cr_uid.get(),
        None => kpanic(format_args!("{func}: missing credential")),
    }
}

/// `uprintf("\n%s: <what>\n", fs->e2fs_fsmnt)` on the current process's terminal.
fn uprintf_fs<G: ProcGlue>(fs: &Ext2Superblock, what: &str) {
    G::with_curproc(|p| {
        if let Some(p) = p {
            crate::tty::uprintf(p.process(), format_args!("\n{}: {}\n", fs.fsmnt(), what));
        }
    });
}

/// `ext2fs_alloc`: allocate a block in the file system; the block's number.
///
/// A preference may be optionally specified. If a preference is given the following
/// hierarchy is used to allocate a block:
///   1) allocate the requested block.
///   2) allocate a rotationally optimal block in the same cylinder.
///   3) allocate a block in the same cylinder group.
///   4) quadratically rehash into other cylinder groups, until an available block is
///      located.
///
/// If no block preference is given the following hierarchy is used to allocate a block:
///   1) allocate a block in the cylinder group that contains the inode for the file.
///   2) quadratically rehash into other cylinder groups, until an available block is
///      located.
pub fn ext2fs_alloc<G: ProcGlue>(
    ip: &Inode<G>,
    _lbn: u32,
    mut bpref: u32,
    cred: Cred<'_>,
) -> Result<u32, Errno> {
    let fs = ip.i_e2fs();
    let uid = cred_uid(cred, "ext2fs_alloc");
    'nospace: {
        if fs.e2fs_fbcount() == 0 {
            break 'nospace;
        }
        if uid != 0 && freespace(fs) == 0 {
            break 'nospace;
        }
        if bpref >= fs.e2fs_bcount() {
            bpref = 0;
        }
        let cg = if bpref == 0 {
            ino_to_cg(fs, ip.i_number)
        } else {
            dtog(fs, bpref)
        };
        let bno = ext2fs_hashalloc(
            ip,
            cg as i32,
            bpref,
            fs.e2fs_bsize.get(),
            ext2fs_alloccg::<G>,
        );
        if bno > 0 {
            let d = ip.i_e2din();
            let nblocks = btodb(i64::from(fs.e2fs_bsize.get())) as u32;
            d.set_i_e2fs_nblock(d.i_e2fs_nblock().wrapping_add(nblocks));
            ip.set_flag(IN_CHANGE | IN_UPDATE);
            return Ok(bno);
        }
    }
    // nospace:
    ext2fs_fserr(fs, uid, "file system full");
    uprintf_fs::<G>(fs, "write failed, file system is full");
    Err(Errno::ENOSPC)
}

/// `ext2fs_inode_alloc`: allocate an inode in the file system, and return its vnode, locked
/// and referenced.
///
/// If allocating a directory, use `ext2fs_dirpref` to select the inode. If allocating in a
/// directory, the following hierarchy is followed:
///   1) allocate the preferred inode.
///   2) allocate an inode in the same cylinder group.
///   3) quadratically rehash into other cylinder groups, until an available inode is
///      located.
///
/// If no inode preference is given the following hierarchy is used to allocate an inode:
///   1) allocate an inode in cylinder group 0.
///   2) quadratically rehash into other cylinder groups, until an available inode is
///      located.
pub fn ext2fs_inode_alloc<G: ProcGlue>(
    pip: &Inode<G>,
    mode: Mode,
    cred: Cred<'_>,
) -> Result<&'static Vnode<G>, Errno> {
    let pvp = pip.itov();
    let fs = pip.i_e2fs();
    'noinodes: {
        if fs.e2fs_ficount() == 0 {
            break 'noinodes;
        }

        let cg = if mode & IFMT == IFDIR {
            ext2fs_dirpref(fs)
        } else {
            ino_to_cg(fs, pip.i_number) as i32
        };
        let ipref = (cg as u32).wrapping_mul(fs.e2fs_ipg()).wrapping_add(1);
        let ino = ext2fs_hashalloc(pip, cg, ipref, mode as i32, ext2fs_nodealloccg::<G>);
        if ino == 0 {
            break 'noinodes;
        }
        let Some(mp) = pvp.mount() else {
            kpanic(format_args!("ext2fs_inode_alloc: vnode without a mount"));
        };
        let vp = match VFS_VGET(mp, Ino::from(ino)) {
            Ok(vp) => vp,
            Err(e) => {
                ext2fs_inode_free(pip, ino, mode);
                return Err(e);
            }
        };
        let ip = vtoi(vp);
        let d = ip.i_e2din();
        if d.i_e2fs_mode() != 0 && d.i_e2fs_nlink() != 0 {
            printf(format_args!(
                "mode = 0{:o}, nlinks {}, inum = {}, fs = {}\n",
                d.i_e2fs_mode(),
                d.i_e2fs_nlink(),
                ip.i_number,
                fs.fsmnt()
            ));
            kpanic(format_args!("ext2fs_valloc: dup alloc"));
        }

        d.clear();

        // Set up a new generation number for this inode.
        d.set_i_e2fs_gen(next_gennumber() as u32);
        return Ok(vp);
    }
    // noinodes:
    ext2fs_fserr(fs, cred_uid(cred, "ext2fs_inode_alloc"), "out of inodes");
    uprintf_fs::<G>(fs, "create/symlink failed, no inodes free");
    Err(Errno::ENOSPC)
}

/// `if (++ext2gennumber < (u_long)gettime()) ext2gennumber = gettime();`: the next
/// generation number.
pub fn next_gennumber() -> u64 {
    let now = gettime() as u64;
    let mut g = EXT2GENNUMBER.load(Ordering::Relaxed).wrapping_add(1);
    if g < now {
        g = now;
    }
    EXT2GENNUMBER.store(g, Ordering::Relaxed);
    g
}

/// `ext2fs_dirpref`: find a cylinder to place a directory.
///
/// The policy implemented by this algorithm is to select from among those cylinder groups
/// with above the average number of free inodes, the one with the smallest number of
/// directories.
fn ext2fs_dirpref(fs: &Ext2Superblock) -> i32 {
    let ncg = fs.e2fs_ncg.get();
    let avgifree = (fs.e2fs_ficount() / ncg as u32) as i32;
    let mut maxspace = 0;
    let mut mincg = -1;
    for cg in 0..ncg {
        let nifree = i32::from(fs.gd_nifree(cg as usize));
        if nifree >= avgifree {
            let nbfree = i32::from(fs.gd_nbfree(cg as usize));
            if mincg == -1 || nbfree > maxspace {
                mincg = cg;
                maxspace = nbfree;
            }
        }
    }
    mincg
}

/// `ext2fs_blkpref`: select the desired position for the next block in a file. The file is
/// logically divided into sections. The first section is composed of the direct blocks.
/// Each additional section contains `fs_maxbpg` blocks.
///
/// If no blocks have been allocated in the first section, the policy is to request a block
/// in the same cylinder group as the inode that describes the file. Otherwise, the policy
/// is to try to allocate the blocks contiguously. The two fields of the ext2 inode
/// extension (see `ufs/ufs/inode.h`) help this. `bap`, if given, holds the block pointers
/// as stored (little-endian), of which entries `0..=baps` are looked at.
pub fn ext2fs_blkpref<G: ProcGlue>(
    ip: &Inode<G>,
    lbn: u32,
    baps: i32,
    bap: Option<&[u8]>,
) -> Daddr {
    let fs = ip.i_e2fs();
    let d = ip.i_e2din();
    // if we are doing contiguous lbn allocation, try to alloc blocks contiguously on disk

    let last_blk = d.i_e2fs_last_blk.get();
    if last_blk != 0 && lbn == d.i_e2fs_last_lblk.get().wrapping_add(1) {
        return Daddr::from(last_blk.wrapping_add(1));
    }

    // bap, if provided, gives us a list of blocks to which we want to stay close

    if let Some(bap) = bap {
        let mut i = baps;
        while i >= 0 {
            let o = i as usize * 4;
            let b = u32::from_le_bytes([bap[o], bap[o + 1], bap[o + 2], bap[o + 3]]);
            if b != 0 {
                return Daddr::from(b.wrapping_add(1));
            }
            i -= 1;
        }
    }

    // fall back to the first block of the cylinder containing the inode

    let cg = ino_to_cg(fs, ip.i_number);
    Daddr::from(
        fs.e2fs_bpg()
            .wrapping_mul(cg)
            .wrapping_add(fs.e2fs_first_dblock())
            .wrapping_add(1),
    )
}

/// `ext2fs_hashalloc`: implement the cylinder overflow algorithm.
///
/// The policy implemented by this algorithm is:
///   1) allocate the block in its requested cylinder group.
///   2) quadratically rehash on the cylinder group number.
///   3) brute force search for a free block.
fn ext2fs_hashalloc<G: ProcGlue>(
    ip: &Inode<G>,
    mut cg: i32,
    pref: u32,
    size: i32,
    allocator: Allocator<G>,
) -> u32 {
    let fs = ip.i_e2fs();
    let ncg = fs.e2fs_ncg.get();
    let icg = cg;
    // 1: preferred cylinder group
    let result = allocator(ip, cg, pref, size);
    if result != 0 {
        return result;
    }
    // 2: quadratic rehash
    let mut i = 1;
    while i < ncg {
        cg += i;
        if cg >= ncg {
            cg -= ncg;
        }
        let result = allocator(ip, cg, 0, size);
        if result != 0 {
            return result;
        }
        i *= 2;
    }
    // 3: brute force search
    // Note that we start at i == 2, since 0 was checked initially, and 1 is always checked
    // in the quadratic rehash.
    cg = (icg + 2) % ncg;
    for _ in 2..ncg {
        let result = allocator(ip, cg, 0, size);
        if result != 0 {
            return result;
        }
        cg += 1;
        if cg == ncg {
            cg = 0;
        }
    }
    0
}

/// `ext2fs_alloccg`: determine whether a block can be allocated.
///
/// Check to see if a block of the appropriate size is available, and if it is, allocate
/// it.
fn ext2fs_alloccg<G: ProcGlue>(ip: &Inode<G>, cg: i32, mut bpref: u32, _size: i32) -> u32 {
    let fs = ip.i_e2fs();
    let cgu = cg as usize;
    if fs.gd_nbfree(cgu) == 0 {
        return 0;
    }
    let (mut t, error) = bread(
        &ip.i_devvp().bufs,
        fsbtodb(fs, Daddr::from(fs.gd_b_bitmap(cgu))),
        fs.e2fs_bsize.get(),
    );
    if error.is_err() || fs.gd_nbfree(cgu) == 0 {
        brelse(t);
        return 0;
    }
    let bbp = t.data();
    let bno = 'gotit: {
        if dtog(fs, bpref) != cg as u32 {
            bpref = 0;
        }
        if bpref != 0 {
            bpref = dtogd(fs, bpref);
            // if the requested block is available, use it
            if isclr(bbp, bpref as usize) {
                break 'gotit bpref;
            }
        }
        // no blocks in the requested cylinder, so take next available one in this cylinder
        // group. first try to get 8 contiguous blocks, then fall back to a single block.
        let start = if bpref != 0 {
            (dtogd(fs, bpref) / NBBY as u32) as i32
        } else {
            0
        };
        let end = howmany(fs.e2fs_fpg() as usize, NBBY) as i32 - start;
        for loc in start..end {
            if bbp[loc as usize] == 0 {
                break 'gotit loc as u32 * NBBY as u32;
            }
        }
        for loc in 0..start {
            if bbp[loc as usize] == 0 {
                break 'gotit loc as u32 * NBBY as u32;
            }
        }

        ext2fs_mapsearch(fs, bbp, bpref)
    };
    // gotit:
    if cfg!(feature = "diagnostic") && !isclr(bbp, bno as usize) {
        kpanic(format_args!(
            "ext2fs_alloccg: dup alloc: cg={} bno={} fs={}",
            cg,
            bno,
            fs.fsmnt()
        ));
    }
    setbit(bbp, bno as usize);
    fs.set_e2fs_fbcount(fs.e2fs_fbcount().wrapping_sub(1));
    fs.set_gd_nbfree(cgu, fs.gd_nbfree(cgu).wrapping_sub(1));
    fs.e2fs_fmod.set(1);
    bdwrite(t);
    (cg as u32)
        .wrapping_mul(fs.e2fs_fpg())
        .wrapping_add(fs.e2fs_first_dblock())
        .wrapping_add(bno)
}

/// `ext2fs_nodealloccg`: determine whether an inode can be allocated.
///
/// Check to see if an inode is available, and if it is, allocate it using the following
/// policy:
///   1) allocate the requested inode.
///   2) allocate the next available inode after the requested inode in the specified
///      cylinder group.
fn ext2fs_nodealloccg<G: ProcGlue>(ip: &Inode<G>, cg: i32, ipref: Ufsino, mode: i32) -> Ufsino {
    let mut ipref = ipref.wrapping_sub(1); // to avoid a lot of (ipref -1)
    let fs = ip.i_e2fs();
    let cgu = cg as usize;
    if fs.gd_nifree(cgu) == 0 {
        return 0;
    }
    let (mut t, error) = bread(
        &ip.i_devvp().bufs,
        fsbtodb(fs, Daddr::from(fs.gd_i_bitmap(cgu))),
        fs.e2fs_bsize.get(),
    );
    if error.is_err() {
        brelse(t);
        return 0;
    }
    let ibp = t.data();
    'gotit: {
        if ipref != 0 {
            ipref %= fs.e2fs_ipg();
            if isclr(ibp, ipref as usize) {
                break 'gotit;
            }
        }
        let mut start = (ipref / NBBY as u32) as usize;
        let mut len = howmany((fs.e2fs_ipg() - ipref) as usize, NBBY);
        let mut loc = skpc(0xff, &ibp[start..start + len]);
        if loc == 0 {
            len = start + 1;
            start = 0;
            loc = skpc(0xff, &ibp[..len]);
            if loc == 0 {
                printf(format_args!(
                    "cg = {}, ipref = {}, fs = {}\n",
                    cg,
                    ipref,
                    fs.fsmnt()
                ));
                kpanic(format_args!("ext2fs_nodealloccg: map corrupted"));
            }
        }
        let i = start + len - loc;
        let map = ibp[i];
        ipref = (i * NBBY) as u32;
        let mut bit = 1u32;
        while bit < (1 << NBBY) {
            if u32::from(map) & bit == 0 {
                break 'gotit;
            }
            bit <<= 1;
            ipref += 1;
        }
        printf(format_args!("fs = {}\n", fs.fsmnt()));
        kpanic(format_args!("ext2fs_nodealloccg: block not in map"));
    }
    // gotit:
    setbit(ibp, ipref as usize);
    fs.set_e2fs_ficount(fs.e2fs_ficount().wrapping_sub(1));
    fs.set_gd_nifree(cgu, fs.gd_nifree(cgu).wrapping_sub(1));
    fs.e2fs_fmod.set(1);
    if mode as Mode & IFMT == IFDIR {
        fs.set_gd_ndirs(cgu, fs.gd_ndirs(cgu).wrapping_add(1));
    }
    bdwrite(t);
    (cg as u32)
        .wrapping_mul(fs.e2fs_ipg())
        .wrapping_add(ipref)
        .wrapping_add(1)
}

/// `ext2fs_blkfree`: free a block.
///
/// The specified block is placed back in the free map.
pub fn ext2fs_blkfree<G: ProcGlue>(ip: &Inode<G>, bno: u32) {
    let fs = ip.i_e2fs();
    let cg = dtog(fs, bno) as usize;
    if bno >= fs.e2fs_bcount() {
        printf(format_args!("bad block {}, ino {}\n", bno, ip.i_number));
        ext2fs_fserr(fs, ip.i_e2din().i_e2fs_uid.get(), "bad block");
        return;
    }
    let (mut t, error) = bread(
        &ip.i_devvp().bufs,
        fsbtodb(fs, Daddr::from(fs.gd_b_bitmap(cg))),
        fs.e2fs_bsize.get(),
    );
    if error.is_err() {
        brelse(t);
        return;
    }
    let bbp = t.data();
    let bno = dtogd(fs, bno);
    if isclr(bbp, bno as usize) {
        kpanic(format_args!(
            "ext2fs_blkfree: freeing free block: dev = 0x{:x}, block = {}, fs = {}",
            ip.i_dev,
            bno,
            fs.fsmnt()
        ));
    }

    clrbit(bbp, bno as usize);
    fs.set_e2fs_fbcount(fs.e2fs_fbcount().wrapping_add(1));
    fs.set_gd_nbfree(cg, fs.gd_nbfree(cg).wrapping_add(1));

    fs.e2fs_fmod.set(1);
    bdwrite(t);
}

/// `ext2fs_inode_free`: free an inode.
///
/// The specified inode is placed back in the free map.
pub fn ext2fs_inode_free<G: ProcGlue>(pip: &Inode<G>, ino: Ufsino, mode: Mode) {
    let fs = pip.i_e2fs();
    if ino > fs.e2fs_icount() || ino < EXT2_FIRSTINO {
        kpanic(format_args!(
            "ifree: range: dev = 0x{:x}, ino = {}, fs = {}",
            pip.i_dev,
            ino,
            fs.fsmnt()
        ));
    }
    let cg = ino_to_cg(fs, ino) as usize;
    let (mut t, error) = bread(
        &pip.i_devvp().bufs,
        fsbtodb(fs, Daddr::from(fs.gd_i_bitmap(cg))),
        fs.e2fs_bsize.get(),
    );
    if error.is_err() {
        brelse(t);
        return;
    }
    let ibp = t.data();
    let ino = (ino - 1) % fs.e2fs_ipg();
    if isclr(ibp, ino as usize) {
        printf(format_args!(
            "dev = 0x{:x}, ino = {}, fs = {}\n",
            pip.i_dev,
            ino,
            fs.fsmnt()
        ));
        if fs.e2fs_ronly.get() == 0 {
            kpanic(format_args!("ifree: freeing free inode"));
        }
    }
    clrbit(ibp, ino as usize);
    fs.set_e2fs_ficount(fs.e2fs_ficount().wrapping_add(1));
    fs.set_gd_nifree(cg, fs.gd_nifree(cg).wrapping_add(1));
    if mode & IFMT == IFDIR {
        fs.set_gd_ndirs(cg, fs.gd_ndirs(cg).wrapping_sub(1));
    }
    fs.e2fs_fmod.set(1);
    bdwrite(t);
}

/// `ext2fs_mapsearch`: find a block in the specified cylinder group.
///
/// It is a panic if a request is made to find a block if none are available.
fn ext2fs_mapsearch(fs: &Ext2Superblock, bbp: &[u8], bpref: u32) -> u32 {
    // find the fragment by searching through the free block map for an appropriate bit
    // pattern
    let mut start = if bpref != 0 {
        (dtogd(fs, bpref) / NBBY as u32) as usize
    } else {
        0
    };
    let mut len = howmany(fs.e2fs_fpg() as usize, NBBY) - start;
    let mut loc = skpc(0xff, &bbp[start..start + len]);
    if loc == 0 {
        len = start + 1;
        start = 0;
        loc = skpc(0xff, &bbp[start..start + len]);
        if loc == 0 {
            printf(format_args!(
                "start = {}, len = {}, fs = {}\n",
                start,
                len,
                fs.fsmnt()
            ));
            kpanic(format_args!("ext2fs_alloccg: map corrupted"));
        }
    }
    let i = start + len - loc;
    let map = bbp[i];
    let mut bno = (i * NBBY) as u32;
    let mut bit = 1u32;
    while bit < (1 << NBBY) {
        if u32::from(map) & bit == 0 {
            return bno;
        }
        bit <<= 1;
        bno += 1;
    }
    printf(format_args!("fs = {}\n", fs.fsmnt()));
    kpanic(format_args!("ext2fs_mapsearch: block not in map"));
}

/// `ext2fs_fserr`: prints the name of a file system with an error diagnostic.
///
/// The form of the error message is:
///     fs: error message
fn ext2fs_fserr(fs: &Ext2Superblock, uid: Uid, cp: &str) {
    log(
        LOG_ERR,
        format_args!("uid {} on {}: {}\n", uid, fs.fsmnt(), cp),
    );
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the allocator: its pure parts (the bitmap search `ext2fs_mapsearch`, the
    // preferred block `ext2fs_blkpref`, the directory group `ext2fs_dirpref`, the order
    // `ext2fs_hashalloc` tries the groups in) over hand-made super-blocks, and the bitmap
    // allocations and frees (`ext2fs_alloc`, `ext2fs_blkfree`, `ext2fs_inode_alloc`,
    // `ext2fs_inode_free`) over the test image, the bitmaps checked on the disk.

    use std::sync::Mutex;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use hal::user::pod_to_bytes;

    use super::super::ext2fs::{EXT2_GD_SIZE, Ext2Gd, e2fs_cgload};
    use super::super::testext2fs::newfs::{self, B, BBITMAP, IBITMAP, NFREE, NIFREE};
    use super::super::testext2fs::{attach, detached, mount, new_file, root, ufsmount};
    use super::*;
    use crate::fs::subr::vput;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs;
    use crate::fs::ufs::ufs::dinode::IFREG;
    use crate::fs::vops::Cred;

    /// A file system of `ncg` groups of 64 blocks (from block 1) and 32 inodes, with these
    /// group descriptors, and inode `ino` on it.
    fn inode(ncg: i32, gds: Vec<Ext2Gd>, ino: Ufsino) -> &'static Inode<TestGlue> {
        let fs = Ext2Superblock::new();
        fs.set_e2fs_fpg(64);
        fs.set_e2fs_bpg(64);
        fs.set_e2fs_first_dblock(1);
        fs.set_e2fs_ipg(32);
        fs.e2fs_ncg.set(ncg);
        fs.gd_alloc(1, 1024);
        let mut b = vec![0u8; 1024];
        for (i, g) in gds.iter().enumerate() {
            assert!(pod_to_bytes(
                g,
                &mut b[i * EXT2_GD_SIZE..(i + 1) * EXT2_GD_SIZE]
            ));
        }
        e2fs_cgload(&b, &fs, 0, 1024);
        detached(&ufsmount(fs), ino)
    }

    #[test]
    fn mapsearch_finds_the_first_clear_bit_from_the_preference() {
        let (_g, _p) = attach(newfs::image(false));
        let ip = inode(1, vec![], 1);
        let fs = ip.i_e2fs();
        let mut map = [0xffu8, 0xff, 0x0f, 0, 0, 0, 0, 0];
        assert_eq!(ext2fs_mapsearch(fs, &map, 0), 20);
        // From byte 3 (block 1 + 25 is bit 25).
        assert_eq!(ext2fs_mapsearch(fs, &map, 26), 24);
        // Nothing from the preference to the end: search again from the start.
        map = [0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        assert_eq!(ext2fs_mapsearch(fs, &map, 34), 7);
    }

    #[test]
    fn blkpref_follows_the_last_block_then_the_pointers_then_the_group() {
        let (_g, _p) = attach(newfs::image(false));
        let ip = inode(2, vec![], 40); // group 1
        let d = ip.i_e2din();
        d.i_e2fs_last_blk.set(100);
        d.i_e2fs_last_lblk.set(4);
        assert_eq!(ext2fs_blkpref(ip, 5, 0, None), 101);

        let mut bap = [0u8; 12];
        bap[..4].copy_from_slice(&10u32.to_le_bytes());
        assert_eq!(ext2fs_blkpref(ip, 7, 2, Some(&bap)), 11);
        bap[8..].copy_from_slice(&30u32.to_le_bytes());
        assert_eq!(ext2fs_blkpref(ip, 7, 2, Some(&bap)), 31);
        assert_eq!(ext2fs_blkpref(ip, 7, 1, Some(&bap)), 11);
        assert_eq!(ext2fs_blkpref(ip, 7, 2, Some(&[0; 12])), 64 + 1 + 1);
        assert_eq!(ext2fs_blkpref(ip, 7, 0, None), 64 + 1 + 1);
    }

    #[test]
    fn dirpref_picks_the_roomiest_group_with_enough_free_inodes() {
        let (_g, _p) = attach(newfs::image(false));
        let gd = |nifree, nbfree| Ext2Gd {
            ext2bgd_nifree: nifree,
            ext2bgd_nbfree: nbfree,
            ..Ext2Gd::default()
        };
        let ip = inode(3, vec![gd(5, 10), gd(20, 3), gd(20, 7)], 1);
        let fs = ip.i_e2fs();
        fs.set_e2fs_ficount(45);
        assert_eq!(ext2fs_dirpref(fs), 2);
        fs.set_e2fs_ficount(3);
        assert_eq!(ext2fs_dirpref(fs), 0);
    }

    /// The groups the recording allocators were asked for.
    static VISITED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

    fn never(_ip: &Inode<TestGlue>, cg: i32, _pref: u32, _size: i32) -> u32 {
        VISITED.lock().unwrap().push(cg);
        0
    }

    fn in_group_4(_ip: &Inode<TestGlue>, cg: i32, _pref: u32, _size: i32) -> u32 {
        VISITED.lock().unwrap().push(cg);
        if cg == 4 { 77 } else { 0 }
    }

    #[test]
    fn hashalloc_tries_the_group_then_rehashes_then_every_group() {
        let (_g, _p) = attach(newfs::image(false));
        let ip = inode(5, vec![], 1);
        VISITED.lock().unwrap().clear();
        assert_eq!(ext2fs_hashalloc(ip, 1, 9, 1024, never), 0);
        assert_eq!(*VISITED.lock().unwrap(), [1, 2, 4, 3, 3, 4, 0]);
        VISITED.lock().unwrap().clear();
        assert_eq!(ext2fs_hashalloc(ip, 1, 9, 1024, in_group_4), 77);
        assert_eq!(*VISITED.lock().unwrap(), [1, 2, 4]);
    }

    /// Bit `bit` of the map in block `blk` of the disk, after the delayed writes.
    fn disk_bit(blk: usize, bit: usize) -> bool {
        testffs::sync(&[]);
        testffs::disk()[blk * B + bit / 8] & (1 << (bit % 8)) != 0
    }

    #[test]
    fn blocks_are_allocated_and_freed_in_the_bitmap_and_the_counters() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = root(mp);
        let ip = vtoi(vp);
        let fs = ip.i_e2fs();
        let cred = Cred::real(p.ucred());
        let nblock = ip.i_e2din().i_e2fs_nblock();

        // No preference: the first byte of eight free blocks (blocks 33..=40), not block 29.
        assert_eq!(ext2fs_alloc(&ip, 0, 0, cred), Ok(33));
        // The preferred block when it is free.
        assert_eq!(ext2fs_alloc(&ip, 0, 29, cred), Ok(29));
        assert_eq!(fs.e2fs_fbcount(), NFREE - 2);
        assert_eq!(fs.gd_nbfree(0), (NFREE - 2) as u16);
        assert_eq!(fs.e2fs_fmod.get(), 1);
        assert_eq!(ip.i_e2din().i_e2fs_nblock(), nblock + 4);
        assert!(ip.i_flag.get() & (IN_CHANGE | IN_UPDATE) == IN_CHANGE | IN_UPDATE);
        assert!(disk_bit(BBITMAP, 32) && disk_bit(BBITMAP, 28));

        ext2fs_blkfree(&ip, 33);
        ext2fs_blkfree(&ip, 29);
        assert_eq!(fs.e2fs_fbcount(), NFREE);
        assert_eq!(fs.gd_nbfree(0), NFREE as u16);
        assert!(!disk_bit(BBITMAP, 32) && !disk_bit(BBITMAP, 28));
        // A block past the file system is refused (logged), the counters untouched.
        ext2fs_blkfree(&ip, 64);
        assert_eq!(fs.e2fs_fbcount(), NFREE);

        // A user's allocation stops at the reserved blocks; root's goes on.
        let mut user = crate::cred::Ucred::new();
        user.cr_uid.set(1000);
        fs.set_e2fs_rbcount(NFREE);
        assert_eq!(
            ext2fs_alloc(&ip, 0, 0, Cred::real(&user)),
            Err(Errno::ENOSPC)
        );
        assert!(ext2fs_alloc(&ip, 0, 0, cred).is_ok());
        drop(ip);
        vput(vp);
    }

    #[test]
    fn inodes_are_allocated_and_freed_in_the_bitmap_and_the_counters() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = new_file(mp, p);
        let ip = vtoi(vp);
        let fs = ip.i_e2fs();
        // The first free inode of group 0, after the image's 14.
        assert_eq!(ip.i_number, 15);
        assert_eq!(fs.e2fs_ficount(), NIFREE - 1);
        assert_eq!(fs.gd_nifree(0), (NIFREE - 1) as u16);
        assert!(disk_bit(IBITMAP, 14));
        let generation = ip.i_e2din().i_e2fs_gen();
        assert!(generation != 0 && u64::from(generation) >= crate::tc::gettime() as u64);
        drop(ip);
        vput(vp);

        // A directory counts in the group's directories.
        let root = root(mp);
        let rip = vtoi(root);
        let dvp = ext2fs_inode_alloc(&rip, IFDIR | 0o755, Cred::real(p.ucred()));
        let dvp = dvp.unwrap_or_else(|e| std::panic!("ext2fs_inode_alloc: {e:?}"));
        assert_eq!(vtoi(dvp).i_number, 16);
        assert_eq!(fs_ndirs(&rip), 2);
        assert!(
            vtoi(dvp).i_e2din().i_e2fs_gen() > generation,
            "a new generation per inode"
        );
        vput(dvp);

        ext2fs_inode_free(&rip, 16, IFDIR | 0o755);
        ext2fs_inode_free(&rip, 15, IFREG | 0o644);
        let fs = rip.i_e2fs();
        assert_eq!(fs.e2fs_ficount(), NIFREE);
        assert_eq!((fs.gd_nifree(0), fs.gd_ndirs(0)), (NIFREE as u16, 1));
        assert!(!disk_bit(IBITMAP, 14) && !disk_bit(IBITMAP, 15));
        drop(rip);
        vput(root);
    }

    fn fs_ndirs(ip: &Inode<TestGlue>) -> u16 {
        ip.i_e2fs().gd_ndirs(0)
    }

    #[test]
    fn no_free_inode_is_enospc() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let root = root(mp);
        let rip = vtoi(root);
        rip.i_e2fs().set_e2fs_ficount(0);
        assert_eq!(
            ext2fs_inode_alloc(&rip, IFREG | 0o644, Cred::real(p.ucred())).map(|_| ()),
            Err(Errno::ENOSPC)
        );
        drop(rip);
        vput(root);
    }
}
/* </TESTS> */
