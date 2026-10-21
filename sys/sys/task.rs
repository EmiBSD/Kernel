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
//! `<sys/task.h>`: `task_add(9)`, the work items a task queue runs in a thread. The queues
//! and their threads are the kernel crate's (`kernel::task`, N4.1); this is the legacy API the
//! callers in `bsd` keep, over it.
//!
//! Upstream: sys/sys/task.h @ 3ce1f3f79392
//!
//! Status: `ported` (M7b); adapted at N4.1: the functions of `kern/kern_task.rs` live here
//! now (`crate::kern::kern_task` names this module), over `kernel::task`.
//!
//! ## Deviations
//! - `struct taskq` is the kernel crate's ([`Taskq`], re-exported), and `taskq_create` hands
//!   out its counted handle ([`TaskqRef`]): `taskq_destroy` takes it and the queue's memory (a
//!   `taskqpl` item) goes with its last handle, as the C's `free`. Three callers keep LZ's
//!   `&'static Taskq` in a `Cell` that readers running beside the destroyer copy with `get()`
//!   (the softnets of `if_.rs`, destroyed once at boot by `softnet_percpu`; softraid's
//!   `sd_taskq`; wsdisplay's `sc_taskq`): no safe cell lends a handle to such readers and
//!   gives it up to a destroyer, so they leak the handle (`TaskqRef::leak`) and destroy with
//!   [`taskq_destroy_leaked`], whose queue's threads are stopped and waited for as the C's
//!   but whose memory stays. What stays: the softnets past `softnet_count()`, `NET_TASKQ`
//!   (8) less one per CPU, once per boot (six `taskqpl` items on two CPUs, four on four);
//!   one item per softraid discipline torn down (a volume created and detached by
//!   `bioctl`); one item per wsdisplay detached. A deviation of those three callers, until
//!   they hold a `TaskqRef` (N6, N7).
//! - `struct task` keeps its fields for the code that links tasks on lists of its own and runs
//!   them (`if_.rs`'s hooks, `ifq.rs`'s serializer: `t_entry`, `t_func`, `t_arg`, `t_flags`),
//!   and carries the kernel's task (`t_node`), which is what a task queue links. `task_set`
//!   sets both; `task_pending` is either one's (the C's one flag: a task is on one list at a
//!   time), and `task_add` refuses a task such a list holds, as the C's `TASK_ONQUEUE` test.
//! - `t_flags` is an `AtomicU32` (`Relaxed`), read without a lock, as the C does.
//! - `t_func` is an `Option`: a task that `task_set` has not seen ([`Task::zeroed`], the C's
//!   zeroed `struct task`) has no function; `TASK_INITIALIZER` is [`Task::new`].
//! - `t_process` (the `kcov` identifier, `#if 1 /* NKCOV > 0 */`) is kept but never written:
//!   `kcov` is not configured.
//! - `task_add` takes a `&'static Task` (LZ's: a pending task outlives its place on the
//!   worklist); a task on a stack frame (`sched_barrier`'s, `ipmi`'s) is the kernel's
//!   `kernel::task::Task`, pinned there (`core::pin::pin!`).

use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

pub use kernel::task::{TASKQ_MPSAFE, TaskFn, Taskq, TaskqRef};

use crate::queue_adapter;
use crate::sys::proc::Process;
use crate::sys::queue::TailqEntry;

/// `TASK_ONQUEUE`: the task is on a list (`t_flags`).
pub const TASK_ONQUEUE: u32 = 1;

/// `struct task`.
pub struct Task {
    /// `t_entry`: the link in a list of its user's own (`if_.rs`'s hooks, `ifq.rs`'s
    /// serializer). Protected by: that list's lock.
    pub t_entry: TailqEntry<Task>,
    /// `t_func`: function to call. Set by the owner (`task_set`) while the task is not pending.
    pub t_func: Cell<Option<TaskFn>>,
    /// `t_arg`: function argument. Set with `t_func`.
    pub t_arg: Cell<*mut c_void>,
    /// `t_flags`: `TASK_ONQUEUE` while on such a list. Written under its lock, read without it.
    pub t_flags: AtomicU32,
    /// `t_process`: kcov identifier (not configured, never written).
    pub t_process: Cell<*const Process>,
    /// The kernel's task, what a task queue links (`kernel::task`).
    pub t_node: kernel::task::Task,
}

// SAFETY: the link and `t_flags` change under the lock of the list the task is on; `t_func`,
// `t_arg` and `t_process` are written by the task's owner while it is not pending and read by
// the list's runner under its lock, which is the C's contract for `task_set(9)`; `t_node` is
// `Sync` of its own.
unsafe impl Sync for Task {}

impl Task {
    /// `TASK_INITIALIZER(_f, _a)`.
    pub const fn new(func: TaskFn, arg: *mut c_void) -> Self {
        Self {
            t_entry: TailqEntry::new(),
            t_func: Cell::new(Some(func)),
            t_arg: Cell::new(arg),
            t_flags: AtomicU32::new(0),
            t_process: Cell::new(ptr::null()),
            t_node: kernel::task::Task::new(func, arg),
        }
    }

    /// A task that `task_set` has not seen yet (all zero, like a `static struct task`).
    pub const fn zeroed() -> Self {
        Self {
            t_entry: TailqEntry::new(),
            t_func: Cell::new(None),
            t_arg: Cell::new(ptr::null_mut()),
            t_flags: AtomicU32::new(0),
            t_process: Cell::new(ptr::null()),
            t_node: kernel::task::Task::unset(),
        }
    }
}

queue_adapter!(
    /// `TAILQ_HEAD(task_list, task)`.
    pub TaskList: Task, t_entry => TailqEntry<Task>
);

/// `systq`: the system task queue, under the kernel lock.
pub static SYSTQ: &Taskq = &kernel::task::SYSTQ;
/// `systqmp`: the system task queue without the kernel lock.
pub static SYSTQMP: &Taskq = &kernel::task::SYSTQMP;

/// `task_pending(_t)`: is the task on a queue, or on a list of its user's, waiting to run?
pub fn task_pending(t: &Task) -> bool {
    t.t_flags.load(Ordering::Relaxed) & TASK_ONQUEUE != 0 || t.t_node.pending()
}

/// `taskq_init()`: the system queues' threads, deferred (called by `main`).
pub fn taskq_init() {
    kernel::task::taskq_init();
}

/// `taskq_create(name, nthreads, ipl, flags)`: the queue's counted handle.
pub fn taskq_create(name: &'static [u8], nthreads: u32, ipl: i32, flags: u32) -> Option<TaskqRef> {
    kernel::task::taskq_create(name, nthreads, ipl, flags)
}

/// `taskq_destroy(tq)`: the queue's threads stopped and waited for; its memory goes with its
/// last handle.
pub fn taskq_destroy(tq: TaskqRef) {
    kernel::task::taskq_destroy(tq);
}

/// `taskq_destroy(tq)` of a queue whose handle was given up for a `&'static Taskq` (the
/// module's deviations): its threads stopped and waited for; its memory stays.
pub fn taskq_destroy_leaked(tq: &'static Taskq) {
    kernel::task::taskq_destroy_leaked(tq);
}

/// `taskq_barrier(tq)`.
pub fn taskq_barrier(tq: &Taskq) {
    kernel::task::taskq_barrier(tq);
}

/// `taskq_del_barrier(tq, t)`.
pub fn taskq_del_barrier(tq: &Taskq, t: &Task) {
    kernel::task::taskq_del_barrier(tq, &t.t_node);
}

/// `task_set(t, fn, arg)`: the function and argument of both faces of the task.
pub fn task_set(t: &Task, func: TaskFn, arg: *mut c_void) {
    t.t_func.set(Some(func));
    t.t_arg.set(arg);
    t.t_flags.store(0, Ordering::Relaxed);
    t.t_node.set(func, arg);
}

/// `task_add(tq, w)`: `true` when the task was added (the C's 1). A task its user's own list
/// holds (`TASK_ONQUEUE` in `t_flags`: `ifq.rs`'s serializer) is not added, as the C's test of
/// its one flag returns 0 before taking the queue's mutex; one pending on a queue already is
/// refused by `kernel::task::task_add`.
pub fn task_add(tq: &Taskq, w: &'static Task) -> bool {
    if w.t_flags.load(Ordering::Relaxed) & TASK_ONQUEUE != 0 {
        return false;
    }
    kernel::task::task_add(tq, hal::pin::pin_static(&w.t_node))
}

/// `task_del(tq, w)`: `true` when the task was pending on `tq`.
pub fn task_del(tq: &Taskq, w: &Task) -> bool {
    kernel::task::task_del(tq, &w.t_node)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn nothing(_arg: *mut c_void) {}

    #[test]
    fn task_set_fills_both_faces_and_pending_reads_either() {
        static T: Task = Task::zeroed();
        assert!(T.t_func.get().is_none());
        task_set(&T, nothing, ptr::without_provenance_mut(5));
        assert!(T.t_func.get().is_some());
        assert_eq!(T.t_arg.get(), ptr::without_provenance_mut(5));
        assert!(!task_pending(&T));
        // A list of the task's user (ifq's serializer) marks it in t_flags.
        T.t_flags.store(TASK_ONQUEUE, Ordering::Relaxed);
        assert!(task_pending(&T));
        T.t_flags.store(0, Ordering::Relaxed);
        assert!(!task_pending(&T));
    }

    #[test]
    fn task_add_refuses_a_task_its_users_list_holds() {
        kernel::testutil::install();
        static TQ: Taskq = kernel::task::taskq_unthreaded(b"tqflags");
        static T: Task = Task::zeroed();
        task_set(&T, nothing, ptr::null_mut());
        // On ifq's serializer list: the C returns 0 and leaves the queue alone.
        T.t_flags.store(TASK_ONQUEUE, Ordering::Relaxed);
        assert!(!task_add(&TQ, &T));
        assert!(!T.t_node.pending());
        assert!(kernel::task::worklist_is_empty(&TQ));
        // Off that list: added, and refused while pending.
        T.t_flags.store(0, Ordering::Relaxed);
        assert!(task_add(&TQ, &T));
        assert!(!task_add(&TQ, &T));
        assert!(task_del(&TQ, &T));
        assert!(kernel::task::worklist_is_empty(&TQ));
    }
}
/* </TESTS> */
