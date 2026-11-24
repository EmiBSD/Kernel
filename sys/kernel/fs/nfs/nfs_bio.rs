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
//! - The C's queue is a `TAILQ` under the kernel lock with no `splbio`; here the fifo is in a
//!   `Mutexed` at `IPL_BIO` (mfs's `mfs_bufq`): `nfs_asyncio` may be reached from a path
//!   that does not hold the kernel lock (a write the page daemon starts), and the park takes
//!   the cache's lock first. One section of the cache's lock (the park) and one of the
//!   queue's per buffer queued, and the same per buffer taken, where the C takes none
//!   (docs/ROADMAP.md N12's row, with the token's cost everywhere else).

use core::cmp::{max, min};
use core::ptr;
use core::sync::atomic::Ordering;

use hal::sleep::{INFSLP, PCATCH, PRIBIO, tsleep_nsec, wakeup, wakeup_one};
use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Off};

use super::nfs_socket::nfs_sigintr;
use super::nfs_subs::NFSSTATS;
use super::nfsmount::NfsMount;
use super::nfsnode::{
    NFLUSHINPROG, NFLUSHWANT, NFS_BUFQLEN, NFS_BUFQMAX, NFS_NUMASYNC, NMODIFIED, NfsNode,
};
use crate::clock::sec_to_nsec;
use crate::fs::bio::{DEV_BSHIFT, getblk, getblk_wait};
use crate::fs::buf::{B_READ, B_WRITEINPROG, BufRef};
use crate::fs::bufq::{BufqDiscipline, bufq_park};
use crate::fs::subr::{vfs, vinvalbuf};
use crate::fs::vnode::Vnode;
use crate::fs::vops::Cred;
use crate::proc::{ProcGlue, Thread};
use crate::sys::mount::NFSMNT_INT;

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
    let vp = np.n_vnode;
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
        q.lock().queue(m);
        NFS_BUFQLEN.fetch_add(1, Ordering::Relaxed);

        wakeup_one(ptr::from_ref(q));
        return Ok(());
    }

    // out:
    NFSSTATS.forcedsync.fetch_add(1, Ordering::Relaxed);
    Err((t, Errno::EIO))
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
