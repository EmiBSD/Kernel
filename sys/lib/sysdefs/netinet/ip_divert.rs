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
 * Copyright (c) 2009 Michele Marchetto <michele@openbsd.org>
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
/* </LICENSES> */

/* <CODE> */
//! The part of `<netinet/ip_divert.h>` userland reads: the `net.inet.divert` sysctl names
//! (`DIVERTCTL_*`) and `struct divstat` (`net.inet.divert.stats`, netstat(1)'s `-s -p
//! divert`).
//!
//! Upstream: sys/netinet/ip_divert.h @ 3ce1f3f79392
//! Upstream: sys/netinet/ip_divert.c @ 3ce1f3f79392
//! LZ: sys/netinet/ip_divert.rs@3c62ede68802
//!
//! ## Redesign
//! - N6.6b1: the ABI items of LZ's `sys/netinet/ip_divert.rs` move into `hal::sys`, where the
//!   kernel's divert(4) (`kernel::netinet::ip_divert`) names them; [`Divstat`] is an
//!   `abi_pod!` image (five `u_long`s, 64 bits on both machines), its size pinned by a test
//!   and by a compile-time check. `DIVERTCTL_NAMES` (sysctl(8)'s table) is userland's.

use core::mem::size_of;

// Names for divert sysctl objects.

/// Receive buffer space.
pub const DIVERTCTL_RECVSPACE: i32 = 1;
/// Send buffer space.
pub const DIVERTCTL_SENDSPACE: i32 = 2;
/// Divert statistics.
pub const DIVERTCTL_STATS: i32 = 3;
/// The number of names.
pub const DIVERTCTL_MAXID: i32 = 4;

/// `struct divstat`: divert(4) statistics, as `sysctl(2)` returns them.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Divstat {
    /// `divs_ipackets`: total input packets.
    pub divs_ipackets: u64,
    /// `divs_noport`: no socket on port.
    pub divs_noport: u64,
    /// `divs_fullsock`: not delivered, input socket full.
    pub divs_fullsock: u64,
    /// `divs_opackets`: total output packets.
    pub divs_opackets: u64,
    /// `divs_errors`: generic errors.
    pub divs_errors: u64,
}

crate::abi_pod!(Divstat {
    divs_ipackets: u64,
    divs_noport: u64,
    divs_fullsock: u64,
    divs_opackets: u64,
    divs_errors: u64,
});

/// `divs_ncounters`: the counters of `enum divstat_counters`, one per member of [`Divstat`]
/// in its order.
pub const DIVS_NCOUNTERS: usize = 5;

// CTASSERT(sizeof(divstat) == (nitems(counters) * sizeof(u_long))).
const _: () = assert!(size_of::<Divstat>() == DIVS_NCOUNTERS * size_of::<u64>());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::base::reftest::defines("sys/netinet/ip_divert.h");
        let mut ctl = crate::base::reftest::assert_defines!(defs;
            DIVERTCTL_RECVSPACE, DIVERTCTL_SENDSPACE, DIVERTCTL_STATS, DIVERTCTL_MAXID);
        // `DIVERTCTL_NAMES` is sysctl(8)'s table, not a value.
        ctl.push("DIVERTCTL_NAMES");
        crate::base::reftest::assert_complete(&defs, "DIVERTCTL_", &ctl);
    }
}
/* </TESTS> */
