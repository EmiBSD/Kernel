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
 * Copyright (c) 1994
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
 *	@(#)cd9660_vnops.c	8.15 (Berkeley) 12/5/94
 */
/* </LICENSES> */

/* <CODE> */
//! The ISO 9660 file system's vnode operations: the tables for files (`cd9660_vops`,
//! [`Cd9660Vnops`]) and for the special files on a disc (`cd9660_specvops`,
//! [`Cd9660SpecVnops`]), attributes, reading a file through the buffer cache, reading a
//! directory (`cd9660_readdir`, which merges associated files for the default format),
//! symbolic links, locking, the strategy routine, `pathconf` and the kqueue filters.
//!
//! Upstream: sys/isofs/cd9660/cd9660_vnops.c @ 3ce1f3f79392
//! LZ: sys/isofs/cd9660/cd9660_vnops.rs@3c62ede68802
//!
//! ## Deviations
//! - `struct isoreaddir` and `cd9660_read`'s read-ahead arrays are on the stack instead of
//!   `malloc(M_TEMP)` (LZ's): `vmstat -m`'s `temp` row counts one request fewer per
//!   `readdir` and per read-ahead.
//! - `iso_uiodir` and `iso_shipdir` return `Err(None)` for the C's -1 (the user's buffer is
//!   full), which `cd9660_readdir` turns into success as the C does; `iso_uiodir` takes the
//!   entry by name (`Ent`) instead of a pointer into the structure. The `bcopy` of
//!   `d_reclen` bytes of the current entry is a copy of the whole entry (the bytes past
//!   `d_reclen` are never copied out) (LZ's).
//! - `cd9660_getattr` reads a zero-length symbolic link's target through `cd9660_readlink`
//!   into a `malloc(M_TEMP)` buffer, as in C, with a kernel `Uio` of one `Iovec` (LZ's).
//! - `cd9660_readlink` gathers the name into a `namei_pool` buffer and copies it out with
//!   `uiomove` for every caller, the C's path for a user's buffer (D-N5.8-11): the C's
//!   direct path for a kernel buffer of `MAXPATHLEN` bytes or more (`namei`'s symbolic link,
//!   `cd9660_getattr`'s) writes the same bytes and leaves the same `uio_resid`, but advances
//!   neither `uio_offset`, which neither caller reads, nor `namei_pool`'s count, so `vmstat
//!   -m`'s `namei` pool counts one request more per such call. A record too short for its
//!   fixed part is `EINVAL` (LZ's).
//! - The C's `eopnotsupp` slots of `cd9660_vops` (`create`, `mknod`, `write`, `remove`,
//!   `rename`, `mkdir`, `rmdir`, `advlock`) are the `Vnops` trait's defaults, which answer
//!   `EOPNOTSUPP` as the C's `eopnotsupp` (and a NULL slot) do. `cd9660_mmap` and
//!   `cd9660_seek`, in no table in C either, take no arguments (LZ's).
//! - `option FIFO` is in GENERIC but `miscfs/fifofs` is not ported: `cd9660_fifovops` comes
//!   with it (`cd9660_vfsops.rs`).
//! - `cd9660_print` prints under `DIAGNOSTIC` or `DEBUG` (features); `VFSLCKDEBUG` is not
//!   configured.
//!
//! ## Redesign
//! - LZ's `CD9660_VOPS` and `CD9660_SPECVOPS` (`bsd` `Vops` of function pointers over
//!   argument structures) -> [`Cd9660Vnops`] and [`Cd9660SpecVnops`], each one `impl
//!   Vnops<G>` of the C's 34 slots (decision 46), members of the glue's `IsofsTables`
//!   (`cd9660_node.rs`: one address each per glue, for `vops_id`; a byte each so that
//!   neither is zero-sized, D-b2-10); the special files' slots other than cd9660's nine are
//!   the kernel's `spec_vops` ones (`fs::spec::spec_shared_slots!`, ext2fs's
//!   `Ext2fsSpecVnops` shape).
//! - The three `filterops` are `impl Filter<G>` for [`Cd9660Read`], [`Cd9660Write`] and
//!   [`Cd9660Vnode`] with their tables `Cd9660Filtops::<G>::READ`/`WRITE`/`VNODE`, made at
//!   compile time (`Filterops::of`, msdosfs's shape); a knote's vnode is its typed hook
//!   (`set_hook_vnode`, `hook_vnode`) where LZ cast `kn_hook` back (one `unsafe`).
//!   `filt_cd9660read` reads the node after its `NOTE_REVOKE` test: a reclaimed vnode has no
//!   node any more.
//! - LZ's lent buffers (`data()`'s `unsafe` views) -> `fs::bio`'s `BufRef` tokens:
//!   `cd9660_read` moves the bytes out of the token (`bread`, or the kernel's `breadn` for the
//!   read-ahead, which `bsd`'s `breadn` adapter wrapped), `cd9660_readdir` holds the
//!   directory's token across each record's Rock Ridge name (the directory vnode's buffer,
//!   which no device read waits for: no held block, D-c1-4), `cd9660_readlink` holds the
//!   record's device block and names it to the Rock Ridge walk (`held`, D-N5.8-U1).
//! - `readdir`'s entries are `struct dirent`s copied out through `fs::subr::dirent_uiomove`
//!   (the struct's first `d_reclen` bytes, which `uiomove(dp, dp->d_reclen, ...)` copies;
//!   one 280-byte serialisation per entry shipped, ROADMAP N12's row), where LZ viewed the
//!   struct as bytes in an `unsafe` block; `cd9660_readlink` writes no raw iovec (above).
//! - `cd9660_strategy` is `ufs_strategy`'s shape: the buffer's token (`IoBuf::into_token`,
//!   `panic(9)` on a lent buffer with none: the `bsd` callers of `VOP_STRATEGY` and
//!   `VOP_BWRITE`, nfs's and softraid's, use their own vnodes or device vnodes), the block
//!   mapped by `VOP_BMAP`, the device's strategy.
//! - The node is `vtoi`'s counted handle (a count pair per operation, ROADMAP N12's row);
//!   `i_flag` and `ci_lastr` its words. 0 `unsafe` (LZ: 6).

use core::marker::PhantomData;

use hal::malloc::{M_WAITOK, MArray};
use hal::sys::dirent::{DT_UNKNOWN, Dirent, MAXNAMLEN, dirent_size};
use hal::sys::errno::Errno;
use hal::sys::stat::ALLPERMS;
use hal::sys::syslimits::NAME_MAX;
use hal::sys::types::{Daddr, Gid, Mode, Nlink, Off, Register, Uid};
use hal::sys::unistd::{
    _PC_CHOWN_RESTRICTED, _PC_LINK_MAX, _PC_NAME_MAX, _PC_NO_TRUNC, _PC_TIMESTAMP_RESOLUTION,
};

use crate::assert::kpanic;
use crate::event::{
    __EV_POLL, __EV_SELECT, EV_EOF, EV_ONESHOT, EVFILT_READ, EVFILT_VNODE, EVFILT_WRITE,
    FILTEROP_ISFD, Filter, Filterops, Knote, NOTE_EOF, NOTE_REVOKE,
};
use crate::file::{FileGlue, foffset};
use crate::fs::bio::{DEV_BSHIFT, biodone, bread, breadn, brelse};
use crate::fs::buf::{B_ERROR, BufRef};
use crate::fs::default;
use crate::fs::isofs::cd9660::cd9660_bmap::cd9660_bmap;
use crate::fs::isofs::cd9660::cd9660_extern::{
    ISO_FTYPE_9660, ISO_FTYPE_DEFAULT, ISO_FTYPE_RRIP, blkoff, blksize, lblkno, lblktosize,
};
use crate::fs::isofs::cd9660::cd9660_lookup::{cd9660_bufatoff, cd9660_lookup, record_ino};
use crate::fs::isofs::cd9660::cd9660_node::{
    IN_ACCESS, cd9660_inactive, cd9660_reclaim, isodirino, vtoi,
};
use crate::fs::isofs::cd9660::cd9660_rrip::{cd9660_rrip_getname, cd9660_rrip_getsymname};
use crate::fs::isofs::cd9660::cd9660_util::isofntrans;
use crate::fs::isofs::cd9660::iso::{
    ASSOCCHAR, Cdino, ISO_DIRECTORY_RECORD_SIZE, IsoDirectoryRecord, isonum_711,
};
use crate::fs::namei::{MAXPATHLEN, pathbuf_get};
use crate::fs::subr::{dirent_uiomove, vaccess, vput};
use crate::fs::ufs::ufs::ufs_vnops::ucred;
use crate::fs::vnode::{
    LK_RWFLAGS, VA_UTIMES_CHANGE, VBLK, VCHR, VDIR, VFIFO, VLNK, VNOVAL, VREG, VSOCK, Vattr, Vnode,
};
use crate::fs::vops::{Cn, Cred, IoBuf, Vnops, vop_abortop, vop_bmap, vop_strategy};
use crate::proc::{ProcGlue, Thread};
use crate::uio::{Iovec, Uio};

/// `M_TEMP`: miscellaneous temporary data buffers (`<sys/malloc.h>`; `bsd`'s `sys/malloc.rs`
/// until N7 moves the header).
const M_TEMP: i32 = 127;

/// `MAX_RA`: the most read-ahead blocks of `cd9660_read`.
const MAX_RA: usize = 32;

/// `cd9660_vops`: the operations of a file, directory or symbolic link on a cd9660 (the
/// glue's `IsofsTables::vops`, one address).
pub struct Cd9660Vnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member: the registry tells tables apart by
    /// their addresses.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> Cd9660Vnops<G> {
    /// The table (the glue's `IsofsTables::vops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `cd9660_specvops`: the operations of a device special file on a cd9660 (the glue's
/// `IsofsTables::specvops`, one address).
pub struct Cd9660SpecVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member (as [`Cd9660Vnops`]).
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> Cd9660SpecVnops<G> {
    /// The table (the glue's `IsofsTables::specvops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// Which of the three entries of a `struct isoreaddir`.
#[derive(Clone, Copy)]
enum Ent {
    /// `saveent`.
    Save,
    /// `assocent`.
    Assoc,
    /// `current`.
    Current,
}

/// `struct isoreaddir`: structure for reading directories.
struct Isoreaddir<'a, 'b, G: ProcGlue> {
    /// `saveent`.
    saveent: Dirent,
    /// `assocent`.
    assocent: Dirent,
    /// `current`.
    current: Dirent,
    /// `saveoff`.
    saveoff: Off,
    /// `assocoff`.
    assocoff: Off,
    /// `curroff`.
    curroff: Off,
    /// `uio`.
    uio: &'a mut Uio<'b, G>,
    /// `uio_off`.
    uio_off: Off,
    /// `eofflag`.
    eofflag: i32,
}

/// `cd9660read_filtops`'s filter: the bytes past the file offset.
pub struct Cd9660Read;

/// `cd9660write_filtops`'s filter: a file is always writable.
pub struct Cd9660Write;

/// `cd9660vnode_filtops`'s filter: the vnode events (`NOTE_*`) the user asked for.
pub struct Cd9660Vnode;

/// The tables of the three filters (`kn_fop`'s targets), made at compile time.
struct Cd9660Filtops<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Cd9660Filtops<G> {
    /// `cd9660read_filtops`.
    const READ: Filterops<G> = Filterops::of::<Cd9660Read>();
    /// `cd9660write_filtops`.
    const WRITE: Filterops<G> = Filterops::of::<Cd9660Write>();
    /// `cd9660vnode_filtops`.
    const VNODE: Filterops<G> = Filterops::of::<Cd9660Vnode>();
}

/// `cd9660_setattr` (`vop_setattr`): only allowed for block and character special devices.
pub fn cd9660_setattr<G: ProcGlue>(vp: &'static Vnode<G>, vap: &Vattr) -> Result<(), Errno> {
    if vap.va_flags != VNOVAL as u64
        || vap.va_uid != VNOVAL as Uid
        || vap.va_gid != VNOVAL as Gid
        || vap.va_atime.tv_nsec != i64::from(VNOVAL)
        || vap.va_mtime.tv_nsec != i64::from(VNOVAL)
        || vap.va_mode != VNOVAL as Mode
        || vap.va_vaflags & VA_UTIMES_CHANGE != 0
    {
        return Err(Errno::EROFS);
    }
    if vap.va_size != VNOVAL as u64 {
        return match vp.vtype() {
            VDIR => Err(Errno::EISDIR),
            VLNK | VREG => Err(Errno::EROFS),
            VCHR | VBLK | VSOCK | VFIFO => Ok(()),
            _ => Err(Errno::EINVAL),
        };
    }

    Err(Errno::EINVAL)
}

/// `cd9660_open` (`vop_open`): nothing to do.
pub fn cd9660_open() -> Result<(), Errno> {
    Ok(())
}

/// `cd9660_close` (`vop_close`): update the times on the inode on writeable file systems;
/// nothing to do here.
pub fn cd9660_close() -> Result<(), Errno> {
    Ok(())
}

/// `cd9660_access` (`vop_access`): check mode permission on inode pointer. Mode is READ,
/// WRITE or EXEC. The mode is shifted to select the owner/group/other fields. The super
/// user is granted all permissions.
pub fn cd9660_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let ino = vtoi(vp).inode();

    vaccess(
        vp.vtype(),
        Mode::from(ino.iso_mode) & ALLPERMS,
        ino.iso_uid,
        ino.iso_gid,
        mode,
        ucred(cred),
    )
}

/// `cd9660_getattr` (`vop_getattr`).
pub fn cd9660_getattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let ino = ip.inode();

    vap.va_fsid = i64::from(ip.i_dev);
    vap.va_fileid = u64::from(ip.i_number);

    vap.va_mode = Mode::from(ino.iso_mode) & ALLPERMS;
    vap.va_nlink = ino.iso_links as Nlink;
    vap.va_uid = ino.iso_uid;
    vap.va_gid = ino.iso_gid;
    vap.va_atime = ino.iso_atime;
    vap.va_mtime = ino.iso_mtime;
    vap.va_ctime = ino.iso_ctime;
    vap.va_rdev = ino.iso_rdev;

    vap.va_size = ip.i_size.get();
    if ip.i_size.get() == 0 && vp.vtype() == VLNK {
        let Some(mut cp) = MArray::new(M_TEMP, M_WAITOK, MAXPATHLEN, |_| 0u8) else {
            kpanic(format_args!("cd9660_getattr: malloc(M_TEMP) failed"));
        };
        let mut aiov = [Iovec::new()];
        let mut auio = Uio::kernel_read(&mut aiov, &mut cp[..], Some(p));
        if cd9660_readlink(vp, &mut auio).is_ok() {
            vap.va_size = (MAXPATHLEN - auio.uio_resid) as u64;
        }
    }
    vap.va_flags = 0;
    vap.va_gen = 1;
    vap.va_blocksize = i64::from(ip.mnt().logical_block_size);
    vap.va_bytes = ip.i_size.get();
    vap.va_type = vp.vtype();
    Ok(())
}

/// `cd9660_read` (`vop_read`): vnode op for reading.
pub fn cd9660_read<G: ProcGlue>(vp: &'static Vnode<G>, uio: &mut Uio<'_, G>) -> Result<(), Errno> {
    let ip = vtoi(vp);

    if uio.uio_resid == 0 {
        return Ok(());
    }
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }
    ip.i_flag.set(ip.i_flag.get() | IN_ACCESS);
    let imp = ip.mnt();
    loop {
        let lbn = lblkno(imp, uio.uio_offset);
        let on = blkoff(imp, uio.uio_offset) as usize;
        let mut n = (imp.logical_block_size as usize - on).min(uio.uio_resid);
        let diff = ip.i_size.get() as Off - uio.uio_offset;
        if diff <= 0 {
            return Ok(());
        }
        if (diff as u64) < n as u64 {
            n = diff as usize;
        }
        let size = blksize(imp);
        let rablock = lbn + 1;
        let (mut bp, error) = if ip.i_ci_lastr.get() + 1 == lbn {
            let mut blks = [0 as Daddr; MAX_RA];
            let mut sizes = [0i32; MAX_RA];
            let mut i = 0;
            while i < MAX_RA && lblktosize(imp, rablock + i as Daddr) < ip.i_size.get() as i64 {
                blks[i] = rablock + i as Daddr;
                sizes[i] = blksize(imp);
                i += 1;
            }
            breadn(&vp.bufs, lbn, size, &blks[..i], &sizes[..i])
        } else {
            bread(&vp.bufs, lbn, size)
        };
        ip.i_ci_lastr.set(lbn);
        n = n.min((size as usize).saturating_sub(bp.buf().resid()));
        if let Err(e) = error {
            brelse(bp);
            return Err(e);
        }

        let error = match bp.data().get_mut(on..on + n) {
            Some(d) => <G::FileGlue as FileGlue>::uiomove(d, uio),
            None => Err(Errno::EIO),
        };

        brelse(bp);
        if error.is_err() || uio.uio_resid == 0 || n == 0 {
            return error;
        }
    }
}

/// `cd9660_ioctl` (`vop_ioctl`).
pub fn cd9660_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `cd9660_mmap`: mmap a file. NB Currently unsupported (in no operations table).
pub fn cd9660_mmap() -> Result<(), Errno> {
    Err(Errno::EINVAL)
}

/// `cd9660_seek`: seek on a file. Nothing to do, so just return (in no operations table).
pub fn cd9660_seek() -> Result<(), Errno> {
    Ok(())
}

/// A `struct dirent` with every byte zero and the type `DT_UNKNOWN`.
const fn dirent_zeroed() -> Dirent {
    Dirent {
        d_fileno: 0,
        d_off: 0,
        d_reclen: 0,
        d_type: DT_UNKNOWN,
        d_namlen: 0,
        __d_padding: [0; 4],
        d_name: [0; MAXNAMLEN + 1],
    }
}

/// `iso_uiodir`: copy one entry out to the user; `Err(None)` when it does not fit (the C's
/// -1, which clears the end-of-file flag).
fn iso_uiodir<G: ProcGlue>(
    idp: &mut Isoreaddir<'_, '_, G>,
    which: Ent,
    off: Off,
) -> Result<(), Option<Errno>> {
    let Isoreaddir {
        saveent,
        assocent,
        current,
        uio,
        uio_off,
        eofflag,
        ..
    } = idp;
    let dp = match which {
        Ent::Save => saveent,
        Ent::Assoc => assocent,
        Ent::Current => current,
    };

    let namlen = usize::from(dp.d_namlen);
    dp.d_name[namlen] = 0;
    dp.d_reclen = dirent_size(dp) as u16;

    if dp.d_name[..namlen].contains(&b'/') {
        // illegal file name
        return Err(Some(Errno::EINVAL));
    }

    if uio.uio_resid < usize::from(dp.d_reclen) {
        *eofflag = 0;
        return Err(None);
    }

    dp.d_off = off;
    dirent_uiomove(dp, uio).map_err(Some)?;
    *uio_off = off;
    Ok(())
}

/// `iso_shipdir`: for the default format, hold the current entry back until the next one
/// shows whether it has an associated file of the same name, then ship them.
fn iso_shipdir<G: ProcGlue>(idp: &mut Isoreaddir<'_, '_, G>) -> Result<(), Option<Errno>> {
    let mut cl = usize::from(idp.current.d_namlen);
    let mut cstart = 0;

    let assoc = cl > 1 && idp.current.d_name[0] == ASSOCCHAR;
    if assoc {
        cl -= 1;
        cstart = 1;
    }

    let (sent, sstart, sl): (&Dirent, usize, i32) = if idp.saveent.d_namlen != 0 {
        (&idp.saveent, 0, i32::from(idp.saveent.d_namlen))
    } else {
        (&idp.assocent, 1, i32::from(idp.assocent.d_namlen) - 1)
    };
    if sl > 0 {
        let sl = sl as usize;
        if sl != cl || sent.d_name[sstart..sstart + sl] != idp.current.d_name[cstart..cstart + cl] {
            if idp.assocent.d_namlen != 0 {
                iso_uiodir(idp, Ent::Assoc, idp.assocoff)?;
                idp.assocent.d_namlen = 0;
            }
            if idp.saveent.d_namlen != 0 {
                iso_uiodir(idp, Ent::Save, idp.saveoff)?;
                idp.saveent.d_namlen = 0;
            }
        }
    }
    idp.current.d_reclen = dirent_size(&idp.current) as u16;
    if assoc {
        idp.assocoff = idp.curroff;
        idp.assocent = idp.current;
    } else {
        idp.saveoff = idp.curroff;
        idp.saveent = idp.current;
    }
    Ok(())
}

/// `cd9660_readdir` (`vop_readdir`): vnode op for readdir.
pub fn cd9660_readdir<G: ProcGlue>(
    vdp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    let dp = vtoi(vdp);
    let imp = dp.mnt();
    let bmask = Off::from(imp.im_bmask);
    let lbs = imp.logical_block_size as usize;

    // These are passed to copyout(), so make sure there's no garbage being leaked in
    // padding or after short names. XXX Is it worth trying to figure out the type?
    let curroff = uio.uio_offset;
    let mut idp = Isoreaddir {
        saveent: dirent_zeroed(),
        assocent: dirent_zeroed(),
        current: dirent_zeroed(),
        saveoff: 0,
        assocoff: 0,
        curroff,
        uio,
        uio_off: curroff,
        eofflag: 1,
    };

    let mut bp: Option<BufRef> = None;
    let mut entryoffsetinblock = (idp.curroff & bmask) as usize;
    if entryoffsetinblock != 0 {
        let (t, _) = cd9660_bufatoff(&dp, idp.curroff)?;
        bp = Some(t);
    }
    let endsearch = dp.i_size.get() as Off;

    let mut error: Result<(), Option<Errno>> = Ok(());
    while idp.curroff < endsearch {
        // If offset is on a block boundary, read the next directory block. Release previous
        // if it exists.
        if idp.curroff & bmask == 0 {
            if let Some(t) = bp.take() {
                brelse(t);
            }
            match cd9660_bufatoff(&dp, idp.curroff) {
                Ok((t, _)) => bp = Some(t),
                Err(e) => {
                    error = Err(Some(e));
                    break;
                }
            }
            entryoffsetinblock = 0;
        }
        let Some(t) = bp.as_ref() else {
            kpanic(format_args!("cd9660_readdir: no directory buffer"));
        };
        let data = t.bytes();

        // Get pointer to next entry.
        let reclen = usize::from(data.get(entryoffsetinblock).copied().unwrap_or(0));
        if reclen == 0 {
            // skip to next block, if any
            idp.curroff = (idp.curroff & !bmask) + Off::from(imp.logical_block_size);
            continue;
        }

        if reclen < ISO_DIRECTORY_RECORD_SIZE {
            error = Err(Some(Errno::EINVAL));
            // illegal entry, stop
            break;
        }

        if entryoffsetinblock + reclen > lbs {
            error = Err(Some(Errno::EINVAL));
            // illegal directory, so stop looking
            break;
        }

        let Some(ep) = data
            .get(entryoffsetinblock..)
            .and_then(IsoDirectoryRecord::new)
        else {
            error = Err(Some(Errno::EINVAL));
            break;
        };
        idp.current.d_namlen = isonum_711(ep.name_len());

        if reclen < ISO_DIRECTORY_RECORD_SIZE + usize::from(idp.current.d_namlen) {
            error = Err(Some(Errno::EINVAL));
            // illegal entry, stop
            break;
        }

        let mut ino: Cdino = if isonum_711(ep.flags()) & 2 != 0 {
            isodirino(&ep, imp)
        } else {
            record_ino(t, entryoffsetinblock)
        };

        idp.curroff += reclen as Off;

        let off = idp.curroff;
        if imp.iso_ftype == ISO_FTYPE_RRIP {
            let mut namelen = 0u16;
            // The directory's block is the directory vnode's buffer: no read of the device
            // waits for it (no held block, `cd9660_rrip.rs`'s deviations).
            cd9660_rrip_getname(
                &ep,
                &mut idp.current.d_name,
                &mut namelen,
                &mut ino,
                imp,
                None,
            );
            idp.current.d_fileno = u64::from(ino);
            idp.current.d_namlen = namelen as u8;
            if idp.current.d_namlen != 0 {
                error = iso_uiodir(&mut idp, Ent::Current, off);
            }
        } else {
            // ISO_FTYPE_DEFAULT || ISO_FTYPE_9660
            idp.current.d_fileno = u64::from(ino);
            idp.current.d_name[..3].copy_from_slice(b"..\0");
            if idp.current.d_namlen == 1 && ep.name0() == 0 {
                idp.current.d_namlen = 1;
                error = iso_uiodir(&mut idp, Ent::Current, off);
            } else if idp.current.d_namlen == 1 && ep.name0() == 1 {
                idp.current.d_namlen = 2;
                error = iso_uiodir(&mut idp, Ent::Current, off);
            } else {
                let n = usize::from(idp.current.d_namlen);
                let mut namelen = 0u16;
                isofntrans(
                    &ep.name()[..n],
                    &mut idp.current.d_name,
                    &mut namelen,
                    imp.iso_ftype == ISO_FTYPE_9660,
                    isonum_711(ep.flags()) & 4 != 0,
                    imp.joliet_level,
                );
                idp.current.d_namlen = namelen as u8;
                error = if imp.iso_ftype == ISO_FTYPE_DEFAULT {
                    iso_shipdir(&mut idp)
                } else {
                    iso_uiodir(&mut idp, Ent::Current, off)
                };
            }
        }
        if error.is_err() {
            break;
        }

        entryoffsetinblock += reclen;
    }

    if error.is_ok() && imp.iso_ftype == ISO_FTYPE_DEFAULT {
        idp.current.d_namlen = 0;
        error = iso_shipdir(&mut idp);
    }
    // the C's error < 0: the user's buffer is full
    let error = error.or_else(|e| e.map_or(Ok(()), Err));

    if let Some(t) = bp {
        brelse(t);
    }

    idp.uio.uio_offset = idp.uio_off;
    *eofflag = idp.eofflag;

    error
}

/// The directory record at `off` in the token's bytes (`(ISODIR *)(bp->b_data + off)`), or
/// `None` when the block's tail is shorter than a record's fixed part.
fn record_at(t: &BufRef, off: usize) -> Option<IsoDirectoryRecord<'_>> {
    t.bytes().get(off..).and_then(IsoDirectoryRecord::new)
}

/// `cd9660_readlink` (`vop_readlink`): return target name of a symbolic link. Shouldn't we
/// get the parents vnode and read the data from there? This could eventually result in
/// deadlocks in `cd9660_lookup`. But otherwise the block read here is in the block buffer
/// two times.
pub fn cd9660_readlink<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let imp = ip.mnt();

    if imp.iso_ftype != ISO_FTYPE_RRIP {
        return Err(Errno::EINVAL);
    }

    // Get parents directory record block that this inode included.
    let number = ip.i_number;
    let blk = Daddr::from(number >> imp.im_bshift) << (imp.im_bshift - DEV_BSHIFT as i32);
    let (bp, error) = bread(&imp.im_devvp.bufs, blk, imp.logical_block_size);
    if error.is_err() {
        brelse(bp);
        return Err(Errno::EINVAL);
    }

    // Setup the directory pointer for this inode (a view of the token's bytes, made again
    // where it is read).
    let off = (number & imp.im_bmask as u32) as usize;

    // Just make sure, we have a right one....
    //   1: Check not cross boundary on block
    let fits = record_at(&bp, off).is_some_and(|d| {
        off + usize::from(isonum_711(d.length())) <= imp.logical_block_size as usize
    });
    if !fits {
        brelse(bp);
        return Err(Errno::EINVAL);
    }

    // Now get a buffer: a namei buffer for every caller (the module's deviations).
    let Some(mut symname) = pathbuf_get::<G>() else {
        kpanic(format_args!("cd9660_readlink: no namei buffer"));
    };

    // Ok, we just gathering a symbolic name in SL record.
    let mut symlen = 0u16;
    let found = record_at(&bp, off).map_or(0, |d| {
        cd9660_rrip_getsymname(&d, &mut symname[..], &mut symlen, imp, Some(blk))
    });
    if found == 0 {
        drop(symname);
        brelse(bp);
        return Err(Errno::EINVAL);
    }
    // Don't forget before you leave from home ;-)
    brelse(bp);

    // return with the symbolic name to caller's.
    let symlen = usize::from(symlen).min(MAXPATHLEN);
    <G::FileGlue as FileGlue>::uiomove(&mut symname[..symlen], uio)
}

/// `cd9660_link` (`vop_link`): read-only.
pub fn cd9660_link<G: ProcGlue>(dvp: &'static Vnode<G>, cnp: &mut Cn<'_, G>) -> Result<(), Errno> {
    let _ = vop_abortop(dvp, cnp);
    vput(dvp);
    Err(Errno::EROFS)
}

/// `cd9660_symlink` (`vop_symlink`): read-only.
pub fn cd9660_symlink<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let _ = vop_abortop(dvp, cnp);
    vput(dvp);
    Err(Errno::EROFS)
}

/// `cd9660_lock` (`vop_lock`): lock an inode.
pub fn cd9660_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    vtoi(vp).i_lock.enter(flags & LK_RWFLAGS)
}

/// `cd9660_unlock` (`vop_unlock`): unlock an inode.
pub fn cd9660_unlock<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    vtoi(vp).i_lock.exit();
    Ok(())
}

/// The token of a buffer handed to `cd9660_strategy` (`panic(9)` on a lent buffer with
/// none: no `bsd` caller lends one to a cd9660 vnode, the module's redesign).
fn strategy_token(bp: IoBuf) -> BufRef {
    match bp.into_token() {
        Some(t) => t,
        None => kpanic(format_args!("cd9660_strategy: a lent buffer with no token")),
    }
}

/// `cd9660_strategy` (`vop_strategy`): calculate the logical to physical mapping if not
/// done already, then call the device strategy routine. A hole's buffer is finished by
/// `biodone` (the C's `splbio(); biodone(bp); splx(s)`: one section of the cache's lock).
pub fn cd9660_strategy<G: ProcGlue>(vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
    let mut bp = strategy_token(bp);
    let ip = vtoi(vp);
    if vp.vtype() == VBLK || vp.vtype() == VCHR {
        kpanic(format_args!("cd9660_strategy: spec"));
    }
    let b = bp.buf();
    if b.blkno() == b.lblkno() {
        let mut blkno: Daddr = b.blkno();
        let error = vop_bmap(vp, b.lblkno(), None, Some(&mut blkno), None);
        b.set_blkno(blkno);
        if let Err(e) = error {
            b.set_error(Some(e));
            b.set(B_ERROR);
            biodone(bp);
            return Err(e);
        }
        if blkno == -1 {
            bp.clear();
        }
    }
    if bp.buf().blkno() == -1 {
        biodone(bp);
        return Ok(());
    }
    let devvp = ip.devvp();
    bp.buf().set_dev(devvp.rdev());
    drop(ip);
    let _ = vop_strategy(devvp, IoBuf::Token(bp));
    Ok(())
}

/// `cd9660_print` (`vop_print`): print out the contents of an inode.
pub fn cd9660_print() -> Result<(), Errno> {
    #[cfg(any(feature = "debug", feature = "diagnostic"))]
    crate::prf::printf(format_args!("tag VT_ISOFS, isofs vnode\n"));
    Ok(())
}

/// `cd9660_islocked` (`vop_islocked`): check for a locked inode.
pub fn cd9660_islocked<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    vtoi(vp).i_lock.status()
}

/// `cd9660_pathconf` (`vop_pathconf`): return POSIX pathconf information applicable to
/// cd9660 filesystems.
pub fn cd9660_pathconf<G: ProcGlue>(
    vp: &'static Vnode<G>,
    name: i32,
    retval: &mut Register,
) -> Result<(), Errno> {
    *retval = match name {
        _PC_LINK_MAX | _PC_CHOWN_RESTRICTED | _PC_NO_TRUNC => 1,
        _PC_NAME_MAX => {
            if vtoi(vp).mnt().iso_ftype == ISO_FTYPE_RRIP {
                NAME_MAX as Register
            } else {
                37
            }
        }
        _PC_TIMESTAMP_RESOLUTION => 1_000_000_000, // one billion nanoseconds
        _ => return Err(Errno::EINVAL),
    };

    Ok(())
}

/// `cd9660_kqfilter` (`vop_kqfilter`): attach a knote to the vnode.
pub fn cd9660_kqfilter<G: ProcGlue>(vp: &'static Vnode<G>, kn: &Knote<G>) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&Cd9660Filtops::<G>::READ)),
        EVFILT_WRITE => kn.kn_fop.set(Some(&Cd9660Filtops::<G>::WRITE)),
        EVFILT_VNODE => kn.kn_fop.set(Some(&Cd9660Filtops::<G>::VNODE)),
        _ => return Err(Errno::EINVAL),
    }

    // kn->kn_hook = (caddr_t)vp: a fresh knote holds no hook yet (as `ufs_kqfilter`'s).
    let _ = kn.set_hook_vnode(vp);

    vp.klist.insert_legacy(kn);

    Ok(())
}

/// `kn->kn_hook` of a cd9660 knote: its vnode.
fn kn_vnode<G: ProcGlue>(kn: &Knote<G>) -> &'static Vnode<G> {
    match kn.hook_vnode() {
        Some(vp) => vp,
        None => kpanic(format_args!("knote {kn:p}: no vnode")),
    }
}

/// `filt_cd9660detach`: unhooks the knote from the vnode.
fn filt_cd9660detach<G: ProcGlue>(kn: &Knote<G>) {
    let vp = kn_vnode(kn);

    vp.klist.remove_legacy(kn);
}

impl<G: ProcGlue> Filter<G> for Cd9660Read {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_cd9660detach`.
    fn detach(kn: &Knote<G>) {
        filt_cd9660detach(kn);
    }

    /// `filt_cd9660read`: the bytes past the file offset; always ready for poll and select.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        let vp = kn_vnode(kn);

        // filesystem is gone, so set the EOF flag and schedule the knote for deletion. The
        // vnode has no node any more (the module's redesign): it is reached only past this
        // test.
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }
        let node = vtoi(vp);

        kn.kn_data()
            .set((node.i_size.get() as i64).wrapping_sub(foffset(&kn.fp())));
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

impl<G: ProcGlue> Filter<G> for Cd9660Write {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_cd9660detach`.
    fn detach(kn: &Knote<G>) {
        filt_cd9660detach(kn);
    }

    /// `filt_cd9660write`: a file is always writable (as far as kqueue is concerned).
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

impl<G: ProcGlue> Filter<G> for Cd9660Vnode {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_cd9660detach`.
    fn detach(kn: &Knote<G>) {
        filt_cd9660detach(kn);
    }

    /// `filt_cd9660vnode`: records the vnode events (`NOTE_*`) the user asked for.
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

/// The nine slots `cd9660_vops` and `cd9660_specvops` share: `access`, `getattr`, `setattr`,
/// `inactive`, `reclaim`, `lock`, `unlock`, `print` and `islocked` are cd9660's in both.
macro_rules! cd9660_common_slots {
    () => {
        fn access(
            &self,
            vp: &'static Vnode<G>,
            mode: i32,
            cred: Cred<'_>,
            _p: &Thread<G>,
        ) -> Result<(), Errno> {
            cd9660_access(vp, mode, cred)
        }
        fn getattr(
            &self,
            vp: &'static Vnode<G>,
            vap: &mut Vattr,
            _cred: Cred<'_>,
            p: &Thread<G>,
        ) -> Result<(), Errno> {
            cd9660_getattr(vp, vap, p)
        }
        fn setattr(
            &self,
            vp: &'static Vnode<G>,
            vap: &mut Vattr,
            _cred: Cred<'_>,
            _p: &Thread<G>,
        ) -> Result<(), Errno> {
            cd9660_setattr(vp, vap)
        }
        fn inactive(&self, vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
            cd9660_inactive(vp, p)
        }
        fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
            cd9660_reclaim(vp)
        }
        fn lock(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
            cd9660_lock(vp, flags)
        }
        fn unlock(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
            cd9660_unlock(vp)
        }
        fn print(&self, _vp: &'static Vnode<G>) -> Result<(), Errno> {
            cd9660_print()
        }
        fn islocked(&self, vp: &'static Vnode<G>) -> i32 {
            cd9660_islocked(vp)
        }
    };
}

/// `cd9660_vops`: each slot the C's (34; the eight `eopnotsupp` slots the trait's defaults,
/// the module's deviations).
impl<G: ProcGlue> Vnops<G> for Cd9660Vnops<G> {
    cd9660_common_slots!();

    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        cd9660_lookup(dvp, vpp, cnp)
    }
    fn open(
        &self,
        _vp: &'static Vnode<G>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        cd9660_open()
    }
    fn close(
        &self,
        _vp: &'static Vnode<G>,
        _fflag: i32,
        _cred: Cred<'_>,
        _p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        cd9660_close()
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        cd9660_read(vp, uio)
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
        cd9660_ioctl()
    }
    fn kqfilter(&self, vp: &'static Vnode<G>, _fflag: i32, kn: &Knote<G>) -> Result<(), Errno> {
        cd9660_kqfilter(vp, kn)
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn fsync(
        &self,
        _vp: &'static Vnode<G>,
        _cred: Cred<'_>,
        _waitfor: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn link(
        &self,
        dvp: &'static Vnode<G>,
        _vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        cd9660_link(dvp, cnp)
    }
    fn symlink(
        &self,
        dvp: &'static Vnode<G>,
        _vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        _vap: &mut Vattr,
        _target: &[u8],
    ) -> Result<(), Errno> {
        cd9660_symlink(dvp, cnp)
    }
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        cd9660_readdir(vp, uio, eofflag)
    }
    fn readlink(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        cd9660_readlink(vp, uio)
    }
    fn abortop(&self, _dvp: &'static Vnode<G>, cnp: &mut Cn<'_, G>) -> Result<(), Errno> {
        default::vop_generic_abortop(cnp)
    }
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        cd9660_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        cd9660_strategy(vp, bp)
    }
    fn pathconf(
        &self,
        vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        cd9660_pathconf(vp, name, retval)
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        default::vop_generic_bwrite(bp)
    }
}

/// `cd9660_specvops`: cd9660's nine slots and `spec_vops`' others (the C's "XXX: Keep in sync
/// with spec_vops").
impl<G: ProcGlue> Vnops<G> for Cd9660SpecVnops<G> {
    cd9660_common_slots!();

    fn close(
        &self,
        vp: &'static Vnode<G>,
        fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        crate::fs::spec::spec_close(vp, fflag, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        crate::fs::spec::spec_read(vp, uio, ioflag)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        crate::fs::spec::spec_write(vp, uio, ioflag)
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
        crate::fs::spec::spec_fsync(vp, waitfor)
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        default::vop_generic_bwrite(bp)
    }

    // XXX: Keep in sync with spec_vops (one macro: the same slots as SpecVops's).
    crate::fs::spec::spec_shared_slots!();
}

const _: () = {
    assert!(
        size_of::<Dirent>() == 8 + 8 + 2 + 1 + 1 + 4 + MAXNAMLEN + 1,
        "struct dirent has no padding"
    );
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the vnode operations: those that need no disc (`setattr`'s refusals,
    // `pathconf`, the filters' events, `iso_shipdir`'s merge into a kernel buffer; LZ's
    // moved), and those over a mount (testcd9660: the makefs image, mkiso's): `read` with
    // its read-ahead, `readdir` (`d_fileno` and `va_fileid`, an associated file merged, a
    // buffer too short), `readlink`, `getattr` of a symbolic link of size 0, device nodes
    // (the special files' table, a node moved to its alias), the kqueue filters through
    // `kevent(2)`, and every other slot of the two tables once. User memory: testfs's path
    // slots and buffer, under the kernel memory's guard every fixture holds.

    use core::ptr;
    use std::sync::MutexGuard;
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use hal::sys::fcntl::{FREAD, O_RDONLY};
    use hal::sys::mount::ISOFSMNT_NORRIP;
    use hal::sys::stat::{S_IFBLK, S_IFCHR, S_IFDIR, S_IFLNK};
    use hal::sys::types::makedev;
    use hal::user::UserPtr;

    use super::*;
    use crate::descrip::{sys_close, sys_read};
    use crate::event::{EV_ADD, Kevent, NOTE_DELETE, NOTE_WRITE, sys_kevent, sys_kqueue};
    use crate::fs::isofs::cd9660::cd9660_extern::IsoFtype;
    use crate::fs::isofs::cd9660::testcd9660::mkiso::{self, Iso, er, nm, pn, px, record, sl, sp};
    use crate::fs::isofs::cd9660::testcd9660::{
        attach_image, image, mount_root, test_mnt, test_node,
    };
    use crate::fs::lookup::{namei, ndinit};
    use crate::fs::mount::Mount;
    use crate::fs::namei::{FOLLOW, LOCKLEAF, LOOKUP, NiDirp};
    use crate::fs::subr::{bdevvp, vattr_null, vfs, vops_id, vref, vrele};
    use crate::fs::syscalls::sys_open;
    use crate::fs::testfs::{TestGlue, UBUF, up};
    use crate::fs::ufs::ffs::testffs::unmount_root;
    use crate::fs::ufs::ufs::testufs::cn;
    use crate::fs::vnode::{LK_EXCLUSIVE, VNON};
    use crate::fs::vops::{vop_getattr, vop_lock, vop_readdir, vop_unlock};

    /// `cd9660_setattr` of `vap` on a vnode of type `t`.
    fn setattr(t: crate::fs::vnode::Vtype, vap: &Vattr) -> Result<(), Errno> {
        let imp = test_mnt(ISO_FTYPE_RRIP, |_| {});
        let (_ip, vp) = test_node(&imp, 20 << 11);
        vp.set_vtype(t);
        cd9660_setattr(vp, vap)
    }

    #[test]
    fn setattr_refuses_everything_but_device_sizes() {
        let (_g, _p) = attach_image(image::image_bytes());
        let mut va = Vattr::new();
        vattr_null(&mut va);
        assert_eq!(setattr(VREG, &va), Err(Errno::EINVAL));
        va.va_size = 0;
        assert_eq!(setattr(VDIR, &va), Err(Errno::EISDIR));
        assert_eq!(setattr(VREG, &va), Err(Errno::EROFS));
        assert_eq!(setattr(VLNK, &va), Err(Errno::EROFS));
        assert_eq!(setattr(VCHR, &va), Ok(()));
        assert_eq!(setattr(VFIFO, &va), Ok(()));
        assert_eq!(setattr(VNON, &va), Err(Errno::EINVAL));
        vattr_null(&mut va);
        va.va_uid = 0;
        assert_eq!(setattr(VCHR, &va), Err(Errno::EROFS));
        vattr_null(&mut va);
        va.va_vaflags |= VA_UTIMES_CHANGE;
        assert_eq!(setattr(VCHR, &va), Err(Errno::EROFS));
    }

    #[test]
    fn pathconf_depends_on_rock_ridge() {
        let (_g, _p) = attach_image(image::image_bytes());
        for (ftype, name_max) in [
            (IsoFtype::ISO_FTYPE_RRIP, 255),
            (IsoFtype::ISO_FTYPE_DEFAULT, 37),
        ] {
            let imp = test_mnt(ftype, |_| {});
            let (_ip, vp) = test_node(&imp, 20 << 11);
            let mut v: Register = 0;
            let mut ask = |name: i32| cd9660_pathconf(vp, name, &mut v).map(|()| v);
            assert_eq!(ask(_PC_NAME_MAX), Ok(name_max));
            assert_eq!(ask(_PC_LINK_MAX), Ok(1));
            assert_eq!(ask(_PC_CHOWN_RESTRICTED), Ok(1));
            assert_eq!(ask(_PC_NO_TRUNC), Ok(1));
            assert_eq!(ask(_PC_TIMESTAMP_RESOLUTION), Ok(1_000_000_000));
            assert_eq!(ask(hal::sys::unistd::_PC_PIPE_BUF), Err(Errno::EINVAL));
        }
    }

    #[test]
    fn filters_report_writability_and_vnode_events() {
        let kn = Knote::<TestGlue>::detached();
        assert!(<Cd9660Write as Filter<TestGlue>>::event(&kn, 0));
        assert_eq!(kn.kn_data().get(), 0);
        kn.kn_sfflags.set(NOTE_WRITE);
        assert!(!<Cd9660Vnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_DELETE)
        ));
        assert!(<Cd9660Vnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_WRITE)
        ));
        assert!(
            <Cd9660Vnode as Filter<TestGlue>>::event(&kn, i64::from(NOTE_REVOKE))
                && kn.has_flags(EV_EOF)
        );
        let kn = Knote::<TestGlue>::detached();
        assert!(
            <Cd9660Write as Filter<TestGlue>>::event(&kn, i64::from(NOTE_REVOKE))
                && kn.has_flags(EV_EOF | EV_ONESHOT)
        );
    }

    /// An `Isoreaddir` over the kernel buffer `buf`.
    fn isoreaddir<'a, 'b>(uio: &'a mut Uio<'b, TestGlue>) -> Isoreaddir<'a, 'b, TestGlue> {
        Isoreaddir {
            saveent: dirent_zeroed(),
            assocent: dirent_zeroed(),
            current: dirent_zeroed(),
            saveoff: 0,
            assocoff: 0,
            curroff: 0,
            uio,
            uio_off: 0,
            eofflag: 1,
        }
    }

    /// The entries `getdents` bytes hold: `(d_fileno, d_off, name)`, the name's NUL checked.
    fn entries(buf: &[u8]) -> Vec<(u64, Off, Vec<u8>)> {
        let mut out = Vec::new();
        let mut off = 0;
        while off + Dirent::NAME_OFFSET <= buf.len() {
            let Some(d) = Dirent::from_bytes(&buf[off..]) else {
                break;
            };
            if d.d_reclen == 0 {
                break;
            }
            let at = off + Dirent::NAME_OFFSET;
            let name = buf[at..at + usize::from(d.d_namlen)].to_vec();
            assert_eq!(buf[at + usize::from(d.d_namlen)], 0);
            out.push((d.d_fileno, d.d_off, name));
            off += usize::from(d.d_reclen);
        }
        out
    }

    #[test]
    fn readdir_ships_an_associated_file_before_its_file() {
        let (_g, p) = attach_image(image::image_bytes());
        let mut buf = [0u8; 512];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        let mut idp = isoreaddir(&mut uio);
        for (i, name) in [&b"FOO"[..], b"=FOO", b"BAR"].into_iter().enumerate() {
            idp.curroff = 100 * (i as Off + 1);
            idp.current.d_fileno = i as u64 + 1;
            idp.current.d_name[..name.len()].copy_from_slice(name);
            idp.current.d_namlen = name.len() as u8;
            iso_shipdir(&mut idp).unwrap();
        }
        idp.current.d_namlen = 0;
        iso_shipdir(&mut idp).unwrap();
        assert_eq!((idp.uio_off, idp.eofflag), (300, 1));
        let used = 512 - idp.uio.uio_resid;
        assert_eq!(
            entries(&buf[..used]),
            [
                (2, 200, b"=FOO".to_vec()),
                (1, 100, b"FOO".to_vec()),
                (3, 300, b"BAR".to_vec())
            ]
        );
    }

    #[test]
    fn readdir_stops_when_the_buffer_is_full_and_refuses_slashes() {
        let (_g, p) = attach_image(image::image_bytes());
        let mut buf = [0u8; 40];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        let mut idp = isoreaddir(&mut uio);
        idp.current.d_name[..3].copy_from_slice(b"a/b");
        idp.current.d_namlen = 3;
        assert_eq!(
            iso_uiodir(&mut idp, Ent::Current, 1),
            Err(Some(Errno::EINVAL))
        );
        idp.current.d_name[..3].copy_from_slice(b"abc");
        assert_eq!(iso_uiodir(&mut idp, Ent::Current, 1), Ok(()));
        assert_eq!(iso_uiodir(&mut idp, Ent::Current, 2), Err(None));
        assert_eq!((idp.uio_off, idp.eofflag), (1, 0));
    }

    /// `image` mounted as the root with `flags`: the guard, the thread, the mount.
    fn setup(
        image: Vec<u8>,
        flags: i32,
    ) -> (
        MutexGuard<'static, ()>,
        &'static Thread<TestGlue>,
        &'static Mount<TestGlue>,
    ) {
        let (g, p) = attach_image(image);
        let (mp, _) = mount_root(p, flags);
        (g, p, mp)
    }

    /// The vnode `path` names, locked and referenced.
    fn lookup(p: &Thread<TestGlue>, path: &[u8]) -> Result<&'static Vnode<TestGlue>, Errno> {
        let mut nd = ndinit(LOOKUP, LOCKLEAF | FOLLOW, NiDirp::Sys(path), p);
        namei(&mut nd)?;
        Ok(nd.ni_vp.expect("a vnode"))
    }

    /// `VOP_READDIR` of the locked `vp` from `offset` into a kernel buffer of `len` bytes:
    /// the result, the entries, the offset left and the end-of-file flag.
    fn readdir(
        p: &Thread<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        offset: Off,
        len: usize,
    ) -> (Result<(), Errno>, Vec<(u64, Off, Vec<u8>)>, Off, i32) {
        let mut buf = vec![0u8; len];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        uio.uio_offset = offset;
        let mut eof = 0;
        let r = vop_readdir(vp, &mut uio, Cred::real(p.ucred()), &mut eof);
        let (off, used) = (uio.uio_offset, len - uio.uio_resid);
        (r, entries(&buf[..used]), off, eof)
    }

    /// `VOP_READ` of `len` bytes at `off` of the locked `vp` into a kernel buffer.
    fn read(
        p: &Thread<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        off: Off,
        len: usize,
    ) -> Result<Vec<u8>, Errno> {
        let mut buf = vec![0u8; len];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        uio.uio_offset = off;
        cd9660_read(vp, &mut uio)?;
        let used = len - uio.uio_resid;
        buf.truncate(used);
        Ok(buf)
    }

    /// `getattr` of the locked `vp`.
    fn getattr(p: &Thread<TestGlue>, vp: &'static Vnode<TestGlue>) -> Vattr {
        let mut va = Vattr::new();
        vop_getattr(vp, &mut va, Cred::real(p.ucred()), p).expect("getattr");
        va
    }

    /// The Rock Ridge root's entries: `d_fileno` is what `getattr` gives as `va_fileid` for
    /// the vnode the name finds (`pwd(1)` and `ls -i` agree), `d_off` the next entry's
    /// offset; the whole directory in one call (end of file), the offset left the last
    /// entry's (the C's `uio_off`, not the block's end).
    #[test]
    fn readdir_entries_are_the_names_vnodes() {
        let (_g, p, mp) = setup(image::image_bytes(), 0);
        let rvp = lookup(p, b"/").unwrap();
        let (r, ents, off, eof) = readdir(p, rvp, 0, 4096);
        r.unwrap();
        assert_eq!(eof, 1);
        assert_eq!(off, 558);
        assert_eq!(ents.last().map(|e| e.1), Some(558));
        let names: Vec<&[u8]> = ents.iter().map(|e| &e.2[..]).collect();
        assert_eq!(names, [&b"."[..], b"..", b"m10c-iso.txt", b"sub"]);
        vput(rvp);
        for (fileno, _, name) in &ents {
            let mut path = b"/".to_vec();
            path.extend_from_slice(name);
            let vp = lookup(p, &path).unwrap();
            assert_eq!(getattr(p, vp).va_fileid, *fileno, "{name:?}");
            vput(vp);
        }
        assert_eq!(ents[2].0, 20 * 2048 + 324);
        unmount_root(p, mp);
    }

    /// A buffer that holds two entries at a time: each call ships what fits, says the
    /// directory goes on (`eofflag` 0), and leaves the offset after the last shipped entry,
    /// where the next call resumes; the names are the whole directory's, once each.
    #[test]
    fn readdir_resumes_where_a_full_buffer_stopped() {
        let (_g, p, mp) = setup(image::image_bytes(), 0);
        let rvp = lookup(p, b"/").unwrap();
        let mut names = Vec::new();
        let mut off = 0;
        loop {
            let (r, ents, next, eof) = readdir(p, rvp, off, 72);
            r.unwrap();
            assert!(ents.len() <= 2);
            names.extend(ents.into_iter().map(|e| e.2));
            off = next;
            if eof == 1 {
                break;
            }
        }
        assert_eq!(
            names,
            [
                b".".to_vec(),
                b"..".to_vec(),
                b"m10c-iso.txt".to_vec(),
                b"sub".to_vec()
            ]
        );
        vput(rvp);
        unmount_root(p, mp);
    }

    /// A plain disc whose root holds `FOO.TXT;1` as an associated file, then as a file, then
    /// `BAR.TXT;1` (blocks 20, 21, 22 their data; each record 42 bytes).
    fn assoc_image() -> Vec<u8> {
        let mut iso = Iso::new(24);
        iso.dir(
            18,
            &[
                record(18, 2048, 2, &[0], &[]),
                record(18, 2048, 2, &[1], &[]),
                record(20, 6, 4, b"FOO.TXT;1", &[]),
                record(21, 4, 0, b"FOO.TXT;1", &[]),
                record(22, 4, 0, b"BAR.TXT;1", &[]),
            ],
        );
        iso.put(20, 0, b"assoc\n");
        iso.put(21, 0, b"foo\n");
        iso.put(22, 0, b"bar\n");
        iso.pvd(16, b"ASSOC", 24, &record(18, 2048, 2, &[0], &[]));
        iso.term(17);
        iso.finish()
    }

    /// The default format holds a file back until the next record shows whether it is the
    /// associated file of the same name, and ships the pair together: `=FOO.TXT` (the
    /// associated one, its own offset) before `FOO.TXT`; a lookup of `=foo.txt` finds the
    /// associated file.
    #[test]
    fn readdir_merges_an_associated_file_and_lookup_finds_it() {
        let (_g, p, mp) = setup(assoc_image(), ISOFSMNT_NORRIP);
        let rvp = lookup(p, b"/").unwrap();
        let (r, ents, _, eof) = readdir(p, rvp, 0, 4096);
        r.unwrap();
        assert_eq!(eof, 1);
        let got: Vec<(Off, Vec<u8>)> = ents.into_iter().map(|e| (e.1, e.2)).collect();
        assert_eq!(
            got,
            [
                (34, b".".to_vec()),
                (68, b"..".to_vec()),
                (110, b"=FOO.TXT".to_vec()),
                (152, b"FOO.TXT".to_vec()),
                (194, b"BAR.TXT".to_vec())
            ]
        );
        vput(rvp);
        let vp = lookup(p, b"/=foo.txt").unwrap();
        assert_eq!(read(p, vp, 0, 100).unwrap(), b"assoc\n");
        vput(vp);
        let vp = lookup(p, b"/foo.txt").unwrap();
        assert_eq!(read(p, vp, 0, 100).unwrap(), b"foo\n");
        vput(vp);
        unmount_root(p, mp);
    }

    /// A plain disc with a file of three blocks and 100 bytes at block 20 (each byte its
    /// offset modulo 251).
    fn long_file_image() -> (Vec<u8>, Vec<u8>) {
        let data: Vec<u8> = (0..3 * 2048 + 100).map(|i| (i % 251) as u8).collect();
        let mut iso = Iso::new(24);
        iso.dir(
            18,
            &[
                record(18, 2048, 2, &[0], &[]),
                record(18, 2048, 2, &[1], &[]),
                record(20, data.len() as u32, 0, b"LONG.DAT;1", &[]),
            ],
        );
        iso.put(20, 0, &data);
        iso.pvd(16, b"LONG", 24, &record(18, 2048, 2, &[0], &[]));
        iso.term(17);
        (iso.finish(), data)
    }

    /// A read within the file, at and past its end, at a negative offset, of nothing; a
    /// sequential read block by block takes the read-ahead path (`ci_lastr + 1 == lbn`:
    /// `breadn`) and reads the same bytes; `IN_ACCESS` is set.
    #[test]
    fn read_returns_the_file_s_bytes_with_and_without_read_ahead() {
        let (img, data) = long_file_image();
        let (_g, p, mp) = setup(img, ISOFSMNT_NORRIP);
        let vp = lookup(p, b"/long.dat").unwrap();
        assert_eq!(read(p, vp, 10, 20).unwrap(), &data[10..30]);
        assert_eq!(read(p, vp, 6000, 1000).unwrap(), &data[6000..]);
        assert_eq!(read(p, vp, data.len() as Off, 10).unwrap(), b"");
        assert_eq!(read(p, vp, -1, 10), Err(Errno::EINVAL));
        assert_eq!(read(p, vp, 0, 0).unwrap(), b"");
        let ip = vtoi(vp);
        assert_ne!(ip.i_flag.get() & IN_ACCESS, 0);
        let mut all = Vec::new();
        for lbn in 0..4 {
            all.extend(read(p, vp, lbn * 2048, 2048).unwrap());
        }
        assert_eq!(all, data);
        assert_eq!(ip.i_ci_lastr.get(), 3);
        drop(ip);
        vput(vp);
        unmount_root(p, mp);
    }

    /// A Rock Ridge disc of the special files: the character device `cdev` (1, 3), the block
    /// device `bdev` (2, 7), the symbolic link `lnk -> ../target` whose record says size 0,
    /// and the directory `dir`.
    fn special_image() -> Vec<u8> {
        let mut iso = Iso::new(22);
        let mut root_su = sp();
        root_su.extend(px(S_IFDIR | 0o755, 2, 0, 0));
        root_su.extend(er());
        let mut dot_su = px(S_IFDIR | 0o755, 2, 0, 0);
        dot_su.extend(nm(b""));
        let su = |mode: u32, extra: Vec<u8>, name: &[u8]| {
            let mut s = px(mode, 1, 5, 6);
            s.extend(extra);
            s.extend(nm(name));
            s
        };
        let dev = |d: hal::sys::types::Dev| pn(0, d.cast_unsigned());
        iso.dir(
            18,
            &[
                record(18, 2048, 2, &[0], &root_su),
                record(18, 2048, 2, &[1], &px(S_IFDIR | 0o755, 2, 0, 0)),
                record(
                    0,
                    0,
                    0,
                    b"BDEV.;1",
                    &su(S_IFBLK | 0o640, dev(makedev(2, 7)), b"bdev"),
                ),
                record(
                    0,
                    0,
                    0,
                    b"CDEV.;1",
                    &su(S_IFCHR | 0o640, dev(makedev(1, 3)), b"cdev"),
                ),
                record(
                    19,
                    2048,
                    2,
                    b"DIR",
                    &su(S_IFDIR | 0o755, Vec::new(), b"dir"),
                ),
                record(
                    0,
                    0,
                    0,
                    b"LNK.DAT;1",
                    &su(S_IFLNK | 0o777, sl(&[(4, b""), (0, b"target")]), b"lnk"),
                ),
            ],
        );
        iso.dir(
            19,
            &[
                record(19, 2048, 2, &[0], &dot_su),
                record(18, 2048, 2, &[1], &px(S_IFDIR | 0o755, 2, 0, 0)),
            ],
        );
        iso.pvd(16, b"SPECIAL", 22, &record(18, 2048, 2, &[0], &[]));
        iso.term(17);
        iso.finish()
    }

    /// Device nodes take the special files' table (`cd9660_specvops`): cd9660's attributes,
    /// `setattr` of a size `Ok`; a block device whose number has a vnode already (`bdevvp`'s)
    /// moves its node to that alias, which the node then names (`checkalias`). A symbolic
    /// link whose record says size 0 has its target's length as its size (`getattr` reads
    /// the link), and `readlink` of a plain disc is `EINVAL`.
    #[test]
    fn special_files_take_the_special_table_and_a_link_its_length() {
        let (_g, p, mp) = setup(special_image(), 0);
        let cdev = lookup(p, b"/cdev").unwrap();
        assert_eq!(cdev.vtype(), VCHR);
        assert_eq!(
            cdev.ops_id(),
            vops_id::<TestGlue>(&vfs::<TestGlue>().cd9660.specvops)
        );
        let va = getattr(p, cdev);
        assert_eq!(
            (va.va_rdev, va.va_mode, va.va_uid, va.va_gid),
            (makedev(1, 3), 0o640, 5, 6)
        );
        let mut size = Vattr::new();
        vattr_null(&mut size);
        size.va_size = 0;
        assert_eq!(cd9660_setattr(cdev, &size), Ok(()));
        vput(cdev);

        let alias = bdevvp::<TestGlue>(makedev(2, 7)).unwrap().unwrap();
        let bdev = lookup(p, b"/bdev").unwrap();
        assert!(ptr::eq(bdev, alias), "the node moved to bdevvp's vnode");
        let ip = vtoi(bdev);
        assert!(ip.i_vnode.get().is_some_and(|v| ptr::eq(v, alias)));
        assert_eq!(bdev.vtype(), VBLK);
        assert_eq!(
            bdev.ops_id(),
            vops_id::<TestGlue>(&vfs::<TestGlue>().cd9660.specvops)
        );
        drop(ip);
        vput(bdev);
        vrele(alias);

        let lnk = {
            let mut nd = ndinit(LOOKUP, LOCKLEAF, NiDirp::Sys(b"/lnk"), p);
            namei(&mut nd).unwrap();
            nd.ni_vp.unwrap()
        };
        assert_eq!(lnk.vtype(), VLNK);
        assert_eq!(vtoi(lnk).i_size.get(), 0);
        assert_eq!(getattr(p, lnk).va_size, 9);
        let mut buf = [0u8; 64];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        cd9660_readlink(lnk, &mut uio).unwrap();
        assert_eq!(uio.uio_resid, 64 - 9);
        assert_eq!(&buf[..9], b"../target");
        vput(lnk);
        unmount_root(p, mp);

        // readlink needs Rock Ridge
        let (mp, _) = mount_root(p, ISOFSMNT_NORRIP);
        let vp = lookup(p, b"/lnk.dat").unwrap();
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        assert_eq!(cd9660_readlink(vp, &mut uio), Err(Errno::EINVAL));
        vput(vp);
        unmount_root(p, mp);
    }

    /// `kevent(2)`: `cd9660_kqfilter` hooks each filter on the vnode's klist: the read
    /// filter answers the bytes past the offset, the write filter is always ready, the vnode
    /// filter has nothing yet; closing the kqueue detaches them; an unknown filter is
    /// `EINVAL`.
    #[test]
    fn the_filters_attach_through_kevent() {
        let (_g, p, mp) = setup(image::image_bytes(), 0);
        let _ = crate::event::kqueue_init::<TestGlue>();
        let fd = sys_open(p, up(0, b"/m10c-iso.txt"), O_RDONLY, 0).expect("open");
        assert_eq!(sys_read(p, fd, UBUF, 5), Ok(5));
        let kfd = sys_kqueue(p).expect("kqueue");
        let changes = [
            Kevent {
                ident: fd as usize,
                filter: EVFILT_READ,
                flags: EV_ADD,
                ..Kevent::default()
            },
            Kevent {
                ident: fd as usize,
                filter: EVFILT_WRITE,
                flags: EV_ADD,
                ..Kevent::default()
            },
            Kevent {
                ident: fd as usize,
                filter: EVFILT_VNODE,
                flags: EV_ADD,
                fflags: NOTE_WRITE | NOTE_DELETE,
                ..Kevent::default()
            },
        ];
        UserPtr::<Kevent>::new(UBUF)
            .copyout_slice(&changes)
            .unwrap();
        let events = UserPtr::<Kevent>::new(UBUF + 0x100);
        assert_eq!(
            sys_kevent(p, kfd, UserPtr::new(UBUF), 3, events, 3, 0),
            Ok(2)
        );
        let mut got = [Kevent::default(); 3];
        events.copyin_slice(&mut got).unwrap();
        let of = |f: i16| *got[..2].iter().find(|e| e.filter == f).expect("an event");
        assert_eq!(of(EVFILT_READ).data, 7);
        assert_eq!(of(EVFILT_WRITE).data, 0);
        sys_close(p, kfd).expect("close kqueue");
        sys_close(p, fd).expect("close");

        let vp = lookup(p, b"/m10c-iso.txt").unwrap();
        let kn = Knote::<TestGlue>::detached();
        kn.kn_filter().set(99);
        assert_eq!(cd9660_kqfilter(vp, &kn), Err(Errno::EINVAL));
        vput(vp);
        unmount_root(p, mp);
    }

    /// The slots of `cd9660_vops` no other test reaches, each once through the table:
    /// `open`, `close`, `ioctl`, `fsync`, `print`, `islocked`, `lock`/`unlock`, `abortop`,
    /// `link`/`symlink` (`EROFS`, the directory released), the eight `eopnotsupp` slots,
    /// `access` (the Rock Ridge mode), and the two functions in no table.
    #[test]
    fn the_other_slots_answer_as_the_c() {
        let (_g, p, mp) = setup(image::image_bytes(), 0);
        let vp = lookup(p, b"/m10c-iso.txt").unwrap();
        let ops = &vfs::<TestGlue>().cd9660.vops;
        let cred = Cred::real(p.ucred());
        assert_eq!(ops.open(vp, FREAD, cred, p), Ok(()));
        assert_eq!(ops.close(vp, FREAD, cred, Some(p)), Ok(()));
        assert_eq!(ops.ioctl(vp, 0, &mut [], 0, cred, p), Err(Errno::ENOTTY));
        assert_eq!(ops.fsync(vp, cred, 0, p), Ok(()));
        assert_eq!(ops.print(vp), Ok(()));
        assert_ne!(ops.islocked(vp), 0);
        // m10c-iso.txt is 0644, owned by root: the test's root may read it.
        assert_eq!(ops.access(vp, crate::fs::vnode::VREAD, cred, p), Ok(()));
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_write(&mut iov, b"x", None);
        assert_eq!(ops.write(vp, &mut uio, 0, cred), Err(Errno::EOPNOTSUPP));
        let mut fl = hal::sys::fcntl::Flock::default();
        assert_eq!(
            ops.advlock(vp, crate::file::LockOwner::File(0), 0, &mut fl, 0),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(cd9660_mmap(), Err(Errno::EINVAL));
        assert_eq!(cd9660_seek(), Ok(()));
        vput(vp);

        let dvp = lookup(p, b"/sub").unwrap();
        let mut c = cn(p, p.ucred(), crate::fs::namei::CREATE, 0, b"x");
        let mut vpp = None;
        let mut vattr = Vattr::new();
        assert_eq!(
            ops.create(dvp, &mut vpp, &mut c, &mut vattr),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            ops.mknod(dvp, &mut vpp, &mut c, &mut vattr),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            ops.mkdir(dvp, &mut vpp, &mut c, &mut vattr),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(ops.remove(dvp, dvp, &mut c), Err(Errno::EOPNOTSUPP));
        assert_eq!(ops.rmdir(dvp, dvp, &mut c), Err(Errno::EOPNOTSUPP));
        assert_eq!(
            ops.rename(
                dvp,
                dvp,
                &mut c,
                dvp,
                None,
                &mut cn(p, p.ucred(), crate::fs::namei::RENAME, 0, b"y")
            ),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(ops.abortop(dvp, &mut c), Ok(()));
        // link and symlink release the directory (locked and referenced: vput)
        vref(dvp);
        let _ = vop_lock(dvp, LK_EXCLUSIVE);
        assert_eq!(ops.link(dvp, dvp, &mut c), Err(Errno::EROFS));
        vref(dvp);
        let _ = vop_lock(dvp, LK_EXCLUSIVE);
        assert_eq!(
            ops.symlink(dvp, &mut vpp, &mut c, &mut vattr, b"t"),
            Err(Errno::EROFS)
        );
        let _ = vop_unlock(dvp);
        vrele(dvp);
        unmount_root(p, mp);
        let _ = mkiso::BS;
    }
}
/* </TESTS> */
