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
//! IPv6's statistics, `<netinet6/ip6_var.h>`'s userland part: `struct ip6stat`
//! (`net.inet6.ip6.stats`, `netstat -s -p ip6`) and the indices of its words, `enum
//! ip6stat_counters` (the C's `_KERNEL` part, beside the image it indexes). The rest of the
//! header is the kernel's (`kernel::netinet6::ip6_var`) and `bsd`'s until N6.5c/d.
//!
//! Upstream: sys/netinet6/ip6_var.h @ 3ce1f3f79392
//! LZ: sys/netinet6/ip6_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `enum ip6stat_counters` is [`Ip6statCounters`] with the C's values; the C indexes its
//!   arrays with arithmetic (`ip6s_nxthist + nxt`, `ip6s_m2m + i`, `ip6s_sources_*` +
//!   scope).
//!
//! ## Redesign
//! - N6.5a: LZ's image and enum in `bsd`'s `sys/netinet6/ip6_var.rs` -> here, an `abi_pod!`
//!   image whose words the kernel's per-CPU counters fill (`Counters::read`), the enum shared
//!   by the kernel and `bsd`'s re-export.

use core::mem::size_of;

/// `struct ip6stat`: IPv6 statistics, as `sysctl(2)` returns them.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ip6stat {
    /// Total packets received.
    pub ip6s_total: u64,
    /// Packet too short.
    pub ip6s_tooshort: u64,
    /// Not enough data.
    pub ip6s_toosmall: u64,
    /// Fragments received.
    pub ip6s_fragments: u64,
    /// Frags dropped (dups, out of space).
    pub ip6s_fragdropped: u64,
    /// Fragments timed out.
    pub ip6s_fragtimeout: u64,
    /// Fragments that exceeded limit.
    pub ip6s_fragoverflow: u64,
    /// Packets forwarded.
    pub ip6s_forward: u64,
    /// Packets rcvd for unreachable dest.
    pub ip6s_cantforward: u64,
    /// Packets forwarded on same net.
    pub ip6s_redirectsent: u64,
    /// Datagrams delivered to upper level.
    pub ip6s_delivered: u64,
    /// Total ip packets generated here.
    pub ip6s_localout: u64,
    /// Lost output due to nobufs, etc.
    pub ip6s_odropped: u64,
    /// Total packets reassembled ok.
    pub ip6s_reassembled: u64,
    /// Datagrams successfully fragmented.
    pub ip6s_fragmented: u64,
    /// Output fragments created.
    pub ip6s_ofragments: u64,
    /// Don't fragment flag was set, etc.
    pub ip6s_cantfrag: u64,
    /// Error in option processing.
    pub ip6s_badoptions: u64,
    /// Packets discarded due to no route.
    pub ip6s_noroute: u64,
    /// ip6 version != 6.
    pub ip6s_badvers: u64,
    /// Total raw ip packets generated.
    pub ip6s_rawout: u64,
    /// Scope error.
    pub ip6s_badscope: u64,
    /// Don't join this multicast group.
    pub ip6s_notmember: u64,
    /// Next header history.
    pub ip6s_nxthist: [u64; 256],
    /// One mbuf.
    pub ip6s_m1: u64,
    /// Two or more mbuf.
    pub ip6s_m2m: [u64; 32],
    /// One ext mbuf.
    pub ip6s_mext1: u64,
    /// Two or more ext mbuf.
    pub ip6s_mext2m: u64,
    /// No match gif found.
    pub ip6s_nogif: u64,
    /// Discarded due to too many headers.
    pub ip6s_toomanyhdr: u64,
    // statistics for improvement of the source address selection algorithm
    // (XXX: hardcoded 16 = # of ip6 multicast scope types + 1)
    /// Number of times that address selection fails.
    pub ip6s_sources_none: u64,
    /// Number of times that an address on the outgoing I/F is chosen.
    pub ip6s_sources_sameif: [u64; 16],
    /// Number of times that an address on a non-outgoing I/F is chosen.
    pub ip6s_sources_otherif: [u64; 16],
    /// Number of times that an address that has the same scope from the destination is
    /// chosen.
    pub ip6s_sources_samescope: [u64; 16],
    /// Number of times that an address that has a different scope from the destination is
    /// chosen.
    pub ip6s_sources_otherscope: [u64; 16],
    /// Number of times that a deprecated address is chosen.
    pub ip6s_sources_deprecated: [u64; 16],
    /// Valid route found in cache.
    pub ip6s_rtcachehit: u64,
    /// Route cache with new destination.
    pub ip6s_rtcachemiss: u64,
    /// Packet received on wrong interface.
    pub ip6s_wrongif: u64,
    /// Lost input due to nobufs, etc.
    pub ip6s_idropped: u64,
}

crate::abi_pod!(Ip6stat {
    ip6s_total: u64,
    ip6s_tooshort: u64,
    ip6s_toosmall: u64,
    ip6s_fragments: u64,
    ip6s_fragdropped: u64,
    ip6s_fragtimeout: u64,
    ip6s_fragoverflow: u64,
    ip6s_forward: u64,
    ip6s_cantforward: u64,
    ip6s_redirectsent: u64,
    ip6s_delivered: u64,
    ip6s_localout: u64,
    ip6s_odropped: u64,
    ip6s_reassembled: u64,
    ip6s_fragmented: u64,
    ip6s_ofragments: u64,
    ip6s_cantfrag: u64,
    ip6s_badoptions: u64,
    ip6s_noroute: u64,
    ip6s_badvers: u64,
    ip6s_rawout: u64,
    ip6s_badscope: u64,
    ip6s_notmember: u64,
    ip6s_nxthist: [u64; 256],
    ip6s_m1: u64,
    ip6s_m2m: [u64; 32],
    ip6s_mext1: u64,
    ip6s_mext2m: u64,
    ip6s_nogif: u64,
    ip6s_toomanyhdr: u64,
    ip6s_sources_none: u64,
    ip6s_sources_sameif: [u64; 16],
    ip6s_sources_otherif: [u64; 16],
    ip6s_sources_samescope: [u64; 16],
    ip6s_sources_otherscope: [u64; 16],
    ip6s_sources_deprecated: [u64; 16],
    ip6s_rtcachehit: u64,
    ip6s_rtcachemiss: u64,
    ip6s_wrongif: u64,
    ip6s_idropped: u64,
});

/// The number of entries of the next header history.
const NXTHIST: usize = 256;
/// The number of entries of the mbuf chain length history.
const M2M: usize = 32;
/// The number of entries of each source selection history.
const SOURCES: usize = 16;

/// `enum ip6stat_counters`: the per-CPU IPv6 counters, one per word of [`Ip6stat`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Ip6statCounters {
    /// `ip6s_total`.
    Ip6sTotal,
    /// `ip6s_tooshort`.
    Ip6sTooshort,
    /// `ip6s_toosmall`.
    Ip6sToosmall,
    /// `ip6s_fragments`.
    Ip6sFragments,
    /// `ip6s_fragdropped`.
    Ip6sFragdropped,
    /// `ip6s_fragtimeout`.
    Ip6sFragtimeout,
    /// `ip6s_fragoverflow`.
    Ip6sFragoverflow,
    /// `ip6s_forward`.
    Ip6sForward,
    /// `ip6s_cantforward`.
    Ip6sCantforward,
    /// `ip6s_redirectsent`.
    Ip6sRedirectsent,
    /// `ip6s_delivered`.
    Ip6sDelivered,
    /// `ip6s_localout`.
    Ip6sLocalout,
    /// `ip6s_odropped`.
    Ip6sOdropped,
    /// `ip6s_reassembled`.
    Ip6sReassembled,
    /// `ip6s_fragmented`.
    Ip6sFragmented,
    /// `ip6s_ofragments`.
    Ip6sOfragments,
    /// `ip6s_cantfrag`.
    Ip6sCantfrag,
    /// `ip6s_badoptions`.
    Ip6sBadoptions,
    /// `ip6s_noroute`.
    Ip6sNoroute,
    /// `ip6s_badvers`.
    Ip6sBadvers,
    /// `ip6s_rawout`.
    Ip6sRawout,
    /// `ip6s_badscope`.
    Ip6sBadscope,
    /// `ip6s_notmember`.
    Ip6sNotmember,
    /// `ip6s_nxthist`: the first of 256.
    Ip6sNxthist,
    /// `ip6s_m1`.
    Ip6sM1 = 23 + NXTHIST,
    /// `ip6s_m2m`: the first of 32.
    Ip6sM2m,
    /// `ip6s_mext1`.
    Ip6sMext1 = 23 + NXTHIST + 1 + M2M,
    /// `ip6s_mext2m`.
    Ip6sMext2m,
    /// `ip6s_nogif`.
    Ip6sNogif,
    /// `ip6s_toomanyhdr`.
    Ip6sToomanyhdr,
    /// `ip6s_sources_none`.
    Ip6sSourcesNone,
    /// `ip6s_sources_sameif`: the first of 16.
    Ip6sSourcesSameif,
    /// `ip6s_sources_otherif`: the first of 16.
    Ip6sSourcesOtherif = 23 + NXTHIST + 1 + M2M + 5 + SOURCES,
    /// `ip6s_sources_samescope`: the first of 16.
    Ip6sSourcesSamescope = 23 + NXTHIST + 1 + M2M + 5 + 2 * SOURCES,
    /// `ip6s_sources_otherscope`: the first of 16.
    Ip6sSourcesOtherscope = 23 + NXTHIST + 1 + M2M + 5 + 3 * SOURCES,
    /// `ip6s_sources_deprecated`: the first of 16.
    Ip6sSourcesDeprecated = 23 + NXTHIST + 1 + M2M + 5 + 4 * SOURCES,
    /// `ip6s_rtcachehit`.
    Ip6sRtcachehit = 23 + NXTHIST + 1 + M2M + 5 + 5 * SOURCES,
    /// `ip6s_rtcachemiss`.
    Ip6sRtcachemiss,
    /// `ip6s_wrongif`.
    Ip6sWrongif,
    /// `ip6s_idropped`.
    Ip6sIdropped,
    /// `ip6s_ncounters`.
    Ip6sNcounters,
}

/// `ip6s_ncounters`.
pub const IP6S_NCOUNTERS: usize = Ip6statCounters::Ip6sNcounters as usize;

// The counters are the statistics' words.
const _: () = assert!(size_of::<Ip6stat>() == IP6S_NCOUNTERS * size_of::<u64>());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_match_the_structure() {
        use core::mem::offset_of;
        let w = |off: usize| off / size_of::<u64>();
        assert_eq!(
            w(offset_of!(Ip6stat, ip6s_nxthist)),
            Ip6statCounters::Ip6sNxthist as usize
        );
        assert_eq!(
            w(offset_of!(Ip6stat, ip6s_m1)),
            Ip6statCounters::Ip6sM1 as usize
        );
        assert_eq!(
            w(offset_of!(Ip6stat, ip6s_mext1)),
            Ip6statCounters::Ip6sMext1 as usize
        );
        assert_eq!(
            w(offset_of!(Ip6stat, ip6s_sources_sameif)),
            Ip6statCounters::Ip6sSourcesSameif as usize
        );
        assert_eq!(
            w(offset_of!(Ip6stat, ip6s_sources_deprecated)),
            Ip6statCounters::Ip6sSourcesDeprecated as usize
        );
        assert_eq!(
            w(offset_of!(Ip6stat, ip6s_idropped)),
            Ip6statCounters::Ip6sIdropped as usize
        );
    }
}
/* </TESTS> */
