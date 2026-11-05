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

/*-
 * Copyright (C) 1994, 1995, 1997 Wolfgang Solfrank.
 * Copyright (C) 1994, 1995, 1997 TooLs GmbH.
 * All rights reserved.
 * Original code by Paul Popelka (paulp@uts.amdahl.com) (see below).
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by TooLs GmbH.
 * 4. The name of TooLs GmbH may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY TOOLS GMBH ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL TOOLS GMBH BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/*
 * Written by Paul Popelka (paulp@uts.amdahl.com)
 *
 * You can do anything you want with this software, just don't say you wrote
 * it, and don't remove this notice.
 *
 * This software is provided "as is".
 *
 * The author supplies this software to be publicly redistributed on the
 * understanding that the author is not responsible for the correct
 * functioning of this software in any circumstances and is not liable for
 * any damages caused by this software.
 *
 * October 1992
 */
/* </LICENSES> */

/* <CODE> */
//! The file allocation table: mapping a file's clusters to disk blocks (`pcbmap`) with the
//! per-denode fat cache, reading and writing FAT entries (`fatentry`, `fatchain`) and keeping
//! every copy of the FAT and the FSInfo block in step (`updatefats`), allocating and freeing
//! clusters against the in-use bitmap (`clusteralloc`, `clusterfree`, `freeclusterchain`,
//! `fillinusemap`), and growing a file (`extendfile`).
//!
//! Upstream: sys/msdosfs/msdosfs_fat.c @ 3ce1f3f79392
//! LZ: sys/msdosfs/msdosfs_fat.rs@3c62ede68802
//!
//! Updating entries in 12 bit fats is a pain in the butt. The following picture shows where
//! nibbles go when moving from a 12 bit cluster number into the appropriate bytes in the FAT.
//!
//! ```text
//!     byte m        byte m+1      byte m+2
//!     +----+----+   +----+----+   +----+----+
//!     |  0    1 |   |  2    3 |   |  4    5 |   FAT bytes
//!     +----+----+   +----+----+   +----+----+
//!
//!     +----+----+----+   +----+----+----+
//!     |  3    0    1 |   |  4    5    2 |
//!     +----+----+----+   +----+----+----+
//!     cluster n          cluster n+1
//! ```
//!
//! Where n is even. m = n + (n >> 2).
//!
//! ## Deviations
//! - The fat cache statistics (`fc_fileextends`, `fc_lfcempty`, `fc_bmapcalls`,
//!   `fc_lmdistance[]`, `fc_largedistance`) are atomics.
//! - The reads and writes of one FAT entry, which the C spells out in `pcbmap`, `fatentry`,
//!   `fatchain`, `freeclusterchain` and `fillinusemap` (the `getushort`/`getulong`, the FAT12
//!   nibble shift and the mask; the FAT12 half-word merge and FAT32's kept high bits), are
//!   `fat_read` and `fat_write` over the FAT block's bytes.
//! - `clusteralloc`'s two identical scans of the in-use bitmap (from the pseudo-random start
//!   to the last cluster, then from 0 to the start) are one function, `scanfree`.
//! - `fatblock`'s optional out-parameters are its tuple result. The optional out-parameters of
//!   `pcbmap`, `clusterfree`, `fatentry`, `chainalloc`, `clusteralloc` and `extendfile` are
//!   `Option<&mut T>`; `extendfile`'s `bpp` is `Option<&mut Option<BufRef>>`, the token of the
//!   first new block.
//! - `getblk(..., 0, INFSLP)` "never fails" in C; here it is `bio::getblk_wait`, which repeats
//!   the call until it yields the buffer.
//! - `fatentry`'s two `DIAGNOSTIC` argument checks are behind feature `diagnostic`; a
//!   `FAT_GET` without a place for the result (a crash in C without `DIAGNOSTIC`) reads the
//!   entry and drops it.
//! - `fillinusemap`'s final `brelse(bp)` is skipped when no FAT block was read (a file system
//!   without clusters), where the C would release NULL.
//! - `pcbmap` answers `EIO` for an entry that does not lie whole inside its FAT block (`bo +
//!   width > bsize`, the entry's two or four bytes), the C's `bo >= bsize` widened to the
//!   entry's width: the user's decision D-N5.7-U2 of 2026-11-05. Only a FAT12 entry can
//!   straddle (at `bo == bsize - 1` of a short last FAT block), and only through a chain value
//!   above `pm_maxcluster`, which `pcbmap` follows unbounded as the C does: there the C reads
//!   one stale byte past `b_bcount` and goes on with a garbage cluster number.
//!
//! ## Redesign
//! - LZ's lent buffers (`&'static Buf` from `bread`/`getblk` and each file's own `unsafe fn
//!   bdata` view of `b_data`: ten `unsafe`) -> the owner's `BufRef` tokens of `fs::bio` (`t.data()`
//!   writable, `t.bytes()` read-only, `t.clear()` for `clrbuf`), each consumed by the call
//!   that ends the busy period (`brelse`, `bwrite`, `bdwrite`, `updatefats`); `pcbmap`'s,
//!   `freeclusterchain`'s and `fillinusemap`'s token is kept across their loops while the
//!   block stays the same, as the C's `bp`; `updatefats` holds the modified block's token and
//!   each mirror's at once (`src.bytes()` into `dst.data()`, the C's `bcopy`), and writes the
//!   FSInfo block's free count at its offset (`fsinfree`, four bytes) where the C casts the
//!   block to `struct fsinfo`. LZ's private `getblk_wait` is `bio::getblk_wait`.
//! - The mount's geometry is read as plain members (one load each, the C's) and the in-use
//!   bitmap as `MArray<AtomicU32>` words changed by a load and a store under the kernel lock
//!   (`usemap_alloc`, `usemap_free`, the C's `|=`/`&=`), with the C's `KASSERT` bound.
//! - Every arithmetic over an on-disk value or a value derived from one keeps the C's unsigned
//!   widths: cluster and block numbers wrap (`wrapping_*`) where the C's `uint32_t` wraps
//!   (`fatofs`, `fatblock`, `cntobn`, the fat cache's `i - 1`, `clusteralloc`'s generator,
//!   `chainlength`'s `pm_maxcluster - start + 1`); the divisions are by `pm_fatdiv` (1 or 2,
//!   set by the FAT's width), `pm_fatblocksize` (`3 * pm_BytesPerSec` or `MAXBSIZE`, nonzero
//!   by `msdosfs_mountfs`'s checks) and `pm_maxcluster + 1` (nonzero: a bitmap of 0 words is
//!   refused at the mount). The walks are the C's loops over the same bytes, O(1) a step.
//!   0 `unsafe` (LZ: 10).

use core::cmp::min;
use core::mem::offset_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::sys::errno::Errno;
use hal::sys::types::Daddr;

use crate::assert::{kassert, kpanic};
use crate::fs::bio::{DEV_BSHIFT, bdwrite, bread, brelse, bwrite, getblk_wait};
use crate::fs::buf::BufRef;
use crate::fs::msdosfs::bpb::{Fsinfo, getulong, getushort, putulong, putushort};
use crate::fs::msdosfs::denode::{
    Denode, FC_LASTFC, FC_LASTMAP, FC_OLASTFC, FC_SIZE, FCE_EMPTY, fc_setcache,
};
use crate::fs::msdosfs::direntry::ATTR_DIRECTORY;
use crate::fs::msdosfs::fat::{
    CLUST_END, CLUST_EOFE, CLUST_FIRST, CLUST_RSRVD, DE_CLEAR, FAT_GET, FAT_GET_AND_SET, FAT_SET,
    FAT12_MASK, FAT16_MASK, FAT32_MASK, MSDOSFSFREE, MSDOSFSROOT, msdosfseof,
};
use crate::fs::msdosfs::msdosfsmount::{
    MSDOSFS_FATMIRROR, MSDOSFSMNT_WAITONFAT, Msdosfsmount, N_INUSEBITS, cntobn, de_bn2cn, de_cn2bn,
    de_cn2off, fatofs, fsi_size,
};
use crate::prf::printf;
use crate::proc::ProcGlue;
use crate::tc::microtime;

/// `DEV_BSIZE`: the size of a disk block (`<sys/param.h>`).
const DEV_BSIZE: u32 = 1 << DEV_BSHIFT;

/// `LMMAX`: the size of `fc_lmdistance[]`.
const LMMAX: usize = 20;

/// `fc_fileextends`: # of file extends.
pub static FC_FILEEXTENDS: AtomicI32 = AtomicI32::new(0);
/// `fc_lfcempty`: # of time last file cluster cache entry was empty.
pub static FC_LFCEMPTY: AtomicI32 = AtomicI32::new(0);
/// `fc_bmapcalls`: # of times pcbmap was called.
pub static FC_BMAPCALLS: AtomicI32 = AtomicI32::new(0);
/// `fc_lmdistance[]`: counters for how far off the last cluster mapped entry was.
pub static FC_LMDISTANCE: [AtomicI32; LMMAX] = [const { AtomicI32::new(0) }; LMMAX];
/// `fc_largedistance`: off by more than `LMMAX`.
pub static FC_LARGEDISTANCE: AtomicI32 = AtomicI32::new(0);

/// The number of bytes one FAT entry spans in a FAT block of the given width: four for FAT32,
/// two for FAT16 and for FAT12 (whose 12 bits are read as the `u_short` that holds them).
fn fat_width(fatmask: u32) -> usize {
    if fatmask == FAT32_MASK { 4 } else { 2 }
}

/// The entry of cluster `cn` at byte `bo` of a FAT block: the 16 or 32 bits there, shifted for
/// an odd FAT12 cluster, masked to the FAT's width.
fn fat_read(fatmask: u32, data: &[u8], bo: usize, cn: u32) -> u32 {
    let mut readcn = if fatmask == FAT32_MASK {
        getulong(&data[bo..])
    } else {
        u32::from(getushort(&data[bo..]))
    };
    if fatmask == FAT12_MASK && cn & 1 != 0 {
        readcn >>= 4;
    }
    readcn & fatmask
}

/// Stores `newcontents` as the entry of cluster `cn` at byte `bo` of a FAT block: a FAT12
/// entry shares its middle byte with its neighbour, and FAT32 keeps the high 4 bits.
fn fat_write(fatmask: u32, data: &mut [u8], bo: usize, cn: u32, newcontents: u32) {
    match fatmask {
        FAT12_MASK => {
            let mut readcn = u32::from(getushort(&data[bo..]));
            if cn & 1 != 0 {
                readcn &= 0x000f;
                readcn |= newcontents << 4;
            } else {
                readcn &= 0xf000;
                readcn |= newcontents & 0xfff;
            }
            putushort(&mut data[bo..], readcn as u16);
        }
        FAT16_MASK => putushort(&mut data[bo..], newcontents as u16),
        FAT32_MASK => {
            // According to spec we have to retain the high order bits of the fat entry.
            let mut readcn = getulong(&data[bo..]);
            readcn &= !FAT32_MASK;
            readcn |= newcontents & FAT32_MASK;
            putulong(&mut data[bo..], readcn);
        }
        _ => {}
    }
}

/// `fatblock(pmp, ofs, &bn, &size, &bo)`: the FAT block holding byte `ofs` of the FAT: its
/// block number on the device, its size in bytes and the offset of `ofs` in it.
fn fatblock<G: ProcGlue>(pmp: &Msdosfsmount<G>, ofs: u32) -> (u32, u32, usize) {
    let mut bn = (ofs / pmp.pm_fatblocksize).wrapping_mul(pmp.pm_fatblocksec);
    let size = min(pmp.pm_fatblocksec, pmp.pm_FATsecs.wrapping_sub(bn)).wrapping_mul(DEV_BSIZE);
    bn = bn
        .wrapping_add(pmp.pm_fatblk)
        .wrapping_add(pmp.pm_curfat.wrapping_mul(pmp.pm_FATsecs));
    (bn, size, (ofs % pmp.pm_fatblocksize) as usize)
}

/// The token of the FAT block `bn` of `bsize` bytes, read (`bread`); `brelse` and the error on
/// a failed read, as every caller's `if (error) { brelse(bp); return (error); }`.
fn fat_bread<G: ProcGlue>(pmp: &Msdosfsmount<G>, bn: u32, bsize: u32) -> Result<BufRef, Errno> {
    let (t, error) = bread(&pmp.pm_devvp.bufs, Daddr::from(bn), bsize as i32);
    match error {
        Ok(()) => Ok(t),
        Err(e) => {
            brelse(t);
            Err(e)
        }
    }
}

/// `pcbmap(dep, findcn, bnp, cnp, sp)`: map the logical cluster number of a file into a
/// physical disk sector that is filesystem relative.
///
/// - `dep`: address of denode representing the file of interest
/// - `findcn`: file relative cluster whose filesystem relative cluster number and/or block
///   number are/is to be found
/// - `bnp`: where to place the file system relative block number, if given
/// - `cnp`: where to place the file system relative cluster number, if given
/// - `sp`: where to place the block size for the file/dir, if given
///
/// This function has one side effect. If the requested file relative cluster is beyond the
/// end of file, then the actual number of clusters in the file is returned in `*cnp` along
/// with `E2BIG`. This is useful for determining how long a directory is. An entry that does
/// not lie whole in its FAT block answers `EIO` (the module's deviations, D-N5.7-U2).
pub fn pcbmap<G: ProcGlue>(
    dep: &Denode<G>,
    findcn: u32,
    bnp: Option<&mut Daddr>,
    cnp: Option<&mut u32>,
    sp: Option<&mut i32>,
) -> Result<(), Errno> {
    FC_BMAPCALLS.fetch_add(1, Ordering::Relaxed);

    // If they don't give us someplace to return a value then don't bother doing anything.
    if bnp.is_none() && cnp.is_none() && sp.is_none() {
        return Ok(());
    }

    let pmp = dep.pmp();
    let fatmask = pmp.pm_fatmask;
    let mut cn = dep.de_StartCluster.get();
    // The "file" that makes up the root directory is contiguous, permanently allocated, of
    // fixed size, and is not made up of clusters. If the cluster number is beyond the end of
    // the root directory, then return the number of clusters in the file.
    if cn == MSDOSFSROOT {
        if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
            let size = dep.de_FileSize.get();
            if de_cn2off(pmp, findcn) >= size {
                if let Some(cnp) = cnp {
                    *cnp = de_bn2cn(pmp, pmp.pm_rootdirsize);
                }
                return Err(Errno::E2BIG);
            }
            if let Some(bnp) = bnp {
                *bnp = Daddr::from(pmp.pm_rootdirblk.wrapping_add(de_cn2bn(pmp, findcn)));
            }
            if let Some(cnp) = cnp {
                *cnp = MSDOSFSROOT;
            }
            if let Some(sp) = sp {
                *sp = min(pmp.pm_bpcluster, size - de_cn2off(pmp, findcn)) as i32;
            }
            return Ok(());
        }
        // just an empty file
        if let Some(cnp) = cnp {
            *cnp = 0;
        }
        return Err(Errno::E2BIG);
    }

    // All other files do I/O in cluster sized blocks.
    if let Some(sp) = sp {
        *sp = pmp.pm_bpcluster as i32;
    }

    // Rummage around in the fat cache, maybe we can avoid tromping thru every fat entry for
    // the file. And, keep track of how far off the cache was from where we wanted to be.
    let mut i = 0;
    fc_lookup(dep, findcn, &mut i, &mut cn);
    let distance = findcn.wrapping_sub(i) as usize;
    if distance >= LMMAX {
        FC_LARGEDISTANCE.fetch_add(1, Ordering::Relaxed);
    } else {
        FC_LMDISTANCE[distance].fetch_add(1, Ordering::Relaxed);
    }

    // Handle all other files or directories the normal way. `bp` is the FAT block read last,
    // `bp_bn` its number (-1, none yet).
    let mut bp: Option<BufRef> = None;
    let mut bp_bn = u32::MAX;
    let mut prevcn = 0;
    let mut hiteof = false;
    while i < findcn {
        // Stop with all reserved clusters, not just with EOF.
        if (cn | !fatmask) >= CLUST_RSRVD {
            hiteof = true;
            break;
        }
        let byteoffset = fatofs(pmp, cn);
        let (bn, bsize, bo) = fatblock(pmp, byteoffset);
        if bn != bp_bn || bp.is_none() {
            if let Some(old) = bp.take() {
                brelse(old);
            }
            bp = Some(fat_bread(pmp, bn, bsize)?);
            bp_bn = bn;
        }
        prevcn = cn;
        // The C's `bo >= bsize`, widened to the entry's bytes (D-N5.7-U2).
        if bo + fat_width(fatmask) > bsize as usize {
            if let Some(b) = bp.take() {
                brelse(b);
            }
            return Err(Errno::EIO);
        }
        if let Some(b) = &bp {
            cn = fat_read(fatmask, b.bytes(), bo, prevcn);
        }

        // Force the special cluster numbers to be the same for all cluster sizes to let the
        // rest of msdosfs handle all cases the same.
        if (cn | !fatmask) >= CLUST_RSRVD {
            cn |= !fatmask;
        }
        i += 1;
    }

    if !hiteof && !msdosfseof(pmp, cn) {
        if let Some(b) = bp {
            brelse(b);
        }
        if let Some(bnp) = bnp {
            *bnp = Daddr::from(cntobn(pmp, cn));
        }
        if let Some(cnp) = cnp {
            *cnp = cn;
        }
        fc_setcache(dep, FC_LASTMAP, i, cn);
        return Ok(());
    }

    // hiteof:
    if let Some(cnp) = cnp {
        *cnp = i;
    }
    if let Some(b) = bp {
        brelse(b);
    }
    // update last file cluster entry in the fat cache
    fc_setcache(dep, FC_LASTFC, i.wrapping_sub(1), prevcn);
    Err(Errno::E2BIG)
}

/// `fc_lookup(dep, findcn, frcnp, fsrcnp)`: find the closest entry in the fat cache to the
/// cluster we are looking for. `*frcnp` and `*fsrcnp` are left alone when no entry is at or
/// before `findcn`.
pub fn fc_lookup<G: ProcGlue>(dep: &Denode<G>, findcn: u32, frcnp: &mut u32, fsrcnp: &mut u32) {
    let mut closest: Option<(u32, u32)> = None;
    for slot in 0..FC_SIZE {
        let fc = dep.fc(slot);
        let cn = fc.fc_frcn;
        if cn != FCE_EMPTY && cn <= findcn && closest.is_none_or(|(c, _)| cn > c) {
            closest = Some((cn, fc.fc_fsrcn));
        }
    }
    if let Some((frcn, fsrcn)) = closest {
        *frcnp = frcn;
        *fsrcnp = fsrcn;
    }
}

/// `fc_purge(dep, frcn)`: purge the fat cache in denode `dep` of all entries relating to file
/// relative cluster `frcn` and beyond.
pub fn fc_purge<G: ProcGlue>(dep: &Denode<G>, frcn: u32) {
    for slot in 0..FC_SIZE {
        let fc = dep.fc(slot);
        if fc.fc_frcn >= frcn {
            fc_setcache(dep, slot, FCE_EMPTY, fc.fc_fsrcn);
        }
    }
}

/// `updatefats(pmp, bp, fatbn)`: update the fat. If mirroring the fat, update all copies,
/// with the first copy as last. Else update only the current fat (ignoring the others).
///
/// - `pmp`: msdosfsmount structure for filesystem to update
/// - `bp`: the token of the modified fat block (written here)
/// - `fatbn`: block number relative to begin of filesystem of the modified fat block.
pub fn updatefats<G: ProcGlue>(pmp: &Msdosfsmount<G>, bp: BufRef, mut fatbn: u32) {
    let waitonfat = pmp.pm_flags.get() & MSDOSFSMNT_WAITONFAT != 0;
    let devvp = pmp.pm_devvp;

    // If we have an FSInfo block, update it.
    let fsinfo = pmp.pm_fsinfo.get();
    if fsinfo != 0 {
        let (mut bpn, error) = bread(&devvp.bufs, Daddr::from(fsinfo), fsi_size(pmp));
        if error.is_err() {
            // Ignore the error, but turn off FSInfo update for the future.
            pmp.pm_fsinfo.set(0);
            brelse(bpn);
        } else {
            // `((struct fsinfo *)bpn->b_data)->fsinfree`: the free count at its offset.
            let at = offset_of!(Fsinfo, fsinfree);
            putulong(&mut bpn.data()[at..], pmp.pm_freeclustercount.get());
            if waitonfat {
                let _ = bwrite(bpn);
            } else {
                bdwrite(bpn);
            }
        }
    }

    if pmp.pm_flags.get() & MSDOSFS_FATMIRROR != 0 {
        // Now copy the block(s) of the modified fat to the other copies of the fat and write
        // them out. This is faster than reading in the other fats and then writing them back
        // out. This could tie up the fat for quite a while. Preventing others from accessing
        // it. To prevent us from going after the fat quite so much we use delayed writes,
        // unless they specified "synchronous" when the filesystem was mounted. If synch is
        // asked for then use bwrite()'s and really slow things down.
        let bcount = bp.buf().bcount();
        for _ in 1..pmp.pm_FATs() {
            fatbn = fatbn.wrapping_add(pmp.pm_FATsecs);
            // getblk() never fails
            let mut bpn = getblk_wait(&devvp.bufs, Daddr::from(fatbn), bcount as i32);
            let src = bp.bytes();
            let dst = bpn.data();
            let n = min(src.len(), dst.len());
            dst[..n].copy_from_slice(&src[..n]);
            if waitonfat {
                let _ = bwrite(bpn);
            } else {
                bdwrite(bpn);
            }
        }
    }

    // Write out the first (or current) fat last.
    if waitonfat {
        let _ = bwrite(bp);
    } else {
        bdwrite(bp);
    }
    // Maybe update fsinfo sector here?
}

/// `usemap_alloc(pmp, cn)`: mark cluster `cn` in use.
fn usemap_alloc<G: ProcGlue>(pmp: &Msdosfsmount<G>, cn: u32) {
    kassert!(cn <= pmp.pm_maxcluster);

    let w = &pmp.pm_inusemap[(cn / N_INUSEBITS) as usize];
    w.store(
        w.load(Ordering::Relaxed) | 1 << (cn % N_INUSEBITS),
        Ordering::Relaxed,
    );
    pmp.pm_freeclustercount
        .set(pmp.pm_freeclustercount.get().wrapping_sub(1));
}

/// `usemap_free(pmp, cn)`: mark cluster `cn` free.
fn usemap_free<G: ProcGlue>(pmp: &Msdosfsmount<G>, cn: u32) {
    kassert!(cn <= pmp.pm_maxcluster);

    pmp.pm_freeclustercount
        .set(pmp.pm_freeclustercount.get().wrapping_add(1));
    let w = &pmp.pm_inusemap[(cn / N_INUSEBITS) as usize];
    w.store(
        w.load(Ordering::Relaxed) & !(1 << (cn % N_INUSEBITS)),
        Ordering::Relaxed,
    );
}

/// The in-use bitmap's word `idx`.
fn usemap_word<G: ProcGlue>(pmp: &Msdosfsmount<G>, idx: u32) -> u32 {
    pmp.pm_inusemap[idx as usize].load(Ordering::Relaxed)
}

/// `clusterfree(pmp, cluster, oldcnp)`: free one cluster; its old FAT entry goes to `*oldcnp`
/// when given.
pub fn clusterfree<G: ProcGlue>(
    pmp: &Msdosfsmount<G>,
    cluster: u32,
    oldcnp: Option<&mut u32>,
) -> Result<(), Errno> {
    let mut oldcn = 0;

    usemap_free(pmp, cluster);
    if let Err(error) = fatentry(FAT_GET_AND_SET, pmp, cluster, Some(&mut oldcn), MSDOSFSFREE) {
        usemap_alloc(pmp, cluster);
        return Err(error);
    }
    // If the cluster was successfully marked free, then update the count of free clusters,
    // and turn off the "allocated" bit in the "in use" cluster bit map.
    if let Some(oldcnp) = oldcnp {
        *oldcnp = oldcn;
    }
    Ok(())
}

/// `fatentry(function, pmp, cn, oldcontents, newcontents)`: get or set or 'get and set' the
/// cluster'th entry in the fat.
///
/// - `function`: whether to get or set a fat entry (`FAT_GET`, `FAT_SET`)
/// - `pmp`: the msdosfsmount structure for the filesystem whose fat is to be manipulated
/// - `cn`: which cluster is of interest
/// - `oldcontents`: receives the contents of the cluster'th entry if this is a get function
/// - `newcontents`: the new value to be written into the cluster'th element of the fat if
///   this is a set function
///
/// This function can also be used to free a cluster by setting the fat entry for a cluster to
/// 0. All copies of the fat are updated if this is a set function. NOTE: If fatentry() marks
/// a cluster as free it does not update the inusemap in the msdosfsmount structure. This is
/// left to the caller.
pub fn fatentry<G: ProcGlue>(
    function: i32,
    pmp: &Msdosfsmount<G>,
    cn: u32,
    oldcontents: Option<&mut u32>,
    newcontents: u32,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    {
        // Be sure they asked us to do something.
        if function & (FAT_SET | FAT_GET) == 0 {
            printf(format_args!(
                "fatentry(): function code doesn't specify get or set\n"
            ));
            return Err(Errno::EINVAL);
        }

        // If they asked us to return a cluster number but didn't tell us where to put it,
        // give them an error.
        if function & FAT_GET != 0 && oldcontents.is_none() {
            printf(format_args!(
                "fatentry(): get function with no place to put result\n"
            ));
            return Err(Errno::EINVAL);
        }
    }

    // Be sure the requested cluster is in the filesystem.
    if cn < CLUST_FIRST || cn > pmp.pm_maxcluster {
        return Err(Errno::EINVAL);
    }

    let fatmask = pmp.pm_fatmask;
    let byteoffset = fatofs(pmp, cn);
    let (bn, bsize, bo) = fatblock(pmp, byteoffset);
    let mut bp = fat_bread(pmp, bn, bsize)?;

    if function & FAT_GET != 0 {
        let mut readcn = fat_read(fatmask, bp.bytes(), bo, cn);
        // map reserved fat entries to same values for all fats
        if (readcn | !fatmask) >= CLUST_RSRVD {
            readcn |= !fatmask;
        }
        if let Some(oldcontents) = oldcontents {
            *oldcontents = readcn;
        }
    }
    if function & FAT_SET != 0 {
        fat_write(fatmask, bp.data(), bo, cn, newcontents);
        updatefats(pmp, bp, bn);
        pmp.pm_fmod.set(1);
        return Ok(());
    }
    brelse(bp);
    Ok(())
}

/// `fatchain(pmp, start, count, fillwith)`: update a contiguous cluster chain.
///
/// - `pmp`: mount point
/// - `start`: first cluster of chain
/// - `count`: number of clusters in chain
/// - `fillwith`: what to write into fat entry of last cluster
fn fatchain<G: ProcGlue>(
    pmp: &Msdosfsmount<G>,
    mut start: u32,
    mut count: u32,
    fillwith: u32,
) -> Result<(), Errno> {
    // Be sure the clusters are in the filesystem.
    if start < CLUST_FIRST || start.wrapping_add(count).wrapping_sub(1) > pmp.pm_maxcluster {
        return Err(Errno::EINVAL);
    }

    let fatmask = pmp.pm_fatmask;
    while count > 0 {
        let byteoffset = fatofs(pmp, start);
        let (bn, bsize, mut bo) = fatblock(pmp, byteoffset);
        let mut bp = fat_bread(pmp, bn, bsize)?;
        let data = bp.data();
        while count > 0 {
            // The entry at `bo` is cluster `start` (before the increment); it points at the
            // next cluster, or holds `fillwith` for the last one.
            start = start.wrapping_add(1);
            count -= 1;
            let newc = if count > 0 { start } else { fillwith };
            fat_write(fatmask, data, bo, start.wrapping_sub(1), newc);
            match fatmask {
                FAT12_MASK => {
                    bo += 1;
                    if start & 1 == 0 {
                        bo += 1;
                    }
                }
                FAT16_MASK => bo += 2,
                FAT32_MASK => bo += 4,
                _ => {}
            }
            if bo >= bsize as usize {
                break;
            }
        }
        updatefats(pmp, bp, bn);
    }
    pmp.pm_fmod.set(1);
    Ok(())
}

/// `chainlength(pmp, start, count)`: check the length of a free cluster chain starting at
/// `start`, up to `count` (the maximum interesting length).
pub fn chainlength<G: ProcGlue>(pmp: &Msdosfsmount<G>, mut start: u32, count: u32) -> u32 {
    let maxcluster = pmp.pm_maxcluster;
    if start > maxcluster {
        return 0;
    }
    let max_idx = maxcluster / N_INUSEBITS;
    let mut idx = start / N_INUSEBITS;
    // As the C, `start` is the bit in its word from here on, also in the bound below.
    start %= N_INUSEBITS;
    let tail = maxcluster.wrapping_sub(start).wrapping_add(1);
    let mut map = usemap_word(pmp, idx);
    map &= !((1u32 << start) - 1);
    if map != 0 {
        let mut len = map.trailing_zeros() - start;
        len = min(len, count);
        len = min(len, tail);
        return len;
    }
    let mut len = N_INUSEBITS - start;
    if len >= count {
        len = min(count, tail);
        return len;
    }
    loop {
        idx += 1;
        if idx > max_idx {
            break;
        }
        if len >= count {
            break;
        }
        map = usemap_word(pmp, idx);
        if map != 0 {
            len = len.wrapping_add(map.trailing_zeros());
            break;
        }
        len = len.wrapping_add(N_INUSEBITS);
    }
    len = min(len, count);
    len = min(len, tail);
    len
}

/// `chainalloc(pmp, start, count, fillwith, retcluster, got)`: allocate the contiguous free
/// clusters `start` to `start + count - 1`.
///
/// - `fillwith`: put this value into the fat entry for the last allocated cluster
/// - `retcluster`: put the first allocated cluster's number here
/// - `got`: how many clusters were actually allocated
pub fn chainalloc<G: ProcGlue>(
    pmp: &Msdosfsmount<G>,
    start: u32,
    count: u32,
    fillwith: u32,
    retcluster: Option<&mut u32>,
    got: Option<&mut u32>,
) -> Result<(), Errno> {
    for k in 0..count {
        usemap_alloc(pmp, start.wrapping_add(k));
    }
    fatchain(pmp, start, count, fillwith)?;
    if let Some(retcluster) = retcluster {
        *retcluster = start;
    }
    if let Some(got) = got {
        *got = count;
    }
    Ok(())
}

/// One of `clusteralloc`'s two scans of the in-use bitmap, over the clusters from `from` to
/// `end` (excluded): the first free run of `count` clusters, or `None` after recording the
/// longest shorter run in `foundcn`/`foundl`.
fn scanfree<G: ProcGlue>(
    pmp: &Msdosfsmount<G>,
    from: u32,
    end: u64,
    count: u32,
    foundcn: &mut u32,
    foundl: &mut u32,
) -> Option<u32> {
    let mut cn = from;
    while u64::from(cn) < end {
        let idx = cn / N_INUSEBITS;
        let mut map = usemap_word(pmp, idx);
        map |= (1u32 << (cn % N_INUSEBITS)) - 1;
        if map != u32::MAX {
            cn = idx * N_INUSEBITS + (map ^ u32::MAX).trailing_zeros();
            let l = chainlength(pmp, cn, count);
            if l >= count {
                return Some(cn);
            }
            if l > *foundl {
                *foundcn = cn;
                *foundl = l;
            }
            cn = cn.wrapping_add(l).wrapping_add(1);
            continue;
        }
        cn = cn.wrapping_add(N_INUSEBITS - cn % N_INUSEBITS);
    }
    None
}

/// `clusteralloc(pmp, start, count, retcluster, got)`: allocate contiguous free clusters.
///
/// - `start`: preferred start of cluster chain (0 for a new file)
/// - `count`: number of clusters requested
/// - `retcluster`: put the first allocated cluster's number here
/// - `got`: how many clusters were actually allocated
pub fn clusteralloc<G: ProcGlue>(
    pmp: &Msdosfsmount<G>,
    mut start: u32,
    count: u32,
    retcluster: Option<&mut u32>,
    got: Option<&mut u32>,
) -> Result<(), Errno> {
    let fillwith = CLUST_EOFE;
    let len;

    if start != 0 {
        len = chainlength(pmp, start, count);
        if len >= count {
            return chainalloc(pmp, start, count, fillwith, retcluster, got);
        }
    } else {
        // This is a new file, initialize start.
        let tv = microtime();
        start = ((tv.tv_usec >> 10) | tv.tv_usec) as u32;
        len = 0;
    }

    // Start at a (pseudo) random place to maximize cluster runs under multiple writers.
    let maxcluster = pmp.pm_maxcluster;
    let newst = start.wrapping_mul(1103515245).wrapping_add(12345) % maxcluster.wrapping_add(1);
    let mut foundl = 0;
    let mut foundcn = 0;

    // From `newst` to the last cluster, then from 0 up to `newst`.
    for (from, end) in [(newst, u64::from(maxcluster) + 1), (0, u64::from(newst))] {
        if let Some(cn) = scanfree(pmp, from, end, count, &mut foundcn, &mut foundl) {
            return chainalloc(pmp, cn, count, fillwith, retcluster, got);
        }
    }

    if foundl == 0 {
        return Err(Errno::ENOSPC);
    }

    if len != 0 {
        chainalloc(pmp, start, len, fillwith, retcluster, got)
    } else {
        chainalloc(pmp, foundcn, foundl, fillwith, retcluster, got)
    }
}

/// `freeclusterchain(pmp, cluster)`: free a chain of clusters, from `cluster` (the number of
/// the 1st cluster in the chain of clusters to be freed) to its end.
pub fn freeclusterchain<G: ProcGlue>(pmp: &Msdosfsmount<G>, mut cluster: u32) -> Result<(), Errno> {
    let fatmask = pmp.pm_fatmask;
    // `bp` is the FAT block changed last, `lbn` its number (-1, none yet).
    let mut bp: Option<BufRef> = None;
    let mut lbn = u32::MAX;

    while cluster >= CLUST_FIRST && cluster <= pmp.pm_maxcluster {
        let byteoffset = fatofs(pmp, cluster);
        let (bn, bsize, bo) = fatblock(pmp, byteoffset);
        if lbn != bn || bp.is_none() {
            if let Some(old) = bp.take() {
                updatefats(pmp, old, lbn);
            }
            bp = Some(fat_bread(pmp, bn, bsize)?);
            lbn = bn;
        }
        usemap_free(pmp, cluster);
        if let Some(b) = &mut bp {
            let data = b.data();
            let next = fat_read(fatmask, data, bo, cluster);
            fat_write(fatmask, data, bo, cluster, MSDOSFSFREE);
            cluster = next;
        }
        if (cluster | !fatmask) >= CLUST_RSRVD {
            cluster |= fatmask;
        }
    }
    if let Some(b) = bp {
        updatefats(pmp, b, lbn);
    }
    Ok(())
}

/// `fillinusemap(pmp)`: read in fat blocks looking for free clusters. For every free cluster
/// found turn off its corresponding bit in the `pm_inusemap`.
pub fn fillinusemap<G: ProcGlue>(pmp: &Msdosfsmount<G>) -> Result<(), Errno> {
    let fatmask = pmp.pm_fatmask;

    // Mark all clusters in use, we mark the free ones in the fat scan loop further down.
    for w in pmp.pm_inusemap.iter() {
        w.store(u32::MAX, Ordering::Relaxed);
    }

    // Figure how many free clusters are in the filesystem by ripping through the fat counting
    // the number of entries whose content is zero. These represent free clusters.
    pmp.pm_freeclustercount.set(0);
    let mut bp: Option<BufRef> = None;
    for cn in CLUST_FIRST..=pmp.pm_maxcluster {
        let byteoffset = fatofs(pmp, cn);
        let bo = (byteoffset % pmp.pm_fatblocksize) as usize;
        if bo == 0 || bp.is_none() {
            // Read new FAT block.
            if let Some(old) = bp.take() {
                brelse(old);
            }
            let (bn, bsize, _) = fatblock(pmp, byteoffset);
            bp = Some(fat_bread(pmp, bn, bsize)?);
        }
        let readcn = match &bp {
            Some(b) => fat_read(fatmask, b.bytes(), bo, cn),
            None => continue,
        };

        if readcn == 0 {
            usemap_free(pmp, cn);
        }
    }
    if let Some(b) = bp {
        brelse(b);
    }
    Ok(())
}

/// `extendfile(dep, count, bpp, ncp, flags)`: allocate a new cluster and chain it onto the
/// end of the file.
///
/// - `dep`: the file to extend
/// - `count`: number of clusters to allocate
/// - `bpp`: where to return the token of the buf header for the first new file block
/// - `ncp`: where to put cluster number of the first newly allocated cluster; if `None`, do
///   not return the cluster number
/// - `flags`: see fat.h (`DE_CLEAR`)
///
/// NOTE: This function is not responsible for turning on the `DE_UPDATE` bit of the
/// `de_flag` field of the denode and it does not change the `de_FileSize` field. This is left
/// for the caller to do.
pub fn extendfile<G: ProcGlue>(
    dep: &Denode<G>,
    mut count: u32,
    mut bpp: Option<&mut Option<BufRef>>,
    mut ncp: Option<&mut u32>,
    flags: i32,
) -> Result<(), Errno> {
    let pmp = dep.pmp();

    // Don't try to extend the root directory.
    if dep.de_StartCluster.get() == MSDOSFSROOT && dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        printf(format_args!(
            "extendfile(): attempt to extend root directory\n"
        ));
        return Err(Errno::ENOSPC);
    }

    // If the "file's last cluster" cache entry is empty, and the file is not empty, then fill
    // the cache entry by calling pcbmap().
    FC_FILEEXTENDS.fetch_add(1, Ordering::Relaxed);
    if dep.fc(FC_LASTFC).fc_frcn == FCE_EMPTY && dep.de_StartCluster.get() != 0 {
        FC_LFCEMPTY.fetch_add(1, Ordering::Relaxed);
        let mut cn = 0;
        // we expect it to return E2BIG
        match pcbmap(dep, CLUST_END, None, Some(&mut cn), None) {
            Err(Errno::E2BIG) => {}
            other => return other,
        }
    }

    // Preserve value for the last cluster before extending the file to speed up further
    // lookups.
    let lastfc = dep.fc(FC_LASTFC);
    fc_setcache(dep, FC_OLASTFC, lastfc.fc_frcn, lastfc.fc_fsrcn);

    while count > 0 {
        // Allocate a new cluster chain and cat onto the end of the file. If the file is empty
        // we make de_StartCluster point to the new block. Note that de_StartCluster being 0
        // is sufficient to be sure the file is empty since we exclude attempts to extend the
        // root directory above, and the root dir is the only file with a startcluster of 0
        // that has blocks allocated (sort of).
        let start = if dep.de_StartCluster.get() == 0 {
            0
        } else {
            dep.fc(FC_LASTFC).fc_fsrcn.wrapping_add(1)
        };
        let mut cn = 0;
        let mut got = 0;
        clusteralloc(pmp, start, count, Some(&mut cn), Some(&mut got))?;

        count -= got;

        // Give them the filesystem relative cluster number if they want it.
        if let Some(ncp) = ncp.take() {
            *ncp = cn;
        }

        let mut frcn;
        if dep.de_StartCluster.get() == 0 {
            dep.de_StartCluster.set(cn);
            frcn = 0;
        } else {
            let lastfc = dep.fc(FC_LASTFC);
            if let Err(error) = fatentry(FAT_SET, pmp, lastfc.fc_fsrcn, None, cn) {
                let _ = clusterfree(pmp, cn, None);
                return Err(error);
            }
            frcn = lastfc.fc_frcn.wrapping_add(1);
        }

        // Update the "last cluster of the file" entry in the denode's fat cache.
        fc_setcache(
            dep,
            FC_LASTFC,
            frcn.wrapping_add(got).wrapping_sub(1),
            cn.wrapping_add(got).wrapping_sub(1),
        );

        if flags & DE_CLEAR != 0 {
            while got > 0 {
                got -= 1;
                // Get the buf header for the new block of the file.
                let mut bp = if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
                    let bp = getblk_wait(
                        &pmp.pm_devvp.bufs,
                        Daddr::from(cntobn(pmp, cn)),
                        pmp.pm_bpcluster as i32,
                    );
                    cn = cn.wrapping_add(1);
                    bp
                } else {
                    let bp = getblk_wait(
                        &dep.detov().bufs,
                        Daddr::from(frcn),
                        pmp.pm_bpcluster as i32,
                    );
                    frcn = frcn.wrapping_add(1);
                    // Do the bmap now, as in msdosfs_write.
                    let b = bp.buf();
                    let mut blkno = b.blkno();
                    if pcbmap(dep, b.lblkno() as u32, Some(&mut blkno), None, None).is_err() {
                        blkno = -1;
                    }
                    b.set_blkno(blkno);
                    if b.blkno() == -1 {
                        kpanic(format_args!("extendfile: pcbmap"));
                    }
                    bp
                };
                bp.clear();
                if let Some(bpp) = bpp.take() {
                    *bpp = Some(bp);
                } else {
                    bdwrite(bp);
                }
            }
        }
    }

    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of `msdosfs_fat.rs`: the FAT entry packing of the three widths over a block's
    // bytes, the FAT block arithmetic, the fat cache, the in-use bitmap searches, and the FAT
    // walks over a FAT laid on testffs's disk (pcbmap with D-N5.7-U2's straddling entry,
    // fatentry with its mirrors and the FSInfo count, fillinusemap, extendfile and
    // freeclusterchain).

    use super::*;
    use crate::fs::msdosfs::denode::DenodeRef;
    use crate::fs::msdosfs::msdosfsmount::{MsdosfsmountRef, test_pmp};
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs;
    use std::vec;
    use std::vec::Vec;

    /// The FAT bytes of `entries` (cluster 0 first) for a FAT of the given width, written through
    /// `fat_write`, and checked back through `fat_read`.
    fn pack(fatmask: u32, mult: u32, div: u32, entries: &[u32]) -> Vec<u8> {
        let mut fat = vec![0u8; entries.len() * 4 + 4];
        for (cn, &v) in entries.iter().enumerate() {
            let bo = (cn as u32 * mult / div) as usize;
            fat_write(fatmask, &mut fat, bo, cn as u32, v);
        }
        for (cn, &v) in entries.iter().enumerate() {
            let bo = (cn as u32 * mult / div) as usize;
            assert_eq!(
                fat_read(fatmask, &fat, bo, cn as u32),
                v & fatmask,
                "cluster {cn}"
            );
        }
        fat
    }

    #[test]
    fn fat12_entries_share_nibbles() {
        // Clusters 0..3: media 0xff8, 0xfff, then 3 -> 0xfff (EOF), 0x123.
        let fat = pack(FAT12_MASK, 3, 2, &[0xff8, 0xfff, 0xfff, 0x123]);
        assert_eq!(&fat[..6], &[0xf8, 0xff, 0xff, 0xff, 0x3f, 0x12]);
        // Rewriting an odd entry leaves its even neighbour alone, and the reverse.
        let mut fat = fat;
        fat_write(FAT12_MASK, &mut fat, 4, 3, 0xabc);
        assert_eq!(fat_read(FAT12_MASK, &fat, 3, 2), 0xfff);
        assert_eq!(fat_read(FAT12_MASK, &fat, 4, 3), 0xabc);
        fat_write(FAT12_MASK, &mut fat, 3, 2, 0x005);
        assert_eq!(fat_read(FAT12_MASK, &fat, 4, 3), 0xabc);
        assert_eq!(&fat[3..6], &[0x05, 0xc0, 0xab]);
        // CLUST_EOFE is cut to 12 bits.
        fat_write(FAT12_MASK, &mut fat, 4, 3, CLUST_EOFE);
        assert_eq!(fat_read(FAT12_MASK, &fat, 4, 3), 0xfff);
        assert_eq!(fat_read(FAT12_MASK, &fat, 3, 2), 0x005);
    }

    #[test]
    fn fat16_and_fat32_entries() {
        let fat = pack(FAT16_MASK, 2, 1, &[0xfff8, 0xffff, 0x0003, 0xffff]);
        assert_eq!(&fat[..8], &[0xf8, 0xff, 0xff, 0xff, 0x03, 0x00, 0xff, 0xff]);
        let mut fat = pack(FAT32_MASK, 4, 1, &[0x0fff_fff8, 0x0fff_ffff, 0x0000_0003]);
        assert_eq!(&fat[8..12], &[0x03, 0, 0, 0]);
        // FAT32 keeps the high four bits of an entry, and reads past them.
        fat[11] = 0xa0;
        assert_eq!(fat_read(FAT32_MASK, &fat, 8, 2), 3);
        fat_write(FAT32_MASK, &mut fat, 8, 2, CLUST_EOFE);
        assert_eq!(&fat[8..12], &[0xff, 0xff, 0xff, 0xaf]);
        assert_eq!(fat_read(FAT32_MASK, &fat, 8, 2), FAT32_MASK);
        // An unknown width writes nothing.
        fat_write(0, &mut fat, 8, 2, 0);
        assert_eq!(&fat[8..12], &[0xff, 0xff, 0xff, 0xaf]);
        assert_eq!(
            (
                fat_width(FAT12_MASK),
                fat_width(FAT16_MASK),
                fat_width(FAT32_MASK)
            ),
            (2, 2, 4)
        );
    }

    #[test]
    fn fat_blocks() {
        let (_g, _p) = testffs::attach(vec![0u8; 4096]);
        // 512-byte sectors: FAT12 blocks of 3 sectors, a 12-sector FAT at block 1, two copies.
        let geom = |secs: u32, curfat: u32| {
            move |pm: &mut Msdosfsmount<TestGlue>| {
                pm.pm_fatblocksize = 3 * 512;
                pm.pm_fatblocksec = 3;
                pm.pm_FATsecs = secs;
                pm.pm_fatblk = 1;
                pm.pm_curfat = curfat;
            }
        };
        let pmp = test_pmp(FAT12_MASK, 4000, geom(12, 0));
        assert_eq!(fatblock(&pmp, fatofs(&pmp, 0)), (1, 1536, 0));
        // Cluster 1025 is at byte 1537: the second block, offset 1.
        assert_eq!(fatblock(&pmp, fatofs(&pmp, 1025)), (4, 1536, 1));
        // The last block is cut at the end of the FAT.
        let pmp = test_pmp(FAT12_MASK, 4000, geom(10, 0));
        assert_eq!(fatblock(&pmp, 5000), (10, 512, 5000 - 3 * 1536));
        // A block past the FAT: FATsecs - bn wraps, as the C's, to a whole block's size.
        assert_eq!(fatblock(&pmp, 4 * 1536).1, 1536);
        // The current FAT of a FAT32 file system that does not mirror.
        let pmp = test_pmp(FAT12_MASK, 4000, geom(10, 1));
        assert_eq!(fatblock(&pmp, 0), (11, 1536, 0));
    }

    #[test]
    fn fat_cache() {
        let (_g, _p) = testffs::attach(vec![0u8; 4096]);
        let pmp = test_pmp(FAT16_MASK, 100, |_| {});
        let dep = Denode::new(None, pmp.pm_dev, pmp, 0, 0);
        fc_purge(&dep, 0);
        let (mut i, mut cn) = (0, 77);
        fc_lookup(&dep, 10, &mut i, &mut cn);
        assert_eq!((i, cn), (0, 77), "an empty cache leaves the start");
        fc_setcache(&dep, FC_LASTMAP, 4, 104);
        fc_setcache(&dep, FC_LASTFC, 9, 109);
        fc_setcache(&dep, FC_OLASTFC, 6, 106);
        fc_lookup(&dep, 8, &mut i, &mut cn);
        assert_eq!((i, cn), (6, 106));
        fc_lookup(&dep, 100, &mut i, &mut cn);
        assert_eq!((i, cn), (9, 109));
        fc_purge(&dep, 6);
        assert_eq!(dep.fc(FC_LASTMAP).fc_frcn, 4);
        assert_eq!(dep.fc(FC_LASTFC).fc_frcn, FCE_EMPTY);
        assert_eq!(dep.fc(FC_OLASTFC).fc_frcn, FCE_EMPTY);
        assert_eq!(
            dep.fc(FC_LASTFC).fc_fsrcn,
            109,
            "a purge keeps fc_fsrcn, as the C's"
        );
        fc_lookup(&dep, 100, &mut i, &mut cn);
        assert_eq!((i, cn), (4, 104));
    }

    #[test]
    fn usemap_and_chainlength() {
        let (_g, _p) = testffs::attach(vec![0u8; 4096]);
        let pmp = test_pmp(FAT16_MASK, 100, |_| {});
        let pmp = &*pmp;
        assert_eq!(chainlength(pmp, 2, 10), 10);
        assert_eq!(chainlength(pmp, 101, 1), 0);
        assert_eq!(chainlength(pmp, 2, 1000), 99);
        usemap_alloc(pmp, 40);
        assert_eq!(pmp.pm_freeclustercount.get(), 98);
        assert_eq!(chainlength(pmp, 30, 20), 10);
        assert_eq!(chainlength(pmp, 40, 20), 0);
        assert_eq!(chainlength(pmp, 2, 100), 38);
        usemap_free(pmp, 40);
        assert_eq!(pmp.pm_freeclustercount.get(), 99);
        assert_eq!(chainlength(pmp, 30, 20), 20);
        // A run that starts in one word and ends in the next.
        usemap_alloc(pmp, 70);
        assert_eq!(chainlength(pmp, 60, 50), 10);
        // A run up to the last cluster stops at the in-use bits past it.
        assert_eq!(chainlength(pmp, 71, 100), 30);
    }

    #[test]
    fn scanfree_finds_runs() {
        let (_g, _p) = testffs::attach(vec![0u8; 4096]);
        let pmp = test_pmp(FAT16_MASK, 100, |_| {});
        let pmp = &*pmp;
        for cn in 2..50 {
            usemap_alloc(pmp, cn);
        }
        usemap_alloc(pmp, 60);
        let (mut foundcn, mut foundl) = (0, 0);
        // From 10: the first free cluster is 50, a run of 10.
        assert_eq!(
            scanfree(pmp, 10, 101, 5, &mut foundcn, &mut foundl),
            Some(50)
        );
        assert_eq!(
            scanfree(pmp, 10, 101, 20, &mut foundcn, &mut foundl),
            Some(61)
        );
        assert_eq!(scanfree(pmp, 10, 101, 50, &mut foundcn, &mut foundl), None);
        assert_eq!((foundcn, foundl), (61, 40));
        let (mut foundcn, mut foundl) = (0, 0);
        assert_eq!(scanfree(pmp, 0, 55, 50, &mut foundcn, &mut foundl), None);
        assert_eq!((foundcn, foundl), (50, 10));
    }

    /// A FAT12 file system's geometry on testffs's 1 MiB disk: 512-byte sectors, one sector a
    /// cluster, two 4-sector FATs from block 1 (mirrored), so the second FAT block is a short
    /// one (one sector of 512 bytes, the C's `min(fatblocksec, FATsecs - bn)`), clusters from
    /// block 9; `maxcluster` 1000.
    const FATBLK: usize = 1;
    const FATSECS: u32 = 4;
    const FIRSTCLUSTER: u32 = 9;

    fn fat12(pm: &mut Msdosfsmount<TestGlue>) {
        pm.pm_bpb.bpbBytesPerSec = 512;
        pm.pm_bpb.bpbFATs = 2;
        pm.pm_BlkPerSec = 1;
        pm.pm_FATsecs = FATSECS;
        pm.pm_fatblk = FATBLK as u32;
        pm.pm_firstcluster = FIRSTCLUSTER;
        pm.pm_fatblocksize = 3 * 512;
        pm.pm_fatblocksec = 3;
        pm.pm_fatsize = FATSECS * 512;
        pm.pm_bnshift = 9;
        pm.pm_bpcluster = 512;
        pm.pm_crbomask = 511;
        pm.pm_cnshift = 9;
        pm.pm_flags.set(MSDOSFS_FATMIRROR);
    }

    /// The disk of [`fat12`]'s file system with the FAT12 entries `chain` (cluster, value)
    /// in its first FAT.
    fn fat12_disk(chain: &[(u32, u32)]) -> Vec<u8> {
        let mut disk = vec![0u8; 1 << 20];
        let fat = &mut disk[FATBLK * 512..(FATBLK + FATSECS as usize) * 512];
        fat_write(FAT12_MASK, fat, 0, 0, 0xff8);
        fat_write(FAT12_MASK, fat, 1, 1, 0xfff);
        for &(cn, v) in chain {
            let bo = (cn * 3 / 2) as usize;
            fat_write(FAT12_MASK, fat, bo, cn, v);
        }
        disk
    }

    /// The mount structure of [`fat12`] over `disk` and a file starting at cluster `start`.
    fn fat12_file(
        disk: Vec<u8>,
        start: u32,
    ) -> (
        std::sync::MutexGuard<'static, ()>,
        MsdosfsmountRef<TestGlue>,
        DenodeRef<TestGlue>,
    ) {
        let (g, _p) = testffs::attach(disk);
        let pmp = test_pmp(FAT12_MASK, 1000, fat12);
        let vp = crate::fs::subr::getnewvnode::<TestGlue>(
            crate::fs::vnode::VT_MSDOSFS,
            Some(pmp.pm_mountp),
            &crate::fs::testfs::TEST_VOPS,
        )
        .unwrap();
        let dep = Denode::new(Some(vp), pmp.pm_dev, pmp.clone(), 0, 0);
        fc_purge(&dep, 0);
        dep.de_StartCluster.set(start);
        (g, pmp, dep.alloc())
    }

    #[test]
    fn pcbmap_walks_the_chain() {
        // 5 -> 6 -> 9 -> EOF.
        let (_g, pmp, dep) = fat12_file(fat12_disk(&[(5, 6), (6, 9), (9, 0xfff)]), 5);
        let (mut bn, mut cn, mut sz) = (0, 0, 0);
        assert_eq!(
            pcbmap(&dep, 0, Some(&mut bn), Some(&mut cn), Some(&mut sz)),
            Ok(())
        );
        assert_eq!((bn, cn, sz), (i64::from(FIRSTCLUSTER + 3), 5, 512));
        assert_eq!(pcbmap(&dep, 2, Some(&mut bn), Some(&mut cn), None), Ok(()));
        assert_eq!((bn, cn), (i64::from(cntobn(&*pmp, 9u32)), 9));
        assert_eq!(
            dep.fc(FC_LASTMAP),
            crate::fs::msdosfs::denode::Fatcache {
                fc_frcn: 2,
                fc_fsrcn: 9
            }
        );
        // Past the end: the file's length in clusters with E2BIG, the last cluster cached.
        assert_eq!(
            pcbmap(&dep, 7, None, Some(&mut cn), None),
            Err(Errno::E2BIG)
        );
        assert_eq!(cn, 3);
        assert_eq!(
            (dep.fc(FC_LASTFC).fc_frcn, dep.fc(FC_LASTFC).fc_fsrcn),
            (2, 9)
        );
        // Nothing to return: nothing done.
        assert_eq!(pcbmap(&dep, 1, None, None, None), Ok(()));
    }

    #[test]
    fn pcbmap_refuses_an_entry_that_straddles_a_short_fat_block() {
        // D-N5.7-U2: the second FAT block is one sector (FATsecs 4 = 3 + 1). A FAT12 chain value
        // above pm_maxcluster whose entry starts at the block's last byte: cluster 1365 is at
        // byte 1365 * 3 / 2 = 2047, the second block's byte 511.
        let mut cn = 0;
        {
            let (_g, _pmp, dep) = fat12_file(fat12_disk(&[(5, 1365)]), 5);
            let (bn, bsize, bo) = fatblock(dep.pmp(), fatofs(dep.pmp(), 1365));
            assert_eq!((bn, bsize, bo), (FATBLK as u32 + 3, 512, 511));
            assert_eq!(pcbmap(&dep, 2, None, Some(&mut cn), None), Err(Errno::EIO));
        }
        // Its even neighbour 1364 lies whole in the block (bytes 510 and 511): followed, as the
        // C follows it, to the 0 read there (a garbage cluster number, no error).
        let (_g, _pmp, dep) = fat12_file(fat12_disk(&[(5, 1364)]), 5);
        assert_eq!(fatblock(dep.pmp(), fatofs(dep.pmp(), 1364)).2, 510);
        assert_eq!(pcbmap(&dep, 2, None, Some(&mut cn), None), Ok(()));
        assert_eq!(cn, 0);
    }

    #[test]
    fn fatentry_writes_every_fat_and_the_fsinfo_count() {
        let (_g, pmp, _dep) = fat12_file(fat12_disk(&[(5, 6), (6, 0xfff)]), 5);
        let mut old = 0;
        assert_eq!(fatentry(FAT_GET, &pmp, 5, Some(&mut old), 0), Ok(()));
        assert_eq!(old, 6);
        // Reserved entries read as the same value for every width.
        assert_eq!(fatentry(FAT_GET, &pmp, 6, Some(&mut old), 0), Ok(()));
        assert_eq!(old, CLUST_EOFE);
        assert_eq!(
            fatentry(FAT_GET, &pmp, 1, Some(&mut old), 0),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            fatentry(FAT_GET, &pmp, 1001, Some(&mut old), 0),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            fatentry(FAT_GET_AND_SET, &pmp, 5, Some(&mut old), 0x123),
            Ok(())
        );
        assert_eq!((old, pmp.pm_fmod.get()), (6, 1));
        testffs::sync(&[]);
        let disk = testffs::disk();
        let at = |fat: usize| FATBLK * 512 + fat * FATSECS as usize * 512;
        // The entry in the first FAT and in its mirror, the neighbours kept.
        for f in 0..2 {
            assert_eq!(fat_read(FAT12_MASK, &disk[at(f)..], 7, 5), 0x123, "FAT {f}");
            assert_eq!(fat_read(FAT12_MASK, &disk[at(f)..], 0, 0), 0xff8, "FAT {f}");
        }
        assert_eq!(fat_read(FAT12_MASK, &disk[at(0)..], 9, 6), 0xfff);
    }

    #[test]
    fn updatefats_writes_the_free_count_into_fsinfo() {
        let mut disk = fat12_disk(&[]);
        disk[100 * 512 + 488..100 * 512 + 492].copy_from_slice(&7u32.to_le_bytes());
        disk[100 * 512 + 1020] = 0x55;
        let (_g, pmp, _dep) = fat12_file(disk, 5);
        pmp.pm_fsinfo.set(100);
        pmp.pm_freeclustercount.set(4242);
        assert_eq!(fatentry(FAT_SET, &pmp, 3, None, 0xfff), Ok(()));
        testffs::sync(&[]);
        let disk = testffs::disk();
        assert_eq!(getulong(&disk[100 * 512 + 488..]), 4242);
        assert_eq!(disk[100 * 512 + 1020], 0x55, "the rest of the block kept");
        assert_eq!(pmp.pm_fsinfo.get(), 100);
    }

    #[test]
    fn fillinusemap_counts_the_free_clusters() {
        // Clusters 2..=5 in use, one bad cluster, the rest free.
        let chain = [(2, 3), (3, 0xfff), (4, 0xfff), (5, 0xfff), (7, 0xff7)];
        let (_g, pmp, _dep) = fat12_file(fat12_disk(&chain), 5);
        assert_eq!(fillinusemap(&pmp), Ok(()));
        assert_eq!(pmp.pm_freeclustercount.get(), 999 - 5);
        assert_eq!(chainlength(&pmp, 2, 10), 0);
        assert_eq!(chainlength(&pmp, 6, 10), 1);
        assert_eq!(chainlength(&pmp, 8, 2000), 993);
    }

    #[test]
    fn extendfile_and_freeclusterchain() {
        // An empty file grows by three clusters, then by one more, then is freed.
        let (_g, pmp, dep) = fat12_file(fat12_disk(&[]), 0);
        assert_eq!(fillinusemap(&pmp), Ok(()));
        let free = pmp.pm_freeclustercount.get();
        let mut first = 0;
        assert_eq!(extendfile(&dep, 3, None, Some(&mut first), 0), Ok(()));
        let start = dep.de_StartCluster.get();
        assert_eq!(start, first);
        assert_eq!(pmp.pm_freeclustercount.get(), free - 3);
        let mut cn = 0;
        assert_eq!(
            pcbmap(&dep, 3, None, Some(&mut cn), None),
            Err(Errno::E2BIG)
        );
        assert_eq!(cn, 3, "three clusters chained");
        // The new block of a cleared extension comes back to the caller, zeroed.
        let mut bp = None;
        assert_eq!(extendfile(&dep, 1, Some(&mut bp), None, DE_CLEAR), Ok(()));
        let Some(t) = bp else { std::panic!("no block") };
        assert_eq!(t.buf().lblkno(), 3);
        assert!(t.bytes().iter().all(|&b| b == 0));
        let mut bn = 0;
        assert_eq!(pcbmap(&dep, 3, Some(&mut bn), None, None), Ok(()));
        assert_eq!(t.buf().blkno(), bn, "the bmap done at once");
        brelse(t);
        assert_eq!(pmp.pm_freeclustercount.get(), free - 4);
        assert_eq!(freeclusterchain(&pmp, start), Ok(()));
        assert_eq!(pmp.pm_freeclustercount.get(), free);
        let mut old = 1;
        assert_eq!(fatentry(FAT_GET, &pmp, start, Some(&mut old), 0), Ok(()));
        assert_eq!(old, MSDOSFSFREE);
        // The root directory of a FAT12 file system cannot grow.
        dep.de_StartCluster.set(MSDOSFSROOT);
        dep.de_Attributes.set(ATTR_DIRECTORY);
        assert_eq!(extendfile(&dep, 1, None, None, 0), Err(Errno::ENOSPC));
    }
}
/* </TESTS> */
