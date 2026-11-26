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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)ip_output.c	8.3 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 output, `netinet6/ip6_output.c`: `ip6_output` (the extension headers of the packet
//! options, the route and path MTU, multicast loopback, pf(4), IPsec and fragmentation), the
//! IPv6-level socket options, the sticky and ancillary packet options of RFC 3542, the IPv6
//! multicast options, the output checksums and the fragment identifiers.
//!
//! Upstream: sys/netinet6/ip6_output.c @ 3ce1f3f79392
//! LZ: sys/netinet6/ip6_output.rs@3c62ede68802
//!
//! [`ip6_output`] takes a packet with a skeletal IPv6 header (flow, length, next header, hop
//! limit, source, destination), adds the extension headers of the packet options (hop-by-hop,
//! destination options before and after the routing header) in their own mbufs after the
//! header, finds the route and the interface (caching the route in a `struct route`), and
//! hands the packet to `if_output_tso`, fragmenting it with [`ip6_fragment`] when it is larger
//! than the path MTU. The socket option half ([`ip6_ctloutput`] .. [`ip6_freemoptions`]) keeps
//! a socket's IPv6 header fields, packet options and multicast options in its control block
//! (`kernel::netinet::in_pcb::Inpcb`).
//!
//! ## Deviations
//! - IPsec (`#ifdef IPSEC`, configured): `ip6_output_ipsec_lookup` and
//!   `ip6_output_ipsec_send` (with `ip6_output_ipsec_pmtu_update`) are `bsd`'s, reached
//!   through the `Ipsec` bridge's `ip6_output_lookup` and `ip6_output_send` legs
//!   (`kernel::net::ipsec_hooks`) at the C's points: they read the SA's members and call
//!   TCP's software TSO and ICMPv6's route cloning, `bsd`'s until N8 (D-18 of N6.5, N6.4a's
//!   R10 shape). Before the bridge is registered `ip6_output` runs as the C without `IPSEC`.
//! - `NPF` (pf(4)) is configured: `pf_test(AF_INET6, PF_OUT, ..)` through the `Pf` bridge's
//!   `test`, which passes every packet before registration; a packet pf tagged
//!   `PF_TAG_REROUTE` reruns the route lookup (the C's `goto reroute` is a labelled loop).
//! - Not configured, a comment at its site: `MROUTING` (`ip6_mforward`).
//! - `idgen32` (`crypto/idgen.c`, `bsd`'s until crypto moves) is reached through the `Inet`
//!   leg `ip6_randomid`, `bsd`'s generator. The counters of software checksums are the
//!   kernel's: UDP's (`udps_outswcsum`) `udpstat_inc` since N6.6b0 (the `Inet` leg
//!   `udpstat_outswcsum` before it), TCP's `tcpstat_inc` since N6.6a1.
//! - `in6_proto_cksum_out` stores no pseudo-header checksum when `ip6_lasthdr` finds the chain
//!   invalid (the C would write at offset -1 plus the field offset), as LZ.
//! - `ip6_ctloutput`'s `PRCO_GETOPT` without an answer mbuf is `panic(9)` (`sogetopt` always
//!   hands one over; the C would write through NULL; LZ answered `EINVAL`), as
//!   `kernel::netinet::ip_output::ip_ctloutput`.
//! - `ip6_pcbopt`'s and `ip6_setmoptions`' `malloc(M_WAITOK)`, which cannot fail in C, are
//!   `panic(9)` when they do (no `malloc(9)` registered), as LZ's and IPv4's.
//! - `ip6_setmoptions` reads the `struct ipv6_mreq` as a copy and embeds the interface's index
//!   in the copy; the C writes it into the option mbuf, which `sosetopt` frees after the call
//!   unread.
//! - `ip6_optlen` and `ip6_copypktopts` of other BSDs do not exist in this OpenBSD:
//!   `copypktopts` is file-local here as in C.
//!
//! ## Redesign
//! - LZ's `ip6_output(m: &'static Mbuf, opt: Option<&Ip6Pktopts>, ..)` over raw option words
//!   -> [`ip6_output`] over the kernel's owned packet (`MbufBox`) and a borrowed view of the
//!   options, [`Ip6OutOpts`] (`kernel::netinet6::ip6_var`): the socket's owned options lend one
//!   (`Ip6Pktopts::as_opts`), MLD lends its `const` router-alert header (`MLD6_HBH`) without
//!   an allocation per report (D-10's second half). The view is nine words on the stack where
//!   the C passes one pointer (named for N12, measured by the `ip6.output.lo` bench row). The
//!   C's `goto freehdrs`/`bad` (free) and `goto done` (consumed) are the same `return` here,
//!   the packet and the extension headers dropped by their owner if they still have one; the
//!   interface reference is an `IfRef`, the SA's a `TdbHold`, `iproute`'s route its `Route`.
//!   `NET_ASSERT_LOCKED()` is the `net` proof parameter; `bsd`'s importers keep LZ's
//!   signature through a wrapper in `sys/netinet6/ip6_output.rs` (`unpark`, the proof
//!   through the `SaToken`). `make_exthdr`'s three `unsafe` reads of option words and its
//!   `SAFETY` that named their producers go: an extension header is a slice that is whole,
//!   `(ip6e_len + 1) << 3` bytes, by type (the view's header fields are set only by
//!   `Ip6OutOpts::with_exthdrs`, which checks them; [`ip6_setpktopt`] checks a user's with
//!   the C's errnos, `MLD6_HBH`'s is asserted at compile time: N6.5c review N5), and
//!   [`ip6_copyexthdr`] copies the slice.
//! - The IPv6 header: LZ's `mtod_ip6`/`mtod_ip6_store` (an unaligned copy through `unsafe`)
//!   -> [`ip6_load`]/[`ip6_store`], `hal::mem::pod`'s view of the first mbuf's bytes when aligned
//!   (the transports' packets), a copy otherwise; `ip6_output` loads the header once, works on
//!   the copy and stores it before each step that reads the packet (IPsec, the loopback copy,
//!   the hop-by-hop check, pf, the checksums, the interface), as `ip_output` does, and only
//!   when the copy changed since the packet's header was last written or read (`dirty`: pf's
//!   rewrite is loaded back and the send after it stores only the loopback's restored scopes,
//!   N6.5c review N2).
//! - The chain of extension headers (the C's `exthdrs` pointers kept into the packet after
//!   `MAKE_CHAIN` links them, and `nexthdrp`, a pointer to the previous header's next-header
//!   byte): the headers are owned by the chain once linked, so the later steps name them by
//!   their offsets in the packet (the hop-by-hop header at 40, the first destination options
//!   after it) and read their next-header byte through `m_copydata` and write it in place in
//!   the mbuf that holds that offset (`set_byte_at`, the C's in-place byte), which stays right
//!   even if pf rebuilt the chain. Not through `m_copyback`, whose walk stops on the mbuf
//!   before an offset that is its `m_len` and makes it writable first (a copy of a shared
//!   cluster for none of its bytes: N6.5c review S1).
//! - `ip6_fragment(m0, ml, ..)`: each fragment built and checked before it goes on `ml` (the
//!   C enqueues it first and keeps writing through its pointer); the fragment header's fields
//!   are made first and `ip6_insertfraghdr` writes it where it makes its room (the trailing
//!   space of an mbuf that keeps its data in itself, or an mbuf of its own: the C's
//!   `*frghdrp`), never into the copy of the unfragmentable headers, which shares `m0`'s
//!   clusters; on failure both are freed.
//! - The socket options: LZ's `ip6_ctloutput(op, so, level, optname, m)` read values with
//!   unaligned raw reads at `mtod(m)` (`mtod_int`, `mtod_bytes`, the `ipv6_mreq`), kept the
//!   packet options and multicast options as raw `malloc` pointers in the pcb with the
//!   extension headers behind more of them (`exthdr_bytes`), and freed them with `unsafe fn
//!   ip6_freepcbopts`/`ip6_freemoptions`. Here [`ip6_ctloutput`] takes the control block
//!   (`sotoinpcb(so)` is `bsd`'s caller's until N6.6 types `so_pcb`), reads a value from the
//!   option mbuf's bytes (`Mbuf::data`, exactly `m_len` long) and writes an answer with
//!   `m_len` set first; the options are `kernel::netinet6::ip6_var::Ip6Pktopts` (an `MBox`
//!   of `M_IP6OPT` the pcb owns, with an `MBox` of `In6Pktinfo` and `MArray`s of the
//!   headers' bytes, the C's `malloc`s) and `Ip6Moptions` (`M_IPMOPTS`, its memberships the
//!   address records' `In6Memberships` array of `M_IPMADDR`, D-11), both lent under the
//!   socket lock check (`Inpcb::outputopts6`, `Inpcb::moptions6`: N6.4c1's `InpLoan`, D-10);
//!   `ip6_freepcbopts` and `ip6_freemoptions` take them by value. `struct in6_multi_mship`
//!   goes (an `In6Memberships` slot per membership); a join makes its room before
//!   `in6_addmulti` (the C's `malloc` of the entry first, `in6_joingroup`'s order) and a
//!   leave finds the newest matching membership (the C's `LIST_INSERT_HEAD` list walked from
//!   its head).
//! - `ip6_id_ctx`'s `StaticCell` (2 `unsafe`) is `bsd`'s generator behind the `ip6_randomid`
//!   leg (the module's deviations).
//!
//! **Lock and spl transitions** against the C (`ip6_output`, the unicast path): the caller's
//! net lock (any mode), no lock of its own; the route cache hit reads the route's validity
//! (`rtisvalid`), the gateway route's `ReadMostly` read when there is one (two atomics, the
//! C's two words); `if_get`/`if_put` one reference count each, as the C; the IPsec send keeps
//! the C's `KERNEL_LOCK()` around `ipsp_process_packet` inside `bsd`; the multicast loopback
//! takes `if_maddrlock` for reading once, as `in6_hasmulti`. No spl change: the C has none.
//! The socket options: `sosetopt`/`sogetopt` hold the socket lock around `ip6_ctloutput`, as
//! the C; each option loan is two claims of the pcb's cell (atomics, no lock, no spl) and a
//! check of the socket lock (two lock-word loads); `in6_addmulti`/`in6_delmulti` take
//! `if_maddrlock` and the kernel lock as the C's.
//!
//! | operation | the C's locks | the kernel's | above the C |
//! |---|---|---|---|
//! | `ip6_output` | the caller's net lock; `KERNEL_LOCK` around `ipsp_process_packet` (:2813) and `ip6_mforward` (:497) | `net: &NetHeld`; the kernel lock inside the `Ipsec` leg; `MROUTING` not configured | none |
//! | `ip6_ctloutput`, `ip6_setmoptions` | the socket lock (the caller's) | the same; the pcb's option cells lent under its check | none (two atomics per loan) |
//! | the transports' `ip6_output` calls (`bsd`'s) | none for the options | the options lent (`InpLoan`) for the call | two atomics per loan (N12) |

use core::mem::{offset_of, size_of};
use core::sync::atomic::Ordering;

use hal::mem::malloc::{M_NOWAIT, M_WAITOK, MArray, MBox};
use hal::mem::pod;
use hal::sync::netlock::NetHeld;
use sysdefs::endian::{htonl, htons};
use sysdefs::errno::Errno;
use sysdefs::net_if::{
    IFCAP_CSUM_TCPv6, IFCAP_CSUM_UDPv6, IFCAP_TSOv6, IFF_LOOPBACK, IFF_MULTICAST,
};
use sysdefs::net_route::{RTF_GATEWAY, RTF_LOCAL, RTV_MTU};
use sysdefs::netinet_icmp6::Icmp6Hdr;
use sysdefs::netinet_in::{
    IPPROTO_DSTOPTS, IPPROTO_FRAGMENT, IPPROTO_HOPOPTS, IPPROTO_ICMPV6, IPPROTO_IPV6, IPPROTO_TCP,
    IPPROTO_UDP, IPSEC_AUTH_LEVEL_DEFAULT, IPSEC_ESP_NETWORK_LEVEL_DEFAULT,
    IPSEC_ESP_TRANS_LEVEL_DEFAULT, IPSEC_IPCOMP_LEVEL_DEFAULT, IPSEC_LEVEL_BYPASS,
    IPSEC_LEVEL_UNIQUE,
};
use sysdefs::netinet_ip6::{
    IP6F_MORE_FRAG, IP6OPT_JUMBO, IP6OPT_PADN, IPV6_MAXPACKET, IPV6_MMTU, Ip6Dest, Ip6Frag, Ip6Hbh,
    Ip6Hdr,
};
use sysdefs::netinet_tcp::Tcphdr;
use sysdefs::netinet_udp::Udphdr;
use sysdefs::netinet6_in6::{
    IPSEC6_OUTSA, IPV6_AUTH_LEVEL, IPV6_AUTOFLOWLABEL, IPV6_CHECKSUM, IPV6_DEFAULT_MULTICAST_LOOP,
    IPV6_DONTFRAG, IPV6_DSTOPTS, IPV6_ESP_NETWORK_LEVEL, IPV6_ESP_TRANS_LEVEL, IPV6_HOPLIMIT,
    IPV6_HOPOPTS, IPV6_IPCOMP_LEVEL, IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP, IPV6_MINHOPCOUNT,
    IPV6_MULTICAST_HOPS, IPV6_MULTICAST_IF, IPV6_MULTICAST_LOOP, IPV6_PATHMTU, IPV6_PIPEX,
    IPV6_PKTINFO, IPV6_PORTRANGE, IPV6_PORTRANGE_DEFAULT, IPV6_PORTRANGE_HIGH, IPV6_PORTRANGE_LOW,
    IPV6_RECVDSTOPTS, IPV6_RECVDSTPORT, IPV6_RECVHOPLIMIT, IPV6_RECVHOPOPTS, IPV6_RECVPATHMTU,
    IPV6_RECVPKTINFO, IPV6_RECVRTHDR, IPV6_RECVTCLASS, IPV6_RTHDR, IPV6_RTHDRDSTOPTS, IPV6_TCLASS,
    IPV6_UNICAST_HOPS, IPV6_USE_MIN_MTU, IPV6_V6ONLY, In6Addr, In6Pktinfo, Ip6Mtuinfo, Ipv6Mreq,
    SockaddrIn6, in6_are_addr_equal, in6_is_addr_mc_intfacelocal, in6_is_addr_mc_linklocal,
    in6_is_addr_multicast, in6_is_addr_unspecified, in6_is_scope_embed,
};
use sysdefs::socket::{AF_INET6, Cmsghdr, SO_RTABLE, cmsg_align, cmsg_len};

use crate::assert::kpanic;
use crate::net::if_::{IfRef, if_input_local, if_output_ml, if_output_tso, inet};
use crate::net::if_var::Ifnet;
use crate::net::ipsec_hooks::{IpsecLevel, SpdError, TdbHold, ipsec};
use crate::net::mbuf::{
    M_BCAST, M_COPYALL, M_DONTWAIT, M_EXT, M_ICMP_CSUM_OUT, M_IPV6_DF_OUT, M_MCAST, M_TCP_CSUM_OUT,
    M_TCP_TSO, M_UDP_CSUM_OUT, M_WAIT, MAX_LINKHDR, MCLBYTES, MLEN, MT_DATA, MT_HEADER, Mbuf,
    MbufBox, MbufList, PACKET_TAG_IPSEC_IN_DONE, PF_TAG_GENERATED, PF_TAG_REROUTE, m_align,
    m_copyback, m_copydata, m_copym, m_dup_pkthdr, m_get, m_gethdr, m_move_pkthdr, m_pullup,
    m_trailingspace, mclget, ml_enqueue, ml_init, ml_len, ml_purge,
};
use crate::net::pf_hooks::{PF_OUT, PF_PASS, pf};
use crate::net::proto::{PRCO_GETOPT, PRCO_SETOPT, SockaddrView};
use crate::net::route::{RT_RESOLVE, Route, Rtentry, rtalloc, rtisvalid};
use crate::net::rtable::{rtable_l2, rtable_loindex};
use crate::net::sockbuf::{SS_ISCONNECTED, SS_PRIV};
use crate::netinet::in_pcb::{
    IN6P_AUTOFLOWLABEL, IN6P_DSTOPTS, IN6P_HIGHPORT, IN6P_HOPLIMIT, IN6P_HOPOPTS, IN6P_LOWPORT,
    IN6P_MTU, IN6P_PKTINFO, IN6P_RECVDSTPORT, IN6P_RTHDR, IN6P_TCLASS, Inpcb, in_pcbset_rtableid,
};
use crate::netinet::ip_output::in_ifcap_cksum;
use crate::netinet::ip_var::ip_mtudisc;
use crate::netinet::tcp_var::{TcpstatCounters, tcpstat_inc};
use crate::netinet::udp_var::{UdpstatCounters, udpstat_inc};
use crate::netinet6::in6::{in6_addmulti, in6_delmulti, in6_hasmulti};
use crate::netinet6::in6_cksum::in6_cksum;
use crate::netinet6::in6_src::in6_selectroute;
use crate::netinet6::ip6_input::{ip6_lasthdr, ip6_process_hopopts};
use crate::netinet6::ip6_var::{
    IP6_DEFMCASTHLIM, IP6PO_DONTFRAG, IP6PO_MINMTU_ALL, IP6PO_MINMTU_DISABLE,
    IP6PO_MINMTU_MCASTONLY, IPV6_FORWARDING, IPV6_FORWARDING_IPSEC, IPV6_MINMTU, IPV6_UNSPECSRC,
    Ip6Moptions, Ip6OutOpts, Ip6Pktopts, M_IP6OPT, ip6stat_add, ip6stat_inc,
};
use crate::proc::{ProcGlue, Thread, suser};
use sysdefs::netinet6_ip6_var::Ip6statCounters;

/// `JUMBOOPTLEN`: length of the jumbo payload option and its padding.
const JUMBOOPTLEN: usize = 8;
/// `sizeof(struct ip6_hdr)`.
const IP6_HDR_LEN: usize = size_of::<Ip6Hdr>();
/// `sizeof(struct sockaddr_in6)`: the destination bytes a route cache holds.
const SIN6_LEN: usize = size_of::<SockaddrIn6>();
/// `sizeof(struct ip6_frag)`.
const FRAG_LEN: usize = size_of::<Ip6Frag>();
/// `M_IPMOPTS`: internet multicast options (`malloc.h`).
const M_IPMOPTS: i32 = 53;
/// `offsetof(struct tcphdr, th_sum)`.
const TH_SUM_OFFSET: i32 = offset_of!(Tcphdr, th_sum) as i32;
/// `offsetof(struct udphdr, uh_sum)`.
const UH_SUM_OFFSET: i32 = offset_of!(Udphdr, uh_sum) as i32;
/// `offsetof(struct icmp6_hdr, icmp6_cksum)`.
const ICMP6_CKSUM_OFFSET: i32 = offset_of!(Icmp6Hdr, icmp6_cksum) as i32;

/// `struct ip6_exthdrs`'s extension header mbufs of a packet being built by `ip6_output`,
/// before they are linked into it (the IPv6 header's mbuf is the packet's first).
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct ip6_exthdrs' ip6e_* names so the C can be grepped"
)]
#[derive(Default)]
struct Ip6Exthdrs {
    /// `ip6e_hbh`: the hop-by-hop options header.
    ip6e_hbh: Option<MbufBox>,
    /// `ip6e_dest1`: the destination options header before a routing header.
    ip6e_dest1: Option<MbufBox>,
    /// `ip6e_dest2`: the destination options header after a routing header.
    ip6e_dest2: Option<MbufBox>,
}

/// `mtod(m, struct ip6_hdr *)`, read: the IPv6 header at the start of `m`'s data, in place
/// when it is aligned for one, copied otherwise. `panic(9)` when the first mbuf holds less
/// than a header, where the C reads past it.
#[inline]
pub fn ip6_load(m: &Mbuf) -> Ip6Hdr {
    let d = m.data();
    if let Some(ip6) = pod::view::<Ip6Hdr>(d, 0) {
        return *ip6;
    }
    match pod::at::<Ip6Hdr>(d, 0) {
        Some(ip6) => ip6,
        None => kpanic(format_args!(
            "ip6_output: an IPv6 header in {} bytes",
            d.len()
        )),
    }
}

/// `*mtod(m, struct ip6_hdr *) = *ip6`: the header written back at the start of `m`'s data,
/// in place when the data is writable, through `m_copyback` (a copy of a shared cluster first)
/// when it is not. `false`, nothing written, when no cluster can be had for that copy: the
/// caller drops the packet. The output path's first mbuf is writable (the transports' and the
/// split header's, made fresh), so the copy is not taken in practice.
#[inline]
#[must_use]
pub fn ip6_store(m: &mut Mbuf, ip6: &Ip6Hdr) -> bool {
    if let Some(d) = m.data_mut() {
        if let Some(v) = pod::view_mut::<Ip6Hdr>(d, 0) {
            *v = *ip6;
            return true;
        }
        if pod::put(d, 0, ip6) {
            return true;
        }
    }
    let mut b = [0u8; IP6_HDR_LEN];
    let _ = pod::put(&mut b, 0, ip6);
    m_copyback(m, 0, &b, M_DONTWAIT).is_ok()
}

/// [`ip6_store`] on the output path: `ENOBUFS` with the packet counted dropped when the
/// header cannot be written back.
#[inline]
fn store(m: &mut Mbuf, ip6: &Ip6Hdr) -> Result<(), Errno> {
    if ip6_store(m, ip6) {
        Ok(())
    } else {
        ip6stat_inc(Ip6statCounters::Ip6sOdropped);
        Err(Errno::ENOBUFS)
    }
}

/// The byte at offset `off` of the packet (an extension header's next-header field).
#[inline]
fn byte_at(m: &Mbuf, off: usize) -> u8 {
    let mut b = [0u8; 1];
    m_copydata(m, off as i32, &mut b);
    b[0]
}

/// The byte at offset `off` of the packet set to `v`, in place in the mbuf of the chain that
/// holds it (the C's `*nexthdrp = v` through the header's pointer): an extension header's
/// next-header byte, in an mbuf this file made (`ip6_copyexthdr`'s, the jumbo option's), so
/// writable; `ENOBUFS` for a read-only one, which no caller hands over (the C writes it
/// anyway). No `m_copyback`: its walk stops on the mbuf before when `off` is that mbuf's
/// `m_len` (the IPv6 header's 40 bytes) and makes that one writable first, a copy of its
/// cluster when it is shared, for a write of none of its bytes (N6.5c review S1).
fn set_byte_at(m: &mut Mbuf, off: usize, v: u8) -> Result<(), Errno> {
    let mut off = off;
    let mut p = m;
    while off >= p.m_len().get() as usize {
        off -= p.m_len().get() as usize;
        p = match p.m_next.get_mut() {
            Some(n) => n,
            None => kpanic(format_args!("ip6_output: a header byte past the packet")),
        };
    }
    match p.data_mut() {
        Some(d) => {
            d[off] = v;
            Ok(())
        }
        None => Err(Errno::ENOBUFS),
    }
}

/// `ro->ro_dstsin6`: a route cache's destination, as the `sockaddr_in6` `route6_cache` wrote.
#[inline]
pub fn ro_sin6(ro: &Route) -> SockaddrIn6 {
    pod::at::<SockaddrIn6>(&ro.ro_dst, 0).unwrap_or_default()
}

/// `ip6_output(m, opt, ro, flags, im6o, seclevel)`: IP6 output. The packet `m` contains a
/// skeletal IP6 header (with pri, len, nxt, hlim, src, dst). This function may modify ver and
/// hlim only. The packet is consumed; the options `opt`, if present, are a borrowed view of
/// the caller's. The caller holds the net lock (`net`).
#[allow(
    clippy::too_many_arguments,
    reason = "the C's signature and the net lock's proof"
)]
pub fn ip6_output(
    m: MbufBox,
    opt: Option<Ip6OutOpts<'_>>,
    ro: Option<&mut Route>,
    flags: i32,
    im6o: Option<&Ip6Moptions>,
    seclevel: Option<&IpsecLevel>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mut m = m;
    // `finaldst = ip6->ip6_dst`: the C keeps the final destination (and updates it when pf
    // reroutes); nothing reads it, so it is not kept here.
    let mut ip6 = ip6_load(&m);

    // MAKE_EXTHDR: the headers of the options, each in a new mbuf.
    let mut exthdrs = Ip6Exthdrs::default();
    if let Some(o) = &opt {
        // Hop-by-Hop options header
        if let Some(h) = o.hbh() {
            exthdrs.ip6e_hbh = Some(ip6_copyexthdr(h)?);
        }
        // Destination options header(1st part)
        if let Some(h) = o.dest1() {
            exthdrs.ip6e_dest1 = Some(ip6_copyexthdr(h)?);
        }
        // Destination options header(2nd part)
        if let Some(h) = o.dest2() {
            exthdrs.ip6e_dest2 = Some(ip6_copyexthdr(h)?);
        }
    }

    let mut tdb: Option<TdbHold> = None;
    if let Some(legs) = ipsec()
        && (legs.in_use() || seclevel.is_some())
    {
        match legs.ip6_output_lookup(&m, seclevel) {
            // -EINVAL: the packet should be silently dropped, typically because we've asked
            // key management for an SA.
            Err(SpdError::Drop) => return Ok(()),
            Err(SpdError::Errno(e)) => return Err(e),
            Ok(t) => tdb = t.map(|t| TdbHold::new(legs, t)),
        }
    }

    // Calculate the total length of the extension header chain. Keep the length of the
    // unfragmentable part for fragmentation.
    let len_of = |h: &Option<MbufBox>| h.as_ref().map_or(0, |h| h.m_len().get());
    let hbh_len = len_of(&exthdrs.ip6e_hbh) as usize;
    let mut optlen: u32 = len_of(&exthdrs.ip6e_hbh) + len_of(&exthdrs.ip6e_dest1);
    let unfragpartlen = optlen + IP6_HDR_LEN as u32;
    // NOTE: we don't add AH/ESP length here. do that later.
    optlen += len_of(&exthdrs.ip6e_dest2);

    // If we need IPsec, or there is at least one extension header, separate IP6 header from
    // the payload (`sproto`, which the C never sets, is 0).
    let mut hdrsplit = false;
    if optlen != 0 {
        m = ip6_splithdr(m)?;
        hdrsplit = true;
    }

    // adjust mbuf packet header length
    let ph = m.m_pkthdr();
    ph.len.set(ph.len.get().wrapping_add(optlen as i32));
    let plen = (ph.len.get() as u32).wrapping_sub(IP6_HDR_LEN as u32);

    // If this is a jumbo payload, insert a jumbo payload option.
    if plen as usize > IPV6_MAXPACKET {
        if !hdrsplit {
            m = ip6_splithdr(m)?;
            hdrsplit = true;
        }
        ip6_insert_jumboopt(&mut exthdrs, &m, plen)?;
        ip6.ip6_plen = 0;
    } else {
        ip6.ip6_plen = htons(plen as u16);
    }
    // The hop-by-hop header's length once the jumbo option is in (it may have made one).
    let hbh_len = exthdrs
        .ip6e_hbh
        .as_ref()
        .map_or(hbh_len, |h| h.m_len().get() as usize);

    // Concatenate headers and fill in next header fields. Here we have, on "m"
    //	IPv6 payload
    // and we insert headers accordingly. Finally, we should be getting:
    //	IPv6 hbh dest1 rthdr ah* [esp* dest2 payload]
    // The headers are linked after the IPv6 header's mbuf; `nexthdrp`, the previous header's
    // next-header byte, is the IPv6 header's `ip6_nxt` until a header is linked, then that
    // header's first byte.
    let (has_hbh, has_dest1) = (exthdrs.ip6e_hbh.is_some(), exthdrs.ip6e_dest1.is_some());
    {
        // We treat dest2 specially. This makes IPsec processing much easier. The goal here is
        // to make mprev point the mbuf prior to dest2. result: IPv6 dest2 payload.
        if let Some(mut d2) = exthdrs.ip6e_dest2.take() {
            if !hdrsplit {
                kpanic(format_args!("ip6_output: assumption failed: hdr not split"));
            }
            set_byte_at(&mut d2, 0, ip6.ip6_nxt)?;
            ip6.ip6_nxt = IPPROTO_DSTOPTS as u8;
            link_after(&mut m, d2);
        }

        // MAKE_CHAIN. result: IPv6 hbh dest1 rthdr dest2 payload.
        if let Some(mut h) = exthdrs.ip6e_hbh.take() {
            if !hdrsplit {
                kpanic(format_args!("assumption failed: hdr not split"));
            }
            set_byte_at(&mut h, 0, ip6.ip6_nxt)?;
            ip6.ip6_nxt = IPPROTO_HOPOPTS as u8;
            link_after(&mut m, h);
        }
        if let Some(mut d1) = exthdrs.ip6e_dest1.take() {
            if !hdrsplit {
                kpanic(format_args!("assumption failed: hdr not split"));
            }
            if has_hbh {
                // `*nexthdrp` is the hop-by-hop header's first byte, at offset 40.
                set_byte_at(&mut d1, 0, byte_at(&m, IP6_HDR_LEN))?;
                set_byte_at(&mut m, IP6_HDR_LEN, IPPROTO_DSTOPTS as u8)?;
                match m.m_next.get_mut() {
                    Some(hbhm) => link_after(hbhm, d1),
                    None => kpanic(format_args!("ip6_output: the hop-by-hop header is gone")),
                }
            } else {
                set_byte_at(&mut d1, 0, ip6.ip6_nxt)?;
                ip6.ip6_nxt = IPPROTO_DSTOPTS as u8;
                link_after(&mut m, d1);
            }
        }
    }

    // Source address validation
    if flags & IPV6_UNSPECSRC == 0 && in6_is_addr_unspecified(&ip6.ip6_src) {
        // XXX: we can probably assume validation in the caller, but we explicitly check the
        // address here for safety.
        ip6stat_inc(Ip6statCounters::Ip6sBadscope);
        return Err(Errno::EOPNOTSUPP);
    }
    if in6_is_addr_multicast(&ip6.ip6_src) {
        ip6stat_inc(Ip6statCounters::Ip6sBadscope);
        return Err(Errno::EOPNOTSUPP);
    }

    ip6stat_inc(Ip6statCounters::Ip6sLocalout);

    // Route packet.
    let orig_rtableid = m.m_pkthdr().ph_rtableid.get();
    let mut caller_ro = ro;
    let mut iproute = Route::new();
    let mut ifp: Option<IfRef> = None;
    let mut src_scope: u16 = 0;
    let mut dst_scope: u16 = 0;
    // reroute: pf(4) asks for a new route lookup after it changed the packet.
    loop {
        // initialize cached route: the caller's, or ours (`ro_pmtu` is the same).
        let r: &mut Route = match caller_ro.as_deref_mut() {
            Some(r) => r,
            None => &mut iproute,
        };
        let mcast = in6_is_addr_multicast(&ip6.ip6_dst);

        // If specified, try to fill in the traffic class field. Do not override if a non-zero
        // value is already set. We check the diffserv field and the ecn field separately.
        if let Some(o) = &opt
            && o.tclass >= 0
        {
            let mut mask = 0;
            if ip6.ip6_flow & htonl(0xfc << 20) == 0 {
                mask |= 0xfc;
            }
            if ip6.ip6_flow & htonl(0x03 << 20) == 0 {
                mask |= 0x03;
            }
            if mask != 0 {
                ip6.ip6_flow |= htonl(((o.tclass & mask) as u32) << 20);
            }
        }

        // fill in or override the hop limit field, if necessary.
        if let Some(o) = &opt
            && o.hlim != -1
        {
            ip6.ip6_hlim = (o.hlim & 0xff) as u8;
        } else if mcast {
            ip6.ip6_hlim = match im6o {
                Some(im6o) => im6o.im6o_hlim,
                None => IP6_DEFMCASTHLIM.load(Ordering::Relaxed) as u8,
            };
        }

        if let Some(t) = tdb.as_ref() {
            // XXX what should we do if ip6_hlim == 0 and the packet gets tunneled? Callee
            // frees mbuf.
            store(&mut m, &ip6)?;
            return t
                .legs()
                .ip6_output_send(t.tdb(), m, r, orig_rtableid, false);
        }

        if mcast {
            // If the caller specify the outgoing interface explicitly, use it.
            if let Some(pi) = opt.as_ref().and_then(|o| o.pktinfo) {
                ifp = IfRef::get(pi.ipi6_ifindex);
            }
            if ifp.is_none()
                && let Some(im6o) = im6o
            {
                ifp = IfRef::get(u32::from(im6o.im6o_ifidx));
            }
        }

        let rtableid = m.m_pkthdr().ph_rtableid.get();
        // `dst = &ro->ro_dstsin6`, or the gateway below: the address the interface is given.
        let mut dst = [0u8; SIN6_LEN];
        if ifp.is_none() {
            let pi_ifindex = opt
                .as_ref()
                .and_then(|o| o.pktinfo)
                .map_or(0, |p| p.ipi6_ifindex);
            let Some(rt) = in6_selectroute(&ip6.ip6_dst, pi_ifindex, r, rtableid, net) else {
                ip6stat_inc(Ip6statCounters::Ip6sNoroute);
                return Err(Errno::EHOSTUNREACH);
            };
            let idx = if rt.rt_flags.get() & RTF_LOCAL != 0 {
                rtable_loindex(rtableid)
            } else {
                rt.rt_ifidx.get()
            };
            // `dst = satosin6(rt->rt_gateway)` of a gateway route to a unicast destination:
            // the gateway read once, copied.
            let gateway = (rt.rt_flags.get() & RTF_GATEWAY != 0 && !mcast)
                .then(|| {
                    rt.with_gateway(|g| {
                        g.and_then(|g| g.get(..SIN6_LEN)).map(|g| {
                            let mut b = [0u8; SIN6_LEN];
                            b.copy_from_slice(g);
                            b
                        })
                    })
                })
                .flatten();
            // We aren't using rtisvalid() here because the UP/DOWN state machine is broken
            // with some Ethernet drivers like em(4). As a result we might try to use an
            // invalid cached route entry while an interface is being detached.
            let Some(i) = IfRef::get(idx) else {
                ip6stat_inc(Ip6statCounters::Ip6sNoroute);
                return Err(Errno::EHOSTUNREACH);
            };
            ifp = Some(i);
            match gateway {
                Some(g) => dst = g,
                None => dst.copy_from_slice(&r.ro_dst[..SIN6_LEN]),
            }
        } else {
            let _ = r.cache6(&ip6.ip6_dst.s6_addr, None, rtableid);
            dst.copy_from_slice(&r.ro_dst[..SIN6_LEN]);
        }
        let Some(i) = ifp.as_ref().map(IfRef::ifp) else {
            ip6stat_inc(Ip6statCounters::Ip6sNoroute);
            return Err(Errno::EHOSTUNREACH);
        };

        if mcast {
            // Multicast
            m.m_flags().set((m.flags() & !M_BCAST) | M_MCAST);

            // Confirm that the outgoing interface supports multicast.
            if i.if_flags.get() & IFF_MULTICAST == 0 {
                ip6stat_inc(Ip6statCounters::Ip6sNoroute);
                return Err(Errno::ENETUNREACH);
            }

            if im6o.is_none_or(|im6o| im6o.im6o_loop != 0) && in6_hasmulti(&ip6.ip6_dst, i) {
                // If we belong to the destination multicast group on the outgoing interface,
                // and the caller did not forbid loopback, loop back a copy. Can't defer
                // TCP/UDP checksumming, do the computation now.
                store(&mut m, &ip6)?;
                in6_proto_cksum_out(&mut m, None);
                ip6_mloopback(i, &mut m, &dst, net);
            }
            // MROUTING: else, when acting as a multicast router (ip6_mforwarding,
            // ip6_mrouter_active, not IPV6_FORWARDING), ip6_mforward() under KERNEL_LOCK,
            // `goto bad` if it fails; not configured.

            // Multicasts with a hoplimit of zero may be looped back, above, but must not be
            // transmitted on a network. Also, multicasts addressed to the loopback interface
            // are not sent -- the above call to ip6_mloopback() will loop back a copy if this
            // host actually belongs to the destination group on the loopback interface.
            if ip6.ip6_hlim == 0
                || i.if_flags.get() & IFF_LOOPBACK != 0
                || in6_is_addr_mc_intfacelocal(&ip6.ip6_dst)
            {
                return Ok(());
            }
        } else {
            // Unicast
            m.m_flags().set(m.flags() & !(M_BCAST | M_MCAST)); // just in case
        }

        // If this packet is going through a loopback interface we won't be able to restore its
        // scope ID using the interface index.
        let looped = i.if_flags.get() & IFF_LOOPBACK != 0;
        if in6_is_scope_embed(&ip6.ip6_src) {
            if looped {
                src_scope = ip6.ip6_src.s6_addr16(1);
            }
            ip6.ip6_src.set_s6_addr16(1, 0);
        }
        if in6_is_scope_embed(&ip6.ip6_dst) {
            if looped {
                dst_scope = ip6.ip6_dst.s6_addr16(1);
            }
            ip6.ip6_dst.set_s6_addr16(1, 0);
        }

        // Determine path MTU.
        let mut mtu = ip6_getpmtu(r.rt(), i);

        // The caller of this function may specify to use the minimum MTU in some cases. An
        // advanced API option (IPV6_USE_MIN_MTU) can also override MTU setting. The logic is
        // a bit complicated; by default, unicast packets will follow path MTU while multicast
        // packets will be sent at the minimum MTU. If IP6PO_MINMTU_ALL is specified, all
        // packets including unicast ones will be sent at the minimum MTU. Multicast packets
        // will always be sent at the minimum MTU unless IP6PO_MINMTU_DISABLE is explicitly
        // specified. See RFC 3542 for more details.
        if mtu > u64::from(IPV6_MMTU)
            && (flags & IPV6_MINMTU != 0
                || opt.as_ref().is_some_and(|o| o.minmtu == IP6PO_MINMTU_ALL)
                || (mcast
                    && opt
                        .as_ref()
                        .is_none_or(|o| o.minmtu != IP6PO_MINMTU_DISABLE)))
        {
            mtu = u64::from(IPV6_MMTU);
        }

        // Whether `ip6` holds changes the packet's header does not have yet: the header is
        // written back once before a step that reads the packet, and only when it changed (the
        // C changes it in place).
        let mut dirty = true;

        // If the outgoing packet contains a hop-by-hop options header, it must be examined and
        // processed even by the source node. (RFC 2460, section 4.)
        if has_hbh {
            store(&mut m, &ip6)?;
            dirty = false;
            let hbhlen =
                (i32::from(byte_at(&m, IP6_HDR_LEN + offset_of!(Ip6Hbh, ip6h_len))) + 1) << 3;
            let mut rtalert: u32 = 0; // returned value is ignored
            let mut jumboplen: u32 = 0; // no more than 1 jumbo payload option!

            m.m_pkthdr().ph_ifidx.set(i.if_index.get());
            let mut mp = Some(m);
            let ok = ip6_process_hopopts(
                &mut mp,
                (IP6_HDR_LEN + size_of::<Ip6Hbh>()) as i32,
                hbhlen - size_of::<Ip6Hbh>() as i32,
                &mut rtalert,
                &mut jumboplen,
                net,
            );
            let Some(mm) = mp.filter(|_| ok) else {
                // m was already freed at this point
                return Err(Errno::EINVAL); // better error?
            };
            m = mm;
            m.m_pkthdr().ph_ifidx.set(0);
        }

        // Packet filter.
        if let Some(p) = pf() {
            if dirty {
                store(&mut m, &ip6)?;
            }
            let (action, left) = p.test(AF_INET6, PF_OUT, i, m);
            if action != PF_PASS {
                // `goto bad` frees the packet as pf left it.
                drop(left);
                return Err(Errno::EACCES);
            }
            let Some(mm) = left else {
                return Ok(());
            };
            m = mm;
            // `ip6 = mtod(m, struct ip6_hdr *)`: pf may have rewritten the header.
            ip6 = ip6_load(&m);
            dirty = false;
            let pfh = &m.m_pkthdr().pf;
            let tags = pfh.flags.get();
            if tags & (PF_TAG_REROUTE | PF_TAG_GENERATED) == (PF_TAG_REROUTE | PF_TAG_GENERATED) {
                // already rerun the route lookup, go on
                pfh.flags.set(tags & !(PF_TAG_GENERATED | PF_TAG_REROUTE));
            } else if tags & PF_TAG_REROUTE != 0 {
                // tag as generated to skip over pf_test on rerun
                pfh.flags.set(tags | PF_TAG_GENERATED);
                // `finaldst = ip6->ip6_dst`: unread (above).
                // `ro = NULL`, then `ro = &iproute; ro->ro_rt = NULL` at reroute: our route
                // given back (`rtfree`), the caller's left to the caller.
                caller_ro = None;
                iproute.ro_rt = None;
                // drop reference since destination changed (`if_put(ifp); ifp = NULL`)
                drop(ifp.take());
                continue;
            }
        }

        if flags & IPV6_FORWARDING != 0
            && flags & IPV6_FORWARDING_IPSEC != 0
            && m.m_pkthdr().ph_tagsset.get() & PACKET_TAG_IPSEC_IN_DONE == 0
        {
            return Err(Errno::EHOSTUNREACH);
        }

        // If the packet is not going on the wire it can be destined to any local address. In
        // this case do not clear its scopes to let ip6_input() find a matching local route.
        if looped {
            if in6_is_scope_embed(&ip6.ip6_src) {
                ip6.ip6_src.set_s6_addr16(1, src_scope);
                dirty = true;
            }
            if in6_is_scope_embed(&ip6.ip6_dst) {
                ip6.ip6_dst.set_s6_addr16(1, dst_scope);
                dirty = true;
            }
        }
        if dirty {
            store(&mut m, &ip6)?;
        }

        // Send the packet to the outgoing interface. If necessary, do IPv6 fragmentation before
        // sending.
        //
        // the logic here is rather complex:
        // 1: normal case (dontfrag == 0)
        // 1-a: send as is if tlen <= path mtu
        // 1-b: fragment if tlen > path mtu
        //
        // 2: if user asks us not to fragment (dontfrag == 1)
        // 2-a: send as is if tlen <= interface mtu
        // 2-b: error if tlen > interface mtu
        let ph = m.m_pkthdr();
        // `int tlen`, compared with the unsigned MTUs as the C's conversions do.
        let tlen: u32 = if ph.csum_flags.get() & M_TCP_TSO != 0 {
            u32::from(ph.ph_mss.get())
        } else {
            ph.len.get() as u32
        };

        let dontfrag = if ph.csum_flags.get() & M_IPV6_DF_OUT != 0 {
            ph.csum_flags.set(ph.csum_flags.get() & !M_IPV6_DF_OUT);
            true
        } else {
            opt.as_ref().is_some_and(|o| o.flags & IP6PO_DONTFRAG != 0)
        };

        if dontfrag && tlen > i.if_mtu.get() {
            // case 2-b
            if ip_mtudisc.load(Ordering::Relaxed) != 0
                && let Some(legs) = ipsec()
            {
                legs.adjust_mtu(&m, mtu as u32);
            }
            return Err(Errno::EMSGSIZE);
        }

        let Some(dsa) = SockaddrView::new(&dst) else {
            kpanic(format_args!("ip6_output: a destination too short"));
        };

        // transmit packet without fragmentation
        if dontfrag || u64::from(tlen) <= mtu {
            // case 1-a and 2-a
            let mut mp = Some(m);
            if_output_tso(i, &mut mp, dsa, r.rt(), i.if_mtu.get())?;
            // A packet left with no error is freed (the C's `goto bad`, "should not happen").
            drop(mp);
            return Ok(());
        }

        // try to fragment the packet. case 1-b
        if mtu < u64::from(IPV6_MMTU) {
            // path MTU cannot be less than IPV6_MMTU
            return Err(Errno::EMSGSIZE);
        } else if ip6.ip6_plen == 0 {
            // jumbo payload cannot be fragmented
            return Err(Errno::EMSGSIZE);
        }

        // Too large for the destination or interface; fragment if possible. Must be able to
        // put at least 8 bytes per fragment.
        let hlen = unfragpartlen as i32;
        if mtu > IPV6_MAXPACKET as u64 {
            mtu = IPV6_MAXPACKET as u64;
        }

        // If we are doing fragmentation, we can't defer TCP/UDP checksumming; compute the
        // checksum and clear the flag.
        in6_proto_cksum_out(&mut m, None);

        // Change the next header field of the last header in the unfragmentable part.
        let nextproto = if has_dest1 {
            let at = IP6_HDR_LEN + if has_hbh { hbh_len } else { 0 };
            let n = byte_at(&m, at);
            set_byte_at(&mut m, at, IPPROTO_FRAGMENT as u8)?;
            n
        } else if has_hbh {
            let n = byte_at(&m, IP6_HDR_LEN);
            set_byte_at(&mut m, IP6_HDR_LEN, IPPROTO_FRAGMENT as u8)?;
            n
        } else {
            let n = ip6.ip6_nxt;
            ip6.ip6_nxt = IPPROTO_FRAGMENT as u8;
            store(&mut m, &ip6)?;
            n
        };

        let mut ml = MbufList::new();
        ip6_fragment(m, &mut ml, hlen, nextproto, mtu)?;
        if_output_ml(i, &mut ml, dsa, r.rt())?;
        ip6stat_inc(Ip6statCounters::Ip6sFragmented);
        return Ok(());
    }
}

/// `n` linked right after `m` (`n->m_next = m->m_next; m->m_next = n`).
#[inline]
fn link_after(m: &mut Mbuf, mut n: MbufBox) {
    if let Some(rest) = m.m_next.take() {
        drop(n.m_next.set(rest));
    }
    drop(m.m_next.set(n));
}

/// The last mbuf of `m`'s chain.
fn last_mbuf(m: &mut Mbuf) -> &mut Mbuf {
    let mut p = m;
    while p.m_next.is_some() {
        p = match p.m_next.get_mut() {
            Some(n) => n,
            None => kpanic(format_args!("ip6_output: a chain link vanished")),
        };
    }
    p
}

/// `ip6_fragment(m0, ml, hlen, nextproto, mtu)`: splits `m0` (unfragmentable part `hlen`
/// bytes, next header `nextproto`) into fragments for `mtu` on `ml`. Consumes the packet; on
/// error `ml` is purged.
pub fn ip6_fragment(
    m0: MbufBox,
    ml: &mut MbufList,
    hlen: i32,
    nextproto: u8,
    mtu: u64,
) -> Result<(), Errno> {
    let mut m0 = m0;
    ml_init(ml);

    let ip6 = ip6_load(&m0);
    let tlen = m0.m_pkthdr().len.get();
    // `(mtu - hlen - sizeof(struct ip6_frag)) & ~7` in the C's unsigned long, then the `int`.
    let mut len = (mtu.wrapping_sub(hlen as u64).wrapping_sub(FRAG_LEN as u64) & !7) as i32;

    let error = 'bad: {
        if len < 8 {
            break 'bad Errno::EMSGSIZE;
        }
        let id = htonl(ip6_randomid());

        // Loop through length of payload, make new header and copy data of each part and link
        // onto chain.
        let mut off = hlen;
        while off < tlen {
            let Some(mut m) = m_gethdr(M_DONTWAIT, MT_HEADER) else {
                break 'bad Errno::ENOBUFS;
            };
            if let Err(e) = m_dup_pkthdr(&mut m, &m0, M_DONTWAIT) {
                break 'bad e;
            }
            m.data_add(MAX_LINKHDR.load(Ordering::Relaxed) as isize);
            let mut mhip6 = ip6;
            m.m_len().set(IP6_HDR_LEN as u32);

            // The fragment header's fields first (the C fills `*ip6f` after
            // `ip6_insertfraghdr` returns; nothing reads them between), so that
            // `ip6_insertfraghdr` writes it where it makes its room.
            let mut offlg = htons(((off - hlen) & !7) as u16);
            if off + len >= tlen {
                len = tlen - off;
            } else {
                offlg |= IP6F_MORE_FRAG;
            }
            let frag = Ip6Frag {
                ip6f_nxt: nextproto,
                ip6f_reserved: 0,
                ip6f_offlg: offlg,
                ip6f_ident: id,
            };
            if let Err(e) = ip6_insertfraghdr(&mut m0, &mut m, hlen, frag) {
                break 'bad e;
            }

            m.m_pkthdr().len.set(hlen + FRAG_LEN as i32 + len);
            mhip6.ip6_plen = htons((m.m_pkthdr().len.get() - IP6_HDR_LEN as i32) as u16);
            if !ip6_store(&mut m, &mhip6) {
                break 'bad Errno::ENOBUFS;
            }
            let Some(n) = m_copym(&mut m0, off, len, M_DONTWAIT) else {
                break 'bad Errno::ENOBUFS;
            };
            drop(last_mbuf(&mut m).m_next.set(n));
            ml_enqueue(ml, m);
            off += len;
        }

        ip6stat_add(Ip6statCounters::Ip6sOfragments, u64::from(ml_len(ml)));
        return Ok(());
    };
    // bad:
    ip6stat_inc(Ip6statCounters::Ip6sOdropped);
    let _ = ml_purge(ml);
    Err(error)
}

/// `ip6_copyexthdr(&m, hdr, hlen)`: a new mbuf holding the extension header `hdr` (whole,
/// `(ip6e_len + 1) << 3` bytes: `Ip6OutOpts::with_exthdrs`' check).
fn ip6_copyexthdr(hdr: &[u8]) -> Result<MbufBox, Errno> {
    let hlen = hdr.len();
    if hlen > MCLBYTES {
        return Err(Errno::ENOBUFS); // XXX
    }

    let Some(mut m) = m_get(M_DONTWAIT, MT_DATA) else {
        return Err(Errno::ENOBUFS);
    };

    if hlen > MLEN {
        mclget(&mut m, M_DONTWAIT);
        if m.flags() & M_EXT == 0 {
            return Err(Errno::ENOBUFS);
        }
    }
    m.m_len().set(hlen as u32);
    match m.data_mut() {
        Some(d) => d.copy_from_slice(hdr),
        None => kpanic(format_args!("ip6_copyexthdr: a fresh mbuf is read-only")),
    }
    Ok(m)
}

/// `ip6_insert_jumboopt(exthdrs, plen)`: inserts the jumbo payload option for payload length
/// `plen` into the hop-by-hop options header, making one if there is none; `ip6m` is the
/// packet's header mbuf, whose length it adjusts.
fn ip6_insert_jumboopt(exthdrs: &mut Ip6Exthdrs, ip6m: &Mbuf, plen: u32) -> Result<(), Errno> {
    // If there is no hop-by-hop options header, allocate new one. If there is one but it
    // doesn't have enough space to store the jumbo payload option, allocate a cluster to
    // store the whole options. Otherwise, use it to store the options.
    let optbuf: usize = match exthdrs.ip6e_hbh.take() {
        None => {
            let Some(mopt) = m_get(M_DONTWAIT, MT_DATA) else {
                return Err(Errno::ENOBUFS);
            };
            mopt.m_len().set(JUMBOOPTLEN as u32);
            let mopt = exthdrs.ip6e_hbh.insert(mopt);
            let d = writable(mopt);
            d[1] = 0; // = ((JUMBOOPTLEN) >> 3) - 1
            0
        }
        Some(old) => {
            let mut mopt = old;
            let optbuf = if (m_trailingspace(&mopt) as usize) < JUMBOOPTLEN {
                // XXX assumption:
                // - exthdrs->ip6e_hbh is not referenced from places other than exthdrs.
                // - exthdrs->ip6e_hbh is not an mbuf chain.
                let oldoptlen = mopt.m_len().get() as usize;

                // XXX: give up if the whole (new) hbh header does not fit even in an mbuf
                // cluster.
                if oldoptlen + JUMBOOPTLEN > MCLBYTES {
                    exthdrs.ip6e_hbh = Some(mopt);
                    return Err(Errno::ENOBUFS);
                }

                // As a consequence, we must always prepare a cluster at this point.
                let mut n = m_get(M_DONTWAIT, MT_DATA);
                if let Some(nn) = n.as_mut() {
                    mclget(nn, M_DONTWAIT);
                    if nn.flags() & M_EXT == 0 {
                        n = None;
                    }
                }
                let Some(mut n) = n else {
                    exthdrs.ip6e_hbh = Some(mopt);
                    return Err(Errno::ENOBUFS);
                };
                n.m_len().set((oldoptlen + JUMBOOPTLEN) as u32);
                writable(&mut n)[..oldoptlen].copy_from_slice(&mopt.data()[..oldoptlen]);
                // m_freem(mopt): the old header's mbuf dropped.
                mopt = n;
                oldoptlen
            } else {
                let at = mopt.m_len().get() as usize;
                mopt.m_len().set((at + JUMBOOPTLEN) as u32);
                at
            };
            let d = writable(&mut mopt);
            d[optbuf] = IP6OPT_PADN;
            d[optbuf + 1] = 0;

            // Adjust the header length according to the pad and the jumbo payload option.
            let lenp = offset_of!(Ip6Hbh, ip6h_len);
            d[lenp] = d[lenp].wrapping_add((JUMBOOPTLEN >> 3) as u8);
            exthdrs.ip6e_hbh = Some(mopt);
            optbuf
        }
    };

    // fill in the option.
    if let Some(mopt) = exthdrs.ip6e_hbh.as_mut() {
        let d = writable(mopt);
        d[optbuf + 2] = IP6OPT_JUMBO;
        d[optbuf + 3] = 4;
        d[optbuf + 4..optbuf + 8].copy_from_slice(&(plen + JUMBOOPTLEN as u32).to_be_bytes());
    }

    // finally, adjust the packet header length
    let ph = ip6m.m_pkthdr();
    ph.len.set(ph.len.get() + JUMBOOPTLEN as i32);

    Ok(())
}

/// The data of an mbuf this file made (writable): its `m_len` bytes.
fn writable(m: &mut Mbuf) -> &mut [u8] {
    match m.data_mut() {
        Some(d) => d,
        None => kpanic(format_args!("ip6_output: a fresh mbuf is read-only")),
    }
}

/// `ip6_insertfraghdr(m0, m, hlen, &frghdrp)`: inserts the fragment header `frag` into
/// fragment `m` after a copy of the unfragmentable header portions (`hlen` bytes) of `m0`:
/// written in the trailing space of the last mbuf when that mbuf keeps its data in itself,
/// else in an mbuf of its own, where the C's `*frghdrp` points. The copy of the headers
/// shares `m0`'s clusters (`m_copym`) and is never written.
fn ip6_insertfraghdr(m0: &mut Mbuf, m: &mut Mbuf, hlen: i32, frag: Ip6Frag) -> Result<(), Errno> {
    if hlen as usize > IP6_HDR_LEN {
        let Some(n) = m_copym(
            m0,
            IP6_HDR_LEN as i32,
            hlen - IP6_HDR_LEN as i32,
            M_DONTWAIT,
        ) else {
            return Err(Errno::ENOBUFS);
        };
        drop(m.m_next.set(n));
    }

    // Search for the last mbuf of unfragmentable part.
    let ph = m.m_pkthdr().len.get();
    let mlast = last_mbuf(m);
    if mlast.flags() & M_EXT == 0 && m_trailingspace(mlast) as usize >= FRAG_LEN {
        // use the trailing space of the last mbuf for fragment hdr
        let at = mlast.m_len().get() as usize;
        mlast.m_len().set((at + FRAG_LEN) as u32);
        put_frag(writable(mlast), at, frag);
        m.m_pkthdr().len.set(ph + FRAG_LEN as i32);
    } else {
        // allocate a new mbuf for the fragment header
        let Some(mut mfrg) = m_get(M_DONTWAIT, MT_DATA) else {
            return Err(Errno::ENOBUFS);
        };
        mfrg.m_len().set(FRAG_LEN as u32);
        put_frag(writable(&mut mfrg), 0, frag);
        drop(mlast.m_next.set(mfrg));
    }

    Ok(())
}

/// The fragment header written at `at` of `d`, the room `ip6_insertfraghdr` just made.
fn put_frag(d: &mut [u8], at: usize, frag: Ip6Frag) {
    if !pod::put(d, at, &frag) {
        kpanic(format_args!(
            "ip6_insertfraghdr: no room for the fragment header"
        ));
    }
}

/// `ip6_getpmtu(rt, ifp, &mtu)`: the path MTU through route `rt` (if any) and `ifp` (the C's
/// error, never set, dropped).
pub fn ip6_getpmtu(rt: Option<&Rtentry>, ifp: &Ifnet) -> u64 {
    let mtu = match rt {
        Some(rt) => {
            let rtmtu = rt.rt_mtu().load(Ordering::Relaxed);
            let mut mtu = rtmtu;
            if mtu == 0 {
                mtu = ifp.if_mtu.get();
            } else if mtu < IPV6_MMTU {
                // RFC8021 IPv6 Atomic Fragments Considered Harmful
                mtu = IPV6_MMTU;
            } else if mtu > ifp.if_mtu.get() {
                // The MTU on the route is larger than the MTU on the interface! This shouldn't
                // happen, unless the MTU of the interface has been changed after the interface
                // was brought up. Change the MTU in the route to match the interface MTU (as
                // long as the field isn't locked).
                mtu = ifp.if_mtu.get();
                if rt.rt_locks().get() & RTV_MTU == 0 {
                    let _ = rt.rt_mtu().compare_exchange(
                        rtmtu,
                        mtu,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
                }
            }
            mtu
        }
        None => ifp.if_mtu.get(),
    };

    u64::from(mtu)
}

/// The `int` value of a socket option (`*mtod(m, int *)` once `m->m_len == sizeof(int)`):
/// `None` without a value or of another length.
#[inline]
fn opt_int(m: Option<&Mbuf>) -> Option<i32> {
    match m?.data() {
        &[a, b, c, d] => Some(i32::from_ne_bytes([a, b, c, d])),
        _ => None,
    }
}

/// The answer `b` written into the option mbuf `m` of a `getsockopt` (`m->m_len = len`, then
/// the value at `mtod(m)`), a cluster added first when it does not fit (`MCLGET(m, M_WAIT)`);
/// `EMSGSIZE` above a cluster (`XXX` in the C, which no IPv6 answer reaches).
fn opt_put(m: &mut Mbuf, b: &[u8]) -> Result<(), Errno> {
    if b.len() > MCLBYTES {
        return Err(Errno::EMSGSIZE); // XXX
    }
    if b.len() > MLEN {
        mclget(m, M_WAIT);
        if m.flags() & M_EXT == 0 {
            return Err(Errno::ENOBUFS);
        }
    }
    m.m_len().set(b.len() as u32);
    match m.data_mut() {
        Some(d) => d.copy_from_slice(b),
        None => kpanic(format_args!("ip6_ctloutput: a read-only option mbuf")),
    }
    Ok(())
}

/// `ip6_ctloutput(op, so, level, optname, m)`: IP6 socket option processing, on the control
/// block `inp` of the socket (`sotoinpcb(so)`) as `curproc` asks it, the net lock held
/// (`net`). `m` holds the value of a `PRCO_SETOPT` (`None` for none) and receives the answer
/// of a `PRCO_GETOPT`.
pub fn ip6_ctloutput<G: ProcGlue>(
    op: i32,
    inp: &Inpcb<G>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let so = inp.socket();
    let privileged = so.has_state(SS_PRIV);
    let uproto = i32::from(so.proto().protocol());

    if level != IPPROTO_IPV6 {
        return Err(Errno::EINVAL);
    }

    G::with_curproc(|p| {
        let Some(p) = p else {
            kpanic(format_args!("ip6_ctloutput: no curproc")); // For IPsec and rdomain
        };
        let rtableid = p.process().ps_rtableid.load(Ordering::Relaxed);
        match op {
            PRCO_SETOPT => ip6_setopt(
                inp,
                p,
                rtableid,
                privileged,
                uproto,
                optname,
                m.as_deref(),
                net,
            ),
            PRCO_GETOPT => match m {
                Some(m) => ip6_getopt(inp, optname, m, net),
                None => kpanic(format_args!("ip6_ctloutput: PRCO_GETOPT without an mbuf")),
            },
            _ => Ok(()),
        }
    })
}

/// `ip6_ctloutput`'s `PRCO_SETOPT` half.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's locals of ip6_ctloutput, passed down"
)]
fn ip6_setopt<G: ProcGlue>(
    inp: &Inpcb<G>,
    p: &Thread<G>,
    rtableid: u32,
    privileged: bool,
    uproto: i32,
    optname: i32,
    m: Option<&Mbuf>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    match optname {
        // Use of some Hop-by-Hop options or some Destination options, might require special
        // privilege. That is, normal applications (without special privilege) might be
        // forbidden from setting certain options in outgoing packets, and might never see
        // certain options in received packets. [RFC 2292 Section 6] KAME specific note: KAME
        // prevents non-privileged users from sending or receiving ANY hbh/dst options in
        // order to avoid overhead of parsing options in the kernel.
        IPV6_RECVHOPOPTS | IPV6_RECVDSTOPTS | IPV6_UNICAST_HOPS | IPV6_MINHOPCOUNT
        | IPV6_HOPLIMIT | IPV6_RECVPKTINFO | IPV6_RECVHOPLIMIT | IPV6_RECVRTHDR
        | IPV6_RECVPATHMTU | IPV6_RECVTCLASS | IPV6_V6ONLY | IPV6_AUTOFLOWLABEL
        | IPV6_RECVDSTPORT => {
            if (optname == IPV6_RECVHOPOPTS || optname == IPV6_RECVDSTOPTS) && !privileged {
                return Err(Errno::EPERM);
            }
            let Some(optval) = opt_int(m) else {
                return Err(Errno::EINVAL);
            };
            // OPTSET
            let optset = |bit: i32| {
                if optval != 0 {
                    inp.set_flags(bit);
                } else {
                    inp.clear_flags(bit);
                }
            };
            match optname {
                IPV6_UNICAST_HOPS => {
                    if !(-1..256).contains(&optval) {
                        return Err(Errno::EINVAL);
                    }
                    // -1 = kernel default
                    inp.inp_hops.set(optval);
                }
                IPV6_MINHOPCOUNT => {
                    if !(0..=255).contains(&optval) {
                        return Err(Errno::EINVAL);
                    }
                    inp.inp_ip6_minhlim().set(optval as u8);
                }
                IPV6_RECVPKTINFO => optset(IN6P_PKTINFO),
                IPV6_HOPLIMIT => {
                    return ip6_pcbopt(
                        inp,
                        IPV6_HOPLIMIT,
                        &optval.to_ne_bytes(),
                        privileged,
                        uproto,
                    );
                }
                IPV6_RECVHOPLIMIT => optset(IN6P_HOPLIMIT),
                IPV6_RECVHOPOPTS => optset(IN6P_HOPOPTS),
                IPV6_RECVDSTOPTS => optset(IN6P_DSTOPTS),
                IPV6_RECVRTHDR => optset(IN6P_RTHDR),
                IPV6_RECVPATHMTU => {
                    // We ignore this option for TCP sockets. (RFC3542 leaves this case
                    // unspecified.)
                    if uproto != IPPROTO_TCP {
                        optset(IN6P_MTU);
                    }
                }
                IPV6_V6ONLY => {
                    // make setsockopt(IPV6_V6ONLY) available only prior to bind(2). see ipng
                    // mailing list, Jun 22 2001.
                    if inp.inp_lport.get() != 0
                        || !in6_is_addr_unspecified(&inp.inp_laddr6.get::<In6Addr>())
                    {
                        return Err(Errno::EINVAL);
                    }
                    // No support for IPv4-mapped addresses.
                    if optval == 0 {
                        return Err(Errno::EINVAL);
                    }
                }
                IPV6_RECVTCLASS => optset(IN6P_TCLASS),
                IPV6_AUTOFLOWLABEL => optset(IN6P_AUTOFLOWLABEL),
                _ => optset(IN6P_RECVDSTPORT), // IPV6_RECVDSTPORT
            }
            Ok(())
        }

        IPV6_TCLASS | IPV6_DONTFRAG | IPV6_USE_MIN_MTU => {
            let Some(optval) = opt_int(m) else {
                return Err(Errno::EINVAL);
            };
            ip6_pcbopt(inp, optname, &optval.to_ne_bytes(), privileged, uproto)
        }

        IPV6_PKTINFO | IPV6_HOPOPTS | IPV6_RTHDR | IPV6_DSTOPTS | IPV6_RTHDRDSTOPTS => {
            // new advanced API (RFC3542)
            if m.is_some_and(|m| m.m_next().is_some()) {
                return Err(Errno::EINVAL); // XXX
            }
            let optbuf: &[u8] = m.map_or(&[], Mbuf::data);
            ip6_pcbopt(inp, optname, optbuf, privileged, uproto)
        }

        IPV6_MULTICAST_IF | IPV6_MULTICAST_HOPS | IPV6_MULTICAST_LOOP | IPV6_JOIN_GROUP
        | IPV6_LEAVE_GROUP => ip6_setmoptions(
            optname,
            &mut inp.moptions6(),
            m,
            inp.inp_rtableid.get(),
            p,
            net,
        ),

        IPV6_PORTRANGE => {
            let Some(optval) = opt_int(m) else {
                return Err(Errno::EINVAL);
            };
            match optval {
                IPV6_PORTRANGE_DEFAULT => {
                    inp.clear_flags(IN6P_LOWPORT);
                    inp.clear_flags(IN6P_HIGHPORT);
                }
                IPV6_PORTRANGE_HIGH => {
                    inp.clear_flags(IN6P_LOWPORT);
                    inp.set_flags(IN6P_HIGHPORT);
                }
                IPV6_PORTRANGE_LOW => {
                    inp.clear_flags(IN6P_HIGHPORT);
                    inp.set_flags(IN6P_LOWPORT);
                }
                _ => return Err(Errno::EINVAL),
            }
            Ok(())
        }

        IPSEC6_OUTSA => Err(Errno::EINVAL),

        // IPSEC is configured.
        IPV6_AUTH_LEVEL | IPV6_ESP_TRANS_LEVEL | IPV6_ESP_NETWORK_LEVEL | IPV6_IPCOMP_LEVEL => {
            let Some(optval) = opt_int(m) else {
                return Err(Errno::EINVAL);
            };
            if !(IPSEC_LEVEL_BYPASS..=IPSEC_LEVEL_UNIQUE).contains(&optval) {
                return Err(Errno::EINVAL);
            }
            let default = match optname {
                IPV6_AUTH_LEVEL => IPSEC_AUTH_LEVEL_DEFAULT,
                IPV6_ESP_TRANS_LEVEL => IPSEC_ESP_TRANS_LEVEL_DEFAULT,
                IPV6_ESP_NETWORK_LEVEL => IPSEC_ESP_NETWORK_LEVEL_DEFAULT,
                _ => IPSEC_IPCOMP_LEVEL_DEFAULT, // IPV6_IPCOMP_LEVEL
            };
            // Below the system's default only for the superuser.
            if optval < default && suser(p).is_err() {
                return Err(Errno::EACCES);
            }
            let mut sl = inp.inp_seclevel.get();
            let level = optval as u8;
            match optname {
                IPV6_AUTH_LEVEL => sl.sl_auth = level,
                IPV6_ESP_TRANS_LEVEL => sl.sl_esp_trans = level,
                IPV6_ESP_NETWORK_LEVEL => sl.sl_esp_network = level,
                _ => sl.sl_ipcomp = level, // IPV6_IPCOMP_LEVEL
            }
            inp.inp_seclevel.set(sl);
            Ok(())
        }

        SO_RTABLE => {
            let Some(&[a, b, c, d, ..]) = m.map(Mbuf::data) else {
                return Err(Errno::EINVAL);
            };
            let rtid = u32::from_ne_bytes([a, b, c, d]);
            if inp.inp_rtableid.get() == rtid {
                return Ok(());
            }
            // needs privileges to switch when already set
            if rtableid != rtid && rtableid != 0 {
                suser(p)?;
            }
            in_pcbset_rtableid(inp, rtid)
        }

        IPV6_PIPEX => match opt_int(m) {
            Some(v) => {
                inp.inp_pipex.set(v);
                Ok(())
            }
            None => Err(Errno::EINVAL),
        },

        _ => Err(Errno::ENOPROTOOPT),
    }
}

/// `ip6_ctloutput`'s `PRCO_GETOPT` half: the answer in `m`.
fn ip6_getopt<G: ProcGlue>(
    inp: &Inpcb<G>,
    optname: i32,
    m: &mut Mbuf,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    // OPTBIT
    let optbit = |bit: i32| i32::from(inp.has_flags(bit));
    let optval = match optname {
        IPV6_RECVHOPOPTS => optbit(IN6P_HOPOPTS),
        IPV6_RECVDSTOPTS => optbit(IN6P_DSTOPTS),
        IPV6_UNICAST_HOPS => inp.inp_hops.get(),
        IPV6_MINHOPCOUNT => i32::from(inp.inp_ip6_minhlim().get()),
        IPV6_RECVPKTINFO => optbit(IN6P_PKTINFO),
        IPV6_RECVHOPLIMIT => optbit(IN6P_HOPLIMIT),
        IPV6_RECVRTHDR => optbit(IN6P_RTHDR),
        IPV6_RECVPATHMTU => optbit(IN6P_MTU),
        IPV6_V6ONLY => 1,
        IPV6_PORTRANGE => {
            if inp.has_flags(IN6P_HIGHPORT) {
                IPV6_PORTRANGE_HIGH
            } else if inp.has_flags(IN6P_LOWPORT) {
                IPV6_PORTRANGE_LOW
            } else {
                0
            }
        }
        IPV6_RECVTCLASS => optbit(IN6P_TCLASS),
        IPV6_AUTOFLOWLABEL => optbit(IN6P_AUTOFLOWLABEL),
        IPV6_RECVDSTPORT => optbit(IN6P_RECVDSTPORT),

        IPV6_PATHMTU => {
            if !inp.socket().has_state(SS_ISCONNECTED) {
                return Err(Errno::ENOTCONN);
            }
            // `rt = in6_pcbrtentry(inp)`: the pcb's cached route, lent with its cache.
            let faddr = inp.inp_faddr6.get::<In6Addr>();
            let laddr = inp.inp_laddr6.get::<In6Addr>();
            let mut ro = inp.route();
            let rt = if in6_is_addr_unspecified(&faddr) {
                None
            } else {
                ro.mpath6(
                    &faddr.s6_addr,
                    Some(&laddr.s6_addr),
                    inp.inp_rtableid.get(),
                    net,
                )
            };
            let Some(rt) = rt.filter(|rt| rtisvalid(Some(rt))) else {
                return Err(Errno::EHOSTUNREACH);
            };
            let Some(ifp) = IfRef::get(rt.rt_ifidx.get()) else {
                return Err(Errno::EHOSTUNREACH);
            };
            // XXX: we dot not consider the case of source routing, or optional information to
            // specify the outgoing interface.
            let mut pmtu = ip6_getpmtu(Some(rt), &ifp);
            drop(ifp); // if_put(ifp)
            drop(ro);
            if pmtu > IPV6_MAXPACKET as u64 {
                pmtu = IPV6_MAXPACKET as u64;
            }
            let mtuinfo = Ip6Mtuinfo {
                ip6m_mtu: pmtu as u32,
                ..Ip6Mtuinfo::default()
            };
            let mut b = [0u8; size_of::<Ip6Mtuinfo>()];
            let _ = pod::put(&mut b, 0, &mtuinfo);
            return opt_put(m, &b);
        }

        IPV6_PKTINFO | IPV6_HOPOPTS | IPV6_RTHDR | IPV6_DSTOPTS | IPV6_RTHDRDSTOPTS
        | IPV6_TCLASS | IPV6_DONTFRAG | IPV6_USE_MIN_MTU => {
            return ip6_getpcbopt(inp.outputopts6().as_deref(), optname, m);
        }

        IPV6_MULTICAST_IF | IPV6_MULTICAST_HOPS | IPV6_MULTICAST_LOOP | IPV6_JOIN_GROUP
        | IPV6_LEAVE_GROUP => {
            return ip6_getmoptions(optname, inp.moptions6_out().as_ref(), m);
        }

        IPSEC6_OUTSA => return Err(Errno::EINVAL),

        // IPSEC is configured.
        IPV6_AUTH_LEVEL | IPV6_ESP_TRANS_LEVEL | IPV6_ESP_NETWORK_LEVEL | IPV6_IPCOMP_LEVEL => {
            let sl = inp.inp_seclevel.get();
            i32::from(match optname {
                IPV6_AUTH_LEVEL => sl.sl_auth,
                IPV6_ESP_TRANS_LEVEL => sl.sl_esp_trans,
                IPV6_ESP_NETWORK_LEVEL => sl.sl_esp_network,
                _ => sl.sl_ipcomp, // IPV6_IPCOMP_LEVEL
            })
        }
        SO_RTABLE => {
            return opt_put(m, &inp.inp_rtableid.get().to_ne_bytes());
        }
        IPV6_PIPEX => inp.inp_pipex.get(),

        _ => return Err(Errno::ENOPROTOOPT),
    };
    opt_put(m, &optval.to_ne_bytes())
}

/// `ip6_raw_ctloutput(op, so, level, optname, m)`: the raw-socket IPv6 options
/// (`IPV6_CHECKSUM`), on the control block `inp` of the socket.
pub fn ip6_raw_ctloutput<G: ProcGlue>(
    op: i32,
    inp: &Inpcb<G>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
) -> Result<(), Errno> {
    let icmp6off = ICMP6_CKSUM_OFFSET;
    let icmp6 = i32::from(inp.socket().proto().protocol()) == IPPROTO_ICMPV6;

    if level != IPPROTO_IPV6 {
        return Err(Errno::EINVAL);
    }

    match optname {
        IPV6_CHECKSUM => {
            // For ICMPv6 sockets, no modification allowed for checksum offset, permit "no
            // change" values to help existing apps.
            //
            // RFC3542 says: "An attempt to set IPV6_CHECKSUM for an ICMPv6 socket will fail."
            // The current behavior does not meet RFC3542.
            match op {
                PRCO_SETOPT => {
                    let Some(optval) = opt_int(m.as_deref()) else {
                        return Err(Errno::EINVAL);
                    };
                    if optval < -1 || (optval > 0 && optval % 2 != 0) {
                        // The API assumes non-negative even offset values or -1 as a special
                        // value.
                        Err(Errno::EINVAL)
                    } else if icmp6 {
                        if optval == icmp6off {
                            Ok(())
                        } else {
                            Err(Errno::EINVAL)
                        }
                    } else {
                        inp.inp_cksum6.set(optval);
                        Ok(())
                    }
                }

                PRCO_GETOPT => {
                    let Some(m) = m else {
                        kpanic(format_args!(
                            "ip6_raw_ctloutput: PRCO_GETOPT without an mbuf"
                        ));
                    };
                    let optval = if icmp6 {
                        icmp6off
                    } else {
                        inp.inp_cksum6.get()
                    };
                    opt_put(m, &optval.to_ne_bytes())
                }

                _ => Err(Errno::EINVAL),
            }
        }

        _ => Err(Errno::ENOPROTOOPT),
    }
}

/// `ip6_initpktopts(opt)`: initializes packet options. Beware that there are non-zero default
/// values in the struct (`Ip6Pktopts::new`'s); the old options' allocations are given back.
pub fn ip6_initpktopts(opt: &mut Ip6Pktopts) {
    *opt = Ip6Pktopts::new();
}

/// `ip6_pcbopt(optname, buf, len, &inp->inp_outputopts6, priv, uproto)`: sets sticky option
/// `optname` from `buf` in the pcb's packet options, allocating them on first use.
fn ip6_pcbopt<G: ProcGlue>(
    inp: &Inpcb<G>,
    optname: i32,
    buf: &[u8],
    priv_: bool,
    uproto: i32,
) -> Result<(), Errno> {
    let mut pktopt = inp.outputopts6();
    if pktopt.is_none() {
        let Some(o) = MBox::new(M_IP6OPT, M_WAITOK, Ip6Pktopts::new()) else {
            kpanic(format_args!("ip6_pcbopt: malloc(M_WAITOK)"));
        };
        *pktopt = Some(o);
    }
    let Some(opt) = pktopt.as_deref_mut() else {
        kpanic(format_args!("ip6_pcbopt: the options just made are gone"));
    };

    ip6_setpktopt(optname, buf, opt, priv_, true, uproto)
}

/// `ip6_getpcbopt(pktopt, optname, m)`: copies sticky option `optname` of `pktopt` (the
/// defaults without options) into option mbuf `m`.
fn ip6_getpcbopt(pktopt: Option<&Ip6Pktopts>, optname: i32, m: &mut Mbuf) -> Result<(), Errno> {
    let deftclass: i32 = 0;
    let defminmtu: i32 = IP6PO_MINMTU_MCASTONLY;
    let intbuf: [u8; size_of::<i32>()];
    let mut pktinfobuf = [0u8; size_of::<In6Pktinfo>()];

    let optdata: &[u8] = match optname {
        IPV6_PKTINFO => {
            // XXX: we don't have to do this every time...
            let pi = pktopt.and_then(Ip6Pktopts::pktinfo).unwrap_or_default();
            let _ = pod::put(&mut pktinfobuf, 0, &pi);
            &pktinfobuf
        }
        IPV6_TCLASS => {
            let v = match pktopt {
                Some(o) if o.ip6po_tclass >= 0 => o.ip6po_tclass,
                _ => deftclass,
            };
            intbuf = v.to_ne_bytes();
            &intbuf
        }
        IPV6_HOPOPTS => pktopt.and_then(|o| o.ip6po_hbh.as_deref()).unwrap_or(&[]),
        IPV6_RTHDR => &[],
        IPV6_RTHDRDSTOPTS => pktopt.and_then(|o| o.ip6po_dest1.as_deref()).unwrap_or(&[]),
        IPV6_DSTOPTS => pktopt.and_then(|o| o.ip6po_dest2.as_deref()).unwrap_or(&[]),
        IPV6_USE_MIN_MTU => {
            intbuf = pktopt.map_or(defminmtu, |o| o.ip6po_minmtu).to_ne_bytes();
            &intbuf
        }
        IPV6_DONTFRAG => {
            let on = i32::from(pktopt.is_some_and(|o| o.ip6po_flags & IP6PO_DONTFRAG != 0));
            intbuf = on.to_ne_bytes();
            &intbuf
        }
        _ => {
            // should not happen
            #[cfg(feature = "diagnostic")]
            kpanic(format_args!("ip6_getpcbopt: unexpected option"));
            #[allow(unreachable_code, reason = "with `diagnostic`, the panic above")]
            return Err(Errno::ENOPROTOOPT);
        }
    };

    opt_put(m, optdata)
}

/// `ip6_clearpktopts(pktopt, optname)`: frees and clears option `optname` of `pktopt` (-1:
/// all of them).
pub fn ip6_clearpktopts(pktopt: &mut Ip6Pktopts, optname: i32) {
    if optname == -1 || optname == IPV6_PKTINFO {
        pktopt.ip6po_pktinfo = None;
    }
    if optname == -1 || optname == IPV6_HOPLIMIT {
        pktopt.ip6po_hlim = -1;
    }
    if optname == -1 || optname == IPV6_TCLASS {
        pktopt.ip6po_tclass = -1;
    }
    if optname == -1 || optname == IPV6_HOPOPTS {
        pktopt.ip6po_hbh = None;
    }
    if optname == -1 || optname == IPV6_RTHDRDSTOPTS {
        pktopt.ip6po_dest1 = None;
    }
    if optname == -1 || optname == IPV6_DSTOPTS {
        pktopt.ip6po_dest2 = None;
    }
}

/// `PKTOPT_EXTHDRCPY`: a `malloc(M_IP6OPT, M_NOWAIT)` copy of an extension header, if any;
/// `Err` when the allocation fails.
fn pktopt_exthdrcpy(src: Option<&[u8]>) -> Result<Option<MArray<u8>>, ()> {
    let Some(src) = src else {
        return Ok(None);
    };
    MArray::new(M_IP6OPT, M_NOWAIT, src.len(), |i| src[i])
        .map(Some)
        .ok_or(())
}

/// `copypktopts(dst, src)`: copies `src` into `dst` (initialized), duplicating its
/// allocations; `ENOBUFS`, `dst`'s options all cleared, when one cannot be had.
fn copypktopts(dst: &mut Ip6Pktopts, src: &Ip6Pktopts) -> Result<(), Errno> {
    dst.ip6po_hlim = src.ip6po_hlim;
    dst.ip6po_tclass = src.ip6po_tclass;
    dst.ip6po_flags = src.ip6po_flags;
    let copied: Result<(), ()> = (|| {
        if let Some(pi) = src.ip6po_pktinfo.as_deref() {
            dst.ip6po_pktinfo = Some(MBox::new(M_IP6OPT, M_NOWAIT, *pi).ok_or(())?);
        }
        dst.ip6po_hbh = pktopt_exthdrcpy(src.ip6po_hbh.as_deref())?;
        dst.ip6po_dest1 = pktopt_exthdrcpy(src.ip6po_dest1.as_deref())?;
        dst.ip6po_dest2 = pktopt_exthdrcpy(src.ip6po_dest2.as_deref())?;
        Ok(())
    })();
    if copied.is_err() {
        // bad:
        ip6_clearpktopts(dst, -1);
        return Err(Errno::ENOBUFS);
    }
    Ok(())
}

/// `ip6_freepcbopts(pktopt)`: frees a pcb's packet options (`ip6_clearpktopts(pktopt, -1)`
/// and the structure: the box's drop).
pub fn ip6_freepcbopts(pktopt: Option<MBox<Ip6Pktopts>>) {
    drop(pktopt);
}

/// `ip6_setmoptions(optname, im6op, m, rtableid)`: sets the IP6 multicast options of a socket
/// (`*im6op`, made here with the C's defaults when it has none) in response to user
/// setsockopt(), and frees them once every option is back to its default. `p` is `curproc`
/// (the superuser test of a join to every group).
pub fn ip6_setmoptions<G: ProcGlue>(
    optname: i32,
    im6op: &mut Option<MBox<Ip6Moptions>>,
    m: Option<&Mbuf>,
    rtableid: u32,
    p: &Thread<G>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let ip6_defmcasthlim_local = IP6_DEFMCASTHLIM.load(Ordering::Relaxed);

    let im6o = if let Some(im6o) = im6op {
        im6o
    } else {
        // No multicast option buffer attached to the pcb; allocate one and initialize to
        // default values.
        let Some(im6o) = MBox::new(
            M_IPMOPTS,
            M_WAITOK,
            Ip6Moptions::output(
                0,
                ip6_defmcasthlim_local as u8,
                IPV6_DEFAULT_MULTICAST_LOOP as u8,
            ),
        ) else {
            kpanic(format_args!("ip6_setmoptions: malloc(M_WAITOK)"));
        };
        im6op.insert(im6o)
    };

    let error = match optname {
        IPV6_MULTICAST_IF => set_multicast_if(im6o, m, rtableid),

        IPV6_MULTICAST_HOPS => {
            // Set the IP6 hoplimit for outgoing multicast packets.
            match opt_int(m) {
                Some(v) if (-1..256).contains(&v) => {
                    im6o.im6o_hlim = if v == -1 {
                        ip6_defmcasthlim_local as u8
                    } else {
                        v as u8
                    };
                    Ok(())
                }
                _ => Err(Errno::EINVAL),
            }
        }

        IPV6_MULTICAST_LOOP => {
            // Set the loopback flag for outgoing multicast packets. Must be zero or one.
            match opt_int(m).map(|v| v as u32) {
                Some(lp) if lp <= 1 => {
                    im6o.im6o_loop = lp as u8;
                    Ok(())
                }
                _ => Err(Errno::EINVAL),
            }
        }

        IPV6_JOIN_GROUP => join_group(im6o, m, rtableid, p, net),
        IPV6_LEAVE_GROUP => leave_group(im6o, m, p, net),

        _ => Err(Errno::EOPNOTSUPP),
    };

    // If all options have default values, no need to keep the option structure.
    if im6o.im6o_ifidx == 0
        && i32::from(im6o.im6o_hlim) == ip6_defmcasthlim_local
        && i32::from(im6o.im6o_loop) == IPV6_DEFAULT_MULTICAST_LOOP
        && im6o.im6o_memberships.is_empty()
    {
        // free(*im6op, M_IPMOPTS, sizeof(**im6op))
        drop(im6op.take());
    }

    error
}

/// `ip6_setmoptions`' `IPV6_MULTICAST_IF`: selects the interface for outgoing multicast
/// packets by its index (0: none).
fn set_multicast_if(im6o: &mut Ip6Moptions, m: Option<&Mbuf>, rtableid: u32) -> Result<(), Errno> {
    let Some(ifindex) = opt_int(m).map(|v| v as u32) else {
        return Err(Errno::EINVAL);
    };
    if ifindex != 0 {
        let Some(ifp) = IfRef::get(ifindex) else {
            return Err(Errno::ENXIO); // XXX EINVAL?
        };
        if ifp.if_rdomain.get() != rtable_l2(rtableid) || ifp.if_flags.get() & IFF_MULTICAST == 0 {
            return Err(Errno::EADDRNOTAVAIL);
        }
        // if_put(ifp): the reference's drop.
    }
    im6o.im6o_ifidx = ifindex as u16;
    Ok(())
}

/// The `struct ipv6_mreq` of a join or leave, `EINVAL` without one of its size; a request for
/// every group (the unspecified address) is the superuser's (`EACCES`), and any other must be
/// a multicast address (`EINVAL`).
fn membership_request<G: ProcGlue>(m: Option<&Mbuf>, p: &Thread<G>) -> Result<Ipv6Mreq, Errno> {
    let Some(mreq) = m
        .map(Mbuf::data)
        .filter(|d| d.len() == size_of::<Ipv6Mreq>())
        .and_then(|d| pod::at::<Ipv6Mreq>(d, 0))
    else {
        return Err(Errno::EINVAL);
    };
    if in6_is_addr_unspecified(&mreq.ipv6mr_multiaddr) {
        // We use the unspecified address to specify to accept all multicast addresses. Only
        // super user is allowed to do this.
        if suser(p).is_err() {
            return Err(Errno::EACCES);
        }
    } else if !in6_is_addr_multicast(&mreq.ipv6mr_multiaddr) {
        return Err(Errno::EINVAL);
    }
    Ok(mreq)
}

/// `ip6_setmoptions`' `IPV6_JOIN_GROUP`: adds a multicast group membership.
fn join_group<G: ProcGlue>(
    im6o: &mut Ip6Moptions,
    m: Option<&Mbuf>,
    rtableid: u32,
    p: &Thread<G>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mut mreq = membership_request(m, p)?;

    // If no interface was explicitly specified, choose an appropriate one according to the
    // given multicast address.
    let ifp = if mreq.ipv6mr_interface == 0 {
        let dst = SockaddrIn6::with_addr(mreq.ipv6mr_multiaddr);
        let mut b = [0u8; SIN6_LEN];
        let _ = pod::put(&mut b, 0, &dst);
        let Some(dsa) = SockaddrView::new(&b) else {
            return Err(Errno::EADDRNOTAVAIL);
        };
        let Some(rt) = rtalloc(dsa, RT_RESOLVE, rtableid, net) else {
            return Err(Errno::EADDRNOTAVAIL);
        };
        let ifp = IfRef::get(rt.rt_ifidx.get());
        drop(rt); // rtfree(rt)
        ifp
    } else {
        // If the interface is specified, validate it.
        let Some(ifp) = IfRef::get(mreq.ipv6mr_interface) else {
            return Err(Errno::ENXIO); // XXX EINVAL?
        };
        Some(ifp)
    };

    // See if we found an interface, and confirm that it supports multicast
    let Some(ifp) = ifp.filter(|ifp| {
        ifp.if_rdomain.get() == rtable_l2(rtableid) && ifp.if_flags.get() & IFF_MULTICAST != 0
    }) else {
        return Err(Errno::EADDRNOTAVAIL);
    };
    // Put interface index into the multicast address, if the address has link/interface-local
    // scope.
    if in6_is_scope_embed(&mreq.ipv6mr_multiaddr) {
        mreq.ipv6mr_multiaddr
            .set_s6_addr16(1, htons(ifp.if_index.get() as u16));
    }
    // See if the membership already exists.
    let group = mreq.ipv6mr_multiaddr;
    let ifidx = ifp.if_index.get();
    if im6o.im6o_memberships.iter().any(|in6m| {
        in6m.in6m_ifidx().get() == ifidx && in6_are_addr_equal(&in6m.in6m_addr(), &group)
    }) {
        return Err(Errno::EADDRINUSE);
    }
    // Everything looks good; add a new record to the multicast address list for the given
    // interface (`in6_joingroup`: the membership's room first, as the C's malloc of it, then
    // `in6_addmulti`; `LIST_INSERT_HEAD` the newest).
    im6o.im6o_memberships.reserve()?;
    let in6m = in6_addmulti(&group, ifp.ifp(), net)?;
    drop(ifp); // if_put(ifp)
    if let Err(in6m) = im6o.im6o_memberships.push(in6m) {
        in6_delmulti(in6m, net);
        return Err(Errno::ENOBUFS);
    }
    Ok(())
}

/// `ip6_setmoptions`' `IPV6_LEAVE_GROUP`: drops a multicast group membership, on the
/// interface the request names (any, when it names none), the newest matching one.
fn leave_group<G: ProcGlue>(
    im6o: &mut Ip6Moptions,
    m: Option<&Mbuf>,
    p: &Thread<G>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mut mreq = membership_request(m, p)?;

    // Put interface index into the multicast address, if the address has link-local scope.
    if in6_is_addr_mc_linklocal(&mreq.ipv6mr_multiaddr) {
        mreq.ipv6mr_multiaddr
            .set_s6_addr16(1, htons(mreq.ipv6mr_interface as u16));
    }

    // If an interface address was specified, get a pointer to its ifnet structure.
    let ifidx = if mreq.ipv6mr_interface == 0 {
        None
    } else {
        let Some(ifp) = IfRef::get(mreq.ipv6mr_interface) else {
            return Err(Errno::ENXIO); // XXX EINVAL?
        };
        Some(ifp.if_index.get())
        // if_put(ifp): the reference's drop.
    };

    // Find the membership in the membership list.
    let group = mreq.ipv6mr_multiaddr;
    let Some(in6m) = im6o.im6o_memberships.take_newest(|in6m| {
        ifidx.is_none_or(|i| in6m.in6m_ifidx().get() == i)
            && in6_are_addr_equal(&in6m.in6m_addr(), &group)
    }) else {
        // Unable to resolve interface
        return Err(Errno::EADDRNOTAVAIL);
    };
    // Give up the multicast address record to which the membership points.
    in6_delmulti(in6m, net);
    Ok(())
}

/// `ip6_getmoptions(optname, im6o, m)`: returns the IP6 multicast options of a socket (`im6o`,
/// `None` when it has none: the defaults) in response to user getsockopt().
pub fn ip6_getmoptions(
    optname: i32,
    im6o: Option<&Ip6Moptions>,
    m: &mut Mbuf,
) -> Result<(), Errno> {
    let defhlim = IP6_DEFMCASTHLIM.load(Ordering::Relaxed) as u32;
    let v: u32 = match optname {
        IPV6_MULTICAST_IF => im6o.map_or(0, |im6o| u32::from(im6o.im6o_ifidx)),
        IPV6_MULTICAST_HOPS => im6o.map_or(defhlim, |im6o| u32::from(im6o.im6o_hlim)),
        // The C answers ip6_defmcasthlim without options (not IPV6_DEFAULT_MULTICAST_LOOP).
        IPV6_MULTICAST_LOOP => im6o.map_or(defhlim, |im6o| u32::from(im6o.im6o_loop)),
        _ => return Err(Errno::EOPNOTSUPP),
    };
    opt_put(m, &v.to_ne_bytes())
}

/// `ip6_freemoptions(im6o)`: discards the IP6 multicast options of a socket, leaving its
/// groups (the newest first, the C's list from its head).
pub fn ip6_freemoptions(im6o: Option<MBox<Ip6Moptions>>, net: &NetHeld<'_>) {
    let Some(mut im6o) = im6o else {
        return;
    };
    while let Some(in6m) = im6o.im6o_memberships.pop_last() {
        in6_delmulti(in6m, net);
    }
    // free(im6o, M_IPMOPTS, sizeof(*im6o))
    drop(im6o);
}

/// `ip6_setpktopts(control, opt, stickyopt, priv, uproto)`: the IPv6 outgoing packet options
/// of the control messages in `control` into `opt`, starting from the sticky options
/// `stickyopt`; `priv_`: privileged socket, `uproto`: the upper-layer protocol.
pub fn ip6_setpktopts(
    control: &Mbuf,
    opt: &mut Ip6Pktopts,
    stickyopt: Option<&Ip6Pktopts>,
    priv_: bool,
    uproto: i32,
) -> Result<(), Errno> {
    ip6_initpktopts(opt);
    if let Some(stickyopt) = stickyopt {
        // If stickyopt is provided, make a local copy of the options for this particular
        // packet, then override them by ancillary objects.
        // XXX: copypktopts() does not copy the cached route to a next hop (if any). This is
        // not very good in terms of efficiency, but we can allow this since this option
        // should be rarely used.
        copypktopts(opt, stickyopt)?;
    }

    // XXX: Currently, we assume all the optional information is stored in a single mbuf.
    if control.m_next().is_some() {
        return Err(Errno::EINVAL);
    }

    let cmsgs = control.data();
    let hdrlen = cmsg_len(0);
    let mut off = 0usize;
    let mut clen = cmsgs.len();
    loop {
        if clen < hdrlen {
            return Err(Errno::EINVAL);
        }
        let Some(cm) = pod::at::<Cmsghdr>(cmsgs, off) else {
            return Err(Errno::EINVAL);
        };
        let cmlen = cm.cmsg_len as usize;
        if cmlen < hdrlen || cmlen > clen || cmsg_align(cmlen) > clen {
            return Err(Errno::EINVAL);
        }
        if cm.cmsg_level == IPPROTO_IPV6 {
            // CMSG_DATA(cm), cm->cmsg_len - CMSG_LEN(0) bytes.
            let data = &cmsgs[off + hdrlen..off + cmlen];
            ip6_setpktopt(cm.cmsg_type, data, opt, priv_, false, uproto)?;
        }

        clen -= cmsg_align(cmlen);
        off += cmsg_align(cmlen);
        if clen == 0 {
            break;
        }
    }

    Ok(())
}

/// The `int` at the start of option value `buf` (its length was checked).
#[inline]
fn buf_int(buf: &[u8]) -> i32 {
    match buf {
        &[a, b, c, d, ..] => i32::from_ne_bytes([a, b, c, d]),
        _ => 0,
    }
}

/// `ip6_setpktopt(optname, buf, len, opt, priv, sticky, uproto)`: sets a particular packet
/// option from `buf`, as a sticky option or an ancillary data item. `buf` can be empty only
/// when it's a sticky option.
pub fn ip6_setpktopt(
    optname: i32,
    buf: &[u8],
    opt: &mut Ip6Pktopts,
    priv_: bool,
    sticky: bool,
    uproto: i32,
) -> Result<(), Errno> {
    let len = buf.len();
    match optname {
        IPV6_PKTINFO => {
            if len != size_of::<In6Pktinfo>() {
                return Err(Errno::EINVAL);
            }
            let Some(pktinfo) = pod::at::<In6Pktinfo>(buf, 0) else {
                return Err(Errno::EINVAL);
            };

            // An application can clear any sticky IPV6_PKTINFO option by doing a "regular"
            // setsockopt with ipi6_addr being in6addr_any and ipi6_ifindex being zero.
            // [RFC 3542, Section 6]
            if opt.ip6po_pktinfo.is_some()
                && pktinfo.ipi6_ifindex == 0
                && in6_is_addr_unspecified(&pktinfo.ipi6_addr)
            {
                ip6_clearpktopts(opt, optname);
                return Ok(());
            }

            if uproto == IPPROTO_TCP && sticky && !in6_is_addr_unspecified(&pktinfo.ipi6_addr) {
                return Err(Errno::EINVAL);
            }

            if pktinfo.ipi6_ifindex != 0 && IfRef::get(pktinfo.ipi6_ifindex).is_none() {
                return Err(Errno::ENXIO);
            }

            // We store the address anyway, and let in6_selectsrc() validate the specified
            // address. This is because ipi6_addr may not have enough information about its
            // scope zone, and we may need additional information (such as outgoing interface
            // or the scope zone of a destination address) to disambiguate the scope.
            // XXX: the delay of the validation may confuse the application when it is used as
            // a sticky option.
            if let Some(p) = opt.ip6po_pktinfo.as_mut() {
                **p = pktinfo;
            } else {
                let Some(p) = MBox::new(M_IP6OPT, M_NOWAIT, pktinfo) else {
                    return Err(Errno::ENOBUFS);
                };
                opt.ip6po_pktinfo = Some(p);
            }
        }

        IPV6_HOPLIMIT => {
            // RFC 3542 deprecated the usage of sticky IPV6_HOPLIMIT to simplify the ordering
            // among hoplimit options.
            if sticky {
                return Err(Errno::ENOPROTOOPT);
            }

            if len != size_of::<i32>() {
                return Err(Errno::EINVAL);
            }
            let hlim = buf_int(buf);
            if !(-1..=255).contains(&hlim) {
                return Err(Errno::EINVAL);
            }

            opt.ip6po_hlim = hlim;
        }

        IPV6_TCLASS => {
            if len != size_of::<i32>() {
                return Err(Errno::EINVAL);
            }
            let tclass = buf_int(buf);
            if !(-1..=255).contains(&tclass) {
                return Err(Errno::EINVAL);
            }

            opt.ip6po_tclass = tclass;
        }

        IPV6_HOPOPTS => {
            // XXX: We don't allow a non-privileged user to set ANY HbH options, since
            // per-option restriction has too much overhead.
            if !priv_ {
                return Err(Errno::EPERM);
            }

            if len == 0 {
                ip6_clearpktopts(opt, IPV6_HOPOPTS);
                return Ok(()); // just remove the option
            }

            // message length validation
            if len < size_of::<Ip6Hbh>() {
                return Err(Errno::EINVAL);
            }
            let hbhlen = (usize::from(buf[offset_of!(Ip6Hbh, ip6h_len)]) + 1) << 3;
            if len != hbhlen {
                return Err(Errno::EINVAL);
            }

            // turn off the previous option, then set the new option.
            ip6_clearpktopts(opt, IPV6_HOPOPTS);
            let Some(h) = MArray::new(M_IP6OPT, M_NOWAIT, hbhlen, |i| buf[i]) else {
                return Err(Errno::ENOBUFS);
            };
            opt.ip6po_hbh = Some(h);
        }

        IPV6_DSTOPTS | IPV6_RTHDRDSTOPTS => {
            if !priv_ {
                // XXX: see the comment for IPV6_HOPOPTS
                return Err(Errno::EPERM);
            }

            if len == 0 {
                ip6_clearpktopts(opt, optname);
                return Ok(()); // just remove the option
            }

            // message length validation
            if len < size_of::<Ip6Dest>() {
                return Err(Errno::EINVAL);
            }
            let destlen = (usize::from(buf[offset_of!(Ip6Dest, ip6d_len)]) + 1) << 3;
            if len != destlen {
                return Err(Errno::EINVAL);
            }

            // turn off the previous option, then set the new option.
            ip6_clearpktopts(opt, optname);
            let Some(d) = MArray::new(M_IP6OPT, M_NOWAIT, destlen, |i| buf[i]) else {
                return Err(Errno::ENOBUFS);
            };
            // Determine the position that the destination options header should be inserted;
            // before or after the routing header.
            if optname == IPV6_RTHDRDSTOPTS {
                opt.ip6po_dest1 = Some(d);
            } else {
                opt.ip6po_dest2 = Some(d);
            }
        }

        IPV6_USE_MIN_MTU => {
            if len != size_of::<i32>() {
                return Err(Errno::EINVAL);
            }
            let minmtupolicy = buf_int(buf);
            if minmtupolicy != IP6PO_MINMTU_MCASTONLY
                && minmtupolicy != IP6PO_MINMTU_DISABLE
                && minmtupolicy != IP6PO_MINMTU_ALL
            {
                return Err(Errno::EINVAL);
            }
            opt.ip6po_minmtu = minmtupolicy;
        }

        IPV6_DONTFRAG => {
            if len != size_of::<i32>() {
                return Err(Errno::EINVAL);
            }

            if uproto == IPPROTO_TCP || buf_int(buf) == 0 {
                // we ignore this option for TCP sockets. (RFC3542 leaves this case
                // unspecified.)
                opt.ip6po_flags &= !IP6PO_DONTFRAG;
            } else {
                opt.ip6po_flags |= IP6PO_DONTFRAG;
            }
        }

        _ => return Err(Errno::ENOPROTOOPT),
    }

    Ok(())
}

/// `ip6_mloopback(ifp, m, dst)`: loops a copy of IP6 multicast packet `m` back to the input
/// queue of `ifp` (called from `ip6_output`), toward `dst` (a `sockaddr_in6`'s bytes).
pub fn ip6_mloopback(ifp: &'static Ifnet, m: &mut Mbuf, dst: &[u8; SIN6_LEN], net: &NetHeld<'_>) {
    // Duplicate the packet.
    let Some(mut copym) = m_copym(m, 0, M_COPYALL, M_NOWAIT) else {
        return;
    };

    // Make sure to deep-copy IPv6 header portion in case the data is in an mbuf cluster, so
    // that we can safely override the IPv6 header portion later.
    if copym.flags() & M_EXT != 0 || (copym.m_len().get() as usize) < IP6_HDR_LEN {
        let Some(c) = m_pullup(copym, IP6_HDR_LEN as i32) else {
            return;
        };
        copym = c;
    }

    #[cfg(feature = "diagnostic")]
    if (copym.m_len().get() as usize) < IP6_HDR_LEN {
        return;
    }

    let mut ip6 = ip6_load(&copym);
    if in6_is_scope_embed(&ip6.ip6_src) {
        ip6.ip6_src.set_s6_addr16(1, 0);
    }
    if in6_is_scope_embed(&ip6.ip6_dst) {
        ip6.ip6_dst.set_s6_addr16(1, 0);
    }
    if !ip6_store(&mut copym, &ip6) {
        return;
    }

    let _ = if_input_local(ifp, copym, dst[1], None, Some(net));
}

/// `ip6_splithdr(m, exthdrs)`: chops the IPv6 header off from the payload into an mbuf of its
/// own, the packet's first (the C's `exthdrs->ip6e_ip6`); `ENOBUFS`, the packet freed, when
/// no mbuf can be had.
fn ip6_splithdr(m: MbufBox) -> Result<MbufBox, Errno> {
    let mut m = m;
    if m.m_len().get() as usize > IP6_HDR_LEN {
        let Some(mut mh) = m_get(M_DONTWAIT, MT_HEADER) else {
            return Err(Errno::ENOBUFS);
        };
        let ip6 = ip6_load(&m);
        m_move_pkthdr(&mut mh, &mut m);
        m_align(&mh, IP6_HDR_LEN as i32);
        m.m_len().set(m.m_len().get() - IP6_HDR_LEN as u32);
        m.data_add(IP6_HDR_LEN as isize);
        mh.m_len().set(IP6_HDR_LEN as u32);
        if !ip6_store(&mut mh, &ip6) {
            return Err(Errno::ENOBUFS);
        }
        drop(mh.m_next.set(m));
        m = mh;
    }
    Ok(m)
}

/// `ip6_randomid()`: a random, non-repeating fragment identification (`idgen32`, through the
/// `Inet` leg: the module's deviations). `panic(9)` before the legs are registered, which
/// `bsd`'s boot does before any interface exists.
pub fn ip6_randomid() -> u32 {
    match inet() {
        Some(i) => i.ip6_randomid(),
        None => kpanic(format_args!(
            "ip6_randomid: no idgen32 before the Inet legs"
        )),
    }
}

/// `in6_cksum_phdr(src, dst, len, nxt)`: the significant parts of the IPv6 checksum
/// pseudo-header for use in a delayed TCP/UDP checksum calculation; `len` and `nxt` in network
/// order. The embedded scope words of `src` and `dst` are left out.
#[inline]
fn in6_cksum_phdr(src: &In6Addr, dst: &In6Addr, len: u32, nxt: u32) -> u16 {
    let mut sum: u32 = 0;

    for a in [src, dst] {
        let embed = in6_is_scope_embed(a);
        for i in 0..8 {
            if i == 1 && embed {
                continue;
            }
            sum += u32::from(a.s6_addr16(i));
        }
    }

    sum += u32::from((len >> 16) as u16) + u32::from(len as u16);

    sum += u32::from((nxt >> 16) as u16) + u32::from(nxt as u16);

    sum = u32::from((sum >> 16) as u16) + u32::from(sum as u16);

    if sum > 0xffff {
        sum -= 0xffff;
    }

    sum as u16
}

/// A 16-bit checksum written at byte `offset` of the packet: in the first mbuf when it holds
/// the two bytes and is writable, else through `m_copyback` (the C's, whose failure it
/// ignores).
#[inline]
fn cksum_store(m: &mut Mbuf, offset: i32, csum: u16) {
    let o = offset as usize;
    if offset >= 0
        && o + size_of::<u16>() <= m.m_len().get() as usize
        && let Some(d) = m.data_mut()
    {
        d[o..o + 2].copy_from_slice(&csum.to_ne_bytes());
        return;
    }
    let _ = m_copyback(m, offset, &csum.to_ne_bytes(), M_NOWAIT);
}

/// `in6_delayed_cksum(m, nxt)`: processes a delayed payload checksum calculation for
/// upper-layer protocol `nxt`.
pub fn in6_delayed_cksum(m: &mut Mbuf, nxt: u8) {
    let mut nxtp: i32 = 0;
    let offset = ip6_lasthdr(m, 0, IPPROTO_IPV6, &mut nxtp);
    let Some(mut offset) = offset.filter(|&o| o > 0) else {
        // If the desired next protocol isn't found, punt.
        return;
    };
    if nxtp != i32::from(nxt) {
        return;
    }
    let mut csum = in6_cksum(
        m,
        0,
        offset as u32,
        (m.m_pkthdr().len.get() - offset) as u32,
    );

    match i32::from(nxt) {
        IPPROTO_TCP => offset += TH_SUM_OFFSET,
        IPPROTO_UDP => {
            offset += UH_SUM_OFFSET;
            if csum == 0 {
                csum = 0xffff;
            }
        }
        IPPROTO_ICMPV6 => offset += ICMP6_CKSUM_OFFSET,
        _ => {}
    }

    cksum_store(m, offset, csum);
}

/// `in6_proto_cksum_out(m, ifp)`: the upper-layer checksum of `m` in software, or left (with
/// the pseudo-header sum filled in) to the hardware of `ifp`. The kernel's Ethernet layer and
/// `if_output_tso` call it directly (the `Inet` leg of the same name gone, D-21).
pub fn in6_proto_cksum_out(m: &mut Mbuf, ifp: Option<&Ifnet>) {
    let ip6 = ip6_load(m);
    let flags = m.m_pkthdr().csum_flags.get();

    // some hw and in6_delayed_cksum need the pseudo header cksum
    if flags & (M_TCP_CSUM_OUT | M_UDP_CSUM_OUT | M_ICMP_CSUM_OUT) != 0 {
        let mut nxt: i32 = 0;
        // An invalid chain (-1 in C) gets no pseudo-header sum (the module's deviations).
        if let Some(mut offset) = ip6_lasthdr(m, 0, IPPROTO_IPV6, &mut nxt) {
            let csum = if flags & M_TCP_TSO != 0 && in_ifcap_cksum(m, ifp, IFCAP_TSOv6) {
                in6_cksum_phdr(&ip6.ip6_src, &ip6.ip6_dst, htonl(0), htonl(nxt as u32))
            } else {
                in6_cksum_phdr(
                    &ip6.ip6_src,
                    &ip6.ip6_dst,
                    htonl((m.m_pkthdr().len.get() - offset) as u32),
                    htonl(nxt as u32),
                )
            };
            if nxt == IPPROTO_TCP {
                offset += TH_SUM_OFFSET;
            } else if nxt == IPPROTO_UDP {
                offset += UH_SUM_OFFSET;
            } else if nxt == IPPROTO_ICMPV6 {
                offset += ICMP6_CKSUM_OFFSET;
            }
            cksum_store(m, offset, csum);
        }
    }

    let no_hw = |cap: u32, proto: i32| {
        ifp.is_none_or(|ifp| {
            ifp.if_capabilities.get() & cap == 0
                || i32::from(ip6.ip6_nxt) != proto
                || ifp.if_bridgeidx.get() != 0
        })
    };
    if flags & M_TCP_CSUM_OUT != 0 {
        if no_hw(IFCAP_CSUM_TCPv6, IPPROTO_TCP) {
            tcpstat_inc(TcpstatCounters::TcpsOutswcsum);
            in6_delayed_cksum(m, IPPROTO_TCP as u8);
            m.m_pkthdr().csum_flags.set(flags & !M_TCP_CSUM_OUT); // Clear
        }
    } else if flags & M_UDP_CSUM_OUT != 0 {
        if no_hw(IFCAP_CSUM_UDPv6, IPPROTO_UDP) {
            udpstat_inc(UdpstatCounters::UdpsOutswcsum);
            in6_delayed_cksum(m, IPPROTO_UDP as u8);
            m.m_pkthdr().csum_flags.set(flags & !M_UDP_CSUM_OUT); // Clear
        }
    } else if flags & M_ICMP_CSUM_OUT != 0 {
        in6_delayed_cksum(m, IPPROTO_ICMPV6 as u8);
        m.m_pkthdr().csum_flags.set(flags & !M_ICMP_CSUM_OUT); // Clear
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `ip6_output.c` over kernel packets: LZ's eight (header splitting,
    // fragment header insertion and fragmentation, the jumbo payload option, the packet option
    // parser and the control message walk, the pseudo-header sum) re-expressed; `ip6_output`
    // over a capturing interface (the extension headers chained, MLD's router alert view, the
    // refusals with their counters, fragments for the path MTU and the minimum MTU policies,
    // the multicast hop limit, the traffic class); `ip6_ctloutput` over every name with the
    // C's errnos; the multicast memberships of two sockets; `IPV6_CHECKSUM`.

    use std::boxed::Box;
    use std::sync::Mutex;
    use std::vec;
    use std::vec::Vec;

    use hal::sync::netlock::NETLOCK;
    use sysdefs::net_if::{IFF_RUNNING, IFF_UP};
    use sysdefs::netinet_in::IPPROTO_IP;
    use sysdefs::netinet_ip6::{IP6OPT_ROUTER_ALERT, IPV6_VERSION};
    use sysdefs::socket::{PF_INET6, SOCK_RAW};
    use sysdefs::sockio::SIOCSIFADDR;

    use super::*;
    use crate::net::domain::Domain;
    use crate::net::if_::{IFNETLIST, test_ifnet_with};
    use crate::net::if_var::NetDriver;
    use crate::net::mbuf::{M_PKTHDR, MT_SOOPTS, ml_dequeue};
    use crate::net::proto::{PR_ADDR, PR_ATOMIC, Protocol, Usrreqs};
    use crate::net::route::tests::testroutes_setup;
    use crate::net::socket::{SoInit, soalloc};
    use crate::netinet::in_pcb::{INP_IPV6, in_pcballoc, testinet};
    use crate::netinet6::in6::tests::{a6, add, iface};
    use crate::netinet6::ip6_var::IP6COUNTERS;
    use crate::netinet6::mld6::MLD6_HBH;
    use crate::proc::cred::{CredRef, Ucred};

    type Tg = testinet::G;

    fn stat(c: Ip6statCounters) -> u64 {
        IP6COUNTERS.read()[c as usize]
    }

    /// An IPv6 header for `plen` payload bytes of protocol `nxt`, 2001:db8::1 to `dst`.
    fn hdr_to(plen: u16, nxt: u8, dst: In6Addr) -> Ip6Hdr {
        let mut ip6 = Ip6Hdr::zeroed();
        ip6.set_ip6_vfc(IPV6_VERSION);
        ip6.ip6_plen = htons(plen);
        ip6.ip6_nxt = nxt;
        ip6.ip6_hlim = 64;
        ip6.ip6_src = a6("2001:db8::1");
        ip6.ip6_dst = dst;
        ip6
    }

    fn hdr(plen: u16, nxt: u8) -> Ip6Hdr {
        hdr_to(plen, nxt, a6("2001:db8::2"))
    }

    /// A packet: the header in a first mbuf of its own, then `payload` in mbufs of at most
    /// `piece` bytes.
    fn packet(ip6: &Ip6Hdr, payload: &[u8], piece: usize) -> MbufBox {
        let mut head = m_gethdr(M_DONTWAIT, MT_DATA).expect("mbuf");
        head.m_len().set(IP6_HDR_LEN as u32);
        assert!(ip6_store(&mut head, ip6));
        for chunk in payload.chunks(piece) {
            let mut m = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
            m.m_len().set(chunk.len() as u32);
            m.data_mut().expect("writable").copy_from_slice(chunk);
            drop(last_mbuf(&mut head).m_next.set(m));
        }
        head.m_pkthdr()
            .len
            .set((IP6_HDR_LEN + payload.len()) as i32);
        head
    }

    /// The bytes `off..off + len` of a chain.
    fn bytes(m: &Mbuf, off: i32, len: i32) -> Vec<u8> {
        let mut v = vec![0u8; len as usize];
        m_copydata(m, off, &mut v);
        v
    }

    /// The whole packet's bytes.
    fn all(m: &Mbuf) -> Vec<u8> {
        bytes(m, 0, m.m_pkthdr().len.get())
    }

    #[test]
    fn ip6_splithdr_moves_header_to_own_mbuf() {
        let _g = testroutes_setup();
        let ip6 = hdr(20, IPPROTO_UDP as u8);
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("mbuf");
        m.m_len().set(60);
        let mut b = [0xabu8; 60];
        assert!(pod::put(&mut b, 0, &ip6));
        m.data_mut().expect("writable").copy_from_slice(&b);
        m.m_pkthdr().len.set(60);

        let h = ip6_splithdr(m).expect("split");
        assert_eq!(h.m_len().get(), 40);
        assert_eq!(h.m_pkthdr().len.get(), 60);
        assert_eq!(ip6_load(&h), ip6);
        let n = h.m_next().get().expect("payload");
        assert_eq!(n.m_len().get(), 20);
        assert_eq!(n.flags() & M_PKTHDR, 0, "the header moved to the new mbuf");
        assert_eq!(bytes(&h, 40, 20), vec![0xab; 20]);

        // A header alone in its mbuf stays where it is.
        let m = packet(&ip6, &[], 1);
        let at = core::ptr::from_ref::<Mbuf>(&m).addr();
        let h = ip6_splithdr(m).expect("split");
        assert_eq!(core::ptr::from_ref::<Mbuf>(&h).addr(), at);
        assert!(h.m_next().get().is_none());
    }

    #[test]
    fn ip6_insertfraghdr_uses_trailing_space() {
        let _g = testroutes_setup();
        let ip6 = hdr(8, IPPROTO_DSTOPTS as u8);
        let mut m0 = packet(&ip6, &[17, 0, 1, 4, 0, 0, 0, 0], 8);
        let mut m = m_gethdr(M_DONTWAIT, MT_HEADER).expect("mbuf");
        m.m_len().set(40);
        m.m_pkthdr().len.set(40);
        let frag = Ip6Frag {
            ip6f_nxt: IPPROTO_UDP as u8,
            ip6f_reserved: 0,
            ip6f_offlg: htons(1232) | IP6F_MORE_FRAG,
            ip6f_ident: htonl(0x0102_0304),
        };
        let mut fb = [0u8; FRAG_LEN];
        assert!(pod::put(&mut fb, 0, &frag));

        // Unfragmentable part is the header alone: the fragment header follows it in `m`.
        ip6_insertfraghdr(&mut m0, &mut m, 40, frag).expect("fraghdr");
        assert_eq!(m.m_len().get(), 48);
        assert_eq!(m.m_pkthdr().len.get(), 48);
        assert!(m.m_next().get().is_none());
        assert_eq!(bytes(&m, 40, 8), fb.to_vec(), "the fragment header written");

        // With a destination options header the copy of it carries the fragment header.
        let mut m = m_gethdr(M_DONTWAIT, MT_HEADER).expect("mbuf");
        m.m_len().set(40);
        m.m_pkthdr().len.set(40);
        ip6_insertfraghdr(&mut m0, &mut m, 48, frag).expect("fraghdr");
        let n = m.m_next().get().expect("copied exthdr");
        assert_eq!(
            n.m_len().get(),
            16,
            "the copy and the fragment header's room"
        );
        assert_eq!(bytes(n, 0, 8), vec![17, 0, 1, 4, 0, 0, 0, 0]);
        assert_eq!(
            bytes(n, 8, 8),
            fb.to_vec(),
            "the fragment header after the copy"
        );
    }

    /// N6.5c review S1's second shape: a header byte at an mbuf boundary (offset 40, the
    /// first byte after the IPv6 header's mbuf) is written into the mbuf that holds it; the
    /// mbuf before it, a shared cluster here, is left alone (`m_copyback`'s walk would have
    /// made it writable first, a copy of its cluster for none of its bytes).
    #[test]
    fn a_header_byte_at_an_mbuf_boundary_leaves_the_mbuf_before_alone() {
        let _g = testroutes_setup();
        let mut a = m_gethdr(M_DONTWAIT, MT_DATA).expect("mbuf");
        mclget(&mut a, M_DONTWAIT);
        assert_ne!(a.flags() & M_EXT, 0, "the header in a cluster");
        a.m_len().set(40);
        a.data_mut().expect("a fresh cluster").fill(0);
        let sharer = m_copym(&mut a, 0, 40, M_DONTWAIT).expect("a sharer of the cluster");
        let at = a.data_raw();
        let mut n = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        n.m_len().set(8);
        n.data_mut().expect("writable").fill(0);
        drop(a.m_next.set(n));
        a.m_pkthdr().len.set(48);

        set_byte_at(&mut a, 40, IPPROTO_DSTOPTS as u8).expect("the byte written");
        assert_eq!(byte_at(&a, 40), IPPROTO_DSTOPTS as u8);
        assert_eq!(a.data_raw(), at, "the cluster before the byte not copied");
        assert!(a.is_readonly(), "still shared with its sharer");
        drop(sharer);
    }

    /// N6.5c review S1: the unfragmentable part ends in a cluster (a 256-byte destination
    /// options header, over `MLEN`), which every fragment's copy of the headers shares
    /// (`m_copym`): the fragment header goes into an mbuf of its own, no fragment copies the
    /// cluster, and every fragment goes out (the C writes `*frghdrp` into that mbuf and cannot
    /// fail there).
    #[test]
    fn fragments_share_the_cluster_of_the_unfragmentable_headers() {
        let _g = testroutes_setup();
        let payload: Vec<u8> = (0..4000u32).map(|i| (i * 3) as u8).collect();
        let mut m0 = packet(&hdr(256 + 4000, IPPROTO_DSTOPTS as u8), &[], 1);
        let mut d = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        mclget(&mut d, M_DONTWAIT);
        assert_ne!(d.flags() & M_EXT, 0, "the header in a cluster");
        d.m_len().set(256);
        let b = d.data_mut().expect("a fresh cluster");
        b.fill(0);
        // Next header UDP, (31 + 1) * 8 bytes, one PadN over the rest.
        b[..4].copy_from_slice(&[IPPROTO_UDP as u8, 31, IP6OPT_PADN, 252]);
        let cluster = d.data_raw();
        drop(last_mbuf(&mut m0).m_next.set(d));
        for chunk in payload.chunks(200) {
            let mut m = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
            m.m_len().set(chunk.len() as u32);
            m.data_mut().expect("writable").copy_from_slice(chunk);
            drop(last_mbuf(&mut m0).m_next.set(m));
        }
        m0.m_pkthdr().len.set(40 + 256 + 4000);
        let dropped = stat(Ip6statCounters::Ip6sOdropped);

        let mut ml = MbufList::new();
        ip6_fragment(m0, &mut ml, 296, IPPROTO_UDP as u8, 1500).expect("every fragment");
        // (1500 - 296 - 8) & ~7 = 1192 bytes per fragment: three, then 424.
        assert_eq!(ml_len(&ml), 4);
        for (i, len) in [1192, 1192, 1192, 424].into_iter().enumerate() {
            let off = i as i32 * 1192;
            let f = ml_dequeue(&mut ml).expect("a fragment");
            let copy = f.m_next().get().expect("the copied headers");
            assert_eq!(
                copy.data_raw(),
                cluster,
                "fragment {i}: the cluster shared, not copied"
            );
            assert_eq!(
                copy.m_len().get(),
                256,
                "fragment {i}: the copy alone in its mbuf"
            );
            let fh = copy.m_next().get().expect("the fragment header's mbuf");
            assert_eq!(fh.m_len().get(), 8);
            let fb = bytes(&f, 296, 8);
            let frag = pod::at::<Ip6Frag>(&fb, 0).expect("a fragment header");
            assert_eq!(frag.ip6f_nxt, IPPROTO_UDP as u8);
            assert_eq!(
                i32::from(u16::from_be(frag.ip6f_offlg & !IP6F_MORE_FRAG)),
                off
            );
            assert_eq!(frag.ip6f_offlg & IP6F_MORE_FRAG != 0, i < 3);
            assert_eq!(f.m_pkthdr().len.get(), 304 + len);
            assert_eq!(
                bytes(&f, 304, len),
                payload[off as usize..(off + len) as usize].to_vec()
            );
        }
        assert_eq!(stat(Ip6statCounters::Ip6sOdropped), dropped);
    }

    #[test]
    fn ip6_fragment_sizes_offsets_and_more_flag() {
        let _g = testroutes_setup();
        let payload: Vec<u8> = (0..3000u32).map(|i| (i * 7) as u8).collect();
        let ip6 = hdr(3000, IPPROTO_UDP as u8);
        let mut ip6f = ip6;
        ip6f.ip6_nxt = IPPROTO_FRAGMENT as u8;
        let m0 = packet(&ip6f, &payload, 200);
        let mut ml = MbufList::new();
        let (frags, dropped) = (
            stat(Ip6statCounters::Ip6sOfragments),
            stat(Ip6statCounters::Ip6sOdropped),
        );

        ip6_fragment(m0, &mut ml, 40, IPPROTO_UDP as u8, 1280).expect("fragment");
        assert_eq!(ml_len(&ml), 3);
        assert_eq!(stat(Ip6statCounters::Ip6sOfragments), frags + 3);

        // (1280 - 40 - 8) & ~7 = 1232 bytes per fragment.
        let expect = [(0, 1232, true), (1232, 1232, true), (2464, 536, false)];
        let mut ident = None;
        for &(off, len, more) in &expect {
            let f = ml_dequeue(&mut ml).expect("fragment");
            assert_eq!(f.m_pkthdr().len.get(), 48 + len);
            let h = ip6_load(&f);
            assert_eq!(i32::from(u16::from_be(h.ip6_plen)), 8 + len);
            assert_eq!(h.ip6_nxt, IPPROTO_FRAGMENT as u8);
            assert_eq!(h.ip6_src, ip6.ip6_src);
            let fb = bytes(&f, 40, 8);
            let frag = pod::at::<Ip6Frag>(&fb, 0).expect("a fragment header");
            assert_eq!(frag.ip6f_nxt, IPPROTO_UDP as u8);
            assert_eq!(frag.ip6f_reserved, 0);
            assert_eq!(
                i32::from(u16::from_be(frag.ip6f_offlg & !IP6F_MORE_FRAG)),
                off
            );
            assert_eq!(frag.ip6f_offlg & IP6F_MORE_FRAG != 0, more);
            assert_eq!(*ident.get_or_insert(frag.ip6f_ident), frag.ip6f_ident);
            assert_ne!(frag.ip6f_ident, 0, "idgen32 never answers 0");
            assert_eq!(
                bytes(&f, 48, len),
                payload[off as usize..(off + len) as usize].to_vec()
            );
        }
        assert_eq!(stat(Ip6statCounters::Ip6sOdropped), dropped);
    }

    #[test]
    fn ip6_fragment_mtu_too_small() {
        let _g = testroutes_setup();
        let m0 = packet(&hdr(100, IPPROTO_UDP as u8), &[0u8; 100], 100);
        let mut ml = MbufList::new();
        let dropped = stat(Ip6statCounters::Ip6sOdropped);
        // (55 - 40 - 8) & ~7 = 0 < 8.
        assert_eq!(
            ip6_fragment(m0, &mut ml, 40, IPPROTO_UDP as u8, 55),
            Err(Errno::EMSGSIZE)
        );
        assert_eq!(ml_len(&ml), 0);
        assert_eq!(stat(Ip6statCounters::Ip6sOdropped), dropped + 1);
    }

    #[test]
    fn ip6_insert_jumboopt_builds_hbh() {
        let _g = testroutes_setup();
        let m = packet(&hdr(0, IPPROTO_UDP as u8), &[], 1);
        let mut ex = Ip6Exthdrs::default();
        ip6_insert_jumboopt(&mut ex, &m, 70000).expect("jumbo");
        let h = ex.ip6e_hbh.as_ref().expect("hbh");
        assert_eq!(h.m_len().get(), 8);
        let b = bytes(h, 0, 8);
        assert_eq!(b[1], 0);
        assert_eq!(&b[2..4], &[IP6OPT_JUMBO, 4]);
        assert_eq!(u32::from_be_bytes([b[4], b[5], b[6], b[7]]), 70008);
        assert_eq!(m.m_pkthdr().len.get(), 48);

        // A second insertion appends PadN and the option to the existing header.
        ip6_insert_jumboopt(&mut ex, &m, 70000).expect("jumbo");
        let h = ex.ip6e_hbh.as_ref().expect("hbh");
        assert_eq!(h.m_len().get(), 16);
        let b = bytes(h, 0, 16);
        assert_eq!(b[1], 1); // ip6h_len
        assert_eq!(&b[8..12], &[IP6OPT_PADN, 0, IP6OPT_JUMBO, 4]);
        assert_eq!(m.m_pkthdr().len.get(), 56);
    }

    /// One bad value per check of `ip6_setpktopt` (each passing every earlier check) and the
    /// C's errno, the options unchanged after it; then the values each check accepts.
    #[test]
    fn ip6_setpktopt_validation() {
        let _g = testroutes_setup();
        let mut opt = Ip6Pktopts::new();
        assert_eq!((opt.ip6po_hlim, opt.ip6po_tclass), (-1, -1));
        assert_eq!(opt.ip6po_minmtu, IP6PO_MINMTU_MCASTONLY);
        let int = |v: i32| v.to_ne_bytes();
        let udp = IPPROTO_UDP;
        let tcp = IPPROTO_TCP;
        let hbh = [0u8, 0, IP6OPT_PADN, 4, 0, 0, 0, 0];
        let long_hbh = [0u8, 1, IP6OPT_PADN, 4, 0, 0, 0, 0];
        let mut pi = [0u8; 20];
        pi[0] = 0x20;
        let mut bad_if = [0u8; 20];
        bad_if[16..].copy_from_slice(&4000u32.to_ne_bytes());
        // A good packet info with four bytes more: the C wants the size exactly.
        let mut long_pi = [0u8; 24];
        long_pi[..20].copy_from_slice(&pi);
        struct Case<'a> {
            what: &'static str,
            name: i32,
            buf: &'a [u8],
            priv_: bool,
            sticky: bool,
            uproto: i32,
            err: Errno,
        }
        use Errno::{EINVAL, ENOPROTOOPT, ENXIO, EPERM};
        #[rustfmt::skip]
        let cases = [
            Case { what: "pktinfo of 16 bytes", name: IPV6_PKTINFO, buf: &[0; 16], priv_: false, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "pktinfo of 24 bytes", name: IPV6_PKTINFO, buf: &long_pi, priv_: false, sticky: false, uproto: udp, err: EINVAL },
            Case { what: "sticky pktinfo address on TCP", name: IPV6_PKTINFO, buf: &pi, priv_: false, sticky: true, uproto: tcp, err: EINVAL },
            Case { what: "pktinfo of an unknown interface", name: IPV6_PKTINFO, buf: &bad_if, priv_: false, sticky: true, uproto: udp, err: ENXIO },
            Case { what: "sticky hoplimit", name: IPV6_HOPLIMIT, buf: &int(64), priv_: false, sticky: true, uproto: udp, err: ENOPROTOOPT },
            Case { what: "hoplimit of 2 bytes", name: IPV6_HOPLIMIT, buf: &[1, 2], priv_: false, sticky: false, uproto: udp, err: EINVAL },
            Case { what: "hoplimit 256", name: IPV6_HOPLIMIT, buf: &int(256), priv_: false, sticky: false, uproto: udp, err: EINVAL },
            Case { what: "hoplimit -2", name: IPV6_HOPLIMIT, buf: &int(-2), priv_: false, sticky: false, uproto: udp, err: EINVAL },
            Case { what: "tclass of 2 bytes", name: IPV6_TCLASS, buf: &[1, 2], priv_: false, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "tclass 256", name: IPV6_TCLASS, buf: &int(256), priv_: false, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "tclass -2", name: IPV6_TCLASS, buf: &int(-2), priv_: false, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "hopopts unprivileged", name: IPV6_HOPOPTS, buf: &hbh, priv_: false, sticky: true, uproto: udp, err: EPERM },
            Case { what: "hopopts of one byte", name: IPV6_HOPOPTS, buf: &hbh[..1], priv_: true, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "hopopts shorter than its length", name: IPV6_HOPOPTS, buf: &long_hbh, priv_: true, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "dstopts unprivileged", name: IPV6_DSTOPTS, buf: &hbh, priv_: false, sticky: true, uproto: udp, err: EPERM },
            Case { what: "rthdrdstopts unprivileged", name: IPV6_RTHDRDSTOPTS, buf: &hbh, priv_: false, sticky: true, uproto: udp, err: EPERM },
            Case { what: "dstopts of one byte", name: IPV6_DSTOPTS, buf: &hbh[..1], priv_: true, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "dstopts of 6 bytes", name: IPV6_DSTOPTS, buf: &hbh[..6], priv_: true, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "min mtu of 2 bytes", name: IPV6_USE_MIN_MTU, buf: &[1, 2], priv_: false, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "min mtu policy 5", name: IPV6_USE_MIN_MTU, buf: &int(5), priv_: false, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "dontfrag of 2 bytes", name: IPV6_DONTFRAG, buf: &[1, 2], priv_: false, sticky: true, uproto: udp, err: EINVAL },
            Case { what: "routing header", name: IPV6_RTHDR, buf: &hbh, priv_: true, sticky: true, uproto: udp, err: ENOPROTOOPT },
            Case { what: "an unknown name", name: 9999, buf: &int(1), priv_: true, sticky: true, uproto: udp, err: ENOPROTOOPT },
        ];
        for c in &cases {
            assert_eq!(
                ip6_setpktopt(c.name, c.buf, &mut opt, c.priv_, c.sticky, c.uproto),
                Err(c.err),
                "{}",
                c.what
            );
            assert!(
                opt.ip6po_pktinfo.is_none() && opt.ip6po_hbh.is_none() && opt.ip6po_dest2.is_none(),
                "{}: nothing stored",
                c.what
            );
            assert_eq!(
                (
                    opt.ip6po_hlim,
                    opt.ip6po_tclass,
                    opt.ip6po_minmtu,
                    opt.ip6po_flags
                ),
                (-1, -1, IP6PO_MINMTU_MCASTONLY, 0),
                "{}: nothing changed",
                c.what
            );
        }

        // Each accepts its bounds.
        ip6_setpktopt(IPV6_HOPLIMIT, &int(255), &mut opt, false, false, udp).expect("255");
        ip6_setpktopt(IPV6_HOPLIMIT, &int(-1), &mut opt, false, false, udp).expect("-1");
        ip6_setpktopt(IPV6_HOPLIMIT, &int(64), &mut opt, false, false, udp).expect("64");
        assert_eq!(opt.ip6po_hlim, 64);
        ip6_setpktopt(IPV6_TCLASS, &int(-1), &mut opt, false, true, udp).expect("-1");
        ip6_setpktopt(IPV6_TCLASS, &int(255), &mut opt, false, true, udp).expect("255");
        ip6_setpktopt(IPV6_TCLASS, &int(0x2e), &mut opt, false, true, udp).expect("0x2e");
        assert_eq!(opt.ip6po_tclass, 0x2e);

        // Minimum MTU policy and don't-fragment (TCP ignores it).
        for p in [
            IP6PO_MINMTU_DISABLE,
            IP6PO_MINMTU_MCASTONLY,
            IP6PO_MINMTU_ALL,
        ] {
            ip6_setpktopt(IPV6_USE_MIN_MTU, &int(p), &mut opt, false, true, udp).expect("policy");
            assert_eq!(opt.ip6po_minmtu, p);
        }
        ip6_setpktopt(IPV6_DONTFRAG, &int(1), &mut opt, false, true, udp).expect("dontfrag");
        assert_ne!(opt.ip6po_flags & IP6PO_DONTFRAG, 0);
        ip6_setpktopt(IPV6_DONTFRAG, &int(1), &mut opt, false, true, tcp).expect("tcp");
        assert_eq!(opt.ip6po_flags & IP6PO_DONTFRAG, 0);

        // Packet info: stored (an ancillary address on TCP is fine), cleared by in6addr_any and
        // index zero.
        ip6_setpktopt(IPV6_PKTINFO, &pi, &mut opt, false, false, tcp).expect("ancillary");
        ip6_setpktopt(IPV6_PKTINFO, &pi, &mut opt, false, true, udp).expect("sticky");
        assert_eq!(opt.pktinfo().expect("pktinfo").ipi6_addr.s6_addr[0], 0x20);
        ip6_setpktopt(IPV6_PKTINFO, &[0; 20], &mut opt, false, true, udp).expect("clear");
        assert!(opt.ip6po_pktinfo.is_none());
        // Without one, a zero packet info is stored (the C stores it then).
        ip6_setpktopt(IPV6_PKTINFO, &[0; 20], &mut opt, false, true, udp).expect("zero");
        assert_eq!(opt.pktinfo(), Some(In6Pktinfo::default()));

        // Hop-by-hop and destination options: privileged, length matching the header's; an
        // empty value removes the option.
        ip6_setpktopt(IPV6_HOPOPTS, &hbh, &mut opt, true, true, udp).expect("hbh");
        assert_eq!(opt.ip6po_hbh.as_deref(), Some(&hbh[..]));
        ip6_setpktopt(IPV6_RTHDRDSTOPTS, &hbh, &mut opt, true, true, udp).expect("dest1");
        assert!(opt.ip6po_dest1.is_some() && opt.ip6po_dest2.is_none());
        ip6_setpktopt(IPV6_DSTOPTS, &hbh, &mut opt, true, true, udp).expect("dest2");
        assert!(opt.ip6po_dest2.is_some());

        // A copy duplicates the allocations; clearing frees them.
        let mut copy = Ip6Pktopts::new();
        copypktopts(&mut copy, &opt).expect("copy");
        assert_eq!(copy.ip6po_hbh.as_deref(), opt.ip6po_hbh.as_deref());
        let (c, o) = (
            copy.ip6po_hbh.as_deref().map(<[u8]>::as_ptr),
            opt.ip6po_hbh.as_deref().map(<[u8]>::as_ptr),
        );
        assert_ne!(c, o, "a block of its own");
        assert_eq!(copy.pktinfo(), opt.pktinfo());
        ip6_setpktopt(IPV6_HOPOPTS, &[], &mut opt, true, true, udp).expect("removed");
        assert!(opt.ip6po_hbh.is_none());
        ip6_clearpktopts(&mut copy, -1);
        ip6_clearpktopts(&mut opt, -1);
        assert!(opt.ip6po_dest1.is_none() && opt.ip6po_dest2.is_none() && opt.pktinfo().is_none());
        assert_eq!((opt.ip6po_hlim, opt.ip6po_tclass), (-1, -1));
    }

    #[test]
    fn ip6_setpktopts_walks_cmsgs() {
        let _g = testroutes_setup();
        let hdrlen = cmsg_len(0);
        let mut buf = Vec::new();
        for (level, type_, v) in [
            (IPPROTO_IPV6, IPV6_HOPLIMIT, 7i32),
            (IPPROTO_TCP, 1, 0),
            (IPPROTO_IPV6, IPV6_TCLASS, 3),
        ] {
            let len = (hdrlen + 4) as u32;
            buf.extend_from_slice(&len.to_ne_bytes());
            buf.extend_from_slice(&level.to_ne_bytes());
            buf.extend_from_slice(&type_.to_ne_bytes());
            buf.resize(buf.len() + hdrlen - 12, 0);
            buf.extend_from_slice(&v.to_ne_bytes());
            buf.resize(cmsg_align(buf.len()), 0);
        }
        let mut control = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        control.m_len().set(buf.len() as u32);
        control.data_mut().expect("writable").copy_from_slice(&buf);

        // The sticky options are the start (their hop limit, class and flags: `copypktopts`
        // copies no minimum MTU policy, as the C's), the control messages override them.
        let mut sticky = Ip6Pktopts::new();
        sticky.ip6po_minmtu = IP6PO_MINMTU_ALL;
        sticky.ip6po_flags = IP6PO_DONTFRAG;
        sticky.ip6po_tclass = 9;
        let mut opt = Ip6Pktopts::new();
        assert_eq!(
            ip6_setpktopts(&control, &mut opt, Some(&sticky), false, IPPROTO_UDP),
            Ok(())
        );
        assert_eq!((opt.ip6po_hlim, opt.ip6po_tclass), (7, 3));
        assert_eq!(opt.ip6po_flags, IP6PO_DONTFRAG);
        assert_eq!(opt.ip6po_minmtu, IP6PO_MINMTU_MCASTONLY, "not copied");

        // A truncated message is rejected.
        control.m_len().set(buf.len() as u32 - 2);
        let r = ip6_setpktopts(&control, &mut opt, None, false, IPPROTO_UDP);
        assert_eq!(r, Err(Errno::EINVAL));
        // A message whose length is shorter than its header.
        control.m_len().set(buf.len() as u32);
        control.data_mut().expect("writable")[..4].copy_from_slice(&4u32.to_ne_bytes());
        let r = ip6_setpktopts(&control, &mut opt, None, false, IPPROTO_UDP);
        assert_eq!(r, Err(Errno::EINVAL));
        // Whole again, then in two mbufs: EINVAL (the C assumes all the optional information
        // in a single mbuf, C :2188).
        control.data_mut().expect("writable")[..4]
            .copy_from_slice(&((hdrlen + 4) as u32).to_ne_bytes());
        let r = ip6_setpktopts(&control, &mut opt, None, false, IPPROTO_UDP);
        assert_eq!(r, Ok(()), "whole again");
        let more = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        drop(control.m_next.set(more));
        let r = ip6_setpktopts(&control, &mut opt, None, false, IPPROTO_UDP);
        assert_eq!(r, Err(Errno::EINVAL), "a second mbuf");
    }

    #[test]
    fn in6_cksum_phdr_skips_embedded_scope() {
        let ip6 = hdr(8, IPPROTO_UDP as u8);
        let sum = in6_cksum_phdr(&ip6.ip6_src, &ip6.ip6_dst, htonl(8), htonl(17));
        // 2001+0db8 twice, 1 + 2, length 8 and protocol 17, folded (network-order words).
        let expect: u32 = 2 * (0x2001 + 0x0db8) + 1 + 2 + 8 + 17;
        assert_eq!(u16::from_be(sum), expect as u16);

        // fe80::1%5 with the scope in word 1: the word is left out.
        let a = a6("fe80:5::1");
        let b = a6("fe80::1");
        assert_eq!(
            in6_cksum_phdr(&a, &a, htonl(8), htonl(17)),
            in6_cksum_phdr(&b, &b, htonl(8), htonl(17))
        );
    }

    /// A driver that keeps the packets it is given and their destinations' bytes, and brings
    /// the interface up on its first address.
    struct CapDriver6 {
        sent: Mutex<Vec<(Vec<u8>, Vec<u8>)>>,
    }

    impl CapDriver6 {
        fn take(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
            core::mem::take(&mut *self.sent.lock().expect("sent"))
        }
    }

    impl NetDriver for CapDriver6 {
        fn ioctl(&self, ifp: &'static Ifnet, cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            if cmd == SIOCSIFADDR {
                ifp.if_flags.set(ifp.if_flags.get() | IFF_UP | IFF_RUNNING);
            }
            Ok(())
        }
        fn output(
            &self,
            _ifp: &'static Ifnet,
            m: MbufBox,
            dst: SockaddrView<'_>,
            _rt: Option<&Rtentry>,
        ) -> Result<(), Errno> {
            self.sent
                .lock()
                .expect("sent")
                .push((all(&m), dst.bytes().to_vec()));
            Ok(())
        }
    }

    /// An interface `name` with MTU `mtu`, multicast, on the list, with 2001:db8::1/64 (its
    /// routes), and its driver, nothing captured yet.
    fn cap_iface(name: &[u8], mtu: u32) -> (&'static Ifnet, &'static CapDriver6) {
        let d: &'static CapDriver6 = Box::leak(Box::new(CapDriver6 {
            sent: Mutex::new(Vec::new()),
        }));
        let ifp = test_ifnet_with(name, d);
        ifp.if_flags.set(IFF_UP | IFF_RUNNING | IFF_MULTICAST);
        ifp.if_mtu.set(mtu);
        crate::net::if_::if_alloc_sadl(ifp);
        {
            let _w = NETLOCK.write();
            IFNETLIST.insert_tail(ifp);
        }
        add(ifp, a6("2001:db8::1"), 64).expect("SIOCAIFADDR_IN6");
        let _ = d.take();
        (ifp, d)
    }

    /// `ip6_output` under the net lock.
    fn send(
        m: MbufBox,
        opt: Option<Ip6OutOpts<'_>>,
        flags: i32,
        im6o: Option<&Ip6Moptions>,
    ) -> Result<(), Errno> {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        ip6_output(m, opt, None, flags, im6o, None, &net)
    }

    #[test]
    fn ip6_output_chains_the_extension_headers_of_its_options() {
        let _g = testroutes_setup();
        let (_ifp, d) = cap_iface(b"i6o0", 1500);
        let dst = a6("2001:db8::2");
        let payload = [9u8; 16];

        // No option: the packet as given, its payload length filled, to the destination.
        let ip6 = hdr_to(0, IPPROTO_UDP as u8, dst);
        let local = stat(Ip6statCounters::Ip6sLocalout);
        send(packet(&ip6, &payload, 16), None, 0, None).expect("sent");
        let out = d.take();
        assert_eq!(out.len(), 1);
        let (p, sa) = &out[0];
        assert_eq!(p.len(), 40 + 16);
        assert_eq!((p[4], p[5], p[6]), (0, 16, IPPROTO_UDP as u8));
        assert_eq!(sa[1], AF_INET6);
        assert_eq!(
            pod::at::<In6Addr>(sa, 8),
            Some(dst),
            "the route's destination"
        );
        assert_eq!(stat(Ip6statCounters::Ip6sLocalout), local + 1);

        // A hop-by-hop header (MLD's router alert, lent as a view) and destination options
        // after the routing header: IPv6 hbh dest2 payload, each next header chained, the
        // payload length the headers' too.
        let dest2 = [0u8, 0, IP6OPT_PADN, 4, 0, 0, 0, 0];
        let opts = Ip6OutOpts::new()
            .with_exthdrs(Some(&MLD6_HBH), None, Some(&dest2))
            .expect("whole headers");
        send(packet(&ip6, &payload, 16), Some(opts), 0, None).expect("sent");
        let (p, _) = d.take().pop().expect("the packet");
        assert_eq!(p.len(), 40 + 8 + 8 + 16);
        assert_eq!((p[5], p[6]), (32, IPPROTO_HOPOPTS as u8));
        assert_eq!(
            p[40..48],
            [
                IPPROTO_DSTOPTS as u8,
                0,
                IP6OPT_PADN,
                0,
                IP6OPT_ROUTER_ALERT,
                2,
                0,
                0
            ],
            "the router alert whole, its next header the destination options"
        );
        assert_eq!(
            p[48], IPPROTO_UDP as u8,
            "dest2's next header the payload's"
        );
        assert_eq!(&p[49..56], &dest2[1..]);
        assert_eq!(&p[56..], &payload);

        // hbh and dest1: IPv6 hbh dest1 payload; the hop-by-hop header's next header (at
        // offset 40, the mbuf after the IPv6 header's) the destination options, theirs the
        // payload's.
        let dest1 = [0u8, 0, IP6OPT_PADN, 4, 0, 0, 0, 0];
        let opts = Ip6OutOpts::new()
            .with_exthdrs(Some(&MLD6_HBH), Some(&dest1), None)
            .expect("whole headers");
        send(packet(&ip6, &payload, 16), Some(opts), 0, None).expect("sent");
        let (p, _) = d.take().pop().expect("the packet");
        assert_eq!(p.len(), 40 + 8 + 8 + 16);
        assert_eq!((p[5], p[6]), (32, IPPROTO_HOPOPTS as u8));
        assert_eq!(
            p[40], IPPROTO_DSTOPTS as u8,
            "hbh's next header the destination options"
        );
        assert_eq!(&p[41..48], &MLD6_HBH[1..]);
        assert_eq!(
            p[48], IPPROTO_UDP as u8,
            "dest1's next header the payload's"
        );
        assert_eq!(&p[49..56], &dest1[1..]);
        assert_eq!(&p[56..], &payload);

        // dest1 alone: IPv6 dest1 payload.
        let opts = Ip6OutOpts::new()
            .with_exthdrs(None, Some(&dest2), None)
            .expect("whole headers");
        send(packet(&ip6, &payload, 16), Some(opts), 0, None).expect("sent");
        let (p, _) = d.take().pop().expect("the packet");
        assert_eq!((p[6], p[40]), (IPPROTO_DSTOPTS as u8, IPPROTO_UDP as u8));
    }

    /// A packet with a hop-by-hop header, then also destination options, over the MTU (C
    /// :698-708): the unfragmentable part's last header takes `IPPROTO_FRAGMENT` as its next
    /// header, and the fragment header carries the one it had.
    #[test]
    fn ip6_output_fragments_after_the_unfragmentable_headers() {
        let _g = testroutes_setup();
        let (_ifp, d) = cap_iface(b"i6f0", 1280);
        let ip6 = hdr_to(0, IPPROTO_UDP as u8, a6("2001:db8::2"));
        let payload = vec![5u8; 2000];
        let dest1 = [0u8, 0, IP6OPT_PADN, 4, 0, 0, 0, 0];

        // hbh alone: IPv6 hbh frag payload, (1280 - 48 - 8) & ~7 = 1224 bytes a fragment.
        let opts = Ip6OutOpts::new()
            .with_exthdrs(Some(&MLD6_HBH), None, None)
            .expect("whole headers");
        send(packet(&ip6, &payload, 200), Some(opts), 0, None).expect("sent");
        let out = d.take();
        assert_eq!(out.len(), 2);
        for (p, _) in &out {
            assert_eq!(p[6], IPPROTO_HOPOPTS as u8);
            assert_eq!(
                p[40], IPPROTO_FRAGMENT as u8,
                "hbh's next header the fragment's"
            );
            assert_eq!(
                p[48], IPPROTO_UDP as u8,
                "the fragment header's the payload's"
            );
        }

        // hbh and dest1: IPv6 hbh dest1 frag payload.
        let opts = Ip6OutOpts::new()
            .with_exthdrs(Some(&MLD6_HBH), Some(&dest1), None)
            .expect("whole headers");
        send(packet(&ip6, &payload, 200), Some(opts), 0, None).expect("sent");
        let out = d.take();
        assert_eq!(out.len(), 2);
        for (p, _) in &out {
            assert_eq!(
                (p[6], p[40]),
                (IPPROTO_HOPOPTS as u8, IPPROTO_DSTOPTS as u8)
            );
            assert_eq!(
                p[48], IPPROTO_FRAGMENT as u8,
                "dest1's next header the fragment's"
            );
            assert_eq!(
                p[56], IPPROTO_UDP as u8,
                "the fragment header's the payload's"
            );
        }
    }

    #[test]
    fn ip6_output_fills_the_traffic_class_and_hop_limit() {
        let _g = testroutes_setup();
        let (ifp, d) = cap_iface(b"i6o1", 1500);
        let flow = |p: &[u8]| u32::from_be_bytes([p[0], p[1], p[2], p[3]]);

        // The traffic class fills only the zero halves of the header's (diffserv, ecn).
        let mut ip6 = hdr_to(0, IPPROTO_UDP as u8, a6("2001:db8::2"));
        ip6.ip6_flow = htonl(0x6000_0000 | (0x01 << 20)); // ecn set, diffserv clear
        let mut opts = Ip6OutOpts::new();
        opts.tclass = 0xb9;
        opts.hlim = 7;
        send(packet(&ip6, &[0; 8], 8), Some(opts), 0, None).expect("sent");
        let (p, _) = d.take().pop().expect("the packet");
        assert_eq!(
            (flow(&p) >> 20) & 0xff,
            0xb8 | 0x01,
            "diffserv filled, ecn kept"
        );
        assert_eq!(p[7], 7, "the options' hop limit");

        // A multicast destination: the multicast options' hop limit (the default without);
        // a hop limit of 0 is not transmitted.
        let g = a6("ff05::1:5");
        let im6o = Ip6Moptions::output(ifp.if_index.get() as u16, 3, 0);
        send(packet(&hdr_to(0, 17, g), &[0; 8], 8), None, 0, Some(&im6o)).expect("sent");
        let (p, sa) = d.take().pop().expect("the packet");
        assert_eq!(p[7], 3);
        assert_eq!(pod::at::<In6Addr>(&sa, 8), Some(g), "to the group itself");
        let im6o = Ip6Moptions::output(ifp.if_index.get() as u16, 0, 0);
        send(packet(&hdr_to(0, 17, g), &[0; 8], 8), None, 0, Some(&im6o)).expect("not sent");
        assert!(d.take().is_empty(), "hop limit 0");
    }

    #[test]
    fn ip6_output_refuses_as_the_c() {
        let _g = testroutes_setup();
        let (ifp, d) = cap_iface(b"i6o2", 1500);
        use Ip6statCounters as C;

        // An unspecified source without IPV6_UNSPECSRC, a multicast source.
        let mut ip6 = hdr(0, IPPROTO_UDP as u8);
        ip6.ip6_src = In6Addr::default();
        let bad = stat(C::Ip6sBadscope);
        assert_eq!(
            send(packet(&ip6, &[0; 8], 8), None, 0, None),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(stat(C::Ip6sBadscope), bad + 1);
        send(packet(&ip6, &[0; 8], 8), None, IPV6_UNSPECSRC, None).expect("allowed");
        ip6.ip6_src = a6("ff02::1");
        assert_eq!(
            send(packet(&ip6, &[0; 8], 8), None, IPV6_UNSPECSRC, None),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(stat(C::Ip6sBadscope), bad + 2);

        // No route.
        let noroute = stat(C::Ip6sNoroute);
        assert_eq!(
            send(
                packet(&hdr_to(0, 17, a6("2001:db9::1")), &[0; 8], 8),
                None,
                0,
                None
            ),
            Err(Errno::EHOSTUNREACH)
        );
        assert_eq!(stat(C::Ip6sNoroute), noroute + 1);

        // Too big to go whole with IPV6_DONTFRAG.
        let mut opts = Ip6OutOpts::new();
        opts.flags = IP6PO_DONTFRAG;
        assert_eq!(
            send(packet(&hdr(1600, 17), &[0; 1600], 200), Some(opts), 0, None),
            Err(Errno::EMSGSIZE)
        );

        // A multicast destination out of an interface without IFF_MULTICAST.
        ifp.if_flags.set(ifp.if_flags.get() & !IFF_MULTICAST);
        let im6o = Ip6Moptions::output(ifp.if_index.get() as u16, 1, 0);
        assert_eq!(
            send(
                packet(&hdr_to(0, 17, a6("ff05::9")), &[0; 8], 8),
                None,
                0,
                Some(&im6o)
            ),
            Err(Errno::ENETUNREACH)
        );
        assert_eq!(stat(C::Ip6sNoroute), noroute + 2);
        assert!(
            d.take().len() == 1,
            "only the unspecified source's packet went out"
        );
    }

    #[test]
    fn ip6_output_fragments_for_the_path_mtu_and_the_minimum_mtu_policies() {
        let _g = testroutes_setup();
        let (ifp, d) = cap_iface(b"i6o3", 1500);
        let payload: Vec<u8> = (0..2000u32).map(|i| i as u8).collect();
        let fragmented = stat(Ip6statCounters::Ip6sFragmented);

        // 2040 bytes over 1500: fragments of (1500 - 40 - 8) & ~7 = 1448.
        send(packet(&hdr(2000, 17), &payload, 200), None, 0, None).expect("sent");
        let out = d.take();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0.len(), 48 + 1448);
        assert_eq!(out[1].0.len(), 48 + 552);
        assert_eq!(out[0].0[6], IPPROTO_FRAGMENT as u8);
        assert_eq!(out[0].0[40], 17, "the fragment header's next header");
        assert_eq!(stat(Ip6statCounters::Ip6sFragmented), fragmented + 1);

        // IPV6_MINMTU: fragments of (1280 - 48) & ~7 = 1232.
        send(
            packet(&hdr(2000, 17), &payload, 200),
            None,
            IPV6_MINMTU,
            None,
        )
        .expect("sent");
        let out = d.take();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0.len(), 48 + 1232);

        // IP6PO_MINMTU_ALL: the same for a unicast packet.
        let mut all_ = Ip6OutOpts::new();
        all_.minmtu = IP6PO_MINMTU_ALL;
        send(packet(&hdr(2000, 17), &payload, 200), Some(all_), 0, None).expect("sent");
        assert_eq!(d.take()[0].0.len(), 48 + 1232);

        // A 1400-byte multicast packet goes at the minimum MTU (fragmented) by default, whole
        // with IP6PO_MINMTU_DISABLE.
        let im6o = Ip6Moptions::output(ifp.if_index.get() as u16, 1, 0);
        let g = hdr_to(1360, 17, a6("ff05::3"));
        send(packet(&g, &payload[..1360], 200), None, 0, Some(&im6o)).expect("sent");
        assert_eq!(d.take().len(), 2, "fragmented at 1280");
        let mut disable = Ip6OutOpts::new();
        disable.minmtu = IP6PO_MINMTU_DISABLE;
        send(
            packet(&g, &payload[..1360], 200),
            Some(disable),
            0,
            Some(&im6o),
        )
        .expect("sent");
        let out = d.take();
        assert_eq!(out.len(), 1, "whole");
        assert_eq!(out[0].0.len(), 1400);
    }

    /// An option mbuf holding `v`.
    fn optm(v: &[u8]) -> MbufBox {
        let mut m = m_get(M_DONTWAIT, i32::from(MT_SOOPTS)).expect("an mbuf");
        m.m_len().set(v.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(v);
        m
    }

    /// `ip6_ctloutput(op, inp, level, name, m)` under the net lock: `v` the value of a
    /// `PRCO_SETOPT` (`None` for none); the answer's bytes.
    fn ctl(
        inp: &Inpcb<Tg>,
        op: i32,
        level: i32,
        name: i32,
        v: Option<&[u8]>,
    ) -> Result<Vec<u8>, Errno> {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut m = optm(v.unwrap_or(&[]));
        let arg = (op == PRCO_GETOPT || v.is_some()).then_some(&mut *m);
        ip6_ctloutput(op, inp, level, name, arg, &net)?;
        Ok(m.data().to_vec())
    }

    /// `setsockopt(IPPROTO_IPV6, name, v)`.
    fn set(inp: &Inpcb<Tg>, name: i32, v: &[u8]) -> Result<(), Errno> {
        ctl(inp, PRCO_SETOPT, IPPROTO_IPV6, name, Some(v)).map(drop)
    }

    /// `getsockopt(IPPROTO_IPV6, name)`.
    fn get(inp: &Inpcb<Tg>, name: i32) -> Result<Vec<u8>, Errno> {
        ctl(inp, PRCO_GETOPT, IPPROTO_IPV6, name, None)
    }

    /// An `int` option's bytes.
    fn int(v: i32) -> Vec<u8> {
        v.to_ne_bytes().to_vec()
    }

    /// `f` as a user that is not the superuser (uid 1000), then `p` root again.
    fn as_user<R>(p: &crate::proc::Thread<Tg>, f: impl FnOnce() -> R) -> R {
        let mut ids = Ucred::new();
        ids.cr_uid.set(1000);
        ids.cr_ruid.set(1000);
        let root = p.set_cred(CredRef::new(ids));
        let r = f();
        if let Some(root) = root {
            drop(p.set_cred(root));
        }
        r
    }

    #[test]
    fn ip6_ctloutput_sets_and_gets_every_option_as_the_c() {
        let (_g, p) = testinet::setup();
        let (so, inp) = testinet::pcb(testinet::table(16));
        inp.set_flags(INP_IPV6);
        use Errno::{EACCES, EINVAL, ENOPROTOOPT, ENOTCONN, EPERM};

        // Another level, either way.
        assert_eq!(
            ctl(&inp, PRCO_SETOPT, IPPROTO_IP, IPV6_V6ONLY, Some(&int(1))),
            Err(EINVAL)
        );
        assert_eq!(
            ctl(&inp, PRCO_GETOPT, IPPROTO_IP, IPV6_V6ONLY, None),
            Err(EINVAL)
        );

        // The `int` options of the first group: no value or another length is EINVAL (the
        // receive-option flags unprivileged EPERM first for the two of hbh/dst), a value is
        // read back.
        let ints = [
            (IPV6_UNICAST_HOPS, 33),
            (IPV6_MINHOPCOUNT, 200),
            (IPV6_RECVPKTINFO, 1),
            (IPV6_RECVHOPLIMIT, 1),
            (IPV6_RECVRTHDR, 1),
            (IPV6_RECVPATHMTU, 1),
            (IPV6_RECVTCLASS, 1),
            (IPV6_AUTOFLOWLABEL, 1),
            (IPV6_RECVDSTPORT, 1),
        ];
        for (name, v) in ints {
            assert_eq!(
                ctl(&inp, PRCO_SETOPT, IPPROTO_IPV6, name, None),
                Err(EINVAL),
                "{name} without a value"
            );
            assert_eq!(set(&inp, name, &[1, 0]), Err(EINVAL), "{name} of 2 bytes");
            assert_eq!(set(&inp, name, &[0; 8]), Err(EINVAL), "{name} of 8 bytes");
            set(&inp, name, &int(v)).expect("set");
            assert_eq!(get(&inp, name), Ok(int(v)), "{name}");
        }
        assert_eq!(inp.inp_hops.get(), 33);
        assert_eq!(inp.inp_ip6_minhlim().get(), 200);

        // OPTSET/OPTBIT.
        let flags = [
            (IPV6_RECVPKTINFO, IN6P_PKTINFO),
            (IPV6_RECVHOPLIMIT, IN6P_HOPLIMIT),
            (IPV6_RECVRTHDR, IN6P_RTHDR),
            (IPV6_RECVPATHMTU, IN6P_MTU),
            (IPV6_RECVTCLASS, IN6P_TCLASS),
            (IPV6_AUTOFLOWLABEL, IN6P_AUTOFLOWLABEL),
            (IPV6_RECVDSTPORT, IN6P_RECVDSTPORT),
        ];
        for (name, bit) in flags {
            assert!(inp.has_flags(bit), "{name}");
            set(&inp, name, &int(0)).expect("cleared");
            assert!(!inp.has_flags(bit), "{name}");
            assert_eq!(get(&inp, name), Ok(int(0)), "{name}");
            set(&inp, name, &int(-5)).expect("any non-zero value sets");
            assert!(inp.has_flags(bit), "{name}");
        }

        // IPV6_RECVHOPOPTS and IPV6_RECVDSTOPTS are the privileged socket's (SS_PRIV): EPERM
        // before any length check.
        for (name, bit) in [
            (IPV6_RECVHOPOPTS, IN6P_HOPOPTS),
            (IPV6_RECVDSTOPTS, IN6P_DSTOPTS),
        ] {
            assert_eq!(set(&inp, name, &[1]), Err(EPERM), "{name}");
            assert_eq!(set(&inp, name, &int(1)), Err(EPERM), "{name}");
            assert!(!inp.has_flags(bit));
            assert_eq!(get(&inp, name), Ok(int(0)), "{name}");
        }
        so.st_set(SS_PRIV);
        for (name, bit) in [
            (IPV6_RECVHOPOPTS, IN6P_HOPOPTS),
            (IPV6_RECVDSTOPTS, IN6P_DSTOPTS),
        ] {
            assert_eq!(set(&inp, name, &[1]), Err(EINVAL), "{name}");
            set(&inp, name, &int(1)).expect("privileged");
            assert!(inp.has_flags(bit), "{name}");
            assert_eq!(get(&inp, name), Ok(int(1)), "{name}");
        }

        // The bounds of IPV6_UNICAST_HOPS (-1..=255) and IPV6_MINHOPCOUNT (0..=255).
        for bad in [256, -2] {
            assert_eq!(
                set(&inp, IPV6_UNICAST_HOPS, &int(bad)),
                Err(EINVAL),
                "hops {bad}"
            );
        }
        assert_eq!(get(&inp, IPV6_UNICAST_HOPS), Ok(int(33)), "kept");
        set(&inp, IPV6_UNICAST_HOPS, &int(-1)).expect("the default");
        set(&inp, IPV6_UNICAST_HOPS, &int(255)).expect("255");
        for bad in [-1, 256] {
            assert_eq!(
                set(&inp, IPV6_MINHOPCOUNT, &int(bad)),
                Err(EINVAL),
                "minhop {bad}"
            );
        }
        set(&inp, IPV6_MINHOPCOUNT, &int(0)).expect("0");
        set(&inp, IPV6_MINHOPCOUNT, &int(255)).expect("255");

        // IPV6_V6ONLY: 1 only (no IPv4-mapped addresses), before bind; always read as 1.
        assert_eq!(set(&inp, IPV6_V6ONLY, &int(0)), Err(EINVAL));
        set(&inp, IPV6_V6ONLY, &int(1)).expect("v6only");
        assert_eq!(get(&inp, IPV6_V6ONLY), Ok(int(1)));
        {
            let g = inp.table().lock();
            inp.set_lport(&g, 7);
        }
        assert_eq!(set(&inp, IPV6_V6ONLY, &int(1)), Err(EINVAL), "bound");
        {
            let g = inp.table().lock();
            inp.set_lport(&g, 0);
        }

        // IPV6_HOPLIMIT as a sticky option: ENOPROTOOPT (RFC 3542), the options made though.
        assert_eq!(set(&inp, IPV6_HOPLIMIT, &int(5)), Err(ENOPROTOOPT));
        assert!(inp.has_outputopts6(), "ip6_pcbopt made the options first");

        // The sticky options through ip6_pcbopt, read back by ip6_getpcbopt.
        assert_eq!(get(&inp, IPV6_TCLASS), Ok(int(0)), "the default class");
        set(&inp, IPV6_TCLASS, &int(0x20)).expect("tclass");
        assert_eq!(get(&inp, IPV6_TCLASS), Ok(int(0x20)));
        assert_eq!(set(&inp, IPV6_TCLASS, &[1, 2]), Err(EINVAL));
        assert_eq!(get(&inp, IPV6_USE_MIN_MTU), Ok(int(IP6PO_MINMTU_MCASTONLY)));
        set(&inp, IPV6_USE_MIN_MTU, &int(IP6PO_MINMTU_ALL)).expect("min mtu");
        assert_eq!(get(&inp, IPV6_USE_MIN_MTU), Ok(int(IP6PO_MINMTU_ALL)));
        assert_eq!(set(&inp, IPV6_USE_MIN_MTU, &int(7)), Err(EINVAL));
        assert_eq!(get(&inp, IPV6_DONTFRAG), Ok(int(0)));
        set(&inp, IPV6_DONTFRAG, &int(1)).expect("dontfrag");
        assert_eq!(get(&inp, IPV6_DONTFRAG), Ok(int(1)));
        assert_eq!(get(&inp, IPV6_PKTINFO), Ok(vec![0; 20]));
        let mut pi = [0u8; 20];
        pi[..16].copy_from_slice(&a6("2001:db8::7").s6_addr);
        set(&inp, IPV6_PKTINFO, &pi).expect("pktinfo");
        assert_eq!(get(&inp, IPV6_PKTINFO), Ok(pi.to_vec()));
        let hbh = [0u8, 0, IP6OPT_PADN, 4, 0, 0, 0, 0];
        set(&inp, IPV6_HOPOPTS, &hbh).expect("privileged hbh");
        assert_eq!(get(&inp, IPV6_HOPOPTS), Ok(hbh.to_vec()));
        set(&inp, IPV6_RTHDRDSTOPTS, &hbh).expect("dest1");
        assert_eq!(get(&inp, IPV6_RTHDRDSTOPTS), Ok(hbh.to_vec()));
        set(&inp, IPV6_DSTOPTS, &hbh).expect("dest2");
        assert_eq!(get(&inp, IPV6_DSTOPTS), Ok(hbh.to_vec()));
        assert_eq!(
            get(&inp, IPV6_RTHDR),
            Ok(vec![]),
            "no routing header option"
        );
        assert_eq!(set(&inp, IPV6_RTHDR, &hbh), Err(ENOPROTOOPT));
        // No value: the empty option, which removes the header.
        ctl(&inp, PRCO_SETOPT, IPPROTO_IPV6, IPV6_HOPOPTS, None).expect("removed");
        assert_eq!(get(&inp, IPV6_HOPOPTS), Ok(vec![]));
        // A chained value is refused.
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock");
            let mut m = optm(&hbh);
            drop(m.m_next.set(optm(&[0])));
            assert_eq!(
                ip6_ctloutput(
                    PRCO_SETOPT,
                    &inp,
                    IPPROTO_IPV6,
                    IPV6_DSTOPTS,
                    Some(&mut m),
                    &net
                ),
                Err(EINVAL)
            );
        }

        // IPV6_PORTRANGE: the two flags exclude each other; another range is EINVAL.
        set(&inp, IPV6_PORTRANGE, &int(IPV6_PORTRANGE_LOW)).expect("low");
        assert!(inp.has_flags(IN6P_LOWPORT) && !inp.has_flags(IN6P_HIGHPORT));
        assert_eq!(get(&inp, IPV6_PORTRANGE), Ok(int(IPV6_PORTRANGE_LOW)));
        set(&inp, IPV6_PORTRANGE, &int(IPV6_PORTRANGE_HIGH)).expect("high");
        assert!(inp.has_flags(IN6P_HIGHPORT) && !inp.has_flags(IN6P_LOWPORT));
        set(&inp, IPV6_PORTRANGE, &int(IPV6_PORTRANGE_DEFAULT)).expect("default");
        assert!(!inp.has_flags(IN6P_LOWPORT | IN6P_HIGHPORT));
        assert_eq!(get(&inp, IPV6_PORTRANGE), Ok(int(0)));
        assert_eq!(set(&inp, IPV6_PORTRANGE, &int(3)), Err(EINVAL));
        assert_eq!(set(&inp, IPV6_PORTRANGE, &[1]), Err(EINVAL));

        // IPSEC6_OUTSA: EINVAL either way.
        assert_eq!(set(&inp, IPSEC6_OUTSA, &int(1)), Err(EINVAL));
        assert_eq!(get(&inp, IPSEC6_OUTSA), Err(EINVAL));

        // The IPsec levels: outside BYPASS..=UNIQUE is EINVAL; below the default, EACCES but
        // for the superuser.
        for name in [
            IPV6_AUTH_LEVEL,
            IPV6_ESP_TRANS_LEVEL,
            IPV6_ESP_NETWORK_LEVEL,
            IPV6_IPCOMP_LEVEL,
        ] {
            for bad in [IPSEC_LEVEL_BYPASS - 1, IPSEC_LEVEL_UNIQUE + 1] {
                assert_eq!(set(&inp, name, &int(bad)), Err(EINVAL), "{name} {bad}");
            }
            assert_eq!(set(&inp, name, &[1]), Err(EINVAL), "{name} of a byte");
            set(&inp, name, &int(IPSEC_LEVEL_UNIQUE)).expect("above the default");
            assert_eq!(
                as_user(p, || set(&inp, name, &int(IPSEC_LEVEL_BYPASS))),
                Err(EACCES),
                "{name}"
            );
            assert_eq!(get(&inp, name), Ok(int(IPSEC_LEVEL_UNIQUE)), "{name} kept");
            set(&inp, name, &int(IPSEC_LEVEL_BYPASS)).expect("the superuser");
            assert_eq!(get(&inp, name), Ok(int(IPSEC_LEVEL_BYPASS)), "{name}");
        }

        // SO_RTABLE: at least a `u_int`; the same table is no change; a table that does not
        // exist is EINVAL.
        assert_eq!(get(&inp, SO_RTABLE), Ok(int(0)));
        assert_eq!(set(&inp, SO_RTABLE, &[0; 2]), Err(EINVAL));
        assert_eq!(
            ctl(&inp, PRCO_SETOPT, IPPROTO_IPV6, SO_RTABLE, None),
            Err(EINVAL)
        );
        set(&inp, SO_RTABLE, &[0; 8]).expect("table 0 again");
        assert_eq!(set(&inp, SO_RTABLE, &int(5)), Err(EINVAL));
        // A process in table 0 may move its socket to any table; one in another table needs
        // the superuser's privilege to move it to a table not its own (C :1331-1333).
        crate::net::rtable::rtable_add(2).expect("table 2");
        as_user(p, || set(&inp, SO_RTABLE, &int(2))).expect("from table 0");
        assert_eq!(get(&inp, SO_RTABLE), Ok(int(2)));
        p.process().ps_rtableid.store(2, Ordering::Relaxed);
        assert_eq!(as_user(p, || set(&inp, SO_RTABLE, &int(0))), Err(EPERM));
        assert_eq!(get(&inp, SO_RTABLE), Ok(int(2)), "unchanged by the refusal");
        set(&inp, SO_RTABLE, &int(0)).expect("the superuser");
        as_user(p, || set(&inp, SO_RTABLE, &int(2))).expect("the process's own table");
        set(&inp, SO_RTABLE, &int(0)).expect("back to table 0");
        p.process().ps_rtableid.store(0, Ordering::Relaxed);

        // IPV6_PIPEX.
        set(&inp, IPV6_PIPEX, &int(3)).expect("pipex");
        assert_eq!(get(&inp, IPV6_PIPEX), Ok(int(3)));
        assert_eq!(set(&inp, IPV6_PIPEX, &[3]), Err(EINVAL));

        // IPV6_PATHMTU: the connected socket's only.
        assert_eq!(get(&inp, IPV6_PATHMTU), Err(ENOTCONN));
        assert_eq!(set(&inp, IPV6_PATHMTU, &int(1)), Err(ENOPROTOOPT));

        // Unknown names.
        assert_eq!(set(&inp, 99, &int(1)), Err(ENOPROTOOPT));
        assert_eq!(get(&inp, 99), Err(ENOPROTOOPT));

        // The multicast options' get side without options: the C's defaults (IPV6_MULTICAST_LOOP
        // answers ip6_defmcasthlim then, as the C).
        let defhlim = IP6_DEFMCASTHLIM.load(Ordering::Relaxed);
        assert_eq!(get(&inp, IPV6_MULTICAST_IF), Ok(int(0)));
        assert_eq!(get(&inp, IPV6_MULTICAST_HOPS), Ok(int(defhlim)));
        assert_eq!(get(&inp, IPV6_MULTICAST_LOOP), Ok(int(defhlim)));
        assert_eq!(get(&inp, IPV6_JOIN_GROUP), Err(Errno::EOPNOTSUPP));

        testinet::release(so, inp);
    }

    #[test]
    fn the_multicast_options_and_the_memberships_of_two_sockets() {
        let (_g, p) = testinet::setup();
        let t = testinet::table(16);
        let (so1, a) = testinet::pcb(t);
        let (so2, b) = testinet::pcb(t);
        a.set_flags(INP_IPV6);
        b.set_flags(INP_IPV6);
        let (if1, d1) = iface(b"i6m0", [0x52, 0x54, 0, 0xbb, 0, 0x11]);
        let (if2, _d2) = iface(b"i6m1", [0x52, 0x54, 0, 0xbb, 0, 0x12]);
        let (i1, i2) = (if1.if_index.get(), if2.if_index.get());
        let mreq = |group: In6Addr, ifidx: u32| {
            let r = Ipv6Mreq {
                ipv6mr_multiaddr: group,
                ipv6mr_interface: ifidx,
            };
            let mut b = vec![0u8; size_of::<Ipv6Mreq>()];
            assert!(pod::put(&mut b, 0, &r));
            b
        };
        use Errno::{EACCES, EADDRINUSE, EADDRNOTAVAIL, EINVAL, ENXIO};

        // An interface without IFF_MULTICAST and one in another routing domain.
        let (if3, _d3) = iface(b"i6m2", [0x52, 0x54, 0, 0xbb, 0, 0x13]);
        if3.if_flags.set(if3.if_flags.get() & !IFF_MULTICAST);
        let (if4, _d4) = iface(b"i6m3", [0x52, 0x54, 0, 0xbb, 0, 0x14]);
        if4.if_rdomain.set(1);
        let (i3, i4) = (if3.if_index.get(), if4.if_index.get());

        // IPV6_MULTICAST_IF/HOPS/LOOP: the bounds, then the defaults give the options back.
        assert_eq!(set(&a, IPV6_MULTICAST_IF, &int(4000)), Err(ENXIO));
        assert_eq!(
            set(&a, IPV6_MULTICAST_IF, &int(i3 as i32)),
            Err(EADDRNOTAVAIL)
        );
        assert_eq!(
            set(&a, IPV6_MULTICAST_IF, &int(i4 as i32)),
            Err(EADDRNOTAVAIL)
        );
        assert!(a.moptions6_out().is_none(), "back to the defaults: freed");
        set(&a, IPV6_MULTICAST_IF, &int(i1 as i32)).expect("if");
        assert_eq!(get(&a, IPV6_MULTICAST_IF), Ok(int(i1 as i32)));
        assert_eq!(set(&a, IPV6_MULTICAST_HOPS, &int(256)), Err(EINVAL));
        set(&a, IPV6_MULTICAST_HOPS, &int(9)).expect("hops");
        assert_eq!(get(&a, IPV6_MULTICAST_HOPS), Ok(int(9)));
        assert_eq!(set(&a, IPV6_MULTICAST_LOOP, &int(2)), Err(EINVAL));
        set(&a, IPV6_MULTICAST_LOOP, &int(0)).expect("loop");
        assert_eq!(get(&a, IPV6_MULTICAST_LOOP), Ok(int(0)));
        let o = a.moptions6_out().expect("options");
        assert_eq!((o.im6o_ifidx, o.im6o_hlim, o.im6o_loop), (i1 as u16, 9, 0));
        set(&a, IPV6_MULTICAST_IF, &int(0)).expect("none");
        set(&a, IPV6_MULTICAST_HOPS, &int(-1)).expect("the default");
        set(&a, IPV6_MULTICAST_LOOP, &int(1)).expect("the default");
        assert!(
            a.moptions6_out().is_none(),
            "every option the default: freed"
        );

        // The requests: a wrong length, not multicast, every group (the superuser's).
        let g = a6("ff05::7");
        assert_eq!(set(&a, IPV6_JOIN_GROUP, &[0; 4]), Err(EINVAL));
        let mut long = mreq(g, i1);
        long.extend_from_slice(&[0; 4]);
        assert_eq!(
            set(&a, IPV6_JOIN_GROUP, &long),
            Err(EINVAL),
            "the size exactly"
        );
        assert_eq!(
            set(&a, IPV6_JOIN_GROUP, &mreq(a6("2001:db8::1"), i1)),
            Err(EINVAL)
        );
        assert_eq!(
            as_user(p, || set(
                &a,
                IPV6_JOIN_GROUP,
                &mreq(In6Addr::default(), i1)
            )),
            Err(EACCES)
        );
        assert_eq!(set(&a, IPV6_JOIN_GROUP, &mreq(g, 4000)), Err(ENXIO));
        assert_eq!(
            set(&a, IPV6_JOIN_GROUP, &mreq(g, i3)),
            Err(EADDRNOTAVAIL),
            "no multicast"
        );
        assert_eq!(
            set(&a, IPV6_JOIN_GROUP, &mreq(g, i4)),
            Err(EADDRNOTAVAIL),
            "another domain"
        );
        assert_eq!(
            set(&a, IPV6_JOIN_GROUP, &mreq(g, 0)),
            Err(EADDRNOTAVAIL),
            "no route to g"
        );
        assert_eq!(
            set(&a, IPV6_LEAVE_GROUP, &mreq(g, 4000)),
            Err(ENXIO),
            "no such interface"
        );
        assert_eq!(
            set(&a, IPV6_LEAVE_GROUP, &mreq(g, 0)),
            Err(EADDRNOTAVAIL),
            "no membership"
        );
        assert!(
            a.moptions6_out().is_none(),
            "the refusals left the defaults"
        );

        // Two sockets join g on if1: one record, the driver told once; a second join of the same
        // socket is EADDRINUSE.
        let mout = crate::testutil::malloc_out(crate::netinet::in_var::M_IPMADDR);
        let adds = d1.adds();
        set(&a, IPV6_JOIN_GROUP, &mreq(g, i1)).expect("a joins");
        set(&b, IPV6_JOIN_GROUP, &mreq(g, i1)).expect("b joins");
        assert_eq!(set(&a, IPV6_JOIN_GROUP, &mreq(g, i1)), Err(EADDRINUSE));
        assert!(in6_hasmulti(&g, if1));
        assert_eq!(d1.adds(), adds + 1, "one record for both");

        // a also joins g on if2; a leave naming no interface drops the newest (if2's), as the
        // C's list walked from its head.
        set(&a, IPV6_JOIN_GROUP, &mreq(g, i2)).expect("a joins on if2");
        assert!(in6_hasmulti(&g, if2));
        set(&a, IPV6_LEAVE_GROUP, &mreq(g, 0)).expect("a leaves one");
        assert!(!in6_hasmulti(&g, if2), "the newest left");
        assert!(in6_hasmulti(&g, if1));
        assert_eq!(set(&a, IPV6_LEAVE_GROUP, &mreq(g, i2)), Err(EADDRNOTAVAIL));

        // a leaves if1's: b still holds the record.
        set(&a, IPV6_LEAVE_GROUP, &mreq(g, i1)).expect("a leaves");
        assert!(in6_hasmulti(&g, if1), "b's membership");
        assert!(
            a.moptions6_out().is_none(),
            "a's options back to the defaults"
        );

        // b's detach leaves its groups (ip6_freemoptions).
        testinet::release(so2, b);
        assert!(!in6_hasmulti(&g, if1));
        testinet::release(so1, a);
        assert_eq!(
            crate::testutil::malloc_out(crate::netinet::in_var::M_IPMADDR),
            mout,
            "the records and the memberships' arrays freed"
        );
    }

    #[test]
    fn ip6_raw_ctloutput_checksum_offsets() {
        let (_g, _p) = testinet::setup();
        let (so, inp) = testinet::pcb(testinet::table(16));
        let raw = |op: i32, v: Option<&[u8]>| -> Result<Vec<u8>, Errno> {
            let mut m = optm(v.unwrap_or(&[]));
            let arg = (op == PRCO_GETOPT || v.is_some()).then_some(&mut *m);
            let _w = NETLOCK.write();
            ip6_raw_ctloutput(op, &inp, IPPROTO_IPV6, IPV6_CHECKSUM, arg)?;
            Ok(m.data().to_vec())
        };
        // Odd offsets and below -1 are EINVAL, a value of another length too; -1 and even
        // offsets are kept (the socket is not ICMPv6).
        for bad in [-2, 3] {
            assert_eq!(
                raw(PRCO_SETOPT, Some(&int(bad))),
                Err(Errno::EINVAL),
                "{bad}"
            );
        }
        assert_eq!(raw(PRCO_SETOPT, Some(&[2])), Err(Errno::EINVAL));
        raw(PRCO_SETOPT, Some(&int(6))).expect("6");
        assert_eq!(raw(PRCO_GETOPT, None), Ok(int(6)));
        raw(PRCO_SETOPT, Some(&int(-1))).expect("-1");
        assert_eq!(raw(PRCO_GETOPT, None), Ok(int(-1)));
        // Another level, another name.
        {
            let _w = NETLOCK.write();
            assert_eq!(
                ip6_raw_ctloutput(PRCO_GETOPT, &inp, IPPROTO_IP, IPV6_CHECKSUM, None),
                Err(Errno::EINVAL)
            );
            assert_eq!(
                ip6_raw_ctloutput(PRCO_GETOPT, &inp, IPPROTO_IPV6, IPV6_V6ONLY, None),
                Err(Errno::ENOPROTOOPT)
            );
        }
        testinet::release(so, inp);
    }

    /// The ICMPv6 raw socket's domain and protocol (`so_proto->pr_protocol` is what
    /// `ip6_raw_ctloutput` reads); its sockets made by `soalloc` alone, as the test
    /// protocol's.
    struct Icmp6Dom;
    /// See [`Icmp6Dom`].
    struct Icmp6Proto;

    static ICMP6_PROTOS: [&dyn Protocol<Tg>; 1] = [&Icmp6Proto];

    impl Domain<Tg> for Icmp6Dom {
        fn family(&self) -> i32 {
            i32::from(PF_INET6)
        }
        fn name(&self) -> &'static str {
            "inet6"
        }
        fn protocols(&self) -> &'static [&'static dyn Protocol<Tg>] {
            &ICMP6_PROTOS
        }
    }

    impl Protocol<Tg> for Icmp6Proto {
        fn domain(&self) -> &'static dyn Domain<Tg> {
            &Icmp6Dom
        }
        fn ty(&self) -> i16 {
            SOCK_RAW as i16
        }
        fn protocol(&self) -> i16 {
            IPPROTO_ICMPV6 as i16
        }
        fn flags(&self) -> i16 {
            PR_ATOMIC | PR_ADDR
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<Tg>> {
            None
        }
    }

    /// `IPV6_CHECKSUM` of an ICMPv6 socket (C :1585-1599): only the ICMPv6 header's own
    /// checksum offset is accepted (the "no change" value), and a read answers it whatever
    /// the pcb's `inp_cksum6` holds.
    #[test]
    fn ip6_raw_ctloutput_of_an_icmpv6_socket() {
        let (_g, _p) = testinet::setup();
        let so = soalloc(
            &Icmp6Proto,
            M_WAIT,
            SoInit {
                so_type: SOCK_RAW,
                state: 0,
                options: 0,
                linger: 0,
                timeo: 0,
                euid: 0,
                ruid: 0,
                egid: 0,
                rgid: 0,
                cpid: 1,
            },
        )
        .expect("an ICMPv6 socket");
        let inp = in_pcballoc(&so, testinet::table(16), M_WAIT).expect("in_pcballoc");
        inp.set_flags(INP_IPV6);
        inp.inp_cksum6.set(-1);
        let raw = |op: i32, v: Option<&[u8]>| -> Result<Vec<u8>, Errno> {
            let mut m = optm(v.unwrap_or(&[]));
            let arg = (op == PRCO_GETOPT || v.is_some()).then_some(&mut *m);
            let _w = NETLOCK.write();
            ip6_raw_ctloutput(op, &inp, IPPROTO_IPV6, IPV6_CHECKSUM, arg)?;
            Ok(m.data().to_vec())
        };
        let icmp6off = offset_of!(Icmp6Hdr, icmp6_cksum) as i32;
        assert_eq!(icmp6off, 2);
        // Another offset, even and valid on another raw socket, or -1: EINVAL; the offset
        // itself is accepted and changes nothing.
        for bad in [0, 4, 6, -1] {
            assert_eq!(
                raw(PRCO_SETOPT, Some(&int(bad))),
                Err(Errno::EINVAL),
                "{bad}"
            );
        }
        raw(PRCO_SETOPT, Some(&int(icmp6off))).expect("no change");
        assert_eq!(inp.inp_cksum6.get(), -1, "the pcb's offset untouched");
        assert_eq!(
            raw(PRCO_GETOPT, None),
            Ok(int(icmp6off)),
            "the header's offset"
        );
        // The checks before the protocol's still come first.
        assert_eq!(raw(PRCO_SETOPT, Some(&int(3))), Err(Errno::EINVAL));
        testinet::release(so, inp);
    }

    /// `IPV6_PATHMTU` of a connected socket (C's `PRCO_GETOPT`): the path MTU of the pcb's
    /// route (the interface's, the route carrying none), at most `IPV6_MAXPACKET`;
    /// `EHOSTUNREACH` without a route to the peer.
    #[test]
    fn ipv6_pathmtu_of_a_connected_socket() {
        let (_g, _p) = testinet::setup();
        let (ifp, _d) = iface(b"i6p0", [0x52, 0x54, 0, 0xbb, 0, 0x21]);
        ifp.if_mtu.set(1400);
        add(ifp, a6("2001:db8::1"), 64).expect("SIOCAIFADDR_IN6");
        let (so, inp) = testinet::pcb(testinet::table(16));
        inp.set_flags(INP_IPV6);
        so.st_set(SS_ISCONNECTED);
        let sin6 = |a: In6Addr, port: u16| SockaddrIn6 {
            sin6_port: htons(port),
            ..SockaddrIn6::with_addr(a)
        };
        let mtuinfo = |mtu: u32| {
            let info = Ip6Mtuinfo {
                ip6m_mtu: mtu,
                ..Ip6Mtuinfo::default()
            };
            let mut b = vec![0u8; size_of::<Ip6Mtuinfo>()];
            assert!(pod::put(&mut b, 0, &info));
            b
        };

        // A peer no route reaches.
        crate::netinet6::in6_pcb::in6_pcbset_addr(
            &inp,
            &sin6(a6("2001:db9::5"), 53),
            &sin6(a6("2001:db8::1"), 1000),
            0,
        )
        .expect("the addresses");
        assert_eq!(get(&inp, IPV6_PATHMTU), Err(Errno::EHOSTUNREACH));
        testinet::release(so, inp);

        // An on-link peer: the interface's MTU, then IPV6_MAXPACKET above it.
        let (so, inp) = testinet::pcb(testinet::table(16));
        inp.set_flags(INP_IPV6);
        so.st_set(SS_ISCONNECTED);
        crate::netinet6::in6_pcb::in6_pcbset_addr(
            &inp,
            &sin6(a6("2001:db8::2"), 53),
            &sin6(a6("2001:db8::1"), 1001),
            0,
        )
        .expect("the addresses");
        assert_eq!(get(&inp, IPV6_PATHMTU), Ok(mtuinfo(1400)));
        ifp.if_mtu.set(70000);
        assert_eq!(get(&inp, IPV6_PATHMTU), Ok(mtuinfo(IPV6_MAXPACKET as u32)));
        ifp.if_mtu.set(1500);
        testinet::release(so, inp);
    }
}
/* </TESTS> */
