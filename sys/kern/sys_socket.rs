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
/// The bytes `ifioctl` works on for `cmd`: the structure the command names
/// (`IOCPARM_LEN(cmd)`, up to `IOCPARM_MAX`, which `sys_ioctl` copied whole) and at least the
/// `struct ifreq` every interface request starts with (its name is read first).
fn ifioctl_need(cmd: u64) -> usize {
    (hal::sys::ioccom::iocparm_len(cmd) as usize).max(size_of::<crate::net::if_::Ifreq>())
}

/// Where `ifioctl` works on `data` for `cmd`: `None` for `data` itself, when it holds
/// [`ifioctl_need`] bytes aligned for `u64` (the kernel's stack buffer and its heap ones of
/// the right length); else a zeroed heap copy of `data` at least that long, `u64`-aligned,
/// whose first `data.len()` bytes go back to `data` after the call. Never a shorter one.
fn ifioctl_buffer(data: &[u8], cmd: u64) -> Result<Option<hal::malloc::MArray<u64>>, Errno> {
    let need = ifioctl_need(cmd);
    if data.len() >= need && data.as_ptr().align_offset(align_of::<u64>()) == 0 {
        return Ok(None);
    }
    let words = need.max(data.len()).div_ceil(size_of::<u64>());
    let Some(mut c) = hal::malloc::MArray::new(
        crate::sys::malloc::M_IOCTLOPS,
        crate::sys::malloc::M_WAITOK,
        words,
        |_| 0u64,
    ) else {
        return Err(Errno::ENOMEM);
    };
    hal::user::bytes_of_mut(&mut c[..])[..data.len()].copy_from_slice(data);
    Ok(Some(c))
}

/// The interface layer's legs of the kernel's socket layer (the `IfLegacy` row of
/// `bridges.toml`): `if.c`'s `ifioctl` and `net_tq`, `bsd`'s until N6.3a.
pub struct BsdIfLegacy;

/// The one [`BsdIfLegacy`], registered by `domaininit`.
pub static IFLEGACY: BsdIfLegacy = BsdIfLegacy;

impl kernel::net::domain::IfLegacy<BsdProcGlue> for BsdIfLegacy {
    /// `ifioctl(so, cmd, data, p)` on `sys_ioctl`'s copy of the request, in place when it is
    /// as long as `ifioctl` works on and aligned for it (the kernel's stack buffer is), else
    /// through a heap copy of that length ([`ifioctl_buffer`]).
    fn ifioctl(&self, so: &Socket, cmd: u64, data: &mut [u8], p: &Proc) -> Result<(), Errno> {
        let mut copy = ifioctl_buffer(data, cmd)?;
        let bytes = match copy.as_mut() {
            Some(c) => hal::user::bytes_of_mut(&mut c[..]).as_mut_ptr(),
            None => data.as_mut_ptr(),
        };
        // SAFETY: `bytes` points at `ifioctl_need(cmd)` or more bytes aligned for `u64`
        // (`ifioctl_buffer`'s promise: `data` itself, checked for both, or the heap copy it
        // made that long), so the request structure `ifioctl` casts it to (`IOCPARM_LEN(cmd)`
        // bytes, at least the `struct ifreq` every interface request starts with) is in
        // bounds and aligned, and nothing else uses the bytes during the call (`data` is
        // ours exclusively, the copy local); `ifioctl` keeps no pointer past the call; `so`
        // is the caller's socket, alive across it.
        let error =
            unsafe { crate::net::if_::ifioctl(core::ptr::from_ref(so).cast(), cmd, bytes, p) };
        if let Some(c) = &copy {
            data.copy_from_slice(&hal::user::bytes_of(&c[..])[..data.len()]);
        }
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

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use super::*;
    use crate::sys::sockio::SIOCGLIFPHYADDR;

    #[test]
    fn ifioctl_works_on_the_whole_request() {
        let (_g, _t, p) = crate::netinet::in_pcb::tests::setup();
        // A request longer than STK_PARAMS (536 bytes): sys_ioctl's heap buffer.
        let need = ifioctl_need(SIOCGLIFPHYADDR);
        assert!(need > 128, "{need}");
        let mut words: Vec<u64> = std::vec![0; need.div_ceil(8) + 1];
        let bytes = hal::user::bytes_of_mut(&mut words[..]);
        // Aligned and long enough: worked on in place.
        assert!(
            ifioctl_buffer(&bytes[..need], SIOCGLIFPHYADDR)
                .unwrap()
                .is_none()
        );
        // Misaligned: a copy, never shorter than the request.
        let c = ifioctl_buffer(&bytes[1..1 + need], SIOCGLIFPHYADDR)
            .unwrap()
            .expect("a copy");
        assert!(c.len() * 8 >= need);
        // A void command's 8 bytes: a copy as long as a struct ifreq.
        let c = ifioctl_buffer(&bytes[..8], 0x2000_6900)
            .unwrap()
            .expect("a copy");
        assert!(c.len() * 8 >= size_of::<crate::net::if_::Ifreq>());

        // Through the bridge, both ways, on an interface that is not there.
        let so = crate::kern::uipc_socket::soalloc(
            &crate::netinet::in_proto::INETSW[1],
            crate::sys::socket::SOCK_DGRAM,
            crate::sys::mbuf::M_WAIT,
        )
        .unwrap();
        for at in [0usize, 1] {
            let data = &mut bytes[at..at + need];
            data[..8].copy_from_slice(b"nosuch0\0");
            let r = kernel::net::domain::IfLegacy::ifioctl(&IFLEGACY, so, SIOCGLIFPHYADDR, data, p);
            assert_eq!(r, Err(Errno::ENXIO), "at {at}");
            assert_eq!(&data[..8], b"nosuch0\0");
        }
        crate::netinet::in_pcb::tests::teardown();
    }
}
/* </TESTS> */
