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
 *	@(#)igmp.h	8.1 (Berkeley) 6/10/93
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
 *	@(#)igmp.c	8.2 (Berkeley) 5/3/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<netinet/igmp.h>`: Internet Group Management Protocol (IGMP) definitions, the packet
//! `struct igmp`, its message types and the membership and router-version states. Written by
//! Steve Deering, Stanford, May 1988. MULTICAST 1.2.
//!
//! Upstream: sys/netinet/igmp.h @ 3ce1f3f79392
//! Upstream: sys/netinet/igmp.c @ 3ce1f3f79392
//! LZ: sys/netinet/igmp.rs@3c62ede68802
//!
//! ## Redesign
//! - LZ's constants and image in `bsd`'s `sys/netinet/igmp.rs` (beside igmp.c) -> `hal`, the
//!   image an `abi_pod!`; the protocol (igmp.c, and the header's `_KERNEL` prototype
//!   `rti_delete`) is `kernel::netinet::igmp`'s.

use core::mem::size_of;

use crate::sys::netinet_in::InAddr;

/// `IGMP_MINLEN`.
pub const IGMP_MINLEN: usize = 8;

/// `IGMP_HOST_MEMBERSHIP_QUERY`: membership query.
pub const IGMP_HOST_MEMBERSHIP_QUERY: u8 = 0x11;
/// `IGMP_v1_HOST_MEMBERSHIP_REPORT`: v1 membership report.
#[allow(non_upper_case_globals)] // the C name
pub const IGMP_v1_HOST_MEMBERSHIP_REPORT: u8 = 0x12;
/// `IGMP_DVMRP`: DVMRP routing message.
pub const IGMP_DVMRP: u8 = 0x13;
/// `IGMP_PIM`: PIM routing message.
pub const IGMP_PIM: u8 = 0x14;
/// `IGMP_v2_HOST_MEMBERSHIP_REPORT`: v2 membership report.
#[allow(non_upper_case_globals)] // the C name
pub const IGMP_v2_HOST_MEMBERSHIP_REPORT: u8 = 0x16;
/// `IGMP_HOST_LEAVE_MESSAGE`: leave-group message.
pub const IGMP_HOST_LEAVE_MESSAGE: u8 = 0x17;
/// `IGMP_MTRACE_REPLY`: traceroute reply.
pub const IGMP_MTRACE_REPLY: u8 = 0x1e;
/// `IGMP_MTRACE_QUERY`: traceroute query.
pub const IGMP_MTRACE_QUERY: u8 = 0x1f;

/// `IGMP_MAX_HOST_REPORT_DELAY`: max delay for response to query (in seconds).
pub const IGMP_MAX_HOST_REPORT_DELAY: u32 = 10;

/// `IGMP_TIMER_SCALE`: denominator for `igmp_timer`.
pub const IGMP_TIMER_SCALE: u32 = 10;

/// `IGMP_DELAYING_MEMBER`: a state of the IGMP v2 state table (`inm_state`).
pub const IGMP_DELAYING_MEMBER: u32 = 1;
/// `IGMP_IDLE_MEMBER`.
pub const IGMP_IDLE_MEMBER: u32 = 2;
/// `IGMP_LAZY_MEMBER`.
pub const IGMP_LAZY_MEMBER: u32 = 3;
/// `IGMP_SLEEPING_MEMBER`.
pub const IGMP_SLEEPING_MEMBER: u32 = 4;
/// `IGMP_AWAKENING_MEMBER`.
pub const IGMP_AWAKENING_MEMBER: u32 = 5;

/// `IGMP_v1_ROUTER`: a state of the IGMP router version cache (`rti_type`).
#[allow(non_upper_case_globals)] // the C name
pub const IGMP_v1_ROUTER: i32 = 1;
/// `IGMP_v2_ROUTER`.
#[allow(non_upper_case_globals)] // the C name
pub const IGMP_v2_ROUTER: i32 = 2;

/// `IGMP_AGE_THRESHOLD`: revert to v2 if we haven't heard from the router in this amount of
/// time.
pub const IGMP_AGE_THRESHOLD: i32 = 540;

/// `struct igmp`: IGMP packet format.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Igmp {
    /// `igmp_type`: version & type of IGMP message.
    pub igmp_type: u8,
    /// `igmp_code`: code for routing sub-messages.
    pub igmp_code: u8,
    /// `igmp_cksum`: IP-style checksum.
    pub igmp_cksum: u16,
    /// `igmp_group`: group address being reported (zero for queries).
    pub igmp_group: InAddr,
}

crate::abi_pod!(Igmp {
    igmp_type: u8,
    igmp_code: u8,
    igmp_cksum: u16,
    igmp_group: InAddr,
});

const _: () = assert!(size_of::<Igmp>() == IGMP_MINLEN);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::reftest::{assert_complete, assert_defines};

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/igmp.h");
        let igmp = assert_defines!(defs;
        IGMP_MINLEN, IGMP_HOST_MEMBERSHIP_QUERY, IGMP_v1_HOST_MEMBERSHIP_REPORT, IGMP_DVMRP,
        IGMP_PIM, IGMP_v2_HOST_MEMBERSHIP_REPORT, IGMP_HOST_LEAVE_MESSAGE, IGMP_MTRACE_REPLY,
        IGMP_MTRACE_QUERY, IGMP_MAX_HOST_REPORT_DELAY, IGMP_TIMER_SCALE, IGMP_DELAYING_MEMBER,
        IGMP_IDLE_MEMBER, IGMP_LAZY_MEMBER, IGMP_SLEEPING_MEMBER, IGMP_AWAKENING_MEMBER,
        IGMP_v1_ROUTER, IGMP_v2_ROUTER, IGMP_AGE_THRESHOLD);
        assert_complete(&defs, "IGMP_", &igmp);
    }
}
/* </TESTS> */
