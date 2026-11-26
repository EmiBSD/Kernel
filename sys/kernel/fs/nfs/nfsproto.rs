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
 * Copyright (c) 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Rick Macklem at The University of Guelph.
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
 *	@(#)nfsproto.h	8.2 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/nfsproto.h>` in the kernel: the header's image (`sysdefs::nfs_nfsproto`, which this
//! module re-exports, so `kernel::fs::nfs::nfsproto::NFSX_V3FATTR` is the one constant) and
//! its conversion macros between the vnode's types and modes and the wire's.
//!
//! Upstream: sys/nfs/nfsproto.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfsproto.rs@3c62ede68802
//!
//! ## Deviations
//! - The conversion macros (`vtonfsv2_mode`, `nfsv3tov_type`, ...) are functions over the
//!   tables of `nfs_subs`, where the C defines them; a type is indexed as the C indexes its
//!   arrays (the wire's type masked with 7, so a hostile type never reads past them).
//!
//! ## Redesign
//! - LZ's `bsd` module `sys/nfs/nfsproto.rs` splits in two (N6.7a0, D-10): the constants and
//!   wire structures, which userland's installed header also has, are `hal`'s image; the
//!   conversions, which need the vnode types (`kernel::fs::vnode`), are here. `bsd`'s
//!   `nfs::nfsproto` re-exports this module, so its importers did not change.

pub use sysdefs::nfs_nfsproto::*;

use super::nfs_subs::{NFSV2_TYPE, NFSV3_TYPE, NV2TOV_TYPE, NV3TOV_TYPE};
use super::xdr_subs::{fxdr_unsigned, txdr_unsigned};
use crate::fs::vnode::{Vtype, makeimode};
use sysdefs::types::Mode;

/// `vtonfsv2_mode(t, m)`: a version 2 mode word (a fifo is sent as a character device).
pub fn vtonfsv2_mode(t: Vtype, m: Mode) -> u32 {
    txdr_unsigned(if t == Vtype::VFIFO {
        makeimode(Vtype::VCHR, m)
    } else {
        makeimode(t, m)
    })
}

/// `vtonfsv3_mode(m)`.
pub const fn vtonfsv3_mode(m: Mode) -> u32 {
    txdr_unsigned(m & 0o7777)
}

/// `nfstov_mode(a)`.
pub const fn nfstov_mode(a: u32) -> Mode {
    fxdr_unsigned(a) & 0o7777
}

/// `vtonfsv2_type(a)`.
pub const fn vtonfsv2_type(a: Vtype) -> u32 {
    txdr_unsigned(NFSV2_TYPE[a as usize] as u32)
}

/// `vtonfsv3_type(a)`.
pub const fn vtonfsv3_type(a: Vtype) -> u32 {
    txdr_unsigned(NFSV3_TYPE[a as usize] as u32)
}

/// `nfsv2tov_type(a)`.
pub const fn nfsv2tov_type(a: u32) -> Vtype {
    NV2TOV_TYPE[(fxdr_unsigned(a) & 0x7) as usize]
}

/// `nfsv3tov_type(a)`.
pub const fn nfsv3tov_type(a: u32) -> Vtype {
    NV3TOV_TYPE[(fxdr_unsigned(a) & 0x7) as usize]
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the conversions between the vnode's types and modes and the wire's.

    use super::*;
    use crate::fs::vnode::Vtype::{VBLK, VDIR, VFIFO, VLNK, VNON, VREG, VSOCK};

    #[test]
    fn type_and_mode_conversions() {
        assert_eq!(nfsv3tov_type(txdr_unsigned(NFFIFO as u32)), VFIFO);
        assert_eq!(nfsv2tov_type(txdr_unsigned(NFSOCK as u32)), VNON);
        assert_eq!(nfsv3tov_type(txdr_unsigned(NFSOCK as u32)), VSOCK);
        assert_eq!(nfsv3tov_type(txdr_unsigned(8 | NFDIR as u32)), VDIR);
        assert_eq!(fxdr_unsigned(vtonfsv3_type(VLNK)), NFLNK as u32);
        assert_eq!(fxdr_unsigned(vtonfsv2_type(VFIFO)), NFCHR as u32);
        assert_eq!(fxdr_unsigned(vtonfsv2_type(VREG)), NFREG as u32);
        assert_eq!(fxdr_unsigned(vtonfsv3_type(VBLK)), NFBLK as u32);
        assert_eq!(fxdr_unsigned(vtonfsv2_mode(VFIFO, 0o644)), 0o020644);
        assert_eq!(fxdr_unsigned(vtonfsv2_mode(VDIR, 0o755)), 0o040755);
        assert_eq!(fxdr_unsigned(vtonfsv3_mode(0o104755)), 0o4755);
        assert_eq!(nfstov_mode(txdr_unsigned(0o100644)), 0o644);
    }

    #[test]
    fn every_wire_type_maps_inside_the_tables() {
        // The mask keeps any word, however hostile, inside the eight entries.
        for w in [0u32, 7, 8, 0xffff_ffff, 0x8000_0007] {
            let _ = nfsv2tov_type(txdr_unsigned(w));
            let _ = nfsv3tov_type(txdr_unsigned(w));
        }
    }
}
/* </TESTS> */
