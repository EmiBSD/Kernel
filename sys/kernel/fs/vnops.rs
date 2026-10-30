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
 *	@(#)vfs_vnops.c	8.5 (Berkeley) 12/8/94
 */
/* </LICENSES> */

/* <CODE> */
//! Common code for vnode operations: `vn_open` (permission checks and `VOP_OPEN` or
//! `VOP_CREATE`), `vn_close`, `vn_rdwr`, `vn_stat`, `vn_writechk`, `vn_fsizechk`,
//! `vn_marktext`, `vn_isunder`, and the file table's vnode operations (`vnops`: `vn_read`,
//! `vn_write`, `vn_ioctl`, `vn_kqfilter`, `vn_statfile`, `vn_closefile`, `vn_seek`).
//! `vn_lock` is `fs::vnode`'s.
//!
//! Upstream: sys/kern/vfs_vnops.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_vnops.rs@3c62ede68802
//!
//! ## Deviations
//! - `vn_fsizechk` returns the overrun instead of filling `*overrun`; `aresid` is an
//!   `Option<&mut usize>` (LZ's).
//!
//! ## Redesign
//! - `vn_rdwr`'s `caddr_t base, int len, enum uio_seg segflg` (LZ: a raw pointer, a length
//!   and the segment, three arguments saying what one enum says) is [`RdwrBuf`] (D-N5.3-10):
//!   a kernel slice to read into or write from, or a user address and a length; the uio is
//!   built from it (`Uio::kernel_read`/`kernel_write`, or a `UIO_USERSPACE` iovec whose
//!   address is never followed here).
//! - The file table's operations are [`VnodeFileOps`], `struct fileops` as the kernel's
//!   `FileOps` (decision 46); the file's vnode is the glue's answer for its object
//!   ([`FileGlue::vnode_of`]: `(struct vnode *)fp->f_data`); each keeps the C's
//!   `KERNEL_LOCK()`/`KERNEL_UNLOCK()` (`hal::klock`). `bsd`'s `vnops` table is the same
//!   operations over these functions.
//! - `vn_ioctl`'s `TIOCSCTTY` keeps the controlling terminal's vnode in the kernel's
//!   `Session::ttyvp` (D-N5.2-9).
//! - `vn_fsizechk`'s `psignal(uio->uio_procp, SIGXFSZ)` takes the thread pinned from its
//!   caller (`p`, the uio's thread): a signal goes to a pinned thread.
//! - `vn_closefile`'s `VOP_ADVLOCK(vp, (caddr_t)fp, ...)` names the owner as
//!   [`LockOwner::File`] (D-N5.3-11).

use crate::file::LockOwner;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::klock;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{
    F_FLOCK, F_UNLCK, FFSYNC, FNONBLOCK, FREAD, FWRITE, Flock, O_APPEND, O_CREAT, O_DIRECTORY,
    O_EXCL, O_NOFOLLOW, O_TRUNC,
};
use hal::sys::filio::{FIOASYNC, FIONREAD};
use hal::sys::mount::{MNT_RDONLY, MNT_SYNCHRONOUS};
use hal::sys::signal::SIGXFSZ;
use hal::sys::stat::{
    S_BLKSIZE, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFREG, S_IFSOCK, Stat,
};
use hal::sys::types::Off;
use hal::sys::unistd::{SEEK_CUR, SEEK_END, SEEK_SET};

use super::getcwd::vfs_getcwd_common;
use super::lookup::namei;
use super::namei::{
    BYPASSUNVEIL, CREATE, FOLLOW, KERNELPATH, LOCKLEAF, LOCKPARENT, LOOKUP, MAXPATHLEN, NOFOLLOW,
    Nameidata,
};
use super::subr::{vattr_null, vput, vref, vrele};
use super::vnode::{
    IO_APPEND, IO_NDELAY, IO_NODELOCKED, IO_NOLIMIT, IO_SYNC, IO_UNIT, LK_EXCLUSIVE, LK_RETRY,
    VA_EXCLUSIVE, VBAD, VBLK, VCHR, VDIR, VFIFO, VLNK, VNON, VREAD, VREG, VSOCK, VTEXT, VWRITE,
    Vattr, Vnode, vn_lock,
};
use super::vops::{
    Cred, vop_abortop, vop_access, vop_advlock, vop_close, vop_create, vop_getattr, vop_ioctl,
    vop_kqfilter, vop_open, vop_read, vop_setattr, vop_unlock, vop_write,
};
use crate::assert::{kassert, kpanic};
use crate::event::Knote;
use crate::file::{FIF_HASLOCK, FO_POSITION, File, FileGlue, FileOps};
use crate::proc::{ProcGlue, Thread};
use crate::resource::RLIMIT_FSIZE;
use crate::termios::TIOCSCTTY;
use crate::uio::{Iovec, Uio, UioRw, UioSeg};

/// `vnops`: the file operations of a `DTYPE_VNODE` file.
pub struct VnodeFileOps;

/// The vnode a vnode file's object is (`(struct vnode *)fp->f_data`): `panic(9)` for another
/// object, which `vnops` is never installed with.
fn file_vnode<G: FileGlue>(fp: &File<G>) -> &'static Vnode<G::Proc> {
    match fp.data().and_then(G::vnode_of) {
        Some(vp) => vp,
        None => kpanic(format_args!("vnops: a file without a vnode")),
    }
}

/// `fp->f_cred`, as the vnode operations take it.
fn file_cred<G: FileGlue>(fp: &File<G>) -> Cred<'_> {
    Cred::real(fp.cred())
}

/// Common code for vnode open operations. Check permissions, and call the `VOP_OPEN` or
/// `VOP_CREATE` routine. `cmode` is the new file's mode under `O_CREAT`.
pub fn vn_open<G: ProcGlue>(
    ndp: &mut Nameidata<'_, G>,
    mut fmode: i32,
    cmode: u32,
) -> Result<(), Errno> {
    let p = ndp.ni_cnd.cn_proc;
    let cred = Cred::real(p.ucred());
    let mut va = Vattr::new();

    // The only valid flags to pass in here from NDINIT are KERNELPATH or BYPASSUNVEIL. This
    // function will override the nameiop based on the fmode and cmode flags, so validate that
    // our caller has not set other flags or operations in the nameidata structure.
    kassert!(ndp.ni_cnd.cn_flags & !(KERNELPATH | BYPASSUNVEIL) == 0);
    kassert!(ndp.ni_cnd.cn_nameiop == 0);

    if fmode & (FREAD | FWRITE) == 0 {
        return Err(Errno::EINVAL);
    }
    if fmode & (O_TRUNC | FWRITE) == O_TRUNC {
        return Err(Errno::EINVAL);
    }
    if fmode & (O_CREAT | O_DIRECTORY) == O_CREAT | O_DIRECTORY {
        return Err(Errno::EINVAL);
    }
    let vp: &'static Vnode<G>;
    if fmode & O_CREAT != 0 {
        ndp.ni_cnd.cn_nameiop = CREATE;
        ndp.ni_cnd.cn_flags |= LOCKPARENT | LOCKLEAF;
        if fmode & O_EXCL == 0 && fmode & O_NOFOLLOW == 0 {
            ndp.ni_cnd.cn_flags |= FOLLOW;
        }
        namei(ndp)?;
        let dvp = expect_vnode(ndp.ni_dvp);

        match ndp.ni_vp {
            None => {
                vattr_null(&mut va);
                va.va_type = VREG;
                va.va_mode = cmode;
                if fmode & O_EXCL != 0 {
                    va.va_vaflags |= VA_EXCLUSIVE;
                }
                let error = vop_create(dvp, &mut ndp.ni_vp, &mut ndp.ni_cnd, &mut va);
                vput(dvp);
                error?;
                fmode &= !O_TRUNC;
                vp = expect_vnode(ndp.ni_vp);
            }
            Some(nvp) => {
                let _ = vop_abortop(dvp, &mut ndp.ni_cnd);
                if ptr::eq(dvp, nvp) {
                    vrele(dvp);
                } else {
                    vput(dvp);
                }
                ndp.ni_dvp = None;
                vp = nvp;
                if fmode & O_EXCL != 0 {
                    vput(vp);
                    return Err(Errno::EEXIST);
                }
                if vp.vtype() == VDIR {
                    vput(vp);
                    return Err(Errno::EISDIR);
                }
                fmode &= !O_CREAT;
            }
        }
    } else {
        ndp.ni_cnd.cn_nameiop = LOOKUP;
        ndp.ni_cnd.cn_flags |= if fmode & O_NOFOLLOW != 0 {
            NOFOLLOW
        } else {
            FOLLOW
        } | LOCKLEAF;
        namei(ndp)?;
        vp = expect_vnode(ndp.ni_vp);
    }

    let error = 'bad: {
        if vp.vtype() == VSOCK {
            break 'bad Errno::EOPNOTSUPP;
        }
        if vp.vtype() == VLNK {
            break 'bad Errno::ELOOP;
        }
        if fmode & O_DIRECTORY != 0 && vp.vtype() != VDIR {
            break 'bad Errno::ENOTDIR;
        }
        if fmode & O_CREAT == 0 {
            if fmode & FREAD != 0
                && let Err(e) = vop_access(vp, VREAD, cred, p)
            {
                break 'bad e;
            }
            if fmode & FWRITE != 0 {
                if vp.vtype() == VDIR {
                    break 'bad Errno::EISDIR;
                }
                if let Err(e) = vn_writechk(vp).and_then(|()| vop_access(vp, VWRITE, cred, p)) {
                    break 'bad e;
                }
            }
        }
        if fmode & O_TRUNC != 0 && vp.vtype() == VREG {
            vattr_null(&mut va);
            va.va_size = 0;
            if let Err(e) = vop_setattr(vp, &mut va, cred, p) {
                break 'bad e;
            }
        }
        if let Err(e) = vop_open(vp, fmode, cred, p) {
            break 'bad e;
        }

        let mut vp = vp;
        // A cloning device's open made a clone (`VCLONED`): the parent's specinfo records it
        // (the C's `struct cloneinfo` in `v_data`), which is taken back here.
        if let Some(ci_vp) = super::spec::clone_restore(vp) {
            ndp.ni_vp = Some(ci_vp); // return cloned vnode
            let _ = vop_unlock(vp); // keep a reference
            vp = ci_vp; // for the increment below
        }

        if fmode & FWRITE != 0 {
            vp.set_writecount(vp.writecount() + 1);
        }
        return Ok(());
    };
    vput(vp);
    Err(error)
}

/// The vnode a successful `namei`/`VOP_CREATE` left in a `nameidata` slot.
fn expect_vnode<G: ProcGlue>(vp: Option<&'static Vnode<G>>) -> &'static Vnode<G> {
    match vp {
        Some(vp) => vp,
        None => kpanic(format_args!("vn_open: no vnode")),
    }
}

/// Check for write permissions on the specified vnode. Prototype text segments cannot be
/// written.
pub fn vn_writechk<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    // Disallow write attempts on read-only file systems; unless the file is a socket or a
    // block or character device resident on the file system.
    if let Some(mp) = vp.mount()
        && mp.flag() & MNT_RDONLY != 0
    {
        match vp.vtype() {
            VREG | VDIR | VLNK => return Err(Errno::EROFS),
            VNON | VCHR | VSOCK | VFIFO | VBAD | VBLK => {}
        }
    }
    // If there's shared text associated with the vnode, try to free it up once. If we fail,
    // we can't allow writing.
    if vp.flag() & VTEXT != 0 && !crate::vm::vnode::uvm_vnp_uncache(vp) {
        return Err(Errno::ETXTBSY);
    }

    Ok(())
}

/// Check whether a write operation would exceed the file size rlimit for the process, if
/// one should be applied for this operation. If a partial write should take place, the uio
/// is adjusted and the amount by which the request would have exceeded the limit is
/// returned. `p` is the uio's thread (`uio_procp`), pinned for the signal.
pub fn vn_fsizechk<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
    p: Option<Pin<&Thread<G>>>,
) -> Result<usize, Errno> {
    let mut overrun = 0;
    if vp.vtype() == VREG
        && let Some(p) = p
        && ioflag & IO_NOLIMIT == 0
    {
        let limit = crate::resource::lim_cur_proc(&p, RLIMIT_FSIZE).min(Off::MAX as u64) as Off;

        // if already at or over the limit, send the signal and fail
        if uio.uio_offset >= limit {
            crate::sig::psignal(p, SIGXFSZ);
            return Err(Errno::EFBIG);
        }

        // otherwise, clamp the write to stay under the limit
        let room = (limit - uio.uio_offset) as u64;
        if uio.uio_resid as u64 > room {
            overrun = (uio.uio_resid as u64 - room) as usize;
            uio.uio_resid = room as usize;
        }
    }

    Ok(overrun)
}

/// Mark a vnode as being the text image of a running process.
pub fn vn_marktext<G: ProcGlue>(vp: &'static Vnode<G>) {
    vp.flag_set(VTEXT);
}

/// Vnode close call.
pub fn vn_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    flags: i32,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    if flags & FWRITE != 0 {
        vp.set_writecount(vp.writecount() - 1);
    }
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = vop_close(vp, flags, cred, p);
    vput(vp);
    error
}

/// The buffer of `vn_rdwr` (`caddr_t base, int len, enum uio_seg segflg`).
pub enum RdwrBuf<'b> {
    /// A kernel buffer to read into (`UIO_SYSSPACE`, `UIO_READ`).
    Kernel(&'b mut [u8]),
    /// A kernel buffer to write from (`UIO_SYSSPACE`, `UIO_WRITE`).
    KernelRead(&'b [u8]),
    /// `len` bytes at the user address `addr` (`UIO_USERSPACE`), either way.
    User {
        /// The user address.
        addr: usize,
        /// The length.
        len: usize,
    },
}

/// Package up an I/O request on a vnode into a uio and do it: `rw` of the buffer at
/// `offset`; the bytes not moved in `aresid`, or `EIO` when some were not and no `aresid`
/// was given.
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn vn_rdwr<G: ProcGlue>(
    rw: UioRw,
    vp: &'static Vnode<G>,
    buf: RdwrBuf<'_>,
    offset: Off,
    ioflg: i32,
    cred: Cred<'_>,
    aresid: Option<&mut usize>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let mut aiov = [Iovec::new()];
    let mut auio = match buf {
        RdwrBuf::Kernel(b) if rw == UioRw::UIO_READ => Uio::kernel_read(&mut aiov, b, p),
        RdwrBuf::Kernel(b) => Uio::kernel_write(&mut aiov, b, p),
        RdwrBuf::KernelRead(b) if rw == UioRw::UIO_WRITE => Uio::kernel_write(&mut aiov, b, p),
        RdwrBuf::KernelRead(_) => kpanic(format_args!(
            "vn_rdwr: a read into a buffer lent for writing"
        )),
        RdwrBuf::User { addr, len } => {
            aiov[0] = Iovec {
                iov_base: ptr::without_provenance_mut(addr),
                iov_len: len,
            };
            Uio {
                uio_iov: &mut aiov,
                uio_offset: 0,
                uio_resid: len,
                uio_segflg: UioSeg::UIO_USERSPACE,
                uio_rw: rw,
                uio_procp: p,
            }
        }
    };
    auio.uio_offset = offset;

    if ioflg & IO_NODELOCKED == 0 {
        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    }
    let mut error = if rw == UioRw::UIO_READ {
        vop_read(vp, &mut auio, ioflg, cred)
    } else {
        vop_write(vp, &mut auio, ioflg, cred)
    };
    if ioflg & IO_NODELOCKED == 0 {
        let _ = vop_unlock(vp);
    }

    match aresid {
        Some(aresid) => *aresid = auio.uio_resid,
        None => {
            if auio.uio_resid != 0 && error.is_ok() {
                error = Err(Errno::EIO);
            }
        }
    }
    error
}

/// File table vnode read routine.
pub fn vn_read<G: FileGlue>(
    fp: &File<G>,
    uio: &mut Uio<'_, G::Proc>,
    fflags: i32,
) -> Result<(), Errno> {
    let vp = file_vnode(fp);
    let cred = file_cred(fp);
    let count = uio.uio_resid;

    klock::kernel_lock();

    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);

    let offset = if fflags & FO_POSITION == 0 {
        uio.uio_offset = fp.offset();
        uio.uio_offset
    } else {
        uio.uio_offset
    };

    let error = 'done: {
        // no wrap around of offsets except on character devices
        if vp.vtype() != VCHR && count as i64 > i64::MAX.wrapping_sub(offset) {
            break 'done Err(Errno::EINVAL);
        }

        if vp.vtype() == VDIR {
            break 'done Err(Errno::EISDIR);
        }

        let ioflag = if fp.flag() & FNONBLOCK != 0 {
            IO_NDELAY
        } else {
            0
        };
        let error = vop_read(vp, uio, ioflag, cred);
        if fflags & FO_POSITION == 0 {
            fp.advance_offset((count - uio.uio_resid) as Off);
        }
        error
    };
    let _ = vop_unlock(vp);
    klock::kernel_unlock();
    error
}

/// File table vnode write routine.
pub fn vn_write<G: FileGlue>(
    fp: &File<G>,
    uio: &mut Uio<'_, G::Proc>,
    fflags: i32,
) -> Result<(), Errno> {
    let vp = file_vnode(fp);
    let cred = file_cred(fp);
    let mut ioflag = IO_UNIT;

    klock::kernel_lock();

    // note: pwrite/pwritev are unaffected by O_APPEND
    if vp.vtype() == VREG && fp.flag() & O_APPEND != 0 && fflags & FO_POSITION == 0 {
        ioflag |= IO_APPEND;
    }
    if fp.flag() & FNONBLOCK != 0 {
        ioflag |= IO_NDELAY;
    }
    if fp.flag() & FFSYNC != 0
        || vp
            .mount()
            .is_some_and(|mp| mp.flag() & MNT_SYNCHRONOUS != 0)
    {
        ioflag |= IO_SYNC;
    }
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    if fflags & FO_POSITION == 0 {
        uio.uio_offset = fp.offset();
    }
    let count = uio.uio_resid;
    let error = vop_write(vp, uio, ioflag, cred);
    if fflags & FO_POSITION == 0 {
        if ioflag & IO_APPEND != 0 {
            fp.set_offset(uio.uio_offset);
        } else {
            fp.advance_offset((count - uio.uio_resid) as Off);
        }
    }
    let _ = vop_unlock(vp);

    klock::kernel_unlock();
    error
}

/// File table wrapper for `vn_stat`.
pub fn vn_statfile<G: FileGlue>(
    fp: &File<G>,
    sb: &mut Stat,
    p: &Thread<G::Proc>,
) -> Result<(), Errno> {
    let vp = file_vnode(fp);

    klock::kernel_lock();
    let error = vn_stat(vp, sb, p);
    klock::kernel_unlock();

    error
}

/// vnode stat routine.
pub fn vn_stat<G: ProcGlue>(
    vp: &'static Vnode<G>,
    sb: &mut Stat,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut va = Vattr::new();

    vop_getattr(vp, &mut va, Cred::real(p.ucred()), p)?;
    // Copy from vattr table
    *sb = Stat::default();
    sb.st_dev = va.va_fsid as i32;
    sb.st_ino = va.va_fileid;
    let mut mode = va.va_mode;
    mode |= match vp.vtype() {
        VREG => S_IFREG,
        VDIR => S_IFDIR,
        VBLK => S_IFBLK,
        VCHR => S_IFCHR,
        VLNK => S_IFLNK,
        VSOCK => S_IFSOCK,
        VFIFO => S_IFIFO,
        _ => return Err(Errno::EBADF),
    };
    sb.st_mode = mode;
    sb.st_nlink = va.va_nlink;
    sb.st_uid = va.va_uid;
    sb.st_gid = va.va_gid;
    sb.st_rdev = va.va_rdev;
    sb.st_size = va.va_size as Off;
    sb.st_atim = va.va_atime.into();
    sb.st_mtim = va.va_mtime.into();
    sb.st_ctim = va.va_ctime.into();
    sb.st_blksize = va.va_blocksize as i32;
    sb.st_flags = va.va_flags as u32;
    sb.st_gen = va.va_gen as u32;
    sb.st_blocks = (va.va_bytes / S_BLKSIZE as u64) as i64;
    Ok(())
}

/// File table vnode ioctl routine.
pub fn vn_ioctl<G: FileGlue>(
    fp: &File<G>,
    com: u64,
    data: &mut [u8],
    p: &Thread<G::Proc>,
) -> Result<(), Errno> {
    let vp = file_vnode(fp);
    let cred = Cred::real(p.ucred());
    let mut vattr = Vattr::new();
    let mut error = Err(Errno::ENOTTY);

    klock::kernel_lock();
    match vp.vtype() {
        VREG | VDIR => {
            if com == FIONREAD {
                error = vop_getattr(vp, &mut vattr, cred, p);
                if error.is_ok() {
                    let n = (vattr.va_size as i64).wrapping_sub(crate::file::foffset(fp));
                    if let Some(out) = data.get_mut(..size_of::<i32>()) {
                        out.copy_from_slice(&(n as i32).to_ne_bytes());
                    }
                }
            } else if com == FIOASYNC {
                // XXX
                error = Ok(()); // XXX
            }
        }

        VFIFO | VCHR | VBLK => {
            error = vop_ioctl(vp, com, data, fp.flag(), cred, p);
            if error.is_ok() && com == TIOCSCTTY {
                let ovp = {
                    let r = klock::reader();
                    p.process().session(&r).and_then(|s| {
                        let ovp = s.ttyvp.get();
                        s.ttyvp.set(Some(vp));
                        vref(vp);
                        ovp
                    })
                };
                // The reader is gone before vrele, which may sleep (VOP_INACTIVE).
                if let Some(ovp) = ovp {
                    vrele(ovp);
                }
            }
        }

        _ => {}
    }
    klock::kernel_unlock();

    error
}

/// File table vnode close routine.
pub fn vn_closefile<G: FileGlue>(fp: &File<G>, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
    let vp = file_vnode(fp);

    klock::kernel_lock();
    if fp.f_iflags.load(Ordering::SeqCst) & FIF_HASLOCK != 0 {
        let mut lf = Flock {
            l_whence: SEEK_SET as i16,
            l_start: 0,
            l_len: 0,
            l_type: F_UNLCK,
            ..Flock::default()
        };
        let _ = vop_advlock(
            vp,
            LockOwner::File(fp.addr()),
            i32::from(F_UNLCK),
            &mut lf,
            F_FLOCK,
        );
    }
    let error = vn_close(vp, fp.flag(), file_cred(fp), p);
    klock::kernel_unlock();
    error
}

/// `vn_kqfilter(fp, kn)`: the vnode's `VOP_KQFILTER`.
pub fn vn_kqfilter<G: FileGlue>(fp: &File<G>, kn: &Knote<G::Proc>) -> Result<(), Errno> {
    klock::kernel_lock();
    let error = vop_kqfilter(file_vnode(fp), fp.flag(), kn);
    klock::kernel_unlock();
    error
}

/// `vn_seek(fp, offset, whence, p)`: the file table's `lseek` of a vnode.
pub fn vn_seek<G: FileGlue>(
    fp: &File<G>,
    offset: &mut Off,
    whence: i32,
    p: &Thread<G::Proc>,
) -> Result<(), Errno> {
    let cred = Cred::real(p.ucred());
    let vp = file_vnode(fp);
    let mut vattr = Vattr::new();

    if vp.vtype() == VFIFO {
        return Err(Errno::ESPIPE);
    }

    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);

    let special = vp.vtype() == VCHR;

    let error = 'out: {
        let newoff = match whence {
            SEEK_CUR => fp.offset().wrapping_add(*offset),
            SEEK_END => {
                klock::kernel_lock();
                let error = vop_getattr(vp, &mut vattr, cred, p);
                klock::kernel_unlock();
                if let Err(e) = error {
                    break 'out Err(e);
                }
                offset.wrapping_add(vattr.va_size as Off)
            }
            SEEK_SET => *offset,
            _ => break 'out Err(Errno::EINVAL),
        };
        if !special && newoff < 0 {
            break 'out Err(Errno::EINVAL);
        }
        fp.set_offset(newoff);
        *offset = newoff;
        Ok(())
    };

    let _ = vop_unlock(vp);
    error
}

impl<G: FileGlue> FileOps<G> for VnodeFileOps {
    fn read(&self, fp: &File<G>, uio: &mut Uio<'_, G::Proc>, fflags: i32) -> Result<(), Errno> {
        vn_read(fp, uio, fflags)
    }
    fn write(&self, fp: &File<G>, uio: &mut Uio<'_, G::Proc>, fflags: i32) -> Result<(), Errno> {
        vn_write(fp, uio, fflags)
    }
    fn ioctl(
        &self,
        fp: &File<G>,
        com: u64,
        data: &mut [u8],
        p: &Thread<G::Proc>,
    ) -> Result<(), Errno> {
        vn_ioctl(fp, com, data, p)
    }
    fn kqfilter(&self, fp: &File<G>, kn: &Knote<G::Proc>) -> Result<(), Errno> {
        vn_kqfilter(fp, kn)
    }
    fn stat(&self, fp: &File<G>, st: &mut Stat, p: &Thread<G::Proc>) -> Result<(), Errno> {
        vn_statfile(fp, st, p)
    }
    fn close(&self, fp: &File<G>, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
        vn_closefile(fp, p)
    }
    fn seek(
        &self,
        fp: &File<G>,
        off: &mut Off,
        whence: i32,
        p: &Thread<G::Proc>,
    ) -> Result<(), Errno> {
        vn_seek(fp, off, whence, p)
    }
}

/// Common code for vnode access operations.
///
/// Check if a directory can be found inside another in the hierarchy.
pub fn vn_isunder<G: ProcGlue>(
    lvp: &'static Vnode<G>,
    rvp: &'static Vnode<G>,
    p: &Thread<G>,
) -> bool {
    vfs_getcwd_common(lvp, Some(rvp), None, (MAXPATHLEN / 2) as i32, 0, p).is_ok()
}
/* </CODE> */
/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the vnode operations over the kernel's test file system: `vn_open`'s
    // checks and errors (LZ's open_stat_close and the cases of the C), `vn_stat`'s types,
    // `vn_rdwr`'s three buffers, `vn_writechk`, `vn_fsizechk`'s clamp, and the file table's
    // operations (`VnodeFileOps`: read, seek, ioctl, stat, close).

    use core::sync::atomic::Ordering as Mem;

    use super::*;
    use crate::descrip::{falloc, fdinsert, fdplock};
    use crate::file::{DTYPE_VNODE, FileRef};
    use crate::fs::namei::NiDirp;
    use crate::fs::testfs::{
        A, F, S, SETATTRS, TestFiles, TestGlue, any_locked, node_of, test_thread, vget_node,
    };
    use crate::testutil::{TestFileData, USER_BASE};

    type G = TestGlue;

    /// `vn_open(path, fmode)`: the vnode, locked and referenced, or the error.
    fn open(
        p: &'static Thread<G>,
        path: &'static [u8],
        fmode: i32,
    ) -> Result<&'static Vnode<G>, Errno> {
        let mut nd = crate::fs::lookup::ndinit(0, 0, NiDirp::Sys(path), p);
        vn_open(&mut nd, fmode, 0o644)?;
        Ok(nd.ni_vp.unwrap())
    }

    /// `vn_close` of a vnode `open` returned (locked).
    fn close(p: &'static Thread<G>, vp: &'static Vnode<G>, fmode: i32) {
        let _ = vop_unlock(vp);
        vn_close(vp, fmode, Cred::real(p.ucred()), Some(p)).unwrap();
    }

    #[test]
    fn open_stat_close() {
        let (_g, p) = test_thread();
        let vp = open(p, b"/f", FREAD).unwrap();
        assert!(node_of(vp).is_some_and(|n| n.idx == F));
        let uses = vp.usecount();
        let mut st = Stat::default();
        vn_stat(vp, &mut st, p).unwrap();
        assert_eq!(st.st_mode, S_IFREG | 0o755);
        assert_eq!(
            (st.st_ino, st.st_size, st.st_blksize, st.st_blocks),
            (105, 512, 512, 2)
        );
        assert_eq!((st.st_dev, st.st_nlink), (99, 1));
        close(p, vp, FREAD);
        assert_eq!(vp.usecount(), uses - 1);
        assert!(!any_locked());

        // The types: a directory, a socket.
        let mp = vp.mount().unwrap();
        let dvp = vget_node(mp, A).unwrap();
        vn_stat(dvp, &mut st, p).unwrap();
        assert_eq!(st.st_mode & S_IFDIR, S_IFDIR);
        vput(dvp);
        let svp = vget_node(mp, S).unwrap();
        vn_stat(svp, &mut st, p).unwrap();
        assert_eq!(st.st_mode & 0o170000, S_IFSOCK);
        vput(svp);
    }

    #[test]
    fn open_refuses_what_the_c_refuses() {
        let (_g, p) = test_thread();
        // The flags.
        assert_eq!(open(p, b"/f", 0).err(), Some(Errno::EINVAL));
        assert_eq!(open(p, b"/f", FREAD | O_TRUNC).err(), Some(Errno::EINVAL));
        assert_eq!(
            open(p, b"/a", FREAD | O_CREAT | O_DIRECTORY).err(),
            Some(Errno::EINVAL)
        );
        // What the name is.
        assert_eq!(
            open(p, b"/f", FREAD | O_CREAT | O_EXCL).err(),
            Some(Errno::EEXIST)
        );
        assert_eq!(open(p, b"/a", FREAD | O_CREAT).err(), Some(Errno::EISDIR));
        assert_eq!(open(p, b"/a", FWRITE).err(), Some(Errno::EISDIR));
        assert_eq!(
            open(p, b"/f", FREAD | O_DIRECTORY).err(),
            Some(Errno::ENOTDIR)
        );
        assert_eq!(open(p, b"/s", FREAD).err(), Some(Errno::EOPNOTSUPP));
        assert_eq!(open(p, b"/nope", FREAD).err(), Some(Errno::ENOENT));
        // A creation the file system cannot make, and one its directory refuses.
        assert_eq!(
            open(p, b"/a/new", FWRITE | O_CREAT).err(),
            Some(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            open(p, b"/a/c/d/new", FWRITE | O_CREAT).err(),
            Some(Errno::EACCES)
        );
        assert!(!any_locked());

        // O_TRUNC of a regular file truncates it (VOP_SETATTR), and a writer is counted.
        let before = SETATTRS.load(Mem::SeqCst);
        let vp = open(p, b"/f", FWRITE | O_TRUNC).unwrap();
        assert_eq!(SETATTRS.load(Mem::SeqCst), before + 1);
        assert_eq!(vp.writecount(), 1);
        close(p, vp, FWRITE);
        assert_eq!(vp.writecount(), 0);
        // On a read-only file system a regular file is not writable (vn_writechk).
        let mp = vp.mount().unwrap();
        mp.flag_set(MNT_RDONLY);
        assert_eq!(open(p, b"/f", FWRITE).err(), Some(Errno::EROFS));
        assert_eq!(vn_writechk(vp), Err(Errno::EROFS));
        mp.flag_clr(MNT_RDONLY);
        assert_eq!(vn_writechk(vp), Ok(()));
        assert!(!any_locked());
    }

    #[test]
    fn rdwr_takes_a_kernel_slice_or_a_user_range() {
        let (_g, p) = test_thread();
        let vp = open(p, b"/f", FREAD).unwrap();
        let _ = vop_unlock(vp);
        let cred = Cred::real(p.ucred());
        // The test file reads 64 bytes at most: 36 of 100 are left.
        let mut buf = [0u8; 100];
        let mut resid = 0;
        vn_rdwr(
            UioRw::UIO_READ,
            vp,
            RdwrBuf::Kernel(&mut buf),
            0,
            0,
            cred,
            Some(&mut resid),
            Some(p),
        )
        .unwrap();
        assert_eq!(resid, 36);
        // Without aresid a short transfer is EIO.
        assert_eq!(
            vn_rdwr(
                UioRw::UIO_READ,
                vp,
                RdwrBuf::Kernel(&mut buf),
                0,
                0,
                cred,
                None,
                Some(p)
            ),
            Err(Errno::EIO)
        );
        // A user range gets the bytes.
        let ub = USER_BASE + 0x6800;
        vn_rdwr(
            UioRw::UIO_READ,
            vp,
            RdwrBuf::User { addr: ub, len: 64 },
            0,
            0,
            cred,
            None,
            Some(p),
        )
        .unwrap();
        let mut out = [0u8; 64];
        hal::user::UserPtr::<u8>::new(ub)
            .copyin_slice(&mut out)
            .unwrap();
        assert!(out.iter().all(|&c| c == b'x'));
        // A write from a kernel slice: the test file system has no VOP_WRITE.
        assert_eq!(
            vn_rdwr(
                UioRw::UIO_WRITE,
                vp,
                RdwrBuf::KernelRead(&buf),
                0,
                0,
                cred,
                None,
                Some(p)
            ),
            Err(Errno::EOPNOTSUPP)
        );
        assert!(!any_locked());
        vn_close(vp, FREAD, cred, Some(p)).unwrap();
    }

    #[test]
    fn fsizechk_clamps_a_write_at_the_limit() {
        let (_g, p) = test_thread();
        let mut limits = crate::resource::Plimit::new();
        for l in &mut limits.pl_rlimit {
            l.rlim_cur = crate::resource::RLIM_INFINITY;
            l.rlim_max = crate::resource::RLIM_INFINITY;
        }
        limits.pl_rlimit[RLIMIT_FSIZE].rlim_cur = 1000;
        drop(
            p.process()
                .set_limit(crate::resource::PlimitRef::new(limits)),
        );
        let vp = open(p, b"/f", FREAD).unwrap();
        let pin = hal::pin::pin_static(p);
        let mut iov = [Iovec::new()];
        let buf = [0u8; 200];
        let mut uio = Uio::kernel_write(&mut iov, &buf, Some(p));
        uio.uio_offset = 900;
        assert_eq!(vn_fsizechk(vp, &mut uio, 0, Some(pin)), Ok(100));
        assert_eq!(uio.uio_resid, 100);
        // No limit with IO_NOLIMIT, nor without a thread, nor for a directory.
        let mut uio = Uio::kernel_write(&mut iov, &buf, Some(p));
        uio.uio_offset = 900;
        assert_eq!(vn_fsizechk(vp, &mut uio, IO_NOLIMIT, Some(pin)), Ok(0));
        assert_eq!(vn_fsizechk(vp, &mut uio, 0, None), Ok(0));
        assert_eq!(uio.uio_resid, 200);
        close(p, vp, FREAD);
    }

    /// A vnode file of `p` (descriptor and reference) on `vp`, as `vn_open`'s caller makes
    /// one.
    fn file_on(
        p: &'static Thread<G>,
        vp: &'static Vnode<G>,
        flag: i32,
    ) -> (FileRef<TestFiles>, i32) {
        let mut g = fdplock(p.fd());
        let (fp, fd) = falloc(p, &mut g).unwrap();
        fp.f_flag.store(flag as u32, Mem::SeqCst);
        fp.install(
            DTYPE_VNODE,
            &VnodeFileOps,
            TestFileData {
                kvnode: Some(vp),
                ..TestFileData::default()
            },
        );
        fdinsert(&mut g, fd, 0, &fp);
        (fp, fd)
    }

    #[test]
    fn the_file_operations_of_a_vnode_file() {
        let (_g, p) = test_thread();
        let vp = open(p, b"/f", FREAD).unwrap();
        let _ = vop_unlock(vp);
        let (fp, _fd) = file_on(p, vp, FREAD);
        // read: the offset advances by what moved.
        let ub = USER_BASE + 0x6800;
        let mut iov = [Iovec {
            iov_base: core::ptr::without_provenance_mut(ub),
            iov_len: 8,
        }];
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: 8,
            uio_segflg: UioSeg::UIO_USERSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: Some(p),
        };
        VnodeFileOps.read(&fp, &mut uio, 0).unwrap();
        assert_eq!(fp.offset(), 8);
        // seek: SET, CUR, END (the size, 512), and the refusals.
        let mut off = 10;
        VnodeFileOps.seek(&fp, &mut off, SEEK_SET, p).unwrap();
        assert_eq!((off, fp.offset()), (10, 10));
        let mut off = 5;
        VnodeFileOps.seek(&fp, &mut off, SEEK_CUR, p).unwrap();
        assert_eq!(fp.offset(), 15);
        let mut off = -2;
        VnodeFileOps.seek(&fp, &mut off, SEEK_END, p).unwrap();
        assert_eq!(fp.offset(), 510);
        assert_eq!(VnodeFileOps.seek(&fp, &mut 0, 9, p), Err(Errno::EINVAL));
        assert_eq!(
            VnodeFileOps.seek(&fp, &mut -1, SEEK_SET, p),
            Err(Errno::EINVAL)
        );
        // ioctl: FIONREAD is what is left after the offset.
        let mut data = [0u8; 4];
        VnodeFileOps.ioctl(&fp, FIONREAD, &mut data, p).unwrap();
        assert_eq!(i32::from_ne_bytes(data), 512 - 510);
        assert_eq!(VnodeFileOps.ioctl(&fp, FIOASYNC, &mut data, p), Ok(()));
        assert_eq!(
            VnodeFileOps.ioctl(&fp, 0x1234, &mut data, p),
            Err(Errno::ENOTTY)
        );
        // stat.
        let mut st = Stat::default();
        VnodeFileOps.stat(&fp, &mut st, p).unwrap();
        assert_eq!(st.st_ino, 105);
        // close: the vnode's reference goes (vn_close).
        let uses = vp.usecount();
        VnodeFileOps.close(&fp, Some(p)).unwrap();
        assert_eq!(vp.usecount(), uses - 1);
        assert!(!any_locked());
        core::mem::forget(fp);
    }
}
/* </TESTS> */
