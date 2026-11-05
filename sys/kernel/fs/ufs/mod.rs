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
//! The UNIX file system's layers in the kernel: OpenBSD `sys/ufs/`. `ufs` is the layer the
//! UFS-like file systems share (the inode, its mount, the hash, block mapping, and, as the
//! sub-slices of N5.5a move them, lookup, the vnode operations, quotas and dirhash); `ffs`
//! the fast file system over it (N5.5b1, N5.5b2); `mfs` the memory file system, an FFS in a
//! process's memory (N5.5b3, feature `mfs`); `ext2fs` is `bsd`'s until N5.6 and runs on the
//! UFS layer.

pub mod ffs;
#[cfg(feature = "mfs")]
pub mod mfs;
#[allow(clippy::module_inception)] // OpenBSD's sys/ufs/ufs
pub mod ufs;
/* </CODE> */
