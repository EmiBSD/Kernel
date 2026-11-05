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
 * Copyright (c) 2002 Networks Associates Technology, Inc.
 * All rights reserved.
 *
 * This software was developed for the FreeBSD Project by Marshall
 * Kirk McKusick and Network Associates Laboratories, the Security
 * Research Division of Network Associates, Inc. under DARPA/SPAWAR
 * contract N66001-01-C-8035 ("CBOSS"), as part of the DARPA CHATS
 * research program.
 *
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
 */
/* </LICENSES> */

/* <CODE> */
//! Block allocation: `balloc` defines the structure of file system storage by allocating
//! the physical blocks on a device given the inode and the logical block number in a file,
//! growing the last fragment into a block and building the indirect blocks on the way.
//!
//! Upstream: sys/ufs/ffs/ffs_balloc.c @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/ffs_balloc.rs@3c62ede68802
//!
//! ## Deviations
//! - The buffer the C returns in `*bpp` is the result: every caller asks for it
//!   (`UFS_BUF_ALLOC` always passes a `bpp`: ufs_lookup.c, ufs_vnops.c, ffs_inode.c,
//!   ffs_vnops.c), so the C's `bpp == NULL` branches have no caller and the block is always
//!   read or made. The buffer `ffs_realloccg` grew for the fragment extended into a block is
//!   written at once, as the C writes `*bpp` (and then overwrites it).
//! - `allocib` (a pointer into the dinode's `di_ib`) is the index it points at; `allociblk`
//!   is an array and a count; the block pointers in an indirect block are read and written at
//!   their index in the buffer's bytes (`inode.rs`).
//! - `VOP_FSYNC(vp, p->p_ucred, MNT_WAIT, p)` of the failure path is skipped when no thread
//!   runs (there is no `p` to pass).
//! - The quota calls are `quota.rs`'s: `ufs_quota.rs`'s with feature `quota` (`option QUOTA`),
//!   the no-quota answers of `ufs_quota_stub.c` without it.
//!
//! ## Redesign
//! - N5.5b1: LZ's buffers lent by address (`&'static Buf`, their data reached through an
//!   `unsafe` `bdata` at nine sites) -> the busy buffers' tokens of the kernel's cache
//!   (`bread`/`getblk` return a `BufRef`, `bwrite`/`bdwrite`/`bawrite`/`brelse` consume it):
//!   each indirect block's pointers are read and written in its token's bytes, and the data
//!   block is the result, so the C's `*bpp` cannot be left stale. The levels of indirection
//!   come from `getlbns` over the inode's `MNINDIR` (the C's `ufs_getlbns(vp, ..)` reaches
//!   the same inode through `VTOI`, a count pair here). Over the in-core `Superblock`.
//! - N5.7c1: the module's private `bread_or_release` is `fs::bio::bread_or_release`, shared
//!   with msdosfs's FAT engine (a pure move: the same body, the same calls).

use hal::sys::errno::Errno;
use hal::sys::mount::MNT_WAIT;
use hal::sys::types::{Daddr, Off};

#[cfg(feature = "ffs2")]
use super::ffs_alloc::ffs2_blkpref;
use super::ffs_alloc::{ffs_alloc, ffs_blkfree, ffs_realloccg, ffs1_blkpref};
#[cfg(feature = "ffs2")]
use super::fs::{FS_UFS2_MAGIC, cgtod, dbtofsb, dtog};
use super::fs::{blkoff, blksize, fragroundup, fsbtodb, lblkno, lblktosize};
use crate::assert::kpanic;
use crate::fs::bio::getblk_wait;
use crate::fs::bio::{
    bawrite, bdwrite, bread, bread_or_release, brelse, btodb, buf_adjcnt, bwrite,
};
use crate::fs::buf::{B_CLRBUF, B_SYNC, BufRef};
#[cfg(feature = "ffs2")]
use crate::fs::buf::{B_DELWRI, B_INVAL};
use crate::fs::ufs::ufs::dinode::{NDADDR, NIADDR};
use crate::fs::ufs::ufs::inode::{IN_CHANGE, IN_UPDATE, Indir, Inode, daddr32_at, set_daddr32_at};
#[cfg(feature = "ffs2")]
use crate::fs::ufs::ufs::inode::{daddr64_at, set_daddr64_at};
use crate::fs::ufs::ufs::quota::ufs_quota_free_blocks;
use crate::fs::ufs::ufs::ufs_bmap::getlbns;
use crate::fs::vnode::Vnode;
use crate::fs::vops::{Cred, vop_fsync};
use crate::proc::ProcGlue;
use crate::vm::vnode::uvm_vnp_setsize;

/// `VOP_FSYNC(vp, p->p_ucred, MNT_WAIT, p)` with `curproc`.
fn fsync_wait<G: ProcGlue>(vp: &'static Vnode<G>) {
    G::with_curproc(|p| {
        if let Some(p) = p {
            let _ = vop_fsync(vp, Cred::real(p.ucred()), MNT_WAIT, p);
        }
    });
}

/// `bwrite` with `B_SYNC` in `flags`, else `bdwrite`: "if required, write synchronously,
/// otherwise use delayed write".
fn write_or_delay(t: BufRef, flags: i32) {
    if flags & B_SYNC != 0 {
        let _ = bwrite(t);
    } else {
        bdwrite(t);
    }
}

/// `ffs1_blkpref` or `ffs2_blkpref`: the format's block preference.
type Blkpref<G> = fn(&Inode<G>, Daddr, i32, Option<&dyn Fn(usize) -> Daddr>) -> Daddr;

/// The direct-block half of `ffs1_balloc`/`ffs2_balloc` (the file's first `NDADDR`
/// blocks), the two's code being the same but for their `blkpref`: the fragment the file
/// ends in grown into a block when the write goes past it, then block `lbn` read, grown or
/// allocated.
fn balloc_direct<G: ProcGlue>(
    ip: &Inode<G>,
    lbn: Daddr,
    size: i32,
    cred: Cred<'_>,
    flags: i32,
    blkpref: Blkpref<G>,
) -> Option<Result<BufRef, Errno>> {
    let vp = ip.itov();
    let fs = ip.i_fs();
    let bsize = fs.fs_bsize();
    let db = |i: usize| ip.dip_db(i);

    // If the next write will extend the file into a new block, and the file is currently
    // composed of a fragment this fragment has to be extended to be a full block.
    let nb = lblkno(fs, ip.dip_size() as Off);
    if nb < NDADDR as Daddr && nb < lbn {
        let osize = blksize(fs, ip, nb) as i32;
        if osize < bsize && osize > 0 {
            let pref = blkpref(ip, nb, nb as i32, Some(&db));
            let (t, newb) = match ffs_realloccg(ip, nb, pref, osize, bsize, cred) {
                Ok(r) => r,
                Err(e) => return Some(Err(e)),
            };

            ip.dip_set_size(lblktosize(fs, nb + 1) as u64);
            uvm_vnp_setsize(vp, ip.dip_size() as i64);
            ip.dip_set_db(nb as usize, newb);
            ip.set_flag(IN_CHANGE | IN_UPDATE);
            if flags & B_SYNC != 0 {
                let _ = bwrite(t);
            } else {
                bawrite(t);
            }
        }
    }
    // The first NDADDR blocks are direct blocks
    if lbn >= NDADDR as Daddr {
        return None;
    }
    let nb = ip.dip_db(lbn as usize);
    if nb != 0 && ip.dip_size() as Off >= lblktosize(fs, lbn + 1) {
        // The block is an already-allocated direct block and the file already extends past
        // this block, thus this must be a whole block. Just read the block.
        return Some(bread_or_release(&vp.bufs, lbn, bsize));
    }
    let (t, newb) = if nb != 0 {
        // Consider need to reallocate a fragment.
        let osize = fragroundup(fs, blkoff(fs, ip.dip_size() as Off)) as i32;
        let nsize = fragroundup(fs, i64::from(size)) as i32;
        if nsize <= osize {
            // The existing block is already at least as big as we want. Just read the
            // block.
            return Some(
                bread_or_release(&vp.bufs, lbn, bsize).inspect(|t| buf_adjcnt(t, i64::from(osize))),
            );
        }
        // The existing block is smaller than we want, grow it.
        let pref = blkpref(ip, lbn, lbn as i32, Some(&db));
        match ffs_realloccg(ip, lbn, pref, osize, nsize, cred) {
            Ok(r) => r,
            Err(e) => return Some(Err(e)),
        }
    } else {
        // The block was not previously allocated, allocate a new block or fragment.
        let nsize = if (ip.dip_size() as Off) < lblktosize(fs, lbn + 1) {
            fragroundup(fs, i64::from(size)) as i32
        } else {
            bsize
        };
        let pref = blkpref(ip, lbn, lbn as i32, Some(&db));
        let newb = match ffs_alloc(ip, lbn, pref, nsize, cred) {
            Ok(b) => b,
            Err(e) => return Some(Err(e)),
        };
        let mut t = getblk_wait(&vp.bufs, lbn, bsize);
        if nsize < bsize {
            t.buf().set_bcount(i64::from(nsize));
        }
        t.buf().set_blkno(fsbtodb(fs, newb));
        if flags & B_CLRBUF != 0 {
            t.clear();
        }
        (t, newb)
    };
    ip.dip_set_db(lbn as usize, newb);
    ip.set_flag(IN_CHANGE | IN_UPDATE);
    Some(Ok(t))
}

/// `ffs1_balloc`: `ffs_balloc` for an FFS1 inode.
pub fn ffs1_balloc<G: ProcGlue>(
    ip: &Inode<G>,
    startoffset: Off,
    size: i32,
    cred: Cred<'_>,
    flags: i32,
) -> Result<BufRef, Errno> {
    let vp = ip.itov();
    let fs = ip.i_fs();
    let bsize = fs.fs_bsize();
    let lbn = lblkno(fs, startoffset);
    let size = blkoff(fs, startoffset) as i32 + size;
    if size > bsize {
        kpanic(format_args!("ffs1_balloc: blk too big"));
    }
    if lbn < 0 {
        return Err(Errno::EFBIG);
    }
    if let Some(r) = balloc_direct(ip, lbn, size, cred, flags, ffs1_blkpref::<G>) {
        return r;
    }

    // Determine the number of levels of indirection.
    let mut pref: Daddr = 0;
    let mut indirs = [Indir::default(); NIADDR + 2];
    let mut num = 0;
    getlbns(ip.nindir() as i64, lbn, &mut indirs, Some(&mut num))?;
    #[cfg(feature = "diagnostic")]
    if num < 1 {
        kpanic(format_args!(
            "ffs1_balloc: ufs_bmaparray returned indirect block"
        ));
    }
    // Fetch the first indirect block allocating if necessary.
    num -= 1;
    let mut nb = ip.dip_ib(indirs[0].in_off as usize);

    let mut allocib: Option<usize> = None;
    let mut allociblk = [0 as Daddr; NIADDR + 1];
    let mut nalloc = 0;
    let mut unwindidx: i32 = -1;

    let error: Errno = 'fail: {
        if nb == 0 {
            pref = ffs1_blkpref(ip, lbn, -indirs[0].in_off - 1, None);
            nb = match ffs_alloc(ip, lbn, pref, bsize, cred) {
                Ok(b) => b,
                Err(e) => break 'fail e,
            };

            allociblk[nalloc] = nb;
            nalloc += 1;
            let mut t = getblk_wait(&vp.bufs, indirs[1].in_lbn, bsize);
            t.buf().set_blkno(fsbtodb(fs, nb));
            t.clear();

            // Write synchronously so that indirect blocks never point at garbage.
            if let Err(e) = bwrite(t) {
                break 'fail e;
            }
            allocib = Some(indirs[0].in_off as usize);
            ip.dip_set_ib(indirs[0].in_off as usize, nb);
            ip.set_flag(IN_CHANGE | IN_UPDATE);
        }

        // Fetch through the indirect blocks, allocating as necessary.
        let mut i = 1usize;
        let mut bp = loop {
            let mut bp = match bread_or_release(&vp.bufs, indirs[i].in_lbn, bsize) {
                Ok(t) => t,
                Err(e) => break 'fail e,
            };
            nb = daddr32_at(bp.data(), indirs[i].in_off as usize);
            if i as i32 == num {
                break bp;
            }
            i += 1;
            if nb != 0 {
                brelse(bp);
                continue;
            }
            if pref == 0 {
                pref = ffs1_blkpref(ip, lbn, i as i32 - num - 1, None);
            }
            nb = match ffs_alloc(ip, lbn, pref, bsize, cred) {
                Ok(b) => b,
                Err(e) => {
                    brelse(bp);
                    break 'fail e;
                }
            };
            allociblk[nalloc] = nb;
            nalloc += 1;
            let mut nbp = getblk_wait(&vp.bufs, indirs[i].in_lbn, bsize);
            nbp.buf().set_blkno(fsbtodb(fs, nb));
            nbp.clear();

            // Write synchronously so that indirect blocks never point at garbage.
            if let Err(e) = bwrite(nbp) {
                brelse(bp);
                break 'fail e;
            }
            set_daddr32_at(bp.data(), indirs[i - 1].in_off as usize, nb);
            if allocib.is_none() && unwindidx < 0 {
                unwindidx = i as i32 - 1;
            }
            // If required, write synchronously, otherwise use delayed write.
            write_or_delay(bp, flags);
        };
        // Get the data block, allocating if necessary.
        if nb == 0 {
            {
                let data = bp.data();
                let bap = |k: usize| daddr32_at(data, k);
                pref = ffs1_blkpref(ip, lbn, indirs[i].in_off, Some(&bap));
            }
            nb = match ffs_alloc(ip, lbn, pref, bsize, cred) {
                Ok(b) => b,
                Err(e) => {
                    brelse(bp);
                    break 'fail e;
                }
            };
            // *allocblk++ = nb: nothing reads it on the way out.
            let mut nbp = getblk_wait(&vp.bufs, lbn, bsize);
            nbp.buf().set_blkno(fsbtodb(fs, nb));
            if flags & B_CLRBUF != 0 {
                nbp.clear();
            }
            set_daddr32_at(bp.data(), indirs[i].in_off as usize, nb);
            // If required, write synchronously, otherwise use delayed write.
            write_or_delay(bp, flags);
            return Ok(nbp);
        }
        brelse(bp);
        let nbp = if flags & B_CLRBUF != 0 {
            match bread_or_release(&vp.bufs, lbn, bsize) {
                Ok(t) => t,
                Err(e) => break 'fail e,
            }
        } else {
            let t = getblk_wait(&vp.bufs, lbn, bsize);
            t.buf().set_blkno(fsbtodb(fs, nb));
            t
        };
        return Ok(nbp);
    };

    // fail:
    // If we have failed to allocate any blocks, simply return the error. This is the usual
    // case and avoids the need to fsync the file.
    if nalloc == 0 && allocib.is_none() && unwindidx == -1 {
        return Err(error);
    }
    // If we have failed part way through block allocation, we have to deallocate any
    // indirect blocks that we have allocated. We have to fsync the file before we start to
    // get rid of all of its dependencies so that we do not leave them dangling. We have to
    // sync it at the end so that the softdep code does not find any untracked changes.
    // Although this is really slow, running out of disk space is not expected to be a
    // common occurrence. The error return from fsync is ignored as we already have an error
    // to return to the user.
    fsync_wait(vp);
    let mut deallocated: i64 = 0;
    for &blk in &allociblk[..nalloc] {
        ffs_blkfree(ip, blk, i64::from(bsize));
        deallocated += i64::from(bsize);
    }
    if let Some(k) = allocib {
        ip.dip_set_ib(k, 0);
    } else if unwindidx >= 0 {
        let (mut bp, r) = bread(&vp.bufs, indirs[unwindidx as usize].in_lbn, bsize);
        if let Err(r) = r {
            kpanic(format_args!(
                "Could not unwind indirect block, error {}",
                r as i32
            ));
        }
        set_daddr32_at(bp.data(), indirs[unwindidx as usize].in_off as usize, 0);
        write_or_delay(bp, flags);
    }
    if deallocated != 0 {
        // Restore user's disk quota because allocation failed.
        let _ = ufs_quota_free_blocks(ip, btodb(deallocated), cred);

        ip.dip_set_blocks(ip.dip_blocks() - btodb(deallocated));
        ip.set_flag(IN_CHANGE | IN_UPDATE);
    }
    fsync_wait(vp);
    Err(error)
}

/// `ffs2_balloc`: `ffs_balloc` for an FFS2 inode.
#[cfg(feature = "ffs2")]
pub fn ffs2_balloc<G: ProcGlue>(
    ip: &Inode<G>,
    off: Off,
    size: i32,
    cred: Cred<'_>,
    flags: i32,
) -> Result<BufRef, Errno> {
    let vp = ip.itov();
    let fs = ip.i_fs();
    let bsize = fs.fs_bsize();
    let mut unwindidx: i32 = -1;

    let lbn = lblkno(fs, off);
    let size = blkoff(fs, off) as i32 + size;

    if size > bsize {
        kpanic(format_args!("ffs2_balloc: block too big"));
    }

    if lbn < 0 {
        return Err(Errno::EFBIG);
    }

    // If the next write will extend the file into a new block, and the file is currently
    // composed of a fragment, this fragment has to be extended to be a full block; the
    // first NDADDR blocks are direct.
    if let Some(r) = balloc_direct(ip, lbn, size, cred, flags, ffs2_blkpref::<G>) {
        return r;
    }

    // Determine the number of levels of indirection.
    let mut pref: Daddr = 0;
    let mut indirs = [Indir::default(); NIADDR + 2];
    let mut num = 0;
    getlbns(ip.nindir() as i64, lbn, &mut indirs, Some(&mut num))?;

    #[cfg(feature = "diagnostic")]
    if num < 1 {
        kpanic(format_args!(
            "ffs2_balloc: ufs_bmaparray returned indirect block"
        ));
    }

    // Fetch the first indirect block allocating it necessary.
    num -= 1;
    let mut nb = ip.dip_ib(indirs[0].in_off as usize);
    let mut allocib: Option<usize> = None;
    let mut allociblk = [0 as Daddr; NIADDR + 1];
    let mut nalloc = 0;

    let error: Errno = 'fail: {
        if nb == 0 {
            pref = ffs2_blkpref(ip, lbn, -indirs[0].in_off - 1, None);
            nb = match ffs_alloc(ip, lbn, pref, bsize, cred) {
                Ok(b) => b,
                Err(e) => break 'fail e,
            };

            allociblk[nalloc] = nb;
            nalloc += 1;
            let mut t = getblk_wait(&vp.bufs, indirs[1].in_lbn, bsize);
            t.buf().set_blkno(fsbtodb(fs, nb));
            t.clear();

            // Write synchronously so that indirect blocks never point at garbage.
            if let Err(e) = bwrite(t) {
                break 'fail e;
            }

            unwindidx = 0;
            allocib = Some(indirs[0].in_off as usize);
            ip.dip_set_ib(indirs[0].in_off as usize, nb);
            ip.set_flag(IN_CHANGE | IN_UPDATE);
        }

        // Fetch through the indirect blocks, allocating as necessary.
        let mut i = 1usize;
        let mut bp = loop {
            let mut bp = match bread_or_release(&vp.bufs, indirs[i].in_lbn, bsize) {
                Ok(t) => t,
                Err(e) => break 'fail e,
            };

            nb = daddr64_at(bp.data(), indirs[i].in_off as usize);

            if i as i32 == num {
                break bp;
            }

            i += 1;

            if nb != 0 {
                brelse(bp);
                continue;
            }

            if pref == 0 {
                pref = ffs2_blkpref(ip, lbn, i as i32 - num - 1, None);
            }

            nb = match ffs_alloc(ip, lbn, pref, bsize, cred) {
                Ok(b) => b,
                Err(e) => {
                    brelse(bp);
                    break 'fail e;
                }
            };
            allociblk[nalloc] = nb;
            nalloc += 1;
            let mut nbp = getblk_wait(&vp.bufs, indirs[i].in_lbn, bsize);
            nbp.buf().set_blkno(fsbtodb(fs, nb));
            nbp.clear();

            // Write synchronously so that indirect blocks never point at garbage.
            if let Err(e) = bwrite(nbp) {
                brelse(bp);
                break 'fail e;
            }

            if unwindidx < 0 {
                unwindidx = i as i32 - 1;
            }

            set_daddr64_at(bp.data(), indirs[i - 1].in_off as usize, nb);

            // If required, write synchronously, otherwise use delayed write.
            write_or_delay(bp, flags);
        };

        // Get the data block, allocating if necessary.
        if nb == 0 {
            {
                let data = bp.data();
                let bap = |k: usize| daddr64_at(data, k);
                pref = ffs2_blkpref(ip, lbn, indirs[num as usize].in_off, Some(&bap));
            }

            nb = match ffs_alloc(ip, lbn, pref, bsize, cred) {
                Ok(b) => b,
                Err(e) => {
                    brelse(bp);
                    break 'fail e;
                }
            };
            // *allocblk++ = nb: nothing reads it on the way out.

            let mut nbp = getblk_wait(&vp.bufs, lbn, bsize);
            nbp.buf().set_blkno(fsbtodb(fs, nb));
            if flags & B_CLRBUF != 0 {
                nbp.clear();
            }

            set_daddr64_at(bp.data(), indirs[num as usize].in_off as usize, nb);

            // if (allocib == NULL && unwindidx < 0) unwindidx = i - 1: nothing reads it on
            // the way out.

            // If required, write synchronously, otherwise use delayed write.
            write_or_delay(bp, flags);

            return Ok(nbp);
        }

        brelse(bp);

        let nbp = if flags & B_CLRBUF != 0 {
            match bread_or_release(&vp.bufs, lbn, bsize) {
                Ok(t) => t,
                Err(e) => break 'fail e,
            }
        } else {
            let mut t = getblk_wait(&vp.bufs, lbn, bsize);
            t.buf().set_blkno(fsbtodb(fs, nb));
            t.clear();
            t
        };

        return Ok(nbp);
    };

    // fail:
    // If we have failed to allocate any blocks, simply return the error. This is the usual
    // case and avoids the need to fsync the file.
    if nalloc == 0 && allocib.is_none() && unwindidx == -1 {
        return Err(error);
    }
    // If we have failed part way through block allocation, we have to deallocate any
    // indirect blocks that we have allocated. We have to fsync the file before we start to
    // get rid of all of its dependencies so that we do not leave them dangling. We have to
    // sync it at the end so that the softdep code does not find any untracked changes.
    // Although this is really slow, running out of disk space is not expected to be a
    // common occurrence. The error return from fsync is ignored as we already have an error
    // to return to the user.
    fsync_wait(vp);
    if unwindidx >= 0 {
        // First write out any buffers we've created to resolve their softdeps. This must be
        // done in reverse order of creation so that we resolve the dependencies in one pass.
        // Write the cylinder group buffers for these buffers too.
        let mut i = num;
        while i >= unwindidx {
            if i == 0 {
                break;
            }

            let bp = getblk_wait(&vp.bufs, indirs[i as usize].in_lbn, bsize);
            if bp.buf().isset(B_DELWRI) {
                let nb = fsbtodb(fs, cgtod(fs, dtog(fs, dbtofsb(fs, bp.buf().blkno()))));
                let _ = bwrite(bp);
                let bp = getblk_wait(&ip.i_devvp().bufs, nb, fs.fs_cgsize());
                if bp.buf().isset(B_DELWRI) {
                    let _ = bwrite(bp);
                } else {
                    bp.buf().set(B_INVAL);
                    brelse(bp);
                }
            } else {
                bp.buf().set(B_INVAL);
                brelse(bp);
            }
            i -= 1;
        }

        // Now that any dependencies that we created have been resolved, we can undo the
        // partial allocation.
        if unwindidx == 0 {
            if let Some(k) = allocib {
                ip.dip_set_ib(k, 0);
            }
            ip.set_flag(IN_CHANGE | IN_UPDATE);
        } else {
            let (mut bp, r) = bread(&vp.bufs, indirs[unwindidx as usize].in_lbn, bsize);
            if r.is_err() {
                kpanic(format_args!("ffs2_balloc: unwind failed"));
            }

            set_daddr64_at(bp.data(), indirs[unwindidx as usize].in_off as usize, 0);
            let _ = bwrite(bp);
        }

        for i in (unwindidx + 1)..=num {
            let bp = getblk_wait(&vp.bufs, indirs[i as usize].in_lbn, bsize);
            bp.buf().set(B_INVAL);
            brelse(bp);
        }
    }

    let mut deallocated: i64 = 0;
    for &blk in &allociblk[..nalloc] {
        ffs_blkfree(ip, blk, i64::from(bsize));
        deallocated += i64::from(bsize);
    }

    if deallocated != 0 {
        // Restore user's disk quota because allocation failed.
        let _ = ufs_quota_free_blocks(ip, btodb(deallocated), cred);

        ip.dip_set_blocks(ip.dip_blocks() - btodb(deallocated));
        ip.set_flag(IN_CHANGE | IN_UPDATE);
    }
    fsync_wait(vp);
    Err(error)
}

/// `ffs_balloc` (`iv_buf_alloc`): balloc defines the structure of file system storage by
/// allocating the physical blocks given the inode and the logical block number in a file;
/// the block's buffer.
pub fn ffs_balloc<G: ProcGlue>(
    ip: &Inode<G>,
    off: Off,
    size: i32,
    cred: Cred<'_>,
    flags: i32,
) -> Result<BufRef, Errno> {
    #[cfg(feature = "ffs2")]
    if ip.i_fs().fs_magic() == FS_UFS2_MAGIC {
        return ffs2_balloc(ip, off, size, cred, flags);
    }
    ffs1_balloc(ip, off, size, cred, flags)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::super::fs::{Superblock, cgdmin, dbtofsb};
    use super::super::testffs::newfs::{FFS1_1G, FFS2_1G, Image, Params, check_with, super_block};
    use super::super::testffs::{disk, new_file, setup, sync};
    use super::*;
    use crate::cred::Ucred;
    use crate::fs::subr::vput;
    use crate::fs::ufs::ufs::inode::vtoi;

    /// The formats the tests build: FFS2's only with `option FFS2`.
    const FORMATS: &[Params] = if cfg!(feature = "ffs2") {
        &[FFS2_1G, FFS1_1G]
    } else {
        &[FFS1_1G]
    };

    /// `fsck`'s check of the synced image against the in-core summaries and totals.
    fn check_incore(
        fs: &Superblock,
        ufs2: bool,
        vps: &[&'static Vnode<crate::fs::testfs::TestGlue>],
    ) {
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

    /// `ffs_balloc` as `ffs_write` calls it (`B_SYNC`: every metadata write synchronous, so
    /// no delayed write is left on the file), the size then set as the write would, the
    /// block written.
    fn write<G: ProcGlue>(ip: &Inode<G>, off: Off, size: i32, cred: Cred<'_>) -> Daddr {
        let t = ffs_balloc(ip, off, size, cred, B_CLRBUF | B_SYNC).unwrap();
        let blkno = t.buf().blkno();
        let end = (off + Off::from(size)) as u64;
        if end > ip.dip_size() {
            ip.dip_set_size(end);
        }
        bwrite(t).unwrap();
        blkno
    }

    #[test]
    fn a_file_grows_from_a_fragment_to_blocks_and_an_indirect_block() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let cred = Cred::real(th.ucred());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let fs = ip.i_fs();
            let (bsize, fsize) = (fs.fs_bsize(), fs.fs_fsize());
            let frag = Daddr::from(fs.fs_frag());
            let dmin = cgdmin(fs, 0);

            // A first write of 100 bytes: one fragment, from the run after the summary's.
            write(&ip, 0, 100, cred);
            assert_eq!(ip.dip_db(0), dmin + 1, "{p:?}");
            assert_eq!(ip.dip_blocks(), btodb(i64::from(fsize)));
            // 2,000 more: the fragment extended in place.
            write(&ip, 100, 2000, cred);
            assert_eq!(ip.dip_db(0), dmin + 1, "{p:?}");
            let frags = (2100 + i64::from(fsize) - 1) / i64::from(fsize);
            assert_eq!(ip.dip_blocks(), btodb(frags * i64::from(fsize)));
            // Into the second block: the first grown into a whole block, moved to the first
            // whole free block (it cannot grow across the block's end), and the second a
            // fragment.
            write(&ip, Off::from(bsize), 10, cred);
            assert_eq!(ip.dip_db(0), dmin + 2 * frag, "{p:?}");
            assert_eq!(ip.dip_size(), bsize as u64 + 10);
            assert!(ip.dip_db(1) != 0);
            assert_eq!(
                ip.dip_blocks(),
                btodb(i64::from(bsize)) + btodb(i64::from(fsize))
            );
            // An existing whole block is read back as it is.
            let t = ffs_balloc(&ip, 0, 10, cred, B_CLRBUF).unwrap();
            assert_eq!(t.buf().bcount(), i64::from(bsize));
            assert_eq!(dbtofsb(fs, t.buf().blkno()), dmin + 2 * frag);
            brelse(t);
            check_incore(fs, p.ufs2, &[vp]);

            // The first block past the direct ones: the second grown whole, the single
            // indirect block allocated and written, the data block under it.
            let lbn = NDADDR as Daddr;
            let data = write(&ip, Off::from(bsize) * lbn, bsize, cred);
            let ib = ip.dip_ib(0);
            assert!(ib != 0);
            assert_eq!(ip.dip_blocks(), 4 * btodb(i64::from(bsize)));
            let mut indirs = [Indir::default(); NIADDR + 2];
            let mut num = 0;
            getlbns(ip.nindir() as i64, lbn, &mut indirs, Some(&mut num)).unwrap();
            let (mut t, r) = bread(&vp.bufs, indirs[1].in_lbn, bsize);
            r.unwrap();
            assert_eq!(dbtofsb(fs, t.buf().blkno()), ib);
            let ptr = if p.ufs2 {
                crate::fs::ufs::ufs::inode::daddr64_at(t.data(), indirs[1].in_off as usize)
            } else {
                daddr32_at(t.data(), indirs[1].in_off as usize)
            };
            brelse(t);
            assert_eq!(ptr, dbtofsb(fs, data));
            check_incore(fs, p.ufs2, &[vp]);
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn a_failed_data_block_gives_the_new_indirect_block_back() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let fs = ip.i_fs();
            let bsize = fs.fs_bsize();
            let nb = fs.fs_cstotal_nbfree();
            // A user may take one more block, no more: freespace() is one block's
            // fragments with all of fs_dsize but those in reserve.
            let (minfree, dsize) = (fs.fs_minfree(), fs.fs_dsize());
            fs.set_fs_minfree(100);
            fs.set_fs_dsize(
                fs.fs_cstotal_nbfree() * i64::from(fs.fs_frag()) + fs.fs_cstotal_nffree()
                    - i64::from(fs.fs_frag()),
            );
            let mut user = Ucred::new();
            user.cr_uid.set(1000);
            let off = Off::from(bsize) * NDADDR as Off;
            assert_eq!(
                ffs_balloc(&ip, off, bsize, Cred::real(&user), B_CLRBUF).map(|_| ()),
                Err(Errno::ENOSPC)
            );
            // The indirect block it had taken is free again and unlinked.
            assert_eq!(ip.dip_ib(0), 0);
            assert_eq!(ip.dip_blocks(), 0);
            assert_eq!(fs.fs_cstotal_nbfree(), nb);
            fs.set_fs_minfree(minfree);
            fs.set_fs_dsize(dsize);
            check_incore(fs, p.ufs2, &[vp]);
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn a_negative_offset_is_efbig() {
        let (_g, th, mp) = setup(Image::new(FORMATS[0]).finish());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        assert_eq!(
            ffs_balloc(&ip, -8192, 1, Cred::real(th.ucred()), 0).map(|_| ()),
            Err(Errno::EFBIG)
        );
        drop(ip);
        vput(vp);
    }
}
/* </TESTS> */
