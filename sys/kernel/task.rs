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
 * Copyright (c) 2013 David Gwynne <dlg@openbsd.org>
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
//! Task queues: `task_add(9)` and `taskq_create(9)`. A task queue owns one or more kernel
//! threads that run the tasks queued on it, in order, in process context. `systq` (under the
//! kernel lock) and `systqmp` (`TASKQ_MPSAFE`) exist from boot.
//!
//! Upstream: sys/kern/kern_task.c @ 3ce1f3f79392
//! Upstream: sys/sys/task.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_task.rs@3c62ede68802
//! LZ: sys/sys/task.rs@3c62ede68802
//!
//! A queue's mutex keeps its state, its worklist and the list of its threads. `task_add` and
//! `task_del` look at the task's `TASK_ONQUEUE` without it first, as the C does (possibly from
//! an interrupt handler), then again under it. A worker takes the first task off the list,
//! copies its function and argument ("to avoid races") and runs the copy with the mutex
//! released; a `TASKQ_MPSAFE` queue's workers drop the kernel lock their thread starts with.
//! A barrier (`taskq_barrier`) waits until every worker has finished what it was running: it
//! queues a barrier task for each worker that is not the caller, and the workers that run it
//! wait together for the last barrier out.
//!
//! Wait channels (all distinct, as the C's): the queue ("bored", and the destroyer's wakeup of
//! idle workers), `tq_running` ("tqdestroy"), `tq_bthreads` ("tqbar"), `tq_bgen`
//! ("tqbarend", "tqbarwait").
//!
//! ## Deviations
//! - `WITNESS` is not configured: the lock object, `WITNESS_INIT`/`CHECKORDER`/`LOCK`/`UNLOCK`
//!   are absent (LZ's).
//! - `kcov` is not configured: `t_process` is not kept, no `kcov_remote_enter`/`leave` (LZ's).
//! - A task without a function (one `task_set` never named) panics when a worker reaches it,
//!   where the C calls through a null pointer (LZ's).
//! - `task_del(tq, t)` of a task pending on another queue panics (the worklist refuses an
//!   element on a list of another lock, `hal::pinlist`'s I2), where the C's `TAILQ_REMOVE`
//!   unlinks it through the wrong head and corrupts both queues.
//!
//! ## Redesign
//! - `struct taskq` allocated by `malloc(M_DEVBUF)` and handed out as a pointer (LZ:
//!   `Option<&'static Taskq>` and an `unsafe fn taskq_destroy(NonNull<Taskq>)`) is a
//!   [`TaskqRef`], a counted handle of the pool `taskqpl` (decision 4): `taskq_destroy` takes it
//!   and the queue's memory goes when its last handle does, the deferred creator's and each
//!   worker's included, so a queue destroyed before its creator ran (`TQ_S_CREATED`) or while
//!   its last worker leaves is never freed under them. `systq` and `systqmp` are statics.
//! - The worklist and the threads' list (`TAILQ`, `SLIST` of stack nodes) are
//!   `hal::pinlist::PinList`s bound to the queue's mutex; a task is linked through
//!   `Pin<&Task>` (`task_add`), a worker's node and a barrier's task are pinned on their
//!   stacks (`core::pin::pin!`) and unlinked before their frames go.
//! - `struct task`'s `t_func`/`t_arg`, set by its owner while it is not pending and copied by a
//!   worker under the queue's mutex, are a small mutex of their own: the C's contract becomes a
//!   lock, so a racing `task_set` is a consistent snapshot, never a torn one.
//! - A thread is handed its queue as a start ticket (`kernel::thread`), not a `void *`.
//! - `taskq_barrier_task(void *tq)` casts its argument back to the queue; here a barrier task
//!   is marked (`TASK_BARRIER`, internal) and the worker that takes it runs the barrier's
//!   protocol on its own queue.
//! - `task_add_local` (LZ's `unsafe fn` for a task on a stack frame) is gone: [`task_add`]
//!   takes `Pin<&Task>`, which a stack frame gives with `core::pin::pin!`.

use core::ffi::c_void;
use core::ops::Deref;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use hal::intr::{IPL_HIGH, IPL_NONE};
use hal::mutex::Mutexed;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{PR_WAITOK, PoolArc, TypedPool};
use hal::sleep::{INFSLP, PWAIT};

use crate::assert::kpanic;
use crate::prf::printf;
use crate::thread::{self, Start};

/// `void (*t_func)(void *)`: the function a task calls, with its argument (the caller's, never
/// dereferenced here).
pub type TaskFn = fn(*mut c_void);

/// `TASK_ONQUEUE`: the task is on a queue's worklist (`t_flags`, as legacy callers spell
/// it; here the task's entry is linked).
pub const TASK_ONQUEUE: u32 = 1;

/// `TASKQ_MPSAFE`: the queue's threads run without the kernel lock.
pub const TASKQ_MPSAFE: u32 = 1 << 0;

/// `t_func` and `t_arg`.
struct Body {
    /// `t_func` (`None`: a task `task_set` never named).
    func: Option<TaskFn>,
    /// `t_arg`.
    arg: AtomicPtr<c_void>,
}

/// `struct task`. Not `Unpin`: it is linked through `Pin<&Task>` and must stay where it is
/// while pending (a `static`, a legacy `&'static`, `core::pin::pin!`).
pub struct Task {
    /// `t_entry`: the link in the queue's worklist.
    entry: PinEntry<Task>,
    /// `t_func`, `t_arg`: set by the owner while not pending, copied by a worker.
    body: Mutexed<Body>,
    /// A barrier's own task: the worker that takes it runs `taskq_barrier_task` on its queue.
    barrier: bool,
}

hal::pin_adapter!(
    /// A queue's worklist, through `Task::entry`.
    TaskList: Task, entry
);

impl Task {
    /// `TASK_INITIALIZER(func, arg)`.
    pub const fn new(func: TaskFn, arg: *mut c_void) -> Self {
        Self::with(Some(func), arg, false)
    }

    /// A task that `task_set` has not seen (all zero, a `static struct task`).
    pub const fn unset() -> Self {
        Self::with(None, ptr::null_mut(), false)
    }

    const fn with(func: Option<TaskFn>, arg: *mut c_void, barrier: bool) -> Self {
        Self {
            entry: PinEntry::new(),
            body: Mutexed::new(
                IPL_HIGH,
                Body {
                    func,
                    arg: AtomicPtr::new(arg),
                },
            ),
            barrier,
        }
    }

    /// `task_set(t, fn, arg)`: what the task calls, for its owner while it is not pending (the
    /// C's contract, which the C does not check: a worker that copies the task meanwhile
    /// gets the old pair or the new one, never a mix).
    pub fn set(&self, func: TaskFn, arg: *mut c_void) {
        let mut b = self.body.lock();
        b.func = Some(func);
        b.arg.store(arg, Ordering::Relaxed);
    }

    /// `task_pending(t)`: is the task on a queue, waiting to run? (Read without the lock, as
    /// the C's macro: the entry's link word, which the queue's holder sets when it links the
    /// task and clears last when it unlinks it, so "pending" and "on the list" are one fact.)
    pub fn pending(&self) -> bool {
        self.entry.is_linked()
    }
}

/// `taskq_thread`'s `struct taskq_thread`: a worker on its queue's list, by its thread's
/// identity (compared, never followed).
struct TaskqThread {
    /// `tt_entry`.
    entry: PinEntry<TaskqThread>,
    /// `tt_thread`: `curproc` as a word.
    thread: usize,
}

hal::pin_adapter!(
    /// A queue's threads, through `TaskqThread::entry`.
    Workers: TaskqThread, entry
);

/// `tq_state`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TqState {
    /// `TQ_S_CREATED`: its threads not made yet.
    Created,
    /// `TQ_S_RUNNING`.
    Running,
    /// `TQ_S_DESTROYED`.
    Destroyed,
}

/// What `tq_mtx` keeps besides the lists.
struct TqLocked {
    /// `tq_state`.
    life: TqState,
    /// `tq_running`: the threads running (the "tqdestroy" channel).
    running: u32,
    /// `tq_barriers`.
    barriers: u32,
    /// `tq_bgen` (the "tqbarend"/"tqbarwait" channel).
    bgen: u32,
    /// `tq_bthreads` (the "tqbar" channel).
    bthreads: u32,
}

/// `struct taskq`.
pub struct Taskq {
    /// `tq_mtx` and what it keeps.
    st: Mutexed<TqLocked>,
    /// `tq_worklist`, under `tq_mtx`.
    worklist: PinList<TaskList>,
    /// `tq_threads`, under `tq_mtx`.
    threads: PinList<Workers>,
    /// `tq_name` (the threads' name; its address is the "bored" channel).
    name: &'static [u8],
    /// `tq_nthreads`.
    nthreads: u32,
    /// `tq_flags`.
    flags: u32,
}

impl Taskq {
    /// A queue in `TQ_S_CREATED` (`taskq_sys`'s initialiser, `taskq_create`'s).
    const fn new(name: &'static [u8], nthreads: u32, ipl: i32, flags: u32) -> Self {
        Self {
            st: Mutexed::new(
                ipl,
                TqLocked {
                    life: TqState::Created,
                    running: 0,
                    barriers: 0,
                    bgen: 0,
                    bthreads: 0,
                },
            ),
            worklist: PinList::new(),
            threads: PinList::new(),
            name,
            nthreads,
            flags,
        }
    }

    /// The "bored" channel (`tq` in the C): the address of the queue's name field, which no
    /// other channel shares.
    fn bored(&self) -> *const () {
        ptr::from_ref(&self.name).cast()
    }
}

/// A task queue's handle: `taskq_create`'s, the pool's counted item (`taskqpl`).
pub type TaskqRef = PoolArc<Taskq>;

/// A queue as a thread or a deferred call holds it: a system queue, or a counted handle.
pub(crate) enum TqHandle {
    /// `systq`, `systqmp`.
    Sys(&'static Taskq),
    /// A queue of `taskq_create`.
    Made(TaskqRef),
}

impl Deref for TqHandle {
    type Target = Taskq;

    fn deref(&self) -> &Taskq {
        match self {
            TqHandle::Sys(t) => t,
            TqHandle::Made(t) => t,
        }
    }
}

impl Clone for TqHandle {
    fn clone(&self) -> Self {
        match self {
            TqHandle::Sys(t) => TqHandle::Sys(t),
            TqHandle::Made(t) => TqHandle::Made(t.clone()),
        }
    }
}

/// `taskq_sys`: `systq`, one thread, under the kernel lock.
pub static SYSTQ: Taskq = Taskq::new(b"systq", 1, IPL_HIGH, 0);
/// `taskq_sys_mp`: `systqmp`, one thread, `TASKQ_MPSAFE`.
pub static SYSTQMP: Taskq = Taskq::new(b"systqmp", 1, IPL_HIGH, TASKQ_MPSAFE);

/// The pool of created queues, `taskqpl` (decision 4: their `malloc(M_DEVBUF)` blocks move to
/// a pool row of their own).
static TASKQ_POOL: TypedPool<Taskq> = TypedPool::new();

/// `taskq_init()` (called by `main`): the system queues' thread creations deferred until
/// threads can be made; the pool of created queues made.
pub fn taskq_init() {
    if TASKQ_POOL.init(IPL_NONE, PR_WAITOK, "taskqpl").is_err() {
        kpanic(format_args!("taskq_init: cannot make taskqpl"));
    }
    // WITNESS_INIT: not configured.
    thread::create_deferred(Start::TaskqCreate(TqHandle::Sys(&SYSTQ)));
    thread::create_deferred(Start::TaskqCreate(TqHandle::Sys(&SYSTQMP)));
}

/// `taskq_create(name, nthreads, ipl, flags)`: a queue whose `nthreads` threads are made as
/// soon as threads can be (`kthread_create_deferred`). `None` only where the C's `malloc`
/// returns `NULL` (never with `M_WAITOK`) or before `taskq_init`.
pub fn taskq_create(name: &'static [u8], nthreads: u32, ipl: i32, flags: u32) -> Option<TaskqRef> {
    let tq = TASKQ_POOL.get_shared(PR_WAITOK, || Taskq::new(name, nthreads, ipl, flags))?;
    // WITNESS: not configured.

    // try to create a thread to guarantee that tasks will be serviced
    thread::create_deferred(Start::TaskqCreate(TqHandle::Made(tq.clone())));
    Some(tq)
}

/// `taskq_destroy(tq)`: a queue in `TQ_S_CREATED` is marked destroyed (its creator, which
/// holds its own handle, finds it so and makes no thread); a running one wakes its idle
/// workers and waits until they are gone. The queue's memory goes with its last handle.
pub fn taskq_destroy(tq: TaskqRef) {
    destroy(&tq);
}

/// [`taskq_destroy`] of a queue whose handle its caller gave up for a `&'static Taskq`
/// (`PoolArc::leak`): its threads stopped and waited for as by `taskq_destroy`; its memory
/// stays, since no handle is left to give it back. Only for the legacy holders that hand the
/// queue out as `&'static Taskq` to readers running beside the destroyer (`bsd`'s softnets,
/// softraid's disciplines, wsdisplay's screens: `sys/sys/task.rs`'s deviations), which no safe
/// cell can empty while such a reference may live; every other caller holds a [`TaskqRef`].
pub fn taskq_destroy_leaked(tq: &'static Taskq) {
    destroy(tq);
}

/// `taskq_destroy`'s protocol.
fn destroy(tq: &Taskq) {
    let mut g = tq.st.lock();
    match g.life {
        TqState::Created => {
            // tq is still referenced by taskq_create_thread
            g.life = TqState::Destroyed;
            return;
        }
        TqState::Running => g.life = TqState::Destroyed,
        TqState::Destroyed => kpanic(format_args!(
            "unexpected {} tq state {:?}",
            Name(tq.name),
            g.life
        )),
    }

    while g.running > 0 {
        hal::sleep::wakeup(tq.bored());
        let running = ptr::from_ref(&g.running);
        let _ = g.sleep_nsec(running, PWAIT, "tqdestroy", INFSLP);
    }
}

/// `taskq_create_thread(tq)` (deferred by `taskq_init`/`taskq_create`): the queue's threads,
/// unless it was destroyed meanwhile.
pub(crate) fn taskq_create_thread(tq: TqHandle) {
    let mut g = tq.st.lock();

    match g.life {
        TqState::Destroyed => return,
        TqState::Created => g.life = TqState::Running,
        TqState::Running => kpanic(format_args!(
            "unexpected {} tq state {:?}",
            Name(tq.name),
            g.life
        )),
    }

    loop {
        g.running += 1;
        drop(g);

        let rv = thread::create(tq.name, Start::TaskqWorker(tq.clone()));

        g = tq.st.lock();
        if rv.is_err() {
            printf(format_args!(
                "unable to create thread for \"{}\" taskq\n",
                Name(tq.name)
            ));

            g.running -= 1;
            // could have been destroyed during kthread_create
            if g.life == TqState::Destroyed && g.running == 0 {
                hal::sleep::wakeup_one(ptr::from_ref(&g.running));
            }
            break;
        }
        if g.running >= tq.nthreads {
            break;
        }
    }
}

/// `taskq_barrier_task(tq)`: run by a worker that took a barrier's task: counted among the
/// barrier's threads, it waits for the last barrier out.
fn taskq_barrier_task(tq: &Taskq) {
    let mut g = tq.st.lock();
    g.bthreads += 1;
    hal::sleep::wakeup(ptr::from_ref(&g.bthreads));

    let bgen0 = g.bgen;
    loop {
        let bgen = ptr::from_ref(&g.bgen);
        let _ = g.sleep_nsec(bgen, PWAIT, "tqbarend", INFSLP);
        if bgen0 != g.bgen {
            break;
        }
    }
}

/// `taskq_do_barrier(tq)`: waits until every thread of the queue but the caller ran a barrier
/// task (each was idle or finished what it ran), the barriers that overlap leaving together.
fn taskq_do_barrier(tq: &Taskq) {
    let t = core::pin::pin!(Task::with(None, ptr::null_mut(), true));
    let t = t.into_ref();
    let thread = hal::sleep::curproc_word();

    let mut g = tq.st.lock();
    g.barriers += 1;

    // is the barrier being run from a task inside the taskq?
    if tq.threads.iter(&g).any(|tt| tt.thread == thread) {
        g.bthreads += 1;
        hal::sleep::wakeup(ptr::from_ref(&g.bthreads));
    }

    while g.bthreads < tq.nthreads {
        // shove the task into the queue for a worker to pick up
        tq.worklist.insert_tail(&mut g, t);
        hal::sleep::wakeup_one(tq.bored());

        let bthreads = ptr::from_ref(&g.bthreads);
        let _ = g.sleep_nsec(bthreads, PWAIT, "tqbar", INFSLP);

        // another thread running a barrier might have done this work for us.
        if t.pending() {
            tq.worklist.remove(&mut g, &t);
        }
    }

    g.barriers -= 1;
    if g.barriers == 0 {
        // we're the last one out
        g.bgen = g.bgen.wrapping_add(1);
        hal::sleep::wakeup(ptr::from_ref(&g.bgen));
        g.bthreads = 0;
    } else {
        let bgen0 = g.bgen;
        loop {
            let bgen = ptr::from_ref(&g.bgen);
            let _ = g.sleep_nsec(bgen, PWAIT, "tqbarwait", INFSLP);
            if bgen0 != g.bgen {
                break;
            }
        }
    }
}

/// `taskq_barrier(tq)`: waits until the tasks the queue's threads were running are done.
pub fn taskq_barrier(tq: &Taskq) {
    // WITNESS_CHECKORDER: not configured.
    taskq_do_barrier(tq);
}

/// `taskq_del_barrier(tq, t)`: `t` deleted, then [`taskq_barrier`]: it is neither pending nor
/// running when this returns.
pub fn taskq_del_barrier(tq: &Taskq, t: &Task) {
    // WITNESS_CHECKORDER: not configured.
    task_del(tq, t);
    taskq_do_barrier(tq);
}

/// `task_add(tq, w)`: queued at the tail of `tq`'s worklist unless pending already; `true`
/// when it was added (the C's 1), and then a worker is woken.
pub fn task_add(tq: &Taskq, w: Pin<&Task>) -> bool {
    if w.pending() {
        return false;
    }

    let mut g = tq.st.lock();
    // On a list of this queue or, in a race the C loses, of another: not added either way.
    let rv = tq.worklist.try_insert_tail(&mut g, w);
    // NKCOV > 0: t_process.
    drop(g);

    if rv {
        hal::sleep::wakeup_one(tq.bored());
    }
    rv
}

/// `task_del(tq, w)`: off `tq`'s worklist when pending there; `true` when it was.
pub fn task_del(tq: &Taskq, w: &Task) -> bool {
    if !w.pending() {
        return false;
    }

    let mut g = tq.st.lock();
    let rv = w.pending();
    if rv {
        tq.worklist.remove(&mut g, w);
    }
    rv
}

/// What a worker takes off the worklist.
enum Work {
    /// A task's function and argument, copied.
    Run(TaskFn, *mut c_void),
    /// A barrier's task.
    Barrier,
}

/// `taskq_next_work(tq, &work)`: the first task taken off the worklist (sleeping "bored" while
/// it is empty and the queue runs), its function and argument copied to avoid races; `None`
/// once the queue no longer runs and its worklist is empty.
fn taskq_next_work(tq: &Taskq) -> Option<Work> {
    let mut g = tq.st.lock();
    loop {
        if !tq.worklist.is_empty(&g) {
            break;
        }
        if g.life != TqState::Running {
            return None;
        }
        let _ = g.sleep_nsec(tq.bored(), PWAIT, "bored", INFSLP);
    }

    // TAILQ_REMOVE then CLR(TASK_ONQUEUE): one step here, the unlink (the task is not touched
    // after it: another queue may link it as soon as it is free).
    let work = tq.worklist.first(&g).map(|next| {
        if next.barrier {
            Work::Barrier
        } else {
            // *work = *next: copy to caller to avoid races
            let b = next.body.lock();
            match b.func {
                Some(func) => Work::Run(func, b.arg.load(Ordering::Relaxed)),
                None => kpanic(format_args!(
                    "taskq_thread: {}: a task without a function",
                    Name(tq.name)
                )),
            }
        }
    });
    tq.worklist.remove_first(&mut g);
    let more = !tq.worklist.is_empty(&g);
    drop(g);

    if more && tq.nthreads > 1 {
        hal::sleep::wakeup_one(tq.bored());
    }
    work
}

/// `taskq_thread(tq)`: a worker. Runs what the worklist gives until the queue is destroyed and
/// emptied, the last one out waking the destroyer; then the thread exits.
pub(crate) fn taskq_thread(tq: TqHandle) -> ! {
    let mpsafe = tq.flags & TASKQ_MPSAFE != 0;
    if mpsafe {
        hal::klock::kernel_unlock();
    }

    let last = taskq_worker(&tq);

    if mpsafe {
        hal::klock::kernel_lock();
    }

    if let Some(running) = last {
        hal::sleep::wakeup_one(running);
    }

    // The handle goes before the thread does (an exit never returns to drop it).
    drop(tq);
    thread::exit(0)
}

/// The body of [`taskq_thread`] while its node is on the queue's list: when it was the last
/// thread out, the channel to wake the destroyer on (taken under the lock, as the C takes
/// `&tq->tq_running` before leaving it). The node, pinned on this frame, is unlinked before
/// the frame goes.
fn taskq_worker(tq: &Taskq) -> Option<*const u32> {
    let me = core::pin::pin!(TaskqThread {
        entry: PinEntry::new(),
        thread: hal::sleep::curproc_word(),
    });
    let me = me.into_ref();

    {
        let mut g = tq.st.lock();
        tq.threads.insert_head(&mut g, me);
    }

    // WITNESS_CHECKORDER: not configured.

    while let Some(work) = taskq_next_work(tq) {
        // WITNESS_LOCK; NKCOV > 0: kcov_remote_enter.
        match work {
            Work::Run(func, arg) => func(arg),
            Work::Barrier => taskq_barrier_task(tq),
        }
        // WITNESS_UNLOCK; NKCOV > 0: kcov_remote_leave.
        hal::sleep::sched_pause();
    }

    let mut g = tq.st.lock();
    tq.threads.remove(&mut g, &me);
    g.running -= 1;
    (g.running == 0).then(|| ptr::from_ref(&g.running))
}

/// A queue's name as the C prints it (`%s`, the bytes as they are).
struct Name(&'static [u8]);

impl core::fmt::Display for Name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for &b in self.0 {
            core::fmt::Write::write_char(f, char::from(b))?;
        }
        Ok(())
    }
}

/// Test hook: whether `tq`'s worklist is empty (a snapshot under its lock).
#[cfg(any(test, feature = "doubles"))]
pub fn worklist_is_empty(tq: &Taskq) -> bool {
    tq.worklist.is_empty(&tq.st.lock())
}

/// Test hook: a queue that is not `taskq_create`'s (no threads ever made), for the host
/// benches and tests that run its work by hand.
#[cfg(any(test, feature = "doubles"))]
pub const fn taskq_unthreaded(name: &'static [u8]) -> Taskq {
    Taskq::new(name, 1, IPL_HIGH, 0)
}

/// Test hook: `tq` (a queue no thread serves) marked running, as its creator would, so that
/// [`run_next_by_hand`] takes its work.
#[cfg(any(test, feature = "doubles"))]
pub fn run_by_hand(tq: &Taskq) {
    tq.st.lock().life = TqState::Running;
}

/// Test hook: one `taskq_next_work` of a queue [`run_by_hand`] started, and the call, by the
/// caller, as a worker would (never on an empty worklist: a worker would sleep "bored").
#[cfg(any(test, feature = "doubles"))]
pub fn run_next_by_hand(tq: &Taskq) {
    match taskq_next_work(tq) {
        Some(Work::Run(func, arg)) => func(arg),
        Some(Work::Barrier) => taskq_barrier_task(tq),
        None => {}
    }
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The worklist bookkeeping is tested without threads (`taskq_next_work` called as a
    // worker would); the workers, the destroy and the barrier with the thread double
    // (`testutil`'s host threads), each on a queue of its own.

    use core::sync::atomic::{AtomicU32, AtomicUsize};
    use std::sync::{Mutex as StdMutex, MutexGuard};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::testutil;

    fn nothing(_arg: *mut c_void) {}

    fn arg(n: usize) -> *mut c_void {
        ptr::without_provenance_mut(n)
    }

    fn pinned(t: &'static Task) -> Pin<&'static Task> {
        hal::pin::pin_static(t)
    }

    /// `taskq_init` once per process (the pool, the system queues' creators).
    fn init() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        testutil::install();
        ONCE.call_once(taskq_init);
    }

    /// What the payload of a test task reads back.
    fn arg_of(w: Work) -> usize {
        match w {
            Work::Run(_, a) => a.addr(),
            Work::Barrier => usize::MAX,
        }
    }

    /// Waits up to ten seconds for `f`.
    fn eventually(mut f: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if f() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        f()
    }

    #[test]
    fn task_set_fills_an_unset_task_like_the_initializer() {
        static T: Task = Task::unset();
        assert!(T.body.lock().func.is_none());
        T.set(nothing, arg(5));
        assert!(T.body.lock().func.is_some());
        assert_eq!(T.body.lock().arg.load(Ordering::Relaxed), arg(5));
        assert!(!T.pending());

        let init = Task::new(nothing, arg(7));
        assert!(init.body.lock().func.is_some());
        assert_eq!(init.body.lock().arg.load(Ordering::Relaxed), arg(7));
        assert!(!init.pending());
    }

    #[test]
    fn add_and_del_keep_the_flag_and_the_worklist_in_step() {
        testutil::install();
        static TQ: Taskq = taskq_unthreaded(b"tqtest");
        static A: Task = Task::new(nothing, ptr::null_mut());
        static B: Task = Task::new(nothing, ptr::null_mut());

        assert!(task_add(&TQ, pinned(&A)));
        assert!(A.pending());
        assert!(!task_add(&TQ, pinned(&A)), "already pending");
        assert!(task_add(&TQ, pinned(&B)));
        assert_eq!(TQ.worklist.iter(&TQ.st.lock()).count(), 2);

        assert!(task_del(&TQ, &A));
        assert!(!A.pending());
        assert!(!task_del(&TQ, &A), "no longer pending");
        assert!(
            TQ.worklist
                .first(&TQ.st.lock())
                .is_some_and(|t| ptr::eq(t, &B))
        );

        assert!(task_del(&TQ, &B));
        assert!(worklist_is_empty(&TQ));
    }

    #[test]
    fn next_work_hands_out_copies_in_order() {
        testutil::install();
        static TQ: Taskq = taskq_unthreaded(b"tqfifo");
        static T1: Task = Task::unset();
        static T2: Task = Task::unset();
        static T3: Task = Task::unset();
        TQ.st.lock().life = TqState::Running;
        for (n, t) in [&T1, &T2, &T3].into_iter().enumerate() {
            t.set(nothing, arg(n + 1));
            assert!(task_add(&TQ, pinned(t)));
        }
        assert!(task_del(&TQ, &T2), "a deleted task is not handed out");

        let w = taskq_next_work(&TQ).expect("T1");
        assert_eq!(arg_of(w), 1);
        assert!(!T1.pending(), "taken off the worklist");
        assert!(task_add(&TQ, pinned(&T1)), "may be queued again at once");

        // A destroyed queue still drains its worklist before the thread is told to stop.
        TQ.st.lock().life = TqState::Destroyed;
        assert_eq!(arg_of(taskq_next_work(&TQ).expect("T3")), 3);
        assert_eq!(arg_of(taskq_next_work(&TQ).expect("T1 again")), 1);
        assert!(taskq_next_work(&TQ).is_none());
    }

    #[test]
    fn next_work_on_an_idle_queue_that_is_not_running_is_none() {
        testutil::install();
        static TQ: Taskq = taskq_unthreaded(b"tqidle");
        assert!(taskq_next_work(&TQ).is_none(), "TQ_S_CREATED and empty");
    }

    #[test]
    fn the_system_queues_start_created_with_one_thread() {
        for (tq, name, flags) in [
            (&SYSTQ, b"systq".as_slice(), 0),
            (&SYSTQMP, b"systqmp", TASKQ_MPSAFE),
        ] {
            assert_eq!(tq.name, name);
            assert_eq!(tq.nthreads, 1);
            assert_eq!(tq.flags, flags);
        }
    }

    #[test]
    fn the_wait_channels_are_distinct() {
        testutil::install();
        static TQ: Taskq = taskq_unthreaded(b"tqchan");
        let g = TQ.st.lock();
        let chans = [
            TQ.bored().addr(),
            ptr::from_ref(&g.running).addr(),
            ptr::from_ref(&g.bthreads).addr(),
            ptr::from_ref(&g.bgen).addr(),
        ];
        for (i, a) in chans.iter().enumerate() {
            assert!(chans[i + 1..].iter().all(|b| b != a), "{chans:x?}");
        }
    }

    #[test]
    fn a_queue_destroyed_before_its_threads_makes_none() {
        init();
        // The thread double runs a deferred creation at once: destroy first, by hand.
        let tq = TASKQ_POOL
            .get_shared(PR_WAITOK, || Taskq::new(b"tqdead", 1, IPL_NONE, 0))
            .expect("a queue");
        let creator = TqHandle::Made(tq.clone());
        taskq_destroy(tq);
        assert_eq!(creator.st.lock().life, TqState::Destroyed);
        // The deferred creation sees TQ_S_DESTROYED and makes no thread; its handle is the
        // last, and the queue goes with it.
        taskq_create_thread(creator);
    }

    static RAN: AtomicU32 = AtomicU32::new(0);

    fn bump(_arg: *mut c_void) {
        RAN.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn a_created_queue_runs_its_tasks_and_is_destroyed_with_its_threads() {
        init();
        static T: Task = Task::new(bump, ptr::null_mut());
        let tq = taskq_create(b"tqrun", 2, IPL_NONE, TASKQ_MPSAFE).expect("a queue");
        assert!(eventually(|| tq.st.lock().running == 2), "two workers");
        let before = RAN.load(Ordering::SeqCst);
        assert!(task_add(&tq, pinned(&T)));
        assert!(eventually(|| RAN.load(Ordering::SeqCst) > before));
        taskq_barrier(&tq);
        assert!(!T.pending());
        let probe = tq.clone();
        taskq_destroy(tq);
        assert_eq!(probe.st.lock().running, 0, "every worker gone");
        assert_eq!(probe.st.lock().life, TqState::Destroyed);
        // The destroyer's and the workers' handles are gone: the probe's is the last, so the
        // queue's `taskqpl` item goes back with it (the C's `free`).
        assert!(eventually(|| PoolArc::count(&probe) == 1));
    }

    static INSIDE: AtomicUsize = AtomicUsize::new(0);
    static BARRIER_DONE: AtomicU32 = AtomicU32::new(0);
    static QUEUE: StdMutex<Option<TaskqRef>> = StdMutex::new(None);

    fn queue() -> MutexGuard<'static, Option<TaskqRef>> {
        QUEUE.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A task that runs a barrier on its own queue (the C's "barrier being run from a task
    /// inside the taskq").
    fn barrier_from_inside(_arg: *mut c_void) {
        let tq = queue().clone().expect("the queue");
        INSIDE.store(hal::sleep::curproc_word(), Ordering::SeqCst);
        taskq_barrier(&tq);
        BARRIER_DONE.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn a_barrier_from_inside_a_worker_counts_its_own_thread() {
        init();
        static T: Task = Task::new(barrier_from_inside, ptr::null_mut());
        let tq = taskq_create(b"tqbarin", 2, IPL_NONE, TASKQ_MPSAFE).expect("a queue");
        assert!(eventually(|| tq.st.lock().running == 2));
        *queue() = Some(tq.clone());
        assert!(task_add(&tq, pinned(&T)));
        assert!(eventually(|| BARRIER_DONE.load(Ordering::SeqCst) == 1));
        // And one from outside after it.
        taskq_barrier(&tq);
        *queue() = None;
        taskq_destroy(tq);
    }

    #[test]
    fn del_barrier_leaves_a_task_neither_pending_nor_running() {
        init();
        static T: Task = Task::new(bump, ptr::null_mut());
        let tq = taskq_create(b"tqdelbar", 1, IPL_NONE, TASKQ_MPSAFE).expect("a queue");
        assert!(eventually(|| tq.st.lock().running == 1));
        task_add(&tq, pinned(&T));
        taskq_del_barrier(&tq, &T);
        assert!(!T.pending());
        taskq_destroy(tq);
    }

    #[test]
    #[should_panic(expected = "not on a list of this lock")]
    fn task_del_of_a_task_pending_on_another_queue_panics() {
        testutil::install();
        static Q1: Taskq = taskq_unthreaded(b"tqdel1");
        static Q2: Taskq = taskq_unthreaded(b"tqdel2");
        static T: Task = Task::new(nothing, ptr::null_mut());
        assert!(task_add(&Q1, pinned(&T)));
        // The C would unlink it through Q2's head; here the worklist refuses it. (T stays on
        // Q1, a static: nothing is dropped linked.)
        task_del(&Q2, &T);
    }

    #[test]
    fn a_task_on_another_queue_is_not_added_twice() {
        testutil::install();
        static Q1: Taskq = taskq_unthreaded(b"tqone");
        static Q2: Taskq = taskq_unthreaded(b"tqtwo");
        static T: Task = Task::new(nothing, ptr::null_mut());
        assert!(task_add(&Q1, pinned(&T)));
        assert!(!task_add(&Q2, pinned(&T)), "pending on the first queue");
        // The race the C would lose: the claim decides, nothing is linked twice.
        let mut g = Q2.st.lock();
        assert!(!Q2.worklist.try_insert_tail(&mut g, pinned(&T)));
        drop(g);
        assert!(task_del(&Q1, &T));
        assert!(task_add(&Q2, pinned(&T)), "free again");
        assert!(task_del(&Q2, &T));
    }
}
/* </TESTS> */
