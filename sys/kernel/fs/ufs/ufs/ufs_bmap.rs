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
 */
/* </LICENSES> */

/* <CODE> */
//! Block mapping: `bmap` converts the logical block number of a file to its physical block
//! number on the disk. The conversion is done by using the logical block number to index
//! into the array of block pointers described by the dinode.
//!
//! Upstream: sys/ufs/ufs/ufs_bmap.c @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/ufs_bmap.rs@3c62ede68802
//!
//! Indirect blocks are on the vnode for the file. They are given negative logical block
//! numbers. Indirect blocks are addressed by the negative address of the first data block
//! to which they point. Double indirect blocks are addressed by one less than the address of
//! the first indirect block to which they point. Triple indirect blocks are addressed by one
//! less than the address of the first double indirect block to which they point.
//!
//! ## Deviations
//! - `ufs_bmaparray` and `ufs_getlbns` take the `struct indir` array as a slice and its count
//!   as `Option<&mut i32>` (the C's NULL `ap`/`nump` is `None`); the block pointers in an
//!   indirect block are read at their index in the buffer's bytes (`inode.rs`,
//!   `daddr32_at`/`daddr64_at`).
//! - `VFSTOUFS(vp->v_mount)`'s `um_nindir`, `um_bptrtodb` and `um_seqinc` are read from the
//!   inode's copies of them (`inode.rs`), the same words; `ufs_getlbns`'s arithmetic is
//!   [`getlbns`] over the count of pointers per block.
//! - `curproc->p_ru.ru_inblock++` is the vnode's `charge_io` (skipped when no thread runs), as
//!   `fs::bio`'s reads charge theirs.
//!
//! ## Redesign
//! - N5.5a: LZ's `getblk`/`biowait`/`brelse` of `bsd`'s buffer adapters over a `&'static Buf`
//!   and its `unsafe` view of the bytes (sys/ufs/ufs/ufs_bmap.rs:198-200) -> the buffer
//!   cache's token (`BufRef`): `getblk` gives it, `VOP_STRATEGY` takes it, `biowait` gives it
//!   back with the read's result and `brelse` consumes it; the bytes are the token's view.
//!   The same sections of the cache's lock as the C's `splbio` pairs: one per `getblk`, one
//!   per `biowait`, one per `brelse`.

use hal::sys::errno::Errno;
use hal::sys::types::Daddr;

use super::dinode::{NDADDR, NIADDR};
use super::inode::{Indir, Inode, daddr32_at, daddr64_at, vtoi};
use super::ufsmount::UM_UFS2;
use crate::assert::kpanic;
use crate::fs::bio::{BCSTATS, biowait, brelse, getblk, incore};
use crate::fs::buf::{B_DELWRI, B_DONE, B_READ, BufRef};
use crate::fs::vnode::Vnode;
use crate::fs::vops::{IoBuf, vop_strategy};
use crate::proc::ProcGlue;
use hal::sleep::INFSLP;

/// `MAXBSIZE`: the largest transfer (`<sys/param.h>`).
const MAXBSIZE: i32 = 64 * 1024;

/// `ufs_bmap` (`vop_bmap`): the device vnode and the disk block of logical block `bn`.
pub fn ufs_bmap<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    vpp: Option<&mut Option<&'static Vnode<G>>>,
    bnp: Option<&mut Daddr>,
    runp: Option<&mut i32>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    // Check for underlying vnode requests and ensure that logical to physical mapping is
    // requested.
    if let Some(vpp) = vpp {
        *vpp = Some(ip.i_devvp());
    }
    let Some(bnp) = bnp else {
        return Ok(());
    };

    bmaparray(vp, &ip, bn, bnp, None, None, runp)
}

/// The buffer of an indirect block, from `getblk` (which cannot fail without `slpflag`).
fn getblk_wait<G: ProcGlue>(vp: &'static Vnode<G>, blkno: Daddr, size: i32) -> BufRef {
    loop {
        if let Some(t) = getblk(&vp.bufs, blkno, size, 0, INFSLP) {
            return t;
        }
    }
}

/// `ufs_bmaparray`: the bmap conversion, and if requested the array of logical blocks which
/// must be traversed to get to a block. Each entry contains the offset into that block that
/// gets you to the next block and the disk address of the block (if it is assigned).
/// `*bnp` is -1 for a block that is not allocated (a hole); `*runp`, if asked for, the
/// number of blocks that follow it contiguously on the disk.
pub fn ufs_bmaparray<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    bnp: &mut Daddr,
    ap: Option<&mut [Indir]>,
    nump: Option<&mut i32>,
    runp: Option<&mut i32>,
) -> Result<(), Errno> {
    bmaparray(vp, &vtoi(vp), bn, bnp, ap, nump, runp)
}

/// [`ufs_bmaparray`] over the inode `ip` of `vp`, whose handle the caller lends ([`ufs_bmap`]
/// took it already: one `vtoi` per call, as the C's two are one pointer).
fn bmaparray<G: ProcGlue>(
    vp: &'static Vnode<G>,
    ip: &Inode<G>,
    mut bn: Daddr,
    bnp: &mut Daddr,
    ap: Option<&mut [Indir]>,
    nump: Option<&mut i32>,
    mut runp: Option<&mut i32>,
) -> Result<(), Errno> {
    let Some(mp) = vp.mount() else {
        kpanic(format_args!("ufs_bmaparray: vnode without a mount"));
    };

    #[cfg(feature = "diagnostic")]
    if ap.is_some() != nump.is_some() {
        kpanic(format_args!("ufs_bmaparray: invalid arguments"));
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
        *bnp = ip.blkptrtodb(ip.dip_db(bn as usize));
        if *bnp == 0 {
            *bnp = -1;
        } else if let Some(runp) = runp {
            bn += 1;
            while bn < NDADDR as Daddr
                && *runp < maxrun
                && ip.is_sequential(ip.dip_db(bn as usize - 1), ip.dip_db(bn as usize))
            {
                bn += 1;
                *runp += 1;
            }
        }
        return Ok(());
    }

    // Get disk address out of indirect block array.
    let mut daddr = ip.dip_ib(xap[0].in_off as usize);

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
        let mut t = getblk_wait(vp, metalbn, mp.iosize() as i32);
        if t.buf().isset(B_DONE | B_DELWRI) {
            // Already there.
        } else if cfg!(feature = "diagnostic") && daddr == 0 {
            kpanic(format_args!("ufs_bmaparray: indirect block not in cache"));
        } else {
            t.buf().set_blkno(ip.blkptrtodb(daddr));
            t.buf().set(B_READ);
            BCSTATS
                .pendingreads
                .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            BCSTATS
                .numreads
                .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            let Some(r) = t.counted() else {
                kpanic(format_args!("ufs_bmaparray: a driver's buffer"));
            };
            let _ = vop_strategy(vp, IoBuf::Token(t));
            vp.bufs.owner().charge_io(false); // XXX
            let (back, rv) = biowait(r);
            if let Err(e) = rv {
                brelse(back);
                return Err(e);
            }
            t = back;
        }

        let data = t.data();
        let off = xap[x].in_off as usize;
        let ufs2 = ip.fstype() == UM_UFS2;
        daddr = if ufs2 {
            daddr64_at(data, off)
        } else {
            daddr32_at(data, off)
        };
        if num == 1
            && daddr != 0
            && let Some(runp) = runp.as_deref_mut()
        {
            let at = |i: usize| {
                if ufs2 {
                    daddr64_at(data, i)
                } else {
                    daddr32_at(data, i)
                }
            };
            let mut i = off + 1;
            while (i as u64) < ip.nindir() && *runp < maxrun && ip.is_sequential(at(i - 1), at(i)) {
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

    let daddr = ip.blkptrtodb(daddr);
    *bnp = if daddr == 0 { -1 } else { daddr };
    Ok(())
}

/// `ufs_getlbns`: an array of logical block number/offset pairs which represent the path of
/// indirect blocks required to access a data block. The first "pair" contains the logical
/// block number of the appropriate single, double or triple indirect block and the offset
/// into the inode indirect block array. Note, the logical block number of the inode
/// single/double/triple indirect block appears twice in the array, once with the offset into
/// the `i_ffs_ib` and once with the offset into the page itself. `ap` holds at least
/// `NIADDR + 1` entries.
pub fn ufs_getlbns<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    ap: &mut [Indir],
    nump: Option<&mut i32>,
) -> Result<(), Errno> {
    if vp.mount().is_none() {
        kpanic(format_args!("ufs_getlbns: vnode without a mount"));
    }
    getlbns(vtoi(vp).nindir() as i64, bn, ap, nump)
}

/// [`ufs_getlbns`] over `nindir` block pointers per indirect block (`MNINDIR(ump)`).
pub fn getlbns(
    nindir: i64,
    mut bn: Daddr,
    ap: &mut [Indir],
    mut nump: Option<&mut i32>,
) -> Result<(), Errno> {
    if let Some(n) = nump.as_deref_mut() {
        *n = 0;
    }
    let mut numlevels = 0;
    let realbn = bn;
    if bn < 0 {
        bn = -bn;
    }

    #[cfg(feature = "diagnostic")]
    if realbn < 0 && realbn > -(NDADDR as Daddr) {
        kpanic(format_args!(
            "ufs_getlbns: Invalid indirect block {} specified",
            realbn
        ));
    }

    // The first NDADDR blocks are direct blocks.
    if bn < NDADDR as Daddr {
        return Ok(());
    }

    // Determine the number of levels of indirection. After this loop is done, blockcnt
    // indicates the number of data blocks possible at the given level of indirection, and
    // NIADDR - i is the number of levels of indirection needed to locate the requested
    // block.
    let mut blockcnt: i64 = 1;
    let mut i = NIADDR as i64;
    bn -= NDADDR as Daddr;
    loop {
        if i == 0 {
            return Err(Errno::EFBIG);
        }
        blockcnt *= nindir;
        if bn < blockcnt {
            break;
        }
        i -= 1;
        bn -= blockcnt;
    }

    // Calculate the address of the first meta-block.
    let mut metalbn = if realbn >= 0 {
        -(realbn - bn + NIADDR as i64 - i)
    } else {
        -(-realbn - bn + NIADDR as i64 - i)
    };

    // At each iteration, off is the offset into the bap array which is an array of disk
    // addresses at the current level of indirection. The logical block number and the offset
    // in that block are stored into the argument array.
    let mut k = 0;
    ap[k] = Indir {
        in_lbn: metalbn,
        in_off: (NIADDR as i64 - i) as i32,
        in_exists: 0,
    };
    k += 1;
    numlevels += 1;
    while i <= NIADDR as i64 {
        // If searching for a meta-data block, quit when found.
        if metalbn == realbn {
            break;
        }

        blockcnt /= nindir;
        let off = (bn / blockcnt) % nindir;

        numlevels += 1;
        ap[k] = Indir {
            in_lbn: metalbn,
            in_off: off as i32,
            in_exists: 0,
        };
        k += 1;

        metalbn -= -1 + off * blockcnt;
        i += 1;
    }

    #[cfg(feature = "diagnostic")]
    if realbn < 0 && metalbn != realbn {
        kpanic(format_args!(
            "ufs_getlbns: indirect block {} not found",
            realbn
        ));
    }

    if let Some(n) = nump {
        *n = numlevels;
    }
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// `ufs_getlbns` of `bn` with `nindir` pointers per block: the levels and the path.
    fn path(nindir: i64, bn: Daddr) -> Result<(i32, std::vec::Vec<(Daddr, i32)>), Errno> {
        let mut ap = [Indir::default(); NIADDR + 1];
        let mut n = -1;
        getlbns(nindir, bn, &mut ap, Some(&mut n))?;
        Ok((
            n,
            ap[..n as usize]
                .iter()
                .map(|a| (a.in_lbn, a.in_off))
                .collect(),
        ))
    }

    #[test]
    fn getlbns_walks_the_c_paths_at_every_boundary() {
        // ufs_bmap.c:227-308's arithmetic with NDADDR = 12, NIADDR = 3 and 4 pointers per
        // block (so the levels end at 12 + 4 = 16, + 16 = 32, + 64 = 96): each row is a
        // block, the number of levels and the (in_lbn, in_off) pairs the C writes, computed
        // by an independent transcription of the C (an indirect block's lbn is minus the
        // first data block below it, a double's one less than its first child's).
        let n = 4;
        let table: &[(Daddr, i32, &[(Daddr, i32)])] = &[
            (0, 0, &[]),
            (11, 0, &[]),
            (12, 2, &[(-12, 0), (-12, 0)]),
            (15, 2, &[(-12, 0), (-12, 3)]),
            (16, 3, &[(-17, 1), (-17, 0), (-16, 0)]),
            (19, 3, &[(-17, 1), (-17, 0), (-16, 3)]),
            (20, 3, &[(-17, 1), (-17, 1), (-20, 0)]),
            (31, 3, &[(-17, 1), (-17, 3), (-28, 3)]),
            (32, 4, &[(-34, 2), (-34, 0), (-33, 0), (-32, 0)]),
            (95, 4, &[(-34, 2), (-34, 3), (-81, 3), (-92, 3)]),
            // A negative bn names an indirect block: the path stops at it.
            (-12, 1, &[(-12, 0)]),
            (-17, 1, &[(-17, 1)]),
            (-16, 2, &[(-17, 1), (-17, 0)]),
        ];
        for &(bn, levels, want) in table {
            let (got_n, got) = path(n, bn).unwrap();
            assert_eq!(got_n, levels, "bn {bn}");
            assert_eq!(got, want, "bn {bn}");
        }
        // Past the triple indirect block: EFBIG.
        assert_eq!(path(n, 96), Err(Errno::EFBIG));
    }

    #[test]
    fn getlbns_with_ffs_sizes() {
        // An FFS2 file system of 16 KiB blocks: 2048 pointers per block.
        let n = 2048;
        assert_eq!(path(n, 11).unwrap(), (0, std::vec![]));
        assert_eq!(path(n, 12).unwrap(), (2, std::vec![(-12, 0), (-12, 0)]));
        assert_eq!(
            path(n, 12 + 2048).unwrap(),
            (3, std::vec![(-2061, 1), (-2061, 0), (-2060, 0)])
        );
        assert_eq!(
            path(n, 12 + 2048 + 2048 * 2048).unwrap(),
            (
                4,
                std::vec![
                    (-4_196_366, 2),
                    (-4_196_366, 0),
                    (-4_196_365, 0),
                    (-4_196_364, 0)
                ]
            )
        );
    }
}
/* </TESTS> */
