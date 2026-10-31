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
 */
/* </LICENSES> */

/* <CODE> */
//! The vnode operations the UFS file systems share: create, link, rename, remove, mkdir,
//! rmdir, symlink, readdir, readlink, attributes and permissions, the inode lock, the
//! strategy that maps a file's logical blocks to the disk, pathconf, advisory locks, the
//! wrappers of the special file operations that keep an inode's times, and the vnode tables
//! of a UFS file system composed from them ([`UfsVnops`], [`UfsSpecVnops`]).
//!
//! Upstream: sys/ufs/ufs/ufs_vnops.c @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/ufs_vnops.rs@3c62ede68802
//!
//! ## Deviations
//! - `option FIFO` is in GENERIC, but `miscfs/fifofs` is not ported: `ufsfifo_read`,
//!   `ufsfifo_write`, `ufsfifo_close` (and `ffs_fifovops`, `ffsfifo_reclaim`) come with it;
//!   until then `ffs_vinit` refuses a fifo with `EOPNOTSUPP`, as a kernel without `FIFO`
//!   does.
//! - `EXT2FS`'s branches (`EXT2FS_ITIMES` in `ufs_itimes`, `ext2fs_size` in
//!   `filt_ufsread`) test the inode's `um_fstype` (`IS_EXT2_VNODE`) and reach ext2fs's
//!   dinode through the glue (`FileGlue::ext2fs_itimes`, `FileGlue::ext2fs_size`: `bsd`'s
//!   ext2fs keeps it until N5.6).
//! - `pool_put(&namei_pool, cnp->cn_pnbuf)` is [`pnbuf_free`].
//! - Credentials that the C dereferences (`cred->cr_uid`) go through [`ucred`], which panics
//!   on `NOCRED`/`FSCRED`, where the C would follow a bad pointer.
//! - `UFS_DIRHASH`'s `ufsdirhash_free` in `ufs_rmdir` is under feature `ufs_dirhash`.
//! - `ufs_print`'s body is compiled under feature `debug` or `diagnostic`, as `VOP_PRINT`
//!   calls it only then.
//!
//! ## Redesign
//! - LZ's `ufs_*(&mut VopXxxArgs)` over `bsd`'s vnode and `VOP_*` adapters -> generic
//!   functions with the kernel's `Vnops<G>` method shapes (`vp`, the C's arguments, the
//!   out-parameters `&mut`), over the kernel's vnode, `vop_*` functions and an `InodeRef`
//!   (`vtoi`, held for the call). `bsd`'s ext2fs still calls eight of them (and the three
//!   `ufsspec_*`) through `bsd`'s adapter (docs/SCAFFOLDING.md).
//! - ffs's two tables (`ffs_vops` and `ffs_specvops`, `sys/ufs/ffs/ffs_vnops.c:60-134`, LZ's
//!   `Vops` statics of `sys/ufs/ffs/ffs_vnops.rs`) are composed here once:
//!   [`UfsVnops<G, F>`] and [`UfsSpecVnops<G, F>`] implement `Vnops<G>` with the UFS slots
//!   calling the functions of this module, the generic slots the kernel's `vop_generic_*`
//!   and `spec_*`, and the three slots that are the file system's own (`read`, `write`,
//!   `fsync`) calling `F:` [`UfsFs`] (the residue of the C's table: decision 46's written
//!   reason, `bridges.toml`'s row while ffs is `bsd`'s). `reclaim` is `ffs_reclaim`'s body,
//!   written in the table (`ufs_reclaim`, then the vnode's node taken: the dinode's and the
//!   inode's frees are their handles' drops), since it needs nothing of ffs's types (`bsd`'s
//!   `ffs_reclaim`, which no table names now, waits for ffs's move, N5.5b). `bsd`'s
//!   `FFS_VOPS` and `FFS_SPECVOPS` are statics of these types.
//! - The knote hook is the vnode itself (`KnHook::Vnode`, `Knote::set_hook_vnode`): LZ's
//!   `kn_hook` address and its `unsafe` deref (`kn_vnode`) are gone; the three filters are
//!   [`UfsRead`], [`UfsWrite`] and [`UfsVnode`], `Filter<G>` impls whose tables are made at
//!   compile time.
//! - A directory block is the busy buffer's token (`ufs_lookup.rs`'s `bufatoff` and
//!   `buf_alloc`), read and written through it: LZ's two `unsafe { bp.data() }` are gone.
//!   The buffer of `VOP_STRATEGY` is the kernel's `IoBuf`, passed on to the device as it came.
//! - `ufs_mkdir`'s call of `ufs_direnter` no longer passes the soft dependency code's
//!   `newdirbp` (`ufs_lookup.rs`).

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use hal::sys::dirent::{DT_DIR, Dirent, iftodt};
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FWRITE, Flock, O_APPEND, O_TRUNC};
use hal::sys::mount::{MNT_NOATIME, MNT_RDONLY};
use hal::sys::param::PAGE_SIZE;
use hal::sys::stat::{
    ALLPERMS, APPEND, IMMUTABLE, S_ISTXT, S_IXGRP, S_IXOTH, S_IXUSR, SF_APPEND, SF_IMMUTABLE,
    SF_SETTABLE, UF_SETTABLE,
};
use hal::sys::syslimits::{LINK_MAX, NAME_MAX};
use hal::sys::types::{Daddr, Dev, Gid, Mode, Nlink, Register, Uid};
use hal::sys::unistd::{
    _PC_2_SYMLINKS, _PC_ALLOC_SIZE_MIN, _PC_CHOWN_RESTRICTED, _PC_FILESIZEBITS, _PC_LINK_MAX,
    _PC_NAME_MAX, _PC_NO_TRUNC, _PC_REC_INCR_XFER_SIZE, _PC_REC_MAX_XFER_SIZE,
    _PC_REC_MIN_XFER_SIZE, _PC_REC_XFER_ALIGN, _PC_SYMLINK_MAX, _PC_TIMESTAMP_RESOLUTION,
};

use super::dinode::{IFDIR, IFLNK, IFMT, IFREG, ISGID, ISUID, MAXSYMLINKLEN_UFS2};
use super::dir::{DIRBLKSIZ, Direct, Dirtemplate, d_ino, d_name, d_namlen, d_reclen, d_type};
use super::inode::{
    IN_ACCESS, IN_CHANGE, IN_LAZYMOD, IN_MODIFIED, IN_RENAME, IN_UPDATE, Inode, InodeRef,
    doingasync, vtoi,
};
use super::quota::{
    UFS_QUOTA_FORCE, UFS_QUOTA_NOGID, UFS_QUOTA_NOUID, UfsQuotaFlags, getinoquota,
    ufs_quota_alloc_blocks2, ufs_quota_alloc_inode, ufs_quota_alloc_inode2, ufs_quota_delete,
    ufs_quota_free_blocks2, ufs_quota_free_inode2,
};
use super::ufs_bmap::ufs_bmap;
use super::ufs_inode::{ufs_inactive, ufs_reclaim};
use super::ufs_lookup::{
    buf_alloc, bufatoff, ufs_checkpath, ufs_dirempty, ufs_direnter, ufs_dirremove, ufs_dirrewrite,
    ufs_lookup, ufs_makedirentry, vmount,
};
use super::ufsmount::UM_EXT2FS;
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::cred::{Ucred, groupmember, suser_ucred};
use crate::event::{
    __EV_POLL, __EV_SELECT, EV_EOF, EV_ONESHOT, EVFILT_READ, EVFILT_VNODE, EVFILT_WRITE,
    FILTEROP_ISFD, Filter, Filterops, Knote, NOTE_ATTRIB, NOTE_DELETE, NOTE_EOF, NOTE_LINK,
    NOTE_RENAME, NOTE_REVOKE, NOTE_TRUNCATE, NOTE_WRITE,
};
use crate::file::{FileGlue, LockOwner, foffset};
use crate::fs::bio::{biodone, biodone_parked, brelse};
use crate::fs::buf::{B_CLRBUF, B_ERROR, BufRef};
use crate::fs::cache::cache_purge;
use crate::fs::default;
use crate::fs::lockf::lf_advlock;
use crate::fs::lookup::vfs_relookup;
use crate::fs::namei::{
    Componentname, DELETE, ISDOTDOT, LOCKLEAF, LOCKPARENT, MAXPATHLEN, MODMASK, SAVESTART,
};
use crate::fs::spec::{spec_close, spec_read, spec_write};
use crate::fs::subr::{vaccess, vgone, vnoperm, vput, vref, vrele};
use crate::fs::vnode::{
    IO_NODELOCKED, IO_SYNC, LK_EXCLUSIVE, LK_RWFLAGS, VA_UTIMES_CHANGE, VA_UTIMES_NULL, VBAD, VBLK,
    VCHR, VDIR, VEXEC, VFIFO, VLNK, VN_KNOTE, VNON, VNOVAL, VREG, VSOCK, VTEXT, VWRITE, Vattr,
    Vnode, iftovt, makeimode, vn_lock,
};
use crate::fs::vnops::{RdwrBuf, vn_rdwr};
use crate::fs::vops::{
    Cn, Cred, IoBuf, Vnops, vop_abortop, vop_access, vop_bmap, vop_bwrite, vop_read, vop_remove,
    vop_strategy, vop_unlock,
};
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::SECURELEVEL;
use crate::tc::getnanotime;
use crate::uio::{Uio, UioRw};
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

/// `mastertemplate`: a virgin directory (no blushing please).
pub const MASTERTEMPLATE: Dirtemplate = Dirtemplate {
    dot_ino: 0,
    dot_reclen: 12,
    dot_type: DT_DIR,
    dot_namlen: 1,
    dot_name: *b".\0\0\0",
    dotdot_ino: 0,
    dotdot_reclen: DIRBLKSIZ as i16 - 12,
    dotdot_type: DT_DIR,
    dotdot_namlen: 2,
    dotdot_name: *b"..\0\0",
};

/// `ufsread_filtops`'s filter: the bytes past the file offset.
pub struct UfsRead;

/// `ufswrite_filtops`'s filter: a file is always writable.
pub struct UfsWrite;

/// `ufsvnode_filtops`'s filter: the vnode events (`NOTE_*`) the user asked for.
pub struct UfsVnode;

/// The tables of the three filters (`kn_fop`'s targets), made at compile time.
struct UfsFiltops<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> UfsFiltops<G> {
    /// `ufsread_filtops`.
    const READ: Filterops<G> = Filterops::of::<UfsRead>();
    /// `ufswrite_filtops`.
    const WRITE: Filterops<G> = Filterops::of::<UfsWrite>();
    /// `ufsvnode_filtops`.
    const VNODE: Filterops<G> = Filterops::of::<UfsVnode>();
}

/// `ffs_vops`: the operations of a file, directory or symbolic link on a UFS file system
/// whose own operations are `F`'s.
pub struct UfsVnops<G: ProcGlue, F: UfsFs<G>> {
    /// The file system's own operations.
    fs: &'static F,
    /// The glue.
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue, F: UfsFs<G>> UfsVnops<G, F> {
    /// The table over `fs`.
    pub const fn new(fs: &'static F) -> Self {
        Self {
            fs,
            _g: PhantomData,
        }
    }
}

/// `ffs_specvops`: the operations of a device special file on a UFS file system whose own
/// operations are `F`'s.
pub struct UfsSpecVnops<G: ProcGlue, F: UfsFs<G>> {
    /// The file system's own operations.
    fs: &'static F,
    /// The glue.
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue, F: UfsFs<G>> UfsSpecVnops<G, F> {
    /// The table over `fs`.
    pub const fn new(fs: &'static F) -> Self {
        Self {
            fs,
            _g: PhantomData,
        }
    }
}

/// The three operations of `ffs_vops` and `ffs_specvops` that are the file system's own
/// (`ffs_read`, `ffs_write`, `ffs_fsync`): the residue of the C's table that the UFS layer
/// does not provide (decision 46's written reason; `bsd`'s ffs implements it until N5.5b,
/// `bridges.toml`). `ffs_reclaim`, ffs's fourth, needs nothing of ffs's types and is the
/// tables' own `reclaim`.
pub trait UfsFs<G: ProcGlue>: Sync + 'static {
    /// `vop_read` of a file, directory or symbolic link.
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno>;
    /// `vop_write`.
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno>;
    /// `vop_fsync`.
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        cred: Cred<'_>,
        waitfor: i32,
        p: &Thread<G>,
    ) -> Result<(), Errno>;
}

/// Whether the vnode's file system is mounted read-only.
fn rdonly<G: ProcGlue>(vp: &Vnode<G>) -> bool {
    vmount(vp).flag() & MNT_RDONLY != 0
}

/// `*cred` of a credential the C dereferences: a real one (`NOCRED`/`FSCRED` panic).
pub fn ucred(cred: Cred<'_>) -> &Ucred {
    match cred.ucred() {
        Some(c) => c,
        None => kpanic(format_args!(
            "ufs: {} credential",
            if cred.is_nocred() {
                "missing"
            } else if cred.is_fscred() {
                "kernel"
            } else {
                "NULL"
            }
        )),
    }
}

/// `pool_put(&namei_pool, cnp->cn_pnbuf)`: give back the pathname buffer.
pub fn pnbuf_free<G: ProcGlue>(cnp: &mut Componentname<'_, G>) {
    drop(cnp.take_pnbuf());
}

/// `ufs_itimes`: update the times in the inode.
pub fn ufs_itimes<G: ProcGlue>(vp: &'static Vnode<G>) {
    itimes(vp, &vtoi(vp));
}

/// [`ufs_itimes`] of the inode `ip` of `vp` a caller holds already (`ufs_getattr`,
/// `ufs_setattr`: one handle per operation, as the C's one `VTOI`).
fn itimes<G: ProcGlue>(vp: &'static Vnode<G>, ip: &Inode<G>) {
    if ip.i_flag.get() & (IN_ACCESS | IN_CHANGE | IN_UPDATE) == 0 {
        return;
    }

    if !rdonly(vp) {
        if ip.fstype() == UM_EXT2FS {
            <G::FileGlue as FileGlue>::ext2fs_itimes(ip);
            // goto out
            ip.clr_flag(IN_ACCESS | IN_CHANGE | IN_UPDATE);
            return;
        }

        if vp.vtype() == VBLK || vp.vtype() == VCHR {
            ip.set_flag(IN_LAZYMOD);
        } else {
            ip.set_flag(IN_MODIFIED);
        }

        let ts = getnanotime();
        if ip.i_flag.get() & IN_ACCESS != 0 {
            ip.dip_set_atime(ts.tv_sec);
            ip.dip_set_atimensec(ts.tv_nsec as i32);
        }
        if ip.i_flag.get() & IN_UPDATE != 0 {
            ip.dip_set_mtime(ts.tv_sec);
            ip.dip_set_mtimensec(ts.tv_nsec as i32);
        }
        if ip.i_flag.get() & IN_CHANGE != 0 {
            ip.dip_set_ctime(ts.tv_sec);
            ip.dip_set_ctimensec(ts.tv_nsec as i32);
            ip.i_modrev.set(ip.i_modrev.get() + 1);
        }
    }

    // out:
    ip.clr_flag(IN_ACCESS | IN_CHANGE | IN_UPDATE);
}

/// `ufs_create` (`vop_create`): create a regular file.
pub fn ufs_create<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    let error = ufs_makeinode(makeimode(vap.va_type, vap.va_mode), dvp, vpp, cnp);
    if error.is_ok() {
        VN_KNOTE(dvp, NOTE_WRITE);
    }
    error
}

/// `ufs_mknod` (`vop_mknod`): make a device special file (or a fifo, a socket's node).
pub fn ufs_mknod<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    ufs_makeinode(makeimode(vap.va_type, vap.va_mode), dvp, vpp, cnp)?;
    VN_KNOTE(dvp, NOTE_WRITE);
    let Some(vp) = *vpp else {
        kpanic(format_args!("ufs_mknod: no vnode"));
    };
    let ip = vtoi(vp);
    ip.set_flag(IN_ACCESS | IN_CHANGE | IN_UPDATE);
    if vap.va_rdev != VNOVAL as Dev {
        // Want to be able to use this to make badblock inodes, so don't truncate the dev
        // number.
        ip.dip_set_rdev(i64::from(vap.va_rdev));
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

/// `ufs_open` (`vop_open`): nothing to do but check the append-only flag.
pub fn ufs_open<G: ProcGlue>(vp: &'static Vnode<G>, mode: i32) -> Result<(), Errno> {
    let ip = vtoi(vp);

    // Files marked append-only must be opened for appending.
    if ip.dip_flags() & APPEND != 0 && mode & (FWRITE | O_APPEND) == FWRITE {
        return Err(Errno::EPERM);
    }

    if mode & O_TRUNC != 0 {
        ip.set_flag(IN_CHANGE | IN_UPDATE);
    }

    Ok(())
}

/// `ufs_close` (`vop_close`): update the times on the inode.
pub fn ufs_close<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if vp.usecount() > 1 {
        ufs_itimes(vp);
    }
    Ok(())
}

/// `ufs_access` (`vop_access`).
pub fn ufs_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);

    // Disallow write attempts on read-only file systems; unless the file is a socket, fifo,
    // or a block or character device resident on the file system.
    if mode & VWRITE != 0 {
        match vp.vtype() {
            VDIR | VLNK | VREG => {
                if rdonly(vp) {
                    return Err(Errno::EROFS);
                }
                getinoquota(&ip)?;
            }
            VBAD | VBLK | VCHR | VSOCK | VFIFO | VNON => {}
        }
    }

    // If immutable bit set, nobody gets to write it.
    if mode & VWRITE != 0 && ip.dip_flags() & IMMUTABLE != 0 {
        return Err(Errno::EPERM);
    }

    if vnoperm(vp) {
        // For VEXEC, at least one of the execute bits must be set.
        if mode & VEXEC != 0
            && vp.vtype() != VDIR
            && ip.dip_mode() & (S_IXUSR | S_IXGRP | S_IXOTH) == 0
        {
            return Err(Errno::EACCES);
        }
        return Ok(());
    }

    vaccess(
        vp.vtype(),
        ip.dip_mode(),
        ip.dip_uid(),
        ip.dip_gid(),
        mode,
        ucred(cred),
    )
}

/// `ufs_getattr` (`vop_getattr`): copy from inode table.
pub fn ufs_getattr<G: ProcGlue>(vp: &'static Vnode<G>, vap: &mut Vattr) -> Result<(), Errno> {
    let ip = vtoi(vp);

    itimes(vp, &ip);

    // Copy from inode table.
    vap.va_fsid = i64::from(ip.i_dev);
    vap.va_fileid = u64::from(ip.i_number);
    vap.va_mode = ip.dip_mode() & !IFMT;
    vap.va_nlink = ip.i_effnlink.get() as Nlink;
    vap.va_uid = ip.dip_uid();
    vap.va_gid = ip.dip_gid();
    vap.va_rdev = ip.dip_rdev() as Dev;
    vap.va_size = ip.dip_size();
    vap.va_atime = Timespec::new(ip.dip_atime(), i64::from(ip.dip_atimensec()));
    vap.va_mtime = Timespec::new(ip.dip_mtime(), i64::from(ip.dip_mtimensec()));
    vap.va_ctime = Timespec::new(ip.dip_ctime(), i64::from(ip.dip_ctimensec()));
    vap.va_flags = u64::from(ip.dip_flags());
    vap.va_gen = u64::from(ip.dip_gen());
    // this doesn't belong here
    vap.va_blocksize = match vp.vtype() {
        VBLK => PAGE_SIZE as i64, // BLKDEV_IOSIZE
        VCHR => crate::vm::pager::MAXBSIZE as i64,
        _ => i64::from(vmount(vp).iosize()),
    };
    vap.va_bytes = (ip.dip_blocks() as u64) << crate::fs::bio::DEV_BSHIFT;
    vap.va_type = vp.vtype();
    vap.va_filerev = ip.i_modrev.get();
    Ok(())
}

/// `ufs_setattr` (`vop_setattr`): set attribute vnode op. called from several syscalls.
pub fn ufs_setattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let mut hint = NOTE_ATTRIB;

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
        if c.cr_uid.get() != ip.dip_uid() && !vnoperm(vp) {
            suser_ucred(c)?;
        }
        if c.cr_uid.get() == 0 || vnoperm(vp) {
            if ip.dip_flags() & (SF_IMMUTABLE | SF_APPEND) != 0
                && SECURELEVEL.load(Ordering::Relaxed) > 0
            {
                return Err(Errno::EPERM);
            }
            ip.dip_set_flags(vap.va_flags as u32);
        } else {
            if ip.dip_flags() & (SF_IMMUTABLE | SF_APPEND) != 0
                || u64::from(vap.va_flags as u32 & UF_SETTABLE) != vap.va_flags
            {
                return Err(Errno::EPERM);
            }
            ip.dip_set_flags(ip.dip_flags() & SF_SETTABLE);
            ip.dip_set_flags(ip.dip_flags() | (vap.va_flags as u32 & UF_SETTABLE));
        }
        ip.set_flag(IN_CHANGE);
        if vap.va_flags as u32 & (IMMUTABLE | APPEND) != 0 {
            return Ok(());
        }
    }
    if ip.dip_flags() & (IMMUTABLE | APPEND) != 0 {
        return Err(Errno::EPERM);
    }
    // Go through the fields and update if not VNOVAL.
    if vap.va_uid != VNOVAL as Uid || vap.va_gid != VNOVAL as Gid {
        if rdonly(vp) {
            return Err(Errno::EROFS);
        }
        ufs_chown(vp, &ip, vap.va_uid, vap.va_gid, cred)?;
    }
    if vap.va_size != VNOVAL as u64 {
        let oldsize = ip.dip_size();
        // Disallow write attempts on read-only file systems; unless the file is a socket,
        // fifo, or a block or character device resident on the file system.
        match vp.vtype() {
            VDIR => return Err(Errno::EISDIR),
            VLNK | VREG if rdonly(vp) => return Err(Errno::EROFS),
            _ => {}
        }
        ip.vtbl().truncate(&ip, vap.va_size as i64, 0, cred)?;
        if vap.va_size < oldsize {
            hint |= NOTE_TRUNCATE;
        }
    }
    if vap.va_vaflags & VA_UTIMES_CHANGE != 0
        || vap.va_atime.tv_nsec != i64::from(VNOVAL)
        || vap.va_mtime.tv_nsec != i64::from(VNOVAL)
    {
        if rdonly(vp) {
            return Err(Errno::EROFS);
        }
        let c = ucred(cred);
        if c.cr_uid.get() != ip.dip_uid()
            && !vnoperm(vp)
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
        itimes(vp, &ip);
        if vap.va_mtime.tv_nsec != i64::from(VNOVAL) {
            ip.dip_set_mtime(vap.va_mtime.tv_sec);
            ip.dip_set_mtimensec(vap.va_mtime.tv_nsec as i32);
        }
        if vap.va_atime.tv_nsec != i64::from(VNOVAL) {
            ip.dip_set_atime(vap.va_atime.tv_sec);
            ip.dip_set_atimensec(vap.va_atime.tv_nsec as i32);
        }
        ip.vtbl().update(&ip, 0)?;
    }
    let mut error = Ok(());
    if vap.va_mode != VNOVAL as Mode {
        if rdonly(vp) {
            return Err(Errno::EROFS);
        }
        error = ufs_chmod(vp, &ip, vap.va_mode as i32, cred);
    }
    VN_KNOTE(vp, hint);
    error
}

/// `ufs_chmod`: change the mode on a file. Inode must be locked before calling (`ip` is
/// `vp`'s, the caller's handle).
pub fn ufs_chmod<G: ProcGlue>(
    vp: &'static Vnode<G>,
    ip: &Inode<G>,
    mode: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let c = ucred(cred);
    let mode = mode as Mode;

    if c.cr_uid.get() != ip.dip_uid() && !vnoperm(vp) {
        suser_ucred(c)?;
    }
    if c.cr_uid.get() != 0 && !vnoperm(vp) {
        if vp.vtype() != VDIR && mode & S_ISTXT != 0 {
            return Err(Errno::EFTYPE);
        }
        if !groupmember(ip.dip_gid(), c) && mode & ISGID != 0 {
            return Err(Errno::EPERM);
        }
    }
    ip.dip_set_mode(ip.dip_mode() & !ALLPERMS);
    ip.dip_set_mode(ip.dip_mode() | (mode & ALLPERMS));
    ip.set_flag(IN_CHANGE);
    if vp.flag() & VTEXT != 0 && ip.dip_mode() & S_ISTXT == 0 {
        let _ = uvm_vnp_uncache(vp);
    }
    Ok(())
}

/// `ufs_chown`: perform chown operation on inode ip; inode must be locked prior to call
/// (`ip` is `vp`'s, the caller's handle).
pub fn ufs_chown<G: ProcGlue>(
    vp: &'static Vnode<G>,
    ip: &Inode<G>,
    uid: Uid,
    gid: Gid,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let c = ucred(cred);
    let mut quota_flags: UfsQuotaFlags = 0;

    let uid = if uid == VNOVAL as Uid {
        ip.dip_uid()
    } else {
        uid
    };
    let gid = if gid == VNOVAL as Gid {
        ip.dip_gid()
    } else {
        gid
    };
    // If we don't own the file, are trying to change the owner of the file, or are not a
    // member of the target group, the caller must be superuser or the call fails.
    if (c.cr_uid.get() != ip.dip_uid()
        || uid != ip.dip_uid()
        || (gid != ip.dip_gid() && !groupmember(gid, c)))
        && !vnoperm(vp)
    {
        suser_ucred(c)?;
    }
    let ogid = ip.dip_gid();
    let ouid = ip.dip_uid();
    let change = ip.dip_blocks();

    if ouid == uid {
        quota_flags |= UFS_QUOTA_NOUID;
    }

    if ogid == gid {
        quota_flags |= UFS_QUOTA_NOGID;
    }

    getinoquota(ip)?;
    let _ = ufs_quota_free_blocks2(ip, change, cred, quota_flags);
    let _ = ufs_quota_free_inode2(ip, cred, quota_flags);
    let _ = ufs_quota_delete(ip);

    ip.dip_set_gid(gid);
    ip.dip_set_uid(uid);

    let error: Result<(), Errno> = 'error: {
        if let Err(e) = getinoquota(ip) {
            break 'error Err(e);
        }

        if let Err(e) = ufs_quota_alloc_blocks2(ip, change, cred, quota_flags) {
            break 'error Err(e);
        }

        if let Err(e) = ufs_quota_alloc_inode2(ip, cred, quota_flags) {
            let _ = ufs_quota_free_blocks2(ip, change, cred, quota_flags);
            break 'error Err(e);
        }

        if getinoquota(ip).is_err() {
            kpanic(format_args!("chown: lost quota"));
        }

        if ouid != uid || ogid != gid {
            ip.set_flag(IN_CHANGE);
        }
        if !vnoperm(vp) {
            if ouid != uid && c.cr_uid.get() != 0 {
                ip.dip_set_mode(ip.dip_mode() & !ISUID);
            }
            if ogid != gid && c.cr_uid.get() != 0 {
                ip.dip_set_mode(ip.dip_mode() & !ISGID);
            }
        }
        return Ok(());
    };

    // error:
    let _ = ufs_quota_delete(ip);

    ip.dip_set_gid(ogid);
    ip.dip_set_uid(ouid);

    if getinoquota(ip).is_ok() {
        let _ = ufs_quota_alloc_blocks2(ip, change, cred, quota_flags | UFS_QUOTA_FORCE);
        let _ = ufs_quota_alloc_inode2(ip, cred, quota_flags | UFS_QUOTA_FORCE);
        let _ = getinoquota(ip);
    }
    error
}

/// `ufs_ioctl` (`vop_ioctl`): no ioctls on UFS files.
pub fn ufs_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `ufs_remove` (`vop_remove`): remove the entry of a file that is not a directory.
pub fn ufs_remove<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    if vp.vtype() == VDIR
        || ip.dip_flags() & (IMMUTABLE | APPEND) != 0
        || vtoi(dvp).dip_flags() & APPEND != 0
    {
        return Err(Errno::EPERM);
    }
    let error = ufs_dirremove(dvp, Some(&ip), cnp.cn_flags, 0);
    VN_KNOTE(vp, NOTE_DELETE);
    VN_KNOTE(dvp, NOTE_WRITE);
    error
}

/// `ufs_link` (`vop_link`): link vnode call.
pub fn ufs_link<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
        kpanic(format_args!("ufs_link: no name"));
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
            if ip.dip_nlink() as Nlink >= LINK_MAX {
                let _ = vop_abortop(dvp, cnp);
                break 'out1 Err(Errno::EMLINK);
            }
            if ip.dip_flags() & (IMMUTABLE | APPEND) != 0 {
                let _ = vop_abortop(dvp, cnp);
                break 'out1 Err(Errno::EPERM);
            }
            ip.i_effnlink.set(ip.i_effnlink.get() + 1);
            ip.dip_set_nlink(ip.dip_nlink() + 1);
            ip.set_flag(IN_CHANGE);
            let mut error = ip.vtbl().update(&ip, 1);
            if error.is_ok() {
                let mut newdir = Direct::new();
                ufs_makedirentry(&ip, cnp, &mut newdir);
                error = ufs_direnter(dvp, Some(vp), &mut newdir, cnp);
            }
            if error.is_err() {
                ip.i_effnlink.set(ip.i_effnlink.get() - 1);
                ip.dip_set_nlink(ip.dip_nlink() - 1);
                ip.set_flag(IN_CHANGE);
            }
            pnbuf_free(cnp);
            VN_KNOTE(vp, NOTE_LINK);
            VN_KNOTE(dvp, NOTE_WRITE);
            error
        };
        // out1:
        if !core::ptr::eq(dvp, vp) {
            let _ = vop_unlock(vp);
        }
        error
    };
    // out2:
    vput(dvp);
    error
}

/// How `ufs_rename` leaves: the C's `bad:` and `out:` labels.
enum RenameExit {
    /// `goto bad`: release the target directory (and target) first.
    Bad(Errno),
    /// `goto out`.
    Out(Errno),
}

/// The four vnodes and two names of a rename, for `abortit:`.
struct Abort<'r, 'c, G: ProcGlue> {
    /// `tdvp`.
    tdvp: &'static Vnode<G>,
    /// `tvp`.
    tvp: Option<&'static Vnode<G>>,
    /// `tcnp`.
    tcnp: &'r mut Cn<'c, G>,
    /// `fdvp`.
    fdvp: &'static Vnode<G>,
    /// `fvp`.
    fvp: &'static Vnode<G>,
}

/// `abortit:` of `ufs_rename`: abort both lookups and release every vnode.
fn rename_abortit<G: ProcGlue>(
    error: Errno,
    a: Abort<'_, '_, G>,
    fcnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let Abort {
        tdvp,
        tvp,
        tcnp,
        fdvp,
        fvp,
    } = a;
    let _ = vop_abortop(tdvp, tcnp);
    if tvp.is_some_and(|t| core::ptr::eq(t, tdvp)) {
        vrele(tdvp);
    } else {
        vput(tdvp);
    }
    if let Some(tvp) = tvp {
        vput(tvp);
    }
    let _ = vop_abortop(fdvp, fcnp);
    vrele(fdvp);
    vrele(fvp);
    Err(error)
}

/// `ufs_rename` (`vop_rename`): rename system call.
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
pub fn ufs_rename<G: ProcGlue>(
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
    let mut oldparent = 0;
    let mut newparent = 0;
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
        kpanic(format_args!("ufs_rename: no name"));
    }
    // Check for cross-device rename.
    let fmp = fvp.mount().map(core::ptr::from_ref);
    if fmp != tdvp.mount().map(core::ptr::from_ref)
        || tvp.is_some_and(|t| fmp != t.mount().map(core::ptr::from_ref))
    {
        abortit!(Errno::EXDEV);
    }

    if let Some(t) = tvp
        && (vtoi(t).dip_flags() & (IMMUTABLE | APPEND) != 0 || vtoi(tdvp).dip_flags() & APPEND != 0)
    {
        abortit!(Errno::EPERM);
    }

    // Check if just deleting a link name or if we've lost a race. If another process
    // completes the same rename after we've looked up the source and have blocked looking up
    // the target, then the source and target inodes may be identical now although the names
    // were never linked.
    if let Some(t) = tvp
        && core::ptr::eq(fvp, t)
    {
        if fvp.vtype() == VDIR {
            // Linked directories are impossible, so we must have lost the race. Pretend
            // that the rename completed before the lookup.
            abortit!(Errno::ENOENT);
        }

        // Release destination completely.
        let _ = vop_abortop(tdvp, tcnp);
        vput(tdvp);
        vput(t);

        // Delete source. There is another race now that everything is unlocked, but this
        // doesn't cause any new complications. relookup() may find a file that is unrelated
        // to the original one, or it may fail. Too bad.
        vrele(fvp);
        fcnp.cn_flags &= !MODMASK;
        fcnp.cn_flags |= LOCKPARENT | LOCKLEAF;
        if fcnp.cn_flags & SAVESTART == 0 {
            kpanic(format_args!("ufs_rename: lost from startdir"));
        }
        fcnp.cn_nameiop = DELETE;
        let mut nfvp = None;
        vfs_relookup(fdvp, &mut nfvp, fcnp)?; // relookup did vrele()
        vrele(fdvp);
        // A successful DELETE lookup of the last component returns its vnode.
        let Some(nfvp) = nfvp else {
            // A DELETE lookup of the last component that succeeds returns its vnode
            // (ufs_lookup's DELETE path), so the C's VOP_REMOVE has one.
            kpanic(format_args!(
                "ufs_rename: relookup of the source found no vnode"
            ));
        };
        return vop_remove(fdvp, nfvp, fcnp);
    }

    if let Err(e) = vn_lock(fvp, LK_EXCLUSIVE) {
        abortit!(e);
    }

    // fvp, tdvp, tvp now locked
    let mut dp = vtoi(fdvp);
    let ip = vtoi(fvp);
    if ip.dip_nlink() as Nlink >= LINK_MAX {
        let _ = vop_unlock(fvp);
        abortit!(Errno::EMLINK);
    }
    if ip.dip_flags() & (IMMUTABLE | APPEND) != 0 || dp.dip_flags() & APPEND != 0 {
        let _ = vop_unlock(fvp);
        abortit!(Errno::EPERM);
    }
    if ip.dip_mode() & IFMT == IFDIR {
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
    VN_KNOTE(fdvp, NOTE_WRITE); // XXX right place?

    // When the target exists, both the directory and target vnodes are returned locked.
    dp = vtoi(tdvp);
    let mut xp: Option<InodeRef<G>> = tvp.map(vtoi);

    let exit: Result<(), RenameExit> = 'body: {
        // 1) Bump link count while we're moving stuff around. If we crash somewhere before
        //    completing our work, the link count may be wrong, but correctable.
        ip.i_effnlink.set(ip.i_effnlink.get() + 1);
        ip.dip_set_nlink(ip.dip_nlink() + 1);
        ip.set_flag(IN_CHANGE);
        if let Err(e) = ip.vtbl().update(&ip, 1) {
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

        // tdvp and tvp locked
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
            // Compensate for the reference ufs_checkpath() loses.
            vref(tdvp);
            // Only tdvp is locked
            if let Err(e) = ufs_checkpath(&ip, &dp, tcnp.cn_cred) {
                vrele(tdvp);
                break 'body Err(RenameExit::Out(e));
            }
            if tcnp.cn_flags & SAVESTART == 0 {
                kpanic(format_args!("ufs_rename: lost to startdir"));
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
                    if dp.dip_nlink() as Nlink >= LINK_MAX {
                        break 'body Err(RenameExit::Bad(Errno::EMLINK));
                    }
                    dp.i_effnlink.set(dp.i_effnlink.get() + 1);
                    dp.dip_set_nlink(dp.dip_nlink() + 1);
                    dp.set_flag(IN_CHANGE);
                    if let Err(e) = dp.vtbl().update(&dp, 1) {
                        dp.i_effnlink.set(dp.i_effnlink.get() - 1);
                        dp.dip_set_nlink(dp.dip_nlink() - 1);
                        dp.set_flag(IN_CHANGE);
                        break 'body Err(RenameExit::Bad(e));
                    }
                }
                let mut newdir = Direct::new();
                ufs_makedirentry(&ip, tcnp, &mut newdir);
                if let Err(e) = ufs_direnter(tdvp, None, &mut newdir, tcnp) {
                    if doingdirectory && newparent != 0 {
                        dp.i_effnlink.set(dp.i_effnlink.get() - 1);
                        dp.dip_set_nlink(dp.dip_nlink() - 1);
                        dp.set_flag(IN_CHANGE);
                        let _ = dp.vtbl().update(&dp, 1);
                    }
                    break 'body Err(RenameExit::Bad(e));
                }
                VN_KNOTE(tdvp, NOTE_WRITE);
                vput(tdvp);
            }
            Some(x) => {
                if x.i_dev != dp.i_dev || x.i_dev != ip.i_dev {
                    kpanic(format_args!("rename: EXDEV"));
                }
                // Short circuit rename(foo, foo).
                if x.i_number == ip.i_number {
                    kpanic(format_args!("ufs_rename: same file"));
                }
                // If the parent directory is "sticky", then the user must own the parent
                // directory, or the destination of the rename, otherwise the destination may
                // not be changed (except by root). This implements append-only directories.
                let tuid = ucred(tcnp.cn_cred).cr_uid.get();
                if dp.dip_mode() & S_ISTXT != 0
                    && tuid != 0
                    && tuid != dp.dip_uid()
                    && x.dip_uid() != tuid
                    && !vnoperm(tdvp)
                {
                    break 'body Err(RenameExit::Bad(Errno::EPERM));
                }
                // Target must be empty if a directory and have no links to it. Also, ensure
                // source and target are compatible (both directories, or both not
                // directories).
                if x.dip_mode() & IFMT == IFDIR {
                    if x.i_effnlink.get() > 2 || !ufs_dirempty(x, dp.i_number, tcnp.cn_cred) {
                        break 'body Err(RenameExit::Bad(Errno::ENOTEMPTY));
                    }
                    if !doingdirectory {
                        break 'body Err(RenameExit::Bad(Errno::ENOTDIR));
                    }
                    cache_purge(tdvp);
                } else if doingdirectory {
                    break 'body Err(RenameExit::Bad(Errno::EISDIR));
                }

                let isrmdir = if doingdirectory && newparent != 0 {
                    newparent as i32
                } else {
                    i32::from(doingdirectory)
                };
                if let Err(e) = ufs_dirrewrite(&dp, x, ip.i_number, iftodt(ip.dip_mode()), isrmdir)
                {
                    break 'body Err(RenameExit::Bad(e));
                }
                if doingdirectory {
                    if newparent == 0 {
                        dp.i_effnlink.set(dp.i_effnlink.get() - 1);
                    }
                    x.i_effnlink.set(x.i_effnlink.get() - 1);
                }
                if doingdirectory {
                    // Truncate inode. The only stuff left in the directory is "." and "..".
                    // The "." reference is inconsequential since we are quashing it. We have
                    // removed the "." reference and the reference in the parent directory,
                    // but there may be other hard links. The soft dependency code will
                    // arrange to do these operations after the parent directory entry has
                    // been deleted on disk, so when running with that code we avoid doing
                    // them now.
                    if newparent == 0 {
                        dp.dip_set_nlink(dp.dip_nlink() - 1);
                        dp.set_flag(IN_CHANGE);
                    }

                    x.dip_set_nlink(x.dip_nlink() - 1);
                    x.set_flag(IN_CHANGE);
                    if let Err(e) = x.vtbl().truncate(x, 0, IO_SYNC, tcnp.cn_cred) {
                        break 'body Err(RenameExit::Bad(e));
                    }
                }
                VN_KNOTE(tdvp, NOTE_WRITE);
                vput(tdvp);
                if let Some(t) = tvp {
                    VN_KNOTE(t, NOTE_DELETE);
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
            return rename_out(e, fdvp, fvp, &ip, doingdirectory);
        }
        Err(RenameExit::Out(e)) => {
            return rename_out(e, fdvp, fvp, &ip, doingdirectory);
        }
    }
    drop(xp);
    drop(dp);

    // 3) Unlink the source.
    fcnp.cn_flags &= !MODMASK;
    fcnp.cn_flags |= LOCKPARENT | LOCKLEAF;
    if fcnp.cn_flags & SAVESTART == 0 {
        kpanic(format_args!("ufs_rename: lost from startdir"));
    }
    let mut nfvp = None;
    if let Err(e) = vfs_relookup(fdvp, &mut nfvp, fcnp) {
        vrele(a_fvp);
        return Err(e);
    }
    vrele(fdvp);
    let Some(fvp) = nfvp else {
        // From name has disappeared.
        if doingdirectory {
            kpanic(format_args!("ufs_rename: lost dir entry"));
        }
        vrele(a_fvp);
        return Ok(());
    };

    let xp = vtoi(fvp);
    let dp = vtoi(fdvp);

    // Ensure that the directory entry still exists and has not changed while the new name
    // has been entered. If the source is a file then the entry may have been unlinked or
    // renamed. In either case there is no further work to be done. If the source is a
    // directory then it cannot have been rmdir'ed; the IN_RENAME flag ensures that it cannot
    // be moved by another rename or removed by a rmdir.
    let mut error = Ok(());
    if InodeRef::<G>::ptr_eq(&xp, &ip) {
        // If the source is a directory with a new parent, the link count of the old parent
        // directory must be decremented and ".." set to point to the new parent.
        if doingdirectory && newparent != 0 {
            xp.i_offset.set(i32::from(MASTERTEMPLATE.dot_reclen));
            let _ = ufs_dirrewrite(&xp, &dp, newparent, DT_DIR, 0);
            cache_purge(fdvp);
        }
        error = ufs_dirremove(fdvp, Some(&xp), fcnp.cn_flags, 0);
        xp.clr_flag(IN_RENAME);
    } else if doingdirectory {
        kpanic(format_args!("ufs_rename: lost dir entry"));
    }
    VN_KNOTE(fvp, NOTE_RENAME);
    vput(fdvp);
    vput(fvp);
    vrele(a_fvp);
    error
}

/// `out:` of `ufs_rename`: release the source directory and undo the link count bump.
fn rename_out<G: ProcGlue>(
    error: Errno,
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    ip: &Inode<G>,
    doingdirectory: bool,
) -> Result<(), Errno> {
    vrele(fdvp);
    if doingdirectory {
        ip.clr_flag(IN_RENAME);
    }
    if vn_lock(fvp, LK_EXCLUSIVE).is_ok() {
        ip.i_effnlink.set(ip.i_effnlink.get() - 1);
        ip.dip_set_nlink(ip.dip_nlink() - 1);
        ip.set_flag(IN_CHANGE);
        ip.clr_flag(IN_RENAME);
        vput(fvp);
    } else {
        vrele(fvp);
    }
    Err(error)
}

/// `ufs_mkdir` (`vop_mkdir`): mkdir system call.
pub fn ufs_mkdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
        kpanic(format_args!("ufs_mkdir: no name"));
    }
    let dp = vtoi(dvp);
    let error: Result<(), Errno> = 'out: {
        if dp.dip_nlink() as Nlink >= LINK_MAX {
            break 'out Err(Errno::EMLINK);
        }
        let dmode = (vap.va_mode & 0o777) | IFDIR;
        // Must simulate part of ufs_makeinode here to acquire the inode, but not have it
        // entered in the parent directory. The entry is made later after writing "." and
        // ".." entries.
        let tvp = match dp.vtbl().inode_alloc(&dp, dmode, cnp.cn_cred) {
            Ok(tvp) => tvp,
            Err(e) => break 'out Err(e),
        };

        let ip = vtoi(tvp);

        ip.dip_set_uid(ucred(cnp.cn_cred).cr_uid.get());
        ip.dip_set_gid(dp.dip_gid());

        if let Err(e) = getinoquota(&ip).and_then(|()| ufs_quota_alloc_inode(&ip, cnp.cn_cred)) {
            pnbuf_free(cnp);
            let _ = ip.vtbl().inode_free(&ip, ip.i_number, dmode);
            vput(tvp);
            vput(dvp);
            return Err(e);
        }

        ip.set_flag(IN_ACCESS | IN_CHANGE | IN_UPDATE);
        ip.dip_set_mode(dmode);
        tvp.set_vtype(VDIR); // Rest init'd in getnewvnode().
        ip.i_effnlink.set(2);
        ip.dip_set_nlink(2);

        let error: Result<(), Errno> = 'bad: {
            // Bump link count in parent directory to reflect work done below. Should be done
            // before reference is create so cleanup is possible if we crash.
            dp.i_effnlink.set(dp.i_effnlink.get() + 1);
            dp.dip_set_nlink(dp.dip_nlink() + 1);
            dp.set_flag(IN_CHANGE);
            if let Err(e) = dp.vtbl().update(&dp, 1) {
                break 'bad Err(e);
            }

            // Initialize directory with "." and ".." from static template.
            let mut dirtemplate = MASTERTEMPLATE;
            dirtemplate.dot_ino = ip.i_number;
            dirtemplate.dotdot_ino = dp.i_number;

            let mut bp = match buf_alloc(&ip, 0, DIRBLKSIZ as i32, cnp.cn_cred, B_CLRBUF) {
                Ok(bp) => bp,
                Err(e) => break 'bad Err(e),
            };
            ip.dip_set_size(DIRBLKSIZ as u64);
            ip.set_flag(IN_CHANGE | IN_UPDATE);
            uvm_vnp_setsize(tvp, ip.dip_size() as i64);
            bp.data()[..Dirtemplate::SIZE].copy_from_slice(&dirtemplate.to_bytes());
            if let Err(e) = ip.vtbl().update(&ip, 1) {
                let _ = vop_bwrite::<G>(IoBuf::Token(bp));
                break 'bad Err(e);
            }

            // Directory set up, now install its entry in the parent directory.
            //
            // If we are not doing soft dependencies, then we must write out the buffer
            // containing the new directory body before entering the new name in the
            // parent. If we are doing soft dependencies, then the buffer containing the new
            // directory body will be passed to and released in the soft dependency code
            // after the code has attached an appropriate ordering dependency to the buffer
            // which ensures that the buffer is written before the new name is written in the
            // parent.
            if let Err(e) = vop_bwrite::<G>(IoBuf::Token(bp)) {
                break 'bad Err(e);
            }
            let mut newdir = Direct::new();
            ufs_makedirentry(&ip, cnp, &mut newdir);
            ufs_direnter(dvp, Some(tvp), &mut newdir, cnp)
        };

        // bad:
        if error.is_ok() {
            VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);
            *vpp = Some(tvp);
        } else {
            dp.i_effnlink.set(dp.i_effnlink.get() - 1);
            dp.dip_set_nlink(dp.dip_nlink() - 1);
            dp.set_flag(IN_CHANGE);
            // No need to do an explicit VOP_TRUNCATE here, vrele will do this for us because
            // we set the link count to 0.
            ip.i_effnlink.set(0);
            ip.dip_set_nlink(0);
            ip.set_flag(IN_CHANGE);
            drop(ip);
            vput(tvp);
        }
        error
    };
    // out:
    pnbuf_free(cnp);
    drop(dp);
    vput(dvp);

    error
}

/// `ufs_rmdir` (`vop_rmdir`): rmdir system call.
pub fn ufs_rmdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let dp = vtoi(dvp);
    // Do not remove a directory that is in the process of being renamed. Verify the
    // directory is empty (and valid). Rmdir ".." will not be valid since ".." will contain a
    // reference to the current directory and thus be non-empty.
    let error: Result<(), Errno> = 'out: {
        if ip.i_flag.get() & IN_RENAME != 0 {
            break 'out Err(Errno::EINVAL);
        }
        if ip.i_effnlink.get() != 2 || !ufs_dirempty(&ip, dp.i_number, cnp.cn_cred) {
            break 'out Err(Errno::ENOTEMPTY);
        }
        if dp.dip_flags() & APPEND != 0 || ip.dip_flags() & (IMMUTABLE | APPEND) != 0 {
            break 'out Err(Errno::EPERM);
        }
        // Delete reference to directory before purging inode. If we crash in between, the
        // directory will be reattached to lost+found,
        dp.i_effnlink.set(dp.i_effnlink.get() - 1);
        ip.i_effnlink.set(ip.i_effnlink.get() - 1);
        if let Err(e) = ufs_dirremove(dvp, Some(&ip), cnp.cn_flags, 1) {
            dp.i_effnlink.set(dp.i_effnlink.get() + 1);
            ip.i_effnlink.set(ip.i_effnlink.get() + 1);
            break 'out Err(e);
        }

        VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);
        cache_purge(dvp);
        // Truncate inode. The only stuff left in the directory is "." and "..". The "."
        // reference is inconsequential since we are quashing it.
        dp.dip_set_nlink(dp.dip_nlink() - 1);
        dp.set_flag(IN_CHANGE);
        ip.dip_set_nlink(ip.dip_nlink() - 1);
        ip.set_flag(IN_CHANGE);
        let error = ip.vtbl().truncate(
            &ip,
            0,
            if doingasync(vp) { 0 } else { IO_SYNC },
            cnp.cn_cred,
        );

        cache_purge(vp);
        // Kill any active hash; i_effnlink == 0, so it will not come back.
        #[cfg(feature = "ufs_dirhash")]
        if ip.i_dirhash.read().is_some() {
            super::ufs_dirhash::ufsdirhash_free(&ip);
        }
        error
    };
    // out:
    VN_KNOTE(vp, NOTE_DELETE);
    drop(dp);
    drop(ip);
    vput(dvp);
    vput(vp);
    error
}

/// `ufs_symlink` (`vop_symlink`): make a symbolic link.
pub fn ufs_symlink<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
    target: &[u8],
) -> Result<(), Errno> {
    if let Err(e) = ufs_makeinode(IFLNK | vap.va_mode, dvp, vpp, cnp) {
        vput(dvp);
        return Err(e);
    }
    VN_KNOTE(dvp, NOTE_WRITE);
    vput(dvp);
    let Some(vp) = *vpp else {
        kpanic(format_args!("ufs_symlink: no vnode"));
    };
    let ip = vtoi(vp);
    let len = target.len();
    let error = if len < ip.ump().um_maxsymlinklen.get() as usize {
        ip.with_shortlink(|s| s[..len].copy_from_slice(target));
        ip.dip_set_size(len as u64);
        ip.set_flag(IN_CHANGE | IN_UPDATE);
        Ok(())
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
    drop(ip);
    vput(vp);
    error
}

/// `ufs_readdir` (`vop_readdir`): vnode op for reading directories. This routine converts
/// the on-disk `struct direct` entries to the `struct dirent` entries expected by userland
/// and the rest of the kernel.
pub fn ufs_readdir<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    const NAME_OFF: usize = Dirent::NAME_OFFSET;
    const D_NAME: usize = Direct::NAME_OFFSET;
    let ip = vtoi(vp);
    // u: a struct dirent, rounded up to 8 bytes.
    let mut u = [0u8; size_of::<Dirent>().next_multiple_of(8)];

    if uio.uio_rw != UioRw::UIO_READ {
        return Err(Errno::EINVAL);
    }

    let count = uio.uio_resid;
    let entries = (uio.uio_offset as usize + count) & (DIRBLKSIZ - 1);

    // Make sure we don't return partial entries.
    if count <= entries {
        return Err(Errno::EINVAL);
    }

    // read from disk, stopping on a block boundary, max 64kB
    let mut readcnt = count.min(64 * 1024) as i64 - entries as i64;

    let mut off = uio.uio_offset;
    let mut done = false;
    let mut error: Result<(), Errno> = Ok(());

    while error.is_ok() && !done && readcnt > 0 {
        let bytesinfile = ip.dip_size() as i64 - off;
        if bytesinfile <= 0 {
            break;
        }
        let (bp, boff) = match bufatoff(&ip, off) {
            Ok(r) => r,
            Err(e) => {
                error = Err(e);
                break;
            }
        };
        let size = bp.buf().bcount() - bp.buf().resid() as i64 - boff as i64;
        if size <= 0 {
            brelse(bp);
            break;
        }

        let mut xfersize = readcnt;
        if size < xfersize {
            xfersize = size;
        }
        if bytesinfile < xfersize {
            xfersize = bytesinfile;
        }

        {
            let dirbuf = &bp.bytes()[boff..];
            let edp = xfersize as usize;
            // Convert and copy back the on-disk struct direct format to the user-space
            // struct dirent format, one entry at a time.
            let mut dp = 0usize;

            // While
            //  - we haven't failed to read or uiomove()
            //  - there's space in the read buf for the head of an entry
            //  - that entry has a valid d_reclen, and
            //  - there's space for the *entire* entry
            // then we're good to process this one.
            while error.is_ok()
                && dp + D_NAME < edp
                && usize::from(d_reclen(dirbuf, dp)) > D_NAME
                && dp + usize::from(d_reclen(dirbuf, dp)) <= edp
            {
                let reclen = usize::from(d_reclen(dirbuf, dp));
                let namlen = usize::from(d_namlen(dirbuf, dp));
                let d_reclen = (namlen + 1 + NAME_OFF).next_multiple_of(8);
                if d_reclen > uio.uio_resid {
                    done = true;
                    break;
                }
                off += reclen as i64;
                u[0..8].copy_from_slice(&u64::from(d_ino(dirbuf, dp)).to_ne_bytes());
                u[8..16].copy_from_slice(&off.to_ne_bytes());
                u[16..18].copy_from_slice(&(d_reclen as u16).to_ne_bytes());
                u[18] = d_type(dirbuf, dp);
                u[19] = namlen as u8;
                let name = d_name(dirbuf, dp, namlen);
                u[NAME_OFF..NAME_OFF + name.len()].copy_from_slice(name);
                u[NAME_OFF + name.len()..d_reclen].fill(0);

                if name.contains(&b'/') {
                    error = Err(Errno::EINVAL);
                    break;
                }

                error = <G::FileGlue as FileGlue>::uiomove(&mut u[..d_reclen], uio);
                dp += reclen;
            }

            // If there was room for an entry in what we read but its d_reclen is bogus, fail
            if dp + D_NAME < edp && usize::from(d_reclen(dirbuf, dp)) <= D_NAME {
                error = Err(Errno::EIO);
            }
        }

        brelse(bp);
        readcnt -= xfersize;
    }

    uio.uio_offset = off;
    *eofflag = i32::from(ip.dip_size() as i64 <= off);

    if vmount(vp).flag() & MNT_NOATIME == 0 || ip.i_flag.get() & (IN_CHANGE | IN_UPDATE) != 0 {
        ip.set_flag(IN_ACCESS);
    }

    error
}

/// `ufs_readlink` (`vop_readlink`): return target name of a symbolic link.
pub fn ufs_readlink<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);

    let isize = ip.dip_size();
    if isize < u64::from(ip.ump().um_maxsymlinklen.get()) {
        let mut link = [0u8; MAXSYMLINKLEN_UFS2];
        let n = isize as usize;
        ip.with_shortlink(|s| link[..n].copy_from_slice(&s[..n]));
        return <G::FileGlue as FileGlue>::uiomove(&mut link[..n], uio);
    }
    drop(ip);
    vop_read(vp, uio, 0, cred)
}

/// `ufs_lock` (`vop_lock`): lock an inode. If its already locked, set the WANT bit and
/// sleep.
pub fn ufs_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    vtoi(vp).i_lock.enter(flags & LK_RWFLAGS)
}

/// `ufs_unlock` (`vop_unlock`): unlock an inode. If WANT bit is on, wakeup.
pub fn ufs_unlock<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    vtoi(vp).i_lock.exit();
    Ok(())
}

/// `ufs_islocked` (`vop_islocked`): check for a locked inode.
pub fn ufs_islocked<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    vtoi(vp).i_lock.status()
}

/// `biodone(bp)` of the buffer of `VOP_STRATEGY` as it came: its token, or the token parked
/// in a lent buffer (the C's `splbio(); biodone(bp); splx(s)`: one section of the cache's
/// lock, which blocks the disk interrupt).
fn strategy_done(bp: IoBuf) {
    match bp {
        IoBuf::Token(t) => biodone(t),
        IoBuf::Lent(b) => {
            if !biodone_parked(b) {
                biodone(BufRef::foreign(b));
            }
        }
    }
}

/// `clrbuf(bp)` of the buffer of `VOP_STRATEGY` as it came: a lent buffer's parked token, or
/// for a lent buffer with none parked (a driver's own buffer) the foreign handle `bsd`'s
/// `clrbuf` takes, given back parked.
fn strategy_clrbuf(bp: &mut IoBuf) {
    match bp {
        IoBuf::Token(t) => t.clear(),
        IoBuf::Lent(b) => {
            let lent = *b;
            let mut t = lent.take_parked().unwrap_or_else(|| BufRef::foreign(lent));
            t.clear();
            crate::fs::buf::Buf::park(t);
        }
    }
}

/// `ufs_strategy` (`vop_strategy`): calculate the logical to physical mapping if not done
/// already, then call the device strategy routine.
pub fn ufs_strategy<G: ProcGlue>(vp: &'static Vnode<G>, mut bp: IoBuf) -> Result<(), Errno> {
    let ip = vtoi(vp);
    if vp.vtype() == VBLK || vp.vtype() == VCHR {
        kpanic(format_args!("ufs_strategy: spec"));
    }
    let b = bp.buf();
    if b.blkno() == b.lblkno() {
        let mut blkno: Daddr = b.blkno();
        let error = vop_bmap(vp, b.lblkno(), None, Some(&mut blkno), None);
        b.set_blkno(blkno);
        if let Err(e) = error {
            b.set_error(Some(e));
            b.set(B_ERROR);
            strategy_done(bp);
            return Err(e);
        }
        if blkno == -1 {
            strategy_clrbuf(&mut bp);
        }
    }
    if bp.buf().blkno() == -1 {
        strategy_done(bp);
        return Ok(());
    }
    let devvp = ip.i_devvp();
    bp.buf().set_dev(devvp.rdev());
    drop(ip);
    let _ = vop_strategy(devvp, bp);
    Ok(())
}

/// `ufs_print` (`vop_print`): print out the contents of an inode.
pub fn ufs_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(any(feature = "debug", feature = "diagnostic"))]
    {
        use hal::sys::types::{major, minor};
        let ip = vtoi(vp);

        crate::prf::printf(format_args!(
            "tag VT_UFS, ino {}, on dev {}, {}",
            ip.i_number,
            major(ip.i_dev),
            minor(ip.i_dev)
        ));
        crate::prf::printf(format_args!(
            " flags 0x{:x}, effnlink {}, nlink {}\n",
            ip.i_flag.get(),
            ip.i_effnlink.get(),
            ip.dip_nlink()
        ));
        crate::prf::printf(format_args!(
            "\tmode 0{:o}, owner {}, group {}, size {}",
            ip.dip_mode(),
            ip.dip_uid(),
            ip.dip_gid(),
            ip.dip_size()
        ));
        // FIFO: fifo_printinfo(vp) for a VFIFO (fifofs, not ported).
        crate::prf::printf(format_args!("\n"));
    }
    #[cfg(not(any(feature = "debug", feature = "diagnostic")))]
    let _ = vp;

    Ok(())
}

/// `ufsspec_read`: read wrapper for special devices: set access flag.
pub fn ufsspec_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    vtoi(vp).set_flag(IN_ACCESS);
    spec_read(vp, uio, ioflag)
}

/// `ufsspec_write`: write wrapper for special devices: set update and change flags.
pub fn ufsspec_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    vtoi(vp).set_flag(IN_CHANGE | IN_UPDATE);
    spec_write(vp, uio, ioflag)
}

/// `ufsspec_close`: close wrapper for special devices. Update the times on the inode then
/// do device close.
pub fn ufsspec_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    fflag: i32,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    if vp.usecount() > 1 {
        ufs_itimes(vp);
    }
    spec_close(vp, fflag, cred, p)
}

/// `ufs_pathconf` (`vop_pathconf`): return POSIX pathconf information applicable to ufs
/// filesystems.
pub fn ufs_pathconf<G: ProcGlue>(
    vp: &'static Vnode<G>,
    name: i32,
    retval: &mut Register,
) -> Result<(), Errno> {
    // `v_mount` is read only by the three names that need it, as the C reads it.
    let mp = || vmount(vp);
    *retval = match name {
        _PC_LINK_MAX => LINK_MAX as Register,
        _PC_NAME_MAX => NAME_MAX as Register,
        _PC_CHOWN_RESTRICTED | _PC_NO_TRUNC | _PC_2_SYMLINKS | _PC_TIMESTAMP_RESOLUTION => 1,
        _PC_ALLOC_SIZE_MIN => mp().bsize() as Register,
        _PC_FILESIZEBITS => 64,
        _PC_REC_INCR_XFER_SIZE | _PC_REC_MIN_XFER_SIZE => mp().iosize() as Register,
        _PC_REC_MAX_XFER_SIZE => -1, // means ``unlimited''
        _PC_REC_XFER_ALIGN => PAGE_SIZE as Register,
        _PC_SYMLINK_MAX => MAXPATHLEN as Register,
        _ => return Err(Errno::EINVAL),
    };

    Ok(())
}

/// `ufs_advlock` (`vop_advlock`): advisory record locking support.
pub fn ufs_advlock<G: ProcGlue>(
    vp: &'static Vnode<G>,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    let ip = vtoi(vp);

    lf_advlock::<G>(&ip.i_lockf, ip.dip_size() as i64, id, op, fl, flags)
}

/// `ufs_makeinode`: allocate a new inode of type `mode` and enter it in the directory
/// `dvp` under the name in `cnp`; `*vpp` is its vnode, referenced and locked.
pub fn ufs_makeinode<G: ProcGlue>(
    mut mode: Mode,
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let pdir = vtoi(dvp);
    #[cfg(feature = "diagnostic")]
    if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
        kpanic(format_args!("ufs_makeinode: no name"));
    }
    *vpp = None;
    if mode & IFMT == 0 {
        mode |= IFREG;
    }

    let tvp = match pdir.vtbl().inode_alloc(&pdir, mode, cnp.cn_cred) {
        Ok(tvp) => tvp,
        Err(e) => {
            pnbuf_free(cnp);
            return Err(e);
        }
    };

    let ip = vtoi(tvp);

    ip.dip_set_gid(pdir.dip_gid());
    let c = ucred(cnp.cn_cred);
    ip.dip_set_uid(c.cr_uid.get());

    if let Err(e) = getinoquota(&ip).and_then(|()| ufs_quota_alloc_inode(&ip, cnp.cn_cred)) {
        pnbuf_free(cnp);
        let _ = ip.vtbl().inode_free(&ip, ip.i_number, mode);
        drop(ip);
        vput(tvp);
        return Err(e);
    }

    ip.set_flag(IN_ACCESS | IN_CHANGE | IN_UPDATE);
    ip.dip_set_mode(mode);
    tvp.set_vtype(iftovt(mode)); // Rest init'd in getnewvnode().
    ip.i_effnlink.set(1);
    ip.dip_set_nlink(1);
    if ip.dip_mode() & ISGID != 0
        && !groupmember(ip.dip_gid(), c)
        && !vnoperm(dvp)
        && suser_ucred(c).is_err()
    {
        ip.dip_set_mode(ip.dip_mode() & !ISGID);
    }

    let error: Result<(), Errno> = 'bad: {
        // Make sure inode goes to disk before directory entry.
        if let Err(e) = ip.vtbl().update(&ip, 1) {
            break 'bad Err(e);
        }

        let mut newdir = Direct::new();
        ufs_makedirentry(&ip, cnp, &mut newdir);
        if let Err(e) = ufs_direnter(dvp, Some(tvp), &mut newdir, cnp) {
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
    ip.i_effnlink.set(0);
    ip.dip_set_nlink(0);
    ip.set_flag(IN_CHANGE);
    tvp.set_vtype(VNON);
    drop(ip);
    vput(tvp);

    error
}

/// `ufs_kqfilter` (`vop_kqfilter`): attach a knote to the vnode.
pub fn ufs_kqfilter<G: ProcGlue>(vp: &'static Vnode<G>, kn: &Knote<G>) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&UfsFiltops::<G>::READ)),
        EVFILT_WRITE => kn.kn_fop.set(Some(&UfsFiltops::<G>::WRITE)),
        EVFILT_VNODE => kn.kn_fop.set(Some(&UfsFiltops::<G>::VNODE)),
        _ => return Err(Errno::EINVAL),
    }

    // kn->kn_hook = (caddr_t)vp. The C stores over whatever was there; here the hook is set
    // once, and a knote reaches VOP_KQFILTER once (kqueue_register attaches a fresh knote;
    // a second EV_ADD of the same ident and filter modifies it, never attaches it again),
    // so it holds no hook yet: a set one is left as it is, with no panic the C lacks.
    let _ = kn.set_hook_vnode(vp);

    vp.klist.insert_legacy(kn);

    Ok(())
}

/// `kn->kn_hook` of a UFS knote: its vnode.
fn kn_vnode<G: ProcGlue>(kn: &Knote<G>) -> &'static Vnode<G> {
    match kn.hook_vnode() {
        Some(vp) => vp,
        None => kpanic(format_args!("knote {kn:p}: no vnode")),
    }
}

/// `filt_ufsdetach`: unhooks the knote from the vnode.
fn filt_ufsdetach<G: ProcGlue>(kn: &Knote<G>) {
    let vp = kn_vnode(kn);

    vp.klist.remove_legacy(kn);
}

impl<G: ProcGlue> Filter<G> for UfsRead {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ufsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_ufsdetach(kn);
    }

    /// `filt_ufsread`: the bytes past the file offset; always ready for poll and select.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        let vp = kn_vnode(kn);

        // filesystem is gone, so set the EOF flag and schedule the knote for deletion. The
        // vnode has no inode any more: vclean reclaims it before its VN_KNOTE(NOTE_REVOKE),
        // so the inode is reached only past this test (the C loads the NULL v_data and does
        // not follow it).
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }
        let ip = vtoi(vp);

        let size = if ip.fstype() == UM_EXT2FS {
            <G::FileGlue as FileGlue>::ext2fs_size(&ip) as i64
        } else {
            ip.dip_size() as i64
        };
        kn.kn_data().set(size - foffset(&kn.fp()));
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

impl<G: ProcGlue> Filter<G> for UfsWrite {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ufsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_ufsdetach(kn);
    }

    /// `filt_ufswrite`: a file is always writable.
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

impl<G: ProcGlue> Filter<G> for UfsVnode {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ufsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_ufsdetach(kn);
    }

    /// `filt_ufsvnode`: records the vnode events (`NOTE_*`) the user asked for.
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

/// The UFS slots of `ffs_vops` and `ffs_specvops` both tables fill alike.
macro_rules! ufs_common_slots {
    () => {
        fn access(
            &self,
            vp: &'static Vnode<G>,
            mode: i32,
            cred: Cred<'_>,
            _p: &Thread<G>,
        ) -> Result<(), Errno> {
            ufs_access(vp, mode, cred)
        }
        fn getattr(
            &self,
            vp: &'static Vnode<G>,
            vap: &mut Vattr,
            _cred: Cred<'_>,
            _p: &Thread<G>,
        ) -> Result<(), Errno> {
            ufs_getattr(vp, vap)
        }
        fn setattr(
            &self,
            vp: &'static Vnode<G>,
            vap: &mut Vattr,
            cred: Cred<'_>,
            p: &Thread<G>,
        ) -> Result<(), Errno> {
            ufs_setattr(vp, vap, cred, p)
        }
        fn fsync(
            &self,
            vp: &'static Vnode<G>,
            cred: Cred<'_>,
            waitfor: i32,
            p: &Thread<G>,
        ) -> Result<(), Errno> {
            self.fs.fsync(vp, cred, waitfor, p)
        }
        fn inactive(&self, vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
            ufs_inactive(vp, p)
        }
        /// `ffs_reclaim`: `ufs_reclaim`, then `vp->v_data = NULL`: the vnode's handle to
        /// the inode goes, and with the last one the dinode and the inode go back to their
        /// pools (the `Dinode` box's and the `InodeRef`'s drops).
        fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
            ufs_reclaim(vp)?;
            drop(vp.take_node());
            Ok(())
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
        fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
            default::vop_generic_revoke(vp, flags)
        }
        fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
            default::vop_generic_bwrite(bp)
        }
    };
}

impl<G: ProcGlue, F: UfsFs<G>> Vnops<G> for UfsVnops<G, F> {
    ufs_common_slots!();

    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        ufs_lookup(dvp, vpp, cnp)
    }
    fn create(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        ufs_create(dvp, vpp, cnp, vap)
    }
    fn mknod(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        ufs_mknod(dvp, vpp, cnp, vap)
    }
    fn open(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        ufs_open(vp, mode)
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
        self.fs.read(vp, uio, ioflag, cred)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        self.fs.write(vp, uio, ioflag, cred)
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
        ufs_remove(dvp, vp, cnp)
    }
    fn link(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        ufs_link(dvp, vp, cnp)
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
        ufs_rename(fdvp, fvp, fcnp, tdvp, tvp, tcnp)
    }
    fn mkdir(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        ufs_mkdir(dvp, vpp, cnp, vap)
    }
    fn rmdir(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        ufs_rmdir(dvp, vp, cnp)
    }
    fn symlink(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
        target: &[u8],
    ) -> Result<(), Errno> {
        ufs_symlink(dvp, vpp, cnp, vap, target)
    }
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        ufs_readdir(vp, uio, eofflag)
    }
    fn readlink(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        ufs_readlink(vp, uio, cred)
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
        ufs_bmap(vp, bn, vpp, bnp, runp)
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
        ufs_pathconf(vp, name, retval)
    }
    fn advlock(
        &self,
        vp: &'static Vnode<G>,
        id: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        ufs_advlock(vp, id, op, fl, flags)
    }
}

impl<G: ProcGlue, F: UfsFs<G>> Vnops<G> for UfsSpecVnops<G, F> {
    ufs_common_slots!();

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

    // XXX: Keep in sync with spec_vops (one macro: the same slots as SpecVops's).
    crate::fs::spec::spec_shared_slots!();
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::testfs::TestGlue;

    use crate::cred::Ucred;
    use crate::fs::namei::{CREATE, LOCKPARENT, LOOKUP, RENAME, SAVENAME, SAVESTART, WANTPARENT};
    use crate::fs::subr::vgone;
    use crate::fs::ufs::ufs::testufs::{self, BSIZE, cn, create, lookup, setup, vattr};
    use crate::fs::vops::{
        VFS_ROOT, vop_access, vop_advlock, vop_close, vop_getattr, vop_ioctl, vop_kqfilter,
        vop_link, vop_lookup, vop_mkdir, vop_open, vop_pathconf, vop_readdir, vop_readlink,
        vop_remove, vop_rename, vop_rmdir, vop_setattr, vop_symlink,
    };
    use crate::uio::Iovec;

    /// A credential of `uid` (group 0), not root's when `uid` is not 0.
    fn user(uid: u32) -> Ucred {
        let mut c = Ucred::new();
        c.cr_uid.set(uid);
        c
    }

    #[test]
    fn create_lookup_and_remove_through_the_table() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        // CREATE of a missing name: EJUSTRETURN with the slot in the root's block (the ".."
        // entry, 12 bytes in, has DIRBLKSIZ - 24 bytes free), the name saved.
        let mut c = cn(p, cred, CREATE, LOCKPARENT, b"f");
        let mut vpp = None;
        assert_eq!(vop_lookup(root, &mut vpp, &mut c), Err(Errno::EJUSTRETURN));
        assert_ne!(c.cn_flags & SAVENAME, 0);
        let dp = vtoi(root);
        assert_eq!(
            (dp.i_offset.get(), dp.i_count.get()),
            (12, BSIZE as i32 - 12)
        );
        let mut va = vattr(VREG, 0o644);
        crate::fs::vops::vop_create(root, &mut vpp, &mut c, &mut va).unwrap();
        let vp = vpp.unwrap();
        let ip = vtoi(vp);
        assert_eq!(
            (ip.i_number, ip.dip_nlink(), ip.dip_mode()),
            (3, 1, IFREG | 0o644)
        );
        assert_eq!(ip.dip_uid(), 0);
        // The inode written before its entry; the entry fits the block's free slot, so the
        // directory itself needs no UFS_UPDATE (ufs_direnter's i_count != 0 path).
        assert_eq!(testufs::updates(), 1);
        drop(ip);
        vput(vp);
        // LOOKUP finds it; the entry is the C's bytes (reclen to the block's end).
        let vp = lookup(p, cred, root, b"f").unwrap();
        assert_eq!(vtoi(vp).i_number, 3);
        assert_eq!(dp.i_reclen.get(), (BSIZE - 24) as u32);
        vput(vp);
        // DELETE, then the name is gone and the inode unlinked.
        let mut c = cn(p, cred, DELETE, LOCKPARENT, b"f");
        let mut vpp = None;
        vop_lookup(root, &mut vpp, &mut c).unwrap();
        let vp = vpp.unwrap();
        // The entry at 24, after "..": i_count is the distance back to the previous entry.
        assert_eq!((dp.i_offset.get(), dp.i_count.get()), (24, 12));
        let ip = vtoi(vp);
        // VOP_REMOVE releases both vnodes.
        vref(root);
        vop_remove(root, vp, &mut c).unwrap();
        assert_eq!((ip.dip_nlink(), ip.i_effnlink.get()), (0, 0));
        drop(ip);
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        assert_eq!(lookup(p, cred, root, b"f").err(), Some(Errno::ENOENT));
        // The unlinked inode was freed with its mode at its last reference (ufs_inactive).
        assert_eq!(testufs::freed(), [(3, IFREG | 0o644)]);
    }

    #[test]
    fn dot_dotdot_and_a_file_that_is_not_a_directory() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        // ".": the directory itself, referenced again.
        let dot = lookup(p, cred, root, b".").unwrap();
        assert!(core::ptr::eq(dot, root));
        vrele(dot);
        let f = create(p, cred, root, b"f").unwrap();
        // A lookup in a file is ENOTDIR (before the access check).
        assert_eq!(lookup(p, cred, f, b"x").err(), Some(Errno::ENOTDIR));
        vput(f);
        // mkdir "d", then ".." from it: the parent unlocked for the race, and relocked for
        // LOCKPARENT at the last component.
        vref(root);
        let mut c = cn(p, cred, CREATE, LOCKPARENT, b"d");
        let mut vpp = None;
        assert_eq!(vop_lookup(root, &mut vpp, &mut c), Err(Errno::EJUSTRETURN));
        let mut va = vattr(VDIR, 0o755);
        vop_mkdir(root, &mut vpp, &mut c, &mut va).unwrap();
        let d = vpp.unwrap();
        let mut c = cn(p, cred, LOOKUP, LOCKPARENT | ISDOTDOT, b"..");
        let mut vpp = None;
        vop_lookup(d, &mut vpp, &mut c).unwrap();
        assert!(core::ptr::eq(vpp.unwrap(), root));
        assert_eq!(c.cn_flags & crate::fs::namei::PDIRUNLOCK, 0);
        assert!(vtoi(d).i_lock.status() != 0, "d relocked");
        vput(root);
        vput(d);
    }

    #[test]
    fn mkdir_writes_dot_and_dotdot_and_rmdir_refuses_a_non_empty_directory() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        vref(root);
        let mut c = cn(p, cred, CREATE, LOCKPARENT, b"d");
        let mut vpp = None;
        assert_eq!(vop_lookup(root, &mut vpp, &mut c), Err(Errno::EJUSTRETURN));
        let mut va = vattr(VDIR, 0o755);
        vop_mkdir(root, &mut vpp, &mut c, &mut va).unwrap();
        let d = vpp.unwrap();
        let ip = vtoi(d);
        assert_eq!((ip.dip_nlink(), ip.i_effnlink.get()), (2, 2));
        assert_eq!(ip.dip_size(), BSIZE as u64);
        assert_eq!(vtoi(root).dip_nlink(), 3, "the new directory's \"..\"");
        // Its block: "." and ".." of the template.
        let mut eof = 0;
        let mut buf = [0u8; 512];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        vop_readdir(d, &mut uio, Cred::real(cred), &mut eof).unwrap();
        drop(uio);
        assert_eq!(eof, 1);
        assert_eq!(&buf[24..26], b".\0");
        assert_eq!(u64::from_ne_bytes(buf[32..40].try_into().unwrap()), 2);
        // A file in it, then rmdir is ENOTEMPTY (the file's entry), d still there.
        let x = create(p, cred, d, b"x").unwrap();
        vput(x);
        drop(ip);
        vput(d);
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        let mut c = cn(p, cred, DELETE, LOCKPARENT, b"d");
        let mut vpp = None;
        vop_lookup(root, &mut vpp, &mut c).unwrap();
        let d = vpp.unwrap();
        vref(root);
        assert_eq!(vop_rmdir(root, d, &mut c), Err(Errno::ENOTEMPTY));
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        let d = lookup(p, cred, root, b"d").unwrap();
        // Remove x, then rmdir succeeds and the parent loses the link.
        let mut c = cn(p, cred, DELETE, LOCKPARENT, b"x");
        let mut vpp = None;
        vop_lookup(d, &mut vpp, &mut c).unwrap();
        let x = vpp.unwrap();
        vop_remove(d, x, &mut c).unwrap(); // x and d released
        let mut c = cn(p, cred, DELETE, LOCKPARENT, b"d");
        let mut vpp = None;
        vop_lookup(root, &mut vpp, &mut c).unwrap();
        let d = vpp.unwrap();
        vref(root);
        vop_rmdir(root, d, &mut c).unwrap();
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        assert_eq!(vtoi(root).dip_nlink(), 2);
        assert_eq!(lookup(p, cred, root, b"d").err(), Some(Errno::ENOENT));
        vput(root);
    }

    #[test]
    fn rename_in_one_directory_moves_the_entry() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        let f = create(p, cred, root, b"f").unwrap();
        let ino = vtoi(f).i_number;
        vput(f);
        // namei's two lookups as rename(2) makes them: the source unlocked, the target's
        // directory locked.
        let mut fc = cn(p, cred, DELETE, WANTPARENT | SAVESTART, b"f");
        let mut fvp = None;
        vop_lookup(root, &mut fvp, &mut fc).unwrap();
        let fvp = fvp.unwrap();
        let _ = vop_unlock(fvp);
        vref(root);
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        let mut tc = cn(p, cred, RENAME, LOCKPARENT | SAVESTART, b"g");
        let mut tvp = None;
        assert_eq!(vop_lookup(root, &mut tvp, &mut tc), Err(Errno::EJUSTRETURN));
        vref(root);
        vop_rename(root, fvp, &mut fc, root, None, &mut tc).unwrap();
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        assert_eq!(lookup(p, cred, root, b"f").err(), Some(Errno::ENOENT));
        let g = lookup(p, cred, root, b"g").unwrap();
        assert_eq!(vtoi(g).i_number, ino);
        assert_eq!(vtoi(g).dip_nlink(), 1, "the link count bump undone");
        vput(g);
        vput(root);
    }

    #[test]
    fn readdir_converts_the_entries_and_reports_the_end() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        let f = create(p, cred, root, b"file").unwrap();
        vput(f);
        let mut eof = 0;
        let mut buf = [0u8; 512];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        vop_readdir(root, &mut uio, Cred::real(cred), &mut eof).unwrap();
        let off = uio.uio_offset;
        drop(uio);
        assert_eq!((eof, off), (1, BSIZE as i64));
        // Three dirents of 32 bytes: ".", "..", "file" (d_type DIR, DIR, REG).
        let types: std::vec::Vec<u8> = (0..3).map(|i| buf[i * 32 + 18]).collect();
        assert_eq!(types, [DT_DIR, DT_DIR, hal::sys::dirent::DT_REG]);
        assert_eq!(&buf[64 + 24..64 + 28], b"file");
        assert_eq!(u16::from_ne_bytes([buf[16], buf[17]]), 32);
        // A request smaller than what is left of the block is EINVAL.
        let mut small = [0u8; 8];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut small, Some(p));
        assert_eq!(
            vop_readdir(root, &mut uio, Cred::real(cred), &mut eof),
            Err(Errno::EINVAL)
        );
        vput(root);
    }

    #[test]
    fn symlinks_short_in_the_inode_and_long_in_a_block() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        for (name, target) in [(&b"s"[..], &b"target"[..]), (&b"l"[..], &[b'x'; 200][..])] {
            let name: &'static [u8] = if name == b"s" { b"s" } else { b"l" };
            vref(root);
            let mut c = cn(p, cred, CREATE, LOCKPARENT, name);
            let mut vpp = None;
            assert_eq!(vop_lookup(root, &mut vpp, &mut c), Err(Errno::EJUSTRETURN));
            let mut va = vattr(VLNK, 0o755);
            vop_symlink(root, &mut vpp, &mut c, &mut va, target).unwrap();
            vn_lock(root, LK_EXCLUSIVE).unwrap();
            let vp = lookup(p, cred, root, name).unwrap();
            let ip = vtoi(vp);
            assert_eq!(ip.dip_size(), target.len() as u64);
            drop(ip);
            let mut buf = [0u8; 256];
            let mut iov = [Iovec::new()];
            let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
            vop_readlink(vp, &mut uio, Cred::real(cred)).unwrap();
            let left = uio.uio_resid;
            drop(uio);
            assert_eq!(&buf[..256 - left], target);
            vput(vp);
        }
        vput(root);
    }

    #[test]
    fn access_and_setattr_keep_the_c_checks_in_order() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        let f = create(p, cred, root, b"f").unwrap();
        let mut other = user(1000);
        // vaccess: write by another user of a 0644 file is EACCES; root may.
        assert_eq!(
            vop_access(f, VWRITE, Cred::real(&other), p),
            Err(Errno::EACCES)
        );
        vop_access(f, VWRITE, Cred::real(cred), p).unwrap();
        // Unsettable attributes: EINVAL first.
        let mut va = vattr(VNON, VNOVAL as Mode);
        va.va_nlink = 5;
        assert_eq!(
            vop_setattr(f, &mut va, Cred::real(cred), p),
            Err(Errno::EINVAL)
        );
        // Flags by a non-owner: suser fails, EPERM.
        let mut va = vattr(VNON, VNOVAL as Mode);
        va.va_flags = 1;
        assert_eq!(
            vop_setattr(f, &mut va, Cred::real(&other), p),
            Err(Errno::EPERM)
        );
        // chmod and chown by a non-owner: EPERM.
        let mut va = vattr(VNON, 0o600);
        assert_eq!(
            vop_setattr(f, &mut va, Cred::real(&other), p),
            Err(Errno::EPERM)
        );
        let mut va = vattr(VNON, VNOVAL as Mode);
        va.va_uid = 1000;
        assert_eq!(
            vop_setattr(f, &mut va, Cred::real(&other), p),
            Err(Errno::EPERM)
        );
        // Root gives the file away; the new owner may not set the sticky bit on a file
        // (EFTYPE) nor set-gid for a group it is not in (EPERM), and chown clears set-id.
        let ip = vtoi(f);
        ip.dip_set_mode(ip.dip_mode() | ISUID);
        vop_setattr(f, &mut va, Cred::real(cred), p).unwrap();
        assert_eq!(ip.dip_uid(), 1000);
        assert_eq!(ip.dip_mode() & ISUID, ISUID, "root's chown keeps set-uid");
        let mut va = vattr(VNON, 0o1644);
        assert_eq!(
            vop_setattr(f, &mut va, Cred::real(&other), p),
            Err(Errno::EFTYPE)
        );
        let mut va = vattr(VNON, 0o2644);
        other.cr_gid.set(5);
        assert_eq!(
            vop_setattr(f, &mut va, Cred::real(&other), p),
            Err(Errno::EPERM)
        );
        let mut va = vattr(VNON, 0o640);
        vop_setattr(f, &mut va, Cred::real(&other), p).unwrap();
        assert_eq!(ip.dip_mode() & ALLPERMS, 0o640);
        // A size: truncated through the file system; a directory refuses one (EISDIR).
        let mut va = vattr(VNON, VNOVAL as Mode);
        va.va_size = 0;
        vop_setattr(f, &mut va, Cred::real(cred), p).unwrap();
        let mut va = vattr(VNON, VNOVAL as Mode);
        va.va_size = 0;
        assert_eq!(
            vop_setattr(root, &mut va, Cred::real(cred), p),
            Err(Errno::EISDIR)
        );
        // getattr copies the inode.
        let mut va = Vattr::new();
        vop_getattr(f, &mut va, Cred::real(cred), p).unwrap();
        assert_eq!((va.va_fileid, va.va_uid, va.va_nlink), (3, 1000, 1));
        assert_eq!(va.va_blocksize, BSIZE as i64);
        // An append-only file must be opened for appending.
        ip.dip_set_flags(APPEND);
        assert_eq!(vop_open(f, FWRITE, Cred::real(cred), p), Err(Errno::EPERM));
        vop_open(f, FWRITE | O_APPEND, Cred::real(cred), p).unwrap();
        ip.dip_set_flags(0);
        drop(ip);
        vput(f);
        vput(root);
    }

    #[test]
    fn link_pathconf_advlock_ioctl_kqfilter_and_close() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        let f = create(p, cred, root, b"f").unwrap();
        // link "h" to f: two links, both names find it.
        let _ = vop_unlock(f);
        let mut c = cn(p, cred, CREATE, LOCKPARENT, b"h");
        let mut vpp = None;
        assert_eq!(vop_lookup(root, &mut vpp, &mut c), Err(Errno::EJUSTRETURN));
        vref(root);
        vop_link(root, f, &mut c).unwrap();
        assert_eq!(vtoi(f).dip_nlink(), 2);
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        let h = lookup(p, cred, root, b"h").unwrap();
        assert!(core::ptr::eq(h, f));
        let mut r: Register = 0;
        vop_pathconf(f, _PC_LINK_MAX, &mut r).unwrap();
        assert_eq!(r, LINK_MAX as Register);
        vop_pathconf(f, _PC_REC_MIN_XFER_SIZE, &mut r).unwrap();
        assert_eq!(r, BSIZE as Register);
        assert_eq!(vop_pathconf(f, 9999, &mut r), Err(Errno::EINVAL));
        let mut fl = Flock {
            l_start: 0,
            l_len: 0,
            l_pid: 0,
            l_type: hal::sys::fcntl::F_WRLCK,
            l_whence: 0,
        };
        vop_advlock(f, LockOwner::Table(1), hal::sys::fcntl::F_GETLK, &mut fl, 0).unwrap();
        assert_eq!(fl.l_type, hal::sys::fcntl::F_UNLCK);
        let mut data = [0u8; 8];
        assert_eq!(
            vop_ioctl(f, 0, &mut data, 0, Cred::real(cred), p),
            Err(Errno::ENOTTY)
        );
        let kn = Knote::<TestGlue>::detached();
        kn.kn_filter().set(crate::event::EVFILT_PROC);
        assert_eq!(vop_kqfilter(f, 0, &kn), Err(Errno::EINVAL));
        // close of a file referenced twice updates its times.
        vtoi(f).set_flag(IN_ACCESS);
        vop_close(f, 0, Cred::real(cred), Some(p)).unwrap();
        assert_eq!(vtoi(f).i_flag.get() & IN_ACCESS, 0);
        vput(h);
        // vgone reclaims: the inode leaves the vnode.
        vgone(f);
        assert!(f.node_with(|_: &Inode<TestGlue>| ()).is_none());
        vput(root);
    }

    #[test]
    fn mknod_abortop_print_fsync_and_revoke() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        // mknod: the node is made and dropped from the cache (vgone), to be found anew.
        let mut c = cn(p, cred, CREATE, LOCKPARENT, b"n");
        let mut vpp = None;
        assert_eq!(vop_lookup(root, &mut vpp, &mut c), Err(Errno::EJUSTRETURN));
        let mut va = vattr(VCHR, 0o600);
        va.va_rdev = hal::sys::types::makedev(1, 3);
        crate::fs::vops::vop_mknod(root, &mut vpp, &mut c, &mut va).unwrap();
        assert!(vpp.is_none());
        let n = lookup(p, cred, root, b"n").unwrap();
        assert_eq!(n.vtype(), VCHR);
        assert_eq!(
            vtoi(n).dip_rdev(),
            i64::from(hal::sys::types::makedev(1, 3))
        );
        vput(n);
        // A lookup aborted gives its name back.
        let mut c = cn(p, cred, CREATE, LOCKPARENT, b"m");
        crate::fs::vops::vop_abortop(root, &mut c).unwrap();
        let f = create(p, cred, root, b"f").unwrap();
        crate::fs::vops::vop_print(f).unwrap();
        crate::fs::vops::vop_fsync(f, Cred::real(cred), hal::sys::mount::MNT_WAIT, p).unwrap();
        let _ = vop_unlock(f);
        // revoke: the vnode is cleaned, its inode reclaimed off it.
        crate::fs::vops::vop_revoke(f, crate::fs::vnode::REVOKEALL).unwrap();
        assert!(f.node_with(|_: &Inode<TestGlue>| ()).is_none());
        vput(root);
    }

    #[test]
    fn a_revoked_file_s_read_filter_ends_without_its_inode() {
        // tail -f on a file, then umount -f: vclean reclaims the inode (the vnode's node
        // goes), then knotes NOTE_REVOKE; the read filter must not reach the inode.
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        let f = create(p, cred, root, b"f").unwrap();
        let kn = Knote::<TestGlue>::detached();
        assert!(kn.set_hook_vnode(f).is_ok());
        let _ = vop_unlock(f);
        vgone(f);
        assert!(f.node_with(|_: &Inode<TestGlue>| ()).is_none());
        assert!(<UfsRead as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF) && kn.has_flags(EV_ONESHOT));
        // The write and vnode filters read no inode at all.
        assert!(<UfsWrite as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(<UfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        vput(root);
    }

    #[test]
    fn a_sticky_directory_keeps_others_from_removing_and_renaming_over_its_files() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let user = user(1000);
        let root = VFS_ROOT(mp).unwrap();
        vtoi(root).dip_set_mode(IFDIR | 0o1777);
        // b is root's, a the user's.
        vput(create(p, cred, root, b"b").unwrap());
        let a = create(p, &user, root, b"a").unwrap();
        assert_eq!(vtoi(a).dip_uid(), 1000);
        vput(a);
        // DELETE of root's b by the user: EPERM (ufs_lookup.c's ISVTX test), after the
        // directory's write access passed.
        let mut c = cn(p, &user, DELETE, LOCKPARENT, b"b");
        let mut vpp = None;
        assert_eq!(vop_lookup(root, &mut vpp, &mut c), Err(Errno::EPERM));
        assert!(vpp.is_none());
        // Its own a it may delete (the lookup answers it).
        let mut c = cn(p, &user, DELETE, LOCKPARENT, b"a");
        vop_lookup(root, &mut vpp, &mut c).unwrap();
        vput(vpp.take().unwrap());
        // rename(a, b) by the user: the target is root's in a sticky directory, EPERM; both
        // names stay.
        let mut fc = cn(p, &user, DELETE, WANTPARENT | SAVESTART, b"a");
        let mut fvp = None;
        vop_lookup(root, &mut fvp, &mut fc).unwrap();
        let fvp = fvp.unwrap();
        let _ = vop_unlock(fvp);
        vref(root);
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        let mut tc = cn(p, &user, RENAME, LOCKPARENT | SAVESTART, b"b");
        let mut tvp = None;
        vop_lookup(root, &mut tvp, &mut tc).unwrap();
        vref(root);
        assert_eq!(
            vop_rename(root, fvp, &mut fc, root, tvp, &mut tc),
            Err(Errno::EPERM)
        );
        vn_lock(root, LK_EXCLUSIVE).unwrap();
        vput(lookup(p, cred, root, b"a").unwrap());
        let b = lookup(p, cred, root, b"b").unwrap();
        assert_eq!(vtoi(b).dip_uid(), 0);
        vput(b);
        vput(root);
    }

    #[test]
    fn a_read_only_mount_and_the_immutable_flags_refuse_writes() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        let f = create(p, cred, root, b"f").unwrap();
        let ip = vtoi(f);
        // IMMUTABLE: nobody writes it (EPERM), even root.
        ip.dip_set_flags(IMMUTABLE);
        assert_eq!(
            vop_access(f, VWRITE, Cred::real(cred), p),
            Err(Errno::EPERM)
        );
        ip.dip_set_flags(0);
        // chflags of a system-immutable file under securelevel > 0: EPERM, even root.
        ip.dip_set_flags(SF_IMMUTABLE);
        let mut va = vattr(VNON, VNOVAL as Mode);
        va.va_flags = 0;
        SECURELEVEL.store(1, Ordering::Relaxed);
        let r = vop_setattr(f, &mut va, Cred::real(cred), p);
        SECURELEVEL.store(0, Ordering::Relaxed);
        assert_eq!(r, Err(Errno::EPERM));
        assert_eq!(ip.dip_flags(), SF_IMMUTABLE);
        let mut va = vattr(VNON, VNOVAL as Mode);
        va.va_flags = 0;
        vop_setattr(f, &mut va, Cred::real(cred), p).unwrap();
        assert_eq!(ip.dip_flags(), 0);
        // A read-only mount: write access, flags and mode are EROFS; read access is not.
        mp.flag_set(MNT_RDONLY);
        assert_eq!(
            vop_access(f, VWRITE, Cred::real(cred), p),
            Err(Errno::EROFS)
        );
        vop_access(f, crate::fs::vnode::VREAD, Cred::real(cred), p).unwrap();
        let mut va = vattr(VNON, VNOVAL as Mode);
        va.va_flags = 0;
        assert_eq!(
            vop_setattr(f, &mut va, Cred::real(cred), p),
            Err(Errno::EROFS)
        );
        let mut va = vattr(VNON, 0o600);
        assert_eq!(
            vop_setattr(f, &mut va, Cred::real(cred), p),
            Err(Errno::EROFS)
        );
        mp.flag_clr(MNT_RDONLY);
        drop(ip);
        vput(f);
        vput(root);
    }

    #[test]
    fn the_spec_table_keeps_the_ufs_slots() {
        let (_g, p, mp) = setup();
        let cred = p.ucred();
        let root = VFS_ROOT(mp).unwrap();
        let t = &testufs::TEST_UFS_SPECVOPS;
        t.access(root, VEXEC, Cred::real(cred), p).unwrap();
        let mut va = Vattr::new();
        t.getattr(root, &mut va, Cred::real(cred), p).unwrap();
        assert_eq!(va.va_fileid, 2);
        assert!(t.islocked(root) != 0);
        t.unlock(root).unwrap();
        t.lock(root, LK_EXCLUSIVE).unwrap();
        t.print(root).unwrap();
        t.fsync(root, Cred::real(cred), 0, p).unwrap();
        let mut vpp = None;
        let mut c = cn(p, cred, LOOKUP, 0, b"x");
        assert_eq!(t.lookup(root, &mut vpp, &mut c), Err(Errno::ENOTDIR));
        let mut r: Register = 0;
        t.pathconf(root, _PC_LINK_MAX, &mut r).unwrap();
        assert_eq!(r, LINK_MAX as Register);
        vput(root);
    }

    #[test]
    fn vnode_filters_record_the_events_asked_for() {
        // EVFILT_VNODE: only the subscribed notes are recorded; revocation ends it.
        let kn = Knote::<TestGlue>::detached();
        kn.kn_sfflags.set(NOTE_WRITE | NOTE_DELETE);
        assert!(!<UfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_ATTRIB)
        ));
        assert!(<UfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_WRITE)
        ));
        assert_eq!(kn.kn_fflags().get(), NOTE_WRITE);
        assert!(<UfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF));

        // EVFILT_WRITE: always writable, EOF and one-shot once revoked.
        let kn = Knote::<TestGlue>::detached();
        assert!(<UfsWrite as Filter<TestGlue>>::event(&kn, 0) && kn.kn_data().get() == 0);
        assert!(<UfsWrite as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF) && kn.has_flags(EV_ONESHOT));

        // The tables carry the C's flags.
        assert_eq!(UfsFiltops::<TestGlue>::READ.f_flags, FILTEROP_ISFD);
        assert!(UfsFiltops::<TestGlue>::VNODE.f_event.is_some());
    }

    #[test]
    fn the_master_template_is_dot_and_dotdot_filling_a_block() {
        let t = MASTERTEMPLATE.to_bytes();
        assert_eq!(d_reclen(&t, 0), 12);
        assert_eq!(d_name(&t, 0, 1), b".");
        assert_eq!(usize::from(d_reclen(&t, 12)), DIRBLKSIZ - 12);
        assert_eq!(d_name(&t, 12, 2), b"..");
        assert_eq!(d_type(&t, 12), DT_DIR);
    }

    #[test]
    fn ioctl_is_enotty() {
        assert_eq!(ufs_ioctl(), Err(Errno::ENOTTY));
    }
}
/* </TESTS> */
