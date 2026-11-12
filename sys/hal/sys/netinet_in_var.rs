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
 * Copyright (c) 1985, 1986, 1993
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
 *	@(#)in_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<netinet/in_var.h>`'s userland part: `struct in_aliasreq`, the request of `SIOCAIFADDR`
//! and `SIOCDIFADDR` (`in_ioctl`).
//!
//! Upstream: sys/netinet/in_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/in_var.rs@3c62ede68802
//!
//! ## Redesign
//! - LZ's `InAliasreq` with its `ifra_ifrau` union (`ifrau_addr`, `ifrau_align`) read through
//!   two `unsafe` accessors -> an `abi_pod!` image whose `ifra_addr` is the union's address
//!   member itself: the union's other member is an `int` that only aligns it, and a
//!   `sockaddr_in` is already of alignment 4 and 16 bytes long, so the layout is the C's to
//!   the byte (the size assert) and no accessor needs `unsafe`. The `_KERNEL` part (`struct
//!   in_ifaddr`, `struct in_multi`) is `kernel::netinet::in_var`'s.

use core::mem::size_of;

use crate::sys::net_if::IFNAMSIZ;
use crate::sys::netinet_in::SockaddrIn;

/// `struct in_aliasreq`: the argument of `SIOCAIFADDR` and `SIOCDIFADDR`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InAliasreq {
    /// `ifra_name`: if name, e.g. "en0".
    pub ifra_name: [u8; IFNAMSIZ],
    /// `ifra_addr` (`ifra_ifrau.ifrau_addr`; the union's `ifrau_align` only aligns it).
    pub ifra_addr: SockaddrIn,
    /// `ifra_dstaddr` (`ifra_broadaddr`).
    pub ifra_dstaddr: SockaddrIn,
    /// `ifra_mask`.
    pub ifra_mask: SockaddrIn,
}

crate::abi_pod!(InAliasreq {
    ifra_name: [u8; IFNAMSIZ],
    ifra_addr: SockaddrIn,
    ifra_dstaddr: SockaddrIn,
    ifra_mask: SockaddrIn,
});

impl InAliasreq {
    /// `ifra_broadaddr` (`ifra_dstaddr`).
    #[inline]
    pub const fn ifra_broadaddr(&self) -> &SockaddrIn {
        &self.ifra_dstaddr
    }
}

// LP64 size of the C structure.
const _: () = assert!(size_of::<InAliasreq>() == 64);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::netinet_in::InAddr;
    use crate::user::bytes_of;

    #[test]
    fn the_address_sits_where_the_union_does() {
        let mut r = InAliasreq::default();
        r.ifra_name[..4].copy_from_slice(b"vio1");
        r.ifra_addr.sin_len = 16;
        r.ifra_addr.sin_family = 2;
        r.ifra_addr.sin_addr = InAddr {
            s_addr: u32::from_ne_bytes([192, 168, 77, 2]),
        };
        r.ifra_mask.sin_addr = InAddr {
            s_addr: u32::from_ne_bytes([255, 255, 255, 0]),
        };
        let rs = [r];
        let b = bytes_of(&rs);
        assert_eq!(&b[..4], b"vio1");
        assert_eq!(b[16..18], [16, 2]);
        assert_eq!(b[20..24], [192, 168, 77, 2]);
        assert_eq!(b[52..56], [255, 255, 255, 0]);
        assert_eq!(core::mem::align_of::<InAliasreq>(), 4);
        assert!(core::ptr::eq(r.ifra_broadaddr(), &r.ifra_dstaddr));
    }
}
/* </TESTS> */
