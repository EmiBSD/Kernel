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
//! The Internet addresses of the interfaces, `netinet/in.c`: the address `ioctl`s, the
//! interface and prefix routes, the broadcast test and the multicast records. The
//! constants and structures of `<netinet/in.h>` are `hal::sys::netinet_in`; the records are
//! `kernel::netinet::in_var`'s.
//!
//! Upstream: sys/netinet/in.h @ 3ce1f3f79392
//! Upstream: sys/netinet/in.c @ 3ce1f3f79392
//! LZ: sys/netinet/in_.rs@3c62ede68802
//!
//! Byte order: as in the C kernel, addresses and ports stay in network order wherever they are
//! stored (`in_addr.s_addr`, `sin_port`), and `INADDR_*`, `IN_CLASS*_NET`/`_HOST` and the
//! `in_class*` tests are network-order values over network-order arguments.
//!
//! ## Deviations
//! - `MROUTING` is not configured (`mrt_ioctl` in `in_control`).
//! - The driver's `SIOCSIFADDR` and `SIOCSIFDSTADDR` get a zeroed `struct ifreq` image where
//!   the C passes the `struct in_ifaddr` (D-14: no driver ported at the pin reads it;
//!   `if_ppp.c` and `if_spppsubr.c`, which do, are not ported).
//! - The records are never freed (`kernel::netinet::in_var`'s deviations); a membership whose
//!   interface has already left the index map when its last reference goes (`if_detach`'s
//!   `in_ifdetach`, after `if_remove`) stays on that interface's list, where the C frees it
//!   there, and is not reused.
//!
//! ## Redesign
//! - The address `ioctl`s over the request's bytes: LZ's `unsafe fn`s over `caddr_t data`
//!   (with an aligned copy of an unaligned request) -> [`in_ioctl`] over `&mut [u8]`, the
//!   `struct ifreq`'s and `struct in_aliasreq`'s addresses read and written by copies at their
//!   offsets (`hal::pod::at`/`put`, the C's `*sin` copies and `*satosin(..) = ia->ia_addr`
//!   stores); `in_sa2sin` and `in_nam2sin` answer the checked `sockaddr_in` by value.
//! - `NET_LOCK()` stays detached (`hal::netlock::lock`) for the requests that change an
//!   address, since the driver's `SIOCSIFADDR` may let it go and take it again (wg(4)'s
//!   `wg_up`, D-14); the steps that need its proof (the routes) borrow it in closures
//!   (`hal::netlock::lend_detached`), never across the driver's two requests. The kernel lock
//!   is taken C-style around the same span, in the C's order (net, then kernel).
//!   `NET_LOCK_SHARED()` of `in_ioctl_get` is a witness (no driver call).
//! - `in_ifaddr_alloc`'s and `in_addmulti`'s `malloc(M_WAITOK | M_ZERO)` take a record off
//!   `in_var`'s free lists first; `free` puts one back.
//! - `in_broadcast`, `in_ifp2ia`, `in_lookupmulti` read the records' own words (`InIfaddr::of`
//!   of each `AF_INET` address, `SinWords::addr`), no copy of the generic part's address.
//!
//! **Lock and spl transitions** against the C: `in_ioctl`'s changes take the net lock and the
//! kernel lock once each, as the C; `in_ioctl_get` the net lock shared; `in_addmulti` and
//! `in_delmulti` `if_maddrlock` and the kernel lock as the C (twice and once, once and once);
//! `in_hasmulti` `if_maddrlock` read once. No spl change: the C has none.

use core::mem::{offset_of, size_of};

use hal::netlock::{NETLOCK, NetHeld};
use hal::pod;
use hal::rwlock::{RW_READ, RW_WRITE};
use hal::sys::endian::{htonl, ntohl};
use hal::sys::errno::Errno;
use hal::sys::ioccom::iocparm_len;
use hal::sys::net_if::{
    IFF_BROADCAST, IFF_LOOPBACK, IFF_MULTICAST, IFF_POINTOPOINT, IFNAMSIZ, IFXF_AUTOCONF4,
};
use hal::sys::net_route::{RTF_BROADCAST, RTF_CLONING, RTF_CONNECTED, RTF_HOST, RTF_MPATH};
use hal::sys::netinet_in::{
    IN_CLASSA_NET, IN_CLASSA_NSHIFT, IN_CLASSB_NET, IN_CLASSC_NET, IN_LOOPBACKNET,
    INADDR_ALLHOSTS_GROUP, INADDR_ANY, InAddr, SockaddrIn, in_classa, in_classb, in_multicast,
    in_rfc3021_subnet,
};
use hal::sys::socket::AF_INET;
use hal::sys::sockio::{
    SIOCADDMULTI, SIOCAIFADDR, SIOCDELMULTI, SIOCDIFADDR, SIOCGIFADDR, SIOCGIFBRDADDR,
    SIOCGIFDSTADDR, SIOCGIFNETMASK, SIOCSIFADDR, SIOCSIFBRDADDR, SIOCSIFDSTADDR, SIOCSIFNETMASK,
};

use crate::assert::{kassert, kpanic};
use crate::net::if_::{IFNETLIST, IfRef, if_addrhooks_run, if_ioctl, ifa_add, ifa_del};
use crate::net::if_var::{Ifaddr, Ifnet};
use crate::net::mbuf::Mbuf;
use crate::net::proto::SockaddrView;
use crate::net::route::{
    ifafree, rt_ifa_add, rt_ifa_addlocal, rt_ifa_del, rt_ifa_dellocal, rt_ifa_purge,
};
use crate::net::rtable::rtable_l2;
use crate::netinet::igmp::{igmp_joingroup, igmp_leavegroup, igmp_sendpkt};
use crate::netinet::igmp_var::IgmpPktinfo;
use crate::netinet::in_var::{InIfaddr, InMulti, in_ifaddr_alloc, in_multi_alloc, in_multi_free};

/// `sizeof(struct sockaddr_in)`.
const SIN_SIZE: usize = size_of::<SockaddrIn>();
/// `sizeof(struct ifreq)`: the name, then the union.
const IFREQ_SIZE: usize = IFNAMSIZ + 16;
/// `offsetof(struct ifreq, ifr_ifru)`: `ifr_addr`, `ifr_dstaddr`, `ifr_broadaddr`.
const IFR_ADDR: usize = IFNAMSIZ;
/// `offsetof(struct in_aliasreq, ifra_addr)`.
const IFRA_ADDR: usize = offset_of!(hal::sys::netinet_in_var::InAliasreq, ifra_addr);
/// `offsetof(struct in_aliasreq, ifra_dstaddr)` (`ifra_broadaddr`).
const IFRA_DSTADDR: usize = offset_of!(hal::sys::netinet_in_var::InAliasreq, ifra_dstaddr);
/// `offsetof(struct in_aliasreq, ifra_mask)`.
const IFRA_MASK: usize = offset_of!(hal::sys::netinet_in_var::InAliasreq, ifra_mask);
/// `offsetof(struct sockaddr, sa_data)`.
const SA_DATA: usize = 2;

/// `NET_ASSERT_LOCKED()`.
#[inline]
fn net_assert_locked() {
    kassert!(matches!(hal::netlock::status(), RW_WRITE | RW_READ));
}

/// The `sockaddr_in` image of `sin`, for the routes' requests.
#[inline]
fn sin_bytes(sin: &SockaddrIn) -> [u8; SIN_SIZE] {
    let mut b = [0u8; SIN_SIZE];
    let _ = pod::put(&mut b, 0, sin);
    b
}

/// The view of a `sockaddr_in` image (16 bytes: always a socket address).
#[inline]
fn sin_view(b: &[u8; SIN_SIZE]) -> SockaddrView<'_> {
    match SockaddrView::new(b) {
        Some(v) => v,
        None => kpanic(format_args!("in: a sockaddr_in too short")),
    }
}

/// A `sockaddr_in` of `addr`, `sin_len` and family set.
#[inline]
fn sin_of(addr: InAddr) -> SockaddrIn {
    SockaddrIn {
        sin_len: SIN_SIZE as u8,
        sin_family: AF_INET,
        sin_addr: addr,
        ..SockaddrIn::default()
    }
}

/// `in_canforward(in)`: whether datagrams to `in_` may be forwarded (it is not in a reserved
/// set of addresses that may not be).
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

/// `in_socktrim(ap)`: trims a mask in a sockaddr: `sin_len` covers the address up to its last
/// non-zero byte.
pub fn in_socktrim(ap: &mut SockaddrIn) {
    let base = offset_of!(SockaddrIn, sin_addr);
    let bytes = ap.sin_addr.s_addr.to_ne_bytes();

    ap.sin_len = 0;
    if let Some(i) = bytes.iter().rposition(|&b| b != 0) {
        ap.sin_len = (base + i + 1) as u8;
    }
}

/// `in_mask2len(mask)`: the prefix length of a contiguous mask.
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

/// `in_len2mask(mask, len)`: the mask of prefix length `len`.
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

/// `in_nam2sin(nam, &sin)`: the `sockaddr_in` in mbuf `nam`, checked (`EINVAL` for a name
/// shorter than its header or not exactly a `sockaddr_in`, `EAFNOSUPPORT` for another family),
/// copied.
pub fn in_nam2sin(nam: &Mbuf) -> Result<SockaddrIn, Errno> {
    let d = nam.data();
    if d.len() < SA_DATA {
        return Err(Errno::EINVAL);
    }
    if d[1] != AF_INET {
        return Err(Errno::EAFNOSUPPORT);
    }
    if usize::from(d[0]) != d.len() {
        return Err(Errno::EINVAL);
    }
    if usize::from(d[0]) != SIN_SIZE {
        return Err(Errno::EINVAL);
    }
    pod::at::<SockaddrIn>(d, 0).ok_or(Errno::EINVAL)
}

/// `in_sa2sin(sa, &sin)`: the socket address at the start of `sa` as a `sockaddr_in`,
/// checked (`EAFNOSUPPORT` for another family, `EINVAL` for another length), copied.
pub fn in_sa2sin(sa: &[u8]) -> Result<SockaddrIn, Errno> {
    if sa.get(1) != Some(&AF_INET) {
        return Err(Errno::EAFNOSUPPORT);
    }
    if sa.first().map(|&l| usize::from(l)) != Some(SIN_SIZE) {
        return Err(Errno::EINVAL);
    }
    pod::at::<SockaddrIn>(sa, 0).ok_or(Errno::EINVAL)
}

/// `in_ifp2ia(ifp)`: the first Internet address of interface `ifp`.
pub fn in_ifp2ia(ifp: &Ifnet) -> Option<&'static InIfaddr> {
    net_assert_locked();

    ifp.if_addrlist.iter().find_map(InIfaddr::of)
}

/// The Internet address of `ifp` whose address is `addr`, or the first one when `addr` is
/// none or `INADDR_ANY` (`in_ioctl`'s and `in_ioctl_get`'s walk: "find first address or
/// exact match").
fn first_or_exact(ifp: &Ifnet, addr: Option<InAddr>) -> Option<&'static InIfaddr> {
    let mut ia = None;
    for ifa in &ifp.if_addrlist {
        let Some(this) = InIfaddr::of(ifa) else {
            continue;
        };
        // find first address or exact match
        if ia.is_none() {
            ia = Some(this);
        }
        let Some(a) = addr.filter(|a| a.s_addr != INADDR_ANY) else {
            break;
        };
        if this.ia_addr.addr().s_addr == a.s_addr {
            ia = Some(this);
            break;
        }
    }
    ia
}

/// `in_control(so, cmd, data, ifp)`: the `pru_control` of the Internet protocols, the
/// address `ioctl`s of `cmd` on `ifp` with `data` the kernel's copy of the request
/// (`IOCPARM_LEN(cmd)` bytes), privileged when the socket is (`SS_PRIV`). `MROUTING`
/// (`mrt_ioctl`) is not configured.
pub fn in_control(
    privileged: bool,
    cmd: u64,
    data: &mut [u8],
    ifp: Option<&'static Ifnet>,
) -> Result<(), Errno> {
    in_ioctl(cmd, data, ifp, privileged)
}

/// `in_ioctl(cmd, data, ifp, privileged)`: the Internet address `ioctl`s. Every change but
/// none of the reads asks for `privileged`, before any lock is taken (the C's `EPERM`s);
/// `data` is the request, `IOCPARM_LEN(cmd)` bytes (`EINVAL` when it is shorter).
pub fn in_ioctl(
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
        SIOCGIFADDR | SIOCGIFNETMASK | SIOCGIFDSTADDR | SIOCGIFBRDADDR => {
            return in_ioctl_get(cmd, data, ifp);
        }
        SIOCSIFADDR => {
            if !privileged {
                return Err(Errno::EPERM);
            }
            return in_ioctl_set_ifaddr(cmd, data, ifp);
        }
        SIOCAIFADDR | SIOCDIFADDR => {
            if !privileged {
                return Err(Errno::EPERM);
            }
            return in_ioctl_change_ifaddr(cmd, data, ifp);
        }
        SIOCSIFNETMASK | SIOCSIFDSTADDR | SIOCSIFBRDADDR => {}
        _ => return Err(Errno::EOPNOTSUPP),
    }

    if !privileged {
        return Err(Errno::EPERM);
    }

    // `ifr->ifr_addr`, which `ifr_dstaddr` and `ifr_broadaddr` share.
    let mut sin = None;
    if data[IFR_ADDR + 1] == AF_INET {
        sin = Some(in_sa2sin(&data[IFR_ADDR..IFR_ADDR + SIN_SIZE])?);
    }

    hal::netlock::lock();
    hal::klock::kernel_lock();

    let error = 'err: {
        let Some(ia) = first_or_exact(ifp, sin.map(|s| s.sin_addr)) else {
            break 'err Err(Errno::EADDRNOTAVAIL);
        };

        match cmd {
            SIOCSIFDSTADDR => {
                if ifp.if_flags.get() & IFF_POINTOPOINT == 0 {
                    break 'err Err(Errno::EINVAL);
                }
                let sin = match in_sa2sin(&data[IFR_ADDR..IFR_ADDR + SIN_SIZE]) {
                    Ok(s) => s,
                    Err(e) => break 'err Err(e),
                };
                let oldaddr = ia.ia_dstaddr.get();
                ia.ia_dstaddr.set(sin);
                let mut req = [0u8; IFREQ_SIZE];
                if let Err(e) = if_ioctl(ifp, SIOCSIFDSTADDR, &mut req) {
                    ia.ia_dstaddr.set(oldaddr);
                    break 'err Err(e);
                }
                hal::netlock::lend_detached(|net| {
                    let _ = in_scrubhost(ia, &oldaddr, net);
                    let _ = in_addhost(ia, &ia.ia_dstaddr.get(), net);
                });
                Ok(())
            }

            SIOCSIFBRDADDR => {
                if ifp.if_flags.get() & IFF_BROADCAST == 0 {
                    break 'err Err(Errno::EINVAL);
                }
                match in_sa2sin(&data[IFR_ADDR..IFR_ADDR + SIN_SIZE]) {
                    Ok(sin) => {
                        ifa_update_broadaddr(ia, &sin);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }

            // SIOCSIFNETMASK
            _ => {
                if data[IFR_ADDR] < 8 {
                    break 'err Err(Errno::EINVAL);
                }
                // do not check inet family or strict len
                let mask = sin_addr_at(data, IFR_ADDR);
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
        }
    };
    // err:
    hal::klock::kernel_unlock();
    hal::netlock::unlock();
    error
}

/// The `sin_addr` of the 16-byte socket address at `off` of `data`, whatever its family and
/// length say (`SIOCSIFNETMASK`'s and `ifra_mask`'s "do not check inet family or strict len").
#[inline]
fn sin_addr_at(data: &[u8], off: usize) -> u32 {
    let mut a = [0u8; 4];
    if let Some(b) = data.get(off + 4..off + 8) {
        a.copy_from_slice(b);
    }
    u32::from_ne_bytes(a)
}

/// `ifa_update_broadaddr(ifp, &ia->ia_ifa, sintosa(sin))` for an Internet address: the
/// broadcast address replaced in place; `panic(9)` when its length would change (the C's
/// "does not support dynamic length").
fn ifa_update_broadaddr(ia: &InIfaddr, sin: &SockaddrIn) {
    if ia.ia_broadaddr().len() != sin.sin_len {
        kpanic(format_args!(
            "ifa_update_broadaddr does not support dynamic length"
        ));
    }
    ia.ia_broadaddr().set(*sin);
}

/// `in_ioctl_set_ifaddr(cmd, data, ifp)`: `SIOCSIFADDR`, the first Internet address of the
/// interface (`data` a `struct ifreq`).
fn in_ioctl_set_ifaddr(cmd: u64, data: &mut [u8], ifp: &'static Ifnet) -> Result<(), Errno> {
    if cmd != SIOCSIFADDR {
        kpanic(format_args!("in_ioctl_set_ifaddr: invalid ioctl {cmd}"));
    }

    let sin = in_sa2sin(&data[IFR_ADDR..IFR_ADDR + SIN_SIZE])?;

    hal::netlock::lock();
    hal::klock::kernel_lock();

    // find first address
    let (ia, newifaddr) = match in_ifp2ia(ifp) {
        Some(ia) => (ia, false),
        None => (in_ifaddr_alloc(ifp), true),
    };

    hal::netlock::lend_detached(|net| in_ifscrub(ifp, ia, net));
    let error = in_ifinit(ifp, ia, &sin, newifaddr);
    if error.is_ok() {
        if_addrhooks_run(ifp);
    }

    hal::klock::kernel_unlock();
    hal::netlock::unlock();
    error
}

/// `in_ioctl_change_ifaddr(cmd, data, ifp)`: `SIOCAIFADDR` (add or change an address, its
/// mask, destination and broadcast address) and `SIOCDIFADDR` (delete it), `data` a
/// `struct in_aliasreq`.
fn in_ioctl_change_ifaddr(cmd: u64, data: &mut [u8], ifp: &'static Ifnet) -> Result<(), Errno> {
    let mut sin = None;
    if data[IFRA_ADDR + 1] == AF_INET {
        sin = Some(in_sa2sin(&data[IFRA_ADDR..IFRA_ADDR + SIN_SIZE])?);
    }

    hal::netlock::lock();
    hal::klock::kernel_lock();

    // find first address, if no exact match wanted
    let ia = ifp.if_addrlist.iter().find_map(|ifa| {
        InIfaddr::of(ifa)
            .filter(|ia| sin.is_none_or(|s| s.sin_addr.s_addr == ia.ia_addr.addr().s_addr))
    });

    let error = 'out: {
        match cmd {
            SIOCAIFADDR => {
                let mut needinit = false;
                let mut masksin = None;
                let mut dstsin = None;
                let mut broadsin = None;

                let mask_len = data[IFRA_MASK];
                if mask_len != 0 {
                    if mask_len < 8 {
                        break 'out Err(Errno::EINVAL);
                    }
                    // do not check inet family or strict len
                    let m = sin_addr_at(data, IFRA_MASK);
                    if ntohl(m) & (!ntohl(m) >> 1) != 0 {
                        // non-contiguous netmask
                        break 'out Err(Errno::EINVAL);
                    }
                    masksin = Some(m);
                }
                let dst_family = data[IFRA_DSTADDR + 1];
                if ifp.if_flags.get() & IFF_POINTOPOINT != 0 && dst_family == AF_INET {
                    match in_sa2sin(&data[IFRA_DSTADDR..IFRA_DSTADDR + SIN_SIZE]) {
                        Ok(s) => dstsin = Some(s),
                        Err(e) => break 'out Err(e),
                    }
                }
                // `ifra_broadaddr` is `ifra_dstaddr`.
                if ifp.if_flags.get() & IFF_BROADCAST != 0 && dst_family == AF_INET {
                    match in_sa2sin(&data[IFRA_DSTADDR..IFRA_DSTADDR + SIN_SIZE]) {
                        Ok(s) => broadsin = Some(s),
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
                        if newifaddr || s.sin_addr.s_addr != ia.ia_addr.addr().s_addr {
                            needinit = true;
                        }
                        s
                    }
                };
                if let Some(m) = masksin {
                    hal::netlock::lend_detached(|net| in_ifscrub(ifp, ia, net));
                    ia.ia_netmask.set(m);
                    let mut sm = ia.ia_sockmask.get();
                    sm.sin_addr.s_addr = m;
                    ia.ia_sockmask.set(sm);
                    needinit = true;
                }
                if let Some(d) = dstsin {
                    hal::netlock::lend_detached(|net| in_ifscrub(ifp, ia, net));
                    ia.ia_dstaddr.set(d);
                    needinit = true;
                }
                if let Some(b) = broadsin {
                    if newifaddr {
                        ia.ia_broadaddr().set(b);
                    } else {
                        ifa_update_broadaddr(ia, &b);
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
                hal::netlock::lend_detached(|net| in_purgeaddr(&ia.ia_ifa, net));
                if_addrhooks_run(ifp);
                Ok(())
            }

            _ => kpanic(format_args!("in_ioctl_change_ifaddr: invalid ioctl {cmd}")),
        }
    };

    hal::klock::kernel_unlock();
    hal::netlock::unlock();
    error
}

/// `in_ioctl_get(cmd, data, ifp)`: the address `ioctl`s that read: `SIOCGIFADDR`,
/// `SIOCGIFNETMASK`, `SIOCGIFDSTADDR`, `SIOCGIFBRDADDR` (`data` a `struct ifreq`).
fn in_ioctl_get(cmd: u64, data: &mut [u8], ifp: &'static Ifnet) -> Result<(), Errno> {
    let mut sin = None;
    if data[IFR_ADDR + 1] == AF_INET {
        data[IFR_ADDR] = SIN_SIZE as u8;
        sin = Some(in_sa2sin(&data[IFR_ADDR..IFR_ADDR + SIN_SIZE])?);
    }

    let _shared = NETLOCK.read();

    let Some(ia) = first_or_exact(ifp, sin.map(|s| s.sin_addr)) else {
        return Err(Errno::EADDRNOTAVAIL);
    };

    // `*satosin(&ifr->ifr_addr) = ..`, `*satosin(&ifr->ifr_dstaddr) = ..`: the union.
    let v = match cmd {
        SIOCGIFADDR => ia.ia_addr.get(),
        SIOCGIFBRDADDR => {
            if ifp.if_flags.get() & IFF_BROADCAST == 0 {
                return Err(Errno::EINVAL);
            }
            ia.ia_broadaddr().get()
        }
        SIOCGIFDSTADDR => {
            if ifp.if_flags.get() & IFF_POINTOPOINT == 0 {
                return Err(Errno::EINVAL);
            }
            ia.ia_dstaddr.get()
        }
        SIOCGIFNETMASK => ia.ia_sockmask.get(),
        _ => kpanic(format_args!("in_ioctl_get: invalid ioctl {cmd}")),
    };
    let _ = pod::put(data, IFR_ADDR, &v);
    Ok(())
}

/// `in_ifscrub(ifp, ia)`: deletes any existing route for an interface.
pub fn in_ifscrub(ifp: &Ifnet, ia: &'static InIfaddr, net: &NetHeld<'_>) {
    if ifp.if_flags.get() & IFF_POINTOPOINT != 0 {
        let _ = in_scrubhost(ia, &ia.ia_dstaddr.get(), net);
    } else if ifp.if_flags.get() & IFF_LOOPBACK == 0 {
        in_remove_prefix(ia, net);
    }
}

/// `in_ifinit(ifp, ia, sin, newaddr)`: initializes an interface's internet address `sin` and
/// routing table entry. The caller holds the net lock detached (`in_ioctl_*`): the driver's
/// `SIOCSIFADDR` may let it go and take it again, so the proof is borrowed around it only.
pub fn in_ifinit(
    ifp: &'static Ifnet,
    ia: &'static InIfaddr,
    sin: &SockaddrIn,
    newaddr: bool,
) -> Result<(), Errno> {
    let i = sin.sin_addr.s_addr;

    net_assert_locked();

    // Always remove the address from the tree to make sure its position gets updated in case
    // the key changes.
    if !newaddr {
        hal::netlock::lend_detached(|net| {
            let _ = rt_ifa_dellocal(&ia.ia_ifa, net);
        });
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
    let mut error = Ok(());
    let mut req = [0u8; IFREQ_SIZE];
    if let Err(e) = if_ioctl(ifp, SIOCSIFADDR, &mut req) {
        ia.ia_addr.set(oldaddr);
        error = Err(e);
    }

    hal::netlock::lend_detached(|net| {
        'out: {
            // Add the address to the local list and the global tree. If an error occurred,
            // put back the original address.
            ifa_add(ifp, &ia.ia_ifa);
            if let Err(e) = rt_ifa_addlocal(&ia.ia_ifa, net) {
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
                b.sin_addr.s_addr = if in_rfc3021_subnet(ia.ia_netmask.get()) {
                    0
                } else {
                    ia.ia_net.get() | !ia.ia_netmask.get()
                };
                ia.ia_broadaddr().set(b);
            }

            if ifp.if_flags.get() & IFF_POINTOPOINT != 0 {
                // XXX We should not even call in_ifinit() in this case.
                if ia.ia_dstaddr.family() != AF_INET {
                    break 'out;
                }
                error = in_addhost(ia, &ia.ia_dstaddr.get(), net);
            } else if ifp.if_flags.get() & IFF_LOOPBACK == 0 {
                error = in_insert_prefix(ia, net);
            }

            // If the interface supports multicast, join the "all hosts" multicast group on
            // that interface.
            if ifp.if_flags.get() & IFF_MULTICAST != 0 && ia.ia_allhosts.get().is_none() {
                let addr = InAddr {
                    s_addr: INADDR_ALLHOSTS_GROUP,
                };
                ia.ia_allhosts.set(in_addmulti(&addr, ifp, net));
            }
        }

        // out:
        if error.is_err() && newaddr {
            in_purgeaddr(&ia.ia_ifa, net);
        }
    });

    error
}

/// `in_purgeaddr(ifa)`: an Internet address removed from its interface and the routing
/// table, its "all hosts" membership left, its interface's reference given back.
pub fn in_purgeaddr(ifa: &'static Ifaddr, net: &NetHeld<'_>) {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!("in_purgeaddr: address without interface"));
    };
    let ia = crate::netinet::in_var::ifatoia(ifa);

    net_assert_locked();

    in_ifscrub(ifp, ia, net);

    let _ = rt_ifa_dellocal(&ia.ia_ifa, net);
    rt_ifa_purge(&ia.ia_ifa, net);
    ifa_del(ifp, &ia.ia_ifa);

    if let Some(inm) = ia.ia_allhosts.get() {
        in_delmulti(inm, net);
        ia.ia_allhosts.set(None);
    }

    ia.ia_ifp().set(None);
    ifafree(&ia.ia_ifa);
}

/// The routing domain of `ia`'s interface (`ia->ia_ifa.ifa_ifp->if_rdomain`).
#[inline]
fn rdomain_of(ia: &InIfaddr) -> u32 {
    ia.ia_ifp().get().map_or(0, |ifp| ifp.if_rdomain.get())
}

/// `in_addhost(ia, dst)`: the host route of `dst` through `ia`.
fn in_addhost(ia: &'static InIfaddr, dst: &SockaddrIn, net: &NetHeld<'_>) -> Result<(), Errno> {
    let b = sin_bytes(dst);
    rt_ifa_add(
        &ia.ia_ifa,
        RTF_HOST | RTF_MPATH,
        sin_view(&b),
        rdomain_of(ia),
        net,
    )
}

/// `in_scrubhost(ia, dst)`: the host route of `dst` through `ia` removed.
fn in_scrubhost(ia: &'static InIfaddr, dst: &SockaddrIn, net: &NetHeld<'_>) -> Result<(), Errno> {
    let b = sin_bytes(dst);
    rt_ifa_del(&ia.ia_ifa, RTF_HOST, sin_view(&b), rdomain_of(ia), net)
}

/// `in_insert_prefix(ia)`: inserts the cloning and broadcast routes for this subnet.
fn in_insert_prefix(ia: &'static InIfaddr, net: &NetHeld<'_>) -> Result<(), Errno> {
    let ifa = &ia.ia_ifa;

    // ifa->ifa_addr and ifa->ifa_broadaddr: the address's own storage, copied.
    let a = sin_bytes(&ia.ia_addr.get());
    rt_ifa_add(
        ifa,
        RTF_CLONING | RTF_CONNECTED | RTF_MPATH,
        sin_view(&a),
        rdomain_of(ia),
        net,
    )?;

    let brd = ia.ia_broadaddr().get();
    if brd.sin_addr.s_addr != 0 {
        let b = sin_bytes(&brd);
        return rt_ifa_add(
            ifa,
            RTF_HOST | RTF_BROADCAST | RTF_MPATH,
            sin_view(&b),
            rdomain_of(ia),
            net,
        );
    }

    Ok(())
}

/// `in_remove_prefix(ia)`: removes the cloning and broadcast routes for this subnet.
fn in_remove_prefix(ia: &'static InIfaddr, net: &NetHeld<'_>) {
    let ifa = &ia.ia_ifa;

    let a = sin_bytes(&ia.ia_addr.get());
    let _ = rt_ifa_del(
        ifa,
        RTF_CLONING | RTF_CONNECTED,
        sin_view(&a),
        rdomain_of(ia),
        net,
    );

    let brd = ia.ia_broadaddr().get();
    if brd.sin_addr.s_addr != 0 {
        let b = sin_bytes(&brd);
        let _ = rt_ifa_del(
            ifa,
            RTF_HOST | RTF_BROADCAST,
            sin_view(&b),
            rdomain_of(ia),
            net,
        );
    }
}

/// `in_broadcast(in, rtableid)`: whether `in_` is a local broadcast address in the routing
/// domain of table `rtableid`: the broadcast address of an Internet address of a broadcast
/// interface there, and not that address itself.
pub fn in_broadcast(in_: InAddr, rtableid: u32) -> bool {
    net_assert_locked();

    let rdomain = rtable_l2(rtableid);

    for ifn in &IFNETLIST {
        if ifn.if_rdomain.get() != rdomain {
            continue;
        }
        if ifn.if_flags.get() & IFF_BROADCAST == 0 {
            continue;
        }
        for ifa in &ifn.if_addrlist {
            if let Some(ia) = InIfaddr::of(ifa)
                && in_.s_addr != ia.ia_addr.addr().s_addr
                && in_.s_addr == ia.ia_broadaddr().addr().s_addr
            {
                return true;
            }
        }
    }
    false
}

/// `in_lookupmulti(addr, ifp)`: the multicast record of `addr` on `ifp`, if joined. Called
/// holding `if_maddrlock`.
pub fn in_lookupmulti(addr: &InAddr, ifp: &Ifnet) -> Option<&'static InMulti> {
    kassert!(matches!(ifp.rt().if_maddrlock.status(), RW_WRITE | RW_READ));

    ifp.if_maddrlist
        .iter()
        .filter_map(InMulti::of)
        .find(|inm| inm.inm_addr().s_addr == addr.s_addr)
}

/// The image of the `struct ifreq` `in_addmulti` and `in_delmulti` hand the driver: its
/// `ifr_addr` the `sockaddr_in` of `addr`.
fn multi_ifreq(addr: InAddr) -> [u8; IFREQ_SIZE] {
    let mut ifr = [0u8; IFREQ_SIZE];
    let _ = pod::put(&mut ifr, IFR_ADDR, &sin_of(addr));
    ifr
}

/// `in_addmulti(addr, ifp)`: `addr` added to the IP multicast addresses of `ifp` (or a
/// reference taken on its record if it is there); `None` when the driver refuses. The first
/// IGMP report of a new group is sent under the net lock the caller holds (`net`).
pub fn in_addmulti(
    addr: &InAddr,
    ifp: &'static Ifnet,
    net: &NetHeld<'_>,
) -> Option<&'static InMulti> {
    let lock = &ifp.rt().if_maddrlock;

    // See if address already in list.
    let _ = lock.enter(RW_WRITE);
    if let Some(inm) = in_lookupmulti(addr, ifp) {
        inm.inm_refcnt().take();
        lock.exit();
        return Some(inm);
    }
    lock.exit();

    // New address; allocate a new multicast record and link it into the interface's
    // multicast list.
    let new_inm = in_multi_alloc();

    // Ask the network driver to update its multicast reception filter appropriately for the
    // new address.
    let mut ifr = multi_ifreq(*addr);
    hal::klock::kernel_lock();
    if if_ioctl(ifp, SIOCADDMULTI, &mut ifr).is_err() {
        hal::klock::kernel_unlock();
        in_multi_free(new_inm);
        return None;
    }
    hal::klock::kernel_unlock();

    let _ = lock.enter(RW_WRITE);
    // check again after unlock and lock
    if let Some(inm) = in_lookupmulti(addr, ifp) {
        inm.inm_refcnt().take();
        lock.exit();
        in_multi_free(new_inm);
        return Some(inm);
    }
    let inm = new_inm;
    inm.inm_sin.set(sin_of(*addr));
    inm.inm_refcnt().init();
    inm.inm_ifidx().set(ifp.if_index.get());

    // Let IGMP know that we have joined a new IP multicast group.
    ifp.if_maddrlist.insert_head(&inm.inm_ifma);
    let mut pkt = IgmpPktinfo::default();
    igmp_joingroup(inm, ifp, &mut pkt);
    lock.exit();

    if pkt.ipi_ifidx != 0 {
        igmp_sendpkt(&pkt, net);
    }
    Some(inm)
}

/// `in_delmulti(inm)`: a reference to the multicast record given back; with the last, IGMP
/// leaves the group, the record leaves its interface and the driver its filter. Under the net
/// lock the caller holds (`net`, for the leave message).
pub fn in_delmulti(inm: &'static InMulti, net: &NetHeld<'_>) {
    if !inm.inm_refcnt().rele() {
        return;
    }

    if let Some(ifp) = IfRef::get(inm.inm_ifidx().get()) {
        let lock = &ifp.rt().if_maddrlock;
        let _ = lock.enter(RW_WRITE);
        // No remaining claims to this record; let IGMP know that we are leaving the multicast
        // group.
        let mut pkt = IgmpPktinfo::default();
        igmp_leavegroup(inm, &ifp, &mut pkt);
        ifp.if_maddrlist.remove(&inm.inm_ifma);
        lock.exit();

        if pkt.ipi_ifidx != 0 {
            igmp_sendpkt(&pkt, net);
        }

        // Notify the network driver to update its multicast reception filter.
        let mut ifr = multi_ifreq(inm.inm_addr());
        hal::klock::kernel_lock();
        let _ = if_ioctl(ifp.ifp(), SIOCDELMULTI, &mut ifr);
        hal::klock::kernel_unlock();
        // if_put(ifp): the reference's drop.
    }

    // free(inm): a record still on a gone interface's list is not reused (the module's
    // deviations).
    if !hal::list::List::<crate::net::if_var::IfMaddrlist>::is_linked(&inm.inm_ifma) {
        in_multi_free(inm);
    }
}

/// `in_hasmulti(addr, ifp)`: whether the multicast group `addr` has been joined by interface
/// `ifp`.
pub fn in_hasmulti(addr: &InAddr, ifp: &Ifnet) -> bool {
    let lock = &ifp.rt().if_maddrlock;
    let _ = lock.enter(RW_READ);
    let joined = in_lookupmulti(addr, ifp).is_some();
    lock.exit();

    joined
}

/// `in_ifdetach(ifp)`: every IPv4 address of `ifp` removed.
pub fn in_ifdetach(ifp: &'static Ifnet, net: &NetHeld<'_>) {
    // nuke any of IPv4 addresses we have
    for ifa in &ifp.if_addrlist {
        if InIfaddr::of(ifa).is_none() {
            continue;
        }
        in_purgeaddr(ifa, net);
        if_addrhooks_run(ifp);
    }

    if ifp.if_xflags.get() & IFXF_AUTOCONF4 != 0 {
        ifp.if_xflags.set(ifp.if_xflags.get() & !IFXF_AUTOCONF4);
    }
}

/// `in_prefixlen2mask(maskp, plen)`: the mask of prefix length `plen`.
pub fn in_prefixlen2mask(maskp: &mut InAddr, plen: i32) {
    maskp.s_addr = if plen == 0 {
        0
    } else {
        htonl(0xffff_ffffu32 << (32 - plen))
    };
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the Internet addresses over a test interface: the address ioctls as the
    // C answers them (privilege, family, length, absent addresses), the routes and the
    // all-hosts membership an address brings and its delete takes away, the record recycled,
    // and the multicast records' counts with IGMP's report and leave message on the wire.

    use std::boxed::Box;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::vec;
    use std::vec::Vec;

    use hal::sys::net_if::IFF_UP;
    use hal::sys::net_route::RTF_LOCAL;
    use hal::sys::netinet_in::{INADDR_ALLROUTERS_GROUP, IPPROTO_IGMP};

    use super::*;
    use crate::net::if_var::NetDriver;
    use crate::net::mbuf::{MbufBox, m_copydata};
    use crate::net::route::rtalloc;
    use crate::net::route::tests::{grace, sin, testroutes_setup};
    use crate::netinet::in_var::free_counts;

    /// A driver that keeps the packets it is given and counts the multicast filter changes.
    struct InDriver {
        sent: Mutex<Vec<Vec<u8>>>,
        addmulti: AtomicU32,
        delmulti: AtomicU32,
    }

    impl NetDriver for InDriver {
        fn ioctl(&self, ifp: &'static Ifnet, cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            match cmd {
                SIOCSIFADDR => ifp.if_flags.set(ifp.if_flags.get() | IFF_UP),
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
            m: MbufBox,
            _dst: SockaddrView<'_>,
            _rt: Option<&crate::net::route::Rtentry>,
        ) -> Result<(), Errno> {
            let mut v = vec![0u8; m.m_pkthdr().len.get() as usize];
            m_copydata(&m, 0, &mut v);
            self.sent.lock().expect("sent").push(v);
            Ok(())
        }
    }

    /// The routing fixture and the IP ids' permutation (for the reports `ip_output` sends).
    fn setup() -> std::sync::MutexGuard<'static, ()> {
        static IDS: std::sync::Once = std::sync::Once::new();
        let g = testroutes_setup();
        IDS.call_once(crate::netinet::ip_id::ip_randomid_init);
        g
    }

    /// A broadcast, multicast interface of MTU 1500 with an `InDriver`.
    fn iface() -> (&'static Ifnet, &'static InDriver) {
        let d: &'static InDriver = Box::leak(Box::new(InDriver {
            sent: Mutex::new(Vec::new()),
            addmulti: AtomicU32::new(0),
            delmulti: AtomicU32::new(0),
        }));
        let ifp = crate::net::if_::test_ifnet_with(b"int0", d);
        ifp.if_flags.set(IFF_BROADCAST | IFF_MULTICAST);
        ifp.if_mtu.set(1500);
        crate::net::if_::if_alloc_sadl(ifp);
        {
            let _w = NETLOCK.write();
            IFNETLIST.insert_tail(ifp);
        }
        (ifp, d)
    }

    fn a(b: [u8; 4]) -> InAddr {
        InAddr {
            s_addr: u32::from_ne_bytes(b),
        }
    }

    /// A `struct in_aliasreq` of `addr`/`mask`.
    fn aliasreq(addr: [u8; 4], mask: Option<[u8; 4]>) -> [u8; 64] {
        let mut r = [0u8; 64];
        let _ = pod::put(&mut r, IFRA_ADDR, &sin_of(a(addr)));
        if let Some(m) = mask {
            let _ = pod::put(&mut r, IFRA_MASK, &sin_of(a(m)));
        }
        r
    }

    /// A `struct ifreq` whose `ifr_addr` is `sa`.
    fn ifreq(sa: Option<SockaddrIn>) -> [u8; IFREQ_SIZE] {
        let mut r = [0u8; IFREQ_SIZE];
        if let Some(sa) = sa {
            let _ = pod::put(&mut r, IFR_ADDR, &sa);
        }
        r
    }

    /// `SIOCGIF*`'s answer: the `sockaddr_in` it wrote into the request.
    fn get(ifp: &'static Ifnet, cmd: u64) -> Result<SockaddrIn, Errno> {
        let mut r = ifreq(None);
        in_ioctl(cmd, &mut r, Some(ifp), false)?;
        Ok(pod::at::<SockaddrIn>(&r, IFR_ADDR).expect("an address"))
    }

    /// Whether table 0 has a route to `addr` and its flags.
    fn route(addr: [u8; 4]) -> Option<u32> {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let d = sin(addr);
        rtalloc(SockaddrView::new(&d).expect("a view"), 0, 0, &net).map(|rt| rt.rt_flags.get())
    }

    #[test]
    fn the_address_ioctls_answer_as_the_c() {
        let _g = setup();
        let (ifp, _d) = iface();

        // The refusals before any lock: no interface, no privilege, an unknown request.
        let mut r = aliasreq([10, 0, 2, 15], Some([255, 255, 255, 0]));
        assert_eq!(in_ioctl(SIOCAIFADDR, &mut r, None, true), Err(Errno::ENXIO));
        for cmd in [
            SIOCSIFADDR,
            SIOCAIFADDR,
            SIOCDIFADDR,
            SIOCSIFNETMASK,
            SIOCSIFDSTADDR,
        ] {
            let mut r = [0u8; 64];
            assert_eq!(in_ioctl(cmd, &mut r, Some(ifp), false), Err(Errno::EPERM));
        }
        let mut r = [0u8; 64];
        assert_eq!(
            in_ioctl(SIOCADDMULTI, &mut r, Some(ifp), true),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(get(ifp, SIOCGIFADDR), Err(Errno::EADDRNOTAVAIL));

        // An address of the wrong length is refused before anything changes.
        let mut bad = aliasreq([10, 0, 2, 15], None);
        bad[IFRA_ADDR] = 15;
        assert_eq!(
            in_ioctl(SIOCAIFADDR, &mut bad, Some(ifp), true),
            Err(Errno::EINVAL)
        );

        // ifconfig int0 inet 10.0.2.15/24.
        let (ia_free, _) = free_counts();
        in_ioctl(SIOCAIFADDR, &mut r_add(), Some(ifp), true).expect("SIOCAIFADDR");
        assert_eq!(
            get(ifp, SIOCGIFADDR).map(|s| s.sin_addr),
            Ok(a([10, 0, 2, 15]))
        );
        let mask = get(ifp, SIOCGIFNETMASK).expect("the mask");
        assert_eq!(mask.sin_addr, a([255, 255, 255, 0]));
        assert_eq!(mask.sin_len, 7, "in_socktrim: up to its last non-zero byte");
        assert_eq!(
            get(ifp, SIOCGIFBRDADDR).map(|s| s.sin_addr),
            Ok(a([10, 0, 2, 255]))
        );
        assert_eq!(
            get(ifp, SIOCGIFDSTADDR),
            Err(Errno::EINVAL),
            "not point-to-point"
        );
        let ia = in_ifp2ia_locked(ifp).expect("the record");
        assert_eq!(ia.ia_net.get(), u32::from_ne_bytes([10, 0, 2, 0]));
        assert!(ifp.if_flags.get() & IFF_UP != 0, "the driver's SIOCSIFADDR");

        // Its routes: the local host, the subnet, the broadcast address; the all-hosts group.
        assert!(route([10, 0, 2, 15]).is_some_and(|f| f & RTF_LOCAL != 0));
        assert!(route([10, 0, 2, 77]).is_some());
        assert!(route([10, 0, 2, 255]).is_some_and(|f| f & RTF_BROADCAST != 0));
        {
            let _w = NETLOCK.read();
            assert!(in_broadcast(a([10, 0, 2, 255]), 0));
            assert!(!in_broadcast(a([10, 0, 2, 15]), 0));
        }
        assert!(in_hasmulti(&a([224, 0, 0, 1]), ifp));

        // A non-contiguous mask, an absent address to delete.
        let mut m = ifreq(Some(sin_of(a([255, 0, 255, 0]))));
        assert_eq!(
            in_ioctl(SIOCSIFNETMASK, &mut m, Some(ifp), true),
            Err(Errno::EINVAL)
        );
        let mut gone = aliasreq([10, 9, 9, 9], None);
        assert_eq!(
            in_ioctl(SIOCDIFADDR, &mut gone, Some(ifp), true),
            Err(Errno::EADDRNOTAVAIL)
        );

        // ifconfig int0 inet 10.0.2.15 delete: the routes and the membership gone, the record
        // back on the free list once the routes gave back their references.
        let mut del = aliasreq([10, 0, 2, 15], None);
        in_ioctl(SIOCDIFADDR, &mut del, Some(ifp), true).expect("SIOCDIFADDR");
        grace();
        assert!(route([10, 0, 2, 77]).is_none());
        assert!(!in_hasmulti(&a([224, 0, 0, 1]), ifp));
        assert_eq!(get(ifp, SIOCGIFADDR), Err(Errno::EADDRNOTAVAIL));
        assert_eq!(
            free_counts().0,
            ia_free + 1,
            "the record recycled, not freed"
        );

        // The next address takes it again.
        in_ioctl(SIOCAIFADDR, &mut r_add(), Some(ifp), true).expect("again");
        assert_eq!(free_counts().0, ia_free);
        assert!(core::ptr::eq(
            in_ifp2ia_locked(ifp).expect("the record"),
            ia
        ));
        let mut del = aliasreq([10, 0, 2, 15], None);
        in_ioctl(SIOCDIFADDR, &mut del, Some(ifp), true).expect("SIOCDIFADDR");
        grace();
    }

    fn r_add() -> [u8; 64] {
        aliasreq([10, 0, 2, 15], Some([255, 255, 255, 0]))
    }

    fn in_ifp2ia_locked(ifp: &Ifnet) -> Option<&'static InIfaddr> {
        let _w = NETLOCK.read();
        in_ifp2ia(ifp)
    }

    /// The IGMP messages `d` was given: (IP destination, IGMP type, group), each with the
    /// Router Alert option and TTL 1.
    fn igmp_sent(d: &InDriver) -> Vec<([u8; 4], u8, [u8; 4])> {
        let mut v = Vec::new();
        for ip in d.sent.lock().expect("sent").drain(..) {
            assert_eq!(ip[0], 0x46, "a 24-byte header: the Router Alert option");
            assert_eq!(ip[8], 1, "TTL 1");
            assert_eq!(i32::from(ip[9]), IPPROTO_IGMP);
            assert_eq!(&ip[20..24], &[0x94, 0x04, 0, 0]);
            v.push((
                [ip[16], ip[17], ip[18], ip[19]],
                ip[24],
                [ip[28], ip[29], ip[30], ip[31]],
            ));
        }
        v
    }

    #[test]
    fn memberships_are_counted_reported_and_left() {
        let _g = setup();
        let (ifp, d) = iface();
        in_ioctl(SIOCAIFADDR, &mut r_add(), Some(ifp), true).expect("SIOCAIFADDR");
        let _ = igmp_sent(d);
        let adds = d.addmulti.load(Ordering::Relaxed);

        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let g = a([239, 1, 2, 3]);
        let inm = in_addmulti(&g, ifp, &net).expect("joined");
        let again = in_addmulti(&g, ifp, &net).expect("joined again");
        assert!(core::ptr::eq(inm, again), "one record for the group");
        assert_eq!(inm.inm_refcnt().read(), 2);
        assert_eq!(
            d.addmulti.load(Ordering::Relaxed),
            adds + 1,
            "one filter change"
        );
        assert_eq!(
            igmp_sent(d),
            [(
                [239, 1, 2, 3],
                hal::sys::netinet_igmp::IGMP_v2_HOST_MEMBERSHIP_REPORT,
                [239, 1, 2, 3]
            )],
            "the first report at once"
        );

        let (_, inm_free) = free_counts();
        in_delmulti(inm, &net);
        assert!(in_hasmulti(&g, ifp), "one claim left");
        assert!(igmp_sent(d).is_empty());
        in_delmulti(inm, &net);
        assert!(!in_hasmulti(&g, ifp));
        let all_routers = INADDR_ALLROUTERS_GROUP.to_ne_bytes();
        assert_eq!(
            igmp_sent(d),
            [(
                all_routers,
                hal::sys::netinet_igmp::IGMP_HOST_LEAVE_MESSAGE,
                all_routers
            )],
            "we may have reported last: a leave message"
        );
        assert_eq!(d.delmulti.load(Ordering::Relaxed), 1);
        assert_eq!(free_counts().1, inm_free + 1);
        drop(net);
        drop(w);

        let mut del = aliasreq([10, 0, 2, 15], None);
        in_ioctl(SIOCDIFADDR, &mut del, Some(ifp), true).expect("SIOCDIFADDR");
        grace();
    }

    #[test]
    fn the_helpers_answer_as_the_c() {
        let mut m = InAddr::default();
        in_len2mask(&mut m, 20);
        assert_eq!(m, a([255, 255, 240, 0]));
        assert_eq!(in_mask2len(&m), 20);
        in_prefixlen2mask(&mut m, 0);
        assert_eq!(m.s_addr, 0);
        in_prefixlen2mask(&mut m, 24);
        assert_eq!(m, a([255, 255, 255, 0]));
        assert!(in_canforward(a([10, 0, 2, 2])));
        assert!(!in_canforward(a([127, 0, 0, 1])));
        assert!(!in_canforward(a([0, 1, 2, 3])));
        assert!(!in_canforward(a([224, 0, 0, 1])));
        let mut s = sin_of(a([255, 255, 0, 0]));
        in_socktrim(&mut s);
        assert_eq!(s.sin_len, 6);
        assert_eq!(
            in_sa2sin(&[16, AF_INET, 0, 0, 1, 2, 3, 4]).map(|s| s.sin_len),
            Err(Errno::EINVAL)
        );
        let good = sin_bytes(&sin_of(a([1, 2, 3, 4])));
        assert_eq!(in_sa2sin(&good).map(|s| s.sin_addr), Ok(a([1, 2, 3, 4])));
        let mut other = good;
        other[1] = 24;
        assert_eq!(in_sa2sin(&other), Err(Errno::EAFNOSUPPORT));
    }
}
/* </TESTS> */
