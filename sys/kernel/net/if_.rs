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
 *	@(#)if.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*
 * Copyright (c) 1980, 1986, 1993
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
 *	@(#)if.c	8.3 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! Network interfaces: `net/if.c`, the interface index map, the softnet task queues and
//! their barriers, the protocol software interrupt (`if_netisr`), the softnet input path
//! (`if_input_process`) and the dispatch to an interface's driver.
//!
//! Upstream: sys/net/if.h @ 3ce1f3f79392
//! Upstream: sys/net/if.c @ 3ce1f3f79392
//! LZ: sys/net/if_.rs@3c62ede68802
//!
//! `if.c` keeps the interfaces on `ifnetlist` and in an index map (`if_get(index)` takes a
//! reference, `if_put` drops it); attaches them (`if_attach`: an index, a send queue `if_snd`
//! and an input queue `if_rcv` bound to a softnet task queue); passes received packets from
//! the input queues to the driver's `if_input` and the protocols in the softnet threads
//! (`if_input_process`, `if_input_proto`); and owns the `netisr` word (`if_netisr`).
//! `<net/if.h>`'s constants and structures are `hal::sys::net_if`'s.
//!
//! Locks and their order (the C's): the index map's `idxmaplk` (an rwlock, taken alone or
//! inside the net lock), the net lock (`hal::netlock::NETLOCK`: `if_input_process` shared,
//! `if_netisr` exclusive). The index map is read under SMR (`hal::smr`): `if_get` is a read
//! section, two loads and a reference count, as the C's. Transitions against the C: `if_get`
//! and `if_put`, one SMR section and two atomics each way (the C's); `if_input_process`, one
//! shared net lock pair around its batch (the C's).
//!
//! What `if.c` asks of the subsystems that have not moved goes through three bridges
//! (bridges.toml), registered by `bsd` at boot, each answering as the C compiled without the
//! subsystem before registration: [`Inet`] (netinet, netinet6: the protocols' software
//! interrupts and `tcp_input_mlist`), [`Routing`] (`route.c`, `rtable.c`, `rtsock.c`) and
//! [`Link`] (bpf(4) and `if_ethersubr.c`).
//!
//! ## Deviations
//! - Without `MULTIPROCESSOR` `NET_TASKQ` is 1; the kernel is always `MULTIPROCESSOR`
//!   (decision 40): `softnet_init` makes eight softnet queues and `softnet_percpu` destroys
//!   those past `softnet_count()`, one per CPU, whose memory stays (`kernel::task`'s
//!   `taskq_destroy_leaked`, LZ's: six `taskqpl` items on two CPUs, four on four).
//! - `malloc(M_WAITOK)` that fails panics where the C would sleep (the index map's arrays;
//!   `malloc(9)` does not sleep yet; LZ's).
//! - The index map's object is an item of the pool "ifmappl" (`hal::smr::SmrPtr` publishes
//!   pool items only, D-N6.3a-5): `vmstat -m` shows one pool row the C has not; the map's
//!   slot array and the used-index bitmap keep the C's `malloc(M_IFADDR)`s and sizes.
//!
//! ## Redesign
//! - LZ's `struct if_idxmap` of `Cell`s and raw arrays (slot 0 holding the length, the old
//!   bitmap reused as the `smr_call` destructor of the old map) -> [`IfMap`], an
//!   `SmrPtr`-published pool item holding its length and its slots (`MArray<Link<Ifnet>>`,
//!   the C's `malloc` and size), beside the used-index bitmap and the serial under
//!   `idxmaplk` (a `Guarded` keyed to that lock). Growth copies every slot into a map twice
//!   as long (a reference taken per slot, the C's `if_ref`), publishes it and retires the
//!   old one: the SMR thread drops it after the grace period, and its `Drop` puts each slot's
//!   reference (`if_idxmap_free`). Cost as the C's: `insert`/`remove` O(1), `alloc` O(limit)
//!   at worst (the free-index scan).
//! - The C's `if_get` returns a counted pointer the caller must `if_put`: [`if_get`] returns
//!   the `&'static Ifnet` with the count taken (the C's two-owner protocol, kept for `bsd`'s
//!   callers), and [`IfRef`] is the guard kernel code uses, which puts it when dropped.
//! - `struct netstack` is a value on `if_input_process`'s stack, lent `&mut` to each input
//!   function: the C reinitialises the softnet's own at each call, so its lists carry nothing
//!   between calls (D-N6.3a-10); `ns_route` is `bsd`'s per softnet.
//! - `if_input_proto` keeps the protocol's input function in the packet's `ph_cookie`, as the
//!   C, as an index into a table of the functions seen ([`proto_fn_id`]): a function pointer
//!   is not made back from a word without `unsafe`.
//! - `net_tq_barriers`' stack tasks with a `struct refcnt` on the stack -> one barrier task per
//!   softnet in the softnet, its count the barrier's own; the callers are serialised by the
//!   barrier's lock (the C's `net_tq_barriers` callers hold the net lock or nothing, and wait
//!   for every softnet in turn either way).

use core::any::Any;
use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};

use hal::intr::IPL_NET;
use hal::link::Link as HalLink;
use hal::list::List;
use hal::malloc::{M_WAITOK, MArray, MBox};
use hal::mutex::Mutexed;
use hal::netlock::NETLOCK;
use hal::once::OnceStatic;
use hal::pool::BoxPool;
use hal::rwlock::{Guarded, RawRwlock};
use hal::smr::{self, SmrLink, SmrLinked, SmrPtr, SmrRead};
use hal::sys::errno::Errno;
use hal::sys::net_if::{IFF_UP, IFNAMSIZ, IFQ_DEFPRIO, IFXF_CLONED, IFXF_MPSAFE};
use hal::sys::socket::{AF_INET, AF_INET6};
use hal::sys::types::SaFamily;

use crate::assert::{kassert, kpanic};
use crate::net::if_var::{
    DETACHED_IOCTL, DETACHED_QSTART, IF_TXMIT_DEFAULT, IFNET_SLOWTIMO, IfRt, Ifnet, IfnetList,
    NetDriver, Netstack,
};
use crate::net::ifq::{
    Ifiqueue, Ifqueue, ifiq_init_tasks, ifq_enqueue, ifq_idx, ifq_init_tasks, ifq_start,
};
use crate::net::mbuf::{Mbuf, MbufBox, MbufList, ml_dequeue, ml_empty, ml_enqueue, ml_len};
use crate::prf::Str;
use crate::task::TaskChain;
use crate::task::{Task, Taskq, TaskqRef, task_add, taskq_create, taskq_destroy_leaked};
use hal::pinlist::PinList;

/// `NET_TASKQ`: the softnet task queues (`MULTIPROCESSOR`'s, the only kernel built).
pub const NET_TASKQ: usize = 8;

/// `BPF_DIRECTION_IN` (`<net/bpf.h>`, `bsd`'s; a copy, docs/SCAFFOLDING.md "copies").
pub const BPF_DIRECTION_IN: u32 = 1;
/// `BPF_DIRECTION_OUT`.
pub const BPF_DIRECTION_OUT: u32 = 1 << 1;

/// `M_IFADDR`: the `malloc(9)` type of the index map's arrays (`<sys/malloc.h>`, `bsd`'s; a
/// copy).
const M_IFADDR: i32 = 9;

/// `M_DEVBUF`: the `malloc(9)` type of `if_attach_queues`' queues (a copy).
const M_DEVBUF: i32 = 2;

/// `USHRT_MAX`: the index map's bound on interfaces and on the serial.
const USHRT_MAX: u32 = 0xffff;

/// The bits of a byte (`NBBY`).
const NBBY: usize = 8;

/// The size of the C's `struct if_idxmap_dtor` on LP64 (a `struct smr_entry` of three
/// pointers and the map's pointer), which the used-index bitmap is at least as long as (the
/// C reuses the old bitmap as the destructor): kept so the bitmap's `malloc(M_IFADDR)` is the
/// C's size.
const IF_IDXMAP_DTOR_SIZE: usize = 32;

/// What the kernel's interface layer asks of the internet protocols (netinet and netinet6,
/// `bsd`'s until N6.4-N6.6): a bridge (bridges.toml's `Inet` row, paid by N6). Before
/// registration every leg does nothing: the C without `INET`/`INET6`.
pub trait Inet: Sync + 'static {
    /// `arpintr()`: the ARP queue (`NETISR_ARP`).
    fn arpintr(&self);
    /// `ipintr()`: the IPv4 queue (`NETISR_IP`).
    fn ipintr(&self);
    /// `ip6intr()`: the IPv6 queue (`NETISR_IPV6`).
    fn ip6intr(&self);
    /// `tcp_input_mlist(ml, af)`: the TCP segments a softnet gathered.
    fn tcp_input_mlist(&self, ml: &mut MbufList, af: SaFamily);
}

/// What the kernel's interface layer asks of the routing code (`route.c`, `rtable.c`,
/// `rtsock.c`, `bsd`'s until N6.3b): a bridge (bridges.toml's `Routing` row). Before
/// registration every leg does nothing.
pub trait Routing: Sync + 'static {
    /// `rtm_ifannounce(ifp, what)`: the routing socket's `RTM_IFANNOUNCE`.
    fn rtm_ifannounce(&self, ifp: &'static Ifnet, what: u16);
    /// `rtm_ifchg(ifp)`: the routing socket's `RTM_IFINFO`.
    fn rtm_ifchg(&self, ifp: &'static Ifnet);
    /// `rt_if_track(ifp)`: the routes over `ifp` follow its link state.
    fn rt_if_track(&self, ifp: &'static Ifnet);
}

/// What the kernel's interface layer asks of the link layers (bpf(4) and `if_ethersubr.c`,
/// `bsd`'s until N6.3c): a bridge (bridges.toml's `Link` row). Before registration every
/// leg does nothing: the C without `NBPFILTER`.
pub trait Link: Sync + 'static {
    /// `bpf_mtap_ether(if_bpf, m, dir)`: taps `m` to the tap `if_bpf` names; `true` drops it.
    fn bpf_mtap_ether(&self, if_bpf: *const u8, m: &Mbuf, dir: u32) -> bool;
    /// `*ifiq->ifiq_bpfp`: the per-queue tap a driver's word names (NULL for none).
    fn bpf_queue_tap(&self, bpfp: &hal::link::PtrWord<*mut u8>) -> *const u8;
}

/// The internet protocols' legs, once registered.
static INET: OnceStatic<&'static dyn Inet> = OnceStatic::new();
/// The routing code's legs, once registered.
static ROUTING: OnceStatic<&'static dyn Routing> = OnceStatic::new();
/// The link layers' legs, once registered.
static LINK: OnceStatic<&'static dyn Link> = OnceStatic::new();

/// Registers the internet protocols' legs, once (`bsd`'s boot, before the first interface).
pub fn register_inet(imp: &'static dyn Inet) -> Result<(), hal::bridges::AlreadyRegistered> {
    INET.set(imp)
        .map_err(|_| hal::bridges::AlreadyRegistered("Inet"))
}

/// Registers the routing code's legs, once.
pub fn register_routing(imp: &'static dyn Routing) -> Result<(), hal::bridges::AlreadyRegistered> {
    ROUTING
        .set(imp)
        .map_err(|_| hal::bridges::AlreadyRegistered("Routing"))
}

/// Registers the link layers' legs, once.
pub fn register_link(imp: &'static dyn Link) -> Result<(), hal::bridges::AlreadyRegistered> {
    LINK.set(imp)
        .map_err(|_| hal::bridges::AlreadyRegistered("Link"))
}

/// The internet protocols' legs, `None` before registration.
#[inline]
pub(crate) fn inet() -> Option<&'static dyn Inet> {
    INET.get().copied()
}

/// The routing code's legs, `None` before registration.
#[inline]
pub(crate) fn routing() -> Option<&'static dyn Routing> {
    ROUTING.get().copied()
}

/// The link layers' legs, `None` before registration.
#[inline]
pub(crate) fn link() -> Option<&'static dyn Link> {
    LINK.get().copied()
}

/// `struct softnet`: a softnet task queue and its barrier.
#[repr(align(64))]
pub struct Softnet {
    /// `sn_name`.
    name: &'static [u8],
    /// `sn_taskq`: set by `softnet_init` (its handle given up for a `&'static`, the module's
    /// deviations).
    taskq: HalLink<Taskq>,
    /// `net_tq_barriers`' task on this softnet.
    bartask: Task,
}

impl Softnet {
    /// Softnet `name`, its queue not made yet.
    const fn new(name: &'static [u8]) -> Self {
        Self {
            name,
            taskq: HalLink::new(),
            bartask: Task::unset(),
        }
    }
}

/// The softnets.
static SOFTNETS: [Softnet; NET_TASKQ] = [
    Softnet::new(b"softnet0"),
    Softnet::new(b"softnet1"),
    Softnet::new(b"softnet2"),
    Softnet::new(b"softnet3"),
    Softnet::new(b"softnet4"),
    Softnet::new(b"softnet5"),
    Softnet::new(b"softnet6"),
    Softnet::new(b"softnet7"),
];

/// `softnet_count()`'s answer, cached at its first call (`nsoftnets`).
static NSOFTNETS: AtomicU32 = AtomicU32::new(0);

/// `net_tq_barriers`' state: the barriers still to run of the current call (the C's stack
/// `struct refcnt`), under its own mutex, the callers serialised by it.
static NET_TQ_BARRIER: Mutexed<u32> = Mutexed::new(IPL_NET, 0);

/// The lock `net_tq_barriers`' callers take in turn (each waits for its own barriers).
static NET_TQ_BARRIER_LOCK: RawRwlock = RawRwlock::new("netbarlk");

/// `ifnetlist`: all attached interfaces, changed with the net lock (and the kernel lock in
/// `ifioctl`'s paths) held, read with either.
pub static IFNETLIST: List<IfnetList> = List::new();

/// `ifinit`: the index map (8 is a nice power of 2 for malloc) and its pool.
pub fn ifinit() {
    let _ = IF_MAP_POOL.init(IPL_NET, 0, "ifmappl");
    if_idxmap_init(8);
}

/// `softnet_init`: the `NET_TASKQ` softnet task queues (`softnet<i>`, one thread each,
/// `IPL_NET`, MP-safe).
pub fn softnet_init() {
    for (i, sn) in SOFTNETS.iter().enumerate() {
        let Some(tq) = taskq_create(sn.name, 1, IPL_NET, crate::task::TASKQ_MPSAFE) else {
            kpanic(format_args!("unable to create network taskq {i}"));
        };
        sn.taskq.set(Some(TaskqRef::leak(tq)));
        sn.bartask.set_ref(net_tq_barrier_task, sn);
    }
}

/// `softnet_percpu`: one softnet per CPU (`softnet_count()`); the others destroyed once.
pub fn softnet_percpu() {
    let n = softnet_count() as usize;
    for sn in &SOFTNETS[n..] {
        if let Some(tq) = sn.taskq.get() {
            // Emptied first: nothing reaches it any more (`net_sn` stops at the count).
            sn.taskq.set(None);
            taskq_destroy_leaked(tq);
        }
    }
}

/// `softnet_count()`: the softnets in use, one per CPU up to `NET_TASKQ`.
pub fn softnet_count() -> u32 {
    let n = NSOFTNETS.load(Ordering::Relaxed);
    if n != 0 {
        return n;
    }
    let n = (hal::cpu::ncpus() as u32).clamp(1, NET_TASKQ as u32);
    NSOFTNETS.store(n, Ordering::Relaxed);
    n
}

/// `net_sn(ifindex)`: the softnet of an interface (or a flow) index.
pub fn net_sn(ifindex: u32) -> &'static Softnet {
    &SOFTNETS[(ifindex % softnet_count()) as usize]
}

/// `net_tq(ifindex)`: the softnet task queue of an interface (or flow) index; `None` before
/// `softnet_init`.
pub fn net_tq(ifindex: u32) -> Option<&'static Taskq> {
    net_sn(ifindex).taskq.get()
}

/// `net_tq_barrier_rele`: one softnet's barrier ran.
fn net_tq_barrier_task(_sn: &'static (dyn Any + Send + Sync)) {
    let mut g = NET_TQ_BARRIER.lock();
    *g -= 1;
    if *g == 0 {
        drop(g);
        hal::sleep::wakeup(&raw const NET_TQ_BARRIER);
    }
}

/// `net_tq_barriers(wmesg)`: waits until every softnet in use has run what was queued on it
/// before the call (a barrier task on each, the C's `refcnt_finalize`).
pub fn net_tq_barriers(wmesg: &'static str) {
    let _serial = NET_TQ_BARRIER_LOCK.write();
    let n = softnet_count() as usize;
    *NET_TQ_BARRIER.lock() = n as u32;
    for sn in &SOFTNETS[..n] {
        match sn.taskq.get() {
            Some(tq) => {
                let _ = task_add(tq, Pin::static_ref(&sn.bartask));
            }
            None => net_tq_barrier_task(sn),
        }
    }
    let mut g = NET_TQ_BARRIER.lock();
    while *g != 0 {
        let _ = g.sleep_nsec(&raw const NET_TQ_BARRIER, 0, wmesg, hal::sleep::INFSLP);
    }
}

/// `if_map`'s published object: the map's length and its slots, one counted reference per
/// interface in a slot (slot 0 never used, the C's "skip ifidx 0").
pub struct IfMap {
    /// The SMR link its retirement is queued by.
    smr: SmrLink,
    /// The slots: `limit` of them (`malloc(M_IFADDR)`, the C's array).
    slots: MArray<HalLink<Ifnet>>,
}

/// The pool of the index map's objects ("ifmappl", D-N6.3a-5).
static IF_MAP_POOL: BoxPool<IfMap> = BoxPool::new();

impl SmrLinked for IfMap {
    const POOL: &'static BoxPool<IfMap> = &IF_MAP_POOL;
    fn smr_link(&self) -> &SmrLink {
        &self.smr
    }
}

impl IfMap {
    /// A map of `limit` empty slots.
    fn new(limit: u32) -> Self {
        let Some(slots) = MArray::new(M_IFADDR, M_WAITOK, limit as usize, |_| HalLink::new())
        else {
            kpanic(format_args!("if_idxmap: out of memory"));
        };
        Self {
            smr: SmrLink::new(),
            slots,
        }
    }

    /// The map's length (`if_idxmap_limit`).
    fn limit(&self) -> u32 {
        self.slots.len() as u32
    }

    /// Slot `index`'s interface.
    fn slot(&self, index: u32) -> Option<&'static Ifnet> {
        self.slots.get(index as usize).and_then(HalLink::get)
    }
}

impl Drop for IfMap {
    /// `if_idxmap_free`: the old map's references put, after the grace period (the SMR
    /// thread, or the last holder).
    fn drop(&mut self) {
        for s in self.slots.iter() {
            if_put(s.get());
        }
    }
}

/// What `idxmaplk` keeps beside the published map.
struct IdxmapState {
    /// `serial`: the next index to try.
    serial: u32,
    /// `count`: the interfaces with an index.
    count: u32,
    /// `usedidx`: the used-index bitmap (`malloc(M_IFADDR)`, at least the C's
    /// destructor's size).
    usedidx: Option<MArray<u8>>,
}

/// `if_idxmap.lock` ("idxmaplk").
static IF_IDXMAP_LOCK: RawRwlock = RawRwlock::new("idxmaplk");

/// The index map's locked state.
static IF_IDXMAP: Guarded<IdxmapState> = Guarded::new_static(
    &IF_IDXMAP_LOCK,
    IdxmapState {
        serial: 0,
        count: 0,
        usedidx: None,
    },
);

/// `if_idxmap.map`: the published map.
static IF_MAP: SmrPtr<IfMap> = SmrPtr::new_static(&IF_IDXMAP_LOCK);

/// The bitmap's size for `limit` indices (`max(howmany(limit, NBBY), sizeof(dtor))`).
fn if_idxmap_usedidx_size(limit: u32) -> usize {
    (limit as usize).div_ceil(NBBY).max(IF_IDXMAP_DTOR_SIZE)
}

/// A zeroed bitmap for `limit` indices.
fn if_idxmap_newusedidx(limit: u32) -> MArray<u8> {
    match MArray::new(M_IFADDR, M_WAITOK, if_idxmap_usedidx_size(limit), |_| 0) {
        Some(b) => b,
        None => kpanic(format_args!("if_idxmap: out of memory")),
    }
}

/// `isset(a, i)`.
fn isset(a: &[u8], i: usize) -> bool {
    a[i / NBBY] & (1 << (i % NBBY)) != 0
}

/// `setbit(a, i)`.
fn setbit(a: &mut [u8], i: usize) {
    a[i / NBBY] |= 1 << (i % NBBY);
}

/// `clrbit(a, i)`.
fn clrbit(a: &mut [u8], i: usize) {
    a[i / NBBY] &= !(1 << (i % NBBY));
}

/// The bitmap, `panic(9)` before `if_idxmap_init`.
fn usedidx(st: &mut IdxmapState) -> &mut [u8] {
    match st.usedidx.as_deref_mut() {
        Some(b) => b,
        None => kpanic(format_args!("if_idxmap used before if_idxmap_init")),
    }
}

/// `if_idxmap_init(limit)`: an empty map of `limit` slots, index 0 blacklisted, the serial
/// at 1. Called again (a test's fresh memory), it starts over: the old map is retired.
pub fn if_idxmap_init(limit: u32) {
    let mut w = IF_IDXMAP_LOCK.write();
    let st = IF_IDXMAP.get_mut(&mut w);
    st.serial = 1; // skip ifidx 0
    st.count = 0;
    let mut b = if_idxmap_newusedidx(limit);
    setbit(&mut b, 0); // blacklist ifidx 0
    drop(st.usedidx.replace(b));

    let Some(map) = IF_MAP_POOL.get(hal::pool::PR_WAITOK, || IfMap::new(limit)) else {
        kpanic(format_args!("if_idxmap: out of memory"));
    };
    if let Some(old) = IF_MAP.store(&mut w, Some(map)) {
        old.defer();
    }
}

/// The next serial, wrapped to 16 bits.
fn next_serial(st: &mut IdxmapState) -> u32 {
    let serial = st.serial;
    st.serial = serial.wrapping_add(1);
    serial & USHRT_MAX
}

/// `if_idxmap_alloc(ifp)`: picks `ifp`'s index (the next free serial), growing the map
/// (twice as long, the old one retired) when the serial passes its length.
pub fn if_idxmap_alloc(ifp: &Ifnet) {
    let mut w = IF_IDXMAP_LOCK.write();

    let mut index = {
        let st = IF_IDXMAP.get_mut(&mut w);
        st.count += 1;
        if st.count >= USHRT_MAX {
            kpanic(format_args!("too many interfaces"));
        }
        next_serial(st)
    };
    let mut limit = match IF_MAP.load_locked(&w) {
        Some(m) => m.limit(),
        None => kpanic(format_args!("if_idxmap used before if_idxmap_init")),
    };

    if index >= limit {
        let olimit = limit;
        limit = olimit * 2;
        let nmap = IfMap::new(limit);
        if let Some(omap) = IF_MAP.load_locked(&w) {
            for i in 1..olimit {
                if let Some(oifp) = omap.slot(i) {
                    // the new map isn't visible yet: no SMR_PTR_SET_LOCKED's membar needed.
                    nmap.slots[i as usize].set(Some(if_ref(oifp)));
                }
            }
        }

        let mut nusedidx = if_idxmap_newusedidx(limit);
        {
            let st = IF_IDXMAP.get_mut(&mut w);
            let o = usedidx(st);
            let n = (olimit as usize).div_ceil(NBBY);
            nusedidx[..n].copy_from_slice(&o[..n]);
            st.usedidx = Some(nusedidx);
        }

        let Some(nmap) = IF_MAP_POOL.get(hal::pool::PR_WAITOK, || nmap) else {
            kpanic(format_args!("if_idxmap: out of memory"));
        };
        if let Some(old) = IF_MAP.store(&mut w, Some(nmap)) {
            // smr_call(if_idxmap_free): the old map's references go after the grace period.
            old.defer();
        }
    }

    // pick the next free index
    let st = IF_IDXMAP.get_mut(&mut w);
    for _ in 0..USHRT_MAX {
        if index != 0 && !isset(usedidx(st), index as usize) {
            break;
        }
        index = next_serial(st);
    }
    kassert!(index != 0 && index < limit);
    kassert!(!isset(usedidx(st), index as usize));

    setbit(usedidx(st), index as usize);
    ifp.if_index.set(index);
}

/// `if_idxmap_insert(ifp)`: publishes `ifp` in its slot, a reference taken for the map.
pub fn if_idxmap_insert(ifp: &'static Ifnet) {
    let index = ifp.if_index.get();

    let w = IF_IDXMAP_LOCK.write();
    let Some(map) = IF_MAP.load_locked(&w) else {
        kpanic(format_args!("if_idxmap used before if_idxmap_init"));
    };
    let limit = map.limit();
    if index == 0 || index >= limit {
        ifp.if_xname.with_str(|n| {
            kpanic(format_args!(
                "{}({:p}) index {index} vs limit {limit}",
                Str(n),
                ifp
            ))
        });
    }
    let slot = &map.slots[index as usize];
    kassert!(slot.is_none());

    // commit
    slot.set(Some(if_ref(ifp)));
}

/// `if_idxmap_remove(ifp)`: `ifp`'s slot emptied and its index freed; once no reader can
/// still see it there (`smr_barrier`), the map's reference put.
pub fn if_idxmap_remove(ifp: &'static Ifnet) {
    if_idxmap_unlink(ifp);

    smr::barrier(false);
    if_put(Some(ifp));
}

/// `if_idxmap_remove`'s locked part.
fn if_idxmap_unlink(ifp: &'static Ifnet) {
    let index = ifp.if_index.get();

    let mut w = IF_IDXMAP_LOCK.write();
    {
        let Some(map) = IF_MAP.load_locked(&w) else {
            kpanic(format_args!("if_idxmap used before if_idxmap_init"));
        };
        kassert!(index != 0 && index < map.limit());
        let slot = &map.slots[index as usize];
        kassert!(slot.points_to(ifp));
        slot.set(None);
    }

    let st = IF_IDXMAP.get_mut(&mut w);
    kassert!(isset(usedidx(st), index as usize));
    st.count -= 1;
    clrbit(usedidx(st), index as usize);
    // end of if_idxmap modifications
}

/// The interface at `index` for the read section `g` (`if_idxmap`'s `SMR_PTR_GET`).
fn if_idxmap_get(index: u32, g: &SmrRead) -> Option<&'static Ifnet> {
    if index == 0 {
        return None;
    }
    IF_MAP.load(g)?.slot(index)
}

/// `if_get(index)`: the interface at `index`, a reference taken (the caller `if_put`s it,
/// the C's protocol; kernel code holds an [`IfRef`] instead).
pub fn if_get(index: u32) -> Option<&'static Ifnet> {
    let g = smr::read();
    let ifp = if_idxmap_get(index, &g)?;
    Some(if_ref(ifp))
}

/// `if_get_smr(index)`: the interface at `index` for a reader inside the SMR read section
/// `g`, no reference taken.
pub fn if_get_smr(index: u32, g: &SmrRead) -> Option<&'static Ifnet> {
    if_idxmap_get(index, g)
}

/// `if_ref(ifp)`: one more reference.
pub fn if_ref(ifp: &'static Ifnet) -> &'static Ifnet {
    ifp.rt().if_refcnt.take();
    ifp
}

/// `if_put(ifp)`: one reference less (`NULL` is nothing); the last one wakes `if_remove`'s
/// `refcnt_finalize`.
pub fn if_put(ifp: Option<&Ifnet>) {
    if let Some(ifp) = ifp {
        ifp.rt().if_refcnt.rele_wake();
    }
}

/// An interface with a reference held, put when dropped (`if_get` ... `if_put`).
pub struct IfRef(&'static Ifnet);

impl IfRef {
    /// `if_get(index)` as a guard.
    pub fn get(index: u32) -> Option<IfRef> {
        if_get(index).map(IfRef)
    }

    /// The interface, for as long as the reference is held or longer (`'static`).
    pub fn ifp(&self) -> &'static Ifnet {
        self.0
    }
}

impl core::ops::Deref for IfRef {
    type Target = Ifnet;
    fn deref(&self) -> &Ifnet {
        self.0
    }
}

impl Drop for IfRef {
    fn drop(&mut self) {
        if_put(Some(self.0));
    }
}

/// `ifp->if_qstart(ifq)`: the driver's start routine for the ring, or the detached stub
/// after `if_detach`'s swap (`if_detached_qstart`: the queue purged).
pub fn if_qstart(ifp: &'static Ifnet, ifq: &'static Ifqueue) {
    if ifp.is_detached(DETACHED_QSTART) {
        if_detached_qstart(ifq);
        return;
    }
    ifp.driver().qstart(ifq);
}

/// `if_qstart_compat`: a driver without `IFXF_MPSAFE`'s `if_start`, under the kernel lock at
/// `splnet`.
pub fn if_qstart_compat<D: NetDriver + ?Sized>(driver: &D, ifq: &'static Ifqueue) {
    let ifp = ifq.ifq_if();

    hal::klock::kernel_lock();
    let s = hal::intr::splraise(IPL_NET);
    driver.start(ifp);
    drop(s);
    hal::klock::kernel_unlock();
}

/// `if_start(ifp)`: the old start path of a driver without `IFXF_MPSAFE`.
pub fn if_start(ifp: &'static Ifnet) {
    kassert!(ifp.if_xflags.get() & IFXF_MPSAFE == 0);
    if_qstart(ifp, ifp.ifq(0));
}

/// `if_detached_qstart`: the detached interface's start, which drops what is queued.
pub fn if_detached_qstart(ifq: &Ifqueue) {
    let _ = crate::net::ifq::ifq_purge(ifq);
}

/// `if_enqueue_ifq`: the default `if_enqueue`, onto the send queue the conditioner picks (of
/// `if_nifqs`), then the ring started.
pub fn if_enqueue_ifq(ifp: &'static Ifnet, m: MbufBox) -> Result<(), Errno> {
    let mut ifq: &'static Ifqueue = ifp.if_snd();

    let n = ifp.if_nifqs();
    if n > 1 {
        // use the operations on the first ifq to pick which of the array gets this mbuf.
        let idx = ifq_idx(ifq, n, &m);
        ifq = ifp.ifq(idx);
    }

    ifq_enqueue(ifq, m)?;

    ifq_start(ifq);

    Ok(())
}

/// The protocol input functions `if_input_proto` was given (the module's redesign): a
/// function's index, kept in the packet's `ph_cookie`, names it again in `if_input_process`.
static PROTO_FNS: [OnceStatic<ProtoFn>; 16] = [const { OnceStatic::new() }; 16];

/// A protocol's input function (`ipv4_input`, `ipv6_input`, ...), as `if_input_proto` calls
/// it: the interface, the packet, the softnet's stack (`None`: at once, the net lock held).
pub type ProtoFn = fn(&'static Ifnet, MbufBox, Option<&mut Netstack>);

/// The index of `f` in the table of protocol input functions, entered on first sight (1-based:
/// 0 is the C's empty cookie). `panic(9)` past the table's 16 functions.
pub fn proto_fn_id(f: ProtoFn) -> usize {
    for (i, slot) in PROTO_FNS.iter().enumerate() {
        match slot.get() {
            Some(g) if core::ptr::fn_addr_eq(*g, f) => return i + 1,
            Some(_) => {}
            None => match slot.set(f) {
                Ok(()) => return i + 1,
                Err(_) => {
                    if slot.get().is_some_and(|g| core::ptr::fn_addr_eq(*g, f)) {
                        return i + 1;
                    }
                }
            },
        }
    }
    kpanic(format_args!(
        "if_input_proto: too many protocol input functions"
    ))
}

/// The protocol input function of a cookie [`proto_fn_id`] made.
fn proto_fn(id: usize) -> Option<ProtoFn> {
    id.checked_sub(1)
        .and_then(|i| PROTO_FNS.get(i))
        .and_then(|s| s.get().copied())
}

/// `if_input_proto(ifp, m, input, ns)`: `m` for the protocol `input`, at once when there is
/// no softnet stack (the caller holds the net lock), otherwise queued on the stack's
/// `ns_proto` with the function in its cookie.
pub fn if_input_proto(ifp: &'static Ifnet, m: MbufBox, input: ProtoFn, ns: Option<&mut Netstack>) {
    let Some(ns) = ns else {
        input(ifp, m, None);
        return;
    };

    m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
    m.m_pkthdr().ph_cookie.set(proto_fn_id(input));
    ml_enqueue(&mut ns.ns_proto, m);
}

/// `if_input_process_proto`: the protocol input function in the packet's cookie, called.
fn if_input_process_proto(ifp: &'static Ifnet, m: MbufBox, ns: &mut Netstack) {
    match proto_fn(m.m_pkthdr().ph_cookie.get()) {
        Some(input) => input(ifp, m, Some(ns)),
        None => kpanic(format_args!("if_input_process_proto: no input function")),
    }
}

/// `if_input_process(ifp, ml, idx)`: the softnet `idx`'s work on a batch an input queue
/// passed (`ifiq_process`), under the shared net lock: each packet to the driver's
/// `if_input`, then what those queued on the stack (`if_vinput`'s packets again through
/// their interface's `if_input`, `if_input_proto`'s through their protocol, TCP's segments
/// through `tcp_input_mlist`) until none is left.
pub fn if_input_process(ifp: &'static Ifnet, ml: &mut MbufList, idx: u32) {
    if ml_empty(ml) {
        return;
    }

    if ifp.if_xflags.get() & IFXF_CLONED == 0 {
        let first = ml.first().map_or(0, |m| core::ptr::from_ref(m).addr());
        hal::random::enqueue_randomness(ml_len(ml) ^ first as u32);
    }

    // We grab the shared netlock for packet processing in the softnet threads. Packets can
    // regrab the exclusive lock via queues. ioctl, sysctl, and socket syscall may use shared
    // lock if access is read only or MP safe. Usually they hold the exclusive net lock.

    let mut ns = Netstack::new(idx);

    let _shared = NETLOCK.read();
    let driver = ifp.driver();
    while let Some(m) = ml_dequeue(ml) {
        driver.input(ifp, m, Some(&mut ns));
    }

    loop {
        while let Some(m) = ml_dequeue(&mut ns.ns_input) {
            let index = m.m_pkthdr().ph_ifidx.get();
            // The interface is borrowed past the read section, as the C does: `if_remove`
            // waits for the softnets (`net_tq_barriers`) before the interface goes.
            let ifp = if_get_smr(index, &smr::read());
            match ifp {
                Some(ifp) => ifp.driver().input(ifp, m, Some(&mut ns)),
                None => drop(m),
            }
        }

        while let Some(m) = ml_dequeue(&mut ns.ns_proto) {
            let index = m.m_pkthdr().ph_ifidx.get();
            let ifp = if_get_smr(index, &smr::read());
            match ifp {
                Some(ifp) => if_input_process_proto(ifp, m, &mut ns),
                None => drop(m),
            }
        }

        if let Some(inet) = inet() {
            inet.tcp_input_mlist(&mut ns.ns_tcp_ml, AF_INET);
            inet.tcp_input_mlist(&mut ns.ns_tcp6_ml, AF_INET6);
        }

        if ml_empty(&ns.ns_input) {
            break;
        }
    }
}

/// `if_attach_common(ifp)`: what an interface needs before it is listed: its driver (the one
/// registered in [`Ifnet::if_driver`]; none is `panic(9)`, the C's `KASSERT(if_ioctl !=
/// NULL)`), an index, the first send and input queues on their softnets, the reference count,
/// the hook lists, `if_txmit` and `if_llprio`'s defaults, all built into [`Ifnet::rt`] once.
/// The C's defaults for a NULL `if_enqueue`, `if_bpf_mtap` and `if_qstart` are the driver's
/// default methods. `panic(9)` for an interface attached twice (LZ's: the C would corrupt
/// `ifnetlist`).
pub fn if_attach_common(ifp: &'static Ifnet) {
    let Some(&driver) = ifp.if_driver.get() else {
        ifp.if_xname
            .with_str(|n| kpanic(format_args!("{}: no driver (if_ioctl)", Str(n))))
    };
    if ifp.if_index.get() != 0 || ifp.rt.is_set() {
        ifp.if_xname.with_str(|n| {
            kpanic(format_args!(
                "{}: attached twice (index {})",
                Str(n),
                ifp.if_index.get()
            ))
        });
    }

    if_idxmap_alloc(ifp);

    let rt = IfRt {
        if_refcnt: hal::refcnt::Refcnt::new(),
        if_snd: Ifqueue::new(ifp, 0, net_tq(0), ifp.if_snd_maxlen.get()),
        if_rcv: Ifiqueue::new(ifp, 0, net_tq(0)),
        ifqs: OnceStatic::new(),
        iqs: OnceStatic::new(),
        if_maddrlock: RawRwlock::new("maddr"),
        if_slowtimo: crate::timeout::Timeout::unset(),
        if_watchdogtask: Task::unset(),
        if_linkstatetask: Task::unset(),
        hooks_mtx: Mutexed::new(hal::intr::IPL_NONE, ()),
        if_addrhooks: hal::pinlist::PinList::new(),
        if_linkstatehooks: hal::pinlist::PinList::new(),
        if_detachhooks: hal::pinlist::PinList::new(),
        driver,
    };
    if ifp.rt.set(rt).is_err() {
        kpanic(format_args!("if_attach_common: attached twice"));
    }
    let rt = ifp.rt();
    rt.if_refcnt.init();
    ifq_init_tasks(&rt.if_snd);
    ifiq_init_tasks(&rt.if_rcv);

    if ifp.if_txmit.get() == 0 {
        ifp.if_txmit.set(IF_TXMIT_DEFAULT);
    }
    ifp.if_llprio.set(IFQ_DEFPRIO as u8);
}

/// `if_attach_queues(ifp, nqs)`: `nqs` send queues, the first the interface's own, the others
/// made here with its length limit (after `if_attach`, once).
pub fn if_attach_queues(ifp: &'static Ifnet, nqs: u32) {
    kassert!(nqs != 0);
    let rt = ifp.rt();
    let maxlen = rt.if_snd.ifq_maxlen.get();

    rt.if_snd.ifq_softc.set(core::ptr::null());
    let Some(map) = MArray::new(M_DEVBUF, M_WAITOK, (nqs - 1) as usize, |i| {
        let i = i as u32 + 1;
        match MBox::new(M_DEVBUF, M_WAITOK, Ifqueue::new(ifp, i, net_tq(i), maxlen)) {
            Some(q) => q,
            None => kpanic(format_args!("if_attach_queues: out of memory")),
        }
    }) else {
        kpanic(format_args!("if_attach_queues: out of memory"));
    };
    if rt.ifqs.set(map).is_err() {
        kpanic(format_args!("if_attach_queues: called twice"));
    }
    if let Some(map) = rt.ifqs.get() {
        for q in map.iter() {
            ifq_init_tasks(q);
        }
    }
}

/// `if_attach_iqueues(ifp, niqs)`: `niqs` input queues, as [`if_attach_queues`].
pub fn if_attach_iqueues(ifp: &'static Ifnet, niqs: u32) {
    kassert!(niqs != 0);
    let rt = ifp.rt();

    rt.if_rcv.ifiq_softc.set(core::ptr::null());
    let Some(map) = MArray::new(M_DEVBUF, M_WAITOK, (niqs - 1) as usize, |i| {
        let i = i as u32 + 1;
        match MBox::new(M_DEVBUF, M_WAITOK, Ifiqueue::new(ifp, i, net_tq(i))) {
            Some(q) => q,
            None => kpanic(format_args!("if_attach_iqueues: out of memory")),
        }
    }) else {
        kpanic(format_args!("if_attach_iqueues: out of memory"));
    };
    if rt.iqs.set(map).is_err() {
        kpanic(format_args!("if_attach_iqueues: called twice"));
    }
    if let Some(map) = rt.iqs.get() {
        for q in map.iter() {
            ifiq_init_tasks(q);
        }
    }
}

/// `if_counters_alloc(ifp)`: the interface's stack counters turned on (at 0).
pub fn if_counters_alloc(ifp: &Ifnet) {
    kassert!(ifp.if_counters.get().is_none());
    ifp.if_counters.turn(true);
}

/// `if_counters_free(ifp)`: the counters turned off.
pub fn if_counters_free(ifp: &Ifnet) {
    kassert!(ifp.if_counters.get().is_some());
    ifp.if_counters.turn(false);
}

/// Forgets the index map of an earlier test (its arrays and pool item live in that test's
/// memory, so nothing may be freed into it) and starts a fresh one of 8 slots, the pool made
/// if it is not: `kernel::testutil`'s rule for a global holding kernel memory.
#[cfg(test)]
pub(crate) fn if_idxmap_test_reset() {
    let _ = IF_MAP_POOL.init(IPL_NET, 0, "ifmappl");
    {
        let mut w = IF_IDXMAP_LOCK.write();
        core::mem::forget(IF_IDXMAP.get_mut(&mut w).usedidx.take());
        core::mem::forget(IF_MAP.take(&mut w));
    }
    if_idxmap_init(8);
}

/// The host tests' softnets: task queues no thread serves (`kernel::task`'s
/// `taskq_unthreaded`), made once and run by hand, in place of `softnet_init`'s.
#[cfg(test)]
pub(crate) fn softnet_test_install() {
    for sn in &SOFTNETS {
        if sn.taskq.is_none() {
            let tq: &'static Taskq =
                std::boxed::Box::leak(std::boxed::Box::new(crate::task::taskq_unthreaded(sn.name)));
            crate::task::run_by_hand(tq);
            sn.taskq.set(Some(tq));
            sn.bartask.set_ref(net_tq_barrier_task, sn);
        }
    }
}

/// `if_ioctl(ifp, cmd, data)`: the driver's `if_ioctl`, or the detached stub after
/// `if_detach`'s swap (`if_detached_ioctl`: `ENODEV`).
pub fn if_ioctl(ifp: &'static Ifnet, cmd: u64, data: &mut [u8]) -> Result<(), Errno> {
    if ifp.is_detached(DETACHED_IOCTL) {
        return if_detached_ioctl(ifp, cmd, data);
    }
    ifp.driver().ioctl(ifp, cmd, data)
}

/// `if_detached_ioctl`: a detached interface answers no request.
pub fn if_detached_ioctl(_ifp: &Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
    Err(Errno::ENODEV)
}

/// `if_attachsetup`'s timers: the watchdog timeout and the two tasks, whose argument is the
/// interface's index (the C's `(void *)ifidx`; `if_slowtimo`'s too here, D-N6.3a-7), then a
/// first `if_slowtimo`.
pub fn if_attach_timers(ifp: &'static Ifnet) {
    let rt = ifp.rt();
    let ifidx = core::ptr::without_provenance_mut(ifp.if_index.get() as usize);
    rt.if_slowtimo
        .set(Some(if_slowtimo), ifidx, crate::timeout::KCLOCK_NONE, 0);
    rt.if_watchdogtask.set(if_watchdog_task, ifidx);
    rt.if_linkstatetask.set(if_linkstate_task, ifidx);
    if_slowtimo(ifidx);
}

/// The interface index a timer or task of [`if_attach_timers`] was set with.
fn timer_ifidx(arg: *mut core::ffi::c_void) -> u32 {
    arg.addr() as u32
}

/// `if_slowtimo`: once a second, the driver's watchdog timer counted down (its task queued
/// when it reaches zero) and the timeout armed again, for a driver that has a watchdog. The
/// interface is found by its index (D-N6.3a-7: one SMR read and count a second where the C
/// follows its pointer).
fn if_slowtimo(arg: *mut core::ffi::c_void) {
    let Some(ifp) = IfRef::get(timer_ifidx(arg)) else {
        return;
    };
    let rt = ifp.rt();
    let s = hal::intr::splraise(IPL_NET);

    if rt.driver.has_watchdog() {
        let timer = ifp.if_timer.get();
        if timer > 0 {
            ifp.if_timer.set(timer - 1);
            if timer - 1 == 0
                && let Some(tq) = net_tq(ifp.if_index.get())
            {
                let _ = task_add(tq, Pin::static_ref(&ifp.ifp().rt().if_watchdogtask));
            }
        }
        let _ = Pin::static_ref(&ifp.ifp().rt().if_slowtimo).add_sec(IFNET_SLOWTIMO);
    }
    drop(s);
}

/// `if_watchdog_task`: the driver's watchdog, under the kernel lock at `splnet`.
fn if_watchdog_task(arg: *mut core::ffi::c_void) {
    let Some(ifp) = IfRef::get(timer_ifidx(arg)) else {
        return;
    };

    hal::klock::kernel_lock();
    let s = hal::intr::splraise(IPL_NET);
    let driver = ifp.rt().driver;
    if driver.has_watchdog() {
        driver.watchdog(ifp.ifp());
    }
    drop(s);
    hal::klock::kernel_unlock();
}

/// `if_linkstate_task`: `if_linkstate` under the net and kernel locks.
fn if_linkstate_task(arg: *mut core::ffi::c_void) {
    let _net = NETLOCK.write();
    hal::klock::kernel_lock();

    if let Some(ifp) = IfRef::get(timer_ifidx(arg)) {
        if_linkstate(ifp.ifp());
    }

    hal::klock::kernel_unlock();
}

/// `if_linkstate(ifp)`: the routing socket and the routes told of the link state (not after a
/// panic), then the link state hooks run. The caller holds the net lock.
pub fn if_linkstate(ifp: &'static Ifnet) {
    if !hal::cpu::past_caring()
        && let Some(r) = routing()
    {
        r.rtm_ifchg(ifp);
        r.rt_if_track(ifp);
    }

    if_hooks_run(ifp, &ifp.rt().if_linkstatehooks);
}

/// `if_link_state_change(ifp)`: `if_linkstate` queued on the interface's softnet.
pub fn if_link_state_change(ifp: &'static Ifnet) {
    if let Some(tq) = net_tq(ifp.if_index.get()) {
        let _ = task_add(tq, Pin::static_ref(&ifp.rt().if_linkstatetask));
    }
}

/// `if_hooks_add(head, tail, t)`: `t` linked on one of the interface's hook lists, at its tail
/// or its head, under the hooks' mutex. `t` stays pinned until it is removed
/// ([`if_hooks_del`]).
fn if_hooks_add(ifp: &Ifnet, list: &PinList<TaskChain>, tail: bool, t: Pin<&Task>) {
    let mut g = ifp.rt().hooks_mtx.lock();
    if tail {
        list.insert_tail(&mut g, t);
    } else {
        list.insert_head(&mut g, t);
    }
}

/// `if_hooks_del(head, t)`: `t` taken off the list.
fn if_hooks_del(ifp: &Ifnet, list: &PinList<TaskChain>, t: &Task) {
    let mut g = ifp.rt().hooks_mtx.lock();
    list.remove(&mut g, t);
}

/// `if_hooks_run(head)`: every hook of the list run in order, the hooks' mutex released
/// around each, a cursor (a task without a function, which the walk skips) pinned on this
/// stack after the hook running so that a hook may add or remove hooks, itself included.
fn if_hooks_run(ifp: &Ifnet, list: &PinList<TaskChain>) {
    let rt = ifp.rt();
    let cursor = core::pin::pin!(Task::unset());
    let cursor = cursor.into_ref();

    let mut g = rt.hooks_mtx.lock();
    if list.is_empty(&g) {
        return;
    }
    list.insert_head(&mut g, cursor);
    // The hook after the cursor: `None` at the end; `Some(None)` for another walk's cursor.
    while let Some(work) = list.next(&g, &cursor).map(Task::work) {
        let _ = list.step_over(&mut g, &cursor);
        if let Some(w) = work {
            drop(g);
            w.run();
            g = rt.hooks_mtx.lock();
        }
    }
    list.remove(&mut g, &cursor);
}

/// `if_addrhook_add(ifp, t)`: an address change hook, at the tail.
pub fn if_addrhook_add(ifp: &Ifnet, t: Pin<&Task>) {
    if_hooks_add(ifp, &ifp.rt().if_addrhooks, true, t);
}

/// `if_addrhook_del(ifp, t)`.
pub fn if_addrhook_del(ifp: &Ifnet, t: &Task) {
    if_hooks_del(ifp, &ifp.rt().if_addrhooks, t);
}

/// `if_addrhooks_run(ifp)`.
pub fn if_addrhooks_run(ifp: &Ifnet) {
    if_hooks_run(ifp, &ifp.rt().if_addrhooks);
}

/// `if_linkstatehook_add(ifp, t)`: a link state hook, at the head.
pub fn if_linkstatehook_add(ifp: &Ifnet, t: Pin<&Task>) {
    if_hooks_add(ifp, &ifp.rt().if_linkstatehooks, false, t);
}

/// `if_linkstatehook_del(ifp, t)`.
pub fn if_linkstatehook_del(ifp: &Ifnet, t: &Task) {
    if_hooks_del(ifp, &ifp.rt().if_linkstatehooks, t);
}

/// `if_detachhook_add(ifp, t)`: a detach hook, at the head.
pub fn if_detachhook_add(ifp: &Ifnet, t: Pin<&Task>) {
    if_hooks_add(ifp, &ifp.rt().if_detachhooks, false, t);
}

/// `if_detachhook_del(ifp, t)`.
pub fn if_detachhook_del(ifp: &Ifnet, t: &Task) {
    if_hooks_del(ifp, &ifp.rt().if_detachhooks, t);
}

/// `if_deactivate(ifp)`: the detach hooks run under the net lock (pseudo-drivers undo their
/// changes).
pub fn if_deactivate(ifp: &Ifnet) {
    let _net = NETLOCK.write();
    if_hooks_run(ifp, &ifp.rt().if_detachhooks);
}

/// `if_input(ifp, ml)`: a driver's received packets, onto its first input queue.
pub fn if_input(ifp: &'static Ifnet, ml: &mut MbufList) {
    let _ = crate::net::ifq::ifiq_input(ifp.if_rcv(), ml);
}

/// `if_isconnected`'s and others' guard: an interface is up.
pub fn if_is_up(ifp: &Ifnet) -> bool {
    ifp.if_flags.get() & IFF_UP != 0
}

/// The bytes of a NUL-padded name (`if_xname`) up to the NUL.
pub fn ifname(b: &[u8; IFNAMSIZ]) -> &[u8] {
    let n = b.iter().position(|&c| c == 0).unwrap_or(IFNAMSIZ);
    &b[..n]
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests of the interface core over the mbuf tests' real memory: every test holds
    // the page system's lock (`mbuf::tests::setup`), so the index map, `ifnetlist` and the
    // counts are each test's own; interfaces are leaked, as a softc's lives for ever.

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::sync::atomic::AtomicU32;
    use std::vec::Vec;

    use super::*;
    use crate::net::ifq::{
        IfqDeq, ifq_barrier, ifq_deq_with, ifq_dequeue, ifq_hdatalen, ifq_is_oactive, ifq_len,
        ifq_purge, ifq_serialize, ifq_stats,
    };
    use crate::net::mbuf::{M_DONTWAIT, m_gethdr};

    /// A driver that counts what it is asked; its input sends a packet round once through
    /// `ns_input` and then to [`proto_count`].
    pub(crate) struct TestDriver {
        /// `if_input` calls.
        pub(crate) inputs: AtomicU32,
        /// `if_qstart` calls.
        pub(crate) qstarts: AtomicU32,
    }

    impl NetDriver for TestDriver {
        fn ioctl(&self, _ifp: &'static Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            Ok(())
        }

        fn qstart(&self, _ifq: &'static Ifqueue) {
            self.qstarts.fetch_add(1, Ordering::Relaxed);
        }

        fn input(&self, ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
            self.inputs.fetch_add(1, Ordering::Relaxed);
            let Some(ns) = ns else {
                drop(m);
                return;
            };
            if m.m_pkthdr().ph_loopcnt.get() == 0 {
                // if_vinput: once more through the interface's input.
                m.m_pkthdr().ph_loopcnt.set(1);
                m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
                ml_enqueue(&mut ns.ns_input, m);
            } else {
                if_input_proto(ifp, m, proto_count, Some(ns));
            }
        }
    }

    static PROTO_INPUTS: AtomicU32 = AtomicU32::new(0);

    fn proto_count(_ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
        assert!(ns.is_some());
        PROTO_INPUTS.fetch_add(1, Ordering::Relaxed);
        drop(m);
    }

    fn proto_other(_ifp: &'static Ifnet, m: MbufBox, _ns: Option<&mut Netstack>) {
        drop(m);
    }

    /// Real memory and a fresh index map.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        let g = crate::net::mbuf::tests::setup();
        softnet_test_install();
        if_idxmap_test_reset();
        g
    }

    /// An interface named `name` with a [`TestDriver`], attached (`if_attach_common` and the
    /// index map's slot; not listed).
    pub(crate) fn test_ifnet(name: &[u8]) -> (&'static Ifnet, &'static TestDriver) {
        let ifp: &'static Ifnet = Box::leak(Box::new(Ifnet::new()));
        let mut n = [0u8; IFNAMSIZ];
        n[..name.len()].copy_from_slice(name);
        ifp.if_xname.set(n);
        let d: &'static TestDriver = Box::leak(Box::new(TestDriver {
            inputs: AtomicU32::new(0),
            qstarts: AtomicU32::new(0),
        }));
        assert!(ifp.if_driver.set(d).is_ok());
        ifp.if_xflags.set(IFXF_MPSAFE);
        if_attach_common(ifp);
        if_idxmap_insert(ifp);
        (ifp, d)
    }

    fn packet(prio: u8, len: i32) -> MbufBox {
        let m = m_gethdr(M_DONTWAIT, hal::sys::mbuf::MT_DATA).expect("an mbuf");
        m.m_pkthdr().pf.prio.set(prio);
        m.m_pkthdr().len.set(len);
        m
    }

    fn map_limit() -> u32 {
        IF_MAP.load(&smr::read()).map_or(0, IfMap::limit)
    }

    #[test]
    fn attached_interfaces_get_unique_indexes_and_the_map_grows() {
        let _g = setup();
        assert_eq!(map_limit(), 8);
        let ifps: Vec<&'static Ifnet> = (0..20).map(|_| test_ifnet(b"tst0").0).collect();
        let idx: Vec<u32> = ifps.iter().map(|i| i.if_index.get()).collect();
        assert_eq!(idx, (1..=20).collect::<Vec<u32>>());
        // 8 -> 16 -> 32, the old maps retired.
        assert_eq!(map_limit(), 32);
        for &ifp in &ifps {
            let before = ifp.rt().if_refcnt.read();
            let got = if_get(ifp.if_index.get()).expect("attached");
            assert!(core::ptr::eq(got, ifp));
            assert_eq!(ifp.rt().if_refcnt.read(), before + 1);
            if_put(Some(got));
            assert_eq!(ifp.rt().if_refcnt.read(), before);
        }
        assert!(if_get(0).is_none() && if_get(21).is_none() && if_get(99).is_none());
        {
            let g = smr::read();
            assert!(if_get_smr(5, &g).is_some_and(|i| core::ptr::eq(i, ifps[4])));
        }
        // The index map's own reference, then the interface's: `if_idxmap_remove`'s locked
        // part empties the slot and frees the index.
        if_idxmap_unlink(ifps[2]);
        assert!(if_get(3).is_none());
        {
            let r = IfRef::get(4).expect("attached");
            assert_eq!(r.if_index.get(), 4);
            assert!(core::ptr::eq(r.ifp(), ifps[3]));
        }
    }

    #[test]
    fn priq_keeps_priorities_and_drops_the_lowest_when_full() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tst0");
        let ifq = ifp.if_snd();
        crate::net::ifq::ifq_init_maxlen(ifq, 3);
        // if_txmit 1: ifq_start runs the start at once (no softnet in the test).
        ifp.if_txmit.set(1);
        ifq_enqueue(ifq, packet(0, 10)).expect("queued");
        ifq_enqueue(ifq, packet(1, 11)).expect("queued");
        ifq_enqueue(ifq, packet(2, 12)).expect("queued");
        assert_eq!(ifq_len(ifq), 3);
        // Full: a packet of priority 3 evicts the head of the lowest non-empty list (0).
        ifq_enqueue(ifq, packet(3, 13)).expect("queued");
        assert_eq!(ifq_len(ifq), 3);
        // Full, nothing lower than 0: refused.
        assert_eq!(ifq_enqueue(ifq, packet(0, 14)), Err(Errno::ENOBUFS));
        let (packets, bytes, qdrops, _, _, _) = ifq_stats(ifq);
        assert_eq!((packets, bytes, qdrops), (4, 46, 2));
        assert_eq!(ifq_hdatalen(ifq), 13);
        let lens: Vec<i32> = core::iter::from_fn(|| ifq_dequeue(ifq))
            .map(|m| m.m_pkthdr().len.get())
            .collect();
        assert_eq!(lens, [13, 12, 11]);
        assert!(ifq_dequeue(ifq).is_none() && ifq_hdatalen(ifq) == 0);
    }

    #[test]
    fn a_dequeue_commits_rolls_back_or_marks_the_ring_full() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tst0");
        let ifq = ifp.if_snd();
        ifp.if_txmit.set(1);
        ifq_enqueue(ifq, packet(3, 100)).expect("queued");
        ifq_enqueue(ifq, packet(3, 200)).expect("queued");
        assert!(
            ifq_deq_with(ifq, |m| {
                assert_eq!(m.m_pkthdr().len.get(), 100);
                IfqDeq::Rollback
            })
            .is_none()
        );
        assert_eq!(ifq_len(ifq), 2);
        assert!(!ifq_is_oactive(ifq));
        assert!(ifq_deq_with(ifq, |_| IfqDeq::RollbackOactive).is_none());
        assert!(ifq_is_oactive(ifq));
        crate::net::ifq::ifq_clr_oactive(ifq);
        let m = ifq_deq_with(ifq, |_| IfqDeq::Commit).expect("the head");
        assert_eq!(m.m_pkthdr().len.get(), 100);
        assert_eq!(ifq_purge(ifq), 1);
        assert_eq!(ifq_len(ifq), 0);
        let (_, _, qdrops, _, _, oactives) = ifq_stats(ifq);
        assert_eq!((qdrops, oactives), (1, 1));
    }

    static SERIAL_RUNS: AtomicU32 = AtomicU32::new(0);

    fn serial_task(t: &'static (dyn Any + Send + Sync)) {
        let ifq = t.downcast_ref::<Ifqueue>().expect("the queue");
        SERIAL_RUNS.fetch_add(1, Ordering::Relaxed);
        // From inside the serializer: queued, run after this one by the same CPU.
        if SERIAL_RUNS.load(Ordering::Relaxed) == 1 {
            ifq_serialize(ifq, Pin::static_ref(second_task()));
        }
    }

    fn second_task() -> &'static Task {
        static T: OnceStatic<&'static Task> = OnceStatic::new();
        T.get().copied().unwrap_or_else(|| {
            let t: &'static Task = Box::leak(Box::new(Task::unset()));
            let _ = T.set(t);
            T.get().copied().expect("set")
        })
    }

    #[test]
    fn the_serializer_runs_its_work_in_order_on_one_cpu() {
        let _g = setup();
        let (ifp, d) = test_ifnet(b"tst0");
        let ifq = ifp.if_snd();
        SERIAL_RUNS.store(0, Ordering::Relaxed);
        let first: &'static Task = Box::leak(Box::new(Task::unset()));
        first.set_ref(serial_task, ifq);
        second_task().set_ref(serial_task, ifq);
        ifq_serialize(ifq, Pin::static_ref(first));
        assert_eq!(SERIAL_RUNS.load(Ordering::Relaxed), 2);
        assert!(!first.pending() && !second_task().pending());
        // The start task through the serializer, to the driver's qstart (IFF_RUNNING set and
        // a packet queued).
        ifp.if_flags.set(hal::sys::net_if::IFF_RUNNING);
        ifp.if_txmit.set(1);
        ifq_enqueue(ifq, packet(1, 1)).expect("queued");
        ifq_start(ifq);
        assert_eq!(d.qstarts.load(Ordering::Relaxed), 1);
        // No serializer at work: the barrier returns at once.
        ifq_barrier(ifq);
        // The test driver's qstart sends nothing: the packet is still queued (and the mbuf
        // tests count every mbuf back).
        assert_eq!(ifq_purge(ifq), 1);
    }

    #[test]
    fn the_softnet_input_path_reaches_the_driver_and_the_protocol() {
        let _g = setup();
        let (ifp, d) = test_ifnet(b"tst0");
        PROTO_INPUTS.store(0, Ordering::Relaxed);
        let mut ml = MbufList::new();
        ml_enqueue(&mut ml, packet(0, 60));
        ml_enqueue(&mut ml, packet(0, 60));
        if_input_process(ifp, &mut ml, 0);
        // Each packet: the driver, the driver again (ns_input), then the protocol.
        assert_eq!(d.inputs.load(Ordering::Relaxed), 4);
        assert_eq!(PROTO_INPUTS.load(Ordering::Relaxed), 2);
        assert!(ml_empty(&ml));
    }

    static HOOK_RUNS: AtomicU32 = AtomicU32::new(0);
    static HOOK_A: OnceStatic<&'static Task> = OnceStatic::new();

    /// A hook that takes itself off its list as it runs.
    fn hook_a(t: &'static (dyn Any + Send + Sync)) {
        let ifp = t.downcast_ref::<Ifnet>().expect("the interface");
        HOOK_RUNS.fetch_add(1, Ordering::Relaxed);
        if let Some(&me) = HOOK_A.get() {
            if_addrhook_del(ifp, me);
        }
    }

    /// A hook that stays.
    fn hook_b(_t: &'static (dyn Any + Send + Sync)) {
        HOOK_RUNS.fetch_add(10, Ordering::Relaxed);
    }

    #[test]
    fn the_hooks_run_in_order_and_may_remove_themselves() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tst0");
        HOOK_RUNS.store(0, Ordering::Relaxed);
        let a: &'static Task = HOOK_A.get().copied().unwrap_or_else(|| {
            let t: &'static Task = Box::leak(Box::new(Task::unset()));
            let _ = HOOK_A.set(t);
            t
        });
        let b: &'static Task = Box::leak(Box::new(Task::unset()));
        a.set_ref(hook_a, ifp);
        b.set_ref(hook_b, ifp);
        if_addrhook_add(ifp, Pin::static_ref(a));
        if_addrhook_add(ifp, Pin::static_ref(b));
        if_addrhooks_run(ifp);
        assert_eq!(HOOK_RUNS.load(Ordering::Relaxed), 11);
        assert!(!a.pending() && b.pending());
        if_addrhooks_run(ifp);
        assert_eq!(HOOK_RUNS.load(Ordering::Relaxed), 21);
        if_addrhook_del(ifp, b);
        assert!(!b.pending());
        if_addrhooks_run(ifp);
        assert_eq!(HOOK_RUNS.load(Ordering::Relaxed), 21);
        // A detached interface answers no request.
        assert_eq!(if_ioctl(ifp, 0, &mut []), Ok(()));
        ifp.detached
            .fetch_or(crate::net::if_var::DETACHED_IOCTL, Ordering::Release);
        assert_eq!(if_ioctl(ifp, 0, &mut []), Err(Errno::ENODEV));
    }

    #[test]
    fn a_protocol_function_keeps_its_cookie() {
        let a = proto_fn_id(proto_count);
        let b = proto_fn_id(proto_other);
        assert_ne!(a, 0);
        assert_ne!(a, b);
        assert_eq!(proto_fn_id(proto_count), a);
        assert!(proto_fn(a).is_some_and(|f| core::ptr::fn_addr_eq(f, proto_count as ProtoFn)));
        assert!(proto_fn(0).is_none());
    }

    #[test]
    fn more_queues_after_attach_are_reached_by_index() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tst0");
        assert_eq!((ifp.if_nifqs(), ifp.if_niqs()), (1, 1));
        if_attach_queues(ifp, 4);
        if_attach_iqueues(ifp, 2);
        assert_eq!((ifp.if_nifqs(), ifp.if_niqs()), (4, 2));
        for i in 0..4 {
            assert_eq!(ifp.ifq(i).idx(), i);
            assert!(core::ptr::eq(ifp.ifq(i).ifq_if(), ifp));
        }
        assert_eq!(ifp.ifiq(1).idx(), 1);
        assert!(core::ptr::eq(ifp.ifq(0), ifp.if_snd()));
        if_counters_alloc(ifp);
        assert!(ifp.if_counters.get().is_some());
        if_counters_free(ifp);
        assert!(ifp.if_counters.get().is_none());
    }
}
/* </TESTS> */
