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
//! constants and structures of `<netinet/in.h>` are `sysdefs::netinet_in`; the records are
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
//!
//! ## Redesign
//! - The address `ioctl`s over the request's bytes: LZ's `unsafe fn`s over `caddr_t data`
//!   (with an aligned copy of an unaligned request) -> [`in_ioctl`] over `&mut [u8]`, the
//!   `struct ifreq`'s and `struct in_aliasreq`'s addresses read and written by copies at their
//!   offsets (`hal::mem::pod::at`/`put`, the C's `*sin` copies and `*satosin(..) = ia->ia_addr`
//!   stores); `in_sa2sin` and `in_nam2sin` answer the checked `sockaddr_in` by value.
//! - `NET_LOCK()` stays detached (`hal::sync::netlock::lock`) for the requests that change an
//!   address, since the driver's `SIOCSIFADDR` may let it go and take it again (wg(4)'s
//!   `wg_up`, D-14); the steps that need its proof (the routes) borrow it in closures
//!   (`hal::sync::netlock::lend_detached`), never across the driver's two requests. The kernel lock
//!   is taken C-style around the same span, in the C's order (net, then kernel).
//!   `NET_LOCK_SHARED()` of `in_ioctl_get` is a witness (no driver call).
//! - (N6.5e's 5e2) A multicast record is a counted handle (`hal::mem::malloc::MArc` of
//!   `kernel::netinet::in_var::InMulti`), freed with its last one where the C frees it: the
//!   interface's list holds one ([`in_addmulti`] links it, [`in_delmulti`] at the last leave
//!   and `if_detach` take it off), each membership one: `in_addmulti` answers the joiner's
//!   (a new record's, or a clone of the list's for a group already joined, beside the C's
//!   `refcnt_take`), `in_delmulti` consumes it. `inm_refcnt` counts the memberships as the C's
//!   (D-8). A membership whose interface has gone when its last reference goes finds no
//!   interface and frees the record with its handle, as the C (the list's handle went with
//!   `if_detach`, D-9): the "stays linked, not reused" deviation is closed.
//! - (N6.5e) An address is a counted handle (`kernel::net::if_var::IfaddrRef`), freed with its
//!   last one: `in_ifaddr_alloc` answers the creation reference, which `in_ifinit` gives to
//!   `ifa_add` for a new address, and `in_purgeaddr` gives back the list's (`ifa_del`'s) with
//!   `ifafree`. The walks and lookups of the interface's list (`in_ifp2ia`, `first_or_exact`,
//!   `in_broadcast`, the change's) lend from a reader of the net cells, counting nothing. The
//!   requests that change an address hold the net lock detached across the driver's
//!   `SIOCSIFADDR`, which may let it go: they clone the address they found before the
//!   reader ends (T1) and keep it for the request, where the C keeps an uncounted pointer
//!   (which the lock let go would not keep alive). `in_ifinit` of an existing address takes the
//!   list's reference off the list (`ifa_del`) and puts it back (`ifa_add`), the C's
//!   re-insertion at the tail.
//! - `in_broadcast`, `in_ifp2ia`, `in_lookupmulti` read the records' own words (`InIfaddr::of`
//!   of each `AF_INET` address, `SinWords::addr`), no copy of the generic part's address.
//!   `in_broadcast` takes the proof of the net lock its walk needs (the C asserts it).
//! - (N6.5e) `if_maddrlock` is taken through its witnesses (`kernel::net::if_var`'s
//!   redesign): [`in_lookupmulti`] lends the record from the caller's witness in either mode
//!   (the list's cells are keyed to the lock: a witness of another lock is `panic(9)`, the
//!   C's `rw_assert_anylock`), counting nothing, as the C's pointer; `in_addmulti` and
//!   `in_delmulti` hand theirs to IGMP's join and leave.
//!
//! **Lock and spl transitions** against the C: `in_ioctl`'s changes take the net lock and the
//! kernel lock once each, as the C; `in_ioctl_get` the net lock shared; `in_addmulti` and
//! `in_delmulti` `if_maddrlock` and the kernel lock as the C (twice and once, once and once);
//! `in_hasmulti` `if_maddrlock` read once. No spl change: the C has none.

use core::mem::{offset_of, size_of};

use hal::mem::malloc::MArc;
use hal::mem::pod;
use hal::sync::netlock::{NETLOCK, NetHeld, NetReader};
use hal::sync::rwlock::{RW_READ, RW_WRITE, Witness};
use sysdefs::endian::{htonl, ntohl};
use sysdefs::errno::Errno;
use sysdefs::ioccom::iocparm_len;
use sysdefs::net_if::{
    IFF_BROADCAST, IFF_LOOPBACK, IFF_MULTICAST, IFF_POINTOPOINT, IFNAMSIZ, IFXF_AUTOCONF4,
};
use sysdefs::net_route::{RTF_BROADCAST, RTF_CLONING, RTF_CONNECTED, RTF_HOST, RTF_MPATH};
use sysdefs::netinet_in::{
    IN_CLASSA_NET, IN_CLASSA_NSHIFT, IN_CLASSB_NET, IN_CLASSC_NET, IN_LOOPBACKNET,
    INADDR_ALLHOSTS_GROUP, INADDR_ANY, InAddr, SockaddrIn, in_classa, in_classb, in_multicast,
    in_rfc3021_subnet,
};
use sysdefs::socket::AF_INET;
use sysdefs::sockio::{
    SIOCADDMULTI, SIOCAIFADDR, SIOCDELMULTI, SIOCDIFADDR, SIOCGIFADDR, SIOCGIFBRDADDR,
    SIOCGIFDSTADDR, SIOCGIFNETMASK, SIOCSIFADDR, SIOCSIFBRDADDR, SIOCSIFDSTADDR, SIOCSIFNETMASK,
};

use crate::assert::{kassert, kpanic};
use crate::net::if_::{IFNETLIST, IfRef, if_addrhooks_run, if_ioctl, ifa_add, ifa_del};
use crate::net::if_var::{IfaddrRef, IfmaList, IfmaddrRef, Ifnet};
use crate::net::mbuf::Mbuf;
use crate::net::proto::SockaddrView;
use crate::net::route::{
    ifafree, rt_ifa_add, rt_ifa_addlocal, rt_ifa_del, rt_ifa_dellocal, rt_ifa_purge,
};
use crate::net::rtable::rtable_l2;
use crate::netinet::igmp::{igmp_joingroup, igmp_leavegroup, igmp_sendpkt};
use crate::netinet::igmp_var::IgmpPktinfo;
use crate::netinet::in_var::{InIfaddr, InMulti, ifatoia, in_ifaddr_alloc, in_multi_alloc};

/// `sizeof(struct sockaddr_in)`.
const SIN_SIZE: usize = size_of::<SockaddrIn>();
/// `sizeof(struct ifreq)`: the name, then the union.
const IFREQ_SIZE: usize = IFNAMSIZ + 16;
/// `offsetof(struct ifreq, ifr_ifru)`: `ifr_addr`, `ifr_dstaddr`, `ifr_broadaddr`.
const IFR_ADDR: usize = IFNAMSIZ;
/// `offsetof(struct in_aliasreq, ifra_addr)`.
const IFRA_ADDR: usize = offset_of!(sysdefs::netinet_in_var::InAliasreq, ifra_addr);
/// `offsetof(struct in_aliasreq, ifra_dstaddr)` (`ifra_broadaddr`).
const IFRA_DSTADDR: usize = offset_of!(sysdefs::netinet_in_var::InAliasreq, ifra_dstaddr);
/// `offsetof(struct in_aliasreq, ifra_mask)`.
const IFRA_MASK: usize = offset_of!(sysdefs::netinet_in_var::InAliasreq, ifra_mask);
/// `offsetof(struct sockaddr, sa_data)`.
const SA_DATA: usize = 2;

/// `NET_ASSERT_LOCKED()`.
#[inline]
fn net_assert_locked() {
    kassert!(matches!(hal::sync::netlock::status(), RW_WRITE | RW_READ));
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

/// `in_ifp2ia(ifp)`: the first Internet address of interface `ifp`, lent for the life of the
/// net cells' reader `r` (whose making is the C's `NET_ASSERT_LOCKED()`).
pub fn in_ifp2ia<'r>(r: &'r NetReader<'_>, ifp: &'r Ifnet) -> Option<&'r IfaddrRef> {
    ifp.if_addrlist()
        .iter(r)
        .find(|ifa| InIfaddr::of(ifa).is_some())
}

/// The Internet address of `ifp` whose address is `addr`, or the first one when `addr` is
/// none or `INADDR_ANY` (`in_ioctl`'s and `in_ioctl_get`'s walk: "find first address or
/// exact match"), lent for the life of `r`.
fn first_or_exact<'r>(
    r: &'r NetReader<'_>,
    ifp: &'r Ifnet,
    addr: Option<InAddr>,
) -> Option<&'r IfaddrRef> {
    let mut ia = None;
    for ifa in ifp.if_addrlist().iter(r) {
        let Some(this) = InIfaddr::of(ifa) else {
            continue;
        };
        // find first address or exact match
        if ia.is_none() {
            ia = Some(ifa);
        }
        let Some(a) = addr.filter(|a| a.s_addr != INADDR_ANY) else {
            break;
        };
        if this.ia_addr.addr().s_addr == a.s_addr {
            ia = Some(ifa);
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

    hal::sync::netlock::lock();
    hal::sync::klock::kernel_lock();

    let error = 'err: {
        // T1: the address, cloned before the reader ends, kept for the request (the driver's
        // request below may let the detached lock go).
        let found = hal::sync::netlock::lend_detached(|net| {
            first_or_exact(&net.reader(), ifp, sin.map(|s| s.sin_addr)).cloned()
        });
        let Some(ifa) = found else {
            break 'err Err(Errno::EADDRNOTAVAIL);
        };
        let ia = ifatoia(&ifa);

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
                hal::sync::netlock::lend_detached(|net| {
                    let _ = in_scrubhost(&ifa, &oldaddr, net);
                    let _ = in_addhost(&ifa, &ia.ia_dstaddr.get(), net);
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
    hal::sync::klock::kernel_unlock();
    hal::sync::netlock::unlock();
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

    hal::sync::netlock::lock();
    hal::sync::klock::kernel_lock();

    // find first address: an existing one cloned before the reader ends (T1), or a new one,
    // whose creation reference `in_ifinit` gives to `ifa_add` and whose clone the request
    // keeps (T1).
    let found = hal::sync::netlock::lend_detached(|net| in_ifp2ia(&net.reader(), ifp).cloned());
    let (ia, created) = if let Some(ia) = found {
        (ia, None)
    } else {
        let created = IfaddrRef::In(in_ifaddr_alloc(ifp));
        (created.clone(), Some(created))
    };

    hal::sync::netlock::lend_detached(|net| in_ifscrub(ifp, &ia, net));
    let error = in_ifinit(ifp, &ia, &sin, created);
    if error.is_ok() {
        if_addrhooks_run(ifp);
    }

    hal::sync::klock::kernel_unlock();
    hal::sync::netlock::unlock();
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

    hal::sync::netlock::lock();
    hal::sync::klock::kernel_lock();

    // find first address, if no exact match wanted (T1: cloned before the reader ends, kept
    // for the request)
    let ia = hal::sync::netlock::lend_detached(|net| {
        ifp.if_addrlist()
            .iter(&net.reader())
            .find(|ifa| {
                InIfaddr::of(ifa).is_some_and(|ia| {
                    sin.is_none_or(|s| s.sin_addr.s_addr == ia.ia_addr.addr().s_addr)
                })
            })
            .cloned()
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

                // A new address: its creation reference for `in_ifinit`'s `ifa_add`, a clone
                // for the request (T1).
                let (ifa, created) = if let Some(ia) = ia {
                    (ia, None)
                } else {
                    let created = IfaddrRef::In(in_ifaddr_alloc(ifp));
                    (created.clone(), Some(created))
                };
                let newifaddr = created.is_some();
                let ia = ifatoia(&ifa);

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
                    hal::sync::netlock::lend_detached(|net| in_ifscrub(ifp, &ifa, net));
                    ia.ia_netmask.set(m);
                    let mut sm = ia.ia_sockmask.get();
                    sm.sin_addr.s_addr = m;
                    ia.ia_sockmask.set(sm);
                    needinit = true;
                }
                if let Some(d) = dstsin {
                    hal::sync::netlock::lend_detached(|net| in_ifscrub(ifp, &ifa, net));
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
                if needinit {
                    if let Err(e) = in_ifinit(ifp, &ifa, &sin, created) {
                        break 'out Err(e);
                    }
                } else if let Some(c) = created {
                    // A new address with no address, mask or destination given: the C's
                    // `malloc`'d record that no `in_ifinit` links is never freed there; its
                    // creation reference is forgotten, so it stays allocated, as there.
                    core::mem::forget(c);
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
                hal::sync::netlock::lend_detached(|net| in_purgeaddr(&ia, net));
                if_addrhooks_run(ifp);
                Ok(())
            }

            _ => kpanic(format_args!("in_ioctl_change_ifaddr: invalid ioctl {cmd}")),
        }
    };

    hal::sync::klock::kernel_unlock();
    hal::sync::netlock::unlock();
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

    let shared = NETLOCK.read();
    let Some(held) = NetHeld::of(&shared) else {
        kpanic(format_args!("in_ioctl_get: not a witness of the net lock"));
    };
    let r = held.reader();

    let Some(ia) = first_or_exact(&r, ifp, sin.map(|s| s.sin_addr)).map(ifatoia) else {
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
pub fn in_ifscrub(ifp: &Ifnet, ia: &IfaddrRef, net: &NetHeld<'_>) {
    if ifp.if_flags.get() & IFF_POINTOPOINT != 0 {
        let _ = in_scrubhost(ia, &ifatoia(ia).ia_dstaddr.get(), net);
    } else if ifp.if_flags.get() & IFF_LOOPBACK == 0 {
        in_remove_prefix(ia, net);
    }
}

/// `in_ifinit(ifp, ia, sin, newaddr)`: initializes an interface's internet address `sin` and
/// routing table entry. The caller holds the net lock detached (`in_ioctl_*`): the driver's
/// `SIOCSIFADDR` may let it go and take it again, so the proof is borrowed around it only.
///
/// `created` is the creation reference of a new address (the C's `newaddr`), which
/// `ifa_add` takes; `ifa` is the caller's own handle of the address (T1).
pub fn in_ifinit(
    ifp: &'static Ifnet,
    ifa: &IfaddrRef,
    sin: &SockaddrIn,
    created: Option<IfaddrRef>,
) -> Result<(), Errno> {
    let i = sin.sin_addr.s_addr;
    let newaddr = created.is_some();
    let ia = ifatoia(ifa);

    net_assert_locked();

    // Always remove the address from the tree to make sure its position gets updated in case
    // the key changes: an existing address's list reference taken off the list, for the
    // `ifa_add` below.
    let lst = match created {
        Some(c) => c,
        None => hal::sync::netlock::lend_detached(|net| {
            let _ = rt_ifa_dellocal(ifa, net);
            ifa_del(ifp, ifa, net)
        }),
    };
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

    hal::sync::netlock::lend_detached(|net| {
        'out: {
            // Add the address to the local list and the global tree. If an error occurred,
            // put back the original address.
            ifa_add(ifp, lst, net);
            if let Err(e) = rt_ifa_addlocal(ifa, net) {
                if !newaddr {
                    // The C never gives this reference back (`ifa_del` without `ifafree`):
                    // the address stays allocated, off its list, as there.
                    core::mem::forget(ifa_del(ifp, ifa, net));
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
                error = in_addhost(ifa, &ia.ia_dstaddr.get(), net);
            } else if ifp.if_flags.get() & IFF_LOOPBACK == 0 {
                error = in_insert_prefix(ifa, net);
            }

            // If the interface supports multicast, join the "all hosts" multicast group on
            // that interface.
            let no_allhosts = ia.ia_allhosts.read().is_none();
            if ifp.if_flags.get() & IFF_MULTICAST != 0 && no_allhosts {
                let addr = InAddr {
                    s_addr: INADDR_ALLHOSTS_GROUP,
                };
                let inm = in_addmulti(&addr, ifp, net);
                // The cell was empty (read above, under the same exclusive net lock): a
                // handle there would be a membership dropped without its leave.
                let old = ia.ia_allhosts.replace(inm);
                kassert!(old.is_none());
                drop(old);
            }
        }

        // out:
        if error.is_err() && newaddr {
            in_purgeaddr(ifa, net);
        }
    });

    error
}

/// `in_purgeaddr(ifa)`: an Internet address removed from its interface and the routing
/// table, its "all hosts" membership left, its interface's reference given back.
pub fn in_purgeaddr(ifa: &IfaddrRef, net: &NetHeld<'_>) {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!("in_purgeaddr: address without interface"));
    };
    let ia = ifatoia(ifa);

    net_assert_locked();

    in_ifscrub(ifp, ifa, net);

    let _ = rt_ifa_dellocal(ifa, net);
    rt_ifa_purge(ifa, net);
    let lst = ifa_del(ifp, ifa, net);

    // The membership's handle taken out of the cell first: `in_delmulti` consumes it.
    if let Some(inm) = ia.ia_allhosts.replace(None) {
        in_delmulti(inm, net);
    }

    ia.ia_ifp().set(None);
    ifafree(lst);
}

/// The routing domain of `ia`'s interface (`ia->ia_ifa.ifa_ifp->if_rdomain`).
#[inline]
fn rdomain_of(ia: &InIfaddr) -> u32 {
    ia.ia_ifp().get().map_or(0, |ifp| ifp.if_rdomain.get())
}

/// `in_addhost(ia, dst)`: the host route of `dst` through `ia`.
fn in_addhost(ifa: &IfaddrRef, dst: &SockaddrIn, net: &NetHeld<'_>) -> Result<(), Errno> {
    let b = sin_bytes(dst);
    rt_ifa_add(
        ifa,
        RTF_HOST | RTF_MPATH,
        sin_view(&b),
        rdomain_of(ifatoia(ifa)),
        net,
    )
}

/// `in_scrubhost(ia, dst)`: the host route of `dst` through `ia` removed.
fn in_scrubhost(ifa: &IfaddrRef, dst: &SockaddrIn, net: &NetHeld<'_>) -> Result<(), Errno> {
    let b = sin_bytes(dst);
    rt_ifa_del(ifa, RTF_HOST, sin_view(&b), rdomain_of(ifatoia(ifa)), net)
}

/// `in_insert_prefix(ia)`: inserts the cloning and broadcast routes for this subnet.
fn in_insert_prefix(ifa: &IfaddrRef, net: &NetHeld<'_>) -> Result<(), Errno> {
    let ia = ifatoia(ifa);

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
fn in_remove_prefix(ifa: &IfaddrRef, net: &NetHeld<'_>) {
    let ia = ifatoia(ifa);

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
/// interface there, and not that address itself. Under the net lock `net` proves (the C's
/// `NET_ASSERT_LOCKED()`: one reader for the walk).
pub fn in_broadcast(in_: InAddr, rtableid: u32, net: &NetHeld<'_>) -> bool {
    let r = net.reader();
    let rdomain = rtable_l2(rtableid);

    for ifn in &IFNETLIST {
        if ifn.if_rdomain.get() != rdomain {
            continue;
        }
        if ifn.if_flags.get() & IFF_BROADCAST == 0 {
            continue;
        }
        for ifa in ifn.if_addrlist().iter(&r) {
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

/// `in_lookupmulti(addr, ifp)`: the multicast record of `addr` on `ifp`, if joined, lent for
/// the life of `w`, a witness of `ifp`'s `if_maddrlock` in either mode (another lock's is
/// `panic(9)`: `rw_assert_anylock`).
pub fn in_lookupmulti<'w, W: Witness + ?Sized>(
    addr: &InAddr,
    ifp: &'w Ifnet,
    w: &'w W,
) -> Option<&'w MArc<InMulti>> {
    lookupmulti(*addr, ifp.if_maddrlist(), w)
}

/// [`in_lookupmulti`]'s walk of the list `list` (`TAILQ_FOREACH`, O(1) a step), for a caller
/// that has the interface's [`IfRt`](crate::net::if_var::IfRt) at hand already.
#[inline]
fn lookupmulti<'w, W: Witness + ?Sized>(
    addr: InAddr,
    list: &'w IfmaList,
    w: &'w W,
) -> Option<&'w MArc<InMulti>> {
    list.iter(w)
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
/// membership counted on its record if it is there); the joiner's handle of the record,
/// which `in_delmulti` takes back, or `None` when the driver refuses. The first IGMP report
/// of a new group is sent under the net lock the caller holds (`net`).
pub fn in_addmulti(addr: &InAddr, ifp: &'static Ifnet, net: &NetHeld<'_>) -> Option<MArc<InMulti>> {
    let lock = &ifp.rt().if_maddrlock;

    // See if address already in list.
    {
        let w = lock.write();
        if let Some(inm) = in_lookupmulti(addr, ifp, &w) {
            inm.inm_refcnt().take();
            // The joiner's handle (D-8), made before the witness ends.
            return Some(inm.clone());
        }
    }

    // New address; allocate a new multicast record and link it into the interface's
    // multicast list.
    let new_inm = in_multi_alloc(ifp);

    // Ask the network driver to update its multicast reception filter appropriately for the
    // new address.
    let mut ifr = multi_ifreq(*addr);
    hal::sync::klock::kernel_lock();
    if if_ioctl(ifp, SIOCADDMULTI, &mut ifr).is_err() {
        hal::sync::klock::kernel_unlock();
        // free(new_inm): its one handle's drop.
        return None;
    }
    hal::sync::klock::kernel_unlock();

    let mut w = lock.write();
    // check again after unlock and lock
    if let Some(inm) = in_lookupmulti(addr, ifp, &w) {
        inm.inm_refcnt().take();
        let inm = inm.clone(); // the joiner's handle (D-8)
        drop(w);
        // free(new_inm): its one handle's drop, after the lock as the C's.
        drop(new_inm);
        return Some(inm);
    }
    let inm = new_inm;
    inm.inm_sin.set(sin_of(*addr));
    inm.inm_refcnt().init();
    inm.inm_ifidx().set(ifp.if_index.get());

    // Let IGMP know that we have joined a new IP multicast group.
    ifp.if_maddrlist()
        .insert_head(&mut w, IfmaddrRef::In(inm.clone())); // the list's handle
    let mut pkt = IgmpPktinfo::default();
    igmp_joingroup(&inm, ifp, &w, &mut pkt);
    drop(w);

    if pkt.ipi_ifidx != 0 {
        igmp_sendpkt(&pkt, net);
    }
    Some(inm)
}

/// `in_delmulti(inm)`: a membership of the multicast record given back with the joiner's
/// handle `inm`; with the last membership, IGMP leaves the group, the record leaves its
/// interface and the driver its filter, and the record is freed with the last handle (this
/// one and the list's, both dropped at the end, where the C frees it). Under the net lock the
/// caller holds (`net`, for the leave message).
pub fn in_delmulti(inm: MArc<InMulti>, net: &NetHeld<'_>) {
    if !inm.inm_refcnt().rele() {
        return;
    }

    // The list's handle of the record, taken off the list below; none when `if_detach`
    // emptied the list already (the interface is gone, D-9).
    let mut lst = None;
    if let Some(ifp) = IfRef::get(inm.inm_ifidx().get()) {
        let mut w = ifp.rt().if_maddrlock.write();
        // No remaining claims to this record; let IGMP know that we are leaving the multicast
        // group.
        let mut pkt = IgmpPktinfo::default();
        igmp_leavegroup(&inm, &ifp, &w, &mut pkt);
        lst = ifp.if_maddrlist().remove(&mut w, &inm.inm_ifma);
        drop(w);

        if pkt.ipi_ifidx != 0 {
            igmp_sendpkt(&pkt, net);
        }

        // Notify the network driver to update its multicast reception filter.
        let mut ifr = multi_ifreq(inm.inm_addr());
        hal::sync::klock::kernel_lock();
        let _ = if_ioctl(ifp.ifp(), SIOCDELMULTI, &mut ifr);
        hal::sync::klock::kernel_unlock();
        // if_put(ifp): the reference's drop.
    }

    // free(inm, M_IPMADDR, sizeof(*inm)): the last of the two handles frees the record.
    drop(lst);
    drop(inm);
}

/// `in_hasmulti(addr, ifp)`: whether the multicast group `addr` has been joined by interface
/// `ifp`.
pub fn in_hasmulti(addr: &InAddr, ifp: &Ifnet) -> bool {
    let rt = ifp.rt();
    let r = rt.if_maddrlock.read();
    lookupmulti(*addr, rt.if_maddrlist(), &r).is_some()
}

/// `in_ifdetach(ifp)`: every IPv4 address of `ifp` removed.
pub fn in_ifdetach(ifp: &'static Ifnet, net: &NetHeld<'_>) {
    // nuke any of IPv4 addresses we have (TAILQ_FOREACH_SAFE: `in_purgeaddr` takes the
    // address it is given off the list; T1/T2, `IfaList::for_each_safe`)
    ifp.if_addrlist().for_each_safe(net, |ifa| {
        if InIfaddr::of(ifa).is_none() {
            return;
        }
        in_purgeaddr(ifa, net);
        if_addrhooks_run(ifp);
    });

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
    // all-hosts membership an address brings and its delete takes away, the records freed
    // with their last handles, and the multicast records' counts with IGMP's report and leave
    // message on the wire.

    use std::boxed::Box;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::vec;
    use std::vec::Vec;

    use sysdefs::net_if::IFF_UP;
    use sysdefs::net_route::RTF_LOCAL;
    use sysdefs::netinet_in::{INADDR_ALLROUTERS_GROUP, IPPROTO_IGMP};

    use super::*;
    use crate::net::if_::M_IFADDR;
    use crate::net::if_var::NetDriver;
    use crate::net::mbuf::{MbufBox, m_copydata};
    use crate::net::route::rtalloc;
    use crate::net::route::tests::{grace, sin, testroutes_setup};
    use crate::netinet::in_var::M_IPMADDR;
    use crate::testutil::malloc_out;

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

        // ifconfig int0 inet 10.0.2.15/24: one record of `M_IFADDR`, and the all-hosts
        // group's of `M_IPMADDR` (the address's membership, `ia_allhosts`).
        let out = malloc_out(M_IFADDR);
        let mout = malloc_out(M_IPMADDR);
        in_ioctl(SIOCAIFADDR, &mut r_add(), Some(ifp), true).expect("SIOCAIFADDR");
        let one = malloc_out(M_IFADDR) - out;
        assert!(one > 0, "the address's record");
        assert!(malloc_out(M_IPMADDR) > mout, "the all-hosts group's record");
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
        assert_eq!(ifatoia(&ia).ia_net.get(), u32::from_ne_bytes([10, 0, 2, 0]));
        assert!(ifp.if_flags.get() & IFF_UP != 0, "the driver's SIOCSIFADDR");

        // Its routes: the local host, the subnet, the broadcast address; the all-hosts group.
        assert!(route([10, 0, 2, 15]).is_some_and(|f| f & RTF_LOCAL != 0));
        assert!(route([10, 0, 2, 77]).is_some());
        assert!(route([10, 0, 2, 255]).is_some_and(|f| f & RTF_BROADCAST != 0));
        {
            let w = NETLOCK.read();
            let net = NetHeld::of(&w).expect("the net lock");
            assert!(in_broadcast(a([10, 0, 2, 255]), 0, &net));
            assert!(!in_broadcast(a([10, 0, 2, 15]), 0, &net));
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

        // ifconfig int0 inet 10.0.2.15 delete: the routes and the membership gone; the record
        // freed with its last reference, which the test's handle holds until its drop (a
        // reference cannot outlive its record).
        let mut del = aliasreq([10, 0, 2, 15], None);
        in_ioctl(SIOCDIFADDR, &mut del, Some(ifp), true).expect("SIOCDIFADDR");
        grace();
        assert!(route([10, 0, 2, 77]).is_none());
        assert!(!in_hasmulti(&a([224, 0, 0, 1]), ifp));
        assert_eq!(
            malloc_out(M_IPMADDR),
            mout,
            "the all-hosts group's record freed with its last membership"
        );
        assert_eq!(get(ifp, SIOCGIFADDR), Err(Errno::EADDRNOTAVAIL));
        assert_eq!(ia.count(), 1, "the test's handle alone");
        assert_eq!(
            ifatoia(&ia).ia_addr.addr(),
            a([10, 0, 2, 15]),
            "still readable"
        );
        assert_eq!(malloc_out(M_IFADDR), out + one, "kept by the test's handle");
        drop(ia);
        assert_eq!(malloc_out(M_IFADDR), out, "freed with its last reference");

        // The next address is a record of its own, freed again by its delete.
        in_ioctl(SIOCAIFADDR, &mut r_add(), Some(ifp), true).expect("again");
        assert_eq!(malloc_out(M_IFADDR), out + one);
        let mut del = aliasreq([10, 0, 2, 15], None);
        in_ioctl(SIOCDIFADDR, &mut del, Some(ifp), true).expect("SIOCDIFADDR");
        grace();
        assert_eq!(
            malloc_out(M_IFADDR),
            out,
            "the routes' references given back"
        );
    }

    fn r_add() -> [u8; 64] {
        aliasreq([10, 0, 2, 15], Some([255, 255, 255, 0]))
    }

    /// The first Internet address of `ifp`, a handle of the test's (the lookup's own lent
    /// address cloned before its reader ends).
    fn in_ifp2ia_locked(ifp: &Ifnet) -> Option<IfaddrRef> {
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        in_ifp2ia(&net.reader(), ifp).cloned()
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
        let out = malloc_out(M_IPMADDR);
        let inm = in_addmulti(&g, ifp, &net).expect("joined");
        let one = malloc_out(M_IPMADDR) - out;
        assert!(one > 0, "one record of M_IPMADDR");
        let again = in_addmulti(&g, ifp, &net).expect("joined again");
        assert!(MArc::ptr_eq(&inm, &again), "one record for the group");
        assert_eq!(malloc_out(M_IPMADDR), out + one, "no second record");
        assert_eq!(inm.inm_refcnt().read(), 2, "two memberships");
        assert_eq!(
            MArc::count(&inm),
            3,
            "the list's handle and the two members'"
        );
        assert_eq!(
            d.addmulti.load(Ordering::Relaxed),
            adds + 1,
            "one filter change"
        );
        assert_eq!(
            igmp_sent(d),
            [(
                [239, 1, 2, 3],
                sysdefs::netinet_igmp::IGMP_v2_HOST_MEMBERSHIP_REPORT,
                [239, 1, 2, 3]
            )],
            "the first report at once"
        );

        in_delmulti(again, &net);
        assert!(in_hasmulti(&g, ifp), "one claim left");
        assert!(igmp_sent(d).is_empty());
        assert_eq!(MArc::count(&inm), 2, "the list's and the member's");
        // The last leave, a handle of the test's kept across it: the record leaves the list
        // and stays readable until that handle goes (a reference cannot outlive its record).
        let kept = inm.clone();
        in_delmulti(inm, &net);
        assert!(!in_hasmulti(&g, ifp));
        assert_eq!(MArc::count(&kept), 1, "the test's alone");
        assert_eq!(kept.inm_addr(), g, "still readable");
        assert_eq!(
            malloc_out(M_IPMADDR),
            out + one,
            "kept by the test's handle"
        );
        drop(kept);
        assert_eq!(malloc_out(M_IPMADDR), out, "freed with its last handle");
        let all_routers = INADDR_ALLROUTERS_GROUP.to_ne_bytes();
        assert_eq!(
            igmp_sent(d),
            [(
                all_routers,
                sysdefs::netinet_igmp::IGMP_HOST_LEAVE_MESSAGE,
                all_routers
            )],
            "we may have reported last: a leave message"
        );
        assert_eq!(d.delmulti.load(Ordering::Relaxed), 1);
        drop(net);
        drop(w);

        let mut del = aliasreq([10, 0, 2, 15], None);
        in_ioctl(SIOCDIFADDR, &mut del, Some(ifp), true).expect("SIOCDIFADDR");
        grace();
    }

    /// `in_lookupmulti` under a witness of another interface's `if_maddrlock`: `panic(9)`
    /// (the C's `rw_assert_anylock`, here the list's key in every build).
    #[test]
    #[should_panic(expected = "Guarded: a witness of another lock")]
    fn in_lookupmulti_under_another_interface_s_lock_is_panic() {
        let _g = setup();
        let (ifp, _) = iface();
        let (other, _) = iface();
        let r = other.rt().if_maddrlock.read();
        let _ = in_lookupmulti(&a([224, 0, 0, 1]), ifp, &r);
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
