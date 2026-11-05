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

/*-
 * Copyright (c) 1997 Manuel Bouyer.
 * Copyright (c) 1991, 1993, 1994
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)ffs_extern.h	8.3 (Berkeley) 4/16/94
 * Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ext2fs/ext2fs_extern.h>`: the prototypes of the functions of `ufs/ext2fs`, the pools
//! and operation tables they share, and `IS_EXT2_VNODE`.
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_extern.h @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_extern.rs@3c62ede68802
//!
//! ## Redesign
//! - N5.6c2: the header's prototypes are the items of the kernel's `ext2fs_*.rs` modules, and
//!   `ext2fs_vops` and `ext2fs_specvops` are the table types [`Ext2fsVnops`] and
//!   [`Ext2fsSpecVnops`] (their one instance each the glue's `UfsTables` members);
//!   `ext2fs_fifovops` waits for `miscfs/fifofs` (`option FIFO`), and the pools
//!   (`ext2fs_inode_pool`, `ext2fs_dinode_pool`) come with `ext2fs_vfsops` at N5.6c3. LZ's
//!   `pub use` of them goes, as `ffs_extern`'s did at N5.5b2 and `mfs_extern`'s at N5.5b3:
//!   their callers name the defining modules. The header's forward declarations of
//!   structures need no Rust; what is left is `IS_EXT2_VNODE`, [`is_ext2_vnode`].
//!
//! [`Ext2fsVnops`]: super::ext2fs_vnops::Ext2fsVnops
//! [`Ext2fsSpecVnops`]: super::ext2fs_vnops::Ext2fsSpecVnops

use crate::fs::vnode::{VT_EXT2FS, Vnode};
use crate::proc::ProcGlue;

/// `IS_EXT2_VNODE(vp)`: whether the vnode belongs to ext2fs.
pub fn is_ext2_vnode<G: ProcGlue>(vp: &Vnode<G>) -> bool {
    vp.tag() == VT_EXT2FS
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::assert;

    use hal::sys::mount::MNT_RDONLY;

    use super::super::testext2fs::{attach, mount, newfs, root};
    use super::*;
    use crate::fs::subr::vput;

    #[test]
    fn an_ext2fs_vnode_is_tagged_so() {
        let (_g, _p) = attach(newfs::image(false));
        let mp = mount(MNT_RDONLY);
        let vp = root(mp);
        assert!(is_ext2_vnode(vp));
        vput(vp);
        // The device under it is no ext2fs vnode.
        assert!(!is_ext2_vnode(crate::fs::ufs::ffs::testffs::devvp()));
    }
}
/* </TESTS> */
