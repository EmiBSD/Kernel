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
 *	@(#)ufs_vnops.c	8.14 (Berkeley) 10/26/94
 */
/* </LICENSES> */

/* <CODE> */
//! The vnode operations the UFS file systems share are the kernel's
//! (`kernel::fs::ufs::ufs::ufs_vnops`, N5.5a3), and so is ffs's vnode table (`ffs_vnops.rs`'s
//! `FFS_VOPS` and `FFS_SPECVOPS` are statics of the kernel's `UfsVnops` and `UfsSpecVnops`).
//! This module keeps LZ's names for `bsd`'s file systems, and the slots of `bsd`'s `Vops`
//! shape that ext2fs's tables still name (`ufs_close`, `ufs_ioctl`, `ufs_kqfilter`,
//! `ufs_lock`, `ufs_unlock`, `ufs_strategy`, `ufs_print`, `ufs_islocked`, the three
//! `ufsspec_*`, and `ufs_pathconf`, which `ext2fs_pathconf` calls): each unpacks its
//! `struct vop_*_args` and calls the kernel's (docs/SCAFFOLDING.md, until ext2fs moves at
//! N5.6).
//!
//! Upstream: sys/ufs/ufs/ufs_vnops.c @ 3ce1f3f79392

use crate::sys::errno::Errno;
use crate::sys::vnode::{
    VopCloseArgs, VopIoctlArgs, VopIslockedArgs, VopKqfilterArgs, VopLockArgs, VopPathconfArgs,
    VopPrintArgs, VopReadArgs, VopStrategyArgs, VopUnlockArgs, VopWriteArgs,
};
use kernel::fs::ufs::ufs::ufs_vnops as k;
pub use kernel::fs::ufs::ufs::ufs_vnops::{
    MASTERTEMPLATE, UfsFs, UfsSpecVnops, UfsVnops, pnbuf_free, ucred, ufs_itimes,
};
use kernel::fs::vops::IoBuf;

/// `ufs_close` (`vop_close`), the slot of `bsd`'s `Vops` tables.
pub fn ufs_close(ap: &mut VopCloseArgs<'_>) -> Result<(), Errno> {
    k::ufs_close(ap.a_vp)
}

/// `ufs_ioctl` (`vop_ioctl`), the slot of `bsd`'s `Vops` tables.
pub fn ufs_ioctl(_ap: &mut VopIoctlArgs<'_>) -> Result<(), Errno> {
    k::ufs_ioctl()
}

/// `ufs_kqfilter` (`vop_kqfilter`), the slot of `bsd`'s `Vops` tables.
pub fn ufs_kqfilter(ap: &mut VopKqfilterArgs<'_>) -> Result<(), Errno> {
    k::ufs_kqfilter(ap.a_vp, ap.a_kn)
}

/// `ufs_lock` (`vop_lock`), the slot of `bsd`'s `Vops` tables.
pub fn ufs_lock(ap: &mut VopLockArgs) -> Result<(), Errno> {
    k::ufs_lock(ap.a_vp, ap.a_flags)
}

/// `ufs_unlock` (`vop_unlock`), the slot of `bsd`'s `Vops` tables.
pub fn ufs_unlock(ap: &mut VopUnlockArgs) -> Result<(), Errno> {
    k::ufs_unlock(ap.a_vp)
}

/// `ufs_islocked` (`vop_islocked`), the slot of `bsd`'s `Vops` tables.
pub fn ufs_islocked(ap: &mut VopIslockedArgs) -> i32 {
    k::ufs_islocked(ap.a_vp)
}

/// `ufs_strategy` (`vop_strategy`), the slot of `bsd`'s `Vops` tables: the buffer as `bsd`
/// lent it.
pub fn ufs_strategy(ap: &mut VopStrategyArgs) -> Result<(), Errno> {
    k::ufs_strategy(ap.a_vp, IoBuf::Lent(ap.a_bp))
}

/// `ufs_print` (`vop_print`), the slot of `bsd`'s `Vops` tables.
pub fn ufs_print(ap: &mut VopPrintArgs) -> Result<(), Errno> {
    k::ufs_print(ap.a_vp)
}

/// `ufsspec_read`, the slot of `bsd`'s `Vops` tables.
pub fn ufsspec_read(ap: &mut VopReadArgs<'_, '_>) -> Result<(), Errno> {
    k::ufsspec_read(ap.a_vp, ap.a_uio, ap.a_ioflag)
}

/// `ufsspec_write`, the slot of `bsd`'s `Vops` tables.
pub fn ufsspec_write(ap: &mut VopWriteArgs<'_, '_>) -> Result<(), Errno> {
    k::ufsspec_write(ap.a_vp, ap.a_uio, ap.a_ioflag)
}

/// `ufsspec_close`, the slot of `bsd`'s `Vops` tables.
pub fn ufsspec_close(ap: &mut VopCloseArgs<'_>) -> Result<(), Errno> {
    k::ufsspec_close(ap.a_vp, ap.a_fflag, ap.a_cred, ap.a_p)
}

/// `ufs_pathconf` (`vop_pathconf`), as `ext2fs_pathconf` calls it.
pub fn ufs_pathconf(ap: &mut VopPathconfArgs<'_>) -> Result<(), Errno> {
    k::ufs_pathconf(ap.a_vp, ap.a_name, ap.a_retval)
}
/* </CODE> */
