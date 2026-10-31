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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)ufsmount.h	8.4 (Berkeley) 10/27/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ufs/ufsmount.h>`: the UFS specific mount structure data (`struct ufsmount`) is the
//! kernel's (`kernel::fs::ufs::ufs::ufsmount`, N5.5a1): this module keeps LZ's names, the
//! members of `bsd`'s types it carries ([`BsdUfsmountG`], the glue's `UfsmountG`) and
//! [`vfstoufs`].
//!
//! Upstream: sys/ufs/ufs/ufsmount.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - The structure is `malloc(M_UFSMNT)`ed by `ffs_mountfs` and `ext2fs_mountfs` and freed by
//!   their unmounts; it is reached as `&'static Ufsmount` through [`vfstoufs`] (the C's
//!   `VFSTOUFS` cast), whose one `unsafe` checks that the mount is a UFS one: kept until
//!   N5.5b types its holders (the kernel's typed slot of the mount waits for its first kernel
//!   reader, N5.5a2's quotas: this member's `um_export` is not `Send`).
//! - `ufsmount_u` (the super-block pointer of FFS or of EXT2FS) is `g.um_fs`; with feature
//!   `ext2fs` (`option EXT2FS`) the union's other member is the separate field `g.um_e2fs`
//!   ([`UfsmountExt::e2fs`]): a mount is FFS's or ext2fs's, never both. The C's `um_e2fsb`
//!   (`um_e2fs->s_es`) names a member that does not exist (nothing uses it);
//!   [`UfsmountExt::with_e2fsb`] is its meaning, the in-core super block `um_e2fs->e2fs` for
//!   the duration of a closure.
//! - `um_export` (`struct netexport`) is kept whether or not `nfsserver` is configured, as the
//!   C does; without the feature `vfs_export` answers `ENOTSUP` and the list stays empty.

use core::cell::Cell;

use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::sys::mount::{Mount, Netexport};
#[cfg(feature = "ext2fs")]
use crate::ufs::ext2fs::ext2fs::{Ext2fs, MExt2fs};
use crate::ufs::ffs::fs::Fs;
pub use kernel::fs::ufs::ufs::ufsmount::{
    QTF_CLOSING, QTF_OPENING, UM_EXT2FS, UM_UFS1, UM_UFS2, blkptrtodb, is_sequential, mnindir,
};

/// `struct ufsmount` of `bsd`'s mounts: the kernel's.
pub type Ufsmount = kernel::fs::ufs::ufs::ufsmount::Ufsmount<BsdProcGlue>;

/// The members of `bsd`'s types a UFS mount carries (the glue's `UfsmountG`,
/// docs/SCAFFOLDING.md).
pub struct BsdUfsmountG {
    /// `um_fs`: pointer to superblock (FFS).
    pub um_fs: Cell<Option<&'static Fs>>,
    /// `um_e2fs` (`ufsmount_u.e2fs`): pointer to the in-core super block (EXT2FS), in place
    /// of `um_fs` for an ext2fs mount.
    #[cfg(feature = "ext2fs")]
    pub um_e2fs: Cell<Option<&'static MExt2fs>>,
    /// `um_export`: export information.
    pub um_export: Netexport,
}

impl Default for BsdUfsmountG {
    fn default() -> Self {
        Self {
            um_fs: Cell::new(None),
            #[cfg(feature = "ext2fs")]
            um_e2fs: Cell::new(None),
            um_export: Netexport::new(),
        }
    }
}

// SAFETY: LZ's argument for `struct ufsmount` (sys/ufs/ufs/ufsmount.rs:125-127 at the pin),
// moved with the members it covered (the same site, not a new one): they are changed under the
// kernel lock while mounting and unmounting, as in C. Paid member by member: the quotas went at
// N5.5a2 (the kernel structure's), `um_fs` at N5.5b, `um_e2fs` at N5.6, `um_export` at N6.
unsafe impl Sync for BsdUfsmountG {}

/// The members of `struct ufsmount` that `bsd`'s types answer.
pub trait UfsmountExt {
    /// `ump->um_fs`, which every mounted UFS has.
    fn fs(&self) -> &'static Fs;
    /// `ump->um_e2fs`, which every mounted ext2fs has (feature `ext2fs`).
    #[cfg(feature = "ext2fs")]
    fn e2fs(&self) -> &'static MExt2fs;
    /// `f(&ump->um_e2fsb)`: the in-core ext2fs super block, for the duration of `f`.
    #[cfg(feature = "ext2fs")]
    fn with_e2fsb<R>(&self, f: impl FnOnce(&Ext2fs) -> R) -> R;
}

impl UfsmountExt for Ufsmount {
    fn fs(&self) -> &'static Fs {
        match self.g.um_fs.get() {
            Some(fs) => fs,
            None => panic(format_args!("ufsmount {:p}: no um_fs", self)),
        }
    }

    #[cfg(feature = "ext2fs")]
    fn e2fs(&self) -> &'static MExt2fs {
        match self.g.um_e2fs.get() {
            Some(fs) => fs,
            None => panic(format_args!("ufsmount {:p}: no um_e2fs", self)),
        }
    }

    #[cfg(feature = "ext2fs")]
    fn with_e2fsb<R>(&self, f: impl FnOnce(&Ext2fs) -> R) -> R {
        self.e2fs().with_e2fs(f)
    }
}

/// `VFSTOUFS(mp)`: convert mount ptr to ufsmount ptr.
pub fn vfstoufs(mp: &Mount) -> &'static Ufsmount {
    let data = mp.data();
    let ufs = matches!(mp.vfc().name(), b"ffs" | b"mfs" | b"ext2fs");
    if data.is_null() || !ufs {
        panic(format_args!(
            "VFSTOUFS: mount {:p} is not a mounted UFS",
            mp
        ));
    }
    // SAFETY: a UFS mount's `mnt_data` (checked above) is the `struct ufsmount` that
    // `ffs_mountfs` allocated, which lives until `ffs_unmount` frees it and clears
    // `mnt_data`.
    unsafe { &*data.cast::<Ufsmount>() }
}
/* </CODE> */
