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
 * Copyright (c) 1982, 1986, 1990, 1993
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
 *	@(#)sys_socket.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The file operations of sockets: `kern/sys_socket.c`, the kernel's
//! (`kernel::net::file::SocketFileOps`) in `bsd`'s `struct fileops` table.
//!
//! Upstream: sys/kern/sys_socket.c @ 3ce1f3f79392
//!
//! A `DTYPE_SOCKET` file's object is its counted socket (`BsdFileData::Socket`);
//! `socketops` turns `read(2)`, `write(2)`, `ioctl(2)`, `kevent(2)`, `fstat(2)` and the
//! last `close(2)` into the kernel's socket file operations.
//!
//! ## Deviations
//! - [`fp_socket`] is `fp->f_data` of a socket file as the `&'static Socket` LZ's callers
//!   keep (the file's reference keeps the socket).

use kernel::file::FileOps;
use kernel::net::file::SocketFileOps;

use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::file::{DTYPE_SOCKET, File, Fileops};
use crate::sys::proc::Proc;
use crate::sys::socketvar::{Socket, as_static};
use crate::sys::stat::Stat;
use crate::sys::uio::Uio;

/// The kernel's operations of a socket file.
static OPS: SocketFileOps<BsdProcGlue> = SocketFileOps::NEW;

/// `socketops`.
pub static SOCKETOPS: Fileops = Fileops {
    fo_read: soo_read,
    fo_write: soo_write,
    fo_ioctl: soo_ioctl,
    fo_kqfilter: soo_kqfilter,
    fo_stat: soo_stat,
    fo_close: soo_close,
    fo_seek: None,
};

/// `fp->f_type = DTYPE_SOCKET; fp->f_ops = &socketops; fp->f_data = so`: `so`'s reference
/// becomes the file's (its last close is the socket's `soclose`).
pub fn fp_install(fp: &File, so: crate::sys::socketvar::SocketRef) {
    let (ops, data) =
        <crate::kern::kern_descrip::BsdFileGlue as kernel::file::FileGlue>::socket_file(so);
    fp.install(DTYPE_SOCKET, ops, data);
}

/// `(struct socket *)fp->f_data` of a socket file.
pub fn fp_socket(fp: &File) -> &'static Socket {
    match fp.data() {
        Some(crate::kern::kern_descrip::BsdFileData::Socket(so)) if fp.f_type() == DTYPE_SOCKET => {
            as_static(so)
        }
        _ => panic(format_args!("file {:#x}: not a socket", fp.addr())),
    }
}

/// `soo_read(fp, uio, fflags)`.
pub fn soo_read(fp: &File, uio: &mut Uio<'_>, fflags: i32) -> Result<(), Errno> {
    OPS.read(fp, uio, fflags)
}

/// `soo_write(fp, uio, fflags)`.
pub fn soo_write(fp: &File, uio: &mut Uio<'_>, fflags: i32) -> Result<(), Errno> {
    OPS.write(fp, uio, fflags)
}

/// `soo_ioctl(fp, cmd, data, p)`.
pub fn soo_ioctl(fp: &File, cmd: u64, data: &mut [u8], p: &Proc) -> Result<(), Errno> {
    OPS.ioctl(fp, cmd, data, p)
}

/// `soo_kqfilter(fp, kn)`.
pub fn soo_kqfilter(fp: &File, kn: &Knote) -> Result<(), Errno> {
    OPS.kqfilter(fp, kn)
}

/// `soo_stat(fp, ub, p)`.
pub fn soo_stat(fp: &File, ub: &mut Stat, p: &Proc) -> Result<(), Errno> {
    OPS.stat(fp, ub, p)
}

/// `soo_close(fp, p)`.
pub fn soo_close(fp: &File, p: Option<&Proc>) -> Result<(), Errno> {
    OPS.close(fp, p)
}
/// The interface layer's legs of the kernel's socket layer (the `IfLegacy` row of
/// `bridges.toml`): `if.c`'s `ifioctl` and `net_tq`, `bsd`'s until N6.3a.
pub struct BsdIfLegacy;

/// The one [`BsdIfLegacy`], registered by `domaininit`.
pub static IFLEGACY: BsdIfLegacy = BsdIfLegacy;

impl kernel::net::domain::IfLegacy<BsdProcGlue> for BsdIfLegacy {
    /// `ifioctl(so, cmd, data, p)` over an aligned copy of the request (`sys_ioctl`'s
    /// buffer, at most `STK_PARAMS` bytes; `ifioctl` reads the structure the command names).
    fn ifioctl(&self, so: &Socket, cmd: u64, data: &mut [u8], p: &Proc) -> Result<(), Errno> {
        let mut aligned = [0u64; 16];
        let len = data.len().min(size_of_val(&aligned));
        hal::user::bytes_of_mut(&mut aligned[..])[..len].copy_from_slice(&data[..len]);
        let bytes = hal::user::bytes_of_mut(&mut aligned[..]).as_mut_ptr();
        // SAFETY: `bytes` names `aligned`'s 128 bytes, a local buffer aligned for any
        // interface request and as long as `sys_ioctl`'s, holding the request's `len` bytes;
        // `ifioctl` reads and writes at most the size the command names (`IOCPARM_LEN(cmd)`,
        // which `sys_ioctl` bounded by the buffer it copied) and keeps no pointer past the
        // call; `so` is the caller's socket, alive across the call.
        let error =
            unsafe { crate::net::if_::ifioctl(core::ptr::from_ref(so).cast(), cmd, bytes, p) };
        data[..len].copy_from_slice(&hal::user::bytes_of(&aligned[..])[..len]);
        error
    }

    /// `net_tq(flowid)`.
    fn net_tq(&self, flowid: u32) -> &'static kernel::task::Taskq {
        match crate::net::if_::net_tq(flowid) {
            Some(tq) => tq,
            None => panic(format_args!("net_tq: no softnet queue")),
        }
    }
}
/* </CODE> */
