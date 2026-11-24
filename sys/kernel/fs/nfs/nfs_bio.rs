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
 * This code is derived from software contributed to Berkeley by
 * Rick Macklem at The University of Guelph.
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
 *	@(#)nfs_bio.c	8.9 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! The NFS client's block I/O through the buffer cache, its call-closed part (N6.7a2): the
//! cache block helpers `nfs_getcacheblk` and `nfs_vinvalbuf`, `nfs_asyncio` (hand a buffer
//! to the nfsiods through `nfs_bufq`) and the block arithmetic of `nfs_bioread`, `nfs_write`
//! and `nfs_doio`.
//!
//! Upstream: sys/nfs/nfs_bio.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_bio.rs@e8249449d905
//!
//! For nfs, cache consistency can only be maintained approximately: a file whose
//! modification time on the server changed since the last read, or that this client wrote,
//! has its cached data thrown away (`nfs_vinvalbuf`). Each buffer keeps the part of its data
//! that is valid (`b_validoff`..`b_validend`) and the part written but not yet sent to the
//! server (`b_dirtyoff`..`b_dirtyend`); NFSv3 unstable writes are remembered with
//! `B_NEEDCOMMIT` and the commit ranges of `nfs_subs`. `nfs_bioread`, `nfs_write` and
//! `nfs_doio` call the RPCs of `nfs_vnops` and come with them (N6.7a3), over these pieces.
//!
//! ## Deviations
//! - `nfs_bufq` is `NfsTables::nfs_bufq` and its counts `nfsnode`'s [`NFS_BUFQLEN`] and
//!   [`NFS_BUFQMAX`] (the header declares them; their addresses are the wait channels, as in
//!   C: an nfsiod sleeps on the queue and wakes `&nfs_bufqlen`). A queued buffer is linked by
//!   its disk-queue link (`fs::bufq::Fifo`, mfs's queue's form), not by `b_freelist`: a buffer
//!   `nfs_asyncio` takes is busy, so it is on no free list and on no disk queue yet.
//! - `nfs_getcacheblk` returns `None` where the C returns NULL (interrupted by a signal).
//! - `nfs_asyncio(bp, readahead)` takes `readahead` as a `bool`, and gives the buffer's token
//!   back with `EIO` when no nfsiod takes it (the caller keeps the buffer, as the C's).
//! - The block arithmetic (`lbn`, `on`, `bn`), the merge of a write into the dirty and valid
//!   ranges, and the zero fill after a short read are small functions ([`nfs_bioblk`],
//!   [`nfs_bufranges`], [`nfs_shortread`]), LZ's, so that host tests can check them.
//!
//! ## Redesign
//! - LZ's functions took the vnode and found the node and the mount again (`VTONFS`,
//!   `VFSTONFS`: an `unsafe` cast each, through `vfstonfs_vp`), and handed buffers around as
//!   `&'static Buf` (`getblk`'s, linked on `NFS_BUFQ` through `b_freelist` by an `unsafe`
//!   insert). Here the vnode operation's node and mount are lent ([`nfs_getcacheblk`] and
//!   [`nfs_vinvalbuf`] take `&NfsMount`, `&NfsNode`: the hot path's rule, no lookup the C
//!   does not make), and a buffer is its busy token (`BufRef`): [`nfs_asyncio`] parks it in
//!   the buffer and queues a counted link (`bufq_park`), the nfsiod takes it back
//!   (`nfs_syscalls`'s `nfs_bufq_get`).
//! - The C's queue is a `TAILQ` under the kernel lock with no `splbio`, which every caller
//!   holds: the system calls, the buffer daemon and the syncer (kernel threads), and the
//!   vnode pager, which takes it around its I/O (`sys/kernel/vm/vnode.rs:1321`) as the
//!   C's page daemon does (C:uvm/uvm_pdaemon.c:313). So the fifo is a cell of the kernel
//!   lock (`Guarded::new_kernel`), changed under its witness, and
//!   `nfs_bufqlen` changes in the same section as the queue: no lock the C lacks. The token
//!   is parked before the section (the cache's lock, `bufq_park`) and taken back after it
//!   (`bufq_unpark`): one section of the cache's lock and one witness section (an spl pair)
//!   per buffer queued, the same per buffer taken, where the C takes neither
//!   (docs/ROADMAP.md N12's row, with the token's cost everywhere else).

use core::cmp::{max, min};
use core::ptr;
use core::sync::atomic::Ordering;

use hal::klock;
use hal::sleep::{INFSLP, PCATCH, PRIBIO, tsleep_nsec, wakeup, wakeup_one};
use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Off};

use super::nfs::{B_INVAFTERWRITE, NfsError, nfs_isv3};
use super::nfs_socket::nfs_sigintr;
use super::nfs_subs::{
    NFSSTATS, nfs_add_tobecommitted_range, nfs_clearcommit, nfs_del_committed_range,
    nfs_del_tobecommitted_range,
};
use super::nfs_vfsops::nfs_fsinfo;
use super::nfs_vnops::{
    buf_vnode, errno_of, nfs_getattr_np, nfs_readlinkrpc, nfs_readrpc, nfs_setcred, nfs_writebp,
    nfs_writerpc, ucred, vmount, with_thread,
};
use super::nfsmount::{NfsMount, vfstonfs};
use super::nfsnode::{
    NFLUSHINPROG, NFLUSHWANT, NFS_BUFQLEN, NFS_BUFQMAX, NFS_NUMASYNC, NMODIFIED, NWRITEERR,
    NfsNode, nfs_invalidate_attrcache, vtonfs,
};
use super::nfsproto::{
    NFS_MAXPATHLEN, NFSV3WRITE_DATASYNC, NFSV3WRITE_FILESYNC, NFSV3WRITE_UNSTABLE,
};
use crate::assert::kpanic;
use crate::clock::sec_to_nsec;
use crate::event::{NOTE_EXTEND, NOTE_TRUNCATE, NOTE_WRITE};
use crate::file::FileGlue;
use crate::fs::bio::{
    BCSTATS, DEV_BSHIFT, bdwrite, biodone, biowait, brelse, buf_dirty, getblk, getblk_wait, incore,
};
use crate::fs::buf::{
    B_ASYNC, B_DELWRI, B_DONE, B_EINTR, B_ERROR, B_INVAL, B_NEEDCOMMIT, B_NOCACHE, B_PHYS, B_READ,
    B_WRITEINPROG, BufRef,
};
use crate::fs::bufq::{BufqDiscipline, bufq_park};
use crate::fs::subr::{vfs, vinvalbuf};
use crate::fs::vnode::Vtype::{VLNK, VREG};
use crate::fs::vnode::{IO_APPEND, IO_SYNC, V_SAVE, VN_KNOTE, VROOT, VTEXT, Vattr, Vnode};
use crate::fs::vnops::vn_fsizechk;
use crate::fs::vops::Cred;
use crate::proc::{ProcGlue, Thread};
use crate::sys::mount::{NFSMNT_GOTFSINFO, NFSMNT_INT, NFSMNT_NFSV3};
use crate::sys::signal::SIGKILL;
use crate::uio::{Iovec, Uio};
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

/// `DEV_BSIZE` (`<sys/param.h>`): the block size `b_blkno` counts in.
pub const DEV_BSIZE: i32 = 1 << DEV_BSHIFT;

/// The cache block of a file offset for blocks of `biosize` bytes (a power of two):
/// `(lbn, on, bn)`, the logical block (`uio_offset / biosize`), the offset in it
/// (`uio_offset & (biosize - 1)`) and the block number in `DEV_BSIZE` units that names the
/// buffer (`lbn * (biosize / DEV_BSIZE)`).
pub fn nfs_bioblk(offset: Off, biosize: i32) -> (Daddr, i32, Daddr) {
    let lbn = offset / Off::from(biosize);
    // The mask keeps the value below `biosize`, an `int`.
    let on = (offset & Off::from(biosize - 1)) as i32;
    let bn = lbn * Daddr::from(biosize / DEV_BSIZE);
    (lbn, on, bn)
}

/// The dirty and valid ranges of a buffer after `n` bytes were copied in at `on`:
/// `(dirtyoff, dirtyend, validoff, validend)` from the old ones (`nfs_write`).
pub fn nfs_bufranges(old: (i32, i32, i32, i32), on: i32, n: i32) -> (i32, i32, i32, i32) {
    let (mut dirtyoff, mut dirtyend, mut validoff, mut validend) = old;
    if dirtyend > 0 {
        dirtyoff = min(on, dirtyoff);
        dirtyend = max(on + n, dirtyend);
    } else {
        dirtyoff = on;
        dirtyend = on + n;
    }
    if validend == 0 || validend < dirtyoff || validoff > dirtyend {
        validoff = dirtyoff;
        validend = dirtyend;
    } else {
        validoff = min(validoff, dirtyoff);
        validend = max(validend, dirtyend);
    }
    (dirtyoff, dirtyend, validoff, validend)
}

/// A read of a `bcount`-byte buffer at file offset `boff` (`b_blkno << DEV_BSHIFT`) that left
/// `resid` bytes unread (`nfs_doio`): `(zero, validend)`, the bytes to zero after the data (a
/// hole in the file, whose later writes have not been pushed to the server yet) and the end
/// of the valid data.
pub fn nfs_shortread(bcount: i64, resid: usize, n_size: u64, boff: Off) -> (usize, i32) {
    // `diff = bp->b_bcount - uiop->uio_resid`, an `int`.
    let diff = bcount.wrapping_sub(resid as i64) as i32;
    // The C computes `n_size - (boff + diff)` in `u_quad_t` and keeps the low `int`.
    let len = n_size.wrapping_sub(boff.wrapping_add(Off::from(diff)) as u64) as i32;
    if len > 0 {
        // `ulmin(len, uiop->uio_resid)`: at most `len`, an `int`.
        let len = min(len as usize, resid);
        (len, diff.wrapping_add(len as i32))
    } else {
        (0, diff)
    }
}

/// `nfs_bioread(vp, uio, ioflag, cred)`: vnode op for read using bio (any similarity to
/// readip() is purely coincidental): a regular file's blocks through the buffer cache, with
/// read-ahead handed to the nfsiods, or a symbolic link's one block. For nfs, cache
/// consistency can only be maintained approximately: if the file's modify time on the server
/// has changed since the last read rpc or you have written to the file, you may have lost
/// data cache consistency with the server, so flush all of the file's data out of the cache;
/// then force a getattr rpc to ensure that you have up to date attributes.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function: the consistency check, read-ahead, the block and the link"
)]
pub fn nfs_bioread<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    _ioflag: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if uio.uio_rw != crate::uio::UioRw::UIO_READ {
        kpanic(format_args!("nfs_read mode"));
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }
    let np = vtonfs(vp);
    let nmp = vfstonfs(vmount(vp));
    let p = uio.uio_procp;
    let cr = ucred(cred);
    if nmp.flag() & (NFSMNT_NFSV3 | NFSMNT_GOTFSINFO) == NFSMNT_NFSV3 {
        let _ = nfs_fsinfo(&nmp, vp, cr, p);
    }
    let biosize = nmp.nm_rsize.load(Ordering::Relaxed);
    let mut vattr = Vattr::new();
    if np.isset(NMODIFIED) {
        nfs_invalidate_attrcache(&np);
        with_thread(p, |pp| nfs_getattr_np(vp, &np, &nmp, &mut vattr, cred, pp))?;
        let _ = np.n_mtime.replace(vattr.va_mtime);
    } else {
        with_thread(p, |pp| nfs_getattr_np(vp, &np, &nmp, &mut vattr, cred, pp))?;
        if *np.n_mtime.read() != vattr.va_mtime {
            nfs_vinvalbuf(&np, &nmp, V_SAVE, cred, p)?;
            let _ = np.n_mtime.replace(vattr.va_mtime);
        }
    }

    // update the cache read creds for this vnode
    nfs_setcred(&np, cr, false);

    loop {
        if vp.flag() & VROOT != 0 && vp.vtype() == VLNK {
            return nfs_readlinkrpc(vp, &np, &nmp, uio, cr).map_err(NfsError::errno);
        }
        let (mut t, on, mut n): (BufRef, i32, i32) = match vp.vtype() {
            VREG => {
                NFSSTATS.biocache_reads.fetch_add(1, Ordering::Relaxed);
                let (lbn, on, bn) = nfs_bioblk(uio.uio_offset, biosize);
                let mut not_readin = true;

                // Start the read ahead(s), as required.
                let readahead = nmp.nm_readahead.load(Ordering::Relaxed);
                if NFS_NUMASYNC.load(Ordering::Relaxed) > 0 && readahead > 0 {
                    let mut nra: i32 = 0;
                    while nra < readahead
                        && (((lbn + 1 + Daddr::from(nra)) * Daddr::from(biosize)) as u64)
                            < np.n_size.load(Ordering::Relaxed)
                    {
                        let rabn = (lbn + 1 + Daddr::from(nra)) * Daddr::from(biosize / DEV_BSIZE);
                        if !incore(&vp.bufs, rabn) {
                            let Some(rabp) = nfs_getcacheblk(&nmp, vp, rabn, biosize, p) else {
                                return Err(Errno::EINTR);
                            };
                            if rabp.buf().flags() & (B_DELWRI | B_DONE) == 0 {
                                rabp.buf().set(B_READ | B_ASYNC);
                                if let Err((rabp, _)) = nfs_asyncio::<G>(rabp, true) {
                                    rabp.buf().set(B_INVAL);
                                    brelse(rabp);
                                }
                            } else {
                                brelse(rabp);
                            }
                        }
                        nra += 1;
                    }
                }

                // again:
                loop {
                    let Some(t) = nfs_getcacheblk(&nmp, vp, bn, biosize, p) else {
                        return Err(Errno::EINTR);
                    };
                    let t = if t.buf().flags() & (B_DONE | B_DELWRI) == 0 {
                        t.buf().set(B_READ);
                        not_readin = false;
                        let (t, error) = nfs_doio_sync(t, vp, &np, &nmp, p);
                        if let Err(e) = error {
                            brelse(t);
                            return Err(e);
                        }
                        t
                    } else {
                        t
                    };
                    // `n = ulmin(biosize - on, uio->uio_resid)`, an `int`.
                    let mut n = ((biosize - on) as usize).min(uio.uio_resid) as i32;
                    // `offdiff = np->n_size - uio->uio_offset`: the `u_quad_t` difference
                    // kept in an `off_t`.
                    let offdiff =
                        np.n_size
                            .load(Ordering::Relaxed)
                            .wrapping_sub(uio.uio_offset as u64) as i64;
                    if offdiff < i64::from(n) {
                        n = offdiff as i32;
                    }
                    let bp = t.buf();
                    if not_readin && n > 0 && (on < bp.validoff() || on + n > bp.validend()) {
                        bp.set(B_INVAFTERWRITE);
                        if bp.dirtyend() > 0 {
                            if !bp.isset(B_DELWRI) {
                                kpanic(format_args!("nfsbioread"));
                            }
                            if nfs_writebp::<G>(t, true) == Err(Errno::EINTR) {
                                return Err(Errno::EINTR);
                            }
                        } else {
                            brelse(t);
                        }
                        continue;
                    }
                    let validend = bp.validend();
                    let diff = if on >= validend { 0 } else { validend - on };
                    if diff < n {
                        n = diff;
                    }
                    break (t, on, n);
                }
            }
            VLNK => {
                NFSSTATS.biocache_readlinks.fetch_add(1, Ordering::Relaxed);
                let Some(t) = nfs_getcacheblk(&nmp, vp, 0, NFS_MAXPATHLEN as i32, p) else {
                    return Err(Errno::EINTR);
                };
                let t = if !t.buf().isset(B_DONE) {
                    t.buf().set(B_READ);
                    let (t, error) = nfs_doio_sync(t, vp, &np, &nmp, p);
                    if let Err(e) = error {
                        brelse(t);
                        return Err(e);
                    }
                    t
                } else {
                    t
                };
                let n = uio
                    .uio_resid
                    .min(NFS_MAXPATHLEN.saturating_sub(t.buf().resid()))
                    as i32;
                (t, 0, n)
            }
            vt => kpanic(format_args!("nfsbioread: type {:x} unexpected", vt as i32)),
        };

        let mut error = Ok(());
        if n > 0 {
            // `0 <= on` and `on + n` is within the block (`n` was clamped to its valid end).
            let (on, len) = (on as usize, n as usize);
            match t.data().get_mut(on..on + len) {
                Some(b) => error = <G::FileGlue as FileGlue>::uiomove(b, uio),
                None => kpanic(format_args!(
                    "nfsbioread: {len} bytes at {on} past the block"
                )),
            }
        }

        if vp.vtype() == VLNK {
            n = 0;
        }

        brelse(t);
        if error.is_err() || uio.uio_resid == 0 || n <= 0 {
            return error;
        }
    }
}

/// `nfs_write` (`vop_write`): vnode op for write using bio. Each block is filled in the
/// buffer cache and its dirty and valid ranges merged; a block that becomes full is written
/// asynchronously, others delayed; a write that would leave a dirty block discontiguous
/// writes the old dirty part first. `IO_SYNC` writes each block at once; `IO_APPEND` asks the
/// server for the size first.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function: the checks, the block loop and its three ways out"
)]
pub fn nfs_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if uio.uio_rw != crate::uio::UioRw::UIO_WRITE {
        kpanic(format_args!("nfs_write mode"));
    }
    if vp.vtype() != VREG {
        return Err(Errno::EIO);
    }
    let np = vtonfs(vp);
    if np.isset(NWRITEERR) {
        np.clr(NWRITEERR);
        return errno_of(np.n_error.load(Ordering::Relaxed));
    }
    let nmp = vfstonfs(vmount(vp));
    let p = uio.uio_procp;
    let cr = ucred(cred);
    if nmp.flag() & (NFSMNT_NFSV3 | NFSMNT_GOTFSINFO) == NFSMNT_NFSV3 {
        let _ = nfs_fsinfo(&nmp, vp, cr, p);
    }
    if ioflag & (IO_APPEND | IO_SYNC) != 0 {
        if np.isset(NMODIFIED) {
            nfs_invalidate_attrcache(&np);
            nfs_vinvalbuf(&np, &nmp, V_SAVE, cred, p)?;
        }
        if ioflag & IO_APPEND != 0 {
            nfs_invalidate_attrcache(&np);
            let mut vattr = Vattr::new();
            with_thread(p, |pp| nfs_getattr_np(vp, &np, &nmp, &mut vattr, cred, pp))?;
            uio.uio_offset = np.n_size.load(Ordering::Relaxed) as Off;
        }
    }
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }

    // do the filesize rlimit check
    let overrun = vn_fsizechk(vp, uio, ioflag, p.and_then(G::thread_pin))?;

    // update the cache write creds for this node.
    nfs_setcred(&np, cr, true);

    // I use nm_rsize, not nm_wsize so that all buffer cache blocks will be the same size
    // within a filesystem. nfs_writerpc will still use nm_wsize when sizing the rpc's.
    let biosize = nmp.nm_rsize.load(Ordering::Relaxed);
    let v3 = nfs_isv3(&nmp);
    let b_proc = p.map_or(0, |p| ptr::from_ref(p).addr());
    let mut extended = false;
    let mut truncated = false;
    let error: Result<(), Errno> = 'out: {
        loop {
            // XXX make sure we aren't cached in the VM page cache
            uvm_vnp_uncache(vp);

            NFSSTATS.biocache_writes.fetch_add(1, Ordering::Relaxed);
            let (_lbn, on, bn) = nfs_bioblk(uio.uio_offset, biosize);
            // `n = ulmin(biosize - on, uio->uio_resid)`, an `int`.
            let n = ((biosize - on) as usize).min(uio.uio_resid) as i32;
            // again:
            loop {
                let Some(mut t) = nfs_getcacheblk(&nmp, vp, bn, biosize, p) else {
                    break 'out Err(Errno::EINTR);
                };
                np.set(NMODIFIED);
                let end = uio.uio_offset + Off::from(n);
                let n_size = np.n_size.load(Ordering::Relaxed);
                if end as u64 > n_size {
                    np.n_size.store(end as u64, Ordering::Relaxed);
                    uvm_vnp_setsize(vp, end);
                    extended = true;
                } else if (end as u64) < n_size {
                    truncated = true;
                }

                // If the new write will leave a contiguous dirty area, just update the
                // b_dirtyoff and b_dirtyend, otherwise force a write rpc of the old dirty area.
                let bp = t.buf();
                if bp.dirtyend() > 0 && (on > bp.dirtyend() || on + n < bp.dirtyoff()) {
                    bp.set_proc_addr(b_proc);
                    if nfs_writebp::<G>(t, true) == Err(Errno::EINTR) {
                        break 'out Err(Errno::EINTR);
                    }
                    continue;
                }

                // `0 <= on` and `on + n <= biosize`, the block's size.
                let (o, len) = (on as usize, n as usize);
                let moved = match t.data().get_mut(o..o + len) {
                    Some(b) => <G::FileGlue as FileGlue>::uiomove(b, uio),
                    None => kpanic(format_args!("nfs_write: {len} bytes at {o} past the block")),
                };
                if let Err(e) = moved {
                    t.buf().set(B_ERROR);
                    brelse(t);
                    break 'out Err(e);
                }
                let bp = t.buf();
                let (dirtyoff, dirtyend, validoff, validend) = nfs_bufranges(
                    (bp.dirtyoff(), bp.dirtyend(), bp.validoff(), bp.validend()),
                    on,
                    n,
                );
                bp.set_dirtyoff(dirtyoff);
                bp.set_dirtyend(dirtyend);
                bp.set_validoff(validoff);
                bp.set_validend(validend);

                // Since this block is being modified, it must be written again and not just
                // committed.
                if v3 {
                    let _lock = np.n_commitlock.write();
                    if bp.isset(B_NEEDCOMMIT) {
                        bp.clr(B_NEEDCOMMIT);
                        nfs_del_tobecommitted_range(&np, bp);
                    }
                    nfs_del_committed_range(&np, bp);
                } else {
                    bp.clr(B_NEEDCOMMIT);
                }

                if ioflag & IO_SYNC != 0 {
                    bp.set_proc_addr(b_proc);
                    if let Err(e) = nfs_writebp::<G>(t, true) {
                        break 'out Err(e);
                    }
                } else if n + on == biosize {
                    bp.set_proc_addr(0);
                    bp.set(B_ASYNC);
                    let _ = nfs_writebp::<G>(t, false);
                } else {
                    bdwrite(t);
                }
                break;
            }
            if uio.uio_resid == 0 || n <= 0 {
                break;
            }
        }

        // out: XXX belongs here??? (`if (wrotedta)`: every way out of the loop but `out`
        // copied data into a block, so the C's flag is always set here.)
        VN_KNOTE(
            vp,
            NOTE_WRITE
                | if extended { NOTE_EXTEND } else { 0 }
                | if truncated { NOTE_TRUNCATE } else { 0 },
        );
        Ok(())
    };

    // out: correct the result for writes clamped by vn_fsizechk()
    uio.uio_resid += overrun;

    error
}

/// `nfs_getcacheblk(vp, bn, size, p)`: get an nfs cache block. Allocate a new one if the
/// block isn't currently in the cache and return the block marked busy. If the calling
/// process is interrupted by a signal for an interruptible mount point, return `None`.
/// `nmp` is the vnode's mount, lent by the caller (the C's `VFSTONFS(vp->v_mount)`).
pub fn nfs_getcacheblk<G: ProcGlue>(
    nmp: &NfsMount<G>,
    vp: &'static Vnode<G>,
    bn: Daddr,
    size: i32,
    p: Option<&Thread<G>>,
) -> Option<BufRef> {
    if nmp.flag() & NFSMNT_INT != 0 {
        let mut bp = getblk(&vp.bufs, bn, size, PCATCH, INFSLP);
        while bp.is_none() {
            if nfs_sigintr(nmp, None, p).is_err() {
                return None;
            }
            bp = getblk(&vp.bufs, bn, size, 0, sec_to_nsec(2));
        }
        bp
    } else {
        // `getblk(.., 0, INFSLP)` sleeps until it has the buffer.
        Some(getblk_wait(&vp.bufs, bn, size))
    }
}

/// `nfs_vinvalbuf(vp, flags, cred, p)`: flush and invalidate all dirty buffers. If another
/// process is already doing the flush, just wait for completion. `np` is the vnode's node
/// and `nmp` its mount, lent by the caller (the C's `VTONFS(vp)`, `VFSTONFS(vp->v_mount)`).
pub fn nfs_vinvalbuf<G: ProcGlue>(
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    flags: i32,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let vp = super::nfsnode::nfstov(np);
    let (sintr, stimeo) = if nmp.flag() & NFSMNT_INT != 0 {
        (PCATCH, sec_to_nsec(2))
    } else {
        (0, INFSLP)
    };
    let chan = ptr::from_ref(&np.n_flag);

    // First wait for any other process doing a flush to complete.
    while np.isset(NFLUSHINPROG) {
        np.set(NFLUSHWANT);
        let error = tsleep_nsec(chan, PRIBIO | sintr, "nfsvinval", stimeo);
        if error.is_err() && sintr != 0 && nfs_sigintr(nmp, None, p).is_err() {
            return Err(Errno::EINTR);
        }
    }

    // Now, flush as required.
    np.set(NFLUSHINPROG);
    let mut error = vinvalbuf(vp, flags, cred, p, sintr, INFSLP);
    while error.is_err() {
        if sintr != 0 && nfs_sigintr(nmp, None, p).is_err() {
            np.clr(NFLUSHINPROG);
            if np.isset(NFLUSHWANT) {
                np.clr(NFLUSHWANT);
                wakeup(chan);
            }
            return Err(Errno::EINTR);
        }
        error = vinvalbuf(vp, flags, cred, p, 0, stimeo);
    }
    np.clr(NMODIFIED | NFLUSHINPROG);
    if np.isset(NFLUSHWANT) {
        np.clr(NFLUSHWANT);
        wakeup(chan);
    }
    Ok(())
}

/// `nfs_asyncio(bp, readahead)`: initiate asynchronous I/O. Return an error if no nfsiods
/// are available. This is mainly to avoid queueing async I/O requests when the nfsiods are
/// all hung on a dead server. On the error (`EIO`, counted in `forcedsync`) the buffer's
/// token comes back, the caller's to do the I/O itself or release; otherwise it waits on
/// `nfs_bufq` for an nfsiod. A read-ahead (`readahead`) gives up rather than wait for room.
pub fn nfs_asyncio<G: ProcGlue>(t: BufRef, readahead: bool) -> Result<(), (BufRef, Errno)> {
    'out: {
        if NFS_NUMASYNC.load(Ordering::Relaxed) == 0 {
            break 'out;
        }

        while NFS_BUFQLEN.load(Ordering::Relaxed) > NFS_BUFQMAX.load(Ordering::Relaxed) {
            if readahead {
                break 'out;
            }
            let _ = tsleep_nsec(ptr::from_ref(&NFS_BUFQLEN), PRIBIO, "nfs_bufq", INFSLP);
        }

        if !t.buf().isset(B_READ) {
            t.buf().set(B_WRITEINPROG);
        }

        let q = &vfs::<G>().nfs.nfs_bufq;
        let m = bufq_park(t);
        {
            let mut w = klock::witness();
            q.get_mut(&mut w).queue(m);
            NFS_BUFQLEN.fetch_add(1, Ordering::Relaxed);
        }

        wakeup_one(ptr::from_ref(q));
        return Ok(());
    }

    // out:
    NFSSTATS.forcedsync.fetch_add(1, Ordering::Relaxed);
    Err((t, Errno::EIO))
}

/// [`nfs_doio`] of a buffer whose I/O its caller waits for (`nfs_bioread`'s, not `B_ASYNC`):
/// the token comes back from `biodone` (the C's caller goes on with the buffer it gave), with
/// the RPC's error.
fn nfs_doio_sync<G: ProcGlue>(
    t: BufRef,
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    p: Option<&Thread<G>>,
) -> (BufRef, Result<(), Errno>) {
    let Some(r) = t.counted() else {
        kpanic(format_args!(
            "nfs_doio: a driver's own buffer {:p}",
            t.buf()
        ));
    };
    let error = nfs_doio_np(t, vp, np, nmp, p);
    let (t, _) = biowait(r);
    (t, error)
}

/// `nfs_doio(bp, p)`: do an I/O operation to/from a cache block. This may be called
/// synchronously (`nfs_strategy`, `p` the caller) or from an nfsiod (`p` `None`). The buffer's
/// token is given up to `biodone` at the end, as the C's.
pub fn nfs_doio<G: ProcGlue>(t: BufRef, p: Option<&Thread<G>>) -> Result<(), Errno> {
    let vp: &'static Vnode<G> = buf_vnode(t.buf());
    let np = vtonfs(vp);
    let nmp = vfstonfs(vmount(vp));
    nfs_doio_np(t, vp, &np, &nmp, p)
}

/// [`nfs_doio`] of the buffer of `vp`, whose node `np` and mount `nmp` the caller lends.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function: physio, a read, a write"
)]
fn nfs_doio_np<G: ProcGlue>(
    mut t: BufRef,
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let flags = t.buf().flags();
    let bcount = usize::try_from(t.buf().bcount()).unwrap_or(0);
    let blkno = t.buf().blkno();
    let mut must_commit = false;
    let mut iov = [Iovec::new()];
    let error: Result<(), NfsError>;
    let resid: usize;

    // Historically, paging was done with physio, but no more.
    if flags & B_PHYS != 0 {
        // mapping was done by vmapbuf()
        let off = blkno << DEV_BSHIFT;
        if flags & B_READ != 0 {
            NFSSTATS.read_physios.fetch_add(1, Ordering::Relaxed);
            let mut uio = Uio::kernel_read(&mut iov, &mut t.data()[..bcount], p);
            uio.uio_offset = off;
            error = nfs_readrpc(vp, np, nmp, &mut uio);
            resid = uio.uio_resid;
        } else {
            let mut iomode = NFSV3WRITE_DATASYNC;
            NFSSTATS.write_physios.fetch_add(1, Ordering::Relaxed);
            let mut uio = Uio::kernel_write(&mut iov, &t.data()[..bcount], p);
            uio.uio_offset = off;
            error = nfs_writerpc(vp, np, nmp, &mut uio, &mut iomode, &mut must_commit);
            resid = uio.uio_resid;
        }
        if let Err(e) = error {
            t.buf().set(B_ERROR);
            t.buf().set_error(Some(e.errno()));
        }
    } else if flags & B_READ != 0 {
        match vp.vtype() {
            VREG => {
                let boff = blkno << DEV_BSHIFT;
                NFSSTATS.read_bios.fetch_add(1, Ordering::Relaxed);
                BCSTATS.pendingreads.fetch_add(1, Ordering::Relaxed);
                BCSTATS.numreads.fetch_add(1, Ordering::Relaxed);
                {
                    let mut uio = Uio::kernel_read(&mut iov, &mut t.data()[..bcount], p);
                    uio.uio_offset = boff;
                    error = nfs_readrpc(vp, np, nmp, &mut uio);
                    resid = uio.uio_resid;
                }
                if error.is_ok() {
                    t.buf().set_validoff(0);
                    if resid != 0 {
                        // If len > 0, there is a hole in the file and no writes after the
                        // hole have been pushed to the server yet. Just zero fill the rest of
                        // the valid area.
                        let (zero, validend) = nfs_shortread(
                            t.buf().bcount(),
                            resid,
                            np.n_size.load(Ordering::Relaxed),
                            boff,
                        );
                        if zero > 0 {
                            // `diff = bcount - resid`: within the buffer, as `zero` is at
                            // most `resid`.
                            let diff = bcount - resid;
                            t.data()[diff..diff + zero].fill(0);
                        }
                        t.buf().set_validend(validend);
                    } else {
                        t.buf().set_validend(t.buf().bcount() as i32);
                    }
                }
                if let Some(p) = p
                    && vp.flag() & VTEXT != 0
                    && *np.n_mtime.read() != np.n_vattr.read().va_mtime
                {
                    crate::tty::uprintf(
                        p.process(),
                        format_args!("Process killed due to text file modification\n"),
                    );
                    if let Some(pp) = G::thread_pin(p) {
                        crate::sig::psignal(pp, SIGKILL);
                    }
                }
            }
            VLNK => {
                NFSSTATS.readlink_bios.fetch_add(1, Ordering::Relaxed);
                BCSTATS.pendingreads.fetch_add(1, Ordering::Relaxed);
                BCSTATS.numreads.fetch_add(1, Ordering::Relaxed);
                let mut uio = Uio::kernel_read(&mut iov, &mut t.data()[..bcount], p);
                uio.uio_offset = 0;
                // `curproc->p_ucred` (the module's Deviations: none is `panic(9)`).
                error = G::with_curproc(|cp| match cp {
                    Some(cp) => nfs_readlinkrpc(vp, np, nmp, &mut uio, cp.ucred()),
                    None => kpanic(format_args!("nfs_doio: a link read with no thread")),
                });
                resid = uio.uio_resid;
            }
            vt => kpanic(format_args!("nfs_doio:  type {:x} unexpected", vt as i32)),
        }
        if let Err(e) = error {
            t.buf().set(B_ERROR);
            t.buf().set_error(Some(e.errno()));
        }
    } else {
        let (dirtyoff, dirtyend) = (t.buf().dirtyoff(), t.buf().dirtyend());
        // The dirty range is within the buffer (`nfs_write` set it there).
        let (lo, hi) = (
            usize::try_from(dirtyoff).unwrap_or(0),
            usize::try_from(dirtyend).unwrap_or(0),
        );
        let off = blkno.wrapping_mul(Off::from(DEV_BSIZE)) + Off::from(dirtyoff);
        NFSSTATS.write_bios.fetch_add(1, Ordering::Relaxed);
        BCSTATS.pendingwrites.fetch_add(1, Ordering::Relaxed);
        BCSTATS.numwrites.fetch_add(1, Ordering::Relaxed);
        let mut iomode = if flags & (B_ASYNC | B_NEEDCOMMIT | B_NOCACHE) == B_ASYNC {
            NFSV3WRITE_UNSTABLE
        } else {
            NFSV3WRITE_FILESYNC
        };
        t.buf().set(B_WRITEINPROG);
        {
            let Some(data) = t.data().get(lo..hi) else {
                kpanic(format_args!(
                    "nfs_doio: dirty range {lo}..{hi} past the buffer"
                ));
            };
            let mut uio = Uio::kernel_write(&mut iov, data, p);
            uio.uio_offset = off;
            error = nfs_writerpc(vp, np, nmp, &mut uio, &mut iomode, &mut must_commit);
            resid = uio.uio_resid;
        }

        let bp = t.buf();
        {
            let _lock = np.n_commitlock.write();
            if error.is_ok() && iomode == NFSV3WRITE_UNSTABLE {
                bp.set(B_NEEDCOMMIT);
                nfs_add_tobecommitted_range(np, bp);
            } else {
                bp.clr(B_NEEDCOMMIT);
                nfs_del_committed_range(np, bp);
            }
        }

        bp.clr(B_WRITEINPROG);

        // For an interrupted write, the buffer is still valid and the write hasn't been
        // pushed to the server yet, so we can't set B_ERROR and report the interruption by
        // setting B_EINTR. For the B_ASYNC case, B_EINTR is not relevant, so the rpc attempt
        // is essentially a noop. For the case of a V3 write rpc not being committed to stable
        // storage, the block is still dirty and requires either a commit rpc or another
        // write rpc with iomode == NFSV3WRITE_FILESYNC before the block is reused. This is
        // indicated by setting the B_DELWRI and B_NEEDCOMMIT flags.
        if error == Err(NfsError::Errno(Errno::EINTR)) || (error.is_ok() && bp.isset(B_NEEDCOMMIT))
        {
            buf_dirty(&t);

            if !t.buf().isset(B_ASYNC) && error.is_err() {
                t.buf().set(B_EINTR);
            }
        } else {
            if let Err(e) = error {
                bp.set(B_ERROR);
                bp.set_error(Some(e.errno()));
                np.n_error.store(e.errno() as i32, Ordering::Relaxed);
                np.set(NWRITEERR);
            }
            bp.set_dirtyoff(0);
            bp.set_dirtyend(0);
        }
    }
    t.buf().set_resid(resid);
    if must_commit {
        nfs_clearcommit(vmount(vp));
    }
    biodone(t);
    error.map_err(NfsError::errno)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the block arithmetic (the C's values, table-driven, and property tests
    // of its bounds), nfs_getcacheblk, nfs_vinvalbuf's flags and its signal path, and
    // nfs_asyncio's queue and its limits.

    use super::*;
    use crate::fs::bio::{bdwrite, brelse};
    use crate::fs::buf::B_READ;
    use crate::fs::nfs::nfs_node::{test_mount, test_node};
    use crate::fs::nfs::nfs_syscalls::{nfs_bufq_get, nfs_bufq_peek};
    use crate::fs::nfs::nfsm_subs::TestRng;
    use crate::fs::vnode::VREG;
    use crate::sys::signal::{SIGINT, sigmask};

    #[test]
    fn the_cache_block_of_an_offset() {
        for (offset, biosize, want) in [
            (0, 8192, (0, 0, 0)),
            (1, 8192, (0, 1, 0)),
            (8191, 8192, (0, 8191, 0)),
            (8192, 8192, (1, 0, 16)),
            (8193, 8192, (1, 1, 16)),
            (100_000, 32_768, (3, 1_696, 192)),
            (1 << 40, 65_536, (1 << 24, 0, 1 << 31)),
            (511, 512, (0, 511, 0)),
            (512, 512, (1, 0, 1)),
        ] {
            assert_eq!(nfs_bioblk(offset, biosize), want, "{offset} {biosize}");
        }
    }

    /// Property: `lbn * biosize + on` is the offset, `on` lies in the block, and `bn` names
    /// the block in `DEV_BSIZE` units, for any offset and power-of-two block size.
    #[test]
    fn the_block_arithmetic_covers_the_offset() {
        let mut rng = TestRng(0x6269_6f62_6c6b_6172);
        for _ in 0..20_000 {
            let biosize = 512i32 << rng.below(8);
            let offset = (rng.next_u64() >> rng.below(64)) as i64 & i64::MAX >> 8;
            let (lbn, on, bn) = nfs_bioblk(offset, biosize);
            assert!(on >= 0 && on < biosize);
            assert_eq!(lbn * i64::from(biosize) + i64::from(on), offset);
            assert_eq!(bn * i64::from(DEV_BSIZE), lbn * i64::from(biosize));
        }
    }

    /// `nfs_write`'s merge of `n` bytes at `on` into a buffer's dirty and valid ranges.
    #[test]
    fn a_write_merges_into_the_dirty_and_valid_ranges() {
        // (dirtyoff, dirtyend, validoff, validend), on, n, result
        for (old, on, n, want) in [
            ((0, 0, 0, 0), 100, 50, (100, 150, 100, 150)),
            ((100, 150, 100, 150), 120, 100, (100, 220, 100, 220)),
            ((100, 150, 100, 150), 10, 20, (10, 150, 10, 150)),
            ((0, 0, 0, 4096), 100, 50, (100, 150, 0, 4096)),
            ((0, 0, 200, 300), 100, 50, (100, 150, 100, 150)),
            ((0, 0, 400, 500), 100, 50, (100, 150, 100, 150)),
            ((0, 0, 140, 300), 100, 50, (100, 150, 100, 300)),
            ((100, 150, 0, 8192), 0, 8192, (0, 8192, 0, 8192)),
        ] {
            assert_eq!(nfs_bufranges(old, on, n), want, "{old:?} {on} {n}");
        }
    }

    /// Property: after a write the dirty range holds the bytes written and the old dirty
    /// range, and the valid range holds the dirty one.
    #[test]
    fn the_merged_ranges_hold_the_write() {
        let mut rng = TestRng(0x6275_6672_616e_6765);
        for _ in 0..20_000 {
            let size = 512i32 << rng.below(8);
            let on = rng.below(size as usize) as i32;
            let n = 1 + rng.below((size - on) as usize) as i32;
            let doff = rng.below(size as usize) as i32;
            let dend = if rng.below(3) == 0 {
                0
            } else {
                doff + rng.below((size - doff) as usize + 1) as i32
            };
            let voff = rng.below(size as usize) as i32;
            let vend = voff + rng.below((size - voff) as usize + 1) as i32;
            let (d0, d1, v0, v1) = nfs_bufranges((doff, dend, voff, vend), on, n);
            assert!(d0 <= on && d1 >= on + n);
            if dend > 0 {
                assert!(d0 <= doff && d1 >= dend);
            }
            assert!(v0 <= d0 && v1 >= d1);
        }
    }

    /// `nfs_doio`'s zero fill after a short read: the bytes from the data's end to the
    /// node's size, at most the bytes left unread, and the valid end after them.
    #[test]
    fn a_short_read_zero_fills_up_to_the_node_size() {
        // bcount, resid, n_size, boff -> (zero, validend)
        for (bcount, resid, n_size, boff, want) in [
            (8192, 4096, 1 << 20, 0, (4096, 8192)),
            (8192, 4096, 6000, 0, (1904, 6000)),
            (8192, 4096, 4096, 0, (0, 4096)),
            (8192, 4096, 100, 0, (0, 4096)),
            (8192, 8192, 8192 + 10, 8192, (10, 10)),
            // The C keeps the low int of the u_quad_t difference: negative here, no fill.
            (8192, 1, 1 << 40, 1 << 30, (0, 8191)),
            (8192, 1, (1 << 30) + 8192, 1 << 30, (1, 8192)),
        ] {
            assert_eq!(
                nfs_shortread(bcount, resid, n_size, boff),
                want,
                "{bcount} {resid} {n_size} {boff}"
            );
        }
    }

    /// `nfs_getcacheblk` gives the block busy, on an interruptible mount and not.
    #[test]
    fn getcacheblk_gives_the_block_busy() {
        for flag in [0, NFSMNT_INT] {
            let t = test_mount(flag);
            let (vp, _np) = test_node(t.mp, VREG, &[1; 8]);
            let tk = nfs_getcacheblk(&t.nmp, vp, 32, 8192, Some(t.p)).expect("a block");
            assert_eq!(tk.buf().lblkno(), 32);
            assert!(tk.buf().isset(crate::fs::buf::B_BUSY));
            brelse(tk);
        }
    }

    /// `nfs_vinvalbuf`: every buffer of the vnode invalidated, `NMODIFIED` and
    /// `NFLUSHINPROG` cleared, a waiter's `NFLUSHWANT` cleared (and woken).
    #[test]
    fn vinvalbuf_flushes_and_clears_the_flags() {
        let t = test_mount(0);
        let (vp, np) = test_node(t.mp, VREG, &[2; 8]);
        for blk in 0..4 {
            bdwrite(getblk_wait(&vp.bufs, blk * 16, 8192));
        }
        assert!(!vp.bufs.dirty_is_empty());
        np.set(NMODIFIED | NFLUSHWANT);
        nfs_vinvalbuf(&np, &t.nmp, 0, Cred::NOCRED, Some(t.p)).expect("flushed");
        assert!(vp.bufs.dirty_is_empty() && vp.bufs.clean_is_empty());
        assert!(!np.isset(NMODIFIED | NFLUSHINPROG | NFLUSHWANT));
    }

    /// On an interruptible mount a thread with an interrupting signal pending gives up the
    /// wait for another flush (its 2 s sleep, then `nfs_sigintr`) with `EINTR`, its
    /// `NFLUSHWANT` left for the flusher; without the signal it would wait on.
    #[test]
    fn vinvalbuf_waits_for_another_flush_until_a_signal() {
        let t = test_mount(NFSMNT_INT);
        crate::sig::test_sigacts(t.p.process());
        let (_vp, np) = test_node(t.mp, VREG, &[3; 8]);
        np.set(NFLUSHINPROG);
        t.p.p_siglist.store(sigmask(SIGINT), Ordering::Relaxed);
        assert_eq!(
            nfs_vinvalbuf(&np, &t.nmp, 0, Cred::NOCRED, Some(t.p)),
            Err(Errno::EINTR)
        );
        t.p.p_siglist.store(0, Ordering::Relaxed);
        assert!(np.isset(NFLUSHINPROG | NFLUSHWANT));
        np.clr(NFLUSHINPROG | NFLUSHWANT);
    }

    /// `nfs_asyncio`: with no nfsiod the buffer comes back (`EIO`, `forcedsync` counted);
    /// otherwise it is queued (a write marked `B_WRITEINPROG`, a read not) and taken back by
    /// an nfsiod; a read-ahead gives up when the queue is over its limit.
    #[test]
    fn asyncio_queues_for_the_nfsiods_within_the_limit() {
        let t = test_mount(0);
        let (vp, _np) = test_node(t.mp, VREG, &[4; 8]);
        let forced = NFSSTATS.forcedsync.load(Ordering::Relaxed);
        let tk = getblk_wait(&vp.bufs, 0, 8192);
        let Err((tk, e)) = nfs_asyncio::<crate::fs::testfs::TestGlue>(tk, false) else {
            panic!("queued with no nfsiod");
        };
        assert_eq!(e, Errno::EIO);
        assert_eq!(NFSSTATS.forcedsync.load(Ordering::Relaxed), forced + 1);
        assert!(!tk.buf().isset(B_WRITEINPROG));

        NFS_NUMASYNC.store(1, Ordering::Relaxed);
        NFS_BUFQMAX.store(1, Ordering::Relaxed);
        assert!(nfs_asyncio::<crate::fs::testfs::TestGlue>(tk, false).is_ok());
        assert_eq!(NFS_BUFQLEN.load(Ordering::Relaxed), 1);
        let rd = getblk_wait(&vp.bufs, 16, 8192);
        rd.buf().set(B_READ);
        assert!(nfs_asyncio::<crate::fs::testfs::TestGlue>(rd, false).is_ok());
        assert_eq!(NFS_BUFQLEN.load(Ordering::Relaxed), 2);
        // Over the limit (2 > 1): a read-ahead gives up.
        let ra = getblk_wait(&vp.bufs, 32, 8192);
        ra.buf().set(B_READ);
        let Err((ra, e)) = nfs_asyncio::<crate::fs::testfs::TestGlue>(ra, true) else {
            panic!("a read-ahead queued over the limit");
        };
        assert_eq!(e, Errno::EIO);
        assert_eq!(NFSSTATS.forcedsync.load(Ordering::Relaxed), forced + 2);
        brelse(ra);

        let w = nfs_bufq_get::<crate::fs::testfs::TestGlue>().expect("the write");
        assert_eq!(w.buf().lblkno(), 0);
        assert!(w.buf().isset(B_WRITEINPROG));
        let r = nfs_bufq_get::<crate::fs::testfs::TestGlue>().expect("the read");
        assert_eq!(r.buf().lblkno(), 16);
        assert!(!r.buf().isset(B_WRITEINPROG));
        assert!(!nfs_bufq_peek::<crate::fs::testfs::TestGlue>());
        assert!(nfs_bufq_get::<crate::fs::testfs::TestGlue>().is_none());
        assert_eq!(NFS_BUFQLEN.load(Ordering::Relaxed), 0);
        w.buf().clr(B_WRITEINPROG);
        brelse(w);
        r.buf().clr(B_READ);
        brelse(r);
        NFS_NUMASYNC.store(0, Ordering::Relaxed);
        NFS_BUFQMAX.store(0, Ordering::Relaxed);
    }
}
/* </TESTS> */
