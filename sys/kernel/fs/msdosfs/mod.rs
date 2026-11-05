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
//! The MS-DOS FAT file system: OpenBSD `sys/msdosfs/`, one module per file of LZ's, feature
//! `msdosfs` (`option MSDOSFS`). N5.7c1 brings the on-disk format (`bootsect`, `bpb`,
//! `direntry`, `fat`), the name and time conversions (`msdosfs_conv`), the mount structure
//! (`msdosfsmount`), the denode type (`denode`) and the FAT engine (`msdosfs_fat`); N5.7c2
//! the denode cache (`msdosfs_denode`), the directories (`msdosfs_lookup`) and the vnode
//! operations with their table (`msdosfs_vnops`); `bsd`'s msdosfs keeps running on its own
//! code until N5.7c3 switches `vfsconflist[]` to this one.

pub mod bootsect;
pub mod bpb;
pub mod denode;
pub mod direntry;
pub mod fat;
pub mod msdosfs_conv;
pub mod msdosfs_denode;
pub mod msdosfs_fat;
pub mod msdosfs_lookup;
pub mod msdosfs_vnops;
pub mod msdosfsmount;
/* </CODE> */
