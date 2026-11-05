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
 * Copyright (C) 2005 Thomas Wang.
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
//! The vnode operations of the msdos file system (`msdosfs_vops`): create, open, close,
//! access, attributes, read and write, fsync, remove, rename, mkdir, rmdir, readdir (with
//! the Win95 long names), the denode lock, bmap and strategy (a file's clusters to the
//! disk's blocks), pathconf, advisory locks and the kqueue filters, and the table
//! ([`MsdosfsVnops`]).
//!
//! Upstream: sys/msdosfs/msdosfs_vnops.c @ 3ce1f3f79392
//! LZ: sys/msdosfs/msdosfs_vnops.rs@3c62ede68802
//!
//! In the ufs filesystem the inodes, superblocks, and indirect blocks are read/written using
//! the vnode for the filesystem. Blocks that represent the contents of a file are
//! read/written using the vnode for the file (including directories when they are
//! read/written as files). This presents problems for the dos filesystem because data that
//! should be in an inode (if dos had them) resides in the directory itself. Since we must
//! update directory entries without the benefit of having the vnode for the directory we must
//! use the vnode for the filesystem. This means that when a directory is actually
//! read/written (via read, write, or readdir, or seek) we must use the vnode for the
//! filesystem instead of the vnode for the directory as would happen in ufs. This is to
//! insure we retrieve the correct block from the buffer cache since the hash value is based
//! upon the vnode address and the desired block number.
//!
//! ## Deviations
//! - The `struct denode ndirent` that `msdosfs_create` and `msdosfs_mkdir` `bzero` on the
//!   stack is a `Denode::new` with no vnode there (msdosfs's `denode.rs`).
//! - `pool_put(&namei_pool, cnp->cn_pnbuf)` is the UFS layer's `pnbuf_free`; credentials the
//!   C dereferences (`cred->cr_uid`) go through its `ucred`, which panics on
//!   `NOCRED`/`FSCRED` where the C would follow a bad pointer.
//! - `getblk(..., 0, INFSLP)` "never fails" in C; here it is `bio::getblk_wait`, which
//!   repeats the call until it yields the buffer.
//! - The `goto`s of `msdosfs_create`, `msdosfs_write`, `msdosfs_rename`, `msdosfs_mkdir`,
//!   `msdosfs_rmdir` and `msdosfs_readdir` are labeled blocks; `msdosfs_rename`'s `bad`,
//!   `bad1` and `out` are the [`RenameExit`] its body ends with, `abortit` is the UFS
//!   layer's `rename_abortit` (the same label).
//! - `msdosfs_readdir` copies the `struct dirent` out as its bytes (`dirent_uiomove`, the
//!   same layout and the same `d_reclen` bytes as `uiomove(&dirbuf, dirbuf.d_reclen, uio)`).
//! - `dosdirtemplate` is a static of two `Direntry`s.
//! - `msdosfs_print` prints under feature `debug` or `diagnostic`; the C's third condition,
//!   `VFSLCKDEBUG`, has no feature here.
//! - The `MSDOSFS_DEBUG` `printf`s are left out: the option is not in GENERIC and has no
//!   feature here. The `DIAGNOSTIC` `HASBUF` checks are behind feature `diagnostic`.
//! - `msdosfs_vops`' `vop_revoke` is the generic `vop_generic_revoke`, `vop_abortop` and
//!   `vop_bwrite` the generic ones, as in the C's table; there is no special file table
//!   (msdosfs has no device files: `mknod` is `EINVAL`).
//! - The FAT12/16 root directory's size is its blocks' bytes (`msdosfs_denode.rs`, the
//!   user's decision D-N5.7-U4): `msdosfs_create`'s and `msdosfs_mkdir`'s `ENOSPC` for a full
//!   root, `msdosfs_readdir`'s end, `msdosfs_getattr`'s size and `msdosfs_read` of the root
//!   follow it; on sectors over 512 bytes the C's size overstates the root.
//!
//! ## Redesign
//! - N5.7c2: LZ's functions over `bsd`'s `struct vop_*_args` and its `Vops` static
//!   (`MSDOSFS_VOPS`) -> generic functions with the kernel's `Vnops<G>` method shapes (`vp`,
//!   the C's arguments, the out-parameters `&mut`) over the kernel's vnode, `vop_*` functions
//!   and the denode's handle (`vtode`, one per operation, the C's one `VTODE`), and the C's
//!   table as one type implementing the kernel's `Vnops<G>` (decision 46: the C's table is
//!   the trait): [`MsdosfsVnops`], written slot by slot with the C's 34 slots, no slot with a
//!   body the C lacks. The table is a member of the glue's `MsdosfsTables` (`vops`), one
//!   address, which the VFS's registry keys it by (`vops_id`; a promoted constant has no
//!   address of its own, D-b2-10); it carries a byte so that it is not zero-sized.
//! - The three `filterops` are `impl Filter<G>` for [`MsdosfsRead`], [`MsdosfsWrite`] and
//!   [`MsdosfsVnode`] (decision 46), their tables `MsdosfsFiltops::READ`/`WRITE`/`VNODE`
//!   made at compile time (`Filterops::of`), as the UFS layer's; a knote's vnode is its
//!   typed hook (`set_hook_vnode`, `hook_vnode`) where LZ cast `kn_hook` back (one `unsafe`).
//!   `filt_msdosfsread` reads the denode after its `NOTE_REVOKE` test: the vnode has no
//!   denode any more then (`vclean` reclaims it before its `NOTE_REVOKE`), where the C loads
//!   the NULL `v_data` and does not follow it.
//! - LZ's lent buffers and its `unsafe fn bdata` (seven `unsafe`) -> `fs::bio`'s `BufRef`
//!   tokens (`t.data()` writable, `t.bytes()` read-only, `t.clear()` for `clrbuf`, each token
//!   consumed by its `brelse`/`bwrite`/`bdwrite`/`bawrite`); `msdosfs_strategy` takes the
//!   buffer's token from the dispatch (`IoBuf::into_token`, a lent one is `panic(9)`: no
//!   `bsd` caller of `VOP_STRATEGY` names a msdosfs vnode) and finishes a hole's buffer with
//!   `biodone`, one section of the cache's lock (the C's `splbio(); biodone(bp); splx(s)`).
//!   LZ's `bread_cluster` of `sys::buf` is `fs::bio::bread_cluster`.
//! - LZ's private `vmount`, `rdonly`, `ucred`, `pnbuf_free`, `cn_proc` and `getblk_wait` are
//!   the UFS layer's and `bio`'s (one helper each).
//! - The denode lock is the denode's `RecursiveRwlock` (`msdosfs_lock` is `ufs_lock`'s
//!   shape), `msdosfs_advlock` the kernel's `lf_advlock` over `de_lockf`.
//! - Every arithmetic over an on-disk value keeps the C's widths: offsets and cluster numbers
//!   wrap at 32 bits where the C's `uint32_t` does (`cn + 1`, `de_FileSize - offset`, the
//!   readdir and read sizes `min`ed as `u_int`); `msdosfs_bmaparray` divides `MAXBSIZE` by the
//!   mount's `f_iosize`, as the C (a mount's statistics are set before any `bmap`).
//! - Cost against the C: the same calls in the same order; no lock and no spl of its own but
//!   `msdosfs_strategy`'s `biodone`. 0 `unsafe` (LZ: 8).

use core::marker::PhantomData;
use core::mem::offset_of;

use hal::malloc::MArc;
use hal::sys::dirent::{DT_DIR, DT_REG, Dirent, MAXNAMLEN, dirent_size};
use hal::sys::errno::Errno;
use hal::sys::fcntl::Flock;
use hal::sys::mount::{
    MNT_NOATIME, MNT_WAIT, MSDOSFSMNT_LONGNAME, MSDOSFSMNT_NOWIN95, MSDOSFSMNT_SHORTNAME,
};
use hal::sys::stat::{
    S_IFDIR, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR, S_IXGRP, S_IXOTH, S_IXUSR,
    SF_ARCHIVED, SF_SETTABLE,
};
use hal::sys::types::{Daddr, Dev, Gid, Mode, Nlink, Off, Register, Uid};
use hal::sys::unistd::{
    _PC_CHOWN_RESTRICTED, _PC_LINK_MAX, _PC_NAME_MAX, _PC_NO_TRUNC, _PC_TIMESTAMP_RESOLUTION,
};

use crate::assert::kpanic;
use crate::cred::{groupmember, suser_ucred};
use crate::event::{
    __EV_POLL, __EV_SELECT, EV_EOF, EV_ONESHOT, EVFILT_READ, EVFILT_VNODE, EVFILT_WRITE,
    FILTEROP_ISFD, Filter, Filterops, Knote, NOTE_ATTRIB, NOTE_DELETE, NOTE_EOF, NOTE_EXTEND,
    NOTE_LINK, NOTE_RENAME, NOTE_REVOKE, NOTE_TRUNCATE, NOTE_WRITE,
};
use crate::file::{FileGlue, LockOwner, foffset};
use crate::fs::bio::{
    bawrite, bdwrite, biodone, bread, bread_cluster, brelse, bwrite, getblk_wait,
};
use crate::fs::cache::cache_purge;
use crate::fs::default;
use crate::fs::lockf::lf_advlock;
use crate::fs::lookup::vfs_relookup;
use crate::fs::msdosfs::bpb::{getulong, getushort, putushort};
use crate::fs::msdosfs::denode::{
    DE_ACCESS, DE_CREATE, DE_MODIFIED, DE_RENAME, DE_UPDATE, Denode, DenodeRef, FC_LASTFC,
    MSDOSFS_FILESIZE_MAX, WIN_MAXLEN, detimes, vtode,
};
use crate::fs::msdosfs::direntry::{
    ATTR_ARCHIVE, ATTR_DIRECTORY, ATTR_READONLY, ATTR_VOLUME, ATTR_WIN95, CASE_LOWER_BASE,
    CASE_LOWER_EXT, Direntry, SLOT_DELETED, SLOT_EMPTY, WIN_LAST,
};
use crate::fs::msdosfs::fat::{MSDOSFSROOT, fat32};
use crate::fs::msdosfs::msdosfs_conv::{
    dos2unixfn, dos2unixtime, unix2dostime, win2unixfn, winChksum,
};
use crate::fs::msdosfs::msdosfs_denode::{
    deextend, detrunc, deupdat, msdosfs_inactive, msdosfs_reclaim, reinsert,
};
use crate::fs::msdosfs::msdosfs_fat::{clusteralloc, clusterfree, extendfile, pcbmap};
use crate::fs::msdosfs::msdosfs_lookup::{
    createde, doscheckpath, dosdirempty, entry_at, entry_put, msdosfs_lookup, removede,
    uniqdosname, winentry_at,
};
use crate::fs::msdosfs::msdosfsmount::{
    cntobn, de_clcount, de_cluster, de_cn2bn, de_cn2off, roottobn, vfstomsdosfs,
};
use crate::fs::namei::{ISDOTDOT, LOCKLEAF, LOCKPARENT, MODMASK, SAVESTART};
use crate::fs::subr::{vaccess, vflushbuf, vput, vrele};
use crate::fs::ufs::ufs::ufs_lookup::vmount;
use crate::fs::ufs::ufs::ufs_vnops::{Abort, pnbuf_free, rdonly, rename_abortit, ucred};
use crate::fs::vnode::{
    IO_APPEND, IO_SYNC, IO_UNIT, LK_EXCLUSIVE, LK_RETRY, LK_RWFLAGS, VA_UTIMES_CHANGE,
    VA_UTIMES_NULL, VBLK, VCHR, VDIR, VN_KNOTE, VNON, VNOVAL, VREG, VWRITE, Vattr, Vnode, vn_lock,
};
use crate::fs::vnops::vn_fsizechk;
use crate::fs::vops::{
    Cn, Cred, IoBuf, Vnops, vop_abortop, vop_access, vop_islocked, vop_strategy, vop_unlock,
};
use crate::proc::{ProcGlue, Thread};
use crate::tc::getnanotime;
use crate::uio::Uio;
use crate::vm::io::MAXBSIZE;
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

/// `sizeof(struct direntry)`.
const DIRENTRY_SIZE: u32 = Direntry::SIZE as u32;

/// How `msdosfs_rename` leaves its body: the C's labels it jumps to (or falls into).
#[derive(Clone, Copy, PartialEq, Eq)]
enum RenameExit {
    /// `bad:`: unlock the source, release its directory, then `bad1`.
    Bad,
    /// `bad1:`: release the target (if still held) and the target directory, then `out`.
    Bad1,
    /// `out:`: clear `DE_RENAME` and release the source.
    Out,
}

/// `msdosfs_vops`: the vnode operations vector of msdos vnodes (the glue's
/// `MsdosfsTables::vops`, one address).
pub struct MsdosfsVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member: the registry tells tables apart by
    /// their addresses.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> MsdosfsVnops<G> {
    /// The table (the glue's `MsdosfsTables::vops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `msdosfsread_filtops`'s filter: the bytes past the file offset.
pub struct MsdosfsRead;

/// `msdosfswrite_filtops`'s filter: a file is always writable.
pub struct MsdosfsWrite;

/// `msdosfsvnode_filtops`'s filter: the vnode events (`NOTE_*`) the user asked for.
pub struct MsdosfsVnode;

/// The tables of the three filters (`kn_fop`'s targets), made at compile time.
struct MsdosfsFiltops<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> MsdosfsFiltops<G> {
    /// `msdosfsread_filtops`.
    const READ: Filterops<G> = Filterops::of::<MsdosfsRead>();
    /// `msdosfswrite_filtops`.
    const WRITE: Filterops<G> = Filterops::of::<MsdosfsWrite>();
    /// `msdosfsvnode_filtops`.
    const VNODE: Filterops<G> = Filterops::of::<MsdosfsVnode>();
}

/// One entry of `dosdirtemplate`: a directory named `name`, lower case, with the C's
/// placeholder modification time and date (`{ 210, 4 }`).
const fn dosdirtemplate_entry(name: [u8; 8]) -> Direntry {
    Direntry {
        deName: name,
        deExtension: *b"   ",
        deAttributes: ATTR_DIRECTORY,
        deLowerCase: CASE_LOWER_BASE | CASE_LOWER_EXT,
        deCTimeHundredth: 0,
        deCTime: [0, 0],
        deCDate: [0, 0],
        deADate: [0, 0],
        deHighClust: [0, 0],
        deMTime: [210, 4],
        deMDate: [210, 4],
        deStartCluster: [0, 0],
        deFileSize: [0, 0, 0, 0],
    }
}

/// `dosdirtemplate`: the "." and ".." entries a new directory starts with.
static DOSDIRTEMPLATE: [Direntry; 2] = [
    dosdirtemplate_entry(*b".       "),
    dosdirtemplate_entry(*b"..      "),
];

/// `msdosfs_create` (`vop_create`): create a regular file.
///
/// On entry the directory to contain the file being created is locked; it stays locked. The
/// pathname buffer is freed always on error, or only if the `SAVESTART` bit in `cn_flags` is
/// clear on success.
pub fn msdosfs_create<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    let pdep = vtode(dvp);

    let error: Errno = 'bad: {
        // If this is the root directory and there is no space left we can't do anything.
        // This is because the root directory can not change size.
        if pdep.de_StartCluster.get() == MSDOSFSROOT
            && pdep.de_fndoffset.get() >= pdep.de_FileSize.get()
        {
            break 'bad Errno::ENOSPC;
        }

        // Create a directory entry for the file, then call createde() to have it installed.
        // NOTE: DOS files are always executable. We use the absence of the owner write bit
        // to make the file readonly.
        #[cfg(feature = "diagnostic")]
        if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
            kpanic(format_args!("msdosfs_create: no name"));
        }
        let ndirent = Denode::new(None, pdep.de_dev, pdep.de_pmp.clone(), 0, 0);
        let mut name = [0u8; 11];
        if let Err(e) = uniqdosname(&pdep, cnp, &mut name) {
            break 'bad e;
        }
        ndirent.set_name(&name);

        ndirent
            .de_Attributes
            .set(if vap.va_mode & VWRITE as Mode != 0 {
                ATTR_ARCHIVE
            } else {
                ATTR_ARCHIVE | ATTR_READONLY
            });
        ndirent.de_StartCluster.set(0);
        ndirent.de_FileSize.set(0);
        ndirent.de_devvp.set(pdep.de_devvp.get());
        ndirent.de_flag.set(DE_ACCESS | DE_CREATE | DE_UPDATE);
        let ts = getnanotime();
        detimes(&ndirent, &ts, &ts, &ts);
        let mut dep = None;
        if let Err(e) = createde(&ndirent, &pdep, Some(&mut dep), cnp) {
            break 'bad e;
        }
        let Some(dep) = dep else {
            kpanic(format_args!("msdosfs_create: no denode"));
        };
        if cnp.cn_flags & SAVESTART == 0 {
            pnbuf_free(cnp);
        }
        VN_KNOTE(dvp, NOTE_WRITE);
        *vpp = Some(dep.detov());
        return Ok(());
    };

    // bad:
    pnbuf_free(cnp);
    Err(error)
}

/// `msdosfs_mknod` (`vop_mknod`): DOS file systems have no special files.
pub fn msdosfs_mknod<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    pnbuf_free(cnp);
    VN_KNOTE(dvp, NOTE_WRITE);
    Err(Errno::EINVAL)
}

/// `msdosfs_open` (`vop_open`): nothing to do.
pub fn msdosfs_open() -> Result<(), Errno> {
    Ok(())
}

/// `msdosfs_close` (`vop_close`): update the times of a file others still use.
pub fn msdosfs_close<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let dep = vtode(vp);

    if vp.usecount() > 1 && vop_islocked(vp) == 0 {
        let ts = getnanotime();
        detimes(&dep, &ts, &ts, &ts);
    }
    Ok(())
}

/// `msdosfs_access` (`vop_access`): the permissions the attributes and the mount's mask give.
pub fn msdosfs_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let dep = vtode(vp);
    let pmp = dep.pmp();

    let mut dosmode: Mode = S_IRUSR | S_IRGRP | S_IROTH;
    if dep.de_Attributes.get() & ATTR_READONLY == 0 {
        dosmode |= S_IWUSR | S_IWGRP | S_IWOTH;
    }
    if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        dosmode |= S_IXUSR | S_IXGRP | S_IXOTH;
    }
    dosmode &= pmp.pm_mask.get();

    vaccess(
        vp.vtype(),
        dosmode,
        pmp.pm_uid.get(),
        pmp.pm_gid.get(),
        mode,
        ucred(cred),
    )
}

/// `msdosfs_getattr` (`vop_getattr`).
pub fn msdosfs_getattr<G: ProcGlue>(vp: &'static Vnode<G>, vap: &mut Vattr) -> Result<(), Errno> {
    let dep = vtode(vp);
    let pmp = dep.pmp();

    let ts = getnanotime();
    detimes(&dep, &ts, &ts, &ts);
    vap.va_fsid = i64::from(dep.de_dev);

    // The following computation of the fileid must be the same as that used in
    // msdosfs_readdir() to compute d_fileno. If not, pwd doesn't work.
    //
    // We now use the starting cluster number as the fileid/fileno. This works for both files
    // and directories (including the root directory, on FAT32). Even on FAT32, this will at
    // most be a 28-bit number, as the high 4 bits of FAT32 cluster numbers are reserved.
    //
    // However, we do need to do something for 0-length files, which will not have a starting
    // cluster number.
    //
    // These files cannot be directories, since (except for /, which is special-cased anyway)
    // directories contain entries for . and .., so must have non-zero length.
    //
    // In this case, we just create a non-cryptographic hash of the original fileid
    // calculation, and set the top bit.
    //
    // This algorithm has the benefit that all directories, and all non-zero-length files,
    // will have fileids that are persistent across mounts and reboots, and that cannot
    // collide (as long as the filesystem is not corrupt). Zero-length files will have fileids
    // that are persistent, but that may collide. We will just have to live with that.
    let mut fileid = dep.de_StartCluster.get();

    if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        // Special-case root
        if dep.de_StartCluster.get() == MSDOSFSROOT {
            fileid = if fat32(pmp) { pmp.pm_rootdirblk } else { 1 };
        }
    } else if dep.de_FileSize.get() == 0 {
        let dirsperblk = u32::from(pmp.pm_BytesPerSec()) / DIRENTRY_SIZE;

        let mut fileid64 = u64::from(if dep.de_dirclust.get() == MSDOSFSROOT {
            roottobn(pmp, 0u32)
        } else {
            cntobn(pmp, dep.de_dirclust.get())
        });
        fileid64 = fileid64.wrapping_mul(u64::from(dirsperblk));
        fileid64 = fileid64.wrapping_add(u64::from(dep.de_diroffset.get() / DIRENTRY_SIZE));

        fileid = fileidhash(fileid64);
    }

    vap.va_fileid = u64::from(fileid);
    vap.va_mode = S_IRUSR | S_IRGRP | S_IROTH;
    if dep.de_Attributes.get() & ATTR_READONLY == 0 {
        vap.va_mode |= S_IWUSR | S_IWGRP | S_IWOTH;
    }
    if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        vap.va_mode |= S_IFDIR;
        vap.va_mode |= if vap.va_mode & S_IRUSR != 0 {
            S_IXUSR
        } else {
            0
        };
        vap.va_mode |= if vap.va_mode & S_IRGRP != 0 {
            S_IXGRP
        } else {
            0
        };
        vap.va_mode |= if vap.va_mode & S_IROTH != 0 {
            S_IXOTH
        } else {
            0
        };
    }
    vap.va_mode &= pmp.pm_mask.get();
    vap.va_nlink = 1;
    vap.va_gid = pmp.pm_gid.get();
    vap.va_uid = pmp.pm_uid.get();
    vap.va_rdev = 0;
    vap.va_size = u64::from(dep.de_FileSize.get());
    vap.va_mtime = dos2unixtime(
        u32::from(dep.de_MDate.get()),
        u32::from(dep.de_MTime.get()),
        0,
    );
    if pmp.pm_flags.get() & MSDOSFSMNT_LONGNAME as u32 != 0 {
        vap.va_atime = dos2unixtime(u32::from(dep.de_ADate.get()), 0, 0);
        vap.va_ctime = dos2unixtime(
            u32::from(dep.de_CDate.get()),
            u32::from(dep.de_CTime.get()),
            u32::from(dep.de_CTimeHundredth.get()),
        );
    } else {
        vap.va_atime = vap.va_mtime;
        vap.va_ctime = vap.va_mtime;
    }
    vap.va_flags = 0;
    if dep.de_Attributes.get() & ATTR_ARCHIVE == 0 {
        vap.va_flags |= u64::from(SF_ARCHIVED);
    }
    vap.va_gen = 0;
    vap.va_blocksize = i64::from(pmp.pm_bpcluster);
    vap.va_bytes =
        u64::from(dep.de_FileSize.get().wrapping_add(pmp.pm_crbomask) & !pmp.pm_crbomask);
    vap.va_type = vp.vtype();
    Ok(())
}

/// `msdosfs_setattr` (`vop_setattr`): the attributes a DOS file has: the archive flag, the
/// size, the access and modification times, and the owner write bit (as the read-only
/// attribute). The owner and group can only be "changed" to the mount's.
pub fn msdosfs_setattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let dep = vtode(vp);
    let pmp = dep.pmp();

    if vap.va_type != VNON
        || vap.va_nlink != VNOVAL as Nlink
        || vap.va_fsid != i64::from(VNOVAL)
        || vap.va_fileid != VNOVAL as u64
        || vap.va_blocksize != i64::from(VNOVAL)
        || vap.va_rdev != VNOVAL as Dev
        || vap.va_bytes != VNOVAL as u64
        || vap.va_gen != VNOVAL as u64
    {
        return Err(Errno::EINVAL);
    }
    if vap.va_flags != VNOVAL as u64 {
        if rdonly(vp) {
            return Err(Errno::EINVAL);
        }
        let c = ucred(cred);
        if c.cr_uid.get() != pmp.pm_uid.get() {
            suser_ucred(c)?;
        }
        // We are very inconsistent about handling unsupported attributes. We ignored the
        // access time and the read and execute bits. We were strict for the other
        // attributes.
        //
        // Here we are strict, stricter than ufs in not allowing users to attempt to set
        // SF_SETTABLE bits or anyone to set unsupported bits. However, we ignore attempts to
        // set ATTR_ARCHIVE for directories `cp -pr' from a more sensible filesystem attempts
        // it a lot.
        if vap.va_flags & u64::from(SF_SETTABLE) != 0 {
            suser_ucred(c)?;
        }
        if vap.va_flags & !u64::from(SF_ARCHIVED) != 0 {
            return Err(Errno::EOPNOTSUPP);
        }
        if vap.va_flags & u64::from(SF_ARCHIVED) != 0 {
            dep.de_Attributes
                .set(dep.de_Attributes.get() & !ATTR_ARCHIVE);
        } else if dep.de_Attributes.get() & ATTR_DIRECTORY == 0 {
            dep.de_Attributes
                .set(dep.de_Attributes.get() | ATTR_ARCHIVE);
        }
        dep.set_flag(DE_MODIFIED);
    }

    if vap.va_uid != VNOVAL as Uid || vap.va_gid != VNOVAL as Gid {
        if rdonly(vp) {
            return Err(Errno::EINVAL);
        }
        let mut uid = vap.va_uid;
        if uid == VNOVAL as Uid {
            uid = pmp.pm_uid.get();
        }
        let mut gid = vap.va_gid;
        if gid == VNOVAL as Gid {
            gid = pmp.pm_gid.get();
        }
        let c = ucred(cred);
        if c.cr_uid.get() != pmp.pm_uid.get()
            || uid != pmp.pm_uid.get()
            || (gid != pmp.pm_gid.get() && !groupmember(gid, c))
        {
            suser_ucred(c)?;
        }
        if uid != pmp.pm_uid.get() || gid != pmp.pm_gid.get() {
            return Err(Errno::EINVAL);
        }
    }

    if vap.va_size != VNOVAL as u64 {
        match vp.vtype() {
            VDIR => return Err(Errno::EISDIR),
            // Truncation is only supported for regular files, Disallow it if the
            // filesystem is read-only.
            VREG if rdonly(vp) => return Err(Errno::EINVAL),
            _ => {
                // According to POSIX, the result is unspecified for file types other than
                // regular files, directories and shared memory objects. We don't support any
                // file types except regular files and directories in this file system, so
                // this (default) case is unreachable and can do anything. Keep falling
                // through to detrunc() for now.
            }
        }
        // The C's `detrunc(dep, vap->va_size, ..)`: the size converted to `uint32_t`.
        detrunc(&dep, vap.va_size as u32, 0, cred, Some(p))?;
    }
    if vap.va_vaflags & VA_UTIMES_CHANGE != 0
        || vap.va_atime.tv_nsec != i64::from(VNOVAL)
        || vap.va_mtime.tv_nsec != i64::from(VNOVAL)
    {
        if rdonly(vp) {
            return Err(Errno::EINVAL);
        }
        let c = ucred(cred);
        if c.cr_uid.get() != pmp.pm_uid.get()
            && let Err(e) = suser_ucred(c)
        {
            if vap.va_vaflags & VA_UTIMES_NULL == 0 {
                return Err(e);
            }
            vop_access(vp, VWRITE, cred, p)?;
        }
        if vp.vtype() != VDIR {
            if pmp.pm_flags.get() & MSDOSFSMNT_NOWIN95 as u32 == 0
                && vap.va_atime.tv_nsec != i64::from(VNOVAL)
            {
                dep.clr_flag(DE_ACCESS);
                let (dd, _, _) = unix2dostime(&vap.va_atime);
                dep.de_ADate.set(dd);
            }
            if vap.va_mtime.tv_nsec != i64::from(VNOVAL) {
                dep.clr_flag(DE_UPDATE);
                let (dd, dt, _) = unix2dostime(&vap.va_mtime);
                dep.de_MDate.set(dd);
                dep.de_MTime.set(dt);
            }
            dep.de_Attributes
                .set(dep.de_Attributes.get() | ATTR_ARCHIVE);
            dep.set_flag(DE_MODIFIED);
        }
    }
    // DOS files only have the ability to have their writability attribute set, so we use
    // the owner write bit to set the readonly attribute.
    if vap.va_mode != VNOVAL as Mode {
        if rdonly(vp) {
            return Err(Errno::EINVAL);
        }
        let c = ucred(cred);
        if c.cr_uid.get() != pmp.pm_uid.get() {
            suser_ucred(c)?;
        }
        if vp.vtype() != VDIR {
            // We ignore the read and execute bits.
            if vap.va_mode & VWRITE as Mode != 0 {
                dep.de_Attributes
                    .set(dep.de_Attributes.get() & !ATTR_READONLY);
            } else {
                dep.de_Attributes
                    .set(dep.de_Attributes.get() | ATTR_READONLY);
            }
            dep.de_Attributes
                .set(dep.de_Attributes.get() | ATTR_ARCHIVE);
            dep.set_flag(DE_MODIFIED);
        }
    }
    VN_KNOTE(vp, NOTE_ATTRIB);
    deupdat(&dep, 1)
}

/// `msdosfs_read` (`vop_read`): read a file, or a directory as a file (through the
/// file system's device vnode, see the module's documentation).
pub fn msdosfs_read<G: ProcGlue>(vp: &'static Vnode<G>, uio: &mut Uio<'_, G>) -> Result<(), Errno> {
    let dep = vtode(vp);
    let pmp = dep.pmp();
    let bpcluster = pmp.pm_bpcluster;

    // If they didn't ask for any data, then we are done.
    if uio.uio_resid == 0 {
        return Ok(());
    }
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }

    let isadir = dep.de_Attributes.get() & ATTR_DIRECTORY != 0;
    let mut error;
    loop {
        if uio.uio_offset >= i64::from(dep.de_FileSize.get()) {
            return Ok(());
        }

        let cn = de_cluster(pmp, uio.uio_offset) as u32;
        let mut size = bpcluster as i32;
        let on = uio.uio_offset as u32 & pmp.pm_crbomask;
        let mut n = u64::from(bpcluster - on).min(uio.uio_resid as u64) as u32;

        // de_FileSize is uint32_t, and we know that uio_offset < de_FileSize, so
        // uio->uio_offset < 2^32. Therefore the cast to uint32_t on the next line is safe.
        let diff = dep.de_FileSize.get().wrapping_sub(uio.uio_offset as u32);
        if diff < n {
            n = diff;
        }

        // If we are operating on a directory file then be sure to do i/o with the vnode for
        // the filesystem instead of the vnode for the directory.
        let (mut bp, r) = if isadir {
            // convert cluster # to block #
            let mut bn: Daddr = 0;
            pcbmap(&dep, cn, Some(&mut bn), None, Some(&mut size))?;
            bread(&pmp.pm_devvp.bufs, bn, size)
        } else if de_cn2off(pmp, cn.wrapping_add(1)) >= dep.de_FileSize.get() {
            bread(&vp.bufs, Daddr::from(cn), size)
        } else {
            bread_cluster(&vp.bufs, Daddr::from(cn), size)
        };
        n = n.min(bpcluster.wrapping_sub(bp.buf().resid() as u32));
        if let Err(e) = r {
            brelse(bp);
            return Err(e);
        }
        error =
            <G::FileGlue as FileGlue>::uiomove(&mut bp.data()[on as usize..(on + n) as usize], uio);
        brelse(bp);
        if !(error.is_ok() && uio.uio_resid > 0 && n != 0) {
            break;
        }
    }
    if !isadir && vmount(vp).flag() & MNT_NOATIME == 0 {
        dep.set_flag(DE_ACCESS);
    }
    error
}

/// `msdosfs_write` (`vop_write`): write data to a file.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its labels kept as the exits of one body"
)]
pub fn msdosfs_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let mut extended = false;
    let dep = vtode(vp);
    let pmp = dep.pmp();
    let bpcluster = pmp.pm_bpcluster;

    let thisvp = match vp.vtype() {
        VREG => {
            if ioflag & IO_APPEND != 0 {
                uio.uio_offset = i64::from(dep.de_FileSize.get());
            }
            vp
        }
        VDIR => return Err(Errno::EISDIR),
        _ => kpanic(format_args!("msdosfs_write(): bad file type")),
    };

    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }

    if uio.uio_resid == 0 {
        return Ok(());
    }

    // Don't bother to try to write files larger than the f/s limit
    if uio.uio_offset > MSDOSFS_FILESIZE_MAX
        || uio.uio_resid as u64 > (MSDOSFS_FILESIZE_MAX - uio.uio_offset) as u64
    {
        return Err(Errno::EFBIG);
    }

    // do the filesize rlimit check
    let p = uio.uio_procp.and_then(G::thread_pin);
    let overrun = vn_fsizechk(vp, uio, ioflag, p)?;

    let error: Result<(), Errno> = 'out: {
        // If the offset we are starting the write at is beyond the end of the file, then
        // they've done a seek. Unix filesystems allow files with holes in them, DOS doesn't so
        // we must fill the hole with zeroed blocks.
        if uio.uio_offset > i64::from(dep.de_FileSize.get())
            && let Err(e) = deextend(&dep, uio.uio_offset as u32, cred)
        {
            break 'out Err(e);
        }

        // Remember some values in case the write fails.
        let resid = uio.uio_resid;
        let osize = dep.de_FileSize.get();
        let mut error: Result<(), Errno> = Ok(());

        'errexit: {
            // If we write beyond the end of the file, extend it to its ultimate size ahead of
            // the time to hopefully get a contiguous area.
            let end = (uio.uio_offset as u64).wrapping_add(resid as u64);
            let lastcn = if end > u64::from(osize) {
                extended = true;
                let count =
                    de_clcount(pmp, end).wrapping_sub(u64::from(de_clcount(pmp, osize))) as u32;
                error = extendfile(&dep, count, None, None, 0);
                if let Err(e) = error
                    && (e != Errno::ENOSPC || ioflag & IO_UNIT != 0)
                {
                    break 'errexit;
                }
                dep.fc(FC_LASTFC).fc_frcn
            } else {
                de_clcount(pmp, osize).wrapping_sub(1)
            };

            loop {
                let croffset = uio.uio_offset as u32 & pmp.pm_crbomask;
                let cn = de_cluster(pmp, uio.uio_offset) as u32;

                if cn > lastcn {
                    error = Err(Errno::ENOSPC);
                    break;
                }

                let end = (uio.uio_offset as u64).wrapping_add(uio.uio_resid as u64);
                let mut bp = if croffset == 0
                    && (de_cluster(pmp, end) > u64::from(cn)
                        || end >= u64::from(dep.de_FileSize.get()))
                {
                    // If either the whole cluster gets written, or we write the cluster from
                    // its start beyond EOF, then no need to read data from disk.
                    let mut bp = getblk_wait(&thisvp.bufs, Daddr::from(cn), bpcluster as i32);
                    bp.clear();
                    // Do the bmap now, since pcbmap needs buffers for the fat table. (see
                    // msdosfs_strategy)
                    let b = bp.buf();
                    if b.blkno() == b.lblkno() {
                        let mut blkno = b.blkno();
                        error = pcbmap(&dep, b.lblkno() as u32, Some(&mut blkno), None, None);
                        b.set_blkno(if error.is_err() { -1 } else { blkno });
                    }
                    if bp.buf().blkno() == -1 {
                        brelse(bp);
                        if error.is_ok() {
                            error = Err(Errno::EIO); // XXX
                        }
                        break;
                    }
                    bp
                } else {
                    // The block we need to write into exists, so read it in.
                    let (bp, r) = bread(&thisvp.bufs, Daddr::from(cn), bpcluster as i32);
                    error = r;
                    if error.is_err() {
                        brelse(bp);
                        break;
                    }
                    bp
                };

                let n = (uio.uio_resid as u64).min(u64::from(bpcluster - croffset)) as u32;
                if uio.uio_offset + i64::from(n) > i64::from(dep.de_FileSize.get()) {
                    dep.de_FileSize.set((uio.uio_offset + i64::from(n)) as u32);
                    uvm_vnp_setsize(vp, i64::from(dep.de_FileSize.get()));
                }
                let _ = uvm_vnp_uncache(vp);
                // Should these vnode_pager_* functions be done on dir files?

                // Copy the data from user space into the buf header.
                error = <G::FileGlue as FileGlue>::uiomove(
                    &mut bp.data()[croffset as usize..(croffset + n) as usize],
                    uio,
                );

                // If they want this synchronous then write it and wait for it. Otherwise, if
                // on a cluster boundary write it asynchronously so we can move on to the next
                // block without delay. Otherwise do a delayed write because we may want to
                // write some more into the block later.
                // (`#if 0`: IO_NOCACHE sets B_NOCACHE.)
                if ioflag & IO_SYNC != 0 {
                    let _ = bwrite(bp);
                } else if n + croffset == bpcluster {
                    bawrite(bp);
                } else {
                    bdwrite(bp);
                }
                dep.set_flag(DE_UPDATE);
                if !(error.is_ok() && uio.uio_resid > 0) {
                    break;
                }
            }

            if resid > uio.uio_resid {
                VN_KNOTE(vp, NOTE_WRITE | if extended { NOTE_EXTEND } else { 0 });
            }

            if dep.de_FileSize.get() < osize {
                VN_KNOTE(vp, NOTE_TRUNCATE);
            }
        }

        // errexit: If the write failed and they want us to, truncate the file back to the
        // size it was before the write was attempted.
        if error.is_err() {
            if ioflag & IO_UNIT != 0 {
                let _ =
                    G::with_curproc(|p| detrunc(&dep, osize, ioflag & IO_SYNC, Cred::NOCRED, p));
                uio.uio_offset -= (resid - uio.uio_resid) as Off;
                uio.uio_resid = resid;
            } else {
                let _ = G::with_curproc(|p| {
                    detrunc(
                        &dep,
                        dep.de_FileSize.get(),
                        ioflag & IO_SYNC,
                        Cred::NOCRED,
                        p,
                    )
                });
                if uio.uio_resid != resid {
                    error = Ok(());
                }
            }
        } else if ioflag & IO_SYNC != 0 {
            error = deupdat(&dep, 1);
        }
        error
    };

    // out: correct the result for writes clamped by vn_fsizechk()
    uio.uio_resid += overrun;
    error
}

/// `msdosfs_ioctl` (`vop_ioctl`): no ioctls on msdos files.
pub fn msdosfs_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `msdosfs_fsync` (`vop_fsync`): flush the blocks of a file to disk.
///
/// This function is worthless for vnodes that represent directories. Maybe we could just do
/// a sync if they try an fsync on a directory file.
pub fn msdosfs_fsync<G: ProcGlue>(vp: &'static Vnode<G>, waitfor: i32) -> Result<(), Errno> {
    vflushbuf(vp, waitfor == MNT_WAIT);
    deupdat(&vtode(vp), i32::from(waitfor == MNT_WAIT))
}

/// `msdosfs_remove` (`vop_remove`): remove the entry of a file that is not a directory.
pub fn msdosfs_remove<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
) -> Result<(), Errno> {
    let dep = vtode(vp);
    let ddep = vtode(dvp);

    let error = if vp.vtype() == VDIR {
        Err(Errno::EPERM)
    } else {
        removede(&ddep, &dep)
    };

    VN_KNOTE(vp, NOTE_DELETE);
    VN_KNOTE(dvp, NOTE_WRITE);

    error
}

/// `msdosfs_link` (`vop_link`): DOS filesystems don't know what links are. But since we
/// already called `msdosfs_lookup()` with create and lockparent, the parent is locked so we
/// have to free it before we return the error.
pub fn msdosfs_link<G: ProcGlue>(dvp: &'static Vnode<G>, cnp: &mut Cn<'_, G>) -> Result<(), Errno> {
    let _ = vop_abortop(dvp, cnp);
    vput(dvp);
    Err(Errno::EOPNOTSUPP)
}

/// `msdosfs_rename` (`vop_rename`).
///
/// Renames on files require moving the denode to a new hash queue since the denode's
/// location is used to compute which hash queue to put the file in. Unless it is a rename in
/// place. For example "mv a b".
///
/// What follows is the basic algorithm:
///
/// ```text
/// if (file move) {
///     if (dest file exists) {
///         remove dest file
///     }
///     if (dest and src in same directory) {
///         rewrite name in existing directory slot
///     } else {
///         write new entry in dest directory
///         update offset and dirclust in denode
///         move denode to new hash chain
///         clear old directory entry
///     }
/// } else {
///     directory move
///     if (dest directory exists) {
///         if (dest is not empty) {
///             return ENOTEMPTY
///         }
///         remove dest directory
///     }
///     if (dest and src in same directory) {
///         rewrite name in existing entry
///     } else {
///         be sure dest is not a child of src directory
///         write entry in dest directory
///         update "." and ".." in moved directory
///         update offset and dirclust in denode
///         move denode to new hash chain
///         clear old directory entry for moved directory
///     }
/// }
/// ```
///
/// On entry: source's parent directory is unlocked; source file or directory is unlocked;
/// destination's parent directory is locked; destination file or directory is locked if it
/// exists. On exit: all denodes should be released.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its labels kept as the exits of one body"
)]
pub fn msdosfs_rename<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    fcnp: &mut Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
    tcnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let mut tvp = tvp;
    let a_fvp = fvp;
    let mut fvp = fvp;
    let mut doingdirectory = false;
    let mut newparent = false;
    // `goto abortit`: abort both lookups, release every vnode, return the error.
    macro_rules! abortit {
        ($e:expr) => {
            return rename_abortit(
                $e,
                Abort {
                    tdvp,
                    tvp,
                    tcnp,
                    fdvp,
                    fvp,
                },
                fcnp,
            )
        };
    }

    let pmp = vfstomsdosfs(vmount(fdvp));

    #[cfg(feature = "diagnostic")]
    if tcnp.cn_flags & crate::fs::namei::HASBUF == 0
        || fcnp.cn_flags & crate::fs::namei::HASBUF == 0
    {
        kpanic(format_args!("msdosfs_rename: no name"));
    }
    // Check for cross-device rename.
    let fmp = fvp.mount().map(core::ptr::from_ref);
    if fmp != tdvp.mount().map(core::ptr::from_ref)
        || tvp.is_some_and(|t| fmp != t.mount().map(core::ptr::from_ref))
    {
        abortit!(Err(Errno::EXDEV));
    }

    // If source and dest are the same, do nothing.
    if tvp.is_some_and(|t| core::ptr::eq(t, fvp)) {
        abortit!(Ok(()));
    }

    if let Err(e) = vn_lock(fvp, LK_EXCLUSIVE | LK_RETRY) {
        abortit!(Err(e));
    }
    let dp = vtode(fdvp);
    let ip = vtode(fvp);

    // Be sure we are not renaming ".", "..", or an alias of ".". This leads to a crippled
    // directory tree. It's pretty tough to do a "ls" or "pwd" with the "." directory entry
    // missing, and "cd .." doesn't work if the ".." entry is missing.
    if ip.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        // Avoid ".", "..", and aliases of "." for obvious reasons.
        if (fcnp.namelen() == 1 && fcnp.name().first() == Some(&b'.'))
            || MArc::ptr_eq(&dp, &ip)
            || fcnp.cn_flags & ISDOTDOT != 0
            || tcnp.cn_flags & ISDOTDOT != 0
            || ip.de_flag.get() & DE_RENAME != 0
        {
            let _ = vop_unlock(fvp);
            abortit!(Err(Errno::EINVAL));
        }
        ip.set_flag(DE_RENAME);
        doingdirectory = true;
    }
    VN_KNOTE(fdvp, NOTE_WRITE); // XXX right place?

    // When the target exists, both the directory and target vnodes are returned locked.
    let mut dp = vtode(tdvp);
    let mut xp: Option<DenodeRef<G>> = tvp.map(vtode);
    // Remember direntry place to use for destination
    let to_diroffset = dp.de_fndoffset.get();
    let to_count = dp.de_fndcnt.get();

    // If ".." must be changed (ie the directory gets a new parent) then the source directory
    // must not be in the directory hierarchy above the target, as this would orphan
    // everything below the source directory. Also the user must have write permission in the
    // source so as to be able to change "..". We must repeat the call to namei, as the parent
    // directory is unlocked by the call to doscheckpath().
    let mut error = vop_access(fvp, VWRITE, tcnp.cn_cred, tcnp.proc());
    let _ = vop_unlock(fvp);
    if vtode(fdvp).de_StartCluster.get() != vtode(tdvp).de_StartCluster.get() {
        newparent = true;
    }
    vrele(fdvp);

    let exit: RenameExit = 'body: {
        if doingdirectory && newparent {
            if error.is_err() {
                // write access check above
                break 'body RenameExit::Bad1;
            }
            if xp.is_some()
                && let Some(t) = tvp
            {
                vput(t);
            }
            // doscheckpath() vput()'s dp, so we have to do a relookup afterwards
            error = doscheckpath(&ip, dp);
            if error.is_err() {
                break 'body RenameExit::Out;
            }
            if tcnp.cn_flags & SAVESTART == 0 {
                kpanic(format_args!("msdosfs_rename: lost to startdir"));
            }
            let mut ntvp = None;
            error = vfs_relookup(tdvp, &mut ntvp, tcnp);
            if error.is_err() {
                break 'body RenameExit::Out;
            }
            tvp = ntvp;
            dp = vtode(tdvp);
            xp = tvp.map(vtode);
        }

        VN_KNOTE(tdvp, NOTE_WRITE);

        if let Some(x) = xp.take() {
            // Target must be empty if a directory and have no links to it. Also, ensure
            // source and target are compatible (both directories, or both not directories).
            if x.de_Attributes.get() & ATTR_DIRECTORY != 0 {
                if !dosdirempty(&x) {
                    error = Err(Errno::ENOTEMPTY);
                    xp = Some(x);
                    break 'body RenameExit::Bad1;
                }
                if !doingdirectory {
                    error = Err(Errno::ENOTDIR);
                    xp = Some(x);
                    break 'body RenameExit::Bad1;
                }
                cache_purge(tdvp);
            } else if doingdirectory {
                error = Err(Errno::EISDIR);
                xp = Some(x);
                break 'body RenameExit::Bad1;
            }
            error = removede(&dp, &x);
            if error.is_err() {
                xp = Some(x);
                break 'body RenameExit::Bad1;
            }
            let t = x.detov();
            drop(x);
            VN_KNOTE(t, NOTE_DELETE);
            vput(t);
        }

        // Convert the filename in tcnp into a dos filename. We copy this into the denode and
        // directory entry for the destination file/directory.
        let mut toname = [0u8; 11];
        error = uniqdosname(&vtode(tdvp), tcnp, &mut toname);
        if error.is_err() {
            break 'body RenameExit::Bad1;
        }

        // Since from wasn't locked at various places above, have to do a relookup here.
        fcnp.cn_flags &= !MODMASK;
        fcnp.cn_flags |= LOCKPARENT | LOCKLEAF;
        if fcnp.cn_flags & SAVESTART == 0 {
            kpanic(format_args!("msdosfs_rename: lost from startdir"));
        }
        if !newparent {
            let _ = vop_unlock(tdvp);
        }
        let mut nfvp = None;
        let _ = vfs_relookup(fdvp, &mut nfvp, fcnp);
        let Some(nfvp) = nfvp else {
            // From name has disappeared.
            if doingdirectory {
                kpanic(format_args!("rename: lost dir entry"));
            }
            vrele(a_fvp);
            if newparent {
                let _ = vop_unlock(tdvp);
            }
            vrele(tdvp);
            return Ok(());
        };
        fvp = nfvp;
        let x = vtode(fvp);
        let zp = vtode(fdvp);
        let from_diroffset = zp.de_fndoffset.get();

        // Ensure that the directory entry still exists and has not changed till now. If the
        // source is a file the entry may have been unlinked or renamed. In either case there
        // is no further work to be done. If the source is a directory then it cannot have
        // been rmdir'ed or renamed; this is prohibited by the DE_RENAME flag.
        if MArc::ptr_eq(&x, &ip) {
            vrele(fvp);

            // First write a new entry in the destination directory and mark the entry in the
            // source directory as deleted. Then move the denode to the correct hash chain for
            // its new location in the filesystem. And, if we moved a directory, then update
            // its .. entry to point to the new parent directory.
            let oldname = ip.name();
            ip.set_name(&toname); // update denode
            dp.de_fndoffset.set(to_diroffset);
            dp.de_fndcnt.set(to_count);
            error = createde(&ip, &dp, None, tcnp);
            if error.is_err() {
                ip.set_name(&oldname);
                if newparent {
                    let _ = vop_unlock(fdvp);
                }
                break 'body RenameExit::Bad;
            }
            ip.de_refcnt.set(ip.de_refcnt.get().wrapping_add(1));
            zp.de_fndoffset.set(from_diroffset);
            error = removede(&zp, &ip);
            if error.is_err() {
                // XXX should really panic here, fs is corrupt
                if newparent {
                    let _ = vop_unlock(fdvp);
                }
                break 'body RenameExit::Bad;
            }

            cache_purge(fvp);

            if !doingdirectory {
                let mut dirclust = ip.de_dirclust.get();
                error = pcbmap(
                    &dp,
                    de_cluster(&pmp, to_diroffset),
                    None,
                    Some(&mut dirclust),
                    None,
                );
                ip.de_dirclust.set(dirclust);
                if error.is_err() {
                    // XXX should really panic here, fs is corrupt
                    if newparent {
                        let _ = vop_unlock(fdvp);
                    }
                    break 'body RenameExit::Bad;
                }
                ip.de_diroffset.set(to_diroffset);
                if ip.de_dirclust.get() != MSDOSFSROOT {
                    ip.de_diroffset.set(ip.de_diroffset.get() & pmp.pm_crbomask);
                }
            }
            reinsert(ip.as_pin());
            if newparent {
                let _ = vop_unlock(fdvp);
            }
        } else {
            if doingdirectory {
                kpanic(format_args!("rename: lost dir entry"));
            }
            vrele(a_fvp);
            if newparent {
                let _ = vop_unlock(fdvp);
            }
        }

        // If we moved a directory to a new parent directory, then we must fixup the ".."
        // entry in the moved directory.
        if doingdirectory && newparent {
            let cn = ip.de_StartCluster.get();
            if cn == MSDOSFSROOT {
                // this should never happen
                kpanic(format_args!(
                    "msdosfs_rename: updating .. in root directory?"
                ));
            }
            let bn = cntobn(&pmp, cn);
            let (mut bp, r) = bread(&pmp.pm_devvp.bufs, Daddr::from(bn), pmp.pm_bpcluster as i32);
            if let Err(e) = r {
                // XXX should really panic here, fs is corrupt
                error = Err(e);
                brelse(bp);
                break 'body RenameExit::Bad;
            }
            let dotdot = Direntry::SIZE;
            let mut dot = entry_at(bp.bytes(), 0);
            let mut dd = entry_at(bp.bytes(), dotdot);
            putushort(&mut dot.deStartCluster, cn as u16);
            let mut pcl = dp.de_StartCluster.get();
            if fat32(&pmp) && pcl == pmp.pm_rootdirblk {
                pcl = 0;
            }
            putushort(&mut dd.deStartCluster, pcl as u16);
            if fat32(&pmp) {
                putushort(&mut dot.deHighClust, (cn >> 16) as u16);
                putushort(&mut dd.deHighClust, (pcl >> 16) as u16);
            }
            entry_put(bp.data(), 0, &dot);
            entry_put(bp.data(), dotdot, &dd);
            error = bwrite(bp);
            if error.is_err() {
                // XXX should really panic here, fs is corrupt
                break 'body RenameExit::Bad;
            }
        }

        VN_KNOTE(fvp, NOTE_RENAME);
        RenameExit::Bad
    };

    if exit == RenameExit::Bad {
        // bad:
        let _ = vop_unlock(fvp);
        vrele(fdvp);
    }
    if exit != RenameExit::Out {
        // bad1:
        if xp.is_some()
            && let Some(t) = tvp
        {
            vput(t);
        }
        vput(tdvp);
    }
    // out:
    ip.clr_flag(DE_RENAME);
    vrele(fvp);
    error
}

/// `msdosfs_mkdir` (`vop_mkdir`): make a directory: a cluster with its "." and ".." entries,
/// and its entry in the parent, which is released (`vput`) in all cases.
pub fn msdosfs_mkdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let pdep = vtode(dvp);
    let pmp = &pdep.de_pmp;
    let bpcluster = pmp.pm_bpcluster;

    let error: Errno = 'bad2: {
        // If this is the root directory and there is no space left we can't do anything.
        // This is because the root directory can not change size.
        if pdep.de_StartCluster.get() == MSDOSFSROOT
            && pdep.de_fndoffset.get() >= pdep.de_FileSize.get()
        {
            break 'bad2 Errno::ENOSPC;
        }

        // Allocate a cluster to hold the about to be created directory.
        let mut newcluster: u32 = 0;
        if let Err(e) = clusteralloc(pmp, 0, 1, Some(&mut newcluster), None) {
            break 'bad2 e;
        }

        let ndirent = Denode::new(None, pdep.de_dev, pmp.clone(), 0, 0);
        ndirent.de_flag.set(DE_ACCESS | DE_CREATE | DE_UPDATE);
        let ts = getnanotime();
        detimes(&ndirent, &ts, &ts, &ts);

        let error: Errno = 'bad: {
            // Now fill the cluster with the "." and ".." entries. And write the cluster to
            // disk. This way it is there for the parent directory to be pointing at if there
            // were a crash.
            let bn = cntobn(pmp, newcluster);
            // always succeeds
            let mut bp = getblk_wait(&pmp.pm_devvp.bufs, Daddr::from(bn), bpcluster as i32);
            bp.data()[..bpcluster as usize].fill(0);
            let dotdot = Direntry::SIZE;
            let mut ents = DOSDIRTEMPLATE;
            let pcl = if fat32(pmp) && pdep.de_StartCluster.get() == pmp.pm_rootdirblk {
                0
            } else {
                pdep.de_StartCluster.get()
            };
            for (denp, cl) in ents.iter_mut().zip([newcluster, pcl]) {
                putushort(&mut denp.deStartCluster, cl as u16);
                putushort(&mut denp.deCDate, ndirent.de_CDate.get());
                putushort(&mut denp.deCTime, ndirent.de_CTime.get());
                denp.deCTimeHundredth = ndirent.de_CTimeHundredth.get();
                putushort(&mut denp.deADate, ndirent.de_ADate.get());
                putushort(&mut denp.deMDate, ndirent.de_MDate.get());
                putushort(&mut denp.deMTime, ndirent.de_MTime.get());
            }
            if fat32(pmp) {
                putushort(&mut ents[0].deHighClust, (newcluster >> 16) as u16);
                putushort(
                    &mut ents[1].deHighClust,
                    (pdep.de_StartCluster.get() >> 16) as u16,
                );
            }
            entry_put(bp.data(), 0, &ents[0]);
            entry_put(bp.data(), dotdot, &ents[1]);

            if let Err(e) = bwrite(bp) {
                break 'bad e;
            }

            // Now build up a directory entry pointing to the newly allocated cluster. This
            // will be written to an empty slot in the parent directory.
            #[cfg(feature = "diagnostic")]
            if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
                kpanic(format_args!("msdosfs_mkdir: no name"));
            }
            let mut name = [0u8; 11];
            if let Err(e) = uniqdosname(&pdep, cnp, &mut name) {
                break 'bad e;
            }
            ndirent.set_name(&name);

            ndirent.de_Attributes.set(ATTR_DIRECTORY);
            ndirent.de_StartCluster.set(newcluster);
            ndirent.de_FileSize.set(0);
            ndirent.de_devvp.set(pdep.de_devvp.get());
            let mut dep = None;
            if let Err(e) = createde(&ndirent, &pdep, Some(&mut dep), cnp) {
                break 'bad e;
            }
            let Some(dep) = dep else {
                kpanic(format_args!("msdosfs_mkdir: no denode"));
            };
            if cnp.cn_flags & SAVESTART == 0 {
                pnbuf_free(cnp);
            }
            VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);
            vput(dvp);
            *vpp = Some(dep.detov());
            return Ok(());
        };

        // bad:
        let _ = clusterfree(pmp, newcluster, None);
        error
    };

    // bad2:
    pnbuf_free(cnp);
    vput(dvp);
    Err(error)
}

/// `msdosfs_rmdir` (`vop_rmdir`): remove an empty directory; both vnodes are released.
pub fn msdosfs_rmdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &Cn<'_, G>,
) -> Result<(), Errno> {
    let ip = vtode(vp);
    let dp = vtode(dvp);
    let mut dvp_held = true;
    let error: Result<(), Errno> = 'out: {
        // Verify the directory is empty (and valid). (Rmdir ".." won't be valid since ".."
        // will contain a reference to the current directory and thus be non-empty.)
        if !dosdirempty(&ip) || ip.de_flag.get() & DE_RENAME != 0 {
            break 'out Err(Errno::ENOTEMPTY);
        }

        VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);

        // Delete the entry from the directory. For dos filesystems this gets rid of the
        // directory entry on disk, the in memory copy still exists but the de_refcnt is <= 0.
        // This prevents it from being found by deget(). When the vput() on dep is done we give
        // up access and eventually msdosfs_reclaim() will be called which will remove it from
        // the denode cache.
        if let Err(e) = removede(&dp, &ip) {
            break 'out Err(e);
        }
        // This is where we decrement the link count in the parent directory. Since dos
        // filesystems don't do this we just purge the name cache and let go of the parent
        // directory denode.
        cache_purge(dvp);
        vput(dvp);
        dvp_held = false;
        // Truncate the directory that is being deleted.
        let error = detrunc(&ip, 0, IO_SYNC, cnp.cn_cred, Some(cnp.proc()));
        cache_purge(vp);
        error
    };
    // out:
    if dvp_held {
        vput(dvp);
    }
    VN_KNOTE(vp, NOTE_DELETE);
    drop((ip, dp));
    vput(vp);
    error
}

/// `msdosfs_symlink` (`vop_symlink`): DOS filesystems don't know what symlinks are.
pub fn msdosfs_symlink<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let _ = vop_abortop(dvp, cnp);
    vput(dvp);
    Err(Errno::EOPNOTSUPP)
}

/// `uiomove(dp, dp->d_reclen, uio)`: copy out the first `d_reclen` bytes of a
/// `struct dirent`.
fn dirent_uiomove<G: ProcGlue>(dp: &Dirent, uio: &mut Uio<'_, G>) -> Result<(), Errno> {
    let mut b = [0u8; size_of::<Dirent>()];
    let at = |f: usize, len: usize| f..f + len;
    b[at(offset_of!(Dirent, d_fileno), 8)].copy_from_slice(&dp.d_fileno.to_ne_bytes());
    b[at(offset_of!(Dirent, d_off), 8)].copy_from_slice(&dp.d_off.to_ne_bytes());
    b[at(offset_of!(Dirent, d_reclen), 2)].copy_from_slice(&dp.d_reclen.to_ne_bytes());
    b[offset_of!(Dirent, d_type)] = dp.d_type;
    b[offset_of!(Dirent, d_namlen)] = dp.d_namlen;
    b[at(offset_of!(Dirent, __d_padding), 4)].copy_from_slice(&dp.__d_padding);
    b[at(Dirent::NAME_OFFSET, MAXNAMLEN + 1)].copy_from_slice(&dp.d_name);
    let reclen = usize::from(dp.d_reclen).min(b.len());
    <G::FileGlue as FileGlue>::uiomove(&mut b[..reclen], uio)
}

/// `msdosfs_readdir` (`vop_readdir`): convert the DOS directory entries (with their Win95
/// long names) to `struct dirent`s. The root directory, which has no "." and "..", gets
/// them simulated at offsets 0 and 32 (`bias`).
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its labels kept as the exits of one body"
)]
pub fn msdosfs_readdir<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    let dep = vtode(vp);
    let pmp = dep.pmp();
    let mut bias: Off = 0;
    let mut wlast: Off = -1;
    let mut chksum: i32 = -1;
    let dsize = Off::from(DIRENTRY_SIZE);

    // msdosfs_readdir() won't operate properly on regular files since it does i/o only with
    // the filesystem vnode, and hence can retrieve the wrong block from the buffer cache for a
    // plain file. So, fail attempts to readdir() on a plain file.
    if dep.de_Attributes.get() & ATTR_DIRECTORY == 0 {
        return Err(Errno::ENOTDIR);
    }

    // To be safe, initialize dirbuf
    let mut dirbuf = Dirent {
        d_fileno: 0,
        d_off: 0,
        d_reclen: 0,
        d_type: 0,
        d_namlen: 0,
        __d_padding: [0; 4],
        d_name: [0; MAXNAMLEN + 1],
    };

    // If the user buffer is smaller than the size of one dos directory entry or the file
    // offset is not a multiple of the size of a directory entry, then we fail the read.
    let count = uio.uio_resid & !(Direntry::SIZE - 1);
    let mut offset = uio.uio_offset;
    if count < Direntry::SIZE || offset & (dsize - 1) != 0 {
        return Err(Errno::EINVAL);
    }
    let lost = uio.uio_resid - count;
    uio.uio_resid = count;

    let dirsperblk = u32::from(pmp.pm_BytesPerSec()) / DIRENTRY_SIZE;
    let shortname = pmp.pm_flags.get() & MSDOSFSMNT_SHORTNAME as u32 != 0;

    let error: Result<(), Errno> = 'out: {
        // If they are reading from the root directory then, we simulate the . and .. entries
        // since these don't exist in the root directory. We also set the offset bias to make
        // up for having to simulate these entries. By this I mean that at file offset 64 we
        // read the first entry in the root directory that lives on disk.
        if dep.de_StartCluster.get() == MSDOSFSROOT
            || (fat32(pmp) && dep.de_StartCluster.get() == pmp.pm_rootdirblk)
        {
            bias = 2 * dsize;
            if offset < bias {
                for n in offset / dsize..2 {
                    dirbuf.d_fileno = if fat32(pmp) {
                        u64::from(pmp.pm_rootdirblk)
                    } else {
                        1
                    };
                    dirbuf.d_type = DT_DIR;
                    let name: &[u8] = if n == 0 { b"." } else { b".." };
                    dirbuf.d_namlen = name.len() as u8;
                    // strlcpy(dirbuf.d_name, name, sizeof dirbuf.d_name)
                    dirbuf.d_name[..name.len()].copy_from_slice(name);
                    dirbuf.d_name[name.len()] = 0;
                    dirbuf.d_reclen = dirent_size(&dirbuf) as u16;
                    dirbuf.d_off = offset + dsize;
                    if uio.uio_resid < usize::from(dirbuf.d_reclen) {
                        break 'out Ok(());
                    }
                    if let Err(e) = dirent_uiomove(&dirbuf, uio) {
                        break 'out Err(e);
                    }
                    offset = dirbuf.d_off;
                }
            }
        }

        let mut error = Ok(());
        while uio.uio_resid > 0 {
            let lbn = de_cluster(pmp, offset - bias) as u32;
            let on = (offset - bias) & Off::from(pmp.pm_crbomask);

            let mut n =
                i64::from(((i64::from(pmp.pm_bpcluster) - on) as u32).min(uio.uio_resid as u32));
            let diff = (i64::from(dep.de_FileSize.get()) - (offset - bias)) as i32;
            if diff <= 0 {
                break;
            }
            n = i64::from((n as u32).min(diff as u32));
            let mut bn: Daddr = 0;
            let mut cn: u32 = 0;
            let mut blsize: i32 = 0;
            error = pcbmap(&dep, lbn, Some(&mut bn), Some(&mut cn), Some(&mut blsize));
            if error.is_err() {
                break;
            }
            let (bp, r) = bread(&pmp.pm_devvp.bufs, bn, blsize);
            if let Err(e) = r {
                brelse(bp);
                return Err(e);
            }
            n = i64::from((n as u32).min((blsize as u32).wrapping_sub(bp.buf().resid() as u32)));

            // Convert from dos directory entries to fs-independent directory entries. The
            // end of the walk: `Some(result)` leaves the function's body (`goto out`).
            let leave: Option<Result<(), Errno>> = 'blk: {
                let data = bp.bytes();
                let mut pos = on as usize;
                while pos < (on + n) as usize {
                    'next: {
                        let dentp = entry_at(data, pos);
                        // If this is an unused entry, we can stop.
                        if dentp.deName[0] == SLOT_EMPTY {
                            break 'blk Some(Ok(()));
                        }
                        // Skip deleted entries.
                        if dentp.deName[0] == SLOT_DELETED {
                            chksum = -1;
                            wlast = -1;
                            break 'next;
                        }

                        // Handle Win95 long directory entries
                        if dentp.deAttributes == ATTR_WIN95 {
                            if shortname {
                                break 'next;
                            }
                            let wep = winentry_at(data, pos);
                            chksum = win2unixfn(&wep, &mut dirbuf, chksum);
                            if wep.weCnt & WIN_LAST != 0 {
                                wlast = offset;
                            }
                            break 'next;
                        }

                        // Skip volume labels
                        if dentp.deAttributes & ATTR_VOLUME != 0 {
                            chksum = -1;
                            wlast = -1;
                            break 'next;
                        }

                        // This computation of d_fileno must match the computation of va_fileid
                        // in msdosfs_getattr.
                        let mut fileno = u32::from(getushort(&dentp.deStartCluster));
                        if fat32(pmp) {
                            fileno |= u32::from(getushort(&dentp.deHighClust)) << 16;
                        }

                        if dentp.deAttributes & ATTR_DIRECTORY != 0 {
                            // Special-case root
                            if fileno == MSDOSFSROOT {
                                fileno = if fat32(pmp) { pmp.pm_rootdirblk } else { 1 };
                            }

                            dirbuf.d_fileno = u64::from(fileno);
                            dirbuf.d_type = DT_DIR;
                        } else {
                            if getulong(&dentp.deFileSize) == 0 {
                                let mut fileno64 = u64::from(if cn == MSDOSFSROOT {
                                    roottobn(pmp, 0u32)
                                } else {
                                    cntobn(pmp, cn)
                                });

                                fileno64 = fileno64.wrapping_mul(u64::from(dirsperblk));
                                fileno64 = fileno64.wrapping_add((pos / Direntry::SIZE) as u64);

                                fileno = fileidhash(fileno64);
                            }

                            dirbuf.d_fileno = u64::from(fileno);
                            dirbuf.d_type = DT_REG;
                        }

                        let name11 = dentp.name11();
                        if chksum == i32::from(winChksum(&name11)) {
                            dirbuf.d_name[usize::from(dirbuf.d_namlen)] = 0;
                        } else {
                            dirbuf.d_namlen =
                                dos2unixfn(&name11, &mut dirbuf.d_name, shortname) as u8;
                        }
                        chksum = -1;
                        dirbuf.d_reclen = dirent_size(&dirbuf) as u16;
                        dirbuf.d_off = offset + dsize;
                        if uio.uio_resid < usize::from(dirbuf.d_reclen) {
                            // Remember long-name offset.
                            if wlast != -1 {
                                offset = wlast;
                            }
                            break 'blk Some(Ok(()));
                        }
                        wlast = -1;
                        if let Err(e) = dirent_uiomove(&dirbuf, uio) {
                            break 'blk Some(Err(e));
                        }
                    }
                    pos += Direntry::SIZE;
                    offset += dsize;
                }
                None
            };
            brelse(bp);
            if let Some(r) = leave {
                break 'out r;
            }
        }
        error
    };

    // out:
    uio.uio_offset = offset;
    uio.uio_resid += lost;
    *eofflag = i32::from(i64::from(dep.de_FileSize.get()) - (offset - bias) <= 0);
    error
}

/// `msdosfs_readlink` (`vop_readlink`): DOS filesystems don't know what symlinks are.
pub fn msdosfs_readlink() -> Result<(), Errno> {
    Err(Errno::EINVAL)
}

/// `msdosfs_lock` (`vop_lock`): take the denode's lock (a recursive rwlock: the thread that
/// holds it may take it again).
pub fn msdosfs_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    vtode(vp).de_lock.enter(flags & LK_RWFLAGS)
}

/// `msdosfs_unlock` (`vop_unlock`).
pub fn msdosfs_unlock<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    vtode(vp).de_lock.exit();
    Ok(())
}

/// `msdosfs_islocked` (`vop_islocked`).
pub fn msdosfs_islocked<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    vtode(vp).de_lock.status()
}

/// `msdosfs_bmap` (`vop_bmap`): the device vnode holding the file system (`vpp`) and the
/// file system relative block number of the file's cluster `bn` (`bnp`).
pub fn msdosfs_bmap<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    vpp: Option<&mut Option<&'static Vnode<G>>>,
    bnp: Option<&mut Daddr>,
    runp: Option<&mut i32>,
) -> Result<(), Errno> {
    let dep = vtode(vp);

    if let Some(vpp) = vpp {
        *vpp = dep.de_devvp.get();
    }
    let Some(bnp) = bnp else {
        return Ok(());
    };

    let cn = bn as u32;
    if i64::from(cn) != bn {
        return Err(Errno::EFBIG);
    }
    drop(dep);

    msdosfs_bmaparray(vp, cn, bnp, runp)
}

/// `msdosfs_bmaparray(vp, cn, bnp, runp)`: map cluster `cn` of the file to a block number
/// (`bnp`) and, with `runp`, count the clusters after it that follow on the disk.
pub fn msdosfs_bmaparray<G: ProcGlue>(
    vp: &'static Vnode<G>,
    cn: u32,
    bnp: &mut Daddr,
    mut runp: Option<&mut i32>,
) -> Result<(), Errno> {
    let dep = vtode(vp);
    let pmp = dep.pmp();
    let mut maxrun: i32 = 0;

    let mp = vmount(vp);

    if let Some(r) = runp.as_deref_mut() {
        // XXX
        // If MAXBSIZE is the largest transfer the disks can handle, we probably want maxrun
        // to be 1 block less so that we don't create a block larger than the device can
        // handle.
        *r = 0;
        maxrun = ((MAXBSIZE as u32 / mp.iosize()).wrapping_sub(1))
            .min(pmp.pm_maxcluster.wrapping_sub(cn)) as i32;
    }

    pcbmap(&dep, cn, Some(&mut *bnp), None, None)?;

    let mut run: i32 = 1;
    while run <= maxrun {
        let mut runbn: Daddr = 0;
        if pcbmap(
            &dep,
            cn.wrapping_add(run as u32),
            Some(&mut runbn),
            None,
            None,
        )
        .is_err()
            || runbn != *bnp + de_cn2bn(pmp, i64::from(run))
        {
            break;
        }
        run += 1;
    }

    if let Some(r) = runp {
        *r = run - 1;
    }

    Ok(())
}

/// The token of the buffer `VOP_STRATEGY` was given: the kernel's file systems pass their
/// own (`IoBuf::Token`), and no `bsd` caller of `VOP_STRATEGY` names a msdosfs vnode (they
/// name device vnodes, `spec_strategy`'s, and NFS's own): a lent one is a buffer that cannot
/// reach here, `panic(9)`.
fn strategy_token(bp: IoBuf) -> crate::fs::buf::BufRef {
    match bp.into_token() {
        Some(t) => t,
        None => kpanic(format_args!(
            "msdosfs_strategy: a lent buffer with no token"
        )),
    }
}

/// `msdosfs_strategy` (`vop_strategy`): map the buffer's cluster to the disk if not done yet,
/// then pass it to the device. A hole's buffer is finished by `biodone` (the C's `splbio();
/// biodone(bp); splx(s)`: one section of the cache's lock).
pub fn msdosfs_strategy<G: ProcGlue>(vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
    let mut bp = strategy_token(bp);
    let dep = vtode(vp);
    let mut error = Ok(());

    if vp.vtype() == VBLK || vp.vtype() == VCHR {
        kpanic(format_args!("msdosfs_strategy: spec"));
    }
    // If we don't already know the filesystem relative block number then get it using
    // pcbmap(). If pcbmap() returns the block number as -1 then we've got a hole in the file.
    // DOS filesystems don't allow files with holes, so we shouldn't ever see this.
    let b = bp.buf();
    if b.blkno() == b.lblkno() {
        let mut blkno = b.blkno();
        error = pcbmap(&dep, b.lblkno() as u32, Some(&mut blkno), None, None);
        b.set_blkno(if error.is_err() { -1 } else { blkno });
        if b.blkno() == -1 {
            bp.clear();
        }
    }
    if bp.buf().blkno() == -1 {
        biodone(bp);
        return error;
    }

    // Read/write the block from/to the disk that contains the desired file block.
    let Some(devvp) = dep.de_devvp.get() else {
        kpanic(format_args!("msdosfs_strategy: denode without a device"));
    };
    drop(dep);
    bp.buf().set_dev(devvp.rdev());
    let _ = vop_strategy(devvp, IoBuf::Token(bp));
    Ok(())
}

/// `msdosfs_print` (`vop_print`): print out the contents of a denode.
pub fn msdosfs_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(any(feature = "debug", feature = "diagnostic"))]
    {
        use hal::sys::types::{major, minor};
        let dep = vtode(vp);

        crate::prf::printf(format_args!(
            "tag VT_MSDOSFS, startcluster {}, dircluster {}, diroffset {} ",
            dep.de_StartCluster.get(),
            dep.de_dirclust.get(),
            dep.de_diroffset.get()
        ));
        crate::prf::printf(format_args!(
            " dev {}, {}, {}\n",
            major(dep.de_dev),
            minor(dep.de_dev),
            if vop_islocked(vp) != 0 {
                "(LOCKED)"
            } else {
                ""
            }
        ));
        #[cfg(feature = "diagnostic")]
        crate::prf::printf(format_args!("\n"));
    }
    #[cfg(not(any(feature = "debug", feature = "diagnostic")))]
    let _ = vp;

    Ok(())
}

/// `msdosfs_advlock` (`vop_advlock`): advisory record locking support.
pub fn msdosfs_advlock<G: ProcGlue>(
    vp: &'static Vnode<G>,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    let dep = vtode(vp);

    lf_advlock::<G>(
        &dep.de_lockf,
        Off::from(dep.de_FileSize.get()),
        id,
        op,
        fl,
        flags,
    )
}

/// `msdosfs_pathconf` (`vop_pathconf`).
pub fn msdosfs_pathconf<G: ProcGlue>(
    vp: &'static Vnode<G>,
    name: i32,
    retval: &mut Register,
) -> Result<(), Errno> {
    let dep = vtode(vp);
    let pmp = dep.pmp();

    *retval = match name {
        _PC_LINK_MAX | _PC_CHOWN_RESTRICTED => 1,
        _PC_NAME_MAX => {
            if pmp.pm_flags.get() & MSDOSFSMNT_LONGNAME as u32 != 0 {
                WIN_MAXLEN as Register
            } else {
                12
            }
        }
        _PC_NO_TRUNC => 0,
        _PC_TIMESTAMP_RESOLUTION => 2_000_000_000, // 2 billion nanoseconds
        _ => return Err(Errno::EINVAL),
    };

    Ok(())
}

/// `fileidhash`: Thomas Wang's hash function, severely hacked to always set the high bit on
/// the number it returns (so no longer a proper hash function).
fn fileidhash(mut fileid: u64) -> u32 {
    let c1: u64 = 0x6e5ea73858134343;
    let c2: u64 = 0xb34e8f99a2ec9ef5;

    // We now have the original fileid value, as 64-bit value. We need to reduce it to
    // 32-bits, with the top bit set.
    fileid ^= (c1 ^ fileid) >> 32;
    fileid = fileid.wrapping_mul(c1);
    fileid ^= (c2 ^ fileid) >> 31;
    fileid = fileid.wrapping_mul(c2);
    fileid ^= (c1 ^ fileid) >> 32;

    (fileid | 0x80000000) as u32
}

/// `msdosfs_kqfilter` (`vop_kqfilter`): attach a knote to the vnode.
pub fn msdosfs_kqfilter<G: ProcGlue>(vp: &'static Vnode<G>, kn: &Knote<G>) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&MsdosfsFiltops::<G>::READ)),
        EVFILT_WRITE => kn.kn_fop.set(Some(&MsdosfsFiltops::<G>::WRITE)),
        EVFILT_VNODE => kn.kn_fop.set(Some(&MsdosfsFiltops::<G>::VNODE)),
        _ => return Err(Errno::EINVAL),
    }

    // kn->kn_hook = (caddr_t)vp: a fresh knote holds no hook yet (as `ufs_kqfilter`'s).
    let _ = kn.set_hook_vnode(vp);

    vp.klist.insert_legacy(kn);

    Ok(())
}

/// `kn->kn_hook` of a msdosfs knote: its vnode.
fn kn_vnode<G: ProcGlue>(kn: &Knote<G>) -> &'static Vnode<G> {
    match kn.hook_vnode() {
        Some(vp) => vp,
        None => kpanic(format_args!("knote {kn:p}: no vnode")),
    }
}

/// `filt_msdosfsdetach`: unhooks the knote from the vnode.
fn filt_msdosfsdetach<G: ProcGlue>(kn: &Knote<G>) {
    let vp = kn_vnode(kn);

    vp.klist.remove_legacy(kn);
}

impl<G: ProcGlue> Filter<G> for MsdosfsRead {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_msdosfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_msdosfsdetach(kn);
    }

    /// `filt_msdosfsread`: the bytes past the file offset; always ready for poll and select.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        let vp = kn_vnode(kn);

        // filesystem is gone, so set the EOF flag and schedule the knote for deletion. The
        // vnode has no denode any more (the module's redesign): it is reached only past
        // this test.
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }
        let dep = vtode(vp);

        kn.kn_data()
            .set(i64::from(dep.de_FileSize.get()) - foffset(&kn.fp()));
        if kn.kn_data().get() == 0 && kn.kn_sfflags.get() & NOTE_EOF != 0 {
            kn.kn_fflags().set(kn.kn_fflags().get() | NOTE_EOF);
            return true;
        }

        if kn.has_flags(__EV_POLL | __EV_SELECT) {
            return true;
        }

        kn.kn_data().get() != 0
    }
}

impl<G: ProcGlue> Filter<G> for MsdosfsWrite {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_msdosfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_msdosfsdetach(kn);
    }

    /// `filt_msdosfswrite`: a file is always writable.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        // filesystem is gone, so set the EOF flag and schedule the knote for deletion.
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }

        kn.kn_data().set(0);
        true
    }
}

impl<G: ProcGlue> Filter<G> for MsdosfsVnode {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_msdosfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_msdosfsdetach(kn);
    }

    /// `filt_msdosfsvnode`: records the vnode events (`NOTE_*`) the user asked for.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        let hint32 = hint as u32;
        if kn.kn_sfflags.get() & hint32 != 0 {
            kn.kn_fflags().set(kn.kn_fflags().get() | hint32);
        }
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF);
            return true;
        }
        kn.kn_fflags().get() != 0
    }
}

/// `msdosfs_vops`: each slot the C's (34; `vop_abortop`, `vop_bwrite` and `vop_revoke` the
/// generic ones).
impl<G: ProcGlue> Vnops<G> for MsdosfsVnops<G> {
    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        msdosfs_lookup(dvp, vpp, cnp)
    }
    fn create(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        msdosfs_create(dvp, vpp, cnp, vap)
    }
    fn mknod(
        &self,
        dvp: &'static Vnode<G>,
        _vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        _vap: &mut Vattr,
    ) -> Result<(), Errno> {
        msdosfs_mknod(dvp, cnp)
    }
    fn open(
        &self,
        _vp: &'static Vnode<G>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        msdosfs_open()
    }
    fn close(
        &self,
        vp: &'static Vnode<G>,
        _fflag: i32,
        _cred: Cred<'_>,
        _p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        msdosfs_close(vp)
    }
    fn access(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        msdosfs_access(vp, mode, cred)
    }
    fn getattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        msdosfs_getattr(vp, vap)
    }
    fn setattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        msdosfs_setattr(vp, vap, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        msdosfs_read(vp, uio)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        msdosfs_write(vp, uio, ioflag, cred)
    }
    fn ioctl(
        &self,
        _vp: &'static Vnode<G>,
        _command: u64,
        _data: &mut [u8],
        _fflag: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        msdosfs_ioctl()
    }
    fn kqfilter(&self, vp: &'static Vnode<G>, _fflag: i32, kn: &Knote<G>) -> Result<(), Errno> {
        msdosfs_kqfilter(vp, kn)
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        _cred: Cred<'_>,
        waitfor: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        msdosfs_fsync(vp, waitfor)
    }
    fn remove(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        _cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        msdosfs_remove(dvp, vp)
    }
    fn link(
        &self,
        dvp: &'static Vnode<G>,
        _vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        msdosfs_link(dvp, cnp)
    }
    fn rename(
        &self,
        fdvp: &'static Vnode<G>,
        fvp: &'static Vnode<G>,
        fcnp: &mut Cn<'_, G>,
        tdvp: &'static Vnode<G>,
        tvp: Option<&'static Vnode<G>>,
        tcnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        msdosfs_rename(fdvp, fvp, fcnp, tdvp, tvp, tcnp)
    }
    fn mkdir(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        _vap: &mut Vattr,
    ) -> Result<(), Errno> {
        msdosfs_mkdir(dvp, vpp, cnp)
    }
    fn rmdir(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        msdosfs_rmdir(dvp, vp, cnp)
    }
    fn symlink(
        &self,
        dvp: &'static Vnode<G>,
        _vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        _vap: &mut Vattr,
        _target: &[u8],
    ) -> Result<(), Errno> {
        msdosfs_symlink(dvp, cnp)
    }
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        msdosfs_readdir(vp, uio, eofflag)
    }
    fn readlink(
        &self,
        _vp: &'static Vnode<G>,
        _uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        msdosfs_readlink()
    }
    fn abortop(&self, _dvp: &'static Vnode<G>, cnp: &mut Cn<'_, G>) -> Result<(), Errno> {
        default::vop_generic_abortop(cnp)
    }
    fn inactive(&self, vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
        msdosfs_inactive(vp, p)
    }
    fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        msdosfs_reclaim(vp)
    }
    fn lock(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        msdosfs_lock(vp, flags)
    }
    fn unlock(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        msdosfs_unlock(vp)
    }
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        msdosfs_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        msdosfs_strategy(vp, bp)
    }
    fn print(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        msdosfs_print(vp)
    }
    fn islocked(&self, vp: &'static Vnode<G>) -> i32 {
        msdosfs_islocked(vp)
    }
    fn pathconf(
        &self,
        vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        msdosfs_pathconf(vp, name, retval)
    }
    fn advlock(
        &self,
        vp: &'static Vnode<G>,
        id: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        msdosfs_advlock(vp, id, op, fl, flags)
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        default::vop_generic_bwrite(bp)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::msdosfs::denode::MSDOSFSROOT_OFS;
    use crate::fs::msdosfs::msdosfs_denode::deget;
    use crate::fs::msdosfs::msdosfs_lookup::test_full_root;
    use crate::fs::namei::CREATE;
    use crate::fs::subr::{vfs, vgone, vops_id};
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ufs::testufs::{cn, vattr};
    use crate::uio::Iovec;
    use std::vec::Vec;

    /// `getdents(2)` of `vp` from `offset` into a buffer of `len` bytes: the result, the
    /// entries' names and cookies, the offset after, the end-of-file flag.
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
        let r = msdosfs_readdir(vp, &mut uio, &mut eof);
        let (off, done) = (uio.uio_offset, len - uio.uio_resid);
        let mut out = Vec::new();
        let mut at = 0;
        while at < done {
            let Some(d) = Dirent::from_bytes(&buf[at..]) else {
                std::panic!("a short entry at {at}");
            };
            let name = buf[at + Dirent::NAME_OFFSET..][..usize::from(d.d_namlen)].to_vec();
            out.push((name, d.d_off));
            at += usize::from(d.d_reclen);
        }
        (r, out, off, eof)
    }

    /// `getdents(2)` of a full FAT16 root of 1 KiB sectors lists its 208 entries after the
    /// simulated "." and "..", and stops at the root's real end (D-N5.7-U4): the C's size,
    /// twice the root's, would go on into the first data cluster and list `PHANTOM.TXT`.
    #[test]
    fn readdir_of_a_full_root_lists_only_its_entries() {
        let (_g, p, pmp) = test_full_root(1024, 2, 208);
        let root = deget(&pmp, MSDOSFSROOT, MSDOSFSROOT_OFS).unwrap();
        let vp = root.detov();
        let (r, ents, off, eof) = readdir(p, vp, 0, 64 * 1024);
        assert_eq!(r, Ok(()));
        assert_eq!(ents.len(), 2 + 208);
        assert_eq!(ents[0], (b".".to_vec(), 32));
        assert_eq!(ents[1], (b"..".to_vec(), 64));
        assert!(ents[2].0.eq_ignore_ascii_case(b"F0000000.TXT"));
        assert!(ents[209].0.eq_ignore_ascii_case(b"F0000207.TXT"));
        assert!(
            !ents
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case(b"PHANTOM.TXT"))
        );
        assert_eq!((off, eof), (64 + 6656, 1));
        // From a cookie in the root's tail: the rest, then the end.
        let (r, ents, off, eof) = readdir(p, vp, 64 + 6144, 64 * 1024);
        assert_eq!((r, ents.len(), off, eof), (Ok(()), 16, 64 + 6656, 1));
        drop(root);
        vput(vp);
    }

    /// A file created in a full FAT16 root of 1 KiB sectors gets `ENOSPC` (D-N5.7-U4): the
    /// lookup's free slot is the root's end, which the C's size, twice the root's, would place
    /// inside it (and `createde` would write the entry into a data cluster).
    #[test]
    fn create_in_a_full_root_answers_enospc() {
        let (_g, p, pmp) = test_full_root(1024, 2, 208);
        let cred = p.ucred();
        let root = deget(&pmp, MSDOSFSROOT, MSDOSFSROOT_OFS).unwrap();
        let vp = root.detov();
        let mut c = cn(p, cred, CREATE, crate::fs::namei::LOCKPARENT, b"NEW.TXT");
        let mut vpp = None;
        assert_eq!(
            crate::fs::msdosfs::msdosfs_lookup::msdosfs_lookup(vp, &mut vpp, &mut c),
            Err(Errno::EJUSTRETURN)
        );
        let va = vattr(VREG, 0o644);
        assert_eq!(
            msdosfs_create(vp, &mut vpp, &mut c, &va),
            Err(Errno::ENOSPC)
        );
        assert!(vpp.is_none());
        // getattr's size of the root is its blocks' bytes.
        let mut va = Vattr::new();
        msdosfs_getattr(vp, &mut va).unwrap();
        assert_eq!(va.va_size, 6656);
        assert_eq!(va.va_fileid, 1, "the FAT12/16 root's fileid");
        drop(root);
        vput(vp);
    }

    /// `fileidhash`'s values are the C's (Thomas Wang's mix, the top bit set), computed
    /// independently: `va_fileid` of an empty file and `d_fileno` of its entry, which
    /// userland compares (`pwd`).
    #[test]
    fn fileidhash_keeps_the_c_s_values() {
        for (v, want) in [
            (0u64, 0xc63b_38de_u32),
            (1, 0x80c2_41f9),
            (0x20, 0xa085_5844),
            (0x1234, 0xe217_3998),
            (16 * 16 + 3, 0xc193_05f6),
            (u64::MAX, 0xc677_a669),
        ] {
            assert_eq!(fileidhash(v), want, "fileidhash({v:#x})");
            assert_ne!(fileidhash(v) & 0x8000_0000, 0);
        }
    }

    /// The table has one address per glue, an id of its own in the VFS's registry (the one
    /// `getnewvnode` gave the root's vnode), distinct from the other file systems'.
    #[test]
    fn the_table_is_one_registered_address() {
        let (_g, _p, pmp) = test_full_root(1024, 2, 208);
        let root = deget(&pmp, MSDOSFSROOT, MSDOSFSROOT_OFS).unwrap();
        let vp = root.detov();
        let t = vfs::<TestGlue>();
        let id = vops_id::<TestGlue>(&t.msdosfs.vops);
        assert_eq!(vp.ops_id(), id);
        assert_eq!(vops_id::<TestGlue>(&t.msdosfs.vops), id);
        assert_ne!(vops_id::<TestGlue>(&t.ufs.ffs_vops), id);
        assert_ne!(size_of::<MsdosfsVnops<TestGlue>>(), 0);
        drop(root);
        vput(vp);
    }

    /// The filters: the vnode filter records the notes asked for; the tables carry the C's
    /// flags; `msdosfs_kqfilter` refuses an unknown filter; once the vnode has no denode
    /// (reclaimed), `NOTE_REVOKE` ends every filter without reaching one. (A knote hooked on
    /// a klist needs a kqueue: N5.7c3's `kevent(2)` over the mount.)

    #[test]
    fn the_filters_attach_detach_and_end_without_the_denode() {
        let (_g, _p, pmp) = test_full_root(1024, 2, 208);
        let dep = deget(&pmp, MSDOSFSROOT, 32).unwrap();
        let vp = dep.detov();
        drop(dep);
        let kn = Knote::<TestGlue>::detached();
        assert!(kn.set_hook_vnode(vp).is_ok());
        kn.kn_sfflags.set(NOTE_WRITE | NOTE_DELETE);
        assert!(!<MsdosfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_ATTRIB)
        ));
        assert!(<MsdosfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_WRITE)
        ));
        assert_eq!(kn.kn_fflags().get(), NOTE_WRITE);
        // The tables carry the C's flags and entries.
        assert_eq!(MsdosfsFiltops::<TestGlue>::READ.f_flags, FILTEROP_ISFD);
        assert!(MsdosfsFiltops::<TestGlue>::VNODE.f_event.is_some());
        assert!(MsdosfsFiltops::<TestGlue>::WRITE.f_detach.is_some());
        // An unknown filter is refused.

        let other = Knote::<TestGlue>::detached();
        other.kn_filter().set(0);
        assert_eq!(msdosfs_kqfilter(vp, &other), Err(Errno::EINVAL));
        // Reclaimed: the read filter ends at NOTE_REVOKE before it would reach the denode.
        let _ = vop_unlock(vp);
        vgone(vp);
        assert!(!vp.has_node());
        assert!(<MsdosfsRead as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF) && kn.has_flags(EV_ONESHOT));
        let kn = Knote::<TestGlue>::detached();
        assert!(<MsdosfsWrite as Filter<TestGlue>>::event(&kn, 0) && kn.kn_data().get() == 0);
        assert!(<MsdosfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF));
    }
}
/* </TESTS> */
