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
 *	@(#)un.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! Definitions for UNIX IPC domain: `<sys/un.h>`.
//!
//! Upstream: sys/sys/un.h @ 3ce1f3f79392
//! LZ: sys/sys/un.rs@3c62ede68802
//!
//! ## Deviations
//! - `SUN_LEN(su)` is userland only (`#ifndef _KERNEL`) and not ported.
//!
//! ## Redesign
//! - N6.2a: moved from `bsd`'s `sys/sys/un.rs` with the UNIX domain's redesign
//!   (`kernel::net::unix`); `struct sockaddr_un` is [`AbiPod`](crate::user::AbiPod) through
//!   [`crate::abi_pod!`], so the kernel reads it in an address mbuf's bytes with no `unsafe`.

use crate::sys::types::SaFamily;

/// The size of `sun_path`.
pub const SUN_PATH_LEN: usize = 104;

/// `struct sockaddr_un`: definitions for UNIX IPC domain.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SockaddrUn {
    /// `sun_len`: sockaddr len excluding NUL.
    pub sun_len: u8,
    /// `sun_family`: `AF_UNIX`.
    pub sun_family: SaFamily,
    /// `sun_path`: path name (gag).
    pub sun_path: [u8; SUN_PATH_LEN],
}

crate::abi_pod!(SockaddrUn {
    sun_len: u8,
    sun_family: SaFamily,
    sun_path: [u8; SUN_PATH_LEN],
});

impl SockaddrUn {
    /// `offsetof(struct sockaddr_un, sun_path)`.
    pub const PATH_OFFSET: usize = core::mem::offset_of!(SockaddrUn, sun_path);
}

const _: () = {
    assert!(size_of::<SockaddrUn>() == 106);
    assert!(SockaddrUn::PATH_OFFSET == 2);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::{bytes_of, pod_from_bytes};

    #[test]
    fn a_sockaddr_un_round_trips_through_its_bytes() {
        let mut su = SockaddrUn {
            sun_len: 9,
            sun_family: 1,
            sun_path: [0; SUN_PATH_LEN],
        };
        su.sun_path[..6].copy_from_slice(b"/tmp/s");
        let b = bytes_of(core::slice::from_ref(&su));
        assert_eq!(&b[..8], &[9, 1, b'/', b't', b'm', b'p', b'/', b's']);
        assert_eq!(pod_from_bytes::<SockaddrUn>(b), Some(su));
        assert_eq!(pod_from_bytes::<SockaddrUn>(&b[..105]), None);
    }
}
/* </TESTS> */
