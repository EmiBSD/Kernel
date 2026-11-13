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
//! The IPv6 addresses of the interfaces, `netinet6/in6.c`: the address `ioctl`s, the
//! connected, local and multicast routes an address brings, the multicast records and the
//! RFC 6724 source-address rules. The constants and structures of `<netinet6/in6.h>` are
//! `hal::sys::netinet6_in6`; the records are `kernel::netinet6::in6_var`'s.
//!
//! Upstream: sys/netinet6/in6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/in6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/in6.rs@3c62ede68802
//!
//! Byte order: as in the C kernel, an `In6Addr` holds the address bytes as they are on the
//! wire, and the 16- and 32-bit views are the native-endian readings of those bytes.
//!
//! ## Deviations
//! - `MROUTING` (`mrt6_ioctl`) and `NCARP` (`carp_iamatch`, the `IFT_CARP` rules of
//!   `in6_ifawithscope`) are not configured: comments at the sites (LZ's).
//! - `in6_mask2len`'s limit pointer is `Option<usize>`, the number of valid mask bytes (the
//!   C's `lim0 - mask`); `None` or more than 16 is the whole address. `in6_update_ifa` passes
//!   `sin6_len - 8` as the C's pointer arithmetic does (a shorter length makes the unsigned
//!   distance huge: the whole mask).
//! - `in6_update_ifa` takes `&In6Aliasreq`: the C's changes of `ifra->ifra_flags` (re-run DAD)
//!   stay in a local; `in6_ioctl_change_ifaddr`, which owns the request, sets
//!   `IN6_IFF_TENTATIVE` in it before the call as the C does.
//! - The `*errorp` out-parameters of `in6_addmulti` are the `Err`; the `malloc(M_NOWAIT)`
//!   failures are `ENOBUFS`.
//! - The two identical blocks of `in6_update_ifa` that add the route of an all-nodes group
//!   (`rtalloc`, the 32-bit key check, `rtrequest(RTM_ADD)`) are one private function,
//!   `in6_add_mcast_route`; the `goto`s of `in6_ifawithscope`'s rules are a labeled block
//!   returning a verdict (skip or replace).
//! - The driver's `SIOCSIFADDR` gets a zeroed `struct ifreq` image where the C passes the
//!   `struct in6_ifaddr` (`kernel::netinet::in_`'s D-14: no driver ported at the pin reads
//!   it).
//! - `SIOCGIFINFO_IN6` and `SIOCGNBRINFO_IN6` (`nd6_ioctl`) are answered by `bsd`'s
//!   `in6_control` until ND moves (N6.5b); [`in6_ioctl`] answers `EOPNOTSUPP` for them,
//!   reached at the pin only by the kernel's own tests.
//! - The records are never freed (`kernel::netinet6::in6_var`'s deviations); a membership
//!   whose interface has already left the index map when its last reference goes stays on
//!   that interface's list, where the C frees it there, and is not reused (IPv4's).
//! - The C's calls into ND (`nd6_dad_start`, `nd6_dad_stop`, `nd6_expire_timer_update`) and
//!   MLD's `ip6_output` go through `Inet` legs until ND and the output path move (N6.5b,
//!   N6.5c; docs/SCAFFOLDING.md).
//!
//! ## Redesign
//! - The address `ioctl`s over the request's bytes: LZ's `unsafe fn`s over `caddr_t data`
//!   (with an aligned copy of an unaligned request, 14 `unsafe`) -> [`in6_ioctl`] over
//!   `&mut [u8]`, the `struct in6_ifreq`'s and `struct in6_aliasreq`'s addresses read and
//!   written by copies at their offsets (`hal::pod::at`/`put`, the C's `*sa6` reads and
//!   `ifr->ifr_*` stores); `in6_sa2sin6` and `in6_nam2sin6` answer the checked
//!   `sockaddr_in6` by value (`kernel::netinet::in_`'s shape).
//! - `KERNEL_LOCK(); NET_LOCK();` (in6.c's order, the reverse of in.c's, both kept: D-17)
//!   with the net lock detached (`hal::netlock::lock`) for the requests that change an
//!   address, since the driver's `SIOCSIFADDR` may let it go and take it again (wg(4)'s
//!   `wg_up`); the steps that need its proof borrow it ([`NetHolding`]), never across the
//!   driver's request. Callers that already hold a proof (`if_up`) pass it.
//!   `NET_LOCK_SHARED()` of `in6_ioctl_get` is a witness (no driver call).
//! - The records are `kernel::netinet6::in6_var`'s (recycled, typed back-links: `ifatoia6`,
//!   `ifmatoin6m` are one load); `in6_joingroup`'s `malloc`'d entry is a slot of the
//!   address's [`In6Memberships`](crate::netinet6::in6_var::In6Memberships).
//! - `in6_ifawithscope` and the lookups read the records' own words (`In6Ifaddr::of` of each
//!   `AF_INET6` address, `Sin6Words::addr`), no copy of the generic part's address.
//!
//! **Lock and spl transitions** against the C:
//!
//! | operation | the C | the kernel | above the C |
//! |---|---|---|---|
//! | `in6_ioctl` add/delete (in6.c:288-289, :394-395) | `KERNEL_LOCK` then `NET_LOCK` exclusive | `klock::kernel_lock()` then `netlock::lock()` (D-17) | none; the reverse of `in_ioctl`'s (in.c:282-283): both orders are the C's own |
//! | `in6_ioctl_get` (in6.c:416) | `NET_LOCK_SHARED` | a read witness | none |
//! | `in6_addmulti`/`in6_delmulti` (in6.c:1037-1130) | `if_maddrlock` write (twice/once); `KERNEL_LOCK` around the driver's `SIOCADDMULTI`/`DELMULTI` | the `SplitRwlock`'s write; `klock` C-style around `if_ioctl` | none |
//! | `in6_hasmulti` | `if_maddrlock` read once | the same | none |
//! | `in6_update_ifa`'s joins, `in6_purgeaddr` | the net lock | the net lock, and `ia6_memberships`' mutex once per join and once per purge | one mutex pair per join (three per new address) and per purge: the ioctl path only |
//!
//! **Holders** against the C: `ifa_refcnt` (`refcnt_init` at a new address, the list's;
//! `ifafree` in `in6_unlink_ifa`; `rt_ifa`'s and DAD's taken by routing and ND); `in6m_refcnt`
//! (`refcnt_init`, `refcnt_take` per extra join, `refcnt_rele` in `in6_delmulti`): the same
//! counts, the same places; the records are recycled at zero (in6_var's deviations).

use core::mem::{offset_of, size_of};
use core::ptr;

use hal::netlock::{NETLOCK, NetHeld};
use hal::pod;
use hal::rwlock::{RW_READ, RW_WRITE};
use hal::sys::endian::{htonl, htons};
use hal::sys::errno::Errno;
use hal::sys::ioccom::iocparm_len;
use hal::sys::net_if::{
    IFF_LOOPBACK, IFF_MULTICAST, IFF_POINTOPOINT, IFF_RUNNING, IFF_UP, IFNAMSIZ, IFT_CARP,
};
use hal::sys::net_route::{
    RTAX_DST, RTAX_GATEWAY, RTAX_IFA, RTAX_NETMASK, RTF_CLONING, RTF_CONNECTED, RTF_HOST,
    RTF_MPATH, RTF_MULTICAST, RTM_ADD, RTM_CHGADDRATTR, RTP_CONNECTED,
};
use hal::sys::netinet6_in6::{
    __IPV6_ADDR_SCOPE_GLOBAL, __IPV6_ADDR_SCOPE_INTFACELOCAL, __IPV6_ADDR_SCOPE_LINKLOCAL,
    __IPV6_ADDR_SCOPE_SITELOCAL, IN6ADDR_INTFACELOCAL_ALLNODES, IN6ADDR_LINKLOCAL_ALLNODES,
    IN6ADDR_LOOPBACK, IN6MASK32, In6Addr, SockaddrIn6, in6_are_addr_equal, in6_is_addr_linklocal,
};
use hal::sys::netinet6_in6_var::{
    IN6_IFF_ANYCAST, IN6_IFF_AUTOCONF, IN6_IFF_DEPRECATED, IN6_IFF_DETACHED, IN6_IFF_DUPLICATED,
    IN6_IFF_TEMPORARY, IN6_IFF_TENTATIVE, In6Aliasreq, In6Ifreq, SIOCAIFADDR_IN6, SIOCDIFADDR_IN6,
    SIOCGIFAFLAG_IN6, SIOCGIFALIFETIME_IN6, SIOCGIFDSTADDR_IN6, SIOCGIFINFO_IN6,
    SIOCGIFNETMASK_IN6, SIOCGNBRINFO_IN6,
};
use hal::sys::netinet6_ip6_var::Ip6statCounters;
use hal::sys::netinet6_nd6::ND6_INFINITE_LIFETIME;
use hal::sys::socket::{AF_INET6, AF_UNSPEC};
use hal::sys::sockio::{
    SIOCADDMULTI, SIOCDELMULTI, SIOCSIFADDR, SIOCSIFBRDADDR, SIOCSIFDSTADDR, SIOCSIFNETMASK,
};
use hal::sys::types::Time;

use crate::assert::{kassert, kpanic};
use crate::net::if_::{IFNETLIST, IfRef, if_addrhooks_run, if_ioctl, ifa_add, ifa_del, inet};
use crate::net::if_var::{IFA_ROUTE, Ifaddr, Ifnet};
use crate::net::mbuf::Mbuf;
use crate::net::proto::SockaddrView;
use crate::net::route::{
    RtAddrinfo, Rtentry, ifafree, rt_ifa_add, rt_ifa_addlocal, rt_ifa_del, rt_ifa_dellocal,
    rt_ifa_purge, rtalloc, rtrequest,
};
use crate::net::rtsock::rtm_addr;
use crate::netinet6::in6_ifattach::in6_ifattach;
use crate::netinet6::in6_var::{
    In6Ifaddr, In6Multi, SIN6_LEN, ia6_dstin6, ia6_maskin6, in6_ifaddr_alloc, in6_multi_alloc,
    in6_multi_free,
};
use crate::netinet6::ip6_var::{ip6stat_inc, ip6stat_inc_idx};
use crate::netinet6::mld6::{mld6_sendpkt, mld6_start_listening, mld6_stop_listening};
use crate::netinet6::mld6_var::Mld6Pktinfo;
use crate::tc::{gettime, getuptime};

/// The size of a `struct sockaddr_in6`.
const SIN6_SIZE: usize = size_of::<SockaddrIn6>();
/// The size of a `struct ifreq` (`SIOCSIFADDR`'s image, `SIOC{ADD,DEL}MULTI`'s request is an
/// `in6_ifreq`).
const IFREQ_SIZE: usize = IFNAMSIZ + 16;
/// `offsetof(struct in6_ifreq, ifr_ifru)`: every member of the union.
const IFR_IFRU: usize = offset_of!(In6Ifreq, ifr_ifru);
/// `offsetof(struct in6_aliasreq, ifra_addr)`.
const IFRA_ADDR: usize = offset_of!(In6Aliasreq, ifra_addr);
/// `offsetof(struct sockaddr, sa_data)`.
const SA_DATA: usize = 2;

/// How the caller of a step that may call the driver holds the net lock: the C's
/// `NET_LOCK()` taken detached by the running process (`in6_ioctl`'s, kept across the
/// driver's `SIOCSIFADDR`, which may let it go: wg(4)'s `wg_up`), or a proof the caller
/// already has (`if_up`'s), under which every step runs.
pub enum NetHolding<'a> {
    /// `hal::netlock::lock()`'s holding: the steps that need the proof borrow it
    /// (`hal::netlock::lend_detached`), the driver's request runs outside any loan.
    Detached,
    /// The caller's proof.
    Held(&'a NetHeld<'a>),
}

impl NetHolding<'_> {
    /// `f` with the net lock's proof.
    #[inline]
    pub fn with<R>(&self, f: impl FnOnce(&NetHeld<'_>) -> R) -> R {
        match self {
            Self::Detached => hal::netlock::lend_detached(f),
            Self::Held(n) => f(n),
        }
    }
}

/// `NET_ASSERT_LOCKED()`.
#[inline]
fn net_assert_locked() {
    kassert!(matches!(hal::netlock::status(), RW_WRITE | RW_READ));
}

/// The `sockaddr_in6` image of `sin6`, for the routes' requests.
#[inline]
fn sin6_bytes(sin6: &SockaddrIn6) -> [u8; SIN6_SIZE] {
    let mut b = [0u8; SIN6_SIZE];
    let _ = pod::put(&mut b, 0, sin6);
    b
}

/// The view of a `sockaddr_in6` image (28 bytes: always a socket address).
#[inline]
fn sin6_view(b: &[u8; SIN6_SIZE]) -> SockaddrView<'_> {
    match SockaddrView::new(b) {
        Some(v) => v,
        None => kpanic(format_args!("in6: a sockaddr_in6 too short")),
    }
}

/// A `sockaddr_in6` of `addr`, `sin6_len` and family set.
#[inline]
fn sin6_of(addr: In6Addr) -> SockaddrIn6 {
    SockaddrIn6::with_addr(addr)
}

/// `IFA6_IS_DEPRECATED(a)`: the preferred lifetime of the address has run out.
#[inline]
pub fn ifa6_is_deprecated(a: &In6Ifaddr) -> bool {
    let pl = a.ia6_lifetime.pltime();
    pl != ND6_INFINITE_LIFETIME && (getuptime() - a.ia6_updatetime.get()) as u32 > pl
}

/// `IFA6_IS_INVALID(a)`: the valid lifetime of the address has run out.
#[inline]
pub fn ifa6_is_invalid(a: &In6Ifaddr) -> bool {
    let vl = a.ia6_lifetime.vltime();
    vl != ND6_INFINITE_LIFETIME && (getuptime() - a.ia6_updatetime.get()) as u32 > vl
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

/// `in6_nam2sin6(nam, &sin6)`: the `sockaddr_in6` in mbuf `nam`, checked (`EINVAL` for a name
/// shorter than its header or not exactly a `sockaddr_in6`, `EAFNOSUPPORT` for another
/// family), copied.
pub fn in6_nam2sin6(nam: &Mbuf) -> Result<SockaddrIn6, Errno> {
    let d = nam.data();
    if d.len() < SA_DATA {
        return Err(Errno::EINVAL);
    }
    if d[1] != AF_INET6 {
        return Err(Errno::EAFNOSUPPORT);
    }
    if usize::from(d[0]) != d.len() {
        return Err(Errno::EINVAL);
    }
    if usize::from(d[0]) != SIN6_SIZE {
        return Err(Errno::EINVAL);
    }
    pod::at::<SockaddrIn6>(d, 0).ok_or(Errno::EINVAL)
}

/// `in6_sa2sin6(sa, &sin6)`: the socket address at the start of `sa` as a `sockaddr_in6`,
/// checked (`EAFNOSUPPORT` for another family, `EINVAL` for another length), copied.
pub fn in6_sa2sin6(sa: &[u8]) -> Result<SockaddrIn6, Errno> {
    if sa.get(1) != Some(&AF_INET6) {
        return Err(Errno::EAFNOSUPPORT);
    }
    if sa.first().map(|&l| usize::from(l)) != Some(SIN6_SIZE) {
        return Err(Errno::EINVAL);
    }
    pod::at::<SockaddrIn6>(sa, 0).ok_or(Errno::EINVAL)
}

/// `in6_control(so, cmd, data, ifp)`: the `pru_control` of the IPv6 protocols, the address
/// `ioctl`s of `cmd` on `ifp` with `data` the kernel's copy of the request
/// (`IOCPARM_LEN(cmd)` bytes), privileged when the socket is (`SS_PRIV`). `MROUTING`
/// (`mrt6_ioctl`) is not configured.
pub fn in6_control(
    privileged: bool,
    cmd: u64,
    data: &mut [u8],
    ifp: Option<&'static Ifnet>,
) -> Result<(), Errno> {
    in6_ioctl(cmd, data, ifp, privileged)
}

/// `in6_ioctl(cmd, data, ifp, privileged)`: the IPv6 address `ioctl`s. The changes ask for
/// `privileged` before any lock is taken (the C's `EPERM`); `data` is the request,
/// `IOCPARM_LEN(cmd)` bytes (`EINVAL` when it is shorter).
pub fn in6_ioctl(
    cmd: u64,
    data: &mut [u8],
    ifp: Option<&'static Ifnet>,
    privileged: bool,
) -> Result<(), Errno> {
    let Some(ifp) = ifp else {
        return Err(Errno::ENXIO);
    };
    if data.len() < iocparm_len(cmd) as usize {
        return Err(Errno::EINVAL);
    }

    match cmd {
        // nd6_ioctl: answered by bsd's in6_control until N6.5b (the module's deviations).
        SIOCGIFINFO_IN6 | SIOCGNBRINFO_IN6 => Err(Errno::EOPNOTSUPP),
        SIOCGIFDSTADDR_IN6 | SIOCGIFNETMASK_IN6 | SIOCGIFAFLAG_IN6 | SIOCGIFALIFETIME_IN6 => {
            in6_ioctl_get(cmd, data, ifp)
        }
        SIOCAIFADDR_IN6 | SIOCDIFADDR_IN6 => {
            if !privileged {
                return Err(Errno::EPERM);
            }
            in6_ioctl_change_ifaddr(cmd, data, ifp)
        }
        // Do not pass those ioctl to driver handler since they are not properly set up.
        // Instead just error out.
        SIOCSIFADDR | SIOCSIFDSTADDR | SIOCSIFBRDADDR | SIOCSIFNETMASK => Err(Errno::EINVAL),
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// `in6_ioctl_change_ifaddr(cmd, data, ifp)`: `SIOCAIFADDR_IN6` (add or change an address,
/// `data` a `struct in6_aliasreq`) and `SIOCDIFADDR_IN6` (delete it, `data` a `struct
/// in6_ifreq`).
fn in6_ioctl_change_ifaddr(cmd: u64, data: &mut [u8], ifp: &'static Ifnet) -> Result<(), Errno> {
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
    let off = match cmd {
        SIOCAIFADDR_IN6 => IFRA_ADDR,
        SIOCDIFADDR_IN6 => IFR_IFRU,
        _ => kpanic(format_args!("in6_ioctl_change_ifaddr: invalid ioctl {cmd}")),
    };
    let mut sa6 = None;
    if data.get(off + 1) == Some(&AF_INET6) {
        sa6 = Some(in6_sa2sin6(&data[off..off + SIN6_SIZE])?);
    }

    hal::klock::kernel_lock();
    hal::netlock::lock();

    let error = 'err: {
        let mut ia6 = None;
        if let Some(mut s) = sa6 {
            let mut r = in6_check_embed_scope(&mut s, ifp.if_index.get());
            if r.is_ok() {
                r = in6_clear_scope_id(&mut s, ifp.if_index.get());
            }
            // The request's address, scoped, written back as the C changes it in place.
            let _ = pod::put(data, off, &s);
            if let Err(e) = r {
                break 'err Err(e);
            }
            sa6 = Some(s);
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
                hal::netlock::lend_detached(|net| in6_purgeaddr(&ia6.ia_ifa, net));
                if_addrhooks_run(ifp);
                Ok(())
            }

            // SIOCAIFADDR_IN6
            _ => {
                let Some(mut ifra) = pod::at::<In6Aliasreq>(data, 0) else {
                    break 'err Err(Errno::EINVAL);
                };
                if ifra.ifra_addr.sin6_family != AF_INET6
                    || usize::from(ifra.ifra_addr.sin6_len) != SIN6_SIZE
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
                    let _ = pod::put(data, 0, &ifra);
                }

                // first, make or update the interface address structure, and link it to the
                // list. try to enable inet6 if there is no link-local yet.
                if let Err(e) = in6_ifattach(ifp, &NetHolding::Detached) {
                    break 'err Err(e);
                }
                if let Err(e) = in6_update_ifa(ifp, &ifra, ia6, &NetHolding::Detached) {
                    break 'err Err(e);
                }

                let ia6 = sa6.and_then(|s| in6ifa_ifpwithaddr(ifp, &s.sin6_addr));
                let Some(ia6) = ia6 else {
                    // this can happen when the user specify the 0 valid lifetime.
                    break 'err Ok(());
                };

                // Perform DAD, if needed.
                if ia6.ia6_flags.get() & IN6_IFF_TENTATIVE != 0
                    && let Some(i) = inet()
                {
                    i.nd6_dad_start(&ia6.ia_ifa);
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

                // ifa->ifa_addr: the address's own socket address, copied.
                let a = sin6_bytes(&ia6.ia_addr.get());
                let error = hal::netlock::lend_detached(|net| {
                    let e = rt_ifa_add(
                        &ia6.ia_ifa,
                        RTF_CLONING | RTF_CONNECTED | RTF_MPATH,
                        sin6_view(&a),
                        ifp.if_rdomain.get(),
                        net,
                    );
                    if e.is_err() {
                        in6_purgeaddr(&ia6.ia_ifa, net);
                    }
                    e
                });
                if let Err(e) = error {
                    break 'err Err(e);
                }
                if_addrhooks_run(ifp);
                Ok(())
            }
        }
    };

    // err:
    hal::netlock::unlock();
    hal::klock::kernel_unlock();
    error
}

/// `in6_ioctl_get(cmd, data, ifp)`: the address `ioctl`s that read: `SIOCGIFDSTADDR_IN6`,
/// `SIOCGIFNETMASK_IN6`, `SIOCGIFAFLAG_IN6`, `SIOCGIFALIFETIME_IN6` (`data` a `struct
/// in6_ifreq`).
fn in6_ioctl_get(cmd: u64, data: &mut [u8], ifp: &'static Ifnet) -> Result<(), Errno> {
    let Some(mut ifr) = pod::at::<In6Ifreq>(data, 0) else {
        return Err(Errno::EINVAL);
    };
    let mut sa6 = None;
    let mut addr = ifr.ifr_addr();
    if addr.sin6_family == AF_INET6 {
        addr.sin6_len = SIN6_LEN;
        ifr.set_ifr_addr(addr);
        let _ = pod::put(data, 0, &ifr);
        sa6 = Some(in6_sa2sin6(&sin6_bytes(&addr))?);
    }

    let w = NETLOCK.read();
    let error = 'err: {
        let mut ia6 = None;
        if let Some(mut s) = sa6 {
            let mut r = in6_check_embed_scope(&mut s, ifp.if_index.get());
            if r.is_ok() {
                r = in6_clear_scope_id(&mut s, ifp.if_index.get());
            }
            ifr.set_ifr_addr(s);
            if let Err(e) = r {
                break 'err Err(e);
            }
            ia6 = in6ifa_ifpwithaddr(ifp, &s.sin6_addr);
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
                    retlt.ia6t_expire = lifetime_expire(updatetime, lt.ia6t_vltime, maxexpire);
                }
                if lt.ia6t_pltime != ND6_INFINITE_LIFETIME {
                    retlt.ia6t_preferred = lifetime_expire(updatetime, lt.ia6t_pltime, maxexpire);
                }
                ifr.set_ifr_lifetime(retlt);
                Ok(())
            }

            _ => kpanic(format_args!("in6_ioctl_get: invalid ioctl {cmd}")),
        }
    };
    // err:
    drop(w);
    if !pod::put(data, 0, &ifr) {
        return Err(Errno::EINVAL);
    }
    error
}

/// `SIOCGIFALIFETIME_IN6`'s expiration of a lifetime `lifetime` seconds long from
/// `updatetime` (uptime): in wall-clock seconds, or `maxexpire` when it would overflow.
fn lifetime_expire(updatetime: Time, lifetime: u32, maxexpire: Time) -> Time {
    if Time::from(lifetime) < maxexpire - updatetime {
        let mut expire = updatetime + Time::from(lifetime);
        if expire != 0 {
            expire -= getuptime();
            expire += gettime();
        }
        expire
    } else {
        maxexpire
    }
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
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let d = sin6_bytes(mltaddr);
    let mut rt = rtalloc(sin6_view(&d), 0, ifp.if_rdomain.get(), net);
    if let Some(r) = &rt {
        // 32bit came from "mltmask": the key's `sin6_addr`, a `sockaddr_in6` here.
        let at = offset_of!(SockaddrIn6, sin6_addr);
        if r.dest().get(at..at + 4) != Some(&mltaddr.sin6_addr.s6_addr[..4]) {
            rt = None; // rtfree(rt): the handle's drop
        }
    }
    if rt.is_some() {
        return Ok(()); // rtfree(rt)
    }
    let a = sin6_bytes(&ia6.ia_addr.get());
    let m = sin6_bytes(mltmask);
    let mut info = RtAddrinfo::new();
    info.rti_ifa = Some(&ia6.ia_ifa);
    info.rti_info[RTAX_DST] = Some(sin6_view(&d));
    info.rti_info[RTAX_GATEWAY] = Some(sin6_view(&a));
    info.rti_info[RTAX_NETMASK] = Some(sin6_view(&m));
    info.rti_info[RTAX_IFA] = Some(sin6_view(&a));
    info.rti_flags = RTF_MULTICAST;
    rtrequest(
        RTM_ADD,
        &mut info,
        RTP_CONNECTED,
        None,
        ifp.if_rdomain.get(),
        net,
    )
}

/// `in6_update_ifa`: update parameters of an IPv6 interface address; adds or changes the
/// address `ifra` describes on `ifp`. `ia6` is the existing address or `None` to allocate
/// one (and link it into the address chains). This function is separated from
/// `in6_control()`. The C's changes of `ifra->ifra_flags` stay in a local here. Under the net
/// lock, held as `net` says.
pub fn in6_update_ifa(
    ifp: &'static Ifnet,
    ifra: &In6Aliasreq,
    ia6: Option<&'static In6Ifaddr>,
    net: &NetHolding<'_>,
) -> Result<(), Errno> {
    let mut host_is_new = false;
    let mut ifra_flags = ifra.ifra_flags;

    net_assert_locked();

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
    if usize::from(ifra.ifra_prefixmask.sin6_len) > SIN6_SIZE {
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
        dst6 = SockaddrIn6::default();
    } else {
        dst6 = ifra.ifra_dstaddr;
        gw6 = SockaddrIn6::default();
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
            let ia6 = in6_ifaddr_alloc(ifp, getuptime());
            ia6.ia_addr.set(ifra.ifra_addr);
            ifa_add(ifp, &ia6.ia_ifa);
            ia6
        }
    };

    // The step that failed before the address is complete: a new address is unlinked again.
    let unlink = |error: Errno| -> Result<(), Errno> {
        // XXX: if a change of an existing address failed, keep the entry anyway.
        if host_is_new {
            net.with(|n| in6_unlink_ifa(ia6, ifp, n));
        }
        Err(error)
    };

    // set prefix mask
    if ifra.ifra_prefixmask.sin6_len != 0 {
        // We prohibit changing the prefix length of an existing address, because
        // + such an operation should be rare in IPv6, and
        // + the operation would confuse prefix management.
        if !ia6.ia_prefixmask.is_empty() && in6_mask2len(&ia6_maskin6(ia6), None) != plen {
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
        if ia6.ia_flags().get() & IFA_ROUTE != 0 {
            // ifa->ifa_dstaddr: the address's own destination address, copied.
            let d = sin6_bytes(&ia6.ia_dstaddr.get());
            let r = net.with(|n| {
                rt_ifa_del(
                    &ia6.ia_ifa,
                    RTF_HOST,
                    sin6_view(&d),
                    ifp.if_rdomain.get(),
                    n,
                )
            });
            if r.is_ok() {
                ia6.ia_flags().set(ia6.ia_flags().get() & !IFA_ROUTE);
            }
        }
        ia6.ia_dstaddr.set(dst6);
    }

    if ifra_flags & IN6_IFF_AUTOCONF != 0
        && gw6.sin6_family == AF_INET6
        && !in6_are_addr_equal(&dst6.sin6_addr, &ia6.ia_gwaddr.addr())
    {
        // Set or update announcing router
        ia6.ia_gwaddr.set(gw6);
    }

    // Set lifetimes. We do not refer to ia6t_expire and ia6t_preferred to see if the address
    // is deprecated or invalidated, but initialize these members for applications.
    ia6.ia6_updatetime.set(getuptime());
    let mut lt = ifra.ifra_lifetime;
    lt.ia6t_expire = if lt.ia6t_vltime != ND6_INFINITE_LIFETIME {
        getuptime() + Time::from(lt.ia6t_vltime)
    } else {
        0
    };
    lt.ia6t_preferred = if lt.ia6t_pltime != ND6_INFINITE_LIFETIME {
        getuptime() + Time::from(lt.ia6t_pltime)
    } else {
        0
    };
    ia6.ia6_lifetime.set(lt);

    // reset the interface and routing table appropriately.
    if let Err(e) = in6_ifinit(ifp, ia6, host_is_new, net) {
        return unlink(e);
    }

    // re-run DAD
    if ia6.ia6_flags.get() & (IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED) != 0 {
        ifra_flags |= IN6_IFF_TENTATIVE;
    }
    // configure address flags.
    ia6.ia6_flags.set(ifra_flags);

    if let Some(i) = inet() {
        i.nd6_expire_timer_update(ia6);
    }

    // We are done if we have simply modified an existing address.
    if !host_is_new {
        // DAD sends RTM_CHGADDRATTR when done.
        if ia6.ia6_flags.get() & IN6_IFF_TENTATIVE == 0 {
            net.with(|n| rtm_addr(RTM_CHGADDRATTR, &ia6.ia_ifa, n));
        }
        return Ok(());
    }

    // Beyond this point, we should call in6_purgeaddr upon an error, not just go to unlink.

    // join necessary multiast groups
    if ifp.if_flags.get() & IFF_MULTICAST != 0 {
        let joined = net.with(|n| in6_join_groups(ifp, ia6, ifra, n));
        if let Err(e) = joined {
            net.with(|n| in6_purgeaddr(&ia6.ia_ifa, n));
            return Err(e);
        }
    }

    Ok(())
}

/// `in6_update_ifa`'s joins of a new address `ia6` on multicast interface `ifp`: the
/// solicited-node group of its host id, the link-local all-nodes group and the
/// interface-local all-nodes group, the two all-nodes groups with their /32 routes; every
/// membership on the address's list.
fn in6_join_groups(
    ifp: &'static Ifnet,
    ia6: &'static In6Ifaddr,
    ifra: &In6Aliasreq,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let ifidx = htons(ifp.if_index.get() as u16);

    // join solicited multicast addr for new host id
    let mut llsol = In6Addr::default();
    llsol.set_s6_addr16(0, htons(0xff02));
    llsol.set_s6_addr16(1, ifidx);
    llsol.set_s6_addr32(1, 0);
    llsol.set_s6_addr32(2, htonl(1));
    llsol.set_s6_addr32(3, ifra.ifra_addr.sin6_addr.s6_addr32(3));
    llsol.s6_addr[12] = 0xff;
    in6_joingroup(ifp, ia6, &llsol, net)?;

    let mltmask = sin6_of(IN6MASK32);

    // join link-local all-nodes address
    let mut mltaddr = sin6_of(IN6ADDR_LINKLOCAL_ALLNODES);
    mltaddr.sin6_addr.set_s6_addr16(1, ifidx);
    mltaddr.sin6_scope_id = 0;

    // XXX: do we really need this automatic routes? We should probably reconsider this
    // stuff. Most applications actually do not need the routes, since they usually specify
    // the outgoing interface.
    in6_add_mcast_route(ifp, ia6, &mltaddr, &mltmask, net)?;
    in6_joingroup(ifp, ia6, &mltaddr.sin6_addr, net)?;

    // join interface-local all-nodes address. (ff01::1%ifN, and ff01::%ifN/32)
    let mut mltaddr = sin6_of(IN6ADDR_INTFACELOCAL_ALLNODES);
    mltaddr.sin6_addr.set_s6_addr16(1, ifidx);
    mltaddr.sin6_scope_id = 0;

    // XXX: again, do we really need the route?
    in6_add_mcast_route(ifp, ia6, &mltaddr, &mltmask, net)?;
    in6_joingroup(ifp, ia6, &mltaddr.sin6_addr, net)?;
    Ok(())
}

/// `in6_joingroup(ifp, addr, &error)` and `LIST_INSERT_HEAD(&ia6->ia6_memberships, imm,
/// i6mm_chain)`: `ifp` joins `addr` for address `ia6`, which records the membership; the
/// group left again when the record cannot (the C's failed `malloc` of the entry, before
/// the join).
fn in6_joingroup(
    ifp: &'static Ifnet,
    ia6: &In6Ifaddr,
    addr: &In6Addr,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let in6m = in6_addmulti(addr, ifp, net)?;
    let pushed = ia6.ia6_memberships.lock().push(in6m);
    if pushed.is_err() {
        in6_delmulti(in6m, net);
    }
    pushed
}

/// `in6_purgeaddr(ifa)`: an IPv6 address removed from its interface and the routing table,
/// its memberships left, its interface's reference given back.
pub fn in6_purgeaddr(ifa: &'static Ifaddr, net: &NetHeld<'_>) {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!("in6_purgeaddr: address without interface"));
    };
    let ia6 = crate::netinet6::in6_var::ifatoia6(ifa);

    // stop DAD processing
    if let Some(i) = inet() {
        i.nd6_dad_stop(ifa);
    }

    // delete route to the destination of the address being purged. The interface must be p2p
    // or loopback in this case.
    if ifp.if_flags.get() & IFF_POINTOPOINT != 0
        && ia6.ia_flags().get() & IFA_ROUTE != 0
        && !ia6.ia_dstaddr.is_empty()
    {
        // ifa->ifa_dstaddr: the address's own destination address, copied.
        let d = sin6_bytes(&ia6.ia_dstaddr.get());
        let r = rt_ifa_del(ifa, RTF_HOST, sin6_view(&d), ifp.if_rdomain.get(), net);
        if r.is_ok() {
            ia6.ia_flags().set(ia6.ia_flags().get() & !IFA_ROUTE);
        }
    }

    // Remove ownaddr's loopback rtentry, if it exists.
    let _ = rt_ifa_dellocal(&ia6.ia_ifa, net);

    // leave from multicast groups we have joined for the interface: the list's head first,
    // the newest (`LIST_INSERT_HEAD`).
    let mut joined = core::mem::take(&mut *ia6.ia6_memberships.lock());
    while let Some(in6m) = joined.pop_last() {
        in6_delmulti(in6m, net);
    }

    in6_unlink_ifa(ia6, ifp, net);
}

/// `in6_unlink_ifa`: takes an address off its interface and drops the list's reference.
fn in6_unlink_ifa(ia6: &'static In6Ifaddr, ifp: &'static Ifnet, net: &NetHeld<'_>) {
    let ifa = &ia6.ia_ifa;

    net_assert_locked();

    // Release the reference to the base prefix.
    let plen = in6_mask2len(&ia6_maskin6(ia6), None);
    if ifp.if_flags.get() & IFF_LOOPBACK == 0 && plen != 128 {
        // ifa->ifa_addr: the address's own socket address, copied.
        let a = sin6_bytes(&ia6.ia_addr.get());
        let _ = rt_ifa_del(
            ifa,
            RTF_CLONING | RTF_CONNECTED,
            sin6_view(&a),
            ifp.if_rdomain.get(),
            net,
        );
    }

    rt_ifa_purge(ifa, net);
    ifa_del(ifp, ifa);

    ia6.ia_ifp().set(None);
    ifafree(ifa);
}

/// `in6_ifinit`: initialize an interface's inet6 address and routing table entry. The
/// driver's request runs under the holding `net` names (outside any loan of a detached one).
fn in6_ifinit(
    ifp: &'static Ifnet,
    ia6: &'static In6Ifaddr,
    newhost: bool,
    net: &NetHolding<'_>,
) -> Result<(), Errno> {
    net_assert_locked();

    // Give the interface a chance to initialize if this is its first address (or it is a CARP
    // interface) and to validate the address if necessary.
    let ifacount = ifp
        .if_addrlist
        .iter()
        .filter(|ifa| In6Ifaddr::of(ifa).is_some())
        .count();

    if ifacount <= 1
        || ifp.if_type.get() == IFT_CARP
        || ifp.if_flags.get() & (IFF_LOOPBACK | IFF_POINTOPOINT) != 0
    {
        let mut req = [0u8; IFREQ_SIZE];
        if_ioctl(ifp, SIOCSIFADDR, &mut req)?;
    }

    ia6.ia_ifa.ifa_metric.set(ifp.if_metric.get() as i32);

    // we could do in(6)_socktrim here, but just omit it at this moment.

    // Special case: If the destination address is specified for a point-to-point interface,
    // install a route to the destination as an interface direct route.
    let plen = in6_mask2len(&ia6_maskin6(ia6), None); // XXX
    if ifp.if_flags.get() & IFF_POINTOPOINT != 0
        && plen == 128
        && ia6.ia_dstaddr.family() == AF_INET6
    {
        // ifa->ifa_dstaddr: the address's own destination address, copied.
        let d = sin6_bytes(&ia6.ia_dstaddr.get());
        net.with(|n| {
            rt_ifa_add(
                &ia6.ia_ifa,
                RTF_HOST | RTF_MPATH,
                sin6_view(&d),
                ifp.if_rdomain.get(),
                n,
            )
        })?;
        ia6.ia_flags().set(ia6.ia_flags().get() | IFA_ROUTE);
    }

    if newhost {
        return net.with(|n| rt_ifa_addlocal(&ia6.ia_ifa, n));
    }

    Ok(())
}

/// `in6_lookupmulti(addr, ifp)`: the `in6_multi` record of `addr` on `ifp`, if joined.
/// Called holding `if_maddrlock`.
pub fn in6_lookupmulti(addr: &In6Addr, ifp: &Ifnet) -> Option<&'static In6Multi> {
    kassert!(matches!(ifp.rt().if_maddrlock.status(), RW_WRITE | RW_READ));

    ifp.if_maddrlist
        .iter()
        .filter_map(In6Multi::of)
        .find(|in6m| in6_are_addr_equal(&in6m.in6m_addr(), addr))
}

/// The image of the `struct in6_ifreq` `in6_addmulti` and `in6_delmulti` hand the driver:
/// its `ifr_addr` the `sockaddr_in6` of `addr`.
fn multi_ifreq(addr: In6Addr) -> In6Ifreq {
    let mut ifr = In6Ifreq::zeroed();
    ifr.set_ifr_addr(sin6_of(addr));
    ifr
}

/// `in6_addmulti(addr, ifp, &error)`: `addr` added to the IPv6 multicast addresses of `ifp`
/// (or a reference taken on its record if it is there); the C's `*errorp` is the `Err`
/// (`ENOBUFS` without memory, the driver's refusal). The first MLD report of a new group is
/// sent under the net lock the caller holds (`net`).
pub fn in6_addmulti(
    addr: &In6Addr,
    ifp: &'static Ifnet,
    net: &NetHeld<'_>,
) -> Result<&'static In6Multi, Errno> {
    let lock = &ifp.rt().if_maddrlock;

    // See if address already in list.
    let _ = lock.enter(RW_WRITE);
    if let Some(in6m) = in6_lookupmulti(addr, ifp) {
        in6m.in6m_refcnt().take();
        lock.exit();
        return Ok(in6m);
    }
    lock.exit();

    // New address; allocate a new multicast record and link it into the interface's
    // multicast list.
    let Some(new_in6m) = in6_multi_alloc() else {
        return Err(Errno::ENOBUFS);
    };

    // Ask the network driver to update its multicast reception filter appropriately for the
    // new address.
    let mut ifr = multi_ifreq(*addr);
    hal::klock::kernel_lock();
    let error = if_ioctl(ifp, SIOCADDMULTI, ifr_bytes(&mut ifr));
    hal::klock::kernel_unlock();
    if let Err(e) = error {
        in6_multi_free(new_in6m);
        return Err(e);
    }

    let _ = lock.enter(RW_WRITE);
    // check again after unlock and lock
    if let Some(in6m) = in6_lookupmulti(addr, ifp) {
        in6m.in6m_refcnt().take();
        lock.exit();
        in6_multi_free(new_in6m);
        return Ok(in6m);
    }
    let in6m = new_in6m;
    in6m.in6m_sin.set(sin6_of(*addr));
    in6m.in6m_refcnt().init();
    in6m.in6m_ifidx().set(ifp.if_index.get());

    // Let MLD6 know that we have joined a new IP6 multicast group.
    ifp.if_maddrlist.insert_head(&in6m.in6m_ifma);
    let mut pkt = Mld6Pktinfo::default(); // pkt.mpi_ifidx = 0
    mld6_start_listening(in6m, ifp, &mut pkt);
    lock.exit();

    if pkt.mpi_ifidx != 0 {
        mld6_sendpkt(&pkt, net);
    }

    Ok(in6m)
}

/// The bytes of a `struct in6_ifreq` image, for the driver's request.
#[inline]
fn ifr_bytes(ifr: &mut In6Ifreq) -> &mut [u8] {
    hal::user::bytes_of_mut(core::slice::from_mut(ifr))
}

/// `in6_delmulti(in6m)`: a reference to the multicast record given back; with the last, MLD
/// leaves the group, the record leaves its interface and the driver its filter. Under the
/// net lock the caller holds (`net`, for the done message).
pub fn in6_delmulti(in6m: &'static In6Multi, net: &NetHeld<'_>) {
    if !in6m.in6m_refcnt().rele() {
        return;
    }

    if let Some(ifp) = IfRef::get(in6m.in6m_ifidx().get()) {
        let lock = &ifp.rt().if_maddrlock;
        let _ = lock.enter(RW_WRITE);
        // No remaining claims to this record; let MLD6 know that we are leaving the multicast
        // group.
        let mut pkt = Mld6Pktinfo::default(); // pkt.mpi_ifidx = 0
        mld6_stop_listening(in6m, ifp.ifp(), &mut pkt);
        ifp.if_maddrlist.remove(&in6m.in6m_ifma);
        lock.exit();

        if pkt.mpi_ifidx != 0 {
            mld6_sendpkt(&pkt, net);
        }

        // Notify the network driver to update its multicast reception filter.
        let mut ifr = multi_ifreq(in6m.in6m_addr());
        hal::klock::kernel_lock();
        let _ = if_ioctl(ifp.ifp(), SIOCDELMULTI, ifr_bytes(&mut ifr));
        hal::klock::kernel_unlock();
        // if_put(ifp): the reference's drop.
    }

    // free(in6m): a record still on a gone interface's list is not reused (the module's
    // deviations).
    if !hal::list::List::<crate::net::if_var::IfMaddrlist>::is_linked(&in6m.in6m_ifma) {
        in6_multi_free(in6m);
    }
}

/// `in6_hasmulti(addr, ifp)`: whether the multicast group `addr` has been joined by
/// interface `ifp`.
pub fn in6_hasmulti(addr: &In6Addr, ifp: &Ifnet) -> bool {
    let lock = &ifp.rt().if_maddrlock;
    let _ = lock.enter(RW_READ);
    let joined = in6_lookupmulti(addr, ifp).is_some();
    lock.exit();

    joined
}

/// `in6ifa_ifpforlinklocal(ifp, ignoreflags)`: an IPv6 link-local address of `ifp` without
/// any of the `IN6_IFF_*` flags `ignoreflags`.
pub fn in6ifa_ifpforlinklocal(ifp: &Ifnet, ignoreflags: i32) -> Option<&'static In6Ifaddr> {
    // The first link-local address without an ignored flag (the C `continue`s past one).
    ifp.if_addrlist
        .iter()
        .filter_map(In6Ifaddr::of)
        .find(|ia6| {
            in6_is_addr_linklocal(&ia6.ia_addr.addr()) && ia6.ia6_flags.get() & ignoreflags == 0
        })
}

/// `in6ifa_ifpwithaddr(ifp, addr)`: the IPv6 address of `ifp` whose address is `addr`.
pub fn in6ifa_ifpwithaddr(ifp: &Ifnet, addr: &In6Addr) -> Option<&'static In6Ifaddr> {
    ifp.if_addrlist
        .iter()
        .filter_map(In6Ifaddr::of)
        .find(|ia6| in6_are_addr_equal(addr, &ia6.ia_addr.addr()))
}

/// `in6_addrscope(addr)`: the scope of the address: node-local, link-local, site-local or
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

/// `in6_addr2scopeid(ifidx, addr)`: the scope zone id of `addr` on interface `ifidx` (the
/// interface index for link- and interface-local scopes, 0 otherwise).
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

/// `in6_matchlen(src, dst)`: the length of the common prefix of `src` and `dst`, in bits
/// (the part where dst and src are equal).
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

/// `in6_prefixlen2mask(maskp, len)`: the mask of prefix length `len` (a length outside
/// 0..=128 is logged and leaves `maskp` alone).
pub fn in6_prefixlen2mask(maskp: &mut In6Addr, len: i32) {
    const MASKARRAY: [u8; 8] = [0x80, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc, 0xfe, 0xff];

    // sanity check
    if !(0..=128).contains(&len) {
        crate::prf::log(
            crate::log::LOG_ERR,
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

/// `in6_ifawithscope(oifp, dst, rdomain, rt)`: the best address out of the same scope (the
/// RFC 6724 source address selection rules as the C applies them) for `dst` on `oifp`,
/// considering the route `rt` and routing domain `rdomain`. Under the net lock.
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
        gw6 = pod::at::<SockaddrIn6>(gw.bytes(), 0).map(|s| s.sin6_addr);
    }

    net_assert_locked();

    // We search for all addresses on all interfaces from the beginning.
    for ifp in &IFNETLIST {
        if ifp.if_rdomain.get() != rdomain {
            continue;
        }
        // NCARP: never use a carp address of an interface which is not the master
        // (`carp_iamatch`); carp is not configured.

        // We can never take an address that breaks the scope zone of the destination.
        if in6_addr2scopeid(ifp.if_index.get(), dst) != in6_addr2scopeid(oifp.if_index.get(), dst) {
            continue;
        }

        for ia in ifp.if_addrlist.iter().filter_map(In6Ifaddr::of) {
            let mut tlen = -1;

            let ifa_addr6 = ia.ia_addr.addr();
            let src_scope = in6_addrscope(&ifa_addr6);
            let flags = ia.ia6_flags.get();

            // Don't use an address before completing DAD nor a duplicated address.
            if flags & (IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED) != 0 {
                continue;
            }

            // RFC 6724 allows anycast addresses as source address because the restriction
            // was removed in RFC 4291. However RFC 4443 states that ICMPv6 responses MUST
            // use a unicast source address.
            //
            // XXX Skip anycast addresses for now since icmp6_reflect() uses this function
            // for source address selection.
            if flags & IN6_IFF_ANYCAST != 0 {
                continue;
            }

            if flags & IN6_IFF_DETACHED != 0 {
                continue;
            }

            let verdict = 'decide: {
                // If this is the first address we find, keep it anyway.
                let Some(best) = ia6_best else {
                    break 'decide Verdict::Replace;
                };
                let best_flags = best.ia6_flags.get();

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
                if flags & IN6_IFF_DEPRECATED != 0 {
                    // If we have already found a non-deprecated candidate, just ignore
                    // deprecated addresses.
                    if best_flags & IN6_IFF_DEPRECATED == 0 {
                        break 'decide Verdict::Skip;
                    }
                } else if best_flags & IN6_IFF_DEPRECATED != 0 {
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
                    let in6_bestgw = best.ia_gwaddr.addr();
                    let in6_newgw = ia.ia_gwaddr.addr();
                    if !in6_are_addr_equal(&in6_bestgw, gw6) && in6_are_addr_equal(&in6_newgw, gw6)
                    {
                        break 'decide Verdict::Replace;
                    }
                }

                // Rule 6: Prefer matching label. We do not implement policy tables.

                // Rule 7: Prefer temporary addresses.
                if best_flags & IN6_IFF_TEMPORARY != 0 && flags & IN6_IFF_TEMPORARY == 0 {
                    break 'decide Verdict::Skip;
                }
                if best_flags & IN6_IFF_TEMPORARY == 0 && flags & IN6_IFF_TEMPORARY != 0 {
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
                let best_pl = best.ia6_updatetime.get() + Time::from(best.ia6_lifetime.pltime());
                let new_pl = ia.ia6_updatetime.get() + Time::from(ia.ia6_lifetime.pltime());
                if best_pl < new_pl {
                    break 'decide Verdict::Replace;
                } else if best_pl > new_pl {
                    break 'decide Verdict::Skip;
                }

                // Prefer address with highest vltime.
                let best_vl = best.ia6_updatetime.get() + Time::from(best.ia6_lifetime.vltime());
                let new_vl = ia.ia6_updatetime.get() + Time::from(ia.ia6_lifetime.vltime());
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
                best_scope = in6_addrscope(&ifa_addr6);
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

/// `in6if_do_dad(ifp)`: whether Duplicate Address Detection runs on `ifp`.
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

// LP64 sizes of the requests this module reads at offsets.
const _: () = {
    assert!(IFR_IFRU == IFNAMSIZ);
    assert!(IFRA_ADDR == IFNAMSIZ);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests for the IPv6 addresses (LZ's, re-expressed over the kernel's interfaces and
    // routes, and the new shapes): the mask, scope and socket address helpers; the lookups;
    // RFC 6724's rules one by one; the address ioctls as the C answers them (privilege,
    // family, read-only flags, lifetimes, absent addresses); an address's link-local
    // companion, routes and memberships, its delete and its record recycled; and the
    // multicast records' counts with two joiners.

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::vec::Vec;

    use hal::sys::net_if::IFT_ETHER;
    use hal::sys::net_route::RTF_LOCAL;
    use hal::sys::netinet6_in6::{IN6ADDR_ANY, IN6MASK0, IN6MASK64, IN6MASK96, IN6MASK128};
    use hal::sys::netinet6_in6_var::In6Addrlifetime;
    use hal::sys::socket::AF_INET;

    use super::*;
    use crate::net::if_::{if_sadl_set_lladdr, test_ifnet_with};
    use crate::net::if_var::NetDriver;
    use crate::net::mbuf::MbufBox;
    use crate::net::route::tests::testroutes_setup;
    use crate::netinet6::in6_var::{free_counts, ia6_in6, in6_ifaddr_alloc};

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
        let mut a = In6Addr::default();
        for (i, w) in g.iter().enumerate() {
            a.s6_addr[2 * i..2 * i + 2].copy_from_slice(&w.to_be_bytes());
        }
        a
    }

    /// `a` with the zone of interface `ifidx` embedded (`s6_addr16[1]`).
    pub(crate) fn zoned(mut a: In6Addr, ifidx: u32) -> In6Addr {
        a.set_s6_addr16(1, htons(ifidx as u16));
        a
    }

    /// A driver that brings the interface up on its first address and counts the multicast
    /// filter changes; it drops what it is given to send.
    pub(crate) struct In6Driver {
        addmulti: AtomicU32,
        delmulti: AtomicU32,
    }

    impl NetDriver for In6Driver {
        fn ioctl(&self, ifp: &'static Ifnet, cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            match cmd {
                SIOCSIFADDR => ifp.if_flags.set(ifp.if_flags.get() | IFF_UP | IFF_RUNNING),
                SIOCADDMULTI => {
                    self.addmulti.fetch_add(1, Ordering::Relaxed);
                }
                SIOCDELMULTI => {
                    self.delmulti.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
            Ok(())
        }
        fn output(
            &self,
            _ifp: &'static Ifnet,
            _m: MbufBox,
            _dst: SockaddrView<'_>,
            _rt: Option<&Rtentry>,
        ) -> Result<(), Errno> {
            Ok(())
        }
    }

    /// The routing fixture (tables 0 of inet and inet6) and the records' free lists.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        testroutes_setup()
    }

    /// An Ethernet interface `name` with hardware address `mac`, multicast, MTU 1500, on the
    /// interface list, and its driver.
    pub(crate) fn iface(name: &[u8], mac: [u8; 6]) -> (&'static Ifnet, &'static In6Driver) {
        let d: &'static In6Driver = Box::leak(Box::new(In6Driver {
            addmulti: AtomicU32::new(0),
            delmulti: AtomicU32::new(0),
        }));
        let ifp = test_ifnet_with(name, d);
        ifp.if_type.set(IFT_ETHER);
        ifp.if_addrlen.set(6);
        ifp.if_flags.set(IFF_UP | IFF_RUNNING | IFF_MULTICAST);
        ifp.if_mtu.set(1500);
        crate::net::if_::if_alloc_sadl(ifp);
        if_sadl_set_lladdr(ifp, &mac);
        let _w = NETLOCK.write();
        IFNETLIST.insert_tail(ifp);
        (ifp, d)
    }

    /// An address `a`/64 with `flags` on `ifp`, linked as `in6_update_ifa` links one (the
    /// kernel's record), with infinite lifetimes and no route.
    pub(crate) fn test_ia6(ifp: &'static Ifnet, a: In6Addr, flags: i32) -> &'static In6Ifaddr {
        let _w = NETLOCK.write();
        let ia = in6_ifaddr_alloc(ifp, 0);
        ia.ia_addr.set(sin6_of(a));
        ia.ia_prefixmask.set(sin6_of(IN6MASK64));
        ia.ia6_flags.set(flags);
        ia.ia6_lifetime.set(In6Addrlifetime {
            ia6t_vltime: ND6_INFINITE_LIFETIME,
            ia6t_pltime: ND6_INFINITE_LIFETIME,
            ..In6Addrlifetime::default()
        });
        ifa_add(ifp, &ia.ia_ifa);
        ia
    }

    /// A `struct in6_aliasreq` of `addr`/`plen` with `flags` and infinite lifetimes.
    fn aliasreq(ifp: &Ifnet, addr: In6Addr, plen: i32, flags: i32) -> [u8; 128] {
        let mut ifra = In6Aliasreq::zeroed();
        ifra.ifra_name = ifp.if_xname.get();
        ifra.ifra_addr = sin6_of(addr);
        let mut mask = IN6ADDR_ANY;
        in6_prefixlen2mask(&mut mask, plen);
        ifra.ifra_prefixmask = sin6_of(mask);
        ifra.ifra_flags = flags;
        ifra.ifra_lifetime.ia6t_vltime = ND6_INFINITE_LIFETIME;
        ifra.ifra_lifetime.ia6t_pltime = ND6_INFINITE_LIFETIME;
        let mut r = [0u8; 128];
        assert!(pod::put(&mut r, 0, &ifra));
        r
    }

    /// A `struct in6_ifreq` whose `ifr_addr` is `addr`'s.
    fn ifreq(ifp: &Ifnet, addr: In6Addr) -> [u8; size_of::<In6Ifreq>()] {
        let mut ifr = In6Ifreq::zeroed();
        ifr.ifr_name = ifp.if_xname.get();
        ifr.set_ifr_addr(sin6_of(addr));
        let mut r = [0u8; size_of::<In6Ifreq>()];
        assert!(pod::put(&mut r, 0, &ifr));
        r
    }

    /// `ifconfig <ifp> inet6 <addr> prefixlen <plen>`: `SIOCAIFADDR_IN6`, privileged.
    fn add(ifp: &'static Ifnet, addr: In6Addr, plen: i32) -> Result<(), Errno> {
        let mut r = aliasreq(ifp, addr, plen, 0);
        in6_ioctl(SIOCAIFADDR_IN6, &mut r, Some(ifp), true)
    }

    /// `ifconfig <ifp> inet6 <addr> delete`: `SIOCDIFADDR_IN6`, privileged.
    fn del(ifp: &'static Ifnet, addr: In6Addr) -> Result<(), Errno> {
        let mut r = ifreq(ifp, addr);
        in6_ioctl(SIOCDIFADDR_IN6, &mut r, Some(ifp), true)
    }

    /// The flags of `addr` on `ifp` (`SIOCGIFAFLAG_IN6`).
    fn aflag(ifp: &'static Ifnet, addr: In6Addr) -> Result<i32, Errno> {
        let mut r = ifreq(ifp, addr);
        in6_ioctl(SIOCGIFAFLAG_IN6, &mut r, Some(ifp), false)?;
        Ok(pod::at::<In6Ifreq>(&r, 0)
            .expect("the request")
            .ifr_flags6())
    }

    /// The flags of table 0's route to `addr` (an exact host or the best match).
    fn route(addr: In6Addr) -> Option<u32> {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let d = sin6_bytes(&sin6_of(addr));
        rtalloc(sin6_view(&d), 0, 0, &net).map(|rt| rt.rt_flags.get())
    }

    /// `in6_ifawithscope` under the net lock.
    fn pick(oifp: &Ifnet, dst: In6Addr, rdomain: u32) -> Option<&'static In6Ifaddr> {
        let _w = NETLOCK.write();
        in6_ifawithscope(oifp, &dst, rdomain, None)
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
        let mut m = IN6MASK128;
        in6_prefixlen2mask(&mut m, 0);
        assert_eq!(m, IN6MASK0);
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
        // An invalid length leaves the mask alone (and logs).
        let _g = crate::net::if_::tests::setup();
        for bad in [-1, 129, i32::MAX, i32::MIN] {
            let mut m = IN6MASK64;
            in6_prefixlen2mask(&mut m, bad);
            assert_eq!(m, IN6MASK64, "an invalid length leaves the mask alone");
        }
    }

    #[test]
    fn mask2len_rejects_non_contiguous_masks_and_keeps_to_its_limit() {
        let mut m = IN6MASK64;
        m.s6_addr[15] = 1;
        assert_eq!(in6_mask2len(&m, None), -1);
        let mut m = In6Addr::default();
        m.s6_addr[..4].copy_from_slice(&[0xff, 0x80, 0, 0x01]);
        assert_eq!(in6_mask2len(&m, None), -1);
        // A hole inside a byte.
        let mut m = In6Addr::default();
        m.s6_addr[..3].copy_from_slice(&[0xff, 0xc0, 0x40]);
        assert_eq!(in6_mask2len(&m, None), -1);
        m.s6_addr[1] = 0xa0;
        m.s6_addr[2] = 0;
        assert_eq!(in6_mask2len(&m, None), -1);

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
    fn matchlen_scopes_and_zones() {
        let x = a6("2001:db8::1");
        for (y, n) in [
            ("2001:db8::1", 128),
            ("2001:db8::3", 126),
            ("2001:db8::0", 127),
            ("2001:db9::1", 31),
            ("a001:db8::1", 0),
            ("2001:db8:8000::1", 32),
        ] {
            assert_eq!(in6_matchlen(&x, &a6(y)), n, "{y}");
        }
        assert_eq!(in6_matchlen(&a6("fe80::1"), &a6("fe80::2")), 126);
        assert_eq!(in6_matchlen(&IN6ADDR_ANY, &IN6ADDR_LOOPBACK), 127);

        let (g, i, l, s) = (
            i32::from(__IPV6_ADDR_SCOPE_GLOBAL),
            i32::from(__IPV6_ADDR_SCOPE_INTFACELOCAL),
            i32::from(__IPV6_ADDR_SCOPE_LINKLOCAL),
            i32::from(__IPV6_ADDR_SCOPE_SITELOCAL),
        );
        for (a, scope) in [
            ("fe80::1", l),
            ("fec0::1", s),
            ("fe00::1", g), // "just in case"
            ("ff01::1", i),
            ("ff02::1", l),
            ("ff05::1", s),
            ("ff0e::1", g),
            ("ff08::1", g), // organization-local: global
            ("::1", i),
            ("::", l),
            ("::2", g),
            ("2001:db8::1", g),
            ("fd00:77::1", g),
        ] {
            assert_eq!(in6_addrscope(&a6(a)), scope, "{a}");
        }
        for (a, zone) in [
            ("fe80::1", 3),
            ("ff02::1", 3),
            ("::1", 3),
            ("ff01::2", 3),
            ("fec0::1", 0),
            ("2001:db8::1", 0),
        ] {
            assert_eq!(in6_addr2scopeid(3, &a6(a)), zone, "{a}");
        }
    }

    #[test]
    fn embedded_scope_and_socket_address_checks() {
        let mut sa = sin6_of(a6("fe80::1"));
        assert_eq!(in6_check_embed_scope(&mut sa, 7), Ok(()));
        assert_eq!(sa.sin6_addr.s6_addr16(1), htons(7));
        assert_eq!(in6_check_embed_scope(&mut sa, 7), Ok(()));
        assert_eq!(in6_check_embed_scope(&mut sa, 8), Err(Errno::EINVAL));
        // Not link-local: untouched.
        let mut g = sin6_of(a6("2001:db8::1"));
        assert_eq!(in6_check_embed_scope(&mut g, 7), Ok(()));
        assert_eq!(g.sin6_addr, a6("2001:db8::1"));

        let mut sa = sin6_of(a6("fe80::1"));
        sa.sin6_scope_id = 7;
        assert_eq!(in6_clear_scope_id(&mut sa, 8), Err(Errno::EINVAL));
        assert_eq!(in6_clear_scope_id(&mut sa, 7), Ok(()));
        assert_eq!(sa.sin6_scope_id, 0);
        let mut g = sin6_of(a6("2001:db8::1"));
        g.sin6_scope_id = 9;
        assert_eq!(in6_clear_scope_id(&mut g, 7), Ok(()));
        assert_eq!(g.sin6_scope_id, 9);

        // in6_sa2sin6: the family, then the length.
        let mut b = sin6_bytes(&sin6_of(a6("fd00::1")));
        assert_eq!(in6_sa2sin6(&b), Ok(sin6_of(a6("fd00::1"))));
        b[0] = 16;
        assert_eq!(in6_sa2sin6(&b), Err(Errno::EINVAL));
        b[0] = SIN6_LEN;
        b[1] = AF_INET;
        assert_eq!(in6_sa2sin6(&b), Err(Errno::EAFNOSUPPORT));
        assert_eq!(in6_sa2sin6(&b[..1]), Err(Errno::EAFNOSUPPORT));
    }

    #[test]
    fn do_dad_needs_an_up_and_running_non_loopback_interface() {
        let _g = crate::net::if_::tests::setup();
        let (ifp, _) = crate::net::if_::test_ifnet(b"tdad0");
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
        let (ifp, _) = iface(b"tlk0", [0x52, 0x54, 0, 0, 0, 1]);
        assert!(in6ifa_ifpforlinklocal(ifp, 0).is_none());
        let t = test_ia6(ifp, a6("fe80:1::5054:ff:febb:2"), IN6_IFF_TENTATIVE);
        let ll = test_ia6(ifp, a6("fe80:1::5054:ff:febb:3"), 0);
        let g = test_ia6(ifp, a6("fd00:77::1"), 0);
        let found = in6ifa_ifpwithaddr(ifp, &a6("fd00:77::1")).expect("on list");
        assert!(ptr::eq(found, g));
        assert!(in6ifa_ifpwithaddr(ifp, &a6("fd00:77::2")).is_none());
        // The first link-local address, unless its flags are ignored: then the next one.
        let first = in6ifa_ifpforlinklocal(ifp, 0).expect("link-local");
        assert!(ptr::eq(first, t));
        let anycast = in6ifa_ifpforlinklocal(ifp, IN6_IFF_ANYCAST).expect("link-local");
        assert!(ptr::eq(anycast, t));
        let untentative = in6ifa_ifpforlinklocal(ifp, IN6_IFF_TENTATIVE).expect("link-local");
        assert!(ptr::eq(untentative, ll));
        // The back-link is the cast.
        assert!(ptr::eq(crate::netinet6::in6_var::ifatoia6(&g.ia_ifa), g));
    }

    #[test]
    fn source_address_follows_the_rfc_6724_rules() {
        let _g = setup();
        let (if1, _) = iface(b"tsa0", [0x52, 0x54, 0, 0, 1, 1]);
        let (if2, _) = iface(b"tsa1", [0x52, 0x54, 0, 0, 1, 2]);
        let i1 = if1.if_index.get();
        let ll1 = test_ia6(if1, zoned(a6("fe80::1"), i1), 0);
        let ula1 = test_ia6(if1, a6("fd00::1"), 0);
        let doc2 = test_ia6(if2, a6("2001:db8::5"), 0);
        let tent2 = test_ia6(if2, a6("2001:db8::99"), IN6_IFF_TENTATIVE);

        // (output interface, destination, the address chosen): one rule each.
        let table: [(&Ifnet, In6Addr, Option<&In6Ifaddr>, &str); 5] = [
            // Rule 5: a global destination takes the global address of the output interface,
            // not the other interface's (and not the link-local one, rule 2).
            (if1, a6("2001:db8::99"), Some(ula1), "rule 5, if1"),
            (
                if2,
                a6("2001:db8::99"),
                Some(doc2),
                "rule 5, if2; rule 8 never sees ::99",
            ),
            // Rule 2: a link-local destination, the smallest scope big enough, in its zone.
            (if1, zoned(a6("fe80::2"), i1), Some(ll1), "rule 2"),
            // Rule 5 before rule 8: the output interface's global address over the other
            // interface's longer match (2001:db8::5 shares 125 bits, fd00::1 none).
            (if1, a6("2001:db8::1"), Some(ula1), "rule 5 before rule 8"),
            // The unspecified destination: link-local scope, if1's.
            (if1, IN6ADDR_ANY, Some(ll1), "the unspecified address"),
        ];
        for (oifp, dst, want, what) in table {
            let got = pick(oifp, dst, 0);
            match want {
                Some(w) => assert!(got.is_some_and(|g| ptr::eq(g, w)), "{what}"),
                None => assert!(got.is_none(), "{what}"),
            }
        }
        // The tentative address is never chosen; nor any in another routing domain.
        assert!(!pick(if2, a6("2001:db8::99"), 0).is_some_and(|g| ptr::eq(g, tent2)));
        assert!(pick(if1, a6("2001:db8::99"), 5).is_none());
    }

    #[test]
    fn source_selection_skips_unusable_addresses_and_avoids_deprecated_ones() {
        let _g = setup();
        let (ifp, _) = iface(b"tsb0", [0x52, 0x54, 0, 0, 2, 1]);
        // All unusable: tentative, duplicated, anycast, detached.
        test_ia6(ifp, a6("2001:db8::10"), IN6_IFF_TENTATIVE);
        test_ia6(ifp, a6("2001:db8::11"), IN6_IFF_DUPLICATED);
        test_ia6(ifp, a6("2001:db8::12"), IN6_IFF_ANYCAST);
        test_ia6(ifp, a6("2001:db8::13"), IN6_IFF_DETACHED);
        assert!(pick(ifp, a6("2001:db8::99"), 0).is_none());

        // Rule 3: a deprecated address loses to a preferred one, whatever the order.
        let dep = test_ia6(ifp, a6("2001:db8::99"), IN6_IFF_DEPRECATED);
        assert!(ptr::eq(pick(ifp, a6("2001:db8::99"), 0).expect("src"), dep));
        let ok = test_ia6(ifp, a6("2001:db8:1::1"), 0);
        assert!(ptr::eq(pick(ifp, a6("2001:db8::99"), 0).expect("src"), ok));
        // Rule 8: of two preferred addresses, the longest matching prefix wins.
        let ok2 = test_ia6(ifp, a6("2001:db8::7"), 0);
        assert!(ptr::eq(pick(ifp, a6("2001:db8::99"), 0).expect("src"), ok2));
        // Rule 7: a temporary address is preferred to a public one, even with a shorter match.
        let tmp = test_ia6(ifp, a6("2001:db8::98"), IN6_IFF_TEMPORARY);
        assert!(ptr::eq(pick(ifp, a6("2001:db8::99"), 0).expect("src"), tmp));
    }

    #[test]
    fn selected_sources_are_counted() {
        use crate::netinet6::ip6_var::IP6COUNTERS;

        let _g = setup();
        let (ifp, _) = iface(b"tsc0", [0x52, 0x54, 0, 0, 3, 1]);
        test_ia6(ifp, a6("2001:db8::1"), 0);
        let none = Ip6statCounters::Ip6sSourcesNone as usize;
        let global =
            Ip6statCounters::Ip6sSourcesSameif as usize + usize::from(__IPV6_ADDR_SCOPE_GLOBAL);
        let before = IP6COUNTERS.read();
        assert!(pick(ifp, a6("2001:db8::2"), 0).is_some());
        assert!(pick(ifp, a6("2001:db8::2"), 9).is_none());
        let after = IP6COUNTERS.read();
        assert_eq!(after[global], before[global] + 1);
        assert_eq!(after[none], before[none] + 1);
    }

    #[test]
    fn the_address_ioctls_answer_as_the_c() {
        let _g = setup();
        let (ifp, _) = iface(b"tio0", [0x52, 0x54, 0, 0, 4, 1]);

        // The refusals before any lock: no interface, no privilege, the requests the C does
        // not pass to the driver, an unknown one, ND's two (bsd's in6_control answers them).
        let mut r = aliasreq(ifp, a6("fd00::1"), 64, 0);
        assert_eq!(
            in6_ioctl(SIOCAIFADDR_IN6, &mut r, None, true),
            Err(Errno::ENXIO)
        );
        for cmd in [SIOCAIFADDR_IN6, SIOCDIFADDR_IN6] {
            let mut r = [0u8; 288];
            assert_eq!(in6_ioctl(cmd, &mut r, Some(ifp), false), Err(Errno::EPERM));
        }
        for (cmd, want) in [
            (SIOCSIFADDR, Errno::EINVAL),
            (SIOCSIFDSTADDR, Errno::EINVAL),
            (SIOCSIFBRDADDR, Errno::EINVAL),
            (SIOCSIFNETMASK, Errno::EINVAL),
            (SIOCADDMULTI, Errno::EOPNOTSUPP),
            (SIOCGIFINFO_IN6, Errno::EOPNOTSUPP),
            (SIOCGNBRINFO_IN6, Errno::EOPNOTSUPP),
        ] {
            let mut r = [0u8; 288];
            assert_eq!(
                in6_ioctl(cmd, &mut r, Some(ifp), true),
                Err(want),
                "{cmd:#x}"
            );
        }
        // A request shorter than its structure.
        let mut short = [0u8; 16];
        assert_eq!(
            in6_ioctl(SIOCAIFADDR_IN6, &mut short, Some(ifp), true),
            Err(Errno::EINVAL)
        );

        // SIOCAIFADDR_IN6: a /0, read-only flags, another family, pltime > vltime.
        assert_eq!(add(ifp, a6("fd00::1"), 0), Err(Errno::EINVAL));
        for flags in [IN6_IFF_DUPLICATED, IN6_IFF_DETACHED, IN6_IFF_DEPRECATED] {
            let mut r = aliasreq(ifp, a6("fd00::2"), 64, flags);
            assert_eq!(
                in6_ioctl(SIOCAIFADDR_IN6, &mut r, Some(ifp), true),
                Err(Errno::EINVAL)
            );
        }
        let mut r = aliasreq(ifp, a6("fd00::3"), 64, 0);
        r[IFRA_ADDR + 1] = AF_INET;
        assert_eq!(
            in6_ioctl(SIOCAIFADDR_IN6, &mut r, Some(ifp), true),
            Err(Errno::EAFNOSUPPORT)
        );
        let mut ifra =
            pod::at::<In6Aliasreq>(&aliasreq(ifp, a6("fd00::4"), 64, 0), 0).expect("the request");
        ifra.ifra_lifetime.ia6t_pltime = 100;
        ifra.ifra_lifetime.ia6t_vltime = 50;
        let mut r = [0u8; 128];
        assert!(pod::put(&mut r, 0, &ifra));
        assert_eq!(
            in6_ioctl(SIOCAIFADDR_IN6, &mut r, Some(ifp), true),
            Err(Errno::EINVAL)
        );
        assert!(in6ifa_ifpwithaddr(ifp, &a6("fd00::4")).is_none());

        // SIOCDIFADDR_IN6 and the gets of an absent address; a link-local address in another
        // zone.
        assert_eq!(del(ifp, a6("fd00::9")), Err(Errno::EADDRNOTAVAIL));
        assert_eq!(aflag(ifp, a6("fd00::9")), Err(Errno::EADDRNOTAVAIL));
        let mut r = ifreq(ifp, zoned(a6("fe80::1"), ifp.if_index.get() + 1));
        assert_eq!(
            in6_ioctl(SIOCGIFAFLAG_IN6, &mut r, Some(ifp), false),
            Err(Errno::EINVAL)
        );
    }

    #[test]
    fn an_address_brings_its_link_local_companion_its_routes_and_memberships() {
        let _g = setup();
        let (ifp, d) = iface(b"tcf0", [0x52, 0x54, 0x00, 0xbb, 0x00, 0x01]);
        let idx = ifp.if_index.get();
        let (_, free_before) = free_counts();

        add(ifp, a6("fd00:77::1"), 64).expect("SIOCAIFADDR_IN6");
        let ia = in6ifa_ifpwithaddr(ifp, &a6("fd00:77::1")).expect("the address");
        assert_eq!(in6_mask2len(&ia6_maskin6(ia), None), 64);
        // The interface came up through SIOCSIFADDR: the address is tentative (DAD's).
        assert_eq!(
            aflag(ifp, a6("fd00:77::1")).expect("its flags") & IN6_IFF_TENTATIVE,
            IN6_IFF_TENTATIVE
        );

        // The link-local address of the MAC's EUI-64 was made first: fe80::5054:ff:febb:1,
        // the zone embedded as the interface index (smoke-inet6's line).
        let ll = in6ifa_ifpforlinklocal(ifp, 0).expect("link-local address");
        assert_eq!(ia6_in6(ll), zoned(a6("fe80::5054:ff:febb:1"), idx));

        // The routes: the connected prefix, the local address.
        let net = route(a6("fd00:77::5")).expect("the connected route");
        assert_eq!(
            net & (RTF_CLONING | RTF_CONNECTED),
            RTF_CLONING | RTF_CONNECTED
        );
        let local = route(a6("fd00:77::1")).expect("the local route");
        assert_eq!(local & (RTF_HOST | RTF_LOCAL), RTF_HOST | RTF_LOCAL);

        // The multicast groups joined: the solicited-node groups of both addresses, the
        // all-nodes groups link- and interface-local, each the driver heard once.
        let snm = zoned(a6("ff02::1:ff00:1"), idx);
        assert!(
            in6_hasmulti(&snm, ifp),
            "solicited-node group of fd00:77::1"
        );
        let snm_ll = zoned(a6("ff02::1:ffbb:1"), idx);
        assert!(
            in6_hasmulti(&snm_ll, ifp),
            "solicited-node group of the link-local"
        );
        assert!(in6_hasmulti(&zoned(IN6ADDR_LINKLOCAL_ALLNODES, idx), ifp));
        assert!(in6_hasmulti(
            &zoned(IN6ADDR_INTFACELOCAL_ALLNODES, idx),
            ifp
        ));
        assert_eq!(d.addmulti.load(Ordering::Relaxed), 4, "two groups shared");
        assert_eq!(ia.ia6_memberships.lock().len(), 3);

        // The address is deleted again and takes its own group with it; the shared ones
        // stay (the link-local address holds them).
        del(ifp, a6("fd00:77::1")).expect("SIOCDIFADDR_IN6");
        assert!(in6ifa_ifpwithaddr(ifp, &a6("fd00:77::1")).is_none());
        assert!(!in6_hasmulti(&snm, ifp));
        assert!(in6_hasmulti(&zoned(IN6ADDR_LINKLOCAL_ALLNODES, idx), ifp));
        assert_eq!(d.delmulti.load(Ordering::Relaxed), 1);
        assert!(in6ifa_ifpforlinklocal(ifp, 0).is_some());
        assert!(route(a6("fd00:77::5")).is_none(), "the prefix route went");

        // Its records are recycled: the next address takes them back (once the routes that
        // held the address are released).
        crate::net::route::tests::grace();
        let (_, free_after) = free_counts();
        assert_eq!(free_after, free_before + 1, "the multicast record freed");
        add(ifp, a6("fd00:77::2"), 64).expect("SIOCAIFADDR_IN6");
        let ia2 = in6ifa_ifpwithaddr(ifp, &a6("fd00:77::2")).expect("the address");
        assert!(ptr::eq(ia2, ia), "the record recycled");
        assert_eq!(free_counts().1, free_before, "the multicast record reused");
    }

    #[test]
    fn a_group_two_owners_join_is_one_record_the_driver_hears_once() {
        let _g = setup();
        let (ifp, d) = iface(b"tmc0", [0x52, 0x54, 0, 0, 5, 1]);
        let g = zoned(a6("ff02::1234"), ifp.if_index.get());
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let a = in6_addmulti(&g, ifp, &net).expect("first join");
        let b = in6_addmulti(&g, ifp, &net).expect("second join");
        assert!(ptr::eq(a, b));
        assert_eq!(d.addmulti.load(Ordering::Relaxed), 1);
        in6_delmulti(a, &net);
        assert!(in6_hasmulti(&g, ifp), "the second owner keeps it");
        assert_eq!(d.delmulti.load(Ordering::Relaxed), 0);
        in6_delmulti(b, &net);
        assert!(!in6_hasmulti(&g, ifp));
        assert_eq!(d.delmulti.load(Ordering::Relaxed), 1);
        drop(w);
    }
}
/* </TESTS> */
