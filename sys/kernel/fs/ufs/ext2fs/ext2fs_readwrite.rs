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
 * Copyright (c) 1993
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
 *	@(#)ufs_readwrite.c	8.8 (Berkeley) 8/4/94
 * Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! ext2fs file reading and writing through the buffer cache: `ext2fs_read` (by the block
//! pointers, `ext2_ind_read`, or by the extent tree, `ext4_ext_read`) and `ext2fs_write`.
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_readwrite.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_readwrite.rs@3c62ede68802
//!
//! ## Deviations
//! - `ext4_ext_read` looks each block up with a fresh path, and releases the leaf buffer
//!   `ext4_ext_find_extent` may hold also when no extent covers the block (the C returns
//!   `EIO` with the buffer still busy). The C's path is reused across iterations, and only
//!   its `ep_bp`, released after each lookup, carries over.
//! - `ext2_ind_read`'s read-ahead is `breadn` with the next block, as in C.
//! - When `ext2fs_setsize` refuses the new size, `ext2fs_write` releases the block's buffer
//!   before it stops; the C leaves it busy.
//! - `ext2fs_write`'s file size limit check (`vn_fsizechk`) signals the uio's thread pinned by
//!   the glue (`ProcGlue::thread_pin`), as the kernel's `ffs_write`; a thread the glue cannot
//!   pin (the host tests') is checked as the C checks a NULL `uio_procp`, not at all.
//! - The setuid and setgid bits are cleared for a real credential that is not root's
//!   (`a_cred && a_cred->cr_uid != 0`); `NOCRED` and `FSCRED`, which the C would follow, clear
//!   nothing.
//!
//! ## Redesign
//! - N5.6c2: LZ's functions over `bsd`'s `VopReadArgs`/`VopWriteArgs` and lent buffers
//!   (`&'static Buf`, their bytes through an `unsafe` `BufLegacy::data`, three sites) ->
//!   generic functions with the kernel's `Vnops<G>` method shapes (`ext2fs_read(vp, uio,
//!   ioflag, cred)`), the kernel's `ffs_read`/`ffs_write` shape: the blocks are the buffer
//!   cache's tokens (`bread`, `breadn` and `ext2fs_buf_alloc`'s), each block's bytes moved by
//!   the glue's `uiomove` over the token's data, then `brelse`, `bwrite`, `bawrite` or
//!   `bdwrite` of the token. The inode is one `vtoi` handle per operation and the
//!   super-block is borrowed from it (`ip.i_e2fs()`).
//! - Cost against the C: the same reads and writes of the same blocks, in the same order;
//!   no lock, no spl.

use hal::sys::errno::Errno;
use hal::sys::mount::MNT_NOATIME;
use hal::sys::types::{Daddr, Off};

use super::ext2fs::{Ext2Superblock, blkoff, e2fs_overflow, fsbtodb, lblkno, lblktosize};
use super::ext2fs_balloc::ext2fs_buf_alloc;
use super::ext2fs_dinode::{EXT2_APPEND, EXT4_EXTENTS};
use super::ext2fs_extents::{
    EXT4_EXT_CACHE_GAP, EXT4_EXT_CACHE_IN, EXT4_EXT_CACHE_NO, Ext4Extent, Ext4ExtentPath,
    ext4_ext_find_extent, ext4_ext_in_cache, ext4_ext_put_cache,
};
use super::ext2fs_inode::{ext2fs_setsize, ext2fs_size, ext2fs_truncate, ext2fs_update};
use crate::assert::kpanic;
use crate::event::{NOTE_EXTEND, NOTE_WRITE};
use crate::file::FileGlue;
use crate::fs::bio::{bawrite, bdwrite, bread, breadn, brelse, bwrite};
use crate::fs::buf::{B_CLRBUF, B_NOCACHE, B_SYNC};
use crate::fs::ufs::ufs::dinode::{ISGID, ISUID};
use crate::fs::ufs::ufs::inode::{IN_ACCESS, IN_CHANGE, IN_UPDATE, Inode, vtoi};
use crate::fs::vnode::Vnode;
use crate::fs::vnode::{IO_APPEND, IO_NOCACHE, IO_SYNC, IO_UNIT, VDIR, VLNK, VN_KNOTE, VREG};
use crate::fs::vnops::vn_fsizechk;
use crate::fs::vops::Cred;
use crate::proc::ProcGlue;
use crate::uio::Uio;
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

/// `ext2fs_read` (`vop_read`): vnode op for reading.
pub fn ext2fs_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    _ioflag: i32,
    _cred: Cred<'_>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fs = ip.i_e2fs();

    if ip.i_e2din().i_e2fs_flags() & EXT4_EXTENTS != 0 {
        ext4_ext_read(vp, &ip, fs, uio)
    } else {
        ext2_ind_read(vp, &ip, fs, uio)
    }
}

/// `ext2_ind_read`: read a file mapped by its block pointers.
fn ext2_ind_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    ip: &Inode<G>,
    fs: &Ext2Superblock,
    uio: &mut Uio<'_, G>,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    {
        if uio.uio_rw != crate::uio::UioRw::UIO_READ {
            kpanic(format_args!("ext2fs_read: mode"));
        }

        if vp.vtype() == VLNK {
            if ext2fs_size(ip) < super::ext2fs_dinode::EXT2_MAXSYMLINKLEN as u64 {
                kpanic(format_args!("ext2fs_read: short symlink"));
            }
        } else if vp.vtype() != VREG && vp.vtype() != VDIR {
            kpanic(format_args!("ext2fs_read: type {}", vp.vtype() as i32));
        }
    }
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }

    let bsize = i64::from(fs.e2fs_bsize.get());
    let mut error = Ok(());
    while uio.uio_resid > 0 {
        let bytesinfile = ext2fs_size(ip) as Off - uio.uio_offset;
        if bytesinfile <= 0 {
            break;
        }
        let lbn = lblkno(fs, uio.uio_offset);
        let nextlbn = lbn + 1;
        let mut size = bsize;
        let blkoffset = blkoff(fs, uio.uio_offset);
        let mut xfersize = bsize - blkoffset;
        if (uio.uio_resid as i64) < xfersize {
            xfersize = uio.uio_resid as i64;
        }
        if bytesinfile < xfersize {
            xfersize = bytesinfile;
        }

        let (mut t, r) = if lblktosize(fs, nextlbn) >= ext2fs_size(ip) as Off {
            bread(&vp.bufs, lbn, size as i32)
        } else if lbn - 1 == ip.i_ci.get().ci_lastr {
            let nextsize = fs.e2fs_bsize.get();
            breadn(&vp.bufs, lbn, size as i32, &[nextlbn], &[nextsize])
        } else {
            bread(&vp.bufs, lbn, size as i32)
        };
        if let Err(e) = r {
            // The C breaks with bp set, and the brelse after the loop releases it.
            brelse(t);
            error = Err(e);
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

    if vp.mount().is_some_and(|mp| mp.flag() & MNT_NOATIME == 0) {
        ip.set_flag(IN_ACCESS);
    }
    error
}

/// `ext4_ext_read`: read a file mapped by its extent tree.
pub fn ext4_ext_read<G: ProcGlue>(
    _vp: &'static Vnode<G>,
    ip: &Inode<G>,
    fs: &Ext2Superblock,
    uio: &mut Uio<'_, G>,
) -> Result<(), Errno> {
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }

    while uio.uio_resid > 0 {
        let bytesinfile = ext2fs_size(ip) as Off - uio.uio_offset;
        if bytesinfile <= 0 {
            break;
        }
        let lbn = lblkno(fs, uio.uio_offset);
        let mut size = i64::from(fs.e2fs_bsize.get());
        let blkoffset = blkoff(fs, uio.uio_offset);

        let mut xfersize = i64::from(fs.e2fs_fsize.get()) - blkoffset;
        xfersize = xfersize.min(uio.uio_resid as i64);
        xfersize = xfersize.min(bytesinfile);

        let mut nex = Ext4Extent::default();
        let cache_type = ext4_ext_in_cache(ip, lbn, &mut nex);
        let pos: Daddr = match cache_type {
            EXT4_EXT_CACHE_NO => {
                let mut path = Ext4ExtentPath::new();
                let ep = ext4_ext_find_extent(fs, ip, lbn, &mut path).and_then(|p| p.ext());
                if let Some(t) = path.ep_bp.take() {
                    brelse(t);
                }
                let Some(ep) = ep else {
                    return Err(Errno::EIO);
                };
                ext4_ext_put_cache(ip, &ep, EXT4_EXT_CACHE_IN);

                lbn - Daddr::from(ep.e_blk)
                    + ((Daddr::from(ep.e_start_hi) << 32) | Daddr::from(ep.e_start_lo))
            }
            // block has not been allocated yet
            EXT4_EXT_CACHE_GAP => return Ok(()),
            EXT4_EXT_CACHE_IN => {
                lbn - Daddr::from(nex.e_blk)
                    + ((Daddr::from(nex.e_start_hi) << 32) | Daddr::from(nex.e_start_lo))
            }
            t => kpanic(format_args!("ext4_ext_read: cache type {t}")),
        };
        let (mut t, error) = bread(&ip.i_devvp().bufs, fsbtodb(fs, pos), size as i32);
        if let Err(e) = error {
            brelse(t);
            return Err(e);
        }
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
        r?;
    }
    Ok(())
}

/// `ext2fs_write` (`vop_write`): vnode op for writing.
pub fn ext2fs_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let mut extended = false;
    let ip = vtoi(vp);

    #[cfg(feature = "diagnostic")]
    if uio.uio_rw != crate::uio::UioRw::UIO_WRITE {
        kpanic(format_args!("ext2fs_write: mode"));
    }

    // If writing 0 bytes, succeed and do not change update time or file offset (standards
    // compliance)
    if uio.uio_resid == 0 {
        return Ok(());
    }

    match vp.vtype() {
        VREG => {
            if ioflag & IO_APPEND != 0 {
                uio.uio_offset = ext2fs_size(&ip) as Off;
            }
            if ip.i_e2din().i_e2fs_flags() & EXT2_APPEND != 0
                && uio.uio_offset != ext2fs_size(&ip) as Off
            {
                return Err(Errno::EPERM);
            }
        }
        VLNK => {}
        VDIR => {
            if ioflag & IO_SYNC == 0 {
                kpanic(format_args!("ext2fs_write: nonsync dir write"));
            }
        }
        _ => kpanic(format_args!("ext2fs_write: type")),
    }

    let fs = ip.i_e2fs();
    if e2fs_overflow(
        fs,
        uio.uio_resid as Off,
        uio.uio_offset + uio.uio_resid as Off,
    ) {
        return Err(Errno::EFBIG);
    }

    // do the filesize rlimit check
    let p = uio.uio_procp.and_then(G::thread_pin);
    let overrun = vn_fsizechk(vp, uio, ioflag, p)?;

    let resid = uio.uio_resid;
    let osize = ext2fs_size(&ip) as Off;
    let mut flags = if ioflag & IO_SYNC != 0 { B_SYNC } else { 0 };
    let bsize = i64::from(fs.e2fs_bsize.get());

    let mut error = Ok(());
    while uio.uio_resid > 0 {
        let lbn = lblkno(fs, uio.uio_offset) as i32;
        let blkoffset = blkoff(fs, uio.uio_offset);
        let mut xfersize = bsize - blkoffset;
        if (uio.uio_resid as i64) < xfersize {
            xfersize = uio.uio_resid as i64;
        }
        if bsize > xfersize {
            flags |= B_CLRBUF;
        } else {
            flags &= !B_CLRBUF;
        }

        let mut t =
            match ext2fs_buf_alloc(&ip, lbn as u32, (blkoffset + xfersize) as i32, cred, flags) {
                Ok(t) => t,
                Err(e) => {
                    error = Err(e);
                    break;
                }
            };
        if uio.uio_offset + xfersize > ext2fs_size(&ip) as Off {
            if let Err(e) = ext2fs_setsize(&ip, (uio.uio_offset + xfersize) as u64) {
                // The C breaks with the buffer still busy (see the module's deviations).
                brelse(t);
                error = Err(e);
                break;
            }
            uvm_vnp_setsize(vp, ext2fs_size(&ip) as Off);
            extended = true;
        }
        let _ = uvm_vnp_uncache(vp);

        let size = bsize - t.buf().resid() as i64;
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
        } else if xfersize + blkoffset == bsize {
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
    if resid > uio.uio_resid && cred.ucred().is_some_and(|c| c.cr_uid.get() != 0) {
        let d = ip.i_e2din();
        d.set_i_e2fs_mode(d.i_e2fs_mode() & !((ISUID | ISGID) as u16));
    }
    if resid > uio.uio_resid {
        VN_KNOTE(vp, NOTE_WRITE | if extended { NOTE_EXTEND } else { 0 });
    }
    if error.is_err() {
        if ioflag & IO_UNIT != 0 {
            let _ = ext2fs_truncate(&ip, osize, ioflag & IO_SYNC, cred);
            uio.uio_offset -= (resid - uio.uio_resid) as i64;
            uio.uio_resid = resid;
        }
    } else if resid > uio.uio_resid && ioflag & IO_SYNC != 0 {
        error = ext2fs_update(&ip, 1);
    }
    // correct the result for writes clamped by vn_fsizechk()
    uio.uio_resid += overrun;
    error
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for reading and writing over the test image through the fixture's vnodes
    // (N5.6c3 repeats them over the real mount): a file by its block pointers, through its
    // indirect block, and by its extent tree; a write that allocates, read back; the
    // negative offset, the append-only file and the setuid bit a write clears.

    use std::vec::Vec;
    use std::{assert_eq, vec};

    use hal::sys::mount::MNT_RDONLY;

    use super::super::testext2fs::newfs::{self, BIG_SIZE, EXT_SIZE, HELLO_DATA};
    use super::super::testext2fs::{attach, mount, new_file, vget};
    use super::*;
    use crate::cred::Ucred;
    use crate::fs::subr::vput;
    use crate::fs::testfs::TestGlue;
    use crate::proc::Thread;
    use crate::uio::Iovec;

    /// The first `len` bytes of `vp` from `offset`, read with `ext2fs_read`, and the result.
    fn read(
        p: &Thread<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        offset: Off,
        len: usize,
    ) -> (Result<(), Errno>, Vec<u8>) {
        let mut buf = vec![0u8; len];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        uio.uio_offset = offset;
        let r = ext2fs_read(vp, &mut uio, 0, Cred::real(p.ucred()));
        let done = len - uio.uio_resid;
        buf.truncate(done);
        (r, buf)
    }

    /// `data` written at `offset` into `vp` with `ext2fs_write` under `cred`: the result and
    /// the bytes left unwritten.
    fn write(
        p: &Thread<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        offset: Off,
        data: &[u8],
        cred: Cred<'_>,
    ) -> (Result<(), Errno>, usize) {
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_write(&mut iov, data, Some(p));
        uio.uio_offset = offset;
        let r = ext2fs_write(vp, &mut uio, 0, cred);
        (r, uio.uio_resid)
    }

    #[test]
    fn read_follows_the_block_pointers_and_the_extent_tree() {
        let (_g, p) = attach(newfs::image(true));
        let mp = mount(MNT_RDONLY);
        for (ino, want) in [
            (12, HELLO_DATA.to_vec()),
            (13, newfs::pattern(BIG_SIZE, 13)),
            (14, newfs::pattern(EXT_SIZE, 14)),
        ] {
            let vp = vget(mp, ino).unwrap();
            // Past the end of the file: what is there.
            let (r, got) = read(p, vp, 0, want.len() + 100);
            assert_eq!(r, Ok(()));
            assert_eq!(got, want, "inode {ino}");
            // From the middle of a block.
            let (r, got) = read(p, vp, 5, 7);
            assert_eq!((r, got), (Ok(()), want[5..12].to_vec()), "inode {ino}");
            // A negative offset.
            assert_eq!(read(p, vp, -1, 1).0, Err(Errno::EINVAL));
            vput(vp);
        }
    }

    #[test]
    fn write_allocates_and_reads_back_and_clears_setuid_for_a_user() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = new_file(mp, p);
        let ip = vtoi(vp);
        let data = newfs::pattern(3000, 7);
        ip.i_e2din()
            .set_i_e2fs_mode(ip.i_e2din().i_e2fs_mode() | ISUID as u16);
        // Root's write keeps the setuid bit.
        let (r, left) = write(p, vp, 0, &data[..1000], Cred::real(p.ucred()));
        assert_eq!((r, left), (Ok(()), 0));
        assert_ne!(u32::from(ip.i_e2din().i_e2fs_mode()) & ISUID, 0);
        // A user's write, across two more blocks, clears it.
        let mut user = Ucred::new();
        user.cr_uid.set(1000);
        let (r, left) = write(p, vp, 1000, &data[1000..], Cred::real(&user));
        assert_eq!((r, left), (Ok(()), 0));
        assert_eq!(u32::from(ip.i_e2din().i_e2fs_mode()) & ISUID, 0);
        assert_eq!(ext2fs_size(&ip), 3000);
        assert_eq!(read(p, vp, 0, 4000), (Ok(()), data.clone()));
        // Append-only: a write anywhere but at the end is EPERM, at the end it goes in.
        ip.i_e2din().set_i_e2fs_flags(EXT2_APPEND);
        assert_eq!(
            write(p, vp, 0, b"x", Cred::real(p.ucred())).0,
            Err(Errno::EPERM)
        );
        assert_eq!(write(p, vp, 3000, b"x", Cred::real(p.ucred())).0, Ok(()));
        assert_eq!(ext2fs_size(&ip), 3001);
        drop(ip);
        vput(vp);
    }
}
/* </TESTS> */
