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
 *	@(#)icmp_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<netinet/icmp_var.h>`'s userland part: `struct icmpstat` (`net.inet.icmp.stats`) and the
//! sysctl names.
//!
//! Upstream: sys/netinet/icmp_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/icmp_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `struct icmpstat`'s `u_long`s are `u64` (LP64), the words `net.inet.icmp.stats` copies;
//!   `ICMPCTL_NAMES` is userland's.
//!
//! ## Redesign
//! - LZ's image in `bsd`'s `sys/netinet/icmp_var.rs` -> `hal`, an `abi_pod!` (N6.4c2); the
//!   `_KERNEL` part (the counters' enum and `icmpstat_inc`) is `kernel::netinet::icmp_var`'s.

use core::mem::size_of;

use crate::netinet_ip_icmp::ICMP_MAXTYPE;

/// The histograms' length: one counter per ICMP type, `ICMP_MAXTYPE + 1`.
pub const ICMP_NHIST: usize = ICMP_MAXTYPE as usize + 1;

// Names for ICMP sysctl objects

/// `ICMPCTL_MASKREPL`: allow replies to netmask requests.
pub const ICMPCTL_MASKREPL: i32 = 1;
/// `ICMPCTL_BMCASTECHO`: reply to icmps to broadcast/mcast.
pub const ICMPCTL_BMCASTECHO: i32 = 2;
/// `ICMPCTL_ERRPPSLIMIT`: ICMP error pps limitation.
pub const ICMPCTL_ERRPPSLIMIT: i32 = 3;
/// `ICMPCTL_REDIRACCEPT`: Accept redirects from routers.
pub const ICMPCTL_REDIRACCEPT: i32 = 4;
/// `ICMPCTL_REDIRTIMEOUT`: Remove routes added via redirects.
pub const ICMPCTL_REDIRTIMEOUT: i32 = 5;
/// `ICMPCTL_TSTAMPREPL`: allow replies to timestamp requests.
pub const ICMPCTL_TSTAMPREPL: i32 = 6;
/// `ICMPCTL_STATS`: ICMP statistics.
pub const ICMPCTL_STATS: i32 = 7;
/// `ICMPCTL_MAXID`.
pub const ICMPCTL_MAXID: i32 = 8;

/// `struct icmpstat`: variables related to this implementation of the internet control
/// message protocol (`net.inet.icmp.stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icmpstat {
    // statistics related to icmp packets generated
    /// `icps_error`: # of calls to icmp_error.
    pub icps_error: u64,
    /// `icps_toofreq`: no error because rate limiter.
    pub icps_toofreq: u64,
    /// `icps_oldshort`: no error because old ip too short.
    pub icps_oldshort: u64,
    /// `icps_oldicmp`: no error because old was icmp.
    pub icps_oldicmp: u64,
    /// `icps_outhist`: the messages sent, by type.
    pub icps_outhist: [u64; ICMP_NHIST],
    // statistics related to input messages processed
    /// `icps_badcode`: icmp_code out of range.
    pub icps_badcode: u64,
    /// `icps_tooshort`: packet < ICMP_MINLEN.
    pub icps_tooshort: u64,
    /// `icps_checksum`: bad checksum.
    pub icps_checksum: u64,
    /// `icps_badlen`: calculated bound mismatch.
    pub icps_badlen: u64,
    /// `icps_reflect`: number of responses.
    pub icps_reflect: u64,
    /// `icps_bmcastecho`: rejected broadcast icmps.
    pub icps_bmcastecho: u64,
    /// `icps_inhist`: the messages received, by type.
    pub icps_inhist: [u64; ICMP_NHIST],
}

crate::abi_pod!(Icmpstat {
    icps_error: u64,
    icps_toofreq: u64,
    icps_oldshort: u64,
    icps_oldicmp: u64,
    icps_outhist: [u64; ICMP_NHIST],
    icps_badcode: u64,
    icps_tooshort: u64,
    icps_checksum: u64,
    icps_badlen: u64,
    icps_reflect: u64,
    icps_bmcastecho: u64,
    icps_inhist: [u64; ICMP_NHIST],
});

const _: () = assert!(size_of::<Icmpstat>() == (10 + 2 * ICMP_NHIST) * size_of::<u64>());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/icmp_var.h");
        let ctl = crate::reftest::assert_defines!(defs;
            ICMPCTL_MASKREPL, ICMPCTL_BMCASTECHO, ICMPCTL_ERRPPSLIMIT, ICMPCTL_REDIRACCEPT,
            ICMPCTL_REDIRTIMEOUT, ICMPCTL_TSTAMPREPL, ICMPCTL_STATS, ICMPCTL_MAXID);
        crate::reftest::assert_complete(
            &defs,
            "ICMPCTL_",
            &[&ctl[..], &["ICMPCTL_NAMES"]].concat(),
        );
    }
}
/* </TESTS> */
