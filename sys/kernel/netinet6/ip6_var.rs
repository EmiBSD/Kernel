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
//! words) is `hal::sys::netinet6_ip6_var`; `struct ip6q`, `struct ip6asfrag`, `struct
//! ip6_moptions` and `struct ip6_pktopts` are `bsd`'s until their modules move (N6.5c/d).
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
//!   [`ICMP6COUNTERS`], `hal::percpu::Counters` here beside the increments that use them (the
//!   C's `cpumem`, N6.4a's R13 for IPv4): read whole by `net.inet6.ip6.stats` and
//!   `net.inet6.icmp6.stats` (`Counters::read`). `ip6stat_inc`, `ip6stat_add`,
//!   `ip6stat_inc_idx`, `icmp6stat_inc` and `icmp6stat_inc_hist` are `#[inline]` (per packet,
//!   across the crate boundary).
//! - The configuration variables, LZ's atomics in `bsd`'s `in6_proto.rs`, are here, the
//!   header's `extern`s (`kernel::netinet::ip_var`'s precedent); `bsd`'s `in6_proto.rs`
//!   re-exports them until the protocol table moves (N6.5d). `ip6_mtudisc_timeout` joins
//!   them.

use core::sync::atomic::AtomicI32;

use hal::percpu::Counters;
use hal::sys::netinet_icmp6::{ICP6S_NCOUNTERS, Icmp6statCounters};
use hal::sys::netinet_ip6::IPV6_DEFHLIM;
use hal::sys::netinet6_in6::IPV6_DEFAULT_MULTICAST_HOPS;
use hal::sys::netinet6_ip6_var::{IP6S_NCOUNTERS, Ip6statCounters};

use crate::netinet::ip_var::IPMTUDISCTIMEOUT;

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

/// `ip6counters`: the IPv6 statistics, per CPU (`ip6_init`'s `counters_alloc`).
pub static IP6COUNTERS: Counters<IP6S_NCOUNTERS> = Counters::new();

/// `icmp6counters`: the ICMPv6 statistics, per CPU (`icmp6_init`'s `counters_alloc`).
pub static ICMP6COUNTERS: Counters<ICP6S_NCOUNTERS> = Counters::new();

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
