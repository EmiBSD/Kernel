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
//! sysctl numbers: `<netinet6/in6.h>` (`hal::sys::netinet6_in6`, re-exported whole since
//! N6.5a); and `netinet6/in6.c`, the IPv6 addresses of the interfaces (the address `ioctl`s,
//! prefix routes, multicast records).
//!
//! Upstream: sys/netinet6/in6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/in6.c @ 3ce1f3f79392
//!
//! `in6.c` shares the module with the header, as `in.c` shares `netinet/in_.rs` with
//! `in.h`. The C header is included by `<netinet/in.h>` unconditionally, so this module is
//! compiled whether or not the `inet6` feature (OpenBSD's `option INET6`) is on.
//!
//! Byte order: as in the C kernel, an [`In6Addr`] holds the address bytes as they are on the
//! wire, and the 16- and 32-bit views (`s6_addr16`, `s6_addr32`) are the native-endian
//! readings of those bytes, so `__IPV6_ADDR_INT32_ONE` is `htonl(1)` and a comparison of
//! words is a comparison of the network-order words, as in C.
//!
//! ## Deviations
//! - `struct in6_addr`'s `__u6_addr` union is the byte array `s6_addr`; `s6_addr8`,
//!   `s6_addr16` and `s6_addr32` are `const fn` accessors (and setters) reading the bytes in
//!   native order. The structure has alignment 1 (4 in C) so that it can sit in the
//!   `__packed` protocol headers; every structure holding one keeps the C size and offsets
//!   (asserted below).
//! - The address macros (`IN6_IS_ADDR_*`, `IN6_ARE_ADDR_EQUAL`, `IN6_IS_SCOPE_*`,
//!   `__IPV6_ADDR_MC_SCOPE`, `IFA6_IS_DEPRECATED`, `IFA6_IS_INVALID`) are functions of the
//!   same name in lower case, `const fn` where they read only the address.
//! - The `*_INIT` initializers and the `in6addr_*`/`in6mask*` objects (defined in `in6.c`)
//!   are `pub const` values: `IN6ADDR_ANY_INIT` and `IN6ADDR_ANY` are the same address. The
//!   kernel masks `IN6MASK0`..`IN6MASK128` stand for both the C macros and the `in6mask*`
//!   objects, and `sa6_any` is [`SA6_ANY`].
//! - `satosin6`, `satosin6_const`, `sin6tosa` and `sin6tosa_const` are pointer casts
//!   (`docs/C_TO_RUST.md`); `ifatoia6` is checked, as `ifatoia` is: it takes the address
//!   family as the type tag and panics on another.
//! - `CTL_IPV6PROTO_NAMES` and `IPV6CTL_NAMES` are `Ctlname` tables.
//! - The userland part (`SIN6_LEN`, `socklen_t`, the `inet6_opt_*` and `inet6_rth_*`
//!   prototypes of libc) is not kernel material. `__KAME__` is not mirrored.
//! - The kernel prototypes are defined by the `.c` files: `ipv6_input`, `ipv6_check`,
//!   `inet6ctlerrmap` (`netinet6/ip6_input.rs`), `in6_cksum` (`netinet6/in6_cksum.rs`),
//!   `in6_proto_cksum_out` (`netinet6/ip6_output.rs`), `in6_embedscope`,
//!   `in6_recoverscope`, `in6_clearscope` (`netinet6/in6_src.rs`), `zeroin6_addr`
//!   (`netinet6/in6_pcb.rs`); `in6_addrscope`, `in6_ifawithscope`, `in6_mask2len`,
//!   `in6_nam2sin6` and `in6_sa2sin6` are this module's (`in6.c`).
//!
//! ### `in6.c`
//! - `in6_mask2len`'s limit pointer is `Option<usize>`, the number of valid mask bytes (the
//!   C's `lim0 - mask`); `None` or more than 16 is the whole address. `in6_update_ifa` passes
//!   `sin6_len - 8` as the C's pointer arithmetic does (a shorter length makes the unsigned
//!   distance huge: the whole mask).
//! - `in6_control` hands the request to `in6_ioctl` through an aligned copy when `sys_ioctl`'s
//!   buffer is not aligned for the structure (as `in_control`); the address inside a request
//!   is read and written unaligned.
//! - `in6_update_ifa` takes `&In6Aliasreq`: the C's changes of `ifra->ifra_flags` (re-run DAD)
//!   stay in a local of the function; `in6_ioctl_change_ifaddr`, which owns the request, sets
//!   `IN6_IFF_TENTATIVE` in it before the call as the C does.
//! - The `*errorp` out-parameters of `in6_addmulti` and `in6_joingroup` are the `Err`; the
//!   `malloc(M_NOWAIT)` failures are `ENOBUFS`. The membership list entries are `malloc`ed
//!   and freed by `in6_joingroup` and `in6_leavegroup`, as in the C.
//! - The two identical blocks of `in6_update_ifa` that add the route of an all-nodes group
//!   (`rtalloc`, the 32-bit key check, `rtrequest(RTM_ADD)`) are one private function,
//!   `in6_add_mcast_route`; the `goto`s of `in6_ifawithscope`'s rules are a labeled block
//!   returning a verdict (skip or replace).
//! - `in6_check_embed_scope`, `in6_clear_scope_id`, `in6_ioctl_get`,
//!   `in6_ioctl_change_ifaddr`, `in6_ifinit` and `in6_unlink_ifa` are private (file-local
//!   prototypes in C); the first two work on a copy of the request's address that is
//!   written back.
//! - `NCARP` (`carp_iamatch`, `IFT_CARP` rules in `in6_ifawithscope` and `in6if_do_dad`) and
//!   `MROUTING` (`mrt6_ioctl`) are not configured: comments at the sites.
//! - The routing code called for `AF_INET6` (`rt_ifa_add`, `rt_ifa_addlocal`, `rtalloc`,
//!   `rtrequest` of the multicast routes) is the generic code of `net/route.rs`.

use core::cell::Cell;
use core::mem::{offset_of, size_of};
use core::ptr::{self, NonNull};

use crate::kern::kern_malloc::{free, malloc};
use crate::kern::kern_tc::{gettime, getuptime};
use crate::kern::subr_prf::{log, panic};
use crate::net::if_::{
    IFF_LOOPBACK, IFF_MULTICAST, IFF_POINTOPOINT, IFF_RUNNING, IFF_UP, IFNETLIST, if_addrhooks_run,
    if_get, if_put, ifa_add, ifa_del, ifp_ioctl,
};
use crate::net::if_types::IFT_CARP;
use crate::net::if_var::{IFA_ROUTE, Ifaddr, Ifnet};
use crate::net::route::{
    RTAX_DST, RTAX_GATEWAY, RTAX_IFA, RTAX_NETMASK, RTF_CLONING, RTF_CONNECTED, RTF_HOST,
    RTF_MPATH, RTF_MULTICAST, RTM_ADD, RTM_CHGADDRATTR, RTP_CONNECTED, RtAddrinfo, Rtentry,
    ifafree, rt_ifa_add, rt_ifa_addlocal, rt_ifa_del, rt_ifa_dellocal, rt_ifa_purge, rtalloc,
    rtfree, rtrequest, sa_of, sa_view,
};
use crate::net::rtable::rt_key;
use crate::net::rtsock::rtm_addr;
use crate::netinet6::in6_ifattach::in6_ifattach;
use crate::netinet6::in6_var::{
    IN6_IFF_ANYCAST, IN6_IFF_AUTOCONF, IN6_IFF_DEPRECATED, IN6_IFF_DETACHED, IN6_IFF_DUPLICATED,
    IN6_IFF_TEMPORARY, IN6_IFF_TENTATIVE, In6Aliasreq, In6Ifaddr, In6Ifreq, In6Multi,
    In6MultiMship, In6MultiMshipList, SIOCAIFADDR_IN6, SIOCDIFADDR_IN6, SIOCGIFAFLAG_IN6,
    SIOCGIFALIFETIME_IN6, SIOCGIFDSTADDR_IN6, SIOCGIFINFO_IN6, SIOCGIFNETMASK_IN6,
    SIOCGNBRINFO_IN6, ia6_dstin6, ia6_maskin6, ia6_sin6, ifa_in6, ifmatoin6m,
};
use crate::netinet6::ip6_var::{Ip6statCounters, ip6stat_inc, ip6stat_inc_idx};
use crate::netinet6::mld6::{mld6_sendpkt, mld6_start_listening, mld6_stop_listening};
use crate::netinet6::mld6_var::Mld6Pktinfo;
use crate::netinet6::nd6::{ND6_INFINITE_LIFETIME, nd6_expire_timer_update, nd6_ioctl};
use crate::netinet6::nd6_nbr::{nd6_dad_start, nd6_dad_stop};
use crate::sys::endian::{htonl, htons};
use crate::sys::errno::Errno;
use crate::sys::ioccom::iocparm_len;
use crate::sys::malloc::{M_IFADDR, M_IPMADDR, M_NOWAIT, M_WAITOK, M_ZERO};
use crate::sys::mbuf::{Mbuf, mtod};
use crate::sys::queue::{ListEntry, ListHead};
use crate::sys::socket::{AF_INET6, AF_UNSPEC, Sockaddr};
use crate::sys::socketvar::{SS_PRIV, Socket};
use crate::sys::sockio::{
    SIOCADDMULTI, SIOCDELMULTI, SIOCSIFADDR, SIOCSIFBRDADDR, SIOCSIFDSTADDR, SIOCSIFNETMASK,
};
use crate::sys::syslog::LOG_ERR;
use crate::sys::systm::{
    kernel_lock, kernel_unlock, net_assert_locked, net_lock, net_lock_shared, net_unlock,
    net_unlock_shared,
};
use crate::sys::types::{SaFamily, Time};

pub use hal::sys::netinet6_in6::*;

/// `IFA6_IS_DEPRECATED(a)`: the preferred lifetime of the address has run out.
pub fn ifa6_is_deprecated(a: &In6Ifaddr) -> bool {
    let lt = a.ia6_lifetime.get();
    lt.ia6t_pltime != ND6_INFINITE_LIFETIME
        && (getuptime() - a.ia6_updatetime.get()) as u32 > lt.ia6t_pltime
}

/// `IFA6_IS_INVALID(a)`: the valid lifetime of the address has run out.
pub fn ifa6_is_invalid(a: &In6Ifaddr) -> bool {
    let lt = a.ia6_lifetime.get();
    lt.ia6t_vltime != ND6_INFINITE_LIFETIME
        && (getuptime() - a.ia6_updatetime.get()) as u32 > lt.ia6t_vltime
}

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

/// `ifatoia6(ifa)`: the IPv6 address an `AF_INET6` interface address is the first member
/// of.
pub fn ifatoia6(ifa: &Ifaddr) -> &In6Ifaddr {
    let addr = ifa.ifa_addr.get();
    // SAFETY: an interface address's `ifa_addr` is a readable socket address (`ifa_add`'s
    // contract).
    if addr.is_null() || unsafe { (*addr).sa_family } != AF_INET6 {
        panic(format_args!("ifatoia6: not an inet6 address"));
    }
    // SAFETY: every `AF_INET6` interface address is the `ia_ifa` member, at offset 0 of the
    // `#[repr(C)]` `In6Ifaddr` that `in6_update_ifa` allocated.
    unsafe { &*core::ptr::from_ref(ifa).cast::<In6Ifaddr>() }
}

/// An interface address with the lifetime its interface's address list gives it.
fn ifa_static(ifa: &Ifaddr) -> &'static Ifaddr {
    // SAFETY: an address on an interface's list lives until `ifa_del` and its last
    // `ifafree`, as the C's pointers do (`docs/C_TO_RUST.md`, reference-counted pool objects).
    unsafe { &*ptr::from_ref(ifa) }
}

/// The IPv6 address of an `AF_INET6` interface address on a list, with that lifetime.
fn ia6_static(ifa: &Ifaddr) -> &'static In6Ifaddr {
    ifatoia6(ifa_static(ifa))
}

/// The family of an interface address.
fn ifa_family(ifa: &Ifaddr) -> SaFamily {
    // SAFETY: an interface address's `ifa_addr` is readable (`ifa_add`'s contract).
    unsafe { (*ifa.ifa_addr.get()).sa_family }
}

/// `in6_mask2len`: the prefix length of mask `mask`, -1 if it is not contiguous. `lim` is
/// the number of valid bytes of the mask (the C's `lim0 - mask`); `None` (`NULL`) or more
/// than 16 means the whole address, and a given limit makes the check of the remaining
/// bits stricter.
pub fn in6_mask2len(mask: &In6Addr, lim: Option<usize>) -> i32 {
    // ignore the scope_id part
    let lim = match lim {
        Some(l) if l <= size_of::<In6Addr>() => l,
        _ => size_of::<In6Addr>(),
    };
    let m = &mask.s6_addr;

    let mut x = 0;
    let mut p = 0;
    while p < lim {
        if m[p] != 0xff {
            break;
        }
        x += 1;
        p += 1;
    }
    let mut y = 0;
    if p < lim {
        while y < 8 {
            if m[p] & (0x80 >> y) == 0 {
                break;
            }
            y += 1;
        }
    }

    // when the limit pointer is given, do a stricter check on the remaining bits.
    if p < lim {
        if y != 0 && m[p] & (0x00ff >> y) != 0 {
            return -1;
        }
        if m[p + 1..lim].iter().any(|&b| b != 0) {
            return -1;
        }
    }

    x * 8 + y
}

/// `in6_nam2sin6`: the `sockaddr_in6` in mbuf `nam`, checked (family, length).
pub fn in6_nam2sin6(nam: &Mbuf) -> Result<*mut SockaddrIn6, Errno> {
    let sa = mtod::<Sockaddr>(nam);

    if (nam.m_len().get() as usize) < offset_of!(Sockaddr, sa_data) {
        return Err(Errno::EINVAL);
    }
    // SAFETY: the mbuf holds at least the length and family bytes.
    let (family, len) = unsafe { ((*sa).sa_family, (*sa).sa_len) };
    if family != AF_INET6 {
        return Err(Errno::EAFNOSUPPORT);
    }
    if u32::from(len) != nam.m_len().get() {
        return Err(Errno::EINVAL);
    }
    if usize::from(len) != size_of::<SockaddrIn6>() {
        return Err(Errno::EINVAL);
    }
    Ok(satosin6(sa))
}

/// `in6_sa2sin6`: `sa` as a `sockaddr_in6`, checked.
///
/// # Safety
///
/// `sa` points at a readable socket address (at least its length and family).
pub unsafe fn in6_sa2sin6(sa: *mut Sockaddr) -> Result<*mut SockaddrIn6, Errno> {
    // SAFETY: the caller's contract.
    let (family, len) = unsafe { ((*sa).sa_family, (*sa).sa_len) };
    if family != AF_INET6 {
        return Err(Errno::EAFNOSUPPORT);
    }
    if usize::from(len) != size_of::<SockaddrIn6>() {
        return Err(Errno::EINVAL);
    }
    Ok(satosin6(sa))
}

/// `in6_control`: the `pru_control` of the IPv6 protocols: the address `ioctl`s, handed
/// to `in6_ioctl` with the socket's `SS_PRIV` (`MROUTING`'s `mrt6_ioctl` is not
/// configured). `data` is the kernel copy of the request (`sys_ioctl`), as long as `cmd`
/// encodes; a request that is not aligned for its structure is handled through an aligned
/// copy.
pub fn in6_control(
    so: &'static Socket,
    cmd: u64,
    data: &mut [u8],
    ifp: Option<&'static Ifnet>,
) -> Result<(), Errno> {
    let privileged = so.has_state(SS_PRIV);

    // MROUTING: SIOCGETSGCNT_IN6, SIOCGETMIFCNT_IN6 through mrt6_ioctl; not configured.
    let len = iocparm_len(cmd) as usize;
    if data.len() < len {
        return Err(Errno::EINVAL);
    }
    if data.as_ptr().align_offset(align_of::<u64>()) == 0 {
        // SAFETY: `data` is the request, as long as `cmd` encodes (checked) and aligned
        // (checked), exclusively ours for the call.
        return unsafe { in6_ioctl(cmd, data.as_mut_ptr(), ifp, privileged) };
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
    let error = unsafe { in6_ioctl(cmd, bytes, ifp, privileged) };
    // SAFETY: as above, back into the caller's buffer.
    unsafe { ptr::copy_nonoverlapping(bytes, data.as_mut_ptr(), len) };
    error
}

/// `in6_ioctl`: the IPv6 address `ioctl`s (`SIOCAIFADDR_IN6`, `SIOCDIFADDR_IN6`, the
/// `SIOCG*_IN6` requests, `SIOCGIFINFO_IN6`/`SIOCGNBRINFO_IN6` through `nd6_ioctl`).
///
/// # Safety
///
/// `data` points at the kernel copy of the request, readable and writable for the
/// size `cmd` encodes and aligned for the request's type.
pub unsafe fn in6_ioctl(
    cmd: u64,
    data: *mut u8,
    ifp: Option<&'static Ifnet>,
    privileged: bool,
) -> Result<(), Errno> {
    let Some(ifp) = ifp else {
        return Err(Errno::ENXIO);
    };

    match cmd {
        // SAFETY: the caller's contract.
        SIOCGIFINFO_IN6 | SIOCGNBRINFO_IN6 => unsafe { nd6_ioctl(cmd, data, ifp) },
        SIOCGIFDSTADDR_IN6 | SIOCGIFNETMASK_IN6 | SIOCGIFAFLAG_IN6 | SIOCGIFALIFETIME_IN6 => {
            // SAFETY: the caller's contract.
            unsafe { in6_ioctl_get(cmd, data, ifp) }
        }
        SIOCAIFADDR_IN6 | SIOCDIFADDR_IN6 => {
            if !privileged {
                return Err(Errno::EPERM);
            }
            // SAFETY: the caller's contract.
            unsafe { in6_ioctl_change_ifaddr(cmd, data, ifp) }
        }
        // Do not pass those ioctl to driver handler since they are not properly set up.
        // Instead just error out.
        SIOCSIFADDR | SIOCSIFDSTADDR | SIOCSIFBRDADDR | SIOCSIFNETMASK => Err(Errno::EINVAL),
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// `in6_ioctl_change_ifaddr`: `SIOCAIFADDR_IN6` (add or change an address) and
/// `SIOCDIFADDR_IN6` (delete it).
///
/// # Safety
///
/// As for [`in6_ioctl`]: `data` is a `struct in6_aliasreq` (`SIOCAIFADDR_IN6`) or a
/// `struct in6_ifreq` (`SIOCDIFADDR_IN6`).
unsafe fn in6_ioctl_change_ifaddr(
    cmd: u64,
    data: *mut u8,
    ifp: &'static Ifnet,
) -> Result<(), Errno> {
    let ifra = data.cast::<In6Aliasreq>();
    let mut newifaddr = false;

    // Find address for this interface, if it exists.
    //
    // In netinet code, we have checked ifra_addr in SIOCSIF*ADDR operation only, and used the
    // first interface address as the target of other operations (without checking
    // ifra_addr). This was because netinet code/API assumed at most 1 interface address per
    // interface. Since IPv6 allows a node to assign multiple addresses on a single interface,
    // we almost always look and check the presence of ifra_addr, and reject invalid ones
    // here. It also decreases duplicated code among SIOC*_IN6 operations.
    //
    // We always require users to specify a valid IPv6 address for the corresponding
    // operation.
    let sa: *mut Sockaddr = match cmd {
        SIOCAIFADDR_IN6 => {
            // SAFETY: the caller's contract: `data` is a `struct in6_aliasreq`.
            let a = unsafe { ptr::addr_of_mut!((*ifra).ifra_ifrau.ifrau_addr) };
            sin6tosa(a)
        }
        SIOCDIFADDR_IN6 => {
            // SAFETY: the caller's contract: `data` is a `struct in6_ifreq`.
            let a = unsafe { ptr::addr_of_mut!((*data.cast::<In6Ifreq>()).ifr_ifru.ifru_addr) };
            sin6tosa(a)
        }
        _ => panic(format_args!("in6_ioctl_change_ifaddr: invalid ioctl {cmd}")),
    };
    let mut sa6: Option<*mut SockaddrIn6> = None;
    // SAFETY: the request's address, readable for its length and family bytes.
    if unsafe { (*sa).sa_family } == AF_INET6 {
        // SAFETY: as above.
        sa6 = Some(unsafe { in6_sa2sin6(sa) }?);
    }

    kernel_lock();
    net_lock();

    let mut ia6: Option<&'static In6Ifaddr> = None;
    let error = 'err: {
        if let Some(p) = sa6 {
            // SAFETY: the request's `sockaddr_in6`, checked by `in6_sa2sin6`; it is copied
            // out and back (the request is aligned for its structure, the address inside
            // it only for the address).
            let mut s = unsafe { ptr::read_unaligned(p) };
            let mut r = in6_check_embed_scope(&mut s, ifp.if_index.get());
            if r.is_ok() {
                r = in6_clear_scope_id(&mut s, ifp.if_index.get());
            }
            // SAFETY: as above.
            unsafe { ptr::write_unaligned(p, s) };
            if let Err(e) = r {
                break 'err Err(e);
            }
            ia6 = in6ifa_ifpwithaddr(ifp, &s.sin6_addr);
        }

        match cmd {
            SIOCDIFADDR_IN6 => {
                // for IPv4, we look for existing in_ifaddr here to allow "ifconfig if0
                // delete" to remove the first IPv4 address on the interface. For IPv6, as
                // the spec allows multiple interface address from the day one, we consider
                // "remove the first one" semantics to be not preferable.
                let Some(ia6) = ia6 else {
                    break 'err Err(Errno::EADDRNOTAVAIL);
                };
                in6_purgeaddr(&ia6.ia_ifa);
                if_addrhooks_run(ifp);
                Ok(())
            }

            SIOCAIFADDR_IN6 => {
                // SAFETY: the caller's contract; `data` is not otherwise referenced.
                let ifra = unsafe { &mut *ifra };
                if ifra.ifra_addr().sin6_family != AF_INET6
                    || usize::from(ifra.ifra_addr().sin6_len) != size_of::<SockaddrIn6>()
                {
                    break 'err Err(Errno::EAFNOSUPPORT);
                }

                // reject read-only flags
                if ifra.ifra_flags & IN6_IFF_DUPLICATED != 0
                    || ifra.ifra_flags & IN6_IFF_DETACHED != 0
                    || ifra.ifra_flags & IN6_IFF_DEPRECATED != 0
                {
                    break 'err Err(Errno::EINVAL);
                }

                if ia6.is_none() {
                    newifaddr = true;
                }

                // Make the address tentative before joining multicast addresses, so that
                // corresponding MLD responses would not have a tentative source address.
                if newifaddr && in6if_do_dad(ifp) {
                    ifra.ifra_flags |= IN6_IFF_TENTATIVE;
                }

                // first, make or update the interface address structure, and link it to the
                // list. try to enable inet6 if there is no link-local yet.
                if let Err(e) = in6_ifattach(ifp) {
                    break 'err Err(e);
                }
                if let Err(e) = in6_update_ifa(ifp, ifra, ia6) {
                    break 'err Err(e);
                }

                ia6 = None;
                if let Some(p) = sa6 {
                    // SAFETY: as above.
                    let s = unsafe { ptr::read_unaligned(p) };
                    ia6 = in6ifa_ifpwithaddr(ifp, &s.sin6_addr);
                }
                let Some(ia6) = ia6 else {
                    // this can happen when the user specify the 0 valid lifetime.
                    break 'err Ok(());
                };

                // Perform DAD, if needed.
                if ia6.ia6_flags.get() & IN6_IFF_TENTATIVE != 0 {
                    nd6_dad_start(&ia6.ia_ifa);
                }

                if !newifaddr {
                    if_addrhooks_run(ifp);
                    break 'err Ok(());
                }

                let plen = in6_mask2len(&ia6_maskin6(ia6), None);
                if ifp.if_flags.get() & IFF_LOOPBACK != 0 || plen == 128 {
                    if_addrhooks_run(ifp);
                    break 'err Ok(()); // No need to install a connected route.
                }

                // The address's own socket address (`ifa_addr` is `ia_addr`), copied.
                let error = rt_ifa_add(
                    &ia6.ia_ifa,
                    RTF_CLONING | RTF_CONNECTED | RTF_MPATH,
                    sa_of(&ia6.ia_addr.get()),
                    ifp.if_rdomain.get(),
                );
                if let Err(e) = error {
                    in6_purgeaddr(&ia6.ia_ifa);
                    break 'err Err(e);
                }
                if_addrhooks_run(ifp);
                Ok(())
            }
            _ => Ok(()),
        }
    };

    // err:
    net_unlock();
    kernel_unlock();
    error
}

/// `in6_ioctl_get`: the address `ioctl`s that read: `SIOCGIFDSTADDR_IN6`,
/// `SIOCGIFNETMASK_IN6`, `SIOCGIFAFLAG_IN6`, `SIOCGIFALIFETIME_IN6`.
///
/// # Safety
///
/// As for [`in6_ioctl`]: `data` is a `struct in6_ifreq`.
unsafe fn in6_ioctl_get(cmd: u64, data: *mut u8, ifp: &'static Ifnet) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifr = unsafe { &mut *data.cast::<In6Ifreq>() };
    let mut addr = ifr.ifr_addr();
    let mut have_sa6 = false;

    if addr.sin6_family == AF_INET6 {
        addr.sin6_len = size_of::<SockaddrIn6>() as u8;
        ifr.set_ifr_addr(addr);
        // SAFETY: `addr` is a local `sockaddr_in6`.
        unsafe { in6_sa2sin6(sin6tosa(&mut addr)) }?;
        have_sa6 = true;
    }

    net_lock_shared();

    let error = 'err: {
        let mut ia6: Option<&'static In6Ifaddr> = None;
        if have_sa6 {
            let mut r = in6_check_embed_scope(&mut addr, ifp.if_index.get());
            if r.is_ok() {
                r = in6_clear_scope_id(&mut addr, ifp.if_index.get());
            }
            ifr.set_ifr_addr(addr);
            if let Err(e) = r {
                break 'err Err(e);
            }
            ia6 = in6ifa_ifpwithaddr(ifp, &addr.sin6_addr);
        }

        // must think again about its semantics
        let Some(ia6) = ia6 else {
            break 'err Err(Errno::EADDRNOTAVAIL);
        };

        match cmd {
            SIOCGIFDSTADDR_IN6 => {
                if ifp.if_flags.get() & IFF_POINTOPOINT == 0 {
                    break 'err Err(Errno::EINVAL);
                }
                // XXX: should we check if ifa_dstaddr is NULL and return an error?
                ifr.set_ifr_dstaddr(ia6.ia_dstaddr.get());
                Ok(())
            }

            SIOCGIFNETMASK_IN6 => {
                ifr.set_ifr_addr(ia6.ia_prefixmask.get());
                Ok(())
            }

            SIOCGIFAFLAG_IN6 => {
                ifr.set_ifr_flags6(ia6.ia6_flags.get());
                Ok(())
            }

            SIOCGIFALIFETIME_IN6 => {
                let lt = ia6.ia6_lifetime.get();
                let updatetime = ia6.ia6_updatetime.get();
                let mut retlt = lt;
                // XXX: adjust expiration time assuming time_t is signed.
                let maxexpire = Time::MAX;
                if lt.ia6t_vltime != ND6_INFINITE_LIFETIME {
                    if Time::from(lt.ia6t_vltime) < maxexpire - updatetime {
                        let mut expire = updatetime + Time::from(lt.ia6t_vltime);
                        if expire != 0 {
                            expire -= getuptime();
                            expire += gettime();
                        }
                        retlt.ia6t_expire = expire;
                    } else {
                        retlt.ia6t_expire = maxexpire;
                    }
                }
                if lt.ia6t_pltime != ND6_INFINITE_LIFETIME {
                    if Time::from(lt.ia6t_pltime) < maxexpire - updatetime {
                        let mut expire = updatetime + Time::from(lt.ia6t_pltime);
                        if expire != 0 {
                            expire -= getuptime();
                            expire += gettime();
                        }
                        retlt.ia6t_preferred = expire;
                    } else {
                        retlt.ia6t_preferred = maxexpire;
                    }
                }
                ifr.set_ifr_lifetime(retlt);
                Ok(())
            }

            _ => panic(format_args!("in6_ioctl_get: invalid ioctl {cmd}")),
        }
    };

    // err:
    net_unlock_shared();
    error
}

/// `in6_check_embed_scope`: embeds the interface index in a link-local address that does
/// not carry one, `EINVAL` if it carries another.
fn in6_check_embed_scope(sa6: &mut SockaddrIn6, ifidx: u32) -> Result<(), Errno> {
    if in6_is_addr_linklocal(&sa6.sin6_addr) {
        if sa6.sin6_addr.s6_addr16(1) == 0 {
            // link ID is not embedded by the user
            sa6.sin6_addr.set_s6_addr16(1, htons(ifidx as u16));
        } else if sa6.sin6_addr.s6_addr16(1) != htons(ifidx as u16) {
            return Err(Errno::EINVAL); // link ID contradicts
        }
    }
    Ok(())
}

/// `in6_clear_scope_id`: clears the scope id of a link-local address, `EINVAL` if it is
/// not the interface's.
fn in6_clear_scope_id(sa6: &mut SockaddrIn6, ifidx: u32) -> Result<(), Errno> {
    if in6_is_addr_linklocal(&sa6.sin6_addr) && sa6.sin6_scope_id != 0 {
        if sa6.sin6_scope_id != ifidx {
            return Err(Errno::EINVAL);
        }
        sa6.sin6_scope_id = 0; // XXX: good way?
    }
    Ok(())
}

/// Adds the route of the "all nodes" group `mltaddr`/32 through address `ia6` (the C's
/// repeated blocks in `in6_update_ifa`): looks the group up first and adds the route when
/// there is none whose first 32 bits match.
fn in6_add_mcast_route(
    ifp: &'static Ifnet,
    ia6: &'static In6Ifaddr,
    mltaddr: &SockaddrIn6,
    mltmask: &SockaddrIn6,
) -> Result<(), Errno> {
    let mut rt = rtalloc(sa_of(mltaddr), 0, ifp.if_rdomain.get());
    if let Some(r) = rt {
        // 32bit came from "mltmask": the key's `sin6_addr`, a `sockaddr_in6` here.
        let at = offset_of!(SockaddrIn6, sin6_addr);
        let key = rt_key(r).get(at..at + 4);
        if Some(&mltaddr.sin6_addr.s6_addr[..4]) != key {
            rtfree(Some(r));
            rt = None;
        }
    }
    match rt {
        None => {
            let ia_addr = ia6.ia_addr.get();
            let mut info = RtAddrinfo::new();
            info.rti_ifa = Some(&ia6.ia_ifa);
            info.rti_info[RTAX_DST] = Some(sa_view(sa_of(mltaddr)));
            info.rti_info[RTAX_GATEWAY] = Some(sa_view(sa_of(&ia_addr)));
            info.rti_info[RTAX_NETMASK] = Some(sa_view(sa_of(mltmask)));
            info.rti_info[RTAX_IFA] = Some(sa_view(sa_of(&ia_addr)));
            info.rti_flags = RTF_MULTICAST;
            rtrequest(
                RTM_ADD,
                &mut info,
                RTP_CONNECTED,
                None,
                ifp.if_rdomain.get(),
            )
        }
        Some(r) => {
            rtfree(Some(r));
            Ok(())
        }
    }
}

/// `in6_update_ifa`: update parameters of an IPv6 interface address; adds or changes the
/// address `ifra` describes on `ifp`. `ia6` is the existing address or `None` to allocate
/// one (and link it into the address chains). This function is separated from
/// `in6_control()`. The C's changes of `ifra->ifra_flags` stay in a local here.
pub fn in6_update_ifa(
    ifp: &'static Ifnet,
    ifra: &In6Aliasreq,
    ia6: Option<&'static In6Ifaddr>,
) -> Result<(), Errno> {
    let mut host_is_new = false;
    let mut ifra_flags = ifra.ifra_flags;

    net_assert_locked("in6_update_ifa");

    // Validate parameters: (ifp == NULL || ifra == NULL) cannot happen with references.

    // The destination address for a p2p link or the address of the announcing router for an
    // autoconf address must have a family of AF_INET6 or AF_UNSPEC.
    if ifp.if_flags.get() & (IFF_POINTOPOINT | IFF_LOOPBACK) != 0
        || ifra_flags & IN6_IFF_AUTOCONF != 0
    {
        if ifra.ifra_dstaddr.sin6_family != AF_INET6 && ifra.ifra_dstaddr.sin6_family != AF_UNSPEC {
            return Err(Errno::EAFNOSUPPORT);
        }
    } else if ifra.ifra_dstaddr.sin6_family != AF_UNSPEC {
        return Err(Errno::EINVAL);
    }

    // validate ifra_prefixmask. don't check sin6_family, netmask does not carry fields other
    // than sin6_len.
    if usize::from(ifra.ifra_prefixmask.sin6_len) > size_of::<SockaddrIn6>() {
        return Err(Errno::EINVAL);
    }
    // Because the IPv6 address architecture is classless, we require users to specify a
    // (non 0) prefix length (mask) for a new address. We also require the prefix (when
    // specified) mask is valid, and thus reject a non-consecutive mask.
    if ia6.is_none() && ifra.ifra_prefixmask.sin6_len == 0 {
        return Err(Errno::EINVAL);
    }
    let plen = if ifra.ifra_prefixmask.sin6_len != 0 {
        // The C's limit is `&ifra_prefixmask + sin6_len` against the mask at offset 8 of
        // the address; a length under 8 makes the (unsigned) distance huge: the whole mask.
        let lim = usize::from(ifra.ifra_prefixmask.sin6_len)
            .checked_sub(offset_of!(SockaddrIn6, sin6_addr));
        let plen = in6_mask2len(&ifra.ifra_prefixmask.sin6_addr, lim);
        if plen <= 0 {
            return Err(Errno::EINVAL);
        }
        plen
    } else {
        // In this case, ia6 must not be NULL. We just use its prefix length.
        match ia6 {
            Some(ia6) => in6_mask2len(&ia6_maskin6(ia6), None),
            None => return Err(Errno::EINVAL),
        }
    };

    let (mut dst6, mut gw6);
    if ifra_flags & IN6_IFF_AUTOCONF != 0 {
        gw6 = ifra.ifra_dstaddr;
        dst6 = SockaddrIn6::zeroed();
    } else {
        dst6 = ifra.ifra_dstaddr;
        gw6 = SockaddrIn6::zeroed();
    }
    if dst6.sin6_family == AF_INET6 {
        in6_check_embed_scope(&mut dst6, ifp.if_index.get())?;

        if ifp.if_flags.get() & (IFF_POINTOPOINT | IFF_LOOPBACK) != 0 && plen != 128 {
            return Err(Errno::EINVAL);
        }
    }
    if gw6.sin6_family == AF_INET6 {
        in6_check_embed_scope(&mut gw6, ifp.if_index.get())?;
    }
    // lifetime consistency check
    let lt = &ifra.ifra_lifetime;
    if lt.ia6t_pltime > lt.ia6t_vltime {
        return Err(Errno::EINVAL);
    }
    if lt.ia6t_vltime == 0 && ia6.is_none() {
        return Ok(()); // there's nothing to do
    }

    // If this is a new address, allocate a new ifaddr and link it into chains.
    let ia6 = match ia6 {
        Some(ia6) => ia6,
        None => {
            host_is_new = true;
            let Some(mem) = malloc(size_of::<In6Ifaddr>(), M_IFADDR, M_WAITOK | M_ZERO) else {
                panic(format_args!("in6_ifaddr: no memory"));
            };
            // SAFETY: a zeroed block of `size_of::<In6Ifaddr>()` bytes; all-zero is a valid
            // `In6Ifaddr` (cells, links, a reference count). It lives until the last
            // `ifafree`.
            let ia6: &'static In6Ifaddr = unsafe { &*mem.as_ptr().cast::<In6Ifaddr>() };
            ia6.ia_ifa.ifa_refcnt.init();
            ia6.ia6_memberships.init();
            // Initialize the address and masks, and put time stamp
            ia6.ia_ifa
                .ifa_addr
                .set(crate::net::if_var::sa_token(), Some(&ia6.ia_addr));
            let mut a = ia6.ia_addr.get();
            a.sin6_family = AF_INET6;
            a.sin6_len = size_of::<SockaddrIn6>() as u8;
            ia6.ia_addr.set(a);
            ia6.ia6_updatetime.set(getuptime());
            if ifp.if_flags.get() & (IFF_POINTOPOINT | IFF_LOOPBACK) != 0 {
                // XXX: some functions expect that ifa_dstaddr is not NULL for p2p
                // interfaces.
                ia6.ia_ifa
                    .ifa_dstaddr
                    .set(crate::net::if_var::sa_token(), Some(&ia6.ia_dstaddr));
            } else {
                ia6.ia_ifa.ifa_dstaddr.clear();
            }
            ia6.ia_ifa
                .ifa_netmask
                .set(crate::net::if_var::sa_token(), Some(&ia6.ia_prefixmask));

            ia6.ia_ifp().set(Some(ifp));
            ia6.ia_addr.set(*ifra.ifra_addr());
            ifa_add(ifp, &ia6.ia_ifa);
            ia6
        }
    };

    // The step that failed before the address is complete: a new address is unlinked again.
    let unlink = |error: Errno| -> Result<(), Errno> {
        // XXX: if a change of an existing address failed, keep the entry anyway.
        if host_is_new {
            in6_unlink_ifa(ia6, ifp);
        }
        Err(error)
    };

    // set prefix mask
    if ifra.ifra_prefixmask.sin6_len != 0 {
        // We prohibit changing the prefix length of an existing address, because
        // + such an operation should be rare in IPv6, and
        // + the operation would confuse prefix management.
        if ia6.ia_prefixmask.get().sin6_len != 0 && in6_mask2len(&ia6_maskin6(ia6), None) != plen {
            return unlink(Errno::EINVAL);
        }
        ia6.ia_prefixmask.set(ifra.ifra_prefixmask);
    }

    // If a new destination address is specified, scrub the old one and install the new
    // destination.
    if ifp.if_flags.get() & (IFF_POINTOPOINT | IFF_LOOPBACK) != 0
        && dst6.sin6_family == AF_INET6
        && !in6_are_addr_equal(&dst6.sin6_addr, &ia6_dstin6(ia6))
    {
        let ifa = &ia6.ia_ifa;

        if ia6.ia_flags().get() & IFA_ROUTE != 0 {
            // The address's own destination address (`ifa_dstaddr` is `ia_dstaddr`).
            let r = rt_ifa_del(
                ifa,
                RTF_HOST,
                sa_of(&ia6.ia_dstaddr.get()),
                ifp.if_rdomain.get(),
            );
            if r.is_ok() {
                ia6.ia_flags().set(ia6.ia_flags().get() & !IFA_ROUTE);
            }
        }
        ia6.ia_dstaddr.set(dst6);
    }

    if ifra_flags & IN6_IFF_AUTOCONF != 0
        && gw6.sin6_family == AF_INET6
        && !in6_are_addr_equal(&dst6.sin6_addr, &ia6.ia_gwaddr.get().sin6_addr)
    {
        // Set or update announcing router
        ia6.ia_gwaddr.set(gw6);
    }

    // Set lifetimes. We do not refer to ia6t_expire and ia6t_preferred to see if the address
    // is deprecated or invalidated, but initialize these members for applications.
    ia6.ia6_updatetime.set(getuptime());
    let mut lt = ifra.ifra_lifetime;
    if lt.ia6t_vltime != ND6_INFINITE_LIFETIME {
        lt.ia6t_expire = getuptime() + Time::from(lt.ia6t_vltime);
    } else {
        lt.ia6t_expire = 0;
    }
    if lt.ia6t_pltime != ND6_INFINITE_LIFETIME {
        lt.ia6t_preferred = getuptime() + Time::from(lt.ia6t_pltime);
    } else {
        lt.ia6t_preferred = 0;
    }
    ia6.ia6_lifetime.set(lt);

    // reset the interface and routing table appropriately.
    if let Err(e) = in6_ifinit(ifp, ia6, host_is_new) {
        return unlink(e);
    }

    // re-run DAD
    if ia6.ia6_flags.get() & (IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED) != 0 {
        ifra_flags |= IN6_IFF_TENTATIVE;
    }
    // configure address flags.
    ia6.ia6_flags.set(ifra_flags);

    nd6_expire_timer_update(ia6);

    // We are done if we have simply modified an existing address.
    if !host_is_new {
        // DAD sends RTM_CHGADDRATTR when done.
        if ia6.ia6_flags.get() & IN6_IFF_TENTATIVE == 0 {
            rtm_addr(RTM_CHGADDRATTR, &ia6.ia_ifa);
        }
        return Ok(());
    }

    // Beyond this point, we should call in6_purgeaddr upon an error, not just go to unlink.

    // join necessary multiast groups
    if ifp.if_flags.get() & IFF_MULTICAST != 0 {
        let cleanup = |error: Errno| -> Result<(), Errno> {
            in6_purgeaddr(&ia6.ia_ifa);
            Err(error)
        };
        // The membership of a joined group goes on the address's list.
        let join = |addr: &In6Addr| -> Result<(), Errno> {
            let imm = in6_joingroup(ifp, addr)?;
            // SAFETY: the membership lives until `in6_leavegroup` frees it, which
            // `in6_purgeaddr` does after taking it off this list.
            unsafe { ia6.ia6_memberships.insert_head(imm) };
            Ok(())
        };

        // join solicited multicast addr for new host id
        let mut llsol = SockaddrIn6::zeroed();
        llsol.sin6_family = AF_INET6;
        llsol.sin6_len = size_of::<SockaddrIn6>() as u8;
        llsol.sin6_addr.set_s6_addr16(0, htons(0xff02));
        llsol
            .sin6_addr
            .set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        llsol.sin6_addr.set_s6_addr32(1, 0);
        llsol.sin6_addr.set_s6_addr32(2, htonl(1));
        llsol
            .sin6_addr
            .set_s6_addr32(3, ifra.ifra_addr().sin6_addr.s6_addr32(3));
        llsol.sin6_addr.s6_addr[12] = 0xff;
        if let Err(e) = join(&llsol.sin6_addr) {
            return cleanup(e);
        }

        let mut mltmask = SockaddrIn6::zeroed();
        mltmask.sin6_len = size_of::<SockaddrIn6>() as u8;
        mltmask.sin6_family = AF_INET6;
        mltmask.sin6_addr = IN6MASK32;

        // join link-local all-nodes address
        let mut mltaddr = SockaddrIn6::zeroed();
        mltaddr.sin6_len = size_of::<SockaddrIn6>() as u8;
        mltaddr.sin6_family = AF_INET6;
        mltaddr.sin6_addr = IN6ADDR_LINKLOCAL_ALLNODES;
        mltaddr
            .sin6_addr
            .set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        mltaddr.sin6_scope_id = 0;

        // XXX: do we really need this automatic routes? We should probably reconsider this
        // stuff. Most applications actually do not need the routes, since they usually
        // specify the outgoing interface.
        if let Err(e) = in6_add_mcast_route(ifp, ia6, &mltaddr, &mltmask) {
            return cleanup(e);
        }
        if let Err(e) = join(&mltaddr.sin6_addr) {
            return cleanup(e);
        }

        // join interface-local all-nodes address. (ff01::1%ifN, and ff01::%ifN/32)
        let mut mltaddr = SockaddrIn6::zeroed();
        mltaddr.sin6_len = size_of::<SockaddrIn6>() as u8;
        mltaddr.sin6_family = AF_INET6;
        mltaddr.sin6_addr = IN6ADDR_INTFACELOCAL_ALLNODES;
        mltaddr
            .sin6_addr
            .set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        mltaddr.sin6_scope_id = 0;

        // XXX: again, do we really need the route?
        if let Err(e) = in6_add_mcast_route(ifp, ia6, &mltaddr, &mltmask) {
            return cleanup(e);
        }
        if let Err(e) = join(&mltaddr.sin6_addr) {
            return cleanup(e);
        }
    }

    Ok(())
}

/// `in6_purgeaddr`: removes an IPv6 address and its routes and memberships.
pub fn in6_purgeaddr(ifa: &'static Ifaddr) {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        panic(format_args!("in6_purgeaddr: address without interface"));
    };
    let ia6 = ifatoia6(ifa);

    // stop DAD processing
    nd6_dad_stop(ifa);

    // delete route to the destination of the address being purged. The interface must be p2p
    // or loopback in this case.
    if ifp.if_flags.get() & IFF_POINTOPOINT != 0
        && ia6.ia_flags().get() & IFA_ROUTE != 0
        && ia6.ia_dstaddr.get().sin6_len != 0
    {
        // The address's own destination address (`ifa_dstaddr` is `ia_dstaddr`).
        let r = rt_ifa_del(
            ifa,
            RTF_HOST,
            sa_of(&ia6.ia_dstaddr.get()),
            ifp.if_rdomain.get(),
        );
        if r.is_ok() {
            ia6.ia_flags().set(ia6.ia_flags().get() & !IFA_ROUTE);
        }
    }

    // Remove ownaddr's loopback rtentry, if it exists.
    let _ = rt_ifa_dellocal(&ia6.ia_ifa);

    // leave from multicast groups we have joined for the interface
    while let Some(imm) = ia6.ia6_memberships.first() {
        // SAFETY: a membership on the list lives until `in6_leavegroup` frees it.
        let imm: &'static In6MultiMship = unsafe { &*ptr::from_ref(imm) };
        // SAFETY: `imm` is on `ia6`'s list (just found there).
        unsafe { ListHead::<In6MultiMshipList>::remove(imm) };
        in6_leavegroup(imm);
    }

    in6_unlink_ifa(ia6, ifp);
}

/// `in6_unlink_ifa`: takes an address off its interface and drops the list's reference.
fn in6_unlink_ifa(ia6: &'static In6Ifaddr, ifp: &'static Ifnet) {
    let ifa = &ia6.ia_ifa;

    net_assert_locked("in6_unlink_ifa");

    // Release the reference to the base prefix.
    let plen = in6_mask2len(&ia6_maskin6(ia6), None);
    if ifp.if_flags.get() & IFF_LOOPBACK == 0 && plen != 128 {
        // The address's own socket address (`ifa_addr` is `ia_addr`).
        let _ = rt_ifa_del(
            ifa,
            RTF_CLONING | RTF_CONNECTED,
            sa_of(&ia6.ia_addr.get()),
            ifp.if_rdomain.get(),
        );
    }

    rt_ifa_purge(ifa);
    ifa_del(ifp, ifa);

    ia6.ia_ifp().set(None);
    ifafree(ifa);
}

/// `in6_ifinit`: initialize an interface's inet6 address and routing table entry.
fn in6_ifinit(ifp: &'static Ifnet, ia6: &'static In6Ifaddr, newhost: bool) -> Result<(), Errno> {
    let mut ifacount = 0;

    net_assert_locked("in6_ifinit");

    // Give the interface a chance to initialize if this is its first address (or it is a CARP
    // interface) and to validate the address if necessary.
    for ifa in ifp.if_addrlist.iter() {
        if ifa_family(ifa) != AF_INET6 {
            continue;
        }
        ifacount += 1;
    }

    if ifacount <= 1
        || ifp.if_type.get() == IFT_CARP
        || ifp.if_flags.get() & (IFF_LOOPBACK | IFF_POINTOPOINT) != 0
    {
        // SAFETY: the C hands the driver the `in6_ifaddr` for SIOCSIFADDR; drivers read it as
        // the `struct ifaddr` it starts with, if at all.
        unsafe { ifp_ioctl(ifp, SIOCSIFADDR, ptr::from_ref(ia6).cast_mut().cast()) }?;
    }

    ia6.ia_ifa.ifa_metric.set(ifp.if_metric.get() as i32);

    // we could do in(6)_socktrim here, but just omit it at this moment.

    // Special case: If the destination address is specified for a point-to-point interface,
    // install a route to the destination as an interface direct route.
    let plen = in6_mask2len(&ia6_maskin6(ia6), None); // XXX
    if ifp.if_flags.get() & IFF_POINTOPOINT != 0
        && plen == 128
        && ia6.ia_dstaddr.get().sin6_family == AF_INET6
    {
        let ifa = &ia6.ia_ifa;
        // The address's own destination address (`ifa_dstaddr` is `ia_dstaddr`).
        rt_ifa_add(
            ifa,
            RTF_HOST | RTF_MPATH,
            sa_of(&ia6.ia_dstaddr.get()),
            ifp.if_rdomain.get(),
        )?;
        ia6.ia_flags().set(ia6.ia_flags().get() | IFA_ROUTE);
    }

    if newhost {
        return rt_ifa_addlocal(&ia6.ia_ifa);
    }

    Ok(())
}

/// `in6_lookupmulti`: looks up the `in6_multi` record for a given IP6 multicast address on a
/// given interface; the matching record if found.
pub fn in6_lookupmulti(addr: &In6Addr, ifp: &Ifnet) -> Option<&'static In6Multi> {
    crate::net::if_var::if_maddrlock_assert_anylock(ifp);

    for ifma in ifp.if_maddrlist.iter() {
        // SAFETY: a record's address is readable (its protocol set it).
        if unsafe { (*ifma.ifma_addr.get()).sa_family } == AF_INET6
            && in6_are_addr_equal(&ifmatoin6m(ifma).in6m_addr(), addr)
        {
            // SAFETY: a record on the list lives until `in6_delmulti` frees it.
            return Some(unsafe { &*ptr::from_ref(ifmatoin6m(ifma)) });
        }
    }
    None
}

/// `in6_addmulti`: adds an address to the list of IP6 multicast addresses for a given
/// interface (or takes a reference on the record if it is there); the C's `*errorp` is the
/// `Err`.
pub fn in6_addmulti(addr: &In6Addr, ifp: &'static Ifnet) -> Result<&'static In6Multi, Errno> {
    // See if address already in list.
    crate::net::if_var::if_maddrlock_enter_write(ifp);
    if let Some(in6m) = in6_lookupmulti(addr, ifp) {
        in6m.in6m_refcnt().take();
        crate::net::if_var::if_maddrlock_exit_write(ifp);
        return Ok(in6m);
    }
    crate::net::if_var::if_maddrlock_exit_write(ifp);

    // New address; allocate a new multicast record and link it into the interface's
    // multicast list.
    let Some(mem) = malloc(size_of::<In6Multi>(), M_IPMADDR, M_NOWAIT | M_ZERO) else {
        return Err(Errno::ENOBUFS);
    };
    // SAFETY: a zeroed block of `size_of::<In6Multi>()` bytes; all-zero is a valid
    // `In6Multi`.
    let new_in6m: &'static In6Multi = unsafe { &*mem.as_ptr().cast::<In6Multi>() };

    // Ask the network driver to update its multicast reception filter appropriately for the
    // new address.
    let mut ifr = In6Ifreq::zeroed();
    let mut sin6 = SockaddrIn6::zeroed();
    sin6.sin6_len = size_of::<SockaddrIn6>() as u8;
    sin6.sin6_family = AF_INET6;
    sin6.sin6_addr = *addr;
    ifr.set_ifr_addr(sin6);
    kernel_lock();
    // SAFETY: `ifr` is a `struct in6_ifreq`, what SIOCADDMULTI takes here.
    let error = unsafe { ifp_ioctl(ifp, SIOCADDMULTI, ptr::from_mut(&mut ifr).cast()) };
    kernel_unlock();
    if let Err(e) = error {
        free(mem, M_IPMADDR, size_of::<In6Multi>());
        return Err(e);
    }

    crate::net::if_var::if_maddrlock_enter_write(ifp);
    // check again after unlock and lock
    if let Some(in6m) = in6_lookupmulti(addr, ifp) {
        in6m.in6m_refcnt().take();
        crate::net::if_var::if_maddrlock_exit_write(ifp);
        free(mem, M_IPMADDR, size_of::<In6Multi>());
        return Ok(in6m);
    }
    let in6m = new_in6m;
    let mut sin = in6m.in6m_sin.get();
    sin.sin6_len = size_of::<SockaddrIn6>() as u8;
    sin.sin6_family = AF_INET6;
    sin.sin6_addr = *addr;
    in6m.in6m_sin.set(sin);
    in6m.in6m_refcnt().init();
    in6m.in6m_ifidx().set(ifp.if_index.get());
    in6m.in6m_ifma
        .ifma_addr
        .set(crate::net::if_var::sa_token(), Some(&in6m.in6m_sin));

    // Let MLD6 know that we have joined a new IP6 multicast group.
    ifp.if_maddrlist.insert_head(&in6m.in6m_ifma);
    let mut pkt = Mld6Pktinfo::default(); // pkt.mpi_ifidx = 0
    mld6_start_listening(in6m, ifp, &mut pkt);
    crate::net::if_var::if_maddrlock_exit_write(ifp);

    if pkt.mpi_ifidx != 0 {
        mld6_sendpkt(&pkt);
    }

    Ok(in6m)
}

/// `in6_delmulti`: deletes a multicast address record: drops a reference, leaving the group
/// with the last one.
pub fn in6_delmulti(in6m: &'static In6Multi) {
    if !in6m.in6m_refcnt().rele() {
        return;
    }

    let ifp = if_get(in6m.in6m_ifidx().get());
    if let Some(ifp) = ifp {
        crate::net::if_var::if_maddrlock_enter_write(ifp);
        // No remaining claims to this record; let MLD6 know that we are leaving the multicast
        // group.
        let mut pkt = Mld6Pktinfo::default(); // pkt.mpi_ifidx = 0
        mld6_stop_listening(in6m, ifp, &mut pkt);
        ifp.if_maddrlist.remove(&in6m.in6m_ifma);
        crate::net::if_var::if_maddrlock_exit_write(ifp);

        if pkt.mpi_ifidx != 0 {
            mld6_sendpkt(&pkt);
        }

        // Notify the network driver to update its multicast reception filter.
        let mut ifr = In6Ifreq::zeroed();
        let mut sin6 = SockaddrIn6::zeroed();
        sin6.sin6_len = size_of::<SockaddrIn6>() as u8;
        sin6.sin6_family = AF_INET6;
        sin6.sin6_addr = in6m.in6m_addr();
        ifr.set_ifr_addr(sin6);
        kernel_lock();
        // SAFETY: `ifr` is a `struct in6_ifreq`, what SIOCDELMULTI takes here.
        let _ = unsafe { ifp_ioctl(ifp, SIOCDELMULTI, ptr::from_mut(&mut ifr).cast()) };
        kernel_unlock();

        if_put(ifp);
    }

    free(NonNull::from(in6m).cast(), M_IPMADDR, size_of::<In6Multi>());
}

/// `in6_hasmulti`: whether the multicast group `addr` has been joined by interface `ifp`.
pub fn in6_hasmulti(addr: &In6Addr, ifp: &Ifnet) -> bool {
    crate::net::if_var::if_maddrlock_enter_read(ifp);
    let joined = in6_lookupmulti(addr, ifp).is_some();
    crate::net::if_var::if_maddrlock_exit_read(ifp);

    joined
}

/// `in6_joingroup`: joins `addr` on `ifp` and returns the membership entry (`malloc`ed,
/// freed by `in6_leavegroup`); the C's `*errorp` is the `Err`.
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

/// `in6_leavegroup`: leaves the group of membership `imm` and frees it.
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

/// `in6ifa_ifpforlinklocal`: finds an IPv6 interface link-local address specific to an
/// interface, one without any of the `IN6_IFF_*` flags `ignoreflags`.
pub fn in6ifa_ifpforlinklocal(ifp: &Ifnet, ignoreflags: i32) -> Option<&'static In6Ifaddr> {
    for ifa in ifp.if_addrlist.iter() {
        if ifa_family(ifa) != AF_INET6 {
            continue;
        }
        if in6_is_addr_linklocal(&ifa_in6(ifa)) {
            if ia6_static(ifa).ia6_flags.get() & ignoreflags != 0 {
                continue;
            }
            return Some(ia6_static(ifa));
        }
    }

    None
}

/// `in6ifa_ifpwithaddr`: finds the internet address corresponding to a given interface and
/// address.
pub fn in6ifa_ifpwithaddr(ifp: &Ifnet, addr: &In6Addr) -> Option<&'static In6Ifaddr> {
    for ifa in ifp.if_addrlist.iter() {
        if ifa_family(ifa) != AF_INET6 {
            continue;
        }
        if in6_are_addr_equal(addr, &ifa_in6(ifa)) {
            return Some(ia6_static(ifa));
        }
    }

    None
}

/// `in6_addrscope`: get a scope of the address. Node-local, link-local, site-local or
/// global (`__IPV6_ADDR_SCOPE_*`).
pub fn in6_addrscope(addr: &In6Addr) -> i32 {
    if addr.s6_addr8(0) == 0xfe {
        let scope = addr.s6_addr8(1) & 0xc0;

        return match scope {
            0x80 => i32::from(__IPV6_ADDR_SCOPE_LINKLOCAL),
            0xc0 => i32::from(__IPV6_ADDR_SCOPE_SITELOCAL),
            _ => i32::from(__IPV6_ADDR_SCOPE_GLOBAL), // just in case
        };
    }

    if addr.s6_addr8(0) == 0xff {
        let scope = addr.s6_addr8(1) & 0x0f;

        // due to other scope such as reserved, return scope doesn't work.
        return match scope {
            __IPV6_ADDR_SCOPE_INTFACELOCAL => i32::from(__IPV6_ADDR_SCOPE_INTFACELOCAL),
            __IPV6_ADDR_SCOPE_LINKLOCAL => i32::from(__IPV6_ADDR_SCOPE_LINKLOCAL),
            __IPV6_ADDR_SCOPE_SITELOCAL => i32::from(__IPV6_ADDR_SCOPE_SITELOCAL),
            _ => i32::from(__IPV6_ADDR_SCOPE_GLOBAL),
        };
    }

    if addr.s6_addr[..15] == IN6ADDR_LOOPBACK.s6_addr[..15] {
        if addr.s6_addr8(15) == 1 {
            // loopback
            return i32::from(__IPV6_ADDR_SCOPE_INTFACELOCAL);
        }
        if addr.s6_addr8(15) == 0 {
            // unspecified
            return i32::from(__IPV6_ADDR_SCOPE_LINKLOCAL);
        }
    }

    i32::from(__IPV6_ADDR_SCOPE_GLOBAL)
}

/// `in6_addr2scopeid`: the scope zone id of `addr` on interface `ifidx` (the interface
/// index for link- and interface-local scopes, 0 otherwise).
pub fn in6_addr2scopeid(ifidx: u32, addr: &In6Addr) -> i32 {
    let scope = in6_addrscope(addr);

    // XXX: we do not distinguish between a link and an I/F. A site-local scope is invalid
    // (0) and the rest is treated as global (0).
    if scope == i32::from(__IPV6_ADDR_SCOPE_INTFACELOCAL)
        || scope == i32::from(__IPV6_ADDR_SCOPE_LINKLOCAL)
    {
        ifidx as i32
    } else {
        0
    }
}

/// `in6_matchlen`: the length of the common prefix of `src` and `dst`, in bits (the part
/// where dst and src are equal).
pub fn in6_matchlen(src: &In6Addr, dst: &In6Addr) -> i32 {
    let mut matched = 0;

    for (&s, &d) in src.s6_addr.iter().zip(dst.s6_addr.iter()) {
        let mut r = d ^ s;
        if r != 0 {
            while r < 128 {
                matched += 1;
                r <<= 1;
            }
            break;
        }
        matched += 8;
    }
    matched
}

/// `in6_prefixlen2mask`: the mask of prefix length `len`.
pub fn in6_prefixlen2mask(maskp: &mut In6Addr, len: i32) {
    const MASKARRAY: [u8; 8] = [0x80, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc, 0xfe, 0xff];

    // sanity check
    if !(0..=128).contains(&len) {
        log(
            LOG_ERR,
            format_args!("in6_prefixlen2mask: invalid prefix length({len})\n"),
        );
        return;
    }

    *maskp = In6Addr::default();
    let bytelen = (len / 8) as usize;
    let bitlen = (len % 8) as usize;
    for b in &mut maskp.s6_addr[..bytelen] {
        *b = 0xff;
    }
    // len == 128 is ok because bitlen == 0 then
    if bitlen != 0 {
        maskp.s6_addr[bytelen] = MASKARRAY[bitlen - 1];
    }
}

/// `in6_ifawithscope`: return the best address out of the same scope (the RFC 6724 source
/// address selection rules as the C applies them), for `dst` on `oifp`, considering the
/// route `rt` and routing domain `rdomain`.
pub fn in6_ifawithscope(
    oifp: &Ifnet,
    dst: &In6Addr,
    rdomain: u32,
    rt: Option<&Rtentry>,
) -> Option<&'static In6Ifaddr> {
    /// The verdict on one candidate address: the C's `continue` or `goto replace`.
    enum Verdict {
        Skip,
        Replace,
    }

    let dst_scope = in6_addrscope(dst);
    let mut best_scope = 0;
    let mut blen = -1;
    let mut ia6_best: Option<&'static In6Ifaddr> = None;
    let mut gw6: Option<In6Addr> = None;

    if let Some(rt) = rt
        && let Some(gw) = rt.gateway_copy()
        && gw.view().family() == AF_INET6
    {
        // An `AF_INET6` gateway is a `sockaddr_in6`.
        gw6 = hal::pod::at::<SockaddrIn6>(gw.bytes(), 0).map(|s| s.sin6_addr);
    }

    net_assert_locked("in6_ifawithscope");

    // We search for all addresses on all interfaces from the beginning.
    for ifp in IFNETLIST.0.iter() {
        if ifp.if_rdomain.get() != rdomain {
            continue;
        }
        // NCARP: never use a carp address of an interface which is not the master
        // (`carp_iamatch`); carp is not configured.

        // We can never take an address that breaks the scope zone of the destination.
        if in6_addr2scopeid(ifp.if_index.get(), dst) != in6_addr2scopeid(oifp.if_index.get(), dst) {
            continue;
        }

        for ifa in ifp.if_addrlist.iter() {
            let mut tlen = -1;

            if ifa_family(ifa) != AF_INET6 {
                continue;
            }

            let ia = ia6_static(ifa);
            let ifa_addr6 = ifa_in6(ifa);
            let src_scope = in6_addrscope(&ifa_addr6);

            // Don't use an address before completing DAD nor a duplicated address.
            if ia.ia6_flags.get() & (IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED) != 0 {
                continue;
            }

            // RFC 6724 allows anycast addresses as source address because the restriction
            // was removed in RFC 4291. However RFC 4443 states that ICMPv6 responses MUST
            // use a unicast source address.
            //
            // XXX Skip anycast addresses for now since icmp6_reflect() uses this function
            // for source address selection.
            if ia.ia6_flags.get() & IN6_IFF_ANYCAST != 0 {
                continue;
            }

            if ia.ia6_flags.get() & IN6_IFF_DETACHED != 0 {
                continue;
            }

            let verdict = 'decide: {
                // If this is the first address we find, keep it anyway.
                let Some(best) = ia6_best else {
                    break 'decide Verdict::Replace;
                };

                // `best` is never NULL beyond this line except within the block labeled
                // "replace".

                // Rule 2: Prefer appropriate scope. Find the address with the smallest scope
                // that is bigger (or equal) to the scope of the destination address. Accept
                // an address with smaller scope than the destination if non exists with
                // bigger scope.
                if best_scope < src_scope {
                    if best_scope < dst_scope {
                        break 'decide Verdict::Replace;
                    }
                    break 'decide Verdict::Skip;
                } else if src_scope < best_scope {
                    if src_scope < dst_scope {
                        break 'decide Verdict::Skip;
                    }
                    break 'decide Verdict::Replace;
                }

                // Rule 3: Avoid deprecated addresses.
                if ia.ia6_flags.get() & IN6_IFF_DEPRECATED != 0 {
                    // If we have already found a non-deprecated candidate, just ignore
                    // deprecated addresses.
                    if best.ia6_flags.get() & IN6_IFF_DEPRECATED == 0 {
                        break 'decide Verdict::Skip;
                    }
                } else if best.ia6_flags.get() & IN6_IFF_DEPRECATED != 0 {
                    break 'decide Verdict::Replace;
                }

                // Rule 4: Prefer home addresses. We do not support home addresses.

                // Rule 5: Prefer outgoing interface
                let best_on_oifp = best.ia_ifp().get().is_some_and(|b| ptr::eq(b, oifp));
                let ifp_is_oifp = ptr::eq(ifp, oifp);
                if best_on_oifp && !ifp_is_oifp {
                    break 'decide Verdict::Skip;
                }
                if !best_on_oifp && ifp_is_oifp {
                    break 'decide Verdict::Replace;
                }

                // Rule 5.5: Prefer addresses in a prefix advertised by the next-hop.
                if let Some(gw6) = &gw6 {
                    let in6_bestgw = best.ia_gwaddr.get().sin6_addr;
                    let in6_newgw = ia.ia_gwaddr.get().sin6_addr;
                    if !in6_are_addr_equal(&in6_bestgw, gw6) && in6_are_addr_equal(&in6_newgw, gw6)
                    {
                        break 'decide Verdict::Replace;
                    }
                }

                // Rule 6: Prefer matching label. We do not implement policy tables.

                // Rule 7: Prefer temporary addresses.
                if best.ia6_flags.get() & IN6_IFF_TEMPORARY != 0
                    && ia.ia6_flags.get() & IN6_IFF_TEMPORARY == 0
                {
                    break 'decide Verdict::Skip;
                }
                if best.ia6_flags.get() & IN6_IFF_TEMPORARY == 0
                    && ia.ia6_flags.get() & IN6_IFF_TEMPORARY != 0
                {
                    break 'decide Verdict::Replace;
                }

                // Rule 8: Use longest matching prefix.
                tlen = in6_matchlen(&ifa_addr6, dst);
                if tlen > blen {
                    // NCARP: don't let carp interfaces win a tie against the output
                    // interface based on matchlen (a carp address is only used if no other
                    // interface has a usable one); carp is not configured.
                    break 'decide Verdict::Replace;
                } else if tlen < blen {
                    break 'decide Verdict::Skip;
                }

                // If the eight rules fail to choose a single address, the tiebreaker is
                // implementation-specific.

                // Prefer address with highest pltime.
                let best_pl =
                    best.ia6_updatetime.get() + Time::from(best.ia6_lifetime.get().ia6t_pltime);
                let new_pl =
                    ia.ia6_updatetime.get() + Time::from(ia.ia6_lifetime.get().ia6t_pltime);
                if best_pl < new_pl {
                    break 'decide Verdict::Replace;
                } else if best_pl > new_pl {
                    break 'decide Verdict::Skip;
                }

                // Prefer address with highest vltime.
                let best_vl =
                    best.ia6_updatetime.get() + Time::from(best.ia6_lifetime.get().ia6t_vltime);
                let new_vl =
                    ia.ia6_updatetime.get() + Time::from(ia.ia6_lifetime.get().ia6t_vltime);
                if best_vl < new_vl {
                    break 'decide Verdict::Replace;
                }

                Verdict::Skip
            };

            if let Verdict::Replace = verdict {
                ia6_best = Some(ia);
                blen = if tlen >= 0 {
                    tlen
                } else {
                    in6_matchlen(&ifa_addr6, dst)
                };
                best_scope = in6_addrscope(&ia6_sin6(ia).sin6_addr);
            }
        }
    }

    // count statistics for future improvements
    match ia6_best {
        None => ip6stat_inc(Ip6statCounters::Ip6sSourcesNone),
        Some(best) => {
            let scope = best_scope as usize;
            if best.ia_ifp().get().is_some_and(|b| ptr::eq(b, oifp)) {
                ip6stat_inc_idx(Ip6statCounters::Ip6sSourcesSameif, scope);
            } else {
                ip6stat_inc_idx(Ip6statCounters::Ip6sSourcesOtherif, scope);
            }

            if best_scope == dst_scope {
                ip6stat_inc_idx(Ip6statCounters::Ip6sSourcesSamescope, scope);
            } else {
                ip6stat_inc_idx(Ip6statCounters::Ip6sSourcesOtherscope, scope);
            }

            if best.ia6_flags.get() & IN6_IFF_DEPRECATED != 0 {
                ip6stat_inc_idx(Ip6statCounters::Ip6sSourcesDeprecated, scope);
            }
        }
    }

    ia6_best
}

/// `in6if_do_dad`: whether Duplicate Address Detection runs on `ifp`.
pub fn in6if_do_dad(ifp: &Ifnet) -> bool {
    if ifp.if_flags.get() & IFF_LOOPBACK != 0 {
        return false;
    }

    // NCARP: DAD does not work currently on carp(4), so it is disabled for IFT_CARP; carp is
    // not configured.

    // Our DAD routine requires the interface up and running. However, some interfaces can be
    // up before the RUNNING status. Additionally, users may try to assign addresses before the
    // interface becomes up (or running). We simply skip DAD in such a case as a work around.
    // XXX: we should rather mark "tentative" on such addresses, and do DAD after the
    // interface becomes ready.
    ifp.if_flags.get() & (IFF_UP | IFF_RUNNING) == IFF_UP | IFF_RUNNING
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    use crate::net::if_var::IfnetLegacy;

    use super::*;

    /// `a:b:c:d:e:f:g:h` from its eight 16-bit groups.
    fn addr(g: [u16; 8]) -> In6Addr {
        let mut a = In6Addr::default();
        for (i, w) in g.iter().enumerate() {
            a.s6_addr[2 * i..2 * i + 2].copy_from_slice(&w.to_be_bytes());
        }
        a
    }

    // ---- in6.c ----

    use std::sync::MutexGuard;
    use std::vec::Vec;

    use crate::net::if_::tests::{setup_net, test_ifnet, test_packet, zeroed_static};
    use crate::net::if_::{IFF_MULTICAST, IFF_RUNNING, IFF_UP, if_attach, ifa_add};
    use crate::netinet6::in6_var::ia6_in6;
    use crate::netinet6::in6_var::{
        IN6_IFF_ANYCAST, IN6_IFF_DEPRECATED, IN6_IFF_DETACHED, IN6_IFF_DUPLICATED,
        IN6_IFF_TEMPORARY, IN6_IFF_TENTATIVE, In6Addrlifetime,
    };
    use crate::sys::systm::{net_lock, net_unlock};

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
    /// `in6_update_ifa` does, with infinite lifetimes.
    pub(crate) fn test_ia6(ifp: &'static Ifnet, a: In6Addr, flags: i32) -> &'static In6Ifaddr {
        // SAFETY: the all-zero `In6Ifaddr` is valid (`netinet6/in6_var.rs`).
        let ia: &'static In6Ifaddr = unsafe { zeroed_static() };
        ia.ia6_memberships.init();
        ia.ia_ifa
            .ifa_addr
            .set(crate::net::if_var::sa_token(), Some(&ia.ia_addr));
        ia.ia_ifa
            .ifa_netmask
            .set(crate::net::if_var::sa_token(), Some(&ia.ia_prefixmask));
        let mut sin6 = SockaddrIn6::with_addr(a);
        sin6.sin6_len = size_of::<SockaddrIn6>() as u8;
        ia.ia_addr.set(sin6);
        let mut mask = SockaddrIn6::with_addr(IN6MASK64);
        mask.sin6_len = size_of::<SockaddrIn6>() as u8;
        ia.ia_prefixmask.set(mask);
        ia.ia_ifp().set(Some(ifp));
        ia.ia6_flags.set(flags);
        ia.ia6_lifetime.set(In6Addrlifetime {
            ia6t_vltime: ND6_INFINITE_LIFETIME,
            ia6t_pltime: ND6_INFINITE_LIFETIME,
            ..In6Addrlifetime::default()
        });
        net_lock();
        // in6_update_ifa: refcnt_init(&ia6->ia_ifa.ifa_refcnt).
        ia.ia_ifa.ifa_refcnt.init();
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
    fn mask2len_and_prefixlen2mask_round_trip() {
        for len in 0..=128 {
            let mut m = In6Addr::default();
            in6_prefixlen2mask(&mut m, len);
            assert_eq!(in6_mask2len(&m, None), len, "len {len}");
            assert_eq!(in6_mask2len(&m, Some(16)), len);
            // Bit by bit: the first `len` bits are set.
            for bit in 0..128usize {
                let set = m.s6_addr[bit / 8] & (0x80 >> (bit % 8)) != 0;
                assert_eq!(set, (bit as i32) < len, "len {len} bit {bit}");
            }
        }
        assert_eq!(IN6MASK0, {
            let mut m = IN6MASK128;
            in6_prefixlen2mask(&mut m, 0);
            m
        });
        for (len, want) in [
            (32, IN6MASK32),
            (64, IN6MASK64),
            (96, IN6MASK96),
            (128, IN6MASK128),
        ] {
            let mut m = In6Addr::default();
            in6_prefixlen2mask(&mut m, len);
            assert_eq!(m, want);
        }
    }

    #[test]
    fn prefixlen2mask_ignores_invalid_lengths() {
        let _g = setup();
        for bad in [-1, 129, i32::MAX, i32::MIN] {
            let mut m = IN6MASK64;
            in6_prefixlen2mask(&mut m, bad);
            assert_eq!(m, IN6MASK64, "an invalid length leaves the mask alone");
        }
    }

    #[test]
    fn mask2len_rejects_non_contiguous_masks() {
        let mut m = IN6MASK64;
        m.s6_addr[15] = 1;
        assert_eq!(in6_mask2len(&m, None), -1);
        let mut m = In6Addr::default();
        m.s6_addr[0] = 0xff;
        m.s6_addr[1] = 0x80;
        m.s6_addr[3] = 0x01;
        assert_eq!(in6_mask2len(&m, None), -1);
        // A hole inside a byte.
        m = In6Addr::default();
        m.s6_addr[0] = 0xff;
        m.s6_addr[1] = 0xc0;
        m.s6_addr[2] = 0x40;
        assert_eq!(in6_mask2len(&m, None), -1);
        m.s6_addr[1] = 0xa0;
        m.s6_addr[2] = 0;
        assert_eq!(in6_mask2len(&m, None), -1);
    }

    #[test]
    fn mask2len_limit_ignores_what_is_beyond_it() {
        // ff ff 80 00 | garbage: with a limit of 4 bytes only the first four count.
        let mut m = In6Addr::default();
        m.s6_addr[..4].copy_from_slice(&[0xff, 0xff, 0x80, 0x00]);
        m.s6_addr[8] = 0xff;
        assert_eq!(in6_mask2len(&m, Some(4)), 17);
        assert_eq!(in6_mask2len(&m, None), -1);
        // The stricter check inside the limit.
        m.s6_addr[3] = 0x01;
        assert_eq!(in6_mask2len(&m, Some(4)), -1);
        // A limit of zero bytes is an empty mask; more than 16 is the whole address.
        assert_eq!(in6_mask2len(&IN6MASK64, Some(0)), 0);
        assert_eq!(in6_mask2len(&IN6MASK64, Some(20)), 64);
        assert_eq!(in6_mask2len(&IN6MASK128, Some(17)), 128);
    }

    #[test]
    fn matchlen_counts_the_common_prefix() {
        let x = a6("2001:db8::1");
        assert_eq!(in6_matchlen(&x, &x), 128);
        assert_eq!(in6_matchlen(&x, &a6("2001:db8::3")), 126);
        assert_eq!(in6_matchlen(&x, &a6("2001:db8::0")), 127);
        assert_eq!(in6_matchlen(&x, &a6("2001:db9::1")), 31);
        assert_eq!(in6_matchlen(&x, &a6("a001:db8::1")), 0);
        assert_eq!(in6_matchlen(&x, &a6("2001:db8:8000::1")), 32);
        assert_eq!(in6_matchlen(&a6("fe80::1"), &a6("fe80::2")), 126);
        assert_eq!(in6_matchlen(&IN6ADDR_ANY, &IN6ADDR_LOOPBACK), 127);
    }

    #[test]
    fn address_scopes() {
        use crate::netinet6::in6::{
            __IPV6_ADDR_SCOPE_GLOBAL as G, __IPV6_ADDR_SCOPE_INTFACELOCAL as I,
            __IPV6_ADDR_SCOPE_LINKLOCAL as L, __IPV6_ADDR_SCOPE_SITELOCAL as S,
        };
        let scope = |s: &str| in6_addrscope(&a6(s));
        assert_eq!(scope("fe80::1"), i32::from(L));
        assert_eq!(scope("fec0::1"), i32::from(S));
        assert_eq!(scope("fe00::1"), i32::from(G)); // "just in case"
        assert_eq!(scope("ff01::1"), i32::from(I));
        assert_eq!(scope("ff02::1"), i32::from(L));
        assert_eq!(scope("ff05::1"), i32::from(S));
        assert_eq!(scope("ff0e::1"), i32::from(G));
        assert_eq!(scope("ff08::1"), i32::from(G)); // organization-local: global
        assert_eq!(scope("::1"), i32::from(I));
        assert_eq!(scope("::"), i32::from(L));
        assert_eq!(scope("::2"), i32::from(G));
        assert_eq!(scope("2001:db8::1"), i32::from(G));
        assert_eq!(scope("fd00:77::1"), i32::from(G));
    }

    #[test]
    fn scope_zone_ids() {
        assert_eq!(in6_addr2scopeid(3, &a6("fe80::1")), 3);
        assert_eq!(in6_addr2scopeid(3, &a6("ff02::1")), 3);
        assert_eq!(in6_addr2scopeid(3, &a6("::1")), 3);
        assert_eq!(in6_addr2scopeid(3, &a6("ff01::2")), 3);
        assert_eq!(in6_addr2scopeid(3, &a6("fec0::1")), 0);
        assert_eq!(in6_addr2scopeid(3, &a6("2001:db8::1")), 0);
    }

    #[test]
    fn embedded_scope_checks() {
        let mut sa = SockaddrIn6::with_addr(a6("fe80::1"));
        assert_eq!(in6_check_embed_scope(&mut sa, 7), Ok(()));
        assert_eq!(sa.sin6_addr.s6_addr16(1), htons(7));
        assert_eq!(in6_check_embed_scope(&mut sa, 7), Ok(()));
        assert_eq!(in6_check_embed_scope(&mut sa, 8), Err(Errno::EINVAL));
        // Not link-local: untouched.
        let mut g = SockaddrIn6::with_addr(a6("2001:db8::1"));
        assert_eq!(in6_check_embed_scope(&mut g, 7), Ok(()));
        assert_eq!(g.sin6_addr, a6("2001:db8::1"));

        let mut sa = SockaddrIn6::with_addr(a6("fe80::1"));
        sa.sin6_scope_id = 7;
        assert_eq!(in6_clear_scope_id(&mut sa, 8), Err(Errno::EINVAL));
        assert_eq!(in6_clear_scope_id(&mut sa, 7), Ok(()));
        assert_eq!(sa.sin6_scope_id, 0);
        let mut g = SockaddrIn6::with_addr(a6("2001:db8::1"));
        g.sin6_scope_id = 9;
        assert_eq!(in6_clear_scope_id(&mut g, 7), Ok(()));
        assert_eq!(g.sin6_scope_id, 9);
    }

    #[test]
    fn sockaddr_checks() {
        let _g = setup();
        let mut sin6 = SockaddrIn6::with_addr(a6("fd00::1"));
        sin6.sin6_len = size_of::<SockaddrIn6>() as u8;
        // SAFETY: a local `sockaddr_in6`.
        let ok = unsafe { in6_sa2sin6(sin6tosa(&mut sin6)) };
        assert_eq!(ok, Ok(&mut sin6 as *mut SockaddrIn6));
        sin6.sin6_len = 16;
        // SAFETY: as above.
        let r = unsafe { in6_sa2sin6(sin6tosa(&mut sin6)) };
        assert_eq!(r, Err(Errno::EINVAL));
        sin6.sin6_len = size_of::<SockaddrIn6>() as u8;
        sin6.sin6_family = crate::sys::socket::AF_INET;
        // SAFETY: as above.
        let r = unsafe { in6_sa2sin6(sin6tosa(&mut sin6)) };
        assert_eq!(r, Err(Errno::EAFNOSUPPORT));

        // From an mbuf: the length must be the mbuf's and a sockaddr_in6's.
        sin6.sin6_family = AF_INET6;
        // SAFETY: `SockaddrIn6` is plain data.
        let bytes = unsafe {
            core::slice::from_raw_parts(ptr::from_ref(&sin6).cast::<u8>(), size_of::<SockaddrIn6>())
        };
        let m = test_packet(bytes);
        assert!(in6_nam2sin6(m).is_ok());
        m.m_len().set(size_of::<SockaddrIn6>() as u32 - 1);
        assert_eq!(in6_nam2sin6(m).err(), Some(Errno::EINVAL));
        m.m_len().set(1);
        assert_eq!(in6_nam2sin6(m).err(), Some(Errno::EINVAL));
        crate::kern::uipc_mbuf::m_freem(m);
    }

    #[test]
    fn do_dad_needs_an_up_and_running_non_loopback_interface() {
        let ifp = test_ifnet(b"tdad0");
        assert!(!in6if_do_dad(ifp));
        ifp.if_flags.set(IFF_UP);
        assert!(!in6if_do_dad(ifp));
        ifp.if_flags.set(IFF_UP | IFF_RUNNING);
        assert!(in6if_do_dad(ifp));
        ifp.if_flags.set(IFF_UP | IFF_RUNNING | IFF_LOOPBACK);
        assert!(!in6if_do_dad(ifp));
    }

    #[test]
    fn address_lookup_on_an_interface() {
        let _g = setup();
        let ifp = test_if(b"tlk0");
        net_lock();
        assert!(in6ifa_ifpforlinklocal(ifp, 0).is_none());
        net_unlock();
        let ll = test_ia6(ifp, a6("fe80:1::5054:ff:febb:2"), IN6_IFF_TENTATIVE);
        let g = test_ia6(ifp, a6("fd00:77::1"), 0);
        assert!(ptr::eq(
            in6ifa_ifpwithaddr(ifp, &a6("fd00:77::1")).expect("on list"),
            g
        ));
        assert!(in6ifa_ifpwithaddr(ifp, &a6("fd00:77::2")).is_none());
        // The link-local address is found unless its flags are ignored.
        assert!(ptr::eq(
            in6ifa_ifpforlinklocal(ifp, 0).expect("link-local"),
            ll
        ));
        assert!(ptr::eq(
            in6ifa_ifpforlinklocal(ifp, IN6_IFF_ANYCAST).expect("link-local"),
            ll
        ));
        assert!(in6ifa_ifpforlinklocal(ifp, IN6_IFF_TENTATIVE).is_none());
    }

    #[test]
    fn source_address_follows_the_rfc_6724_rules() {
        let _g = setup();
        let if1 = test_if(b"tsa0");
        let if2 = test_if(b"tsa1");
        let ll1 = test_ia6(if1, a6("fe80:1::1"), 0);
        let ula1 = test_ia6(if1, a6("fd00::1"), 0);
        let doc2 = test_ia6(if2, a6("2001:db8::5"), 0);
        net_lock();
        let pick = |oifp: &Ifnet, dst: &str| in6_ifawithscope(oifp, &a6(dst), 0, None);

        // Global destination: the global address on the output interface (rule 5), not the
        // link-local one (rule 2) nor the other interface's.
        assert!(ptr::eq(pick(if1, "2001:db8::99").expect("src"), ula1));
        assert!(ptr::eq(pick(if2, "2001:db8::99").expect("src"), doc2));
        // Link-local destination: the zone is the output interface's, the smallest scope that is
        // big enough.
        assert!(ptr::eq(pick(if1, "fe80:1::2").expect("src"), ll1));
        // A routing domain without interfaces has no address.
        assert!(in6_ifawithscope(if1, &a6("2001:db8::99"), 5, None).is_none());
        net_unlock();
    }

    #[test]
    fn source_selection_skips_unusable_addresses_and_avoids_deprecated_ones() {
        let _g = setup();
        let ifp = test_if(b"tsb0");
        // All unusable: tentative, duplicated, anycast, detached.
        test_ia6(ifp, a6("2001:db8::10"), IN6_IFF_TENTATIVE);
        test_ia6(ifp, a6("2001:db8::11"), IN6_IFF_DUPLICATED);
        test_ia6(ifp, a6("2001:db8::12"), IN6_IFF_ANYCAST);
        test_ia6(ifp, a6("2001:db8::13"), IN6_IFF_DETACHED);
        net_lock();
        assert!(in6_ifawithscope(ifp, &a6("2001:db8::99"), 0, None).is_none());
        net_unlock();

        // Rule 3: a deprecated address loses to a preferred one, whatever the order.
        let dep = test_ia6(ifp, a6("2001:db8::99"), IN6_IFF_DEPRECATED);
        net_lock();
        assert!(ptr::eq(
            in6_ifawithscope(ifp, &a6("2001:db8::99"), 0, None).expect("src"),
            dep
        ));
        net_unlock();
        let ok = test_ia6(ifp, a6("2001:db8:1::1"), 0);
        net_lock();
        assert!(ptr::eq(
            in6_ifawithscope(ifp, &a6("2001:db8::99"), 0, None).expect("src"),
            ok
        ));
        net_unlock();
        let ok2 = test_ia6(ifp, a6("2001:db8::7"), 0);
        // Rule 8: of two preferred addresses, the longest matching prefix wins.
        net_lock();
        assert!(ptr::eq(
            in6_ifawithscope(ifp, &a6("2001:db8::99"), 0, None).expect("src"),
            ok2
        ));
        net_unlock();
        // Rule 7: a temporary address is preferred to a public one, even with a shorter match.
        let tmp = test_ia6(ifp, a6("2001:db8::98"), IN6_IFF_TEMPORARY);
        net_lock();
        let best = in6_ifawithscope(ifp, &a6("2001:db8::99"), 0, None).expect("src");
        net_unlock();
        assert!(ptr::eq(best, tmp));
    }

    #[test]
    fn selected_sources_are_counted() {
        use crate::netinet6::ip6_input::IP6COUNTERS;
        use crate::netinet6::ip6_var::Ip6statCounters;

        let _g = setup();
        let ifp = test_if(b"tsc0");
        test_ia6(ifp, a6("2001:db8::1"), 0);
        let none = Ip6statCounters::Ip6sSourcesNone as usize;
        let global =
            Ip6statCounters::Ip6sSourcesSameif as usize + usize::from(__IPV6_ADDR_SCOPE_GLOBAL);
        let before_none = IP6COUNTERS.read()[none];
        let before_same = IP6COUNTERS.read()[global];
        net_lock();
        assert!(in6_ifawithscope(ifp, &a6("2001:db8::2"), 0, None).is_some());
        assert!(in6_ifawithscope(ifp, &a6("2001:db8::2"), 9, None).is_none());
        net_unlock();
        assert_eq!(IP6COUNTERS.read()[global], before_same + 1);
        assert_eq!(IP6COUNTERS.read()[none], before_none + 1);
    }

    /// A driver `ioctl` that accepts the address and multicast requests (and brings the
    /// interface up on the first address, as drivers do).
    ///
    /// # Safety
    ///
    /// `IfIoctlFn`'s contract.
    unsafe fn accepting_ioctl(ifp: &'static Ifnet, cmd: u64, _data: *mut u8) -> Result<(), Errno> {
        use crate::sys::sockio::{SIOCADDMULTI, SIOCDELMULTI, SIOCSIFADDR};
        match cmd {
            SIOCSIFADDR => {
                ifp.if_flags.set(ifp.if_flags.get() | IFF_UP | IFF_RUNNING);
                Ok(())
            }
            SIOCADDMULTI | SIOCDELMULTI => Ok(()),
            _ => Err(Errno::ENOTTY),
        }
    }

    /// The test driver's `if_output`: the packets the address code sends (MLD reports, DAD
    /// solicitations) are dropped, as on a link nobody listens to.
    ///
    /// # Safety
    ///
    /// As for `if_output`: `dst` is a readable socket address.
    unsafe fn discard_output(
        _ifp: &'static Ifnet,
        m: &'static crate::sys::mbuf::Mbuf,
        _dst: *const crate::sys::socket::Sockaddr,
        _rt: Option<&'static crate::net::route::Rtentry>,
    ) -> Result<(), Errno> {
        crate::kern::uipc_mbuf::m_freem(m);
        Ok(())
    }

    /// An attached Ethernet-like interface `name` with hardware address `mac` and a driver that
    /// accepts what the IPv6 address code asks of it.
    fn test_driver_if(name: &[u8], mac: [u8; 6]) -> &'static Ifnet {
        let ifp = test_ifnet(name);
        ifp.if_ioctl().set(Some(accepting_ioctl));
        ifp.if_output().set(Some(discard_output));
        ifp.if_type.set(crate::net::if_types::IFT_ETHER);
        ifp.if_flags.set(IFF_MULTICAST);
        ifp.if_mtu.set(1500);
        let mut sdl = crate::net::if_dl::SockaddrDl {
            sdl_alen: 6,
            ..Default::default()
        };
        sdl.sdl_data[..6].copy_from_slice(&mac);
        crate::net::if_var::if_sadl_test_set(ifp, sdl);
        if_attach(ifp);
        ifp
    }

    /// `ifconfig <ifp> inet6 <addr> prefixlen <plen>`, from the kernel: `SIOCAIFADDR_IN6`.
    fn configure6(ifp: &'static Ifnet, a: In6Addr, plen: i32) -> Result<(), Errno> {
        let mut ifra = In6Aliasreq::zeroed();
        ifra.ifra_name = ifp.if_xname.get();
        *ifra.ifra_addr_mut() = SockaddrIn6::with_addr(a);
        ifra.ifra_prefixmask = SockaddrIn6::with_addr(IN6ADDR_ANY);
        in6_prefixlen2mask(&mut ifra.ifra_prefixmask.sin6_addr, plen);
        ifra.ifra_lifetime.ia6t_vltime = ND6_INFINITE_LIFETIME;
        ifra.ifra_lifetime.ia6t_pltime = ND6_INFINITE_LIFETIME;
        // SAFETY: an `in6_aliasreq`, from the kernel itself, aligned as a local.
        unsafe {
            in6_ioctl(
                SIOCAIFADDR_IN6,
                ptr::from_mut(&mut ifra).cast(),
                Some(ifp),
                true,
            )
        }
    }

    #[test]
    fn siocaifaddr_in6_makes_the_address_the_link_local_one_and_the_memberships() {
        use crate::netinet::ip_input::tests::{OURS, setup as setup_ip};

        let (_g, _t) = setup_ip();
        let ifp = test_driver_if(b"tcf0", OURS);
        let idx = ifp.if_index.get();

        configure6(ifp, a6("fd00:77::1"), 64).expect("SIOCAIFADDR_IN6");

        let ia = in6ifa_ifpwithaddr(ifp, &a6("fd00:77::1")).expect("the address");
        assert_eq!(in6_mask2len(&ia6_maskin6(ia), None), 64);
        assert_eq!(ia.ia6_flags.get() & IN6_IFF_DUPLICATED, 0);

        // The link-local address of the MAC's EUI-64 was made first (fe80::5054:ff:fe12:3456,
        // the zone embedded as the interface index).
        let ll = in6ifa_ifpforlinklocal(ifp, 0).expect("link-local address");
        let mut want = a6("fe80::5054:ff:fe12:3456");
        want.set_s6_addr16(1, htons(idx as u16));
        assert_eq!(ia6_in6(ll), want);
        assert_eq!(OURS[..2], [0x52, 0x54]);

        // The multicast groups joined: solicited-node of both, all-nodes link- and
        // interface-local.
        net_lock();
        let joined = |g: In6Addr| {
            crate::net::if_var::if_maddrlock_enter_read(ifp);
            let r = in6_lookupmulti(&g, ifp).is_some();
            crate::net::if_var::if_maddrlock_exit_read(ifp);
            r
        };
        let mut snm = a6("ff02::1:ff00:1");
        snm.set_s6_addr16(1, htons(idx as u16));
        assert!(joined(snm), "solicited-node group of fd00:77::1");
        let mut allnodes = IN6ADDR_LINKLOCAL_ALLNODES;
        allnodes.set_s6_addr16(1, htons(idx as u16));
        assert!(joined(allnodes), "ff02::1");
        net_unlock();

        // The address is deleted again with SIOCDIFADDR_IN6 and takes its group with it.
        let mut ifr = In6Ifreq::zeroed();
        ifr.ifr_name = ifp.if_xname.get();
        ifr.set_ifr_addr(SockaddrIn6::with_addr(a6("fd00:77::1")));
        // SAFETY: an `in6_ifreq`, from the kernel itself.
        unsafe {
            in6_ioctl(
                SIOCDIFADDR_IN6,
                ptr::from_mut(&mut ifr).cast(),
                Some(ifp),
                true,
            )
        }
        .expect("SIOCDIFADDR_IN6");
        assert!(in6ifa_ifpwithaddr(ifp, &a6("fd00:77::1")).is_none());
        assert!(in6ifa_ifpforlinklocal(ifp, 0).is_some());
    }

    #[test]
    fn siocaifaddr_in6_validates_the_request() {
        use crate::netinet::ip_input::tests::{setup as setup_ip, test_ether};

        let (_g, _t) = setup_ip();
        let ifp = test_ether();
        // Not a /0 and a non-contiguous mask.
        assert_eq!(configure6(ifp, a6("fd00:77::1"), 0), Err(Errno::EINVAL));
        // The read-only flags are refused.
        let mut ifra = In6Aliasreq::zeroed();
        *ifra.ifra_addr_mut() = SockaddrIn6::with_addr(a6("fd00:77::2"));
        ifra.ifra_prefixmask = SockaddrIn6::with_addr(IN6MASK64);
        ifra.ifra_flags = IN6_IFF_DUPLICATED;
        // SAFETY: as above.
        let r = unsafe {
            in6_ioctl(
                SIOCAIFADDR_IN6,
                ptr::from_mut(&mut ifra).cast(),
                Some(ifp),
                true,
            )
        };
        assert_eq!(r, Err(Errno::EINVAL));
        // Unprivileged, no interface, a request not for IPv6.
        // SAFETY: as above.
        let r = unsafe {
            in6_ioctl(
                SIOCAIFADDR_IN6,
                ptr::from_mut(&mut ifra).cast(),
                Some(ifp),
                false,
            )
        };
        assert_eq!(r, Err(Errno::EPERM));
        // SAFETY: as above.
        let r = unsafe { in6_ioctl(SIOCAIFADDR_IN6, ptr::from_mut(&mut ifra).cast(), None, true) };
        assert_eq!(r, Err(Errno::ENXIO));
        // SAFETY: as above.
        let r = unsafe {
            in6_ioctl(
                SIOCSIFADDR,
                ptr::from_mut(&mut ifra).cast(),
                Some(ifp),
                true,
            )
        };
        assert_eq!(r, Err(Errno::EINVAL));
    }
}
/* </TESTS> */
