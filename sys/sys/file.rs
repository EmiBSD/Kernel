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
 *	@(#)file.h	8.2 (Berkeley) 8/20/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/file.h>` for `bsd`: the kernel's open file (`kernel::file`) bound to `bsd`'s glue,
//! `struct fileops` as `bsd`'s operations vectors are written, and the reference macros.
//!
//! Upstream: sys/sys/file.h @ 3ce1f3f79392
//! LZ: sys/sys/file.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.3: `struct file` moved into `kernel::file` (a counted pool item, its object installed
//!   once); this module names its one instance (`File`, `FileRef` over `BsdFileGlue`,
//!   `sys/kern/kern_descrip.rs`), re-exports the `DTYPE_*`, `FIF_*` and `FO_POSITION`
//!   constants, and keeps `struct fileops` as `bsd`'s vectors are written (`VNOPS`,
//!   `SOCKETOPS`, `PIPEOPS`, `KQUEUEOPS`, `CONSFILEOPS`): the kernel calls them through
//!   [`FileOps`](kernel::file::FileOps), implemented here over the function pointers.
//!   `FREF`/`FRELE` are [`fref`]/[`frele`] over the handle, and the object of a file is read
//!   through [`FileExt`] (a vnode) and the subsystems' own accessors (`fp_socket`,
//!   `fp_kqueue`, `pipe_of`), by reference: `f_data` is no longer cast.

use kernel::file::FileOps;

pub use kernel::file::{
    DTYPE_DMABUF, DTYPE_KQUEUE, DTYPE_PIPE, DTYPE_SOCKET, DTYPE_SYNC, DTYPE_VMM, DTYPE_VNODE,
    FDUP_MAX_COUNT, FIF_HASLOCK, FIF_INSERTED, FO_POSITION, FileStats,
};

use crate::kern::kern_descrip::{BsdFileData, BsdFileGlue};
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::proc::Proc;
use crate::sys::stat::Stat;
use crate::sys::types::Off;
use crate::sys::ucred::Ucred;
use crate::sys::uio::Uio;
use crate::sys::vnode::Vnode;

/// `struct file` of `bsd`'s descriptors.
pub type File = kernel::file::File<BsdFileGlue>;

/// A counted reference to a `struct file` (`f_count`).
pub type FileRef = kernel::file::FileRef<BsdFileGlue>;

/// `struct fileops`: file operations. `fo_read`, `fo_write` and `fo_close` could be called
/// without the kernel lock held.
pub struct Fileops {
    /// `fo_read(fp, uio, fflags)`.
    pub fo_read: fn(&File, &mut Uio<'_>, i32) -> Result<(), Errno>,
    /// `fo_write(fp, uio, fflags)`.
    pub fo_write: fn(&File, &mut Uio<'_>, i32) -> Result<(), Errno>,
    /// `fo_ioctl(fp, com, data, p)`: `data` is the kernel copy of the argument.
    pub fo_ioctl: fn(&File, u64, &mut [u8], &Proc) -> Result<(), Errno>,
    /// `fo_kqfilter(fp, kn)`: attaches the knote to the file's object (`kern_event.c`).
    pub fo_kqfilter: fn(&File, &Knote) -> Result<(), Errno>,
    /// `fo_stat(fp, ub, p)`.
    pub fo_stat: fn(&File, &mut Stat, &Proc) -> Result<(), Errno>,
    /// `fo_close(fp, p)`: `p` is `None` when closing a file that was in a message.
    pub fo_close: fn(&File, Option<&Proc>) -> Result<(), Errno>,
    /// `fo_seek(fp, offset, whence, p)`, NULL for the types that cannot seek.
    pub fo_seek: Option<FoSeek>,
}

/// The type of `fo_seek(fp, offset, whence, p)`.
pub type FoSeek = fn(&File, &mut Off, i32, &Proc) -> Result<(), Errno>;

impl FileOps<BsdFileGlue> for Fileops {
    fn read(&self, fp: &File, uio: &mut Uio<'_>, fflags: i32) -> Result<(), Errno> {
        (self.fo_read)(fp, uio, fflags)
    }

    fn write(&self, fp: &File, uio: &mut Uio<'_>, fflags: i32) -> Result<(), Errno> {
        (self.fo_write)(fp, uio, fflags)
    }

    fn ioctl(&self, fp: &File, com: u64, data: &mut [u8], p: &Proc) -> Result<(), Errno> {
        (self.fo_ioctl)(fp, com, data, p)
    }

    fn kqfilter(&self, fp: &File, kn: &Knote) -> Result<(), Errno> {
        (self.fo_kqfilter)(fp, kn)
    }

    fn stat(&self, fp: &File, st: &mut Stat, p: &Proc) -> Result<(), Errno> {
        (self.fo_stat)(fp, st, p)
    }

    fn close(&self, fp: &File, p: Option<&Proc>) -> Result<(), Errno> {
        (self.fo_close)(fp, p)
    }

    fn seek(&self, fp: &File, off: &mut Off, whence: i32, p: &Proc) -> Result<(), Errno> {
        match self.fo_seek {
            Some(seek) => seek(fp, off, whence, p),
            None => Err(Errno::ESPIPE),
        }
    }
}

/// What `bsd` reads of a file's object and credentials.
pub trait FileExt {
    /// `(struct vnode *)fp->f_data` of a `DTYPE_VNODE` file (`panic(9)` for another).
    fn vnode(&self) -> &'static Vnode;
    /// `fp->f_cred`, as the vnode layer takes it (a pointer the file keeps alive).
    fn f_cred_ptr(&self) -> *const Ucred;
}

impl FileExt for File {
    fn vnode(&self) -> &'static Vnode {
        match self.data() {
            Some(BsdFileData::Vnode(vp)) if self.f_type() == DTYPE_VNODE => match vp.get() {
                Some(vp) => vp,
                None => {
                    crate::kern::subr_prf::panic(format_args!("file {:#x}: no vnode", self.addr()))
                }
            },
            _ => crate::kern::subr_prf::panic(format_args!("file {:#x}: not a vnode", self.addr())),
        }
    }

    fn f_cred_ptr(&self) -> *const Ucred {
        kernel::cred::CredRef::as_ptr(self.cred())
    }
}

/// `FREF(fp)`: one more reference to `fp`.
pub fn fref(fp: &FileRef) -> FileRef {
    fp.fref()
}

/// `FRELE(fp, p)`: gives a reference back; the last one closes the file (`fdrop`).
pub fn frele<'a>(fp: FileRef, p: impl Into<Option<&'a Proc>>) -> Result<(), Errno> {
    fp.frele(p.into())
}

/// `foffset(fp)`: the file's offset, read under `f_mtx`.
pub fn foffset(fp: &File) -> Off {
    kernel::file::foffset(fp)
}
/* </CODE> */
