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
 *	@(#)mfsnode.h	8.2 (Berkeley) 8/11/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/mfs/mfsnode.h>`: the control data of the memory based file system (`struct
//! mfsnode`), the node of the block device vnode `mfs_mount` makes for the file system's
//! "disk", whose blocks live in the memory of the process that called `mount(2)`.
//!
//! Upstream: sys/ufs/mfs/mfsnode.h @ 3ce1f3f79392
//! LZ: sys/ufs/mfs/mfsnode.rs@3c62ede68802
//!
//! ## Deviations
//! - `mfs_bufq` (`struct bufq`, `bufq_init(BUFQ_FIFO)`) is the fifo discipline itself
//!   (`fs::bufq::Fifo`, the C's `bufq_fifo`) under the node's own mutex at `IPL_BIO` (the C's
//!   `bufq_mtx`), not a queue on the global registry `bufqs`: a registered queue must outlive
//!   every buffer's back link (`b_bq`), and the node dies at reclaim. What the registry adds
//!   and this queue lacks: `bufq_quiesce`'s stop for suspend (no suspend path on arm64) and the
//!   outstanding count, through which the C's `bwrite` throttles a writer once `bufq_hi`
//!   buffers are queued on the file system (`bufq_wait`); nothing userland sees but the
//!   timing of a writer above that mark (D-N5.5b-3).
//! - `mfs_buflist` ("list of I/O requests") is gone: nothing in the C reads or writes it.
//! - `mfs_baseoff` is a user address (`usize`) of the process that mounted the file system,
//!   which `mfs_doio` reaches with `copyin`/`copyout`; `mfs_size` is the C's `long` (`i64`).
//! - `VTOMFS` ([`vtomfs`]) checks the vnode's tag and that its node is an mfsnode (LZ's
//!   check) and answers a counted handle ([`MfsnodeRef`]).
//!
//! ## Redesign
//! - LZ's `Cell` members, `malloc(M_MFSNODE)` written with `ptr::write` into `v_data` and cast
//!   back by `vtomfs` (an `unsafe` deref giving every caller a `&'static Mfsnode`) -> an
//!   [`Mfsnode`] built whole by `mfs_mount` (its members are written once, before the vnode
//!   is used, as the C fills them before `ffs_mountfs`) in a counted `malloc(M_MFSNODE)` block
//!   ([`MfsnodeRef`], `hal::malloc::MArc`) erased into the device vnode's typed slot
//!   (`Vnode::set_node`) where the C sets `v_data`; `mfs_reclaim` takes it back
//!   (`Vnode::take_node`) where the C frees it, and the last handle frees the block (the
//!   vnode's, or `mfs_start`'s, which the file system's process holds until it returns).
//!   `mfs_shutdown` is atomic: `mfs_close` sets it (any process), `mfs_start` reads it.
//! - Cost against the C: `VTOMFS` is one read of the slot and a count pair where the C loads
//!   `v_data` (`mfs_strategy` per request, `mfs_close`, `mfs_inactive`, `mfs_reclaim`,
//!   `mfs_start` once); the queue's mutex is the C's `bufq_mtx` (one section per queue and
//!   dequeue, as `bufq_queue`/`bufq_dequeue`).

use core::sync::atomic::{AtomicI32, Ordering};

use hal::intr::{IPL_BIO, splraise};
use hal::malloc::{M_WAITOK, MArc};
use hal::mutex::Mutexed;
use hal::sys::errno::Errno;
use hal::sys::types::Pid;

use crate::assert::kpanic;
use crate::fs::bio::biodone;
use crate::fs::buf::{B_ERROR, BufRef};
use crate::fs::bufq::{BufqDiscipline, Fifo, bufq_park, bufq_unpark};
use crate::fs::vnode::{VT_MFS, Vnode};
use crate::proc::ProcGlue;

/// `M_MFSNODE`: the malloc type of the mfsnodes (`<sys/malloc.h>`; `vmstat -m`'s `MFS node`).
pub const M_MFSNODE: i32 = 46;

/// `struct mfsnode`: this structure defines the control data for the memory based file system.
pub struct Mfsnode<G: ProcGlue> {
    /// `mfs_vnode`: vnode associated with this mfsnode.
    pub mfs_vnode: &'static Vnode<G>,
    /// `mfs_bufq`: the queue of I/O requests for the file system's process (the deviations).
    queue: Mutexed<Fifo>,
    /// `mfs_baseoff`: base of file system in memory (a user address of the process that
    /// mounted it).
    pub mfs_baseoff: usize,
    /// `mfs_size`: size of memory file system.
    pub mfs_size: i64,
    /// `mfs_tid`: supporting thread's tid.
    pub mfs_tid: Pid,
    /// `mfs_shutdown`: shutdown request.
    pub mfs_shutdown: AtomicI32,
}

/// `struct mfsnode *` held: a counted handle to a device vnode's node, in a block of
/// `malloc(M_MFSNODE)` that the last handle frees. The device vnode's typed slot holds one
/// until reclaim, `mfs_start` one while it serves, each operation reached from the vnode one
/// ([`vtomfs`]).
pub type MfsnodeRef<G> = MArc<Mfsnode<G>>;

impl<G: ProcGlue> Mfsnode<G> {
    /// `mfsp = malloc(sizeof *mfsp, M_MFSNODE, M_WAITOK | M_ZERO)` and its members set
    /// (`mfs_mount`): the node of the device vnode `vp` over the `size` bytes at the user
    /// address `base` of the thread `tid`, its queue empty (`bufq_init(BUFQ_FIFO)`), in its
    /// counted block, one handle. `M_WAITOK` sleeps for the memory and never fails, as the
    /// C's; no `M_ZERO`: the block is written whole.
    pub fn new(vp: &'static Vnode<G>, base: usize, size: i64, tid: Pid) -> MfsnodeRef<G> {
        let node = Self {
            mfs_vnode: vp,
            queue: Mutexed::new(IPL_BIO, Fifo::new()),
            mfs_baseoff: base,
            mfs_size: size,
            mfs_tid: tid,
            mfs_shutdown: AtomicI32::new(0),
        };
        match MArc::new(M_MFSNODE, M_WAITOK, node) {
            Some(n) => n,
            None => kpanic(format_args!("mfs_mount: malloc(M_MFSNODE) failed")),
        }
    }

    /// `bufq_queue(&mfsp->mfs_bufq, bp)`: the request's token parked in its buffer, the buffer
    /// at the queue's tail.
    pub fn queue(&self, t: BufRef) {
        let m = bufq_park(t);
        self.queue.lock().queue(m);
    }

    /// `bufq_dequeue(&mfsp->mfs_bufq)`: the request at the queue's head, its token taken back.
    pub fn dequeue(&self) -> Option<BufRef> {
        let m = self.queue.lock().dequeue()?;
        Some(bufq_unpark(m))
    }

    /// `bufq_peek(&mfsp->mfs_bufq)`: whether a request is queued.
    pub fn queued(&self) -> bool {
        self.queue.lock().peek()
    }

    /// `bufq_destroy(&mfsp->mfs_bufq)`'s drain: every request still queued fails with `ENXIO`,
    /// `biodone` at `splbio` as an interrupt would call it.
    pub fn drain(&self) {
        while let Some(t) = self.dequeue() {
            t.buf().set_error(Some(Errno::ENXIO));
            t.buf().set(B_ERROR);
            let s = splraise(IPL_BIO);
            biodone(t);
            drop(s);
        }
    }

    /// `mfsp->mfs_shutdown == 1`.
    pub fn shutdown(&self) -> bool {
        self.mfs_shutdown.load(Ordering::Relaxed) == 1
    }
}

/// `VTOMFS(vp)`: convert a vnode pointer to an mfsnode pointer, held. `panic(9)` when the
/// vnode is not an mfs device vnode with its node (LZ's check).
pub fn vtomfs<G: ProcGlue>(vp: &Vnode<G>) -> MfsnodeRef<G> {
    match vp.node_arc::<Mfsnode<G>>() {
        Some(n) if vp.tag() == VT_MFS => n,
        _ => kpanic(format_args!("VTOMFS: vnode {vp:p} is not an mfs node")),
    }
}

/// `MFSTOV(mfsp)`: convert an mfsnode pointer to a vnode pointer.
pub fn mfstov<G: ProcGlue>(mfsp: &Mfsnode<G>) -> &'static Vnode<G> {
    mfsp.mfs_vnode
}
/* </CODE> */
