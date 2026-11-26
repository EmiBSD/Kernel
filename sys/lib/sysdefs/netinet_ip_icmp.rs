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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)ip_icmp.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! `<netinet/ip_icmp.h>`: Interface Control Message Protocol definitions, the message
//! `struct icmp`, its types and codes and the lengths the protocol checks. Per RFC 792,
//! September 1981; RFC 950, August 1985 (Address Mask Request / Reply); RFC 1256, September
//! 1991 (Router Advertisement and Solicitation); RFC 1108, November 1991 (Param Problem,
//! Missing Req. Option); RFC 1393, January 1993 (Traceroute); RFC 1475, June 1993 (Datagram
//! Conversion Error); RFC 1812, June 1995 (adm prohib, host precedence, precedence cutoff);
//! RFC 2002, October 1996 (Mobility changes to Router Advertisement).
//!
//! Upstream: sys/netinet/ip_icmp.h @ 3ce1f3f79392
//! Upstream: sys/netinet/ip_icmp.c @ 3ce1f3f79392
//! LZ: sys/netinet/ip_icmp.rs@3c62ede68802
//!
//! [`Icmp`] is the message as it is on the wire, multi-byte members in network order.
//!
//! ## Deviations
//! - `ICMP_V6ADVLEN(p)`, `ICMP_ADVLEN(p)` and `ICMP_INFOTYPE(type)` are `const fn`s.
//!
//! ## Redesign
//! - LZ's header half of `bsd`'s `sys/netinet/ip_icmp.rs` -> `hal` (N6.4c2, with ICMP's move
//!   into `kernel::netinet::ip_icmp`). LZ's `struct icmp` held its two unions as `#[repr(C)]`
//!   unions read through `unsafe` accessors (`icmp_accessor!`, `icmp_ip`, `icmp_ip_mut`,
//!   `icmp_v6advlen`, `icmp_advlen`); here the unions are their bytes, as `hal`'s other
//!   unions (`net_if`'s `ifr_ifru`): `icmp_hun` the second word's four bytes, `icmp_dun` the
//!   union's largest member itself, the returned `struct ip` (20 bytes, the union's size and
//!   alignment), the other members (`id_ts`, `id_mask`, `id_radv`, `id_data`) read and written
//!   at their offsets in its image. Every accessor keeps LZ's name and type, so `bsd`'s
//!   readers (pf's header union, TCP's path MTU) compile unchanged; the image is an
//!   `abi_pod!`, no `unsafe`. LZ's `icmp_data()` (the address of the data that runs past the
//!   structure) has no reader and is not kept.

use core::mem::size_of;

use crate::netinet_in::InAddr;
use crate::netinet_ip::Ip;

/// `ICMP_EXT_HDR_VERSION`: the RFC 4884 extension header's version (high nibble).
pub const ICMP_EXT_HDR_VERSION: u8 = 0x20;
/// `ICMP_EXT_HDR_VMASK`.
pub const ICMP_EXT_HDR_VMASK: u8 = 0xf0;
/// `ICMP_EXT_OFFSET`: the least offset of an extension header.
pub const ICMP_EXT_OFFSET: usize = 128;

/// `ICMP_EXT_MPLS`: the MPLS label stack object class.
pub const ICMP_EXT_MPLS: u8 = 1;
/// `ICMP_EXT_IFINFO`: the interface information object class.
pub const ICMP_EXT_IFINFO: u8 = 2;

// Lower bounds on packet lengths for various types. For the error advice packets must first
// insure that the packet is large enough to contain the returned ip header. Only then can we
// do the check to see if 64 bits of packet data have been returned, since we need to check
// the returned ip header length.

/// `ICMP_MINLEN`: abs minimum.
pub const ICMP_MINLEN: usize = 8;
/// `ICMP_TSLEN`: timestamp.
pub const ICMP_TSLEN: usize = 8 + 3 * size_of::<u32>();
/// `ICMP_MASKLEN`: address mask.
pub const ICMP_MASKLEN: usize = 12;
/// `ICMP_ADVLENMIN`: min.
pub const ICMP_ADVLENMIN: usize = 8 + size_of::<Ip>() + 8;
/// `ICMP_V6ADVLENMIN`: for IPv6 transition related ICMP errors, the shortest such message.
pub const ICMP_V6ADVLENMIN: usize = 8 + size_of::<Ip>() + 40;
/// `ICMP_ADVLENMAX`: maximum.
pub const ICMP_ADVLENMAX: usize = 8 + 60 + 40;

// Definition of type and code field values (https://www.iana.org/assignments/icmp-parameters).

/// Echo reply.
pub const ICMP_ECHOREPLY: u8 = 0;
/// Dest unreachable, codes:
pub const ICMP_UNREACH: u8 = 3;
/// Bad net.
pub const ICMP_UNREACH_NET: u8 = 0;
/// Bad host.
pub const ICMP_UNREACH_HOST: u8 = 1;
/// Bad protocol.
pub const ICMP_UNREACH_PROTOCOL: u8 = 2;
/// Bad port.
pub const ICMP_UNREACH_PORT: u8 = 3;
/// `IP_DF` caused drop.
pub const ICMP_UNREACH_NEEDFRAG: u8 = 4;
/// Src route failed.
pub const ICMP_UNREACH_SRCFAIL: u8 = 5;
/// Unknown net.
pub const ICMP_UNREACH_NET_UNKNOWN: u8 = 6;
/// Unknown host.
pub const ICMP_UNREACH_HOST_UNKNOWN: u8 = 7;
/// Src host isolated.
pub const ICMP_UNREACH_ISOLATED: u8 = 8;
/// For crypto devs.
pub const ICMP_UNREACH_NET_PROHIB: u8 = 9;
/// Ditto.
pub const ICMP_UNREACH_HOST_PROHIB: u8 = 10;
/// Bad tos for net.
pub const ICMP_UNREACH_TOSNET: u8 = 11;
/// Bad tos for host.
pub const ICMP_UNREACH_TOSHOST: u8 = 12;
/// Prohibited access.
pub const ICMP_UNREACH_FILTER_PROHIB: u8 = 13;
/// Precedence violation.
pub const ICMP_UNREACH_HOST_PRECEDENCE: u8 = 14;
/// Precedence cutoff.
pub const ICMP_UNREACH_PRECEDENCE_CUTOFF: u8 = 15;
/// Packet lost, slow down.
pub const ICMP_SOURCEQUENCH: u8 = 4;
/// Shorter route, codes:
pub const ICMP_REDIRECT: u8 = 5;
/// For network.
pub const ICMP_REDIRECT_NET: u8 = 0;
/// For host.
pub const ICMP_REDIRECT_HOST: u8 = 1;
/// For tos and net.
pub const ICMP_REDIRECT_TOSNET: u8 = 2;
/// For tos and host.
pub const ICMP_REDIRECT_TOSHOST: u8 = 3;
/// Alternate host address.
pub const ICMP_ALTHOSTADDR: u8 = 6;
/// Echo service.
pub const ICMP_ECHO: u8 = 8;
/// Router advertisement.
pub const ICMP_ROUTERADVERT: u8 = 9;
/// Normal advertisement.
pub const ICMP_ROUTERADVERT_NORMAL: u8 = 0;
/// Selective routing.
pub const ICMP_ROUTERADVERT_NOROUTE_COMMON: u8 = 16;
/// Router solicitation.
pub const ICMP_ROUTERSOLICIT: u8 = 10;
/// Time exceeded, code:
pub const ICMP_TIMXCEED: u8 = 11;
/// TTL==0 in transit.
pub const ICMP_TIMXCEED_INTRANS: u8 = 0;
/// TTL==0 in reass.
pub const ICMP_TIMXCEED_REASS: u8 = 1;
/// IP header bad.
pub const ICMP_PARAMPROB: u8 = 12;
/// Req. opt. absent.
pub const ICMP_PARAMPROB_ERRATPTR: u8 = 0;
/// Req. opt. absent.
pub const ICMP_PARAMPROB_OPTABSENT: u8 = 1;
/// Bad length.
pub const ICMP_PARAMPROB_LENGTH: u8 = 2;
/// Timestamp request.
pub const ICMP_TSTAMP: u8 = 13;
/// Timestamp reply.
pub const ICMP_TSTAMPREPLY: u8 = 14;
/// Information request.
pub const ICMP_IREQ: u8 = 15;
/// Information reply.
pub const ICMP_IREQREPLY: u8 = 16;
/// Address mask request.
pub const ICMP_MASKREQ: u8 = 17;
/// Address mask reply.
pub const ICMP_MASKREPLY: u8 = 18;
/// Traceroute.
pub const ICMP_TRACEROUTE: u8 = 30;
/// Data conversion error.
pub const ICMP_DATACONVERR: u8 = 31;
/// Mobile host redirect.
pub const ICMP_MOBILE_REDIRECT: u8 = 32;
/// IPv6 where-are-you.
pub const ICMP_IPV6_WHEREAREYOU: u8 = 33;
/// IPv6 i-am-here.
pub const ICMP_IPV6_IAMHERE: u8 = 34;
/// Mobile registration req.
pub const ICMP_MOBILE_REGREQUEST: u8 = 35;
/// Mobile registration reply.
pub const ICMP_MOBILE_REGREPLY: u8 = 36;
/// SKIP.
pub const ICMP_SKIP: u8 = 39;
/// Photuris.
pub const ICMP_PHOTURIS: u8 = 40;
/// Unknown sec index.
pub const ICMP_PHOTURIS_UNKNOWN_INDEX: u8 = 1;
/// Auth failed.
pub const ICMP_PHOTURIS_AUTH_FAILED: u8 = 2;
/// Decrypt failed.
pub const ICMP_PHOTURIS_DECRYPT_FAILED: u8 = 3;

/// `ICMP_MAXTYPE`: the highest type.
pub const ICMP_MAXTYPE: u8 = 40;

/// `struct icmp_ra_addr`: ICMP Router Advertisement data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IcmpRaAddr {
    /// Router address.
    pub ira_addr: u32,
    /// Preference.
    pub ira_preference: u32,
}

crate::abi_pod!(IcmpRaAddr {
    ira_addr: u32,
    ira_preference: u32,
});

/// `struct ih_exthdr`: RFC 4884 extended header, a member of `icmp_hun`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IhExthdr {
    /// `iex_pad`.
    pub iex_pad: u8,
    /// `iex_length`: length of the original datagram, in 32-bit words.
    pub iex_length: u8,
}

crate::abi_pod!(IhExthdr {
    iex_pad: u8,
    iex_length: u8,
});

/// `struct ih_idseq`: identifier and sequence number of the echo and timestamp messages, a
/// member of `icmp_hun`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IhIdseq {
    /// `icd_id`.
    pub icd_id: u16,
    /// `icd_seq`.
    pub icd_seq: u16,
}

crate::abi_pod!(IhIdseq {
    icd_id: u16,
    icd_seq: u16,
});

/// `struct ih_pmtu`: `ICMP_UNREACH_NEEDFRAG` -- Path MTU Discovery (RFC 1191), a member of
/// `icmp_hun`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IhPmtu {
    /// `ipm_void`.
    pub ipm_void: u16,
    /// `ipm_nextmtu`: MTU of the next hop.
    pub ipm_nextmtu: u16,
}

crate::abi_pod!(IhPmtu {
    ipm_void: u16,
    ipm_nextmtu: u16,
});

/// `struct ih_rtradv`: router advertisement header, a member of `icmp_hun`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IhRtradv {
    /// `irt_num_addrs`: number of addresses.
    pub irt_num_addrs: u8,
    /// `irt_wpa`: words per address.
    pub irt_wpa: u8,
    /// `irt_lifetime`.
    pub irt_lifetime: u16,
}

crate::abi_pod!(IhRtradv {
    irt_num_addrs: u8,
    irt_wpa: u8,
    irt_lifetime: u16,
});

/// `struct id_ts`: the timestamps of the timestamp messages, a member of `icmp_dun`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IdTs {
    /// `its_otime`: originate timestamp.
    pub its_otime: u32,
    /// `its_rtime`: receive timestamp.
    pub its_rtime: u32,
    /// `its_ttime`: transmit timestamp.
    pub its_ttime: u32,
}

crate::abi_pod!(IdTs {
    its_otime: u32,
    its_rtime: u32,
    its_ttime: u32,
});

/// `struct id_ip`: the offending header an error message returns (options and then 64 bits
/// of data follow it), the largest member of `icmp_dun`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IdIp {
    /// `idi_ip`.
    pub idi_ip: Ip,
}

crate::abi_pod!(IdIp { idi_ip: Ip });

/// `struct icmp`: structure of an icmp header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Icmp {
    /// `icmp_type`: type of message, see below.
    pub icmp_type: u8,
    /// `icmp_code`: type sub code.
    pub icmp_code: u8,
    /// `icmp_cksum`: ones complement cksum of struct.
    pub icmp_cksum: u16,
    /// `icmp_hun`, the union of the header's second word (`ih_pptr`, `ih_exthdr`,
    /// `ih_gwaddr`, `ih_idseq`, `ih_void`, `ih_pmtu`, `ih_rtradv`): its four bytes.
    pub icmp_hun: [u8; 4],
    /// `icmp_dun`, the union of the data (`id_ts`, `id_ip`, `id_radv`, `id_mask`,
    /// `id_data`): its largest member, the returned IP header, whose image the others share.
    pub icmp_dun: Ip,
}

crate::abi_pod!(Icmp {
    icmp_type: u8,
    icmp_code: u8,
    icmp_cksum: u16,
    icmp_hun: [u8; 4],
    icmp_dun: Ip,
});

impl Icmp {
    /// Two bytes of `icmp_hun` at `off`, as the C's `u_int16_t` member reads them.
    #[inline]
    fn hun16(&self, off: usize) -> u16 {
        u16::from_ne_bytes([self.icmp_hun[off], self.icmp_hun[off + 1]])
    }

    /// Writes two bytes of `icmp_hun` at `off`.
    #[inline]
    fn set_hun16(&mut self, off: usize, v: u16) {
        self.icmp_hun[off..off + 2].copy_from_slice(&v.to_ne_bytes());
    }

    /// `icmp_pptr` (`icmp_hun.ih_pptr`): the parameter problem pointer.
    pub fn icmp_pptr(&self) -> u8 {
        self.icmp_hun[0]
    }

    /// Sets `icmp_pptr`.
    pub fn set_icmp_pptr(&mut self, v: u8) {
        self.icmp_hun[0] = v;
    }

    /// `icmp_length` (`icmp_hun.ih_exthdr.iex_length`): the RFC 4884 length.
    pub fn icmp_length(&self) -> u8 {
        self.icmp_hun[1]
    }

    /// Sets `icmp_length`.
    pub fn set_icmp_length(&mut self, v: u8) {
        self.icmp_hun[1] = v;
    }

    /// `icmp_gwaddr` (`icmp_hun.ih_gwaddr`): the redirect's gateway.
    pub fn icmp_gwaddr(&self) -> InAddr {
        InAddr {
            s_addr: u32::from_ne_bytes(self.icmp_hun),
        }
    }

    /// Sets `icmp_gwaddr`.
    pub fn set_icmp_gwaddr(&mut self, v: InAddr) {
        self.icmp_hun = v.s_addr.to_ne_bytes();
    }

    /// `icmp_id` (`icmp_hun.ih_idseq.icd_id`): the echo identifier, network order.
    pub fn icmp_id(&self) -> u16 {
        self.hun16(0)
    }

    /// Sets `icmp_id`.
    pub fn set_icmp_id(&mut self, v: u16) {
        self.set_hun16(0, v);
    }

    /// `icmp_seq` (`icmp_hun.ih_idseq.icd_seq`): the echo sequence number, network order.
    pub fn icmp_seq(&self) -> u16 {
        self.hun16(2)
    }

    /// Sets `icmp_seq`.
    pub fn set_icmp_seq(&mut self, v: u16) {
        self.set_hun16(2, v);
    }

    /// `icmp_void` (`icmp_hun.ih_void`).
    pub fn icmp_void(&self) -> i32 {
        i32::from_ne_bytes(self.icmp_hun)
    }

    /// Sets `icmp_void`.
    pub fn set_icmp_void(&mut self, v: i32) {
        self.icmp_hun = v.to_ne_bytes();
    }

    /// `icmp_pmvoid` (`icmp_hun.ih_pmtu.ipm_void`).
    pub fn icmp_pmvoid(&self) -> u16 {
        self.hun16(0)
    }

    /// Sets `icmp_pmvoid`.
    pub fn set_icmp_pmvoid(&mut self, v: u16) {
        self.set_hun16(0, v);
    }

    /// `icmp_nextmtu` (`icmp_hun.ih_pmtu.ipm_nextmtu`): the next hop's MTU, network order.
    pub fn icmp_nextmtu(&self) -> u16 {
        self.hun16(2)
    }

    /// Sets `icmp_nextmtu`.
    pub fn set_icmp_nextmtu(&mut self, v: u16) {
        self.set_hun16(2, v);
    }

    /// `icmp_num_addrs` (`icmp_hun.ih_rtradv.irt_num_addrs`).
    pub fn icmp_num_addrs(&self) -> u8 {
        self.icmp_hun[0]
    }

    /// Sets `icmp_num_addrs`.
    pub fn set_icmp_num_addrs(&mut self, v: u8) {
        self.icmp_hun[0] = v;
    }

    /// `icmp_wpa` (`icmp_hun.ih_rtradv.irt_wpa`).
    pub fn icmp_wpa(&self) -> u8 {
        self.icmp_hun[1]
    }

    /// Sets `icmp_wpa`.
    pub fn set_icmp_wpa(&mut self, v: u8) {
        self.icmp_hun[1] = v;
    }

    /// `icmp_lifetime` (`icmp_hun.ih_rtradv.irt_lifetime`).
    pub fn icmp_lifetime(&self) -> u16 {
        self.hun16(2)
    }

    /// Sets `icmp_lifetime`.
    pub fn set_icmp_lifetime(&mut self, v: u16) {
        self.set_hun16(2, v);
    }

    /// `icmp_ip` (`icmp_dun.id_ip.idi_ip`): the returned IP header of an error message.
    pub fn icmp_ip(&self) -> &Ip {
        &self.icmp_dun
    }

    /// `icmp_ip`, to write it.
    pub fn icmp_ip_mut(&mut self) -> &mut Ip {
        &mut self.icmp_dun
    }
}

/// `struct icmp_ext_hdr`: the RFC 4884 extension header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IcmpExtHdr {
    /// `ieh_version`: only high nibble used.
    pub ieh_version: u8,
    /// `ieh_res`: reserved, must be zero.
    pub ieh_res: u8,
    /// `ieh_cksum`: ones complement cksum of ext hdr.
    pub ieh_cksum: u16,
}

crate::abi_pod!(IcmpExtHdr {
    ieh_version: u8,
    ieh_res: u8,
    ieh_cksum: u16,
});

/// `struct icmp_ext_obj_hdr`: an RFC 4884 extension object header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IcmpExtObjHdr {
    /// `ieo_length`: length of obj incl this header.
    pub ieo_length: u16,
    /// `ieo_cnum`: class number.
    pub ieo_cnum: u8,
    /// `ieo_ctype`: sub class type.
    pub ieo_ctype: u8,
}

crate::abi_pod!(IcmpExtObjHdr {
    ieo_length: u16,
    ieo_cnum: u8,
    ieo_ctype: u8,
});

/// `offsetof(struct icmp, icmp_cksum)`: where the checksum of an ICMP message is.
pub const ICMP_CKSUM_OFFSET: usize = core::mem::offset_of!(Icmp, icmp_cksum);
/// `offsetof(struct icmp, icmp_hun)`: where the header's second word is.
pub const ICMP_HUN_OFFSET: usize = core::mem::offset_of!(Icmp, icmp_hun);
/// `offsetof(struct icmp, icmp_ip)`: where an error message's returned header is
/// (`ICMP_MINLEN`).
pub const ICMP_IP_OFFSET: usize = core::mem::offset_of!(Icmp, icmp_dun);

/// `ICMP_V6ADVLEN(p)`: the length of an IPv6 transition error message for `p`.
pub const fn icmp_v6advlen(p: &Icmp) -> usize {
    8 + ((p.icmp_dun.ip_hl() as usize) << 2) + 40
}

/// `ICMP_ADVLEN(p)`: the length of an error message for `p`. N.B.: must separately check
/// that `ip_hl >= 5`.
pub const fn icmp_advlen(p: &Icmp) -> usize {
    8 + ((p.icmp_dun.ip_hl() as usize) << 2) + 8
}

/// `ICMP_INFOTYPE(type)`: whether `t` is an informational (not an error) message type.
pub const fn icmp_infotype(t: u8) -> bool {
    matches!(
        t,
        ICMP_ECHOREPLY
            | ICMP_ECHO
            | ICMP_ROUTERADVERT
            | ICMP_ROUTERSOLICIT
            | ICMP_TSTAMP
            | ICMP_TSTAMPREPLY
            | ICMP_IREQ
            | ICMP_IREQREPLY
            | ICMP_MASKREQ
            | ICMP_MASKREPLY
    )
}

const _: () = {
    assert!(size_of::<Icmp>() == 28);
    assert!(ICMP_HUN_OFFSET == 4);
    assert!(ICMP_IP_OFFSET == ICMP_MINLEN);
    assert!(size_of::<IdTs>() <= size_of::<Ip>());
    assert!(size_of::<IcmpRaAddr>() <= size_of::<Ip>());
    assert!(size_of::<IcmpExtHdr>() == 4 && size_of::<IcmpExtObjHdr>() == 4);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::reftest::{assert_complete, assert_defines};

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/ip_icmp.h");
        let icmp = assert_defines!(defs;
        ICMP_EXT_HDR_VERSION, ICMP_EXT_HDR_VMASK, ICMP_EXT_OFFSET, ICMP_EXT_MPLS,
        ICMP_EXT_IFINFO, ICMP_MINLEN, ICMP_MASKLEN, ICMP_ADVLENMAX, ICMP_ECHOREPLY,
        ICMP_UNREACH, ICMP_UNREACH_NET, ICMP_UNREACH_HOST, ICMP_UNREACH_PROTOCOL,
        ICMP_UNREACH_PORT, ICMP_UNREACH_NEEDFRAG, ICMP_UNREACH_SRCFAIL,
        ICMP_UNREACH_NET_UNKNOWN, ICMP_UNREACH_HOST_UNKNOWN, ICMP_UNREACH_ISOLATED,
        ICMP_UNREACH_NET_PROHIB, ICMP_UNREACH_HOST_PROHIB, ICMP_UNREACH_TOSNET,
        ICMP_UNREACH_TOSHOST, ICMP_UNREACH_FILTER_PROHIB, ICMP_UNREACH_HOST_PRECEDENCE,
        ICMP_UNREACH_PRECEDENCE_CUTOFF, ICMP_SOURCEQUENCH, ICMP_REDIRECT, ICMP_REDIRECT_NET,
        ICMP_REDIRECT_HOST, ICMP_REDIRECT_TOSNET, ICMP_REDIRECT_TOSHOST, ICMP_ALTHOSTADDR,
        ICMP_ECHO, ICMP_ROUTERADVERT, ICMP_ROUTERADVERT_NORMAL,
        ICMP_ROUTERADVERT_NOROUTE_COMMON, ICMP_ROUTERSOLICIT, ICMP_TIMXCEED,
        ICMP_TIMXCEED_INTRANS, ICMP_TIMXCEED_REASS, ICMP_PARAMPROB, ICMP_PARAMPROB_ERRATPTR,
        ICMP_PARAMPROB_OPTABSENT, ICMP_PARAMPROB_LENGTH, ICMP_TSTAMP, ICMP_TSTAMPREPLY,
        ICMP_IREQ, ICMP_IREQREPLY, ICMP_MASKREQ, ICMP_MASKREPLY, ICMP_TRACEROUTE,
        ICMP_DATACONVERR, ICMP_MOBILE_REDIRECT, ICMP_IPV6_WHEREAREYOU, ICMP_IPV6_IAMHERE,
        ICMP_MOBILE_REGREQUEST, ICMP_MOBILE_REGREPLY, ICMP_SKIP, ICMP_PHOTURIS,
        ICMP_PHOTURIS_UNKNOWN_INDEX, ICMP_PHOTURIS_AUTH_FAILED, ICMP_PHOTURIS_DECRYPT_FAILED,
        ICMP_MAXTYPE);
        // The lengths written with sizeof: compare the text and the values.
        assert_eq!(defs["ICMP_TSLEN"], "(8 + 3 * sizeof (u_int32_t))");
        assert_eq!(ICMP_TSLEN, 20);
        assert_eq!(defs["ICMP_ADVLENMIN"], "(8 + sizeof (struct ip) + 8)");
        assert_eq!(ICMP_ADVLENMIN, 36);
        assert_eq!(defs["ICMP_V6ADVLENMIN"], "(8 + sizeof(struct ip) + 40)");
        assert_eq!(ICMP_V6ADVLENMIN, 68);
        let sized = ["ICMP_TSLEN", "ICMP_ADVLENMIN", "ICMP_V6ADVLENMIN"];
        assert_complete(&defs, "ICMP_", &[&icmp[..], &sized].concat());
    }
}
/* </TESTS> */
