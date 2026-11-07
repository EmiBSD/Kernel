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
 * Copyright (c) 2019-2020 Visa Hankala
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
 * Copyright (c) 2019 Visa Hankala
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
 * Copyright (c) 1991, 1993
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
 *	@(#)queue.h	8.5 (Berkeley) 8/20/94
 */
/* </LICENSES> */

/* <CODE> */
//! Safe memory reclamation, `smr_call(9)`: readers walk shared data inside a read section
//! without locks; a writer unlinks an object under its lock and hands its release to the
//! deferred queue, which runs it once every CPU has passed a quiescent state (a context
//! switch or the idle loop: [`idle`]) after the release was queued, so no read section that
//! could have seen the object is still open. The protocol of `kern_smr.c` and `<sys/smr.h>`:
//! the read sections ([`read`], [`read_enter`]/[`read_leave`]), the per-CPU queues and the
//! system's ([`defer_link`], `smr_dispatch`), the grace period (`grace_wait`, which only
//! [`Batch::run`] calls, before it runs anything), the barrier ([`barrier`]); the SMR thread
//! that drives it is `kernel::smr`'s (it needs the kernel's clock, `ratecheck` and
//! `kthread_create`), and the CPU walk it waits over is the kernel's scheduler, handed to
//! [`Batch::run`] as a [`CpuWalk`].
//!
//! Upstream: sys/kern/kern_smr.c @ 3ce1f3f79392
//! Upstream: sys/sys/smr.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_smr.rs@3c62ede68802
//! LZ: sys/sys/smr.rs@3c62ede68802
//!
//! **The queues.** A deferred call is an [`SmrLink`] (`struct smr_entry`: three words, the
//! next link, the function, its argument) embedded in the object it releases, so a deferral
//! allocates nothing. A link is idle, queued (on exactly one list: its CPU's, the system's,
//! or the SMR thread's private batch) or run; [`defer_link`] claims an idle (or run) link by
//! a compare-and-swap of its state, so two CPUs never queue one link, and the thread marks
//! it run after reading its function and argument and before calling it, the call being its
//! last use of the link. A CPU's list is changed at `splhigh` by that CPU (the C's
//! `smr_call_impl`); the system's list under [`DEFERRED`]'s mutex at `IPL_HIGH` (`smr_lock`);
//! the batch by the SMR thread alone.
//!
//! **The grace period.** [`GRACE_PERIOD`] is announced by `grace_wait` (incremented, one
//! caller at a time: [`GRACE`]); each CPU's view of it advances only in [`idle`], which is
//! called from the scheduler's switch and idle loop, never inside a read section (checked
//! always, not only under `diagnostic`: it is the premise of [`SmrPtr::load`]). `grace_wait`
//! makes every running CPU whose view is behind pass [`idle`] (the calling thread pegged to it
//! in turn, [`CpuWalk::peg`]) and checks that it did, always, so when it returns every read
//! section open at the announcement has ended. It runs only inside [`Batch::run`], before the
//! batch's calls, so no release can be run without one.
//!
//! **Two read-section counts.** [`read`]'s guards count in one per-CPU word, the C-style
//! sections of [`read_enter`]/[`read_leave`] (`bsd`'s) in another, so a stray
//! [`read_leave`] can end only a C-style section, never a guard's that a loaded reference
//! rests on. [`idle`] refuses either.
//!
//! ## Soundness
//!
//! - S1, links: a pointer on a list names a queued [`SmrLink`] that stays valid until the
//!   thread marks it run: one of `bsd`'s `'static` entries ([`defer_link`]), the link of an
//!   item a [`SmrRetired`] gave up to the queue (the item freed only by its release, after
//!   the mark), or [`barrier`]'s own link on its frame (which returns only once it sees the
//!   mark). A list changes only under its owner (the CPU at `splhigh`, the mutex, the
//!   thread), so its walks never race.
//! - S2, the published target: an [`SmrPtr`]'s word names the item its owner cell holds,
//!   alive while published; a target leaves only through [`SmrPtr::store`] into an
//!   [`SmrRetired`] whose one way out is its deferral, released by the thread only after a
//!   [`grace_wait`] that started after the retirement.
//! - S3, quiescence: a CPU inside a read section never runs [`idle`] ([`idle`] panics on a
//!   non-zero depth, and [`SmrRead`] is neither `Send` nor `Sync`, so it stays on its CPU;
//!   only its drop lowers its count), so its view cannot reach a period announced after its
//!   section began; `grace_wait` cannot return while that section lives, given the
//!   [`CpuWalk`] it is handed: `running(n)` must not call a CPU that runs threads not
//!   running (`sys/hal/trust.rs`, the walk's contract, which the soundness rests on; `hal`
//!   checks it only at best, by relaxed loads of a skipped CPU's counts, and panics when it
//!   happens to see one inside a section), and a `peg` that does not make the CPU pass
//!   [`idle`] is `panic(9)` (checked after every peg, in every kernel).
//!
//! ## Deviations
//! - `WITNESS` and `dt(4)` are not configured (the C's `smr_lock_obj` and `TRACEPOINT`s).
//! - The read-section depth is counted always, not only under `DIAGNOSTIC`, and [`idle`]
//!   inside a read section is `panic(9)` always: a safe reader's reference must not outlive
//!   its protection (S3). One relaxed load and store per enter and leave.
//! - [`barrier`] dispatches its CPU's queue itself once its link is queued, where the C
//!   leaves that to the switch its sleep makes (`smr_idle`): the same moment.
//!
//! ## Redesign
//! - LZ's `SmrEntry` with a `SimpleqEntry` link, its `unsafe impl Sync`, and the C's
//!   `smr_call(&'static SmrEntry, func, arg)` whose double queueing the C catches only under
//!   `DIAGNOSTIC` (`KASSERT(smr->smr_func == NULL)`) -> [`SmrLink`], whose state word makes a
//!   second queueing of a queued link `panic(9)` in every kernel (a corrupted list would be
//!   memory unsafety); a run link may be queued again, as in the C, and `diagnostic` keeps the
//!   C's `KASSERT` on it. `smr_init` is [`SmrLink::init`].
//! - `kern_smr.c`'s globals under `smr_lock` (`smr_deferred`, `smr_ndeferred`,
//!   `smr_expedite`) -> one `Mutexed<Deferred>` at `IPL_HIGH` ([`DEFERRED`]); the five SMR
//!   members of `struct schedstate_percpu` (`spc_deferred`, `spc_ndeferred`, `spc_smrdepth`,
//!   `spc_smrexpedite`, `spc_smrgp`) -> this module's `PerCpu<SmrCpu>`, with the CPU's idle
//!   thread's address beside them for `smr_cpu_is_idle` ([`set_idle_thread`]).
//! - `smr_thread`'s two sleeps and its queue concatenation -> [`wait`] and [`collect`] (a
//!   [`Batch`] that [`Batch::run`] runs); the thread itself is `kernel::smr::thread`.
//!   `smr_wakeup` and `smr_wakeup_tmo` are gone: a dispatch that finds the system's queue
//!   empty wakes the thread at once, and the thread pauses `SMR_PAUSE` before collecting
//!   unless the work is expedited, so a first call still waits about 100 ms, as the C's
//!   timeout makes it, and `hal` needs no timeout.
//! - `smr_grace_wait`'s `CPU_INFO_FOREACH`, `CPU_IS_RUNNING` and `sched_peg_curproc` -> a
//!   [`CpuWalk`] the caller passes (the kernel's scheduler; a test's fake): a parameter, not a
//!   bridge (decision 39).
//! - `smr_barrier_impl`'s `struct cond` and the `'static` entry LZ lent from its frame by an
//!   `unsafe` cast -> [`barrier`]'s link on its frame, queued by pointer, the wait's
//!   condition the link's own run mark: no cast, no lifetime extended.
//! - `SMR_PTR_GET`/`SMR_PTR_GET_LOCKED`/`SMR_PTR_SET_LOCKED` over a raw pointer -> [`SmrPtr`]:
//!   a published pool item ([`SmrLinked`]), loaded inside a read section for that section's
//!   lifetime ([`SmrPtr::load`]) or under the writer's lock, which the pointer is keyed to as
//!   a `Guarded` cell is ([`SmrPtr::load_locked`]); a store gives the old target back as an
//!   [`SmrRetired`], whose only way out is the deferred release (its drop defers it), so an
//!   object can be neither freed early nor leaked by accident. `bsd`'s `SmrPtr`,
//!   `SmrSlistHead` and `SmrSlistEntry` (`sys/sys/smr.rs`) stay for their users until those
//!   move (N6.3, N8).

use core::cell::Cell;
use core::ffi::c_void;
use core::marker::PhantomData;
use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, AtomicU8, AtomicU32, AtomicUsize, Ordering, fence};

use crate::assert::{hal_panic, kassert};
use crate::cpu;
use crate::intr::{IPL_HIGH, splhigh};
use crate::mutex::Mutexed;
use crate::percpu::{PerCpu, PerCpuInit};
use crate::pool::{BoxPool, PoolBox};
use crate::rwlock::{Exclusive, Guarded, RawRwlock, Witness};
use crate::sleep::{self, INFSLP, PVM};

/// `SMR_PAUSE`: the pause between rounds, 100 ms, in nanoseconds.
const SMR_PAUSE_NSEC: u64 = 100_000_000;

/// A link's state word when it is the last of its list (a queued link otherwise holds its
/// successor). Never an address: links are aligned to 8.
const TAIL: *mut SmrLink = ptr::without_provenance_mut(1);

/// A link's state word once the SMR thread took it off its batch to run its call.
const RAN: *mut SmrLink = ptr::without_provenance_mut(2);

/// `struct smr_entry`: a deferred call, embedded in the object it releases. Idle as made
/// (all zero, as the C's `M_ZERO` memory and `smr_init` leave it).
pub struct SmrLink {
    /// `smr_list`: null while idle; while queued the next link or `TAIL`; `RAN` once run.
    next: AtomicPtr<SmrLink>,
    /// `smr_func`: written by the CPU that claimed the link, before it is on a list; read by
    /// the SMR thread after taking it off its batch.
    func: Cell<Option<fn(*mut c_void)>>,
    /// `smr_arg`.
    arg: AtomicPtr<c_void>,
}

impl SmrLink {
    /// An idle link.
    pub const fn new() -> Self {
        Self {
            next: AtomicPtr::new(ptr::null_mut()),
            func: Cell::new(None),
            arg: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// `smr_init(smr)`: the link idle again once its call ran (nothing for an idle one).
    /// `panic(9)` for a queued link, whose list would be cut.
    pub fn init(&self) {
        match self
            .next
            .compare_exchange(RAN, ptr::null_mut(), Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {}
            Err(p) if p.is_null() => {}
            Err(_) => hal_panic(format_args!("smr_init: entry on a queue")),
        }
    }

    /// Claims the link for one deferral of `func(arg)`: idle (or run, as the C allows) to
    /// queued-and-last. `panic(9)` for a link already queued.
    fn claim(&self, func: fn(*mut c_void), arg: *mut c_void) {
        let mut from = ptr::null_mut();
        loop {
            match self
                .next
                .compare_exchange(from, TAIL, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => break,
                Err(p) if p == RAN && from.is_null() => {
                    // KASSERT(smr->smr_func == NULL): the C asks for `smr_init` between two
                    // calls under `DIAGNOSTIC`.
                    kassert!(p != RAN);
                    from = RAN;
                }
                Err(_) => hal_panic(format_args!("smr_call: entry already queued")),
            }
        }
        // The claim made this CPU the link's only writer until it is on a list.
        self.func.set(Some(func));
        self.arg.store(arg, Ordering::Relaxed);
    }
}

impl Default for SmrLink {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: `SmrLink` is `Sync` for its one non-atomic field, `func` (a `Cell`): it is written
// only by the CPU whose compare-and-swap of `next` won the link from idle or run (`claim`),
// before the link is on any list, and read only by the SMR thread after taking the link off
// its private batch; the claim's `AcqRel` exchange, the list moves under `splhigh` and the
// mutex, and the thread's `Release` store of `RAN` after its read order those accesses, so no
// two of them race. Every other access is atomic.
unsafe impl Sync for SmrLink {}

/// A singly-linked list of queued links, oldest first (`SIMPLEQ`), changed only by its owner
/// (S1); the words are atomics so the lists can be statics.
struct Chain {
    head: AtomicPtr<SmrLink>,
    tail: AtomicPtr<SmrLink>,
}

impl Chain {
    const fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
            tail: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// `SIMPLEQ_INSERT_TAIL` of a link just claimed (its word is `TAIL`).
    fn append(&self, l: NonNull<SmrLink>) {
        let l = l.as_ptr();
        let t = self.tail.load(Ordering::Relaxed);
        if t.is_null() {
            self.head.store(l, Ordering::Relaxed);
        } else {
            link_ref(t).next.store(l, Ordering::Release);
        }
        self.tail.store(l, Ordering::Relaxed);
    }

    /// `SIMPLEQ_CONCAT(self, other)`: `other`'s links after this list's, `other` emptied.
    fn concat(&self, other: &Chain) {
        let h = other.head.swap(ptr::null_mut(), Ordering::Relaxed);
        let t = other.tail.swap(ptr::null_mut(), Ordering::Relaxed);
        if h.is_null() {
            return;
        }
        let mine = self.tail.load(Ordering::Relaxed);
        if mine.is_null() {
            self.head.store(h, Ordering::Relaxed);
        } else {
            link_ref(mine).next.store(h, Ordering::Release);
        }
        self.tail.store(t, Ordering::Relaxed);
    }
}

/// One CPU's SMR state (the five members of `struct schedstate_percpu` and its idle thread).
struct SmrCpu {
    /// `spc_deferred`: the calls this CPU queued, changed at `splhigh` by this CPU.
    deferred: Chain,
    /// `spc_ndeferred`.
    ndeferred: AtomicU32,
    /// `spc_smrdepth`, the guards' half: the [`SmrRead`] sections open on this CPU (counted
    /// always: S3).
    depth: AtomicU32,
    /// `spc_smrdepth`, the C-style half: the [`read_enter`] sections open on this CPU.
    cdepth: AtomicU32,
    /// `spc_smrexpedite`.
    expedite: AtomicU8,
    /// `spc_smrgp`: this CPU's view of the grace period, read by [`grace_wait`] elsewhere.
    gp: AtomicU8,
    /// `spc_idleproc`'s address (0 before the scheduler names it), for `smr_cpu_is_idle`.
    idle: AtomicUsize,
}

impl PerCpuInit for SmrCpu {
    const INIT: Self = SmrCpu {
        deferred: Chain::new(),
        ndeferred: AtomicU32::new(0),
        depth: AtomicU32::new(0),
        cdepth: AtomicU32::new(0),
        expedite: AtomicU8::new(0),
        gp: AtomicU8::new(0),
        idle: AtomicUsize::new(0),
    };
}

/// `smr_deferred`, `smr_ndeferred` and `smr_expedite`: the calls every CPU dispatched,
/// waiting for the SMR thread.
struct Deferred {
    links: Chain,
    n: u32,
    expedite: bool,
}

/// A read section (`smr_read_enter` .. `smr_read_leave`): what an [`SmrPtr::load`] borrows.
/// Neither `Send` nor `Sync`: it belongs to the CPU it was opened on, which must not switch
/// threads (sleep, yield) while it lives; [`idle`] refuses to run under one.
#[must_use = "dropping the section at once ends it"]
pub struct SmrRead {
    /// The CPU the section was opened on (the guard never leaves it).
    cpu: &'static SmrCpu,
    _not_send: PhantomData<*const ()>,
}

impl Drop for SmrRead {
    #[inline]
    fn drop(&mut self) {
        leave(self.cpu);
    }
}

/// An object [`SmrPtr`] publishes: a pool item of its own type's one pool, carrying the
/// [`SmrLink`] its deferred release is queued by (`struct smr_entry` embedded in the object,
/// the C's `if_map`, `bpf_program_smr`).
pub trait SmrLinked: Send + Sync + Sized + 'static {
    /// The pool every published item of the type comes from (checked at
    /// [`SmrPtr::store`]), which its release puts it back into.
    const POOL: &'static BoxPool<Self>;
    /// The item's own link (checked to lie within the item).
    fn smr_link(&self) -> &SmrLink;
}

/// `SMR_PTR_GET`/`SMR_PTR_GET_LOCKED`/`SMR_PTR_SET_LOCKED` over one published object: readers
/// load it inside a read section; the writer, holding the lock the pointer is keyed to, loads
/// it and stores a new one, the old one coming back as an [`SmrRetired`].
pub struct SmrPtr<T: SmrLinked> {
    /// What readers load (`Acquire`): the owner cell's item, null when it holds none.
    word: AtomicPtr<T>,
    /// The published item's box, under the writer's lock: moved whole, never dereferenced
    /// mutably, so the readers' shared references coexist with it.
    owner: Guarded<Option<PoolBox<T>>>,
}

impl<T: SmrLinked> SmrPtr<T> {
    /// A null pointer whose writers hold the `static` lock `lock`.
    pub const fn new_static(lock: &'static RawRwlock) -> Self {
        Self {
            word: AtomicPtr::new(ptr::null_mut()),
            owner: Guarded::new_static(lock, None),
        }
    }

    /// A null pointer whose writers hold `lock` (a lock embedded beside it).
    pub fn new_keyed(lock: &RawRwlock) -> Self {
        Self {
            word: AtomicPtr::new(ptr::null_mut()),
            owner: Guarded::new_keyed(lock, None),
        }
    }

    /// `SMR_PTR_GET(pptr)`: the published object, for the read section `g`.
    pub fn load<'g>(&'g self, _g: &'g SmrRead) -> Option<&'g T> {
        let p = self.word.load(Ordering::Acquire);
        // SAFETY: a non-null word was stored by `store` from the item its owner cell then
        // held (`PoolBox::as_ptr`, the box's own pointer), an initialised `T` that only shared
        // references reach (the cell never dereferences it mutably). That item stays alive
        // while published, and once replaced it is held by an `SmrRetired` whose only way out
        // is its deferral, run by the SMR thread only after a `grace_wait` announced after
        // the retirement (S2). The section `g` was opened on this CPU before this load and
        // stays on it (`SmrRead` is neither `Send` nor `Sync`), and this CPU's view of the
        // grace period cannot advance while it lives (`idle` panics under a read section, and
        // only `g`'s drop lowers the guards' count, S3), so that `grace_wait` cannot return,
        // nor the item be freed, before `g` ends: `grace_wait` waits for every CPU the
        // `CpuWalk` calls running (its contract, a trust boundary of `sys/hal/trust.rs`: a CPU
        // that may run threads is called running) and checks that each it pegged passed
        // `idle`. The reference borrows `g` and `self`, so it cannot outlive either.
        unsafe { p.as_ref() }
    }

    /// `SMR_PTR_GET_LOCKED(pptr)`: the published object, for the writer holding the lock the
    /// pointer is keyed to (a witness of another lock is `panic(9)`).
    pub fn load_locked<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a T> {
        self.owner.get(w).as_deref()
    }

    /// `SMR_PTR_SET_LOCKED(pptr, val)`: publishes `new` (or nothing) once everything it holds
    /// is written (`Release`), under the writer's lock held alone; the old object, if any,
    /// comes back retired. `panic(9)` for an item of another pool than `T::POOL`.
    pub fn store<W: Exclusive + ?Sized>(
        &self,
        w: &mut W,
        new: Option<PoolBox<T>>,
    ) -> Option<SmrRetired<T>> {
        if let Some(b) = &new {
            check_pool(b);
        }
        let slot = self.owner.get_mut(w);
        let p = new
            .as_ref()
            .map_or(ptr::null_mut(), |b| PoolBox::as_ptr(b).cast_mut());
        let old = mem::replace(slot, new);
        self.word.store(p, Ordering::Release);
        old.map(|b| SmrRetired { item: Some(b) })
    }

    /// `SMR_PTR_SET_LOCKED(pptr, NULL)`: unpublishes the object, which comes back retired.
    pub fn take<W: Exclusive + ?Sized>(&self, w: &mut W) -> Option<SmrRetired<T>> {
        self.store(w, None)
    }
}

impl<T: SmrLinked> Drop for SmrPtr<T> {
    fn drop(&mut self) {
        // No reader borrows the pointer any more (`&mut self`); the object goes the deferred
        // way all the same, as the C's destroyers do.
        self.word.store(ptr::null_mut(), Ordering::Relaxed);
        if let Some(b) = self.owner.get_mut_owned().take() {
            drop(SmrRetired { item: Some(b) });
        }
    }
}

/// An object unpublished by [`SmrPtr::store`], which readers may still be looking at: its
/// one way out is [`SmrRetired::defer`] (its drop defers it too), the release after the next
/// grace period. Neither the object nor its box can be reached through it.
#[must_use = "a retired object is released by its deferral (dropping it defers it)"]
pub struct SmrRetired<T: SmrLinked> {
    /// The box, until it is given up to the queue.
    item: Option<PoolBox<T>>,
}

impl<T: SmrLinked> SmrRetired<T> {
    /// `smr_call(&obj->smr, release, obj)`: the object's link queued on this CPU, the box
    /// given up to the queue, put back into `T::POOL` by the SMR thread after a grace period.
    pub fn defer(mut self) {
        if let Some(b) = self.item.take() {
            defer_box(b);
        }
    }
}

impl<T: SmrLinked> Drop for SmrRetired<T> {
    fn drop(&mut self) {
        if let Some(b) = self.item.take() {
            defer_box(b);
        }
    }
}

/// A batch of deferred calls taken by [`collect`] for the SMR thread, oldest first; nothing
/// else reaches its links. Its calls run only after a grace period ([`Batch::run`]); dropped
/// without running, its links stay queued for ever.
#[must_use = "a batch not run leaves its calls queued for ever"]
pub struct Batch {
    head: *mut SmrLink,
}

impl Batch {
    /// `smr_grace_wait()` over `cpus`, then the calls, in their order: each link marked run,
    /// then its function called with its argument (the call may free the object the link is
    /// in); how many ran. `panic(9)` inside a read section of this CPU (its own quiescent
    /// state would be a lie).
    pub fn run(self, cpus: &dyn CpuWalk) -> u64 {
        grace_wait(cpus);
        let mut count = 0;
        let mut p = self.head;
        while !p.is_null() && p != TAIL {
            let l = link_ref(p);
            let next = l.next.load(Ordering::Acquire);
            let func = l.func.get();
            let arg = l.arg.load(Ordering::Relaxed);
            // The link's last use: from here it may be claimed again or freed.
            l.next.store(RAN, Ordering::Release);
            match func {
                Some(f) => f(arg),
                None => hal_panic(format_args!("smr_thread: entry without a function")),
            }
            count += 1;
            p = next;
        }
        count
    }
}

/// The CPUs a grace period waits for, as the scheduler knows them (`CPU_INFO_FOREACH`,
/// `CPU_IS_RUNNING`, `sched_peg_curproc`): the kernel's, or a test's fake.
///
/// A trust boundary (`sys/hal/trust.rs`): [`CpuWalk::running`] must answer `true` for every
/// CPU that may run a thread, since a CPU it skips is not waited for; `hal` refuses a skipped
/// CPU it finds inside a read section and checks after every [`CpuWalk::peg`] that the CPU
/// passed a quiescent state, so a wrong `peg` is `panic(9)`, never a release too early.
pub trait CpuWalk {
    /// `CPU_IS_RUNNING(ci)` of CPU `n`: the contract above.
    fn running(&self, n: usize) -> bool;
    /// `sched_peg_curproc(ci)`: the calling thread moves to CPU `n` (whose switch runs
    /// [`idle`] there) and stays there.
    fn peg(&self, n: usize);
    /// `sched_unpeg_curproc()`.
    fn unpeg(&self);
}

/// The SMR state of every CPU.
static CPUS: PerCpu<SmrCpu> = PerCpu::new();

/// `smr_lock` with what it guards.
static DEFERRED: Mutexed<Deferred> = Mutexed::new(
    IPL_HIGH,
    Deferred {
        links: Chain::new(),
        n: 0,
        expedite: false,
    },
);

/// `smr_grace_period`: the current grace period, announced by `grace_wait`.
static GRACE_PERIOD: AtomicU8 = AtomicU8::new(0);

/// One `grace_wait` at a time (the C has one SMR thread), so a CPU's view is checked against
/// the one period announced, by equality.
static GRACE: RawRwlock = RawRwlock::new("smrgrace");

/// The SMR thread's wait channel (the C's `&smr_ndeferred`).
static WAIT: u8 = 0;

/// A queued link, by its pointer on a list (S1).
fn link_ref<'a>(p: *mut SmrLink) -> &'a SmrLink {
    // SAFETY: every caller passes a pointer read from a list (a `Chain`'s words, a queued
    // link's `next`, a `Batch`'s head) while it owns that list (S1: the CPU at `splhigh`, the
    // mutex, the thread), and never `TAIL` or `RAN` (both checked first). Such a pointer was
    // put there by `append`/`concat` from a `NonNull` of a claimed link, which stays valid
    // until the thread marks it run: a `'static` entry, an item kept alive by its queued
    // release, or `barrier`'s frame, which waits for the mark. The reference is used before
    // that mark (`Batch::run` reads the link before marking it), and links are only read
    // through it (their words are atomics, `func` is read by its one reader).
    unsafe { &*p }
}

/// `smr_cpu_is_idle(ci)` of the running CPU: it runs its idle thread (an interrupt that
/// preempted the idle loop).
fn cpu_is_idle(c: &SmrCpu) -> bool {
    let idle = c.idle.load(Ordering::Relaxed);
    idle != 0 && cpu::curcpu().curproc().addr() == idle
}

/// The scheduler names CPU `n`'s idle thread (`spc_idleproc`, its address), for
/// `smr_cpu_is_idle`.
pub fn set_idle_thread(n: usize, addr: usize) {
    if let Some(c) = CPUS.cpu(n) {
        c.idle.store(addr, Ordering::Relaxed);
    }
}

/// One more read section open on `c`, this CPU's state. Only this CPU writes its depth; an
/// interrupt's sections nest and leave it as they found it.
#[inline]
fn enter(c: &SmrCpu) {
    c.depth
        .store(c.depth.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
}

/// One read section fewer on `c`, this CPU's state. `panic(9)` when none is open.
#[inline]
fn leave(c: &SmrCpu) {
    match c.depth.load(Ordering::Relaxed).checked_sub(1) {
        Some(n) => c.depth.store(n, Ordering::Relaxed),
        None => hal_panic(format_args!("smr_read_leave: no read section")),
    }
}

/// `smr_read_enter()`: opens a C-style read section on this CPU (a section begun in one
/// function and ended in another, `bsd`'s); new code takes [`read`].
pub fn read_enter() {
    let c = &CPUS.get().cdepth;
    c.store(c.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
}

/// `smr_read_leave()`: ends a C-style read section opened on this CPU ([`read_enter`]'s; a
/// guard's is ended only by its drop). `panic(9)` when none is open.
pub fn read_leave() {
    let c = &CPUS.get().cdepth;
    match c.load(Ordering::Relaxed).checked_sub(1) {
        Some(n) => c.store(n, Ordering::Relaxed),
        None => hal_panic(format_args!("smr_read_leave: no read section")),
    }
}

/// The read sections of both kinds open on `c`.
fn sections(c: &SmrCpu) -> u32 {
    c.depth.load(Ordering::Relaxed) + c.cdepth.load(Ordering::Relaxed)
}

/// `smr_read_enter()` .. `smr_read_leave()` as a guard.
#[inline]
pub fn read() -> SmrRead {
    let cpu = CPUS.get();
    enter(cpu);
    SmrRead {
        cpu,
        _not_send: PhantomData,
    }
}

/// `SMR_ASSERT_CRITICAL()` (`diagnostic`): inside a read section (nothing after a panic or in
/// ddb).
pub fn assert_critical() {
    if !cpu::past_caring() {
        kassert!(sections(CPUS.get()) > 0);
    }
}

/// `SMR_ASSERT_NONCRITICAL()` (`diagnostic`): outside every read section (nothing after a
/// panic or in ddb).
pub fn assert_noncritical() {
    if !cpu::past_caring() {
        kassert!(sections(CPUS.get()) == 0);
    }
}

/// `smr_dispatch(spc)`: this CPU's calls moved onto the system's queue; the SMR thread woken
/// when the work is expedited or the queue was empty (the C arms a 100 ms timeout for the
/// latter; the thread's own pause keeps that delay).
fn dispatch(c: &SmrCpu) {
    let mut g = DEFERRED.lock();
    let wake = g.n == 0;
    g.links.concat(&c.deferred);
    g.n = g.n.wrapping_add(c.ndeferred.swap(0, Ordering::Relaxed));
    g.expedite |= c.expedite.swap(0, Ordering::Relaxed) != 0;
    let expedite = g.expedite;
    drop(g);
    if expedite || wake {
        sleep::wakeup(&raw const WAIT);
    }
}

/// `smr_idle()`: this CPU is in a quiescent state (the scheduler's switch and idle loop): its
/// queued calls dispatched, its view of the grace period brought up to date once every read
/// before it is done (`membar_exit`). `panic(9)` inside a read section (S3), except after a
/// panic or in ddb, where the view is left as it is.
pub fn idle() {
    let c = CPUS.get();
    if sections(c) != 0 {
        if cpu::past_caring() {
            return;
        }
        hal_panic(format_args!("smr_idle: inside a read section"));
    }
    if c.ndeferred.load(Ordering::Relaxed) > 0 {
        dispatch(c);
    }
    let gp = GRACE_PERIOD.load(Ordering::Acquire);
    if c.gp.load(Ordering::Relaxed) != gp {
        fence(Ordering::Release); // membar_exit()
        c.gp.store(gp, Ordering::Relaxed);
    }
}

/// `smr_call_impl(smr, func, arg, expedite)` of a claimed link: queued on this CPU at
/// `splhigh`, dispatched at once when the CPU is idle (an interrupt that preempted the idle
/// loop: the link would wait for the next switch otherwise).
fn call(l: NonNull<SmrLink>, expedite: bool) {
    let c = CPUS.get();
    let s = splhigh();
    c.deferred.append(l);
    c.ndeferred.fetch_add(1, Ordering::Relaxed);
    if expedite {
        c.expedite.store(1, Ordering::Relaxed);
    }
    drop(s);
    if cpu_is_idle(c) {
        dispatch(c);
    }
}

/// `smr_call(entry, func, arg)`: `func(arg)` once every CPU has left the read sections open
/// now. `panic(9)` when `entry` is queued already.
pub fn defer_link(entry: &'static SmrLink, func: fn(*mut c_void), arg: *mut c_void) {
    entry.claim(func, arg);
    call(NonNull::from(entry), false);
}

/// The release of a retired item: the box given up to the queue by [`defer_box`] put back.
fn release_box<T: SmrLinked>(arg: *mut c_void) {
    let Some(raw) = NonNull::new(arg.cast::<T>()) else {
        hal_panic(format_args!("smr: a release without its item"));
    };
    // SAFETY: `arg` is the item pointer of a box of `T::POOL` (checked at `store`, the one
    // way a box becomes an `SmrRetired`) that `defer_box`
    // gave up (`PoolBox::into_raw`), queued with this function as
    // its link's only call, and the link is run once (`Batch::run` marks it before the call,
    // and a queued link cannot be claimed again), so this rebuilds the item's one handle
    // exactly once; no reference made while it was published survives: the readers' ended
    // before the grace period that precedes this run (S2, S3), and the writers' borrowed the
    // `SmrPtr`'s cell, which no longer holds it.
    drop(unsafe { PoolBox::from_raw(raw, T::POOL) });
}

/// `panic(9)` unless `b` is an item of `T::POOL` (what its release puts it back into).
fn check_pool<T: SmrLinked>(b: &PoolBox<T>) {
    if !ptr::eq(PoolBox::pool(b), T::POOL) {
        hal_panic(format_args!("smr: an item of another pool"));
    }
}

/// `smr_call` of a retired item's link, the box given up to the queue, its release putting
/// it back. The link is asked for once and checked to lie within the item, which nothing
/// frees before that release; the item's pool was checked when it was published.
fn defer_box<T: SmrLinked>(b: PoolBox<T>) {
    let item = PoolBox::as_ptr(&b);
    let link = b.smr_link();
    let at = ptr::from_ref(link).addr();
    if at < item.addr() || at + size_of::<SmrLink>() > item.addr() + size_of::<T>() {
        hal_panic(format_args!("smr: an item's link outside the item"));
    }
    link.claim(release_box::<T>, item.cast_mut().cast());
    let l = NonNull::from(link);
    let _ = PoolBox::into_raw(b);
    call(l, false);
}

/// `smr_grace_wait()`: the next grace period announced, and every running CPU whose view is
/// behind made to pass [`idle`] (the calling thread pegged to it), so every read section
/// open at the announcement has ended when it returns. One caller at a time ([`GRACE`]),
/// never inside a read section of its own CPU, whose view it sets; the CPU walk's contract
/// (`sys/hal/trust.rs`) checked where `hal` can see it: a skipped CPU inside a section, or a
/// peg that leaves the view behind, is `panic(9)`.
fn grace_wait(cpus: &dyn CpuWalk) {
    let _one = GRACE.write();
    let me = CPUS.get();
    if sections(me) != 0 {
        hal_panic(format_args!("smr_grace_wait: inside a read section"));
    }
    let gp = GRACE_PERIOD.load(Ordering::Relaxed).wrapping_add(1);
    GRACE_PERIOD.store(gp, Ordering::Release);
    me.gp.store(gp, Ordering::Relaxed);
    for n in 0..cpu::ncpus() {
        let Some(c) = CPUS.cpu(n) else { continue };
        if !cpus.running(n) {
            if sections(c) != 0 {
                hal_panic(format_args!(
                    "smr_grace_wait: cpu{n} said not running is inside a read section"
                ));
            }
            continue;
        }
        if c.gp.load(Ordering::Acquire) == gp {
            continue;
        }
        cpus.peg(n);
        if c.gp.load(Ordering::Acquire) != gp {
            hal_panic(format_args!(
                "smr_grace_wait: cpu{n} pegged but did not pass a quiescent state"
            ));
        }
    }
    cpus.unpeg();
}

/// The SMR thread's wait (`smr_thread`'s sleeps on `smr_ndeferred` under `smr_lock`):
/// "bored" until a call is dispatched, then a pause of `SMR_PAUSE` unless the work is
/// expedited (a dispatch to an empty queue wakes the thread at once, so this pause is the
/// C's timeout).
pub fn wait() {
    let mut g = DEFERRED.lock();
    while g.n == 0 {
        let _ = g.sleep_nsec(&raw const WAIT, PVM, "bored", INFSLP);
    }
    if !g.expedite {
        let _ = g.sleep_nsec(&raw const WAIT, PVM, "pause", SMR_PAUSE_NSEC);
    }
}

/// `SIMPLEQ_CONCAT(&deferred, &smr_deferred)`: the system's queue taken whole for the SMR
/// thread, its count and expedite flag cleared.
pub fn collect() -> Batch {
    let mut g = DEFERRED.lock();
    let head = g.links.head.swap(ptr::null_mut(), Ordering::Relaxed);
    g.links.tail.store(ptr::null_mut(), Ordering::Relaxed);
    g.n = 0;
    g.expedite = false;
    Batch { head }
}

/// `smr_barrier_impl(expedite)`: waits until every read section open now has ended (and the
/// calls this CPU queued before ran); `smr_flush()` with `expedite`. Nothing after a panic
/// or in ddb.
pub fn barrier(expedite: bool) {
    if cpu::past_caring() {
        return;
    }
    let link = SmrLink::new();
    let chan = ptr::from_ref(&link);
    link.claim(barrier_wake, chan.cast_mut().cast());
    call(NonNull::from(&link), expedite);
    // This CPU's queue dispatched now, which the C leaves to the switch of its sleep below
    // (`smr_idle`): the same moment, and no wait for a switch that a CPU with nothing else
    // to run may not make soon.
    dispatch(CPUS.get());
    // `cond_wait(&c, "smrbar")`: the condition is the link's run mark, which the SMR thread
    // stores before calling `barrier_wake`, its last use of the link.
    loop {
        sleep::sleep_setup(chan, PVM, "smrbar");
        let wait = link.next.load(Ordering::Acquire) != RAN;
        let _ = sleep::sleep_finish(INFSLP, wait);
        if !wait {
            break;
        }
    }
}

/// The barrier's call (`cond_signal_handler`): its argument is the waiting frame's link, an
/// address compared, never followed.
fn barrier_wake(arg: *mut c_void) {
    sleep::wakeup_one(arg.cast_const());
}

/// Forgets every queued call and the per-CPU counts (never runs or frees them), for the host
/// tests that load fresh kernel memory: links of an earlier test's memory are dropped from
/// the lists, never followed again (the read-section depths and the views are left).
#[cfg(not(target_os = "none"))]
pub fn smr_test_reset() {
    let mut g = DEFERRED.lock();
    g.links = Chain::new();
    g.n = 0;
    g.expedite = false;
    drop(g);
    for n in 0..cpu::MAXCPUS {
        if let Some(c) = CPUS.cpu(n) {
            let s = splhigh();
            c.deferred.head.store(ptr::null_mut(), Ordering::Relaxed);
            c.deferred.tail.store(ptr::null_mut(), Ordering::Relaxed);
            c.ndeferred.store(0, Ordering::Relaxed);
            c.expedite.store(0, Ordering::Relaxed);
            drop(s);
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use std::thread;
    use std::vec::Vec;

    use super::*;
    use crate::pool::PR_WAITOK;
    use crate::testutil;

    /// The SMR state is the system's: its tests run one at a time, each on fresh queues.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn serial() -> MutexGuard<'static, ()> {
        testutil::install();
        let g = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
        smr_test_reset();
        g
    }

    /// No CPU but the caller's.
    struct Alone;

    impl CpuWalk for Alone {
        fn running(&self, _n: usize) -> bool {
            false
        }
        fn peg(&self, n: usize) {
            panic!("peg({n}) with no other CPU");
        }
        fn unpeg(&self) {}
    }

    static CALLS: AtomicU64 = AtomicU64::new(0);

    fn count(_arg: *mut c_void) {
        CALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn leaked_link() -> &'static SmrLink {
        Box::leak(Box::new(SmrLink::new()))
    }

    #[test]
    fn a_deferred_call_runs_once_after_a_grace_period() {
        let _s = serial();
        let before = CALLS.load(Ordering::SeqCst);
        let l = leaked_link();
        defer_link(l, count, ptr::null_mut());
        assert_eq!(collect().run(&Alone), 0, "still on this CPU's queue");
        idle(); // this CPU's quiescent state dispatches it
        assert_eq!(collect().run(&Alone), 1);
        assert_eq!(CALLS.load(Ordering::SeqCst), before + 1);
        assert_eq!(collect().run(&Alone), 0, "run once");
        // A run entry is queued again after smr_init, as in the C.
        l.init();
        defer_link(l, count, ptr::null_mut());
        idle();
        assert_eq!(collect().run(&Alone), 1);
        assert_eq!(CALLS.load(Ordering::SeqCst), before + 2);
    }

    #[test]
    fn a_queued_link_is_queued_once_and_kept() {
        let _s = serial();
        let l = leaked_link();
        defer_link(l, count, ptr::null_mut());
        let again = core::panic::AssertUnwindSafe(|| defer_link(l, count, ptr::null_mut()));
        assert!(std::panic::catch_unwind(again).is_err());
        assert!(
            std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| l.init())).is_err(),
            "smr_init of a queued entry"
        );
        smr_test_reset();
    }

    #[test]
    fn idle_inside_a_read_section_panics_and_leaves_the_view() {
        let _s = serial();
        let me = CPUS.get();
        let gp = me.gp.load(Ordering::SeqCst);
        GRACE_PERIOD.store(gp.wrapping_add(1), Ordering::SeqCst);
        let g = read();
        read_enter(); // nested, as an interrupt's
        read_leave();
        assert!(std::panic::catch_unwind(idle).is_err());
        assert_eq!(
            me.gp.load(Ordering::SeqCst),
            gp,
            "no quiescent state under a section"
        );
        // A C-style leave cannot end the guard's section (two counts).
        assert!(
            std::panic::catch_unwind(read_leave).is_err(),
            "no C-style section"
        );
        assert!(
            std::panic::catch_unwind(idle).is_err(),
            "the guard's section still open"
        );
        drop(g);
        read_enter();
        assert!(
            std::panic::catch_unwind(idle).is_err(),
            "a C-style section open"
        );
        read_leave();
        idle();
        assert_eq!(me.gp.load(Ordering::SeqCst), gp.wrapping_add(1));
    }

    /// A walk whose `peg` does nothing (a broken contract `hal` can see).
    struct NoPeg(usize);

    impl CpuWalk for NoPeg {
        fn running(&self, n: usize) -> bool {
            n == self.0
        }
        fn peg(&self, _n: usize) {}
        fn unpeg(&self) {}
    }

    /// A walk that calls every CPU not running (the contract broken where `hal` checks it).
    struct NoneRunning;

    impl CpuWalk for NoneRunning {
        fn running(&self, _n: usize) -> bool {
            false
        }
        fn peg(&self, _n: usize) {}
        fn unpeg(&self) {}
    }

    #[test]
    fn a_grace_period_is_never_cut_short() {
        let _s = serial();
        // A run inside a read section of its own CPU.
        let g = read();
        assert!(std::panic::catch_unwind(|| collect().run(&Alone)).is_err());
        drop(g);
        // A CPU in a read section that the walk says is not running, and a peg that does
        // not make a CPU pass a quiescent state: both refused, nothing run.
        let (tx, rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        thread::scope(|s| {
            s.spawn(move || {
                let g = read();
                tx.send(cpu::cpu_number()).expect("sent");
                let _ = done_rx.recv();
                drop(g);
            });
            let n = rx.recv().expect("the reader's CPU");
            assert!(std::panic::catch_unwind(|| collect().run(&NoneRunning)).is_err());
            assert!(std::panic::catch_unwind(|| collect().run(&NoPeg(n))).is_err());
            done_tx.send(()).expect("sent");
        });
    }

    #[test]
    fn a_barrier_returns_once_the_calls_before_it_ran() {
        let _s = serial();
        let stop = AtomicBool::new(false);
        thread::scope(|s| {
            // The SMR thread.
            s.spawn(|| {
                loop {
                    wait();
                    let _ = collect().run(&Alone);
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                }
            });
            let before = CALLS.load(Ordering::SeqCst);
            defer_link(leaked_link(), count, ptr::null_mut());
            barrier(false); // smr_barrier(): after the thread's pause
            assert_eq!(CALLS.load(Ordering::SeqCst), before + 1);
            defer_link(leaked_link(), count, ptr::null_mut());
            stop.store(true, Ordering::SeqCst);
            let t = std::time::Instant::now();
            barrier(true); // smr_flush(): no pause; the thread's last round
            assert!(
                t.elapsed() < std::time::Duration::from_millis(80),
                "a flush skips SMR_PAUSE (100 ms)"
            );
            assert_eq!(CALLS.load(Ordering::SeqCst), before + 2);
        });
    }

    /// A published object: its release counted.
    struct Obj {
        link: SmrLink,
        value: u64,
        freed: AtomicBool,
    }

    impl Drop for Obj {
        fn drop(&mut self) {
            self.freed.store(true, Ordering::SeqCst);
            FREED.fetch_add(1, Ordering::SeqCst);
        }
    }

    static OBJS: BoxPool<Obj> = BoxPool::new();
    static FREED: AtomicUsize = AtomicUsize::new(0);

    impl SmrLinked for Obj {
        const POOL: &'static BoxPool<Obj> = &OBJS;
        fn smr_link(&self) -> &SmrLink {
            &self.link
        }
    }

    fn obj(value: u64) -> PoolBox<Obj> {
        let _ = OBJS.init(0, PR_WAITOK, "smrtest");
        OBJS.get(PR_WAITOK, || Obj {
            link: SmrLink::new(),
            value,
            freed: AtomicBool::new(false),
        })
        .expect("an item")
    }

    static WRITERS: RawRwlock = RawRwlock::new("smrptr");
    static OTHER: RawRwlock = RawRwlock::new("other");

    #[test]
    fn the_writer_loads_and_stores_under_its_lock() {
        let _s = serial();
        let p: SmrPtr<Obj> = SmrPtr::new_keyed(&WRITERS);
        let mut w = WRITERS.write();
        assert!(p.store(&mut w, Some(obj(1))).is_none());
        assert_eq!(p.load_locked(&w).map(|o| o.value), Some(1));
        let freed = FREED.load(Ordering::SeqCst);
        let old = p
            .store(&mut w, Some(obj(2)))
            .expect("the first object back");
        drop(w);
        let g = read();
        assert_eq!(p.load(&g).map(|o| o.value), Some(2));
        drop(g);
        old.defer();
        assert_eq!(
            FREED.load(Ordering::SeqCst),
            freed,
            "not before a grace period"
        );
        let o = OTHER.write();
        assert!(
            std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| p
                .load_locked(&o)
                .is_some()))
            .is_err(),
            "a witness of another lock"
        );
        drop(o);
        drop(p); // the last object retired too
        idle();
        assert_eq!(collect().run(&Alone), 2);
        assert_eq!(FREED.load(Ordering::SeqCst), freed + 2);
    }

    /// The readers' CPUs of the stress test: `peg(n)` asks reader `n` to pass a quiescent
    /// state and waits until it has.
    struct Readers {
        cpus: Vec<AtomicUsize>,
        asked: Vec<AtomicBool>,
    }

    impl CpuWalk for Readers {
        fn running(&self, n: usize) -> bool {
            self.cpus.iter().any(|c| c.load(Ordering::SeqCst) == n)
        }
        fn peg(&self, n: usize) {
            let Some(i) = self.cpus.iter().position(|c| c.load(Ordering::SeqCst) == n) else {
                panic!("peg({n}) of no reader");
            };
            self.asked[i].store(true, Ordering::SeqCst);
            while self.asked[i].load(Ordering::SeqCst) {
                thread::yield_now();
            }
        }
        fn unpeg(&self) {}
    }

    #[test]
    fn a_reader_never_sees_a_released_object() {
        const READERS: usize = 4;
        const ROUNDS: u64 = 200;
        let _s = serial();
        static P: SmrPtr<Obj> = SmrPtr::new_static(&WRITERS);
        let walk = Readers {
            cpus: (0..READERS).map(|_| AtomicUsize::new(usize::MAX)).collect(),
            asked: (0..READERS).map(|_| AtomicBool::new(false)).collect(),
        };
        let started = AtomicUsize::new(0);
        let stop = AtomicBool::new(false);
        let seen = AtomicU64::new(0);
        let freed = FREED.load(Ordering::SeqCst);
        let mut released = 0;
        thread::scope(|s| {
            for i in 0..READERS {
                let (walk, started, stop, seen) = (&walk, &started, &stop, &seen);
                s.spawn(move || {
                    walk.cpus[i].store(cpu::cpu_number(), Ordering::SeqCst);
                    started.fetch_add(1, Ordering::SeqCst);
                    while !stop.load(Ordering::SeqCst) {
                        let g = read();
                        if let Some(o) = P.load(&g) {
                            for _ in 0..64 {
                                core::hint::spin_loop();
                            }
                            assert!(!o.freed.load(Ordering::SeqCst), "a released object");
                            seen.fetch_add(1, Ordering::Relaxed);
                        }
                        drop(g);
                        if walk.asked[i].load(Ordering::SeqCst) {
                            idle();
                            walk.asked[i].store(false, Ordering::SeqCst);
                        }
                    }
                });
            }
            while started.load(Ordering::SeqCst) < READERS {
                thread::yield_now();
            }
            let me = cpu::cpu_number();
            let mut cpus: Vec<usize> = walk.cpus.iter().map(|c| c.load(Ordering::SeqCst)).collect();
            cpus.push(me);
            cpus.sort_unstable();
            cpus.dedup();
            assert_eq!(cpus.len(), READERS + 1, "every thread a CPU of its own");
            for r in 0..ROUNDS {
                let mut w = WRITERS.write();
                let old = P.store(&mut w, Some(obj(r)));
                drop(w);
                drop(old); // deferred
                idle();
                released += collect().run(&walk);
            }
            stop.store(true, Ordering::SeqCst);
        });
        let mut w = WRITERS.write();
        let last = P.take(&mut w);
        drop(w);
        drop(last);
        idle();
        released += collect().run(&Alone);
        assert_eq!(released, ROUNDS, "each object released once");
        assert_eq!(FREED.load(Ordering::SeqCst), freed + ROUNDS as usize);
        assert!(seen.load(Ordering::SeqCst) > 0, "the readers saw objects");
    }

    #[test]
    fn an_item_of_another_pool_or_with_a_foreign_link_is_refused() {
        let _s = serial();
        static ELSEWHERE: SmrLink = SmrLink::new();
        struct Stray {
            _v: u64,
        }
        static STRAYS: BoxPool<Stray> = BoxPool::new();
        impl SmrLinked for Stray {
            const POOL: &'static BoxPool<Stray> = &STRAYS;
            fn smr_link(&self) -> &SmrLink {
                &ELSEWHERE
            }
        }
        let _ = STRAYS.init(0, PR_WAITOK, "smrstray");
        let p: SmrPtr<Stray> = SmrPtr::new_keyed(&WRITERS);
        let b = STRAYS.get(PR_WAITOK, || Stray { _v: 0 }).expect("an item");
        let mut w = WRITERS.write();
        // Published (its pool is the type's); its deferral refuses the link.
        assert!(p.store(&mut w, Some(b)).is_none());
        let r = p.take(&mut w).expect("the item back");
        drop(w);
        assert!(
            std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| r.defer())).is_err(),
            "a link outside the item"
        );
        // An item of another pool than the type's is refused at its publication.
        static ELSEWHERE_OBJS: BoxPool<Obj> = BoxPool::new();
        let _ = ELSEWHERE_OBJS.init(0, PR_WAITOK, "smrelse");
        let q: SmrPtr<Obj> = SmrPtr::new_keyed(&WRITERS);
        let o = ELSEWHERE_OBJS
            .get(PR_WAITOK, || Obj {
                link: SmrLink::new(),
                value: 0,
                freed: AtomicBool::new(false),
            })
            .expect("an item");
        let mut w = WRITERS.write();
        let r = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
            q.store(&mut w, Some(o)).is_some()
        }));
        assert!(r.is_err(), "an item of another pool");
        drop(w);
    }
}
/* </TESTS> */
