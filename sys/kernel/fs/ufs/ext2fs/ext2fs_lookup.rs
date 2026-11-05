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

/*	$OpenBSD: ext2fs_lookup.c,v 1.48 2024/09/20 02:00:46 jsg Exp $	*/
/*	$NetBSD: ext2fs_lookup.c,v 1.16 2000/08/03 20:29:26 thorpej Exp $	*/

/*
 * Modified for NetBSD 1.2E
 * May 1997, Manuel Bouyer
 * Laboratoire d'informatique de Paris VI
 */
/*
 *  modified for Lites 1.1
 *
 *  Aug 1995, Godmar Back (gback@cs.utah.edu)
 *  University of Utah, Department of Computer Science
 */
/*
 * Copyright (c) 1989, 1993
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
 *	@(#)ufs_lookup.c	8.6 (Berkeley) 4/1/94
 */
/* </LICENSES> */

/* <CODE> */
//! ext2fs directories: reading them as `struct dirent`s (`ext2fs_readdir`), looking a name up
//! and finding where a new entry would go (`ext2fs_lookup`, `ext2fs_search_dirblock`), and
//! the routines that check, add, remove and rewrite directory entries (`ext2fs_dirbadentry`,
//! `ext2fs_direnter`, `ext2fs_dirremove`, `ext2fs_dirrewrite`), test a directory for
//! emptiness and walk `..` to keep renames from making loops (`ext2fs_dirempty`,
//! `ext2fs_checkpath`).
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_lookup.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_lookup.rs@3c62ede68802
//!
//! The file is `ufs_lookup.c` for the ext2 directory format: an entry does not store the
//! terminating NUL of its name, so the on-disk entries cannot be handed out as `struct dirent`s
//! and are converted one by one (`ext2fs_dirconv2ffs`); the directory block is the file
//! system block (`e2fs_bsize`), not `DIRBLKSIZ`.
//!
//! ## Deviations
//! - A `struct ext2fs_direct *` into a directory block is an offset into the block's bytes,
//!   read and written by `ext2fs_dir.rs`'s accessors (`e2d_ino`, `e2d_reclen`, ...), which
//!   convert from and to little-endian; `memcpy` of an entry that may overlap is
//!   `copy_within`.
//! - `ext2fs_dirconv2ffs` writes the `struct dirent` as the bytes userland reads (native
//!   order) and returns its `d_reclen`. `ext2fs_readdir`'s conversion stops with `EIO` at an
//!   entry whose header or name would run past the bytes read: the C reads on into its
//!   buffer.
//! - `ext2fs_search_dirblock` takes the next entry from `offset` at the top of its loop. The
//!   C's `continue` after a mangled entry leaves `ep` where it was, so on a read-only mount
//!   (where `ufs_dirbad` does not panic) it would report the same entry forever; here the
//!   skip to the end of the block ends the block's search, as in `ufs_lookup`. A name whose
//!   bytes would run past the block does not match. It returns whether it found the name
//!   (the C's `*foundp`; its error is always 0, so `ext2fs_lookup` has no error to take from
//!   it).
//! - `ext2fs_dirbadentry` takes the super-block its caller holds (the C's
//!   `VTOI(dp)->i_e2fs`) and returns `false` (the C's 0: it panics on a bad entry);
//!   `ext2fs_dirempty` returns `bool`.
//! - `ext2fs_direnter` zeroes the new entry's name padding: the C copies the byte after the
//!   name (`cn_namelen + 1`) and leaves the rest of its stack `struct ext2fs_direct` as it
//!   was.
//! - `dirchk` is `ufs_lookup.rs`'s [`DIRCHK`], the C's `extern int dirchk`.
//!
//! ## Redesign
//! - N5.6c2: LZ's functions over `bsd`'s `VopLookupArgs`/`VopReaddirArgs`, lent buffers
//!   (`&'static Buf`, their bytes through an `unsafe` `BufLegacy::data`, five sites) and a
//!   `malloc(M_TEMP)` read buffer sliced through an `unsafe` view -> generic functions with
//!   the kernel's `Vnops<G>` method shapes (`ext2fs_lookup(vdp, vpp, cnp)`,
//!   `ext2fs_readdir(vp, uio, cred, eofflag)`), the directory block the busy buffer's token
//!   of `ext2fs_bufatoff` (read through `bytes()`, written through `data()`, given back by
//!   `vop_bwrite` or `brelse`), and the read buffer an owned `MArray<u8>` of `M_TEMP`, freed
//!   by its drop where the C frees it. The directory's inode is one `vtoi` handle per
//!   operation (the C's one `VTOI`), and the super-block is borrowed from it
//!   (`ip.i_e2fs()`). `VOP_ACCESS`, `VOP_READ`, `VOP_WRITE`, `VOP_BWRITE` are the kernel's
//!   `vop_*`; `VFS_VGET` the kernel's; `vn_rdwr` runs as `curproc` (`ProcGlue::with_curproc`).
//! - Cost against the C: the same reads of the same blocks, one `memset` of a `struct
//!   dirent` per entry; no lock, no spl.

use core::mem::{offset_of, size_of};
use core::sync::atomic::Ordering;

use hal::malloc::{M_WAITOK, MArray};
use hal::sys::dirent::{DT_UNKNOWN, Dirent, dirent_recsize};
use hal::sys::errno::Errno;
use hal::sys::mount::MNT_RDONLY;
use hal::sys::types::Off;

use super::ext2fs::{E2FS_REV0, EXT2F_INCOMPAT_FTYPE, Ext2Superblock};
use super::ext2fs_dir::{
    Doff, Ext2fsDirect, Ext2fsDirtemplate, Ext2fsSearchslot, Slotstatus, e2d_ino, e2d_name,
    e2d_namlen, e2d_reclen, e2iftodt, ext2fs_dirsiz, inot2ext2dt, set_e2d_ino, set_e2d_reclen,
    set_e2d_type,
};
use super::ext2fs_inode::{ext2fs_setsize, ext2fs_size, ext2fs_truncate};
use super::ext2fs_subr::ext2fs_bufatoff;
use crate::assert::kpanic;
use crate::file::FileGlue;
use crate::fs::bio::brelse;
use crate::fs::buf::BufRef;
use crate::fs::cache::{cache_enter, cache_lookup};
use crate::fs::getcwd::M_TEMP;
use crate::fs::mount::Mount;
use crate::fs::namei::{
    CREATE, Componentname, DELETE, ISDOTDOT, ISLASTCN, LOCKPARENT, LOOKUP, MAKEENTRY, PDIRUNLOCK,
    RENAME, SAVENAME, WANTPARENT,
};
use crate::fs::subr::{vput, vref};
use crate::fs::ufs::ufs::dinode::{ISVTX, ROOTINO, Ufsino};
use crate::fs::ufs::ufs::inode::{IN_CHANGE, IN_UPDATE, Inode, vtoi};
use crate::fs::ufs::ufs::ufs_lookup::{DIRCHK, ufs_dirbad};
use crate::fs::ufs::ufs::ufs_vnops::ucred;
use crate::fs::vnode::{IO_NODELOCKED, IO_SYNC, LK_EXCLUSIVE, LK_RETRY, VDIR, VEXEC, VWRITE};
use crate::fs::vnode::{Vnode, vn_lock};
use crate::fs::vnops::{RdwrBuf, vn_rdwr};
use crate::fs::vops::vop_write;
use crate::fs::vops::{Cred, IoBuf, VFS_VGET, vop_access, vop_bwrite, vop_read, vop_unlock};
use crate::prf::printf;
use crate::proc::ProcGlue;
use crate::uio::{Iovec, Uio, UioRw};

/// `MINDIRSIZ`: the first half of a `struct ext2fs_dirtemplate`, an entry whose name is at
/// most 4 bytes.
const MINDIRSIZ: usize = Ext2fsDirtemplate::SIZE / 2;

/// The bytes of a `struct dirent` (`<sys/dirent.h>`), in the order userland reads them.
type DirentBytes = [u8; size_of::<Dirent>()];

/// The vnode's mount, which an ext2fs vnode always has.
fn vmount<G: ProcGlue>(vp: &Vnode<G>) -> &'static Mount<G> {
    match vp.mount() {
        Some(mp) => mp,
        None => kpanic(format_args!("ext2fs: vnode {vp:p} without a mount")),
    }
}

/// Whether directory entries keep the file type: a revision 1 file system with the
/// `FILETYPE` incompatible feature (`ip->i_e2fs->e2fs.e2fs_rev > E2FS_REV0 &&
/// (e2fs_features_incompat & EXT2F_INCOMPAT_FTYPE)`).
pub(super) fn has_ftype(fs: &Ext2Superblock) -> bool {
    fs.e2fs_rev() > E2FS_REV0 && fs.e2fs_features_incompat() & EXT2F_INCOMPAT_FTYPE != 0
}

/// `ext2fs_dirconv2ffs`: the `struct dirent` of the on-disk entry at `off` in `b`, written
/// into `ffsdir` (all of it: the C's `memset` first); returns its `d_reclen`.
///
/// The problem that is tackled here is the fact that FFS includes the terminating zero on
/// disk while EXT2FS doesn't; this implies that we need to introduce some padding. For
/// instance, a filename "sbin" has normally a reclen 12 in EXT2, but 16 in FFS. This reminds
/// me of that Pepsi commercial: 'Kid saved a lousy nine cents...' If it wasn't for that, the
/// complete ufs code for directories would have worked w/o changes (except for the
/// difference in DIRBLKSIZ).
fn ext2fs_dirconv2ffs(b: &[u8], off: usize, ffsdir: &mut DirentBytes) -> u16 {
    ffsdir.fill(0);
    let namlen = e2d_namlen(b, off);
    ffsdir[offset_of!(Dirent, d_fileno)..][..8]
        .copy_from_slice(&u64::from(e2d_ino(b, off)).to_ne_bytes());
    ffsdir[offset_of!(Dirent, d_namlen)] = namlen;
    ffsdir[offset_of!(Dirent, d_type)] = DT_UNKNOWN; // don't know more here
    // XXX Right now this can't happen, but if one day MAXNAMLEN != E2FS_MAXNAMLEN we should
    // handle this more gracefully! (e2d_namlen is too small for such a comparison.)
    // strncpy(ffsdir->d_name, e2dir->e2d_name, ffsdir->d_namlen):
    let name = e2d_name(b, off, usize::from(namlen));
    let n = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    ffsdir[Dirent::NAME_OFFSET..][..n].copy_from_slice(&name[..n]);

    // Godmar thinks: since e2dir->e2d_reclen can be big and means nothing anyway, we compute
    // our own reclen according to what we think is right.
    let reclen = dirent_recsize(usize::from(namlen)) as u16;
    ffsdir[offset_of!(Dirent, d_reclen)..][..2].copy_from_slice(&reclen.to_ne_bytes());
    reclen
}

/// `ext2fs_readdir` (`vop_readdir`): vnode op for reading directories.
///
/// Convert the on-disk entries to `<sys/dirent.h>` entries. The problem is that the
/// conversion will blow up some entries by four bytes, so it can't be done in place. This is
/// too bad. Right now the conversion is done entry by entry, the converted entry is sent via
/// uiomove.
///
/// XXX allocate a buffer, convert as many entries as possible, then send the whole buffer to
/// uiomove
pub fn ext2fs_readdir<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    cred: Cred<'_>,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fs = ip.i_e2fs();
    let mut off: Off = uio.uio_offset;

    if vp.vtype() != VDIR {
        return Err(Errno::ENOTDIR);
    }

    let mut e2fs_count = uio.uio_resid;
    let entries =
        (uio.uio_offset as usize).wrapping_add(e2fs_count) & (fs.e2fs_bsize.get() as usize - 1);

    // Make sure we don't return partial entries.
    if e2fs_count <= entries {
        return Err(Errno::EINVAL);
    }

    e2fs_count -= entries;
    let Some(mut dirbuf) = MArray::new(M_TEMP, M_WAITOK, e2fs_count, |_| 0u8) else {
        kpanic(format_args!("ext2fs_readdir: no memory"));
    };

    let mut aiov = [Iovec::new()];
    let (mut error, auio_resid) = {
        let mut auio = Uio::kernel_read(&mut aiov, &mut dirbuf[..], uio.uio_procp);
        auio.uio_offset = uio.uio_offset;
        auio.uio_rw = uio.uio_rw;
        (vop_read(vp, &mut auio, 0, cred), auio.uio_resid)
    };
    if error.is_ok() {
        let readcnt = e2fs_count - auio_resid;
        let buf = &dirbuf[..readcnt];
        let mut dstd: DirentBytes = [0; size_of::<Dirent>()];
        let mut dp = 0usize;
        while dp < readcnt {
            let e2d_reclen = usize::from(e2d_reclen(buf, dp));
            if e2d_reclen == 0
                || dp + Ext2fsDirect::NAME_OFFSET + usize::from(e2d_namlen(buf, dp)) > readcnt
            {
                error = Err(Errno::EIO);
                break;
            }
            let d_reclen = usize::from(ext2fs_dirconv2ffs(buf, dp, &mut dstd));
            let d_namlen = usize::from(dstd[offset_of!(Dirent, d_namlen)]);
            if dstd[Dirent::NAME_OFFSET..][..d_namlen].contains(&b'/') {
                error = Err(Errno::EINVAL);
                break;
            }
            if d_reclen > uio.uio_resid {
                break;
            }
            let d_off = off + e2d_reclen as Off;
            dstd[offset_of!(Dirent, d_off)..][..8].copy_from_slice(&d_off.to_ne_bytes());
            error = <G::FileGlue as FileGlue>::uiomove(&mut dstd[..d_reclen], uio);
            if error.is_err() {
                break;
            }
            off += e2d_reclen as Off;
            // advance dp
            dp += e2d_reclen;
        }
        // we need to correct uio_offset
        uio.uio_offset = off;
    }
    drop(dirbuf);
    *eofflag = i32::from(ext2fs_size(&ip) as Off <= uio.uio_offset);
    error
}

/// `ext2fs_lookup` (`vop_lookup`): convert a component of a pathname into a pointer to a
/// locked inode. This is a very central and rather complicated routine. If the file system
/// is not maintained in a strict tree hierarchy, this can result in a deadlock situation (see
/// comments in code below).
///
/// The `cnp->cn_nameiop` argument is `LOOKUP`, `CREATE`, `RENAME`, or `DELETE` depending on
/// whether the name is to be looked up, created, renamed, or deleted. When `CREATE`, `RENAME`,
/// or `DELETE` is specified, information usable in creating, renaming, or deleting a
/// directory entry may be calculated. If flag has `LOCKPARENT` or'ed into it and the target
/// of the pathname exists, lookup returns both the target and its parent directory locked.
/// When creating or renaming and `LOCKPARENT` is specified, the target may not be ".". When
/// deleting and `LOCKPARENT` is specified, the target may be ".", but the caller must check
/// to ensure it does an `vrele` and `vput` instead of two `vput`s.
///
/// Overall outline of `ext2fs_lookup`:
///
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
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its labels kept as the exits of one body"
)]
pub fn ext2fs_lookup<G: ProcGlue>(
    vdp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Componentname<'_, G>,
) -> Result<(), Errno> {
    let cred = cnp.cn_cred;
    let flags = cnp.cn_flags;
    let nameiop = cnp.cn_nameiop;

    let mut ss = Ext2fsSearchslot {
        slotstatus: Slotstatus::Found,
        slotoffset: -1,
        slotfreespace: 0,
        slotsize: 0,
        slotneeded: 0,
    };

    let mut bp: Option<BufRef> = None;
    *vpp = None;
    let dp = vtoi(vdp);
    let fs = dp.i_e2fs();
    let dirblksize = fs.e2fs_bsize.get();
    let lockparent = flags & LOCKPARENT != 0;
    let wantparent = flags & (LOCKPARENT | WANTPARENT) != 0;

    // Check accessibility of directory.
    vop_access(vdp, VEXEC, cred, cnp.proc())?;

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

    // Suppress search for slots unless creating file and at end of pathname, in which case
    // we watch for a place to put the new file in case it doesn't already exist.
    if (nameiop == CREATE || nameiop == RENAME) && flags & ISLASTCN != 0 {
        ss.slotstatus = Slotstatus::None;
        ss.slotneeded = ext2fs_dirsiz(cnp.namelen() as usize) as i32;
    }

    // If there is cached information on a previous search of this directory, pick up where
    // we last left off. We cache only lookups as these are the most common and have the
    // greatest payoff. Caching CREATE has little benefit as it usually must search the entire
    // directory to determine that the entry does not exist. Caching the location of the last
    // DELETE or RENAME has not reduced profiling time and hence has been removed in the
    // interest of simplicity.
    // `VFSTOUFS(vdp->v_mount)->um_mountp` is `vdp->v_mount` itself (`ext2fs_mountfs` sets
    // `um_mountp` to its mount): read without a count on the lookup path.
    let bmask = vmount(vdp).iosize() as i32 - 1;
    let mut entryoffsetinblock: i32;
    let mut numdirpasses;
    if nameiop != LOOKUP || dp.i_diroff.get() == 0 || dp.i_diroff.get() as u64 > ext2fs_size(&dp) {
        dp.i_offset.set(0);
        numdirpasses = 1;
    } else {
        dp.i_offset.set(dp.i_diroff.get());
        entryoffsetinblock = dp.i_offset.get() & bmask;
        if entryoffsetinblock != 0 {
            let (b, _) = ext2fs_bufatoff(&dp, Off::from(dp.i_offset.get()))?;
            bp = Some(b);
        }
        numdirpasses = 2;
    }
    let mut prevoff: Doff = dp.i_offset.get();
    let mut endsearch = (ext2fs_size(&dp) as usize).next_multiple_of(dirblksize as usize) as Doff;
    let mut enduseful: Doff = 0;

    // The entry found: its offset in its block and its name length.
    let mut found: Option<(i32, u8)> = None;
    'searchloop: loop {
        while dp.i_offset.get() < endsearch {
            // If necessary, get the next directory block.
            if let Some(b) = bp.take() {
                brelse(b);
            }

            let (b, _) = ext2fs_bufatoff(&dp, Off::from(dp.i_offset.get()))?;
            entryoffsetinblock = 0;

            // If still looking for a slot, and at a dirblksize boundary, have to start looking
            // for free space again.
            if ss.slotstatus == Slotstatus::None {
                ss.slotoffset = -1;
                ss.slotfreespace = 0;
            }

            let data = b.bytes();
            if ext2fs_search_dirblock(
                &dp,
                data,
                cnp,
                &mut entryoffsetinblock,
                &mut prevoff,
                &mut enduseful,
                &mut ss,
            ) {
                let ep = (entryoffsetinblock & bmask) as usize;
                // foundentry:
                dp.i_ino.set(e2d_ino(data, ep));
                dp.i_reclen.set(u32::from(e2d_reclen(data, ep)));
                found = Some((entryoffsetinblock, e2d_namlen(data, ep)));
                bp = Some(b);
                break 'searchloop;
            }
            bp = Some(b);
        }
        // notfound:
        // If we started in the middle of the directory and failed to find our target, we
        // must check the beginning as well.
        if numdirpasses == 2 {
            numdirpasses -= 1;
            dp.i_offset.set(0);
            endsearch = dp.i_diroff.get();
            continue 'searchloop;
        }
        break;
    }

    let Some((entryoffsetinblock, namlen)) = found else {
        if let Some(b) = bp {
            brelse(b);
        }
        // If creating, and at end of pathname and current directory has not been removed,
        // then can consider allowing file to be created.
        if (nameiop == CREATE || nameiop == RENAME)
            && flags & ISLASTCN != 0
            && dp.i_e2din().i_e2fs_nlink() != 0
        {
            // Creation of files on a read-only mounted file system is pointless, so don't
            // proceed any further.
            if vmount(vdp).flag() & MNT_RDONLY != 0 {
                return Err(Errno::EROFS);
            }
            // Access for write is interpreted as allowing creation of files in the
            // directory.
            vop_access(vdp, VWRITE, cred, cnp.proc())?;
            // Return an indication of where the new directory entry should be put. If we
            // didn't find a slot, then set dp->i_count to 0 indicating that the new slot
            // belongs at the end of the directory. If we found a slot, then the new entry can
            // be put in the range from dp->i_offset to dp->i_offset + dp->i_count.
            if ss.slotstatus == Slotstatus::None {
                dp.i_offset
                    .set((ext2fs_size(&dp) as usize).next_multiple_of(dirblksize as usize) as Doff);
                dp.i_count.set(0);
                enduseful = dp.i_offset.get();
            } else {
                dp.i_offset.set(ss.slotoffset);
                dp.i_count.set(ss.slotsize);
                if enduseful < ss.slotoffset + ss.slotsize {
                    enduseful = ss.slotoffset + ss.slotsize;
                }
            }
            dp.i_endoff
                .set((enduseful as usize).next_multiple_of(dirblksize as usize) as Doff);
            dp.set_flag(IN_CHANGE | IN_UPDATE);
            // We return with the directory locked, so that the parameters we set up above
            // will still be valid if we actually decide to do a direnter(). We return ni_vp
            // == NULL to indicate that the entry does not currently exist; we leave a
            // pointer to the (locked) directory inode in ndp->ni_dvp. The pathname buffer is
            // saved so that the name can be obtained later.
            //
            // NB - if the directory is unlocked, then this information cannot be used.
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
    };

    // found:
    let Some(b) = bp else {
        kpanic(format_args!("ext2fs_lookup: no directory block"));
    };
    // Check that directory length properly reflects presence of this entry.
    let reach = entryoffsetinblock as u64 + ext2fs_dirsiz(usize::from(namlen)) as u64;
    if reach > ext2fs_size(&dp) {
        ufs_dirbad(&dp, dp.i_offset.get(), "i_size too small");
        if let Err(e) = ext2fs_setsize(&dp, reach) {
            brelse(b);
            return Err(e);
        }
        dp.set_flag(IN_CHANGE | IN_UPDATE);
    }
    brelse(b);

    // Found component in pathname. If the final component of path name, save information in
    // the cache as to where the entry was found.
    if flags & ISLASTCN != 0 && nameiop == LOOKUP {
        dp.i_diroff.set(dp.i_offset.get() & !(dirblksize - 1));
    }

    // If deleting, and at end of pathname, return parameters which can be used to remove
    // file. If the wantparent flag isn't set, we return only the directory (in
    // ndp->ni_dvp), otherwise we go on and lock the inode, being careful with ".".
    if nameiop == DELETE && flags & ISLASTCN != 0 {
        // Write access to directory required to delete files.
        vop_access(vdp, VWRITE, cred, cnp.proc())?;
        // Return pointer to current entry in dp->i_offset, and distance past previous entry
        // (if there is a previous entry in this block) in dp->i_count. Save directory inode
        // pointer in ndp->ni_dvp for dirremove().
        if dp.i_offset.get() & (dirblksize - 1) == 0 {
            dp.i_count.set(0);
        } else {
            dp.i_count.set(dp.i_offset.get() - prevoff);
        }
        if dp.i_number == dp.i_ino.get() {
            vref(vdp);
            *vpp = Some(vdp);
            return Ok(());
        }
        let tdp = VFS_VGET(vmount(vdp), u64::from(dp.i_ino.get()))?;
        // If directory is "sticky", then user must own the directory, or the file in it,
        // else she may not delete it (unless she's root). This implements append-only
        // directories.
        let uid = ucred(cred).cr_uid.get();
        if u32::from(dp.i_e2din().i_e2fs_mode()) & ISVTX != 0
            && uid != 0
            && uid != dp.i_e2din().i_e2fs_uid.get()
            && vtoi(tdp).i_e2din().i_e2fs_uid.get() != uid
        {
            vput(tdp);
            return Err(Errno::EPERM);
        }
        *vpp = Some(tdp);
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
        vop_access(vdp, VWRITE, cred, cnp.proc())?;
        // Careful about locking second inode. This can only occur if the target is ".".
        if dp.i_number == dp.i_ino.get() {
            return Err(Errno::EISDIR);
        }
        let tdp = VFS_VGET(vmount(vdp), u64::from(dp.i_ino.get()))?;
        *vpp = Some(tdp);
        cnp.cn_flags |= SAVENAME;
        if !lockparent {
            let _ = vop_unlock(vdp);
            cnp.cn_flags |= PDIRUNLOCK;
        }
        return Ok(());
    }

    // Step through the translation in the name. We do not `vput' the directory because we
    // may need it again if a symbolic link is relative to the current directory. Instead we
    // save it unlocked as "pdp". We must get the target inode before unlocking the directory
    // to insure that the inode will not be removed before we get it. We prevent deadlock by
    // always fetching inodes from the root, moving down the directory tree. Thus when
    // following backward pointers ".." we must unlock the parent directory before getting
    // the requested directory. There is a potential race condition here if both the current
    // and parent directories are removed before the VFS_VGET for the inode associated with
    // ".." returns. We hope that this occurs infrequently since we cannot avoid this race
    // condition without implementing a sophisticated deadlock detection algorithm. Note also
    // that this simple deadlock detection scheme will not work if the file system has any
    // hard links other than ".." that point backwards in the directory structure.
    let pdp = vdp;
    if flags & ISDOTDOT != 0 {
        let _ = vop_unlock(pdp); // race to get the inode
        cnp.cn_flags |= PDIRUNLOCK;
        let tdp = match VFS_VGET(vmount(vdp), u64::from(dp.i_ino.get())) {
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
    } else if dp.i_number == dp.i_ino.get() {
        vref(vdp); // we want ourself, ie "."
        *vpp = Some(vdp);
    } else {
        let tdp = VFS_VGET(vmount(vdp), u64::from(dp.i_ino.get()))?;
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

/// `ext2fs_search_dirblock`: look for the name of `cnp` in the directory block `data` of
/// `ip`, from `*entryoffsetinblockp` (and `ip->i_offset`, the same place in the file) on,
/// noting free space for a new entry in `*ssp` on the way. Whether the name is there (the
/// C's `*foundp`); on a match `*entryoffsetinblockp` is the entry's offset in the block.
pub fn ext2fs_search_dirblock<G: ProcGlue>(
    ip: &Inode<G>,
    data: &[u8],
    cnp: &Componentname<'_, G>,
    entryoffsetinblockp: &mut i32,
    prevoffp: &mut Doff,
    endusefulp: &mut Doff,
    ssp: &mut Ext2fsSearchslot,
) -> bool {
    let mut offset = *entryoffsetinblockp;
    let fs = ip.i_e2fs();
    let dirblksize = fs.e2fs_bsize.get();

    let lim = dirblksize - ext2fs_dirsiz(0) as i32;

    while offset < lim {
        let ep = offset as usize;
        let reclen = i32::from(e2d_reclen(data, ep));
        // Full validation checks are slow, so we only check enough to insure forward progress
        // through the directory. Complete checks can be run by patching "dirchk" to be true.
        if reclen == 0
            || (DIRCHK.load(Ordering::Relaxed) != 0 && ext2fs_dirbadentry(fs, data, offset))
        {
            ufs_dirbad(ip, ip.i_offset.get(), "mangled entry");
            let i = dirblksize - (offset & (dirblksize - 1));
            ip.i_offset.set(ip.i_offset.get() + i);
            offset += i;
            continue;
        }

        // If an appropriate sized slot has not yet been found, check to see if one is
        // available. Also accumulate space in the current block so that we can determine if
        // compaction is viable.
        if ssp.slotstatus != Slotstatus::Found {
            let mut size = reclen;

            if e2d_ino(data, ep) != 0 {
                size -= ext2fs_dirsiz(usize::from(e2d_namlen(data, ep))) as i32;
            }
            if size > 0 {
                if size >= ssp.slotneeded {
                    ssp.slotstatus = Slotstatus::Found;
                    ssp.slotoffset = ip.i_offset.get();
                    ssp.slotsize = reclen;
                } else if ssp.slotstatus == Slotstatus::None {
                    ssp.slotfreespace += size;
                    if ssp.slotoffset == -1 {
                        ssp.slotoffset = ip.i_offset.get();
                    }
                    if ssp.slotfreespace >= ssp.slotneeded {
                        ssp.slotstatus = Slotstatus::Compact;
                        ssp.slotsize = ip.i_offset.get() + reclen - ssp.slotoffset;
                    }
                }
            }
        }

        // Check for a name match.
        if e2d_ino(data, ep) != 0 {
            let namlen = usize::from(e2d_namlen(data, ep));
            let name = e2d_name(data, ep, namlen);
            if namlen as i64 == cnp.namelen() && name.len() == namlen && name == cnp.name() {
                // Save directory entry's inode number and reclen in ndp->ni_ufs area, and
                // release directory buffer.
                return true;
            }
        }
        *prevoffp = ip.i_offset.get();
        ip.i_offset.set(ip.i_offset.get() + reclen);
        offset += reclen;
        *entryoffsetinblockp = offset;
        if e2d_ino(data, ep) != 0 {
            *endusefulp = ip.i_offset.get();
        }
    }

    false
}

/// `ext2fs_dirbadentry`: do consistency checking on the directory entry at
/// `entryoffsetinblock` in `de`, a block of a directory on `fs` (changed so that it conforms
/// to Linux's `ext2fs_check_dir_entry`): record length must be a multiple of 4 and at least
/// the size of an entry with a one-byte name; record must be large enough to contain the
/// entry; entry must fit in the rest of its block; inode number must be in the file system.
/// A bad entry is reported and the system panics; `false` otherwise.
pub fn ext2fs_dirbadentry(fs: &Ext2Superblock, de: &[u8], entryoffsetinblock: i32) -> bool {
    let dirblksize = fs.e2fs_bsize.get();
    let off = entryoffsetinblock as usize;
    let reclen = i32::from(e2d_reclen(de, off));
    let namlen = e2d_namlen(de, off);

    let error_msg = if reclen < ext2fs_dirsiz(1) as i32 {
        // e2d_namlen = 1
        Some("rec_len is smaller than minimal")
    } else if reclen % 4 != 0 {
        Some("rec_len % 4 != 0")
    } else if reclen < ext2fs_dirsiz(usize::from(namlen)) as i32 {
        Some("reclen is too small for name_len")
    } else if entryoffsetinblock + reclen > dirblksize {
        Some("directory entry across blocks")
    } else if e2d_ino(de, off) > fs.e2fs_icount() {
        Some("inode out of bounds")
    } else {
        None
    };

    if let Some(msg) = error_msg {
        printf(format_args!(
            "bad directory entry: {}\noffset={}, inode={}, rec_len={}, name_len={} \n",
            msg,
            entryoffsetinblock,
            e2d_ino(de, off),
            reclen,
            namlen
        ));
        kpanic(format_args!("ext2fs_dirbadentry"));
    }
    false
}

/// `ext2fs_direnter`: write a directory entry after a call to namei, using the parameters
/// that it left in nameidata. The argument `ip` is the inode which the new directory entry
/// will refer to. `dvp` is a pointer to the directory to be written, which was left locked
/// by namei. Remaining parameters (`dp->i_offset`, `dp->i_count`) indicate how the space for
/// the new entry is to be obtained.
pub fn ext2fs_direnter<G: ProcGlue>(
    ip: &Inode<G>,
    dvp: &'static Vnode<G>,
    cnp: &Componentname<'_, G>,
) -> Result<(), Errno> {
    let fs = ip.i_e2fs();
    let dirblksize = fs.e2fs_bsize.get();

    #[cfg(feature = "diagnostic")]
    if cnp.cn_flags & SAVENAME == 0 {
        kpanic(format_args!("direnter: missing name"));
    }
    let dp = vtoi(dvp);
    let mut newdir = Ext2fsDirect::new();
    newdir.e2d_ino = ip.i_number;
    newdir.e2d_namlen = cnp.namelen() as u8;
    newdir.e2d_type = if has_ftype(fs) {
        inot2ext2dt(e2iftodt(ip.i_e2din().i_e2fs_mode()))
    } else {
        0
    };
    let name = cnp.name();
    newdir.e2d_name[..name.len()].copy_from_slice(name);
    let newentrysize = ext2fs_dirsiz(cnp.namelen() as usize);
    if dp.i_count.get() == 0 {
        // If dp->i_count is 0, then namei could find no space in the directory. Here,
        // dp->i_offset will be on a directory block boundary and we will write the new entry
        // into a fresh block.
        if dp.i_offset.get() & (dirblksize - 1) != 0 {
            kpanic(format_args!("ext2fs_direnter: newblk"));
        }
        newdir.e2d_reclen = dirblksize as u16;
        let mut entry = [0u8; size_of::<Ext2fsDirect>()];
        newdir.write_to(&mut entry, 0, newentrysize);
        let mut aiov = [Iovec::new()];
        let error = {
            let mut auio = Uio::<G>::kernel_write(&mut aiov, &entry[..newentrysize], None);
            auio.uio_offset = Off::from(dp.i_offset.get());
            vop_write(dvp, &mut auio, IO_SYNC, cnp.cn_cred)
        };
        if i64::from(dirblksize) > i64::from(vmount(dvp).bsize()) {
            // XXX should grow with balloc()
            kpanic(format_args!("ext2fs_direnter: frag size"));
        } else if error.is_ok() {
            ext2fs_setsize(
                &dp,
                (ext2fs_size(&dp) as usize).next_multiple_of(dirblksize as usize) as u64,
            )?;
            dp.set_flag(IN_CHANGE);
        }
        return error;
    }

    // If dp->i_count is non-zero, then namei found space for the new entry in the range
    // dp->i_offset to dp->i_offset + dp->i_count in the directory. To use this space, we may
    // have to compact the entries located there, by copying them together towards the
    // beginning of the block, leaving the free space in one usable chunk at the end.

    // Get the block containing the space for the new directory entry.
    let (mut bp, boff) = ext2fs_bufatoff(&dp, Off::from(dp.i_offset.get()))?;
    {
        let dirbuf = &mut bp.data()[boff..];
        // Find space for the new entry. In the simple case, the entry at offset base will
        // have the space. If it does not, then namei arranged that compacting the region
        // dp->i_offset to dp->i_offset + dp->i_count would yield the space.
        let mut ep = 0usize;
        let mut dsize = ext2fs_dirsiz(usize::from(e2d_namlen(dirbuf, ep)));
        let mut spacefree = i32::from(e2d_reclen(dirbuf, ep)) - dsize as i32;
        let mut loc = i32::from(e2d_reclen(dirbuf, ep));
        while loc < dp.i_count.get() {
            let nep = loc as usize;
            if e2d_ino(dirbuf, ep) != 0 {
                // trim the existing slot
                set_e2d_reclen(dirbuf, ep, dsize as u16);
                ep += dsize;
            } else {
                // overwrite; nothing there; header is ours
                spacefree += dsize as i32;
            }
            dsize = ext2fs_dirsiz(usize::from(e2d_namlen(dirbuf, nep)));
            spacefree += i32::from(e2d_reclen(dirbuf, nep)) - dsize as i32;
            loc += i32::from(e2d_reclen(dirbuf, nep));
            dirbuf.copy_within(nep..nep + dsize, ep);
        }
        // Update the pointer fields in the previous entry (if any), copy in the new entry,
        // and write out the block.
        if e2d_ino(dirbuf, ep) == 0 {
            #[cfg(feature = "diagnostic")]
            if spacefree + (dsize as i32) < newentrysize as i32 {
                kpanic(format_args!("ext2fs_direnter: compact1"));
            }
            newdir.e2d_reclen = (spacefree + dsize as i32) as u16;
        } else {
            #[cfg(feature = "diagnostic")]
            if spacefree < newentrysize as i32 {
                printf(format_args!(
                    "ext2fs_direnter: compact2 {} {}",
                    spacefree as u32, newentrysize
                ));
                kpanic(format_args!("ext2fs_direnter: compact2"));
            }
            newdir.e2d_reclen = spacefree as u16;
            set_e2d_reclen(dirbuf, ep, dsize as u16);
            ep += dsize;
        }
        newdir.write_to(dirbuf, ep, newentrysize);
    }
    let mut error = vop_bwrite::<G>(IoBuf::Token(bp));
    dp.set_flag(IN_CHANGE | IN_UPDATE);
    if error.is_ok() && dp.i_endoff.get() != 0 && (dp.i_endoff.get() as u64) < ext2fs_size(&dp) {
        error = ext2fs_truncate(&dp, Off::from(dp.i_endoff.get()), IO_SYNC, cnp.cn_cred);
    }
    error
}

/// `ext2fs_dirremove`: remove a directory entry after a call to namei, using the parameters
/// which it left in nameidata. The entry `dp->i_offset` contains the offset into the
/// directory of the entry to be eliminated. The `dp->i_count` field contains the size of the
/// previous record in the directory. If this is 0, the first entry is being deleted, so we
/// need only zero the inode number to mark the entry as free. If the entry is not the first
/// in the directory, we must reclaim the space of the now empty record by adding the record
/// size to the size of the previous entry.
pub fn ext2fs_dirremove<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    _cnp: &Componentname<'_, G>,
) -> Result<(), Errno> {
    let dp = vtoi(dvp);
    if dp.i_count.get() == 0 {
        // First entry in block: set d_ino to zero.
        let (mut bp, off) = ext2fs_bufatoff(&dp, Off::from(dp.i_offset.get()))?;
        set_e2d_ino(bp.data(), off, 0);
        let error = vop_bwrite::<G>(IoBuf::Token(bp));
        dp.set_flag(IN_CHANGE | IN_UPDATE);
        return error;
    }
    // Collapse new free space into previous entry.
    let (mut bp, off) = ext2fs_bufatoff(&dp, Off::from(dp.i_offset.get() - dp.i_count.get()))?;
    {
        let data = bp.data();
        let r = u32::from(e2d_reclen(data, off)) + dp.i_reclen.get();
        set_e2d_reclen(data, off, r as u16);
    }
    let error = vop_bwrite::<G>(IoBuf::Token(bp));
    dp.set_flag(IN_CHANGE | IN_UPDATE);
    error
}

/// `ext2fs_dirrewrite`: rewrite an existing directory entry to point at the inode supplied.
/// The parameters describing the directory entry are set up by a call to namei.
pub fn ext2fs_dirrewrite<G: ProcGlue>(
    dp: &Inode<G>,
    ip: &Inode<G>,
    _cnp: &Componentname<'_, G>,
) -> Result<(), Errno> {
    let (mut bp, off) = ext2fs_bufatoff(dp, Off::from(dp.i_offset.get()))?;
    {
        let data = bp.data();
        set_e2d_ino(data, off, ip.i_number);
        let t = if has_ftype(ip.i_e2fs()) {
            inot2ext2dt(e2iftodt(ip.i_e2din().i_e2fs_mode()))
        } else {
            0
        };
        set_e2d_type(data, off, t);
    }
    let error = vop_bwrite::<G>(IoBuf::Token(bp));
    dp.set_flag(IN_CHANGE | IN_UPDATE);
    error
}

/// `ext2fs_dirempty`: check if a directory is empty or not (only `.` and `..`, the latter
/// naming `parentino`). Inode supplied must be locked.
///
/// Using a struct dirtemplate here is not precisely what we want, but better than using a
/// struct ext2fs_direct.
///
/// NB: does not handle corrupted directories.
pub fn ext2fs_dirempty<G: ProcGlue>(ip: &Inode<G>, parentino: Ufsino, cred: Cred<'_>) -> bool {
    let mut dbuf = [0u8; Ext2fsDirtemplate::SIZE];
    let mut off: Off = 0;
    while (off as u64) < ext2fs_size(ip) {
        let mut count = 0usize;
        let error = G::with_curproc(|p| {
            vn_rdwr(
                UioRw::UIO_READ,
                ip.itov(),
                RdwrBuf::Kernel(&mut dbuf[..MINDIRSIZ]),
                off,
                IO_NODELOCKED,
                cred,
                Some(&mut count),
                p,
            )
        });
        // Since we read MINDIRSIZ, residual must be 0 unless we're at end of file.
        if error.is_err() || count != 0 {
            return false;
        }
        let reclen = e2d_reclen(&dbuf, 0);
        // avoid infinite loops
        if reclen == 0 {
            return false;
        }
        off += Off::from(reclen);
        // skip empty entries
        let ino = e2d_ino(&dbuf, 0);
        if ino == 0 {
            continue;
        }
        // accept only "." and ".."
        let namlen = e2d_namlen(&dbuf, 0);
        if namlen > 2 {
            return false;
        }
        let name = e2d_name(&dbuf, 0, 2);
        if name[0] != b'.' {
            return false;
        }
        // At this point namlen must be 1 or 2. 1 implies ".", 2 implies ".." if second char
        // is also "."
        if namlen == 1 {
            continue;
        }
        if name[1] == b'.' && ino == parentino {
            continue;
        }
        return false;
    }
    true
}

/// `ext2fs_checkpath`: check if source directory is in the path of the target directory.
/// Target is supplied locked, source is unlocked. The target is always vput before
/// returning.
pub fn ext2fs_checkpath<G: ProcGlue>(
    source: &Inode<G>,
    target: &Inode<G>,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let mut vp = Some(target.itov());
    let rootino = ROOTINO;
    let error: Result<(), Errno> = 'out: {
        if target.i_number == source.i_number {
            break 'out Err(Errno::EEXIST);
        }
        if target.i_number == rootino {
            break 'out Ok(());
        }

        loop {
            let Some(cur) = vp else {
                break 'out Ok(());
            };
            if cur.vtype() != VDIR {
                break 'out Err(Errno::ENOTDIR);
            }
            let mut dirbuf = [0u8; Ext2fsDirtemplate::SIZE];
            if let Err(e) = G::with_curproc(|p| {
                vn_rdwr(
                    UioRw::UIO_READ,
                    cur,
                    RdwrBuf::Kernel(&mut dirbuf),
                    0,
                    IO_NODELOCKED,
                    cred,
                    None,
                    p,
                )
            }) {
                break 'out Err(e);
            }
            let t = Ext2fsDirtemplate::from_le_bytes(&dirbuf);
            if t.dotdot_namlen != 2 || t.dotdot_name[0] != b'.' || t.dotdot_name[1] != b'.' {
                break 'out Err(Errno::ENOTDIR);
            }
            let ino = t.dotdot_ino;
            if ino == source.i_number {
                break 'out Err(Errno::EINVAL);
            }
            if ino == rootino {
                break 'out Ok(());
            }
            let mp = vmount(cur);
            vput(cur);
            match VFS_VGET(mp, u64::from(ino)) {
                Ok(next) => vp = Some(next),
                Err(e) => {
                    vp = None;
                    break 'out Err(e);
                }
            }
        }
    };

    // out:
    if error == Err(Errno::ENOTDIR) {
        printf(format_args!("checkpath: .. not a directory\n"));
        kpanic(format_args!("checkpath"));
    }
    if let Some(vp) = vp {
        vput(vp);
    }
    error
}

const _: () = {
    // The entry `ext2fs_readdir` converts fits the `struct dirent` it copies out.
    assert!(dirent_recsize(255) <= size_of::<Dirent>());
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the directory code: its pure parts (`ext2fs_dirconv2ffs`, the checks of
    // `ext2fs_dirbadentry` in the C's order) and, over the test image's root directory
    // (".", "..", "hello.txt", "big", "ext") through the fixture's vnodes (N5.6c3 repeats them
    // over the real mount), `ext2fs_readdir`'s cookies, its partial-entry `EINVAL` and the
    // bounds of an entry (`e2d_reclen`, `e2d_namlen`), and `ext2fs_lookup`'s three answers
    // (found, `ENOENT`, the `CREATE` slot) with `ext2fs_direnter` and `ext2fs_dirremove`.

    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::vec::Vec;
    use std::{assert, assert_eq, format};

    use hal::sys::mount::MNT_RDONLY;

    use super::super::testext2fs::newfs::{self, B, ROOTDIR};
    use super::super::testext2fs::{attach, mount, new_file, root};
    use super::*;
    use crate::fs::namei::DELETE;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ufs::testufs::cn;
    use crate::proc::Thread;
    use crate::testutil::printf_count;

    /// The image with the root directory's bytes at `off` replaced by `b`.
    fn patched(off: usize, b: &[u8]) -> Vec<u8> {
        let mut img = newfs::image(false);
        img[ROOTDIR * B + off..ROOTDIR * B + off + b.len()].copy_from_slice(b);
        img
    }

    /// `getdents` of `vp` from `offset` into a buffer of `len` bytes: the result, the
    /// entries' names and cookies (`d_off`), the new offset and the end-of-file flag.
    #[allow(clippy::type_complexity)]
    fn readdir(
        p: &Thread<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        offset: Off,
        len: usize,
    ) -> (Result<(), Errno>, Vec<(Vec<u8>, Off)>, Off, i32) {
        let mut buf = std::vec![0u8; len];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        uio.uio_offset = offset;
        let mut eof = 0;
        let r = ext2fs_readdir(vp, &mut uio, Cred::real(p.ucred()), &mut eof);
        let (off, done) = (uio.uio_offset, len - uio.uio_resid);
        let mut out = Vec::new();
        let mut at = 0;
        while at < done {
            let d = Dirent::from_bytes(&buf[at..]).unwrap();
            let name = buf[at + Dirent::NAME_OFFSET..][..usize::from(d.d_namlen)].to_vec();
            out.push((name, d.d_off));
            assert_eq!(
                usize::from(d.d_reclen),
                dirent_recsize(usize::from(d.d_namlen))
            );
            at += usize::from(d.d_reclen);
        }
        (r, out, off, eof)
    }

    #[test]
    fn dirconv2ffs_writes_the_dirent_userland_reads() {
        let mut b = [0u8; 32];
        b[0..4].copy_from_slice(&12u32.to_le_bytes());
        b[4..6].copy_from_slice(&20u16.to_le_bytes());
        b[6] = 9;
        b[7] = 1; // EXT2_FT_REG_FILE: not passed on (DT_UNKNOWN)
        b[8..17].copy_from_slice(b"hello.txt");
        let mut d = [0xa5u8; size_of::<Dirent>()];
        assert_eq!(ext2fs_dirconv2ffs(&b, 0, &mut d), 40);
        let e = Dirent::from_bytes(&d).unwrap();
        assert_eq!((e.d_fileno, e.d_off, e.d_reclen), (12, 0, 40));
        assert_eq!((e.d_type, e.d_namlen), (DT_UNKNOWN, 9));
        assert_eq!(&d[24..33], b"hello.txt");
        assert!(d[33..].iter().all(|&c| c == 0), "the C's memset");
        // strncpy stops at a NUL inside the name; d_namlen stays the entry's.
        b[8..12].copy_from_slice(b"ab\0c");
        b[6] = 4;
        assert_eq!(ext2fs_dirconv2ffs(&b, 0, &mut d), 32);
        assert_eq!((d[19], &d[24..28]), (4, &b"ab\0\0"[..]));
    }

    #[test]
    fn dirbadentry_reports_the_first_failed_check_in_the_c_order() {
        let fs = Ext2Superblock::new();
        fs.e2fs_bsize.set(1024);
        fs.set_e2fs_icount(100);
        let entry = |ino: u32, reclen: u16, namlen: u8| {
            let mut b = [0u8; 1024];
            for off in [0usize, 1016] {
                b[off..off + 4].copy_from_slice(&ino.to_le_bytes());
                b[off + 4..off + 6].copy_from_slice(&reclen.to_le_bytes());
                b[off + 6] = namlen;
            }
            b
        };
        // A good entry: the inode count itself is in bounds.
        assert!(!ext2fs_dirbadentry(&fs, &entry(100, 12, 1), 0));
        for (ino, reclen, namlen, off, msg) in [
            (7001, 8, 1, 0, "rec_len is smaller than minimal"),
            // 13 also fails "name_len" (DIRSIZ(9) is 20): the % 4 check comes first.
            (7002, 13, 9, 0, "rec_len % 4 != 0"),
            (7003, 16, 9, 0, "reclen is too small for name_len"),
            (7004, 12, 1, 1016, "directory entry across blocks"),
            (101, 12, 1, 0, "inode out of bounds"),
        ] {
            let b = entry(ino, reclen, namlen);
            let r = catch_unwind(AssertUnwindSafe(|| ext2fs_dirbadentry(&fs, &b, off)));
            assert!(r.is_err(), "{msg}: no panic");
            let line = format!(
                "bad directory entry: {msg}\noffset={off}, inode={ino}, rec_len={reclen}, name_len={namlen} \n"
            );
            assert!(printf_count(&line) >= 1, "{msg}: not reported");
        }
    }

    #[test]
    fn readdir_converts_the_root_directory_with_its_cookies() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(MNT_RDONLY);
        let vp = root(mp);
        let names: [&[u8]; 5] = [b".", b"..", b"hello.txt", b"big", b"ext"];
        let (r, ents, off, eof) = readdir(p, vp, 0, B);
        assert_eq!(r, Ok(()));
        let want: Vec<(Vec<u8>, Off)> = names
            .iter()
            .map(|n| n.to_vec())
            .zip([12, 24, 44, 56, B as Off])
            .collect();
        assert_eq!(ents, want);
        assert_eq!((off, eof), (B as Off, 1));
        // From a cookie: the rest of the block (the count less the bytes past the block).
        let (r, ents, off, eof) = readdir(p, vp, 24, B - 24);
        assert_eq!(r, Ok(()));
        assert_eq!(ents, want[2..]);
        assert_eq!((off, eof), (B as Off, 1));
        // A buffer that would end inside a block holds no whole entry: EINVAL, nothing moved.
        let (r, ents, off, _) = readdir(p, vp, 0, B / 2);
        assert_eq!((r, ents.len(), off), (Err(Errno::EINVAL), 0, 0));
        vput(vp);
    }

    #[test]
    fn readdir_stops_at_an_entry_with_no_length_with_eio() {
        // "big"'s e2d_reclen (at 44 + 4) zeroed: the three entries before it are copied out.
        let (_g, p) = attach(patched(48, &[0, 0]));
        let mp = mount(MNT_RDONLY);
        let vp = root(mp);
        let (r, ents, off, eof) = readdir(p, vp, 0, B);
        assert_eq!((r, ents.len(), off, eof), (Err(Errno::EIO), 3, 44, 0));
        vput(vp);
    }

    #[test]
    fn readdir_refuses_a_name_past_the_bytes_read_and_a_slash() {
        // "ext" shortened to end at 1014, where an entry's 20-byte name would run past the
        // block: EIO (LZ's bound; the C reads on), the offset after "ext".
        let mut tail = [0u8; 10];
        tail[0..4].copy_from_slice(&15u32.to_le_bytes());
        tail[4..6].copy_from_slice(&10u16.to_le_bytes());
        tail[6] = 20;
        let mut img = patched(60, &958u16.to_le_bytes());
        img[ROOTDIR * B + 1014..ROOTDIR * B + 1024].copy_from_slice(&tail);
        let (g, p) = attach(img);
        let mp = mount(MNT_RDONLY);
        let vp = root(mp);
        let (r, ents, off, _) = readdir(p, vp, 0, B);
        assert_eq!((r, ents.len(), off), (Err(Errno::EIO), 5, 1014));
        vput(vp);
        drop(g);
        // "big" renamed "b/g" on the disk: EINVAL at it.
        let (_g, p) = attach(patched(52, b"b/g"));
        let mp = mount(MNT_RDONLY);
        let vp = root(mp);
        let (r, ents, off, _) = readdir(p, vp, 0, B);
        assert_eq!((r, ents.len(), off), (Err(Errno::EINVAL), 3, 44));
        vput(vp);
    }

    #[test]
    fn lookup_finds_misses_and_makes_room_for_a_name() {
        let (_g, p) = attach(newfs::image(false));
        let cred = p.ucred();
        let mp = mount(0);
        // The file "new" will name (inode 15), made first: new_file locks the root itself.
        let nvp = new_file(mp, p);
        let dvp = root(mp);
        let dp = vtoi(dvp);
        // Found: the entry's inode, locked; the directory stays locked (LOCKPARENT).
        let mut c = cn(p, cred, LOOKUP, LOCKPARENT, b"hello.txt");
        let mut vpp = None;
        ext2fs_lookup(dvp, &mut vpp, &mut c).unwrap();
        let vp = vpp.unwrap();
        assert_eq!(vtoi(vp).i_number, 12);
        assert_eq!((dp.i_offset.get(), dp.i_reclen.get()), (24, 20));
        // The block of the entry is remembered for the next LOOKUP.
        assert_eq!(dp.i_diroff.get(), 0);
        vput(vp);
        // Missing.
        let mut c = cn(p, cred, LOOKUP, LOCKPARENT, b"nope");
        let mut vpp = None;
        assert_eq!(ext2fs_lookup(dvp, &mut vpp, &mut c), Err(Errno::ENOENT));
        assert!(vpp.is_none());
        // CREATE: the slot is "ext"'s, the only entry with room (968 - 12 bytes free).
        let mut c = cn(p, cred, CREATE, LOCKPARENT, b"new");
        let mut vpp = None;
        assert_eq!(
            ext2fs_lookup(dvp, &mut vpp, &mut c),
            Err(Errno::EJUSTRETURN)
        );
        assert_ne!(c.cn_flags & SAVENAME, 0);
        assert_eq!((dp.i_offset.get(), dp.i_count.get()), (56, 968));
        assert_eq!(dp.i_endoff.get(), B as i32);
        // Enter the new file there: "ext" is trimmed to 12 bytes, "new" takes the rest.
        let nip = vtoi(nvp);
        ext2fs_direnter(&nip, dvp, &c).unwrap();
        let (t, _) = ext2fs_bufatoff(&dp, 0).unwrap();
        let blk = t.bytes();
        assert_eq!(
            (e2d_reclen(blk, 56), e2d_reclen(blk, 68)),
            (12, (B - 68) as u16)
        );
        assert_eq!((e2d_ino(blk, 68), e2d_namlen(blk, 68)), (nip.i_number, 3));
        assert_eq!(e2d_name(blk, 68, 3), b"new");
        // The file type: the image has FILETYPE (EXT2_FT_REG_FILE).
        assert_eq!(blk[68 + 7], 1);
        brelse(t);
        drop(nip);
        vput(nvp);
        let mut c = cn(p, cred, LOOKUP, LOCKPARENT, b"new");
        let mut vpp = None;
        ext2fs_lookup(dvp, &mut vpp, &mut c).unwrap();
        vput(vpp.unwrap());
        // DELETE it: the previous entry ("ext") takes its room back.
        let mut c = cn(p, cred, DELETE, LOCKPARENT, b"new");
        let mut vpp = None;
        ext2fs_lookup(dvp, &mut vpp, &mut c).unwrap();
        assert_eq!((dp.i_offset.get(), dp.i_count.get()), (68, 12));
        ext2fs_dirremove(dvp, &c).unwrap();
        vput(vpp.unwrap());
        let (t, _) = ext2fs_bufatoff(&dp, 0).unwrap();
        assert_eq!(e2d_reclen(t.bytes(), 56), (B - 56) as u16);
        brelse(t);
        drop(dp);
        vput(dvp);
    }
}
/* </TESTS> */
