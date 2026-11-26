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
 *	@(#)in_proto.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The IPv6 protocol switch `inet6sw[]` and `inet6domain`, and the IPv6 configuration
//! variables: `netinet6/in6_proto.c`.
//!
//! Upstream: sys/netinet6/in6_proto.c @ 3ce1f3f79392
//!
//! Since N6.5d the IPv6 domain, `ip6_protox[]` and the IPv6, ICMPv6, destination options,
//! routing header and fragment header entries are the kernel's (`kernel::netinet6::in6_proto`),
//! and TCP's since N6.6a5 (`Tcp6Protocol`), UDP's since N6.6b0 (`Udp6Protocol`), raw IPv6's,
//! divert's and the raw wildcard's since N6.6b1 (`Rip6Protocol`, `Divert6Protocol`). The rows
//! here are `bsd`'s five in the C's order: AH, ESP and IPComp, IPv4 and IPv6 in IPv6, raw
//! sockets served by the kernel's raw IPv6 requests and options (`pr_rip`, D-2 of N6.6b; their
//! inputs and sysctls are `bsd`'s until N8); `bsd`'s `sys/kern/uipc_domain.rs` hands
//! the kernel the table of its entries and these rows, and the rows' `pr_domain` is this
//! file's `inet6domain` (`bsd`'s `struct domain`, whose protocols are that table).
//!
//! Locks: \[a\] atomic operations. The sysctl variables are the kernel's
//! (`kernel::netinet6::ip6_var`, N6.5a), re-exported here for `bsd`'s importers (pf, TCP, UDP,
//! IP-in-IP) until they move.
//!
//! ## Deviations
//! - `IPSEC` is configured (M9c): AH, ESP and IPComp take `ah46_input`, `esp46_input` and
//!   `ipcomp46_input`, as in C.
//! - `NGIF` is 0 (`ipip_input` serves `IPPROTO_IPV4` and `IPPROTO_IPV6`); `MPLS`, `NCARP`,
//!   `NETHERIP` and `NGRE` are not configured: their entries are comments. `NPF` is:
//!   `IPPROTO_DIVERT` (`netinet6/ip6_divert.c`) has its entry, the kernel's. `SMALL_KERNEL`
//!   is not set, so the sysctl handlers are in the table.

use core::mem::{offset_of, size_of};

use crate::netinet::in_::{IPPROTO_AH, IPPROTO_ESP, IPPROTO_IPCOMP, IPPROTO_IPV4, IPPROTO_IPV6};
use crate::netinet::ip_ipip::ipip_input;
use crate::netinet::ipsec_input::{
    ah_sysctl, ah46_input, esp_sysctl, esp46_input, ipcomp_sysctl, ipcomp46_input,
};
use crate::netinet6::in6::SockaddrIn6;
use crate::sys::domain::Domain;
use crate::sys::protosw::{PR_ADDR, PR_ATOMIC, PR_MPSYSCTL, Protosw};
use crate::sys::socket::{AF_INET6, SOCK_RAW};

/// `inet6sw[]`'s rows that are `bsd`'s, in the C's order (the other entries are the kernel's:
/// `sys/kern/uipc_domain.rs`'s `INET6_PROTOS` is the whole table). Each is a raw socket
/// protocol whose requests and options are the kernel's raw IPv6's (`pr_rip`).
pub static INET6SW: [Protosw; 5] = [
    // IPV6: the kernel's (kernel::netinet6::in6_proto::Ip6Protocol).
    // UDP: the kernel's (kernel::netinet6::in6_proto::Udp6Protocol, N6.6b0).
    // RAW: the kernel's (kernel::netinet6::in6_proto::Rip6Protocol::RAW, N6.6b1).
    // ICMPV6, DSTOPTS, ROUTING, FRAGMENT: the kernel's (kernel::netinet6::in6_proto's
    // Icmp6Protocol, Dest6Protocol, Route6Protocol, Frag6Protocol).
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_AH as i16,
        pr_flags: PR_ATOMIC | PR_ADDR | PR_MPSYSCTL,
        pr_input: Some(ah46_input),
        pr_rip: true,
        pr_sysctl: Some(ah_sysctl),
        ..Protosw::new(&INET6DOMAIN)
    },
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_ESP as i16,
        pr_flags: PR_ATOMIC | PR_ADDR | PR_MPSYSCTL,
        pr_input: Some(esp46_input),
        pr_rip: true,
        pr_sysctl: Some(esp_sysctl),
        ..Protosw::new(&INET6DOMAIN)
    },
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_IPCOMP as i16,
        pr_flags: PR_ATOMIC | PR_ADDR | PR_MPSYSCTL,
        pr_input: Some(ipcomp46_input),
        pr_rip: true,
        pr_sysctl: Some(ipcomp_sysctl),
        ..Protosw::new(&INET6DOMAIN)
    },
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_IPV4 as i16,
        pr_flags: PR_ATOMIC | PR_ADDR,
        // NGIF > 0: in6_gif_input; not configured.
        pr_input: Some(ipip_input),
        pr_rip: true, // XXX
        ..Protosw::new(&INET6DOMAIN)
    },
    Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: IPPROTO_IPV6 as i16,
        pr_flags: PR_ATOMIC | PR_ADDR,
        // NGIF > 0: in6_gif_input; not configured.
        pr_input: Some(ipip_input),
        pr_rip: true, // XXX
        ..Protosw::new(&INET6DOMAIN)
    },
    // MPLS && NGIF > 0: IPPROTO_MPLS through in6_gif_input; NCARP > 0: IPPROTO_CARP
    // (carp6_proto_input); neither configured.
    // DIVERT (NPF > 0): the kernel's (kernel::netinet6::in6_proto::Divert6Protocol, N6.6b1).
    // NETHERIP > 0: IPPROTO_ETHERIP (ip6_etherip_input); NGRE > 0: IPPROTO_GRE
    // (gre_input6); neither configured.
    // The raw wildcard: the kernel's (kernel::netinet6::in6_proto::Rip6Protocol::WILD, N6.6b1).
];

/// `inet6domain` as `bsd`'s rows name it (`pr_domain`); the domain the kernel's socket layer
/// walks is the kernel's `Inet6Domain`, of the same family, sizes and table.
pub static INET6DOMAIN: Domain = Domain {
    dom_family: AF_INET6 as i32,
    dom_name: b"inet6",
    dom_init: None,
    dom_externalize: None,
    dom_dispose: None,
    dom_protosw: &INET6SW,
    dom_sasize: size_of::<SockaddrIn6>() as u32,
    dom_rtoffset: offset_of!(SockaddrIn6, sin6_addr) as u32,
    dom_maxplen: 128,
};

// Internet configuration info: the kernel's (kernel::netinet6::ip6_var).

pub use kernel::netinet6::ip6_var::{
    IP6_DAD_COUNT, IP6_DAD_PENDING, IP6_DEFHLIM, IP6_DEFMCASTHLIM, IP6_FORWARDING,
    IP6_HDRNESTLIMIT, IP6_MAXDYNROUTES, IP6_MAXFRAGPACKETS, IP6_MAXFRAGS, IP6_MCAST_PMTU,
    IP6_MFORWARDING, IP6_MTUDISC_TIMEOUT, IP6_MULTIPATH, IP6_NEIGHBORGCTHRESH, IP6_SENDREDIRECTS,
};

// ICMPV6 parameters: the kernel's (kernel::netinet6::ip6_var).

pub use kernel::netinet6::ip6_var::{ICMP6_REDIRTIMEOUT, ICMP6ERRPPSLIM};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_of_the_inet6_domain_and_a_raw_ip6_row() {
        assert!(
            INET6SW
                .iter()
                .all(|pr| core::ptr::eq(pr.pr_domain, &INET6DOMAIN))
        );
        assert_eq!(INET6DOMAIN.dom_sasize, 28);
        assert_eq!(INET6DOMAIN.dom_rtoffset, 8);
        // The rows are bsd's alone, raw sockets of the kernel's raw IPv6 requests and options
        // (pr_rip, N6.6b1: the raw IPv6, divert and wildcard rows are the kernel's), and no
        // row is one of the kernel's entries.
        assert!(INET6SW.iter().all(|pr| pr.pr_type == SOCK_RAW as i16
            && pr.pr_rip
            && pr.pr_usrreqs.is_none()
            && pr.pr_ctloutput.is_none()));
        assert!(INET6SW.iter().all(|pr| {
            ![
                crate::netinet::in_::IPPROTO_ICMPV6,
                crate::netinet::in_::IPPROTO_DSTOPTS,
                crate::netinet::in_::IPPROTO_ROUTING,
                crate::netinet::in_::IPPROTO_FRAGMENT,
            ]
            .contains(&i32::from(pr.pr_protocol))
                && pr.pr_protocol != 0
                && i32::from(pr.pr_protocol) != crate::netinet::in_::IPPROTO_RAW
                && i32::from(pr.pr_protocol) != crate::netinet::in_::IPPROTO_DIVERT
        }));
        assert_eq!(
            INET6SW
                .iter()
                .filter(|pr| i32::from(pr.pr_protocol) == IPPROTO_IPV6)
                .count(),
            1,
            "IPv6 in IPv6 only: the IPv6 entry is the kernel's"
        );
    }
}
/* </TESTS> */
