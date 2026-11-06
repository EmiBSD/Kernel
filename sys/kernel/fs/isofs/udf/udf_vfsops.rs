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
 * Copyright (c) 2001, 2002 Scott Long <scottl@freebsd.org>
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
 * $FreeBSD: src/sys/fs/udf/udf_vfsops.c,v 1.25 2005/01/25 15:52:03 phk Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! UDF file-system-type operations: `udf_init` (the pools) and `udf_checktag` (N5.8u1);
//! the mount, unmount, root, `statfs`, `vget`, file handles and the partition maps follow
//! with the file system's table (N5.8u2).
//!
//! Upstream: sys/isofs/udf/udf_vfsops.c @ 3ce1f3f79392
//! LZ: sys/isofs/udf/udf_vfsops.rs@3c62ede68802
//!
//! Ported to OpenBSD by Pedro Martelletto in February 2005.
//!
//! ## Deviations
//! - `udf_init` takes no `struct vfsconf *` (the C's ignores it); a second call (a host
//!   test's, a second `vfsinit`) keeps the pools made first: their `init` answers `EBUSY`.
//!
//! ## Redesign
//! - LZ's three `Pool` statics and `pool_init`s -> the glue's [`UdfTables`]
//!   (`udf_extern.rs`), made here with the C's sizes' types, IPL, flags and names; `unode_pool`
//!   recorded (`init_recorded`, the UFS inode pool's way: `VTOU` counts a handle back from
//!   the unode the vnode lends). `udf_checktag` reads the tag's 16 bytes by value. 0
//!   `unsafe` in this part (LZ's file: 16, the rest N5.8u2's).
//!
//! [`UdfTables`]: crate::fs::isofs::udf::udf_extern::UdfTables

use hal::intr::IPL_NONE;
use hal::pool::PR_WAITOK;
use hal::sys::endian::letoh16;
use hal::sys::errno::Errno;

use crate::fs::isofs::udf::ecma167_udf::DescTag;
use crate::fs::subr::vfs;
use crate::proc::ProcGlue;

/// `udf_init(vfsp)` (`vfs_init`): the pools of the name buffers, the unodes and the
/// directory streams.
pub fn udf_init<G: ProcGlue>() -> Result<(), Errno> {
    let t = &vfs::<G>().udf;
    for r in [
        t.trans_pool.init(IPL_NONE, PR_WAITOK, "udftrpl"),
        t.unode_pool.init_recorded(IPL_NONE, PR_WAITOK, "udfndpl"),
        t.ds_pool.init(IPL_NONE, PR_WAITOK, "udfdspl"),
    ] {
        match r {
            Ok(()) | Err(Errno::EBUSY) => {}
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

/// Check the descriptor tag for both the correct id and correct checksum. Return zero if
/// all is good, `EINVAL` if not.
pub fn udf_checktag(tag: &DescTag, id: u16) -> Result<(), Errno> {
    if letoh16(tag.id) != id {
        return Err(Errno::EINVAL);
    }

    // The sum of the tag's first 15 bytes but the checksum's own (byte 4), as a `uint8_t`.
    let itag = tag.as_bytes();
    let mut cksum: u8 = 0;
    for &b in &itag[..15] {
        cksum = cksum.wrapping_add(b);
    }
    cksum = cksum.wrapping_sub(itag[4]);

    if cksum == tag.cksum {
        return Ok(());
    }

    Err(Errno::EINVAL)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// A tag: `id`, the checksum `sum` (the right one when `None`), the rest of LZ's test's.
    fn tag(id: u16, sum: Option<u8>) -> DescTag {
        let mut t = [0u8; 16];
        t[0..2].copy_from_slice(&id.to_le_bytes());
        t[2..4].copy_from_slice(&2u16.to_le_bytes());
        t[12..16].copy_from_slice(&0xfedc_ba98u32.to_le_bytes());
        let good = t
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 4 && i < 15)
            .fold(0u8, |s, (_, &b)| s.wrapping_add(b));
        t[4] = sum.unwrap_or(good);
        DescTag::at(&t, 0).unwrap()
    }

    #[test]
    fn checktag_wants_the_id_and_the_sum() {
        assert_eq!(udf_checktag(&tag(2, None), 2), Ok(()));
        assert_eq!(udf_checktag(&tag(2, None), 1), Err(Errno::EINVAL));
        let good = tag(2, None).cksum;
        assert_eq!(
            udf_checktag(&tag(2, Some(good.wrapping_add(1))), 2),
            Err(Errno::EINVAL)
        );
        // Byte 15 (the high byte of tag_loc) is outside the sum, as in the C (i < 15).
        let mut b = tag(261, None).as_bytes();
        b[15] ^= 0xff;
        assert_eq!(udf_checktag(&DescTag::at(&b, 0).unwrap(), 261), Ok(()));
        b[14] ^= 0xff;
        assert_eq!(
            udf_checktag(&DescTag::at(&b, 0).unwrap(), 261),
            Err(Errno::EINVAL)
        );
    }
}
/* </TESTS> */
