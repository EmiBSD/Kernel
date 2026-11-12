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
//! numerous additions: `<netinet/in.h>`; and `netinet/in.c`, the Internet addresses of the
//! interfaces (the address `ioctl`s, the interface and prefix routes, multicast records).
//!
//! Upstream: sys/netinet/in.h @ 3ce1f3f79392
//! Upstream: sys/netinet/in.c @ 3ce1f3f79392
//!
//! `in` is a Rust keyword, so the module is `in_` (`docs/C_TO_RUST.md`); `in.c` shares it, as
//! `if.c` shares `if_`.
//!
//! Byte order: as in the C kernel, addresses and ports stay in network order wherever they are
//! stored (`in_addr.s_addr`, `sin_port`). The kernel's `__IPADDR(x)` is `htonl(x)`: "by
//! byte-swapping the constants, we avoid ever having to byte-swap IP addresses inside the
//! kernel", so `INADDR_*`, `IN_CLASS*_NET`/`_HOST` and the `in_class*` tests are network-order
//! values and take network-order arguments. (User-level programs rely on these macros not
//! doing byte-swapping; this is the kernel's half.)
//!
//! Status: `in.h` `wip` (the sysctl name tables), `in.c` `ported` (M7b).
//!
//! Since N6.4a `in.h`'s userland half is `hal::sys::netinet_in`, re-exported here whole.
//!
//! ## Deviations
//! - `CTL_IPPROTO_NAMES` and `IPCTL_NAMES` (`struct ctlname` tables) come with
//!   `<sys/sysctl.h>`.
//! - `ifatoia` is `<netinet/in_var.h>`'s (`netinet/in_var.rs`); `inetctlerrmap` and
//!   `zeroin_addr` come with the `.c` files that define them, as do the prototypes
//!   (`in_cksum` in `netinet/in_cksum.rs`, `ipv4_input` in `netinet/ip_input.rs`, ...).
//! - The `#include <netinet6/in6.h>` at the end of the C header is not mirrored: `in6.h` is a
//!   module of its own when it is ported.
//! - `INADDR_NONE` (userland only), `htons` and friends outside the kernel, and the userland
//!   prototypes (`bindresvport`) are not kernel material; the kernel's byte-order functions
//!   are in `sys/endian.rs`.
//! - `struct in_addr`'s `s_addr` is a `u32`: `in_addr_t` is `crate::sys::types::InAddr`, a
//!   name this struct also needs.
//! - `satosin`, `satosin_const` and `sintosa` are pointer casts (`docs/C_TO_RUST.md`);
//!   `in_hosteq` and `in_nullhost` are `const fn`s.
//! - `in_control` has the `pru_control` signature (`sys/protosw.rs`): the socket, and the
//!   kernel copy of the request as a byte slice, handed to `in_ioctl` (through an aligned
//!   copy if needed) with the socket's `SS_PRIV`. The kernel's own requests (the boot
//!   self-test configures an interface without a socket) call `in_ioctl` as privileged.
//!   `MROUTING` (`mrt_ioctl`) is not configured.
//! - The address `ioctl`s are `unsafe fn`s over `caddr_t data` (`docs/C_TO_RUST.md`, the
//!   `ioctl` row); the `struct sockaddr_in *` the C keeps into the request are copies or raw
//!   pointers into it. `in_ioctl_set_ifaddr` and `in_ioctl_change_ifaddr` share the
//!   allocation of a new address (`in_ifaddr_alloc`); `malloc(M_WAITOK)` that fails panics.
//! - `in_nam2sin` and `in_sa2sin` return the `sockaddr_in` pointer instead of storing it
//!   through an out parameter. Booleans are `bool` (`in_canforward`, `in_broadcast`,
//!   `in_hasmulti`, `in_ifinit`'s `newaddr`, `in_ioctl`'s `privileged`).

use core::mem::{offset_of, size_of};
use core::ptr::{self, NonNull};

use hal::user::bytes_of_mut;

use crate::kern::kern_malloc::{free, malloc};
use crate::kern::subr_prf::panic;
use crate::net::if_::{
    IFF_BROADCAST, IFF_LOOPBACK, IFF_MULTICAST, IFF_POINTOPOINT, IFNETLIST, IFXF_AUTOCONF4, Ifreq,
    if_addrhooks_run, if_get, if_put, ifa_add, ifa_del, ifa_update_broadaddr, ifp_ioctl,
};
use crate::net::if_var::{Ifaddr, Ifnet};
use crate::net::route::{
    RTF_BROADCAST, RTF_CLONING, RTF_CONNECTED, RTF_HOST, RTF_MPATH, ifafree, rt_ifa_add,
    rt_ifa_addlocal, rt_ifa_del, rt_ifa_dellocal, rt_ifa_purge, sa_of,
};
use crate::net::rtable::rtable_l2;
use crate::netinet::igmp::{igmp_joingroup, igmp_leavegroup, igmp_sendpkt};
use crate::netinet::igmp_var::IgmpPktinfo;
use crate::netinet::in_var::{InAliasreq, InIfaddr, InMulti, ifatoia, ifmatoinm};
use crate::sys::endian::{htonl, ntohl};
use crate::sys::errno::Errno;
use crate::sys::ioccom::iocparm_len;
use crate::sys::malloc::{M_IFADDR, M_IPMADDR, M_WAITOK, M_ZERO};
use crate::sys::mbuf::{Mbuf, mtod};
use crate::sys::socket::{AF_INET, Sockaddr};
use crate::sys::socketvar::{SS_PRIV, Socket};
use crate::sys::sockio::{
    SIOCADDMULTI, SIOCAIFADDR, SIOCDELMULTI, SIOCDIFADDR, SIOCGIFADDR, SIOCGIFBRDADDR,
    SIOCGIFDSTADDR, SIOCGIFNETMASK, SIOCSIFADDR, SIOCSIFBRDADDR, SIOCSIFDSTADDR, SIOCSIFNETMASK,
};
use crate::sys::systm::{
    kernel_lock, kernel_unlock, net_assert_locked, net_lock, net_lock_shared, net_unlock,
    net_unlock_shared,
};
use crate::sys::types::SaFamily;

pub use hal::sys::netinet_in::*;

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

/// An interface address with the lifetime its interface's address list gives it.
fn ifa_static(ifa: &Ifaddr) -> &'static Ifaddr {
    // SAFETY: an address on an interface's list lives until `ifa_del` and its last
    // `ifafree`, as the C's pointers do (`docs/C_TO_RUST.md`, reference-counted pool objects).
    unsafe { &*ptr::from_ref(ifa) }
}

/// The Internet address of an `AF_INET` interface address on a list, with that lifetime.
fn ia_static(ifa: &Ifaddr) -> &'static InIfaddr {
    ifatoia(ifa_static(ifa))
}

/// The family of an interface address.
fn ifa_family(ifa: &Ifaddr) -> SaFamily {
    // SAFETY: an interface address's `ifa_addr` is readable (`ifa_add`'s contract).
    unsafe { (*ifa.ifa_addr.get()).sa_family }
}

/// `in_canforward`: whether datagrams to `in_` may be forwarded (it is not in a reserved set
/// of addresses that may not be).
pub fn in_canforward(in_: InAddr) -> bool {
    if in_multicast(in_.s_addr) {
        return false;
    }
    if in_classa(in_.s_addr) {
        let net = in_.s_addr & IN_CLASSA_NET;
        if net == 0 || net == htonl(IN_LOOPBACKNET << IN_CLASSA_NSHIFT) {
            return false;
        }
    }
    true
}

/// `in_socktrim`: trims a mask in a sockaddr: `sin_len` covers the address up to its last
/// non-zero byte.
pub fn in_socktrim(ap: &mut SockaddrIn) {
    let base = offset_of!(SockaddrIn, sin_addr);
    let bytes = ap.sin_addr.s_addr.to_ne_bytes();

    ap.sin_len = 0;
    if let Some(i) = bytes.iter().rposition(|&b| b != 0) {
        ap.sin_len = (base + i + 1) as u8;
    }
}

/// `in_mask2len`: the prefix length of a contiguous mask.
pub fn in_mask2len(mask: &InAddr) -> i32 {
    let p = mask.s_addr.to_ne_bytes();
    let x = p.iter().position(|&b| b != 0xff).unwrap_or(p.len());
    let mut y = 0;
    if x < p.len() {
        while y < 8 {
            if p[x] & (0x80 >> y) == 0 {
                break;
            }
            y += 1;
        }
    }
    (x * 8) as i32 + y
}

/// `in_len2mask`: the mask of prefix length `len`.
pub fn in_len2mask(mask: &mut InAddr, len: i32) {
    let mut p = [0u8; 4];
    let full = (len / 8) as usize;
    for b in p.iter_mut().take(full) {
        *b = 0xff;
    }
    if len % 8 != 0 {
        p[full] = ((0xff00u32 >> (len % 8)) & 0xff) as u8;
    }
    mask.s_addr = u32::from_ne_bytes(p);
}

/// `in_nam2sin`: the `sockaddr_in` in mbuf `nam`, checked.
pub fn in_nam2sin(nam: &Mbuf) -> Result<*mut SockaddrIn, Errno> {
    let sa = mtod::<Sockaddr>(nam);

    if (nam.m_len().get() as usize) < offset_of!(Sockaddr, sa_data) {
        return Err(Errno::EINVAL);
    }
    // SAFETY: the mbuf holds at least the length and family bytes.
    let (family, len) = unsafe { ((*sa).sa_family, (*sa).sa_len) };
    if family != AF_INET {
        return Err(Errno::EAFNOSUPPORT);
    }
    if u32::from(len) != nam.m_len().get() {
        return Err(Errno::EINVAL);
    }
    if usize::from(len) != size_of::<SockaddrIn>() {
        return Err(Errno::EINVAL);
    }
    Ok(satosin(sa))
}

/// `in_sa2sin`: `sa` as a `sockaddr_in`, checked.
///
/// # Safety
///
/// `sa` points at a readable socket address (at least its length and family).
pub unsafe fn in_sa2sin(sa: *mut Sockaddr) -> Result<*mut SockaddrIn, Errno> {
    // SAFETY: the caller's contract.
    let (family, len) = unsafe { ((*sa).sa_family, (*sa).sa_len) };
    if family != AF_INET {
        return Err(Errno::EAFNOSUPPORT);
    }
    if usize::from(len) != size_of::<SockaddrIn>() {
        return Err(Errno::EINVAL);
    }
    Ok(satosin(sa))
}

/// `in_ifp2ia`: the first Internet address of interface `ifp`.
pub fn in_ifp2ia(ifp: &Ifnet) -> Option<&'static InIfaddr> {
    net_assert_locked("in_ifp2ia");

    ifp.if_addrlist
        .iter()
        .find(|ifa| ifa_family(ifa) == AF_INET)
        .map(ia_static)
}

/// `in_control`: the `pru_control` of the Internet protocols: the address `ioctl`s on
/// interface `ifp`. `data` is the kernel copy of the request (`sys_ioctl`), as long as `cmd`
/// encodes; a request that is not aligned for its structure is handled through an aligned
/// copy.
pub fn in_control(
    so: &'static Socket,
    cmd: u64,
    data: &mut [u8],
    ifp: Option<&'static Ifnet>,
) -> Result<(), Errno> {
    in_control_req(cmd, data, ifp, so.has_state(SS_PRIV))
}

/// [`in_control`]'s request, privileged or not as the caller says: the socket's `SS_PRIV`,
/// or the kernel's own request with no socket (the kernel's `ifioctl` through the `Inet`
/// leg `in_ioctl`).
pub fn in_control_req(
    cmd: u64,
    data: &mut [u8],
    ifp: Option<&'static Ifnet>,
    privileged: bool,
) -> Result<(), Errno> {
    // MROUTING: SIOCGETVIFCNT, SIOCGETSGCNT through mrt_ioctl; not configured.
    let len = iocparm_len(cmd) as usize;
    if data.len() < len {
        return Err(Errno::EINVAL);
    }
    if data.as_ptr().align_offset(align_of::<u64>()) == 0 {
        // SAFETY: `data` is the request, as long as `cmd` encodes (checked) and aligned
        // (checked), exclusively ours for the call.
        return unsafe { in_ioctl(cmd, data.as_mut_ptr(), ifp, privileged) };
    }
    // sys_ioctl's on-stack buffer: at most STK_PARAMS bytes.
    let mut aligned = [0u64; 16];
    if len > size_of_val(&aligned) {
        return Err(Errno::EINVAL);
    }
    let bytes = aligned.as_mut_ptr().cast::<u8>();
    // SAFETY: both buffers hold `len` bytes and do not overlap.
    unsafe { ptr::copy_nonoverlapping(data.as_ptr(), bytes, len) };
    // SAFETY: an aligned copy of the request, as long as `cmd` encodes.
    let error = unsafe { in_ioctl(cmd, bytes, ifp, privileged) };
    // SAFETY: as above, back into the caller's buffer.
    unsafe { ptr::copy_nonoverlapping(bytes, data.as_mut_ptr(), len) };
    error
}

/// The `ifreq` of an address `ioctl`.
///
/// # Safety
///
/// `data` points at a `struct ifreq`, exclusively the caller's for the returned lifetime.
unsafe fn data_ifreq<'a>(data: *mut u8) -> &'a mut Ifreq {
    // SAFETY: the caller's contract.
    unsafe { &mut *data.cast::<Ifreq>() }
}

/// `in_ioctl`: the Internet address `ioctl`s.
///
/// # Safety
///
/// `data` points at the kernel copy of the request, aligned for and as long as the structure
/// `cmd` encodes (`IfIoctlFn`'s contract).
pub unsafe fn in_ioctl(
    cmd: u64,
    data: *mut u8,
    ifp: Option<&'static Ifnet>,
    privileged: bool,
) -> Result<(), Errno> {
    let Some(ifp) = ifp else {
        return Err(Errno::ENXIO);
    };

    match cmd {
        SIOCGIFADDR | SIOCGIFNETMASK | SIOCGIFDSTADDR | SIOCGIFBRDADDR => {
            // SAFETY: the caller's contract.
            return unsafe { in_ioctl_get(cmd, data, ifp) };
        }
        SIOCSIFADDR => {
            if !privileged {
                return Err(Errno::EPERM);
            }
            // SAFETY: the caller's contract.
            return unsafe { in_ioctl_set_ifaddr(cmd, data, ifp) };
        }
        SIOCAIFADDR | SIOCDIFADDR => {
            if !privileged {
                return Err(Errno::EPERM);
            }
            // SAFETY: the caller's contract.
            return unsafe { in_ioctl_change_ifaddr(cmd, data, ifp) };
        }
        SIOCSIFNETMASK | SIOCSIFDSTADDR | SIOCSIFBRDADDR => {}
        _ => return Err(Errno::EOPNOTSUPP),
    }

    if !privileged {
        return Err(Errno::EPERM);
    }

    // SAFETY: the caller's contract: these commands take a `struct ifreq`.
    let ifr = unsafe { data_ifreq(data) };
    let mut sin: *mut SockaddrIn = ptr::null_mut();
    if ifr.ifr_addr().sa_family == AF_INET {
        // SAFETY: the request's address.
        sin = unsafe { in_sa2sin(ifr.ifr_addr_mut()) }?;
    }

    net_lock();
    kernel_lock();

    let mut ia: Option<&'static InIfaddr> = None;
    for ifa in ifp.if_addrlist.iter() {
        if ifa_family(ifa) != AF_INET {
            continue;
        }
        // find first address or exact match
        if ia.is_none() {
            ia = Some(ia_static(ifa));
        }
        // SAFETY: `sin` is NULL or the request's `sockaddr_in`.
        if sin.is_null() || unsafe { (*sin).sin_addr.s_addr } == INADDR_ANY {
            break;
        }
        // SAFETY: as above.
        if ia_static(ifa).ia_addr.get().sin_addr.s_addr == unsafe { (*sin).sin_addr.s_addr } {
            ia = Some(ia_static(ifa));
            break;
        }
    }

    let error = 'err: {
        let Some(ia) = ia else {
            break 'err Err(Errno::EADDRNOTAVAIL);
        };

        match cmd {
            SIOCSIFDSTADDR => {
                if ifp.if_flags.get() & IFF_POINTOPOINT == 0 {
                    break 'err Err(Errno::EINVAL);
                }
                // SAFETY: the request's destination address.
                let sin = match unsafe { in_sa2sin(ifr.ifr_dstaddr_mut()) } {
                    Ok(s) => s,
                    Err(e) => break 'err Err(e),
                };
                let oldaddr = ia.ia_dstaddr.get();
                // SAFETY: checked by `in_sa2sin`.
                ia.ia_dstaddr.set(unsafe { *sin });
                // SAFETY: the C hands the driver the `in_ifaddr` for SIOCSIFDSTADDR.
                let error =
                    unsafe { ifp_ioctl(ifp, SIOCSIFDSTADDR, ptr::from_ref(ia).cast_mut().cast()) };
                if error.is_err() {
                    ia.ia_dstaddr.set(oldaddr);
                    break 'err error;
                }
                let _ = in_scrubhost(ia, &oldaddr);
                let _ = in_addhost(ia, &ia.ia_dstaddr.get());
                Ok(())
            }

            SIOCSIFBRDADDR => {
                if ifp.if_flags.get() & IFF_BROADCAST == 0 {
                    break 'err Err(Errno::EINVAL);
                }
                // SAFETY: the request's broadcast address.
                let sin = match unsafe { in_sa2sin(ifr.ifr_broadaddr_mut()) } {
                    Ok(s) => s,
                    Err(e) => break 'err Err(e),
                };
                // SAFETY: a checked `sockaddr_in`; the address is on this interface.
                unsafe { ifa_update_broadaddr(ifp, &ia.ia_ifa, sintosa(sin)) };
                Ok(())
            }

            SIOCSIFNETMASK => {
                if ifr.ifr_addr().sa_len < 8 {
                    break 'err Err(Errno::EINVAL);
                }
                // do not check inet family or strict len
                let sin = satosin(ifr.ifr_addr_mut());
                // SAFETY: the request's address is 16 bytes, a `sockaddr_in`'s size.
                let mask = unsafe { (*sin).sin_addr.s_addr };
                if ntohl(mask) & (!ntohl(mask) >> 1) != 0 {
                    // non-contiguous netmask
                    break 'err Err(Errno::EINVAL);
                }
                ia.ia_netmask.set(mask);
                let mut sm = ia.ia_sockmask.get();
                sm.sin_addr.s_addr = mask;
                ia.ia_sockmask.set(sm);
                Ok(())
            }
            _ => Ok(()),
        }
    };
    // err:
    kernel_unlock();
    net_unlock();
    error
}

/// A new, empty Internet address for `ifp` (the C's `malloc(M_ZERO)` and setup in
/// `in_ioctl_set_ifaddr` and `in_ioctl_change_ifaddr`).
fn in_ifaddr_alloc(ifp: &'static Ifnet) -> &'static InIfaddr {
    let Some(mem) = malloc(size_of::<InIfaddr>(), M_IFADDR, M_WAITOK | M_ZERO) else {
        panic(format_args!("in_ifaddr: no memory"));
    };
    // SAFETY: a zeroed block of `size_of::<InIfaddr>()` bytes; all-zero is a valid `InIfaddr`
    // (cells, links, a reference count). It lives until the last `ifafree`.
    let ia: &'static InIfaddr = unsafe { &*mem.as_ptr().cast::<InIfaddr>() };
    ia.ia_ifa.ifa_refcnt.init();
    let mut addr = ia.ia_addr.get();
    addr.sin_family = AF_INET;
    addr.sin_len = size_of::<SockaddrIn>() as u8;
    ia.ia_addr.set(addr);
    let token = crate::net::if_var::sa_token();
    ia.ia_ifa.ifa_addr.set(token, Some(&ia.ia_addr));
    ia.ia_ifa.ifa_dstaddr.set(token, Some(&ia.ia_dstaddr));
    ia.ia_ifa.ifa_netmask.set(token, Some(&ia.ia_sockmask));
    let mut sm = ia.ia_sockmask.get();
    sm.sin_len = 8;
    ia.ia_sockmask.set(sm);
    if ifp.if_flags.get() & IFF_BROADCAST != 0 {
        let mut b = ia.ia_broadaddr().get();
        b.sin_len = size_of::<SockaddrIn>() as u8;
        b.sin_family = AF_INET;
        ia.ia_broadaddr().set(b);
    }
    ia.ia_ifp().set(Some(ifp));
    ia
}

/// `in_ioctl_set_ifaddr`: `SIOCSIFADDR`, the first Internet address of the interface.
///
/// # Safety
///
/// As for [`in_ioctl`]: `data` is a `struct ifreq`.
unsafe fn in_ioctl_set_ifaddr(cmd: u64, data: *mut u8, ifp: &'static Ifnet) -> Result<(), Errno> {
    if cmd != SIOCSIFADDR {
        panic(format_args!("in_ioctl_set_ifaddr: invalid ioctl {cmd}"));
    }

    // SAFETY: the caller's contract.
    let ifr = unsafe { data_ifreq(data) };
    // SAFETY: the request's address.
    let sin = unsafe { in_sa2sin(ifr.ifr_addr_mut()) }?;
    // SAFETY: checked by `in_sa2sin`.
    let sin = unsafe { *sin };

    net_lock();
    kernel_lock();

    // find first address
    let found = ifp
        .if_addrlist
        .iter()
        .find(|ifa| ifa_family(ifa) == AF_INET)
        .map(ia_static);
    let (ia, newifaddr) = match found {
        Some(ia) => (ia, false),
        None => (in_ifaddr_alloc(ifp), true),
    };

    in_ifscrub(ifp, ia);
    let error = in_ifinit(ifp, ia, &sin, newifaddr);
    if error.is_ok() {
        if_addrhooks_run(ifp);
    }

    kernel_unlock();
    net_unlock();
    error
}

/// `in_ioctl_change_ifaddr`: `SIOCAIFADDR` (add or change an address, its mask, destination
/// and broadcast address) and `SIOCDIFADDR` (delete it).
///
/// # Safety
///
/// As for [`in_ioctl`]: `data` is a `struct in_aliasreq`.
unsafe fn in_ioctl_change_ifaddr(
    cmd: u64,
    data: *mut u8,
    ifp: &'static Ifnet,
) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifra = unsafe { &mut *data.cast::<InAliasreq>() };
    let mut sin: Option<SockaddrIn> = None;
    let mut dstsin: Option<SockaddrIn> = None;
    let mut broadsin: Option<SockaddrIn> = None;
    let mut masksin: Option<SockaddrIn> = None;

    if ifra.ifra_addr().sin_family == AF_INET {
        // SAFETY: the request's address.
        let s = unsafe { in_sa2sin(sintosa(ifra.ifra_addr_mut())) }?;
        // SAFETY: checked by `in_sa2sin`.
        sin = Some(unsafe { *s });
    }

    net_lock();
    kernel_lock();

    let mut ia: Option<&'static InIfaddr> = None;
    for ifa in ifp.if_addrlist.iter() {
        if ifa_family(ifa) != AF_INET {
            continue;
        }
        // find first address, if no exact match wanted
        if sin.is_none_or(|s| s.sin_addr.s_addr == ia_static(ifa).ia_addr.get().sin_addr.s_addr) {
            ia = Some(ia_static(ifa));
            break;
        }
    }

    let error = 'out: {
        match cmd {
            SIOCAIFADDR => {
                let mut needinit = false;

                if ifra.ifra_mask.sin_len != 0 {
                    if ifra.ifra_mask.sin_len < 8 {
                        break 'out Err(Errno::EINVAL);
                    }
                    // do not check inet family or strict len
                    let m = ifra.ifra_mask;
                    if ntohl(m.sin_addr.s_addr) & (!ntohl(m.sin_addr.s_addr) >> 1) != 0 {
                        // non-contiguous netmask
                        break 'out Err(Errno::EINVAL);
                    }
                    masksin = Some(m);
                }
                if ifp.if_flags.get() & IFF_POINTOPOINT != 0
                    && ifra.ifra_dstaddr.sin_family == AF_INET
                {
                    // SAFETY: the request's destination address.
                    match unsafe { in_sa2sin(sintosa(&mut ifra.ifra_dstaddr)) } {
                        // SAFETY: checked by `in_sa2sin`.
                        Ok(s) => dstsin = Some(unsafe { *s }),
                        Err(e) => break 'out Err(e),
                    }
                }
                if ifp.if_flags.get() & IFF_BROADCAST != 0
                    && ifra.ifra_broadaddr().sin_family == AF_INET
                {
                    // SAFETY: the request's broadcast address.
                    match unsafe { in_sa2sin(sintosa(&mut ifra.ifra_dstaddr)) } {
                        // SAFETY: checked by `in_sa2sin`.
                        Ok(s) => broadsin = Some(unsafe { *s }),
                        Err(e) => break 'out Err(e),
                    }
                }

                let (ia, newifaddr) = match ia {
                    Some(ia) => (ia, false),
                    None => (in_ifaddr_alloc(ifp), true),
                };

                let sin = match sin {
                    None => ia.ia_addr.get(),
                    Some(s) => {
                        if newifaddr || s.sin_addr.s_addr != ia.ia_addr.get().sin_addr.s_addr {
                            needinit = true;
                        }
                        s
                    }
                };
                if let Some(m) = masksin {
                    in_ifscrub(ifp, ia);
                    ia.ia_netmask.set(m.sin_addr.s_addr);
                    let mut sm = ia.ia_sockmask.get();
                    sm.sin_addr.s_addr = m.sin_addr.s_addr;
                    ia.ia_sockmask.set(sm);
                    needinit = true;
                }
                if let Some(d) = dstsin {
                    in_ifscrub(ifp, ia);
                    ia.ia_dstaddr.set(d);
                    needinit = true;
                }
                if let Some(b) = broadsin {
                    if newifaddr {
                        ia.ia_broadaddr().set(b);
                    } else {
                        let mut b = b;
                        // SAFETY: a checked `sockaddr_in`; the address is on this interface.
                        unsafe { ifa_update_broadaddr(ifp, &ia.ia_ifa, sintosa(&mut b)) };
                    }
                }
                if needinit && let Err(e) = in_ifinit(ifp, ia, &sin, newifaddr) {
                    break 'out Err(e);
                }
                if_addrhooks_run(ifp);
                Ok(())
            }
            SIOCDIFADDR => {
                let Some(ia) = ia else {
                    break 'out Err(Errno::EADDRNOTAVAIL);
                };
                // Even if the individual steps were safe, shouldn't these kinds of changes
                // happen atomically? What should happen to a packet that was routed after the
                // scrub but before the other steps?
                in_purgeaddr(&ia.ia_ifa);
                if_addrhooks_run(ifp);
                Ok(())
            }

            _ => panic(format_args!("in_ioctl_change_ifaddr: invalid ioctl {cmd}")),
        }
    };

    kernel_unlock();
    net_unlock();
    error
}

/// `in_ioctl_get`: the address `ioctl`s that read: `SIOCGIFADDR`, `SIOCGIFNETMASK`,
/// `SIOCGIFDSTADDR`, `SIOCGIFBRDADDR`.
///
/// # Safety
///
/// As for [`in_ioctl`]: `data` is a `struct ifreq`.
unsafe fn in_ioctl_get(cmd: u64, data: *mut u8, ifp: &'static Ifnet) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifr = unsafe { data_ifreq(data) };
    let mut sin: *mut SockaddrIn = ptr::null_mut();

    let sa = ifr.ifr_addr_mut();
    if sa.sa_family == AF_INET {
        sa.sa_len = size_of::<SockaddrIn>() as u8;
        // SAFETY: the request's address.
        sin = unsafe { in_sa2sin(sa) }?;
    }

    net_lock_shared();

    let mut ia: Option<&'static InIfaddr> = None;
    for ifa in ifp.if_addrlist.iter() {
        if ifa_family(ifa) != AF_INET {
            continue;
        }
        // find first address or exact match
        if ia.is_none() {
            ia = Some(ia_static(ifa));
        }
        // SAFETY: `sin` is NULL or the request's `sockaddr_in`.
        if sin.is_null() || unsafe { (*sin).sin_addr.s_addr } == INADDR_ANY {
            break;
        }
        // SAFETY: as above.
        if ia_static(ifa).ia_addr.get().sin_addr.s_addr == unsafe { (*sin).sin_addr.s_addr } {
            ia = Some(ia_static(ifa));
            break;
        }
    }

    let error = 'err: {
        let Some(ia) = ia else {
            break 'err Err(Errno::EADDRNOTAVAIL);
        };

        // The request's addresses are 16 bytes, a `sockaddr_in`'s size and alignment 1; the
        // writes are unaligned-safe.
        let put = |dst: *mut Sockaddr, v: SockaddrIn| {
            // SAFETY: `dst` is one of the request's 16-byte addresses.
            unsafe { ptr::write_unaligned(satosin(dst), v) };
        };
        match cmd {
            SIOCGIFADDR => {
                put(ifr.ifr_addr_mut(), ia.ia_addr.get());
                Ok(())
            }

            SIOCGIFBRDADDR => {
                if ifp.if_flags.get() & IFF_BROADCAST == 0 {
                    break 'err Err(Errno::EINVAL);
                }
                put(ifr.ifr_dstaddr_mut(), ia.ia_broadaddr().get());
                Ok(())
            }

            SIOCGIFDSTADDR => {
                if ifp.if_flags.get() & IFF_POINTOPOINT == 0 {
                    break 'err Err(Errno::EINVAL);
                }
                put(ifr.ifr_dstaddr_mut(), ia.ia_dstaddr.get());
                Ok(())
            }

            SIOCGIFNETMASK => {
                put(ifr.ifr_addr_mut(), ia.ia_sockmask.get());
                Ok(())
            }

            _ => panic(format_args!("in_ioctl_get: invalid ioctl {cmd}")),
        }
    };

    net_unlock_shared();
    error
}

/// `in_ifscrub`: deletes any existing route for an interface.
pub fn in_ifscrub(ifp: &Ifnet, ia: &'static InIfaddr) {
    if ifp.if_flags.get() & IFF_POINTOPOINT != 0 {
        let _ = in_scrubhost(ia, &ia.ia_dstaddr.get());
    } else if ifp.if_flags.get() & IFF_LOOPBACK == 0 {
        in_remove_prefix(ia);
    }
}

/// `in_ifinit`: initializes an interface's internet address `sin` and routing table entry.
pub fn in_ifinit(
    ifp: &'static Ifnet,
    ia: &'static InIfaddr,
    sin: &SockaddrIn,
    newaddr: bool,
) -> Result<(), Errno> {
    let i = sin.sin_addr.s_addr;
    let mut error: Result<(), Errno> = Ok(());

    net_assert_locked("in_ifinit");

    // Always remove the address from the tree to make sure its position gets updated in case
    // the key changes.
    if !newaddr {
        let _ = rt_ifa_dellocal(&ia.ia_ifa);
        ifa_del(ifp, &ia.ia_ifa);
    }
    let oldaddr = ia.ia_addr.get();
    ia.ia_addr.set(*sin);

    if ia.ia_netmask.get() == 0 {
        let netmask = if in_classa(i) {
            IN_CLASSA_NET
        } else if in_classb(i) {
            IN_CLASSB_NET
        } else {
            IN_CLASSC_NET
        };
        ia.ia_netmask.set(netmask);
        let mut sm = ia.ia_sockmask.get();
        sm.sin_addr.s_addr = netmask;
        ia.ia_sockmask.set(sm);
    }

    // Give the interface a chance to initialize if this is its first address, and to validate
    // the address if necessary.
    // SAFETY: the C hands the driver the `in_ifaddr` for SIOCSIFADDR; drivers read it as the
    // `struct ifaddr` it starts with, if at all.
    if let Err(e) = unsafe { ifp_ioctl(ifp, SIOCSIFADDR, ptr::from_ref(ia).cast_mut().cast()) } {
        ia.ia_addr.set(oldaddr);
        error = Err(e);
    }

    'out: {
        // Add the address to the local list and the global tree. If an error occurred, put
        // back the original address.
        ifa_add(ifp, &ia.ia_ifa);
        let rterror = rt_ifa_addlocal(&ia.ia_ifa);

        if let Err(e) = rterror {
            if !newaddr {
                ifa_del(ifp, &ia.ia_ifa);
            }
            if error.is_ok() {
                error = Err(e);
            }
            break 'out;
        }
        if error.is_err() {
            break 'out;
        }

        ia.ia_net.set(i & ia.ia_netmask.get());
        let mut sm = ia.ia_sockmask.get();
        in_socktrim(&mut sm);
        ia.ia_sockmask.set(sm);
        // Add route for the network.
        ia.ia_ifa.ifa_metric.set(ifp.if_metric.get() as i32);
        if ifp.if_flags.get() & IFF_BROADCAST != 0 {
            let mut b = ia.ia_broadaddr().get();
            if in_rfc3021_subnet(ia.ia_netmask.get()) {
                b.sin_addr.s_addr = 0;
            } else {
                b.sin_addr.s_addr = ia.ia_net.get() | !ia.ia_netmask.get();
            }
            ia.ia_broadaddr().set(b);
        }

        if ifp.if_flags.get() & IFF_POINTOPOINT != 0 {
            // XXX We should not even call in_ifinit() in this case.
            if ia.ia_dstaddr.get().sin_family != AF_INET {
                break 'out;
            }
            error = in_addhost(ia, &ia.ia_dstaddr.get());
        } else if ifp.if_flags.get() & IFF_LOOPBACK == 0 {
            error = in_insert_prefix(ia);
        }

        // If the interface supports multicast, join the "all hosts" multicast group on that
        // interface.
        if ifp.if_flags.get() & IFF_MULTICAST != 0 && ia.ia_allhosts.get().is_none() {
            let addr = InAddr {
                s_addr: INADDR_ALLHOSTS_GROUP,
            };
            ia.ia_allhosts.set(in_addmulti(&addr, ifp));
        }
    }

    // out:
    if error.is_err() && newaddr {
        in_purgeaddr(&ia.ia_ifa);
    }

    error
}

/// `in_purgeaddr`: removes an Internet address from its interface and the routing table.
pub fn in_purgeaddr(ifa: &'static Ifaddr) {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        panic(format_args!("in_purgeaddr: address without interface"));
    };
    let ia = ifatoia(ifa);

    net_assert_locked("in_purgeaddr");

    in_ifscrub(ifp, ia);

    let _ = rt_ifa_dellocal(&ia.ia_ifa);
    rt_ifa_purge(&ia.ia_ifa);
    ifa_del(ifp, &ia.ia_ifa);

    if let Some(inm) = ia.ia_allhosts.get() {
        in_delmulti(inm);
        ia.ia_allhosts.set(None);
    }

    ia.ia_ifp().set(None);
    ifafree(&ia.ia_ifa);
}

/// `in_addhost`: the host route of `dst` through `ia`.
fn in_addhost(ia: &'static InIfaddr, dst: &SockaddrIn) -> Result<(), Errno> {
    let rdomain = ia.ia_ifp().get().map_or(0, |ifp| ifp.if_rdomain.get());
    rt_ifa_add(&ia.ia_ifa, RTF_HOST | RTF_MPATH, sa_of(dst), rdomain)
}

/// `in_scrubhost`: removes the host route of `dst` through `ia`.
fn in_scrubhost(ia: &'static InIfaddr, dst: &SockaddrIn) -> Result<(), Errno> {
    let rdomain = ia.ia_ifp().get().map_or(0, |ifp| ifp.if_rdomain.get());
    rt_ifa_del(&ia.ia_ifa, RTF_HOST, sa_of(dst), rdomain)
}

/// `in_insert_prefix`: inserts the cloning and broadcast routes for this subnet.
fn in_insert_prefix(ia: &'static InIfaddr) -> Result<(), Errno> {
    let ifa = &ia.ia_ifa;
    let rdomain = ifa.ifa_ifp.get().map_or(0, |ifp| ifp.if_rdomain.get());

    // ifa->ifa_addr and ifa->ifa_broadaddr: the address's own storage (ia_addr,
    // ia_broadaddr), copied.
    rt_ifa_add(
        ifa,
        RTF_CLONING | RTF_CONNECTED | RTF_MPATH,
        sa_of(&ia.ia_addr.get()),
        rdomain,
    )?;

    if ia.ia_broadaddr().get().sin_addr.s_addr != 0 {
        return rt_ifa_add(
            ifa,
            RTF_HOST | RTF_BROADCAST | RTF_MPATH,
            sa_of(&ia.ia_broadaddr().get()),
            rdomain,
        );
    }

    Ok(())
}

/// `in_remove_prefix`: removes the cloning and broadcast routes for this subnet.
fn in_remove_prefix(ia: &'static InIfaddr) {
    let ifa = &ia.ia_ifa;
    let rdomain = ifa.ifa_ifp.get().map_or(0, |ifp| ifp.if_rdomain.get());

    let _ = rt_ifa_del(
        ifa,
        RTF_CLONING | RTF_CONNECTED,
        sa_of(&ia.ia_addr.get()),
        rdomain,
    );

    if ia.ia_broadaddr().get().sin_addr.s_addr != 0 {
        let _ = rt_ifa_del(
            ifa,
            RTF_HOST | RTF_BROADCAST,
            sa_of(&ia.ia_broadaddr().get()),
            rdomain,
        );
    }
}

/// `in_broadcast`: whether `in_` is a local broadcast address in the routing domain of table
/// `rtableid`.
pub fn in_broadcast(in_: InAddr, rtableid: u32) -> bool {
    net_assert_locked("in_broadcast");

    let rdomain = rtable_l2(rtableid);

    for ifn in IFNETLIST.0.iter() {
        if ifn.if_rdomain.get() != rdomain {
            continue;
        }
        if ifn.if_flags.get() & IFF_BROADCAST == 0 {
            continue;
        }
        for ifa in ifn.if_addrlist.iter() {
            if ifa_family(ifa) == AF_INET {
                let ia = ifatoia(ifa);
                if in_.s_addr != ia.ia_addr.get().sin_addr.s_addr
                    && in_.s_addr == ia.ia_broadaddr().get().sin_addr.s_addr
                {
                    return true;
                }
            }
        }
    }
    false
}

/// `in_lookupmulti`: the multicast record of `addr` on `ifp`, if joined.
pub fn in_lookupmulti(addr: &InAddr, ifp: &Ifnet) -> Option<&'static InMulti> {
    crate::net::if_var::if_maddrlock_assert_anylock(ifp);

    for ifma in ifp.if_maddrlist.iter() {
        // SAFETY: a record's address is readable (its protocol set it).
        if unsafe { (*ifma.ifma_addr.get()).sa_family } == AF_INET
            && ifmatoinm(ifma).inm_addr().s_addr == addr.s_addr
        {
            // SAFETY: a record on the list lives until `in_delmulti` frees it.
            return Some(unsafe { &*ptr::from_ref(ifmatoinm(ifma)) });
        }
    }
    None
}

/// `in_addmulti`: adds `addr` to the IP multicast addresses of `ifp` (or takes a reference
/// on the record if it is there); `None` when the driver refuses.
pub fn in_addmulti(addr: &InAddr, ifp: &'static Ifnet) -> Option<&'static InMulti> {
    // See if address already in list.
    crate::net::if_var::if_maddrlock_enter_write(ifp);
    if let Some(inm) = in_lookupmulti(addr, ifp) {
        inm.inm_refcnt().take();
        crate::net::if_var::if_maddrlock_exit_write(ifp);
        return Some(inm);
    }
    crate::net::if_var::if_maddrlock_exit_write(ifp);

    // New address; allocate a new multicast record and link it into the interface's multicast
    // list.
    let Some(mem) = malloc(size_of::<InMulti>(), M_IPMADDR, M_WAITOK | M_ZERO) else {
        panic(format_args!("in_addmulti: no memory"));
    };
    // SAFETY: a zeroed block of `size_of::<InMulti>()` bytes; all-zero is a valid `InMulti`.
    let new_inm: &'static InMulti = unsafe { &*mem.as_ptr().cast::<InMulti>() };

    // Ask the network driver to update its multicast reception filter appropriately for the
    // new address.
    let mut ifr = Ifreq::zeroed();
    let sin = SockaddrIn {
        sin_len: size_of::<SockaddrIn>() as u8,
        sin_family: AF_INET,
        sin_addr: *addr,
        ..SockaddrIn::default()
    };
    // `ifr_addr` is 16 bytes, a `sockaddr_in`'s size.
    if !hal::pod::put(
        bytes_of_mut(core::slice::from_mut(ifr.ifr_addr_mut())),
        0,
        &sin,
    ) {
        panic(format_args!(
            "in_addmulti: a sockaddr_in past an ifreq's address"
        ));
    }
    kernel_lock();
    if kernel::net::if_::if_ioctl(
        ifp,
        SIOCADDMULTI,
        bytes_of_mut(core::slice::from_mut(&mut ifr)),
    )
    .is_err()
    {
        kernel_unlock();
        free(mem, M_IPMADDR, size_of::<InMulti>());
        return None;
    }
    kernel_unlock();

    crate::net::if_var::if_maddrlock_enter_write(ifp);
    // check again after unlock and lock
    if let Some(inm) = in_lookupmulti(addr, ifp) {
        inm.inm_refcnt().take();
        crate::net::if_var::if_maddrlock_exit_write(ifp);
        free(mem, M_IPMADDR, size_of::<InMulti>());
        return Some(inm);
    }
    let inm = new_inm;
    inm.inm_sin.set(sin);
    inm.inm_refcnt().init();
    inm.inm_ifidx().set(ifp.if_index.get());
    inm.inm_ifma
        .ifma_addr
        .set(crate::net::if_var::sa_token(), Some(&inm.inm_sin));

    // Let IGMP know that we have joined a new IP multicast group.
    ifp.if_maddrlist.insert_head(&inm.inm_ifma);
    let mut pkt = IgmpPktinfo::default(); // pkt.ipi_ifidx = 0
    igmp_joingroup(inm, ifp, &mut pkt);
    crate::net::if_var::if_maddrlock_exit_write(ifp);

    if pkt.ipi_ifidx != 0 {
        igmp_sendpkt(&pkt);
    }

    Some(inm)
}

/// `in_delmulti`: drops a reference on a multicast record, deleting it with the last one.
pub fn in_delmulti(inm: &'static InMulti) {
    if !inm.inm_refcnt().rele() {
        return;
    }

    let ifp = if_get(inm.inm_ifidx().get());
    if let Some(ifp) = ifp {
        crate::net::if_var::if_maddrlock_enter_write(ifp);
        // No remaining claims to this record; let IGMP know that we are leaving the multicast
        // group.
        let mut pkt = IgmpPktinfo::default(); // pkt.ipi_ifidx = 0
        igmp_leavegroup(inm, ifp, &mut pkt);
        ifp.if_maddrlist.remove(&inm.inm_ifma);
        crate::net::if_var::if_maddrlock_exit_write(ifp);

        if pkt.ipi_ifidx != 0 {
            igmp_sendpkt(&pkt);
        }

        // Notify the network driver to update its multicast reception filter.
        let mut ifr = Ifreq::zeroed();
        let sin = SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_addr: inm.inm_addr(),
            ..SockaddrIn::default()
        };
        // `ifr_addr` is 16 bytes, a `sockaddr_in`'s size.
        if !hal::pod::put(
            bytes_of_mut(core::slice::from_mut(ifr.ifr_addr_mut())),
            0,
            &sin,
        ) {
            panic(format_args!(
                "in_delmulti: a sockaddr_in past an ifreq's address"
            ));
        }
        kernel_lock();
        let _ = kernel::net::if_::if_ioctl(
            ifp,
            SIOCDELMULTI,
            bytes_of_mut(core::slice::from_mut(&mut ifr)),
        );
        kernel_unlock();

        if_put(ifp);
    }

    free(NonNull::from(inm).cast(), M_IPMADDR, size_of::<InMulti>());
}

/// `in_hasmulti`: whether `ifp` joined the multicast group `addr`.
pub fn in_hasmulti(addr: &InAddr, ifp: &Ifnet) -> bool {
    crate::net::if_var::if_maddrlock_enter_read(ifp);
    let joined = in_lookupmulti(addr, ifp).is_some();
    crate::net::if_var::if_maddrlock_exit_read(ifp);

    joined
}

/// `in_ifdetach`: removes every IPv4 address of `ifp`.
pub fn in_ifdetach(ifp: &'static Ifnet) {
    // nuke any of IPv4 addresses we have
    for ifa in ifp.if_addrlist.iter() {
        if ifa_family(ifa) != AF_INET {
            continue;
        }
        in_purgeaddr(ifa_static(ifa));
        if_addrhooks_run(ifp);
    }

    if ifp.if_xflags.get() & IFXF_AUTOCONF4 != 0 {
        ifp.if_xflags.set(ifp.if_xflags.get() & !IFXF_AUTOCONF4);
    }
}

/// `in_prefixlen2mask`: the network-order mask of prefix length `plen`.
pub fn in_prefixlen2mask(maskp: &mut InAddr, plen: i32) {
    if plen == 0 {
        maskp.s_addr = 0;
    } else {
        maskp.s_addr = htonl(0xffff_ffffu32 << (32 - plen));
    }
}

/* </CODE> */
