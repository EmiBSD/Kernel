/*	$KAME: in6.h,v 1.83 2001/03/29 02:55:07 jinmei Exp $	*/
/*	$KAME: in6.c,v 1.372 2004/06/14 08:14:21 itojun Exp $	*/
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
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
 * All rights reserved.
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
 *	@(#)in.h	8.3 (Berkeley) 1/3/94
 */

/*
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
 * All rights reserved.
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
//! IPv6 addresses and socket addresses, the `IPV6_*` socket options and the `net.inet6`
//! sysctl numbers: `<netinet6/in6.h>` (`hal::sys::netinet6_in6`, re-exported whole); and
//! `netinet6/in6.c`, the IPv6 addresses of the interfaces, the kernel's since N6.5a
//! (`kernel::netinet6::in6`), with LZ's shapes for `bsd`'s callers.
//!
//! Upstream: sys/netinet6/in6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/in6.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `satosin6`, `satosin6_const`, `sin6tosa` and `sin6tosa_const` are pointer casts
//!   (`docs/C_TO_RUST.md`) for `bsd`'s raw socket addresses; `in6_nam2sin6` checks through
//!   the kernel's and answers the pointer into the caller's mbuf, as LZ's callers use it.
//! - `in6_control` has the `pru_control` signature (`sys/protosw.rs`): the socket's
//!   `SS_PRIV` and the kernel copy of the request; `SIOCGIFINFO_IN6` and `SIOCGNBRINFO_IN6`
//!   are answered here by `nd6_ioctl` (`bsd`'s until N6.5b) over an aligned copy of the
//!   request, the rest by the kernel's `in6_control`. `in6_ioctl` is the raw request of the
//!   kernel's own callers and tests.
//! - `in6_addmulti`, `in6_delmulti` and `in6_purgeaddr` keep LZ's signatures, the net lock's
//!   proof made through the `SaToken` (their callers hold it: ND under the net lock,
//!   `ip6_setmoptions` under the socket lock). `in6_joingroup` and `in6_leavegroup` keep
//!   LZ's `malloc`'d `struct in6_multi_mship` for `ip6_setmoptions`' list until the socket
//!   options move (N6.5c; the kernel's addresses keep theirs in an array).

use core::cell::Cell;
use core::mem::size_of;
use core::ptr::{self, NonNull};

use crate::kern::kern_malloc::{free, malloc};
use crate::net::if_::net_held;
use crate::net::if_var::{Ifaddr, Ifnet};
use crate::netinet6::in6_var::{In6Multi, In6MultiMship, SIOCGIFINFO_IN6, SIOCGNBRINFO_IN6};
use crate::netinet6::nd6::nd6_ioctl;
use crate::sys::errno::Errno;
use crate::sys::ioccom::iocparm_len;
use crate::sys::malloc::{M_IPMADDR, M_NOWAIT};
use crate::sys::mbuf::{Mbuf, mtod};
use crate::sys::queue::ListEntry;
use crate::sys::socket::Sockaddr;
use crate::sys::socketvar::{SS_PRIV, Socket};

pub use hal::sys::netinet6_in6::*;
pub use kernel::netinet6::in6::{
    ifa6_is_deprecated, ifa6_is_invalid, in6_addr2scopeid, in6_hasmulti, in6_ifawithscope,
    in6_lookupmulti, in6_mask2len, in6_prefixlen2mask, in6ifa_ifpforlinklocal, in6ifa_ifpwithaddr,
};
pub use kernel::netinet6::in6_var::ifatoia6;

/// `satosin6(sa)`: a generic `sockaddr` seen as a `sockaddr_in6`.
pub const fn satosin6(sa: *mut Sockaddr) -> *mut SockaddrIn6 {
    sa.cast()
}

/// `satosin6_const(sa)`: a generic `sockaddr` seen as a `sockaddr_in6`, read-only.
pub const fn satosin6_const(sa: *const Sockaddr) -> *const SockaddrIn6 {
    sa.cast()
}

/// `sin6tosa(sin6)`: a `sockaddr_in6` seen as a generic `sockaddr`.
pub const fn sin6tosa(sin6: *mut SockaddrIn6) -> *mut Sockaddr {
    sin6.cast()
}

/// `sin6tosa_const(sin6)`: a `sockaddr_in6` seen as a generic `sockaddr`, read-only.
pub const fn sin6tosa_const(sin6: *const SockaddrIn6) -> *const Sockaddr {
    sin6.cast()
}

/// `in6_nam2sin6(nam, &sin6)`: the `sockaddr_in6` in mbuf `nam`, checked by the kernel's.
pub fn in6_nam2sin6(nam: &Mbuf) -> Result<*mut SockaddrIn6, Errno> {
    kernel::netinet6::in6::in6_nam2sin6(nam)?;
    Ok(satosin6(mtod::<Sockaddr>(nam)))
}

/// `in6_control(so, cmd, data, ifp)`: the `pru_control` of the IPv6 protocols: the address
/// `ioctl`s on interface `ifp` (the kernel's), privileged as the socket is; ND's two
/// requests through `nd6_ioctl` (see the module's deviations).
pub fn in6_control(
    so: &'static Socket,
    cmd: u64,
    data: &mut [u8],
    ifp: Option<&'static Ifnet>,
) -> Result<(), Errno> {
    if !matches!(cmd, SIOCGIFINFO_IN6 | SIOCGNBRINFO_IN6) {
        return kernel::netinet6::in6::in6_control(so.has_state(SS_PRIV), cmd, data, ifp);
    }
    let Some(ifp) = ifp else {
        return Err(Errno::ENXIO);
    };
    let len = iocparm_len(cmd) as usize;
    if data.len() < len {
        return Err(Errno::EINVAL);
    }
    // sys_ioctl's on-stack buffer: at most STK_PARAMS bytes; an aligned copy of the request.
    let mut aligned = [0u64; 16];
    if len > size_of_val(&aligned) {
        return Err(Errno::EINVAL);
    }
    let bytes = aligned.as_mut_ptr().cast::<u8>();
    // SAFETY: both buffers hold `len` bytes and do not overlap.
    unsafe { ptr::copy_nonoverlapping(data.as_ptr(), bytes, len) };
    // SAFETY: an aligned copy of the request, as long as `cmd` encodes, ours for the call.
    let error = unsafe { nd6_ioctl(cmd, bytes, ifp) };
    // SAFETY: as above, back into the caller's buffer.
    unsafe { ptr::copy_nonoverlapping(bytes, data.as_mut_ptr(), len) };
    error
}

/// `in6_ioctl(cmd, data, ifp, privileged)`: the IPv6 address `ioctl`s (the kernel's) over a
/// raw request.
///
/// # Safety
///
/// `data` points at the kernel copy of the request, as long as the structure `cmd` encodes,
/// exclusively the caller's for the call.
pub unsafe fn in6_ioctl(
    cmd: u64,
    data: *mut u8,
    ifp: Option<&'static Ifnet>,
    privileged: bool,
) -> Result<(), Errno> {
    let len = iocparm_len(cmd) as usize;
    // SAFETY: the caller's contract: `len` bytes at `data`, the caller's alone for the call.
    let req = unsafe { core::slice::from_raw_parts_mut(data, len) };
    kernel::netinet6::in6::in6_ioctl(cmd, req, ifp, privileged)
}

/// `in6_purgeaddr(ifa)`: the kernel's, under the net lock the caller holds.
pub fn in6_purgeaddr(ifa: &'static Ifaddr) {
    kernel::netinet6::in6::in6_purgeaddr(ifa, &net_held("in6_purgeaddr"));
}

/// `in6_addmulti(addr, ifp, &error)`: the kernel's, under the net lock the caller holds.
pub fn in6_addmulti(addr: &In6Addr, ifp: &'static Ifnet) -> Result<&'static In6Multi, Errno> {
    kernel::netinet6::in6::in6_addmulti(addr, ifp, &net_held("in6_addmulti"))
}

/// `in6_delmulti(in6m)`: the kernel's, under the net lock the caller holds.
pub fn in6_delmulti(in6m: &'static In6Multi) {
    kernel::netinet6::in6::in6_delmulti(in6m, &net_held("in6_delmulti"));
}

/// `in6_joingroup(ifp, addr, &error)`: joins `addr` on `ifp` and returns the membership entry
/// (`malloc`ed, freed by `in6_leavegroup`); the C's `*errorp` is the `Err`.
pub fn in6_joingroup(ifp: &'static Ifnet, addr: &In6Addr) -> Result<&'static In6MultiMship, Errno> {
    let Some(mem) = malloc(size_of::<In6MultiMship>(), M_IPMADDR, M_NOWAIT) else {
        return Err(Errno::ENOBUFS);
    };
    let maddr = match in6_addmulti(addr, ifp) {
        Ok(m) => m,
        Err(e) => {
            // *errorp is already set
            free(mem, M_IPMADDR, size_of::<In6MultiMship>());
            return Err(e);
        }
    };
    let imm = mem.as_ptr().cast::<In6MultiMship>();
    // SAFETY: a block of `size_of::<In6MultiMship>()` bytes from the allocator, suitably
    // aligned; it is initialized here and lives until `in6_leavegroup` frees it.
    unsafe {
        imm.write(In6MultiMship {
            i6mm_maddr: Cell::new(Some(maddr)),
            i6mm_chain: ListEntry::new(),
        });
        Ok(&*imm)
    }
}

/// `in6_leavegroup(imm)`: leaves the group of membership `imm` and frees it.
pub fn in6_leavegroup(imm: &'static In6MultiMship) {
    if let Some(maddr) = imm.i6mm_maddr.get() {
        in6_delmulti(maddr);
    }
    free(
        NonNull::from(imm).cast(),
        M_IPMADDR,
        size_of::<In6MultiMship>(),
    );
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // The fixtures `bsd`'s IPv6 tests share (the in6.c tests are the kernel's,
    // `kernel::netinet6::in6`).

    use std::sync::MutexGuard;
    use std::vec::Vec;

    use super::*;
    use crate::net::if_::tests::{setup_net, test_ifnet};
    use crate::net::if_::{IFF_MULTICAST, IFF_RUNNING, IFF_UP, if_attach, ifa_add};
    use crate::netinet6::in6_var::{In6Addrlifetime, In6Ifaddr, in6_ifaddr_alloc};
    use crate::netinet6::nd6::ND6_INFINITE_LIFETIME;
    use crate::sys::systm::{net_lock, net_unlock};

    /// `a:b:c:d:e:f:g:h` from its eight 16-bit groups.
    fn addr(g: [u16; 8]) -> In6Addr {
        let mut a = In6Addr::default();
        for (i, w) in g.iter().enumerate() {
            a.s6_addr[2 * i..2 * i + 2].copy_from_slice(&w.to_be_bytes());
        }
        a
    }

    /// The network test setup.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        setup_net()
    }

    /// An attached interface `name`, up and running, with multicast.
    pub(crate) fn test_if(name: &[u8]) -> &'static Ifnet {
        let ifp = test_ifnet(name);
        ifp.if_flags.set(IFF_UP | IFF_RUNNING | IFF_MULTICAST);
        ifp.if_mtu.set(1500);
        if_attach(ifp);
        ifp
    }

    /// An IPv6 address `a`/64 with `flags` on `ifp`, linked into its address list as
    /// `in6_update_ifa` does (the kernel's record), with infinite lifetimes and no route.
    pub(crate) fn test_ia6(ifp: &'static Ifnet, a: In6Addr, flags: i32) -> &'static In6Ifaddr {
        net_lock();
        let ia = in6_ifaddr_alloc(ifp, 0);
        ia.ia_addr.set(SockaddrIn6::with_addr(a));
        ia.ia_prefixmask.set(SockaddrIn6::with_addr(IN6MASK64));
        ia.ia6_flags.set(flags);
        ia.ia6_lifetime.set(In6Addrlifetime {
            ia6t_vltime: ND6_INFINITE_LIFETIME,
            ia6t_pltime: ND6_INFINITE_LIFETIME,
            ..In6Addrlifetime::default()
        });
        ifa_add(ifp, &ia.ia_ifa);
        net_unlock();
        ia
    }

    /// `a:b::c` text to an address (the forms the tests use).
    pub(crate) fn a6(s: &str) -> In6Addr {
        let (head, tail) = s.split_once("::").unwrap_or((s, ""));
        let parse = |p: &str| -> Vec<u16> {
            p.split(':')
                .filter(|g| !g.is_empty())
                .map(|g| u16::from_str_radix(g, 16).expect("hex group"))
                .collect()
        };
        let (h, t) = (parse(head), parse(tail));
        let mut g = [0u16; 8];
        g[..h.len()].copy_from_slice(&h);
        g[8 - t.len()..].copy_from_slice(&t);
        addr(g)
    }

    #[test]
    fn the_fixtures_make_what_they_say() {
        let _g = setup();
        let ifp = test_if(b"tfx0");
        assert_eq!(a6("fd00:77::1").s6_addr[..4], [0xfd, 0, 0, 0x77]);
        assert_eq!(a6("fd00:77::1").s6_addr[15], 1);
        let ia = test_ia6(ifp, a6("fd00:77::1"), 0);
        net_lock();
        let found = in6ifa_ifpwithaddr(ifp, &a6("fd00:77::1"));
        net_unlock();
        assert!(found.is_some_and(|f| core::ptr::eq(f, ia)));
    }
}
/* </TESTS> */
