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
 * Copyright (c) 1991, 1993, 1994
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
 *	@(#)ufs_vfsops.c	8.4 (Berkeley) 4/16/94
 */
/* </LICENSES> */

/* <CODE> */
//! The file-system-type operations the UFS file systems share: `ufs_start`, `ufs_root` and
//! the generic half of `fhtovp` are the kernel's (`kernel::fs::ufs::ufs::ufs_vfsops`,
//! N5.5a1); `ufs_check_export` and `ufs_init` stay here.
//!
//! Upstream: sys/ufs/ufs/ufs_vfsops.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `ufs_init` is the kernel's (N5.5a2), over `bsd`'s `initialvnodes`; this module keeps the
//!   `Vfsops` slot's shape.
//! - `ufs_check_export`: without `nfsserver` there is no export list (`vfs_export_lookup`
//!   finds nothing), so every client is refused with `EACCES`, as the C does when the lookup
//!   fails. `credanonp` is a `*const Ucred` into the entry's `netc_anon`.

use core::ptr;
use core::sync::atomic::Ordering;

use crate::sys::errno::Errno;
use crate::sys::mbuf::Mbuf;
use crate::sys::mount::{Mount, Vfsconf};
use crate::sys::ucred::Ucred;
use crate::sys::vnode::vfs_export_lookup;
use crate::ufs::ufs::ufsmount::vfstoufs;
pub use kernel::fs::ufs::ufs::ufs_vfsops::{ufs_fhtovp, ufs_root, ufs_start};

/// `ufs_check_export` (`vfs_checkexp`): verify a remote client has export rights and return
/// these rights via `exflagsp` and `credanonp`.
pub fn ufs_check_export(
    mp: &'static Mount,
    nam: &Mbuf,
    exflagsp: &mut i32,
    credanonp: &mut *const Ucred,
) -> Result<(), Errno> {
    let ump = vfstoufs(mp);

    // Get the export permission structure for this <mp, client> tuple.
    let Some(np) = vfs_export_lookup(mp, &ump.g.um_export, Some(nam)) else {
        return Err(Errno::EACCES);
    };

    *exflagsp = np.netc_exflags.get();
    *credanonp = ptr::from_ref(&*np.netc_anon);
    Ok(())
}

/// `ufs_init` (`vfs_init`): initialize UFS file systems, done only once (the kernel's).
pub fn ufs_init(_vfsp: &'static Vfsconf) -> Result<(), Errno> {
    kernel::fs::ufs::ufs::ufs_vfsops::ufs_init::<crate::kern::proc_glue::BsdProcGlue>(
        crate::conf::param::INITIALVNODES.load(Ordering::Relaxed),
    )
}
/* </CODE> */
