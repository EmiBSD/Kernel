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
//! `ext2fs_extents`, `ext2fs_bswap`) and the allocators, `balloc`, the block maps and
//! truncate (`ext2fs_alloc`, `ext2fs_balloc`, `ext2fs_bmap`, `ext2fs_inode`,
//! `ext2fs_subr`); N5.6c2 the directories, file I/O, the vnode operations and their two
//! tables (`ext2fs_lookup`, `ext2fs_readwrite`, `ext2fs_vnops`, `ext2fs_extern`); N5.6c3 the
//! mount (`ext2fs_vfsops`: `Ext2fsVfs<G>`, `ext2fs_mountroot`) and the fixture's real mount,
//! unwired: `bsd`'s ext2fs serves the mounts until N5.6c3's switch.

#[allow(clippy::module_inception)] // OpenBSD's sys/ufs/ext2fs/ext2fs.h
pub mod ext2fs;
pub mod ext2fs_alloc;
pub mod ext2fs_balloc;
pub mod ext2fs_bmap;
pub mod ext2fs_bswap;
pub mod ext2fs_dinode;
pub mod ext2fs_dir;
pub mod ext2fs_extents;
pub mod ext2fs_extern;
pub mod ext2fs_inode;
pub mod ext2fs_lookup;
pub mod ext2fs_readwrite;
pub mod ext2fs_subr;
pub mod ext2fs_vfsops;
pub mod ext2fs_vnops;
#[cfg(any(test, feature = "doubles"))]
pub mod testext2fs;
/* </CODE> */
