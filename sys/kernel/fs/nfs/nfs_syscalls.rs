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
 *	@(#)nfs_syscalls.c	8.5 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! The client's asynchronous I/O threads (`nfssvc_iod`, the nfsiods), their call-closed part
//! (N6.7a2): the table of the running ones (`nfs_asyncdaemon[]`), an nfsiod's start and end
//! (its slot, its share of `nfs_bufqmax`), taking a buffer off `nfs_bufq`, and the delayed
//! write it picks up beside a write.
//!
//! Upstream: sys/nfs/nfs_syscalls.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_syscalls.rs@e8249449d905
//!
//! The nfsiods do read-ahead and write-behind operations on the block I/O cache: each takes
//! the buffers `nfs_asyncio` queues and does their RPCs (`nfs_doio`). [`nfssvc_iod`]'s loop,
//! which calls `nfs_doio`, and [`nfs_getset_niothreads`], which starts it, came with
//! `nfs_doio` (N6.7a3), over these pieces; `nfssvc(2)` and the server's half (`nfssvc_nfsd`,
//! `nfsrv_*`) are `nfssvc`'s (N6.7b2, the server since N6.7b4).
//!
//! ## Deviations
//! - `nfs_asyncdaemon[]` holds the nfsiods' thread ids (0: a free slot; thread 0 is process
//!   0's, never an nfsiod) in `NfsTables::nfs_asyncdaemon`, where the C keeps `struct proc *`:
//!   `nfs_getset_niothreads` signals a surplus one through `tfind`.
//! - A slot is claimed by a compare-and-swap, where the C's search and store are one step
//!   under the kernel lock.
//!
//! ## Redesign
//! - LZ's `NFS_ASYNCDAEMON` was an array of `AtomicPtr<Proc>` (an `unsafe` deref to signal
//!   one), its queue walk an `unsafe` unlink of `&'static Buf`s from `NFS_BUFQ`, and the
//!   delayed write a raw walk of `v_dirtyblkhd` at `splbio`. Here the table holds tids, a
//!   queued buffer comes back as its busy token ([`nfs_bufq_get`]: the counted link
//!   `nfs_asyncio` queued, its parked token taken back), and the delayed write is the
//!   buffer cache's dirty scan ([`nfs_iod_nextwrite`], `vnode_dirty_scan`, which takes the
//!   buffer off its free queue and makes it busy in the section that finds it, the C's
//!   `bufcache_take` and `buf_acquire`). The C does the delayed write's first part of
//!   `nfs_bwrite` (flags, `buf_undirty`, `v_numoutput++`) in the same `splbio` section; here
//!   it is the scan's section, `buf_undirty`'s and `numoutput_inc`'s: two sections of the
//!   cache's lock more per write behind (docs/ROADMAP.md N12's row).
//! - N6.7a3: [`nfssvc_iod`] is a `fork::Start::NfsIod` thread (`nfs_kqpoll`'s form): its tid
//!   is its slot's value; the surplus nfsiod [`nfs_getset_niothreads`] kills is found by that
//!   tid on the thread hash under a reader of the kernel lock (the C's `psignal` of the
//!   slot's `struct proc *`), and a slot whose thread is gone already is counted as the C's.

use core::ptr;
use core::sync::atomic::Ordering;

use hal::sync::klock;
use hal::sync::sleep::{INFSLP, PCATCH, PWAIT, tsleep_nsec, wakeup, wakeup_one};
use hal::sys::errno::Errno;
use hal::sys::types::Pid;

use super::nfs::nfs_niothreads;
use super::nfs_bio::nfs_doio;
use super::nfsnode::{NFS_BUFQLEN, NFS_BUFQMAX, NFS_NUMASYNC};
use crate::assert::kpanic;
use crate::fs::bio::{BCSTATS, DirtyPick, DirtyScan, VnodeBufs, buf_undirty, vnode_dirty_scan};
use crate::fs::buf::{
    B_ASYNC, B_BUSY, B_DELWRI, B_DONE, B_ERROR, B_NEEDCOMMIT, B_NOCACHE, B_READ, BufRef,
};
use crate::fs::bufq::{BufqDiscipline, bufq_unpark};
use crate::fs::subr::vfs;
use crate::proc::ProcGlue;
use crate::sys::nfs_nfs::NFS_MAXASYNCDAEMON;
use crate::sys::signal::SIGKILL;

/// The number of slots of `nfs_asyncdaemon[]`.
pub const NFS_NASYNCDAEMON: usize = NFS_MAXASYNCDAEMON as usize;

/// A running nfsiod's place: its slot of `nfs_asyncdaemon[]` (`myiod`) and the buffers it
/// added to `nfs_bufqmax` (`bufcount`), which its end takes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NfsIod {
    /// `myiod`.
    pub myiod: usize,
    /// `bufcount`.
    pub bufcount: u32,
}

/// `nfssvc_iod`'s start, before its loop: the thread `tid` takes the first free slot of
/// `nfs_asyncdaemon[]` (`EBUSY`, its `kthread_exit` status, when every slot is taken) and
/// counts in `nfs_numasync`; `nfs_bufqmax` is held to a quarter of the buffer cache's kva
/// slots and of its buffers, and grows by this nfsiod's share (at most 256, an eighth of
/// either, nothing when the quarter clamped it); whoever waits for room on the queue is woken.
pub fn nfs_iod_attach<G: ProcGlue>(tid: Pid) -> Result<NfsIod, Errno> {
    let kvaslots = BCSTATS.kvaslots.load(Ordering::Relaxed);
    let numbufs = BCSTATS.numbufs.load(Ordering::Relaxed);
    let mut bufcount = 256.min(kvaslots / 8).min(numbufs / 8);

    // Assign my position or return error if too many already running.
    let t = &vfs::<G>().nfs;
    let Some(myiod) = t.nfs_asyncdaemon.iter().position(|slot| {
        slot.compare_exchange(0, tid, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }) else {
        return Err(Errno::EBUSY);
    };
    NFS_NUMASYNC.fetch_add(1, Ordering::Relaxed);

    // Upper limit on how many bufs we'll queue up for this iod.
    if i64::from(NFS_BUFQMAX.load(Ordering::Relaxed)) > kvaslots / 4 {
        NFS_BUFQMAX.store(clamp_u32(kvaslots / 4), Ordering::Relaxed);
        bufcount = 0;
    }
    if i64::from(NFS_BUFQMAX.load(Ordering::Relaxed)) > numbufs / 4 {
        NFS_BUFQMAX.store(clamp_u32(numbufs / 4), Ordering::Relaxed);
        bufcount = 0;
    }

    let bufcount = clamp_u32(bufcount);
    NFS_BUFQMAX.fetch_add(bufcount, Ordering::Relaxed);
    // wake up anyone waiting for room to enqueue IO
    wakeup(ptr::from_ref(&NFS_BUFQLEN));
    Ok(NfsIod { myiod, bufcount })
}

/// `uint32_t`'s value of a count of the buffer cache's (`nfs_bufqmax` is unsigned): a
/// negative one is 0.
fn clamp_u32(n: i64) -> u32 {
    u32::try_from(n.max(0)).unwrap_or(u32::MAX)
}

/// `nfssvc_iod`'s end, before its `kthread_exit(error)`: the slot freed, the count and the
/// queue's limit taken back.
pub fn nfs_iod_detach<G: ProcGlue>(iod: NfsIod) {
    vfs::<G>().nfs.nfs_asyncdaemon[iod.myiod].store(0, Ordering::Relaxed);
    NFS_NUMASYNC.fetch_sub(1, Ordering::Relaxed);
    NFS_BUFQMAX.fetch_sub(iod.bufcount, Ordering::Relaxed);
}

/// The channel an idle nfsiod sleeps on (`&nfs_bufq`, "nfsidl"), which `nfs_asyncio` wakes.
pub fn nfs_bufq_chan<G: ProcGlue>() -> *const () {
    ptr::from_ref(&vfs::<G>().nfs.nfs_bufq).cast()
}

/// Whether `nfs_bufq` holds a buffer (`TAILQ_FIRST(&nfs_bufq) != NULL`).
pub fn nfs_bufq_peek<G: ProcGlue>() -> bool {
    let r = klock::reader();
    vfs::<G>().nfs.nfs_bufq.get(&r).peek()
}

/// Take one off the front of the list (`TAILQ_REMOVE` of `TAILQ_FIRST(&nfs_bufq)`,
/// `nfs_bufqlen--`, and one waiter for room woken): the buffer's token, which `nfs_asyncio`
/// parked in it; `None` for an empty queue.
pub fn nfs_bufq_get<G: ProcGlue>() -> Option<BufRef> {
    let m = {
        let mut w = klock::witness();
        let m = vfs::<G>().nfs.nfs_bufq.get_mut(&mut w).dequeue()?;
        NFS_BUFQLEN.fetch_sub(1, Ordering::Relaxed);
        m
    };
    wakeup_one(ptr::from_ref(&NFS_BUFQLEN));
    Some(bufq_unpark(m))
}

/// The delayed write `nfssvc_iod` does beside a write of the vnode whose buffers `vb` are:
/// "Look for a delayed write for the same vnode, so I can do it now. We must grab it before
/// calling `nfs_doio()` to avoid any risk of the vnode getting vclean()'d while we are doing
/// the write rpc." The first dirty buffer that is delayed and neither busy, to be committed,
/// nor uncached is taken (`B_ASYNC` set) and given the first part of `nfs_bwrite()` up to,
/// but not including, `nfs_strategy()`: `B_READ`, `B_DONE` and `B_ERROR` cleared, made clean,
/// one more write in progress on the vnode.
pub fn nfs_iod_nextwrite(vb: &'static VnodeBufs) -> Option<BufRef> {
    let pick = |bp: &crate::fs::buf::Buf| {
        if bp.flags() & (B_BUSY | B_DELWRI | B_NEEDCOMMIT | B_NOCACHE) != B_DELWRI {
            return DirtyPick::Skip;
        }
        bp.set(B_ASYNC);
        DirtyPick::Take
    };
    let DirtyScan::Taken(t) = vnode_dirty_scan(vb, "nfsiod", 0, hal::sync::sleep::INFSLP, pick)
    else {
        return None;
    };
    t.buf().clr(B_READ | B_DONE | B_ERROR);
    buf_undirty(&t);
    vb.numoutput_inc();
    Some(t)
}

/// `nfssvc_iod(arg)` (a `fork::Start::NfsIod` thread): an asynchronous I/O thread for client
/// nfs. It does read-ahead and write-behind operations on the block I/O cache: each buffer
/// `nfs_asyncio` queues, and beside a write the delayed writes of the same vnode. Never
/// returns: it exits when it fails (`EBUSY` with every slot taken) or gets killed (the
/// signal ends its idle sleep).
pub fn nfssvc_iod<G: ProcGlue>() -> ! {
    let tid = G::with_curproc(|p| match p {
        Some(p) => p.p_tid.get(),
        None => kpanic(format_args!("nfssvc_iod: no thread")),
    });

    // Assign my position or return error if too many already running.
    let iod = match nfs_iod_attach::<G>(tid) {
        Ok(iod) => iod,
        Err(e) => crate::thread::kthread_exit::<G>(e as i32),
    };

    // Just loop around doin our stuff until SIGKILL.
    let mut error = Ok(());
    loop {
        while !nfs_bufq_peek::<G>() && error.is_ok() {
            error = tsleep_nsec(nfs_bufq_chan::<G>(), PWAIT | PCATCH, "nfsidl", INFSLP);
        }
        while let Some(mut t) = nfs_bufq_get::<G>() {
            if t.buf().isset(B_READ) {
                let _ = nfs_doio::<G>(t, None);
                continue;
            }
            loop {
                // Look for a delayed write for the same vnode, so I can do it now, grabbed
                // before nfs_doio() (`nfs_iod_nextwrite`).
                let Some(vb) = t.buf().vnode_bufs() else {
                    kpanic(format_args!(
                        "nfssvc_iod: buffer {:p} without b_vp",
                        t.buf()
                    ));
                };
                let nbp = nfs_iod_nextwrite(vb);
                let _ = nfs_doio::<G>(t, None);
                match nbp {
                    Some(n) => t = n,
                    None => break,
                }
            }
        }
        if let Err(e) = error {
            nfs_iod_detach::<G>(iod);
            crate::thread::kthread_exit::<G>(e as i32);
        }
    }
}

/// `nfs_getset_niothreads(set)`: with `set`, the number of nfsiods asked for
/// (`nfs_niothreads`, clamped to `0..=NFS_MAXASYNCDAEMON`) is made true: the missing ones
/// are started, a surplus one is sent `SIGKILL` (found by its thread id under the kernel
/// lock); without, `nfs_niothreads` reads back how many run (unless it is still `-1`, none
/// started yet).
pub fn nfs_getset_niothreads<G: ProcGlue>(set: bool) {
    let slots = &vfs::<G>().nfs.nfs_asyncdaemon;
    let have = slots
        .iter()
        .filter(|d| d.load(Ordering::Relaxed) != 0)
        .count() as i32;

    if set {
        // clamp to sane range
        let n = nfs_niothreads
            .load(Ordering::Relaxed)
            .clamp(0, NFS_MAXASYNCDAEMON);
        nfs_niothreads.store(n, Ordering::Relaxed);

        let mut start = n - have;
        while start > 0 {
            let _ = crate::thread::kthread_create::<G>(b"nfsio", crate::fork::Start::NfsIod);
            start -= 1;
        }

        let r = klock::reader();
        for d in slots {
            if start >= 0 {
                break;
            }
            let tid = d.load(Ordering::Relaxed);
            if tid != 0 {
                let p = G::tables()
                    .tidhash(tid)
                    .iter_pinned(&r)
                    .find(|p| p.p_tid.get() == tid);
                if let Some(p) = p {
                    crate::sig::psignal(p, SIGKILL);
                }
                start += 1;
            }
        }
    } else if nfs_niothreads.load(Ordering::Relaxed) >= 0 {
        nfs_niothreads.store(have, Ordering::Relaxed);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the nfsiods' table and queue limit (start, end, the twenty-first
    // refused) and of the delayed write an nfsiod picks up beside a write.

    use super::*;
    use crate::fs::bio::{bdwrite, brelse, getblk_wait, vnode_dirty_for_each};
    use crate::fs::nfs::nfs_node::{test_mount, test_node};
    use crate::fs::testfs::TestGlue as G;
    use crate::fs::vnode::VREG;

    /// The buffer cache's counts set for a test, put back at its end.
    struct Counts(i64, i64);

    impl Counts {
        fn set(kvaslots: i64, numbufs: i64) -> Self {
            let old = Counts(
                BCSTATS.kvaslots.load(Ordering::Relaxed),
                BCSTATS.numbufs.load(Ordering::Relaxed),
            );
            BCSTATS.kvaslots.store(kvaslots, Ordering::Relaxed);
            BCSTATS.numbufs.store(numbufs, Ordering::Relaxed);
            old
        }
    }

    impl Drop for Counts {
        fn drop(&mut self) {
            BCSTATS.kvaslots.store(self.0, Ordering::Relaxed);
            BCSTATS.numbufs.store(self.1, Ordering::Relaxed);
        }
    }

    /// Each nfsiod takes its own slot and adds its share to `nfs_bufqmax`: 256 at most, an
    /// eighth of the kva slots or of the buffers; a limit above a quarter of either is cut
    /// there and that nfsiod adds nothing. The twenty-first is refused (`EBUSY`); an end
    /// gives the slot and the share back.
    #[test]
    fn the_nfsiods_take_slots_and_their_share_of_the_queue() {
        let _t = test_mount(0);
        let _c = Counts::set(4096, 1000);
        let mut iods = std::vec::Vec::new();
        for tid in 1..=NFS_NASYNCDAEMON as i32 {
            let iod = nfs_iod_attach::<G>(100 + tid).expect("a slot");
            assert_eq!(iod.myiod, (tid - 1) as usize);
            iods.push(iod);
        }
        // min(256, 4096 / 8, 1000 / 8) = 125 each, until the limit passes 1000 / 4 = 250.
        assert_eq!(iods[0].bufcount, 125);
        assert_eq!(iods[1].bufcount, 125);
        // The third saw 250 > 250? no: it adds its 125 (375); the fourth cuts to 250.
        assert_eq!(iods[2].bufcount, 125);
        assert_eq!(iods[3].bufcount, 0);
        assert_eq!(
            NFS_NUMASYNC.load(Ordering::Relaxed),
            NFS_NASYNCDAEMON as i32
        );
        assert_eq!(
            nfs_iod_attach::<G>(999),
            Err(Errno::EBUSY),
            "every slot is taken"
        );
        assert_eq!(
            NFS_NUMASYNC.load(Ordering::Relaxed),
            NFS_NASYNCDAEMON as i32
        );
        let t = &vfs::<G>().nfs;
        assert_eq!(t.nfs_asyncdaemon[4].load(Ordering::Relaxed), 105);
        let max = NFS_BUFQMAX.load(Ordering::Relaxed);
        nfs_iod_detach::<G>(iods[4]);
        assert_eq!(t.nfs_asyncdaemon[4].load(Ordering::Relaxed), 0);
        assert_eq!(NFS_BUFQMAX.load(Ordering::Relaxed), max - iods[4].bufcount);
        let again = nfs_iod_attach::<G>(555).expect("the freed slot");
        assert_eq!(again.myiod, 4);
        for iod in iods.iter().filter(|i| i.myiod != 4).chain([&again]) {
            nfs_iod_detach::<G>(*iod);
        }
        assert_eq!(NFS_NUMASYNC.load(Ordering::Relaxed), 0);
        assert!(
            t.nfs_asyncdaemon
                .iter()
                .all(|s| s.load(Ordering::Relaxed) == 0)
        );
    }

    /// The limit's two cuts: above a quarter of the kva slots, then above a quarter of the
    /// buffers; a cut nfsiod adds nothing.
    #[test]
    fn the_queue_limit_is_cut_to_a_quarter_of_the_cache() {
        let _t = test_mount(0);
        for (kva, bufs, before, after, share) in [
            (4096, 4096, 0, 256, 256),
            (4096, 4096, 2000, 1024, 0),
            (4096, 400, 500, 100, 0),
            (64, 4096, 0, 8, 8),
            (64, 4096, 17, 16, 0),
            (0, 0, 0, 0, 0),
        ] {
            let _c = Counts::set(kva, bufs);
            NFS_BUFQMAX.store(before, Ordering::Relaxed);
            let iod = nfs_iod_attach::<G>(7).expect("a slot");
            assert_eq!(iod.bufcount, share, "{kva} {bufs} {before}");
            assert_eq!(
                NFS_BUFQMAX.load(Ordering::Relaxed),
                after,
                "{kva} {bufs} {before}"
            );
            nfs_iod_detach::<G>(iod);
        }
        NFS_BUFQMAX.store(0, Ordering::Relaxed);
    }

    /// The delayed write picked beside a write: the first dirty buffer that is delayed and
    /// neither busy, to be committed, nor uncached, taken busy and asynchronous, its read,
    /// done and error bits cleared, made clean, one more write in progress on its vnode.
    #[test]
    fn an_nfsiod_picks_the_first_idle_delayed_write() {
        let t = test_mount(0);
        let (vp, _np) = test_node(t.mp, VREG, &[5; 8]);
        assert!(nfs_iod_nextwrite(&vp.bufs).is_none(), "nothing dirty");
        for blk in 0..4 {
            bdwrite(getblk_wait(&vp.bufs, blk * 16, 8192));
        }
        vnode_dirty_for_each(&vp.bufs, |bp| match bp.lblkno() {
            0 => bp.set(B_NEEDCOMMIT),
            16 => bp.set(B_NOCACHE),
            _ => {}
        });
        let busy = getblk_wait(&vp.bufs, 32, 8192);
        let out = vp.bufs.numoutput();
        let tk = nfs_iod_nextwrite(&vp.bufs).expect("block 48");
        let bp = tk.buf();
        assert_eq!(bp.lblkno(), 48);
        assert!(bp.isset(B_ASYNC | B_BUSY));
        assert!(!bp.isset(B_DELWRI) && !bp.isset(B_READ | B_DONE | B_ERROR));
        assert_eq!(vp.bufs.numoutput(), out + 1);
        let mut dirty = std::vec::Vec::new();
        vnode_dirty_for_each(&vp.bufs, |b| dirty.push(b.lblkno()));
        dirty.sort_unstable();
        assert_eq!(dirty, [0, 16, 32], "block 48 is clean now");
        assert!(nfs_iod_nextwrite(&vp.bufs).is_none(), "none left to pick");
        brelse(busy);
        bp.clr(B_ASYNC);
        brelse(tk);
        vnode_dirty_for_each(&vp.bufs, |b| b.clr(B_NEEDCOMMIT | B_NOCACHE));
    }
}
/* </TESTS> */
