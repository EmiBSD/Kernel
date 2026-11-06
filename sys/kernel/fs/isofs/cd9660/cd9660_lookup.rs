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
//!
//! ## Redesign
//! - LZ's lent directory buffers (two `unsafe` views of `b_data`, the buffer held as
//!   `&'static Buf`) -> `fs::bio`'s `BufRef` tokens: the directory's token is held across
//!   the record's Rock Ridge name (`cd9660_rrip_getname`, whose continuation reads are of the
//!   device's vnode, another buffer) and across `cd9660_vget_internal`'s use of the record, as
//!   the C holds `bp`, and released where the C releases it; `cd9660_bufatoff` is
//!   `bread_or_release` of the directory's vnode.
//! - The directory node is `vtoi`'s counted handle (a count pair per lookup, ROADMAP N12's
//!   row), its offsets words; `iso_nchstats` is a static of the kernel's `Nchstats`; LZ's
//!   private `vmount` is the UFS layer's shared one (`ufs_lookup.rs`, msdosfs's too). 0
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
use crate::fs::isofs::cd9660::cd9660_vfsops::cd9660_vget_internal;
use crate::fs::isofs::cd9660::iso::{
    ASSOCCHAR, Cdino, ISO_DIRECTORY_RECORD_SIZE, IsoDirectoryRecord, isonum_711,
};
use crate::fs::namei::{
    CREATE, Componentname, DELETE, ISDOTDOT, ISLASTCN, LOCKPARENT, LOOKUP, MAKEENTRY, PDIRUNLOCK,
    RENAME,
};
use crate::fs::subr::{vput, vref};
use crate::fs::ufs::ufs::ufs_lookup::vmount;
use crate::fs::vnode::{LK_EXCLUSIVE, LK_RETRY, VEXEC, Vnode, vn_lock};
use crate::fs::vops::{vop_access, vop_unlock};
use crate::proc::ProcGlue;

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

/// The token a scan holds, which it holds whenever it reads a record (the C's `bp`).
fn held(bp: Option<&BufRef>) -> &BufRef {
    match bp {
        Some(t) => t,
        None => kpanic(format_args!("cd9660_lookup: no directory buffer")),
    }
}

/// `dbtob(bp->b_blkno) + entryoffsetinblock`: the inode number of a file's record, its byte
/// offset on the disc (the `daddr_t` sum stored into the `cdino_t`).
pub(crate) fn record_ino(t: &BufRef, entryoffsetinblock: usize) -> Cdino {
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
        let tdp = match cd9660_vget_internal(vmount(vdp), target, relocated, None) {
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
        let error = cd9660_vget_internal(vmount(vdp), target, relocated, rec);
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
    // Host tests for the directory scan over real mounts (testcd9660: the makefs image
    // mounted by `iso_mountfs` with and without Rock Ridge, mkiso's directory of two blocks),
    // the vnodes `cd9660_vget_internal` gives for what the scan found: Rock Ridge and plain
    // ISO 9660 names, `.`, `..`, a missing name, the second pass, a read-only mount's
    // refusals, a match the scan went past into the next block (`foundino`'s re-read of the
    // saved block, N5.8c1's review), and `cd9660_bufatoff`.

    use std::sync::MutexGuard;
    use std::vec::Vec;

    use hal::sys::mount::ISOFSMNT_NORRIP;

    use super::*;
    use crate::fs::isofs::cd9660::cd9660_node::vtoi;
    use crate::fs::isofs::cd9660::testcd9660::mkiso::{self, Iso, record};
    use crate::fs::isofs::cd9660::testcd9660::{attach_image, image, mount_root, root};
    use crate::fs::mount::Mount;
    use crate::fs::subr::vrele;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs::unmount_root;
    use crate::fs::ufs::ufs::testufs::cn;
    use crate::fs::vnode::VDIR;
    use crate::proc::Thread;

    /// `image` mounted as the root with the mount arguments' `flags`, its root directory's
    /// vnode locked and referenced.
    fn setup(
        image: Vec<u8>,
        flags: i32,
    ) -> (
        MutexGuard<'static, ()>,
        &'static Thread<TestGlue>,
        &'static Mount<TestGlue>,
        &'static Vnode<TestGlue>,
    ) {
        let (g, p) = attach_image(image);
        let (mp, _) = mount_root(p, flags);
        (g, p, mp, root(mp))
    }

    /// `cd9660_lookup` of `name` in `dvp` (`LOCKPARENT`, the last component): the result,
    /// the vnode and the component's flags.
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
        let r = cd9660_lookup(dvp, &mut vpp, &mut c);
        (r, vpp, c.cn_flags)
    }

    /// What `cd9660_vget_internal` read for a vnode: the node's inode number, extent and
    /// size.
    fn ident(vp: &'static Vnode<TestGlue>) -> (Cdino, u64, u64) {
        let ip = vtoi(vp);
        (ip.i_number, ip.iso_extent.get(), ip.i_size.get())
    }

    #[test]
    fn rock_ridge_names_find_their_records() {
        let (_g, p, mp, dvp) = setup(image::image_bytes(), 0);
        let dp = vtoi(dvp);
        // a file: its inode number is its record's byte offset (block 20, byte 324)
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"m10c-iso.txt");
        r.expect("found");
        let vp = vp.expect("a vnode");
        assert_eq!(ident(vp), (20 * 2048 + 324, 23, 12));
        // the last component of a LOOKUP remembers where it was found
        assert_eq!(dp.i_diroff.get(), 324);
        vput(vp);
        // a directory: its inode number is its data's byte offset
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"sub");
        r.expect("found");
        let vp = vp.expect("a vnode");
        assert_eq!(ident(vp), (21 << 11, 21, 2048));
        assert_eq!(vp.vtype(), VDIR);
        vput(vp);
        // the ISO 9660 name is no Rock Ridge name
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"M10C_ISO.TXT;1");
        assert_eq!(r, Err(Errno::ENOENT));
        assert!(vp.is_none());
        // a CREATE of a missing name answers EJUSTRETURN
        assert_eq!(lookup(p, dvp, CREATE, 0, b"new").0, Err(Errno::EJUSTRETURN));
        drop(dp);
        vput(dvp);
        unmount_root(p, mp);
    }

    #[test]
    fn dot_is_the_directory_itself() {
        let (_g, p, _mp, dvp) = setup(image::image_bytes(), 0);
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b".");
        r.expect("found");
        assert!(vp.is_some_and(|v| core::ptr::eq(v, dvp)));
        vrele(dvp);
        vput(dvp);
    }

    #[test]
    fn dotdot_unlocks_the_directory_and_finds_the_parent() {
        let (_g, p, _mp, dvp) = setup(image::image_bytes(), ISOFSMNT_NORRIP);
        let (r, sub, _) = lookup(p, dvp, LOOKUP, 0, b"SUB");
        r.expect("found");
        let sub = sub.expect("a vnode");
        // the root unlocked, as namei leaves a parent it is done with
        let _ = vop_unlock(dvp);
        let (r, vp, flags) = lookup(p, sub, LOOKUP, ISDOTDOT, b"..");
        r.expect("found");
        // `..` is the root's vnode, from the node cache (a relocated lookup: the scan's `ino`
        // is 0 for `..`)
        let vp = vp.expect("a vnode");
        assert!(core::ptr::eq(vp, dvp));
        // LOCKPARENT at the last component locks the directory again
        assert_eq!(flags & PDIRUNLOCK, 0);
        assert_ne!(vtoi(sub).i_lock.status(), 0);
        vput(vp);
        vput(sub);
        vrele(dvp);
    }

    #[test]
    fn plain_names_match_without_case_or_version() {
        let (_g, p, _mp, dvp) = setup(image::image_bytes(), ISOFSMNT_NORRIP);
        // found past its record (`foundino`): the vnode is the saved record's
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"m10c_iso.txt");
        r.expect("found");
        let vp = vp.expect("a vnode");
        assert_eq!(ident(vp), (20 * 2048 + 324, 23, 12));
        vput(vp);
        assert_eq!(
            lookup(p, dvp, LOOKUP, 0, b"m10c-iso.txt").0,
            Err(Errno::ENOENT)
        );
        vput(dvp);
    }

    #[test]
    fn a_lookup_from_the_last_offset_wraps_to_the_start() {
        let (_g, p, _mp, dvp) = setup(image::image_bytes(), 0);
        let dp = vtoi(dvp);
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
        let vp = vp.expect("a vnode");
        assert_eq!(ident(vp).0, 20 * 2048 + 324);
        vput(vp);
        drop(dp);
        vput(dvp);
    }

    #[test]
    fn a_read_only_mount_refuses_a_delete_or_rename() {
        let (_g, p, mp, dvp) = setup(image::image_bytes(), 0);
        assert_ne!(mp.flag() & MNT_RDONLY, 0);
        assert_eq!(
            lookup(p, dvp, DELETE, 0, b"m10c-iso.txt").0,
            Err(Errno::EROFS)
        );
        assert_eq!(
            lookup(p, dvp, RENAME, 0, b"m10c-iso.txt").0,
            Err(Errno::EROFS)
        );
        vput(dvp);
    }

    /// A plain image whose root directory has two blocks: `.`, `..`, 43 fillers and
    /// `TARGET.TXT;1` fill the first (to byte 2006 of 2048), `ZZZZ.TXT;1` (44 bytes) starts
    /// the second; the target's data is block 20, the other file's block 21.
    fn two_block_root() -> Vec<u8> {
        let mut iso = Iso::new(22);
        let mut recs = std::vec![
            record(18, 4096, 2, &[0], &[]),
            record(18, 4096, 2, &[1], &[]),
        ];
        for i in 0..43 {
            recs.push(record(
                21,
                0,
                0,
                std::format!("F{i:03}.TXT;1").as_bytes(),
                &[],
            ));
        }
        recs.push(record(20, 7, 0, b"TARGET.TXT;1", &[]));
        recs.push(record(21, 5, 0, b"ZZZZ.TXT;1", &[]));
        assert_eq!(iso.dir(18, &recs), 4096);
        iso.put(20, 0, b"target\n");
        iso.put(21, 0, b"zzzz\n");
        iso.pvd(16, b"TWO", 22, &record(18, 4096, 2, &[0], &[]));
        iso.term(17);
        iso.finish()
    }

    /// A match the scan only knows for one when it reads the next record, in the next block
    /// (`foundino`): the saved offset's block is read again for the record handed to
    /// `cd9660_vget_internal` (cd9660_lookup.c:301-307), so the vnode is the target's.
    #[test]
    fn a_match_found_past_a_block_end_is_read_again() {
        let img = two_block_root();
        // The packing the test relies on: the target ends at byte 2006, the next record in
        // the second block.
        assert_eq!(img[18 * 2048 + 1960], 46);
        assert_eq!(
            &img[18 * 2048 + 1960 + 33..18 * 2048 + 1960 + 45],
            b"TARGET.TXT;1"
        );
        assert_eq!(img[18 * 2048 + 2006], 0);
        assert_eq!(&img[19 * 2048 + 33..19 * 2048 + 43], b"ZZZZ.TXT;1");
        let (_g, p, _mp, dvp) = setup(img, ISOFSMNT_NORRIP);
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"target.txt");
        r.expect("found");
        let vp = vp.expect("a vnode");
        assert_eq!(ident(vp), (18 * 2048 + 1960, 20, 7));
        assert_eq!(vtoi(dvp).i_offset.get(), 1960);
        vput(vp);
        let (r, vp, _) = lookup(p, dvp, LOOKUP, 0, b"zzzz.txt");
        r.expect("found");
        let vp = vp.expect("a vnode");
        assert_eq!(ident(vp), (19 * 2048, 21, 5));
        vput(vp);
        vput(dvp);
        let _ = mkiso::BS;
    }

    #[test]
    fn bufatoff_reads_the_directory_block_of_an_offset() {
        let (_g, _p, _mp, dvp) = setup(image::image_bytes(), 0);
        let dp = vtoi(dvp);
        let (t, off) = cd9660_bufatoff(&dp, 324).expect("read");
        assert_eq!(off, 324);
        assert_eq!(&t.bytes()[324..324 + 128], image::FILE_REC);
        // the directory's block 0 is the device's block 20
        assert_eq!(t.buf().blkno(), 20 << 2);
        brelse(t);
        drop(dp);
        vput(dvp);
    }
}
/* </TESTS> */
