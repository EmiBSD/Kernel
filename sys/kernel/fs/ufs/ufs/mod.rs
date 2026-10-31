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
//! The UFS layer: OpenBSD `sys/ufs/ufs/`, one module per file of LZ's (N5.5a1: the headers
//! `dinode`, `dir`, `inode`, `ufsmount`, `ufs_extern` and the files `ufs_bmap`, `ufs_ihash`,
//! `ufs_vfsops`; N5.5a2: `dirhash`, `quota`, `ufs_dirhash` (feature `ufs_dirhash`,
//! `option UFS_DIRHASH`), `ufs_inode`, `ufs_quota`; the rest follow at N5.5a3).

pub mod dinode;
pub mod dir;
pub mod dirhash;
pub mod inode;
pub mod quota;
pub mod ufs_bmap;
#[cfg(feature = "ufs_dirhash")]
pub mod ufs_dirhash;
pub mod ufs_extern;
pub mod ufs_ihash;
pub mod ufs_inode;
pub mod ufs_quota;
pub mod ufs_vfsops;
pub mod ufsmount;
/* </CODE> */
