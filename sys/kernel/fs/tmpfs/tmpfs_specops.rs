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
 * Copyright (c) 2005 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Julio M. Merino Vidal, developed as part of Google's Summer of Code
 * 2005 program.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! tmpfs vnode interface for special devices: the operations of a block or character
//! device node stored in a tmpfs file system (`tmpfs_specvops`, [`TmpfsSpecVnops`]), which
//! keep the node's times and leave the device work to `spec_vops`.
//!
//! Upstream: sys/tmpfs/tmpfs_specops.c @ 3ce1f3f79392
//! LZ: sys/tmpfs/tmpfs_specops.rs@3c62ede68802
//!
//! ## Redesign
//! - LZ's `TMPFS_SPECVOPS` (a `bsd` `Vops`, its badops closures) -> [`TmpfsSpecVnops`], one
//!   `impl Vnops<G>` (decision 46), a member of the glue's `TmpfsTables` (one address per
//!   glue for `vops_id`, a byte so that it is not zero-sized): tmpfs's nine common slots
//!   (`tmpfs_vnops.rs`'s `tmpfs_common_slots!`), `tmpfs_spec_read`/`tmpfs_spec_write`,
//!   `spec_close`, `spec_fsync`, `vop_generic_revoke`, `vop_generic_bwrite`, and the kernel's
//!   `spec_vops` slots for the rest (`fs::spec::spec_shared_slots!`, the C's "keep in sync
//!   with spec_vops"; cd9660's `Cd9660SpecVnops` shape).

use core::marker::PhantomData;

use hal::sys::errno::Errno;

use crate::fs::default;
use crate::fs::tmpfs::tmpfs::{TMPFS_NODE_ACCESSED, TMPFS_NODE_MODIFIED, vptonode};
use crate::fs::tmpfs::tmpfs_subr::tmpfs_update;
use crate::fs::tmpfs::tmpfs_vnops::tmpfs_common_slots;
use crate::fs::vnode::Vnode;
use crate::fs::vops::{Cred, IoBuf, Vnops};
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;

/// `tmpfs_specvops`: vnode operations vector used for special devices stored in a tmpfs
/// file system (the glue's `TmpfsTables::specvops`, one address).
pub struct TmpfsSpecVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member (as `TmpfsVnops`).
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> TmpfsSpecVnops<G> {
    /// The table (the glue's `TmpfsTables::specvops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `tmpfs_spec_read` (`vop_read`): mark the node accessed, then read the device.
pub fn tmpfs_spec_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    tmpfs_update(&vptonode(vp), TMPFS_NODE_ACCESSED);
    crate::fs::spec::spec_read(vp, uio, ioflag)
}

/// `tmpfs_spec_write` (`vop_write`): mark the node modified, then write the device.
pub fn tmpfs_spec_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    tmpfs_update(&vptonode(vp), TMPFS_NODE_MODIFIED);
    crate::fs::spec::spec_write(vp, uio, ioflag)
}

/// `tmpfs_specvops`: tmpfs's slots and `spec_vops`' others (the C's "keep in sync with
/// spec_vops").
impl<G: ProcGlue> Vnops<G> for TmpfsSpecVnops<G> {
    tmpfs_common_slots!();

    fn close(
        &self,
        vp: &'static Vnode<G>,
        fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        crate::fs::spec::spec_close(vp, fflag, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        tmpfs_spec_read(vp, uio, ioflag)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        tmpfs_spec_write(vp, uio, ioflag)
    }
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        _cred: Cred<'_>,
        waitfor: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        crate::fs::spec::spec_fsync(vp, waitfor)
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        default::vop_generic_bwrite(bp)
    }

    // keep in sync with spec_vops (one macro: the same slots as SpecVops's).
    crate::fs::spec::spec_shared_slots!();
}
/* </CODE> */
