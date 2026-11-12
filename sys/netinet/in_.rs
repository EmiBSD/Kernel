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
 */

/*
 * Copyright (C) 2001 WIDE Project.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*
 * Copyright (c) 1982, 1986, 1991, 1993
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
 *	@(#)in.c	8.2 (Berkeley) 11/15/93
 */
/* </LICENSES> */

/* <CODE> */
//! Constants and structures defined by the internet system, per RFC 790, September 1981, and
//! numerous additions: `<netinet/in.h>` (`hal::sys::netinet_in`, re-exported whole); and
//! `netinet/in.c`, the Internet addresses of the interfaces, the kernel's since N6.4a
//! (`kernel::netinet::in_`), with LZ's shapes for `bsd`'s callers.
//!
//! Upstream: sys/netinet/in.h @ 3ce1f3f79392
//! Upstream: sys/netinet/in.c @ 3ce1f3f79392
//!
//! `in` is a Rust keyword, so the module is `in_` (`docs/C_TO_RUST.md`); `in.c` shares it, as
//! `if.c` shares `if_`.
//!
//! ## Deviations
//! - `satosin`, `satosin_const` and `sintosa` are pointer casts (`docs/C_TO_RUST.md`) for
//!   `bsd`'s raw socket addresses; `in_nam2sin` checks through the kernel's and answers the
//!   pointer into the caller's mbuf, as LZ's callers use it.
//! - `in_control` has the `pru_control` signature (`sys/protosw.rs`): the socket's `SS_PRIV`
//!   and the kernel copy of the request, handed to the kernel's `in_control`; `in_ioctl` is
//!   the raw request of the kernel's own callers and tests.
//! - `in_addmulti` and `in_delmulti` keep LZ's signatures, the net lock's proof made through
//!   the `SaToken` (their callers hold it: `ip_setmoptions` under the socket lock, pfsync).

use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, mtod};
use crate::sys::socket::Sockaddr;
use crate::sys::socketvar::{SS_PRIV, Socket};

pub use hal::sys::netinet_in::*;
pub use kernel::netinet::in_::{
    in_broadcast, in_canforward, in_hasmulti, in_ifp2ia, in_len2mask, in_lookupmulti, in_mask2len,
    in_prefixlen2mask, in_socktrim,
};

/// `satosin(sa)`: a generic `sockaddr` seen as a `sockaddr_in`.
pub const fn satosin(sa: *mut Sockaddr) -> *mut SockaddrIn {
    sa.cast()
}

/// `satosin_const(sa)`: a generic `sockaddr` seen as a `sockaddr_in`, read-only.
pub const fn satosin_const(sa: *const Sockaddr) -> *const SockaddrIn {
    sa.cast()
}

/// `sintosa(sin)`: a `sockaddr_in` seen as a generic `sockaddr`.
pub const fn sintosa(sin: *mut SockaddrIn) -> *mut Sockaddr {
    sin.cast()
}

/// `in_nam2sin(nam, &sin)`: the `sockaddr_in` in mbuf `nam`, checked by the kernel's.
pub fn in_nam2sin(nam: &Mbuf) -> Result<*mut SockaddrIn, Errno> {
    kernel::netinet::in_::in_nam2sin(nam)?;
    Ok(satosin(mtod::<Sockaddr>(nam)))
}

/// `in_control(so, cmd, data, ifp)`: the `pru_control` of the Internet protocols: the
/// address `ioctl`s on interface `ifp` (the kernel's), privileged as the socket is.
pub fn in_control(
    so: &'static Socket,
    cmd: u64,
    data: &mut [u8],
    ifp: Option<&'static crate::net::if_var::Ifnet>,
) -> Result<(), Errno> {
    kernel::netinet::in_::in_control(so.has_state(SS_PRIV), cmd, data, ifp)
}

/// `in_ioctl(cmd, data, ifp, privileged)`: the Internet address `ioctl`s (the kernel's) over
/// a raw request.
///
/// # Safety
///
/// `data` points at the kernel copy of the request, as long as the structure `cmd` encodes,
/// exclusively the caller's for the call.
pub unsafe fn in_ioctl(
    cmd: u64,
    data: *mut u8,
    ifp: Option<&'static crate::net::if_var::Ifnet>,
    privileged: bool,
) -> Result<(), Errno> {
    let len = crate::sys::ioccom::iocparm_len(cmd) as usize;
    // SAFETY: the caller's contract: `len` bytes at `data`, the caller's alone for the call.
    let req = unsafe { core::slice::from_raw_parts_mut(data, len) };
    kernel::netinet::in_::in_ioctl(cmd, req, ifp, privileged)
}

/// `in_addmulti(addr, ifp)`: the kernel's, under the net lock the caller holds.
pub fn in_addmulti(
    addr: &InAddr,
    ifp: &'static crate::net::if_var::Ifnet,
) -> Option<&'static kernel::netinet::in_var::InMulti> {
    kernel::netinet::in_::in_addmulti(addr, ifp, &crate::net::if_::net_held("in_addmulti"))
}

/// `in_delmulti(inm)`: the kernel's, under the net lock the caller holds.
pub fn in_delmulti(inm: &'static kernel::netinet::in_var::InMulti) {
    kernel::netinet::in_::in_delmulti(inm, &crate::net::if_::net_held("in_delmulti"));
}
/* </CODE> */
