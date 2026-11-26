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
/* </LICENSES> */

/* <CODE> */
//! Multicast Listener Discovery, implementation-specific definitions: the listening states
//! and the report a membership change asks for: `<netinet6/mld6_var.h>` (all `_KERNEL`).
//!
//! Upstream: sys/netinet6/mld6_var.h @ 3ce1f3f79392
//! LZ: sys/netinet6/mld6_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `MLD_RANDOM_DELAY(X)` is [`mld_random_delay`]. The prototypes are `kernel::netinet6::
//!   mld6`'s.
//!
//! ## Redesign
//! - LZ's module moved into the kernel (N6.5a), over `hal::random`; `bsd` reaches it under
//!   LZ's path through `sys/netinet6/mod.rs`'s re-export.
//! - `struct mld6_pktinfo`'s `mpi_list` is the list node `mld6_fasttimo` makes of each report
//!   (`kernel::netinet6::mld6`, the C's `malloc(M_MRTABLE, M_NOWAIT)` entries), not a member:
//!   the other callers keep the record on their stack (`kernel::netinet::igmp_var`'s shape;
//!   LZ's was a `Vec`).

use hal::random::arc4random_uniform;
use sysdefs::netinet6::in6::In6Addr;

// States for MLD stop-listening processing

/// `MLD_OTHERLISTENER`.
pub const MLD_OTHERLISTENER: u32 = 0;
/// `MLD_IREPORTEDLAST`.
pub const MLD_IREPORTEDLAST: u32 = 1;

/// `struct mld6_pktinfo`: the report or done message a membership change asks
/// `mld6_sendpkt` to send (`mpi_ifidx` 0: none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mld6Pktinfo {
    /// `mpi_addr`: the group.
    pub mpi_addr: In6Addr,
    /// `mpi_rdomain`.
    pub mpi_rdomain: u32,
    /// `mpi_ifidx`: the interface the message leaves on.
    pub mpi_ifidx: u32,
    /// `mpi_type`: the MLD message type.
    pub mpi_type: i32,
}

/// `MLD_RANDOM_DELAY(X)`: a random timer value between 1 and `x`.
#[inline]
pub fn mld_random_delay(x: u32) -> u32 {
    arc4random_uniform(x) + 1
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_random_delay_is_between_one_and_x() {
        // arc4random's generator: the test doubles' (alone, this test would draw before any
        // other test registered them).
        crate::testutil::install();
        for _ in 0..1000 {
            let d = mld_random_delay(5);
            assert!((1..=5).contains(&d));
        }
        assert_eq!(mld_random_delay(1), 1);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet6/mld6_var.h");
        crate::reftest::assert_defines!(defs; MLD_OTHERLISTENER, MLD_IREPORTEDLAST);
    }
}
/* </TESTS> */
