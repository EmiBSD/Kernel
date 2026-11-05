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
                Err($e),
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

    /// The vnode and directory operations (`ext2fs_vnops.rs`, `ext2fs_lookup.rs`) over the
    /// real mount, through the system calls' own paths (`sys_open`, `sys_read`, `sys_write`,
    /// `sys_link`, `sys_unlink`, `sys_rename`, `sys_mkdir`, `sys_rmdir`, `sys_symlink`,
    /// `sys_mknod`, ...) on a fresh ext2 file system built in memory (testext2fs's
    /// `newfs::mkfs`),
    /// mounted read-write as the root (moved from `bsd`'s tests, N5.6c3). After the unmount
    /// every test runs [`mounted::fsck`] on the disk: a small `fsck_ext2fs` that checks what
    /// `e2fsck -fn` would (directory record chains, link counts, block counts in 512-byte
    /// units, duplicate blocks, the bitmaps, and the free and directory counts of the super
    /// block and the group descriptor). User memory: testfs's path slots and buffer
    /// (`testfs::up`, USER_BASE + 0xe000..0xf000), which `ext2fs_vfsops`'s tests share
    /// (both hold MEMORY: one at a time).
    mod mounted {
        use core::mem::offset_of;
        use std::collections::BTreeMap;
        use std::string::String;
        use std::sync::MutexGuard;
        use std::vec::Vec;
        use std::{assert, assert_eq, assert_ne, format, vec};

        use hal::sys::dirent::Dirent;
        use hal::sys::fcntl::{O_CREAT, O_RDONLY, O_RDWR};
        use hal::sys::mount::MNT_RDONLY;
        use hal::sys::stat::{S_IFCHR, S_IFIFO};
        use hal::sys::syslimits::LINK_MAX;
        use hal::sys::types::makedev;
        use hal::sys::unistd::{_PC_LINK_MAX, _PC_TIMESTAMP_RESOLUTION};
        use hal::user::UserPtr;

        use super::super::super::ext2fs::{E2FS_ISCLEAN, Ext2fs};
        use super::super::super::ext2fs_dinode::{EXT2_ROOTINO, Ext2fsDinode};
        use super::super::super::ext2fs_dir::{
            EXT2_FT_REG_FILE, EXT2_FT_SYMLINK, e2d_type, e2iftodt, ext2fs_dirsiz, inot2ext2dt,
        };
        use super::super::super::testext2fs::mount_root;
        use super::super::super::testext2fs::newfs::{
            B, BBITMAP, GDT, IBITMAP, ITABLE, MKFS_IPG, MKFS_NBLK, itblocks, mkfs,
        };
        use super::*;
        use crate::clock::Timespec;
        use crate::descrip::{sys_close, sys_read, sys_write};
        use crate::fs::lookup::{namei, ndinit};
        use crate::fs::mount::Mount;
        use crate::fs::namei::{FOLLOW, LOCKLEAF, LOOKUP, NOFOLLOW, NiDirp};
        use crate::fs::syscalls::{
            sys_fsync, sys_link, sys_mkdir, sys_mknod, sys_open, sys_readlink, sys_rename,
            sys_rmdir, sys_symlink, sys_unlink,
        };
        use crate::fs::testfs::{UBUF, UBUF_LEN, up};
        use crate::fs::ufs::ffs::testffs::{disk, unmount_root};
        use crate::fs::ufs::ufs::dinode::{IFDIR, IFLNK, IFMT};
        use crate::fs::ufs::ufs::inode::IN_MODIFIED;
        use crate::fs::ufs::ufs::ufsmount::vfstoufs;
        use crate::fs::vnode::{LK_RETRY, VLNK, vn_lock};
        use crate::fs::vops::{VFS_VGET, vop_getattr, vop_pathconf, vop_readdir, vop_setattr};
        use crate::uio::Uio;

        /// Blocks in the file system (one group).
        const NBLK: usize = MKFS_NBLK;
        /// Inodes in the file system (one group).
        const IPG: usize = MKFS_IPG;

        /// What `fsck` found: every allocated inode with its mode, link count, size and data,
        /// and every directory's entries.
        pub(super) struct Fs {
            inodes: BTreeMap<u32, Ino>,
            dirs: BTreeMap<u32, Vec<(Vec<u8>, u32, u8)>>,
        }

        /// An allocated inode, as `fsck` read it.
        pub(super) struct Ino {
            mode: u16,
            nlink: u16,
            uid: u32,
            gid: u32,
            size: u64,
            data: Vec<u8>,
        }

        fn le16(d: &[u8], o: usize) -> u16 {
            u16::from_le_bytes([d[o], d[o + 1]])
        }

        fn le32(d: &[u8], o: usize) -> u32 {
            u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
        }

        fn bit(d: &[u8], blk: usize, bit: usize) -> bool {
            d[blk * B + bit / 8] & (1 << (bit % 8)) != 0
        }

        /// A small `fsck_ext2fs -n` of the unmounted disk: panics on any inconsistency
        /// `e2fsck -fn` would report, and returns what it read.
        #[allow(clippy::too_many_lines)] // one pass of fsck, as the C's phases
        pub(super) fn fsck() -> Fs {
            let d = disk();
            let sb = B;
            let isz = usize::from(le16(&d, sb + offset_of!(Ext2fs, e2fs_inode_size)));
            assert_eq!(
                le16(&d, sb + offset_of!(Ext2fs, e2fs_state)),
                E2FS_ISCLEAN,
                "clean"
            );
            let first_ino = le32(&d, sb + offset_of!(Ext2fs, e2fs_first_ino));
            let itend = ITABLE + itblocks(isz);

            let mut claimed = vec![false; NBLK];
            for c in claimed.iter_mut().take(itend).skip(1) {
                *c = true;
            }
            let claim = |claimed: &mut Vec<bool>, b: u32, who: u32| {
                let b = b as usize;
                assert!(
                    b >= itend && b < NBLK,
                    "inode {who}: block {b} out of range"
                );
                assert!(!claimed[b], "inode {who}: block {b} claimed twice");
                claimed[b] = true;
            };

            let mut inodes = BTreeMap::new();
            for ino in 1..=IPG as u32 {
                let at = ITABLE * B + (ino as usize - 1) * isz;
                let f = |o: usize| le32(&d, at + o);
                let mode = le16(&d, at + offset_of!(Ext2fsDinode, e2di_mode));
                let nlink = le16(&d, at + offset_of!(Ext2fsDinode, e2di_nlink));
                let dtime = f(offset_of!(Ext2fsDinode, e2di_dtime));
                if !bit(&d, IBITMAP, ino as usize - 1) {
                    // e2fsck takes an inode with links for one in use, and wants a deletion
                    // time on a freed one.
                    assert_eq!(nlink, 0, "free inode {ino} with links");
                    assert!(
                        mode == 0 || dtime != 0,
                        "deleted inode {ino} has zero dtime"
                    );
                    continue;
                }
                if ino < first_ino && ino != EXT2_ROOTINO {
                    continue;
                }
                assert_ne!(mode, 0, "inode {ino} allocated with mode 0");
                assert_ne!(nlink, 0, "inode {ino} allocated with no links");
                assert_eq!(dtime, 0, "inode {ino} in use, but has dtime set");
                let size = u64::from(f(offset_of!(Ext2fsDinode, e2di_size)));
                let nblock = f(offset_of!(Ext2fsDinode, e2di_nblock));
                let blocks: Vec<u32> = (0..15)
                    .map(|i| f(offset_of!(Ext2fsDinode, e2di_blocks) + 4 * i))
                    .collect();
                let uid = u32::from(le16(&d, at + offset_of!(Ext2fsDinode, e2di_uid_low)))
                    | u32::from(le16(&d, at + offset_of!(Ext2fsDinode, e2di_uid_high))) << 16;
                let gid = u32::from(le16(&d, at + offset_of!(Ext2fsDinode, e2di_gid_low)))
                    | u32::from(le16(&d, at + offset_of!(Ext2fsDinode, e2di_gid_high))) << 16;

                let fast = u32::from(mode) & IFMT == IFLNK && size < EXT2_MAXSYMLINKLEN as u64;
                let special = !matches!(u32::from(mode) & IFMT, IFREG | IFDIR | IFLNK);
                let mut data = Vec::new();
                let mut nblk = 0u32;
                if special {
                    // A device keeps its number in the first block pointer.
                    assert_eq!((size, nblock), (0, 0), "special file {ino} with data");
                    assert!(
                        blocks[1..].iter().all(|&b| b == 0),
                        "special file {ino}: pointers"
                    );
                    data.extend_from_slice(&blocks[0].to_le_bytes());
                } else if fast {
                    assert_eq!(nblock, 0, "fast symlink {ino} has blocks");
                    let s = at + offset_of!(Ext2fsDinode, e2di_blocks);
                    data.extend_from_slice(&d[s..s + size as usize]);
                } else {
                    // The logical blocks in order (0 for a hole), claiming each block.
                    let mut lblocks: Vec<u32> = blocks[..12].to_vec();
                    let ptrs = |b: u32| -> Vec<u32> {
                        (0..B / 4)
                            .map(|i| le32(&d, b as usize * B + 4 * i))
                            .collect()
                    };
                    for &b in blocks[..12].iter().filter(|&&b| b != 0) {
                        claim(&mut claimed, b, ino);
                        nblk += 1;
                    }
                    if blocks[12] != 0 {
                        claim(&mut claimed, blocks[12], ino);
                        nblk += 1;
                        lblocks.extend(ptrs(blocks[12]));
                    }
                    if blocks[13] != 0 {
                        claim(&mut claimed, blocks[13], ino);
                        nblk += 1;
                        for b in ptrs(blocks[13]).into_iter().filter(|&b| b != 0) {
                            claim(&mut claimed, b, ino);
                            nblk += 1;
                            lblocks.extend(ptrs(b));
                        }
                    }
                    assert_eq!(blocks[14], 0, "inode {ino}: triple indirect block");
                    for &b in lblocks[12..].iter().filter(|&&b| b != 0) {
                        claim(&mut claimed, b, ino);
                        nblk += 1;
                    }
                    assert_eq!(nblock, 2 * nblk, "inode {ino}: i_blocks in 512-byte units");
                    let nlb = (size as usize).div_ceil(B);
                    assert!(
                        lblocks.iter().skip(nlb).all(|&b| b == 0),
                        "inode {ino}: blocks past its size"
                    );
                    for i in 0..nlb {
                        match lblocks.get(i).copied().unwrap_or(0) {
                            0 => data.extend_from_slice(&[0; B]),
                            b => data.extend_from_slice(&d[b as usize * B..(b as usize + 1) * B]),
                        }
                    }
                    data.truncate(size as usize);
                }
                inodes.insert(
                    ino,
                    Ino {
                        mode,
                        nlink,
                        uid,
                        gid,
                        size,
                        data,
                    },
                );
            }

            // Directories: record chains, entries, and the references they make.
            let mut refs: BTreeMap<u32, u16> = BTreeMap::new();
            let mut dirs = BTreeMap::new();
            for (&ino, ip) in inodes
                .iter()
                .filter(|(_, ip)| u32::from(ip.mode) & IFMT == IFDIR)
            {
                assert!(
                    ip.size > 0 && ip.size % B as u64 == 0,
                    "dir {ino}: size {}",
                    ip.size
                );
                let mut ents = Vec::new();
                for blk in ip.data.chunks(B) {
                    let mut off = 0;
                    while off < B {
                        let e = le32(blk, off);
                        let reclen = usize::from(le16(blk, off + 4));
                        let namlen = usize::from(blk[off + 6]);
                        assert!(
                            reclen >= 8 && reclen % 4 == 0,
                            "dir {ino}: rec_len {reclen}"
                        );
                        assert!(off + reclen <= B, "dir {ino}: entry across blocks");
                        if e != 0 {
                            assert!(
                                reclen >= ext2fs_dirsiz(namlen),
                                "dir {ino}: rec_len too small"
                            );
                            let target = inodes.get(&e).unwrap_or_else(|| {
                                std::panic!("dir {ino}: entry to free inode {e}")
                            });
                            let want = inot2ext2dt(e2iftodt(target.mode));
                            assert_eq!(e2d_type(blk, off), want, "dir {ino}: file type of {e}");
                            ents.push((blk[off + 8..off + 8 + namlen].to_vec(), e, blk[off + 7]));
                            *refs.entry(e).or_default() += 1;
                        }
                        off += reclen;
                    }
                    assert_eq!(
                        off, B,
                        "dir {ino}: the record chain ends at the block's end"
                    );
                }
                assert_eq!(ents[0].0, b".", "dir {ino}: first entry");
                assert_eq!(ents[0].1, ino, "dir {ino}: `.`");
                assert_eq!(ents[1].0, b"..", "dir {ino}: second entry");
                dirs.insert(ino, ents);
            }
            for (&ino, ip) in &inodes {
                assert_eq!(
                    ip.nlink,
                    refs.get(&ino).copied().unwrap_or(0),
                    "inode {ino}: links"
                );
            }
            // Every directory's ".." is the directory that names it (the root's is itself).
            for (&ino, ents) in &dirs {
                let parents: Vec<u32> = dirs
                    .iter()
                    .filter(|(_, es)| es[2..].iter().any(|e| e.1 == ino))
                    .map(|(&p, _)| p)
                    .collect();
                let want = if ino == EXT2_ROOTINO {
                    vec![]
                } else {
                    vec![ents[1].1]
                };
                assert_eq!(parents, want, "dir {ino}: `..` and the entries naming it");
            }

            // Bitmaps and counters.
            let mut freeb = 0;
            for (b, &c) in claimed.iter().enumerate().skip(1) {
                assert_eq!(bit(&d, BBITMAP, b - 1), c, "block {b} in the bitmap");
                freeb += u32::from(!c);
            }
            let mut freei = 0;
            for i in 1..=IPG as u32 {
                let used = bit(&d, IBITMAP, i as usize - 1);
                if i >= first_ino {
                    assert_eq!(used, inodes.contains_key(&i), "inode {i} in the bitmap");
                }
                freei += u32::from(!used);
            }
            let ndirs = dirs.len() as u16;
            assert_eq!(
                le32(&d, sb + offset_of!(Ext2fs, e2fs_fbcount)),
                freeb,
                "sb free blocks"
            );
            assert_eq!(
                le32(&d, sb + offset_of!(Ext2fs, e2fs_ficount)),
                freei,
                "sb free inodes"
            );
            assert_eq!(le16(&d, GDT * B + 12), freeb as u16, "gd free blocks");
            assert_eq!(le16(&d, GDT * B + 14), freei as u16, "gd free inodes");
            assert_eq!(le16(&d, GDT * B + 16), ndirs, "gd directories");
            Fs { inodes, dirs }
        }

        impl Fs {
            /// The inode `path` names (absolute, no symbolic links).
            fn lookup(&self, path: &str) -> Option<u32> {
                let mut ino = EXT2_ROOTINO;
                for c in path.split('/').filter(|c| !c.is_empty()) {
                    ino = self.dirs.get(&ino)?.iter().find(|e| e.0 == c.as_bytes())?.1;
                }
                Some(ino)
            }

            fn ino(&self, path: &str) -> &Ino {
                let i = self
                    .lookup(path)
                    .unwrap_or_else(|| std::panic!("{path} on disk"));
                &self.inodes[&i]
            }

            /// The names in directory `path`, without `.` and `..`.
            fn names(&self, path: &str) -> Vec<Vec<u8>> {
                let i = self.lookup(path).unwrap();
                self.dirs[&i][2..].iter().map(|e| e.0.clone()).collect()
            }
        }

        /// The locks a test holds: the timecounter tests' and the memory's.
        type Guards = (MutexGuard<'static, ()>, MutexGuard<'static, ()>);

        /// A fresh file system of `isz`-byte inodes mounted read-write as "/" and made the
        /// thread's current directory, on the mount list as `sys_mount` leaves it.
        ///
        /// The clock is set to 2023 first (under the timecounter tests' lock, taken before the
        /// memory's): `ext2fs_inactive` marks a freed inode by its deletion time, and a time
        /// of 0 would leave it looking alive, which a booted kernel never sees.
        pub(super) fn setup_root(
            isz: usize,
        ) -> (Guards, &'static Thread<TestGlue>, &'static Mount<TestGlue>) {
            let t = crate::tc::tests::LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let (g, p) = attach(mkfs(isz));
            crate::tc::tc_setrealtimeclock(&Timespec::new(1_700_000_000, 0));
            assert!(crate::tc::getnanotime().tv_sec >= 1_700_000_000);
            let mp = mount_root(p, 0);
            ((t, g), p, mp)
        }

        /// `s` as the first path argument (testext2fs's slot 0).
        fn c(s: &str) -> UserPtr<u8> {
            up(0, s.as_bytes())
        }

        /// `s` as the second path argument (slot 1).
        fn c2(s: &str) -> UserPtr<u8> {
            up(1, s.as_bytes())
        }

        /// The vnode `path` names, unlocked and referenced.
        fn lookup(p: &Thread<TestGlue>, path: &str) -> Result<&'static Vnode<TestGlue>, Errno> {
            let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(path.as_bytes()), p);
            namei(&mut nd)?;
            Ok(nd.ni_vp.expect("a vnode"))
        }

        /// `stat(path)` (`lstat` with `NOFOLLOW`).
        fn stat_flags(p: &Thread<TestGlue>, path: &str, follow: u64) -> Result<Vattr, Errno> {
            let mut nd = ndinit(LOOKUP, LOCKLEAF | follow, NiDirp::Sys(path.as_bytes()), p);
            namei(&mut nd)?;
            let vp = nd.ni_vp.expect("a vnode");
            let mut va = Vattr::new();
            let error = vop_getattr(vp, &mut va, Cred::real(p.ucred()), p);
            vput(vp);
            error.map(|()| va)
        }

        fn stat(p: &Thread<TestGlue>, path: &str) -> Result<Vattr, Errno> {
            stat_flags(p, path, FOLLOW)
        }

        /// The whole contents of the file `path`.
        fn read_file(p: &Thread<TestGlue>, path: &str) -> Vec<u8> {
            let fd = sys_open(p, c(path), O_RDONLY, 0).expect("open");
            let mut out = Vec::new();
            loop {
                let n = sys_read(p, fd, UBUF, UBUF_LEN).expect("read");
                if n == 0 {
                    break;
                }
                let mut b = vec![0u8; n];
                UserPtr::<u8>::new(UBUF).copyin_slice(&mut b).unwrap();
                out.extend_from_slice(&b);
            }
            sys_close(p, fd).expect("close");
            out
        }

        /// A file `path` holding `data` (created, or truncated by nothing: new names only),
        /// `fsync`ed.
        fn write_file(p: &Thread<TestGlue>, path: &str, data: &[u8]) -> Result<(), Errno> {
            let fd = sys_open(p, c(path), O_RDWR | O_CREAT, 0o644)?;
            for chunk in data.chunks(UBUF_LEN) {
                UserPtr::<u8>::new(UBUF).copyout_slice(chunk).unwrap();
                assert_eq!(sys_write(p, fd, UBUF, chunk.len())?, chunk.len());
            }
            sys_fsync(p, fd)?;
            sys_close(p, fd)?;
            Ok(())
        }

        /// `VOP_SETATTR` with the vnode locked.
        fn setattr(
            p: &Thread<TestGlue>,
            path: &str,
            f: impl FnOnce(&mut Vattr),
        ) -> Result<(), Errno> {
            let vp = lookup(p, path)?;
            let mut va = novals();
            f(&mut va);
            let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
            let error = vop_setattr(vp, &mut va, Cred::real(p.ucred()), p);
            vput(vp);
            error
        }

        /// One `struct dirent` `getdents` returned.
        struct Ent {
            name: Vec<u8>,
            off: i64,
            fileno: u64,
        }

        /// One `VOP_READDIR` of the directory `path` at `offset` into a buffer of `size`
        /// bytes: the entries, the new offset and the EOF flag.
        fn readdir(
            p: &Thread<TestGlue>,
            path: &str,
            offset: i64,
            size: usize,
        ) -> Result<(Vec<Ent>, i64, i32), Errno> {
            let mut nd = ndinit(LOOKUP, LOCKLEAF | FOLLOW, NiDirp::Sys(path.as_bytes()), p);
            namei(&mut nd)?;
            let vp = nd.ni_vp.expect("a vnode");
            let mut buf = vec![0u8; size];
            let mut iov = [Iovec::new()];
            let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
            uio.uio_offset = offset;
            let mut eof = 0;
            let error = vop_readdir(vp, &mut uio, Cred::real(p.ucred()), &mut eof);
            let used = size - uio.uio_resid;
            let newoff = uio.uio_offset;
            drop(uio);
            vput(vp);
            error?;

            let mut ents = Vec::new();
            let mut off = 0;
            while off < used {
                let d = Dirent::from_bytes(&buf[off..]).expect("a dirent");
                let name = &buf[off + Dirent::NAME_OFFSET..][..usize::from(d.d_namlen)];
                assert_eq!(
                    buf[off + Dirent::NAME_OFFSET + name.len()],
                    0,
                    "NUL-terminated"
                );
                ents.push(Ent {
                    name: name.to_vec(),
                    off: d.d_off,
                    fileno: d.d_fileno,
                });
                off += usize::from(d.d_reclen);
            }
            Ok((ents, newoff, eof))
        }

        /// Every name `readdir` gives for `path`, `size` bytes at a time from offset 0,
        /// without the free entries (`d_fileno` 0, which `readdir(3)` skips): the names and
        /// how many free entries there were.
        fn readdir_all(p: &Thread<TestGlue>, path: &str, size: usize) -> (Vec<Vec<u8>>, usize) {
            let mut names = Vec::new();
            let mut free = 0;
            let mut off = 0;
            loop {
                let (ents, noff, eof) = readdir(p, path, off, size).expect("readdir");
                for e in &ents {
                    if e.fileno == 0 {
                        free += 1;
                    } else {
                        names.push(e.name.clone());
                    }
                }
                if let Some(last) = ents.last() {
                    assert_eq!(last.off, noff, "the last d_off is the new offset");
                }
                off = noff;
                if eof != 0 {
                    return (names, free);
                }
                assert!(!ents.is_empty(), "progress");
            }
        }

        /// `n` bytes of a pattern that differs per `seed`.
        fn pattern(n: usize, seed: u8) -> Vec<u8> {
            (0..n)
                .map(|i| ((i * 7 + usize::from(seed)) % 251) as u8)
                .collect()
        }

        /// The UFS mount's super block's free counts.
        fn free_counts(mp: &'static Mount<TestGlue>) -> (u32, u32) {
            let ump = vfstoufs(mp);
            let fs = ump.um_e2fs();
            (fs.e2fs_fbcount(), fs.e2fs_ficount())
        }

        #[test]
        fn lookups_creates_writes_links_and_removes() {
            let (_g, p, mp) = setup_root(128);

            // Lookups of what is there and what is not.
            assert_eq!(stat(p, "/").unwrap().va_fileid, u64::from(EXT2_ROOTINO));
            assert_eq!(stat(p, "/.").unwrap().va_fileid, u64::from(EXT2_ROOTINO));
            assert_eq!(stat(p, "/..").unwrap().va_fileid, u64::from(EXT2_ROOTINO));
            assert_eq!(stat(p, "/missing").err(), Some(Errno::ENOENT));
            assert_eq!(stat(p, "/missing/x").err(), Some(Errno::ENOENT));

            // A file, written across a few blocks and read back.
            let data = pattern(3000, 1);
            write_file(p, "/hello", &data).unwrap();
            let va = stat(p, "/hello").unwrap();
            assert_eq!(va.va_size, 3000);
            assert_eq!(va.va_type, VREG);
            assert_eq!(va.va_mode, 0o644);
            assert_eq!(va.va_nlink, 1);
            assert_eq!(va.va_bytes, 3 * B as u64);
            assert_eq!(read_file(p, "/hello"), data);
            assert_eq!(stat(p, "/hello/x").err(), Some(Errno::ENOTDIR));
            // A second, big enough for an indirect block.
            let big = pattern(14 * B + 7, 2);
            write_file(p, "/big", &big).unwrap();
            assert_eq!(read_file(p, "/big"), big);
            assert_eq!(stat(p, "/big").unwrap().va_bytes, 16 * B as u64);

            // Hard links.
            sys_link(p, c("/hello"), c2("/hl")).unwrap();
            assert_eq!(stat(p, "/hl").unwrap().va_nlink, 2);
            assert_eq!(stat(p, "/hl").unwrap().va_fileid, va.va_fileid);
            assert_eq!(sys_link(p, c("/hello"), c2("/big")), Err(Errno::EEXIST));
            sys_unlink(p, c("/hello")).unwrap();
            assert_eq!(stat(p, "/hello").err(), Some(Errno::ENOENT));
            assert_eq!(stat(p, "/hl").unwrap().va_nlink, 1);
            assert_eq!(read_file(p, "/hl"), data);

            // A removed file's blocks and inode go back (inactive, then reclaim).
            let (fb, fi) = free_counts(mp);
            sys_unlink(p, c("/big")).unwrap();
            assert_eq!(free_counts(mp), (fb + 16, fi + 1));

            // pathconf
            let vp = lookup(p, "/hl").unwrap();
            let mut r = 0;
            vop_pathconf(vp, _PC_TIMESTAMP_RESOLUTION, &mut r).unwrap();
            assert_eq!(r, 1_000_000_000);
            vop_pathconf(vp, _PC_LINK_MAX, &mut r).unwrap();
            assert_eq!(r, LINK_MAX as Register);
            vrele(vp);

            unmount_root(p, mp);
            let fs = fsck();
            assert_eq!(fs.names("/"), [b"hl".to_vec()]);
            assert_eq!(fs.ino("/hl").data, data);
            assert_eq!(fs.ino("/hl").nlink, 1);
        }

        #[test]
        fn directories_are_made_renamed_and_removed() {
            let (_g, p, mp) = setup_root(256);

            sys_mkdir(p, c("/d"), 0o755).unwrap();
            sys_mkdir(p, c("/d/e"), 0o700).unwrap();
            sys_mkdir(p, c("/f"), 0o755).unwrap();
            assert_eq!(sys_mkdir(p, c("/d"), 0o755), Err(Errno::EEXIST));
            let d = stat(p, "/d").unwrap();
            assert_eq!(
                (d.va_type, d.va_mode, d.va_nlink, d.va_size),
                (VDIR, 0o755, 3, B as u64)
            );
            assert_eq!(stat(p, "/").unwrap().va_nlink, 4);
            assert_eq!(stat(p, "/d/e/..").unwrap().va_fileid, d.va_fileid);
            write_file(p, "/d/x", b"x data").unwrap();
            write_file(p, "/d/e/inner", b"inner").unwrap();

            // A file, within a directory, then across directories.
            sys_rename(p, c("/d/x"), c2("/d/x2")).unwrap();
            sys_rename(p, c("/d/x2"), c2("/f/y")).unwrap();
            assert_eq!(stat(p, "/d/x2").err(), Some(Errno::ENOENT));
            assert_eq!(read_file(p, "/f/y"), b"x data");
            // Over an existing file, whose inode goes.
            write_file(p, "/f/z", b"zzz").unwrap();
            let (_, fi) = free_counts(mp);
            sys_rename(p, c("/f/y"), c2("/f/z")).unwrap();
            assert_eq!(read_file(p, "/f/z"), b"x data");
            assert_eq!(free_counts(mp).1, fi + 1);
            // Two names of one file: rename(2) does nothing.
            sys_link(p, c("/f/z"), c2("/f/w")).unwrap();
            sys_rename(p, c("/f/w"), c2("/f/z")).unwrap();
            assert_eq!(stat(p, "/f/z").unwrap().va_nlink, 2);
            sys_unlink(p, c("/f/w")).unwrap();
            assert_eq!(stat(p, "/f/z").unwrap().va_nlink, 1);

            // A directory across directories: ".." follows (dirrewrite), the parents' link
            // counts too.
            sys_rename(p, c("/d/e"), c2("/f/e")).unwrap();
            assert_eq!(stat(p, "/d").unwrap().va_nlink, 2);
            assert_eq!(stat(p, "/f").unwrap().va_nlink, 3);
            assert_eq!(
                stat(p, "/f/e/..").unwrap().va_fileid,
                stat(p, "/f").unwrap().va_fileid
            );
            assert_eq!(read_file(p, "/f/e/inner"), b"inner");
            // Into its own subtree (checkpath), a directory onto a file, a file onto a
            // directory, onto a non-empty directory (dirempty): refused.
            assert_eq!(sys_rename(p, c("/f"), c2("/f/e/g")), Err(Errno::EINVAL));
            assert_eq!(sys_rename(p, c("/f/e"), c2("/f/z")), Err(Errno::ENOTDIR));
            assert_eq!(sys_rename(p, c("/f/z"), c2("/f/e")), Err(Errno::EISDIR));
            sys_mkdir(p, c("/d/full"), 0o755).unwrap();
            write_file(p, "/d/full/f", b"").unwrap();
            assert_eq!(
                sys_rename(p, c("/f/e"), c2("/d/full")),
                Err(Errno::ENOTEMPTY)
            );
            // A directory within its parent, then over an empty directory elsewhere.
            sys_rename(p, c("/f/e"), c2("/f/e2")).unwrap();
            assert_eq!(stat(p, "/f").unwrap().va_nlink, 3);
            sys_mkdir(p, c("/d/empty"), 0o755).unwrap();
            assert_eq!(stat(p, "/d").unwrap().va_nlink, 4);
            sys_rename(p, c("/f/e2"), c2("/d/empty")).unwrap();
            assert_eq!(stat(p, "/d").unwrap().va_nlink, 4);
            assert_eq!(stat(p, "/f").unwrap().va_nlink, 2);
            assert_eq!(read_file(p, "/d/empty/inner"), b"inner");
            assert_eq!(stat(p, "/d/empty/..").unwrap().va_fileid, d.va_fileid);

            // rmdir: not a non-empty one; unlink: not a directory.
            assert_eq!(sys_rmdir(p, c("/d/full")), Err(Errno::ENOTEMPTY));
            assert_eq!(sys_unlink(p, c("/d/full")), Err(Errno::EPERM));
            sys_unlink(p, c("/d/full/f")).unwrap();
            sys_rmdir(p, c("/d/full")).unwrap();
            assert_eq!(stat(p, "/d/full").err(), Some(Errno::ENOENT));
            assert_eq!(stat(p, "/d").unwrap().va_nlink, 3);

            unmount_root(p, mp);
            let fs = fsck();
            assert_eq!(fs.names("/"), [b"d".to_vec(), b"f".to_vec()]);
            assert_eq!(fs.names("/d"), [b"empty".to_vec()]);
            assert_eq!(fs.names("/f"), [b"z".to_vec()]);
            assert_eq!(fs.names("/d/empty"), [b"inner".to_vec()]);
            assert_eq!(fs.ino("/d/empty/inner").data, b"inner");
            assert_eq!(fs.ino("/d").nlink, 3);
            assert_eq!(fs.ino("/").nlink, 4);
            assert_eq!(fs.ino("/d/empty").mode, 0o40700);
        }

        #[test]
        fn symbolic_links_short_and_long() {
            let (_g, p, mp) = setup_root(128);
            write_file(p, "/target", b"through the link").unwrap();
            sys_symlink(p, c("target"), c2("/s")).unwrap();
            let long: String = (0..100).map(|i| (b'a' + (i % 26) as u8) as char).collect();
            sys_symlink(p, c(&long), c2("/l")).unwrap();
            assert_eq!(sys_symlink(p, c("x"), c2("/s")), Err(Errno::EEXIST));
            // readlink of both, lstat, and a lookup through the short one.
            let readlink = |path: &str| {
                let n = sys_readlink(p, c(path), UBUF, 300).unwrap();
                let mut buf = vec![0u8; n];
                UserPtr::<u8>::new(UBUF).copyin_slice(&mut buf).unwrap();
                buf
            };
            assert_eq!(readlink("/s"), b"target");
            assert_eq!(readlink("/l"), long.as_bytes());
            let s = stat_flags(p, "/s", NOFOLLOW).unwrap();
            assert_eq!((s.va_type, s.va_size, s.va_bytes), (VLNK, 6, 0));
            let l = stat_flags(p, "/l", NOFOLLOW).unwrap();
            assert_eq!((l.va_type, l.va_size, l.va_bytes), (VLNK, 100, B as u64));
            assert_eq!(read_file(p, "/s"), b"through the link");

            unmount_root(p, mp);
            let fs = fsck();
            assert_eq!(fs.ino("/s").data, b"target");
            assert_eq!(fs.ino("/l").data, long.as_bytes());
            assert_eq!(u32::from(fs.ino("/s").mode) & IFMT, IFLNK);
            let root = &fs.dirs[&EXT2_ROOTINO];
            assert!(root.iter().any(|e| e.0 == b"s" && e.2 == EXT2_FT_SYMLINK));
            assert!(
                root.iter()
                    .any(|e| e.0 == b"target" && e.2 == EXT2_FT_REG_FILE)
            );
        }

        #[test]
        fn readdir_spans_blocks_and_skips_deleted_entries() {
            let (_g, p, mp) = setup_root(128);
            sys_mkdir(p, c("/t"), 0o755).unwrap();
            write_file(p, "/t/base", b"b").unwrap();
            // 40 links with 60-byte names: 68-byte entries, 15 to a block, three blocks.
            let name = |i: usize| format!("{i:02}{}", "n".repeat(58));
            for i in 0..40 {
                sys_link(p, c("/t/base"), c2(&format!("/t/{}", name(i)))).unwrap();
            }
            assert_eq!(stat(p, "/t").unwrap().va_size, 3 * B as u64);
            assert_eq!(stat(p, "/t/base").unwrap().va_nlink, 41);
            let mut want: Vec<Vec<u8>> = vec![b".".to_vec(), b"..".to_vec(), b"base".to_vec()];
            want.extend((0..40).map(|i| name(i).into_bytes()));
            // A buffer must reach the end of a directory block; 1024 bytes of 68-byte ext2
            // entries are more than 1024 bytes of dirents, so the smaller reads stop and
            // resume mid-block.
            for size in [1024, 1500, 4096] {
                let (names, free) = readdir_all(p, "/t", size);
                assert_eq!(names, want, "{size}-byte reads");
                assert_eq!(free, 0);
            }

            // Remove the first entry of the second block (its inode is zeroed, the entry
            // stays) and one in the middle of the third (its space goes to the entry before).
            let first2 = 14; // ".", "..", "base" (36 bytes) and 14 names fill the first block
            for i in [first2, 33] {
                let path = format!("/t/{}", name(i));
                sys_unlink(p, c(&path)).unwrap();
                assert_eq!(stat(p, &path).err(), Some(Errno::ENOENT));
            }
            want.retain(|n| *n != name(first2).into_bytes() && *n != name(33).into_bytes());
            let (names, free) = readdir_all(p, "/t", 1024);
            assert_eq!(names, want);
            assert_eq!(
                free, 1,
                "the free first entry of a block is listed with d_fileno 0"
            );
            // A new name fits in the freed space: the directory does not grow.
            sys_link(p, c("/t/base"), c2("/t/new")).unwrap();
            assert_eq!(stat(p, "/t").unwrap().va_size, 3 * B as u64);
            // A partial entry is refused, a read past the end is empty.
            assert_eq!(readdir(p, "/t", 0, 10).err(), Some(Errno::EINVAL));
            let (ents, _, eof) = readdir(p, "/t", 3 * B as i64, 1024).unwrap();
            assert!(ents.is_empty() && eof != 0);
            assert_eq!(readdir(p, "/t/base", 0, 1024).err(), Some(Errno::ENOTDIR));

            unmount_root(p, mp);
            let fs = fsck();
            assert_eq!(fs.ino("/t/base").nlink, 40);
            assert_eq!(fs.names("/t").len(), 40);
        }

        #[test]
        fn entries_are_compacted_to_make_room() {
            let (_g, p, mp) = setup_root(128);
            sys_mkdir(p, c("/c"), 0o755).unwrap();
            // 168 names of 12-byte entries: 83 after "." and ".." fill the first block but 4
            // bytes, 85 the second.
            write_file(p, "/c/x0", b"x").unwrap();
            for i in 1..168 {
                sys_link(p, c("/c/x0"), c2(&format!("/c/x{i}"))).unwrap();
            }
            assert_eq!(stat(p, "/c").unwrap().va_size, 2 * B as u64);
            let unlink = |name: &str| sys_unlink(p, c(&format!("/c/{name}"))).unwrap();
            let link = |name: &str| sys_link(p, c("/c/x0"), c2(&format!("/c/{name}"))).unwrap();

            // Two 12-byte holes in the first block, none big enough for a 16-byte entry alone:
            // the entries between them move up to make one.
            unlink("x10");
            unlink("x20");
            link("yyyyy");
            // In the second block the hole starts with a free first entry (inode 0).
            unlink("x83");
            unlink("x85");
            link("zzzzz");
            assert_eq!(stat(p, "/c").unwrap().va_size, 2 * B as u64, "no new block");
            assert_eq!(read_file(p, "/c/yyyyy"), b"x");
            assert_eq!(read_file(p, "/c/zzzzz"), b"x");
            let (names, free) = readdir_all(p, "/c", 4096);
            assert_eq!(names.len(), 2 + 168 - 4 + 2);
            assert_eq!(free, 0);

            unmount_root(p, mp);
            let fs = fsck();
            let names = fs.names("/c");
            let at = |n: &[u8]| names.iter().position(|x| x == n).unwrap();
            // yyyyy took the place of x11..x20 moved up, before x21; zzzzz followed x84.
            assert!(at(b"x9") < at(b"yyyyy") && at(b"yyyyy") < at(b"x21"));
            assert!(at(b"x84") < at(b"zzzzz") && at(b"zzzzz") < at(b"x86"));
            assert_eq!(fs.ino("/c/x0").nlink, 166);
        }

        #[test]
        fn attributes_are_set_and_read() {
            let (_g, p, mp) = setup_root(128);
            write_file(p, "/f", &pattern(5000, 3)).unwrap();

            // chmod and chown (as root).
            setattr(p, "/f", |va| va.va_mode = 0o4751).unwrap();
            assert_eq!(stat(p, "/f").unwrap().va_mode, 0o4751);
            setattr(p, "/f", |va| {
                va.va_uid = 70_000;
                va.va_gid = 20;
            })
            .unwrap();
            let va = stat(p, "/f").unwrap();
            assert_eq!((va.va_uid, va.va_gid), (70_000, 20));
            // A directory cannot be truncated.
            assert_eq!(setattr(p, "/", |va| va.va_size = 0), Err(Errno::EISDIR));
            // Unsettable attributes.
            assert_eq!(setattr(p, "/f", |va| va.va_nlink = 3), Err(Errno::EINVAL));

            // Truncate: shorter, then empty; the blocks go back.
            let (fb, _) = free_counts(mp);
            setattr(p, "/f", |va| va.va_size = 100).unwrap();
            assert_eq!(read_file(p, "/f"), pattern(100, 3));
            assert_eq!(free_counts(mp).0, fb + 4);
            assert_eq!(stat(p, "/f").unwrap().va_bytes, B as u64);

            // Times.
            setattr(p, "/f", |va| {
                va.va_mtime = Timespec::new(1_234_567_890, 0);
                va.va_atime = Timespec::new(1_000_000_000, 0);
            })
            .unwrap();
            let va = stat(p, "/f").unwrap();
            assert_eq!(va.va_mtime.tv_sec, 1_234_567_890);
            assert_eq!(va.va_atime.tv_sec, 1_000_000_000);

            // Immutable files cannot be written or removed.
            setattr(p, "/f", |va| va.va_flags = u64::from(SF_IMMUTABLE)).unwrap();
            assert_eq!(stat(p, "/f").unwrap().va_flags, u64::from(SF_IMMUTABLE));
            assert_eq!(setattr(p, "/f", |va| va.va_mode = 0o600), Err(Errno::EPERM));
            assert_eq!(sys_unlink(p, c("/f")), Err(Errno::EPERM));
            setattr(p, "/f", |va| va.va_flags = 0).unwrap();
            setattr(p, "/f", |va| va.va_size = 0).unwrap();
            assert_eq!(stat(p, "/f").unwrap().va_bytes, 0);

            // A device node: mknod reloads it as a special file (vinit's alias, the special
            // files' table). Fifos need option FIFO's fifofs.
            let dev = makedev(2, 7);
            sys_mknod(p, c("/tty"), S_IFCHR | 0o600, dev).unwrap();
            let va = stat(p, "/tty").unwrap();
            assert_eq!((va.va_type, va.va_rdev), (VCHR, dev));
            let vp = lookup(p, "/tty").unwrap();
            assert_eq!(
                vp.ops_id(),
                vops_id::<TestGlue>(&vfs::<TestGlue>().ufs.ext2fs_specvops)
            );
            vrele(vp);
            assert_eq!(
                sys_mknod(p, c("/fifo"), S_IFIFO | 0o600, 0),
                Err(Errno::EOPNOTSUPP)
            );

            unmount_root(p, mp);
            let fs = fsck();
            let f = fs.ino("/f");
            assert_eq!((f.uid, f.gid, f.size, f.mode), (70_000, 20, 0, 0o104751));
            assert_eq!(fs.ino("/tty").mode, 0o20600);
            assert_eq!(fs.names("/"), [b"f".to_vec(), b"tty".to_vec()]);
        }

        #[test]
        fn a_second_vget_of_a_cached_root_owned_inode_returns() {
            // ufs_ihashget's link count check reads i_e2fs_nlink for an ext2fs inode; read as
            // a UFS1 dinode, the nlink of a file owned by uid 0 is its uid_low, 0, and the
            // lookup would wait for the inode to go away forever.
            let (_g, p, mp) = setup_root(128);
            write_file(p, "/root-owned", b"r").unwrap();
            let ino = stat(p, "/root-owned").unwrap().va_fileid;
            let vp = lookup(p, "/root-owned").unwrap();
            assert_eq!(vtoi(vp).i_e2din().i_e2fs_uid.get(), 0);
            let dev = vtoi(vp).i_dev;
            vrele(vp);
            for _ in 0..2 {
                let vp = VFS_VGET(mp, ino).unwrap();
                assert_eq!(u64::from(vtoi(vp).i_number), ino);
                vput(vp);
            }
            let vp = crate::fs::ufs::ufs::ufs_ihash::ufs_ihashget::<TestGlue>(dev, ino as Ufsino)
                .expect("in the hash");
            vput(vp);
            unmount_root(p, mp);
            fsck();
        }

        #[test]
        fn a_read_only_mount_refuses_changes_and_closes_update_the_times() {
            let (_g, p, mp) = setup_root(128);
            write_file(p, "/kept", b"kept").unwrap();
            // A file open twice: its last but one close runs ufs_close's itimes, which an
            // ext2fs inode takes into its own dinode (`EXT2FS_ITIMES`).
            let fd1 = sys_open(p, c("/kept"), O_RDONLY, 0).unwrap();
            let fd2 = sys_open(p, c("/kept"), O_RDONLY, 0).unwrap();
            let vp = lookup(p, "/kept").unwrap();
            let ip = vtoi(vp);
            ip.i_e2din().set_i_e2fs_atime(5);
            ip.set_flag(IN_ACCESS);
            sys_close(p, fd2).unwrap();
            assert_eq!(ip.i_flag.get() & IN_ACCESS, 0, "itimes ran");
            assert_ne!(ip.i_e2din().i_e2fs_atime(), 5, "into the ext2fs dinode");
            assert_ne!(ip.i_flag.get() & IN_MODIFIED, 0);
            drop(ip);
            vrele(vp);
            sys_close(p, fd1).unwrap();
            unmount_root(p, mp);
            fsck();

            // Read-only: lookups for a change answer EROFS.
            let mp = mount_root(p, MNT_RDONLY);
            assert_eq!(read_file(p, "/kept"), b"kept");
            assert_eq!(write_file(p, "/new", b"x"), Err(Errno::EROFS));
            assert_eq!(sys_unlink(p, c("/kept")), Err(Errno::EROFS));
            assert_eq!(sys_mkdir(p, c("/d"), 0o755), Err(Errno::EROFS));
            assert_eq!(sys_rename(p, c("/kept"), c2("/k2")), Err(Errno::EROFS));
            unmount_root(p, mp);
            fsck();
        }

        #[test]
        fn a_read_filter_reports_the_bytes_past_the_offset() {
            use crate::event::{EV_ADD, EVFILT_READ, Kevent, sys_kevent, sys_kqueue};
            let (_g, p, mp) = setup_root(128);
            write_file(p, "/k", &pattern(3000, 4)).unwrap();
            let fd = sys_open(p, c("/k"), O_RDONLY, 0).unwrap();
            assert_eq!(sys_read(p, fd, UBUF, 1000), Ok(1000));
            // The kqueue pools (a second init of the process answers EBUSY).
            let _ = crate::event::kqueue_init::<TestGlue>();
            let kfd = sys_kqueue(p).unwrap();
            let change = Kevent {
                ident: fd as usize,
                filter: EVFILT_READ,
                flags: EV_ADD,
                ..Kevent::default()
            };
            UserPtr::<Kevent>::new(UBUF)
                .copyout_slice(&[change])
                .unwrap();
            let events = UserPtr::<Kevent>::new(UBUF + 0x100);
            // ufs_kqfilter attaches filt_ufsread, whose EXT2FS branch takes the size from the
            // ext2fs dinode (`ext2fs_size`): 3000 - 1000 bytes to read.
            assert_eq!(
                sys_kevent(p, kfd, UserPtr::new(UBUF), 1, events, 1, 0),
                Ok(1)
            );
            let mut got = [Kevent::default()];
            events.copyin_slice(&mut got).unwrap();
            assert_eq!((got[0].ident, got[0].data), (fd as usize, 2000));
            sys_close(p, kfd).unwrap();
            sys_close(p, fd).unwrap();
            unmount_root(p, mp);
            fsck();
        }
    }
}
/* </TESTS> */
