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
//! The second extended file system: OpenBSD `sys/ufs/ext2fs/`, one module per file of LZ's,
//! on the UFS layer (`fs::ufs::ufs`), feature `ext2fs` (`option EXT2FS`). N5.6c1 brings the
//! on-disk format and the in-core super-block (`ext2fs`, `ext2fs_dinode`, `ext2fs_dir`,
//! `ext2fs_extents`, `ext2fs_bswap`), unwired: `bsd`'s ext2fs serves the mounts until
//! N5.6c3.

#[allow(clippy::module_inception)] // OpenBSD's sys/ufs/ext2fs/ext2fs.h
pub mod ext2fs;
pub mod ext2fs_bswap;
pub mod ext2fs_dinode;
pub mod ext2fs_dir;
pub mod ext2fs_extents;
#[cfg(any(test, feature = "doubles"))]
pub mod testext2fs;
/* </CODE> */
