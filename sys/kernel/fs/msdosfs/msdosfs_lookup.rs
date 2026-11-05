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
//! Directory operations of the msdos file system: `msdosfs_lookup` (find a name in a
//! directory, or the place a new entry for it would go), `createde` (write a directory entry,
//! with its Win95 long name entries), `dosdirempty`, `doscheckpath` (keep a rename from
//! moving a directory below itself), `readep`/`readde` (read the block of a directory entry),
//! `removede` (delete an entry and its long name entries) and `uniqdosname` (make a short
//! name no other entry has).
//!
//! Upstream: sys/msdosfs/msdosfs_lookup.c @ 3ce1f3f79392
//! LZ: sys/msdosfs/msdosfs_lookup.rs@3c62ede68802
//!
//! When a directory is searched, the blocks holding its entries are read and examined. The
//! entries hold what an inode holds in a unix file system, so part of a directory's contents
//! may also be in memory-resident denodes. A process that changes a directory must therefore
//! own the disk block of the entry first (`bread`/`brelse` give that exclusivity), then
//! update the block and the denode, then write the block, so that the blocks and the denodes
//! agree.
//!
//! ## Deviations
//! - A `struct direntry *` into a buffer is the entry's offset in the buffer's bytes, read
//!   with `Direntry::at` and written back with `Direntry::put` (`Winentry` for a long name
//!   entry at the same offset); `ep--` subtracts the entry size. `removede` checks that it is
//!   not at the start of the block before it steps back, where the C steps back and then does
//!   not look at the entry. Every such offset lies inside its block by construction (the
//!   scans step by 32 below a block size that is a multiple of 512; `createde`'s and
//!   `removede`'s offsets are a found entry's or a free slot's), and `readep` checks its own
//!   (below): an entry past its block is `panic(9)` ([`entry_at`]).
//! - `readep` answers `EIO` when the entry's 32 bytes do not lie inside the block it reads
//!   (`bptoep(diroffset) + 32 > blsize`, [`direntry_in_block`]): the user's decision
//!   D-N5.7-U4 of 2026-11-05, beside U4's root size (`msdosfs_denode.rs`). The test comes
//!   before the read (no block is read, as after a failed `bread` none is held): past the
//!   root's end the C's block can be of no bytes at all. The C reads the entry (and `deupdat`
//!   writes it) past `b_bcount`, stale bytes of the buffer's pages or an unmapped page. Every
//!   caller's entry
//!   lies inside on a sound file system; the check is the belt for a crafted one (a FAT12/16
//!   root whose entries a scan reached past its block, the root's shortened tail). Userland
//!   sees `EIO` where OpenBSD makes a denode of stale bytes. `msdosfs_fhtovp` refuses a handle
//!   failing the same predicate with `ESTALE` before `deget` (D-N5.7-U1, N5.7c3).
//! - The directory search of `msdosfs_lookup` (up to its `found`/`foundroot`/`notfound`
//!   labels) is the private `msdosfs_lookup_search`, which returns where it ended; the
//!   function then does what the labels' code does.
//! - `readep` and `readde` return the busy buffer's token and the offset of the entry in its
//!   data (the C's `bptoep`), where the C fills `*bpp` and `*epp`; on error the buffer is
//!   already released (the C sets `*bpp = NULL`).
//! - `createde` takes the caller's `depp` as `Option<&mut Option<DenodeRef>>` (the C's NULL is
//!   `None`) and sets it to `None` when `deget` fails, as `deget` NULLs it in the C.
//! - `dosdirempty` returns `bool` (the C's 1 and 0).
//! - `doscheckpath`'s `DIAGNOSTIC` check that both denodes are on one file system is behind
//!   feature `diagnostic`.
//! - The `MSDOSFS_DEBUG` `printf`s are left out: the option is not in GENERIC and has no
//!   feature here.
//!
//! ## Redesign
//! - LZ's lent buffers and its `unsafe fn bdata` view of `b_data` (ten `unsafe`) -> `fs::bio`'s
//!   `BufRef` tokens: each scan copies the entry it looks at out of the token's bytes
//!   (`Direntry::at`, a 32-byte copy where the C reads one to eleven bytes in place, O(1) a
//!   step as the C's: D-N5.7-6, docs/ROADMAP.md N12's row), and the writers put the entry
//!   back (`Direntry::put`, `Winentry::put`) before the token's `bwrite`. `createde` keeps one
//!   token at a time across its long name loop, as the C's `bp`. 0 `unsafe` (LZ: 10).
//! - The denodes are counted handles (`DenodeRef`): `deget`'s result lends its vnode to the
//!   caller (`*vpp`, the C's reference) and the handle goes at the end of the call;
//!   `doscheckpath` walks up with one handle at a time, as the C's `dep`.
//! - LZ's `cn_proc` is the componentname's own `proc()`.
//! - Every arithmetic over an on-disk value keeps the C's widths: the scan's `diroff`,
//!   `frcn` and `slotoffset` (the C's `int`s) and the offsets of `createde` and `removede`
//!   wrap at 32 bits (a cyclic cluster chain of a crafted image makes the scan run on, as the
//!   C's).

use hal::sys::errno::Errno;
use hal::sys::mount::{MSDOSFSMNT_NOWIN95, MSDOSFSMNT_SHORTNAME};
use hal::sys::types::Daddr;

use crate::assert::kpanic;
use crate::fs::bio::{bread, brelse, bwrite};
use crate::fs::buf::BufRef;
use crate::fs::cache::{cache_enter, cache_lookup};
use crate::fs::msdosfs::bpb::getushort;
use crate::fs::msdosfs::denode::{Denode, DenodeRef, MSDOSFSROOT_OFS, de_externalize, vtode};
use crate::fs::msdosfs::direntry::{
    ATTR_DIRECTORY, ATTR_VOLUME, ATTR_WIN95, Direntry, SLOT_DELETED, SLOT_EMPTY, Winentry,
};
use crate::fs::msdosfs::fat::{DE_CLEAR, MSDOSFSROOT, fat32};
use crate::fs::msdosfs::msdosfs_conv::{unix2dosfn, unix2winfn, winChkName, winChksum, winSlotCnt};
use crate::fs::msdosfs::msdosfs_denode::{deget, detrunc};
use crate::fs::msdosfs::msdosfs_fat::{extendfile, pcbmap};
use crate::fs::msdosfs::msdosfsmount::{
    Msdosfsmount, MsdosfsmountRef, bptoep, cntobn, de_blk, de_bn2off, de_clcount, de_cluster,
    de_cn2off, detobn,
};
use crate::fs::namei::{
    CREATE, Componentname, DELETE, ISDOTDOT, ISLASTCN, LOCKPARENT, MAKEENTRY, PDIRUNLOCK, RENAME,
    SAVENAME, WANTPARENT,
};
use crate::fs::subr::{vput, vref};
use crate::fs::vnode::{LK_EXCLUSIVE, LK_RETRY, VEXEC, VROOT, VWRITE, Vnode, vn_lock};
use crate::fs::vops::{Cred, vop_access, vop_unlock};
use crate::prf::printf;
use crate::proc::ProcGlue;

/// `sizeof(struct direntry)`.
const DIRENTRY_SIZE: u32 = Direntry::SIZE as u32;

/// The name of the `..` entry, padded as a DOS name.
const DOTDOT_NAME: [u8; 11] = *b"..         ";
/// The name of the `.` entry, padded as a DOS name.
const DOT_NAME: [u8; 11] = *b".          ";

/// Where the directory search of `msdosfs_lookup` ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Search {
    /// `found`/`foundroot`: the entry is a directory (`isadir`) starting at cluster `scn`;
    /// its denode is the one at (`cluster`, `blkoff`).
    Found {
        /// `isadir`: the entry is a directory.
        isadir: bool,
        /// `scn`: the entry's starting cluster.
        scn: u32,
        /// `cluster`: the directory cluster of the denode.
        cluster: u32,
        /// `blkoff`: the offset of the denode's entry.
        blkoff: u32,
    },
    /// `notfound`: the name is not in the directory; a new entry needing `wincnt` slots would
    /// start at `slotoffset` (the DOS entry is its last slot).
    NotFound {
        /// `slotoffset`: where the first of the new entry's slots would go.
        slotoffset: u32,
        /// `wincnt`: the slots the name needs, its long name entries and the DOS entry.
        wincnt: i32,
    },
}

/// What one block of the search held.
enum Block {
    /// Scanned to its end: the next cluster.
    Next,
    /// An entry never used: the rest of the directory is empty.
    Empty,
    /// The name, at the `Search::Found` given.
    Found(Search),
}

/// The directory entry at byte `off` of a directory block: inside the block by construction
/// (the module's deviations), so `panic(9)` when it is not.
pub(crate) fn entry_at(data: &[u8], off: usize) -> Direntry {
    match Direntry::at(data, off) {
        Some(dp) => dp,
        None => kpanic(format_args!("msdosfs: entry {off} past its block")),
    }
}

/// The long name entry at byte `off` of a directory block (as [`entry_at`]).
pub(crate) fn winentry_at(data: &[u8], off: usize) -> Winentry {
    match Winentry::at(data, off) {
        Some(wep) => wep,
        None => kpanic(format_args!("msdosfs: entry {off} past its block")),
    }
}

/// Writes the directory entry `dp` back at byte `off` of its block (as [`entry_at`]).
pub(crate) fn entry_put(data: &mut [u8], off: usize, dp: &Direntry) {
    if !Direntry::put(data, off, dp) {
        kpanic(format_args!("msdosfs: entry {off} past its block"));
    }
}

/// `msdosfs_lookup` (`vop_lookup`): look the last component of `cnp` up in the directory
/// `vdp`, which is locked.
///
/// Found, the entry's vnode is returned locked in `vpp`. Not found, a `CREATE` or `RENAME`
/// of the last component answers `EJUSTRETURN` and leaves in the directory's denode where the
/// new entry should go (`de_fndoffset`, `de_fndcnt`); anything else answers `ENOENT`. The
/// directory stays locked unless `PDIRUNLOCK` is set on return.
pub fn msdosfs_lookup<G: ProcGlue>(
    vdp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Componentname<'_, G>,
) -> Result<(), Errno> {
    let nameiop = cnp.cn_nameiop;

    cnp.cn_flags &= !PDIRUNLOCK; // XXX why this ??
    let flags = cnp.cn_flags;

    let dp = vtode(vdp);
    let pmp = &dp.de_pmp;
    *vpp = None;
    let lockparent = flags & LOCKPARENT != 0;
    let wantparent = flags & (LOCKPARENT | WANTPARENT) != 0;

    // Check accessibility of directory.
    if dp.de_Attributes.get() & ATTR_DIRECTORY == 0 {
        return Err(Errno::ENOTDIR);
    }
    vop_access(vdp, VEXEC, cnp.cn_cred, cnp.proc())?;

    // We now have a segment name to search for, and a directory to search.
    //
    // Before tediously performing a linear scan of the directory, check the name cache to see
    // if the directory/name pair we are looking for is known already.
    if let Some(vp) = cache_lookup(vdp, cnp)? {
        *vpp = Some(vp);
        return Ok(());
    }

    let vroot = vdp.flag() & VROOT != 0;
    let (isadir, mut scn, cluster, blkoff) =
        match msdosfs_lookup_search(&dp, vroot, cnp, nameiop, flags)? {
            Search::Found {
                isadir,
                scn,
                cluster,
                blkoff,
            } => (isadir, scn, cluster, blkoff),
            Search::NotFound { slotoffset, wincnt } => {
                // We hold no disk buffers at this point.
                //
                // If we get here we didn't find the entry we were looking for. But that's ok
                // if we are creating or renaming and are at the end of the pathname and the
                // directory hasn't been removed.
                if (nameiop == CREATE || nameiop == RENAME)
                    && flags & ISLASTCN != 0
                    && dp.de_refcnt.get() != 0
                {
                    // Access for write is interpreted as allowing creation of files in the
                    // directory.
                    vop_access(vdp, VWRITE, cnp.cn_cred, cnp.proc())?;
                    // Return an indication of where the new directory entry should be put.
                    dp.de_fndoffset.set(slotoffset);
                    dp.de_fndcnt.set(wincnt.wrapping_sub(1));

                    // We return with the directory locked, so that the parameters we set up
                    // above will still be valid if we actually decide to do a direnter().
                    // We return ni_vp == NULL to indicate that the entry does not currently
                    // exist; we leave a pointer to the (locked) directory inode in
                    // ndp->ni_dvp. The pathname buffer is saved so that the name can be
                    // obtained later.
                    //
                    // NB - if the directory is unlocked, then this information cannot be
                    // used.
                    cnp.cn_flags |= SAVENAME;
                    if !lockparent {
                        let _ = vop_unlock(vdp);
                        cnp.cn_flags |= PDIRUNLOCK;
                    }
                    return Err(Errno::EJUSTRETURN);
                }
                // Insert name into cache (as non-existent) if appropriate.
                if cnp.cn_flags & MAKEENTRY != 0 && nameiop != CREATE {
                    cache_enter(vdp, *vpp, cnp.name());
                }
                return Err(Errno::ENOENT);
            }
        };

    // foundroot: isadir, scn, cluster and blkoff describe the entry; no buffer is held.
    if fat32(pmp) && scn == MSDOSFSROOT {
        scn = pmp.pm_rootdirblk;
    }

    // If deleting, and at end of pathname, return parameters which can be used to remove
    // file. If the wantparent flag isn't set, we return only the directory (in
    // ndp->ni_dvp), otherwise we go on and lock the inode, being careful with ".".
    if nameiop == DELETE && flags & ISLASTCN != 0 {
        // Don't allow deleting the root.
        if blkoff == MSDOSFSROOT_OFS {
            return Err(Errno::EROFS); // really? XXX
        }

        // Write access to directory required to delete files.
        vop_access(vdp, VWRITE, cnp.cn_cred, cnp.proc())?;

        // Return pointer to current entry in dp->i_offset. Save directory inode pointer in
        // ndp->ni_dvp for dirremove().
        if dp.de_StartCluster.get() == scn && isadir {
            // "."
            vref(vdp);
            *vpp = Some(vdp);
            return Ok(());
        }
        let tdp = deget(pmp, cluster, blkoff)?;
        *vpp = Some(tdp.detov());
        if !lockparent {
            let _ = vop_unlock(vdp);
            cnp.cn_flags |= PDIRUNLOCK;
        }
        return Ok(());
    }

    // If rewriting (RENAME), return the inode and the information required to rewrite the
    // present directory. Must get inode of directory entry to verify it's a regular file, or
    // empty directory.
    if nameiop == RENAME && wantparent && flags & ISLASTCN != 0 {
        if blkoff == MSDOSFSROOT_OFS {
            return Err(Errno::EROFS); // really? XXX
        }

        vop_access(vdp, VWRITE, cnp.cn_cred, cnp.proc())?;

        // Careful about locking second inode. This can only occur if the target is ".".
        if dp.de_StartCluster.get() == scn && isadir {
            return Err(Errno::EISDIR);
        }

        let tdp = deget(pmp, cluster, blkoff)?;
        *vpp = Some(tdp.detov());
        cnp.cn_flags |= SAVENAME;
        if !lockparent {
            let _ = vop_unlock(vdp);
        }
        return Ok(());
    }

    // Step through the translation in the name. We do not `vput' the directory because we
    // may need it again if a symbolic link is relative to the current directory. Instead we
    // save it unlocked as "pdp". We must get the target inode before unlocking the
    // directory to insure that the inode will not be removed before we get it. We prevent
    // deadlock by always fetching inodes from the root, moving down the directory tree. Thus
    // when following backward pointers ".." we must unlock the parent directory before
    // getting the requested directory. There is a potential race condition here if both the
    // current and parent directories are removed before the VFS_VGET for the inode
    // associated with ".." returns. We hope that this occurs infrequently since we cannot
    // avoid this race condition without implementing a sophisticated deadlock detection
    // algorithm. Note also that this simple deadlock detection scheme will not work if the
    // file system has any hard links other than ".." that point backwards in the directory
    // structure.
    let pdp = vdp;
    if flags & ISDOTDOT != 0 {
        let _ = vop_unlock(pdp); // race to get the inode
        cnp.cn_flags |= PDIRUNLOCK;
        let tdp = match deget(pmp, cluster, blkoff) {
            Ok(tdp) => tdp,
            Err(error) => {
                if vn_lock(pdp, LK_EXCLUSIVE | LK_RETRY).is_ok() {
                    cnp.cn_flags &= !PDIRUNLOCK;
                }
                return Err(error);
            }
        };
        if lockparent && flags & ISLASTCN != 0 {
            if let Err(error) = vn_lock(pdp, LK_EXCLUSIVE | LK_RETRY) {
                vput(tdp.detov());
                return Err(error);
            }
            cnp.cn_flags &= !PDIRUNLOCK;
        }
        *vpp = Some(tdp.detov());
    } else if dp.de_StartCluster.get() == scn && isadir {
        vref(vdp); // we want ourself, ie "."
        *vpp = Some(vdp);
    } else {
        let tdp = deget(pmp, cluster, blkoff)?;
        if !lockparent || flags & ISLASTCN == 0 {
            let _ = vop_unlock(pdp);
            cnp.cn_flags |= PDIRUNLOCK;
        }
        *vpp = Some(tdp.detov());
    }

    // Insert name into cache if appropriate.
    if cnp.cn_flags & MAKEENTRY != 0 {
        cache_enter(vdp, *vpp, cnp.name());
    }
    Ok(())
}

/// The directory search of `msdosfs_lookup`: look the name of `cnp` up in the directory
/// `dp` (the root directory when `vroot`), up to the C's `found`, `foundroot` and `notfound`
/// labels. A match sets `dp.de_fndoffset` and `dp.de_fndcnt` to the entry's offset and the
/// long name entries a rename may reuse.
fn msdosfs_lookup_search<G: ProcGlue>(
    dp: &Denode<G>,
    vroot: bool,
    cnp: &Componentname<'_, G>,
    nameiop: u64,
    flags: u64,
) -> Result<Search, Errno> {
    let pmp = dp.pmp();
    let name = cnp.name();

    // If they are going after the . or .. entry in the root directory, they won't find it.
    // DOS filesystems don't have them in the root directory. So, we fake it. deget() is in on
    // this scam too.
    if vroot && name.first() == Some(&b'.') && (name.len() == 1 || name == b"..") {
        return Ok(Search::Found {
            isadir: true,
            scn: MSDOSFSROOT,
            cluster: MSDOSFSROOT,
            blkoff: MSDOSFSROOT_OFS,
        });
    }

    let mut dosfilename = [0u8; 11];
    let mut wincnt = 1;
    let mut olddos = true;
    match unix2dosfn(name, &mut dosfilename, 0) {
        0 => return Err(Errno::EINVAL),
        2 => wincnt = winSlotCnt(name) + 1,
        3 => {
            olddos = false;
            wincnt = winSlotCnt(name) + 1;
        }
        _ => {}
    }
    let shortname = pmp.pm_flags.get() & MSDOSFSMNT_SHORTNAME as u32 != 0;
    if shortname {
        wincnt = 1;
    }

    // Suppress search for slots unless creating file and at end of pathname, in which case
    // we watch for a place to put the new file in case it doesn't already exist.
    let mut slotcount = wincnt;
    if (nameiop == CREATE || nameiop == RENAME) && flags & ISLASTCN != 0 {
        slotcount = 0;
    }
    let mut slotoffset: u32 = 0;

    // XXX UNIX allows filenames with trailing dots and blanks; we don't. Most of the
    // routines in msdosfs_conv.c adjust for this, but winChkName() does not, so we do it
    // here. Otherwise, a file such as ".foobar." cannot be retrieved properly.
    //
    // (Note that this is also faster: perform the adjustment once, rather than on each call
    // to winChkName. However, it is still a nasty hack.)
    let adjlen = name
        .iter()
        .rposition(|&c| c != b' ' && c != b'.')
        .map_or(0, |i| i + 1);
    let adj = &name[..adjlen];

    // The outer loop ranges over the clusters that make up the directory. Note that the root
    // directory is different from all other directories. It has a fixed number of blocks
    // that are not part of the pool of allocatable clusters. So, we treat it a little
    // differently. The root directory starts at "cluster" 0.
    let mut chksum: i32 = -1;
    let mut diroff: u32 = 0;
    let mut frcn: u32 = 0;
    loop {
        let mut bn: Daddr = 0;
        let mut cluster: u32 = 0;
        let mut blsize: i32 = 0;
        match pcbmap(
            dp,
            frcn,
            Some(&mut bn),
            Some(&mut cluster),
            Some(&mut blsize),
        ) {
            Ok(()) => {}
            Err(Errno::E2BIG) => break,
            Err(error) => return Err(error),
        }
        let (bp, error) = bread(&pmp.pm_devvp.bufs, bn, blsize);
        if let Err(error) = error {
            brelse(bp);
            return Err(error);
        }
        let outcome = {
            let data = bp.bytes();
            let mut blkoff: u32 = 0;
            loop {
                if blkoff >= blsize as u32 {
                    break Block::Next;
                }
                let dep = entry_at(data, blkoff as usize);
                // If the slot is empty and we are still looking for an empty then remember
                // this one. If the slot is not empty then check to see if it matches what we
                // are looking for. If the slot has never been filled with anything, then the
                // remainder of the directory has never been used, so there is no point in
                // searching it.
                if dep.deName[0] == SLOT_EMPTY || dep.deName[0] == SLOT_DELETED {
                    // Drop memory of previous long matches
                    chksum = -1;

                    if slotcount < wincnt {
                        slotcount += 1;
                        slotoffset = diroff;
                    }
                    if dep.deName[0] == SLOT_EMPTY {
                        break Block::Empty;
                    }
                } else {
                    // If there wasn't enough space for our winentries, forget about the empty
                    // space
                    if slotcount < wincnt {
                        slotcount = 0;
                    }

                    if dep.deAttributes == ATTR_WIN95 {
                        // Check for Win95 long filename entry
                        if !shortname {
                            chksum = winChkName(adj, &winentry_at(data, blkoff as usize), chksum);
                        }
                    } else if dep.deAttributes & ATTR_VOLUME != 0 {
                        // Ignore volume labels (anywhere, not just the root directory).
                        chksum = -1;
                    } else {
                        // Check for a checksum or name match
                        let name11 = dep.name11();
                        let chksum_ok = chksum == i32::from(winChksum(&name11));
                        if !chksum_ok && (!olddos || dosfilename != name11) {
                            chksum = -1;
                        } else {
                            // Remember where this directory entry came from for whoever did
                            // this lookup.
                            dp.de_fndoffset.set(diroff);
                            if chksum_ok && nameiop == RENAME {
                                // Target had correct long name directory entries, reuse them
                                // as needed.
                                dp.de_fndcnt.set(wincnt.wrapping_sub(1));
                            } else {
                                // Long name directory entries not present or corrupt, can
                                // only reuse dos directory entry.
                                dp.de_fndcnt.set(0);
                            }
                            break Block::Found(found(pmp, &dep, cluster, blkoff, diroff));
                        }
                    }
                }
                blkoff = blkoff.wrapping_add(DIRENTRY_SIZE);
                diroff = diroff.wrapping_add(DIRENTRY_SIZE);
            }
        };
        // Release the buffer holding the directory cluster just searched (for a match: to
        // allow deget to read the entry again; reserving it here and giving it to deget could
        // result in a deadlock).
        brelse(bp);
        match outcome {
            Block::Next => frcn = frcn.wrapping_add(1),
            Block::Empty => break,
            Block::Found(s) => return Ok(s),
        }
    }

    // notfound: fixup the slot description to point to the place where we might put the new
    // DOS direntry (putting the Win95 long name entries before that)
    if slotcount == 0 {
        slotcount = 1;
        slotoffset = diroff;
    }
    if wincnt > slotcount {
        slotoffset =
            slotoffset.wrapping_add(DIRENTRY_SIZE.wrapping_mul((wincnt - slotcount) as u32));
    }
    Ok(Search::NotFound { slotoffset, wincnt })
}

/// `found:` of the search: the denode's place of the entry `dep` that matched at `blkoff` of
/// the block of `cluster` (`diroff` from the directory's start).
fn found<G: ProcGlue>(
    pmp: &Msdosfsmount<G>,
    dep: &Direntry,
    cluster: u32,
    blkoff: u32,
    diroff: u32,
) -> Search {
    let isadir = dep.deAttributes & ATTR_DIRECTORY != 0;
    let mut scn = u32::from(getushort(&dep.deStartCluster));
    if fat32(pmp) {
        scn |= u32::from(getushort(&dep.deHighClust)) << 16;
        if scn == pmp.pm_rootdirblk {
            // There should actually be 0 here. Just ignore the error.
            scn = MSDOSFSROOT;
        }
    }

    let mut cluster = cluster;
    let mut blkoff = blkoff;
    if cluster == MSDOSFSROOT {
        blkoff = diroff;
    }

    if isadir {
        cluster = scn;
        blkoff = if cluster == MSDOSFSROOT {
            MSDOSFSROOT_OFS
        } else {
            0
        };
    }
    Search::Found {
        isadir,
        scn,
        cluster,
        blkoff,
    }
}

/// `createde(dep, ddep, depp, cnp)`: write the directory entry of `dep` (a template, often
/// the caller's stack denode) into the directory `ddep` at `ddep.de_fndoffset`, preceded by
/// `ddep.de_fndcnt` Win95 long name entries for `cnp`'s name, which `msdosfs_lookup` found
/// room for. When `depp` is given, the new entry's denode is returned there, locked.
pub fn createde<G: ProcGlue>(
    dep: &Denode<G>,
    ddep: &Denode<G>,
    depp: Option<&mut Option<DenodeRef<G>>>,
    cnp: &Componentname<'_, G>,
) -> Result<(), Errno> {
    let pmp = &ddep.de_pmp;

    // If no space left in the directory then allocate another cluster and chain it onto the
    // end of the file. There is one exception to this. That is, if the root directory has no
    // more space it can NOT be expanded. extendfile() checks for and fails attempts to extend
    // the root directory. We just return an error in that case.
    if ddep.de_fndoffset.get() >= ddep.de_FileSize.get() {
        let diroffset = ddep
            .de_fndoffset
            .get()
            .wrapping_add(DIRENTRY_SIZE)
            .wrapping_sub(ddep.de_FileSize.get());
        let dirclust = de_clcount(pmp, diroffset);
        if let Err(error) = extendfile(ddep, dirclust, None, None, DE_CLEAR) {
            let _ = detrunc(
                ddep,
                ddep.de_FileSize.get(),
                0,
                Cred::NOCRED,
                Some(cnp.proc()),
            );
            return Err(error);
        }

        // Update the size of the directory
        ddep.de_FileSize.set(
            ddep.de_FileSize
                .get()
                .wrapping_add(de_cn2off(pmp, dirclust)),
        );
    }

    // We just read in the cluster with space. Copy the new directory entry in. Then write it
    // to disk. NOTE: DOS directories do not get smaller as clusters are emptied.
    let mut bn: Daddr = 0;
    let mut dirclust: u32 = 0;
    let mut blsize: i32 = 0;
    pcbmap(
        ddep,
        de_cluster(pmp, ddep.de_fndoffset.get()),
        Some(&mut bn),
        Some(&mut dirclust),
        Some(&mut blsize),
    )?;
    let mut diroffset = ddep.de_fndoffset.get();
    if dirclust != MSDOSFSROOT {
        diroffset &= pmp.pm_crbomask;
    }
    let (mut bp, error) = bread(&pmp.pm_devvp.bufs, bn, blsize);
    if let Err(error) = error {
        brelse(bp);
        return Err(error);
    }
    let mut ndep = bptoep(pmp, ddep.de_fndoffset.get());

    let mut dp = entry_at(bp.bytes(), ndep);
    de_externalize(&mut dp, dep);
    entry_put(bp.data(), ndep, &dp);

    // Now write the Win95 long name
    if ddep.de_fndcnt.get() > 0 {
        let chksum = winChksum(&dp.name11());
        let un = cnp.name();
        let mut cnt = 1;

        loop {
            ddep.de_fndcnt.set(ddep.de_fndcnt.get().wrapping_sub(1));
            if ddep.de_fndcnt.get() < 0 {
                break;
            }
            if ddep.de_fndoffset.get() & pmp.pm_crbomask == 0 {
                bwrite(bp)?;

                ddep.de_fndoffset
                    .set(ddep.de_fndoffset.get().wrapping_sub(DIRENTRY_SIZE));
                pcbmap(
                    ddep,
                    de_cluster(pmp, ddep.de_fndoffset.get()),
                    Some(&mut bn),
                    None,
                    Some(&mut blsize),
                )?;

                let (b, error) = bread(&pmp.pm_devvp.bufs, bn, blsize);
                if let Err(error) = error {
                    brelse(b);
                    return Err(error);
                }
                bp = b;
                ndep = bptoep(pmp, ddep.de_fndoffset.get());
            } else {
                ndep -= Direntry::SIZE;
                ddep.de_fndoffset
                    .set(ddep.de_fndoffset.get().wrapping_sub(DIRENTRY_SIZE));
            }
            let mut wep = winentry_at(bp.bytes(), ndep);
            let more = unix2winfn(un, &mut wep, cnt, chksum);
            if !Winentry::put(bp.data(), ndep, &wep) {
                kpanic(format_args!("msdosfs: entry {ndep} past its block"));
            }
            cnt += 1;
            if more == 0 {
                break;
            }
        }
    }

    bwrite(bp)?;

    // If they want us to return with the denode gotten.
    if let Some(depp) = depp {
        if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
            dirclust = dep.de_StartCluster.get();
            if fat32(pmp) && dirclust == pmp.pm_rootdirblk {
                dirclust = MSDOSFSROOT;
            }
            diroffset = if dirclust == MSDOSFSROOT {
                MSDOSFSROOT_OFS
            } else {
                0
            };
        }
        return match deget(pmp, dirclust, diroffset) {
            Ok(ndep) => {
                *depp = Some(ndep);
                Ok(())
            }
            Err(error) => {
                *depp = None;
                Err(error)
            }
        };
    }

    Ok(())
}

/// `dosdirempty(dep)`: be sure a directory is empty except for "." and "..". `true` if
/// empty, `false` if not empty or on error.
pub fn dosdirempty<G: ProcGlue>(dep: &Denode<G>) -> bool {
    let pmp = dep.pmp();

    // Since the filesize field in directory entries for a directory is zero, we just have to
    // feel our way through the directory until we hit end of file.
    let mut cn: u32 = 0;
    loop {
        let mut bn: Daddr = 0;
        let mut blsize: i32 = 0;
        match pcbmap(dep, cn, Some(&mut bn), None, Some(&mut blsize)) {
            Ok(()) => {}
            Err(Errno::E2BIG) => return true, // it's empty
            Err(_) => return false,
        }
        let (bp, error) = bread(&pmp.pm_devvp.bufs, bn, blsize);
        if error.is_err() {
            brelse(bp);
            return false;
        }
        let verdict = {
            let data = bp.bytes();
            let mut verdict = None;
            for dentp in data[..blsize as usize].as_chunks::<{ Direntry::SIZE }>().0 {
                let dentp = entry_at(dentp, 0);
                if dentp.deName[0] != SLOT_DELETED && dentp.deAttributes & ATTR_VOLUME == 0 {
                    // In dos directories an entry whose name starts with SLOT_EMPTY (0) starts
                    // the beginning of the unused part of the directory, so we can just return
                    // that it is empty.
                    if dentp.deName[0] == SLOT_EMPTY {
                        verdict = Some(true);
                        break;
                    }
                    // Any names other than "." and ".." in a directory mean it is not empty.
                    let name11 = dentp.name11();
                    if name11 != DOT_NAME && name11 != DOTDOT_NAME {
                        verdict = Some(false); // not empty
                        break;
                    }
                }
            }
            verdict
        };
        brelse(bp);
        if let Some(v) = verdict {
            return v;
        }
        cn = cn.wrapping_add(1);
    }
}

/// `doscheckpath(source, target)`: check to see if the directory described by `target` is in
/// some subdirectory of `source`. This prevents something like the following from
/// succeeding and leaving a bunch or files and directories orphaned: `mv /a/b/c
/// /a/b/c/d/e/f`, where c and f are directories (`source` is the denode of /a/b/c, `target`
/// that of /a/b/c/d/e/f).
///
/// `Ok` if `target` is NOT a subdirectory of `source`, an error otherwise. The target's
/// vnode is always released (`vput`) on return.
pub fn doscheckpath<G: ProcGlue>(source: &Denode<G>, target: DenodeRef<G>) -> Result<(), Errno> {
    let mut dep: Option<DenodeRef<G>> = Some(target);
    let mut bp: Option<BufRef> = None;

    let error: Result<(), Errno> = 'out: {
        let Some(target) = dep.as_ref() else {
            break 'out Ok(());
        };
        if target.de_Attributes.get() & ATTR_DIRECTORY == 0
            || source.de_Attributes.get() & ATTR_DIRECTORY == 0
        {
            break 'out Err(Errno::ENOTDIR);
        }
        if target.de_StartCluster.get() == source.de_StartCluster.get() {
            break 'out Err(Errno::EEXIST);
        }
        if target.de_StartCluster.get() == MSDOSFSROOT {
            break 'out Ok(());
        }
        let pmp: MsdosfsmountRef<G> = target.de_pmp.clone();
        #[cfg(feature = "diagnostic")]
        if !hal::malloc::MArc::ptr_eq(&pmp, &source.de_pmp) {
            kpanic(format_args!(
                "doscheckpath: source and target on different filesystems"
            ));
        }
        if fat32(&pmp) && target.de_StartCluster.get() == pmp.pm_rootdirblk {
            break 'out Ok(());
        }

        loop {
            let Some(d) = dep.as_ref() else {
                break 'out Ok(());
            };
            if d.de_Attributes.get() & ATTR_DIRECTORY == 0 {
                break 'out Err(Errno::ENOTDIR);
            }
            let scn = d.de_StartCluster.get();
            let (b, error) = bread(
                &pmp.pm_devvp.bufs,
                Daddr::from(cntobn(&pmp, scn)),
                pmp.pm_bpcluster as i32,
            );
            let b = bp.insert(b);
            if let Err(error) = error {
                break 'out Err(error);
            }

            let ep = entry_at(b.bytes(), Direntry::SIZE);
            if ep.deAttributes & ATTR_DIRECTORY == 0 || ep.name11() != DOTDOT_NAME {
                break 'out Err(Errno::ENOTDIR);
            }
            let mut scn = u32::from(getushort(&ep.deStartCluster));
            if fat32(&pmp) {
                scn |= u32::from(getushort(&ep.deHighClust)) << 16;
            }

            if scn == source.de_StartCluster.get() {
                break 'out Err(Errno::EINVAL);
            }
            if scn == MSDOSFSROOT {
                break 'out Ok(());
            }
            if fat32(&pmp) && scn == pmp.pm_rootdirblk {
                // scn should be 0 in this case, but we silently ignore the error.
                break 'out Ok(());
            }

            if let Some(d) = dep.take() {
                vput(d.detov());
            }
            if let Some(b) = bp.take() {
                brelse(b);
            }
            // NOTE: deget() clears dep on error
            match deget(&pmp, scn, 0) {
                Ok(n) => dep = Some(n),
                Err(error) => break 'out Err(error),
            }
        }
    };

    // out:
    if let Some(b) = bp {
        brelse(b);
    }
    if error == Err(Errno::ENOTDIR) {
        printf(format_args!("doscheckpath(): .. not a directory?\n"));
    }
    if let Some(d) = dep {
        vput(d.detov());
    }
    error
}

/// `readep`'s block size for the entry at (`dirclust`, `diroffset`): a cluster, or the FAT12/16
/// root directory's tail when the cluster would pass the root's end.
fn readep_blsize<G: ProcGlue>(pmp: &Msdosfsmount<G>, dirclust: u32, diroffset: u32) -> u32 {
    let mut blsize = pmp.pm_bpcluster;
    if dirclust == MSDOSFSROOT && de_blk(pmp, diroffset.wrapping_add(blsize)) > pmp.pm_rootdirsize {
        blsize = de_bn2off(pmp, pmp.pm_rootdirsize) & pmp.pm_crbomask;
    }
    blsize
}

/// Whether the directory entry at (`dirclust`, `diroffset`) lies whole inside the block
/// `readep` reads for it (`bptoep(pmp, bp, diroffset) + 32 <= blsize`): the predicate of
/// `readep`'s `EIO` (D-N5.7-U4) and of `msdosfs_fhtovp`'s `ESTALE` (D-N5.7-U1). For the
/// position of an entry `deget` reads, not for the root directory's own (`MSDOSFSROOT_OFS`,
/// which `deget` manufactures).
pub fn direntry_in_block<G: ProcGlue>(
    pmp: &Msdosfsmount<G>,
    dirclust: u32,
    diroffset: u32,
) -> bool {
    bptoep(pmp, diroffset) + Direntry::SIZE <= readep_blsize(pmp, dirclust, diroffset) as usize
}

/// `readep(pmp, dirclust, diroffset, bpp, epp)`: read in the disk block containing the
/// directory entry (`dirclust`, `diroffset`) and return the token of the buf header, busy,
/// and the offset of the directory entry within the block. `EIO`, nothing read, when the
/// entry does not lie whole inside the block (the module's deviations, D-N5.7-U4).
pub fn readep<G: ProcGlue>(
    pmp: &Msdosfsmount<G>,
    dirclust: u32,
    diroffset: u32,
) -> Result<(BufRef, usize), Errno> {
    let blsize = readep_blsize(pmp, dirclust, diroffset);
    let off = bptoep(pmp, diroffset);
    // The entry outside the block the C reads (D-N5.7-U4): refused before the read, which
    // for a position past the root's end would ask the device for as little as no bytes.
    if off + Direntry::SIZE > blsize as usize {
        return Err(Errno::EIO);
    }
    let bn = detobn(pmp, dirclust, diroffset);
    let (bp, error) = bread(&pmp.pm_devvp.bufs, Daddr::from(bn), blsize as i32);
    if let Err(error) = error {
        brelse(bp);
        return Err(error);
    }
    Ok((bp, off))
}

/// `readde(dep, bpp, epp)`: read in the disk block containing the directory entry `dep` came
/// from and return the token of the buf header, busy, and the offset of the directory entry
/// within the block.
pub fn readde<G: ProcGlue>(dep: &Denode<G>) -> Result<(BufRef, usize), Errno> {
    readep(dep.pmp(), dep.de_dirclust.get(), dep.de_diroffset.get())
}

/// `removede(pdep, dep)`: remove a directory entry, the one at `pdep.de_fndoffset` in the
/// directory `pdep` (with the Win95 long name entries before it), which describes the file
/// `dep`.
///
/// At this point the file represented by the directory entry to be removed is still full
/// length until noone has it open. When the file no longer being used `msdosfs_inactive()`
/// is called and will truncate the file to 0 length. When the vnode containing the denode is
/// needed for some other purpose by VFS it will call `msdosfs_reclaim()` which will remove
/// the denode from the denode cache.
pub fn removede<G: ProcGlue>(pdep: &Denode<G>, dep: &Denode<G>) -> Result<(), Errno> {
    let pmp = pdep.pmp();
    let nowin95 = pmp.pm_flags.get() & MSDOSFSMNT_NOWIN95 as u32 != 0;
    let crbomask = pmp.pm_crbomask;
    let mut offset = pdep.de_fndoffset.get();

    dep.de_refcnt.set(dep.de_refcnt.get().wrapping_sub(1));
    offset = offset.wrapping_add(DIRENTRY_SIZE);
    loop {
        offset = offset.wrapping_sub(DIRENTRY_SIZE);
        let mut bn: Daddr = 0;
        let mut blsize: i32 = 0;
        pcbmap(
            pdep,
            de_cluster(pmp, offset),
            Some(&mut bn),
            None,
            Some(&mut blsize),
        )?;
        let (mut bp, error) = bread(&pmp.pm_devvp.bufs, bn, blsize);
        if let Err(error) = error {
            brelse(bp);
            return Err(error);
        }
        let mut ep = bptoep(pmp, offset);
        // Check whether, if we came here the second time, i.e. when underflowing into the
        // previous block, the last entry in this block is a longfilename entry, too.
        if entry_at(bp.bytes(), ep).deAttributes != ATTR_WIN95 && offset != pdep.de_fndoffset.get()
        {
            brelse(bp);
            break;
        }
        offset = offset.wrapping_add(DIRENTRY_SIZE);
        loop {
            // We are a bit aggressive here in that we delete any Win95 entries preceding
            // this entry, not just the ones we "own". Since these presumably aren't valid
            // anyway, there should be no harm.
            offset = offset.wrapping_sub(DIRENTRY_SIZE);
            let mut e = entry_at(bp.bytes(), ep);
            e.deName[0] = SLOT_DELETED;
            entry_put(bp.data(), ep, &e);
            if nowin95 || offset & crbomask == 0 {
                break;
            }
            ep -= Direntry::SIZE;
            if entry_at(bp.bytes(), ep).deAttributes != ATTR_WIN95 {
                break;
            }
        }
        bwrite(bp)?;
        if nowin95 || offset & crbomask != 0 || offset == 0 {
            break;
        }
    }
    Ok(())
}

/// `uniqdosname(dep, cnp, cp)`: create in `cp` a DOS name for `cnp`'s name that no entry of
/// the directory `dep` has, trying the generation numbers 1, 2, ... in turn.
pub fn uniqdosname<G: ProcGlue>(
    dep: &Denode<G>,
    cnp: &Componentname<'_, G>,
    cp: &mut [u8; 11],
) -> Result<(), Errno> {
    let pmp = dep.pmp();

    let mut r#gen: u32 = 1;
    loop {
        // Generate DOS name with generation number
        if unix2dosfn(cnp.name(), cp, r#gen) == 0 {
            return Err(if r#gen == 1 {
                Errno::EINVAL
            } else {
                Errno::EEXIST
            });
        }

        // Now look for a dir entry with this exact name
        let mut cn: u32 = 0;
        loop {
            let mut bn: Daddr = 0;
            let mut blsize: i32 = 0;
            match pcbmap(dep, cn, Some(&mut bn), None, Some(&mut blsize)) {
                Ok(()) => {}
                Err(Errno::E2BIG) => return Ok(()), // EOF reached and not found
                Err(error) => return Err(error),
            }
            let (bp, error) = bread(&pmp.pm_devvp.bufs, bn, blsize);
            if let Err(error) = error {
                brelse(bp);
                return Err(error);
            }
            // `Some(false)`: the last used entry, and not found; `Some(true)`: found.
            let seen = {
                let data = bp.bytes();
                let mut seen = None;
                for dentp in data[..blsize as usize].as_chunks::<{ Direntry::SIZE }>().0 {
                    let dentp = entry_at(dentp, 0);
                    if dentp.deName[0] == SLOT_EMPTY {
                        // Last used entry and not found
                        seen = Some(false);
                        break;
                    }
                    // Ignore volume labels and Win95 entries
                    if dentp.deAttributes & ATTR_VOLUME != 0 {
                        continue;
                    }
                    if dentp.name11() == *cp {
                        seen = Some(true);
                        break;
                    }
                }
                seen
            };
            brelse(bp);
            match seen {
                Some(false) => return Ok(()),
                // EEXIST: try the next generation number.
                Some(true) => break,
                None => cn = cn.wrapping_add(1),
            }
        }
        r#gen = r#gen.wrapping_add(1);
    }
}

/// The block number of [`test_full_root`]'s root directory.
#[cfg(test)]
pub(crate) const TEST_ROOTBLK: u32 = 8;

/// D-N5.7-U4's image, for the tests of the denode cache, the directories and the vnode
/// operations before the mount (N5.7c3): a FAT16 mount structure of the test glue over
/// testffs's disk, its sectors `bps` bytes and its clusters `spc` sectors, its root directory
/// of `rootents` entries at block [`TEST_ROOTBLK`], every entry used (`F0000000.TXT`,
/// `F0000001.TXT`, ...), and an entry `PHANTOM.TXT` right after the root's end, the first
/// bytes of the first data cluster (what a scan past the root's real end finds); the mask
/// `0777`, the
/// owner and group 0. `msdosfs_init` run; the guard of the kernel's memory, the thread and
/// the structure (published) returned.
#[cfg(test)]
pub(crate) fn test_full_root(
    bps: u16,
    spc: u8,
    rootents: u16,
) -> (
    std::sync::MutexGuard<'static, ()>,
    &'static crate::proc::Thread<crate::fs::testfs::TestGlue>,
    MsdosfsmountRef<crate::fs::testfs::TestGlue>,
) {
    use crate::fs::msdosfs::direntry::ATTR_ARCHIVE;
    use crate::fs::msdosfs::fat::FAT16_MASK;
    use crate::fs::msdosfs::msdosfs_denode::msdosfs_init;
    use crate::fs::msdosfs::msdosfsmount::test_pmp;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs;
    use crate::fs::word::U32Word;

    let rootdirsize = (u32::from(rootents) * DIRENTRY_SIZE).div_ceil(512);
    let bpcluster = u32::from(spc) * u32::from(bps);
    let root = TEST_ROOTBLK as usize * 512;
    let mut img = std::vec![0u8; 1 << 16];
    let entry = |name: &[u8; 11]| {
        let mut e = Direntry {
            deAttributes: ATTR_ARCHIVE,
            ..Direntry::default()
        };
        e.set_name11(name);
        e
    };
    for i in 0..usize::from(rootents) {
        let mut n = *b"F0000000TXT";
        n[1..8].copy_from_slice(std::format!("{i:07}").as_bytes());
        assert!(Direntry::put(
            &mut img,
            root + i * Direntry::SIZE,
            &entry(&n)
        ));
    }
    let phantom = root + rootdirsize as usize * 512;
    assert!(Direntry::put(&mut img, phantom, &entry(b"PHANTOM TXT")));
    let (g, p) = testffs::attach(img);
    let pmp = test_pmp(FAT16_MASK, 100, |pm| {
        pm.pm_bpb.bpbBytesPerSec = bps;
        pm.pm_bpb.bpbSecPerClust = spc;
        pm.pm_bpb.bpbRootDirEnts = rootents;
        pm.pm_BlkPerSec = u32::from(bps) / 512;
        pm.pm_rootdirblk = TEST_ROOTBLK;
        pm.pm_rootdirsize = rootdirsize;
        pm.pm_firstcluster = TEST_ROOTBLK + rootdirsize;
        pm.pm_bnshift = 9;
        pm.pm_bpcluster = bpcluster;
        pm.pm_crbomask = bpcluster - 1;
        pm.pm_cnshift = bpcluster.trailing_zeros();
        pm.pm_mask = U32Word::new(0o777);
    });
    if let Err(e) = msdosfs_init::<TestGlue>() {
        std::panic!("msdosfs_init: {e:?}");
    }
    (g, p, pmp)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::namei::LOOKUP;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ufs::testufs::cn;

    /// The FAT16 root of 1 KiB sectors and 2 KiB clusters: 208 entries, 13 blocks, a tail of
    /// 512 bytes in the root's fourth cluster (the brief's second worked example).
    fn one_k() -> (
        std::sync::MutexGuard<'static, ()>,
        &'static crate::proc::Thread<TestGlue>,
        MsdosfsmountRef<TestGlue>,
    ) {
        test_full_root(1024, 2, 208)
    }

    #[test]
    fn an_entry_lies_in_readep_s_block_up_to_its_last_32_bytes() {
        let (_g, _p, pmp) = one_k();
        // A cluster of a subdirectory: the last 31 offsets of the cluster are outside.
        let outside: std::vec::Vec<u32> = (0..2048u32)
            .filter(|&o| !direntry_in_block(&pmp, 2, 4096 + o))
            .collect();
        assert_eq!(outside, (2017..2048).collect::<std::vec::Vec<_>>());
        // The root's full clusters likewise; its tail cluster (6144..6655) holds 512 bytes.
        assert!(direntry_in_block(&pmp, MSDOSFSROOT, 2016));
        assert!(!direntry_in_block(&pmp, MSDOSFSROOT, 2017));
        assert!(direntry_in_block(&pmp, MSDOSFSROOT, 6144 + 480));
        assert!((481..512).all(|o| !direntry_in_block(&pmp, MSDOSFSROOT, 6144 + o)));
        // Past the root's end: the phantom entry (6656: 512 in a 512-byte tail), then any
        // later offset (7008: 864).
        assert!(!direntry_in_block(&pmp, MSDOSFSROOT, 6656));
        assert!(!direntry_in_block(&pmp, MSDOSFSROOT, 7008));
    }

    #[test]
    fn readep_answers_eio_for_an_entry_past_its_block() {
        {
            let (_g, _p, pmp) = one_k();
            assert_eq!(
                readep(&pmp, MSDOSFSROOT, 6656).map(|(t, off)| (brelse(t), off).1),
                Err(Errno::EIO)
            );
            // The last entry of the tail: read, the buffer released after.
            let (t, off) = readep(&pmp, MSDOSFSROOT, 6624).unwrap();
            assert_eq!(off, 480);
            assert_eq!(entry_at(t.bytes(), off).name11(), *b"F0000207TXT");
            brelse(t);
            // Both buffers were released: the block reads again.
            let (t, _) = readep(&pmp, MSDOSFSROOT, 6624).unwrap();
            brelse(t);
        }
        // 4 KiB sectors, 512 entries: a root of four whole clusters, so the cluster past its
        // end has a tail of no bytes at all (the brief's first worked example).
        let (_g, _p, pmp) = test_full_root(4096, 1, 512);
        assert_eq!(
            readep(&pmp, MSDOSFSROOT, 16384).map(|(t, off)| (brelse(t), off).1),
            Err(Errno::EIO)
        );
        let (t, off) = readep(&pmp, MSDOSFSROOT, 16352).unwrap();
        assert_eq!(off, 4064);
        assert_eq!(entry_at(t.bytes(), off).name11(), *b"F0000511TXT");
        brelse(t);
    }

    #[test]
    fn a_lookup_ends_at_the_real_end_of_a_full_root() {
        let (_g, p, pmp) = one_k();
        let cred = p.ucred();
        let root = deget(&pmp, MSDOSFSROOT, MSDOSFSROOT_OFS).unwrap();
        let rvp = root.detov();
        // The phantom entry past the root's end is not found (D-N5.7-U4).
        let mut c = cn(p, cred, LOOKUP, LOCKPARENT, b"PHANTOM.TXT");
        let mut vpp = None;
        assert_eq!(msdosfs_lookup(rvp, &mut vpp, &mut c), Err(Errno::ENOENT));
        // The last real entry is.
        let mut c = cn(p, cred, LOOKUP, LOCKPARENT, b"F0000207.TXT");
        let mut vpp = None;
        msdosfs_lookup(rvp, &mut vpp, &mut c).unwrap();
        let vp = vpp.unwrap();
        let dep = vtode(vp);
        assert_eq!(
            (dep.de_dirclust.get(), dep.de_diroffset.get()),
            (MSDOSFSROOT, 6624)
        );
        assert_eq!(root.de_fndoffset.get(), 6624);
        drop(dep);
        vput(vp);
        // CREATE: the new entry would go at the root's end, which create refuses (ENOSPC,
        // msdosfs_vnops.rs's test).
        let mut c = cn(p, cred, CREATE, LOCKPARENT, b"NEW.TXT");
        let mut vpp = None;
        assert_eq!(
            msdosfs_lookup(rvp, &mut vpp, &mut c),
            Err(Errno::EJUSTRETURN)
        );
        assert_eq!((root.de_fndoffset.get(), root.de_fndcnt.get()), (6656, 0));
        drop(root);
        vput(rvp);
    }

    #[test]
    fn uniqdosname_takes_the_next_generation_a_directory_lacks() {
        let (_g, p, pmp) = one_k();
        let cred = p.ucred();
        // The root's first entry renamed to the name a long name's first generation takes.
        let mut d = crate::fs::ufs::ffs::testffs::disk();
        let at = TEST_ROOTBLK as usize * 512;
        let mut e = entry_at(&d, at);
        e.set_name11(b"AVERYL~1TXT");
        entry_put(&mut d, at, &e);
        crate::fs::ufs::ffs::testffs::set_disk(d);
        let root = deget(&pmp, MSDOSFSROOT, MSDOSFSROOT_OFS).unwrap();
        let c = cn(p, cred, CREATE, LOCKPARENT, b"averylongname.txt");
        let mut name = [0u8; 11];
        uniqdosname(&root, &c, &mut name).unwrap();
        assert_eq!(&name, b"AVERYL~2TXT");
        // A name no entry has keeps its first generation.
        let c = cn(p, cred, CREATE, LOCKPARENT, b"anotherlongone.txt");
        uniqdosname(&root, &c, &mut name).unwrap();
        assert_eq!(&name, b"ANOTHE~1TXT");
        let rvp = root.detov();
        drop(root);
        vput(rvp);
    }
}
/* </TESTS> */
