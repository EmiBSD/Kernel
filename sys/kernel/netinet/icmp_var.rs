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
//! Variables related to this implementation of the internet control message protocol: the
//! kernel's half of `<netinet/icmp_var.h>`, the per-CPU ICMP statistics (`icmpcounters`,
//! defined by `ip_icmp.c`) and `icmpstat_inc`. The userland half (`struct icmpstat`, the
//! `ICMPCTL_*` names) is `sysdefs::netinet_icmp_var`.
//!
//! Upstream: sys/netinet/icmp_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/icmp_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `enum icmpstat_counters` is [`IcmpstatCounters`] with the C's values; the C indexes the
//!   two histograms with arithmetic (`icps_outhist + type`), which [`icmpstat_inc_hist`]
//!   does.
//!
//! ## Redesign
//! - LZ's `ICMPCOUNTERS`, a static array of atomics in `bsd`'s `ip_icmp.rs` -> [`ICMPCOUNTERS`],
//!   a `hal::sync::percpu::Counters` here beside the enum that indexes it (`ipcounters`' shape,
//!   `kernel::netinet::ip_var`): per CPU, as the C's `cpumem`, read whole by
//!   `net.inet.icmp.stats`. `icmpstat_inc` and `icmpstat_inc_hist` are `#[inline]`.

use core::mem::size_of;

use hal::sync::percpu::Counters;
use sysdefs::netinet_icmp_var::{ICMP_NHIST, Icmpstat};

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
    IcpsBadcode = 4 + ICMP_NHIST,
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
    IcpsNcounters = 4 + ICMP_NHIST + 6 + ICMP_NHIST,
}

/// `icps_ncounters`: the number of ICMP counters.
pub const ICPS_NCOUNTERS: usize = IcmpstatCounters::IcpsNcounters as usize;

/// `icmpcounters`: the ICMP statistics, per CPU (`counters_alloc(icps_ncounters)` in
/// `icmp_init`).
pub static ICMPCOUNTERS: Counters<ICPS_NCOUNTERS> = Counters::new();

/// `icmpstat_inc(c)`.
#[inline]
pub fn icmpstat_inc(c: IcmpstatCounters) {
    ICMPCOUNTERS.inc(c as usize);
}

/// `icmpstat_inc(hist + type)`: the histogram `hist`'s counter of ICMP type `type_`.
#[inline]
pub fn icmpstat_inc_hist(hist: IcmpstatCounters, type_: u8) {
    ICMPCOUNTERS.inc(hist as usize + usize::from(type_));
}

// The counters are the structure's words (`CTASSERT` in `icmp_sysctl_icmpstat`).
const _: () = assert!(size_of::<Icmpstat>() == ICPS_NCOUNTERS * size_of::<u64>());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::mem::offset_of;

    use super::*;

    #[test]
    fn counters_match_the_structure() {
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
}
/* </TESTS> */
