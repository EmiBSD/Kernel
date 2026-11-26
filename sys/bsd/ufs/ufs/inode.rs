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
 * Copyright (c) 1982, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)inode.h	8.5 (Berkeley) 7/8/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ufs/inode.h>`: the in-core inode is the kernel's (`kernel::fs::ufs::ufs::inode`,
//! N5.5a1): this module keeps LZ's names `Inode` and `vtoi` for the two `bsd` files that
//! still name a UFS inode, `quota.rs` (LZ's signatures of `option QUOTA`'s calls) and
//! `ufs_dirhash.rs`'s tests (over the kernel's ffs).
//!
//! Upstream: sys/ufs/ufs/inode.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - The ext2fs members of the C's `struct inode` (`inode_u.e2fs`, `inode_ext.e2fs`, the
//!   dinode `i_e2din`) are the kernel inode's (`Inode::i_e2fs`, `Dinode::Ext2`) since N5.6c3,
//!   when `bsd`'s ext2fs and this module's `BsdInodeG` (the glue's `InodeG`) went; the C's
//!   `i_e2fs_faddr_hi` names a member that does not exist and is left out (the kernel's
//!   `ext2fs_dinode.rs`). `i_dquot[]` and the dirhash are the kernel inode's since N5.5a2,
//!   ffs's `i_fs` since N5.5b2.
//! - The `struct inode_vtbl` calls (`UFS_TRUNCATE`, `UFS_UPDATE`, ...) are the kernel's
//!   `InodeOps` methods, which the kernel's file systems call as `ip.vtbl().truncate(..)`;
//!   this module's wrappers of them (and the aliases `InodeRef`, `InodeInit` and the other
//!   re-exports) went with their last `bsd` caller, `bsd`'s ext2fs (N5.6c3).

use crate::kern::proc_glue::BsdProcGlue;
pub use kernel::fs::ufs::ufs::inode::vtoi;

/// `struct inode` of `bsd`'s threads: the kernel's.
pub type Inode = kernel::fs::ufs::ufs::inode::Inode<BsdProcGlue>;
/* </CODE> */
