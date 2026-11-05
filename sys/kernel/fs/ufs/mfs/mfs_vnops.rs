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
 *	@(#)mfs_vnops.c	8.5 (Berkeley) 7/28/94
 */
/* </LICENSES> */

/* <CODE> */
//! The memory based file system's vnode operations: `mfs_vops` (the operations of its
//! "device" vnode) and `mfs_open`, `mfs_ioctl`, `mfs_strategy`, `mfs_doio`, `mfs_close`,
//! `mfs_inactive`, `mfs_reclaim` and `mfs_print`.
//!
//! Upstream: sys/ufs/mfs/mfs_vnops.c @ 3ce1f3f79392
//! LZ: sys/ufs/mfs/mfs_vnops.rs@3c62ede68802
//!
//! The I/O model: the file system lives in the address space of the process that called
//! `mount(2)` (`mount_mfs(8)`'s child), which stays inside `mfs_start`. `mfs_strategy` queues
//! a request on the mfsnode and wakes that process, which copies the data in or out with
//! `mfs_doio`; when the process itself asks (its own mount, or the unmount it makes on a
//! signal), the copy is done at once.
//!
//! ## Deviations
//! - `mfs_print` prints under features `diagnostic` or `debug` (the C's
//!   `DEBUG || DIAGNOSTIC`); without them it does nothing, as the C. `VFSLCKDEBUG` is not
//!   configured.
//! - The queue is the node's own fifo, without the registry of disk queues
//!   (`mfsnode.rs`'s deviations): `mfs_reclaim` fails what is still queued with `ENXIO`, as
//!   `bufq_destroy`'s drain does.
//!
//! ## Redesign
//! - LZ's `static MFS_VOPS: Vops` of function pointers over argument structures ->
//!   [`MfsVnops`], `impl Vnops<G>` with the C's slots: `open`, `close`, `ioctl`, `inactive`,
//!   `reclaim`, `strategy`, `print` are this module's functions; `revoke`, `fsync`
//!   (`spec_fsync`), `bmap` and `bwrite` the generic ones; `lock`, `unlock` and `islocked`
//!   the C's `nullop`; every other slot `vop_generic_badop`, as the C's table names them. The
//!   table is a member of the glue's `UfsTables` (`mfs_vops`), whose address the VFS's
//!   registry keys it by (`vops_id`; a promoted constant has no address of its own,
//!   docs/IDIOMS.md).
//! - `mfs_strategy` takes the request's token (`IoBuf`) and either does the I/O at once or
//!   parks the token in the buffer on the node's queue for `mfs_start`; `mfs_doio` copies
//!   between the token's bytes (`BufRef::data`, `b_bcount` of them) and the user memory with
//!   `hal::user::UserPtr`, where LZ took the buffer's data as an `unsafe` slice of a lent
//!   `&'static Buf`.
//! - `mfs_doio` closes LZ's deviation for a request that starts past the end of the file
//!   system: the clamp leaves `b_bcount` negative, as the C's, and the transfer fails
//!   `EFAULT`, as the C's `copyin`/`copyout` of that length (an overflowing range) does; LZ
//!   moved no bytes and reported success. A request that starts at the end moves none and
//!   succeeds, as the C's.
//! - Cost against the C: `VTOMFS` is a count pair per request (`mfsnode.rs`); `mfs_doio`'s
//!   `splbio`/`splx` pair around `biodone` is the C's.

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::intr::{IPL_BIO, splraise};
use hal::sleep::{INFSLP, wakeup};
use hal::sys::errno::Errno;
use hal::sys::fcntl::Flock;
use hal::sys::types::{Daddr, Register};
use hal::user::UserPtr;

use super::mfsnode::{Mfsnode, vtomfs};
use crate::assert::kpanic;
use crate::event::Knote;
use crate::file::LockOwner;
use crate::fs::bio::{DEV_BSHIFT, biodone};
use crate::fs::buf::{B_ERROR, B_READ, BufRef};
use crate::fs::default;
use crate::fs::spec::spec_fsync;
use crate::fs::subr::vinvalbuf;
use crate::fs::vnode::{V_SAVE, VBLK, Vattr, Vnode};
use crate::fs::vops::{Cn, Cred, IoBuf, Vnops, vop_unlock};
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;

/// `mfs_vops`: mfs vnode operations (the glue's `UfsTables::mfs_vops`, one address).
pub struct MfsVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member: the registry tells tables apart by
    /// their addresses.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> MfsVnops<G> {
    /// The table.
    pub const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

impl<G: ProcGlue> Default for MfsVnops<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `mfs_open` (`vop_open`): open called to allow memory filesystem to initialize and validate
/// before actual IO. Record our process identifier so we can tell when we are doing I/O to
/// ourself.
pub fn mfs_open<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if vp.vtype() != VBLK {
        kpanic(format_args!("mfs_open not VBLK"));
    }
    let _ = vp;
    Ok(())
}

/// `mfs_ioctl` (`vop_ioctl`): ioctl operation.
pub fn mfs_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `mfs_strategy` (`vop_strategy`): pass I/O requests to the memory filesystem process.
pub fn mfs_strategy<G: ProcGlue>(vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
    if vp.vtype() != VBLK || vp.usecount() == 0 {
        kpanic(format_args!("mfs_strategy: bad dev"));
    }

    let mfsp = vtomfs(vp);
    let Some(t) = bp.into_token() else {
        kpanic(format_args!("mfs_strategy: a buffer with no token"));
    };
    if G::with_curproc(|p| p.is_some_and(|p| p.p_tid.get() == mfsp.mfs_tid)) {
        mfs_doio(&mfsp, t);
    } else {
        mfsp.queue(t);
        wakeup(ptr::from_ref(vp));
    }
    Ok(())
}

/// `mfs_doio`: memory file system I/O. Runs in the file system's process (or in the closer's,
/// `mfs_close`): `mfs_baseoff` is an address in its memory, which `copyin` (a read of the file
/// system) and `copyout` (a write) reach.
pub fn mfs_doio<G: ProcGlue>(mfsp: &Mfsnode<G>, mut t: BufRef) {
    let offset = t.buf().blkno() << DEV_BSHIFT;

    if t.buf().bcount() > mfsp.mfs_size - offset {
        t.buf().set_bcount(mfsp.mfs_size - offset);
    }

    let base = UserPtr::<u8>::new(mfsp.mfs_baseoff.wrapping_add(offset as usize));
    // A negative count (a request past the end) is the C's copy of an overflowing length.
    let error = if t.buf().bcount() < 0 {
        Err(Errno::EFAULT)
    } else if t.buf().bcount() == 0 {
        Ok(()) // the C's copy of no bytes, wherever they would start
    } else if t.buf().isset(B_READ) {
        base.copyin_slice(t.data())
    } else {
        base.copyout_slice(t.bytes())
    };
    t.buf().set_error(error.err());
    if error.is_err() {
        t.buf().set(B_ERROR);
    } else {
        t.buf().set_resid(0);
    }
    let s = splraise(IPL_BIO);
    biodone(t);
    drop(s);
}

/// The requests queued on `mfsp`, each done by [`mfs_doio`] and its sleeper woken
/// (`wakeup(bp)`): `mfs_start`'s and `mfs_close`'s loop.
pub(super) fn mfs_serve<G: ProcGlue>(mfsp: &Mfsnode<G>, stop: impl Fn() -> bool) {
    while !stop() {
        let Some(t) = mfsp.dequeue() else {
            break;
        };
        let bp = ptr::from_ref(t.buf());
        mfs_doio(mfsp, t);
        wakeup(bp);
    }
}

/// `mfs_close` (`vop_close`): memory filesystem close routine.
pub fn mfs_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let mfsp = vtomfs(vp);

    // Finish any pending I/O requests.
    mfs_serve(&mfsp, || false);

    // On last close of a memory filesystem we must invalidate any in core blocks, so that we
    // can free up its vnode.
    vinvalbuf(vp, V_SAVE, cred, p, 0, INFSLP)?;

    // There should be no way to have any more buffers on this vnode.
    #[cfg(feature = "diagnostic")]
    if mfsp.queued() {
        crate::prf::printf(format_args!("mfs_close: dirty buffers\n"));
    }

    // Send a request to the filesystem server to exit.
    mfsp.mfs_shutdown.store(1, Ordering::Relaxed);
    wakeup(ptr::from_ref(vp));
    Ok(())
}

/// `mfs_inactive` (`vop_inactive`): memory filesystem inactive routine.
pub fn mfs_inactive<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    {
        let mfsp = vtomfs(vp);
        if mfsp.mfs_shutdown.load(Ordering::Relaxed) != 0 && mfsp.queued() {
            kpanic(format_args!("mfs_inactive: not inactive"));
        }
    }
    let _ = vop_unlock(vp);
    Ok(())
}

/// `mfs_reclaim` (`vop_reclaim`): reclaim a memory filesystem devvp so that it can be reused.
/// The node leaves the vnode's slot; the block is freed with the last handle (`mfs_start`'s,
/// when the file system's process has not returned yet).
pub fn mfs_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let mfsp = vtomfs(vp);

    mfsp.drain();

    drop(vp.take_node());
    drop(mfsp);
    Ok(())
}

/// `mfs_print` (`vop_print`): print out the contents of an mfsnode.
pub fn mfs_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(any(feature = "debug", feature = "diagnostic"))]
    {
        let mfsp = vtomfs(vp);

        crate::prf::printf(format_args!(
            "tag VT_MFS, tid {}, base {:#x}, size {}\n",
            mfsp.mfs_tid, mfsp.mfs_baseoff, mfsp.mfs_size
        ));
    }
    let _ = vp;
    Ok(())
}

/// `mfs_vops`: each slot the C's.
impl<G: ProcGlue> Vnops<G> for MfsVnops<G> {
    fn lookup(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
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
        vp: &'static Vnode<G>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        mfs_open(vp)
    }
    fn close(
        &self,
        vp: &'static Vnode<G>,
        _fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        mfs_close(vp, cred, p)
    }
    fn access(
        &self,
        _: &'static Vnode<G>,
        _: i32,
        _: Cred<'_>,
        _: &Thread<G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn getattr(
        &self,
        _: &'static Vnode<G>,
        _: &mut Vattr,
        _: Cred<'_>,
        _: &Thread<G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn setattr(
        &self,
        _: &'static Vnode<G>,
        _: &mut Vattr,
        _: Cred<'_>,
        _: &Thread<G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn read(
        &self,
        _: &'static Vnode<G>,
        _: &mut Uio<'_, G>,
        _: i32,
        _: Cred<'_>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn write(
        &self,
        _: &'static Vnode<G>,
        _: &mut Uio<'_, G>,
        _: i32,
        _: Cred<'_>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
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
        mfs_ioctl()
    }
    fn kqfilter(&self, _: &'static Vnode<G>, _: i32, _: &Knote<G>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        _cred: Cred<'_>,
        waitfor: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        spec_fsync(vp, waitfor)
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
    fn inactive(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        mfs_inactive(vp)
    }
    fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        mfs_reclaim(vp)
    }
    fn lock(&self, _vp: &'static Vnode<G>, _flags: i32) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn unlock(&self, _vp: &'static Vnode<G>) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn islocked(&self, _vp: &'static Vnode<G>) -> i32 {
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
        default::vop_generic_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        mfs_strategy(vp, bp)
    }
    fn print(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        mfs_print(vp)
    }
    fn pathconf(&self, _: &'static Vnode<G>, _: i32, _: &mut Register) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn advlock(
        &self,
        _: &'static Vnode<G>,
        _: LockOwner,
        _: i32,
        _: &mut Flock,
        _: i32,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        default::vop_generic_bwrite(bp)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the memory file system's I/O over a device vnode made as `mfs_mount`
    // makes it, its memory a run of the copy double's user memory: `mfs_doio` through the
    // buffer cache (reads, writes, the clamp at the end of the file system, a request past
    // the end, a bad address), `mfs_strategy`'s two paths (the file system's own process does
    // the I/O at once, any other queues the request for it), the queue's drains (`mfs_close`
    // serves it, `mfs_reclaim` fails it) and the node's free at reclaim. The cross-thread path
    // (another process sleeping in `biowait` while `mfs_start` serves it) runs in the smoke
    // (D-N5.5b-6).

    extern crate std;

    use std::vec::Vec;

    use hal::sys::types::makedev;

    use super::super::mfs_vfsops::fixture::{UIMAGE, attach, image};
    use super::super::mfsnode::{M_MFSNODE, MfsnodeRef};
    use super::*;
    use crate::fs::bio::{bread, brelse, bwrite, getblk};
    use crate::fs::buf::{B_ASYNC, B_DONE};
    use crate::fs::subr::{checkalias, getnewvnode, vfs};
    use crate::fs::testfs::TestGlue;
    use crate::fs::vnode::VT_MFS;
    use crate::fs::vops::{vop_ioctl, vop_open, vop_print, vop_strategy};
    use crate::testutil::malloc_out;

    /// The memory: 16 KiB of a pattern.
    const LEN: usize = 16 << 10;

    fn memory() -> Vec<u8> {
        (0..LEN as u32).map(|i| (i % 251) as u8).collect()
    }

    /// A device vnode as `mfs_mount` makes one (minor `minor`), its node over the [`LEN`]
    /// bytes at [`UIMAGE`] of the thread `tid`: the vnode and a handle to the node.
    fn devvp(minor: u32, tid: i32) -> (&'static Vnode<TestGlue>, MfsnodeRef<TestGlue>) {
        let vp = getnewvnode(VT_MFS, None, &vfs::<TestGlue>().ufs.mfs_vops).unwrap();
        vp.set_vtype(VBLK);
        assert!(checkalias(vp, makedev(255, minor), None).is_none());
        let mfsp = Mfsnode::new(vp, UIMAGE, LEN as i64, tid);
        vp.set_node(hal::malloc::MArc::erase(mfsp.clone()));
        (vp, mfsp)
    }

    #[test]
    fn mfs_doio_reads_and_writes_the_memory() {
        let mem = memory();
        let (_g, p) = attach(&mem);
        let (vp, _mfsp) = devvp(900, p.p_tid.get());

        // A read of the third sector: the bytes at offset 1024, done at once (our process).
        let (t, r) = bread(&vp.bufs, 2, 1024);
        assert_eq!(r, Ok(()));
        assert!(t.buf().isset(B_DONE));
        assert_eq!(t.buf().resid(), 0);
        assert_eq!(t.bytes(), &mem[1024..2048]);
        brelse(t);

        // A write of the last sector goes to offset 15872 and leaves the rest alone.
        let mut t = getblk(&vp.bufs, 31, 512, 0, INFSLP).unwrap();
        t.data().fill(0xa5);
        assert_eq!(bwrite(t), Ok(()));
        let now = image(LEN);
        assert!(now[15872..].iter().all(|&b| b == 0xa5));
        assert_eq!(&now[..15872], &mem[..15872]);
    }

    #[test]
    fn mfs_doio_clamps_at_the_end_and_fails_past_it() {
        let mem = memory();
        let (_g, p) = attach(&mem);
        let (vp, mfsp) = devvp(901, p.p_tid.get());

        // 2048 bytes asked at offset 15360: only 1024 are left, and b_bcount says so.
        let (t, r) = bread(&vp.bufs, 30, 2048);
        assert_eq!(r, Ok(()));
        assert_eq!(t.buf().bcount(), 1024);
        assert_eq!(t.bytes(), &mem[15360..]);
        t.buf().set(crate::fs::buf::B_INVAL);
        brelse(t);

        // A request at the very end moves nothing and succeeds, as the C's copy of 0 bytes.
        let (t, r) = bread(&vp.bufs, 32, 512);
        assert_eq!(r, Ok(()));
        assert_eq!(t.buf().bcount(), 0);
        t.buf().set(crate::fs::buf::B_INVAL);
        brelse(t);

        // Past the end: the count goes negative, as the C's, and the copy of that length
        // faults.
        let (t, r) = bread(&vp.bufs, 40, 512);
        assert_eq!(r, Err(Errno::EFAULT));
        assert!(t.buf().isset(B_ERROR));
        assert!(t.buf().bcount() < 0);
        t.buf().set(crate::fs::buf::B_INVAL);
        brelse(t);
        drop(mfsp);
    }

    #[test]
    fn mfs_doio_reports_a_bad_address() {
        let (_g, p) = attach(&memory());
        let vp = getnewvnode(VT_MFS, None, &vfs::<TestGlue>().ufs.mfs_vops).unwrap();
        vp.set_vtype(VBLK);
        assert!(checkalias(vp, makedev(255, 902), None).is_none());
        // The process's memory is gone: no such address (below the copy double's).
        let mfsp = Mfsnode::new(vp, 0x1000, LEN as i64, p.p_tid.get());
        vp.set_node(hal::malloc::MArc::erase(mfsp.clone()));
        let (t, r) = bread(&vp.bufs, 0, 512);
        assert_eq!(r, Err(Errno::EFAULT));
        assert!(t.buf().isset(B_ERROR));
        assert!(t.buf().isset(B_DONE));
        t.buf().set(crate::fs::buf::B_INVAL);
        brelse(t);
    }

    #[test]
    fn another_process_s_request_waits_on_the_queue_until_the_server_does_it() {
        let mem = memory();
        let (_g, p) = attach(&mem);
        // The file system's process is another one.
        let (vp, mfsp) = devvp(903, p.p_tid.get() + 1000);

        // An asynchronous read: queued, and nothing is copied until the queue is served.
        let t = getblk(&vp.bufs, 4, 1024, 0, INFSLP).unwrap();
        t.buf().set(B_READ | B_ASYNC);
        vop_strategy(vp, IoBuf::Token(t)).unwrap();
        assert!(mfsp.queued());

        // The server's loop (what `mfs_start` does on the wakeup): the request done, the
        // buffer back in the cache with the bytes.
        mfs_serve(&mfsp, || false);
        assert!(!mfsp.queued());
        let t = getblk(&vp.bufs, 4, 1024, 0, INFSLP).unwrap();
        assert!(t.buf().isset(B_DONE));
        assert_eq!(t.bytes(), &mem[2048..3072]);
        brelse(t);

        // mfs_close serves what is queued (the C's "finish any pending I/O requests"), then
        // tells the server to exit.
        let t = getblk(&vp.bufs, 8, 512, 0, INFSLP).unwrap();
        t.buf().set(B_READ | B_ASYNC);
        vop_strategy(vp, IoBuf::Token(t)).unwrap();
        assert!(mfsp.queued());
        mfs_close(vp, Cred::NOCRED, Some(p)).unwrap();
        assert!(!mfsp.queued());
        assert!(mfsp.shutdown());
    }

    #[test]
    fn reclaim_fails_what_is_queued_and_frees_the_node() {
        let (_g, p) = attach(&memory());
        let before = malloc_out(M_MFSNODE);
        let (vp, mfsp) = devvp(904, p.p_tid.get() + 1000);
        assert!(malloc_out(M_MFSNODE) > before);

        let t = getblk(&vp.bufs, 1, 512, 0, INFSLP).unwrap();
        t.buf().set(B_READ | B_ASYNC);
        mfsp.queue(t);
        drop(mfsp);

        // The reclaim drains the queue (the request fails ENXIO, as bufq_destroy's drain) and
        // empties the slot; the last handle frees the block.
        mfs_reclaim(vp).unwrap();
        assert!(!vp.has_node());
        assert_eq!(malloc_out(M_MFSNODE), before);
        // The failed read left nothing valid in the cache.
        let t = getblk(&vp.bufs, 1, 512, 0, INFSLP).unwrap();
        assert!(!t.buf().isset(B_DONE));
        t.buf().set(crate::fs::buf::B_INVAL);
        brelse(t);
    }

    #[test]
    fn the_other_slots_are_the_c_s() {
        let (_g, p) = attach(&memory());
        let (vp, _mfsp) = devvp(905, p.p_tid.get());
        let mut data = [0u8; 4];
        assert_eq!(
            vop_ioctl(vp, 0, &mut data, 0, Cred::NOCRED, p),
            Err(Errno::ENOTTY)
        );
        assert_eq!(vop_open(vp, 0, Cred::NOCRED, p), Ok(()));
        assert_eq!(vop_print(vp), Ok(()));
        let ops = &vfs::<TestGlue>().ufs.mfs_vops;
        assert_eq!(ops.islocked(vp), 0);
        assert_eq!(ops.lock(vp, 0), Ok(()));
        assert_eq!(ops.unlock(vp), Ok(()));
        let mut bn = 0;
        assert_eq!(ops.bmap(vp, 7, None, Some(&mut bn), None), Ok(()));
        assert_eq!(bn, 7);
    }

    #[test]
    #[should_panic(expected = "badop")]
    fn a_slot_the_c_leaves_to_badop_panics() {
        let (_g, p) = attach(&memory());
        let (vp, _mfsp) = devvp(906, p.p_tid.get());
        let mut vap = Vattr::default();
        let _ = vfs::<TestGlue>()
            .ufs
            .mfs_vops
            .getattr(vp, &mut vap, Cred::NOCRED, p);
    }
}
/* </TESTS> */
