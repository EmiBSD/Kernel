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
 * Copyright (c) 2013 Pedro Martelletto
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
//! tmpfs vnode interface for named pipes: the operations of a fifo stored in a tmpfs file
//! system (`tmpfs_fifovops`, [`TmpfsFifoVnops`]), which keep the node's times and leave the
//! pipe to fifofs.
//!
//! Upstream: sys/tmpfs/tmpfs_fifoops.c @ 3ce1f3f79392
//! LZ: sys/tmpfs/tmpfs_fifoops.rs@3c62ede68802
//!
//! ## Deviations
//! - `option FIFO` is in GENERIC but `miscfs/fifofs` is not ported (the ffs precedent):
//!   the `fifo_*` operations the table installs (`fifo_open`, `fifo_close`, `fifo_ioctl`,
//!   `fifo_kqfilter`, `fifo_pathconf`, `fifo_advlock`) and the `fifo_read`/`fifo_write`
//!   that `tmpfs_fifo_read`/`tmpfs_fifo_write` end in are visible gaps (`unported!`,
//!   `ENOSYS`). Nothing reaches them yet: `tmpfs_vnode_get` refuses a fifo node with
//!   `EOPNOTSUPP` and never installs this table (LZ's; D-N5.9-14).
//!
//! ## Redesign
//! - LZ's `TMPFS_FIFOVOPS` (a `bsd` `Vops`, its badops and `unported!` closures) ->
//!   [`TmpfsFifoVnops`], one `impl Vnops<G>` of the C's slots (decision 46), a member of the
//!   glue's `TmpfsTables` (one address per glue): tmpfs's nine common slots
//!   (`tmpfs_vnops.rs`'s `tmpfs_common_slots!`), the C's `vop_generic_*` and the fifo gaps.

use core::marker::PhantomData;

use hal::sys::errno::Errno;
use hal::sys::fcntl::Flock;
use hal::sys::types::{Daddr, Register};

use crate::event::Knote;
use crate::file::LockOwner;
use crate::fs::default;
use crate::fs::tmpfs::tmpfs::{TMPFS_NODE_ACCESSED, TMPFS_NODE_MODIFIED, vptonode};
use crate::fs::tmpfs::tmpfs_subr::tmpfs_update;
use crate::fs::tmpfs::tmpfs_vnops::{tmpfs_bwrite, tmpfs_common_slots};
use crate::fs::vnode::{Vattr, Vnode};
use crate::fs::vops::{Cn, Cred, IoBuf, Vnops};
use crate::prf::unported;
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;

/// `tmpfs_fifovops`: vnode operations vector used for fifos stored in a tmpfs file system
/// (the glue's `TmpfsTables::fifovops`, one address; installed by nothing until fifofs is
/// ported, the module's deviations).
pub struct TmpfsFifoVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member (as `TmpfsVnops`).
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> TmpfsFifoVnops<G> {
    /// The table (the glue's `TmpfsTables::fifovops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `tmpfs_fifo_read` (`vop_read`): mark the node accessed, then read the pipe.
pub fn tmpfs_fifo_read<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    tmpfs_update(&vptonode(vp), TMPFS_NODE_ACCESSED);
    // fifo_read(v): miscfs/fifofs, not ported.
    Err(unported!("fifo_read (miscfs/fifofs)"))
}

/// `tmpfs_fifo_write` (`vop_write`): mark the node modified, then write the pipe.
pub fn tmpfs_fifo_write<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    tmpfs_update(&vptonode(vp), TMPFS_NODE_MODIFIED);
    // fifo_write(v): miscfs/fifofs, not ported.
    Err(unported!("fifo_write (miscfs/fifofs)"))
}

/// `tmpfs_fifo_fsync` (`vop_fsync`): nothing to flush.
pub fn tmpfs_fifo_fsync() -> Result<(), Errno> {
    Ok(())
}

/// `tmpfs_fifovops`: tmpfs's slots and `fifo_vops`' others (the C's "XXX: Keep in sync with
/// fifo_vops").
impl<G: ProcGlue> Vnops<G> for TmpfsFifoVnops<G> {
    tmpfs_common_slots!();

    fn read(
        &self,
        vp: &'static Vnode<G>,
        _uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        tmpfs_fifo_read(vp)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        _uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        tmpfs_fifo_write(vp)
    }
    fn fsync(
        &self,
        _vp: &'static Vnode<G>,
        _cred: Cred<'_>,
        _waitfor: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        tmpfs_fifo_fsync()
    }
    fn bwrite(&self, _bp: IoBuf) -> Result<(), Errno> {
        tmpfs_bwrite()
    }

    // XXX: Keep in sync with fifo_vops.
    fn lookup(
        &self,
        _dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        _cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_lookup(vpp)
    }
    fn create(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn mknod(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn open(
        &self,
        _vp: &'static Vnode<G>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_open (miscfs/fifofs)"))
    }
    fn close(
        &self,
        _vp: &'static Vnode<G>,
        _fflag: i32,
        _cred: Cred<'_>,
        _p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_close (miscfs/fifofs)"))
    }
    fn ioctl(
        &self,
        _vp: &'static Vnode<G>,
        _command: u64,
        _data: &mut [u8],
        _fflag: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_ioctl (miscfs/fifofs)"))
    }
    fn kqfilter(&self, _vp: &'static Vnode<G>, _fflag: i32, _kn: &Knote<G>) -> Result<(), Errno> {
        Err(unported!("fifo_kqfilter (miscfs/fifofs)"))
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn remove(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn link(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn rename(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
        _: &'static Vnode<G>,
        _: Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn mkdir(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn rmdir(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn symlink(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
        _: &[u8],
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn readdir(
        &self,
        _: &'static Vnode<G>,
        _: &mut Uio<'_, G>,
        _: Cred<'_>,
        _: &mut i32,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn readlink(&self, _: &'static Vnode<G>, _: &mut Uio<'_, G>, _: Cred<'_>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn abortop(&self, _: &'static Vnode<G>, _: &mut Cn<'_, G>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        default::vop_generic_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, _: &'static Vnode<G>, _: IoBuf) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn pathconf(
        &self,
        _vp: &'static Vnode<G>,
        _name: i32,
        _retval: &mut Register,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_pathconf (miscfs/fifofs)"))
    }
    fn advlock(
        &self,
        _vp: &'static Vnode<G>,
        _id: LockOwner,
        _op: i32,
        _fl: &mut Flock,
        _flags: i32,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_advlock (miscfs/fifofs)"))
    }
}
/* </CODE> */
