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
 * Copyright (c) 1997 Manuel Bouyer.
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)ufs_vnops.c	8.14 (Berkeley) 10/26/94
 * Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! The second extended file system's vnode operations: create, mknod, open, access,
//! attributes (with `ext2fs_chmod` and `ext2fs_chown`), remove, link, rename, mkdir, rmdir,
//! symlink, readlink, pathconf, advisory locks, `ext2fs_makeinode`, fsync and reclaim, and the
//! tables for files (`ext2fs_vops`, [`Ext2fsVnops`]) and for the special files that live on an
//! ext2fs (`ext2fs_specvops`, [`Ext2fsSpecVnops`]). The directory operations they build on
//! (`ext2fs_lookup`, `ext2fs_readdir`, `ext2fs_direnter`, ...) are `ext2fs_lookup.rs`.
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_vnops.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_vnops.rs@3c62ede68802
//!
//! ## Deviations
//! - `option FIFO` is in GENERIC but `miscfs/fifofs` is not ported: `ext2fs_fifovops` and
//!   `ext2fsfifo_reclaim` come with it (and the C's slip `vop_access = ufsfifo_close` in that
//!   table with them). Meanwhile `mknod`/`mkfifo` refuse fifos (`vfs_syscalls.rs`) and
//!   `ext2fs_vinit` refuses a fifo already on the disk (`ext2fs_subr.rs`), both with
//!   `EOPNOTSUPP`, as a kernel without `FIFO` does.
//! - `pool_put(&namei_pool, cnp->cn_pnbuf)` is `ufs_vnops.rs`'s [`pnbuf_free`]; credentials
//!   the C dereferences go through its [`ucred`], which panics on `NOCRED`/`FSCRED`.
//! - `ext2fs_rename`'s `vfs_relookup` calls keep the references balanced when the lookup
//!   fails, as `ufs_rename` does: the C ignores the result of the two lookups of the source
//!   (then follows a NULL vnode, or releases `fdvp` once more than it holds), and lets
//!   `ext2fs_checkpath` consume the reference on `tdvp` that a failed lookup of the target
//!   releases again. Here `tdvp` gets `ufs_rename`'s compensating reference around
//!   `ext2fs_checkpath`; the source's lookup of the `fvp == tvp` case returns its error
//!   (`ENOENT` when the name went away; a success always brings the vnode, and none is a
//!   `panic(9)`, as `ufs_rename`'s, where the C's `VOP_REMOVE` would follow NULL); the final
//!   lookup of the source holds `fdvp` across
//!   the call and, when it fails, ends the rename as when the name has disappeared (0, as the
//!   C's ignored result gives). LZ's.
//! - `ext2fs_setattr` keeps the C's precedence in the flags it sets: `SF_APPEND` sets
//!   `EXT2_APPEND` only, otherwise `SF_IMMUTABLE` sets `EXT2_IMMUTABLE`.
//! - The inode's in-core dinode (`i_e2din`) is freed with the inode's last handle, not at
//!   `ext2fs_reclaim` (`ext2fs_dinode.rs`, D-N5.6-2): a handle kept past the reclaim finds a
//!   dinode, never a freed one.
//!
//! ## Redesign
//! - N5.6c2: LZ's functions over `bsd`'s `struct vop_*_args` and its `Vops` statics
//!   (`EXT2FS_VOPS`, `EXT2FS_SPECVOPS`) -> generic functions with the kernel's `Vnops<G>`
//!   method shapes (`vp`, the C's arguments, the out-parameters `&mut`) over the kernel's
//!   vnode, `vop_*` functions and the inode's handle (`vtoi`, one per operation, the C's one
//!   `VTOI`), and the C's two tables as two types implementing the kernel's `Vnops<G>`
//!   (decision 46: the C's table is the trait): [`Ext2fsVnops`] and [`Ext2fsSpecVnops`],
//!   written slot by slot as `MfsVnops` is (D-N5.6-4). The slots the C fills with the UFS
//!   layer's operations call the kernel's `ufs_*` (`ufs_close`, `ufs_ioctl`,
//!   `ufs_kqfilter`, `ufs_lock`, `ufs_unlock`, `ufs_strategy`, `ufs_print`, `ufs_islocked`,
//!   `ufsspec_*`), the generic ones the kernel's `vop_generic_*`, the special file's the
//!   kernel's `spec_*` (`spec_shared_slots!`, the one list beside `spec_vops`); the C's
//!   NULL `vop_revoke` of `ext2fs_vops` is the trait's default (`EOPNOTSUPP`), and no slot
//!   has a body the C's table lacks. The two tables are members of the glue's `UfsTables`
//!   (`ext2fs_vops`, `ext2fs_specvops`), one address each, which the VFS's registry keys them
//!   by (`vops_id`; a promoted constant has no address of its own, D-b2-10); each carries a
//!   byte so that neither is zero-sized. N5.6c3 names them in `ext2fs_vget`.
//! - `ext2fs_rename`'s `abortit:` and its exits are the UFS layer's (`ufs_vnops.rs`'s
//!   `rename_abortit`, `Abort`, `RenameExit`): the C's `ext2fs_rename` repeats `ufs_rename`'s
//!   label word for word; `out:` is ext2fs's own ([`rename_out`]: `fdvp` was released
//!   before). `vmount` and `rdonly` are the UFS layer's too.
//! - `ext2fs_setattr` reads `kern.securelevel` through its inner function's parameter
//!   (`ext2fs_setattr_at(.., &SECURELEVEL)`), as `ufs_setattr` does (D-N5.5b-7), so a host
//!   test passes a word of its own.
//! - `ext2fs_mknod` calls `vgone` once, as the C: LZ called it twice.
//! - Cost against the C: the same calls in the same order; no lock and no spl of its own.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::sys::errno::Errno;
use hal::sys::fcntl::{FWRITE, Flock, O_APPEND};
use hal::sys::mount::{MNT_NOATIME, MNT_WAIT};
use hal::sys::param::PAGE_SIZE;
use hal::sys::stat::{ACCESSPERMS, ALLPERMS, APPEND, IMMUTABLE, S_ISTXT, SF_APPEND, SF_IMMUTABLE};
use hal::sys::syslimits::LINK_MAX;
use hal::sys::types::{Daddr, Dev, Gid, Mode, Nlink, Off, Register, Uid};
use hal::sys::unistd::_PC_TIMESTAMP_RESOLUTION;

use super::ext2fs_alloc::ext2fs_inode_alloc;
use super::ext2fs_bmap::ext2fs_bmap;
use super::ext2fs_dinode::{EXT2_APPEND, EXT2_IMMUTABLE, EXT2_MAXSYMLINKLEN};
use super::ext2fs_dir::{EXT2_FT_DIR, Ext2fsDirtemplate};
use super::ext2fs_inode::{
    ext2fs_inactive, ext2fs_itimes, ext2fs_setsize, ext2fs_size, ext2fs_truncate, ext2fs_update,
};
use super::ext2fs_lookup::{
    ext2fs_checkpath, ext2fs_dirempty, ext2fs_direnter, ext2fs_dirremove, ext2fs_dirrewrite,
    ext2fs_lookup, ext2fs_readdir, has_ftype,
};
use super::ext2fs_readwrite::{ext2fs_read, ext2fs_write};
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::cred::{groupmember, suser_ucred};
use crate::event::Knote;
use crate::file::{FileGlue, LockOwner};
use crate::fs::cache::cache_purge;
use crate::fs::default;
use crate::fs::lockf::lf_advlock;
use crate::fs::lookup::vfs_relookup;
use crate::fs::namei::{DELETE, ISDOTDOT, LOCKLEAF, LOCKPARENT, MODMASK, SAVESTART};
use crate::fs::subr::{vaccess, vflushbuf, vgone, vput, vref, vrele};
use crate::fs::ufs::ufs::dinode::{IFDIR, IFLNK, IFMT, IFREG, ISGID, ISUID, Ufsino};
use crate::fs::ufs::ufs::inode::vtoi;
use crate::fs::ufs::ufs::inode::{IN_ACCESS, IN_CHANGE, IN_RENAME, IN_UPDATE, Inode, InodeRef};
use crate::fs::ufs::ufs::ufs_ihash::ufs_ihashrem;
use crate::fs::ufs::ufs::ufs_lookup::{ufs_dirbad, vmount};
use crate::fs::ufs::ufs::ufs_vnops::{
    Abort, RenameExit, pnbuf_free, rdonly, rename_abortit, ucred, ufs_close, ufs_ioctl,
    ufs_islocked, ufs_kqfilter, ufs_lock, ufs_pathconf, ufs_print, ufs_strategy, ufs_unlock,
    ufsspec_close, ufsspec_read, ufsspec_write,
};
use crate::fs::vnode::{
    IO_NODELOCKED, IO_SYNC, LK_EXCLUSIVE, VA_UTIMES_CHANGE, VA_UTIMES_NULL, VBLK, VCHR, VDIR, VLNK,
    VNON, VNOVAL, VREG, VTEXT, VWRITE, Vattr, Vnode, iftovt, makeimode, vn_lock,
};
use crate::fs::vnops::{RdwrBuf, vn_rdwr};
use crate::fs::vops::vop_unlock;
use crate::fs::vops::{Cn, Cred, IoBuf, Vnops, vop_abortop, vop_access, vop_read, vop_remove};
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::SECURELEVEL;
use crate::uio::{Uio, UioRw};
use crate::vm::vnode::uvm_vnp_uncache;

/// `ext2fs_vops`: the operations of a file, directory or symbolic link on an ext2fs (the
/// glue's `UfsTables::ext2fs_vops`, one address).
pub struct Ext2fsVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member: the registry tells tables apart by
    /// their addresses.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> Ext2fsVnops<G> {
    /// The table (the glue's `UfsTables::ext2fs_vops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `ext2fs_specvops`: the operations of a device special file on an ext2fs (the glue's
/// `UfsTables::ext2fs_specvops`, one address).
pub struct Ext2fsSpecVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member (as [`Ext2fsVnops`]).
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> Ext2fsSpecVnops<G> {
    /// The table (the glue's `UfsTables::ext2fs_specvops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `ip->i_e2fs_nlink += d` (`d` is -1, 1 or -2).
fn nlink_add<G: ProcGlue>(ip: &Inode<G>, d: i32) {
    let din = ip.i_e2din();
    din.set_i_e2fs_nlink((i32::from(din.i_e2fs_nlink()) + d) as u16);
}

/// `ip->i_e2fs_mode` as a `mode_t`.
fn e2mode<G: ProcGlue>(ip: &Inode<G>) -> Mode {
    Mode::from(ip.i_e2din().i_e2fs_mode())
}

/// `ext2fs_create` (`vop_create`): create a regular file.
pub fn ext2fs_create<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    ext2fs_makeinode(makeimode(vap.va_type, vap.va_mode), dvp, vpp, cnp)
}

/// `ext2fs_mknod` (`vop_mknod`): mknod vnode call.
pub fn ext2fs_mknod<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    ext2fs_makeinode(makeimode(vap.va_type, vap.va_mode), dvp, vpp, cnp)?;
    let Some(vp) = *vpp else {
        kpanic(format_args!("ext2fs_mknod: no vnode"));
    };
    let ip = vtoi(vp);
    ip.set_flag(IN_ACCESS | IN_CHANGE | IN_UPDATE);
    if vap.va_rdev != VNOVAL as Dev {
        // Want to be able to use this to make badblock inodes, so don't truncate the dev
        // number.
        ip.i_e2din().set_e2di_rdev((vap.va_rdev as u32).to_le());
    }
    // Remove inode so that it will be reloaded by VFS_VGET and checked to see if it is an
    // alias of an existing entry in the inode cache.
    drop(ip);
    vput(vp);
    vp.set_vtype(VNON);
    vgone(vp);
    *vpp = None;
    Ok(())
}

/// `ext2fs_open` (`vop_open`): open called. Just check the `APPEND` flag.
pub fn ext2fs_open<G: ProcGlue>(vp: &'static Vnode<G>, mode: i32) -> Result<(), Errno> {
    // Files marked append-only must be opened for appending.
    if vtoi(vp).i_e2din().i_e2fs_flags() & EXT2_APPEND != 0 && mode & (FWRITE | O_APPEND) == FWRITE
    {
        return Err(Errno::EPERM);
    }
    Ok(())
}

/// `ext2fs_access` (`vop_access`).
pub fn ext2fs_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let d = ip.i_e2din();

    // If immutable bit set, nobody gets to write it.
    if mode & VWRITE != 0 && d.i_e2fs_flags() & EXT2_IMMUTABLE != 0 {
        return Err(Errno::EPERM);
    }

    vaccess(
        vp.vtype(),
        Mode::from(d.i_e2fs_mode()),
        d.i_e2fs_uid.get(),
        d.i_e2fs_gid.get(),
        mode,
        ucred(cred),
    )
}

/// `ext2fs_getattr` (`vop_getattr`): copy from inode table.
pub fn ext2fs_getattr<G: ProcGlue>(vp: &'static Vnode<G>, vap: &mut Vattr) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let d = ip.i_e2din();

    ext2fs_itimes(&ip);
    // Copy from inode table
    vap.va_fsid = i64::from(ip.i_dev);
    vap.va_fileid = u64::from(ip.i_number);
    vap.va_mode = Mode::from(d.i_e2fs_mode()) & ALLPERMS;
    vap.va_nlink = Nlink::from(d.i_e2fs_nlink());
    vap.va_uid = d.i_e2fs_uid.get();
    vap.va_gid = d.i_e2fs_gid.get();
    vap.va_rdev = u32::from_le(d.e2di_rdev()) as Dev;
    vap.va_size = ext2fs_size(&ip);
    vap.va_atime = Timespec::new(i64::from(d.i_e2fs_atime()), 0);
    vap.va_mtime = Timespec::new(i64::from(d.i_e2fs_mtime()), 0);
    vap.va_ctime = Timespec::new(i64::from(d.i_e2fs_ctime()), 0);
    vap.va_flags = if d.i_e2fs_flags() & EXT2_APPEND != 0 {
        u64::from(SF_APPEND)
    } else {
        0
    };
    if d.i_e2fs_flags() & EXT2_IMMUTABLE != 0 {
        vap.va_flags |= u64::from(SF_IMMUTABLE);
    }
    vap.va_gen = u64::from(d.i_e2fs_gen());
    // this doesn't belong here
    vap.va_blocksize = match vp.vtype() {
        VBLK => PAGE_SIZE as i64, // BLKDEV_IOSIZE
        VCHR => crate::vm::pager::MAXBSIZE as i64,
        _ => i64::from(vmount(vp).iosize()),
    };
    vap.va_bytes = u64::from(d.i_e2fs_nblock()) << crate::fs::bio::DEV_BSHIFT;
    vap.va_type = vp.vtype();
    vap.va_filerev = ip.i_modrev.get();
    Ok(())
}

/// `ext2fs_setattr` (`vop_setattr`): set attribute vnode op. called from several syscalls.
pub fn ext2fs_setattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    ext2fs_setattr_at(vp, vap, cred, p, &SECURELEVEL)
}

/// [`ext2fs_setattr`] under the securelevel `securelevel` (`kern.securelevel`'s word, the
/// system's; a host test passes its own, D-N5.5b-7).
fn ext2fs_setattr_at<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
    securelevel: &AtomicI32,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let d = ip.i_e2din();

    // Check for unsettable attributes.
    if vap.va_type != VNON
        || vap.va_nlink != VNOVAL as Nlink
        || vap.va_fsid != i64::from(VNOVAL)
        || vap.va_fileid != VNOVAL as u64
        || vap.va_blocksize != i64::from(VNOVAL)
        || vap.va_rdev != VNOVAL as Dev
        || vap.va_bytes as i32 != VNOVAL
        || vap.va_gen != VNOVAL as u64
    {
        return Err(Errno::EINVAL);
    }
    if vap.va_flags != VNOVAL as u64 {
        if rdonly(vp) {
            return Err(Errno::EROFS);
        }
        let c = ucred(cred);
        if c.cr_uid.get() != d.i_e2fs_uid.get() {
            suser_ucred(c)?;
        }
        if c.cr_uid.get() == 0 {
            if d.i_e2fs_flags() & (EXT2_APPEND | EXT2_IMMUTABLE) != 0
                && securelevel.load(Ordering::Relaxed) > 0
            {
                return Err(Errno::EPERM);
            }
            let mut f = d.i_e2fs_flags() & !(EXT2_APPEND | EXT2_IMMUTABLE);
            // `A ? EXT2_APPEND : 0 | B ? EXT2_IMMUTABLE : 0`, as C parses it.
            f |= if vap.va_flags & u64::from(SF_APPEND) != 0 {
                EXT2_APPEND
            } else if vap.va_flags & u64::from(SF_IMMUTABLE) != 0 {
                EXT2_IMMUTABLE
            } else {
                0
            };
            d.set_i_e2fs_flags(f);
        } else {
            return Err(Errno::EPERM);
        }
        ip.set_flag(IN_CHANGE);
        if vap.va_flags & u64::from(IMMUTABLE | APPEND) != 0 {
            return Ok(());
        }
    }
    if d.i_e2fs_flags() & (EXT2_APPEND | EXT2_IMMUTABLE) != 0 {
        return Err(Errno::EPERM);
    }
    // Go through the fields and update iff not VNOVAL.
    if vap.va_uid != VNOVAL as Uid || vap.va_gid != VNOVAL as Gid {
        if rdonly(vp) {
            return Err(Errno::EROFS);
        }
        ext2fs_chown(vp, &ip, vap.va_uid, vap.va_gid, cred)?;
    }
    if vap.va_size != VNOVAL as u64 {
        // Disallow write attempts on read-only file systems; unless the file is a socket,
        // fifo, or a block or character device resident on the file system.
        match vp.vtype() {
            VDIR => return Err(Errno::EISDIR),
            VLNK | VREG if rdonly(vp) => return Err(Errno::EROFS),
            _ => {}
        }
        ext2fs_truncate(&ip, vap.va_size as Off, 0, cred)?;
    }
    if vap.va_vaflags & VA_UTIMES_CHANGE != 0
        || vap.va_atime.tv_nsec != i64::from(VNOVAL)
        || vap.va_mtime.tv_nsec != i64::from(VNOVAL)
    {
        if rdonly(vp) {
            return Err(Errno::EROFS);
        }
        let c = ucred(cred);
        if c.cr_uid.get() != d.i_e2fs_uid.get()
            && let Err(e) = suser_ucred(c)
        {
            if vap.va_vaflags & VA_UTIMES_NULL == 0 {
                return Err(e);
            }
            vop_access(vp, VWRITE, cred, p)?;
        }
        if vap.va_mtime.tv_nsec != i64::from(VNOVAL) {
            ip.set_flag(IN_CHANGE | IN_UPDATE);
        } else if vap.va_vaflags & VA_UTIMES_CHANGE != 0 {
            ip.set_flag(IN_CHANGE);
        }
        if vap.va_atime.tv_nsec != i64::from(VNOVAL)
            && (vmount(vp).flag() & MNT_NOATIME == 0
                || ip.i_flag.get() & (IN_CHANGE | IN_UPDATE) != 0)
        {
            ip.set_flag(IN_ACCESS);
        }
        ext2fs_itimes(&ip);
        if vap.va_mtime.tv_nsec != i64::from(VNOVAL) {
            d.set_i_e2fs_mtime(vap.va_mtime.tv_sec as u32);
        }
        if vap.va_atime.tv_nsec != i64::from(VNOVAL) {
            d.set_i_e2fs_atime(vap.va_atime.tv_sec as u32);
        }
        ext2fs_update(&ip, 1)?;
    }
    if vap.va_mode != VNOVAL as Mode {
        if rdonly(vp) {
            return Err(Errno::EROFS);
        }
        return ext2fs_chmod(vp, &ip, vap.va_mode, cred);
    }
    Ok(())
}

/// `ext2fs_chmod`: change the mode on a file. Inode must be locked before calling (`ip` is
/// `vp`'s, the caller's handle).
fn ext2fs_chmod<G: ProcGlue>(
    vp: &'static Vnode<G>,
    ip: &Inode<G>,
    mode: Mode,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let c = ucred(cred);
    let d = ip.i_e2din();

    if c.cr_uid.get() != d.i_e2fs_uid.get() {
        suser_ucred(c)?;
    }
    if c.cr_uid.get() != 0 {
        if vp.vtype() != VDIR && mode & S_ISTXT != 0 {
            return Err(Errno::EFTYPE);
        }
        if !groupmember(d.i_e2fs_gid.get(), c) && mode & ISGID != 0 {
            return Err(Errno::EPERM);
        }
    }
    let m = (e2mode(ip) & !ALLPERMS) | (mode & ALLPERMS);
    d.set_i_e2fs_mode(m as u16);
    ip.set_flag(IN_CHANGE);
    if vp.flag() & VTEXT != 0 && e2mode(ip) & S_ISTXT == 0 {
        let _ = uvm_vnp_uncache(vp);
    }
    Ok(())
}

/// `ext2fs_chown`: perform chown operation on inode ip; inode must be locked prior to call
/// (`ip` is `vp`'s, the caller's handle).
fn ext2fs_chown<G: ProcGlue>(
    _vp: &'static Vnode<G>,
    ip: &Inode<G>,
    uid: Uid,
    gid: Gid,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let c = ucred(cred);
    let d = ip.i_e2din();

    let uid = if uid == VNOVAL as Uid {
        d.i_e2fs_uid.get()
    } else {
        uid
    };
    let gid = if gid == VNOVAL as Gid {
        d.i_e2fs_gid.get()
    } else {
        gid
    };
    // If we don't own the file, are trying to change the owner of the file, or are not a
    // member of the target group, the caller must be superuser or the call fails.
    if c.cr_uid.get() != d.i_e2fs_uid.get()
        || uid != d.i_e2fs_uid.get()
        || (gid != d.i_e2fs_gid.get() && !groupmember(gid, c))
    {
        suser_ucred(c)?;
    }
    let ogid = d.i_e2fs_gid.get();
    let ouid = d.i_e2fs_uid.get();

    d.i_e2fs_gid.set(gid);
    d.i_e2fs_uid.set(uid);
    if ouid != uid || ogid != gid {
        ip.set_flag(IN_CHANGE);
    }
    if ouid != uid && c.cr_uid.get() != 0 {
        d.set_i_e2fs_mode((e2mode(ip) & !ISUID) as u16);
    }
    if ogid != gid && c.cr_uid.get() != 0 {
        d.set_i_e2fs_mode((e2mode(ip) & !ISGID) as u16);
    }
    Ok(())
}

/// `ext2fs_remove` (`vop_remove`): remove the entry of a file that is not a directory (the
/// kernel's `vop_remove` releases both vnodes, as OpenBSD's `VOP_REMOVE` does).
pub fn ext2fs_remove<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    if vp.vtype() == VDIR
        || ip.i_e2din().i_e2fs_flags() & (EXT2_IMMUTABLE | EXT2_APPEND) != 0
        || vtoi(dvp).i_e2din().i_e2fs_flags() & EXT2_APPEND != 0
    {
        return Err(Errno::EPERM);
    }
    let error = ext2fs_dirremove(dvp, cnp);
    if error.is_ok() {
        nlink_add(&ip, -1);
        ip.set_flag(IN_CHANGE);
    }
    error
}

/// `ext2fs_link` (`vop_link`): link vnode call.
pub fn ext2fs_link<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
        kpanic(format_args!("ext2fs_link: no name"));
    }
    let error: Result<(), Errno> = 'out2: {
        if !core::ptr::eq(dvp, vp)
            && let Err(e) = vn_lock(vp, LK_EXCLUSIVE)
        {
            let _ = vop_abortop(dvp, cnp);
            break 'out2 Err(e);
        }
        let ip = vtoi(vp);
        let error: Result<(), Errno> = 'out1: {
            if Nlink::from(ip.i_e2din().i_e2fs_nlink()) >= LINK_MAX {
                let _ = vop_abortop(dvp, cnp);
                break 'out1 Err(Errno::EMLINK);
            }
            if ip.i_e2din().i_e2fs_flags() & (EXT2_IMMUTABLE | EXT2_APPEND) != 0 {
                let _ = vop_abortop(dvp, cnp);
                break 'out1 Err(Errno::EPERM);
            }
            nlink_add(&ip, 1);
            ip.set_flag(IN_CHANGE);
            let mut error = ext2fs_update(&ip, 1);
            if error.is_ok() {
                error = ext2fs_direnter(&ip, dvp, cnp);
            }
            if error.is_err() {
                nlink_add(&ip, -1);
                ip.set_flag(IN_CHANGE);
            }
            pnbuf_free(cnp);
            error
        };
        // out1:
        drop(ip);
        if !core::ptr::eq(dvp, vp) {
            let _ = vop_unlock(vp);
        }
        error
    };
    // out2:
    vput(dvp);
    error
}

/// `ext2fs_rename` (`vop_rename`): rename system call.
///
/// `rename("foo", "bar");` is essentially `unlink("bar"); link("foo", "bar");
/// unlink("foo");` but "atomically". Can't do full commit without saving state in the inode
/// on disk which isn't feasible at this time. Best we can do is always guarantee the target
/// exists.
///
/// Basic algorithm is:
///
/// 1. Bump link count on source while we're linking it to the target. This also ensure the
///    inode won't be deleted out from underneath us while we work (it may be truncated by a
///    concurrent `trunc` or `open` for creation).
/// 2. Link source to destination. If destination already exists, delete it first.
/// 3. Unlink source reference to inode if still around. If a directory was moved and the
///    parent of the destination is different from the source, patch the ".." entry in the
///    directory.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its labels kept as the exits of one body"
)]
pub fn ext2fs_rename<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    fcnp: &mut Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
    tcnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let a_fvp = fvp;
    let mut tvp = tvp;
    let mut doingdirectory = false;
    let mut oldparent: Ufsino = 0;
    let mut newparent: Ufsino = 0;
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

    #[cfg(feature = "diagnostic")]
    if tcnp.cn_flags & crate::fs::namei::HASBUF == 0
        || fcnp.cn_flags & crate::fs::namei::HASBUF == 0
    {
        kpanic(format_args!("ext2fs_rename: no name"));
    }
    // Check for cross-device rename.
    let fmp = fvp.mount().map(core::ptr::from_ref);
    if fmp != tdvp.mount().map(core::ptr::from_ref)
        || tvp.is_some_and(|t| fmp != t.mount().map(core::ptr::from_ref))
    {
        abortit!(Errno::EXDEV);
    }

    // Check if just deleting a link name.
    if let Some(t) = tvp
        && (vtoi(t).i_e2din().i_e2fs_flags() & (EXT2_IMMUTABLE | EXT2_APPEND) != 0
            || vtoi(tdvp).i_e2din().i_e2fs_flags() & EXT2_APPEND != 0)
    {
        abortit!(Errno::EPERM);
    }
    if let Some(t) = tvp
        && core::ptr::eq(fvp, t)
    {
        if fvp.vtype() == VDIR {
            abortit!(Errno::EINVAL);
        }

        // Release destination completely.
        let _ = vop_abortop(tdvp, tcnp);
        vput(tdvp);
        vput(t);

        // Delete source.
        vrele(fvp);
        fcnp.cn_flags &= !MODMASK;
        fcnp.cn_flags |= LOCKPARENT | LOCKLEAF;
        if fcnp.cn_flags & SAVESTART == 0 {
            kpanic(format_args!("ext2fs_rename: lost from startdir"));
        }
        fcnp.cn_nameiop = DELETE;
        let mut nfvp = None;
        vfs_relookup(fdvp, &mut nfvp, fcnp)?; // relookup did vrele()
        vrele(fdvp);
        let Some(nfvp) = nfvp else {
            // A DELETE lookup of the last component that succeeds returns its vnode, so the
            // C's VOP_REMOVE has one (it would follow a NULL one); as ufs_rename.
            kpanic(format_args!(
                "ext2fs_rename: relookup of the source found no vnode"
            ));
        };
        return vop_remove(fdvp, nfvp, fcnp);
    }
    if let Err(e) = vn_lock(fvp, LK_EXCLUSIVE) {
        abortit!(e);
    }
    let mut dp = vtoi(fdvp);
    let ip = vtoi(fvp);
    if Nlink::from(ip.i_e2din().i_e2fs_nlink()) >= LINK_MAX {
        let _ = vop_unlock(fvp);
        abortit!(Errno::EMLINK);
    }
    if ip.i_e2din().i_e2fs_flags() & (EXT2_IMMUTABLE | EXT2_APPEND) != 0
        || dp.i_e2din().i_e2fs_flags() & EXT2_APPEND != 0
    {
        let _ = vop_unlock(fvp);
        abortit!(Errno::EPERM);
    }
    if e2mode(&ip) & IFMT == IFDIR {
        let mut error = vop_access(fvp, VWRITE, tcnp.cn_cred, tcnp.proc());
        if error.is_ok()
            && let Some(t) = tvp
        {
            error = vop_access(t, VWRITE, tcnp.cn_cred, tcnp.proc());
        }
        if error.is_err() {
            let _ = vop_unlock(fvp);
            abortit!(Errno::EACCES);
        }
        // Avoid ".", "..", and aliases of "." for obvious reasons.
        if (fcnp.namelen() == 1 && fcnp.name() == b".")
            || InodeRef::<G>::ptr_eq(&dp, &ip)
            || fcnp.cn_flags & ISDOTDOT != 0
            || tcnp.cn_flags & ISDOTDOT != 0
            || ip.i_flag.get() & IN_RENAME != 0
        {
            let _ = vop_unlock(fvp);
            abortit!(Errno::EINVAL);
        }
        ip.set_flag(IN_RENAME);
        oldparent = dp.i_number;
        doingdirectory = true;
    }
    vrele(fdvp);

    // When the target exists, both the directory and target vnodes are returned locked.
    dp = vtoi(tdvp);
    let mut xp: Option<InodeRef<G>> = tvp.map(vtoi);
    let mut error: Result<(), Errno> = Ok(());

    let exit: Result<(), RenameExit> = 'body: {
        // 1) Bump link count while we're moving stuff around. If we crash somewhere before
        //    completing our work, the link count may be wrong, but correctable.
        nlink_add(&ip, 1);
        ip.set_flag(IN_CHANGE);
        if let Err(e) = ext2fs_update(&ip, 1) {
            let _ = vop_unlock(fvp);
            break 'body Err(RenameExit::Bad(e));
        }

        // If ".." must be changed (ie the directory gets a new parent) then the source
        // directory must not be in the directory hierarchy above the target, as this would
        // orphan everything below the source directory. Also the user must have write
        // permission in the source so as to be able to change "..". We must repeat the call
        // to namei, as the parent directory is unlocked by the call to checkpath().
        let access = vop_access(fvp, VWRITE, tcnp.cn_cred, tcnp.proc());
        let _ = vop_unlock(fvp);
        if oldparent != dp.i_number {
            newparent = dp.i_number;
        }
        if doingdirectory && newparent != 0 {
            if let Err(e) = access {
                // write access check above
                break 'body Err(RenameExit::Bad(e));
            }
            if xp.is_some()
                && let Some(t) = tvp
            {
                vput(t);
            }
            // Compensate for the reference ext2fs_checkpath() loses (the module's
            // deviations).
            vref(tdvp);
            if let Err(e) = ext2fs_checkpath(&ip, &dp, tcnp.cn_cred) {
                vrele(tdvp);
                break 'body Err(RenameExit::Out(e));
            }
            if tcnp.cn_flags & SAVESTART == 0 {
                kpanic(format_args!("ext2fs_rename: lost to startdir"));
            }
            let mut ntvp = None;
            if let Err(e) = vfs_relookup(tdvp, &mut ntvp, tcnp) {
                break 'body Err(RenameExit::Out(e));
            }
            tvp = ntvp;
            vrele(tdvp); // relookup() acquired a reference
            dp = vtoi(tdvp);
            xp = tvp.map(vtoi);
        }
        // 2) If target doesn't exist, link the target to the source and unlink the source.
        //    Otherwise, rewrite the target directory entry to reference the source inode and
        //    expunge the original entry's existence.
        match xp.as_deref() {
            None => {
                if dp.i_dev != ip.i_dev {
                    kpanic(format_args!("rename: EXDEV"));
                }
                // Account for ".." in new directory. When source and destination have the
                // same parent we don't fool with the link count.
                if doingdirectory && newparent != 0 {
                    if Nlink::from(dp.i_e2din().i_e2fs_nlink()) >= LINK_MAX {
                        break 'body Err(RenameExit::Bad(Errno::EMLINK));
                    }
                    nlink_add(&dp, 1);
                    dp.set_flag(IN_CHANGE);
                    if let Err(e) = ext2fs_update(&dp, 1) {
                        break 'body Err(RenameExit::Bad(e));
                    }
                }
                if let Err(e) = ext2fs_direnter(&ip, tdvp, tcnp) {
                    if doingdirectory && newparent != 0 {
                        nlink_add(&dp, -1);
                        dp.set_flag(IN_CHANGE);
                        let _ = ext2fs_update(&dp, 1);
                    }
                    break 'body Err(RenameExit::Bad(e));
                }
                vput(tdvp);
            }
            Some(x) => {
                if x.i_dev != dp.i_dev || x.i_dev != ip.i_dev {
                    kpanic(format_args!("rename: EXDEV"));
                }
                // Short circuit rename(foo, foo).
                if x.i_number == ip.i_number {
                    kpanic(format_args!("rename: same file"));
                }
                // If the parent directory is "sticky", then the user must own the parent
                // directory, or the destination of the rename, otherwise the destination may
                // not be changed (except by root). This implements append-only directories.
                let tuid = ucred(tcnp.cn_cred).cr_uid.get();
                if e2mode(&dp) & S_ISTXT != 0
                    && tuid != 0
                    && tuid != dp.i_e2din().i_e2fs_uid.get()
                    && x.i_e2din().i_e2fs_uid.get() != tuid
                {
                    break 'body Err(RenameExit::Bad(Errno::EPERM));
                }
                // Target must be empty if a directory and have no links to it. Also, ensure
                // source and target are compatible (both directories, or both not
                // directories).
                if e2mode(x) & IFMT == IFDIR {
                    if !ext2fs_dirempty(x, dp.i_number, tcnp.cn_cred)
                        || x.i_e2din().i_e2fs_nlink() > 2
                    {
                        break 'body Err(RenameExit::Bad(Errno::ENOTEMPTY));
                    }
                    if !doingdirectory {
                        break 'body Err(RenameExit::Bad(Errno::ENOTDIR));
                    }
                    cache_purge(tdvp);
                } else if doingdirectory {
                    break 'body Err(RenameExit::Bad(Errno::EISDIR));
                }
                if let Err(e) = ext2fs_dirrewrite(&dp, &ip, tcnp) {
                    break 'body Err(RenameExit::Bad(e));
                }
                // If the target directory is in the same directory as the source directory,
                // decrement the link count on the parent of the target directory.
                if doingdirectory && newparent == 0 {
                    nlink_add(&dp, -1);
                    dp.set_flag(IN_CHANGE);
                }
                vput(tdvp);
                // Adjust the link count of the target to reflect the dirrewrite above. If
                // this is a directory it is empty and there are no links to it, so we can
                // squash the inode and any space associated with it. We disallowed renaming
                // over top of a directory with links to it above, as the remaining link
                // would point to a directory without "." or ".." entries.
                nlink_add(x, -1);
                if doingdirectory {
                    nlink_add(x, -1);
                    if x.i_e2din().i_e2fs_nlink() != 0 {
                        kpanic(format_args!("rename: linked directory"));
                    }
                    error = ext2fs_truncate(x, 0, IO_SYNC, tcnp.cn_cred);
                }
                x.set_flag(IN_CHANGE);
                if let Some(t) = tvp {
                    vput(t);
                }
                // xp = NULL: step 3 looks the source up again.
            }
        }
        Ok(())
    };

    match exit {
        Ok(()) => {}
        Err(RenameExit::Bad(e)) => {
            if let Some(x) = xp.as_deref() {
                vput(x.itov());
            }
            vput(dp.itov());
            return rename_out(e, fvp, &ip, doingdirectory);
        }
        Err(RenameExit::Out(e)) => return rename_out(e, fvp, &ip, doingdirectory),
    }
    drop(xp);
    drop(dp);

    // 3) Unlink the source.
    fcnp.cn_flags &= !MODMASK;
    fcnp.cn_flags |= LOCKPARENT | LOCKLEAF;
    if fcnp.cn_flags & SAVESTART == 0 {
        kpanic(format_args!("ext2fs_rename: lost from startdir"));
    }
    let mut nfvp = None;
    // Held across the lookup, which releases it when it fails (the module's deviations).
    vref(fdvp);
    if vfs_relookup(fdvp, &mut nfvp, fcnp).is_ok() {
        vrele(fdvp);
    }
    let Some(nfvp) = nfvp else {
        // From name has disappeared.
        if doingdirectory {
            kpanic(format_args!("ext2fs_rename: lost dir entry"));
        }
        vrele(a_fvp);
        return Ok(());
    };
    let x = vtoi(nfvp);
    let dp = vtoi(fdvp);

    // Ensure that the directory entry still exists and has not changed while the new name
    // has been entered. If the source is a file then the entry may have been unlinked or
    // renamed. In either case there is no further work to be done. If the source is a
    // directory then it cannot have been rmdir'ed; its link count of three would cause a
    // rmdir to fail with ENOTEMPTY. The IRENAME flag ensures that it cannot be moved by
    // another rename.
    if InodeRef::<G>::ptr_eq(&x, &ip) {
        // If the source is a directory with a new parent, the link count of the old parent
        // directory must be decremented and ".." set to point to the new parent.
        if doingdirectory && newparent != 0 {
            nlink_add(&dp, -1);
            dp.set_flag(IN_CHANGE);
            let mut dirbuf = [0u8; Ext2fsDirtemplate::SIZE];
            let read = G::with_curproc(|p| {
                vn_rdwr(
                    UioRw::UIO_READ,
                    nfvp,
                    RdwrBuf::Kernel(&mut dirbuf),
                    0,
                    IO_NODELOCKED,
                    tcnp.cn_cred,
                    None,
                    p,
                )
            });
            if read.is_ok() {
                let mut t = Ext2fsDirtemplate::from_le_bytes(&dirbuf);
                if t.dotdot_namlen != 2 || t.dotdot_name[0] != b'.' || t.dotdot_name[1] != b'.' {
                    ufs_dirbad(&x, 12, "ext2fs_rename: mangled dir");
                } else {
                    t.dotdot_ino = newparent;
                    let out = t.to_le_bytes();
                    let _ = G::with_curproc(|p| {
                        vn_rdwr(
                            UioRw::UIO_WRITE,
                            nfvp,
                            RdwrBuf::KernelRead(&out),
                            0,
                            IO_NODELOCKED | IO_SYNC,
                            tcnp.cn_cred,
                            None,
                            p,
                        )
                    });
                    cache_purge(fdvp);
                }
            }
        }
        error = ext2fs_dirremove(fdvp, fcnp);
        if error.is_ok() {
            nlink_add(&x, -1);
            x.set_flag(IN_CHANGE);
        }
        x.clr_flag(IN_RENAME);
    } else if doingdirectory {
        kpanic(format_args!("ext2fs_rename: lost dir entry"));
    }
    drop(dp);
    drop(x);
    vput(fdvp);
    vput(nfvp);
    vrele(a_fvp);
    error
}

/// `out:` of `ext2fs_rename`: undo the link count bump on the source (`fdvp` was released
/// before the body).
fn rename_out<G: ProcGlue>(
    error: Errno,
    fvp: &'static Vnode<G>,
    ip: &Inode<G>,
    doingdirectory: bool,
) -> Result<(), Errno> {
    if doingdirectory {
        ip.clr_flag(IN_RENAME);
    }
    if vn_lock(fvp, LK_EXCLUSIVE).is_ok() {
        nlink_add(ip, -1);
        ip.set_flag(IN_CHANGE);
        vput(fvp);
    } else {
        vrele(fvp);
    }
    Err(error)
}

/// `ext2fs_mkdir` (`vop_mkdir`): mkdir system call.
pub fn ext2fs_mkdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
        kpanic(format_args!("ext2fs_mkdir: no name"));
    }
    let dp = vtoi(dvp);
    let error: Result<(), Errno> = 'out: {
        if Nlink::from(dp.i_e2din().i_e2fs_nlink()) >= LINK_MAX {
            break 'out Err(Errno::EMLINK);
        }
        let dmode = (vap.va_mode & ACCESSPERMS) | IFDIR;
        // Must simulate part of ext2fs_makeinode here to acquire the inode, but not have it
        // entered in the parent directory. The entry is made later after writing "." and ".."
        // entries.
        let tvp = match ext2fs_inode_alloc(&dp, dmode, cnp.cn_cred) {
            Ok(tvp) => tvp,
            Err(e) => break 'out Err(e),
        };
        let ip = vtoi(tvp);
        let d = ip.i_e2din();
        d.i_e2fs_uid.set(ucred(cnp.cn_cred).cr_uid.get());
        d.i_e2fs_gid.set(dp.i_e2din().i_e2fs_gid.get());
        ip.set_flag(IN_ACCESS | IN_CHANGE | IN_UPDATE);
        d.set_i_e2fs_mode(dmode as u16);
        tvp.set_vtype(VDIR); // Rest init'd in getnewvnode().
        d.set_i_e2fs_nlink(2);
        // The C stores this result and overwrites it at once with the parent's.
        let _ = ext2fs_update(&ip, 1);

        let error: Result<(), Errno> = 'bad: {
            // Bump link count in parent directory to reflect work done below. Should be done
            // before reference is created so reparation is possible if we crash.
            nlink_add(&dp, 1);
            dp.set_flag(IN_CHANGE);
            if let Err(e) = ext2fs_update(&dp, 1) {
                break 'bad Err(e);
            }

            // Initialize directory with "." and ".." from static template.
            let ftype = has_ftype(ip.i_e2fs());
            let bsize = dp.i_e2fs().e2fs_bsize.get();
            let dirtemplate = Ext2fsDirtemplate {
                dot_ino: ip.i_number,
                dot_reclen: 12,
                dot_namlen: 1,
                dot_type: if ftype { EXT2_FT_DIR } else { 0 },
                dot_name: *b".\0\0\0",
                dotdot_ino: dp.i_number,
                dotdot_reclen: (bsize - 12) as i16,
                dotdot_namlen: 2,
                dotdot_type: if ftype { EXT2_FT_DIR } else { 0 },
                dotdot_name: *b"..\0\0",
            };
            let tb = dirtemplate.to_le_bytes();
            if let Err(e) = G::with_curproc(|p| {
                vn_rdwr(
                    UioRw::UIO_WRITE,
                    tvp,
                    RdwrBuf::KernelRead(&tb),
                    0,
                    IO_NODELOCKED | IO_SYNC,
                    cnp.cn_cred,
                    None,
                    p,
                )
            }) {
                nlink_add(&dp, -1);
                dp.set_flag(IN_CHANGE);
                break 'bad Err(e);
            }
            if i64::from(bsize) > i64::from(vmount(dvp).bsize()) {
                // XXX should grow with balloc()
                kpanic(format_args!("ext2fs_mkdir: blksize"));
            }
            if let Err(e) = ext2fs_setsize(&ip, bsize as u64) {
                nlink_add(&dp, -1);
                dp.set_flag(IN_CHANGE);
                break 'bad Err(e);
            }
            ip.set_flag(IN_CHANGE);

            // Directory set up, now install its entry in the parent directory.
            let error = ext2fs_direnter(&ip, dvp, cnp);
            if error.is_err() {
                nlink_add(&dp, -1);
                dp.set_flag(IN_CHANGE);
            }
            error
        };
        // bad:
        // No need to do an explicit VOP_TRUNCATE here, vrele will do this for us because we
        // set the link count to 0.
        if error.is_err() {
            d.set_i_e2fs_nlink(0);
            ip.set_flag(IN_CHANGE);
            drop(ip);
            vput(tvp);
        } else {
            *vpp = Some(tvp);
        }
        error
    };
    // out:
    pnbuf_free(cnp);
    drop(dp);
    vput(dvp);
    error
}

/// `ext2fs_rmdir` (`vop_rmdir`): rmdir system call.
pub fn ext2fs_rmdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let mut a_dvp = Some(dvp);
    let ip = vtoi(vp);
    // Verify the directory is empty (and valid). (Rmdir ".." won't be valid since ".." will
    // contain a reference to the current directory and thus be non-empty.)
    let error: Result<(), Errno> = 'out: {
        let dp = vtoi(dvp);
        if ip.i_e2din().i_e2fs_nlink() != 2 || !ext2fs_dirempty(&ip, dp.i_number, cnp.cn_cred) {
            break 'out Err(Errno::ENOTEMPTY);
        }
        if dp.i_e2din().i_e2fs_flags() & EXT2_APPEND != 0
            || ip.i_e2din().i_e2fs_flags() & (EXT2_IMMUTABLE | EXT2_APPEND) != 0
        {
            break 'out Err(Errno::EPERM);
        }
        // Delete reference to directory before purging inode. If we crash in between, the
        // directory will be reattached to lost+found,
        if let Err(e) = ext2fs_dirremove(dvp, cnp) {
            break 'out Err(e);
        }
        nlink_add(&dp, -1);
        dp.set_flag(IN_CHANGE);
        cache_purge(dvp);
        drop(dp);
        vput(dvp);
        a_dvp = None;
        // Truncate inode. The only stuff left in the directory is "." and "..". The "."
        // reference is inconsequential since we're quashing it. The ".." reference has
        // already been adjusted above. We've removed the "." reference and the reference in
        // the parent directory, but there may be other hard links so decrement by 2 and
        // worry about them later.
        nlink_add(&ip, -2);
        let error = ext2fs_truncate(&ip, 0, IO_SYNC, cnp.cn_cred);
        cache_purge(ip.itov());
        error
    };
    // out:
    drop(ip);
    if let Some(dvp) = a_dvp {
        vput(dvp);
    }
    vput(vp);
    error
}

/// `ext2fs_symlink` (`vop_symlink`): symlink -- make a symbolic link.
pub fn ext2fs_symlink<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
    target: &[u8],
) -> Result<(), Errno> {
    let error = ext2fs_makeinode(IFLNK | vap.va_mode, dvp, vpp, cnp);
    vput(dvp);
    error?;
    let Some(vp) = *vpp else {
        kpanic(format_args!("ext2fs_symlink: no vnode"));
    };
    let len = target.len();
    let error = if len < EXT2_MAXSYMLINKLEN {
        let ip = vtoi(vp);
        ip.i_e2din().set_e2di_shortlink(0, target);
        ext2fs_setsize(&ip, len as u64).map(|()| ip.set_flag(IN_CHANGE | IN_UPDATE))
    } else {
        G::with_curproc(|p| {
            vn_rdwr(
                UioRw::UIO_WRITE,
                vp,
                RdwrBuf::KernelRead(target),
                0,
                IO_NODELOCKED,
                cnp.cn_cred,
                None,
                p,
            )
        })
    };
    // bad:
    vput(vp);
    error
}

/// `ext2fs_readlink` (`vop_readlink`): return target name of a symbolic link.
pub fn ext2fs_readlink<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);

    let isize = ext2fs_size(&ip);
    if isize < EXT2_MAXSYMLINKLEN as u64 {
        let mut link = ip.i_e2din().e2di_shortlink();
        return <G::FileGlue as FileGlue>::uiomove(&mut link[..isize as usize], uio);
    }
    drop(ip);
    vop_read(vp, uio, 0, cred)
}

/// `ext2fs_pathconf` (`vop_pathconf`): return POSIX pathconf information applicable to ext2
/// filesystems.
pub fn ext2fs_pathconf<G: ProcGlue>(
    vp: &'static Vnode<G>,
    name: i32,
    retval: &mut Register,
) -> Result<(), Errno> {
    match name {
        _PC_TIMESTAMP_RESOLUTION => {
            *retval = 1_000_000_000; // 1 billion nanoseconds
            Ok(())
        }
        _ => ufs_pathconf(vp, name, retval),
    }
}

/// `ext2fs_advlock` (`vop_advlock`): advisory record locking support.
pub fn ext2fs_advlock<G: ProcGlue>(
    vp: &'static Vnode<G>,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    let ip = vtoi(vp);

    lf_advlock::<G>(&ip.i_lockf, ext2fs_size(&ip) as Off, id, op, fl, flags)
}

/// `ext2fs_makeinode`: allocate a new inode of type `mode` and enter it in the directory
/// `dvp` under the name in `cnp`; `*vpp` is its vnode, referenced and locked.
pub fn ext2fs_makeinode<G: ProcGlue>(
    mut mode: Mode,
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let pdir = vtoi(dvp);
    #[cfg(feature = "diagnostic")]
    if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
        kpanic(format_args!("ext2fs_makeinode: no name"));
    }
    *vpp = None;
    if mode & IFMT == 0 {
        mode |= IFREG;
    }

    let tvp = match ext2fs_inode_alloc(&pdir, mode, cnp.cn_cred) {
        Ok(tvp) => tvp,
        Err(e) => {
            pnbuf_free(cnp);
            return Err(e);
        }
    };
    let ip = vtoi(tvp);
    let d = ip.i_e2din();
    let c = ucred(cnp.cn_cred);
    d.i_e2fs_gid.set(pdir.i_e2din().i_e2fs_gid.get());
    d.i_e2fs_uid.set(c.cr_uid.get());
    ip.set_flag(IN_ACCESS | IN_CHANGE | IN_UPDATE);
    d.set_i_e2fs_mode(mode as u16);
    tvp.set_vtype(iftovt(mode)); // Rest init'd in getnewvnode().
    d.set_i_e2fs_nlink(1);
    if e2mode(&ip) & ISGID != 0 && !groupmember(d.i_e2fs_gid.get(), c) && suser_ucred(c).is_err() {
        d.set_i_e2fs_mode((e2mode(&ip) & !ISGID) as u16);
    }

    let error: Result<(), Errno> = 'bad: {
        // Make sure inode goes to disk before directory entry.
        if let Err(e) = ext2fs_update(&ip, 1) {
            break 'bad Err(e);
        }
        if let Err(e) = ext2fs_direnter(&ip, dvp, cnp) {
            break 'bad Err(e);
        }
        if cnp.cn_flags & SAVESTART == 0 {
            pnbuf_free(cnp);
        }
        *vpp = Some(tvp);
        return Ok(());
    };

    // bad:
    // Write error occurred trying to update the inode or the directory so must deallocate
    // the inode.
    pnbuf_free(cnp);
    d.set_i_e2fs_nlink(0);
    ip.set_flag(IN_CHANGE);
    tvp.set_vtype(VNON);
    drop(ip);
    vput(tvp);
    error
}

/// `ext2fs_fsync` (`vop_fsync`): synch an open file.
pub fn ext2fs_fsync<G: ProcGlue>(vp: &'static Vnode<G>, waitfor: i32) -> Result<(), Errno> {
    vflushbuf(vp, waitfor == MNT_WAIT);
    ext2fs_update(&vtoi(vp), i32::from(waitfor == MNT_WAIT))
}

/// `ext2fs_reclaim` (`vop_reclaim`): reclaim an inode so that it can be used for other
/// purposes. The vnode's handle to the inode goes (`vp->v_data = NULL`); with the last
/// handle the dinode and the inode go back to their pools (`ext2dinopl`, `ext2inopl`).
pub fn ext2fs_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    crate::fs::ufs::ufs::ufs_inode::pushing_active(vp, "ext2fs_reclaim: pushing active");

    // Remove the inode from its hash chain.
    let ip = vtoi(vp);
    ufs_ihashrem(&ip);

    // Purge old data structures associated with the inode.
    cache_purge(vp);
    vrele(ip.i_devvp());

    drop(ip);
    drop(vp.take_node());
    Ok(())
}

/// The slots `ext2fs_vops` and `ext2fs_specvops` fill alike: ext2fs's attributes, fsync,
/// inactive and reclaim, the UFS layer's lock, unlock, print and islocked, the generic
/// bwrite.
macro_rules! ext2fs_common_slots {
    () => {
        fn access(
            &self,
            vp: &'static Vnode<G>,
            mode: i32,
            cred: Cred<'_>,
            _p: &Thread<G>,
        ) -> Result<(), Errno> {
            ext2fs_access(vp, mode, cred)
        }
        fn getattr(
            &self,
            vp: &'static Vnode<G>,
            vap: &mut Vattr,
            _cred: Cred<'_>,
            _p: &Thread<G>,
        ) -> Result<(), Errno> {
            ext2fs_getattr(vp, vap)
        }
        fn setattr(
            &self,
            vp: &'static Vnode<G>,
            vap: &mut Vattr,
            cred: Cred<'_>,
            p: &Thread<G>,
        ) -> Result<(), Errno> {
            ext2fs_setattr(vp, vap, cred, p)
        }
        fn fsync(
            &self,
            vp: &'static Vnode<G>,
            _cred: Cred<'_>,
            waitfor: i32,
            _p: &Thread<G>,
        ) -> Result<(), Errno> {
            ext2fs_fsync(vp, waitfor)
        }
        fn inactive(&self, vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
            ext2fs_inactive(vp, p)
        }
        fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
            ext2fs_reclaim(vp)
        }
        fn lock(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
            ufs_lock(vp, flags)
        }
        fn unlock(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
            ufs_unlock(vp)
        }
        fn islocked(&self, vp: &'static Vnode<G>) -> i32 {
            ufs_islocked(vp)
        }
        fn print(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
            ufs_print(vp)
        }
        fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
            default::vop_generic_bwrite(bp)
        }
    };
}

/// `ext2fs_vops`: each slot the C's; `vop_revoke` is NULL there, the trait's default here.
impl<G: ProcGlue> Vnops<G> for Ext2fsVnops<G> {
    ext2fs_common_slots!();

    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        ext2fs_lookup(dvp, vpp, cnp)
    }
    fn create(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        ext2fs_create(dvp, vpp, cnp, vap)
    }
    fn mknod(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        ext2fs_mknod(dvp, vpp, cnp, vap)
    }
    fn open(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        ext2fs_open(vp, mode)
    }
    fn close(
        &self,
        vp: &'static Vnode<G>,
        _fflag: i32,
        _cred: Cred<'_>,
        _p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        ufs_close(vp)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        ext2fs_read(vp, uio, ioflag, cred)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        ext2fs_write(vp, uio, ioflag, cred)
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
        ufs_ioctl()
    }
    fn kqfilter(&self, vp: &'static Vnode<G>, _fflag: i32, kn: &Knote<G>) -> Result<(), Errno> {
        ufs_kqfilter(vp, kn)
    }
    fn remove(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        ext2fs_remove(dvp, vp, cnp)
    }
    fn link(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        ext2fs_link(dvp, vp, cnp)
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
        ext2fs_rename(fdvp, fvp, fcnp, tdvp, tvp, tcnp)
    }
    fn mkdir(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        ext2fs_mkdir(dvp, vpp, cnp, vap)
    }
    fn rmdir(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        ext2fs_rmdir(dvp, vp, cnp)
    }
    fn symlink(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
        target: &[u8],
    ) -> Result<(), Errno> {
        ext2fs_symlink(dvp, vpp, cnp, vap, target)
    }
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        ext2fs_readdir(vp, uio, cred, eofflag)
    }
    fn readlink(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        ext2fs_readlink(vp, uio, cred)
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
        ext2fs_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        ufs_strategy(vp, bp)
    }
    fn pathconf(
        &self,
        vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        ext2fs_pathconf(vp, name, retval)
    }
    fn advlock(
        &self,
        vp: &'static Vnode<G>,
        id: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        ext2fs_advlock(vp, id, op, fl, flags)
    }
}

/// `ext2fs_specvops`: each slot the C's.
impl<G: ProcGlue> Vnops<G> for Ext2fsSpecVnops<G> {
    ext2fs_common_slots!();

    fn close(
        &self,
        vp: &'static Vnode<G>,
        fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        ufsspec_close(vp, fflag, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        ufsspec_read(vp, uio, ioflag)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        ufsspec_write(vp, uio, ioflag)
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }

    // XXX: Keep in sync with spec_vops (one macro: the same slots as SpecVops's).
    crate::fs::spec::spec_shared_slots!();
}

// `ext2fs_fifovops` and `ext2fsfifo_reclaim` (`#ifdef FIFO`) wait for `miscfs/fifofs` (the
// module's deviations).
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for what the fixture's vnodes reach without the real mount (N5.6c3 calls
    // every slot over `Ext2fs::<TestGlue>::VFS`): the two tables' own addresses in the VFS's
    // registry and `ext2fs_vops`'s NULL revoke, the attributes (`ext2fs_getattr`'s
    // translation, `ext2fs_setattr`'s flags under an injected securelevel), the append-only
    // and immutable checks of `ext2fs_open` and `ext2fs_access`, `ext2fs_pathconf`, and the
    // short symbolic link of `ext2fs_readlink`.

    use core::sync::atomic::AtomicI32;
    use std::{assert, assert_eq, assert_ne};

    use hal::sys::mount::MNT_RDONLY;

    use super::super::testext2fs::newfs::{self, HELLO};
    use super::super::testext2fs::{attach, mount, new_file, root, vget};
    use super::*;
    use crate::fs::subr::{vattr_null, vfs, vops_id};
    use crate::fs::testfs::TestGlue;
    use crate::uio::Iovec;

    /// A `struct vattr` with every member `VNOVAL` (`vattr_null`).
    fn novals() -> Vattr {
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va
    }

    #[test]
    fn the_two_tables_have_addresses_of_their_own() {
        let (_g, _p) = attach(newfs::image(false));
        let t = &vfs::<TestGlue>().ufs;
        let file = vops_id::<TestGlue>(&t.ext2fs_vops);
        let spec = vops_id::<TestGlue>(&t.ext2fs_specvops);
        assert_ne!(file, spec);
        assert_ne!(file, vops_id::<TestGlue>(&t.ffs_vops));
        assert_ne!(spec, vops_id::<TestGlue>(&t.ffs_specvops));
        // ext2fs_vops's vop_revoke is NULL: the trait's default.
        let mp = mount(MNT_RDONLY);
        let vp = root(mp);
        assert_eq!(t.ext2fs_vops.revoke(vp, 0), Err(Errno::EOPNOTSUPP));
        vput(vp);
    }

    #[test]
    fn getattr_translates_the_dinode() {
        let (_g, _p) = attach(newfs::image(false));
        let mp = mount(MNT_RDONLY);
        let vp = vget(mp, 12).unwrap();
        let mut va = novals();
        ext2fs_getattr(vp, &mut va).unwrap();
        assert_eq!((va.va_fileid, va.va_type, va.va_mode), (12, VREG, 0o644));
        // The 32-bit owner the split dinode fields make (the image's 1000 | 1 << 16).
        assert_eq!((va.va_nlink, va.va_uid, va.va_gid), (1, 66536, 20));
        assert_eq!(va.va_size, newfs::HELLO_DATA.len() as u64);
        // e2di_rdev overlays the first block pointer, whatever the type (the C's).
        assert_eq!(va.va_rdev, HELLO as Dev);
        assert_eq!((va.va_blocksize, va.va_flags), (1024, 0));
        let d = vtoi(vp);
        assert_eq!(va.va_bytes, u64::from(d.i_e2din().i_e2fs_nblock()) * 512);
        d.i_e2din().set_i_e2fs_flags(EXT2_APPEND | EXT2_IMMUTABLE);
        ext2fs_getattr(vp, &mut va).unwrap();
        assert_eq!(va.va_flags, u64::from(SF_APPEND | SF_IMMUTABLE));
        drop(d);
        vput(vp);
    }

    #[test]
    fn open_access_and_pathconf_keep_the_c_checks() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(MNT_RDONLY);
        let vp = vget(mp, 12).unwrap();
        let cred = Cred::real(p.ucred());
        let d = vtoi(vp);
        assert_eq!(ext2fs_open(vp, FWRITE), Ok(()));
        assert_eq!(ext2fs_access(vp, VWRITE, cred), Ok(()));
        d.i_e2din().set_i_e2fs_flags(EXT2_APPEND);
        assert_eq!(ext2fs_open(vp, FWRITE), Err(Errno::EPERM));
        assert_eq!(ext2fs_open(vp, FWRITE | O_APPEND), Ok(()));
        d.i_e2din().set_i_e2fs_flags(EXT2_IMMUTABLE);
        // No EROFS here (the C's ext2fs_access has none): the immutable bit's EPERM.
        assert_eq!(ext2fs_access(vp, VWRITE, cred), Err(Errno::EPERM));
        let mut r: Register = 0;
        ext2fs_pathconf(vp, _PC_TIMESTAMP_RESOLUTION, &mut r).unwrap();
        assert_eq!(r, 1_000_000_000);
        ext2fs_pathconf(vp, hal::sys::unistd::_PC_LINK_MAX, &mut r).unwrap();
        assert_eq!(r, LINK_MAX as Register);
        assert_eq!(ext2fs_pathconf(vp, 9999, &mut r), Err(Errno::EINVAL));
        drop(d);
        vput(vp);
    }

    #[test]
    fn setattr_maps_the_flags_and_keeps_the_securelevel_check() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = vget(mp, 12).unwrap();
        let cred = Cred::real(p.ucred());
        let ip = vtoi(vp);
        let level = AtomicI32::new(1);
        // SF_APPEND | SF_IMMUTABLE sets EXT2_APPEND alone: the C's precedence.
        let mut va = novals();
        va.va_flags = u64::from(SF_APPEND | SF_IMMUTABLE);
        assert_eq!(ext2fs_setattr_at(vp, &mut va, cred, p, &level), Ok(()));
        assert_eq!(ip.i_e2din().i_e2fs_flags(), EXT2_APPEND);
        // Clearing it at securelevel 1 is EPERM, at 0 it goes.
        let mut va = novals();
        va.va_flags = 0;
        assert_eq!(
            ext2fs_setattr_at(vp, &mut va, cred, p, &level),
            Err(Errno::EPERM)
        );
        level.store(0, Ordering::Relaxed);
        assert_eq!(ext2fs_setattr_at(vp, &mut va, cred, p, &level), Ok(()));
        assert_eq!(ip.i_e2din().i_e2fs_flags(), 0);
        // An unsettable attribute: EINVAL before anything.
        let mut va = novals();
        va.va_nlink = 3;
        assert_eq!(
            ext2fs_setattr_at(vp, &mut va, cred, p, &level),
            Err(Errno::EINVAL)
        );
        // A mode change: the permission bits only.
        let mut va = novals();
        va.va_mode = 0o4600;
        assert_eq!(ext2fs_setattr_at(vp, &mut va, cred, p, &level), Ok(()));
        assert_eq!(u32::from(ip.i_e2din().i_e2fs_mode()), IFREG | 0o4600);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn readlink_reads_a_short_link_from_the_dinode() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = new_file(mp, p);
        let ip = vtoi(vp);
        ip.i_e2din().set_i_e2fs_mode((IFLNK | 0o755) as u16);
        vp.set_vtype(VLNK);
        ip.i_e2din().set_e2di_shortlink(0, b"/etc/target");
        ext2fs_setsize(&ip, 11).unwrap();
        let mut buf = [0u8; 64];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        ext2fs_readlink(vp, &mut uio, Cred::real(p.ucred())).unwrap();
        assert_eq!(uio.uio_resid, 64 - 11);
        assert_eq!(&buf[..11], b"/etc/target");
        assert!(buf[11..].iter().all(|&c| c == 0));
        drop(ip);
        vput(vp);
    }
}
/* </TESTS> */
