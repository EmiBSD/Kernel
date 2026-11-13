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
//! Variables related to this implementation of the internet control message protocol:
//! `<netinet/icmp_var.h>`.
//!
//! Upstream: sys/netinet/icmp_var.h @ 3ce1f3f79392
//!
//! Status: `ported` (M7b).
//!
//! ## Deviations
//! - `enum icmpstat_counters` is [`IcmpstatCounters`] with the C's values; the C indexes the
//!   two histograms with arithmetic (`icps_outhist + type`), which [`icmpstat_inc_hist`] does.
//! - `icmpcounters` (`struct cpumem *`, `<sys/percpu.h>` not ported) is the static array of
//!   atomics `ICMPCOUNTERS` in `netinet/ip_icmp.rs`, which defines the C's pointer.
//! - `ICMPCTL_NAMES` (a `struct ctlname` table) comes with `<sys/sysctl.h>`.
//! - The userland half (`struct icmpstat`, the `ICMPCTL_*` names) is
//!   `hal::sys::netinet_icmp_var`'s since N6.4c2, re-exported here.

use core::mem::size_of;
use core::sync::atomic::Ordering;

use crate::netinet::ip_icmp::{ICMP_MAXTYPE, ICMPCOUNTERS};

/// The number of entries of each histogram.
const NHIST: usize = ICMP_MAXTYPE as usize + 1;

pub use hal::sys::netinet_icmp_var::{
    ICMPCTL_BMCASTECHO, ICMPCTL_ERRPPSLIMIT, ICMPCTL_MASKREPL, ICMPCTL_MAXID, ICMPCTL_REDIRACCEPT,
    ICMPCTL_REDIRTIMEOUT, ICMPCTL_STATS, ICMPCTL_TSTAMPREPL, Icmpstat,
};

/// `enum icmpstat_counters`: the per-CPU ICMP counters, one per word of [`Icmpstat`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum IcmpstatCounters {
    /// `icps_error`.
    IcpsError,
    /// `icps_toofreq`.
    IcpsToofreq,
    /// `icps_oldshort`.
    IcpsOldshort,
    /// `icps_oldicmp`.
    IcpsOldicmp,
    /// `icps_outhist`: the first of `ICMP_MAXTYPE + 1`.
    IcpsOuthist,
    /// `icps_badcode`.
    IcpsBadcode = 4 + NHIST,
    /// `icps_tooshort`.
    IcpsTooshort,
    /// `icps_checksum`.
    IcpsChecksum,
    /// `icps_badlen`.
    IcpsBadlen,
    /// `icps_reflect`.
    IcpsReflect,
    /// `icps_bmcastecho`.
    IcpsBmcastecho,
    /// `icps_inhist`: the first of `ICMP_MAXTYPE + 1`.
    IcpsInhist,
    /// `icps_ncounters`.
    IcpsNcounters = 4 + NHIST + 6 + NHIST,
}

/// `icps_ncounters`.
pub const ICPS_NCOUNTERS: usize = IcmpstatCounters::IcpsNcounters as usize;

/// `icmpstat_inc(c)`.
pub fn icmpstat_inc(c: IcmpstatCounters) {
    ICMPCOUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

/// `icmpstat_inc(icps_outhist + type)` / `icmpstat_inc(icps_inhist + type)`: bumps entry
/// `type_` of histogram `hist`.
pub fn icmpstat_inc_hist(hist: IcmpstatCounters, type_: u8) {
    ICMPCOUNTERS[hist as usize + usize::from(type_)].fetch_add(1, Ordering::Relaxed);
}

// The counters are the structure's words.
const _: () = assert!(size_of::<Icmpstat>() == ICPS_NCOUNTERS * size_of::<u64>());
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
            w(offset_of!(Icmpstat, icps_outhist)),
            IcmpstatCounters::IcpsOuthist as usize
        );
        assert_eq!(
            w(offset_of!(Icmpstat, icps_badcode)),
            IcmpstatCounters::IcpsBadcode as usize
        );
        assert_eq!(
            w(offset_of!(Icmpstat, icps_inhist)),
            IcmpstatCounters::IcpsInhist as usize
        );
    }

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
