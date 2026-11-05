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
 *	@(#)ffs_alloc.c	8.11 (Berkeley) 10/27/94
 */
/* </LICENSES> */

/* <CODE> */
//! The fast file system's allocator: blocks and fragments (`ffs_alloc`, `ffs_realloccg`,
//! `ffs_blkfree`), inodes (`ffs_inode_alloc`, `ffs_freefile`), the placement policies
//! (`ffs_dirpref`, `ffs1_blkpref`, `ffs2_blkpref`) and the cylinder group searches under
//! them (`ffs_hashalloc`, `ffs_alloccg`, `ffs_alloccgblk`, `ffs_nodealloccg`,
//! `ffs_mapsearch`, `ffs_clusteracct`).
//!
//! Upstream: sys/ufs/ffs/ffs_alloc.c @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/ffs_alloc.rs@3c62ede68802
//!
//! ## Deviations
//! - `ffs1_blkpref`/`ffs2_blkpref`'s `bap` (the block pointer array the C passes, either the
//!   dinode's `di_db` or an indirect block's data) is a function that reads entry `i`, or
//!   `None` for the C's NULL.
//! - `ffs_hashalloc`'s allocator is a `fn` pointer of the C's type.
//! - `ffs_alloc`'s `*bnp` is its result; `ffs_realloccg` returns the grown buffer and the new
//!   block (the C's `*bpp` and `*blknop`): its one caller, `ffs_balloc`, always asks for both
//!   (every `UFS_BUF_ALLOC` passes a `bpp`), so the C's `bpp == NULL` branches have no caller.
//! - The C's `DIAGNOSTIC` "missing credential" check is made always: the C would follow
//!   `NOCRED` (`(struct ucred *)-1`) at the next line. The kernel's check panics on
//!   `FSCRED` and a NULL credential too, which the C would follow the same way.
//! - `uprintf`'s message goes to the current thread's process (`curproc->p_p`), and nowhere
//!   when no thread runs (the C would follow a NULL `curproc`).
//! - The quota calls are `quota.rs`'s: `ufs_quota.rs`'s with feature `quota` (`option QUOTA`),
//!   the no-quota answers of `ufs_quota_stub.c` without it.
//!
//! ## Redesign
//! - N5.5b1: LZ's functions over `&'static Fs` and a cylinder group cast out of a lent buffer
//!   (`CgBuf::new`, `unsafe` at each of its eight call sites) -> over the in-core
//!   `Superblock` (`ip.i_fs()`) and a [`CgView`] over the busy buffer's token
//!   (`ffs_cgread` returns the `BufRef` `bread` gave, the view borrows its bytes until the
//!   `bdwrite`/`brelse` that consumes it): 0 `unsafe`, the same reads and writes of the
//!   same bytes, a bounds check per map access.
//! - LZ's three `static StaticCell<Timeval>` rate limits written through `unsafe`
//!   `get_mut` -> `Mutexed<Timespec>` statics read by `kernel::fork::ratecheck`: the one lock
//!   section is the C's `ratecheck_mtx` section (`kern_time.c`), on the error message's path
//!   only. `uprintf` is the kernel's (`tty.rs`), which closes LZ's visible `unported!` gap:
//!   "write failed, file system is full" and "create/symlink failed, no inodes free" reach
//!   the process's terminal as the C's do.
//! - LZ's `crate::ufs::ffs::fs::add`/`sub` over `Cell`s -> the counters' own `add_*`
//!   accessors (`fs_cstotal`, `fs_cs(fs, cg)`, the cylinder group's `cg_cs`, `cg_frsum`).

use core::sync::atomic::Ordering;

use hal::intr::IPL_NONE;
use hal::mutex::Mutexed;
#[cfg(feature = "ffs2")]
use hal::random::arc4random;
use hal::random::arc4random_uniform;
use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Ino, Mode, Uid};

use super::ffs_subr::{ffs_clrblock, ffs_fragacct, ffs_isblock, ffs_isfreeblock, ffs_setblock};
use super::fs::{
    AROUND, Cg, CgView, FRAGTBL, FS_OPTSPACE, FS_OPTTIME, FS_UFS2_MAGIC, INSIDE, Superblock,
    blkmap, blknum, cbtocylno, cbtorpos, cgbase, cgdata, cgmeta, cgtod, clrbit, dtog, dtogd,
    fragnum, fragoff, fragstoblks, freespace, fsbtodb, ino_to_cg, isclr, isset, nindir, numfrags,
    setbit,
};
#[cfg(feature = "ffs2")]
use super::fs::{ino_to_fsba, inopb};
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::fork::ratecheck;
#[cfg(feature = "ffs2")]
use crate::fs::bio::{bawrite, getblk};
use crate::fs::bio::{bdwrite, bread, brelse, btodb, buf_adjcnt};
use crate::fs::buf::{B_DONE, BufRef};
use crate::fs::ufs::ufs::dinode::{IFDIR, IFMT, NDADDR, Ufsino};
use crate::fs::ufs::ufs::inode::{IN_CHANGE, IN_UPDATE, Inode, vtoi};
use crate::fs::ufs::ufs::quota::{ufs_quota_alloc_blocks, ufs_quota_free_blocks};
use crate::fs::vnode::{VROOT, Vnode};
use crate::fs::vops::{Cred, VFS_VGET};
use crate::log::LOG_ERR;
use crate::prf::{log, printf};
use crate::proc::ProcGlue;
use crate::vm::vnode::uvm_vnp_uncache;
use libkern::{scanc, skpc};

/// `fserr_interval`: at most one "file system full" message every 2 seconds.
const FSERR_INTERVAL: Timespec = Timespec::new(2, 0);

/// `ffs_alloc`'s `fsfull_last`.
static FFS_ALLOC_FSFULL_LAST: Mutexed<Timespec> = Mutexed::new(IPL_NONE, Timespec::ZERO);
/// `ffs_realloccg`'s `fsfull_last`.
static FFS_REALLOCCG_FSFULL_LAST: Mutexed<Timespec> = Mutexed::new(IPL_NONE, Timespec::ZERO);
/// `ffs_inode_alloc`'s `fsnoinodes_last`.
static FFS_INODE_ALLOC_FSNOINODES_LAST: Mutexed<Timespec> = Mutexed::new(IPL_NONE, Timespec::ZERO);

/// `ratecheck(&last, &fserr_interval)` on one of the statics above (one lock section, the
/// C's `ratecheck_mtx`).
fn fserr_ratecheck(last: &Mutexed<Timespec>) -> bool {
    ratecheck(&mut last.lock(), &FSERR_INTERVAL)
}

/// `ffs_fserr(fs, uid, cp)`: report a file system error.
fn ffs_fserr(fs: &Superblock, uid: Uid, cp: &str) {
    log(
        LOG_ERR,
        format_args!("uid {} on {}: {}\n", uid, fs.fsmnt(), cp),
    );
}

/// `uprintf("\n%s: <what>\n", fs->fs_fsmnt)` on the current process's terminal.
fn uprintf_fs<G: ProcGlue>(fs: &Superblock, what: &str) {
    G::with_curproc(|p| {
        if let Some(p) = p {
            crate::tty::uprintf(p.process(), format_args!("\n{}: {}\n", fs.fsmnt(), what));
        }
    });
}

/// The uid of a credential the allocator is handed (`NOCRED` is a caller's bug, which
/// `DIAGNOSTIC` catches as "missing credential").
fn cred_uid(cred: Cred<'_>, func: &str) -> Uid {
    match cred.ucred() {
        Some(c) => c.cr_uid.get(),
        None => kpanic(format_args!("{func}: missing credential")),
    }
}

/// The cylinder group of a busy buffer of `ffs_cgread`.
fn cgview(t: &mut BufRef) -> CgView<'_> {
    CgView::new(t.data())
}

/// `ffs_alloc`: allocate a block in the file system; the block's address.
///
/// The size of the requested block is given, which must be some multiple of `fs_fsize` and
/// <= `fs_bsize`. A preference may be optionally specified. If a preference is given the
/// following hierarchy is used to allocate a block:
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
pub fn ffs_alloc<G: ProcGlue>(
    ip: &Inode<G>,
    _lbn: Daddr,
    mut bpref: Daddr,
    size: i32,
    cred: Cred<'_>,
) -> Result<Daddr, Errno> {
    let fs = ip.i_fs();
    if cfg!(feature = "diagnostic")
        && (size as u32 > fs.fs_bsize() as u32 || fragoff(fs, i64::from(size)) != 0)
    {
        printf(format_args!(
            "dev = 0x{:x}, bsize = {}, size = {}, fs = {}\n",
            ip.i_dev,
            fs.fs_bsize(),
            size,
            fs.fsmnt()
        ));
        kpanic(format_args!("ffs_alloc: bad size"));
    }
    let uid = cred_uid(cred, "ffs_alloc");

    'nospace: {
        if size == fs.fs_bsize() && fs.fs_cstotal_nbfree() == 0 {
            break 'nospace;
        }
        if uid != 0 && freespace(fs, fs.fs_minfree()) <= 0 {
            break 'nospace;
        }

        ufs_quota_alloc_blocks(ip, btodb(i64::from(size)), cred)?;

        // Start allocation in the preferred block's cylinder group or the file's inode's
        // cylinder group if no preferred block was specified.
        if bpref >= fs.fs_size() {
            bpref = 0;
        }
        let cg = if bpref == 0 {
            ino_to_cg(fs, ip.i_number)
        } else {
            dtog(fs, bpref)
        };

        // Try allocating a block.
        let bno = ffs_hashalloc(ip, cg, bpref, size, ffs_alloccg::<G>);
        if bno > 0 {
            // allocation successful, update inode data
            ip.dip_set_blocks(ip.dip_blocks() + btodb(i64::from(size)));
            ip.set_flag(IN_CHANGE | IN_UPDATE);
            return Ok(bno);
        }

        // Restore user's disk quota because allocation failed.
        let _ = ufs_quota_free_blocks(ip, btodb(i64::from(size)), cred);
    }

    // nospace:
    if fserr_ratecheck(&FFS_ALLOC_FSFULL_LAST) {
        ffs_fserr(fs, uid, "file system full");
        uprintf_fs::<G>(fs, "write failed, file system is full");
    }
    Err(Errno::ENOSPC)
}

/// `ffs_realloccg`: reallocate a fragment to a bigger size; the file's buffer for the block,
/// grown and zero-filled past `osize`, and the new block's address.
///
/// The number and size of the old block is given, and a preference and new size is also
/// specified. The allocator attempts to extend the original block. Failing that, the regular
/// block allocator is invoked to get an appropriate block.
pub fn ffs_realloccg<G: ProcGlue>(
    ip: &Inode<G>,
    lbprev: Daddr,
    mut bpref: Daddr,
    osize: i32,
    nsize: i32,
    cred: Cred<'_>,
) -> Result<(BufRef, Daddr), Errno> {
    let mut bp: Option<BufRef> = None;
    let mut quota_updated: Daddr = 0;

    let fs = ip.i_fs();
    if cfg!(feature = "diagnostic")
        && (osize as u32 > fs.fs_bsize() as u32
            || fragoff(fs, i64::from(osize)) != 0
            || nsize as u32 > fs.fs_bsize() as u32
            || fragoff(fs, i64::from(nsize)) != 0)
    {
        printf(format_args!(
            "dev = 0x{:x}, bsize = {}, osize = {}, nsize = {}, fs = {}\n",
            ip.i_dev,
            fs.fs_bsize(),
            osize,
            nsize,
            fs.fsmnt()
        ));
        kpanic(format_args!("ffs_realloccg: bad size"));
    }
    let uid = cred_uid(cred, "ffs_realloccg");

    let error: Errno = 'error: {
        'nospace: {
            if uid != 0 && freespace(fs, fs.fs_minfree()) <= 0 {
                break 'nospace;
            }

            let bprev = ip.dip_db(lbprev as usize);

            if bprev == 0 {
                printf(format_args!(
                    "dev = 0x{:x}, bsize = {}, bprev = {}, fs = {}\n",
                    ip.i_dev,
                    fs.fs_bsize(),
                    bprev,
                    fs.fsmnt()
                ));
                kpanic(format_args!("ffs_realloccg: bad bprev"));
            }

            // Allocate the extra space in the buffer.
            let (t, r) = bread(&ip.itov().bufs, lbprev, fs.fs_bsize());
            let t = bp.insert(t);
            if let Err(e) = r {
                break 'error e;
            }
            buf_adjcnt(t, i64::from(osize));

            if let Err(e) = ufs_quota_alloc_blocks(ip, btodb(i64::from(nsize - osize)), cred) {
                break 'error e;
            }

            quota_updated = btodb(i64::from(nsize - osize));

            // Check for extension in the existing location.
            let cg = dtog(fs, bprev);
            let bno = ffs_fragextend(ip, cg, bprev, osize, nsize);
            if bno != 0 {
                ip.dip_set_blocks(ip.dip_blocks() + btodb(i64::from(nsize - osize)));
                ip.set_flag(IN_CHANGE | IN_UPDATE);
                if t.buf().blkno() != fsbtodb(fs, bno) {
                    kpanic(format_args!("ffs_realloccg: bad blockno"));
                }
                return Ok((grow(bp, osize, nsize, "ffs_realloccg: small buf"), bno));
            }
            // Allocate a new disk location.
            if bpref >= fs.fs_size() {
                bpref = 0;
            }
            let request = match fs.fs_optim() {
                FS_OPTSPACE => {
                    // Allocate an exact sized fragment. Although this makes best use of
                    // space, we will waste time relocating it if the file continues to grow.
                    // If the fragmentation is less than half of the minimum free reserve, we
                    // choose to begin optimizing for time.
                    if !(fs.fs_minfree() < 5
                        || fs.fs_cstotal_nffree()
                            > fs.fs_dsize() * i64::from(fs.fs_minfree()) / (2 * 100))
                    {
                        fs.set_fs_optim(FS_OPTTIME);
                    }
                    nsize
                }
                FS_OPTTIME => {
                    // At this point we have discovered a file that is trying to grow a small
                    // fragment to a larger fragment. To save time, we allocate a full sized
                    // block, then free the unused portion. If the file continues to grow,
                    // the `ffs_fragextend' call above will be able to grow it in place
                    // without further copying. If aberrant programs cause disk fragmentation
                    // to grow within 2% of the free reserve, we choose to begin optimizing
                    // for space.
                    if fs.fs_cstotal_nffree()
                        >= fs.fs_dsize() * i64::from(fs.fs_minfree() - 2) / 100
                    {
                        fs.set_fs_optim(FS_OPTSPACE);
                    }
                    fs.fs_bsize()
                }
                optim => {
                    printf(format_args!(
                        "dev = 0x{:x}, optim = {}, fs = {}\n",
                        ip.i_dev,
                        optim,
                        fs.fsmnt()
                    ));
                    kpanic(format_args!("ffs_realloccg: bad optim"));
                }
            };
            let bno = ffs_hashalloc(ip, cg, bpref, request, ffs_alloccg::<G>);
            if bno <= 0 {
                break 'nospace;
            }

            let _ = uvm_vnp_uncache(ip.itov());
            ffs_blkfree(ip, bprev, i64::from(osize));
            if nsize < request {
                ffs_blkfree(
                    ip,
                    bno + numfrags(fs, i64::from(nsize)),
                    i64::from(request - nsize),
                );
            }
            ip.dip_set_blocks(ip.dip_blocks() + btodb(i64::from(nsize - osize)));
            ip.set_flag(IN_CHANGE | IN_UPDATE);
            if let Some(t) = &bp {
                t.buf().set_blkno(fsbtodb(fs, bno));
            }
            return Ok((grow(bp, osize, nsize, "ffs_realloccg: small buf 2"), bno));
        }

        // nospace:
        if fserr_ratecheck(&FFS_REALLOCCG_FSFULL_LAST) {
            ffs_fserr(fs, uid, "file system full");
            uprintf_fs::<G>(fs, "write failed, file system is full");
        }
        Errno::ENOSPC
    };

    // error:
    if let Some(t) = bp {
        brelse(t);
    }

    // Restore user's disk quota because allocation failed.
    if quota_updated != 0 {
        let _ = ufs_quota_free_blocks(ip, quota_updated, cred);
    }

    Err(error)
}

/// `ffs_realloccg`'s success: the buffer `bread` gave, its count grown to `nsize`, marked
/// done, the bytes past `osize` zero (`small` is the C's `DIAGNOSTIC` panic when the buffer
/// is too small).
fn grow(bp: Option<BufRef>, osize: i32, nsize: i32, small: &str) -> BufRef {
    let Some(mut t) = bp else {
        kpanic(format_args!("ffs_realloccg: no buffer"));
    };
    if cfg!(feature = "diagnostic") && i64::from(nsize) > t.buf().bufsize() {
        kpanic(format_args!("{small}"));
    }
    buf_adjcnt(&t, i64::from(nsize));
    t.buf().set(B_DONE);
    t.data()[osize as usize..nsize as usize].fill(0);
    t
}

/// `ffs_inode_alloc` (`iv_inode_alloc`): allocate an inode in the file system, and return
/// its vnode, referenced and locked.
///
/// If allocating a directory, use `ffs_dirpref` to select the inode. If allocating in a
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
pub fn ffs_inode_alloc<G: ProcGlue>(
    pip: &Inode<G>,
    mode: Mode,
    cred: Cred<'_>,
) -> Result<&'static Vnode<G>, Errno> {
    let pvp = pip.itov();
    let fs = pip.i_fs();

    'noinodes: {
        if fs.fs_cstotal_nifree() == 0 {
            break 'noinodes;
        }

        let mut ipref = if mode & IFMT == IFDIR {
            ffs_dirpref(pip)
        } else {
            pip.i_number
        };
        if ipref >= fs.fs_ncg().wrapping_mul(fs.fs_ipg()) {
            ipref = 0;
        }
        let cg = ino_to_cg(fs, ipref);

        // Track number of dirs created one after another in a same cg without intervening by
        // files.
        fs.with_contigdirs(|c| {
            let Some(n) = c.get(cg as usize) else {
                kpanic(format_args!("fs_contigdirs: cg {cg}"));
            };
            let v = n.load(Ordering::Relaxed);
            if mode & IFMT == IFDIR {
                if v < 255 {
                    n.store(v + 1, Ordering::Relaxed);
                }
            } else if v > 0 {
                n.store(v - 1, Ordering::Relaxed);
            }
        });
        let ino = ffs_hashalloc(
            pip,
            cg,
            Daddr::from(ipref),
            mode as i32,
            ffs_nodealloccg::<G>,
        ) as Ufsino;
        if ino == 0 {
            break 'noinodes;
        }
        let Some(mp) = pvp.mount() else {
            kpanic(format_args!("ffs_inode_alloc: vnode without a mount"));
        };
        let vp = match VFS_VGET(mp, Ino::from(ino)) {
            Ok(vp) => vp,
            Err(e) => {
                let _ = ffs_inode_free(pip, ino, mode);
                return Err(e);
            }
        };

        let ip = vtoi(vp);

        if ip.dip_mode() != 0 {
            printf(format_args!(
                "mode = 0{:o}, inum = {}, fs = {}\n",
                ip.dip_mode(),
                ip.i_number,
                fs.fsmnt()
            ));
            kpanic(format_args!("ffs_valloc: dup alloc"));
        }

        if ip.dip_blocks() != 0 {
            printf(format_args!(
                "free inode {}/{} had {} blocks\n",
                fs.fsmnt(),
                ino,
                ip.dip_blocks()
            ));
            ip.dip_set_blocks(0);
        }

        ip.dip_set_flags(0);

        // Set up a new generation number for this inode. On wrap, we make sure to assign a
        // number != 0 and != UINT_MAX (the original value).
        if ip.dip_gen() != 0 {
            ip.dip_set_gen(ip.dip_gen().wrapping_add(1));
        }
        while ip.dip_gen() == 0 {
            ip.dip_set_gen(arc4random_uniform(u32::MAX));
        }

        return Ok(vp);
    }

    // noinodes:
    if fserr_ratecheck(&FFS_INODE_ALLOC_FSNOINODES_LAST) {
        ffs_fserr(fs, cred_uid(cred, "ffs_inode_alloc"), "out of inodes");
        uprintf_fs::<G>(fs, "create/symlink failed, no inodes free");
    }
    Err(Errno::ENOSPC)
}

/// `ffs_dirpref`: find a cylinder group to place a directory.
///
/// The policy implemented by this algorithm is to allocate a directory inode in the same
/// cylinder group as its parent directory, but also to reserve space for its files inodes
/// and data. Restrict the number of directories which may be allocated one after another in
/// the same cylinder group without intervening allocation of files.
///
/// If we allocate a first level directory then force allocation in another cylinder group.
pub fn ffs_dirpref<G: ProcGlue>(pip: &Inode<G>) -> Ufsino {
    let fs = pip.i_fs();
    let ncg = fs.fs_ncg();

    let avgifree = (fs.fs_cstotal_nifree() / i64::from(ncg)) as u32;
    let avgbfree = (fs.fs_cstotal_nbfree() / i64::from(ncg)) as u32;
    let avgndir = (fs.fs_cstotal_ndir() / i64::from(ncg)) as u32;
    let cs = |cg: u32| fs.fs_cs(cg);

    let cg: u32 = 'end: {
        // Force allocation in another cg if creating a first level dir.
        let prefcg = if pip.itov().flag() & VROOT != 0 {
            let prefcg = arc4random_uniform(ncg);
            let mut mincg = prefcg;
            let mut minndir = fs.fs_ipg();
            for cg in (prefcg..ncg).chain(0..prefcg) {
                if (cs(cg).cs_ndir() as u32) < minndir
                    && cs(cg).cs_nifree() as u32 >= avgifree
                    && cs(cg).cs_nbfree() as u32 >= avgbfree
                {
                    mincg = cg;
                    minndir = cs(cg).cs_ndir() as u32;
                }
            }
            break 'end mincg;
        } else {
            ino_to_cg(fs, pip.i_number)
        };

        // Count various limits which used for optimal allocation of a directory inode.
        let maxndir = avgndir.wrapping_add(fs.fs_ipg() / 16).min(fs.fs_ipg());
        let minifree = (avgifree - avgifree / 4).max(1);
        let minbfree = (avgbfree - avgbfree / 4).max(1);

        let cgsize = (fs.fs_fsize() as u32).wrapping_mul(fs.fs_fpg() as u32);
        let mut dirsize = fs.fs_avgfilesize().wrapping_mul(fs.fs_avgfpdir());
        let curdirsize = cgsize
            .wrapping_sub(avgbfree.wrapping_mul(fs.fs_bsize() as u32))
            .checked_div(avgndir)
            .unwrap_or(0);
        if dirsize < curdirsize {
            dirsize = curdirsize;
        }
        let mut maxcontigdirs = match avgbfree
            .wrapping_mul(fs.fs_bsize() as u32)
            .checked_div(dirsize)
        {
            Some(n) => n.min(255),
            None => 0, // dirsize overflowed
        };
        if fs.fs_avgfpdir() > 0 {
            maxcontigdirs = maxcontigdirs.min(fs.fs_ipg() / fs.fs_avgfpdir());
        }
        if maxcontigdirs == 0 {
            maxcontigdirs = 1;
        }

        // Limit number of dirs in one cg and reserve space for regular files, but only if
        // we have no deficit in inodes or space.
        //
        // We are trying to find a suitable cylinder group nearby our preferred cylinder
        // group to place a new directory. We scan from our preferred cylinder group forward
        // looking for a cylinder group that meets our criterion. If we get to the final
        // cylinder group and do not find anything, we start scanning forwards from the
        // beginning of the filesystem. While it might seem sensible to start scanning
        // backwards or even to alternate looking forward and backward, this approach fails
        // badly when the filesystem is nearly full. Specifically, we first search all the
        // areas that have no space and finally try the one preceding that. We repeat this on
        // every request and in the case of the final block end up searching the entire
        // filesystem. By jumping to the front of the filesystem, our future forward searches
        // always look in new cylinder groups so finds every possible block after one pass
        // over the filesystem.
        let found = fs.with_contigdirs(|contig| {
            (prefcg..ncg).chain(0..prefcg).find(|&cg| {
                (cs(cg).cs_ndir() as u32) < maxndir
                    && cs(cg).cs_nifree() as u32 >= minifree
                    && cs(cg).cs_nbfree() as u32 >= minbfree
                    && contig
                        .get(cg as usize)
                        .is_some_and(|c| u32::from(c.load(Ordering::Relaxed)) < maxcontigdirs)
            })
        });
        if let Some(cg) = found {
            break 'end cg;
        }
        // This is a backstop when we have deficit in space.
        for cg in (prefcg..ncg).chain(0..prefcg) {
            if cs(cg).cs_nifree() as u32 >= avgifree {
                break 'end cg;
            }
        }
        // The C falls out of its last loop with cg == prefcg.
        prefcg
    };
    // end:
    fs.fs_ipg().wrapping_mul(cg)
}

/// `ffs1_blkpref`: select the desired position for the next block in a file.
///
/// The file is logically divided into sections. The first section is composed of the direct
/// blocks. Each additional section contains `fs_maxbpg` blocks.
///
/// If no blocks have been allocated in the first section, the policy is to request a block
/// in the same cylinder group as the inode that describes the file. The first indirect is
/// allocated immediately following the last direct block and the data blocks for the first
/// indirect immediately follow it.
///
/// If no blocks have been allocated in any other section, the indirect block(s) are
/// allocated in the same cylinder group as its inode in an area reserved immediately
/// following the inode blocks. The policy for the data blocks is to place them in a cylinder
/// group with a greater than average number of free blocks. An appropriate cylinder group is
/// found by using a rotor that sweeps the cylinder groups. When a new group of blocks is
/// needed, the sweep begins in the cylinder group following the cylinder group from which
/// the previous allocation was made. The sweep continues until a cylinder group with greater
/// than the average number of free blocks is found. If the allocation is for the first block
/// in an indirect block, the information on the previous allocation is unavailable; here a
/// best guess is made based upon the logical block number being allocated.
pub fn ffs1_blkpref<G: ProcGlue>(
    ip: &Inode<G>,
    lbn: Daddr,
    indx: i32,
    bap: Option<&dyn Fn(usize) -> Daddr>,
) -> Daddr {
    crate::assert::kassert!(indx <= 0 || bap.is_some());
    let fs = ip.i_fs();
    // Allocation of indirect blocks is indicated by passing negative values in indx: -1 for
    // single indirect, -2 for double indirect, -3 for triple indirect. As noted below, we
    // attempt to allocate the first indirect inline with the file data. For all later
    // indirect blocks, the data is often allocated in other cylinder groups. However to
    // speed random file access and to speed up fsck, the filesystem reserves the first
    // fs_metaspace blocks (typically half of fs_minfree) of the data area of each cylinder
    // group to hold these later indirect blocks.
    let inocg = ino_to_cg(fs, ip.i_number);
    if indx < 0 {
        // Our preference for indirect blocks is the zone at the beginning of the inode's
        // cylinder group data area that we try to reserve for indirect blocks.
        let mut pref = cgmeta(fs, inocg) as u32;
        // If we are allocating the first indirect block, try to place it immediately
        // following the last direct block.
        if indx == -1 && lbn < NDADDR as Daddr + nindir(fs) && ip.dip_db(NDADDR - 1) != 0 {
            pref = (ip.dip_db(NDADDR - 1) + Daddr::from(fs.fs_frag())) as u32;
        }
        return Daddr::from(pref as i32);
    }
    // The C returns an int32_t.
    Daddr::from(blkpref_data(ip, fs, inocg, lbn, indx, bap, false) as i32)
}

/// `ffs2_blkpref`: same as above, for UFS2.
#[cfg(feature = "ffs2")]
pub fn ffs2_blkpref<G: ProcGlue>(
    ip: &Inode<G>,
    lbn: Daddr,
    indx: i32,
    bap: Option<&dyn Fn(usize) -> Daddr>,
) -> Daddr {
    crate::assert::kassert!(indx <= 0 || bap.is_some());
    let fs = ip.i_fs();
    // Allocation of indirect blocks is indicated by passing negative values in indx (see
    // ffs1_blkpref).
    let inocg = ino_to_cg(fs, ip.i_number);
    if indx < 0 {
        // Our preference for indirect blocks is the zone at the beginning of the inode's
        // cylinder group data area that we try to reserve for indirect blocks.
        let mut pref = cgmeta(fs, inocg);
        // If we are allocating the first indirect block, try to place it immediately
        // following the last direct block.
        if indx == -1 && lbn < NDADDR as Daddr + nindir(fs) && ip.dip_db(NDADDR - 1) != 0 {
            pref = ip.dip_db(NDADDR - 1) + Daddr::from(fs.fs_frag());
        }
        return pref;
    }
    blkpref_data(ip, fs, inocg, lbn, indx, bap, true)
}

/// The data block half of `ffs1_blkpref`/`ffs2_blkpref` (`indx >= 0`), which the two share
/// but for the start of the cylinder group sweep and where the sweep's answer lies.
fn blkpref_data<G: ProcGlue>(
    ip: &Inode<G>,
    fs: &Superblock,
    inocg: u32,
    lbn: Daddr,
    indx: i32,
    bap: Option<&dyn Fn(usize) -> Daddr>,
    ufs2: bool,
) -> Daddr {
    let bap_at = |i: i32| match bap {
        Some(f) => f(i as usize),
        None => kpanic(format_args!("ffs_blkpref: no block array")),
    };
    // If we are allocating the first data block in the first indirect block and the
    // indirect has been allocated in the data block area, try to place it immediately
    // following the indirect block.
    if lbn == NDADDR as Daddr {
        let pref = ip.dip_ib(0);
        if pref != 0 && pref >= cgdata(fs, inocg) && pref < cgbase(fs, inocg + 1) {
            return pref + Daddr::from(fs.fs_frag());
        }
    }
    // If we are the beginning of a file, or we have already allocated the maximum number of
    // blocks per cylinder group, or we do not have a block allocated immediately preceding
    // us, then we need to decide where to start allocating new blocks.
    if indx % fs.fs_maxbpg() == 0 || bap_at(indx - 1) == 0 {
        // If we are allocating a directory data block, we want to place it in the metadata
        // area.
        if ip.dip_mode() & IFMT == IFDIR {
            return cgmeta(fs, inocg);
        }
        // Until we fill all the direct and all the first indirect's blocks, we try to
        // allocate in the data area of the inode's cylinder group.
        if lbn < NDADDR as Daddr + nindir(fs) {
            return cgdata(fs, inocg);
        }
        // Find a cylinder with greater than average number of unused data blocks.
        let mut startcg = if indx == 0 || bap_at(indx - 1) == 0 {
            inocg.wrapping_add((lbn / Daddr::from(fs.fs_maxbpg())) as u32)
        } else if ufs2 {
            dtog(fs, bap_at(indx - 1) + 1)
        } else {
            dtog(fs, bap_at(indx - 1)).wrapping_add(1)
        };
        let ncg = fs.fs_ncg();
        startcg %= ncg;
        let avgbfree = (fs.fs_cstotal_nbfree() / i64::from(ncg)) as u32;

        if ufs2 {
            for cg in (startcg..ncg).chain(0..startcg) {
                if fs.fs_cs(cg).cs_nbfree() as u32 >= avgbfree {
                    return cgbase(fs, cg) + Daddr::from(fs.fs_frag());
                }
            }
        } else {
            for cg in (startcg..ncg).chain(0..=startcg) {
                if fs.fs_cs(cg).cs_nbfree() as u32 >= avgbfree {
                    fs.set_fs_cgrotor(cg as i32);
                    return cgdata(fs, cg);
                }
            }
        }
        return 0;
    }
    // Otherwise, we just always try to lay things out contiguously.
    bap_at(indx - 1) + Daddr::from(fs.fs_frag())
}

/// The allocator `ffs_hashalloc` tries in each cylinder group: the inode, the cylinder
/// group, the preference and the size (or the mode, for inodes); 0 when it found nothing.
pub type FfsAllocator<G> = fn(&Inode<G>, u32, Daddr, i32) -> Daddr;

/// `ffs_hashalloc`: implement the cylinder overflow algorithm.
///
/// The policy implemented by this algorithm is:
///   1) allocate the block in its requested cylinder group.
///   2) quadratically rehash on the cylinder group number.
///   3) brute force search for a free block.
pub fn ffs_hashalloc<G: ProcGlue>(
    ip: &Inode<G>,
    mut cg: u32,
    pref: Daddr,
    size: i32,
    allocator: FfsAllocator<G>,
) -> Daddr {
    let fs = ip.i_fs();
    let ncg = fs.fs_ncg();
    let icg = cg;
    // 1: preferred cylinder group
    let result = allocator(ip, cg, pref, size);
    if result != 0 {
        return result;
    }
    // 2: quadratic rehash
    let mut i: u32 = 1;
    while i < ncg {
        cg = cg.wrapping_add(i);
        if cg >= ncg {
            cg = cg.wrapping_sub(ncg);
        }
        let result = allocator(ip, cg, 0, size);
        if result != 0 {
            return result;
        }
        i = i.wrapping_mul(2);
    }
    // 3: brute force search
    // Note that we start at i == 2, since 0 was checked initially, and 1 is always checked
    // in the quadratic rehash.
    cg = icg.wrapping_add(2) % ncg;
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

/// `ffs_cgread`: the busy buffer of cylinder group `cg`'s block, or `None` when it cannot be
/// read or is not one (a block shorter than `struct cg`, from a bad `fs_cgsize`, is not one:
/// the C would read its magic past the block).
pub fn ffs_cgread<G: ProcGlue>(fs: &Superblock, ip: &Inode<G>, cg: u32) -> Option<BufRef> {
    let (mut t, error) = bread(
        &ip.i_devvp().bufs,
        fsbtodb(fs, cgtod(fs, cg)),
        fs.fs_cgsize(),
    );
    if error.is_err() {
        brelse(t);
        return None;
    }

    if t.bytes().len() < size_of::<Cg>() || !cgview(&mut t).cg_chkmagic() {
        brelse(t);
        return None;
    }

    Some(t)
}

/// `cgp->cg_ffs2_time = cgp->cg_time = now.tv_sec`.
fn cg_settime(cgp: &mut CgView<'_>) {
    let now = clock_now();
    cgp.set_cg_ffs2_time(now);
    cgp.set_cg_time(now as i32);
}

/// `nanotime(&now); now.tv_sec`.
fn clock_now() -> i64 {
    crate::tc::nanotime().tv_sec
}

/// `ffs_fragextend`: determine whether a fragment can be extended. Check to see if the
/// necessary fragments are available, and if they are, allocate them; `bprev`, or 0.
pub fn ffs_fragextend<G: ProcGlue>(
    ip: &Inode<G>,
    cg: u32,
    bprev: Daddr,
    osize: i32,
    nsize: i32,
) -> Daddr {
    let fs = ip.i_fs();
    if i64::from(fs.fs_cs(cg).cs_nffree()) < numfrags(fs, i64::from(nsize - osize)) {
        return 0;
    }
    let frags = numfrags(fs, i64::from(nsize));
    let bbase = fragnum(fs, bprev);
    if bbase > fragnum(fs, bprev + frags - 1) {
        // cannot extend across a block boundary
        return 0;
    }

    let Some(mut t) = ffs_cgread(fs, ip, cg) else {
        return 0;
    };

    let extended = {
        let mut cgp = cgview(&mut t);
        cg_settime(&mut cgp);

        let bno = dtogd(fs, bprev);
        let ofrags = numfrags(fs, i64::from(osize));
        if (ofrags..frags).any(|i| isclr(cgp.cg_blksfree(), (bno + i) as usize)) {
            false
        } else {
            // the current fragment can be extended
            // deduct the count on fragment being extended into
            // increase the count on the remaining fragment (if any)
            // allocate the extended piece
            let mut i = frags;
            while i < i64::from(fs.fs_frag()) - bbase {
                if isclr(cgp.cg_blksfree(), (bno + i) as usize) {
                    break;
                }
                i += 1;
            }
            cgp.add_cg_frsum((i - ofrags) as usize, -1);
            if i != frags {
                cgp.add_cg_frsum((i - frags) as usize, 1);
            }
            for i in ofrags..frags {
                clrbit(cgp.cg_blksfree(), (bno + i) as usize);
                cgp.add_cg_cs_nffree(-1);
                fs.add_fs_cstotal_nffree(-1);
                fs.fs_cs(cg).add_cs_nffree(-1);
            }
            fs.set_fs_fmod(1);
            true
        }
    };
    if !extended {
        brelse(t);
        return 0;
    }

    bdwrite(t);
    bprev
}

/// `ffs_alloccg`: determine whether a block can be allocated. Check to see if a block of
/// the appropriate size is available, and if it is, allocate it.
pub fn ffs_alloccg<G: ProcGlue>(ip: &Inode<G>, cg: u32, mut bpref: Daddr, size: i32) -> Daddr {
    let fs = ip.i_fs();
    if fs.fs_cs(cg).cs_nbfree() == 0 && size == fs.fs_bsize() {
        return 0;
    }

    let Some(mut t) = ffs_cgread(fs, ip, cg) else {
        return 0;
    };

    // The block, or `None` for a release without a change.
    let r: Option<Daddr> = 'done: {
        let mut cgp = cgview(&mut t);
        if cgp.cg_cs_nbfree() == 0 && size == fs.fs_bsize() {
            break 'done None;
        }

        cg_settime(&mut cgp);

        if size == fs.fs_bsize() {
            // allocate and return a complete data block
            break 'done Some(ffs_alloccgblk(ip, &mut cgp, bpref));
        }
        // check to see if any fragments are already available
        // allocsiz is the size which will be allocated, hacking it down to a smaller size if
        // necessary
        let frags = numfrags(fs, i64::from(size)) as i32;
        let mut allocsiz = frags;
        while allocsiz < fs.fs_frag() {
            if cgp.cg_frsum(allocsiz as usize) != 0 {
                break;
            }
            allocsiz += 1;
        }
        if allocsiz == fs.fs_frag() {
            // no fragments were available, so a block will be allocated, and hacked up
            if cgp.cg_cs_nbfree() == 0 {
                break 'done None;
            }
            let bno = ffs_alloccgblk(ip, &mut cgp, bpref);
            bpref = dtogd(fs, bno);
            for i in frags..fs.fs_frag() {
                setbit(cgp.cg_blksfree(), (bpref + Daddr::from(i)) as usize);
            }
            let i = fs.fs_frag() - frags;
            cgp.add_cg_cs_nffree(i);
            fs.add_fs_cstotal_nffree(i64::from(i));
            fs.fs_cs(cg).add_cs_nffree(i);
            fs.set_fs_fmod(1);
            cgp.add_cg_frsum(i as usize, 1);
            break 'done Some(bno);
        }
        let bno = ffs_mapsearch(fs, &mut cgp, bpref, allocsiz);
        if bno < 0 {
            break 'done None;
        }

        for i in 0..frags {
            clrbit(cgp.cg_blksfree(), (bno + Daddr::from(i)) as usize);
        }
        cgp.add_cg_cs_nffree(-frags);
        fs.add_fs_cstotal_nffree(-i64::from(frags));
        fs.fs_cs(cg).add_cs_nffree(-frags);
        fs.set_fs_fmod(1);
        cgp.add_cg_frsum(allocsiz as usize, -1);
        if frags != allocsiz {
            cgp.add_cg_frsum((allocsiz - frags) as usize, 1);
        }

        Some(cgbase(fs, cg) + bno)
    };
    if let Some(blkno) = r {
        bdwrite(t);
        blkno
    } else {
        brelse(t);
        0
    }
}

/// `ffs_alloccgblk`: allocate a block in a cylinder group. Note that this routine only
/// allocates `fs_bsize` blocks; these blocks may be fragmented by the routine that allocates
/// them.
pub fn ffs_alloccgblk<G: ProcGlue>(ip: &Inode<G>, cgp: &mut CgView<'_>, mut bpref: Daddr) -> Daddr {
    let fs = ip.i_fs();
    let cgx = cgp.cg_cgx();

    if bpref == 0 {
        bpref = Daddr::from(cgp.cg_rotor());
    } else {
        let cgbpref = dtog(fs, bpref);
        if cgbpref != cgx {
            // map bpref to correct zone in this cg
            if bpref < cgdata(fs, cgbpref) {
                bpref = cgmeta(fs, cgx);
            } else {
                bpref = cgdata(fs, cgx);
            }
        }
    }
    // If the requested block is available, use it.
    let mut bno = dtogd(fs, blknum(fs, bpref));
    if !ffs_isblock(fs, cgp.cg_blksfree(), fragstoblks(fs, bno)) {
        // Take the next available block in this cylinder group.
        bno = ffs_mapsearch(fs, cgp, bpref, fs.fs_frag());
        if bno < 0 {
            return 0;
        }

        // Update cg_rotor only if allocated from the data zone
        if bno >= dtogd(fs, cgdata(fs, cgx)) {
            cgp.set_cg_rotor(bno as u32);
        }
    }

    // gotit:
    let blkno = fragstoblks(fs, bno);
    ffs_clrblock(fs, cgp.cg_blksfree(), blkno);
    ffs_clusteracct(fs, cgp, blkno, -1);
    cgp.add_cg_cs_nbfree(-1);
    fs.add_fs_cstotal_nbfree(-1);
    fs.fs_cs(cgx).add_cs_nbfree(-1);

    if fs.fs_magic() != FS_UFS2_MAGIC {
        let cylno = cbtocylno(fs, bno);
        cgp.add_cg_blks(fs, cylno, cbtorpos(fs, bno), -1);
        cgp.add_cg_blktot(cylno, -1);
    }

    fs.set_fs_fmod(1);
    cgbase(fs, cgx) + bno
}

/// `ffs_nodealloccg`: inode allocation routine; the inode number, or 0.
pub fn ffs_nodealloccg<G: ProcGlue>(ip: &Inode<G>, cg: u32, mut ipref: Daddr, mode: i32) -> Daddr {
    // For efficiency, before looking at the bitmaps for free inodes, check the counters kept
    // in the superblock cylinder group summaries, and in the cylinder group itself.
    let fs = ip.i_fs();
    if fs.fs_cs(cg).cs_nifree() == 0 {
        return 0;
    }

    let Some(mut t) = ffs_cgread(fs, ip, cg) else {
        return 0;
    };

    // The inode block FFS2 initialises (getblk, filled; written after the group).
    #[cfg(feature = "ffs2")]
    let mut ibp: Option<BufRef> = None;
    let ok = {
        let mut cgp = cgview(&mut t);
        if cgp.cg_cs_nifree() == 0 {
            false
        } else {
            // We are committed to the allocation from now on, so update the time on the
            // cylinder group.
            cg_settime(&mut cgp);

            'gotit: {
                // If there was a preferred location for the new inode, try to find it.
                if ipref != 0 {
                    ipref %= Daddr::from(fs.fs_ipg());
                    if isclr(cgp.cg_inosused(), ipref as usize) {
                        break 'gotit; // inode is free, grab it.
                    }
                }

                // Otherwise, look for the next available inode, starting at cg_irotor (the
                // position in the bitmap of the last used inode).
                let irotor = cgp.cg_irotor();
                let mut start = (irotor / 8) as usize;
                let mut len = howmany_u32(fs.fs_ipg().wrapping_sub(irotor));
                let mut loc = skpc(0xff, map_range(cgp.cg_inosused(), start, len));
                if loc == 0 {
                    // If we didn't find a free inode in the upper part of the bitmap (from
                    // cg_irotor to the end), then look at the bottom part (from 0 to
                    // cg_irotor).
                    len = start + 1;
                    start = 0;
                    loc = skpc(0xff, map_range(cgp.cg_inosused(), start, len));
                    if loc == 0 {
                        // If we failed again, then either the bitmap or the counters kept for
                        // the cylinder group are wrong.
                        printf(format_args!(
                            "cg = {}, irotor = {}, fs = {}\n",
                            cg,
                            cgp.cg_irotor(),
                            fs.fsmnt()
                        ));
                        kpanic(format_args!("ffs_nodealloccg: map corrupted"));
                    }
                }

                // skpc() returns the position relative to the end
                let i = start + len - loc;

                // Okay, so now in 'i' we have the location in the bitmap of a byte holding a
                // free inode. Find the corresponding bit and set it, updating cg_irotor as
                // well, accordingly.
                let map = cgp.cg_inosused()[i];
                ipref = (i * 8) as Daddr;
                let mut bit = 1u32;
                while bit < 1 << 8 {
                    if u32::from(map) & bit == 0 {
                        cgp.set_cg_irotor(ipref as u32);
                        break 'gotit;
                    }
                    bit <<= 1;
                    ipref += 1;
                }

                printf(format_args!("fs = {}\n", fs.fsmnt()));
                kpanic(format_args!("ffs_nodealloccg: block not in map"));
            }

            // gotit:
            #[cfg(feature = "ffs2")]
            {
                // For FFS2, check if all inodes in this cylinder group have been used at
                // least once. If they haven't, and we are allocating an inode past the last
                // allocated block of inodes, read in a block and initialize all inodes in it.
                if fs.fs_magic() == FS_UFS2_MAGIC
                    // Inode is beyond last initialized block of inodes?
                    && ipref + Daddr::from(inopb(fs)) > Daddr::from(cgp.cg_initediblk())
                    // Has any inode not been used at least once?
                    && cgp.cg_initediblk() < cgp.cg_ffs2_niblk()
                {
                    let ino = cg
                        .wrapping_mul(fs.fs_ipg())
                        .wrapping_add(cgp.cg_initediblk());
                    let mut b = loop {
                        if let Some(b) = getblk(
                            &ip.i_devvp().bufs,
                            fsbtodb(fs, ino_to_fsba(fs, ino)),
                            fs.fs_bsize(),
                            0,
                            hal::sleep::INFSLP,
                        ) {
                            break b;
                        }
                    };

                    let data = b.data();
                    data.fill(0);

                    // Give each inode a generation number
                    let dsize = size_of::<crate::fs::ufs::ufs::dinode::Ufs2Dinode>();
                    let gen_off =
                        core::mem::offset_of!(crate::fs::ufs::ufs::dinode::Ufs2Dinode, di_gen);
                    for i in 0..inopb(fs) as usize {
                        let off = i * dsize + gen_off;
                        let mut generation = 0i32;
                        while generation == 0 {
                            generation = arc4random() as i32;
                        }
                        match data.get_mut(off..off + 4) {
                            Some(d) => d.copy_from_slice(&generation.to_ne_bytes()),
                            None => kpanic(format_args!("ffs_nodealloccg: INOPB past the block")),
                        }
                    }

                    // Update the counter of initialized inodes
                    cgp.set_cg_initediblk(cgp.cg_initediblk().wrapping_add(inopb(fs)));
                    ibp = Some(b);
                }
            }

            setbit(cgp.cg_inosused(), ipref as usize);

            // Update the counters we keep on free inodes
            cgp.add_cg_cs_nifree(-1);
            fs.add_fs_cstotal_nifree(-1);
            fs.fs_cs(cg).add_cs_nifree(-1);
            fs.set_fs_fmod(1); // file system was modified

            // Update the counters we keep on allocated directories
            if mode as Mode & IFMT == IFDIR {
                cgp.add_cg_cs_ndir(1);
                fs.add_fs_cstotal_ndir(1);
                fs.fs_cs(cg).add_cs_ndir(1);
            }
            true
        }
    };
    if !ok {
        brelse(t);
        return 0;
    }

    bdwrite(t);

    #[cfg(feature = "ffs2")]
    if let Some(ibp) = ibp {
        bawrite(ibp);
    }

    // Return the allocated inode number
    Daddr::from(cg.wrapping_mul(fs.fs_ipg())) + ipref
}

/// `howmany(x, NBBY)` of a `u_int` count, as `int` (`ffs_nodealloccg`'s `len`).
fn howmany_u32(x: u32) -> usize {
    x.div_ceil(8) as usize
}

/// `&map[start]` for `len` bytes, the C's `skpc`/`scanc` window: `panic(9)` past the block,
/// where the C would scan past it.
fn map_range(map: &[u8], start: usize, len: usize) -> &[u8] {
    match map.get(start..start.saturating_add(len)) {
        Some(m) => m,
        None => kpanic(format_args!(
            "ffs: map bytes {start}+{len} past the cylinder group"
        )),
    }
}

/// `ffs_blkfree`: free a block or fragment.
///
/// The specified block or fragment is placed back in the free map. If a fragment is
/// deallocated, a possible block reassembly is checked.
pub fn ffs_blkfree<G: ProcGlue>(ip: &Inode<G>, mut bno: Daddr, size: i64) {
    let fs = ip.i_fs();
    if size as u32 > fs.fs_bsize() as u32
        || fragoff(fs, size) != 0
        || fragnum(fs, bno) + numfrags(fs, size) > i64::from(fs.fs_frag())
    {
        printf(format_args!(
            "dev = 0x{:x}, bsize = {}, size = {}, fs = {}\n",
            ip.i_dev,
            fs.fs_bsize(),
            size,
            fs.fsmnt()
        ));
        kpanic(format_args!("ffs_blkfree: bad size"));
    }
    let cg = dtog(fs, bno);
    if i64::from(bno as u32) >= fs.fs_size() {
        printf(format_args!("bad block {}, ino {}\n", bno, ip.i_number));
        ffs_fserr(fs, ip.dip_uid(), "bad block");
        return;
    }
    let Some(mut t) = ffs_cgread(fs, ip, cg) else {
        return;
    };

    {
        let mut cgp = cgview(&mut t);
        cg_settime(&mut cgp);

        bno = dtogd(fs, bno);
        if size == i64::from(fs.fs_bsize()) {
            let blkno = fragstoblks(fs, bno);
            if !ffs_isfreeblock(fs, cgp.cg_blksfree(), blkno) {
                printf(format_args!(
                    "dev = 0x{:x}, block = {}, fs = {}\n",
                    ip.i_dev,
                    bno,
                    fs.fsmnt()
                ));
                kpanic(format_args!("ffs_blkfree: freeing free block"));
            }
            ffs_setblock(fs, cgp.cg_blksfree(), blkno);
            ffs_clusteracct(fs, &mut cgp, blkno, 1);
            cgp.add_cg_cs_nbfree(1);
            fs.add_fs_cstotal_nbfree(1);
            fs.fs_cs(cg).add_cs_nbfree(1);

            if fs.fs_magic() != FS_UFS2_MAGIC {
                let i = cbtocylno(fs, bno);
                cgp.add_cg_blks(fs, i, cbtorpos(fs, bno), 1);
                cgp.add_cg_blktot(i, 1);
            }
        } else {
            let bbase = bno - fragnum(fs, bno);
            // decrement the counts associated with the old frags
            let blk = blkmap(fs, cgp.cg_blksfree(), bbase);
            ffs_fragacct(fs, blk, &mut cgp, -1);
            // deallocate the fragment
            let frags = numfrags(fs, size);
            let mut i = 0;
            while i < frags {
                if isset(cgp.cg_blksfree(), (bno + i) as usize) {
                    printf(format_args!(
                        "dev = 0x{:x}, block = {}, fs = {}\n",
                        ip.i_dev,
                        bno + i,
                        fs.fsmnt()
                    ));
                    kpanic(format_args!("ffs_blkfree: freeing free frag"));
                }
                setbit(cgp.cg_blksfree(), (bno + i) as usize);
                i += 1;
            }
            cgp.add_cg_cs_nffree(i as i32);
            fs.add_fs_cstotal_nffree(i);
            fs.fs_cs(cg).add_cs_nffree(i as i32);
            // add back in counts associated with the new frags
            let blk = blkmap(fs, cgp.cg_blksfree(), bbase);
            ffs_fragacct(fs, blk, &mut cgp, 1);
            // if a complete block has been reassembled, account for it
            let blkno = fragstoblks(fs, bbase);
            if ffs_isblock(fs, cgp.cg_blksfree(), blkno) {
                cgp.add_cg_cs_nffree(-fs.fs_frag());
                fs.add_fs_cstotal_nffree(-i64::from(fs.fs_frag()));
                fs.fs_cs(cg).add_cs_nffree(-fs.fs_frag());
                ffs_clusteracct(fs, &mut cgp, blkno, 1);
                cgp.add_cg_cs_nbfree(1);
                fs.add_fs_cstotal_nbfree(1);
                fs.fs_cs(cg).add_cs_nbfree(1);

                if fs.fs_magic() != FS_UFS2_MAGIC {
                    let i = cbtocylno(fs, bbase);
                    cgp.add_cg_blks(fs, i, cbtorpos(fs, bbase), 1);
                    cgp.add_cg_blktot(i, 1);
                }
            }
        }
        fs.set_fs_fmod(1);
    }
    bdwrite(t);
}

/// `ffs_inode_free` (`iv_inode_free`).
pub fn ffs_inode_free<G: ProcGlue>(pip: &Inode<G>, ino: Ufsino, mode: Mode) -> Result<(), Errno> {
    ffs_freefile(pip, ino, mode)
}

/// `ffs_freefile`: do the actual free operation. The specified inode is placed back in the
/// free map.
pub fn ffs_freefile<G: ProcGlue>(pip: &Inode<G>, ino: Ufsino, mode: Mode) -> Result<(), Errno> {
    let fs = pip.i_fs();
    if ino >= fs.fs_ipg().wrapping_mul(fs.fs_ncg()) {
        kpanic(format_args!(
            "ffs_freefile: range: dev = 0x{:x}, ino = {}, fs = {}",
            pip.i_dev,
            ino,
            fs.fsmnt()
        ));
    }

    let cg = ino_to_cg(fs, ino);
    let Some(mut t) = ffs_cgread(fs, pip, cg) else {
        return Ok(());
    };

    {
        let mut cgp = cgview(&mut t);
        cg_settime(&mut cgp);

        let ino = ino % fs.fs_ipg();
        if isclr(cgp.cg_inosused(), ino as usize) {
            printf(format_args!(
                "dev = 0x{:x}, ino = {}, fs = {}\n",
                pip.i_dev,
                ino,
                fs.fsmnt()
            ));
            if fs.fs_ronly() == 0 {
                kpanic(format_args!("ffs_freefile: freeing free inode"));
            }
        }
        clrbit(cgp.cg_inosused(), ino as usize);
        if ino < cgp.cg_irotor() {
            cgp.set_cg_irotor(ino);
        }
        cgp.add_cg_cs_nifree(1);
        fs.add_fs_cstotal_nifree(1);
        fs.fs_cs(cg).add_cs_nifree(1);
        if mode & IFMT == IFDIR {
            cgp.add_cg_cs_ndir(-1);
            fs.add_fs_cstotal_ndir(-1);
            fs.fs_cs(cg).add_cs_ndir(-1);
        }
        fs.set_fs_fmod(1);
    }
    bdwrite(t);
    Ok(())
}

/// `ffs_mapsearch`: find a block of the specified size in the specified cylinder group.
///
/// It is a panic if a request is made to find a block if none are available.
pub fn ffs_mapsearch(fs: &Superblock, cgp: &mut CgView<'_>, bpref: Daddr, allocsiz: i32) -> Daddr {
    let frag = fs.fs_frag();
    let Some(tbl) = usize::try_from(frag)
        .ok()
        .and_then(|f| FRAGTBL.get(f).copied().flatten())
    else {
        kpanic(format_args!("ffs_mapsearch: fs_frag {frag}"));
    };
    let mask = (1u32 << (allocsiz - 1 + (frag % 8))) as u8;

    // find the fragment by searching through the free block map for an appropriate bit
    // pattern
    let mut start = if bpref != 0 {
        (dtogd(fs, bpref) / 8) as usize
    } else {
        (cgp.cg_frotor() / 8) as usize
    };
    let mut len = howmany_u32(fs.fs_fpg() as u32).wrapping_sub(start);
    let mut loc = scanc(map_range(cgp.cg_blksfree(), start, len), tbl, mask);
    if loc == 0 {
        len = start + 1;
        start = 0;
        loc = scanc(map_range(cgp.cg_blksfree(), start, len), tbl, mask);
        if loc == 0 {
            printf(format_args!(
                "start = {}, len = {}, fs = {}\n",
                start,
                len,
                fs.fsmnt()
            ));
            kpanic(format_args!("ffs_alloccg: map corrupted"));
        }
    }
    let mut bno = ((start + len - loc) * 8) as Daddr;
    cgp.set_cg_frotor(bno as u32);
    // found the byte in the map
    // sift through the bits to find the selected frag
    let end = bno + 8;
    while bno < end {
        let blk = blkmap(fs, cgp.cg_blksfree(), bno) << 1;
        let mut field = AROUND[allocsiz as usize];
        let mut subfield = INSIDE[allocsiz as usize];
        for pos in 0..=(frag - allocsiz) {
            if blk & field == subfield {
                return bno + Daddr::from(pos);
            }
            field <<= 1;
            subfield <<= 1;
        }
        bno += Daddr::from(frag);
    }
    printf(format_args!("bno = {}, fs = {}\n", bno, fs.fsmnt()));
    kpanic(format_args!("ffs_alloccg: block not in map"));
}

/// `ffs_clusteracct`: update the cluster map because of an allocation or free.
///
/// `cnt == 1` means free; `cnt == -1` means allocating.
pub fn ffs_clusteracct(fs: &Superblock, cgp: &mut CgView<'_>, blkno: Daddr, cnt: i32) {
    let contigsumsize = fs.fs_contigsumsize();
    if contigsumsize <= 0 {
        return;
    }
    let nclusterblks = i64::from(cgp.cg_nclusterblks());
    // Allocate or clear the actual block.
    if cnt > 0 {
        setbit(cgp.cg_clustersfree(), blkno as usize);
    } else {
        clrbit(cgp.cg_clustersfree(), blkno as usize);
    }
    let freemapp = cgp.cg_clustersfree();
    // Find the size of the cluster going forward.
    let start = blkno + 1;
    let end = (start + i64::from(contigsumsize)).min(nclusterblks);
    let mut i = start;
    while i < end && isset(freemapp, i as usize) {
        i += 1;
    }
    let forw = i - start;
    // Find the size of the cluster going backward.
    let start = blkno - 1;
    let end = (start - i64::from(contigsumsize)).max(-1);
    let mut i = start;
    while i > end && isset(freemapp, i as usize) {
        i -= 1;
    }
    let back = start - i;
    // Account for old cluster and the possibly new forward and back clusters.
    let i = (back + forw + 1).min(i64::from(contigsumsize));
    cgp.add_cg_clustersum(i as usize, cnt);
    if back > 0 {
        cgp.add_cg_clustersum(back as usize, -cnt);
    }
    if forw > 0 {
        cgp.add_cg_clustersum(forw as usize, -cnt);
    }
    // Update cluster summary information.
    let mut i = contigsumsize;
    while i > 0 {
        if cgp.cg_clustersum(i as usize) > 0 {
            break;
        }
        i -= 1;
    }
    let cgx = cgp.cg_cgx();
    fs.set_maxcluster(cgx, i);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::cell::RefCell;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use super::super::fs::{CgView, cgdmin};
    #[cfg(feature = "ffs2")]
    use super::super::fs::{ino_to_fsba, ino_to_fsbo};
    use super::super::testffs::newfs::{
        FFS1_1G, FFS2_1G, FFS2_4M, Image, Params, check_with, super_block,
    };
    use super::super::testffs::{disk, new_file, setup, sync};
    use super::*;
    use crate::cred::Ucred;
    use crate::fs::subr::vput;
    use crate::fs::ufs::ufs::dinode::IFREG;

    /// The formats the tests build: FFS2's only with `option FFS2`.
    const FORMATS: &[Params] = if cfg!(feature = "ffs2") {
        &[FFS2_1G, FFS1_1G]
    } else {
        &[FFS1_1G]
    };

    /// `fsck`'s check of the synced image against the in-core summaries and totals.
    fn check_incore(fs: &Superblock, ufs2: bool) {
        sync(&[]);
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

    /// `cg_frsum[i]` of cylinder group `cg`, read through the cache.
    fn frsum<G: ProcGlue>(ip: &Inode<G>, cg: u32, i: usize) -> u32 {
        let mut t = ffs_cgread(ip.i_fs(), ip, cg).unwrap();
        let v = CgView::new(t.data()).cg_frsum(i);
        brelse(t);
        v
    }

    #[test]
    fn blocks_and_fragments_are_taken_and_given_back_as_the_c_takes_them() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let cred = Cred::real(th.ucred());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let fs = ip.i_fs();
            let (bsize, fsize, frag) = (fs.fs_bsize(), fs.fs_fsize(), fs.fs_frag());
            let (nb, nf) = (fs.fs_cstotal_nbfree(), fs.fs_cstotal_nffree());
            // newfs left two runs of 7 free fragments: after the summary's one, and after the
            // root directory's.
            assert_eq!(frsum(&ip, 0, 7), 2);

            // A whole block, no preference: the cylinder group's first whole free block,
            // the one after the root directory's (cg_rotor 0 is in the boot area, so
            // ffs_mapsearch from cg_frotor).
            let b = ffs_alloc(&ip, 0, 0, bsize, cred).unwrap();
            assert_eq!(b, cgdmin(fs, 0) + 2 * Daddr::from(frag), "{p:?}");
            assert_eq!(fs.fs_cstotal_nbfree(), nb - 1);
            assert_eq!(fs.fs_cs(0).cs_nbfree() as i64, nb - 1);
            assert_eq!(ip.dip_blocks(), btodb(i64::from(bsize)));

            // Three fragments: cut from the first run of 7 (the search wraps from cg_frotor,
            // past the whole blocks, to the map's start).
            let f = ffs_alloc(&ip, 1, 0, 3 * fsize, cred).unwrap();
            assert_eq!(f, cgdmin(fs, 0) + 1, "{p:?}");
            assert_eq!(fs.fs_cstotal_nffree(), nf - 3);
            assert_eq!(frsum(&ip, 0, 7), 1);
            assert_eq!(frsum(&ip, 0, 4), 1);
            check_incore(fs, p.ufs2);

            // Both back: the counters and the maps as newfs left them.
            ffs_blkfree(&ip, f, i64::from(3 * fsize));
            ffs_blkfree(&ip, b, i64::from(bsize));
            assert_eq!(fs.fs_cstotal_nbfree(), nb);
            assert_eq!(fs.fs_cstotal_nffree(), nf);
            assert_eq!(frsum(&ip, 0, 7), 2);
            assert_eq!(frsum(&ip, 0, 4), 0);
            check_incore(fs, p.ufs2);

            ip.dip_set_blocks(0);
            drop(ip);
            vput(vp);
        }
    }

    // FFS2 only (its 1 KB fragments and block positions): without `option FFS2` the kernel
    // mounts no FFS2 image.
    #[cfg(feature = "ffs2")]
    #[test]
    fn fragments_reassembled_into_a_block_count_as_a_block() {
        let (_g, th, mp) = setup(Image::new(FFS2_1G).finish());
        let cred = Cred::real(th.ucred());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        let fs = ip.i_fs();
        let (nb, nf) = (fs.fs_cstotal_nbfree(), fs.fs_cstotal_nffree());
        // Both runs of 7 taken whole, then two fragments: no fragment is free, so the first
        // whole block is broken (cg_frsum[6] for the rest).
        let f7 = ffs_alloc(&ip, 0, 0, 7 * 1024, cred).unwrap();
        assert_eq!(f7, cgdmin(fs, 0) + 1);
        let g7 = ffs_alloc(&ip, 1, 0, 7 * 1024, cred).unwrap();
        assert_eq!(g7, cgdmin(fs, 0) + 9);
        assert_eq!(frsum(&ip, 0, 7), 0);
        let f2 = ffs_alloc(&ip, 2, 0, 2 * 1024, cred).unwrap();
        assert_eq!(f2, cgdmin(fs, 0) + 16, "a broken block's first fragments");
        assert_eq!(fs.fs_cstotal_nbfree(), nb - 1);
        assert_eq!(fs.fs_cstotal_nffree(), nf - 14 + 6);
        assert_eq!(frsum(&ip, 0, 6), 1);
        // The two back: the block is whole again.
        ffs_blkfree(&ip, f2, 2 * 1024);
        assert_eq!(fs.fs_cstotal_nbfree(), nb);
        assert_eq!(fs.fs_cstotal_nffree(), nf - 14);
        assert_eq!(frsum(&ip, 0, 6), 0);
        ffs_blkfree(&ip, g7, 7 * 1024);
        ffs_blkfree(&ip, f7, 7 * 1024);
        check_incore(fs, true);
        ip.dip_set_blocks(0);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn a_full_file_system_refuses_a_user_but_not_the_super_user() {
        let (_g, th, mp) = setup(Image::new(FORMATS[0]).finish());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        let fs = ip.i_fs();
        let mut user = Ucred::new();
        user.cr_uid.set(1000);
        // freespace() <= 0 with all of fs_dsize in reserve.
        let minfree = fs.fs_minfree();
        fs.set_fs_minfree(100);
        assert_eq!(
            ffs_alloc(&ip, 0, 0, fs.fs_bsize(), Cred::real(&user)),
            Err(Errno::ENOSPC)
        );
        let b = ffs_alloc(&ip, 0, 0, fs.fs_bsize(), Cred::real(th.ucred())).unwrap();
        ffs_blkfree(&ip, b, i64::from(fs.fs_bsize()));
        fs.set_fs_minfree(minfree);
        ip.dip_set_blocks(0);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn an_inode_is_the_first_free_one_and_comes_back() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let rootvp = super::super::testffs::root(mp);
            let rip = vtoi(rootvp);
            let fs = rip.i_fs();
            let ni = fs.fs_cstotal_nifree();
            // Inodes 0 to 2 are newfs's: the root's preference (2) is taken, the map is
            // searched from cg_irotor.
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            assert_eq!(ip.i_number, 3);
            assert!(ip.dip_gen() != 0, "a generation number");
            assert_eq!(fs.fs_cstotal_nifree(), ni - 1);
            assert_eq!(fs.fs_cs(0).cs_nifree() as i64, ni - 1);
            let mut t = ffs_cgread(fs, &rip, 0).unwrap();
            assert_eq!(CgView::new(t.data()).cg_irotor(), 3);
            brelse(t);
            check_incore(fs, p.ufs2);
            // A directory: ffs_dirpref's group (the only one), counted in cs_ndir and in
            // fs_contigdirs.
            let nd = fs.fs_cstotal_ndir();
            let d = ffs_inode_alloc(&rip, IFDIR | 0o755, Cred::real(th.ucred())).unwrap();
            assert_eq!(vtoi(d).i_number, 4);
            assert_eq!(fs.fs_cstotal_ndir(), nd + 1);
            assert_eq!(fs.with_contigdirs(|c| c[0].load(Ordering::Relaxed)), 1);
            // Both back.
            ffs_freefile(&rip, 4, IFDIR).unwrap();
            ffs_freefile(&rip, 3, IFREG).unwrap();
            assert_eq!(fs.fs_cstotal_nifree(), ni);
            assert_eq!(fs.fs_cstotal_ndir(), nd);
            let mut t = ffs_cgread(fs, &rip, 0).unwrap();
            assert_eq!(
                CgView::new(t.data()).cg_irotor(),
                3,
                "lowered to the freed inode"
            );
            brelse(t);
            check_incore(fs, p.ufs2);
            drop((ip, rip));
            vput(d);
            vput(vp);
            vput(rootvp);
        }
    }

    #[cfg(feature = "ffs2")]
    #[test]
    fn an_ffs2_inode_past_the_initialised_blocks_initialises_the_next() {
        let (_g, _th, mp) = setup(Image::new(FFS2_1G).finish());
        let rootvp = super::super::testffs::root(mp);
        let rip = vtoi(rootvp);
        let fs = rip.i_fs();
        let inopb = fs.fs_inopb();
        // newfs initialised two blocks of inodes (cg_initediblk 64): inode 100 is past them.
        let mut t = ffs_cgread(fs, &rip, 0).unwrap();
        assert_eq!(CgView::new(t.data()).cg_initediblk(), 2 * inopb);
        brelse(t);
        assert_eq!(ffs_nodealloccg(&rip, 0, 100, IFREG as i32), 100);
        let mut t = ffs_cgread(fs, &rip, 0).unwrap();
        assert_eq!(CgView::new(t.data()).cg_initediblk(), 3 * inopb);
        brelse(t);
        // The block of inodes 64 to 95 is written with a generation number in each.
        sync(&[]);
        let d = disk();
        let blk = fsbtodb(fs, ino_to_fsba(fs, 64)) as usize * 512;
        for i in 64..96 {
            let din = crate::fs::ufs::ufs::inode::dinode2_at(&d[blk..], ino_to_fsbo(fs, i));
            assert!(din.di_gen != 0, "inode {i}");
            assert_eq!(din.di_mode, 0);
        }
        ffs_freefile(&rip, 100, IFREG).unwrap();
        drop(rip);
        vput(rootvp);
    }

    #[test]
    fn freeing_a_free_inode_on_a_read_only_mount_is_only_reported() {
        let (_g, _th, mp) = setup(Image::new(FFS2_1G).finish());
        let rootvp = super::super::testffs::root(mp);
        let rip = vtoi(rootvp);
        let fs = rip.i_fs();
        fs.set_fs_ronly(1);
        let ni = fs.fs_cstotal_nifree();
        ffs_freefile(&rip, 7, IFREG).unwrap();
        // The C counts it free all the same.
        assert_eq!(fs.fs_cstotal_nifree(), ni + 1);
        drop(rip);
        vput(rootvp);
    }

    #[test]
    fn the_block_preferences_follow_the_c_policies() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let fs = ip.i_fs();
            let frag = Daddr::from(fs.fs_frag());
            let blkpref = |lbn: Daddr, indx: i32, bap: Option<&dyn Fn(usize) -> Daddr>| {
                #[cfg(feature = "ffs2")]
                if p.ufs2 {
                    return ffs2_blkpref(&ip, lbn, indx, bap);
                }
                ffs1_blkpref(&ip, lbn, indx, bap)
            };
            let none = |_: usize| 0;
            // The first data block: the data zone of the inode's group.
            assert_eq!(blkpref(0, 0, Some(&none)), cgdata(fs, 0));
            // Right after the previous block.
            let prev = |i: usize| if i == 4 { 1000 } else { 0 };
            assert_eq!(blkpref(5, 5, Some(&prev)), 1000 + frag);
            // No block before: the data zone again.
            assert_eq!(blkpref(5, 5, Some(&none)), cgdata(fs, 0));
            // An indirect block: the metadata zone, or after the last direct block.
            assert_eq!(blkpref(NDADDR as Daddr, -1, None), cgmeta(fs, 0));
            ip.dip_set_db(NDADDR - 1, 2000);
            assert_eq!(blkpref(NDADDR as Daddr, -1, None), 2000 + frag);
            // The first block under the first indirect, which is in the data zone: after it.
            ip.dip_set_ib(0, cgdata(fs, 0) + 40);
            assert_eq!(
                blkpref(NDADDR as Daddr, 0, Some(&none)),
                cgdata(fs, 0) + 40 + frag
            );
            // A directory's blocks: the metadata zone.
            ip.dip_set_mode(IFDIR | 0o755);
            assert_eq!(blkpref(1, 1, Some(&none)), cgmeta(fs, 0));
            // Past the first indirect's blocks with no block before: the sweep's group.
            ip.dip_set_mode(IFREG | 0o644);
            let lbn = NDADDR as Daddr + nindir(fs) + 1;
            let want = if p.ufs2 {
                cgbase(fs, 0) + frag
            } else {
                cgdata(fs, 0)
            };
            assert_eq!(blkpref(lbn, 1, Some(&none)), want);
            ip.dip_set_db(NDADDR - 1, 0);
            ip.dip_set_ib(0, 0);
            drop(ip);
            vput(vp);
        }
    }

    /// Cylinder group 0's cluster summary entry `i` and cluster map bit `blk`, through the
    /// cache.
    #[cfg(feature = "ffs2")]
    fn cluster<G: ProcGlue>(ip: &Inode<G>, i: usize, blk: usize) -> (i32, bool) {
        let mut t = ffs_cgread(ip.i_fs(), ip, 0).unwrap();
        let mut v = CgView::new(t.data());
        let r = (v.cg_clustersum(i), isset(v.cg_clustersfree(), blk));
        brelse(t);
        r
    }

    // FFS2 only: without `option FFS2` the kernel mounts no FFS2 image.
    #[cfg(feature = "ffs2")]
    #[test]
    fn the_cluster_summary_follows_block_allocations_and_frees() {
        let (_g, th, mp) =
            setup(Image::new(super::super::testffs::newfs::FFS2_1G_CLUSTERS).finish());
        let cred = Cred::real(th.ucred());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        let fs = ip.i_fs();
        let bsize = fs.fs_bsize();
        // newfs: one run of free blocks from 29 (after the root directory's) to the end,
        // counted as a run of fs_contigsumsize; fs_maxcluster starts at fs_contigsumsize.
        assert_eq!(fs.fs_contigsumsize(), 4);
        assert_eq!(fs.maxcluster(0), 4);
        assert_eq!(cluster(&ip, 4, 29), (1, true));
        // The run's first block: the run behind it is still long.
        let b1 = ffs_alloc(&ip, 0, 0, bsize, cred).unwrap();
        assert_eq!(b1, 29 * 8);
        assert_eq!(cluster(&ip, 4, 29), (1, false));
        // Block 32: the run splits into 30-31 (a run of 2) and 33 on (still 4).
        let b2 = ffs_alloc(&ip, 1, 32 * 8, bsize, cred).unwrap();
        assert_eq!(b2, 32 * 8);
        assert_eq!(cluster(&ip, 2, 32), (1, false));
        assert_eq!(cluster(&ip, 4, 33), (1, true));
        assert_eq!(fs.maxcluster(0), 4);
        check_incore(fs, true);
        // Both back: one run again.
        ffs_blkfree(&ip, b2, i64::from(bsize));
        assert_eq!(cluster(&ip, 2, 32), (0, true));
        ffs_blkfree(&ip, b1, i64::from(bsize));
        assert_eq!(cluster(&ip, 4, 29), (1, true));
        check_incore(fs, true);
        ip.dip_set_blocks(0);
        drop(ip);
        vput(vp);
    }

    /// A cylinder group block of eight blocks of eight fragments with a cluster summary of
    /// 4 and the cluster map's first byte `map` (summary and map at 168 and 188).
    fn cluster_block(map: u8, sums: [i32; 5]) -> Vec<u8> {
        let c = super::super::fs::Cg {
            cg_magic: super::super::fs::CG_MAGIC,
            cg_clustersumoff: 168,
            cg_clusteroff: 188,
            cg_nclusterblks: 8,
            ..Default::default()
        };
        let mut b = vec![0u8; 256];
        assert!(hal::user::pod_to_bytes(&c, &mut b));
        for (i, s) in sums.iter().enumerate() {
            b[168 + i * 4..172 + i * 4].copy_from_slice(&s.to_ne_bytes());
        }
        b[188] = map;
        b
    }

    /// An in-core super-block of one group of 64 fragments, 8 per block, and `contig`.
    fn small_fs(contig: i32) -> Superblock {
        let mut f = super::super::fs::Fs::zeroed();
        f.fs_ncg = 1;
        f.fs_frag = 8;
        f.fs_fragshift = 3;
        f.fs_fsize = 1024;
        f.fs_fpg = 64;
        f.fs_contigsumsize = contig;
        super::super::testffs::newfs::incore(&f)
    }

    #[test]
    fn clusteracct_counts_the_runs_and_lowers_maxcluster() {
        let fs = small_fs(4);
        // Blocks 0 and 1 free: one run of 2.
        let mut b = cluster_block(0b0000_0011, [0, 0, 1, 0, 0]);
        let mut v = CgView::new(&mut b);
        // Block 0 taken: the run of 2 becomes a run of 1, the longest now.
        ffs_clusteracct(&fs, &mut v, 0, -1);
        assert_eq!((v.cg_clustersum(1), v.cg_clustersum(2)), (1, 0));
        assert!(!isset(v.cg_clustersfree(), 0));
        assert_eq!(fs.maxcluster(0), 1);
        // Back: a run of 2 again.
        ffs_clusteracct(&fs, &mut v, 0, 1);
        assert_eq!((v.cg_clustersum(1), v.cg_clustersum(2)), (0, 1));
        assert_eq!(fs.maxcluster(0), 2);
        // Block 3 freed between 2 (taken) and 4..7 (free): a run of 5, counted as 4.
        drop(v);
        let mut b = cluster_block(0b1111_0000, [0, 0, 0, 0, 1]);
        let mut v = CgView::new(&mut b);
        ffs_clusteracct(&fs, &mut v, 3, 1);
        assert_eq!(
            v.cg_clustersum(4),
            1,
            "the run behind absorbed, capped at 4"
        );
        assert_eq!(fs.maxcluster(0), 4);
        // Without a cluster summary nothing is touched.
        let none = small_fs(0);
        let mut b = cluster_block(0, [0; 5]);
        let mut v = CgView::new(&mut b);
        ffs_clusteracct(&none, &mut v, 3, 1);
        assert!(!isset(v.cg_clustersfree(), 3));
    }

    #[test]
    fn mapsearch_finds_the_c_fragment_and_moves_frotor() {
        let fs = small_fs(0);
        // (free map's 8 bytes, bpref, allocsiz, the fragment found, cg_frotor after).
        let table: [([u8; 8], Daddr, i32, Daddr, u32); 5] = [
            // A run of 3 at fragments 18-20: found from the map's start.
            ([0, 0, 0b0001_1100, 0, 0, 0, 0, 0], 0, 3, 18, 16),
            // The same from a preference past it: the search wraps to the start.
            ([0, 0, 0b0001_1100, 0, 0, 0, 0, 0], 40, 3, 18, 16),
            // A whole block (byte 4 all free) for a block's request.
            ([0, 0, 0b0001_1100, 0, 0xff, 0, 0, 0], 0, 8, 32, 32),
            // A run of 1 among runs of 2: the run of 1 at fragment 62.
            ([0b0000_0110, 0, 0, 0, 0, 0, 0, 0b0100_0000], 0, 1, 62, 56),
            // The preference's byte first: the run of 2 at 49-50, not the one at 1-2.
            ([0b0000_0110, 0, 0, 0, 0, 0, 0b0000_0110, 0], 48, 2, 49, 48),
        ];
        for (map, bpref, allocsiz, want, frotor) in table {
            let mut b = vec![0u8; 256];
            let c = super::super::fs::Cg {
                cg_magic: super::super::fs::CG_MAGIC,
                cg_freeoff: 168,
                ..Default::default()
            };
            assert!(hal::user::pod_to_bytes(&c, &mut b));
            b[168..176].copy_from_slice(&map);
            let mut v = CgView::new(&mut b);
            assert_eq!(ffs_mapsearch(&fs, &mut v, bpref, allocsiz), want, "{map:?}");
            assert_eq!(v.cg_frotor(), frotor);
        }
    }

    std::thread_local! {
        static TRIED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }

    /// An allocator that finds nothing and records the groups it is asked.
    fn nothing(_: &Inode<crate::fs::testfs::TestGlue>, cg: u32, _: Daddr, _: i32) -> Daddr {
        TRIED.with(|t| t.borrow_mut().push(cg));
        0
    }

    #[test]
    fn hashalloc_tries_the_group_then_the_quadratic_rehash_then_each_group() {
        let p = Params {
            size: 8 << 20,
            ..FFS2_4M
        };
        let (_g, th, mp) = setup(Image::new(p).finish());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        assert_eq!(ip.i_fs().fs_ncg(), 4);
        TRIED.with(|t| t.borrow_mut().clear());
        assert_eq!(ffs_hashalloc(&ip, 1, 0, 0, nothing), 0);
        // 1; 1+1, 2+2-4; then from 1+2 the two groups the rehash may have missed.
        assert_eq!(TRIED.with(|t| t.borrow().clone()), vec![1, 2, 0, 3, 0]);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn a_cylinder_group_without_its_magic_is_not_read() {
        let mut img = Image::new(FFS2_1G).finish();
        // Cylinder group 0's magic, zeroed.
        let fs = super_block(&img, true);
        let cg0 = ((Daddr::from(fs.fs_cblkno)) << fs.fs_fsbtodb) as usize * 512;
        img[cg0 + 4..cg0 + 8].fill(0);
        let (_g, _th, mp) = setup(img);
        let rootvp = super::super::testffs::root(mp);
        let rip = vtoi(rootvp);
        assert!(ffs_cgread(rip.i_fs(), &rip, 0).is_none());
        assert_eq!(ffs_alloccg(&rip, 0, 0, fs.fs_bsize), 0);
        drop(rip);
        vput(rootvp);
    }
}
/* </TESTS> */
