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
 *	@(#)ffs_vnops.c	8.10 (Berkeley) 8/10/94
 */
/* </LICENSES> */

/* <CODE> */
//! The fast file system's vnode operations: the tables for files (`ffs_vops`) and for the
//! special files that live on it (`ffs_specvops`), and the operations that are FFS's own:
//! reading and writing a file through the buffer cache (`ffs_read`, `ffs_write`) and flushing
//! it (`ffs_fsync`); `ffs_reclaim` is the composed tables' own (`ufs_vnops.rs`).
//!
//! Upstream: sys/ufs/ffs/ffs_vnops.c @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/ffs_vnops.rs@3c62ede68802
//!
//! ## Deviations
//! - `option FIFO` is in GENERIC but `miscfs/fifofs` is not ported: `ffs_fifovops` and
//!   `ffsfifo_reclaim` come with it, and `ffs_vinit` refuses fifos meanwhile
//!   (`ffs_subr.rs`).
//! - `ffs_fsync`'s `LIST_FOREACH_SAFE` that restarts from the head after each write is a
//!   scan of the cache's dirty list from its head each time (`vnode_dirty_scan`); the
//!   `B_SCANNED` marks keep it from visiting a buffer twice, as in C (LZ's).
//! - `ffs_write`'s file size limit check (`vn_fsizechk`) signals the uio's thread pinned by
//!   the glue (`ProcGlue::thread_pin`); a thread the glue cannot pin (the host tests') is
//!   checked as the C checks a NULL `uio_procp`, not at all.
//!
//! ## Redesign
//! - N5.5a3: ffs's two tables (`ffs_vops`, `ffs_specvops`) are the UFS layer's composed
//!   tables (`UfsVnops<G, F>`, `UfsSpecVnops<G, F>`, `ufs_vnops.rs`), whose three slots that
//!   are the file system's own call `F: UfsFs<G>`; `ffs_reclaim`'s body is the tables'
//!   `reclaim` (D-N5.5b-8: `lineage.toml`'s `merged` row).
//! - N5.5b2: LZ's `FFS_VOPS`/`FFS_SPECVOPS` statics over `bsd`'s `FfsVtbl` -> the glue's
//!   tables' `UfsTables::ffs_vops` and `ffs_specvops` (`ufs_vfsops.rs`: a `static`'s members,
//!   one address each per glue, which the VFS's registry keys the tables by) over the
//!   kernel's [`Ffs<G>`], which implements
//!   [`UfsFs<G>`] with [`ffs_read`], [`ffs_write`] and [`ffs_fsync`]: generic functions over
//!   the kernel's vnode, `Uio` and buffer tokens (`bread`, `bread_cluster`, `UFS_BUF_ALLOC`'s
//!   token), each block's bytes moved by the glue's `uiomove` over the token's data, where LZ
//!   lent the buffers and sliced them through an `unsafe` view; LZ's `VopReadArgs`,
//!   `VopWriteArgs` and `VopFsyncArgs` packing goes (the arguments are the methods'). Not
//!   promoted constants: the two tables hold the same bytes, so their promotions were merged
//!   into one address and the device vnodes got the file table's id (`init` answered ENOTTY
//!   to `TIOCSCTTY` on `/dev/console`). The spl pairs are LZ's: `ffs_fsync`'s `splbio` around
//!   the dirty-list scan, released for each write.

use hal::intr::{IPL_BIO, splraise_raw, splx};
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::mount::{MNT_NOATIME, MNT_WAIT};
use hal::sys::stat::APPEND;

use super::ffs_vfsops::Ffs;
use super::fs::{blkoff, blksize, lblkno, lblktosize};
use crate::assert::kpanic;
use crate::event::{NOTE_EXTEND, NOTE_WRITE};
use crate::file::FileGlue;
use crate::fs::bio::{DirtyPick, DirtyScan, bawrite, bdwrite, bread, bread_cluster, brelse};
use crate::fs::bio::{bwrite, vnode_dirty_for_each, vnode_dirty_scan};
use crate::fs::buf::{B_BUSY, B_CLRBUF, B_DELWRI, B_NOCACHE, B_SCANNED, B_SYNC, Buf};
use crate::fs::subr::{vnoperm, vwaitforio};
use crate::fs::ufs::ufs::dinode::{ISGID, ISUID, NIADDR};
use crate::fs::ufs::ufs::inode::{IN_ACCESS, IN_CHANGE, IN_UPDATE, vtoi};
use crate::fs::ufs::ufs::ufs_lookup::buf_alloc;
use crate::fs::ufs::ufs::ufs_vnops::UfsFs;
use crate::fs::vnode::{
    IO_APPEND, IO_NOCACHE, IO_SYNC, IO_UNIT, VDIR, VLNK, VN_KNOTE, VREG, Vnode,
};
use crate::fs::vnops::vn_fsizechk;
use crate::fs::vops::Cred;
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

/// The slots of `ffs_vops` and `ffs_specvops` that are ffs's own.
impl<G: ProcGlue> UfsFs<G> for Ffs<G> {
    /// `vop_read`: [`ffs_read`].
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        ffs_read(vp, uio, ioflag, cred)
    }

    /// `vop_write`: [`ffs_write`].
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        ffs_write(vp, uio, ioflag, cred)
    }

    /// `vop_fsync`: [`ffs_fsync`].
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        cred: Cred<'_>,
        waitfor: i32,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ffs_fsync(vp, cred, waitfor, p)
    }
}

/// `ffs_read` (`vop_read`): vnode op for reading.
pub fn ffs_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    _ioflag: i32,
    _cred: Cred<'_>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);

    #[cfg(feature = "diagnostic")]
    {
        if uio.uio_rw != crate::uio::UioRw::UIO_READ {
            kpanic(format_args!("ffs_read: mode"));
        }

        if vp.vtype() == VLNK {
            if ip.dip_size() < u64::from(ip.ump().um_maxsymlinklen.get()) {
                kpanic(format_args!("ffs_read: short symlink"));
            }
        } else if vp.vtype() != VREG && vp.vtype() != VDIR {
            kpanic(format_args!("ffs_read: type {}", vp.vtype() as i32));
        }
    }
    let fs = ip.i_fs();
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }

    let mut error = Ok(());
    while uio.uio_resid > 0 {
        let bytesinfile = ip.dip_size() as i64 - uio.uio_offset;
        if bytesinfile <= 0 {
            break;
        }
        let lbn = lblkno(fs, uio.uio_offset);
        let nextlbn = lbn + 1;
        let mut size = i64::from(fs.fs_bsize()); // WAS blksize(fs, ip, lbn);
        let blkoffset = blkoff(fs, uio.uio_offset);
        let mut xfersize = i64::from(fs.fs_bsize()) - blkoffset;
        if (uio.uio_resid as i64) < xfersize {
            xfersize = uio.uio_resid as i64;
        }
        if bytesinfile < xfersize {
            xfersize = bytesinfile;
        }

        let (mut t, r) = if lblktosize(fs, nextlbn) >= ip.dip_size() as i64 {
            bread(&vp.bufs, lbn, size as i32)
        } else if lbn - 1 == ip.i_ci.get().ci_lastr || uio.uio_resid as i64 > xfersize {
            bread_cluster(&vp.bufs, lbn, size as i32)
        } else {
            bread(&vp.bufs, lbn, size as i32)
        };

        if let Err(e) = r {
            error = Err(e);
            brelse(t);
            break;
        }
        let mut ci = ip.i_ci.get();
        ci.ci_lastr = lbn;
        ip.i_ci.set(ci);

        // We should only get non-zero b_resid when an I/O error has occurred, which should
        // cause us to break above. However, if the short read did not cause an error, then
        // we want to ensure that we do not uiomove bad or uninitialized data.
        size -= t.buf().resid() as i64;
        if size < xfersize {
            if size == 0 {
                brelse(t);
                break;
            }
            xfersize = size;
        }
        let range = blkoffset as usize..(blkoffset + xfersize) as usize;
        let r = <G::FileGlue as FileGlue>::uiomove(&mut t.data()[range], uio);
        brelse(t);
        if let Err(e) = r {
            error = Err(e);
            break;
        }
    }
    if vp.mount().is_some_and(|mp| mp.flag() & MNT_NOATIME == 0)
        || ip.i_flag.get() & (IN_CHANGE | IN_UPDATE) != 0
    {
        ip.set_flag(IN_ACCESS);
    }
    error
}

/// `ffs_write` (`vop_write`): vnode op for writing.
pub fn ffs_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let mut extended = false;
    let ip = vtoi(vp);

    #[cfg(feature = "diagnostic")]
    if uio.uio_rw != crate::uio::UioRw::UIO_WRITE {
        kpanic(format_args!("ffs_write: mode"));
    }

    // If writing 0 bytes, succeed and do not change update time or file offset (standards
    // compliance)
    if uio.uio_resid == 0 {
        return Ok(());
    }

    match vp.vtype() {
        VREG => {
            if ioflag & IO_APPEND != 0 {
                uio.uio_offset = ip.dip_size() as i64;
            }
            if ip.dip_flags() & APPEND != 0 && uio.uio_offset != ip.dip_size() as i64 {
                return Err(Errno::EPERM);
            }
        }
        VLNK => {}
        VDIR => {
            if ioflag & IO_SYNC == 0 {
                kpanic(format_args!("ffs_write: nonsync dir write"));
            }
        }
        t => kpanic(format_args!("ffs_write: type {}", t as i32)),
    }

    let fs = ip.i_fs();
    if uio.uio_offset < 0
        || (uio.uio_offset as u64).wrapping_add(uio.uio_resid as u64) > fs.fs_maxfilesize()
    {
        return Err(Errno::EFBIG);
    }

    // do the filesize rlimit check
    let p = uio.uio_procp.and_then(G::thread_pin);
    let overrun = vn_fsizechk(vp, uio, ioflag, p)?;

    let resid = uio.uio_resid;
    let osize = ip.dip_size() as i64;
    let mut flags = if ioflag & IO_SYNC != 0 { B_SYNC } else { 0 };

    let mut error = Ok(());
    while uio.uio_resid > 0 {
        let lbn = lblkno(fs, uio.uio_offset);
        let blkoffset = blkoff(fs, uio.uio_offset);
        let mut xfersize = i64::from(fs.fs_bsize()) - blkoffset;
        if (uio.uio_resid as i64) < xfersize {
            xfersize = uio.uio_resid as i64;
        }
        if i64::from(fs.fs_bsize()) > xfersize {
            flags |= B_CLRBUF;
        } else {
            flags &= !B_CLRBUF;
        }

        let mut t = match buf_alloc(&ip, uio.uio_offset, xfersize as i32, cred, flags) {
            Ok(t) => t,
            Err(e) => {
                error = Err(e);
                break;
            }
        };
        if (uio.uio_offset + xfersize) as u64 > ip.dip_size() {
            ip.dip_set_size((uio.uio_offset + xfersize) as u64);
            uvm_vnp_setsize(vp, ip.dip_size() as i64);
            extended = true;
        }
        let _ = uvm_vnp_uncache(vp);

        let size = blksize(fs, &ip, lbn) as i64 - t.buf().resid() as i64;
        if size < xfersize {
            xfersize = size;
        }

        {
            let data = t.data();
            let range = blkoffset as usize..(blkoffset + xfersize) as usize;
            error = <G::FileGlue as FileGlue>::uiomove(&mut data[range.clone()], uio);
            // If the buffer is not already filled and we encounter an error while trying to
            // fill it, we have to clear out any garbage data from the pages instantiated for
            // the buffer. If we do not, a failed uiomove() during a write can leave the
            // prior contents of the pages exposed to a userland mmap.
            //
            // Note that we don't need to clear buffers that were allocated with the
            // B_CLRBUF flag set.
            if error.is_err() && flags & B_CLRBUF == 0 {
                data[range].fill(0);
            }
        }

        if ioflag & IO_NOCACHE != 0 {
            t.buf().set(B_NOCACHE);
        }

        if ioflag & IO_SYNC != 0 {
            let _ = bwrite(t);
        } else if xfersize + blkoffset == i64::from(fs.fs_bsize()) {
            bawrite(t);
        } else {
            bdwrite(t);
        }

        if error.is_err() || xfersize == 0 {
            break;
        }
        ip.set_flag(IN_CHANGE | IN_UPDATE);
    }
    // If we successfully wrote any data, and we are not the superuser we clear the setuid
    // and setgid bits as a precaution against tampering.
    if resid > uio.uio_resid && cred.ucred().is_some_and(|c| c.cr_uid.get() != 0) && !vnoperm(vp) {
        ip.dip_set_mode(ip.dip_mode() & !(ISUID | ISGID));
    }
    if resid > uio.uio_resid {
        VN_KNOTE(vp, NOTE_WRITE | if extended { NOTE_EXTEND } else { 0 });
    }
    if error.is_err() {
        if ioflag & IO_UNIT != 0 {
            let _ = ip.vtbl().truncate(&ip, osize, ioflag & IO_SYNC, cred);
            uio.uio_offset -= (resid - uio.uio_resid) as i64;
            uio.uio_resid = resid;
        }
    } else if resid > uio.uio_resid && ioflag & IO_SYNC != 0 {
        error = ip.vtbl().update(&ip, 1);
    }
    // correct the result for writes clamped by vn_fsizechk()
    uio.uio_resid += overrun;
    error
}

/// `ffs_fsync` (`vop_fsync`): synch an open file.
pub fn ffs_fsync<G: ProcGlue>(
    vp: &'static Vnode<G>,
    _cred: Cred<'_>,
    waitfor: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    // Flush all dirty buffers associated with a vnode.
    let mut passes = NIADDR as i32 + 1;
    let mut skipmeta = waitfor == MNT_WAIT;
    let mut s = splraise_raw(IPL_BIO);
    'loop_: loop {
        vnode_dirty_for_each(&vp.bufs, |bp| bp.clr(B_SCANNED));
        loop {
            // The cache's scan of the dirty list, from its head each time: since we may have
            // slept during the I/O, we need to start from a known point.
            let pick = |bp: &Buf| {
                // Reasons to skip this buffer: it has already been considered on this pass,
                // this pass is the first time through on a synchronous flush request and the
                // buffer being considered is metadata, the buffer has dependencies that will
                // cause it to be redirtied and it has not already been deferred, or it is
                // already being written.
                if bp.isset(B_BUSY | B_SCANNED) {
                    return DirtyPick::Skip;
                }
                if !bp.isset(B_DELWRI) {
                    kpanic(format_args!("ffs_fsync: not dirty"));
                }
                if skipmeta && bp.lblkno() < 0 {
                    return DirtyPick::Skip;
                }
                DirtyPick::Take
            };
            let DirtyScan::Taken(t) = vnode_dirty_scan(&vp.bufs, "ffs_fsync", 0, INFSLP, pick)
            else {
                break;
            };
            t.buf().set(B_SCANNED);
            splx(s);
            // On our final pass through, do all I/O synchronously so that we can find out if
            // our flush is failing because of write errors.
            if passes > 0 || waitfor != MNT_WAIT {
                bawrite(t);
            } else {
                bwrite(t)?;
            }
            s = splraise_raw(IPL_BIO);
        }
        if skipmeta {
            skipmeta = false;
            continue 'loop_;
        }
        if waitfor == MNT_WAIT {
            let _ = vwaitforio(vp, 0, "ffs_fsync", INFSLP);

            // Ensure that any filesystem metadata associated with the vnode has been
            // written.
            splx(s);
            // XXX softdep was here. reconsider this locking dance
            s = splraise_raw(IPL_BIO);
            if !vp.bufs.dirty_is_empty() {
                // Block devices associated with filesystems may have new I/O requests
                // posted for them even if the vnode is locked, so no amount of trying will
                // get them clean. Thus we give block devices a good effort, then just give
                // up. For all other file types, go around and try again until it is clean.
                if passes > 0 {
                    passes -= 1;
                    continue 'loop_;
                }
                #[cfg(feature = "diagnostic")]
                if vp.vtype() != crate::fs::vnode::VBLK {
                    crate::fs::subr::vprint(Some("ffs_fsync: dirty"), vp);
                }
            }
        }
        break;
    }
    splx(s);
    let ip = vtoi(vp);
    ip.vtbl().update(&ip, i32::from(waitfor == MNT_WAIT))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // ffs_read's and ffs_write's checks and error paths, on a file of an image mounted by
    // testffs (the kernel's ffs_mountfs). The transfers are UIO_SYSSPACE uios over the
    // tests' own buffers (testffs::attach turns the copy of kernel iovecs on); a copy that
    // fails is an iovec at BAD, which the copy double refuses with EFAULT as the machine's
    // copyin refuses an unmapped user page. No user memory is used.

    use std::assert_eq;
    use std::vec;
    use std::vec::Vec;

    use hal::sys::types::Off;

    use super::*;
    use crate::cred::Ucred;
    use crate::fs::subr::vput;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs::newfs::{FFS1_1G, FFS2_1G, Image, Params};
    use crate::fs::ufs::ffs::testffs::{new_file, pattern, setup};
    use crate::fs::ufs::ufs::dinode::IFREG;
    use crate::uio::{Iovec, UioRw, UioSeg};

    /// The image: FFS2's with `option FFS2`, FFS1's without (ffs_mountfs refuses an FFS2
    /// super-block without it, as the C's ffs_validate does).
    const ONE: Params = if cfg!(feature = "ffs2") {
        FFS2_1G
    } else {
        FFS1_1G
    };

    /// A kernel address the copy double refuses with `EFAULT`: below 4 GiB and outside its
    /// foreign run, where the host maps nothing.
    const BAD: usize = 0x1000;

    /// A kernel iovec over `b`.
    fn kiov(b: &[u8]) -> Iovec {
        Iovec {
            iov_base: b.as_ptr().cast_mut().cast(),
            iov_len: b.len(),
        }
    }

    /// An iovec of `n` bytes whose copy fails.
    fn bad(n: usize) -> Iovec {
        Iovec {
            iov_base: core::ptr::without_provenance_mut(BAD),
            iov_len: n,
        }
    }

    /// `ffs_write` of the kernel iovecs `iovs` (whose buffers outlive the call) at `off`:
    /// the result, then the uio's offset and resid after it.
    fn write(
        vp: &'static Vnode<TestGlue>,
        iovs: &mut [Iovec],
        off: Off,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> (Result<(), Errno>, Off, usize) {
        let resid = iovs.iter().map(|v| v.iov_len).sum();
        let mut uio = Uio {
            uio_iov: iovs,
            uio_offset: off,
            uio_resid: resid,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_WRITE,
            uio_procp: None,
        };
        let r = ffs_write(vp, &mut uio, ioflag, cred);
        (r, uio.uio_offset, uio.uio_resid)
    }

    /// The file's `n` bytes at `off`, read by `ffs_read`.
    fn read(vp: &'static Vnode<TestGlue>, off: Off, n: usize) -> Vec<u8> {
        let mut out = vec![0u8; n];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut out, None);
        uio.uio_offset = off;
        assert_eq!(ffs_read(vp, &mut uio, 0, Cred::NOCRED), Ok(()));
        assert_eq!(uio.uio_resid, 0);
        out
    }

    #[test]
    fn a_read_at_a_negative_offset_is_einval() {
        let (_g, th, mp) = setup(Image::new(ONE).finish());
        let vp = new_file(mp, th);
        let mut out = [0u8; 8];
        for len in [8, 0] {
            let mut iov = [Iovec::new()];
            let mut uio = Uio::kernel_read(&mut iov, &mut out[..len], None);
            uio.uio_offset = -1;
            // Checked before the empty transfer's early return, as in C.
            assert_eq!(ffs_read(vp, &mut uio, 0, Cred::NOCRED), Err(Errno::EINVAL));
            assert_eq!((uio.uio_offset, uio.uio_resid), (-1, len));
        }
        // At 0, an empty file reads nothing and succeeds.
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut out, None);
        assert_eq!(ffs_read(vp, &mut uio, 0, Cred::NOCRED), Ok(()));
        assert_eq!((uio.uio_offset, uio.uio_resid), (0, 8));
        vput(vp);
    }

    #[test]
    fn a_write_past_the_largest_file_is_efbig() {
        let (_g, th, mp) = setup(Image::new(ONE).finish());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        let max = ip.i_fs().fs_maxfilesize();
        let b = [1u8; 2];
        let cred = Cred::real(th.ucred());
        // Ending one byte past fs_maxfilesize, and at a negative offset.
        for off in [(max - 1) as Off, -1] {
            assert_eq!(
                write(vp, &mut [kiov(&b)], off, 0, cred),
                (Err(Errno::EFBIG), off, 2)
            );
        }
        assert_eq!(ip.dip_size(), 0);
        assert_eq!(ip.dip_blocks(), 0);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn an_append_only_file_is_written_at_its_end_only() {
        let (_g, th, mp) = setup(Image::new(ONE).finish());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        let cred = Cred::real(th.ucred());
        let b = pattern(100, 3);
        assert_eq!(write(vp, &mut [kiov(&b)], 0, 0, cred), (Ok(()), 100, 0));
        ip.dip_set_flags(ip.dip_flags() | APPEND);
        // Anywhere but at its size: EPERM, nothing written (even by root).
        for off in [0, 50, 101] {
            assert_eq!(
                write(vp, &mut [kiov(&b)], off, 0, cred),
                (Err(Errno::EPERM), off, 100)
            );
        }
        assert_eq!(ip.dip_size(), 100);
        // IO_APPEND moves the offset to the size first; at the size it is allowed.
        assert_eq!(
            write(vp, &mut [kiov(&b)], 0, IO_APPEND, cred),
            (Ok(()), 200, 0)
        );
        assert_eq!(write(vp, &mut [kiov(&b)], 200, 0, cred), (Ok(()), 300, 0));
        assert_eq!(ip.dip_size(), 300);
        assert_eq!(read(vp, 100, 100), &b[..]);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn io_unit_truncates_and_rewinds_a_failed_write() {
        let (_g, th, mp) = setup(Image::new(ONE).finish());
        let cred = Cred::real(th.ucred());
        for ioflag in [0, IO_UNIT] {
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let bsize = ip.i_fs().fs_bsize() as usize;
            let data = pattern(bsize, 5);
            // A whole block copied, then a copy that fails in the second block.
            let r = write(vp, &mut [kiov(&data), bad(10)], 0, ioflag, cred);
            if ioflag == 0 {
                // What was moved stays, and the size the second block's pass had set.
                assert_eq!(r, (Err(Errno::EFAULT), bsize as Off, 10));
                assert_eq!(ip.dip_size(), bsize as u64 + 10);
                assert_eq!(read(vp, 0, bsize), &data[..]);
            } else {
                // The file is truncated back to its size before the write (its blocks
                // freed) and the uio rewound as if nothing had been moved.
                assert_eq!(r, (Err(Errno::EFAULT), 0, bsize + 10));
                assert_eq!(ip.dip_size(), 0);
                assert_eq!(ip.dip_blocks(), 0);
            }
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn a_writer_other_than_root_clears_setuid_and_setgid() {
        let (_g, th, mp) = setup(Image::new(ONE).finish());
        let mut user = Ucred::new();
        user.cr_uid.set(1000);
        let root = Ucred::new();
        let b = pattern(10, 1);
        let mode = IFREG | ISUID | ISGID | 0o755;
        for (cred, kept) in [(&root, true), (&user, false)] {
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            ip.dip_set_mode(mode);
            // Writing nothing returns first: the bits stay whoever writes.
            assert_eq!(
                write(vp, &mut [kiov(&b[..0])], 0, 0, Cred::real(cred)),
                (Ok(()), 0, 0)
            );
            assert_eq!(ip.dip_mode(), mode);
            assert_eq!(
                write(vp, &mut [kiov(&b)], 0, 0, Cred::real(cred)),
                (Ok(()), 10, 0)
            );
            let want = if kept { mode } else { IFREG | 0o755 };
            assert_eq!(ip.dip_mode(), want);
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn a_failed_copy_into_a_whole_block_zeroes_it() {
        let (_g, th, mp) = setup(Image::new(ONE).finish());
        let cred = Cred::real(th.ucred());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        let bsize = ip.i_fs().fs_bsize() as usize;
        let old = pattern(bsize, 9);
        // A part of a block (B_CLRBUF: the block is read in first): a failed copy leaves the
        // file's bytes as they were.
        assert_eq!(
            write(vp, &mut [kiov(&old)], 0, 0, cred),
            (Ok(()), bsize as Off, 0)
        );
        assert_eq!(
            write(vp, &mut [bad(100)], 0, 0, cred),
            (Err(Errno::EFAULT), 0, 100)
        );
        assert_eq!(read(vp, 0, bsize), &old[..]);
        // A whole block (no B_CLRBUF): what the buffer held is cleared where the copy
        // failed, so no earlier bytes of its pages survive, and the size is kept.
        assert_eq!(
            write(vp, &mut [bad(bsize)], 0, 0, cred),
            (Err(Errno::EFAULT), 0, bsize)
        );
        assert_eq!(read(vp, 0, bsize), vec![0u8; bsize]);
        assert_eq!(ip.dip_size(), bsize as u64);
        drop(ip);
        vput(vp);
    }
}
/* </TESTS> */
