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
 * Copyright (C) 2001 WIDE Project.
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
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)raw_ip.c	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! The part of `<netinet6/raw_ip6.h>` userland reads: `struct rip6stat` (the node
//! `{CTL_NET, PF_INET6, IPPROTO_RAW, RIPV6CTL_STATS}`, netstat(1)'s `-s -p rip6`) and its
//! sysctl names (`RIPV6CTL_*`).
//!
//! Upstream: sys/netinet6/raw_ip6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/raw_ip6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/raw_ip6.rs@3c62ede68802
//!
//! ## Redesign
//! - N6.6b1: the ABI items of LZ's `sys/netinet6/raw_ip6.rs` move into `hal::sys`, where the
//!   kernel's raw IPv6 (`kernel::netinet6::raw_ip6`) names them; [`Rip6stat`] is an `abi_pod!`
//!   image, its size pinned by a test and by a compile-time check. `RIPM6CTL_NAMES` (LZ's
//!   `Ctlname` table) is sysctl(8)'s, userland's.

use core::mem::size_of;

/// RIP6 stats.
pub const RIPV6CTL_STATS: i32 = 1;
/// The number of names.
pub const RIPV6CTL_MAXID: i32 = 2;

/// `struct rip6stat`: raw IPv6 statistics, as `sysctl(2)` returns them.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rip6stat {
    /// `rip6s_ipackets`: total input packets.
    pub rip6s_ipackets: u64,
    /// `rip6s_isum`: input checksum computations.
    pub rip6s_isum: u64,
    /// `rip6s_badsum`: of above, checksum error.
    pub rip6s_badsum: u64,
    /// `rip6s_nosock`: no matching socket.
    pub rip6s_nosock: u64,
    /// `rip6s_nosockmcast`: of above, arrived as multicast.
    pub rip6s_nosockmcast: u64,
    /// `rip6s_fullsock`: not delivered, input socket full.
    pub rip6s_fullsock: u64,
    /// `rip6s_opackets`: total output packets.
    pub rip6s_opackets: u64,
}

crate::abi_pod!(Rip6stat {
    rip6s_ipackets: u64,
    rip6s_isum: u64,
    rip6s_badsum: u64,
    rip6s_nosock: u64,
    rip6s_nosockmcast: u64,
    rip6s_fullsock: u64,
    rip6s_opackets: u64,
});

/// `rip6s_ncounters`: the counters of `enum rip6stat_counters`, one per member of
/// [`Rip6stat`] in its order.
pub const RIP6S_NCOUNTERS: usize = 7;

// CTASSERT(sizeof(rip6stat) == rip6s_ncounters * sizeof(uint64_t)).
const _: () = assert!(size_of::<Rip6stat>() == RIP6S_NCOUNTERS * size_of::<u64>());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::bytes_of;

    #[test]
    fn the_statistics_are_the_c_layout() {
        let s = Rip6stat {
            rip6s_ipackets: 1,
            rip6s_opackets: 0x0102_0304_0506_0708,
            ..Rip6stat::default()
        };
        let one = [s];
        let b = bytes_of(&one);
        assert_eq!(b.len(), 56);
        assert_eq!(b[..8], 1u64.to_ne_bytes());
        assert_eq!(b[48..56], 0x0102_0304_0506_0708u64.to_ne_bytes());
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet6/raw_ip6.h");
        let ctl = crate::reftest::assert_defines!(defs; RIPV6CTL_STATS, RIPV6CTL_MAXID);
        crate::reftest::assert_complete(&defs, "RIPV6CTL_", &ctl);
    }
}
/* </TESTS> */
