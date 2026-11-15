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
 * Copyright (c) 1985, 1986, 1993
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
 *	@(#)in_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<netinet6/in6_var.h>`'s userland part: the address lifetimes, the interface statistics
//! of RFC 2465/2466, the requests of the per-address `SIOC*_IN6` commands (`struct
//! in6_ifreq`, `struct in6_aliasreq`), the commands and the `IN6_IFF_*` flags. The `_KERNEL`
//! part (`struct in6_ifaddr`, `struct in6_multi`) is `kernel::netinet6::in6_var`'s.
//!
//! Upstream: sys/netinet6/in6_var.h @ 3ce1f3f79392
//! LZ: sys/netinet6/in6_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `SIOCGETSGCNT_IN6` and `SIOCGETMIFCNT_IN6` encode the sizes of `struct sioc_sg_req6`
//!   (80 bytes) and `struct sioc_mif_req6` (40 bytes) of `<netinet6/ip6_mroute.h>`, which is
//!   not ported (`MROUTING` is not configured).
//!
//! ## Redesign
//! - LZ's `#[repr(C)] union In6IfreqIfru` and `union In6AliasreqIfrau` read through `unsafe`
//!   accessors -> `abi_pod!` images: `struct in6_ifreq`'s union is its 272 bytes
//!   (`ifr_ifru`, the largest member `struct icmp6_ifstat`, aligned 8 as the C's), each member
//!   read and written by copy at offset 0 (`ifr_addr`, `ifr_flags6`, `ifr_lifetime`, ...);
//!   `struct in6_aliasreq`'s `ifra_addr` is the union's address member itself (the other
//!   member is an `int` that only aligns it, and a `sockaddr_in6` is already 4-aligned and
//!   28 bytes long: `hal::sys::netinet_in_var::InAliasreq`'s shape). No `unsafe`.

use core::mem::size_of;

use crate::assert::hal_panic;
use crate::sys::ioccom::{_ioc, _iow, _iowr, IOC_INOUT};
use crate::sys::net_if::IFNAMSIZ;
use crate::sys::netinet6_in6::SockaddrIn6;
use crate::sys::netinet6_nd6::{In6Nbrinfo, In6Ndireq};
use crate::sys::types::Time;
use crate::user::AbiPod;

/// `struct in6_addrlifetime`: the lifetimes of an address. pltime/vltime are just for future
/// reference (required to implement the 2 hour rule for hosts); they are never modified by
/// `nd6_timeout` or anywhere else. userland -> kernel: accept pltime/vltime; kernel ->
/// userland: throw up everything; in kernel: modify preferred/expire only.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct In6Addrlifetime {
    /// Valid lifetime expiration time.
    pub ia6t_expire: Time,
    /// Preferred lifetime expiration time.
    pub ia6t_preferred: Time,
    /// Valid lifetime.
    pub ia6t_vltime: u32,
    /// Prefix lifetime.
    pub ia6t_pltime: u32,
}

crate::abi_pod!(In6Addrlifetime {
    ia6t_expire: Time,
    ia6t_preferred: Time,
    ia6t_vltime: u32,
    ia6t_pltime: u32,
});

/// `struct in6_ifstat`: IPv6 interface statistics, as defined in RFC2465
/// Ipv6IfStatsEntry (p12).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct In6Ifstat {
    /// # of total input datagram.
    pub ifs6_in_receive: u64,
    /// # of datagrams with invalid hdr.
    pub ifs6_in_hdrerr: u64,
    /// # of datagrams exceeded MTU.
    pub ifs6_in_toobig: u64,
    /// # of datagrams with no route.
    pub ifs6_in_noroute: u64,
    /// # of datagrams with invalid dst.
    pub ifs6_in_addrerr: u64,
    /// # of datagrams with unknown proto (increment on final dst if).
    pub ifs6_in_protounknown: u64,
    /// # of truncated datagrams.
    pub ifs6_in_truncated: u64,
    /// # of discarded datagrams (fragment timeout is not here).
    pub ifs6_in_discard: u64,
    /// # of datagrams delivered to ULP (increment on final dst if).
    pub ifs6_in_deliver: u64,
    /// # of datagrams forwarded (increment on outgoing if).
    pub ifs6_out_forward: u64,
    /// # of outgoing datagrams from ULP (does not include forwards).
    pub ifs6_out_request: u64,
    /// # of discarded datagrams.
    pub ifs6_out_discard: u64,
    /// # of datagrams fragmented.
    pub ifs6_out_fragok: u64,
    /// # of datagrams failed on fragment.
    pub ifs6_out_fragfail: u64,
    /// # of fragment datagrams (this is # after fragment).
    pub ifs6_out_fragcreat: u64,
    /// # of incoming fragmented packets (increment on final dst if).
    pub ifs6_reass_reqd: u64,
    /// # of reassembled packets (this is # after reass; increment on final dst if).
    pub ifs6_reass_ok: u64,
    /// # of reass failures (may not be packet count; increment on final dst if).
    pub ifs6_reass_fail: u64,
    /// # of inbound multicast datagrams.
    pub ifs6_in_mcast: u64,
    /// # of outbound multicast datagrams.
    pub ifs6_out_mcast: u64,
}

crate::abi_pod!(In6Ifstat {
    ifs6_in_receive: u64,
    ifs6_in_hdrerr: u64,
    ifs6_in_toobig: u64,
    ifs6_in_noroute: u64,
    ifs6_in_addrerr: u64,
    ifs6_in_protounknown: u64,
    ifs6_in_truncated: u64,
    ifs6_in_discard: u64,
    ifs6_in_deliver: u64,
    ifs6_out_forward: u64,
    ifs6_out_request: u64,
    ifs6_out_discard: u64,
    ifs6_out_fragok: u64,
    ifs6_out_fragfail: u64,
    ifs6_out_fragcreat: u64,
    ifs6_reass_reqd: u64,
    ifs6_reass_ok: u64,
    ifs6_reass_fail: u64,
    ifs6_in_mcast: u64,
    ifs6_out_mcast: u64,
});

/// `struct icmp6_ifstat`: ICMPv6 interface statistics, as defined in RFC2466
/// Ipv6IfIcmpEntry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Icmp6Ifstat {
    // Input statistics
    /// ipv6IfIcmpInMsgs, total # of input messages.
    pub ifs6_in_msg: u64,
    /// ipv6IfIcmpInErrors, # of input error messages.
    pub ifs6_in_error: u64,
    /// ipv6IfIcmpInDestUnreachs, # of input dest unreach errors.
    pub ifs6_in_dstunreach: u64,
    /// ipv6IfIcmpInAdminProhibs, # of input administratively prohibited errs.
    pub ifs6_in_adminprohib: u64,
    /// ipv6IfIcmpInTimeExcds, # of input time exceeded errors.
    pub ifs6_in_timeexceed: u64,
    /// ipv6IfIcmpInParmProblems, # of input parameter problem errors.
    pub ifs6_in_paramprob: u64,
    /// ipv6IfIcmpInPktTooBigs, # of input packet too big errors.
    pub ifs6_in_pkttoobig: u64,
    /// ipv6IfIcmpInEchos, # of input echo requests.
    pub ifs6_in_echo: u64,
    /// ipv6IfIcmpInEchoReplies, # of input echo replies.
    pub ifs6_in_echoreply: u64,
    /// ipv6IfIcmpInRouterSolicits, # of input router solicitations.
    pub ifs6_in_routersolicit: u64,
    /// ipv6IfIcmpInRouterAdvertisements, # of input router advertisements.
    pub ifs6_in_routeradvert: u64,
    /// ipv6IfIcmpInNeighborSolicits, # of input neighbor solicitations.
    pub ifs6_in_neighborsolicit: u64,
    /// ipv6IfIcmpInNeighborAdvertisements, # of input neighbor advertisements.
    pub ifs6_in_neighboradvert: u64,
    /// ipv6IfIcmpInRedirects, # of input redirects.
    pub ifs6_in_redirect: u64,
    /// ipv6IfIcmpInGroupMembQueries, # of input MLD queries.
    pub ifs6_in_mldquery: u64,
    /// ipv6IfIcmpInGroupMembResponses, # of input MLD reports.
    pub ifs6_in_mldreport: u64,
    /// ipv6IfIcmpInGroupMembReductions, # of input MLD done.
    pub ifs6_in_mlddone: u64,
    // Output statistics. We should solve unresolved routing problem...
    /// ipv6IfIcmpOutMsgs, total # of output messages.
    pub ifs6_out_msg: u64,
    /// ipv6IfIcmpOutErrors, # of output error messages.
    pub ifs6_out_error: u64,
    /// ipv6IfIcmpOutDestUnreachs, # of output dest unreach errors.
    pub ifs6_out_dstunreach: u64,
    /// ipv6IfIcmpOutAdminProhibs, # of output administratively prohibited errs.
    pub ifs6_out_adminprohib: u64,
    /// ipv6IfIcmpOutTimeExcds, # of output time exceeded errors.
    pub ifs6_out_timeexceed: u64,
    /// ipv6IfIcmpOutParmProblems, # of output parameter problem errors.
    pub ifs6_out_paramprob: u64,
    /// ipv6IfIcmpOutPktTooBigs, # of output packet too big errors.
    pub ifs6_out_pkttoobig: u64,
    /// ipv6IfIcmpOutEchos, # of output echo requests.
    pub ifs6_out_echo: u64,
    /// ipv6IfIcmpOutEchoReplies, # of output echo replies.
    pub ifs6_out_echoreply: u64,
    /// ipv6IfIcmpOutRouterSolicits, # of output router solicitations.
    pub ifs6_out_routersolicit: u64,
    /// ipv6IfIcmpOutRouterAdvertisements, # of output router advertisements.
    pub ifs6_out_routeradvert: u64,
    /// ipv6IfIcmpOutNeighborSolicits, # of output neighbor solicitations.
    pub ifs6_out_neighborsolicit: u64,
    /// ipv6IfIcmpOutNeighborAdvertisements, # of output neighbor advertisements.
    pub ifs6_out_neighboradvert: u64,
    /// ipv6IfIcmpOutRedirects, # of output redirects.
    pub ifs6_out_redirect: u64,
    /// ipv6IfIcmpOutGroupMembQueries, # of output MLD queries.
    pub ifs6_out_mldquery: u64,
    /// ipv6IfIcmpOutGroupMembResponses, # of output MLD reports.
    pub ifs6_out_mldreport: u64,
    /// ipv6IfIcmpOutGroupMembReductions, # of output MLD done.
    pub ifs6_out_mlddone: u64,
}

crate::abi_pod!(Icmp6Ifstat {
    ifs6_in_msg: u64,
    ifs6_in_error: u64,
    ifs6_in_dstunreach: u64,
    ifs6_in_adminprohib: u64,
    ifs6_in_timeexceed: u64,
    ifs6_in_paramprob: u64,
    ifs6_in_pkttoobig: u64,
    ifs6_in_echo: u64,
    ifs6_in_echoreply: u64,
    ifs6_in_routersolicit: u64,
    ifs6_in_routeradvert: u64,
    ifs6_in_neighborsolicit: u64,
    ifs6_in_neighboradvert: u64,
    ifs6_in_redirect: u64,
    ifs6_in_mldquery: u64,
    ifs6_in_mldreport: u64,
    ifs6_in_mlddone: u64,
    ifs6_out_msg: u64,
    ifs6_out_error: u64,
    ifs6_out_dstunreach: u64,
    ifs6_out_adminprohib: u64,
    ifs6_out_timeexceed: u64,
    ifs6_out_paramprob: u64,
    ifs6_out_pkttoobig: u64,
    ifs6_out_echo: u64,
    ifs6_out_echoreply: u64,
    ifs6_out_routersolicit: u64,
    ifs6_out_routeradvert: u64,
    ifs6_out_neighborsolicit: u64,
    ifs6_out_neighboradvert: u64,
    ifs6_out_redirect: u64,
    ifs6_out_mldquery: u64,
    ifs6_out_mldreport: u64,
    ifs6_out_mlddone: u64,
});

/// The size of `struct in6_ifreq`'s union: its largest member, `struct icmp6_ifstat`.
pub const IN6_IFREQ_IFRU_LEN: usize = size_of::<Icmp6Ifstat>();

/// `struct in6_ifreq`: the argument of the per-address `SIOC*_IN6` requests. The union
/// `ifr_ifru` (an address, the flags, the metric, a user pointer, the lifetimes, the
/// statistics) is its 272 bytes, every member at offset 0, read and written by the
/// accessors below.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct In6Ifreq {
    /// `ifr_name`: if name, e.g. "en0".
    pub ifr_name: [u8; IFNAMSIZ],
    /// `ifr_ifru`.
    pub ifr_ifru: [u8; IN6_IFREQ_IFRU_LEN],
}

crate::abi_pod!(In6Ifreq {
    ifr_name: [u8; IFNAMSIZ],
    ifr_ifru: [u8; IN6_IFREQ_IFRU_LEN],
});

/// A member of the union `u` read at its offset 0 (every member fits: the union is the
/// largest one's size).
#[inline]
fn ru_get<T: AbiPod>(u: &[u8]) -> T {
    match crate::pod::at::<T>(u, 0) {
        Some(v) => v,
        None => hal_panic(format_args!("in6_ifreq: a member longer than the union")),
    }
}

/// A member of the union `u` written at its offset 0.
#[inline]
fn ru_put<T: AbiPod>(u: &mut [u8], v: &T) {
    if !crate::pod::put(u, 0, v) {
        hal_panic(format_args!("in6_ifreq: a member longer than the union"));
    }
}

/// Defines the `ifr_*` shorthands of `struct in6_ifreq`'s union members.
macro_rules! in6_ifreq_accessor {
    ($($(#[$doc:meta])* $get:ident, $set:ident => $ty:ty;)*) => {
        $(
            $(#[$doc])*
            #[inline]
            pub fn $get(&self) -> $ty {
                ru_get(&self.ifr_ifru)
            }

            $(#[$doc])*
            #[inline]
            pub fn $set(&mut self, v: $ty) {
                ru_put(&mut self.ifr_ifru, &v);
            }
        )*
    };
}

impl In6Ifreq {
    /// An all-zero request.
    pub const fn zeroed() -> Self {
        Self {
            ifr_name: [0; IFNAMSIZ],
            ifr_ifru: [0; IN6_IFREQ_IFRU_LEN],
        }
    }

    in6_ifreq_accessor! {
        /// `ifr_addr` (`ifr_ifru.ifru_addr`).
        ifr_addr, set_ifr_addr => SockaddrIn6;
        /// `ifr_dstaddr` (`ifr_ifru.ifru_dstaddr`).
        ifr_dstaddr, set_ifr_dstaddr => SockaddrIn6;
        /// `ifr_flags` (`ifr_ifru.ifru_flags`).
        ifr_flags, set_ifr_flags => i16;
        /// `ifr_ifru.ifru_flags6`.
        ifr_flags6, set_ifr_flags6 => i32;
        /// `ifr_ifru.ifru_metric`.
        ifr_metric, set_ifr_metric => i32;
        /// `ifr_ifru.ifru_data`: a user address.
        ifr_data, set_ifr_data => u64;
        /// `ifr_ifru.ifru_lifetime`.
        ifr_lifetime, set_ifr_lifetime => In6Addrlifetime;
        /// `ifr_ifru.ifru_stat`.
        ifr_stat, set_ifr_stat => In6Ifstat;
        /// `ifr_ifru.ifru_icmp6stat`.
        ifr_icmp6stat, set_ifr_icmp6stat => Icmp6Ifstat;
    }
}

impl Default for In6Ifreq {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// `struct in6_aliasreq`: the argument of `SIOCAIFADDR_IN6`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct In6Aliasreq {
    /// `ifra_name`.
    pub ifra_name: [u8; IFNAMSIZ],
    /// `ifra_addr` (`ifra_ifrau.ifrau_addr`; the union's `ifrau_align` only aligns it).
    pub ifra_addr: SockaddrIn6,
    /// `ifra_dstaddr`.
    pub ifra_dstaddr: SockaddrIn6,
    /// `ifra_prefixmask`.
    pub ifra_prefixmask: SockaddrIn6,
    /// `ifra_flags`.
    pub ifra_flags: i32,
    /// `ifra_lifetime`.
    pub ifra_lifetime: In6Addrlifetime,
}

crate::abi_pod!(In6Aliasreq {
    ifra_name: [u8; IFNAMSIZ],
    ifra_addr: SockaddrIn6,
    ifra_dstaddr: SockaddrIn6,
    ifra_prefixmask: SockaddrIn6,
    ifra_flags: i32,
    ifra_lifetime: In6Addrlifetime,
});

impl In6Aliasreq {
    /// An all-zero request.
    pub const fn zeroed() -> Self {
        Self {
            ifra_name: [0; IFNAMSIZ],
            ifra_addr: SockaddrIn6::zeroed(),
            ifra_dstaddr: SockaddrIn6::zeroed(),
            ifra_prefixmask: SockaddrIn6::zeroed(),
            ifra_flags: 0,
            ifra_lifetime: In6Addrlifetime {
                ia6t_expire: 0,
                ia6t_preferred: 0,
                ia6t_vltime: 0,
                ia6t_pltime: 0,
            },
        }
    }

    /// `ifra_addr` (`ifra_ifrau.ifrau_addr`), LZ's accessor.
    #[inline]
    pub const fn ifra_addr(&self) -> &SockaddrIn6 {
        &self.ifra_addr
    }

    /// `ifra_addr`, writable.
    #[inline]
    pub const fn ifra_addr_mut(&mut self) -> &mut SockaddrIn6 {
        &mut self.ifra_addr
    }
}

/// `SIOCDIFADDR_IN6`.
pub const SIOCDIFADDR_IN6: u64 = _iow::<In6Ifreq>(b'i', 25);
/// `SIOCAIFADDR_IN6`.
pub const SIOCAIFADDR_IN6: u64 = _iow::<In6Aliasreq>(b'i', 26);

/// `SIOCGIFDSTADDR_IN6`.
pub const SIOCGIFDSTADDR_IN6: u64 = _iowr::<In6Ifreq>(b'i', 34);
/// `SIOCGIFNETMASK_IN6`.
pub const SIOCGIFNETMASK_IN6: u64 = _iowr::<In6Ifreq>(b'i', 37);

/// `SIOCGIFAFLAG_IN6`.
pub const SIOCGIFAFLAG_IN6: u64 = _iowr::<In6Ifreq>(b'i', 73);

/// `SIOCGIFINFO_IN6`.
pub const SIOCGIFINFO_IN6: u64 = _iowr::<In6Ndireq>(b'i', 108);
/// `SIOCGNBRINFO_IN6`.
pub const SIOCGNBRINFO_IN6: u64 = _iowr::<In6Nbrinfo>(b'i', 78);

/// `SIOCGIFALIFETIME_IN6`.
pub const SIOCGIFALIFETIME_IN6: u64 = _iowr::<In6Ifreq>(b'i', 81);

/// `SIOCGETSGCNT_IN6` (`struct sioc_sg_req6`, 80 bytes).
pub const SIOCGETSGCNT_IN6: u64 = _ioc(IOC_INOUT, b'u', 106, 80);
/// `SIOCGETMIFCNT_IN6` (`struct sioc_mif_req6`, 40 bytes).
pub const SIOCGETMIFCNT_IN6: u64 = _ioc(IOC_INOUT, b'u', 107, 40);

/// Anycast address.
pub const IN6_IFF_ANYCAST: i32 = 0x01;
/// Tentative address.
pub const IN6_IFF_TENTATIVE: i32 = 0x02;
/// DAD detected duplicate.
pub const IN6_IFF_DUPLICATED: i32 = 0x04;
/// May be detached from the link.
pub const IN6_IFF_DETACHED: i32 = 0x08;
/// Deprecated address.
pub const IN6_IFF_DEPRECATED: i32 = 0x10;
/// Autoconfigurable address.
pub const IN6_IFF_AUTOCONF: i32 = 0x40;
/// RFC 4941 temporary address.
pub const IN6_IFF_TEMPORARY: i32 = 0x80;

// LP64 sizes of the C structures.
const _: () = {
    assert!(size_of::<In6Addrlifetime>() == 24);
    assert!(size_of::<In6Ifstat>() == 20 * 8);
    assert!(size_of::<Icmp6Ifstat>() == 34 * 8);
    assert!(size_of::<In6Ifreq>() == IFNAMSIZ + 34 * 8);
    assert!(core::mem::align_of::<In6Ifreq>() == 8);
    assert!(size_of::<In6Aliasreq>() == 128);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::mem::offset_of;

    use super::*;
    use crate::sys::netinet6_in6::In6Addr;
    use crate::user::bytes_of;

    #[test]
    fn ioctl_numbers_are_openbsds() {
        // The values OpenBSD's userland uses (LP64).
        assert_eq!(SIOCAIFADDR_IN6, 0x8080_691a);
        assert_eq!(SIOCDIFADDR_IN6, 0x8120_6919);
        assert_eq!(SIOCGIFAFLAG_IN6, 0xc120_6949);
        assert_eq!(SIOCGIFALIFETIME_IN6, 0xc120_6951);
        assert_eq!(SIOCGIFINFO_IN6, 0xc018_696c);
        assert_eq!(SIOCGNBRINFO_IN6, 0xc038_694e);
    }

    #[test]
    fn the_union_members_sit_at_its_start() {
        let mut r = In6Ifreq::zeroed();
        r.ifr_name[..4].copy_from_slice(b"vio1");
        let a = SockaddrIn6::with_addr(In6Addr::new([
            0xfd, 0, 0, 0x77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]));
        r.set_ifr_addr(a);
        assert_eq!(r.ifr_addr(), a);
        assert_eq!(r.ifr_dstaddr(), a);
        r.set_ifr_flags6(0x12);
        assert_eq!(r.ifr_flags6(), 0x12);
        // ifru_flags6 overwrote the address's first four bytes (sin6_len, family, port).
        assert_eq!(r.ifr_addr().sin6_addr, a.sin6_addr);
        let lt = In6Addrlifetime {
            ia6t_expire: 1,
            ia6t_preferred: 2,
            ia6t_vltime: 3,
            ia6t_pltime: 4,
        };
        r.set_ifr_lifetime(lt);
        assert_eq!(r.ifr_lifetime(), lt);
        let rs = [r];
        let b = bytes_of(&rs);
        assert_eq!(&b[..4], b"vio1");
        assert_eq!(b[16..24], 1i64.to_ne_bytes());
        assert_eq!(b[36..40], 4u32.to_ne_bytes());
        assert_eq!(offset_of!(In6Aliasreq, ifra_addr), 16);
        assert_eq!(offset_of!(In6Aliasreq, ifra_prefixmask), 72);
        assert_eq!(offset_of!(In6Aliasreq, ifra_lifetime), 104);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet6/in6_var.h");
        let iff = crate::reftest::assert_defines!(defs;
            IN6_IFF_ANYCAST, IN6_IFF_TENTATIVE, IN6_IFF_DUPLICATED, IN6_IFF_DETACHED,
            IN6_IFF_DEPRECATED, IN6_IFF_AUTOCONF, IN6_IFF_TEMPORARY);
        crate::reftest::assert_complete(&defs, "IN6_IFF_", &iff);
    }
}
/* </TESTS> */
