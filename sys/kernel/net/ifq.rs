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
//! Interface send and receive queues: `<net/ifq.h>` and `net/ifq.c`, see `ifq_enq(9)`.
//!
//! Upstream: sys/net/ifq.h @ 3ce1f3f79392
//! Upstream: sys/net/ifq.c @ 3ce1f3f79392
//! LZ: sys/net/ifq.rs@3c62ede68802
//!
//! `struct ifqueue` sits between the network stack and a driver's transmission of packets:
//! when the stack has finished generating a packet it queues it on an ifqueue
//! (`ifq_enqueue`) and notifies the driver to start transmission (`ifq_start`). A network
//! device may have several transmit rings, each with its own ifqueue (`if_attach_queues`).
//! The ifqueue is also where traffic conditioning happens: the queue keeps its packets in a
//! conditioner (`priq`, the default, keeps one list per priority and drops from a lower
//! priority when full; pf's `hfsc` and `fq_codel` replace it, `ifq_attach`). The ifqueue API
//! takes the queue lock on behalf of the conditioners, so they only have to keep or reject
//! mbufs; a conditioner that drops a packet while the lock is held hands it to
//! [`IfqLocked::mfreem`], which frees it after the lock is released.
//!
//! An ifqueue also serialises work: `ifq_start`, `ifq_restart` and `ifq_serialize` run their
//! tasks one at a time, on the first CPU that dispatches work; a second CPU only queues its
//! task and returns. `if_qstart` therefore never runs twice at once for one ring.
//!
//! `struct ifiqueue` is the receive side: a driver hands a list of packets to `ifiq_input`
//! (`if_input`), which counts them and queues them for the interface's softnet task, where
//! `ifiq_process` passes them to `if_input_process`.
//!
//! Locks: `ifq_mtx` ([`Ifqueue`]'s `st`), `ifq_task_mtx` (`ser`) and `ifiq_mtx` are leaf
//! mutexes at `IPL_NET`, as the C's; `ifq_len` and `ifq_oactive` are read without them (the
//! C's `READ_ONCE`). Channels: `ifq_deq_sleep`'s (the queue), `ifq_barrier`'s ("ifqbar").
//!
//! ## Deviations
//! - `kstat(4)` is not ported: the `NKSTAT > 0` code (`ifq_kstat_tpl`, `ifq_kstat_copy`,
//!   `ifiq_kstat_*`, the `kstat_create`/`kstat_install` in `ifq_init`/`ifiq_init` and the
//!   `kstat_destroy` calls) is not compiled, as with `NKSTAT` 0 (LZ's).
//! - `priq_alloc` panics when `malloc(M_WAITOK)` fails, where the C would sleep (`malloc(9)`
//!   does not sleep yet; LZ's).
//! - `net_ifiq_sysctl`'s `#if 0` pressure code is not translated (it is compiled out in C);
//!   the function returns `EOPNOTSUPP` as the C does (LZ's).
//!
//! ## Redesign
//! - LZ's queues of `Cell`s behind `unsafe impl Sync`, embedded in `struct ifnet` zeroed and
//!   set up by `ifq_init` -> [`Ifqueue`] and [`Ifiqueue`], built whole by `if_attach_common`
//!   into the interface's `IfRt` (or `MBox`ed by `if_attach_queues`): the state `ifq_mtx`
//!   keeps is the data of a `hal::mutex::Mutexed` ([`IfqState`]), `ifq_len` and
//!   `ifq_oactive` atomics beside it for the unlocked readers, the serializer's list a
//!   `PinList` of tasks under `ifq_task_mtx`. A queue's own tasks (`ifq_start`, `ifq_restart`,
//!   `ifq_bundle`, `ifiq_task`) take the queue as a `&'static` argument
//!   (`kernel::task::Task::set_ref`) where the C passes its address.
//! - `struct ifq_ops` and `void *ifq_q` -> [`Discipline`]: priq native (its `struct priq` an
//!   `MBox` of `M_DEVBUF`, as the C's `malloc`), or a [`QueueDiscipline`] table over the
//!   conditioner's word (`bsd`'s `hfsc` and `fq_codel`, N8; bridges.toml's
//!   `QueueDiscipline` row). `ifqop_deq_begin`'s `void **cookiep` is a returned word.
//! - The C's `ifq_deq_begin` returns with `ifq_mtx` held until `ifq_deq_commit` or
//!   `ifq_deq_rollback`: one call here, [`ifq_deq_with`], whose closure sees the packet with
//!   the mutex held and answers commit or rollback.
//! - `ifq_barrier`'s condition variable and task on the caller's stack -> one barrier task in
//!   the queue: each caller records a generation under `ifq_task_mtx`, the task (linked at the
//!   serializer's tail) marks the generation it was linked at done and links itself again
//!   when later callers came, and a caller sleeps on the mutex until its generation is done
//!   (no allocation, the C's guarantee: every task queued before the call has run).
//! - `ifq_q_enter`/`ifq_q_leave` (the mutex held between two calls) -> [`ifq_q_with`].

use core::any::Any;
use core::cmp::min;
use core::ffi::c_void;
use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};

use hal::intr::IPL_NET;
use hal::link::{Link, PtrWord};
use hal::malloc::{M_WAITOK, MBox};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::pinlist::PinList;
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::net_if::{IFF_RUNNING, IFQ_MAXPRIO, IFQ_NQUEUES, IFXF_MONITOR, IfData};

use crate::assert::{kassert, kpanic};
use crate::net::if_var::{AtomicCell, Ifnet};
use crate::net::mbuf::{
    M_FLOWID, M_MCAST, Mbuf, MbufBox, MbufList, ml_dequeue, ml_empty, ml_enlist, ml_enqueue,
    ml_len, ml_purge,
};
use crate::task::{Task, TaskChain, Taskq, task_add, task_del, taskq_del_barrier};

/// `IFQ_MAXLEN`: the default length of a send queue.
pub const IFQ_MAXLEN: u32 = 256;

/// `M_DEVBUF`: the `malloc(9)` type of `struct priq` (`<sys/malloc.h>`, `bsd`'s; a copy,
/// docs/SCAFFOLDING.md "copies").
const M_DEVBUF: i32 = 2;

/// A conditioner of `bsd`'s (`struct ifq_ops` of `hfsc` and `fq_codel`, pf's, N8) over its
/// state word (`void *ifq_q`, its own `malloc`): the C's seven operations, each run with the
/// queue's mutex held ([`IfqLocked`]), except `idx`, `alloc` and `free`.
pub trait QueueDiscipline: Sync + 'static {
    /// `ifqop_idx`: which of `nqueues` send queues gets `m`.
    fn idx(&self, nqueues: u32, m: &Mbuf) -> u32;
    /// `ifqop_alloc`: the conditioner's state for queue `idx` (`arg` is `ifq_attach`'s).
    fn alloc(&self, idx: u32, arg: usize) -> usize;
    /// `ifqop_free`: the state `alloc` made, no longer used.
    fn free(&self, idx: u32, q: usize);
    /// `ifqop_enq`: keeps `m` and returns `None`, or returns the packet to drop (`m`
    /// itself, or one it evicts).
    fn enq(&self, q: usize, st: &mut IfqLocked<'_>, m: MbufBox) -> Option<MbufBox>;
    /// `ifqop_deq_begin`: the next packet and its cookie, left queued.
    fn deq_begin<'a>(&self, q: usize, st: &'a mut IfqLocked<'_>) -> Option<(&'a Mbuf, usize)>;
    /// `ifqop_deq_commit`: takes the packet `deq_begin` returned with `cookie` off.
    fn deq_commit(&self, q: usize, st: &mut IfqLocked<'_>, cookie: usize) -> MbufBox;
    /// `ifqop_purge`: moves every packet to `ml`.
    fn purge(&self, q: usize, st: &mut IfqLocked<'_>, ml: &mut MbufList);
}

/// `struct priq`: priq internal structures, one packet list per priority.
pub struct Priq {
    /// `pq_lists`.
    pq_lists: [MbufList; IFQ_NQUEUES as usize],
}

impl Priq {
    /// Empty lists.
    const fn new() -> Self {
        Self {
            pq_lists: [const { MbufList::new() }; IFQ_NQUEUES as usize],
        }
    }
}

/// `ifq_ops` and `ifq_q`: the queue's conditioner.
pub enum Discipline {
    /// `ifq_priq_ops` with its `struct priq`.
    Priq(MBox<Priq>),
    /// A conditioner of `bsd`'s and its state word.
    Ops(&'static dyn QueueDiscipline, usize),
}

/// What `ifq_mtx` keeps.
pub struct IfqState {
    /// `ifq_ops` and `ifq_q` (`None` once `ifq_destroy` freed them).
    disc: Option<Discipline>,
    /// `ifq_free`: mbufs to free once `ifq_mtx` is released.
    free: MbufList,
    /// `ifq_packets`.
    packets: u64,
    /// `ifq_bytes`.
    bytes: u64,
    /// `ifq_qdrops`.
    qdrops: u64,
    /// `ifq_errors`.
    errors: u64,
    /// `ifq_mcasts`.
    mcasts: u64,
    /// `ifq_oactives`.
    oactives: u32,
}

/// `ifq_task_mtx`'s state.
struct Serializer {
    /// `ifq_serializer`: the CPU running the queue's work, `None` when none.
    cpu: Option<usize>,
    /// `ifq_barrier`'s generations: the last one asked for, the one the barrier task was
    /// linked at, and the last one done.
    bar_asked: u64,
    /// See `bar_asked`.
    bar_linked: u64,
    /// See `bar_asked`.
    bar_done: u64,
}

/// `struct ifqueue`: an interface send queue (see the module's documentation).
pub struct Ifqueue {
    /// `ifq_if`: the interface.
    ifq_if: Link<Ifnet>,
    /// `ifq_softnet`: the softnet task queue the queue's work goes to.
    ifq_softnet: Link<Taskq>,
    /// `ifq_softc` (`_ifq_ptr._ifq_softc`): the driver's ring (its word).
    pub ifq_softc: PtrWord<c_void>,
    /// `ifq_mtx` and what it keeps.
    st: Mutexed<IfqState>,
    /// `ifq_len`: packets queued. Written under `ifq_mtx`, read without it.
    ifq_len: AtomicU32,
    /// `ifq_oactive`: the driver's ring is full.
    ifq_oactive: AtomicU32,
    /// `ifq_task_mtx` and the serializer.
    ser: Mutexed<Serializer>,
    /// `ifq_task_list`: work waiting for the serialiser, under `ifq_task_mtx`.
    ser_list: PinList<TaskChain>,
    /// `ifq_bundle`: the deferred `ifq_start` (transmit mitigation).
    ifq_bundle: Task,
    /// `ifq_start`.
    ifq_start: Task,
    /// `ifq_restart`.
    ifq_restart: Task,
    /// `ifq_barrier`'s task.
    ifq_bartask: Task,
    /// `ifq_maxlen`.
    pub ifq_maxlen: AtomicCell<u32>,
    /// `ifq_idx`.
    ifq_idx: u32,
}

/// `ifq_mtx` held: the queue's state and the queue, for the conditioners
/// ([`QueueDiscipline`]) and `ifq_deq_set_oactive`.
pub struct IfqLocked<'a> {
    /// The queue.
    ifq: &'a Ifqueue,
    /// The guard.
    g: MutexedGuard<'a, IfqState>,
}

impl IfqLocked<'_> {
    /// The queue.
    pub fn ifq(&self) -> &Ifqueue {
        self.ifq
    }

    /// `ifq_mfreem`: a conditioner drops `m` while `ifq_mtx` is held; it is freed once the
    /// mutex is released.
    pub fn mfreem(&mut self, m: MbufBox) {
        self.ifq.ifq_len.fetch_sub(1, Ordering::Relaxed);
        self.g.qdrops += 1;
        ml_enqueue(&mut self.g.free, m);
    }

    /// `ifq_mfreeml`: as `mfreem`, for a list.
    pub fn mfreeml(&mut self, ml: &mut MbufList) {
        let n = ml_len(ml);
        self.ifq.ifq_len.fetch_sub(n, Ordering::Relaxed);
        self.g.qdrops += u64::from(n);
        ml_enlist(&mut self.g.free, ml);
    }

    /// `ifq_deq_set_oactive`: `ifq_set_oactive` with `ifq_mtx` held.
    pub fn set_oactive(&mut self) {
        if self.ifq.ifq_oactive.load(Ordering::Relaxed) == 0 {
            self.ifq.ifq_oactive.store(1, Ordering::Relaxed);
            self.g.oactives += 1;
        }
    }

    /// `ifq_len`, as the conditioner sees it under the mutex.
    pub fn ifq_len(&self) -> u32 {
        self.ifq.ifq_len.load(Ordering::Relaxed)
    }

    /// `ifq_maxlen`.
    pub fn maxlen(&self) -> u32 {
        self.ifq.ifq_maxlen.get()
    }

    /// `ifq_deq_leave`: the mutex released, then what the conditioner dropped meanwhile
    /// freed.
    fn leave(mut self) {
        let mut ml = MbufList::new();
        ml_enlist(&mut ml, &mut self.g.free);
        drop(self);
        if !ml_empty(&ml) {
            let _ = ml_purge(&mut ml);
        }
    }
}

impl Ifqueue {
    /// `ifq_init(ifq, ifp, idx)`'s state: queue `idx` of `ifp`, whose work goes to `softnet`,
    /// with the priq conditioner and the length limit `maxlen` (`IFQ_MAXLEN` when 0, the C's
    /// `if (ifq->ifq_maxlen == 0)`). Its tasks are set once it is in place
    /// ([`ifq_init_tasks`]).
    pub fn new(
        ifp: &'static Ifnet,
        idx: u32,
        softnet: Option<&'static Taskq>,
        maxlen: u32,
    ) -> Self {
        // default to priq
        let Some(pq) = MBox::new(M_DEVBUF, M_WAITOK, Priq::new()) else {
            kpanic(format_args!("priq_alloc: out of memory"));
        };
        let ifq_softnet = Link::new();
        ifq_softnet.set(softnet);
        Self {
            ifq_if: Link::to(ifp),
            ifq_softnet,
            ifq_softc: PtrWord::new(),
            st: Mutexed::new(
                IPL_NET,
                IfqState {
                    disc: Some(Discipline::Priq(pq)),
                    free: MbufList::new(),
                    packets: 0,
                    bytes: 0,
                    qdrops: 0,
                    errors: 0,
                    mcasts: 0,
                    oactives: 0,
                },
            ),
            ifq_len: AtomicU32::new(0),
            ifq_oactive: AtomicU32::new(0),
            ser: Mutexed::new(
                IPL_NET,
                Serializer {
                    cpu: None,
                    bar_asked: 0,
                    bar_linked: 0,
                    bar_done: 0,
                },
            ),
            ser_list: PinList::new(),
            ifq_bundle: Task::unset(),
            ifq_start: Task::unset(),
            ifq_restart: Task::unset(),
            ifq_bartask: Task::unset(),
            ifq_maxlen: {
                let c = AtomicCell::new();
                c.set(if maxlen == 0 { IFQ_MAXLEN } else { maxlen });
                c
            },
            ifq_idx: idx,
        }
    }

    /// The queue's interface (`ifq->ifq_if`).
    pub fn ifq_if(&self) -> &'static Ifnet {
        match self.ifq_if.get() {
            Some(ifp) => ifp,
            None => kpanic(format_args!("ifqueue used before ifq_init")),
        }
    }

    /// The queue's softnet task queue.
    fn softnet(&self) -> &'static Taskq {
        match self.ifq_softnet.get() {
            Some(tq) => tq,
            None => kpanic(format_args!("ifqueue used before ifq_init")),
        }
    }

    /// `ifq_idx`.
    pub fn idx(&self) -> u32 {
        self.ifq_idx
    }

    /// `ifq_mtx` taken (`ifq_deq_enter`).
    fn enter(&self) -> IfqLocked<'_> {
        IfqLocked {
            ifq: self,
            g: self.st.lock(),
        }
    }
}

/// `ifq_init`'s `task_set`s, once the queue is where it stays.
pub fn ifq_init_tasks(ifq: &'static Ifqueue) {
    ifq.ifq_bundle.set_ref(ifq_bundle_task, ifq);
    ifq.ifq_start.set_ref(ifq_start_task, ifq);
    ifq.ifq_restart.set_ref(ifq_restart_task, ifq);
    ifq.ifq_bartask.set_ref(ifq_barrier_task, ifq);
}

/// The queue a task of [`ifq_init_tasks`] was set with.
fn task_ifq(t: &'static (dyn Any + Send + Sync)) -> &'static Ifqueue {
    match t.downcast_ref::<Ifqueue>() {
        Some(ifq) => ifq,
        None => kpanic(format_args!("ifqueue task without its queue")),
    }
}

/// `ifq_run_start`.
fn ifq_run_start(ifq: &'static Ifqueue) {
    ifq_serialize(ifq, Pin::static_ref(&ifq.ifq_start));
}

/// `ifq_serialize`: runs `t` in the queue's serialisation context: at once if no CPU is
/// running the queue's work, otherwise after the work in progress, on the CPU doing it. `t`
/// stays pinned until it has run (a `'static` task, or one its caller waits for).
pub fn ifq_serialize(ifq: &Ifqueue, t: Pin<&Task>) {
    if t.pending() {
        return;
    }

    let mut g = ifq.ser.lock();
    // On no list (checked under the mutex): linked at the tail; on one, left there.
    let _ = ifq.ser_list.try_insert_tail(&mut g, t);

    if g.cpu.is_none() {
        g.cpu = Some(hal::mutex::mtx_curcpu());

        loop {
            // work = *t: copy to caller to avoid races.
            let Some(work) = ifq.ser_list.first(&g).map(Task::work) else {
                break;
            };
            ifq.ser_list.remove_first(&mut g);
            drop(g);

            match work {
                Some(w) => w.run(),
                None => kpanic(format_args!("ifq_serialize: task without a function")),
            }

            g = ifq.ser.lock();
        }

        g.cpu = None;
    }
}

/// `ifq_start`: notifies the driver that packets wait on `ifq`. With fewer than `if_txmit`
/// packets queued the start is deferred to the softnet task (transmit mitigation).
pub fn ifq_start(ifq: &'static Ifqueue) {
    if ifq_len(ifq) >= min(ifq.ifq_if().if_txmit.get(), ifq.ifq_maxlen.get()) {
        let _ = task_del(ifq.softnet(), &ifq.ifq_bundle);
        ifq_run_start(ifq);
    } else {
        let _ = task_add(ifq.softnet(), Pin::static_ref(&ifq.ifq_bundle));
    }
}

/// `ifq_start_task`: calls the driver's `if_qstart` if the interface is running, the queue
/// is not empty and the ring is not full.
fn ifq_start_task(t: &'static (dyn Any + Send + Sync)) {
    let ifq = task_ifq(t);
    let ifp = ifq.ifq_if();

    if ifp.if_flags.get() & IFF_RUNNING == 0 || ifq_empty(ifq) || ifq_is_oactive(ifq) {
        return;
    }

    crate::net::if_::if_qstart(ifp, ifq);
}

/// `ifq_set_oactive`: the driver's ring is full.
pub fn ifq_set_oactive(ifq: &Ifqueue) {
    if ifq.ifq_oactive.load(Ordering::Relaxed) != 0 {
        return;
    }

    let mut g = ifq.st.lock();
    if ifq.ifq_oactive.load(Ordering::Relaxed) == 0 {
        ifq.ifq_oactive.store(1, Ordering::Relaxed);
        g.oactives += 1;
    }
}

/// `ifq_restart_task`: clears `oactive` and calls the driver's `if_qstart`.
fn ifq_restart_task(t: &'static (dyn Any + Send + Sync)) {
    let ifq = task_ifq(t);

    ifq_clr_oactive(ifq);
    crate::net::if_::if_qstart(ifq.ifq_if(), ifq);
}

/// `ifq_bundle_task`: the deferred `ifq_start`.
fn ifq_bundle_task(t: &'static (dyn Any + Send + Sync)) {
    ifq_run_start(task_ifq(t));
}

/// `ifq_barrier`'s task: the generation it was linked at is done; linked again at the tail
/// when later callers asked meanwhile.
fn ifq_barrier_task(t: &'static (dyn Any + Send + Sync)) {
    let ifq = task_ifq(t);
    let mut g = ifq.ser.lock();
    g.bar_done = g.bar_linked;
    if g.bar_asked > g.bar_linked {
        g.bar_linked = g.bar_asked;
        let _ = ifq
            .ser_list
            .try_insert_tail(&mut g, Pin::static_ref(&ifq.ifq_bartask));
    }
    drop(g);
    hal::sleep::wakeup(&raw const ifq.ser);
}

/// `ifq_barrier`: waits until the work running in the queue's serialisation context (the
/// driver's start routine) and the work queued before the call has finished.
pub fn ifq_barrier(ifq: &'static Ifqueue) {
    let _ = task_del(ifq.softnet(), &ifq.ifq_bundle);

    let mut g = ifq.ser.lock();
    if g.cpu.is_none() {
        return;
    }

    g.bar_asked += 1;
    let mine = g.bar_asked;
    // Linked only when no barrier is outstanding (none linked, none running): one the
    // serialiser has taken off but whose body has not run yet must not be linked again
    // before the work queued meanwhile, or it would report this caller's barrier done
    // early; its body links it again for every barrier asked meanwhile. The C gives each
    // caller a task of its own (ifq.c:186-199).
    if g.bar_done == g.bar_linked {
        g.bar_linked = mine;
        let _ = ifq
            .ser_list
            .try_insert_tail(&mut g, Pin::static_ref(&ifq.ifq_bartask));
    }
    while g.bar_done < mine {
        let _ = g.sleep_nsec(&raw const ifq.ser, 0, "ifqbar", INFSLP);
    }
}

/// `ifq_attach`: replaces the queue's conditioner with `newops` (its state made with `arg`);
/// the pending packets move to the new one (and are dropped if it refuses them).
pub fn ifq_attach(ifq: &Ifqueue, newops: &'static dyn QueueDiscipline, arg: usize) {
    let newq = newops.alloc(ifq.ifq_idx, arg);
    ifq_attach_disc(ifq, Discipline::Ops(newops, newq));
}

/// `ifq_attach(ifq, ifq_priq_ops, NULL)`: the default conditioner back (pf's queues removed),
/// a fresh `struct priq` (`priq_alloc`).
pub fn ifq_attach_priq(ifq: &Ifqueue) {
    let Some(pq) = MBox::new(M_DEVBUF, M_WAITOK, Priq::new()) else {
        kpanic(format_args!("priq_alloc: out of memory"));
    };
    ifq_attach_disc(ifq, Discipline::Priq(pq));
}

/// `ifq_attach`'s body: the packets moved from the old conditioner to `new`, which replaces
/// it; the old one's state freed after the mutex.
fn ifq_attach_disc(ifq: &Ifqueue, new: Discipline) {
    let mut ml = MbufList::new();
    let mut free_ml = MbufList::new();

    let mut st = ifq.enter();
    disc_purge(&mut st, &mut ml);
    ifq.ifq_len.store(0, Ordering::Relaxed);

    let old = st.g.disc.replace(new);

    while let Some(m) = ml_dequeue(&mut ml) {
        match disc_enq(&mut st, m) {
            Some(m) => {
                st.g.qdrops += 1;
                ml_enqueue(&mut free_ml, m);
            }
            None => {
                ifq.ifq_len.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    drop(st);

    // The old conditioner's state, no longer reachable from the queue.
    disc_free(old, ifq.ifq_idx);

    let _ = ml_purge(&mut free_ml);
}

/// `ifq_destroy`: tears the queue down in `if_detach`, freeing what is still queued (the
/// net lock not held, the C's `NET_ASSERT_UNLOCKED`).
pub fn ifq_destroy(ifq: &Ifqueue) {
    let mut ml = MbufList::new();

    // NKSTAT > 0: kstat_destroy(ifq->ifq_kstat): not configured.

    taskq_del_barrier(ifq.softnet(), &ifq.ifq_bundle);

    // don't need to lock because this is the last use of the ifq
    let mut st = ifq.enter();
    disc_purge(&mut st, &mut ml);
    let old = st.g.disc.take();
    drop(st);
    disc_free(old, ifq.ifq_idx);

    let _ = ml_purge(&mut ml);
}

/// `ifq_add_data`: adds the queue's counters to `data` (`SIOCGIFDATA`).
pub fn ifq_add_data(ifq: &Ifqueue, data: &mut IfData) {
    let g = ifq.st.lock();
    data.ifi_opackets += g.packets;
    data.ifi_obytes += g.bytes;
    data.ifi_oqdrops += g.qdrops;
    data.ifi_omcasts += g.mcasts;
    // ifp->if_data.ifi_oerrors
}

/// `ifq_enqueue`: tries to fit `m` on the queue. The conditioner may drop another packet to
/// make room; `ENOBUFS` when `m` itself was dropped (and freed, after the mutex).
pub fn ifq_enqueue(ifq: &Ifqueue, m: MbufBox) -> Result<(), Errno> {
    let len = u64::try_from(m.m_pkthdr().len.get()).unwrap_or(0);
    let mcast = m.flags() & M_MCAST != 0;
    let addr = core::ptr::from_ref::<Mbuf>(&m).addr();

    let mut st = ifq.enter();
    let dm = disc_enq(&mut st, m);
    let dropped_m = dm
        .as_ref()
        .is_some_and(|dm| core::ptr::from_ref::<Mbuf>(dm).addr() == addr);
    if !dropped_m {
        st.g.packets += 1;
        st.g.bytes += len;
        if mcast {
            st.g.mcasts += 1;
        }
    }

    if dm.is_none() {
        ifq.ifq_len.fetch_add(1, Ordering::Relaxed);
    } else {
        st.g.qdrops += 1;
    }
    drop(st);

    drop(dm);

    if dropped_m {
        Err(Errno::ENOBUFS)
    } else {
        Ok(())
    }
}

/// What [`ifq_deq_with`]'s closure answers about the packet it saw.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IfqDeq {
    /// `ifq_deq_commit`: take it off the queue.
    Commit,
    /// `ifq_deq_rollback`: leave it.
    Rollback,
    /// `ifq_deq_set_oactive` then `ifq_deq_rollback`: the ring is full.
    RollbackOactive,
}

/// `ifq_deq_begin`, then `ifq_deq_commit` or `ifq_deq_rollback` (the module's redesign): `f`
/// sees the next packet with `ifq_mtx` held and answers; the packet taken off on
/// [`IfqDeq::Commit`]. `None` when the queue is empty or `f` rolled back.
pub fn ifq_deq_with(ifq: &Ifqueue, f: impl FnOnce(&Mbuf) -> IfqDeq) -> Option<MbufBox> {
    let mut st = ifq.enter();
    if ifq.ifq_len.load(Ordering::Relaxed) == 0 {
        st.leave();
        return None;
    }
    let seen = disc_deq_begin(&mut st).map(|(m, cookie)| (f(m), cookie));
    let m = match seen {
        Some((IfqDeq::Commit, cookie)) => {
            let m = disc_deq_commit(&mut st, cookie);
            ifq.ifq_len.fetch_sub(1, Ordering::Relaxed);
            Some(m)
        }
        Some((IfqDeq::RollbackOactive, _)) => {
            st.set_oactive();
            None
        }
        Some((IfqDeq::Rollback, _)) | None => None,
    };
    st.leave();
    m
}

/// `ifq_dequeue`: the next packet, taken off the queue.
pub fn ifq_dequeue(ifq: &Ifqueue) -> Option<MbufBox> {
    ifq_deq_with(ifq, |_| IfqDeq::Commit)
}

/// `ifq_deq_sleep`: dequeues a packet, sleeping on the queue until one arrives (`tun(4)`).
/// `EWOULDBLOCK` when empty and `nbio`; `EIO` when `alive` drops to zero while sleeping.
pub fn ifq_deq_sleep(
    ifq: &Ifqueue,
    nbio: bool,
    priority: i32,
    wmesg: &'static str,
    sleeping: &AtomicU32,
    alive: &AtomicU32,
) -> Result<MbufBox, Errno> {
    let mut st = ifq.enter();
    let result = if ifq.ifq_len.load(Ordering::Relaxed) == 0 && nbio {
        Err(Errno::EWOULDBLOCK)
    } else {
        loop {
            if let Some((_, cookie)) = disc_deq_begin(&mut st) {
                let m = disc_deq_commit(&mut st, cookie);
                ifq.ifq_len.fetch_sub(1, Ordering::Relaxed);
                break Ok(m);
            }

            sleeping.fetch_add(1, Ordering::Relaxed);
            let error =
                st.g.sleep_nsec(core::ptr::from_ref(ifq), priority, wmesg, INFSLP);
            sleeping.fetch_sub(1, Ordering::Relaxed);
            if let Err(e) = error {
                break Err(e);
            }
            if alive.load(Ordering::Relaxed) == 0 {
                break Err(Errno::EIO);
            }
        }
    };
    st.leave();

    result
}

/// `ifq_hdatalen`: the length of the packet at the head of the queue, 0 when empty.
pub fn ifq_hdatalen(ifq: &Ifqueue) -> i32 {
    if ifq_empty(ifq) {
        return 0;
    }

    let mut len = 0;
    let _ = ifq_deq_with(ifq, |m| {
        len = m.m_pkthdr().len.get();
        IfqDeq::Rollback
    });
    len
}

/// `ifq_init_maxlen`: sets the queue's length limit. This is not MP safe, use only during
/// attach.
pub fn ifq_init_maxlen(ifq: &Ifqueue, maxlen: u32) {
    ifq.ifq_maxlen.set(maxlen);
}

/// `ifq_purge`: drops every queued packet (counted as `qdrops`); returns how many.
pub fn ifq_purge(ifq: &Ifqueue) -> u32 {
    let mut ml = MbufList::new();

    let mut st = ifq.enter();
    disc_purge(&mut st, &mut ml);
    let rv = ifq.ifq_len.swap(0, Ordering::Relaxed);
    st.g.qdrops += u64::from(rv);
    drop(st);

    kassert!(rv == ml_len(&ml));

    let _ = ml_purge(&mut ml);

    rv
}

/// `ifq_q_enter` and `ifq_q_leave` (the module's redesign): `f` on the conditioner's state
/// word with `ifq_mtx` held, if `ops` is the queue's conditioner; `None` otherwise.
pub fn ifq_q_with<R>(
    ifq: &Ifqueue,
    ops: &'static dyn QueueDiscipline,
    f: impl FnOnce(usize, &mut IfqLocked<'_>) -> R,
) -> Option<R> {
    let mut st = ifq.enter();
    let q = match &st.g.disc {
        Some(Discipline::Ops(cur, q)) if same_ops(*cur, ops) => *q,
        _ => {
            st.leave();
            return None;
        }
    };
    let r = f(q, &mut st);
    st.leave();
    Some(r)
}

/// Whether two conditioners are one table (its data address: a table is a non-zero-sized
/// `static`, docs/IDIOMS.md's `VopsId` lesson).
fn same_ops(a: &dyn QueueDiscipline, b: &dyn QueueDiscipline) -> bool {
    core::ptr::addr_eq(a, b)
}

/// `ifq->ifq_errors++`: a driver's output error on the queue (under `ifq_mtx` here, where
/// the C's drivers increment the counter with no lock: the error path only).
pub fn ifq_inc_errors(ifq: &Ifqueue) {
    ifq.st.lock().errors += 1;
}

/// The packets an input queue holds, taken off (a host test's look at what was queued).
#[cfg(any(test, feature = "doubles"))]
pub fn ifiq_test_take(ifiq: &Ifiqueue) -> MbufList {
    let ml = core::mem::take(&mut ifiq.st.lock().ml);
    ifiq.ml_len.store(0, Ordering::Relaxed);
    ml
}

/// `ifq_len(ifq)`: packets queued, read without the mutex.
pub fn ifq_len(ifq: &Ifqueue) -> u32 {
    ifq.ifq_len.load(Ordering::Relaxed)
}

/// `ifq_empty(ifq)`.
pub fn ifq_empty(ifq: &Ifqueue) -> bool {
    ifq_len(ifq) == 0
}

/// `ifq_is_priq`: whether the queue uses the default conditioner.
pub fn ifq_is_priq(ifq: &Ifqueue) -> bool {
    matches!(ifq.st.lock().disc, Some(Discipline::Priq(_)))
}

/// `ifq_clr_oactive`: the driver's ring has room again.
pub fn ifq_clr_oactive(ifq: &Ifqueue) {
    ifq.ifq_oactive.store(0, Ordering::Relaxed);
}

/// `ifq_is_oactive`.
pub fn ifq_is_oactive(ifq: &Ifqueue) -> bool {
    ifq.ifq_oactive.load(Ordering::Relaxed) != 0
}

/// `ifq_restart`: clears `oactive` and runs the driver's start routine in the serialisation
/// context (from the transmit completion path).
pub fn ifq_restart(ifq: &'static Ifqueue) {
    ifq_serialize(ifq, Pin::static_ref(&ifq.ifq_restart));
}

/// `ifq_idx`: which of `nifqs` send queues gets `m`, as the queue's conditioner decides.
pub fn ifq_idx(ifq: &Ifqueue, nifqs: u32, m: &Mbuf) -> u32 {
    match &ifq.st.lock().disc {
        Some(Discipline::Ops(ops, _)) => ops.idx(nifqs, m),
        _ => priq_idx(nifqs, m),
    }
}

/// `ifq_len` and the counters a queue's conditioner reports (`ifq_kstat_copy`'s, and the
/// tests'): packets, bytes, qdrops, errors, mcasts, oactives.
pub fn ifq_stats(ifq: &Ifqueue) -> (u64, u64, u64, u64, u64, u32) {
    let g = ifq.st.lock();
    (g.packets, g.bytes, g.qdrops, g.errors, g.mcasts, g.oactives)
}

/// The queue's conditioner, `panic(9)` after `ifq_destroy` (the C uses freed memory).
fn disc_of<'a>(st: &'a IfqLocked<'_>) -> &'a Discipline {
    match &st.g.disc {
        Some(d) => d,
        None => kpanic(format_args!("ifqueue used after ifq_destroy")),
    }
}

/// `ifq->ifq_ops->ifqop_enq(ifq, m)`.
fn disc_enq(st: &mut IfqLocked<'_>, m: MbufBox) -> Option<MbufBox> {
    let len = st.ifq_len();
    let maxlen = st.maxlen();
    match &mut st.g.disc {
        Some(Discipline::Priq(pq)) => priq_enq(pq, len, maxlen, m),
        Some(Discipline::Ops(ops, q)) => {
            let (ops, q) = (*ops, *q);
            ops.enq(q, st, m)
        }
        None => kpanic(format_args!("ifqueue used after ifq_destroy")),
    }
}

/// `ifq->ifq_ops->ifqop_deq_begin(ifq, &cookie)`.
fn disc_deq_begin<'a>(st: &'a mut IfqLocked<'_>) -> Option<(&'a Mbuf, usize)> {
    let ops = match disc_of(st) {
        Discipline::Ops(ops, q) => Some((*ops, *q)),
        Discipline::Priq(_) => None,
    };
    match ops {
        Some((ops, q)) => ops.deq_begin(q, st),
        None => match disc_of(st) {
            Discipline::Priq(pq) => priq_deq_begin(pq),
            Discipline::Ops(..) => None,
        },
    }
}

/// `ifq->ifq_ops->ifqop_deq_commit(ifq, m, cookie)`: the packet taken off.
fn disc_deq_commit(st: &mut IfqLocked<'_>, cookie: usize) -> MbufBox {
    match &mut st.g.disc {
        Some(Discipline::Priq(pq)) => priq_deq_commit(pq, cookie),
        Some(Discipline::Ops(ops, q)) => {
            let (ops, q) = (*ops, *q);
            ops.deq_commit(q, st, cookie)
        }
        None => kpanic(format_args!("ifqueue used after ifq_destroy")),
    }
}

/// `ifq->ifq_ops->ifqop_purge(ifq, ml)`.
fn disc_purge(st: &mut IfqLocked<'_>, ml: &mut MbufList) {
    match &mut st.g.disc {
        Some(Discipline::Priq(pq)) => priq_purge(pq, ml),
        Some(Discipline::Ops(ops, q)) => {
            let (ops, q) = (*ops, *q);
            ops.purge(q, st, ml);
        }
        None => {}
    }
}

/// `ifqop_free(idx, q)` of a conditioner taken out of its queue (priq's box freed here).
fn disc_free(d: Option<Discipline>, idx: u32) {
    match d {
        Some(Discipline::Ops(ops, q)) => ops.free(idx, q),
        Some(Discipline::Priq(pq)) => drop(pq),
        None => {}
    }
}

/// `ifiq_maxlen_drop`: `ifiq_input` drops a list when this many packets wait.
pub static IFIQ_MAXLEN_DROP: AtomicU32 = AtomicU32::new(2048 * 5);
/// `ifiq_maxlen_return`: `ifiq_input` reports pressure above this many.
pub static IFIQ_MAXLEN_RETURN: AtomicU32 = AtomicU32::new(2048 * 3);

/// What `ifiq_mtx` keeps.
struct IfiqState {
    /// `ifiq_ml`: packets waiting for `ifiq_process`.
    ml: MbufList,
    /// `ifiq_packets`.
    packets: u64,
    /// `ifiq_bytes`.
    bytes: u64,
    /// `ifiq_fdrops`: dropped by a filter.
    fdrops: u64,
    /// `ifiq_qdrops`: dropped because the queue was full.
    qdrops: u64,
    /// `ifiq_errors` (read by `ifiq_kstat_copy` only: kstat(4) is not configured).
    #[allow(
        dead_code,
        reason = "kstat(4)'s counter, not configured (the module's deviations)"
    )]
    errors: u64,
    /// `ifiq_mcasts` (kstat(4)'s, as `ifiq_errors`).
    #[allow(
        dead_code,
        reason = "kstat(4)'s counter, not configured (the module's deviations)"
    )]
    mcasts: u64,
    /// `ifiq_noproto` (kstat(4)'s, as `ifiq_errors`).
    #[allow(
        dead_code,
        reason = "kstat(4)'s counter, not configured (the module's deviations)"
    )]
    noproto: u64,
    /// `ifiq_enqueues`: number of times a list of packets were put on `ifiq_ml`.
    enqueues: u64,
    /// `ifiq_dequeues`: number of times a list of packets were pulled off `ifiq_ml`.
    dequeues: u64,
}

/// `struct ifiqueue`: an interface receive queue.
pub struct Ifiqueue {
    /// `ifiq_if`: the interface.
    ifiq_if: Link<Ifnet>,
    /// `ifiq_bpfp`: a per-queue `bpf(4)` tap (`caddr_t *`, the driver's word, read by the
    /// `Link` leg).
    pub ifiq_bpfp: PtrWord<*mut u8>,
    /// `ifiq_softnet`: the softnet task queue that processes the packets.
    ifiq_softnet: Link<Taskq>,
    /// `ifiq_softc` (`_ifiq_ptr._ifiq_softc`): the driver's word.
    pub ifiq_softc: PtrWord<c_void>,
    /// `ifiq_mtx` and what it keeps.
    st: Mutexed<IfiqState>,
    /// `ml_len(&ifiq->ifiq_ml)`, read without the mutex (`ifiq_len`).
    ml_len: AtomicU32,
    /// `ifiq_task`: `ifiq_process`.
    ifiq_task: Task,
    /// `ifiq_idx`.
    ifiq_idx: u32,
}

impl Ifiqueue {
    /// `ifiq_init(ifiq, ifp, idx)`'s state: input queue `idx` of `ifp`, processed on
    /// `softnet`. Its task is set once it is in place ([`ifiq_init_tasks`]).
    pub fn new(ifp: &'static Ifnet, idx: u32, softnet: Option<&'static Taskq>) -> Self {
        let ifiq_softnet = Link::new();
        ifiq_softnet.set(softnet);
        Self {
            ifiq_if: Link::to(ifp),
            ifiq_bpfp: PtrWord::new(),
            ifiq_softnet,
            ifiq_softc: PtrWord::new(),
            st: Mutexed::new(
                IPL_NET,
                IfiqState {
                    ml: MbufList::new(),
                    packets: 0,
                    bytes: 0,
                    fdrops: 0,
                    qdrops: 0,
                    errors: 0,
                    mcasts: 0,
                    noproto: 0,
                    enqueues: 0,
                    dequeues: 0,
                },
            ),
            ml_len: AtomicU32::new(0),
            ifiq_task: Task::unset(),
            ifiq_idx: idx,
        }
    }

    /// The queue's interface (`ifiq->ifiq_if`).
    pub fn ifiq_if(&self) -> &'static Ifnet {
        match self.ifiq_if.get() {
            Some(ifp) => ifp,
            None => kpanic(format_args!("ifiqueue used before ifiq_init")),
        }
    }

    /// The queue's softnet task queue.
    fn softnet(&self) -> &'static Taskq {
        match self.ifiq_softnet.get() {
            Some(tq) => tq,
            None => kpanic(format_args!("ifiqueue used before ifiq_init")),
        }
    }

    /// `ifiq_idx`.
    pub fn idx(&self) -> u32 {
        self.ifiq_idx
    }
}

/// `ifiq_init`'s `task_set`, once the queue is where it stays.
pub fn ifiq_init_tasks(ifiq: &'static Ifiqueue) {
    ifiq.ifiq_task.set_ref(ifiq_process, ifiq);
}

/// `ifiq_destroy`: tears the queue down in `if_detach` (the net lock not held).
pub fn ifiq_destroy(ifiq: &Ifiqueue) {
    // NKSTAT > 0: kstat_destroy(ifiq->ifiq_kstat): not configured.

    taskq_del_barrier(ifiq.softnet(), &ifiq.ifiq_task);

    // don't need to lock because this is the last use of the ifiq
    let mut ml = core::mem::take(&mut ifiq.st.lock().ml);
    ifiq.ml_len.store(0, Ordering::Relaxed);
    let _ = ml_purge(&mut ml);
}

/// `ifiq_input`: a driver's received packets (`if_input`): stamps them with the interface,
/// taps them, counts them and queues them for the softnet task. Returns `true` when the queue
/// is under pressure (the driver may slow its ring refill, `if_rxr_livelocked`).
pub fn ifiq_input(ifiq: &'static Ifiqueue, ml: &mut MbufList) -> bool {
    let ifp = ifiq.ifiq_if();
    let mut bytes: u64 = 0;
    let mut fdrops: u64 = 0;

    if ml_empty(ml) {
        return false;
    }

    for m in ml.iter() {
        let ph = m.m_pkthdr();
        ph.ph_ifidx.set(ifp.if_index.get());
        ph.ph_rtableid.set(ifp.if_rdomain.get());
        bytes += u64::try_from(ph.len.get()).unwrap_or(0);
    }
    let packets = u64::from(ml_len(ml));

    let link = crate::net::if_::link();
    let ifiq_bpf = link.map_or(core::ptr::null(), |l| l.bpf_queue_tap(&ifiq.ifiq_bpfp));
    let if_bpf = ifp.if_bpf.get();
    if !ifiq_bpf.is_null() || !if_bpf.is_null() {
        let mut ml0 = core::mem::take(ml);
        let driver = ifp.driver();
        let mtap = |arg: *const u8, m: &Mbuf| {
            !arg.is_null() && driver.bpf_mtap(arg, m, crate::net::if_::BPF_DIRECTION_IN)
        };
        while let Some(m) = ml_dequeue(&mut ml0) {
            let mut drop_it = false;
            if mtap(ifiq_bpf, &m) {
                drop_it = true;
            }
            if mtap(if_bpf, &m) {
                drop_it = true;
            }
            if drop_it {
                drop(m);
                fdrops += 1;
            } else {
                ml_enqueue(ml, m);
            }
        }

        if ml_empty(ml) {
            let mut g = ifiq.st.lock();
            g.packets += packets;
            g.bytes += bytes;
            g.fdrops += fdrops;
            return false;
        }
    }

    let len = {
        let mut g = ifiq.st.lock();
        g.packets += packets;
        g.bytes += bytes;
        g.fdrops += fdrops;

        let len = ml_len(&g.ml);
        if ifp.if_xflags.get() & IFXF_MONITOR == 0 {
            if len > IFIQ_MAXLEN_DROP.load(Ordering::Relaxed) {
                g.qdrops += u64::from(ml_len(ml));
            } else {
                g.enqueues += 1;
                ml_enlist(&mut g.ml, ml);
                ifiq.ml_len.store(ml_len(&g.ml), Ordering::Relaxed);
            }
        }
        len
    };

    if ml_empty(ml) {
        let _ = task_add(ifiq.softnet(), Pin::static_ref(&ifiq.ifiq_task));
    } else {
        let _ = ml_purge(ml);
    }

    len > IFIQ_MAXLEN_RETURN.load(Ordering::Relaxed)
}

/// `ifiq_add_data`: adds the queue's counters to `data` (`SIOCGIFDATA`).
pub fn ifiq_add_data(ifiq: &Ifiqueue, data: &mut IfData) {
    let g = ifiq.st.lock();
    data.ifi_ipackets += g.packets;
    data.ifi_ibytes += g.bytes;
    data.ifi_iqdrops += g.qdrops;
}

/// `ifiq_enqueue_qlim`: queues one packet for the softnet task; with `qlim` nonzero the
/// packet is dropped when that many already wait. Always `Ok` (the C returns 0).
pub fn ifiq_enqueue_qlim(ifiq: &'static Ifiqueue, m: MbufBox, qlim: u32) -> Result<(), Errno> {
    let ifp = ifiq.ifiq_if();
    let len = u64::try_from(m.m_pkthdr().len.get()).unwrap_or(0);

    m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
    m.m_pkthdr().ph_rtableid.set(ifp.if_rdomain.get());

    let if_bpf = ifp.if_bpf.get();
    if !if_bpf.is_null()
        && ifp
            .driver()
            .bpf_mtap(if_bpf, &m, crate::net::if_::BPF_DIRECTION_IN)
    {
        {
            let mut g = ifiq.st.lock();
            g.packets += 1;
            g.bytes += len;
            g.fdrops += 1;
        }
        drop(m);
        return Ok(());
    }

    let dropped = {
        let mut g = ifiq.st.lock();
        g.packets += 1;
        g.bytes += len;

        if qlim != 0 && ml_len(&g.ml) >= qlim {
            g.qdrops += 1;
            Some(m)
        } else {
            g.enqueues += 1;
            ml_enqueue(&mut g.ml, m);
            ifiq.ml_len.store(ml_len(&g.ml), Ordering::Relaxed);
            None
        }
    };

    if let Some(m) = dropped {
        drop(m);
        return Ok(());
    }

    let _ = task_add(ifiq.softnet(), Pin::static_ref(&ifiq.ifiq_task));

    Ok(())
}

/// `ifiq_enqueue`: `ifiq_enqueue_qlim` without a limit.
pub fn ifiq_enqueue(ifiq: &'static Ifiqueue, m: MbufBox) -> Result<(), Errno> {
    ifiq_enqueue_qlim(ifiq, m, 0)
}

/// `ifiq_len(ifiq)`: packets waiting, read without the mutex.
pub fn ifiq_len(ifiq: &Ifiqueue) -> u32 {
    ifiq.ml_len.load(Ordering::Relaxed)
}

/// `ifiq_empty(ifiq)`.
pub fn ifiq_empty(ifiq: &Ifiqueue) -> bool {
    ifiq_len(ifiq) == 0
}

/// The receive counters (the tests' and `ifiq_kstat_copy`'s): packets, bytes, fdrops,
/// qdrops, enqueues, dequeues.
pub fn ifiq_stats(ifiq: &Ifiqueue) -> (u64, u64, u64, u64, u64, u64) {
    let g = ifiq.st.lock();
    (
        g.packets, g.bytes, g.fdrops, g.qdrops, g.enqueues, g.dequeues,
    )
}

/// `ifiq_process`: the softnet task: takes the waiting packets and hands them to
/// `if_input_process`.
fn ifiq_process(t: &'static (dyn Any + Send + Sync)) {
    let Some(ifiq) = t.downcast_ref::<Ifiqueue>() else {
        kpanic(format_args!("ifiq_process without its queue"));
    };

    if ifiq_empty(ifiq) {
        return;
    }

    let mut ml = {
        let mut g = ifiq.st.lock();
        g.dequeues += 1;
        ifiq.ml_len.store(0, Ordering::Relaxed);
        core::mem::take(&mut g.ml)
    };

    crate::net::if_::if_input_process(ifiq.ifiq_if(), &mut ml, ifiq.ifiq_idx);
}

/// `net_ifiq_sysctl`: `net.link.ifrxq`; the pressure knobs are disabled (`#if 0` in C since
/// 6.6), so every request is `EOPNOTSUPP`.
pub fn net_ifiq_sysctl(_name: &[i32]) -> Result<(), Errno> {
    // pressure is disabled for 6.6-release
    Err(Errno::EOPNOTSUPP)
}

/// `priq_idx`: the flow id picks the queue.
fn priq_idx(nqueues: u32, m: &Mbuf) -> u32 {
    let mut flow = 0;

    if m.m_pkthdr().csum_flags.get() & M_FLOWID != 0 {
        flow = u32::from(m.m_pkthdr().ph_flowid.get());
    }

    flow % nqueues
}

/// `priq_enq`: queues `m` on the list of its priority; when the queue (`len` packets of
/// `maxlen`) is full, drops a packet of a lower priority instead, or refuses `m` if there is
/// none.
fn priq_enq(pq: &mut Priq, len: u32, maxlen: u32, m: MbufBox) -> Option<MbufBox> {
    let mprio = usize::from(m.m_pkthdr().pf.prio.get());
    kassert!(mprio as u32 <= IFQ_MAXPRIO);

    let mut n = None;
    // Find a lower priority queue to drop from
    if len >= maxlen {
        match pq.pq_lists[..mprio].iter_mut().find(|pl| !ml_empty(pl)) {
            Some(pl) => n = ml_dequeue(pl),
            // There's no lower priority queue that we can drop from so don't enqueue this
            // one.
            None => return Some(m),
        }
    }

    ml_enqueue(&mut pq.pq_lists[mprio], m);

    n
}

/// `priq_deq_begin`: the first packet of the highest priority, with its list as the cookie.
fn priq_deq_begin(pq: &Priq) -> Option<(&Mbuf, usize)> {
    pq.pq_lists
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, pl)| pl.first().map(|m| (m, i)))
}

/// `priq_deq_commit`: the first packet of the list `cookie` names, taken off.
fn priq_deq_commit(pq: &mut Priq, cookie: usize) -> MbufBox {
    match pq.pq_lists.get_mut(cookie).and_then(ml_dequeue) {
        Some(m) => m,
        None => kpanic(format_args!("priq_deq_commit: an empty list")),
    }
}

/// `priq_purge`: moves every packet to `ml`, highest priority first.
fn priq_purge(pq: &mut Priq, ml: &mut MbufList) {
    for pl in pq.pq_lists.iter_mut().rev() {
        ml_enlist(ml, pl);
    }
}
/* </CODE> */
