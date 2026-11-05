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
//! The UFS layer: OpenBSD `sys/ufs/ufs/`.
//!
//! The UFS layer is the kernel's (`kernel::fs::ufs::ufs`); what is left here keeps LZ's names
//! for `bsd`'s users: headers (types) `dinode` (libsa's tests), `dir` (the NFS server),
//! `dirhash`, `inode`, `quota`; files (functions) `ufs_dirhash` (feature `ufs_dirhash`,
//! `option UFS_DIRHASH`), `ufs_quota` (feature `quota`, `option QUOTA`) and `ufs_vfsops`
//! (`ufs_check_export`, the NFS server's). `ufs_quota_stub.c` is skipped (see `quota.rs`).
//! `ufs_bmap`, `ufs_extern`, `ufs_ihash`, `ufs_inode`, `ufs_lookup`, `ufs_vnops` and
//! `ufsmount` went with `bsd`'s ext2fs, their last user (N5.6c3).

#[forbid(unsafe_code)]
pub mod dinode;
#[forbid(unsafe_code)]
pub mod dir;
pub mod dirhash;
#[forbid(unsafe_code)]
pub mod inode;
pub mod quota;
#[cfg(feature = "ufs_dirhash")]
pub mod ufs_dirhash;
#[cfg(feature = "quota")]
pub mod ufs_quota;
#[forbid(unsafe_code)]
pub mod ufs_vfsops;
/* </CODE> */
