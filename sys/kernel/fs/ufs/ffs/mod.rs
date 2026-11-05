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
//! The fast file system: OpenBSD `sys/ufs/ffs/`, one module per file of LZ's: the header
//! `fs` (and `ffs_extern`), the tables, the helpers, the allocators, `balloc` and truncate
//! (N5.5b1), and the mount, the `vfsops` and the vnode operations (N5.5b2).

pub mod ffs_alloc;
pub mod ffs_balloc;
pub mod ffs_extern;
pub mod ffs_inode;
pub mod ffs_subr;
pub mod ffs_tables;
pub mod ffs_vfsops;
pub mod ffs_vnops;
pub mod fs;
#[cfg(any(test, feature = "doubles"))]
pub mod testffs;
/* </CODE> */
