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
 * Copyright (c) 1989, 1993
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
 *	@(#)dead_vnops.c	8.2 (Berkeley) 11/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! The operations of a dead vnode (`dead_vops`): what `vclean` leaves behind when it revokes
//! a vnode from its file system. Opens fail as if the device did not exist, reads return
//! EOF (ttys) or `EIO`, most other operations `EBADF`, and the operations that may race with
//! the cleaning wait for it to finish (`chkvnlock`) and retry on the vnode's new operations.
//!
//! Upstream: sys/miscfs/deadfs/dead_vnops.c @ 3ce1f3f79392
//! LZ: sys/miscfs/deadfs/dead_vnops.rs@3c62ede68802
//!
//! ## Redesign
//! - LZ's `static DEAD_VOPS: Vops` of closures over argument structures -> [`DeadVops`], a
//!   kernel table with a fixed id (`VopsId::DEAD`, what `vclean` sets and `kind()` answers).
//!   `chkvnlock` returns `bool` (LZ's).

use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr;

use hal::sys::errno::Errno;
use hal::sys::fcntl::Flock;
use hal::sys::types::{Daddr, Register};

use crate::event::{__EV_POLL, EVFILT_EXCEPT, EVFILT_READ, EVFILT_WRITE, Knote, Kqueue};
use crate::fs::buf::B_ERROR;
use crate::fs::default;
use crate::fs::vnode::{LK_DRAIN, PINOD, VISTTY, VNODE_MTX, VXLOCK, VXWANT, Vattr, Vnode};
use crate::fs::vops::{
    Cn, Cred, IoBuf, Vnops, vop_bmap, vop_ioctl, vop_lock, vop_strategy, vop_unlock,
};
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;

/// `dead_vops`: the operations of a revoked vnode.
pub struct DeadVops<G: ProcGlue>(PhantomData<G>);

impl<G: ProcGlue> DeadVops<G> {
    /// The table.
    pub const TABLE: DeadVops<G> = DeadVops(PhantomData);
}

/// `dead_ebadf`: empty vnode failed operation.
fn dead_ebadf() -> Result<(), Errno> {
    Err(Errno::EBADF)
}

impl<G: ProcGlue> Vnops<G> for DeadVops<G> {
    fn lookup(
        &self,
        _dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        _cnp: &mut Cn<G>,
    ) -> Result<(), Errno> {
        default::vop_generic_lookup(vpp)
    }
    fn create(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn mknod(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn open(&self, _: &'static Vnode<G>, _: i32, _: Cred<'_>, _: &Thread<G>) -> Result<(), Errno> {
        dead_open()
    }
    fn close(
        &self,
        _: &'static Vnode<G>,
        _: i32,
        _: Cred<'_>,
        _: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn access(
        &self,
        _: &'static Vnode<G>,
        _: i32,
        _: Cred<'_>,
        _: &Thread<G>,
    ) -> Result<(), Errno> {
        dead_ebadf()
    }
    fn getattr(
        &self,
        _: &'static Vnode<G>,
        _: &mut Vattr,
        _: Cred<'_>,
        _: &Thread<G>,
    ) -> Result<(), Errno> {
        dead_ebadf()
    }
    fn setattr(
        &self,
        _: &'static Vnode<G>,
        _: &mut Vattr,
        _: Cred<'_>,
        _: &Thread<G>,
    ) -> Result<(), Errno> {
        dead_ebadf()
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        _: &mut Uio<'_, G>,
        _: i32,
        _: Cred<'_>,
    ) -> Result<(), Errno> {
        dead_read(vp)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        _: &mut Uio<'_, G>,
        _: i32,
        _: Cred<'_>,
    ) -> Result<(), Errno> {
        dead_write(vp)
    }
    fn ioctl(
        &self,
        vp: &'static Vnode<G>,
        command: u64,
        data: &mut [u8],
        fflag: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        // Device ioctl operation: once the vnode has finished changing, its new operations
        // answer.
        if !chkvnlock(vp) {
            return Err(Errno::EBADF);
        }
        vop_ioctl(vp, command, data, fflag, cred, p)
    }
    fn kqfilter(&self, _vp: &'static Vnode<G>, _fflag: i32, kn: &Knote<G>) -> Result<(), Errno> {
        dead_kqfilter(kn)
    }
    fn fsync(&self, _: &'static Vnode<G>, _: Cred<'_>, _: i32, _: &Thread<G>) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn remove(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn link(&self, _: &'static Vnode<G>, _: &'static Vnode<G>, _: &mut Cn<G>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn rename(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<G>,
        _: &'static Vnode<G>,
        _: Option<&'static Vnode<G>>,
        _: &mut Cn<G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn mkdir(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn rmdir(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn symlink(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<G>,
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
        dead_ebadf()
    }
    fn readlink(&self, _: &'static Vnode<G>, _: &mut Uio<'_, G>, _: Cred<'_>) -> Result<(), Errno> {
        dead_ebadf()
    }
    fn abortop(&self, _: &'static Vnode<G>, _: &mut Cn<G>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn inactive(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        // dead_inactive: nothing to do but unlock.
        let _ = vop_unlock(vp);
        Ok(())
    }
    fn reclaim(&self, _: &'static Vnode<G>, _: Option<&Thread<G>>) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn lock(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        dead_lock(vp, flags)
    }
    fn unlock(&self, _: &'static Vnode<G>) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn islocked(&self, _: &'static Vnode<G>) -> i32 {
        0 // nullop
    }
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        // Wait until the vnode has finished changing state.
        if !chkvnlock(vp) {
            return Err(Errno::EIO);
        }
        vop_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, _vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        dead_strategy::<G>(bp)
    }
    fn print(&self, _: &'static Vnode<G>) -> Result<(), Errno> {
        if cfg!(any(feature = "debug", feature = "diagnostic")) {
            crate::prf::printf(format_args!("tag VT_NON, dead vnode\n"));
        }
        Ok(())
    }
    fn pathconf(&self, _: &'static Vnode<G>, _: i32, _: &mut Register) -> Result<(), Errno> {
        dead_ebadf()
    }
    fn advlock(
        &self,
        _: &'static Vnode<G>,
        _: *const c_void,
        _: i32,
        _: &mut Flock,
        _: i32,
    ) -> Result<(), Errno> {
        dead_ebadf()
    }
    fn bwrite(&self, _bp: IoBuf) -> Result<(), Errno> {
        Ok(()) // nullop
    }
}

/// Open always fails as if device did not exist.
pub fn dead_open() -> Result<(), Errno> {
    Err(Errno::ENXIO)
}

/// Vnode op for read.
pub fn dead_read<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if chkvnlock(vp) {
        crate::assert::kpanic(format_args!("dead_read: lock"));
    }
    // Return EOF for tty devices, EIO for others
    if vp.flag() & VISTTY == 0 {
        return Err(Errno::EIO);
    }
    Ok(())
}

/// Vnode op for write.
pub fn dead_write<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if chkvnlock(vp) {
        crate::assert::kpanic(format_args!("dead_write: lock"));
    }
    Err(Errno::EIO)
}

/// `dead_kqfilter`: read, write and (poll's) except filters see the dead filter ops.
pub fn dead_kqfilter<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ | EVFILT_WRITE => kn.kn_fop.set(Some(&Kqueue::<G>::DEAD_FILTOPS)),
        EVFILT_EXCEPT => {
            if !kn.has_flags(__EV_POLL) {
                return Err(Errno::EINVAL);
            }
            kn.kn_fop.set(Some(&Kqueue::<G>::DEAD_FILTOPS));
        }
        _ => return Err(Errno::EINVAL),
    }
    Ok(())
}

/// Just call the device strategy routine (of the vnode the buffer names, once it has
/// finished changing); a buffer of a vnode that stays dead fails (`B_ERROR`, `biodone`).
pub fn dead_strategy<G: ProcGlue>(bp: IoBuf) -> Result<(), Errno> {
    let vp = bp
        .buf()
        .vnode_bufs()
        .and_then(|vb| vb.owner().as_any().downcast_ref::<Vnode<G>>());
    match vp {
        Some(vp) if chkvnlock(vp) => vop_strategy(vp, bp),
        _ => {
            bp.buf().set(B_ERROR);
            match bp.into_token() {
                Some(t) => crate::fs::bio::biodone(t),
                None => {
                    crate::assert::kpanic(format_args!("dead_strategy: a buffer with no token"))
                }
            }
            Err(Errno::EIO)
        }
    }
}

/// Wait until the vnode has finished changing state.
pub fn dead_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    if flags & LK_DRAIN != 0 || !chkvnlock(vp) {
        return Ok(());
    }
    vop_lock(vp, flags)
}

/// We have to wait during times when the vnode is in a state of change. Returns whether it
/// had to wait.
pub fn chkvnlock<G: ProcGlue>(vp: &'static Vnode<G>) -> bool {
    let mut locked = false;
    let mut g = VNODE_MTX.lock();
    while vp.vlock.get(&g).lflag & VXLOCK != 0 {
        vp.vlock.get_mut(&mut g).lflag |= VXWANT;
        let _ = g.sleep(ptr::from_ref(vp), PINOD, "chkvnlock");
        locked = true;
    }
    locked
}
/* </CODE> */
