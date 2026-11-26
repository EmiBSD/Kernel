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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)udp_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The part of `<netinet/udp_var.h>` userland reads: the `net.inet.udp` sysctl names
//! (`UDPCTL_*`), `struct udpstat` (`net.inet.udp.stats`, netstat(1)'s `-s`) and `struct
//! udpiphdr` (the IP overlay and UDP header pflow(4) exports and `udp_output` writes).
//!
//! Upstream: sys/netinet/udp_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/udp_var.rs@3c62ede68802
//!
//! ## Deviations
//! - The `ui_*` shorthands of `struct udpiphdr` are its members' members (`ui_i.ih_pr`,
//!   `ui_u.uh_sport`, ...), as LZ's.
//! - `UDPCTL_NAMES` (sysctl(8)'s table of `struct ctlname`) is not a value: not here.
//!
//! ## Redesign
//! - N6.6b0: the ABI items of LZ's `sys/netinet/udp_var.rs` move into `hal::sys`, where the
//!   kernel's UDP (`kernel::netinet::udp_usrreq`) and `bsd`'s readers name them; the kernel
//!   part (`enum udpstat_counters`, `udpstat_inc`) is `kernel::netinet::udp_var`'s.
//! - [`Udpstat`] and [`Udpiphdr`] are `abi_pod!` images (their tiling checked at compile
//!   time, so a sysctl copies their bytes out and `udp_output` writes the header with
//!   `hal::mem::pod::put`); sizes and offsets pinned against the C (clang's static asserts,
//!   `aarch64-unknown-openbsd`): 104 and 28 bytes, no hole.

use core::mem::size_of;

use crate::sys::netinet_ip_var::Ipovly;
use crate::sys::netinet_udp::Udphdr;

// Names for UDP sysctl objects.

/// Checksum UDP packets.
pub const UDPCTL_CHECKSUM: i32 = 1;
/// Return bad dynamic port bitmap.
pub const UDPCTL_BADDYNAMIC: i32 = 2;
/// Receive buffer space.
pub const UDPCTL_RECVSPACE: i32 = 3;
/// Send buffer space.
pub const UDPCTL_SENDSPACE: i32 = 4;
/// UDP statistics.
pub const UDPCTL_STATS: i32 = 5;
/// Root only port bitmap.
pub const UDPCTL_ROOTONLY: i32 = 6;
/// `UDPCTL_MAXID`.
pub const UDPCTL_MAXID: i32 = 7;

/// `struct udpiphdr`: the IP overlay and the UDP header, as `udp_output` prepends them.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Udpiphdr {
    /// `ui_i`: overlaid ip structure.
    pub ui_i: Ipovly,
    /// `ui_u`: udp header.
    pub ui_u: Udphdr,
}

crate::abi_pod!(Udpiphdr {
    ui_i: Ipovly,
    ui_u: Udphdr,
});

/// `struct udpstat`: UDP statistics, as `sysctl(2)` returns them (`u_long` members).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Udpstat {
    // input statistics:
    /// `udps_ipackets`: total input packets.
    pub udps_ipackets: u64,
    /// `udps_hdrops`: packet shorter than header.
    pub udps_hdrops: u64,
    /// `udps_badsum`: checksum error.
    pub udps_badsum: u64,
    /// `udps_nosum`: no checksum.
    pub udps_nosum: u64,
    /// `udps_badlen`: data length larger than packet.
    pub udps_badlen: u64,
    /// `udps_noport`: no socket on port.
    pub udps_noport: u64,
    /// `udps_noportbcast`: of above, arrived as broadcast.
    pub udps_noportbcast: u64,
    /// `udps_nosec`: dropped for lack of ipsec.
    pub udps_nosec: u64,
    /// `udps_fullsock`: not delivered, input socket full.
    pub udps_fullsock: u64,
    /// `udps_pcbhashmiss`: input packets missing pcb hash.
    pub udps_pcbhashmiss: u64,
    /// `udps_inswcsum`: input software-csummed packets.
    pub udps_inswcsum: u64,
    // output statistics:
    /// `udps_opackets`: total output packets.
    pub udps_opackets: u64,
    /// `udps_outswcsum`: output software-csummed packets.
    pub udps_outswcsum: u64,
}

crate::abi_pod!(Udpstat {
    udps_ipackets: u64,
    udps_hdrops: u64,
    udps_badsum: u64,
    udps_nosum: u64,
    udps_badlen: u64,
    udps_noport: u64,
    udps_noportbcast: u64,
    udps_nosec: u64,
    udps_fullsock: u64,
    udps_pcbhashmiss: u64,
    udps_inswcsum: u64,
    udps_opackets: u64,
    udps_outswcsum: u64,
});

// The sizes and offsets the C compiler gives these structures for `aarch64-unknown-openbsd`
// (both LP64 and little-endian, as amd64; the network-order members are wire bytes).
const _: () = {
    assert!(size_of::<Udpiphdr>() == 28);
    assert!(core::mem::offset_of!(Udpiphdr, ui_u) == 20);
    assert!(core::mem::offset_of!(Udpiphdr, ui_i) + core::mem::offset_of!(Ipovly, ih_len) == 10);
    assert!(size_of::<Udpstat>() == 104);
    assert!(core::mem::offset_of!(Udpstat, udps_opackets) == 88);
    assert!(align_of::<Udpstat>() == 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::reftest::{assert_complete, assert_defines};

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/udp_var.h");
        let ctl = assert_defines!(defs;
            UDPCTL_CHECKSUM, UDPCTL_BADDYNAMIC, UDPCTL_RECVSPACE, UDPCTL_SENDSPACE,
            UDPCTL_STATS, UDPCTL_ROOTONLY, UDPCTL_MAXID);
        // `UDPCTL_NAMES` is sysctl(8)'s table, not a value.
        assert_complete(&defs, "UDPCTL_", &[&ctl[..], &["UDPCTL_NAMES"]].concat());
    }
}
/* </TESTS> */
