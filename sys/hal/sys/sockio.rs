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
 * Copyright (c) 1982, 1986, 1990, 1993, 1994
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
 *	@(#)sockio.h	8.1 (Berkeley) 3/28/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/sockio.h>`'s socket group: the `ioctl(2)` commands a socket answers itself
//! (`soo_ioctl`), `'s'`.
//!
//! Upstream: sys/sys/sockio.h @ 3ce1f3f79392
//! LZ: sys/sys/sockio.rs@3c62ede68802
//!
//! Each command encodes its direction, group, number and the size of its argument
//! (`ioccom.h`).
//!
//! ## Redesign
//! - N6.2a: the three `'s'` commands moved from `bsd`'s `sys/sys/sockio.rs` with the socket
//!   file (`kernel::net::file`). The interface commands (`'i'`) are sized by `<net/if.h>`'s
//!   structures (`struct ifreq`, `ifaliasreq`, ...), `bsd`'s until the interface layer moves
//!   (N6.3a), and stay in `bsd`'s file until then.

use crate::sys::ioccom::{_ior, _iow};

/// At oob mark?.
pub const SIOCATMARK: u64 = _ior::<i32>(b's', 7);
/// Set process group.
pub const SIOCSPGRP: u64 = _iow::<i32>(b's', 8);
/// Get process group.
pub const SIOCGPGRP: u64 = _ior::<i32>(b's', 9);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_socket_group_is_the_cs() {
        assert_eq!(SIOCATMARK, 0x4004_7307);
        assert_eq!(SIOCSPGRP, 0x8004_7308);
        assert_eq!(SIOCGPGRP, 0x4004_7309);
    }
}
/* </TESTS> */
