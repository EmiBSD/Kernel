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
 * Copyright (c) 1988 Stephen Deering.
 * Copyright (c) 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Stephen Deering of Stanford University.
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
 *	@(#)igmp_var.h	8.1 (Berkeley) 7/19/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<netinet/igmp_var.h>`'s userland part: `struct igmpstat` (`net.inet.igmp.stats`) and the
//! sysctl names. Written by Steve Deering, Stanford, May 1988. Modified by Rosen Sharma,
//! Stanford, Aug 1994. Modified by Bill Fenner, Xerox PARC, Feb 1995. MULTICAST 1.3.
//!
//! Upstream: sys/netinet/igmp_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/igmp_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `struct igmpstat`'s `u_long`s are `u64` (LP64), the words `net.inet.igmp.stats` copies;
//!   `IGMPCTL_NAMES` is userland's.
//!
//! ## Redesign
//! - LZ's image in `bsd`'s `sys/netinet/igmp_var.rs` -> `hal`, an `abi_pod!`; the `_KERNEL`
//!   part (the counters, `struct igmp_pktinfo`) is `kernel::netinet::igmp_var`'s.

use core::mem::size_of;

/// `IGMPCTL_STATS`: IGMP statistics.
pub const IGMPCTL_STATS: i32 = 1;
/// `IGMPCTL_MAXID`.
pub const IGMPCTL_MAXID: i32 = 2;

/// `struct igmpstat`: `net.inet.igmp.stats`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Igmpstat {
    /// `igps_rcv_total`: total IGMP messages received.
    pub igps_rcv_total: u64,
    /// `igps_rcv_tooshort`: received with too few bytes.
    pub igps_rcv_tooshort: u64,
    /// `igps_rcv_badsum`: received with bad checksum.
    pub igps_rcv_badsum: u64,
    /// `igps_rcv_queries`: received membership queries.
    pub igps_rcv_queries: u64,
    /// `igps_rcv_badqueries`: received invalid queries.
    pub igps_rcv_badqueries: u64,
    /// `igps_rcv_reports`: received membership reports.
    pub igps_rcv_reports: u64,
    /// `igps_rcv_badreports`: received invalid reports.
    pub igps_rcv_badreports: u64,
    /// `igps_rcv_ourreports`: received reports for our groups.
    pub igps_rcv_ourreports: u64,
    /// `igps_snd_reports`: sent membership reports.
    pub igps_snd_reports: u64,
}

crate::abi_pod!(Igmpstat {
    igps_rcv_total: u64,
    igps_rcv_tooshort: u64,
    igps_rcv_badsum: u64,
    igps_rcv_queries: u64,
    igps_rcv_badqueries: u64,
    igps_rcv_reports: u64,
    igps_rcv_badreports: u64,
    igps_rcv_ourreports: u64,
    igps_snd_reports: u64,
});

const _: () = assert!(size_of::<Igmpstat>() == 9 * size_of::<u64>());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/igmp_var.h");
        crate::reftest::assert_defines!(defs; IGMPCTL_STATS, IGMPCTL_MAXID);
    }
}
/* </TESTS> */
