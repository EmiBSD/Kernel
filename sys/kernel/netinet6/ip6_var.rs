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
 *	@(#)ip_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 implementation variables, the kernel's half of `<netinet6/ip6_var.h>`: the per-CPU
//! statistics (`ip6counters`, and ICMPv6's `icmp6counters` beside them), the packet-option
//! and `ip6_output` flags, and the configuration variables (`ip6_defhlim`, `ip6_forwarding`,
//! ..., defined by `in6_proto.c`). The userland half (`struct ip6stat` and the indices of its
//! words) is `hal::sys::netinet6_ip6_var`. The control options of outgoing packets
//! ([`Ip6Pktopts`], with the borrowed view `ip6_output` takes, [`Ip6OutOpts`]) and the
//! multicast options ([`Ip6Moptions`]) are here since N6.5c, the reassembly records
//! ([`Ip6q`], [`Ip6asfrag`]) since N6.5d.
//!
//! Upstream: sys/netinet6/ip6_var.h @ 3ce1f3f79392
//! LZ: sys/netinet6/ip6_var.rs@3c62ede68802
//!
//! Locks: \[a\] the configuration variables are atomics (relaxed loads and stores, the C's
//! `atomic_load_int` and its sysctl's word writes).
//!
//! ## Deviations
//! - The C indexes the counters with arithmetic (`ip6s_nxthist + nxt`, `ip6s_m2m + i`,
//!   `ip6s_sources_*` + scope), which [`ip6stat_inc_idx`] does; `icmp6stat_inc(icp6s_outhist
//!   + type)` is [`icmp6stat_inc_hist`].
//! - `ip6_mrouter[]`, `ip6_mrouter_active` and `ip6_mforward` (`netinet6/ip6_mroute.c`) are not
//!   compiled: `MROUTING` is not configured.
//!
//! ## Redesign
//! - LZ's `IP6COUNTERS` (a static array of atomics in `bsd`'s `ip6_input.rs`) and
//!   `ICMP6COUNTERS` (`icmp6.rs`'s), `<sys/percpu.h>` not ported -> [`IP6COUNTERS`] and
//!   [`ICMP6COUNTERS`], [`StatCounters`] here beside the increments that use them: the
//!   scalar words per CPU (`hal::percpu::Counters`, the C's `cpumem`, N6.4a's R13 for IPv4),
//!   the 256-word histograms (`ip6s_nxthist`, `icp6s_outhist`, `icp6s_inhist`) as LZ's
//!   shared atomics with relaxed adds. `hal::percpu` gives each CPU of `MAXCPUS` (256) a copy
//!   at compile time where the C allocates `ncpus` at boot: per CPU, the three histograms
//!   alone were 1.5 MiB of the 1.86 MiB of `.bss` the whole structures took (N6.5a's review,
//!   S3); split, the two statistics take about 0.37 MiB. Both are summed into the one
//!   `struct ip6stat` / `struct icmp6stat` image by [`StatCounters::read_into`], which
//!   `net.inet6.ip6.stats` and `net.inet6.icmp6.stats` call on their heap image (the C's
//!   `malloc(M_TEMP)` and `counters_read`); only the scalar words' sums pass through the
//!   stack (`Counters::read` returns them by value: 145 and 29 words). `ip6stat_inc`,
//!   `ip6stat_add`, `ip6stat_inc_idx`, `icmp6stat_inc` and `icmp6stat_inc_hist` are
//!   `#[inline]` (per packet, across the crate boundary); a word's place costs one or two
//!   compares, folded away when the counter is a constant.
//! - The configuration variables, LZ's atomics in `bsd`'s `in6_proto.rs`, are here, the
//!   header's `extern`s (`kernel::netinet::ip_var`'s precedent); `bsd`'s `in6_proto.rs`
//!   re-exports them for its importers (pf, TCP, UDP, IP-in-IP) until they move, and their
//!   `moved` rows sit under `kernel::netinet6::in6_proto`, the C file's module (N6.5d).
//!   `ip6_mtudisc_timeout` joins them.
//! - N6.5c: LZ's `struct ip6_pktopts` (raw `NonNull`s to the `malloc(M_IP6OPT)` packet info
//!   and extension headers, read through `unsafe` by `ip6_output`, the source selection and
//!   the socket options, with an invariant written on the field) -> [`Ip6Pktopts`], whose
//!   allocations it owns: an `MBox` of the `In6Pktinfo`, an `MArray` of each header's bytes
//!   (the C's `malloc`s of `M_IP6OPT`, freed by the drop: `ip6_clearpktopts`), and
//!   [`Ip6OutOpts`], the borrowed view `ip6_output` takes (D-10: the pcb's options lend one,
//!   MLD its `const` router alert). LZ's `struct ip6_moptions` (a `LIST_HEAD` of `bsd`'s
//!   `in6_multi_mship` entries) -> [`Ip6Moptions`], its memberships an `In6Memberships`
//!   (`kernel::netinet6::in6_var`, D-11). The sizes against the C's (`vmstat -m`'s `ip6opt`
//!   and `ipmoptions` rows, D-4): `struct ip6_pktopts` 56 bytes (LP64), [`Ip6Pktopts`] the
//!   size test's; the C's `struct ip6_moptions` 16, [`Ip6Moptions`] the size test's (the
//!   memberships' array and count where the C has a list head).
//! - N6.5d: LZ's `struct ip6q` and `struct ip6asfrag` (pool items written whole through
//!   `unsafe`, linked by `bsd`'s `TailqEntry`/`ListEntry` into a `Sync`-by-hand queue head,
//!   every member a `Cell`) -> [`Ip6q`] and [`Ip6asfrag`], items of the "ip6q" and "ip6af"
//!   `BoxPool`s (`kernel::netinet6::frag6`'s, the C's names), the IPv4 shape of
//!   `kernel::netinet::ip_var`'s `Ipq`/`Ipqent` (D-12): the datagram owns its fragments on an
//!   `OwnedSlist` by offset, the reassembly queue owns the datagrams on another
//!   (`frag6::FRAG6`), and a datagram is changed off its queue (its members plain fields; only
//!   `ip6q_ttl`, which `frag6_slowtimo` counts down in place, is a `Cell`). The fragment owns
//!   its mbuf (`ip6af_m`). The sizes against the C's (`vmstat -m`'s `ip6q` and `ip6af`, D-4):
//!   `frag6`'s size test.

use core::cell::Cell;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use hal::malloc::{MArray, MBox};
use hal::owned::{OwnedSlist, SlistLink};
use hal::percpu::Counters;
use hal::pool::{BoxPool, Pooled};
use hal::sys::netinet_icmp6::{ICP6S_NCOUNTERS, Icmp6statCounters};
use hal::sys::netinet_ip6::IPV6_DEFHLIM;
use hal::sys::netinet6_in6::{IPV6_DEFAULT_MULTICAST_HOPS, In6Addr, In6Pktinfo};
use hal::sys::netinet6_ip6_var::{IP6S_NCOUNTERS, Ip6statCounters};

use crate::assert::kpanic;
use crate::net::mbuf::MbufBox;
use crate::netinet::ip_var::IPMTUDISCTIMEOUT;
use crate::netinet6::in6_var::In6Memberships;

/// Default: send at min MTU for multicast.
pub const IP6PO_MINMTU_MCASTONLY: i32 = -1;
/// Always perform pmtu disc.
pub const IP6PO_MINMTU_DISABLE: i32 = 0;
/// Always send at min MTU.
pub const IP6PO_MINMTU_ALL: i32 = 1;

/// Disable fragmentation (`IPV6_DONTFRAG`).
pub const IP6PO_DONTFRAG: i32 = 0x04;

// flags passed to ip6_output or ip6_forward as last parameter

/// Allow :: as the source address.
pub const IPV6_UNSPECSRC: i32 = 0x01;
/// Most of IPv6 header exists.
pub const IPV6_FORWARDING: i32 = 0x02;
/// Use minimum MTU (`IPV6_USE_MIN_MTU`).
pub const IPV6_MINMTU: i32 = 0x04;
/// Redirected by pf.
pub const IPV6_REDIRECT: i32 = 0x08;
/// Only packets processed by IPsec.
pub const IPV6_FORWARDING_IPSEC: i32 = 0x10;

/// `M_IP6OPT`: IPv6 options (`malloc.h`).
pub const M_IP6OPT: i32 = 123;

/// `struct ip6_moptions`: attached to `inpcb.inp_moptions6` and passed to `ip6_output` when
/// IPv6 multicast options are in use (the module's redesign).
pub struct Ip6Moptions {
    /// `im6o_memberships`: the groups the socket joined.
    pub im6o_memberships: In6Memberships,
    /// ifp index for outgoing multicasts.
    pub im6o_ifidx: u16,
    /// Hoplimit for outgoing multicasts.
    pub im6o_hlim: u8,
    /// 1 >= hear sends if a member.
    pub im6o_loop: u8,
}

impl Ip6Moptions {
    /// Options of interface `ifidx`, hop limit `hlim` and loopback switch `loop_` with no
    /// membership: a new set's defaults (`ip6_setmoptions`), and the three members
    /// `ip6_output` reads that its callers lend (ND's and MLD's stack `ip6_moptions`, the
    /// transports' copy of their pcb's).
    pub const fn output(ifidx: u16, hlim: u8, loop_: u8) -> Self {
        Self {
            im6o_memberships: In6Memberships::new(),
            im6o_ifidx: ifidx,
            im6o_hlim: hlim,
            im6o_loop: loop_,
        }
    }
}

/// `struct ip6_pktopts`: control options for outgoing packets, owning their allocations (the
/// module's redesign). A new value is `ip6_initpktopts`' ([`Ip6Pktopts::new`]).
pub struct Ip6Pktopts {
    /// Hoplimit for outgoing packets (-1: the default).
    pub ip6po_hlim: i32,
    /// Outgoing IF/address information (`malloc(M_IP6OPT)`).
    pub ip6po_pktinfo: Option<MBox<In6Pktinfo>>,
    /// Hop-by-Hop options header, `(ip6h_len + 1) << 3` bytes (`ip6_setpktopt`'s check).
    pub ip6po_hbh: Option<MArray<u8>>,
    /// Destination options header (before a routing header), `(ip6d_len + 1) << 3` bytes.
    pub ip6po_dest1: Option<MArray<u8>>,
    /// Destination options header (after a routing header), `(ip6d_len + 1) << 3` bytes.
    pub ip6po_dest2: Option<MArray<u8>>,
    /// Traffic class (-1: the default).
    pub ip6po_tclass: i32,
    /// Fragment vs PMTU discovery policy (`IP6PO_MINMTU_*`).
    pub ip6po_minmtu: i32,
    /// `IP6PO_*` flags.
    pub ip6po_flags: i32,
}

impl Ip6Pktopts {
    /// `ip6_initpktopts(opt)`: no option, the non-zero defaults set (the default hop limit and
    /// traffic class, the minimum MTU for multicast only).
    pub const fn new() -> Self {
        Self {
            ip6po_hlim: -1,
            ip6po_pktinfo: None,
            ip6po_hbh: None,
            ip6po_dest1: None,
            ip6po_dest2: None,
            ip6po_tclass: -1,
            ip6po_minmtu: IP6PO_MINMTU_MCASTONLY,
            ip6po_flags: 0,
        }
    }

    /// `*opt->ip6po_pktinfo`, read by value (`None` without packet info).
    #[inline]
    pub fn pktinfo(&self) -> Option<In6Pktinfo> {
        self.ip6po_pktinfo.as_deref().copied()
    }

    /// The view of these options `ip6_output` takes.
    #[inline]
    pub fn as_opts(&self) -> Ip6OutOpts<'_> {
        let o = Ip6OutOpts {
            hlim: self.ip6po_hlim,
            pktinfo: self.ip6po_pktinfo.as_deref(),
            tclass: self.ip6po_tclass,
            minmtu: self.ip6po_minmtu,
            flags: self.ip6po_flags,
            ..Ip6OutOpts::new()
        };
        // Every header here is `ip6_setpktopt`'s copy of one it checked whole (or
        // `copypktopts`' copy of such a copy).
        match o.with_exthdrs(
            self.ip6po_hbh.as_deref(),
            self.ip6po_dest1.as_deref(),
            self.ip6po_dest2.as_deref(),
        ) {
            Some(o) => o,
            None => kpanic(format_args!(
                "ip6_pktopts: an extension header that is not whole"
            )),
        }
    }
}

impl Default for Ip6Pktopts {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether `h` is a whole IPv6 extension header: at least a `struct ip6_ext` (2 bytes), and
/// exactly `(ip6e_len + 1) << 3` bytes, `ip6e_len` its second byte (what `ip6_setpktopt`
/// checks of a user's header, in that order; so never fewer than 8; MLD's `const` router
/// alert asserts it at compile time).
pub const fn ip6_exthdr_whole(h: &[u8]) -> bool {
    h.len() >= 2 && h.len() == ((h[1] as usize) + 1) << 3
}

/// The options `ip6_output` reads, borrowed (the C's `struct ip6_pktopts *opt`, D-10): the
/// members of an [`Ip6Pktopts`] ([`Ip6Pktopts::as_opts`]), or a caller's own (MLD's
/// router-alert header, a `const` image). Each extension header is whole
/// ([`ip6_exthdr_whole`]): the header fields are private, set only by
/// [`Ip6OutOpts::with_exthdrs`], which checks them (N6.5c review N5), so `ip6_output`'s walk of
/// the chain by the headers' own length bytes meets the lengths it copied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ip6OutOpts<'a> {
    /// `ip6po_hlim` (-1: the default).
    pub hlim: i32,
    /// `ip6po_pktinfo`.
    pub pktinfo: Option<&'a In6Pktinfo>,
    /// `ip6po_hbh`, whole.
    hbh: Option<&'a [u8]>,
    /// `ip6po_dest1`, whole.
    dest1: Option<&'a [u8]>,
    /// `ip6po_dest2`, whole.
    dest2: Option<&'a [u8]>,
    /// `ip6po_tclass` (-1: the default).
    pub tclass: i32,
    /// `ip6po_minmtu`.
    pub minmtu: i32,
    /// `ip6po_flags`.
    pub flags: i32,
}

impl<'a> Ip6OutOpts<'a> {
    /// `ip6_initpktopts`' values: no option, the defaults.
    pub const fn new() -> Self {
        Self {
            hlim: -1,
            pktinfo: None,
            hbh: None,
            dest1: None,
            dest2: None,
            tclass: -1,
            minmtu: IP6PO_MINMTU_MCASTONLY,
            flags: 0,
        }
    }

    /// These options with the extension headers `hbh`, `dest1` and `dest2` (`None`: none of
    /// that kind); `None` when one is not whole ([`ip6_exthdr_whole`]).
    pub fn with_exthdrs(
        self,
        hbh: Option<&'a [u8]>,
        dest1: Option<&'a [u8]>,
        dest2: Option<&'a [u8]>,
    ) -> Option<Self> {
        let whole = |h: Option<&[u8]>| h.is_none_or(ip6_exthdr_whole);
        (whole(hbh) && whole(dest1) && whole(dest2)).then_some(Self {
            hbh,
            dest1,
            dest2,
            ..self
        })
    }

    /// `ip6po_hbh`: the hop-by-hop options header, whole.
    #[inline]
    pub fn hbh(&self) -> Option<&'a [u8]> {
        self.hbh
    }

    /// `ip6po_dest1`: the first destination options header, whole.
    #[inline]
    pub fn dest1(&self) -> Option<&'a [u8]> {
        self.dest1
    }

    /// `ip6po_dest2`: the second destination options header, whole.
    #[inline]
    pub fn dest2(&self) -> Option<&'a [u8]> {
        self.dest2
    }
}

impl Default for Ip6OutOpts<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct ip6q`: IP6 reassembly queue structure, an item of the "ip6q" pool. Each fragment
/// being reassembled is attached to one of these structures. \[Q\] All of it is changed under
/// `frag6_mutex` (`kernel::netinet6::frag6::FRAG6`'s guard), the datagram taken off the
/// queue for the change and put back in its place, but for its time to live, which
/// `frag6_slowtimo` counts down while it is listed.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct ip6q's ip6q_* names so the C can be grepped"
)]
pub struct Ip6q {
    /// `ip6q_queue`: the next datagram of `frag6_queue`.
    ip6q_queue: SlistLink<Ip6q>,
    /// `ip6q_asfrag`: the fragments, by offset.
    pub ip6q_asfrag: OwnedSlist<Ip6asfragList>,
    /// `ip6q_src`.
    pub ip6q_src: In6Addr,
    /// `ip6q_dst`.
    pub ip6q_dst: In6Addr,
    /// `ip6q_unfrglen`: len of unfragmentable part, -1 until the first fragment arrives.
    pub ip6q_unfrglen: i32,
    /// `ip6q_nfrag`: # of fragments.
    pub ip6q_nfrag: i32,
    /// `ip6q_rdomain`: routing domain for reassembly.
    pub ip6q_rdomain: u32,
    /// `ip6q_ident`: fragment identification (network order, as the header holds it).
    pub ip6q_ident: u32,
    /// `ip6q_nxt`: `ip6f_nxt` in first fragment.
    pub ip6q_nxt: u8,
    /// `ip6q_ecn`.
    pub ip6q_ecn: u8,
    /// `ip6q_ttl`: time to live in slowtimo units.
    pub ip6q_ttl: Cell<u8>,
}

impl Ip6q {
    /// The queue of datagram `ident` from `src` to `dst` in `rdomain`, empty (the first
    /// fragment has not arrived: `ip6q_unfrglen` -1), with ECN bits `ecn` and `ttl` to live.
    pub const fn new(
        rdomain: u32,
        ident: u32,
        src: In6Addr,
        dst: In6Addr,
        ecn: u8,
        ttl: u8,
    ) -> Self {
        Self {
            ip6q_queue: SlistLink::new(),
            ip6q_asfrag: OwnedSlist::new(),
            ip6q_src: src,
            ip6q_dst: dst,
            ip6q_unfrglen: -1,
            ip6q_nfrag: 0,
            ip6q_rdomain: rdomain,
            ip6q_ident: ident,
            ip6q_nxt: 0,
            ip6q_ecn: ecn,
            ip6q_ttl: Cell::new(ttl),
        }
    }
}

impl Pooled for Ip6q {
    const POOL: &'static BoxPool<Self> = &crate::netinet6::frag6::IP6Q_POOL;
}

hal::slist_adapter!(
    /// `TAILQ_HEAD(ip6q_head, ip6q)` through `ip6q_queue`: `frag6_queue`.
    pub Ip6qList: Ip6q, ip6q_queue
);

/// `struct ip6asfrag`: a fragment on a reassembly queue, an item of the "ip6af" pool.
pub struct Ip6asfrag {
    /// `ip6af_list`: the next fragment of its datagram.
    ip6af_list: SlistLink<Ip6asfrag>,
    /// `ip6af_m`: the fragment; `Some` while it is queued.
    pub ip6af_m: Option<MbufBox>,
    /// `ip6af_offset`: offset in `ip6af_m` to next header.
    pub ip6af_offset: i32,
    /// `ip6af_frglen`: fragmentable part length.
    pub ip6af_frglen: i32,
    /// `ip6af_off`: fragment offset.
    pub ip6af_off: i32,
    /// `ip6af_mff`: more fragment bit in frag off (network order).
    pub ip6af_mff: u16,
}

impl Ip6asfrag {
    /// The fragment `m` at fragment offset `off` with `frglen` bytes of its fragmentable part
    /// at `offset`, more of them following when `mff`.
    pub const fn new(m: MbufBox, offset: i32, frglen: i32, off: i32, mff: u16) -> Self {
        Self {
            ip6af_list: SlistLink::new(),
            ip6af_m: Some(m),
            ip6af_offset: offset,
            ip6af_frglen: frglen,
            ip6af_off: off,
            ip6af_mff: mff,
        }
    }
}

impl Pooled for Ip6asfrag {
    const POOL: &'static BoxPool<Self> = &crate::netinet6::frag6::IP6AF_POOL;
}

hal::slist_adapter!(
    /// `LIST_HEAD(ip6asfrag_list, ip6asfrag)` through `ip6af_list`: a datagram's fragments.
    pub Ip6asfragList: Ip6asfrag, ip6af_list
);

/// `ip6s_nxthist`: the image word it starts at and its length (the C's `[256]`).
const IP6_NXTHIST: (usize, usize) = (
    Ip6statCounters::Ip6sNxthist as usize,
    Ip6statCounters::Ip6sM1 as usize - Ip6statCounters::Ip6sNxthist as usize,
);
/// The histogram words of `struct ip6stat`.
const IP6_NHIST: usize = IP6_NXTHIST.1;
/// The scalar words of `struct ip6stat`.
const IP6_NSCALARS: usize = IP6S_NCOUNTERS - IP6_NHIST;
/// `icp6s_outhist` and `icp6s_inhist`: where each starts in the image, and its length.
const ICMP6_HISTS: [(usize, usize); 2] = [
    (
        Icmp6statCounters::Icp6sOuthist as usize,
        Icmp6statCounters::Icp6sBadcode as usize - Icmp6statCounters::Icp6sOuthist as usize,
    ),
    (
        Icmp6statCounters::Icp6sInhist as usize,
        Icmp6statCounters::Icp6sNdToomanyopt as usize - Icmp6statCounters::Icp6sInhist as usize,
    ),
];
/// The histogram words of `struct icmp6stat`.
const ICMP6_NHIST: usize = ICMP6_HISTS[0].1 + ICMP6_HISTS[1].1;
/// The scalar words of `struct icmp6stat`.
const ICMP6_NSCALARS: usize = ICP6S_NCOUNTERS - ICMP6_NHIST;

/// Where a statistics structure keeps its histograms: the image word each starts at and its
/// length, in image order. A type rather than a field, so a [`StatCounters`] static holds
/// only zeros and stays in `.bss` (a reference in it would move the whole static, its
/// per-CPU copies included, into `.data` and the kernel image).
pub trait StatHists {
    /// The histograms' ranges of image words.
    const HISTS: &'static [(usize, usize)];
}

/// `struct ip6stat`'s histogram, `ip6s_nxthist`.
pub struct Ip6statHists;

impl StatHists for Ip6statHists {
    const HISTS: &'static [(usize, usize)] = &[IP6_NXTHIST];
}

/// `struct icmp6stat`'s histograms, `icp6s_outhist` and `icp6s_inhist`.
pub struct Icmp6statHists;

impl StatHists for Icmp6statHists {
    const HISTS: &'static [(usize, usize)] = &ICMP6_HISTS;
}

/// The counters of a statistics structure of `N` `uint64_t` words (`struct cpumem *` of
/// `counters_alloc(N)`): its `S` scalar words per CPU ([`Counters`]), its `H` histogram
/// words (the ranges `L` names) shared, LZ's atomics. The image is summed by
/// [`StatCounters::read_into`].
pub struct StatCounters<L, const N: usize, const S: usize, const H: usize> {
    scalars: Counters<S>,
    hist: [AtomicU64; H],
    layout: PhantomData<L>,
}

/// Where a word of the image is kept.
enum Word {
    /// The scalar of that index, per CPU.
    Cpu(usize),
    /// The histogram word of that index.
    Hist(usize),
}

impl<L: StatHists, const N: usize, const S: usize, const H: usize> StatCounters<L, N, S, H> {
    /// `counters_alloc(N)`: every word at 0. `L`'s ranges lie inside the image, in order,
    /// and cover `H` words; `S + H` is `N` (checked at compile time where a static is built).
    const fn new() -> Self {
        let hists = L::HISTS;
        let mut covered = 0;
        let mut end = 0;
        let mut i = 0;
        while i < hists.len() {
            assert!(hists[i].0 >= end && hists[i].0 + hists[i].1 <= N);
            covered += hists[i].1;
            end = hists[i].0 + hists[i].1;
            i += 1;
        }
        assert!(covered == H && S + H == N);
        Self {
            scalars: Counters::new(),
            hist: [const { AtomicU64::new(0) }; H],
            layout: PhantomData,
        }
    }

    /// Where word `c` of the image is kept: in a histogram, or the scalar whose index is
    /// `c` less the histogram words before it.
    #[inline]
    fn word(c: usize) -> Word {
        let mut before = 0;
        for &(start, len) in L::HISTS {
            if c < start {
                break;
            }
            if c - start < len {
                return Word::Hist(before + (c - start));
            }
            before += len;
        }
        Word::Cpu(c - before)
    }

    /// `counters_inc(cm, c)`.
    #[inline]
    pub fn inc(&self, c: usize) {
        self.add(c, 1);
    }

    /// `counters_add(cm, c, v)`.
    #[inline]
    pub fn add(&self, c: usize, v: u64) {
        match Self::word(c) {
            Word::Cpu(i) => self.scalars.add(i, v),
            Word::Hist(i) => {
                self.hist[i].fetch_add(v, Ordering::Relaxed);
            }
        }
    }

    /// `counters_dec(cm, c)`.
    #[inline]
    pub fn dec(&self, c: usize) {
        match Self::word(c) {
            Word::Cpu(i) => self.scalars.dec(i),
            Word::Hist(i) => {
                self.hist[i].fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    /// `counters_read(cm, (uint64_t *)image, N, NULL)`: the sums of the counters written into
    /// `image`, word after word in native byte order, as long as `image` is (the statistics
    /// sysctls give it the structure's `N * 8` bytes, on the heap as the C's `malloc`). Each
    /// CPU's scalar words are read whole (`Counters::read`); a histogram word is one load.
    pub fn read_into(&self, image: &mut [u8]) {
        let scalars = self.scalars.read();
        let (words, _) = image.as_chunks_mut::<{ size_of::<u64>() }>();
        for (c, w) in words.iter_mut().enumerate().take(N) {
            let v = match Self::word(c) {
                Word::Cpu(i) => scalars[i],
                Word::Hist(i) => self.hist[i].load(Ordering::Relaxed),
            };
            *w = v.to_ne_bytes();
        }
    }

    /// The sums, word by word: [`StatCounters::read_into`] over an array on the caller's
    /// stack, for tests (a kernel stack takes the image on the heap).
    #[cfg(any(test, feature = "doubles"))]
    pub fn read(&self) -> [u64; N] {
        let mut image = [[0u8; size_of::<u64>()]; N];
        self.read_into(image.as_flattened_mut());
        image.map(u64::from_ne_bytes)
    }
}

/// `ip6counters`: the IPv6 statistics (`ip6_init`'s `counters_alloc(ip6s_ncounters)`), the
/// scalar words per CPU, `ip6s_nxthist` shared.
pub static IP6COUNTERS: StatCounters<Ip6statHists, IP6S_NCOUNTERS, IP6_NSCALARS, IP6_NHIST> =
    StatCounters::new();

/// `icmp6counters`: the ICMPv6 statistics (`icmp6_init`'s
/// `counters_alloc(icp6s_ncounters)`), the scalar words per CPU, `icp6s_outhist` and
/// `icp6s_inhist` shared.
pub static ICMP6COUNTERS: StatCounters<
    Icmp6statHists,
    ICP6S_NCOUNTERS,
    ICMP6_NSCALARS,
    ICMP6_NHIST,
> = StatCounters::new();

// Internet configuration info (`in6_proto.c`).

/// \[a\] `ip6_forwarding`: no forwarding unless sysctl to enable.
pub static IP6_FORWARDING: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ip6_mforwarding`: no multicast forwarding unless ...
pub static IP6_MFORWARDING: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ip6_multipath`: no using multipath routes unless ...
pub static IP6_MULTIPATH: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ip6_sendredirects`.
pub static IP6_SENDREDIRECTS: AtomicI32 = AtomicI32::new(1);
/// \[a\] `ip6_defhlim`: default hop limit.
pub static IP6_DEFHLIM: AtomicI32 = AtomicI32::new(IPV6_DEFHLIM as i32);
/// \[a\] `ip6_defmcasthlim`: default multicast hop limit.
pub static IP6_DEFMCASTHLIM: AtomicI32 = AtomicI32::new(IPV6_DEFAULT_MULTICAST_HOPS);
/// \[a\] `ip6_maxfragpackets`: maximum packets in reassembly queue.
pub static IP6_MAXFRAGPACKETS: AtomicI32 = AtomicI32::new(200);
/// \[a\] `ip6_maxfrags`: maximum fragments in reassembly queue.
pub static IP6_MAXFRAGS: AtomicI32 = AtomicI32::new(200);
/// \[a\] `ip6_hdrnestlimit`: upper limit of # of extension headers (appropriate?).
pub static IP6_HDRNESTLIMIT: AtomicI32 = AtomicI32::new(10);
/// \[a\] `ip6_dad_count`: DupAddrDetectionTransmits.
pub static IP6_DAD_COUNT: AtomicI32 = AtomicI32::new(1);
/// `ip6_dad_pending`: number of currently running DADs.
pub static IP6_DAD_PENDING: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ip6_mcast_pmtu`: enable pMTU discovery for multicast?
pub static IP6_MCAST_PMTU: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ip6_neighborgcthresh`: threshold # of NDP entries for GC.
pub static IP6_NEIGHBORGCTHRESH: AtomicI32 = AtomicI32::new(2048);
/// \[a\] `ip6_maxdynroutes`: max # of routes created via redirect.
pub static IP6_MAXDYNROUTES: AtomicI32 = AtomicI32::new(4096);
/// \[a\] `ip6_mtudisc_timeout`: mtu discovery.
pub static IP6_MTUDISC_TIMEOUT: AtomicI32 = AtomicI32::new(IPMTUDISCTIMEOUT);
/// `icmp6_redirtimeout`: cache time for redirect routes, 10 minutes (changed under
/// `sysctl_lock`).
pub static ICMP6_REDIRTIMEOUT: AtomicI32 = AtomicI32::new(10 * 60);
/// \[a\] `icmp6errppslim`: 100pps.
pub static ICMP6ERRPPSLIM: AtomicI32 = AtomicI32::new(100);

/// `ip6stat_inc(c)`.
#[inline]
pub fn ip6stat_inc(c: Ip6statCounters) {
    IP6COUNTERS.inc(c as usize);
}

/// `ip6stat_add(c, v)`.
#[inline]
pub fn ip6stat_add(c: Ip6statCounters, v: u64) {
    IP6COUNTERS.add(c as usize, v);
}

/// `counters_dec(ip6counters, c)`.
#[inline]
pub fn ip6stat_dec(c: Ip6statCounters) {
    IP6COUNTERS.dec(c as usize);
}

/// `ip6stat_inc(base + i)`: entry `i` of the counter array that starts at `base`
/// (`ip6s_nxthist`, `ip6s_m2m`, `ip6s_sources_*`).
#[inline]
pub fn ip6stat_inc_idx(base: Ip6statCounters, i: usize) {
    IP6COUNTERS.inc(base as usize + i);
}

/// `icmp6stat_inc(c)` (`<netinet/icmp6.h>`).
#[inline]
pub fn icmp6stat_inc(c: Icmp6statCounters) {
    ICMP6COUNTERS.inc(c as usize);
}

/// `icmp6stat_inc(icp6s_outhist + type)` / `icmp6stat_inc(icp6s_inhist + type)`: bumps entry
/// `type_` of histogram `hist`.
#[inline]
pub fn icmp6stat_inc_hist(hist: Icmp6statCounters, type_: u8) {
    ICMP6COUNTERS.inc(hist as usize + usize::from(type_));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counters_count_per_index() {
        let before = IP6COUNTERS.read();
        ip6stat_inc(Ip6statCounters::Ip6sNogif);
        ip6stat_add(Ip6statCounters::Ip6sNogif, 4);
        ip6stat_dec(Ip6statCounters::Ip6sNogif);
        ip6stat_inc_idx(Ip6statCounters::Ip6sNxthist, 58);
        let after = IP6COUNTERS.read();
        let nogif = Ip6statCounters::Ip6sNogif as usize;
        let hist58 = Ip6statCounters::Ip6sNxthist as usize + 58;
        assert!(after[nogif].wrapping_sub(before[nogif]) >= 4);
        assert!(after[hist58] > before[hist58]);

        let before = ICMP6COUNTERS.read();
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        icmp6stat_inc_hist(Icmp6statCounters::Icp6sOuthist, 131);
        let after = ICMP6COUNTERS.read();
        let short = Icmp6statCounters::Icp6sTooshort as usize;
        let out131 = Icmp6statCounters::Icp6sOuthist as usize + 131;
        assert!(after[short] > before[short]);
        assert!(after[out131] > before[out131]);
    }

    #[test]
    fn the_histograms_are_the_c_arrays() {
        assert_eq!(IP6_NXTHIST, (23, 256));
        assert_eq!(ICMP6_HISTS, [(3, 256), (3 + 256 + 5, 256)]);
        assert_eq!((IP6_NSCALARS, ICMP6_NSCALARS), (401 - 256, 541 - 512));
    }

    #[test]
    fn a_split_image_keeps_every_word_in_place() {
        // A small image: words 0-1 scalar, 2-4 a histogram, 5 scalar, 6-7 a histogram, 8
        // scalar. Every word gets its own count (word c bumped c + 1 times).
        struct TestHists;
        impl StatHists for TestHists {
            const HISTS: &'static [(usize, usize)] = &[(2, 3), (6, 2)];
        }
        static T: StatCounters<TestHists, 9, 4, 5> = StatCounters::new();
        for c in 0..9 {
            T.add(c, c as u64 + 1);
        }
        T.dec(3);
        T.dec(8);
        let mut image = [0u8; 9 * 8];
        T.read_into(&mut image);
        let words: [u64; 9] = core::array::from_fn(|c| {
            u64::from_ne_bytes(image[c * 8..c * 8 + 8].try_into().unwrap())
        });
        assert_eq!(words, [1, 2, 3, 3, 5, 6, 7, 8, 8]);
        assert_eq!(T.read(), words);
        // A short image takes the words it has room for.
        let mut short = [0u8; 3 * 8 + 4];
        T.read_into(&mut short);
        assert_eq!(u64::from_ne_bytes(short[16..24].try_into().unwrap()), 3);
        assert_eq!(short[24..], [0; 4]);
    }

    #[test]
    fn the_image_words_are_the_c_offsets() {
        let total = Ip6statCounters::Ip6sTotal as usize;
        let m1 = Ip6statCounters::Ip6sM1 as usize;
        let hist6 = Ip6statCounters::Ip6sNxthist as usize + 6;
        let before = IP6COUNTERS.read();
        ip6stat_inc(Ip6statCounters::Ip6sTotal);
        ip6stat_inc(Ip6statCounters::Ip6sM1);
        ip6stat_inc_idx(Ip6statCounters::Ip6sNxthist, 6);
        let mut image = std::vec![0u8; IP6S_NCOUNTERS * 8];
        IP6COUNTERS.read_into(&mut image);
        let word = |c: usize| u64::from_ne_bytes(image[c * 8..c * 8 + 8].try_into().unwrap());
        assert!(word(total) > before[total]);
        assert!(word(m1) > before[m1]);
        assert!(word(hist6) > before[hist6]);

        let inhist = Icmp6statCounters::Icp6sInhist as usize + 128;
        let toomany = Icmp6statCounters::Icp6sNdToomanyopt as usize;
        let before = ICMP6COUNTERS.read();
        icmp6stat_inc_hist(Icmp6statCounters::Icp6sInhist, 128);
        icmp6stat_inc(Icmp6statCounters::Icp6sNdToomanyopt);
        let after = ICMP6COUNTERS.read();
        assert!(after[inhist] > before[inhist]);
        assert!(after[toomany] > before[toomany]);
    }

    /// N6.5c review N5: the view's extension headers are whole or refused; the owned options'
    /// view carries theirs.
    #[test]
    fn the_view_takes_only_whole_extension_headers() {
        let one = [0u8, 0, 1, 4, 0, 0, 0, 0];
        let two = [0u8, 1, 1, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(ip6_exthdr_whole(&one) && ip6_exthdr_whole(&two));
        let short_of_its_length = [0u8, 1, 1, 4, 0, 0, 0, 0];
        for (what, h) in [
            ("empty", &[][..]),
            ("one byte", &[0u8][..]),
            ("under a unit", &[0u8, 0, 1, 2, 0, 0][..]),
            (
                "shorter than its length byte says",
                &short_of_its_length[..],
            ),
            ("longer than its length byte says", &two[..12]),
        ] {
            assert!(!ip6_exthdr_whole(h), "{what}");
            let o = Ip6OutOpts::new();
            assert_eq!(o.with_exthdrs(Some(h), None, None), None, "hbh {what}");
            assert_eq!(o.with_exthdrs(None, Some(h), None), None, "dest1 {what}");
            assert_eq!(o.with_exthdrs(None, None, Some(h)), None, "dest2 {what}");
        }
        let mut o = Ip6OutOpts::new();
        o.hlim = 9;
        let o = o
            .with_exthdrs(Some(&one), None, Some(&two))
            .expect("whole headers");
        assert_eq!(
            (o.hbh(), o.dest1(), o.dest2()),
            (Some(&one[..]), None, Some(&two[..]))
        );
        assert_eq!(o.hlim, 9, "the other options kept");

        let mut sticky = Ip6Pktopts::new();
        sticky.ip6po_dest1 = MArray::new(M_IP6OPT, hal::malloc::M_WAITOK, two.len(), |i| two[i]);
        let v = sticky.as_opts();
        assert_eq!(
            (v.hbh(), v.dest1(), v.dest2()),
            (None, Some(&two[..]), None)
        );
    }

    /// The options' sizes against the C's (D-4: `vmstat -m`'s `ip6opt` and `ipmoptions`
    /// malloc rows count their blocks): the C's `struct ip6_pktopts` is 56 bytes on LP64 (an
    /// `int`, four pointers, three `int`s, padded), its `struct ip6_moptions` 16 (a list head and
    /// four bytes); LZ's were the same structures (`NonNull`s, a `ListHead`). The kernel's
    /// options own their allocations: an `MBox` is 32 bytes (the pointer, its malloc type, its
    /// allocator), an `MArray` 40, so `Ip6Pktopts` is 168 (a 256-byte bucket where the C's fits 64) and
    /// `Ip6Moptions` 56 (64 where the C's fits 16, the memberships' array and count in place of
    /// the list head). The view `ip6_output` takes is nine words.
    #[test]
    fn the_options_sizes_against_the_c() {
        use core::mem::size_of;
        #[allow(dead_code, reason = "a layout replica: only its size is read")]
        #[repr(C)]
        struct CIp6Pktopts {
            ip6po_hlim: i32,
            ip6po_pktinfo: usize,
            ip6po_hbh: usize,
            ip6po_dest1: usize,
            ip6po_dest2: usize,
            ip6po_tclass: i32,
            ip6po_minmtu: i32,
            ip6po_flags: i32,
        }
        #[allow(dead_code, reason = "a layout replica: only its size is read")]
        #[repr(C)]
        struct CIp6Moptions {
            im6o_memberships: usize,
            im6o_ifidx: u16,
            im6o_hlim: u8,
            im6o_loop: u8,
        }
        std::println!(
            "ip6_pktopts C {} kernel {}; ip6_moptions C {} kernel {}; Ip6OutOpts {}",
            size_of::<CIp6Pktopts>(),
            size_of::<Ip6Pktopts>(),
            size_of::<CIp6Moptions>(),
            size_of::<Ip6Moptions>(),
            size_of::<Ip6OutOpts<'_>>()
        );
        assert_eq!(size_of::<CIp6Pktopts>(), 56);
        assert_eq!(size_of::<CIp6Moptions>(), 16);
        assert_eq!(size_of::<Ip6Pktopts>(), 168);
        assert_eq!(size_of::<Ip6Moptions>(), 56);
        assert_eq!(size_of::<Ip6OutOpts<'_>>(), 72);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet6/ip6_var.h");
        let ipv6 = crate::reftest::assert_defines!(defs;
            IPV6_UNSPECSRC, IPV6_FORWARDING, IPV6_MINMTU, IPV6_REDIRECT, IPV6_FORWARDING_IPSEC);
        crate::reftest::assert_complete(&defs, "IPV6_", &ipv6);
        let po = crate::reftest::assert_defines!(defs;
            IP6PO_MINMTU_MCASTONLY, IP6PO_MINMTU_DISABLE, IP6PO_MINMTU_ALL, IP6PO_DONTFRAG);
        crate::reftest::assert_complete(&defs, "IP6PO_", &po);
    }
}
/* </TESTS> */
