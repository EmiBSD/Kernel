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

/*-
 * Copyright (c) 2009 Bruce Simpson.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. The name of the author may not be used to endorse or promote
 *    products derived from this software without specific prior written
 *    permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 */

/*
 * Copyright (C) 1998 WIDE Project.
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
 *	@(#)igmp.c	8.1 (Berkeley) 7/19/93
 */
/* </LICENSES> */

/* <CODE> */
//! Multicast Listener Discovery: the MLDv2 message formats of `<netinet6/mld6.h>`. See
//! `<netinet/icmp6.h>` (`sysdefs::netinet::icmp6`) for `struct mld_hdr` (MLDv1 query and
//! host report format). MLD itself (`mld6.c`) is `kernel::netinet6::mld6`'s.
//!
//! Upstream: sys/netinet6/mld6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/mld6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/mld6.rs@3c62ede68802
//!
//! ## Deviations
//! - The `MLD_MRC_*`, `MLD_QQIC_*`, `MLD_QRESV`, `MLD_SFLAG` and `MLD_QRV` macros are
//!   `const fn`s; `mld_numrecs` is [`Mldv2Report::mld_numrecs`]. The structures are
//!   `#[repr(C)]` without padding (the C's are `__packed`; sizes asserted below).
//!
//! ## Redesign
//! - N6.5a: the header half of LZ's `bsd` `sys/netinet6/mld6.rs` (which `mld6.c` shared)
//!   moved here, each structure an `abi_pod!` image.

use core::mem::size_of;

use crate::base::endian::ntohs;
use crate::netinet::icmp6::Icmp6Hdr;
use crate::netinet6::in6::In6Addr;

/// Minimum length of any MLD protocol message.
pub const MLD_MINLEN: usize = size_of::<Icmp6Hdr>();

/// `MLD_V2_REPORT_MAXRECS`.
pub const MLD_V2_REPORT_MAXRECS: u32 = 65535;

// MLDv2 report modes.

/// Don't send a record.
pub const MLD_DO_NOTHING: u8 = 0;
/// MODE_IN.
pub const MLD_MODE_IS_INCLUDE: u8 = 1;
/// MODE_EX.
pub const MLD_MODE_IS_EXCLUDE: u8 = 2;
/// TO_IN.
pub const MLD_CHANGE_TO_INCLUDE_MODE: u8 = 3;
/// TO_EX.
pub const MLD_CHANGE_TO_EXCLUDE_MODE: u8 = 4;
/// ALLOW_NEW.
pub const MLD_ALLOW_NEW_SOURCES: u8 = 5;
/// BLOCK_OLD.
pub const MLD_BLOCK_OLD_SOURCES: u8 = 6;

// MLDv2 query types.

/// `MLD_V2_GENERAL_QUERY`.
pub const MLD_V2_GENERAL_QUERY: i32 = 1;
/// `MLD_V2_GROUP_QUERY`.
pub const MLD_V2_GROUP_QUERY: i32 = 2;
/// `MLD_V2_GROUP_SOURCE_QUERY`.
pub const MLD_V2_GROUP_SOURCE_QUERY: i32 = 3;

/// Maximum report interval for MLDv1 host membership reports (in seconds).
pub const MLD_V1_MAX_RI: u32 = 10;

/// `MLD_TIMER_SCALE`: the MLD code field specifies time in milliseconds.
pub const MLD_TIMER_SCALE: u32 = 1000;

/// `struct mldv2_query`: MLD v2 query format (followed by 1..numsrc source addresses).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mldv2Query {
    /// ICMPv6 header.
    pub mld_icmp6_hdr: Icmp6Hdr,
    /// Address being queried.
    pub mld_addr: In6Addr,
    /// Reserved/suppress/robustness.
    pub mld_misc: u8,
    /// Querier's query interval.
    pub mld_qqi: u8,
    /// Number of sources.
    pub mld_numsrc: u16,
}

crate::abi_pod!(Mldv2Query {
    mld_icmp6_hdr: Icmp6Hdr,
    mld_addr: In6Addr,
    mld_misc: u8,
    mld_qqi: u8,
    mld_numsrc: u16,
});

/// `MLD_V2_QUERY_MINLEN`.
pub const MLD_V2_QUERY_MINLEN: usize = size_of::<Mldv2Query>();

/// `struct mldv2_report`: MLDv2 host membership report header (`mld_type`:
/// `MLDV2_LISTENER_REPORT`), followed by 1..numgrps records.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mldv2Report {
    /// `mld_icmp6_hdr`.
    pub mld_icmp6_hdr: Icmp6Hdr,
}

crate::abi_pod!(Mldv2Report {
    mld_icmp6_hdr: Icmp6Hdr,
});

impl Mldv2Report {
    /// `mld_numrecs` (`mld_icmp6_hdr.icmp6_data16[1]`), overlaid on the ICMPv6 header.
    pub fn mld_numrecs(&self) -> u16 {
        self.mld_icmp6_hdr.icmp6_data16(1)
    }

    /// `mld_numrecs = v`.
    pub fn set_mld_numrecs(&mut self, v: u16) {
        self.mld_icmp6_hdr.set_icmp6_data16(1, v);
    }
}

/// `struct mldv2_record`: a report record (followed by 1..numsrc source addresses).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mldv2Record {
    /// Record type.
    pub mr_type: u8,
    /// Length of auxiliary data.
    pub mr_datalen: u8,
    /// Number of sources.
    pub mr_numsrc: u16,
    /// Address being reported.
    pub mr_addr: In6Addr,
}

crate::abi_pod!(Mldv2Record {
    mr_type: u8,
    mr_datalen: u8,
    mr_numsrc: u16,
    mr_addr: In6Addr,
});

/// `MLD_MRC_EXP(x)`: the exponent of a network-order Maximum Response Code.
#[inline]
pub const fn mld_mrc_exp(x: u16) -> u16 {
    (ntohs(x) >> 12) & 0x0007
}

/// `MLD_MRC_MANT(x)`: the mantissa of a network-order Maximum Response Code.
#[inline]
pub const fn mld_mrc_mant(x: u16) -> u16 {
    ntohs(x) & 0x0fff
}

/// `MLD_QQIC_EXP(x)`.
#[inline]
pub const fn mld_qqic_exp(x: u8) -> u8 {
    (x >> 4) & 0x07
}

/// `MLD_QQIC_MANT(x)`.
#[inline]
pub const fn mld_qqic_mant(x: u8) -> u8 {
    x & 0x0f
}

/// `MLD_QRESV(x)`.
#[inline]
pub const fn mld_qresv(x: u8) -> u8 {
    (x >> 4) & 0x0f
}

/// `MLD_SFLAG(x)`.
#[inline]
pub const fn mld_sflag(x: u8) -> u8 {
    (x >> 3) & 0x01
}

/// `MLD_QRV(x)`.
#[inline]
pub const fn mld_qrv(x: u8) -> u8 {
    x & 0x07
}

// The sizes of the C's `__packed` structures.
const _: () = {
    assert!(size_of::<Mldv2Query>() == 28);
    assert!(size_of::<Mldv2Report>() == 8);
    assert!(size_of::<Mldv2Record>() == 20);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::mem::offset_of;

    use super::*;
    use crate::base::endian::htons;

    #[test]
    fn layouts_and_the_query_macros() {
        assert_eq!(offset_of!(Mldv2Query, mld_addr), 8);
        assert_eq!(offset_of!(Mldv2Query, mld_misc), 24);
        assert_eq!(offset_of!(Mldv2Query, mld_numsrc), 26);
        assert_eq!(offset_of!(Mldv2Record, mr_addr), 4);
        assert_eq!(MLD_MINLEN, 8);
        assert_eq!(MLD_V2_QUERY_MINLEN, 28);
        let mut r = Mldv2Report::default();
        r.set_mld_numrecs(htons(3));
        assert_eq!(r.mld_icmp6_hdr.icmp6_data16(1), htons(3));
        assert_eq!(r.mld_numrecs(), htons(3));
        // Maximum Response Code 0x8123: exponent 0, mantissa 0x123 (network order in).
        assert_eq!(mld_mrc_exp(htons(0x8123)), 0);
        assert_eq!(mld_mrc_mant(htons(0x8123)), 0x123);
        assert_eq!(mld_mrc_exp(htons(0x7000)), 7);
        assert_eq!((mld_qqic_exp(0xb5), mld_qqic_mant(0xb5)), (3, 5));
        assert_eq!(
            (mld_qresv(0xfa), mld_sflag(0xfa), mld_qrv(0xfa)),
            (0xf, 1, 2)
        );
    }
}
/* </TESTS> */
