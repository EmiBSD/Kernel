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
 * Copyright (c) 1982, 1986, 1993, 1994
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
 *	@(#)uio.h	8.5 (Berkeley) 2/22/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/uio.h>` for `bsd`: the kernel's scatter/gather types (`kernel::uio`), with `Uio`
//! bound to `bsd`'s threads.
//!
//! Upstream: sys/sys/uio.h @ 3ce1f3f79392
//! LZ: sys/sys/uio.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.3: `struct iovec`, `enum uio_rw`, `enum uio_seg` and `struct uio` moved into
//!   `kernel::uio` with the descriptor code that reads them; this module re-exports them and
//!   names the one `Uio` that `bsd` builds (`kernel::uio::Uio<'a, BsdProcGlue>`, whose
//!   `uio_procp` is `bsd`'s `Proc`), so its 98 users (the 5 frozen amd64 files among them,
//!   decision 44) keep their paths, struct literals and fields. LZ's deviations went with the
//!   types (`kernel::uio`).

pub use kernel::uio::{Iovec, UIO_MAXIOV, UIO_SMALLIOV, UioRw, UioSeg};

use crate::kern::proc_glue::BsdProcGlue;

/// `struct uio` of `bsd`'s threads: a scatter/gather transfer in progress.
pub type Uio<'a> = kernel::uio::Uio<'a, BsdProcGlue>;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bsd_uio_is_written_as_before() {
        let mut iov = [
            Iovec {
                iov_base: core::ptr::null_mut(),
                iov_len: 4,
            },
            Iovec::new(),
        ];
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 7,
            uio_resid: 4,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_WRITE,
            uio_procp: None,
        };
        assert_eq!(uio.uio_iovcnt(), 2);
        // `uio->uio_iov++; uio->uio_iovcnt--`: the slice shortens.
        let rest = core::mem::take(&mut uio.uio_iov);
        uio.uio_iov = &mut rest[1..];
        assert_eq!(uio.uio_iovcnt(), 1);
        assert_eq!(uio.uio_iov[0].iov_len, 0);
        assert_eq!(UIO_SMALLIOV, 8);
        assert_eq!(UIO_MAXIOV, 1024);
    }
}
/* </TESTS> */
