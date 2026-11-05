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
//! N5.5a1): this module keeps LZ's names for what of `bsd` still names a UFS inode (the
//! quota and dirhash adapters and their tests, over the kernel's ffs).
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
//! - The `struct inode_vtbl` calls (`UFS_TRUNCATE`, `UFS_UPDATE`, ...) are functions with the
//!   macros' names over the kernel's `InodeOps` ([`Inode::vtbl`]); `iv_inode_alloc` and
//!   `iv_buf_alloc` return the vnode or buffer the C passes back through `vpp`/`bpp`,
//!   `iv_bufatoff` the buffer and the offset of `res` in it, lent (`IoBuf::Lent`) as `bsd`'s
//!   buffers are.

use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::buf::Buf;
use crate::sys::errno::Errno;
use crate::sys::types::{Mode, Off};
use crate::sys::vnode::Vnode;
use crate::ufs::ufs::dinode::Ufsino;
pub use kernel::fs::ufs::ufs::inode::{
    CiCell, DinField, DinWords, Dinode, I32Word, IN_ACCESS, IN_CHANGE, IN_EXLOCK, IN_HASHED,
    IN_LAZYMOD, IN_MODIFIED, IN_RENAME, IN_SHLOCK, IN_UPDATE, Indir, InodeOps, U32Word, U64Word,
    Ufid, Ufs1Din, Ufs2Din, daddr32_at, daddr64_at, dinode1_at, dinode2_at, doingasync,
    set_daddr32_at, set_daddr64_at, set_dinode1_at, set_dinode2_at, vtoi,
};
use kernel::fs::vops::{Cred, IoBuf};

/// `struct inode` of `bsd`'s threads: the kernel's.
pub type Inode = kernel::fs::ufs::ufs::inode::Inode<BsdProcGlue>;
/// A counted handle to one of `bsd`'s inodes ([`vtoi`]'s).
pub type InodeRef = kernel::fs::ufs::ufs::inode::InodeRef<BsdProcGlue>;
/// What [`Inode::new`] takes.
pub type InodeInit = kernel::fs::ufs::ufs::inode::InodeInit<BsdProcGlue>;

/// `UFS_TRUNCATE(ip, off, flags, cred)`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_TRUNCATE(ip: &Inode, off: Off, flags: i32, cred: Cred<'_>) -> Result<(), Errno> {
    ip.vtbl().truncate(ip, off, flags, cred)
}

/// `UFS_UPDATE(ip, sync)`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_UPDATE(ip: &Inode, sync: i32) -> Result<(), Errno> {
    ip.vtbl().update(ip, sync)
}

/// `UFS_INODE_ALLOC(pip, mode, cred, vpp)`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_INODE_ALLOC(pip: &Inode, mode: Mode, cred: Cred<'_>) -> Result<&'static Vnode, Errno> {
    pip.vtbl().inode_alloc(pip, mode, cred)
}

/// `UFS_INODE_FREE(pip, ino, mode)`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_INODE_FREE(pip: &Inode, ino: Ufsino, mode: Mode) -> Result<(), Errno> {
    pip.vtbl().inode_free(pip, ino, mode)
}

/// The buffer an `InodeOps` method gave, as `bsd`'s callers hold it: lent (its token parked
/// in it), as `bsd`'s file systems give theirs.
fn lent(b: IoBuf) -> &'static Buf {
    match b {
        IoBuf::Lent(bp) => bp,
        IoBuf::Token(t) => crate::sys::buf::lend(t),
    }
}

/// `UFS_BUF_ALLOC(ip, startoffset, size, cred, flags, bpp)`: the buffer the C returns in
/// `*bpp`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_BUF_ALLOC(
    ip: &Inode,
    startoffset: Off,
    size: i32,
    cred: Cred<'_>,
    flags: i32,
) -> Result<&'static Buf, Errno> {
    ip.vtbl()
        .buf_alloc(ip, startoffset, size, cred, flags)
        .map(lent)
}

/// `UFS_BUFATOFF(ip, offset, res, bpp)`: the buffer and the offset of `res` in its data, lent
/// as `bsd`'s buffers are (no `bsd` caller is left since N5.5a3: the kernel's `ufs_lookup` and
/// `ufs_readdir` take the inode's token).
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_BUFATOFF(ip: &Inode, offset: Off) -> Result<(&'static Buf, usize), Errno> {
    ip.vtbl()
        .bufatoff(ip, offset)
        .map(|(b, off)| (lent(b), off))
}
/* </CODE> */
