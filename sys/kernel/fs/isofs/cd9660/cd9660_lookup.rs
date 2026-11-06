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
 * Copyright (c) 1989, 1993, 1994
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley
 * by Pace Willisson (pace@blitz.com).  The Rock Ridge Extension
 * Support code is derived from software contributed to Berkeley
 * by Atsushi Murai (amurai@spec.co.jp).
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
 *	from: @(#)ufs_lookup.c	7.33 (Berkeley) 5/19/91
 *
 *	@(#)cd9660_lookup.c	8.5 (Berkeley) 12/5/94
 */
/* </LICENSES> */

/* <CODE> */
//! Directory lookup on ISO 9660: `cd9660_lookup` (convert a component of a pathname into a
//! locked vnode, by a linear scan of the directory's records) and `cd9660_bufatoff` (the
//! buffer of a directory block).
//!
//! Upstream: sys/isofs/cd9660/cd9660_lookup.c @ 3ce1f3f79392
//! LZ: sys/isofs/cd9660/cd9660_lookup.rs@3c62ede68802
//!
//! ## Deviations
//! - The C's `goto`s between `searchloop`, `foundino`, `notfound` and `found` are a loop
//!   over the passes and a labelled block whose value says where the scan ended.
//! - A `struct iso_directory_record *` into the directory buffer is its offset in the
//!   buffer, viewed as an [`IsoDirectoryRecord`].
//! - `cd9660_bufatoff` returns the buffer and the offset of `offset` in it (the C's `res`)
//!   instead of filling `*bpp` and `*res`.
//! - The Rock Ridge name buffer is a `NAME_MAX + 1` byte array on the stack instead of a
//!   `NAME_MAX` byte `malloc(M_TEMP)`: the name translation of a record without an `NM`
//!   entry can be one byte longer than `NAME_MAX` (the associated-file prefix), which the C
//!   would write past its buffer. So `vmstat -m`'s `temp` row counts one request fewer per
//!   Rock Ridge record scanned (LZ's deviation).
//! - `NOSORTBUG` is not defined, as in GENERIC.
//! - Until N5.8c2 brings `cd9660_vfsops.rs`, `cd9660_lookup` takes `cd9660_vget_internal` as
//!   its argument `vget_internal` (the one tie of the C's lookup to the mount code: the cycle
//!   `cd9660_vget_internal` -> `cd9660_rrip_analyze` -> `cd9660_bufatoff` -> lookup's).
//!
//! ## Redesign
//! - LZ's lent directory buffers (two `unsafe` views of `b_data`, the buffer held as
//!   `&'static Buf`) -> `fs::bio`'s `BufRef` tokens: the directory's token is held across
//!   the record's Rock Ridge name (`cd9660_rrip_getname`, whose continuation reads are of the
//!   device's vnode, another buffer) and across `cd9660_vget_internal`'s use of the record, as
//!   the C holds `bp`, and released where the C releases it; `cd9660_bufatoff` is
//!   `bread_or_release` of the directory's vnode.
//! - The directory node is `vtoi`'s counted handle (a count pair per lookup, ROADMAP N12's
//!   row), its offsets words; `iso_nchstats` is a static of the kernel's `Nchstats`. 0
//!   `unsafe` (LZ: 2).

use core::sync::atomic::Ordering;

use hal::sys::errno::Errno;
use hal::sys::mount::MNT_RDONLY;
use hal::sys::syslimits::NAME_MAX;
use hal::sys::types::Off;

use crate::assert::kpanic;
use crate::fs::bio::{DEV_BSHIFT, bread_or_release, brelse};
use crate::fs::buf::BufRef;
use crate::fs::cache::{Nchstats, cache_enter, cache_lookup};
use crate::fs::isofs::cd9660::cd9660_extern::{ISO_FTYPE_RRIP, blkoff, blksize, lblkno};
use crate::fs::isofs::cd9660::cd9660_node::{Doff, IsoNode, isodirino, vtoi};
use crate::fs::isofs::cd9660::cd9660_rrip::cd9660_rrip_getname;
use crate::fs::isofs::cd9660::cd9660_util::isofncmp;
use crate::fs::isofs::cd9660::iso::{
    ASSOCCHAR, Cdino, ISO_DIRECTORY_RECORD_SIZE, IsoDirectoryRecord, isonum_711,
};
use crate::fs::mount::Mount;
use crate::fs::namei::{
    CREATE, Componentname, DELETE, ISDOTDOT, ISLASTCN, LOCKPARENT, LOOKUP, MAKEENTRY, PDIRUNLOCK,
    RENAME,
};
use crate::fs::subr::{vput, vref};
use crate::fs::vnode::{LK_EXCLUSIVE, LK_RETRY, VEXEC, Vnode, vn_lock};
use crate::fs::vops::{vop_access, vop_unlock};
use crate::proc::ProcGlue;

/// `cd9660_vget_internal(mp, ino, vpp, relocated, isodir)` (`cd9660_vfsops.c`, N5.8c2): the
/// locked vnode of the file `ino` of `mp`, from the record `isodir` when the caller holds it.
pub type VgetInternal<G> = fn(
    &'static Mount<G>,
    Cdino,
    bool,
    Option<IsoDirectoryRecord<'_>>,
) -> Result<&'static Vnode<G>, Errno>;

/// Where the scan of a directory ended.
enum Scan {
    /// `found`: the entry at the given offset in the buffer is the one.
    Found(usize),
    /// `foundino`: a match was seen (`ino`, at `saveoffset`), the scan has gone past it.
    FoundIno(usize),
    /// `notfound`.
    NotFound,
}

/// `iso_nchstats`.
pub static ISO_NCHSTATS: Nchstats = Nchstats::new();

/// The vnode's mount, which a cd9660 vnode always has.
fn vmount<G: ProcGlue>(vp: &Vnode<G>) -> &'static Mount<G> {
    match vp.mount() {
        Some(mp) => mp,
        None => kpanic(format_args!("cd9660: vnode {vp:p} without a mount")),
    }
}

/// The token a scan holds, which it holds whenever it reads a record (the C's `bp`).
fn held(bp: Option<&BufRef>) -> &BufRef {
    match bp {
        Some(t) => t,
        None => kpanic(format_args!("cd9660_lookup: no directory buffer")),
    }
}

/// `dbtob(bp->b_blkno) + entryoffsetinblock`: the inode number of a file's record, its byte
/// offset on the disc (the `daddr_t` sum stored into the `cdino_t`).
fn record_ino(t: &BufRef, entryoffsetinblock: usize) -> Cdino {
    (t.buf().blkno() << DEV_BSHIFT).wrapping_add(entryoffsetinblock as i64) as Cdino
}

/// `cd9660_lookup` (`vop_lookup`): convert a component of a pathname into a pointer to a
/// locked inode. This is a very central and rather complicated routine. If the file system
/// is not maintained in a strict tree hierarchy, this can result in a deadlock situation
/// (see comments in code below).
///
/// The flag argument is `LOOKUP`, `CREATE`, `RENAME`, or `DELETE` depending on whether the
/// name is to be looked up, created, renamed, or deleted. When `CREATE`, `RENAME`, or
/// `DELETE` is specified, information usable in creating, renaming, or deleting a directory
/// entry may be calculated. If flag has `LOCKPARENT` or'ed into it and the target of the
/// pathname exists, lookup returns both the target and its parent directory locked. When
/// creating or renaming and `LOCKPARENT` is specified, the target may not be ".". When
/// deleting and `LOCKPARENT` is specified, the target may be ".", but the caller must check
/// to ensure it does an `vrele` and `iput` instead of two `iput`s.
///
/// Overall outline of `cd9660_lookup`:
/// - check accessibility of directory
/// - look for name in cache, if found, then if at end of path and deleting or creating,
///   drop it, else return name
/// - search for name in directory, to found or notfound
/// - notfound: if creating, return locked directory, leaving info on available slots, else
///   return error
/// - found: if at end of path and deleting, return information to allow delete; if at end
///   of path and rewriting (`RENAME` and `LOCKPARENT`), lock target inode and return info to
///   allow rewrite; if not at end, add name to cache; if at end and neither creating nor
///   deleting, add name to cache
///
/// NOTE: (`LOOKUP | LOCKPARENT`) currently returns the parent inode unlocked.
pub fn cd9660_lookup<G: ProcGlue>(
    vdp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Componentname<'_, G>,
    vget_internal: VgetInternal<G>,
) -> Result<(), Errno> {
    let nameiop = cnp.cn_nameiop;

    cnp.cn_flags &= !PDIRUNLOCK;
    let flags = cnp.cn_flags;

    let mut bp: Option<BufRef> = None;
    *vpp = None;
    let dp = vtoi(vdp);
    let imp = dp.mnt();
    let lockparent = flags & LOCKPARENT != 0;

    // Check accessibility of directory.
    vop_access(vdp, VEXEC, cnp.cn_cred, cnp.proc())?;

    if flags & ISLASTCN != 0
        && vmount(vdp).flag() & MNT_RDONLY != 0
        && (nameiop == DELETE || nameiop == RENAME)
    {
        return Err(Errno::EROFS);
    }

    // We now have a segment name to search for, and a directory to search.
    //
    // Before tediously performing a linear scan of the directory, check the name cache to
    // see if the directory/name pair we are looking for is known already.
    if let Some(vp) = cache_lookup(vdp, cnp)? {
        *vpp = Some(vp);
        return Ok(());
    }

    let mut name = cnp.name();
    // A leading `=' means, we are looking for an associated file
    let assoc = imp.iso_ftype != ISO_FTYPE_RRIP && name.first() == Some(&ASSOCCHAR);
    if assoc {
        name = &name[1..];
    }

    // If there is cached information on a previous search of this directory, pick up where
    // we last left off. We cache only lookups as these are the most common and have the
    // greatest payoff. Caching CREATE has little benefit as it usually must search the entire
    // directory to determine that the entry does not exist. Caching the location of the last
    // DELETE or RENAME has not reduced profiling time and hence has been removed in the
    // interest of simplicity.
    let bmask = imp.im_bmask as Doff;
    let lbs = imp.logical_block_size as usize;
    let mut entryoffsetinblock: usize;
    let mut numdirpasses;
    if nameiop != LOOKUP || dp.i_diroff.get() == 0 || dp.i_diroff.get() > dp.i_size.get() {
        entryoffsetinblock = 0;
        dp.i_offset.set(0);
        numdirpasses = 1;
    } else {
        dp.i_offset.set(dp.i_diroff.get());
        entryoffsetinblock = (dp.i_offset.get() & bmask) as usize;
        if entryoffsetinblock != 0 {
            let (t, _) = cd9660_bufatoff(&dp, dp.i_offset.get() as Off)?;
            bp = Some(t);
        }
        numdirpasses = 2;
        ISO_NCHSTATS.ncs_2passes.fetch_add(1, Ordering::Relaxed);
    }
    let mut endsearch = dp.i_size.get();
    let mut saveoffset: Doff = Doff::MAX; // the C's int -1
    let mut ino: Cdino = 0;

    // The record that was found: its offset in `bp`.
    let ep: usize = 'searchloop: loop {
        let scan = 'scan: {
            while dp.i_offset.get() < endsearch {
                // If offset is on a block boundary, read the next directory block. Release
                // previous if it exists.
                if dp.i_offset.get() & bmask == 0 {
                    if let Some(t) = bp.take() {
                        brelse(t);
                    }
                    let (t, _) = cd9660_bufatoff(&dp, dp.i_offset.get() as Off)?;
                    bp = Some(t);
                    entryoffsetinblock = 0;
                }
                let t = held(bp.as_ref());
                let data = t.bytes();

                // Get pointer to next entry.
                let reclen = usize::from(data.get(entryoffsetinblock).copied().unwrap_or(0));
                if reclen == 0 {
                    // skip to next block, if any
                    dp.i_offset
                        .set((dp.i_offset.get() & !bmask) + imp.logical_block_size as Doff);
                    continue;
                }

                if reclen < ISO_DIRECTORY_RECORD_SIZE {
                    // illegal entry, stop
                    break;
                }

                if entryoffsetinblock + reclen > lbs {
                    // entries are not allowed to cross boundaries
                    break;
                }

                let Some(rec) = data
                    .get(entryoffsetinblock..)
                    .and_then(IsoDirectoryRecord::new)
                else {
                    break;
                };
                let namelen = usize::from(isonum_711(rec.name_len()));

                if reclen < ISO_DIRECTORY_RECORD_SIZE + namelen {
                    // illegal entry, stop
                    break;
                }

                // Check for a name match.
                if imp.iso_ftype == ISO_FTYPE_RRIP {
                    ino = if isonum_711(rec.flags()) & 2 != 0 {
                        isodirino(&rec, imp)
                    } else {
                        record_ino(t, entryoffsetinblock)
                    };
                    let mut i_ino = ino;
                    let mut altname = [0u8; NAME_MAX + 1];
                    let mut altlen = 0u16;
                    // The directory's block is the directory vnode's buffer: no read of the
                    // device waits for it (no held block, `cd9660_rrip.rs`'s deviations).
                    cd9660_rrip_getname(&rec, &mut altname, &mut altlen, &mut i_ino, imp, None);
                    dp.i_ino.set(i_ino);
                    let altlen = usize::from(altlen).min(altname.len());
                    if altlen as i64 == cnp.namelen() && name == &altname[..altlen] {
                        break 'scan Scan::Found(entryoffsetinblock);
                    }
                    ino = 0;
                } else if (isonum_711(rec.flags()) & 4 != 0) == assoc {
                    if (name.len() == 1 && name[0] == b'.') || flags & ISDOTDOT != 0 {
                        let want = u8::from(flags & ISDOTDOT != 0);
                        if namelen == 1 && rec.name0() == want {
                            // Save directory entry's inode number and release directory
                            // buffer.
                            dp.i_ino.set(isodirino(&rec, imp));
                            break 'scan Scan::Found(entryoffsetinblock);
                        }
                        if namelen != 1 || rec.name0() != 0 {
                            break 'scan Scan::NotFound;
                        }
                    } else if isofncmp(name, &rec.name()[..namelen], imp.joliet_level) == 0 {
                        ino = if isonum_711(rec.flags()) & 2 != 0 {
                            isodirino(&rec, imp)
                        } else {
                            record_ino(t, entryoffsetinblock)
                        };
                        saveoffset = dp.i_offset.get();
                    } else if ino != 0 {
                        break 'scan Scan::FoundIno(entryoffsetinblock);
                    }
                    // NOSORTBUG (not defined): on some CDs directory entries are not sorted
                    // correctly, a res < 0 would go to notfound and a res > 0 on the second
                    // pass would add a third.
                }
                dp.i_offset.set(dp.i_offset.get() + reclen as Doff);
                entryoffsetinblock += reclen;
            }
            if ino != 0 {
                Scan::FoundIno(entryoffsetinblock)
            } else {
                Scan::NotFound
            }
        };

        match scan {
            Scan::Found(ep) => break 'searchloop ep,
            Scan::FoundIno(mut ep) => {
                dp.i_ino.set(ino);
                if saveoffset != dp.i_offset.get() {
                    if lblkno(imp, dp.i_offset.get() as Off) != lblkno(imp, saveoffset as Off) {
                        if let Some(t) = bp.take() {
                            brelse(t);
                        }
                        let (t, _) = cd9660_bufatoff(&dp, saveoffset as Off)?;
                        bp = Some(t);
                    }
                    entryoffsetinblock = (saveoffset & bmask) as usize;
                    ep = entryoffsetinblock;
                    dp.i_offset.set(saveoffset);
                }
                break 'searchloop ep;
            }
            Scan::NotFound => {
                // If we started in the middle of the directory and failed to find our
                // target, we must check the beginning as well.
                if numdirpasses == 2 {
                    numdirpasses -= 1;
                    dp.i_offset.set(0);
                    endsearch = dp.i_diroff.get();
                    continue 'searchloop;
                }
                if let Some(t) = bp {
                    brelse(t);
                }

                // Insert name into cache (as non-existent) if appropriate.
                if cnp.cn_flags & MAKEENTRY != 0 {
                    cache_enter(vdp, *vpp, cnp.name());
                }
                if nameiop == CREATE || nameiop == RENAME {
                    return Err(Errno::EJUSTRETURN);
                }
                return Err(Errno::ENOENT);
            }
        }
    };

    // found:
    if numdirpasses == 2 {
        ISO_NCHSTATS.ncs_pass2.fetch_add(1, Ordering::Relaxed);
    }

    // Found component in pathname. If the final component of path name, save information
    // in the cache as to where the entry was found.
    if flags & ISLASTCN != 0 && nameiop == LOOKUP {
        dp.i_diroff.set(dp.i_offset.get());
    }

    // Step through the translation in the name. We do not `iput' the directory because we
    // may need it again if a symbolic link is relative to the current directory. Instead we
    // save it unlocked as "pdp". We must get the target inode before unlocking the directory
    // to insure that the inode will not be removed before we get it. We prevent deadlock by
    // always fetching inodes from the root, moving down the directory tree. Thus when
    // following backward pointers ".." we must unlock the parent directory before getting
    // the requested directory. There is a potential race condition here if both the current
    // and parent directories are removed before the `iget' for the inode associated with
    // ".." returns. We hope that this occurs infrequently since we cannot avoid this race
    // condition without implementing a sophisticated deadlock detection algorithm. Note also
    // that this simple deadlock detection scheme will not work if the file system has any
    // hard links other than ".." that point backwards in the directory structure.
    let pdp = vdp;
    // If ino is different from dp->i_ino, it's a relocated directory.
    let relocated = dp.i_ino.get() != ino;
    let target = dp.i_ino.get();
    if flags & ISDOTDOT != 0 {
        if let Some(t) = bp.take() {
            brelse(t);
        }
        drop(dp);
        let _ = vop_unlock(pdp); // race to get the inode
        cnp.cn_flags |= PDIRUNLOCK;
        let tdp = match vget_internal(vmount(vdp), target, relocated, None) {
            Ok(tdp) => tdp,
            Err(e) => {
                if vn_lock(pdp, LK_EXCLUSIVE | LK_RETRY).is_ok() {
                    cnp.cn_flags &= !PDIRUNLOCK;
                }
                return Err(e);
            }
        };
        if lockparent && flags & ISLASTCN != 0 {
            if let Err(e) = vn_lock(pdp, LK_EXCLUSIVE) {
                vput(tdp);
                return Err(e);
            }
            cnp.cn_flags &= !PDIRUNLOCK;
        }
        *vpp = Some(tdp);
    } else if dp.i_number == target {
        if let Some(t) = bp.take() {
            brelse(t);
        }
        vref(vdp); // we want ourself, ie "."
        *vpp = Some(vdp);
    } else {
        let t = held(bp.as_ref());
        let rec = t.bytes().get(ep..).and_then(IsoDirectoryRecord::new);
        let error = vget_internal(vmount(vdp), target, relocated, rec);
        if let Some(t) = bp.take() {
            brelse(t);
        }
        let tdp = error?;
        if !lockparent || flags & ISLASTCN == 0 {
            let _ = vop_unlock(pdp);
            cnp.cn_flags |= PDIRUNLOCK;
        }
        *vpp = Some(tdp);
    }

    // Insert name into cache if appropriate.
    if cnp.cn_flags & MAKEENTRY != 0 {
        cache_enter(vdp, *vpp, cnp.name());
    }
    Ok(())
}

/// `cd9660_bufatoff`: return buffer with the contents of block `offset` from the beginning
/// of directory `ip`, and the offset of `offset` in it (the C's `res`, a pointer to the
/// remaining space in the directory).
pub fn cd9660_bufatoff<G: ProcGlue>(
    ip: &IsoNode<G>,
    offset: Off,
) -> Result<(BufRef, usize), Errno> {
    let vp = ip.itov();
    let imp = ip.mnt();
    let lbn = lblkno(imp, offset);
    let bsize = blksize(imp);

    let t = bread_or_release(&vp.bufs, lbn, bsize)?;
    Ok((t, blkoff(imp, offset) as usize))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the directory scan over the makefs image's root and `sub/` on testffs's
    // disk, read through a directory vnode of testcd9660's table: Rock Ridge and plain ISO
    // 9660 names, `.`, `..`, a missing name, the second pass, the relocated test, and the
    // record handed to `cd9660_vget_internal` (a recording double until N5.8c2).

    use std::cell::RefCell;
    use std::vec::Vec;

    use super::*;
    use crate::fs::isofs::cd9660::cd9660_extern::{
        ISO_FTYPE_DEFAULT, ISO_FTYPE_RRIP, IsoFtype, IsoMntRef,
    };
    use crate::fs::isofs::cd9660::cd9660_node::IsoNodeRef;
    use crate::fs::isofs::cd9660::testcd9660::{attach_image, image, test_mnt, test_node};
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ufs::testufs::cn;
    use crate::fs::vops::vop_lock;
    use crate::proc::Thread;

    std::thread_local! {
        /// What the double of `cd9660_vget_internal` was asked: the inode number, the
        /// relocated flag, and the record's name when one was given.
        static ASKED: RefCell<Vec<(Cdino, bool, Option<Vec<u8>>)>> = const { RefCell::new(Vec::new()) };
        /// The mount the double makes its nodes on.
        static MNT: RefCell<Option<IsoMntRef<TestGlue>>> = const { RefCell::new(None) };
    }

    /// The double of `cd9660_vget_internal`: records the call, answers a new locked vnode.
    fn vget_double(
        _mp: &'static Mount<TestGlue>,
        ino: Cdino,
        relocated: bool,
        isodir: Option<IsoDirectoryRecord<'_>>,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        let name = isodir.map(|r| r.name()[..usize::from(isonum_711(r.name_len()))].to_vec());
        ASKED.with(|a| a.borrow_mut().push((ino, relocated, name)));
        let imp = MNT.with(|m| m.borrow().clone()).expect("a mount");
        let (_ip, vp) = test_node(&imp, ino);
        vop_lock(vp, LK_EXCLUSIVE).expect("locked");
        Ok(vp)
    }

    /// The image's root directory as a locked vnode of a mount of `ftype`, and the thread.
    fn root(
        ftype: IsoFtype,
    ) -> (
        std::sync::MutexGuard<'static, ()>,
        &'static Thread<TestGlue>,
        IsoNodeRef<TestGlue>,
        &'static Vnode<TestGlue>,
    ) {
        let (g, p) = attach_image(image::image_bytes());
        let imp = test_mnt(ftype, |_| {});
        MNT.with(|m| *m.borrow_mut() = Some(imp.clone()));
        ASKED.with(|a| a.borrow_mut().clear());
        let (ip, vp) = test_node(&imp, 20 << 11);
        ip.iso_extent.set(20);
        ip.iso_start.set(20);
        ip.i_size.set(2048);
        vop_lock(vp, LK_EXCLUSIVE).expect("locked");
        (g, p, ip, vp)
    }

    /// `cd9660_lookup` of `name` in `dvp`: the result and the vnode.
    fn lookup(
        p: &'static Thread<TestGlue>,
        dvp: &'static Vnode<TestGlue>,
        op: u64,
        flags: u64,
        name: &'static [u8],
    ) -> (Result<(), Errno>, Option<&'static Vnode<TestGlue>>, u64) {
        let cred = p.ucred();
        let mut c = cn(p, cred, op, LOCKPARENT | flags, name);
        let mut vpp = None;
        let r = cd9660_lookup(dvp, &mut vpp, &mut c, vget_double);
        (r, vpp, c.cn_flags)
    }

    /// The double's calls.
    fn asked() -> Vec<(Cdino, bool, Option<Vec<u8>>)> {
        ASKED.with(|a| a.borrow().clone())
    }

    #[test]
    fn rock_ridge_names_find_their_records() {
        let (_g, p, dp, dvp) = root(ISO_FTYPE_RRIP);
        // a file: its inode number is its record's byte offset (block 20, byte 324)
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"m10c-iso.txt");
        r.expect("found");
        assert_eq!(
            asked(),
            [(20 * 2048 + 324, false, Some(b"M10C_ISO.TXT;1".to_vec()))]
        );
        // the last component of a LOOKUP remembers where it was found
        assert_eq!(dp.i_diroff.get(), 324);
        vput(vp.expect("a vnode"));
        // a directory: its inode number is its data's byte offset
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"sub");
        r.expect("found");
        assert_eq!(asked()[1], (21 << 11, false, Some(b"SUB".to_vec())));
        vput(vp.expect("a vnode"));
        // the ISO 9660 name is no Rock Ridge name
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"M10C_ISO.TXT;1");
        assert_eq!(r, Err(Errno::ENOENT));
        assert!(vp.is_none());
        // a CREATE of a missing name answers EJUSTRETURN
        assert_eq!(lookup(p, dvp, CREATE, 0, b"new").0, Err(Errno::EJUSTRETURN));
        assert_eq!(asked().len(), 2);
    }

    #[test]
    fn dot_is_the_directory_itself() {
        let (_g, p, _dp, dvp) = root(ISO_FTYPE_RRIP);
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b".");
        r.expect("found");
        assert!(vp.is_some_and(|v| core::ptr::eq(v, dvp)));
        assert!(asked().is_empty());
        crate::fs::subr::vrele(dvp);
    }

    #[test]
    fn dotdot_unlocks_the_parent_and_asks_for_no_record() {
        let (_g, p, dp, dvp) = root(ISO_FTYPE_DEFAULT);
        let (r, vp, flags) = lookup(p, dvp, LOOKUP, ISDOTDOT, b"..");
        r.expect("found");
        // the root's parent is the root: its data's offset, the record not handed over
        assert_eq!(asked(), [(20 << 11, true, None)]);
        // LOCKPARENT at the last component locks the parent again
        assert_eq!(flags & PDIRUNLOCK, 0);
        assert_ne!(dp.i_lock.status(), 0);
        vput(vp.expect("a vnode"));
    }

    #[test]
    fn plain_names_match_without_case_or_version() {
        let (_g, p, _dp, dvp) = root(ISO_FTYPE_DEFAULT);
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"m10c_iso.txt");
        r.expect("found");
        // found past its record (`foundino`): the record handed over is the saved one
        assert_eq!(
            asked(),
            [(20 * 2048 + 324, false, Some(b"M10C_ISO.TXT;1".to_vec()))]
        );

        vput(vp.expect("a vnode"));
        assert_eq!(
            lookup(p, dvp, LOOKUP, 0, b"m10c-iso.txt").0,
            Err(Errno::ENOENT)
        );
    }

    #[test]
    fn a_lookup_from_the_last_offset_wraps_to_the_start() {
        let (_g, p, dp, dvp) = root(ISO_FTYPE_RRIP);
        // the last lookup ended at the file's record: "sub" comes after it, the second pass
        // finds nothing new; a name before it needs the second pass
        dp.i_diroff.set(324);
        let passes = ISO_NCHSTATS.ncs_2passes.load(Ordering::Relaxed);
        let pass2 = ISO_NCHSTATS.ncs_pass2.load(Ordering::Relaxed);
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"sub");
        r.expect("found");
        vput(vp.expect("a vnode"));
        assert!(ISO_NCHSTATS.ncs_2passes.load(Ordering::Relaxed) > passes);
        assert!(ISO_NCHSTATS.ncs_pass2.load(Ordering::Relaxed) > pass2);
        dp.i_diroff.set(452);
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"m10c-iso.txt");
        r.expect("found on the second pass");
        vput(vp.expect("a vnode"));
        assert_eq!(asked()[1].0, 20 * 2048 + 324);
    }

    #[test]
    fn a_read_only_mount_refuses_a_delete_or_rename() {
        let (_g, p, _dp, dvp) = root(ISO_FTYPE_RRIP);
        let mp = dvp.mount().expect("a mount");
        mp.set_flag(mp.flag() | MNT_RDONLY);
        assert_eq!(
            lookup(p, dvp, DELETE, 0, b"m10c-iso.txt").0,
            Err(Errno::EROFS)
        );
        assert_eq!(
            lookup(p, dvp, RENAME, 0, b"m10c-iso.txt").0,
            Err(Errno::EROFS)
        );
        assert!(asked().is_empty());
    }

    #[test]
    fn bufatoff_reads_the_directory_block_of_an_offset() {
        let (_g, _p, dp, _dvp) = root(ISO_FTYPE_RRIP);
        let (t, off) = cd9660_bufatoff(&dp, 324).expect("read");
        assert_eq!(off, 324);
        assert_eq!(&t.bytes()[324..324 + 128], image::FILE_REC);
        // the directory's block 0 is the device's block 20
        assert_eq!(t.buf().blkno(), 20 << 2);
        brelse(t);
    }
}
/* </TESTS> */
