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
 * Copyright (c) 2015 David Gwynne <dlg@openbsd.org>
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
/* </LICENSES> */

/* <CODE> */
//! `<net/ifq.h>` for `bsd`'s network code: the interface send and input queues are the
//! kernel's (`kernel::net::ifq`, N6.3a), re-exported here; the functions that take or give
//! a packet forward over `bsd`'s parked mbufs, and `struct ifq_ops` stays `bsd`'s for pf's
//! conditioners (hfsc, fq_codel), each table a kernel `QueueDiscipline` (bridges.toml's row,
//! paid by N8).
//!
//! Upstream: sys/net/ifq.h @ 3ce1f3f79392
//! Upstream: sys/net/ifq.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `struct ifq_ops` is a table of `unsafe fn`s: each operation trusts that the state word
//!   it is given was made by the same table's `ifqop_alloc` and that `ifq_mtx` is held (LZ's
//!   contract). The kernel keeps the word beside the table and lends the locked queue
//!   (`IfqLocked`) where the C's operations read `ifq->ifq_q` themselves.
//! - `ifqop_deq_commit` is given the packet `ifqop_deq_begin` returned: the kernel's commit
//!   names it by its cookie alone, so the table's `ifqop_deq_begin` runs once more under the
//!   same hold to find it (pf's conditioners only, N8's).
//! - `ifq_priq_ops` is the kernel's own conditioner (`ifq_attach_priq`), not a table.
//! - `kstat(4)` is not ported (LZ's).

use core::ffi::c_void;
use core::pin::Pin;

use kernel::net::ifq::QueueDiscipline;
use kernel::net::mbuf::MbufBox;

use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, MbufList, park, unpark};
use crate::sys::task::Task;

pub use kernel::net::ifq::{
    IFIQ_MAXLEN_DROP, IFIQ_MAXLEN_RETURN, IFQ_MAXLEN, Ifiqueue, IfqDeq, IfqLocked, Ifqueue,
    ifiq_add_data, ifiq_destroy, ifiq_empty, ifiq_len, ifq_add_data, ifq_attach_priq, ifq_barrier,
    ifq_clr_oactive, ifq_destroy, ifq_empty, ifq_hdatalen, ifq_idx, ifq_inc_errors,
    ifq_init_maxlen, ifq_is_oactive, ifq_is_priq, ifq_len, ifq_purge, ifq_restart, ifq_set_oactive,
    ifq_start,
};

/// `unsigned int (*ifqop_idx)(unsigned int, const struct mbuf *)`.
pub type IfqopIdxFn = fn(u32, &Mbuf) -> u32;
/// `struct mbuf *(*ifqop_enq)(struct ifqueue *, struct mbuf *)`: keeps `m` and returns NULL,
/// or returns the packet to drop (`m` itself, or one it evicts).
///
/// # Safety
///
/// `q` was made by the same table's `ifqop_alloc` and is still the queue's; `st` is the
/// queue's `ifq_mtx` held.
pub type IfqopEnqFn =
    unsafe fn(*mut c_void, &mut IfqLocked<'_>, &'static Mbuf) -> Option<&'static Mbuf>;
/// `struct mbuf *(*ifqop_deq_begin)(struct ifqueue *, void **)`: the next packet and its
/// cookie for `ifqop_deq_commit`, without removing it.
///
/// # Safety
///
/// As for [`IfqopEnqFn`].
pub type IfqopDeqBeginFn =
    unsafe fn(*mut c_void, &mut IfqLocked<'_>) -> Option<(&'static Mbuf, *mut c_void)>;
/// `void (*ifqop_deq_commit)(struct ifqueue *, struct mbuf *, void *)`: removes the packet
/// `ifqop_deq_begin` returned.
///
/// # Safety
///
/// As for [`IfqopEnqFn`]; `m` and `cookie` come from `ifqop_deq_begin` under the same hold of
/// `ifq_mtx`.
pub type IfqopDeqCommitFn = unsafe fn(*mut c_void, &mut IfqLocked<'_>, &'static Mbuf, *mut c_void);
/// `void (*ifqop_purge)(struct ifqueue *, struct mbuf_list *)`: moves every packet to `ml`.
///
/// # Safety
///
/// As for [`IfqopEnqFn`].
pub type IfqopPurgeFn = unsafe fn(*mut c_void, &mut IfqLocked<'_>, &MbufList);
/// `void *(*ifqop_alloc)(unsigned int, void *)`: the conditioner's state for queue `idx`.
pub type IfqopAllocFn = fn(u32, *mut c_void) -> *mut c_void;
/// `void (*ifqop_free)(unsigned int, void *)`.
///
/// # Safety
///
/// The state came from the same table's `ifqop_alloc` and is no longer used.
pub type IfqopFreeFn = unsafe fn(u32, *mut c_void);

/// `struct ifq_ops`: a traffic conditioner of pf's.
pub struct IfqOps {
    /// `ifqop_idx`: picks the send queue of a packet.
    pub ifqop_idx: IfqopIdxFn,
    /// `ifqop_enq`.
    pub ifqop_enq: IfqopEnqFn,
    /// `ifqop_deq_begin`.
    pub ifqop_deq_begin: IfqopDeqBeginFn,
    /// `ifqop_deq_commit`.
    pub ifqop_deq_commit: IfqopDeqCommitFn,
    /// `ifqop_purge`.
    pub ifqop_purge: IfqopPurgeFn,
    /// `ifqop_alloc`.
    pub ifqop_alloc: IfqopAllocFn,
    /// `ifqop_free`.
    pub ifqop_free: IfqopFreeFn,
}

/// The state word as the table's operations take it (its address, exposed by `alloc`).
fn word(q: usize) -> *mut c_void {
    core::ptr::with_exposed_provenance_mut(q)
}

impl QueueDiscipline for IfqOps {
    fn idx(&self, nqueues: u32, m: &Mbuf) -> u32 {
        (self.ifqop_idx)(nqueues, m)
    }

    fn alloc(&self, idx: u32, arg: usize) -> usize {
        (self.ifqop_alloc)(idx, word(arg)).expose_provenance()
    }

    fn free(&self, idx: u32, q: usize) {
        // SAFETY: the kernel gives back the word this table's `alloc` made for the queue
        // (`ifq_attach` keeps the pair together), once it is off the queue (`ifq_attach`'s
        // replacement, `ifq_destroy`): `ifqop_free`'s contract.
        unsafe { (self.ifqop_free)(idx, word(q)) }
    }

    fn enq(&self, q: usize, st: &mut IfqLocked<'_>, m: MbufBox) -> Option<MbufBox> {
        // SAFETY: `q` is this table's word for the queue `st` locks (the kernel's pairing in
        // `Discipline::Ops`, made by `ifq_attach` from this table's `alloc`), and `st` is the
        // queue's `ifq_mtx` held: `ifqop_enq`'s contract.
        unsafe { (self.ifqop_enq)(word(q), st, park(m)) }.map(unpark)
    }

    fn deq_begin<'a>(&self, q: usize, st: &'a mut IfqLocked<'_>) -> Option<(&'a Mbuf, usize)> {
        // SAFETY: as in `enq`.
        unsafe { (self.ifqop_deq_begin)(word(q), st) }
            .map(|(m, cookie)| (m, cookie.expose_provenance()))
    }

    fn deq_commit(&self, q: usize, st: &mut IfqLocked<'_>, cookie: usize) -> MbufBox {
        // SAFETY: as in `enq`; the packet is `deq_begin`'s again, under the same hold of
        // `ifq_mtx` as the kernel's begin that gave `cookie` (the module's deviations), so
        // the pair is the one `ifqop_deq_commit` takes.
        let begun = unsafe { (self.ifqop_deq_begin)(word(q), st) };
        let Some((m, _)) = begun else {
            crate::kern::subr_prf::panic(format_args!("ifqop_deq_commit: no packet"));
        };
        // SAFETY: as above.
        unsafe { (self.ifqop_deq_commit)(word(q), st, m, word(cookie)) };
        unpark(m)
    }

    fn purge(&self, q: usize, st: &mut IfqLocked<'_>, ml: &mut kernel::net::mbuf::MbufList) {
        let out = MbufList::new();
        // SAFETY: as in `enq`.
        unsafe { (self.ifqop_purge)(word(q), st, &out) };
        out.change(|l| kernel::net::mbuf::ml_enlist(ml, l));
    }
}

/// `net_ifiq_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: `net.link.ifrxq`.
pub fn net_ifiq_sysctl(
    name: &[i32],
    _oldp: usize,
    _oldlenp: Option<&mut usize>,
    _newp: usize,
    _newlen: usize,
) -> Result<(), Errno> {
    kernel::net::ifq::net_ifiq_sysctl(name)
}

/// `ifq_attach(ifq, ops, opsarg)`: the queue's conditioner replaced by the table `ops` with
/// its state made from `opsarg`.
pub fn ifq_attach(ifq: &Ifqueue, ops: &'static IfqOps, opsarg: *mut c_void) {
    kernel::net::ifq::ifq_attach(ifq, ops, opsarg.expose_provenance());
}

/// `ifq_q_enter(ifq, ops)` ... `ifq_q_leave(ifq, q)`: `f` on the conditioner's state word
/// with `ifq_mtx` held, if `ops` is the queue's conditioner; `None` otherwise.
pub fn ifq_q_with<R>(
    ifq: &Ifqueue,
    ops: &'static IfqOps,
    f: impl FnOnce(*mut c_void, &mut IfqLocked<'_>) -> R,
) -> Option<R> {
    kernel::net::ifq::ifq_q_with(ifq, ops, |q, st| f(word(q), st))
}

/// `ifq_mfreem(ifq, m)`: a packet a conditioner drops, freed after `ifq_mtx`.
pub fn ifq_mfreem(st: &mut IfqLocked<'_>, m: &'static Mbuf) {
    st.mfreem(unpark(m));
}

/// `ifq_mfreeml(ifq, ml)`: the packets a conditioner drops, freed after `ifq_mtx`.
pub fn ifq_mfreeml(st: &mut IfqLocked<'_>, ml: &MbufList) {
    ml.change(|l| st.mfreeml(l));
}

/// `ifq_serialize(ifq, t)`: the task `t` run by the queue's serialiser.
pub fn ifq_serialize(ifq: &Ifqueue, t: &'static Task) {
    kernel::net::ifq::ifq_serialize(ifq, Pin::static_ref(&t.t_node));
}

/// `ifq_enqueue(ifq, m)`: `m` on the send queue; `ENOBUFS` when it was dropped (and freed).
pub fn ifq_enqueue(ifq: &Ifqueue, m: &'static Mbuf) -> Result<(), Errno> {
    kernel::net::ifq::ifq_enqueue(ifq, unpark(m))
}

/// `ifq_dequeue(ifq)`: the next packet, taken off the queue.
pub fn ifq_dequeue(ifq: &Ifqueue) -> Option<&'static Mbuf> {
    kernel::net::ifq::ifq_dequeue(ifq).map(park)
}

/// `ifq_deq_with`'s `bsd` form: `f` sees the next packet with `ifq_mtx` held (the C's
/// `ifq_deq_begin`), the packet taken off on `IfqDeq::Commit` (`ifq_deq_commit`).
pub fn ifq_deq_with(ifq: &Ifqueue, f: impl FnOnce(&Mbuf) -> IfqDeq) -> Option<&'static Mbuf> {
    kernel::net::ifq::ifq_deq_with(ifq, f).map(park)
}

/// `ifiq_input(ifiq, ml)`: a driver's received packets onto the input queue; `true` under
/// pressure.
pub fn ifiq_input(ifiq: &'static Ifiqueue, ml: &MbufList) -> bool {
    ml.change(|l| kernel::net::ifq::ifiq_input(ifiq, l))
}

/// `ifiq_enqueue(ifiq, m)`.
pub fn ifiq_enqueue(ifiq: &'static Ifiqueue, m: &'static Mbuf) -> Result<(), Errno> {
    kernel::net::ifq::ifiq_enqueue(ifiq, unpark(m))
}

/// `ifiq_enqueue_qlim(ifiq, m, qlim)`.
pub fn ifiq_enqueue_qlim(
    ifiq: &'static Ifiqueue,
    m: &'static Mbuf,
    qlim: u32,
) -> Result<(), Errno> {
    kernel::net::ifq::ifiq_enqueue_qlim(ifiq, unpark(m), qlim)
}
/* </CODE> */
