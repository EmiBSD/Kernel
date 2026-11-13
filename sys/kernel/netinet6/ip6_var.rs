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
//!   re-exports them until the protocol table moves (N6.5d). `ip6_mtudisc_timeout` joins
//!   them.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

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
