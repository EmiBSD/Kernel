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
 * Copyright (c) 1998, 1999 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Jason R. Thorpe of the Numerical Aerospace Simulation Facility,
 * NASA Ames Research Center.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! Kernel threads, as the kernel crate makes them: `kthread_create(9)`,
//! `kthread_create_deferred` and its queue (`kthread_run_deferred_queue`), `kthread_exit`,
//! and pegging a thread to the primary CPU.
//!
//! Upstream: sys/kern/kern_kthread.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_kthread.rs@3c62ede68802
//!
//! Making a thread is [`kthread_create`] over `kernel::fork::fork1` (N4.6a); the kernel's own
//! subsystems (timeout(9), task(9)), which name no glue, reach it through the operations
//! `kernel::fork::init` registers (`Ops`), as the host tests' doubles do. The deferred
//! creations are this module's own since N4.5a: the kernel's ([`Start`]s) and `bsd`'s (a function and its
//! argument, [`kthread_create_deferred`]) wait in one queue, in order, until
//! [`kthread_run_deferred_queue`].
//!
//! **The start ticket.** The C hands a new thread a function and a `void *` (`softclock_thread`
//! and `NULL`, `taskq_thread` and the queue), which the thread casts back. Here no pointer
//! crosses: the kernel files what the new thread is to run as a [`Start`] in [`STARTS`] and
//! hands `fork1` the slot's number plus one, a ticket (`kernel::fork::Start::Kernel`); the
//! new thread's start calls [`run`] with it, which takes the `Start` back out, in safe code: what a ticket names is a
//! value of this crate's own types (a queue's counted handle), never a re-typed address. A
//! ticket is never 0: `bsd` carries it as the thread's `void *`, and `fork1` hands a thread
//! whose argument is `NULL` its own `p` instead (`kern_fork.c:437`, `arg ? arg : p`;
//! `kern_fork.rs:581`), so a ticket 0 would come back as the new thread's address (the first
//! boot of N4.1 panicked so, "ticket 0xffffff80184dd4d0 names no thread"). A ticket that cannot
//! be filed (no memory to grow the table) is the C's failure of the allocation behind it:
//! `kthread_create`'s `ENOMEM`.
//!
//! ## Deviations
//! - A deferred creation that cannot be queued (no memory to grow the queue) panics "unable to
//!   allocate kthread_q", as the C's `malloc(M_NOWAIT)` failure does.
//! - The queue's storage is the global allocator's, itself `malloc(M_TEMP, M_NOWAIT)`
//!   (`bsd`'s `rust_alloc`), so `vmstat -m`'s `temp` row counts the queue's buffer (allocated
//!   as it grows, given back once the queue ran) where the C counts one `M_TEMP` block per
//!   entry: the same row, fewer calls, nothing left in use after boot.
//!
//! ## Redesign
//! - `kthread_create(func, arg, NULL, name)` with `arg` cast back by `func` -> [`create`] with
//!   a [`Start`]: the argument is typed, owned by the ticket until the thread takes it.
//! - LZ's `kthread_q` was a `SIMPLEQ` of `malloc`'d `struct kthread_q` entries behind an
//!   `unsafe impl Sync` (filled and run by the boot CPU alone). The queue is a
//!   `VecDeque<Deferred>` inside a `Mutexed` ([`DEFERRED`]): an entry is a kernel [`Start`] or
//!   `bsd`'s function with its argument's exposed address (`Deferred::Legacy`), called with
//!   the pointer remade from it; the mutex is left before an entry runs, since a creation
//!   forks and may sleep.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use hal::intr::IPL_NONE;
use hal::mutex::Mutexed;
use hal::once::OnceStatic;
use hal::sys::errno::Errno;

use crate::assert::kpanic;

/// What a kernel thread made through this module runs, or what a deferred creation calls:
/// filed under a ticket until the thread takes it (a deferred one waits in [`DEFERRED`]). The
/// variants are the kernel's subsystems that make threads (timeout(9), task(9)).
pub(crate) enum Start {
    /// timeout(9)'s `softclock_create_thread` (deferred by `timeout_proc_init`).
    SoftclockCreate,
    /// timeout(9)'s `softclock_thread`.
    Softclock,
    /// timeout(9)'s `softclock_thread_mp`.
    SoftclockMp,
    /// task(9)'s `taskq_create_thread(tq)` (deferred by `taskq_init`, `taskq_create`).
    TaskqCreate(crate::task::TqHandle),
    /// task(9)'s `taskq_thread(tq)`, a worker.
    TaskqWorker(crate::task::TqHandle),
}

/// `struct kthread_q`: a creation put off until threads can be made.
enum Deferred {
    /// One of the kernel's.
    Kernel(Start),
    /// One of `bsd`'s: `kq_func`, and `kq_arg`'s exposed address.
    Legacy(fn(*mut c_void), usize),
}

/// The starts filed and not yet taken, by ticket; a taken slot is `None` and reused.
static STARTS: Mutexed<Vec<Option<Start>>> = Mutexed::new(IPL_NONE, Vec::new());

/// `kthread_q`: the deferred creations, in the order they were asked for.
static DEFERRED: Mutexed<VecDeque<Deferred>> = Mutexed::new(IPL_NONE, VecDeque::new());

/// `kthread_create_now`: set once the standard kernel threads exist (deferred creations run
/// at once from then on).
pub static KTHREAD_CREATE_NOW: AtomicBool = AtomicBool::new(false);

/// Files `s` under a ticket, the slot's index plus one (never 0: `fork1` replaces a `NULL`
/// argument by the new thread's `p`, the module docs): a free slot, or a new one when the
/// table can grow; `s` back when it cannot.
fn ticket(s: Start) -> Result<usize, Start> {
    let mut v = STARTS.lock();
    if let Some(i) = v.iter().position(Option::is_none) {
        v[i] = Some(s);
        return Ok(i + 1);
    }
    if v.try_reserve(1).is_err() {
        return Err(s);
    }
    v.push(Some(s));
    Ok(v.len())
}

/// The start filed under `t`, taken out (`None` for a ticket not filed or taken already,
/// and for 0, which names no slot).
fn take(t: usize) -> Option<Start> {
    let i = t.checked_sub(1)?;
    STARTS.lock().get_mut(i).and_then(Option::take)
}

/// `kthread_create(9)`: a kernel thread named `name` that runs `s`. `ENOMEM` when the ticket
/// cannot be filed; `fork1`'s error otherwise (the start is dropped then).
pub(crate) fn create(name: &'static [u8], s: Start) -> Result<(), Errno> {
    let t = ticket(s).map_err(|_| Errno::ENOMEM)?;
    (ops().create)(name, t).inspect_err(|_| drop(take(t)))
}

/// `kthread_create_deferred`'s queueing: `d` waits in [`DEFERRED`], or panics as the C's
/// allocation of its entry does.
fn defer(d: Deferred) {
    let mut q = DEFERRED.lock();
    if q.try_reserve(1).is_err() {
        drop(q);
        kpanic(format_args!("unable to allocate kthread_q"));
    }
    q.push_back(d);
}

/// `kthread_create_deferred(9)` of the kernel's: `s` runs once threads can be made, or at
/// once after that.
pub(crate) fn create_deferred(s: Start) {
    if KTHREAD_CREATE_NOW.load(Ordering::Relaxed) {
        run_start_deferred(s);
        return;
    }
    defer(Deferred::Kernel(s));
}

/// `kthread_create_deferred(func, arg)`: defer the creation of a kernel thread. Once the
/// standard kernel threads and processes have been created, this queue will be run to
/// callback to the caller to create threads for e.g. file systems and device drivers.
pub fn kthread_create_deferred(func: fn(*mut c_void), arg: *mut c_void) {
    if KTHREAD_CREATE_NOW.load(Ordering::Relaxed) {
        func(arg);
        return;
    }
    defer(Deferred::Legacy(func, arg.expose_provenance()));
}

/// `kthread_run_deferred_queue()`: runs the deferred creations, in order; each one runs with
/// the queue's mutex left (it forks, and may sleep).
pub fn kthread_run_deferred_queue() {
    // No longer need to defer kthread creation.
    KTHREAD_CREATE_NOW.store(true, Ordering::Relaxed);
    drain();
    // The queue's buffer goes back too (the C freed each entry).
    *DEFERRED.lock() = VecDeque::new();
}

/// The deferred creations run, first to last, until the queue is empty.
fn drain() {
    loop {
        let next = DEFERRED.lock().pop_front();
        match next {
            None => break,
            Some(Deferred::Kernel(s)) => run_start_deferred(s),
            Some(Deferred::Legacy(func, arg)) => func(ptr::with_exposed_provenance_mut(arg)),
        }
    }
}

/// What a kernel thread made by [`create`] runs: called by `bsd`'s thread function with the
/// ticket it was given. Never returns (each start ends in [`exit`] or runs for ever).
pub fn run(ticket: usize) -> ! {
    match take(ticket) {
        Some(Start::Softclock) => crate::timeout::softclock_thread(),
        Some(Start::SoftclockMp) => crate::timeout::softclock_thread_mp(),
        Some(Start::TaskqWorker(tq)) => crate::task::taskq_thread(tq),
        Some(Start::SoftclockCreate | Start::TaskqCreate(_)) | None => {
            kpanic(format_args!("kthread: ticket {ticket} names no thread"))
        }
    }
}

/// What a deferred creation of the kernel's does when its turn comes.
fn run_start_deferred(s: Start) {
    match s {
        Start::SoftclockCreate => crate::timeout::softclock_create_thread(),
        Start::TaskqCreate(tq) => crate::task::taskq_create_thread(tq),
        Start::Softclock | Start::SoftclockMp | Start::TaskqWorker(_) => {
            kpanic(format_args!("kthread: a thread start deferred as a call"))
        }
    }
}

/// `kthread_exit(9)`.
pub(crate) fn exit(ecode: i32) -> ! {
    (ops().exit)(ecode)
}

/// `sched_peg_curproc` of the primary CPU.
pub(crate) fn peg_to_primary_cpu() {
    (ops().peg)();
}

/// The registered [`Ops`]: making a thread before `kernel::fork::init` is a boot-order bug.
fn ops() -> &'static Ops {
    match OPS.get() {
        Some(o) => o,
        None => kpanic(format_args!("kthread: no thread operations registered")),
    }
}

/// The host tests' thread operations (`kernel::testutil`'s host threads), installed in
/// place of a glue's; a second registration keeps the first.
#[cfg(any(test, feature = "doubles"))]
pub(crate) fn install_test_ops(ops: Ops) {
    let _ = OPS.set(ops);
}

/// What the kernel's own subsystems (timeout(9), task(9)), which name no glue, make and end
/// their threads with: `kernel::fork::init`'s registration of this glue's
/// [`kthread_create`], [`kthread_exit`] and the peg (`kernel::fork` and `kernel::exit` are
/// generic over the glue; these callers are not).
pub(crate) struct Ops {
    /// `kthread_create(func, ticket, NULL, name)` of a ticket's start.
    pub(crate) create: fn(&'static [u8], usize) -> Result<(), Errno>,
    /// `kthread_exit(ecode)`.
    pub(crate) exit: fn(i32) -> !,
    /// `sched_peg_curproc` of the primary CPU.
    pub(crate) peg: fn(),
}

/// The registered [`Ops`] (once, by `kernel::fork::init`).
static OPS: OnceStatic<Ops> = OnceStatic::new();

/// Registers this glue's thread operations for the kernel's own subsystems; a second
/// registration keeps the first.
pub(crate) fn install_ops<G: crate::proc::ProcGlue>() {
    let _ = OPS.set(Ops {
        create: create_ticket::<G>,
        exit: kthread_exit::<G>,
        peg: peg_primary::<G>,
    });
}

/// [`Ops::create`]: a kernel thread whose start is the ticket `t` (`kernel::fork::Start::Kernel`).
fn create_ticket<G: crate::proc::ProcGlue>(name: &'static [u8], t: usize) -> Result<(), Errno> {
    kthread_create::<G>(name, crate::fork::Start::Kernel(t)).map(drop)
}

/// [`Ops::peg`]: `softclock_thread`'s "be conservative for the moment": the running thread
/// pegged to the primary CPU.
fn peg_primary<G: crate::proc::ProcGlue>() {
    let t = G::sched();
    let primary = (0..hal::cpu::MAXCPUS).find(|&n| t.all_cpus.isset(n) && G::cpu_is_primary(n));
    let Some(n) = primary else {
        kpanic(format_args!("softclock_thread: no primary CPU"));
    };
    crate::sched::sched_peg_curproc::<G>(n);
}

/// `kthread_create(9)`: a kernel thread named `name` that runs `start`, forked from process
/// 0 (sharing its address space and descriptors, leaving no zombie); its tid (the C's
/// `*newpp`, which no caller follows: the four that kept it store the tid or compare it).
pub fn kthread_create<G: crate::proc::ProcGlue>(
    name: &[u8],
    start: crate::fork::Start,
) -> Result<hal::sys::types::Pid, Errno> {
    kthread_create_with::<G>(name, start, &mut |_| {})
}

/// [`kthread_create`] that lends the new thread to `child` once it is linked and before it
/// can run (`bsd`'s `kthread_create`, whose callers keep the thread, the C's `*newpp`).
pub fn kthread_create_with<G: crate::proc::ProcGlue>(
    name: &[u8],
    start: crate::fork::Start,
    child: &mut dyn FnMut(core::pin::Pin<&crate::proc::Thread<G>>),
) -> Result<hal::sys::types::Pid, Errno> {
    use crate::proc::{FORK_NOZOMBIE, FORK_SHAREFILES, FORK_SHAREVM, FORK_SYSTEM};

    hal::klock::kernel_lock(); // KERNEL_LOCK()

    // First, create the new process. Share the memory, file descriptors and don't leave the
    // exit status around for the parent to wait for.
    let child = match crate::fork::fork1_with(
        G::tables().proc0(),
        FORK_SHAREVM | FORK_SHAREFILES | FORK_NOZOMBIE | FORK_SYSTEM,
        start,
        child,
    ) {
        Ok(c) => c,
        Err(error) => {
            hal::klock::kernel_unlock(); // KERNEL_UNLOCK()
            return Err(error);
        }
    };

    // Name it as specified.
    child.process.set_comm(name);

    hal::klock::kernel_unlock(); // KERNEL_UNLOCK()

    // All done!
    Ok(child.tid)
}

/// `kthread_exit(9)`: the running kernel thread exits (a non-zero code is printed: its
/// parent, process 0, does nothing with it).
pub fn kthread_exit<G: crate::proc::ProcGlue>(ecode: i32) -> ! {
    hal::thread::with_current::<crate::proc::Thread<G>, _>(|p| {
        if ecode != 0 {
            let comm = p.process().comm();
            hal::prf::printf(format_args!(
                "WARNING: thread `{}' ({}) exits with status {}\n",
                Bytes(&comm),
                p.p_tid.get(),
                ecode
            ));
        }
        G::exit1(&p, ecode, 0, crate::proc::EXIT_NORMAL)
    })
}

/// A command name printed as its bytes (`%s`).
struct Bytes<'a>(&'a [u8]);

impl core::fmt::Display for Bytes<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for &b in self.0 {
            core::fmt::Write::write_char(f, char::from(b))?;
        }
        Ok(())
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicUsize;

    use super::*;
    use crate::testutil;

    /// The word that crosses to `bsd` is never 0 (`fork1` would hand the thread its own `p`
    /// for it), 0 names nothing, and a ticket round-trips once.
    #[test]
    fn a_ticket_is_never_zero_and_round_trips_once() {
        testutil::install();
        let t = ticket(Start::Softclock).ok().unwrap();
        assert_ne!(t, 0);
        assert!(take(0).is_none());
        assert!(matches!(take(t), Some(Start::Softclock)));
        assert!(take(t).is_none());
        // The freed slot is reused, still never as 0.
        let again = ticket(Start::SoftclockMp).ok().unwrap();
        assert_eq!(again, t);
        assert!(matches!(take(again), Some(Start::SoftclockMp)));
    }

    /// The order the deferred calls ran in, as the arguments' first bytes.
    static ORDER: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
    /// How many deferred calls ran.
    static RAN: AtomicUsize = AtomicUsize::new(0);

    /// A deferred call: records the byte its argument points at.
    fn record(arg: *mut c_void) {
        let at = RAN.fetch_add(1, Ordering::Relaxed);
        let v = BYTES
            .iter()
            .position(|b| ptr::eq(b, arg.cast::<u8>()))
            .unwrap_or(usize::MAX);
        ORDER[at].store(v, Ordering::Relaxed);
    }

    /// The arguments the deferred calls point at.
    static BYTES: [u8; 3] = [10, 20, 30];

    /// `bsd`'s deferred creations wait in order and are called with their argument, the
    /// pointer as it was given; the queue is empty after the drain.
    #[test]
    fn deferred_creations_run_in_order_with_their_argument() {
        // The tests' install has run the queue already, so nothing else is deferred now.
        testutil::install();
        for b in [2, 0, 1] {
            defer(Deferred::Legacy(
                record,
                ptr::from_ref(&BYTES[b]).cast_mut().expose_provenance(),
            ));
        }
        drain();
        assert_eq!(RAN.load(Ordering::Relaxed), 3);
        let order: [usize; 3] = core::array::from_fn(|i| ORDER[i].load(Ordering::Relaxed));
        assert_eq!(order, [2, 0, 1]);
        assert!(DEFERRED.lock().is_empty());
    }

    /// Once the queue has run (as `kthread_run_deferred_queue` leaves it; the tests' install
    /// sets it), a deferred creation is called at once.
    #[test]
    fn after_the_queue_ran_a_deferred_creation_is_called_at_once() {
        static CALLED: AtomicUsize = AtomicUsize::new(0);
        fn bump(arg: *mut c_void) {
            CALLED.fetch_add(arg.addr(), Ordering::Relaxed);
        }
        testutil::install();
        assert!(KTHREAD_CREATE_NOW.load(Ordering::Relaxed));
        kthread_create_deferred(bump, ptr::without_provenance_mut(5));
        assert_eq!(CALLED.load(Ordering::Relaxed), 5);
    }
}
/* </TESTS> */
