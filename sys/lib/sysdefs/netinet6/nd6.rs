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
/* </LICENSES> */

/* <CODE> */
//! Neighbor Discovery, `<netinet6/nd6.h>`'s userland part: the reachability states, the
//! interface's ND parameters and the requests of `SIOCGIFINFO_IN6` and `SIOCGNBRINFO_IN6`
//! (read by ndp(8)). The `_KERNEL` part is `bsd`'s `sys/netinet6/nd6.rs` until N6.5b.
//!
//! Upstream: sys/netinet6/nd6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/nd6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/nd6.rs@3c62ede68802
//!
//! ## Redesign
//! - N6.5a: LZ's images in `bsd`'s `sys/netinet6/nd6.rs` -> here, each an `abi_pod!` image
//!   with its `const` size; `bsd` keeps its copies until ND moves (N6.5b).

use core::mem::size_of;

use crate::base::types::Time;
use crate::net::if_::IFNAMSIZ;
use crate::netinet6::in6::In6Addr;

/// `ND6_LLINFO_PURGE`.
pub const ND6_LLINFO_PURGE: i16 = -3;
/// `ND6_LLINFO_NOSTATE`.
pub const ND6_LLINFO_NOSTATE: i16 = -2;
/// `ND6_LLINFO_INCOMPLETE`.
pub const ND6_LLINFO_INCOMPLETE: i16 = 0;
/// `ND6_LLINFO_REACHABLE`.
pub const ND6_LLINFO_REACHABLE: i16 = 1;
/// `ND6_LLINFO_STALE`.
pub const ND6_LLINFO_STALE: i16 = 2;
/// `ND6_LLINFO_DELAY`.
pub const ND6_LLINFO_DELAY: i16 = 3;
/// `ND6_LLINFO_PROBE`.
pub const ND6_LLINFO_PROBE: i16 = 4;

/// `struct nd_ifinfo`: the Neighbor Discovery state of an interface.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NdIfinfo {
    /// \[N\] Reachable Time.
    pub reachable: u32,
    /// \[N\] BaseReachable recalc timer.
    pub recalctm: i32,
}

crate::abi_pod!(NdIfinfo {
    reachable: u32,
    recalctm: i32,
});

/// `struct in6_nbrinfo`: the argument of `SIOCGNBRINFO_IN6`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct In6Nbrinfo {
    /// If name, e.g. "en0".
    pub ifname: [u8; IFNAMSIZ],
    /// IPv6 address of the neighbor.
    pub addr: In6Addr,
    /// Lifetime for NDP state transition.
    pub expire: Time,
    /// Number of queries already sent for addr.
    pub asked: i64,
    /// If it acts as a router.
    pub isrouter: i32,
    /// Reachability state.
    pub state: i32,
}

crate::abi_pod!(In6Nbrinfo {
    ifname: [u8; IFNAMSIZ],
    addr: In6Addr,
    expire: Time,
    asked: i64,
    isrouter: i32,
    state: i32,
});

/// `struct in6_ndireq`: the argument of `SIOCGIFINFO_IN6`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct In6Ndireq {
    /// `ifname`.
    pub ifname: [u8; IFNAMSIZ],
    /// `ndi`.
    pub ndi: NdIfinfo,
}

crate::abi_pod!(In6Ndireq {
    ifname: [u8; IFNAMSIZ],
    ndi: NdIfinfo,
});

/// `ND6_INFINITE_LIFETIME`.
pub const ND6_INFINITE_LIFETIME: u32 = 0xffff_ffff;

// LP64 sizes of the C structures.
const _: () = {
    assert!(size_of::<NdIfinfo>() == 8);
    assert!(size_of::<In6Nbrinfo>() == 56);
    assert!(size_of::<In6Ndireq>() == 24);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::mem::offset_of;

    use super::*;

    #[test]
    fn layouts_are_the_cs() {
        assert_eq!(offset_of!(In6Nbrinfo, addr), 16);
        assert_eq!(offset_of!(In6Nbrinfo, expire), 32);
        assert_eq!(offset_of!(In6Nbrinfo, isrouter), 48);
        assert_eq!(offset_of!(In6Ndireq, ndi), 16);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::base::reftest::defines("sys/netinet6/nd6.h");
        crate::base::reftest::assert_defines!(defs;
            ND6_LLINFO_PURGE, ND6_LLINFO_NOSTATE, ND6_LLINFO_INCOMPLETE, ND6_LLINFO_REACHABLE,
            ND6_LLINFO_STALE, ND6_LLINFO_DELAY, ND6_LLINFO_PROBE, ND6_INFINITE_LIFETIME);
    }
}
/* </TESTS> */
