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
/* </LICENSES> */

/* <CODE> */
//! The ISO 9660 file system: OpenBSD `sys/isofs/cd9660/`, one module per file of LZ's,
//! feature `cd9660` (`option CD9660`). N5.8c1 brings the on-disc format (`iso`) and the
//! names (`cd9660_util`); N5.8c2 the mount and the file system's table
//! (`cd9660_vfsops`, `Cd9660Vfs`) and the vnode operations (`cd9660_vnops`), and switches
//! `vfsconflist[]` to this one. `testcd9660` is the tests' fixture (host tests and benches
//! only).

pub mod cd9660_util;
pub mod iso;
#[cfg(any(test, feature = "doubles"))]
pub mod testcd9660;
/* </CODE> */
