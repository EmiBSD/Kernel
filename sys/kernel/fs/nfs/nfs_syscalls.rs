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
//! the buffers `nfs_asyncio` queues and does their RPCs (`nfs_doio`). `nfssvc_iod`'s loop,
//! which calls `nfs_doio`, and `nfs_getset_niothreads`, which starts it, come with
//! `nfs_doio` (N6.7a3), over these pieces; `nfssvc(2)` and the server's half (`nfssvc_nfsd`,
//! `nfsrv_*`) stay `bsd`'s until N6.7b (D-5).
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

use core::ptr;
use core::sync::atomic::Ordering;

use hal::sleep::{wakeup, wakeup_one};
use hal::sys::errno::Errno;
use hal::sys::types::Pid;

use super::nfsnode::{NFS_BUFQLEN, NFS_BUFQMAX, NFS_NUMASYNC};
use crate::fs::bio::{BCSTATS, DirtyPick, DirtyScan, VnodeBufs, buf_undirty, vnode_dirty_scan};
use crate::fs::buf::{
    B_ASYNC, B_BUSY, B_DELWRI, B_DONE, B_ERROR, B_NEEDCOMMIT, B_NOCACHE, B_READ, BufRef,
};
use crate::fs::bufq::{BufqDiscipline, bufq_unpark};
use crate::fs::subr::vfs;
use crate::proc::ProcGlue;
use crate::sys::nfs_nfs::NFS_MAXASYNCDAEMON;

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
    vfs::<G>().nfs.nfs_bufq.lock().peek()
}

/// Take one off the front of the list (`TAILQ_REMOVE` of `TAILQ_FIRST(&nfs_bufq)`,
/// `nfs_bufqlen--`, and one waiter for room woken): the buffer's token, which `nfs_asyncio`
/// parked in it; `None` for an empty queue.
pub fn nfs_bufq_get<G: ProcGlue>() -> Option<BufRef> {
    let m = vfs::<G>().nfs.nfs_bufq.lock().dequeue()?;
    NFS_BUFQLEN.fetch_sub(1, Ordering::Relaxed);
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
    let DirtyScan::Taken(t) = vnode_dirty_scan(vb, "nfsiod", 0, hal::sleep::INFSLP, pick) else {
        return None;
    };
    t.buf().clr(B_READ | B_DONE | B_ERROR);
    buf_undirty(&t);
    vb.numoutput_inc();
    Some(t)
}
/* </CODE> */
