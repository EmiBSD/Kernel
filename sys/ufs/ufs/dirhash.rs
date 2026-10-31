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
 * Copyright (c) 2001 Ian Dowse.  All rights reserved.
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
 * $FreeBSD: src/sys/ufs/ufs/dirhash.h,v 1.4 2003/01/01 18:48:59 schweikh Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ufs/dirhash.h>`: the hash of a large directory is the kernel's
//! (`kernel::fs::ufs::ufs::dirhash`, N5.5a2): this module keeps LZ's names for `bsd`.
//!
//! Upstream: sys/ufs/ufs/dirhash.h @ 3ce1f3f79392

pub use kernel::fs::ufs::ufs::dirhash::{
    DH_BLKOFFMASK, DH_BLKOFFSHIFT, DH_NBLKOFF, DH_NFSTATS, DH_SCOREINIT, DH_SCOREMAX, DIRALIGN,
    DIRHASH_DEL, DIRHASH_EMPTY, DhArrays, DhBlock, DhTables, Dirhash, DirhashList,
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ufs/dirhash.h");
        for (name, v) in [
            ("DIRHASH_EMPTY", i64::from(DIRHASH_EMPTY)),
            ("DIRHASH_DEL", i64::from(DIRHASH_DEL)),
            ("DIRALIGN", i64::from(DIRALIGN)),
            ("DH_SCOREINIT", i64::from(DH_SCOREINIT)),
            ("DH_SCOREMAX", i64::from(DH_SCOREMAX)),
            ("DH_BLKOFFSHIFT", i64::from(DH_BLKOFFSHIFT)),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(v), "{name}");
        }
    }
}
/* </TESTS> */
