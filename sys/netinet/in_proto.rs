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
//! The TCP/IP protocol family: the protocol switch `inetsw[]`, `ip_protox[]` and
//! `inetdomain`.
//!
//! Upstream: sys/netinet/in_proto.c @ 3ce1f3f79392
//!
//! IP, ICMP, UDP, TCP, raw IP, IP-in-IP, IGMP, AH, ESP and IPComp, in the C's order. Since
//! N6.4c2 the domain and `ip_protox[]` are the kernel's (`kernel::netinet::in_proto`), and so
//! are the IP, ICMP and IGMP entries, which have no row here since N6.5d (`bsd`'s
//! `pffindproto`, the last reader of their data-only rows, went with `bsd`'s `ip6_init`),
//! and TCP's since N6.6a5 (`kernel::netinet::in_proto::TcpProtocol`), UDP's since N6.6b0
//! (`UdpProtocol`), raw IP's, divert's and the raw wildcard's since N6.6b1 (`RipProtocol`,
//! `DivertProtocol`); the rows left here (IP-in-IP, IPsec, pfsync) are raw sockets served by
//! the kernel's raw requests and options (`pr_rip`, D-2 of N6.6b; their own inputs, control
//! inputs and sysctls are `bsd`'s until N8);
//! `bsd`'s `sys/kern/uipc_domain.rs` hands the kernel the table of its entries and the
//! kernel's.
//!
//! Status: `ported` (M7b). The licence block carries the NRL notice with its advertising
//! clause, accepted as BSD-4 (`.claude/rules/scope-and-stubs.md`).
//!
//! ## Deviations
//! - IGMP's entry (the kernel's) has the functions of `netinet/igmp.rs`; its `pr_ctloutput`
//!   and `pr_usrreqs` are the raw ones, as in C.
//! - `IPSEC` is configured (M9c): AH, ESP and IPComp come after IGMP, as in C.
//! - `NGIF` is 0 (`ipip_input` serves `IPPROTO_IPV4`, and with `INET6`, feature `inet6`,
//!   `IPPROTO_IPV6`); `MPLS`, `NGRE`, `NCARP` and `NETHERIP` are not configured: their
//!   entries are comments. The length of `inetsw[]` counts the `INET6` entry. `NPFSYNC` is:
//!   `IPPROTO_PFSYNC` goes to `pfsync_input4` (`net/if_pfsync.rs`); so is `NPF`:
//!   `IPPROTO_DIVERT` (`netinet/ip_divert.c`) has its entry, the kernel's.
//!   `SMALL_KERNEL` is not set, so the sysctl handlers are in the table.
//! - `ip_protox[]` holds atomics (`ip_init` writes it once, every input reads it).

use core::mem::{offset_of, size_of};

use crate::net::if_pfsync::{pfsync_input4, pfsync_sysctl};
#[cfg(feature = "inet6")]
use crate::netinet::in_::IPPROTO_IPV6;
use crate::netinet::in_::{
    IPPROTO_AH, IPPROTO_ESP, IPPROTO_IPCOMP, IPPROTO_IPV4, IPPROTO_PFSYNC, SockaddrIn,
};
use crate::netinet::in_pcb::in_init;
use crate::netinet::ip_ipip::{ipip_init, ipip_input, ipip_sysctl};
use crate::netinet::ipsec_input::{
    ah_sysctl, ah4_ctlinput, ah46_input, esp_sysctl, esp4_ctlinput, esp46_input, ipcomp_sysctl,
    ipcomp46_input,
};
use crate::sys::domain::Domain;
use crate::sys::protosw::{PR_ADDR, PR_ATOMIC, PR_MPSYSCTL, Protosw};
use crate::sys::socket::{AF_INET, SOCK_RAW};

/// `inetsw[]`'s rows that are `bsd`'s, in the C's order (the other entries are the kernel's:
/// `sys/kern/uipc_domain.rs`'s `INET_PROTOS` is the whole table). Each is a raw socket
/// protocol whose requests and options are the kernel's raw IP's (`pr_rip`).
pub static INETSW: [Protosw; 5 + cfg!(feature = "inet6") as usize] = [
    // IP, UDP, TCP: the kernel's (kernel::netinet::in_proto's IpProtocol, UdpProtocol,
    // TcpProtocol).
    // RAW: the kernel's (kernel::netinet::in_proto::RipProtocol::RAW, N6.6b1).
    // ICMP: the kernel's (kernel::netinet::in_proto::IcmpProtocol).
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_IPV4 as i16,
        pr_flags: PR_ATOMIC | PR_ADDR | PR_MPSYSCTL,
        // NGIF > 0: in_gif_input; not configured.
        pr_input: Some(ipip_input),
        pr_rip: true,
        pr_sysctl: Some(ipip_sysctl),
        pr_init: Some(ipip_init),
        ..Protosw::new(&INETDOMAIN)
    },
    #[cfg(feature = "inet6")]
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_IPV6 as i16,
        pr_flags: PR_ATOMIC | PR_ADDR,
        // NGIF > 0: in_gif_input; not configured.
        pr_input: Some(ipip_input),
        pr_rip: true, // XXX
        ..Protosw::new(&INETDOMAIN)
    },
    // MPLS && NGIF > 0: IPPROTO_MPLS through in_gif_input; not configured.
    // IGMP: the kernel's (kernel::netinet::in_proto::IgmpProtocol).
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_AH as i16,
        pr_flags: PR_ATOMIC | PR_ADDR | PR_MPSYSCTL,
        pr_input: Some(ah46_input),
        pr_ctlinput: Some(ah4_ctlinput),
        pr_rip: true,
        pr_sysctl: Some(ah_sysctl),
        ..Protosw::new(&INETDOMAIN)
    },
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_ESP as i16,
        pr_flags: PR_ATOMIC | PR_ADDR | PR_MPSYSCTL,
        pr_input: Some(esp46_input),
        pr_ctlinput: Some(esp4_ctlinput),
        pr_rip: true,
        pr_sysctl: Some(esp_sysctl),
        ..Protosw::new(&INETDOMAIN)
    },
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_IPCOMP as i16,
        pr_flags: PR_ATOMIC | PR_ADDR | PR_MPSYSCTL,
        pr_input: Some(ipcomp46_input),
        pr_rip: true,
        pr_sysctl: Some(ipcomp_sysctl),
        ..Protosw::new(&INETDOMAIN)
    },
    // NGRE > 0: IPPROTO_GRE; NCARP > 0: IPPROTO_CARP; neither configured.
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_PFSYNC as i16,
        pr_flags: PR_ATOMIC | PR_ADDR | PR_MPSYSCTL,
        pr_input: Some(pfsync_input4),
        pr_rip: true,
        pr_sysctl: Some(pfsync_sysctl),
        ..Protosw::new(&INETDOMAIN)
    },
    // DIVERT (NPF > 0): the kernel's (kernel::netinet::in_proto::DivertProtocol, N6.6b1).
    // NETHERIP > 0: IPPROTO_ETHERIP; not configured.
    // The raw wildcard: the kernel's (kernel::netinet::in_proto::RipProtocol::WILD, N6.6b1).
];

/// `inetdomain`.
pub static INETDOMAIN: Domain = Domain {
    dom_family: AF_INET as i32,
    dom_name: b"inet",
    dom_init: Some(in_init),
    dom_externalize: None,
    dom_dispose: None,
    dom_protosw: &INETSW,
    dom_sasize: size_of::<SockaddrIn>() as u32,
    dom_rtoffset: offset_of!(SockaddrIn, sin_addr) as u32,
    dom_maxplen: 32,
};
/* </CODE> */
