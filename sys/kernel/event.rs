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
 * Copyright (c) 1999,2000,2001 Jonathan Lemon <jlemon@FreeBSD.org>
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * $FreeBSD: src/sys/kern/kern_event.c,v 1.22 2001/02/23 20:32:42 jlemon Exp $
 */

/*-
 * Copyright (c) 1999,2000,2001 Jonathan Lemon <jlemon@FreeBSD.org>
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	$FreeBSD: src/sys/sys/event.h,v 1.11 2001/02/24 01:41:31 jlemon Exp $
 */

/*-
 * Copyright (c) 1999,2000 Jonathan Lemon <jlemon@FreeBSD.org>
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	$FreeBSD: src/sys/sys/eventvar.h,v 1.3 2000/05/26 02:06:54 jake Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! `kqueue(2)` and `kevent(2)`: the knotes and klists every event source hangs its waiters
//! on, the kqueue and its pending queue, the system filters (`EVFILT_PROC`, `EVFILT_SIGNAL`,
//! `EVFILT_TIMER`, `EVFILT_USER`, the descriptor filters' dispatch, `seltrue`, the dead and
//! bad-descriptor filters), and the per-thread queue `select(2)` and `poll(2)` are built on
//! (`kqpoll_*`).
//!
//! Upstream: sys/kern/kern_event.c @ 3ce1f3f79392
//! Upstream: sys/sys/event.h @ 3ce1f3f79392
//! Upstream: sys/sys/eventvar.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_event.rs@3c62ede68802
//! LZ: sys/sys/event.rs@3c62ede68802
//! LZ: sys/sys/eventvar.rs@3c62ede68802
//!
//! A kqueue ([`Kqueue`]) is an item of the pool `kqueuepl`, held by counted handles
//! ([`KqRef`], `kq_refcnt`): its file's object, its thread's `p_kq`, every scan running on it,
//! every knote it owns and its pending task. A knote ([`Knote`]) is an item of `knotepl`
//! owned by one chain of its kqueue (a descriptor slot of `kq_knlist` or a bucket of
//! `kq_knhash`, `hal::owned::OwnedList`s under `kq_lock`); while it lives it is also linked
//! by address on its object's klist ([`Klist`], `kn_selnext`) and, while active, on the
//! kqueue's pending queue (`kq_head`, `kn_tqe`), both `hal::pinlist::PinList`s.
//!
//! `kqueue_register` turns a `struct kevent` change into a knote: it finds or makes the knote
//! in its chain, attaches it to its object through the filter (`f_attach`; for a descriptor
//! filter the file's `fo_kqfilter`, which hooks it on the object's klist), and queues it when
//! its event is already pending. Objects call `knote`/`knote_locked` on their klist; an active
//! knote is queued on `kq_head`, and `kqueue_scan` hands the queued events out, the scan's
//! markers bounding what one call collects. A knote being worked on is `KN_PROCESSING`
//! (`knote_acquire`/`knote_release`), which serializes its filter calls without holding
//! `kq_lock` across them.
//!
//! Locks (the C's legend): `[I]` immutable after creation, `[q]` `kq_lock` (a spin mutex at
//! `IPL_HIGH`), `[o]` the object's lock, `[L]` `kqueue_klist_lock`, `[t]` `ft_mtx`, `[a]`
//! atomic operations. Order: object lock, then `kq_lock`; `ft_mtx`, then `kq_lock`.
//!
//! ## Deviations
//! - `PR_WAITOK`/`M_WAITOK` allocations are checked: `kqueue_alloc` and `kqueue_register`
//!   answer `ENOMEM` when a pool or `malloc(9)` gives nothing (before registration, or under
//!   the host tests' failing doubles), where the C cannot fail (LZ's).
//! - `kqpoll_init`, `kqpoll_done` and `kqpoll_exit` take the thread (the C reads `curproc`);
//!   `filt_procattach` and `filt_sigattach` read it through the glue ([`ProcGlue::curproc`]).
//! - `KQUEUE_DEBUG` (`kqueue_check`) and `KTRACE` (`ktrevent`, `ktrreltimespec`) are not
//!   configured. `KLIST_ASSERT_LOCKED` is the witness each klist operation takes.
//! - A thread's poll kqueue stays allocated, emptied and terminated, from `kqpoll_exit` until
//!   the thread's own memory goes (`p_kq` is set once, [`Thread::p_kq`]), where the C's
//!   `KQRELE` frees it at once (`vmstat -m kqueuepl` only).
//! - A chain links its knotes at its tail, where the C's `SLIST_INSERT_HEAD` links them at
//!   its head: the knotes of one descriptor are removed (`knote_fdclose`, `kqueue_purge`)
//!   oldest first. What userland sees does not change: a closed descriptor's poll knotes all
//!   report `EBADF` (`POLLNVAL`, the same `select(2)` error) in either order.
//!
//! ## Redesign
//! - LZ's knotes and kqueues were raw pool items handed out as `&'static` (`knote_drop` and
//!   `KQRELE` were `unsafe fn`s whose callers vouched they did not use them after), linked by
//!   `queue.h` adapters of raw pointers, the knote's `kn_kq` and the scan's `kqs_kq` raw
//!   pointers, `kq_knlist`/`kq_knhash` raw `malloc` arrays copied by `ptr::read`. Now a kqueue
//!   is freed with its last [`KqRef`] (`KQRELE` is a drop: the table's `fd_kqlist` unlinked
//!   then), a knote is owned by its chain and freed only when the chain gives its box back
//!   (`OwnedList::take`), and a holder that drops `kq_lock` across a filter call keeps the
//!   knote by a counted reference ([`KnHold`], `hal::owned::SecondaryRef`): the chain refuses
//!   to give back a counted knote, so `KN_PROCESSING`'s ownership is a type. The chains are
//!   `MArray<OwnedList<..>>`s grown by moving the heads (a head has an identity, not an
//!   address).
//! - `struct klist`'s lock operations (`klistops`, three `unsafe fn` pointers over an opaque
//!   `kl_arg`) are three kinds ([`KlistKind`]): the kernel lock (`kl_ops == NULL`), the
//!   owner's lock, whose witness the owner passes (`ps_mtx`, `kqueue_klist_lock`), and the
//!   klist's own `IPL_HIGH` mutex, taken inside the legacy lock its `bsd` owner still holds
//!   (`klist_init_mutex`/`klist_init_rwlock`: sockets, audio, midi, bpf, wsevent, fuse); a
//!   klist operation that would lock a klist of the wrong kind stops the kernel.
//! - `kn_ptr`'s union is three members (`kn_fp` a one-word `FileSlot` under a leaf mutex,
//!   `p_process` a counted process handle under its own, `p_useract` an atomic); the `[o]`
//!   words of `kn_kevent` are relaxed atomics with LZ's `get()`/`set()`; `kn_status` is a cell
//!   keyed to `kq_lock`.
//! - `filt_timer`'s `struct filter_timer` (a `malloc` block behind `kn_hook`, its timeout's
//!   argument the knote) is a `kqtimerpl` item the knote holds ([`FiltTimer`]); its timeout
//!   takes the item as a counted, erased argument (`Timeout::set_shared`) and reaches the
//!   knote by the counted reference the timer keeps while it is attached. Its expiries count
//!   under `ft_mtx` and reach `kn_data` when the event is collected.
//! - `kq_task`'s argument is the kqueue as a counted, erased handle (`Task::set_shared`),
//!   set when the task is first scheduled and dropped by `kqueue_terminate` after its barrier.
//! - The identity tests of `kn_fop` against `dead_filtops`, `badfd_filtops` and
//!   `user_filtops` are flag bits of `f_flags` that only those filters carry
//!   ([`FILTEROP_DEAD`], ...), so a copy of the filter (`bsd`'s statics) is recognised.
//! - The scan markers are knotes the caller pins on its stack ([`Knote::marker`]); the scan's
//!   state unlinks its end marker and drops its reference when it is dropped
//!   (`kqueue_scan_finish`).
//! - `filt_proc` runs under `ps_mtx` inside the walk of `ps_klist` and cannot drop it: what
//!   the C does there dropping `ps_mtx` runs after the walk, under `kqueue_ps_list_lock`.
//!   `NOTE_EXIT`'s unlink of the knote from the process's klist is made by
//!   `knote_processexit` once the walk is over; `NOTE_TRACK`'s registration on the child by
//!   `knote_processfork`, which then activates the parent's knote if it has an event, so the
//!   order of the queue (the child's `NOTE_CHILD` first) and `NOTE_TRACKERR` are the C's.
//! - `fd_kqlist` is a `PinList` under its own spin mutex in the table (the C's `fdplock`),
//!   walked by `knote_fdclose` one counted kqueue at a time, so no table lock is held when a
//!   kqueue's last reference drops and unlinks it (the C's `rw_status == RW_WRITE` test is
//!   gone with it).

use core::cell::Cell;
use core::ffi::c_void;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU8, AtomicU32, Ordering};

use hal::intr::{IPL_HIGH, IPL_NONE, IPL_SOFTCLOCK};
use hal::klock;
use hal::link::Link;
use hal::malloc::{M_WAITOK, MArray, MBox};
use hal::mutex::{Mutexed, MutexedGuard, SharedMutex};
use hal::once::OnceStatic;
use hal::owned::{OwnedEntry, OwnedList, SecondaryRef};
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{AnyArc, BoxPool, PR_WAITOK, PoolArc, PoolBox, PoolHome, TypedPool};
use hal::rwlock::{Divisible, Guarded, RW_WRITE, RawRwlock, Witness};
use hal::sleep::{INFSLP, PCATCH};
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FNONBLOCK, FREAD, FWRITE, O_CLOEXEC};
use hal::sys::pledge::PLEDGE_PROC;
use hal::sys::types::Pid;
use hal::sys::wait::w_exitcode;
use hal::user::UserPtr;

use crate::assert::{kassert, kpanic};
use crate::clock::{self, Timespec};
use crate::descrip::{
    FdRef, UF_EXCLOSE, fd_checkclosed, fd_getfile, fdinsert, fdpassertlocked, fdplock,
};
use crate::file::{DTYPE_KQUEUE, File, FileGlue, FileOps, FileRef, FileSlot};
use crate::proc::{
    I16Word, I64Word, PID_MAX, PS_EXITING, PS_PLEDGE, ProcGlue, Process, ProcessRef, Thread,
    U16Word, U32Word, UsizeWord, prfind,
};
use crate::task::{SYSTQMP, Task, task_add, taskq_del_barrier};
use crate::timeout::{KCLOCK_NONE, Timeout};

/// The file glue of a process glue's threads.
type Files<G> = <G as ProcGlue>::FileGlue;

/// `EVFILT_READ`.
pub const EVFILT_READ: i16 = -1;
/// `EVFILT_WRITE`.
pub const EVFILT_WRITE: i16 = -2;
/// `EVFILT_AIO`: attached to aio requests.
pub const EVFILT_AIO: i16 = -3;
/// `EVFILT_VNODE`: attached to vnodes.
pub const EVFILT_VNODE: i16 = -4;
/// `EVFILT_PROC`: attached to struct process.
pub const EVFILT_PROC: i16 = -5;
/// `EVFILT_SIGNAL`: attached to struct process.
pub const EVFILT_SIGNAL: i16 = -6;
/// `EVFILT_TIMER`: timers.
pub const EVFILT_TIMER: i16 = -7;
/// `EVFILT_DEVICE`: devices.
pub const EVFILT_DEVICE: i16 = -8;
/// `EVFILT_EXCEPT`: exceptional conditions.
pub const EVFILT_EXCEPT: i16 = -9;
/// `EVFILT_USER`: user event.
pub const EVFILT_USER: i16 = -10;
/// `EVFILT_SYSCOUNT`.
pub const EVFILT_SYSCOUNT: i32 = 10;

/// `EV_ADD`: add event to kq (implies enable).
pub const EV_ADD: u16 = 0x0001;
/// `EV_DELETE`: delete event from kq.
pub const EV_DELETE: u16 = 0x0002;
/// `EV_ENABLE`: enable event.
pub const EV_ENABLE: u16 = 0x0004;
/// `EV_DISABLE`: disable event (not reported).
pub const EV_DISABLE: u16 = 0x0008;
/// `EV_ONESHOT`: only report one occurrence.
pub const EV_ONESHOT: u16 = 0x0010;
/// `EV_CLEAR`: clear event state after reporting.
pub const EV_CLEAR: u16 = 0x0020;
/// `EV_RECEIPT`: force `EV_ERROR` on success, data=0.
pub const EV_RECEIPT: u16 = 0x0040;
/// `EV_DISPATCH`: disable event after reporting.
pub const EV_DISPATCH: u16 = 0x0080;
/// `EV_SYSFLAGS`: reserved by system.
pub const EV_SYSFLAGS: u16 = 0xf800;
/// `EV_FLAG1`: filter-specific flag.
pub const EV_FLAG1: u16 = 0x2000;
/// `EV_EOF`: EOF detected.
pub const EV_EOF: u16 = 0x8000;
/// `EV_ERROR`: error, data contains errno.
pub const EV_ERROR: u16 = 0x4000;

/// `NOTE_LOWAT`: low water mark.
pub const NOTE_LOWAT: u32 = 0x0001;
/// `NOTE_EOF`: return on EOF.
pub const NOTE_EOF: u32 = 0x0002;
/// `NOTE_OOB`: OOB data on a socket.
pub const NOTE_OOB: u32 = 0x0004;
/// `NOTE_DELETE`: vnode was removed.
pub const NOTE_DELETE: u32 = 0x0001;
/// `NOTE_WRITE`: data contents changed.
pub const NOTE_WRITE: u32 = 0x0002;
/// `NOTE_EXTEND`: size increased.
pub const NOTE_EXTEND: u32 = 0x0004;
/// `NOTE_ATTRIB`: attributes changed.
pub const NOTE_ATTRIB: u32 = 0x0008;
/// `NOTE_LINK`: link count changed.
pub const NOTE_LINK: u32 = 0x0010;
/// `NOTE_RENAME`: vnode was renamed.
pub const NOTE_RENAME: u32 = 0x0020;
/// `NOTE_REVOKE`: vnode access was revoked.
pub const NOTE_REVOKE: u32 = 0x0040;
/// `NOTE_TRUNCATE`: vnode was truncated.
pub const NOTE_TRUNCATE: u32 = 0x0080;
/// `NOTE_EXIT`: process exited.
pub const NOTE_EXIT: u32 = 0x8000_0000;
/// `NOTE_FORK`: process forked.
pub const NOTE_FORK: u32 = 0x4000_0000;
/// `NOTE_EXEC`: process exec'd.
pub const NOTE_EXEC: u32 = 0x2000_0000;
/// `NOTE_PCTRLMASK`: mask for hint bits.
pub const NOTE_PCTRLMASK: u32 = 0xf000_0000;
/// `NOTE_PDATAMASK`: mask for pid.
pub const NOTE_PDATAMASK: u32 = 0x000f_ffff;
/// `NOTE_TRACK`: follow across forks.
pub const NOTE_TRACK: u32 = 0x0000_0001;
/// `NOTE_TRACKERR`: could not track child.
pub const NOTE_TRACKERR: u32 = 0x0000_0002;
/// `NOTE_CHILD`: am a child process.
pub const NOTE_CHILD: u32 = 0x0000_0004;
/// `NOTE_CHANGE`: device change event.
pub const NOTE_CHANGE: u32 = 0x0000_0001;
/// `NOTE_MSECONDS`: data is milliseconds.
pub const NOTE_MSECONDS: u32 = 0x0000_0000;
/// `NOTE_SECONDS`: data is seconds.
pub const NOTE_SECONDS: u32 = 0x0000_0001;
/// `NOTE_USECONDS`: data is microseconds.
pub const NOTE_USECONDS: u32 = 0x0000_0002;
/// `NOTE_NSECONDS`: data is nanoseconds.
pub const NOTE_NSECONDS: u32 = 0x0000_0003;
/// `NOTE_ABSTIME`: timeout is absolute.
pub const NOTE_ABSTIME: u32 = 0x0000_0010;
/// `NOTE_FFNOP`: ignore input fflags.
pub const NOTE_FFNOP: u32 = 0x0000_0000;
/// `NOTE_FFAND`: AND fflags.
pub const NOTE_FFAND: u32 = 0x4000_0000;
/// `NOTE_FFOR`: OR fflags.
pub const NOTE_FFOR: u32 = 0x8000_0000;
/// `NOTE_FFCOPY`: copy fflags.
pub const NOTE_FFCOPY: u32 = 0xc000_0000;
/// `NOTE_FFCTRLMASK`: masks for operations.
pub const NOTE_FFCTRLMASK: u32 = 0xc000_0000;
/// `NOTE_FFLAGSMASK`.
pub const NOTE_FFLAGSMASK: u32 = 0x00ff_ffff;
/// `NOTE_TRIGGER`: trigger the event.
pub const NOTE_TRIGGER: u32 = 0x0100_0000;

/// `__EV_SELECT`: match behavior of select (kernel only).
pub const __EV_SELECT: u16 = 0x0800;
/// `__EV_POLL`: match behavior of poll (kernel only).
pub const __EV_POLL: u16 = 0x1000;
/// `__EV_HUP`: device or socket disconnected (kernel only).
pub const __EV_HUP: u16 = EV_FLAG1;
/// `EVFILT_MARKER`: placemarker for tailq.
pub const EVFILT_MARKER: i16 = 0xf;

/// `NOTE_SUBMIT`: initial knote submission (hint flag for in-kernel use; must not equal any
/// existing note).
pub const NOTE_SUBMIT: u32 = 0x0100_0000;
/// `KN_HASHSIZE`: buckets of a kqueue's hash of non-descriptor knotes (XXX should be
/// tunable).
pub const KN_HASHSIZE: usize = 64;
/// `NOTE_SIGNAL`: flag indicating hint is a signal. Used by `EVFILT_SIGNAL`, and also shared
/// by `EVFILT_PROC` (all knotes attached to `ps_klist`).
pub const NOTE_SIGNAL: u32 = 0x0800_0000;

/// `FILTEROP_ISFD`: each knote of this filter is associated with a file descriptor (`ident`
/// == file descriptor).
pub const FILTEROP_ISFD: i32 = 0x0000_0001;
/// `FILTEROP_MPSAFE`: the kqueue subsystem can invoke `f_attach`, `f_detach`, `f_modify`
/// and `f_process` without the kernel lock.
pub const FILTEROP_MPSAFE: i32 = 0x0000_0002;
/// The filter is `dead_filtops` (or a copy of it): the C's `kn_fop == &dead_filtops`, a flag
/// no other filter carries (the module's redesign).
pub const FILTEROP_DEAD: i32 = 0x0001_0000;
/// The filter is `badfd_filtops` (or a copy of it): the C's `kn_fop == &badfd_filtops`.
pub const FILTEROP_BADFD: i32 = 0x0002_0000;
/// The filter is `user_filtops` (or a copy of it): the C's `kn_fop == &user_filtops`.
pub const FILTEROP_USER: i32 = 0x0004_0000;

/// `KN_ACTIVE`: event has been triggered.
pub const KN_ACTIVE: i32 = 0x0001;
/// `KN_QUEUED`: event is on queue.
pub const KN_QUEUED: i32 = 0x0002;
/// `KN_DISABLED`: event is disabled.
pub const KN_DISABLED: i32 = 0x0004;
/// `KN_DETACHED`: knote is detached.
pub const KN_DETACHED: i32 = 0x0008;
/// `KN_PROCESSING`: knote is being processed.
pub const KN_PROCESSING: i32 = 0x0010;
/// `KN_WAITING`: waiting on processing.
pub const KN_WAITING: i32 = 0x0020;

/// `KQ_NEVENTS`: minimize copy{in,out} calls.
pub const KQ_NEVENTS: usize = 8;
/// `KQEXTENT`: linear growth by this amount.
pub const KQEXTENT: i32 = 256;
/// `KQ_SLEEP`: a thread sleeps in `kqueue_scan` for events (`kinfo_file.kq_state`, which
/// `fstat(1)` prints as `W`).
pub const KQ_SLEEP: i32 = 0x02;
/// `KQ_DYING`: the kqueue is being closed.
pub const KQ_DYING: i32 = 0x04;
/// `KQ_TASK`: `kq_task` was scheduled.
pub const KQ_TASK: i32 = 0x08;

/// `M_KEVENT`: the `malloc(9)` type of kqueue's data structures.
pub const M_KEVENT: i32 = 137;

/// `PSOCK`: the priority of a sleep for events.
const PSOCK: i32 = 24;
/// `MAXTSLP`: the longest sleep that is not `INFSLP`.
const MAXTSLP: u64 = u64::MAX - 1;
/// `NSIG`: the number of signals, signal 0 included.
const NSIG: usize = 33;
/// One second in nanoseconds (`SEC_TO_NSEC(1)`).
const SEC_NSEC: u64 = 1_000_000_000;

/// `NOTE_TIMER_UNITMASK`: the unit bits of an `EVFILT_TIMER`'s `fflags`.
const NOTE_TIMER_UNITMASK: u32 = NOTE_SECONDS | NOTE_MSECONDS | NOTE_USECONDS | NOTE_NSECONDS;

/// `struct kevent`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Kevent {
    /// `ident`: identifier for this event.
    pub ident: usize,
    /// `filter`: filter for event.
    pub filter: i16,
    /// `flags`: action flags for kqueue.
    pub flags: u16,
    /// `fflags`: filter flag value.
    pub fflags: u32,
    /// `data`: filter data value.
    pub data: i64,
    /// `udata`: opaque user data identifier (a user pointer the kernel never follows).
    pub udata: usize,
}

hal::abi_pod!(Kevent {
    ident: usize,
    filter: i16,
    flags: u16,
    fflags: u32,
    data: i64,
    udata: usize
});

/// `EV_SET(kevp, a, b, c, d, e, f)`.
pub const fn ev_set(
    ident: usize,
    filter: i16,
    flags: u16,
    fflags: u32,
    data: i64,
    udata: usize,
) -> Kevent {
    Kevent {
        ident,
        filter,
        flags,
        fflags,
        data,
        udata,
    }
}

/// `struct timespec` as `kevent(2)` reads it from userland.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct UserTimespec {
    tv_sec: i64,
    tv_nsec: i64,
}

hal::abi_pod!(UserTimespec {
    tv_sec: i64,
    tv_nsec: i64
});

/// The function types of `struct filterops`.
pub type FAttach<G> = fn(&Knote<G>) -> Result<(), Errno>;
/// `f_detach(kn)`.
pub type FDetach<G> = fn(&Knote<G>);
/// `f_event(kn, hint)`: whether the event is active.
pub type FEvent<G> = fn(&Knote<G>, i64) -> bool;
/// `f_modify(kev, kn)`: whether the knote has become active.
pub type FModify<G> = fn(&mut Kevent, &Knote<G>) -> bool;
/// `f_process(kn, kev)`: whether the event should be returned to the user.
pub type FProcess<G> = fn(&Knote<G>, Option<&mut Kevent>) -> bool;

/// `struct filterops`: the event filter interface.
///
/// `f_flags` defines properties of the event filter (`FILTEROP_ISFD`, `FILTEROP_MPSAFE`).
/// `f_attach` attaches the knote to the object. `f_detach` detaches the knote from the
/// object; the object must not use this knote for delivering events after this callback has
/// returned. `f_event` notifies the filter about an event (called through `knote()`).
/// `f_modify` modifies the knote with new state from the user and returns `true` if the
/// knote has become active. `f_process` checks if the event is active and returns `true` if
/// the event should be returned to the user; if `kev` is given and the event is active, the
/// callback should store the event's state in it for delivery to the user.
///
/// Concurrency control: the kqueue subsystem serializes calls of `f_attach`, `f_detach`,
/// `f_modify` and `f_process`.
pub struct Filterops<G: ProcGlue> {
    /// `f_flags`.
    pub f_flags: i32,
    /// `f_attach(kn)`.
    pub f_attach: Option<FAttach<G>>,
    /// `f_detach(kn)`.
    pub f_detach: Option<FDetach<G>>,
    /// `f_event(kn, hint)`.
    pub f_event: Option<FEvent<G>>,
    /// `f_modify(kev, kn)`.
    pub f_modify: Option<FModify<G>>,
    /// `f_process(kn, kev)`.
    pub f_process: Option<FProcess<G>>,
}

/// `kn_kevent`: the knote's `struct kevent`, one word per member: `ident` and `filter` are
/// `[I]`, the others `[o]` (LZ's `get()`/`set()`, relaxed atomics).
pub struct KnKevent {
    /// `ident`.
    pub ident: UsizeWord,
    /// `filter`.
    pub filter: I16Word,
    /// `flags`.
    pub flags: U16Word,
    /// `fflags`.
    pub fflags: U32Word,
    /// `data`.
    pub data: I64Word,
    /// `udata`.
    pub udata: UsizeWord,
}

impl KnKevent {
    /// A zeroed event.
    pub const fn new() -> Self {
        Self {
            ident: UsizeWord::new(0),
            filter: I16Word::new(0),
            flags: U16Word::new(0),
            fflags: U32Word::new(0),
            data: I64Word::new(0),
            udata: UsizeWord::new(0),
        }
    }

    /// `*kev = kn->kn_kevent`.
    pub fn get(&self) -> Kevent {
        Kevent {
            ident: self.ident.get(),
            filter: self.filter.get(),
            flags: self.flags.get(),
            fflags: self.fflags.get(),
            data: self.data.get(),
            udata: self.udata.get(),
        }
    }

    /// `kn->kn_kevent = *kev`.
    pub fn set(&self, kev: &Kevent) {
        self.ident.set(kev.ident);
        self.filter.set(kev.filter);
        self.flags.set(kev.flags);
        self.fflags.set(kev.fflags);
        self.data.set(kev.data);
        self.udata.set(kev.udata);
    }
}

impl Default for KnKevent {
    fn default() -> Self {
        Self::new()
    }
}

/// `kn_hook`: the object, for the filter, as an address the kernel never follows (the
/// filters still in `bsd` cast it back).
#[derive(Default)]
pub struct HookWord(AtomicPtr<c_void>);

impl HookWord {
    /// `NULL`.
    pub const fn new() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
    }

    /// The address.
    pub fn get(&self) -> *mut c_void {
        self.0.load(Ordering::Relaxed)
    }

    /// Stores `p`.
    pub fn set(&self, p: *mut c_void) {
        self.0.store(p, Ordering::Relaxed);
    }
}

/// `pmark`: the work a process klist's walk leaves for after it (the C does it inside the
/// walk, which here lends the list and its lock to every filter call).
const PMARK_TRACK: u8 = 0x01;
/// `filt_proc`'s `NOTE_EXIT`: the knote leaves its process's klist after the walk.
const PMARK_UNLINK: u8 = 0x02;
/// `KN_DETACHED`, mirrored for the readers that hold `kqueue_ps_list_lock` without `kq_lock`
/// (the C: "one of them is enough here").
const PMARK_DETACHED: u8 = 0x04;

/// `struct knote`.
///
/// Locking: \[I\] immutable after creation, \[o\] object lock, \[q\] `kn_kq->kq_lock`.
pub struct Knote<G: ProcGlue> {
    /// `kn_link`: the chain that owns it (a descriptor slot of `kq_knlist` or a bucket of
    /// `kq_knhash`), \[q\].
    pub link: OwnedEntry<Knote<G>>,
    /// `kn_tqe`: the kqueue's pending queue (`kq_head`), \[q\].
    pub tqe: PinEntry<Knote<G>>,
    /// `kn_selnext`: the object's klist, \[o\].
    pub selnext: PinEntry<Knote<G>>,
    /// \[I\] `kn_kq`: which queue we are on (`None` for a scan marker).
    kq: Option<KqRef<G>>,
    /// `kn_kevent`.
    pub kn_kevent: KnKevent,
    /// \[q\] `kn_status`: `KN_*`.
    status: Guarded<i32>,
    /// \[o\] `kn_sfflags`: saved filter flags.
    pub kn_sfflags: U32Word,
    /// \[o\] `kn_sdata`: saved data field.
    pub kn_sdata: I64Word,
    /// `kn_ptr.p_useract`: user event active (`EVFILT_USER`).
    useract: AtomicBool,
    /// `PMARK_*`: a process klist's work after its walk, and `KN_DETACHED`'s mirror.
    pmark: AtomicU8,
    /// `kn_ptr.p_process`: the process an `EVFILT_PROC` or `EVFILT_SIGNAL` knote watches,
    /// counted, under a leaf mutex (set by the attach, cleared by `NOTE_EXIT`).
    process: Mutexed<Option<ProcessRef<G>>>,
    /// \[I\] `kn_pollid`.
    pub kn_pollid: U32Word,
    /// \[o\] `kn_ptr.p_fp`: the file of a `FILTEROP_ISFD` knote, with the reference the knote
    /// holds; a slot under a leaf mutex at `IPL_HIGH`, read or swapped from any filter.
    fp: Mutexed<FileSlot<Files<G>>>,
    /// `kn_fop`: the filter.
    pub kn_fop: Link<Filterops<G>>,
    /// \[o\] `kn_hook`: the object, for the filters that keep its address.
    pub kn_hook: HookWord,
    /// `EVFILT_TIMER`'s timer (`struct filter_timer`, the C's `kn_hook`).
    timer: OnceStatic<PoolArc<FiltTimer<G>>>,
}

/// The lock a [`Knote::detached`]'s status is keyed to (host tests only; never taken).
#[cfg(any(test, feature = "doubles"))]
static DETACHED_KNOTE_LOCK: Mutexed<()> = Mutexed::new(IPL_HIGH, ());

impl<G: ProcGlue> Knote<G> {
    /// A zeroed knote (`PR_ZERO`) whose status `status` keys to its lock, of kqueue `kq`.
    fn zeroed(status: Guarded<i32>, kq: Option<KqRef<G>>) -> Self {
        Self {
            link: OwnedEntry::new(),
            tqe: PinEntry::new(),
            selnext: PinEntry::new(),
            status,
            kq,
            kn_kevent: KnKevent::new(),
            kn_sfflags: U32Word::new(0),
            kn_sdata: I64Word::new(0),
            useract: AtomicBool::new(false),
            pmark: AtomicU8::new(0),
            process: Mutexed::new(IPL_HIGH, None),
            kn_pollid: U32Word::new(0),
            fp: Mutexed::new(IPL_HIGH, FileSlot::new()),
            kn_fop: Link::new(),
            kn_hook: HookWord::new(),
            timer: OnceStatic::new(),
        }
    }

    /// A knote of `kq`, as `pool_get(&knote_pool, PR_ZERO)` and `kqueue_register` make one.
    fn new(kq: &KqRef<G>) -> Self {
        Self::zeroed(Guarded::new_mutexed(&kq.lock, 0), Some(kq.clone()))
    }

    /// A scan marker of `kq` (`kqueue_scan_setup`'s `kqs_start`/`kqs_end`): `EVFILT_MARKER`,
    /// `KN_PROCESSING`, no kqueue, to be pinned on the scanning thread's stack.
    pub fn marker(kq: &Kqueue<G>) -> Self {
        let kn = Self::zeroed(Guarded::new_mutexed(&kq.lock, KN_PROCESSING), None);
        kn.kn_kevent.filter.set(EVFILT_MARKER);
        kn
    }

    /// A knote of no kqueue, never attached nor linked, for host tests of a filter's event
    /// function alone (what it reads of `kn_sfflags`/`kn_sdata` and writes into the kevent).
    #[cfg(any(test, feature = "doubles"))]
    pub fn detached() -> Self {
        Self::zeroed(Guarded::new_static_mutexed(&DETACHED_KNOTE_LOCK, 0), None)
    }

    /// `kn_id`: `kn_kevent.ident` \[I\].
    pub fn kn_id(&self) -> &UsizeWord {
        &self.kn_kevent.ident
    }

    /// `kn_filter`: `kn_kevent.filter` \[I\].
    pub fn kn_filter(&self) -> &I16Word {
        &self.kn_kevent.filter
    }

    /// `kn_flags`: `kn_kevent.flags` \[o\].
    pub fn kn_flags(&self) -> &U16Word {
        &self.kn_kevent.flags
    }

    /// `kn_fflags`: `kn_kevent.fflags` \[o\].
    pub fn kn_fflags(&self) -> &U32Word {
        &self.kn_kevent.fflags
    }

    /// `kn_data`: `kn_kevent.data` \[o\].
    pub fn kn_data(&self) -> &I64Word {
        &self.kn_kevent.data
    }

    /// `kn_udata`: `kn_kevent.udata` \[o\].
    pub fn kn_udata(&self) -> &UsizeWord {
        &self.kn_kevent.udata
    }

    /// `kn_fp`: `kn_ptr.p_fp` \[o\], its slot.
    pub fn kn_fp(&self) -> &Mutexed<FileSlot<Files<G>>> {
        &self.fp
    }

    /// `kn->kn_fp`, which every attached `FILTEROP_ISFD` knote has: a reference of the
    /// caller's (the knote keeps its own until it is dropped).
    pub fn fp(&self) -> FileRef<Files<G>> {
        let fp = self.fp.lock().get().cloned();
        match fp {
            Some(fp) => fp,
            None => kpanic(format_args!("knote {:p}: no kn_fp", self)),
        }
    }

    /// `kn->kn_fop`, which every knote but a scan marker has.
    pub fn fop(&self) -> &'static Filterops<G> {
        match self.kn_fop.get() {
            Some(fop) => fop,
            None => kpanic(format_args!("knote {:p}: no kn_fop", self)),
        }
    }

    /// `kn->kn_kq`, which every knote but a scan marker has.
    pub fn kq(&self) -> &Kqueue<G> {
        self.kq_ref()
    }

    /// `kn->kn_kq`, counted.
    pub fn kq_ref(&self) -> &KqRef<G> {
        match &self.kq {
            Some(kq) => kq,
            None => kpanic(format_args!("knote {:p}: no kn_kq", self)),
        }
    }

    /// `kn->kn_status`, for the holder of its kqueue's lock.
    pub fn status<W: Witness + ?Sized>(&self, w: &W) -> i32 {
        *self.status.get(w)
    }

    /// `kn->kn_flags |= flags`.
    pub fn set_flags(&self, flags: u16) {
        self.kn_flags().set(self.kn_flags().get() | flags);
    }

    /// `kn->kn_flags &= ~flags`.
    pub fn clear_flags(&self, flags: u16) {
        self.kn_flags().set(self.kn_flags().get() & !flags);
    }

    /// `kn->kn_flags & flags`, as a test.
    pub fn has_flags(&self, flags: u16) -> bool {
        self.kn_flags().get() & flags != 0
    }

    /// The knote, pinned: a knote its chain owns is a pool item that never moves (the
    /// pending queue and the object's klist link it by address).
    pub fn pinned(&self) -> Pin<&Self> {
        match hal::owned::pinned::<G::KnLink>(self) {
            Some(p) => p,
            None => kpanic(format_args!("knote {:p}: owned by no kqueue", self)),
        }
    }

    /// `kn->kn_ptr.p_process`, counted: the process an attached, not detached `EVFILT_PROC`
    /// or `EVFILT_SIGNAL` knote watches.
    fn process(&self) -> ProcessRef<G> {
        let pr = self.process.lock().clone();
        match pr {
            Some(pr) => pr,
            None => kpanic(format_args!("knote {:p}: no process", self)),
        }
    }

    /// Whether the knote's filter carries `flag` (`FILTEROP_DEAD`, ...).
    fn fop_is(&self, flag: i32) -> bool {
        self.kn_fop.get().is_some_and(|f| f.f_flags & flag != 0)
    }

    /// `kn->kn_status`, mutably, for the holder of its kqueue's lock.
    fn status_mut<'a, W: hal::rwlock::Exclusive + ?Sized>(&'a self, w: &'a mut W) -> &'a mut i32 {
        self.status.get_mut(w)
    }
}

/// A knote held by a counted reference across the unlocked part of its processing
/// (`KN_PROCESSING`): its chain cannot give it back while the reference lives, and
/// `knote_drop` takes it back through it (`hal::owned::OwnedList::take`).
pub struct KnHold<G: ProcGlue>(SecondaryRef<G::KnLink>);

impl<G: ProcGlue> KnHold<G> {
    /// A reference to `kn`, which its chain owns.
    fn of(kn: &Knote<G>) -> Self {
        let mut r = SecondaryRef::new();
        r.set(Some(kn));
        Self(r)
    }

    /// The knote.
    pub fn kn(&self) -> &Knote<G> {
        match self.0.get() {
            Some(kn) => kn,
            None => kpanic(format_args!("KnHold: no knote")),
        }
    }
}

/// How a klist is locked (`kl_ops`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KlistKind {
    /// `kl_ops == NULL`: the kernel lock (at `splhigh` for the forms that lock).
    Kernel,
    /// A lock its owner holds and passes as a witness (`ps_mtx`, `kqueue_klist_lock`): only
    /// the `_locked` forms.
    Owner,
    /// The klist's own `IPL_HIGH` mutex, for an owner whose lock the kernel cannot witness
    /// (`klist_init_mutex`/`klist_init_rwlock` of a `bsd` object), taken inside that lock.
    Own,
}

/// `KlistKind::Kernel`'s byte (all-zero: a zeroed klist is the kernel lock's).
const KIND_KERNEL: u8 = 0;
/// `KlistKind::Owner`'s byte.
const KIND_OWNER: u8 = 1;
/// `KlistKind::Own`'s byte.
const KIND_OWN: u8 = 2;

/// `struct klist`: the knotes an object delivers events to, and how to lock them. All zero
/// is an empty klist of the kernel lock (`M_ZERO` softcs, `PR_ZERO` sockets embed one).
pub struct Klist<G: ProcGlue> {
    /// `kl_list`, through `kn_selnext`.
    list: PinList<G::KnSelnext>,
    /// The own mutex of a [`KlistKind::Own`] klist, made by [`Klist::init_own`].
    own: OnceStatic<SharedMutex>,
    /// `KIND_*`.
    kind: AtomicU8,
    /// The knotes on the list, for `klist_empty`'s unlocked read.
    n: AtomicU32,
}

/// Holds the kernel lock for its life (`KERNEL_LOCK()` ... `KERNEL_UNLOCK()`).
struct KernelHold;

impl KernelHold {
    fn take() -> Self {
        klock::kernel_lock();
        Self
    }

    /// The kernel lock of a `bsd` caller of a `_locked` form, which holds it (asserted under
    /// `diagnostic`, as the C's `KLIST_ASSERT_LOCKED`), taken once more, recursively, so the
    /// witness the form takes is this CPU's whatever the caller's path.
    fn legacy() -> Self {
        #[cfg(feature = "diagnostic")]
        klock::kernel_assert_locked();
        Self::take()
    }
}

impl Drop for KernelHold {
    fn drop(&mut self) {
        klock::kernel_unlock();
    }
}

impl<G: ProcGlue> Klist<G> {
    /// An empty klist of the kernel lock (`kl_ops == NULL`), all zero: a zeroed object
    /// (`M_ZERO`) that embeds one holds this klist (every field's zero is its `new()`: the
    /// atomics, and hal's `PinList` and `OnceStatic`, whose tests check it).
    pub const fn new() -> Self {
        Self {
            list: PinList::new(),
            own: OnceStatic::new(),
            kind: AtomicU8::new(KIND_KERNEL),
            n: AtomicU32::new(0),
        }
    }

    /// An empty klist of its owner's lock ([`KlistKind::Owner`]).
    pub const fn owner() -> Self {
        Self {
            list: PinList::new(),
            own: OnceStatic::new(),
            kind: AtomicU8::new(KIND_OWNER),
            n: AtomicU32::new(0),
        }
    }

    /// `klist_init_mutex(klist, mtx)`, `klist_init_rwlock(klist, rwl)`: the klist gets its
    /// own mutex ([`KlistKind::Own`]), before its first use; the owner's lock is no longer
    /// named (the owner keeps holding it around its `_locked` calls).
    pub fn init_own(&self) {
        let _ = self.own.set(Mutexed::new(IPL_HIGH, ()));
        self.kind.store(KIND_OWN, Ordering::Release);
    }

    /// How the klist is locked.
    pub fn kind(&self) -> KlistKind {
        match self.kind.load(Ordering::Acquire) {
            KIND_OWNER => KlistKind::Owner,
            KIND_OWN => KlistKind::Own,
            _ => KlistKind::Kernel,
        }
    }

    /// `klist_empty(klist)`, read without the list's lock, as the C reads it.
    pub fn is_empty(&self) -> bool {
        self.n.load(Ordering::Relaxed) == 0
    }

    /// The own mutex of an `Own` klist.
    fn own(&self) -> &SharedMutex {
        match self.own.get() {
            Some(m) => m,
            None => kpanic(format_args!("klist {:p}: no own mutex", self)),
        }
    }

    /// `panic(9)` for a form of the wrong kind.
    fn wrong_kind(&self, what: &str) -> ! {
        kpanic(format_args!(
            "{what}: klist {:p} is locked by its owner ({:?})",
            self,
            self.kind()
        ))
    }

    /// `klist_insert_locked(klist, kn)`: hooks a knote on the list, with the witness of its
    /// lock (`KLIST_ASSERT_LOCKED`: the witness itself).
    pub fn insert_locked<W: Divisible + ?Sized>(&self, w: &mut W, kn: &Knote<G>) {
        self.list.insert_head(w, kn.pinned());
        self.n.fetch_add(1, Ordering::Relaxed);
    }

    /// `klist_remove_locked(klist, kn)`: unhooks a knote, with the witness of its lock.
    pub fn remove_locked<W: Divisible + ?Sized>(&self, w: &mut W, kn: &Knote<G>) {
        self.list.remove(w, kn);
        self.n.fetch_sub(1, Ordering::Relaxed);
    }

    /// `klist_insert(klist, kn)`: hooks a knote, taking the list's lock.
    pub fn insert(&self, kn: &Knote<G>) {
        match self.kind() {
            KlistKind::Kernel => {
                let _k = KernelHold::take();
                let mut w = klock::witness();
                let _s = hal::intr::splhigh();
                self.insert_locked(&mut w, kn);
            }
            KlistKind::Own => {
                let mut g = self.own().lock();
                self.insert_locked(&mut g, kn);
            }
            KlistKind::Owner => self.wrong_kind("klist_insert"),
        }
    }

    /// `klist_remove(klist, kn)`: unhooks a knote, taking the list's lock.
    pub fn remove(&self, kn: &Knote<G>) {
        match self.kind() {
            KlistKind::Kernel => {
                let _k = KernelHold::take();
                let mut w = klock::witness();
                let _s = hal::intr::splhigh();
                self.remove_locked(&mut w, kn);
            }
            KlistKind::Own => {
                let mut g = self.own().lock();
                self.remove_locked(&mut g, kn);
            }
            KlistKind::Owner => self.wrong_kind("klist_remove"),
        }
    }

    /// `klist_insert_locked(klist, kn)` of a `bsd` caller that holds the list's lock as the
    /// C knows it: the kernel lock (asserted under `diagnostic`, the C's
    /// `KLIST_ASSERT_LOCKED`, and taken once more, recursively, for the witness), or, for an
    /// `Own` klist, the owner's lock, inside which the own mutex is taken.
    pub fn insert_legacy(&self, kn: &Knote<G>) {
        match self.kind() {
            KlistKind::Kernel => {
                let _k = KernelHold::legacy();
                let mut w = klock::witness();
                self.insert_locked(&mut w, kn);
            }
            KlistKind::Own => {
                let mut g = self.own().lock();
                self.insert_locked(&mut g, kn);
            }
            KlistKind::Owner => self.wrong_kind("klist_insert_locked"),
        }
    }

    /// `klist_remove_locked(klist, kn)` of a `bsd` caller (as [`Klist::insert_legacy`]).
    pub fn remove_legacy(&self, kn: &Knote<G>) {
        match self.kind() {
            KlistKind::Kernel => {
                let _k = KernelHold::legacy();
                let mut w = klock::witness();
                self.remove_locked(&mut w, kn);
            }
            KlistKind::Own => {
                let mut g = self.own().lock();
                self.remove_locked(&mut g, kn);
            }
            KlistKind::Owner => self.wrong_kind("klist_remove_locked"),
        }
    }

    /// `knote_locked(list, hint)`: walk down the list, activating the knotes whose event has
    /// triggered, with a witness of its lock.
    pub fn knote_locked<W: Witness + ?Sized>(&self, w: &W, hint: i64) {
        for kn in self.list.iter(w) {
            if filter_event(kn, hint) {
                let kq = kn.kq();
                let mut g = kq.lock.lock();
                knote_activate(kq, &mut g, kn);
            }
        }
    }

    /// `knote(list, hint)`: as [`Klist::knote_locked`], taking the list's lock.
    pub fn knote(&self, hint: i64) {
        match self.kind() {
            KlistKind::Kernel => {
                let _k = KernelHold::take();
                let _s = hal::intr::splhigh();
                let r = klock::reader();
                self.knote_locked(&r, hint);
            }
            KlistKind::Own => {
                let g = self.own().lock();
                self.knote_locked(&g, hint);
            }
            KlistKind::Owner => self.wrong_kind("knote"),
        }
    }

    /// `knote_locked(list, hint)` of a `bsd` caller that holds the list's lock as the C
    /// knows it (as [`Klist::insert_legacy`]; the kernel lock's walk needs its reader only).
    pub fn knote_legacy(&self, hint: i64) {
        match self.kind() {
            KlistKind::Kernel => {
                let _k = KernelHold::legacy();
                let r = klock::reader();
                self.knote_locked(&r, hint);
            }
            KlistKind::Own => {
                let g = self.own().lock();
                self.knote_locked(&g, hint);
            }
            KlistKind::Owner => self.wrong_kind("knote_locked"),
        }
    }

    /// `klist_free(klist)`: the list must be empty.
    pub fn free(&self) {
        if !self.is_empty() {
            kpanic(format_args!("klist_free: klist {:p} is not empty", self));
        }
    }

    /// The first knote on the list, for the holder of its lock.
    pub fn first<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a Knote<G>> {
        self.list.first(w)
    }

    /// `klist_invalidate(list)` of a klist the kernel can lock (`Kernel`, `Own`): detach
    /// all knotes from the list; the descriptor knotes are rewired to indicate EOF. The
    /// caller must not hold any lock that can block the filters' callbacks that run with
    /// `KN_PROCESSING`. The kernel lock is held across (it is the C's klist lock for a
    /// `Kernel` klist, taken again at each step there).
    pub fn invalidate(&self) {
        match self.kind() {
            KlistKind::Kernel => {
                let _k = KernelHold::take();
                self.invalidate_with(klock::witness);
            }
            KlistKind::Own => {
                let m = self.own();
                self.invalidate_with(|| m.lock());
            }
            KlistKind::Owner => self.wrong_kind("klist_invalidate"),
        }
    }

    /// `klist_invalidate(list)` with `lock` taking the list's lock and returning its witness
    /// (an `Owner` klist: `ps_klist` under `ps_mtx`).
    pub fn invalidate_with<W: Divisible>(&self, mut lock: impl FnMut() -> W) {
        let p = G::curproc();

        kassert!(!hal::netlock::net_lock_write_held());

        loop {
            let w = lock();
            let Some(kn) = self.list.first(&w) else {
                return;
            };
            let kq = kn.kq_ref().clone();
            let mut g = kq.lock.lock();
            let h = match knote_try_acquire(&mut g, KnHold::of(kn)) {
                Ok(h) => h,
                Err(chan) => {
                    // The kqueue's lock, then the list's, released before the sleep.
                    drop(g);
                    drop(w);
                    // XXX Timeout resolves potential loss of wakeup.
                    let _ = hal::sleep::tsleep_nsec(chan, 0, "kqepts", SEC_NSEC);
                    continue;
                }
            };
            drop(g);
            drop(w);
            filter_detach(h.kn());
            if h.kn().fop().f_flags & FILTEROP_ISFD != 0 {
                h.kn().kn_fop.set(Some(&Kqueue::<G>::DEAD_FILTOPS));
                filter_event(h.kn(), 0);
                let mut g = kq.lock.lock();
                knote_activate(&kq, &mut g, h.kn());
                knote_release(&mut g, h);
            } else {
                knote_drop(&kq, h, p);
            }
        }
    }
}

impl<G: ProcGlue> Default for Klist<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// \[q\] What `kq_lock` keeps of a kqueue besides its pending queue: the chains that own its
/// knotes.
pub struct KqState<G: ProcGlue> {
    /// `kq_nknotes`: # of registered knotes.
    nknotes: u32,
    /// `kq_knlistsize`: size of `kq_knlist`.
    knlistsize: i32,
    /// `kq_knlist`: list of attached knotes, indexed by descriptor.
    knlist: Option<MArray<OwnedList<G::KnLink>>>,
    /// `kq_knhashmask`: size of `kq_knhash`, minus one.
    knhashmask: u64,
    /// `kq_knhash`: hash table for attached knotes.
    knhash: Option<MArray<OwnedList<G::KnLink>>>,
}

impl<G: ProcGlue> KqState<G> {
    const fn new() -> Self {
        Self {
            nknotes: 0,
            knlistsize: 0,
            knlist: None,
            knhashmask: 0,
            knhash: None,
        }
    }

    /// The chain of `(isfd, ident)`: a descriptor's slot or a hash bucket; `None` past the
    /// table, or before the hash exists.
    fn slot(&self, isfd: bool, ident: usize) -> Option<&OwnedList<G::KnLink>> {
        if isfd {
            self.knlist.as_ref()?.get(ident)
        } else {
            let mask = self.knhashmask;
            if mask == 0 {
                return None;
            }
            self.knhash
                .as_ref()?
                .get(kn_hash(ident as u64, mask) as usize)
        }
    }

    /// [`KqState::slot`], mutably.
    fn slot_mut(&mut self, isfd: bool, ident: usize) -> Option<&mut OwnedList<G::KnLink>> {
        if isfd {
            self.knlist.as_mut()?.get_mut(ident)
        } else {
            let mask = self.knhashmask;
            if mask == 0 {
                return None;
            }
            self.knhash
                .as_mut()?
                .get_mut(kn_hash(ident as u64, mask) as usize)
        }
    }
}

/// `kq_lock` held: the guard of a kqueue's state, the witness of its pending queue and of
/// its knotes' `kn_status`.
type KqGuard<'a, G> = MutexedGuard<'a, KqState<G>>;

/// `struct kqueue`.
///
/// Locking: \[I\] immutable after creation, \[L\] `kqueue_klist_lock`, \[a\] atomic
/// operations, \[q\] `kq_lock`.
pub struct Kqueue<G: ProcGlue> {
    /// `kq_lock`: lock for queue access, and what it keeps.
    lock: Mutexed<KqState<G>>,
    /// \[q\] `kq_head`: list of pending events.
    head: PinList<G::KnTqe>,
    /// \[q\] `kq_count`: # of pending events (written under `kq_lock`, read without it by
    /// `kqueue_stat` and `kinfo_file`, as the C).
    count: AtomicI32,
    /// \[q\] `kq_state`: `KQ_*` (read without the lock by `kinfo_file`).
    state: AtomicI32,
    /// \[L\] `kq_klist`: knotes of other kqueues.
    klist: Klist<G>,
    /// \[I\] `kq_fdp`: fd table of this kq.
    fdp: FdRef<Files<G>>,
    /// `kq_next`: the link in the table's `fd_kqlist`.
    pub next: PinEntry<Kqueue<G>>,
    /// `kq_task`: deferring of activation (a `malloc(M_KEVENT)` block, pinned where the task
    /// queue links it).
    task: MBox<Task>,
    home: PoolHome<Kqueue<G>>,
}

hal::pool_home!(impl[G: ProcGlue] Kqueue<G>, home);

impl<G: ProcGlue> Kqueue<G> {
    /// `kq_count`, read without `kq_lock` (`kqueue_stat`'s `st_size`, `kinfo_file`).
    pub fn count(&self) -> i32 {
        self.count.load(Ordering::Relaxed)
    }

    /// `kq_state`, read without `kq_lock` (`kinfo_file`).
    pub fn state(&self) -> i32 {
        self.state.load(Ordering::Relaxed)
    }

    /// `kq_knotes`, read under `kq_lock`.
    pub fn nknotes(&self) -> u32 {
        self.lock.lock().nknotes
    }

    /// `kq_klist`.
    pub fn klist(&self) -> &Klist<G> {
        &self.klist
    }

    /// `kq_fdp`.
    pub fn fdp(&self) -> &FdRef<Files<G>> {
        &self.fdp
    }

    /// The kqueue's address (`kinfo_file.f_data`).
    pub fn addr(&self) -> usize {
        ptr::from_ref(self).addr()
    }

    /// `kq_state |= bits`, under `kq_lock`.
    fn state_or(&self, _g: &KqGuard<'_, G>, bits: i32) {
        self.state.fetch_or(bits, Ordering::Relaxed);
    }

    /// `kq_state &= ~bits`, under `kq_lock`.
    fn state_clear(&self, _g: &KqGuard<'_, G>, bits: i32) {
        self.state.fetch_and(!bits, Ordering::Relaxed);
    }

    /// `kq_count += n`, under `kq_lock`.
    fn count_add(&self, _g: &KqGuard<'_, G>, n: i32) {
        self.count.fetch_add(n, Ordering::Relaxed);
    }

    /// `kqread_filtops`: `EVFILT_READ` on a kqueue.
    pub const KQREAD_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
        f_attach: None,
        f_detach: Some(filt_kqdetach::<G>),
        f_event: Some(filt_kqueue::<G>),
        f_modify: Some(filt_kqueuemodify::<G>),
        f_process: Some(filt_kqueueprocess::<G>),
    };

    /// `proc_filtops`: `EVFILT_PROC`.
    pub const PROC_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_MPSAFE,
        f_attach: Some(filt_procattach::<G>),
        f_detach: Some(filt_procdetach::<G>),
        f_event: Some(filt_proc::<G>),
        f_modify: Some(filt_procmodify::<G>),
        f_process: Some(filt_procprocess::<G>),
    };

    /// `sig_filtops`: `EVFILT_SIGNAL`.
    pub const SIG_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_MPSAFE,
        f_attach: Some(filt_sigattach::<G>),
        f_detach: Some(filt_sigdetach::<G>),
        f_event: Some(filt_signal::<G>),
        f_modify: Some(filt_procmodify::<G>),
        f_process: Some(filt_procprocess::<G>),
    };

    /// `file_filtops`: the descriptor filters, which the file's `fo_kqfilter` replaces.
    pub const FILE_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
        f_attach: Some(filt_fileattach::<G>),
        f_detach: None,
        f_event: None,
        f_modify: None,
        f_process: None,
    };

    /// `timer_filtops`: `EVFILT_TIMER`.
    pub const TIMER_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_MPSAFE,
        f_attach: Some(filt_timerattach::<G>),
        f_detach: Some(filt_timerdetach::<G>),
        f_event: None,
        f_modify: Some(filt_timermodify::<G>),
        f_process: Some(filt_timerprocess::<G>),
    };

    /// `user_filtops`: `EVFILT_USER`.
    pub const USER_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_MPSAFE | FILTEROP_USER,
        f_attach: Some(filt_userattach::<G>),
        f_detach: Some(filt_userdetach::<G>),
        f_event: None,
        f_modify: Some(filt_usermodify::<G>),
        f_process: Some(filt_userprocess::<G>),
    };

    /// `seltrue_filtops`: the full kqfilter entry for device switch tables, which has the
    /// same effect as a filter using `filt_seltrue()` as filter method.
    pub const SELTRUE_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
        f_attach: None,
        f_detach: Some(filt_seltruedetach::<G>),
        f_event: Some(filt_seltrue::<G>),
        f_modify: Some(filt_seltruemodify::<G>),
        f_process: Some(filt_seltrueprocess::<G>),
    };

    /// `dead_filtops`: the filter of a knote whose object is gone (a revoked vnode, an
    /// invalidated klist).
    pub const DEAD_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE | FILTEROP_DEAD,
        f_attach: None,
        f_detach: Some(filt_deaddetach::<G>),
        f_event: Some(filt_dead::<G>),
        f_modify: Some(filt_seltruemodify::<G>),
        f_process: Some(filt_seltrueprocess::<G>),
    };

    /// `badfd_filtops`: for use with kqpoll, a closed descriptor's knote reporting `EBADF`.
    pub const BADFD_FILTOPS: Filterops<G> = Filterops {
        f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE | FILTEROP_BADFD,
        f_attach: None,
        f_detach: Some(filt_deaddetach::<G>),
        f_event: Some(filt_badfd::<G>),
        f_modify: Some(filt_seltruemodify::<G>),
        f_process: Some(filt_seltrueprocess::<G>),
    };

    /// A [`test_knote`]'s filterops.
    #[cfg(any(test, feature = "doubles"))]
    const TEST_FILTOPS: Filterops<G> = test_filtops();
}

impl<G: ProcGlue> Drop for Kqueue<G> {
    /// `KQRELE`'s last reference: off its table's `fd_kqlist` (under the list's mutex), then
    /// its arrays, its task and its item go back. Its knotes are gone (each holds a
    /// reference) and so are its pending entries.
    fn drop(&mut self) {
        if self.next.is_linked() {
            let fdp = &self.fdp;
            let mut g = fdp.kqlist_mtx.lock();
            fdp.kqlist.remove(&mut g, self);
        }
        let st = self.lock.get_mut();
        kassert!(st.nknotes == 0);
        kassert!(self.count.load(Ordering::Relaxed) == 0);
        self.klist.free();
    }
}

/// A counted reference to a kqueue (`struct kqueue *` with `kq_refcnt`'s count): the file's
/// object, `p_kq`, a scan, a knote, the pending task. The last one frees the kqueue
/// (`KQRELE`).
pub struct KqRef<G: ProcGlue>(PoolArc<Kqueue<G>>);

impl<G: ProcGlue> KqRef<G> {
    /// One more reference to `kq` (`KQREF` of a kqueue a list or a knote lends).
    pub fn from_ref(kq: &Kqueue<G>) -> Option<Self> {
        PoolArc::from_ref(kq).map(Self)
    }

    /// [`KqRef::from_ref`] for a kqueue whose last reference may be gone (its drop under way,
    /// waiting to unlink it): `None` then.
    pub fn try_from_ref(kq: &Kqueue<G>) -> Option<Self> {
        PoolArc::try_from_ref(kq).map(Self)
    }

    /// `kq_refcnt`: the references alive now (a snapshot).
    pub fn refs(&self) -> usize {
        PoolArc::count(&self.0)
    }

    /// The kqueue's address (`kinfo_file.f_data`).
    pub fn addr(&self) -> usize {
        self.0.addr()
    }

    /// Whether two references name the same kqueue.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        PoolArc::ptr_eq(&a.0, &b.0)
    }

    /// The reference with its type erased (a task's argument).
    fn erase(self) -> AnyArc {
        PoolArc::erase(self.0)
    }
}

impl<G: ProcGlue> Clone for KqRef<G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<G: ProcGlue> core::ops::Deref for KqRef<G> {
    type Target = Kqueue<G>;

    fn deref(&self) -> &Kqueue<G> {
        &self.0
    }
}

/// \[t\] What `ft_mtx` keeps of a timer.
struct TimerState<G: ProcGlue> {
    /// `ft_reschedule`.
    reschedule: bool,
    /// The knote the timer belongs to, counted while it is attached (`filt_timerdetach`
    /// gives the count back before the knote can be dropped).
    kn: SecondaryRef<G::KnLink>,
}

/// `struct filter_timer`: an `EVFILT_TIMER` knote's timer, an item of `kqtimerpl` that the
/// knote holds and its timeout holds while it is set (its counted, erased argument).
///
/// Locking: \[I\] immutable after creation, \[t\] `ft_mtx`.
pub struct FiltTimer<G: ProcGlue> {
    /// \[t\] `ft_to` (`malloc(M_KEVENT)`, pinned where the wheel links it).
    to: MBox<Timeout>,
    /// `ft_mtx`, and what it keeps.
    st: Mutexed<TimerState<G>>,
    /// \[I\] The knote's kqueue.
    kq: KqRef<G>,
}

/// `struct kqueue_scan_state`: a scan of `kq`, its markers pinned by its caller
/// ([`Knote::marker`]); the reference it holds goes, and its end marker leaves the queue,
/// when it is dropped (`kqueue_scan_finish`).
pub struct KqueueScanState<'m, G: ProcGlue> {
    /// `kqs_kq`: kqueue of this scan, counted.
    kq: KqRef<G>,
    /// `kqs_start`: start marker.
    start: Pin<&'m Knote<G>>,
    /// `kqs_end`: end marker.
    end: Pin<&'m Knote<G>>,
    /// `kqs_nevent`: number of events collected.
    nevent: Cell<i32>,
    /// `kqs_queued`: if set, end marker is in queue.
    queued: Cell<bool>,
}

impl<G: ProcGlue> KqueueScanState<'_, G> {
    /// `scan->kqs_kq`.
    pub fn kq(&self) -> &KqRef<G> {
        &self.kq
    }
}

impl<G: ProcGlue> Drop for KqueueScanState<'_, G> {
    /// `kqueue_scan_finish(scan)`: unlinks the end marker and drops the scan's reference.
    fn drop(&mut self) {
        let kq = &self.kq;
        {
            let g = kq.lock.lock();
            kassert!(self.start.kn_filter().get() == EVFILT_MARKER);
            kassert!(self.start.status(&g) == KN_PROCESSING);
            kassert!(self.end.kn_filter().get() == EVFILT_MARKER);
            kassert!(self.end.status(&g) == KN_PROCESSING);
        }
        if self.queued.get() {
            self.queued.set(false);
            let mut g = kq.lock.lock();
            kq.head.remove(&mut g, self.end.get_ref());
        }
        // KQRELE: the reference goes with the state.
    }
}

/// \[L\] `kqueue_klist_lock`: the lock of every kqueue's `kq_klist` (at `IPL_MPFLOOR`: a
/// mutex of `IPL_NONE` raises to the floor).
pub static KQUEUE_KLIST_LOCK: SharedMutex = Mutexed::new(IPL_NONE, ());

/// `kqueue_ps_list_lock`: with a process's `ps_mtx`, guards its `ps_klist`.
pub static KQUEUE_PS_LIST_LOCK: RawRwlock = RawRwlock::new("kqpsl");

/// \[a\] `kq_usereventsmax`: per process.
pub static KQ_USEREVENTSMAX: AtomicU32 = AtomicU32::new(1024);

/// The kqueue and knote pools of one glue's processes (`kqueue_pool`, `knote_pool`) and the
/// timers' (`kqtimerpl`, the C's `malloc(M_KEVENT)` of `struct filter_timer`).
pub struct KqPools<G: ProcGlue> {
    /// `kqueue_pool` ("kqueuepl"), recorded: a knote, a list and the pending task count a
    /// lent kqueue again.
    pub kqueuepl: TypedPool<Kqueue<G>>,
    /// `knote_pool` ("knotepl").
    pub knotepl: BoxPool<Knote<G>>,
    /// "kqtimerpl".
    pub kqtimerpl: TypedPool<FiltTimer<G>>,
}

impl<G: ProcGlue> KqPools<G> {
    /// The pools, not made yet (`kqueue_init`).
    pub const fn new() -> Self {
        Self {
            kqueuepl: TypedPool::new(),
            knotepl: BoxPool::new(),
            kqtimerpl: TypedPool::new(),
        }
    }
}

impl<G: ProcGlue> Default for KqPools<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `KN_HASH(val, mask)`.
const fn kn_hash(val: u64, mask: u64) -> u64 {
    (val ^ (val >> 8)) & mask
}

/// The kqueue a kqueue file holds (`fp->f_data`).
fn fp_kqueue<G: ProcGlue>(fp: &File<Files<G>>) -> KqRef<G> {
    match fp.data().and_then(<Files<G> as FileGlue>::kqueue_of) {
        Some(kq) if fp.f_type() == DTYPE_KQUEUE => kq.clone(),
        _ => kpanic(format_args!("file {:#x}: not a kqueue", fp.addr())),
    }
}

/// `kn->kn_fop->f_event(kn, hint)`, which the filters that call it directly have.
fn fop_event<G: ProcGlue>(kn: &Knote<G>, hint: i64) -> bool {
    match kn.fop().f_event {
        Some(f_event) => f_event(kn, hint),
        None => kpanic(format_args!("knote {:p}: no f_event", kn)),
    }
}

/// `kqueue_init`: the kqueue, knote and timer pools of `G`'s processes.
pub fn kqueue_init<G: ProcGlue>() -> Result<(), Errno> {
    let pools = &G::tables().kq;
    let mpfloor = hal::intr::mpfloor();
    pools
        .kqueuepl
        .init_recorded(mpfloor, PR_WAITOK, "kqueuepl")?;
    pools.knotepl.init(mpfloor, PR_WAITOK, "knotepl")?;
    pools.kqtimerpl.init(mpfloor, PR_WAITOK, "kqtimerpl")
}

/// `kqueue_init_percpu`: the knote pool's per-CPU caches (`pool_cache_init`).
pub fn kqueue_init_percpu<G: ProcGlue>() {
    G::tables().kq.knotepl.cache_init();
}

/// `filt_fileattach`: the file's `fo_kqfilter` attaches the knote and picks its filter.
fn filt_fileattach<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    let fp = kn.fp();

    fp.ops().kqfilter(&fp, kn)
}

/// `kqueue_kqfilter`: a kqueue is readable when it has pending events (`fo_kqfilter` of a
/// kqueue file).
pub fn kqueue_kqfilter<G: ProcGlue>(fp: &File<Files<G>>, kn: &Knote<G>) -> Result<(), Errno> {
    let kq = fp_kqueue::<G>(fp);

    if kn.kn_filter().get() != EVFILT_READ {
        return Err(Errno::EINVAL);
    }

    kn.kn_fop.set(Some(&Kqueue::<G>::KQREAD_FILTOPS));
    let mut g = KQUEUE_KLIST_LOCK.lock();
    kq.klist.insert_locked(&mut g, kn);
    Ok(())
}

/// `filt_kqdetach`.
fn filt_kqdetach<G: ProcGlue>(kn: &Knote<G>) {
    let kq = fp_kqueue::<G>(&kn.fp());

    let mut g = KQUEUE_KLIST_LOCK.lock();
    kq.klist.remove_locked(&mut g, kn);
}

/// `filt_kqueue_common`: `kn_data` is the kqueue's pending count (`kq_lock` held).
fn filt_kqueue_common<G: ProcGlue>(kn: &Knote<G>, kq: &Kqueue<G>, _g: &KqGuard<'_, G>) -> bool {
    kn.kn_data().set(i64::from(kq.count()));

    kn.kn_data().get() > 0
}

/// `filt_kqueue`.
fn filt_kqueue<G: ProcGlue>(kn: &Knote<G>, _hint: i64) -> bool {
    let kq = fp_kqueue::<G>(&kn.fp());

    let g = kq.lock.lock();
    filt_kqueue_common(kn, &kq, &g)
}

/// `filt_kqueuemodify`.
fn filt_kqueuemodify<G: ProcGlue>(kev: &mut Kevent, kn: &Knote<G>) -> bool {
    let kq = fp_kqueue::<G>(&kn.fp());

    let g = kq.lock.lock();
    knote_assign(kev, kn);
    filt_kqueue_common(kn, &kq, &g)
}

/// `filt_kqueueprocess`.
fn filt_kqueueprocess<G: ProcGlue>(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
    let kq = fp_kqueue::<G>(&kn.fp());

    let g = kq.lock.lock();
    let active = if kev.is_some() && kn.has_flags(EV_ONESHOT) {
        true
    } else {
        filt_kqueue_common(kn, &kq, &g)
    };
    if active {
        knote_submit(kn, kev);
    }

    active
}

/// `kqueue_ps_list_lock` taken for writing unless this thread holds it already (the C's
/// `rw_status(&kqueue_ps_list_lock) == RW_WRITE` test of a nested registration).
fn ps_list_write() -> Option<hal::rwlock::WriteWitness<'static>> {
    if KQUEUE_PS_LIST_LOCK.status() == RW_WRITE {
        None
    } else {
        Some(KQUEUE_PS_LIST_LOCK.write())
    }
}

/// `filt_procattach`: hooks the knote on the process `kn_id`.
fn filt_procattach<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    // The C reads curproc unconditionally: a registration always runs on a thread, so the
    // pledge check below cannot be skipped by a missing one.
    let Some(cp) = G::curproc() else {
        kpanic(format_args!("filt_procattach: no curproc"));
    };
    if cp.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0
        && cp.p_pledge.get() & PLEDGE_PROC == 0
    {
        return Err(G::pledge_fail(cp, Errno::EPERM, PLEDGE_PROC));
    }

    if kn.kn_id().get() > PID_MAX as usize {
        return Err(Errno::ESRCH);
    }

    let _k = KernelHold::take();
    let pr = {
        let r = klock::reader();
        prfind::<G, _>(&r, kn.kn_id().get() as Pid).map(G::process_ref)
    };
    let Some(pr) = pr else {
        return Err(Errno::ESRCH);
    };

    // exiting processes can't be specified
    if pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING != 0 {
        return Err(Errno::ESRCH);
    }

    let old = kn.process.lock().replace(pr.clone());
    drop(old);
    kn.set_flags(EV_CLEAR); // automatically set

    // internal flag indicating registration done by kernel
    if kn.has_flags(EV_FLAG1) {
        kn.kn_data().set(kn.kn_sdata.get()); // ppid
        kn.kn_fflags().set(NOTE_CHILD);
        kn.clear_flags(EV_FLAG1);
        kassert!(KQUEUE_PS_LIST_LOCK.status() == RW_WRITE);
    }

    // this needs both the ps_mtx and exclusive kqueue_ps_list_lock.
    let _w = ps_list_write();
    let mut g = pr.ps_mtx.lock();
    pr.ps_klist.insert_locked(&mut g, kn);

    Ok(())
}

/// `filt_procdetach`: the knote may be attached to a different process, which may exit,
/// leaving nothing for the knote to be attached to. So when the process exits, the knote
/// is marked as `KN_DETACHED` and also flagged as `EV_ONESHOT` so it will be deleted when
/// read out. However, as part of the knote deletion, this routine is called, so a check is
/// needed to avoid actually performing a detach, because the original process does not
/// exist any more.
fn filt_procdetach<G: ProcGlue>(kn: &Knote<G>) {
    // We set KN_DETACHED with both kqueue_ps_list_lock rwlock and &kq->kq_lock held. One of
    // them is enough here.
    let _w = KQUEUE_PS_LIST_LOCK.write();
    if kn.pmark.load(Ordering::Relaxed) & PMARK_DETACHED == 0 {
        let pr = kn.process();

        let mut g = pr.ps_mtx.lock();
        pr.ps_klist.remove_locked(&mut g, kn);
    }
}

/// `filt_proc`: records the process event the user asked for. `NOTE_EXIT` detaches the
/// knote (it leaves the process's klist after the walk, `knote_processexit`); `NOTE_FORK`
/// with `NOTE_TRACK` asks for a knote on the child: registered after the walk, where the C
/// drops `ps_mtx` inside it, and only then is the knote activated if it has an event
/// (`knote_processfork`), so the child's `NOTE_CHILD` knote is queued first and a failed
/// registration's `NOTE_TRACKERR` is reported, as the C's order.
fn filt_proc<G: ProcGlue>(kn: &Knote<G>, hint: i64) -> bool {
    // mask off extra data
    let event = (hint as u32) & NOTE_PCTRLMASK;

    // if the user is interested in this event, record it.
    if kn.kn_sfflags.get() & event != 0 {
        kn.kn_fflags().set(kn.kn_fflags().get() | event);
    }

    // process is gone, so flag the event as finished and remove it from the process's klist
    if event == NOTE_EXIT {
        let pr = kn.process();

        kassert!(KQUEUE_PS_LIST_LOCK.status() == RW_WRITE);
        {
            let kq = kn.kq();
            let mut g = kq.lock.lock();
            *kn.status_mut(&mut g) |= KN_DETACHED;
            kn.pmark.fetch_or(PMARK_DETACHED, Ordering::Relaxed);
        }

        kn.pmark.fetch_or(PMARK_UNLINK, Ordering::Relaxed);
        kn.set_flags(EV_EOF | EV_ONESHOT);
        kn.kn_data().set(i64::from(w_exitcode(
            pr.ps_xexit.get() as i32,
            pr.ps_xsig.get(),
        )));
        let old = kn.process.lock().take();
        drop(old);
        return true;
    }

    // process forked, and user wants to track the new process, so attach a new knote to it,
    // and immediately report an event with the parent's pid.
    if event == NOTE_FORK && kn.kn_sfflags.get() & NOTE_TRACK != 0 {
        kassert!(KQUEUE_PS_LIST_LOCK.status() == RW_WRITE);
        kn.pmark.fetch_or(PMARK_TRACK, Ordering::Relaxed);
        // The C returns `kn_fflags != 0` after the registration: knote_processfork activates.
        return false;
    }

    kn.kn_fflags().get() != 0
}

/// `filt_proc`'s registration of a knote on the child `pid` of a `NOTE_TRACK` knote, made
/// with `kqueue_ps_list_lock` held and `ps_mtx` released (`knote_processfork`).
fn filt_proc_track<G: ProcGlue>(kn: &Knote<G>, pid: Pid) {
    // register knote with new process.
    let mut kev = Kevent {
        ident: pid as usize,
        filter: kn.kn_filter().get(),
        flags: kn.kn_flags().get() | EV_ADD | EV_ENABLE | EV_FLAG1,
        fflags: kn.kn_sfflags.get(),
        data: kn.kn_id().get() as i64, // parent
        udata: kn.kn_udata().get(),    // preserve udata
    };

    kassert!(KQUEUE_PS_LIST_LOCK.status() == RW_WRITE);
    if kqueue_register(kn.kq_ref(), &mut kev, 0, None).is_err() {
        kn.kn_fflags().set(kn.kn_fflags().get() | NOTE_TRACKERR);
    }
}

/// `filt_procmodify`.
fn filt_procmodify<G: ProcGlue>(kev: &mut Kevent, kn: &Knote<G>) -> bool {
    let _w = KQUEUE_PS_LIST_LOCK.write();
    if kn.pmark.load(Ordering::Relaxed) & PMARK_DETACHED == 0 {
        let pr = kn.process();

        let _g = pr.ps_mtx.lock();
        knote_modify(kev, kn)
    } else {
        knote_assign(kev, kn);
        kn.kn_fflags().get() != 0
    }
}

/// `filt_procprocess`.
fn filt_procprocess<G: ProcGlue>(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
    let _w = ps_list_write();

    if kn.pmark.load(Ordering::Relaxed) & PMARK_DETACHED == 0 {
        let pr = kn.process();

        let _g = pr.ps_mtx.lock();
        knote_process(kn, kev)
    } else {
        let active = kn.kn_fflags().get() != 0;
        if active {
            knote_submit(kn, kev);
        }
        active
    }
}

/// `filt_sigattach`: signal knotes are shared with proc knotes, so we apply a mask to the
/// hint in order to differentiate them from process hints. This could be avoided by using
/// a signal-specific knote list, but probably isn't worth the trouble.
fn filt_sigattach<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    let Some(cp) = G::curproc() else {
        kpanic(format_args!("filt_sigattach: no curproc"));
    };
    let pr = cp.process_ref().clone();

    if kn.kn_id().get() >= NSIG {
        return Err(Errno::EINVAL);
    }

    let old = kn.process.lock().replace(pr.clone());
    drop(old);
    kn.set_flags(EV_CLEAR); // automatically set

    // this needs both the ps_mtx and exclusive kqueue_ps_list_lock.
    let _w = KQUEUE_PS_LIST_LOCK.write();
    let mut g = pr.ps_mtx.lock();
    pr.ps_klist.insert_locked(&mut g, kn);

    Ok(())
}

/// `filt_sigdetach`.
fn filt_sigdetach<G: ProcGlue>(kn: &Knote<G>) {
    let pr = kn.process();

    let _w = KQUEUE_PS_LIST_LOCK.write();
    let mut g = pr.ps_mtx.lock();
    pr.ps_klist.remove_locked(&mut g, kn);
}

/// `filt_signal`: counts the deliveries of signal `kn_id`.
fn filt_signal<G: ProcGlue>(kn: &Knote<G>, hint: i64) -> bool {
    if hint & i64::from(NOTE_SIGNAL) != 0 {
        let hint = hint & !i64::from(NOTE_SIGNAL);

        if kn.kn_id().get() as i64 == hint {
            kn.kn_data().set(kn.kn_data().get() + 1);
        }
    }
    kn.kn_data().get() != 0
}

/// `filt_timervalidate(sfflags, sdata, ts)`: the timer's period, from its data and unit.
fn filt_timervalidate(sfflags: u32, sdata: i64) -> Result<Timespec, Errno> {
    if sfflags & !(NOTE_TIMER_UNITMASK | NOTE_ABSTIME) != 0 {
        return Err(Errno::EINVAL);
    }

    match sfflags & NOTE_TIMER_UNITMASK {
        NOTE_SECONDS => Ok(Timespec::new(sdata, 0)),
        NOTE_MSECONDS => Ok(Timespec::new(sdata / 1000, (sdata % 1000) * 1_000_000)),
        NOTE_USECONDS => Ok(Timespec::new(sdata / 1_000_000, (sdata % 1_000_000) * 1000)),
        NOTE_NSECONDS => Ok(Timespec::new(sdata / 1_000_000_000, sdata % 1_000_000_000)),
        _ => Err(Errno::EINVAL),
    }
}

/// `kn->kn_hook` of an `EVFILT_TIMER` knote: its timer.
fn filt_timer<G: ProcGlue>(kn: &Knote<G>) -> &PoolArc<FiltTimer<G>> {
    match kn.timer.get() {
        Some(ft) => ft,
        None => kpanic(format_args!("knote {:p}: no timer", kn)),
    }
}

/// `filt_timeradd(kn, ts)`: arms the timer for `ts` (relative, or absolute with
/// `NOTE_ABSTIME`); `ft_mtx` held (`reschedule` is its `ft_reschedule`).
fn filt_timeradd<G: ProcGlue>(
    ft: &FiltTimer<G>,
    reschedule: &mut bool,
    kn: &Knote<G>,
    ts: &Timespec,
) {
    *reschedule = !kn.has_flags(EV_ONESHOT) && kn.kn_sfflags.get() & NOTE_ABSTIME == 0;

    if kn.kn_sfflags.get() & NOTE_ABSTIME != 0 {
        let now = clock::nanotime();
        if *ts > now {
            let expiry = ts.sub(now);
            // XXX timeout_abs_ts with CLOCK_REALTIME
            ft.to.as_pin().add(clock::tstohz(&expiry));
        } else {
            // Expire immediately.
            filt_dotimerexpire(ft, reschedule, kn);
        }
        return;
    }

    let mut tticks = clock::tstohz(ts);
    // Remove extra tick from tstohz() if timeout has fired before.
    if ft.to.triggered() {
        tticks -= 1;
    }
    ft.to.as_pin().add(if tticks > 0 { tticks } else { 1 });
}

/// `filt_dotimerexpire(kn)`: counts an expiry, activates the knote and re-arms a periodic
/// timer; `ft_mtx` held.
fn filt_dotimerexpire<G: ProcGlue>(ft: &FiltTimer<G>, reschedule: &mut bool, kn: &Knote<G>) {
    let kq = &ft.kq;

    kn.kn_data().set(kn.kn_data().get() + 1);

    {
        let mut g = kq.lock.lock();
        knote_activate(kq, &mut g, kn);
    }

    if *reschedule {
        // The period was validated by filt_timerattach or filt_timermodify.
        if let Ok(ts) = filt_timervalidate(kn.kn_sfflags.get(), kn.kn_sdata.get()) {
            filt_timeradd(ft, reschedule, kn, &ts);
        }
    }
}

/// `filt_timerexpire(knx)`: the timeout's function; its argument is the timer.
fn filt_timerexpire<G: ProcGlue>(arg: &AnyArc) {
    let Some(ft) = arg.downcast_ref::<FiltTimer<G>>() else {
        kpanic(format_args!("filt_timerexpire: not a timer"));
    };

    let mut st = ft.st.lock();
    let TimerState { reschedule, kn } = &mut *st;
    if let Some(kn) = kn.get() {
        filt_dotimerexpire(ft, reschedule, kn);
    }
}

/// The `EVFILT_TIMER` and `EVFILT_USER` limit of a table: `fd_nuserevents` counted, `ENOMEM`
/// past `kq_usereventsmax` (the count given back).
fn userevents_take<G: ProcGlue>(kq: &Kqueue<G>) -> Result<(), Errno> {
    let n = &kq.fdp.fd_nuserevents;
    let nuserevents = n.fetch_add(1, Ordering::SeqCst) + 1;
    if nuserevents > KQ_USEREVENTSMAX.load(Ordering::Relaxed) {
        n.fetch_sub(1, Ordering::SeqCst);
        return Err(Errno::ENOMEM);
    }
    Ok(())
}

/// `filt_timerattach`: data contains amount of time to sleep.
fn filt_timerattach<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    let kq = kn.kq_ref();

    let ts = filt_timervalidate(kn.kn_sfflags.get(), kn.kn_sdata.get())?;

    userevents_take(kq)?;

    if kn.kn_sfflags.get() & NOTE_ABSTIME == 0 {
        kn.set_flags(EV_CLEAR); // automatically set
    }

    let made = MBox::new(M_KEVENT, M_WAITOK, Timeout::unset()).and_then(|to| {
        let mut hold = SecondaryRef::new();
        hold.set(Some(kn));
        G::tables()
            .kq
            .kqtimerpl
            .get_shared(PR_WAITOK, || FiltTimer {
                to,
                st: Mutexed::new(
                    IPL_SOFTCLOCK,
                    TimerState {
                        reschedule: false,
                        kn: hold,
                    },
                ),
                kq: kq.clone(),
            })
    });
    let Some(ft) = made else {
        // M_WAITOK cannot fail in the C (see the module's deviations).
        kq.fdp.fd_nuserevents.fetch_sub(1, Ordering::SeqCst);
        return Err(Errno::ENOMEM);
    };
    ft.to.set_shared(
        filt_timerexpire::<G>,
        PoolArc::erase(ft.clone()),
        KCLOCK_NONE,
        0,
    );
    if kn.timer.set(ft.clone()).is_err() {
        kpanic(format_args!("filt_timerattach: knote {:p} has a timer", kn));
    }

    let mut st = ft.st.lock();
    filt_timeradd(&ft, &mut st.reschedule, kn, &ts);

    Ok(())
}

/// `filt_timerdetach`: stops the timer; its memory goes with the knote.
fn filt_timerdetach<G: ProcGlue>(kn: &Knote<G>) {
    let ft = filt_timer(kn);

    ft.st.lock().reschedule = false;

    ft.to.del_barrier();
    // The timeout's argument and the timer's count on the knote go back: the knote can be
    // dropped, and the timer with it.
    ft.to.clear();
    ft.st.lock().kn.set(None);
    kn.kq().fdp.fd_nuserevents.fetch_sub(1, Ordering::SeqCst);
}

/// `filt_timermodify`: resets the timer to the new period; any pending events are
/// discarded.
fn filt_timermodify<G: ProcGlue>(kev: &mut Kevent, kn: &Knote<G>) -> bool {
    let kq = kn.kq();
    let ft = filt_timer(kn);

    let ts = match filt_timervalidate(kev.fflags, kev.data) {
        Ok(ts) => ts,
        Err(error) => {
            kev.flags |= EV_ERROR;
            kev.data = i64::from(error.as_i32());
            return false;
        }
    };

    // Reset the timer. Any pending events are discarded.
    ft.st.lock().reschedule = false;

    ft.to.del_barrier();

    let mut st = ft.st.lock();
    {
        let mut g = kq.lock.lock();
        if kn.status(&g) & KN_QUEUED != 0 {
            knote_dequeue(kq, &mut g, kn);
        }
        *kn.status_mut(&mut g) &= !KN_ACTIVE;
    }

    kn.kn_data().set(0);
    knote_assign(kev, kn);
    // Reinit timeout to invoke tick adjustment again.
    ft.to.set_shared(
        filt_timerexpire::<G>,
        PoolArc::erase(ft.clone()),
        KCLOCK_NONE,
        0,
    );
    filt_timeradd(ft, &mut st.reschedule, kn, &ts);

    false
}

/// `filt_timerprocess`: active once the timer expired since the last delivery.
fn filt_timerprocess<G: ProcGlue>(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
    let ft = filt_timer(kn);

    let _st = ft.st.lock();
    let active = kn.kn_data().get() != 0;
    if active {
        knote_submit(kn, kev);
    }

    active
}

/// `filt_userattach`.
fn filt_userattach<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    userevents_take(kn.kq())?;

    kn.useract
        .store(kn.kn_sfflags.get() & NOTE_TRIGGER != 0, Ordering::Relaxed);
    kn.kn_fflags().set(kn.kn_sfflags.get() & NOTE_FFLAGSMASK);
    kn.kn_data().set(kn.kn_sdata.get());

    Ok(())
}

/// `filt_userdetach`.
fn filt_userdetach<G: ProcGlue>(kn: &Knote<G>) {
    kn.kq().fdp.fd_nuserevents.fetch_sub(1, Ordering::SeqCst);
}

/// `filt_usermodify`: `NOTE_TRIGGER` activates, the `NOTE_FF*` control combines `fflags`.
fn filt_usermodify<G: ProcGlue>(kev: &mut Kevent, kn: &Knote<G>) -> bool {
    if kev.fflags & NOTE_TRIGGER != 0 {
        kn.useract.store(true, Ordering::Relaxed);
    }

    let ffctrl = kev.fflags & NOTE_FFCTRLMASK;
    let fflags = kev.fflags & NOTE_FFLAGSMASK;
    match ffctrl {
        NOTE_FFNOP => {}
        NOTE_FFAND => kn.kn_fflags().set(kn.kn_fflags().get() & fflags),
        NOTE_FFOR => kn.kn_fflags().set(kn.kn_fflags().get() | fflags),
        NOTE_FFCOPY => kn.kn_fflags().set(fflags),
        _ => {
            // ignored, should not happen
        }
    }

    kn.kn_data().set(kev.data);
    kn.kn_udata().set(kev.udata);

    // Allow clearing of an activated event.
    if kev.flags & EV_CLEAR != 0 {
        kn.useract.store(false, Ordering::Relaxed);
    }

    kn.useract.load(Ordering::Relaxed)
}

/// `filt_userprocess`.
fn filt_userprocess<G: ProcGlue>(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
    let active = kn.useract.load(Ordering::Relaxed);
    if active && let Some(kev) = kev {
        *kev = kn.kn_kevent.get();
        if kn.has_flags(EV_CLEAR) {
            kn.useract.store(false, Ordering::Relaxed);
        }
    }

    active
}

/// `filt_seltrue`: this filter "event" routine simulates `seltrue()`.
pub fn filt_seltrue<G: ProcGlue>(kn: &Knote<G>, _hint: i64) -> bool {
    // We don't know how much data can be read/written, but we know that it *can* be. This
    // is about as good as select/poll does as well.
    kn.kn_data().set(0);
    true
}

/// `filt_seltruemodify`.
pub fn filt_seltruemodify<G: ProcGlue>(kev: &mut Kevent, kn: &Knote<G>) -> bool {
    knote_assign(kev, kn);
    fop_event(kn, 0)
}

/// `filt_seltrueprocess`.
pub fn filt_seltrueprocess<G: ProcGlue>(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
    let active = fop_event(kn, 0);
    if active {
        knote_submit(kn, kev);
    }
    active
}

/// `filt_seltruedetach`: this provides full kqfilter entry for device switch tables, which
/// has same effect as filter using `filt_seltrue()` as filter method.
pub fn filt_seltruedetach<G: ProcGlue>(_kn: &Knote<G>) {
    // Nothing to do
}

/// `seltrue_kqfilter(dev, kn)`: the kqfilter of a device that is always ready.
pub fn seltrue_kqfilter<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ | EVFILT_WRITE => kn.kn_fop.set(Some(&Kqueue::<G>::SELTRUE_FILTOPS)),
        _ => return Err(Errno::EINVAL),
    }

    // Nothing more to do
    Ok(())
}

/// `filt_dead`: the object is gone.
fn filt_dead<G: ProcGlue>(kn: &Knote<G>, _hint: i64) -> bool {
    if kn.kn_filter().get() == EVFILT_EXCEPT {
        // Do not deliver event because there is no out-of-band data. However, let HUP
        // condition pass for poll(2).
        if !kn.has_flags(__EV_POLL) {
            kn.set_flags(EV_DISABLE);
            return false;
        }
    }

    kn.set_flags(EV_EOF | EV_ONESHOT);
    if kn.has_flags(__EV_POLL) {
        kn.set_flags(__EV_HUP);
    }
    kn.kn_data().set(0);
    true
}

/// `filt_deaddetach`.
fn filt_deaddetach<G: ProcGlue>(_kn: &Knote<G>) {
    // Nothing to do
}

/// `filt_badfd`: the descriptor was closed under a poll or select.
fn filt_badfd<G: ProcGlue>(kn: &Knote<G>, _hint: i64) -> bool {
    kn.set_flags(EV_ERROR | EV_ONESHOT);
    kn.kn_data().set(i64::from(Errno::EBADF.as_i32()));
    true
}

/// `filter_attach(kn)`.
fn filter_attach<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    let fop = kn.fop();
    let Some(f_attach) = fop.f_attach else {
        kpanic(format_args!(
            "filter_attach: knote {:p} has no f_attach",
            kn
        ));
    };

    if fop.f_flags & FILTEROP_MPSAFE != 0 {
        f_attach(kn)
    } else {
        let _k = KernelHold::take();
        f_attach(kn)
    }
}

/// `filter_detach(kn)`.
fn filter_detach<G: ProcGlue>(kn: &Knote<G>) {
    let fop = kn.fop();
    let Some(f_detach) = fop.f_detach else {
        kpanic(format_args!(
            "filter_detach: knote {:p} has no f_detach",
            kn
        ));
    };

    if fop.f_flags & FILTEROP_MPSAFE != 0 {
        f_detach(kn);
    } else {
        let _k = KernelHold::take();
        f_detach(kn);
    }
}

/// `filter_event(kn, hint)`.
fn filter_event<G: ProcGlue>(kn: &Knote<G>, hint: i64) -> bool {
    if kn.fop().f_flags & FILTEROP_MPSAFE == 0 {
        klock::kernel_assert_locked();
    }

    fop_event(kn, hint)
}

/// `filter_modify(kev, kn)`.
fn filter_modify<G: ProcGlue>(kev: &mut Kevent, kn: &Knote<G>) -> bool {
    let fop = kn.fop();

    if fop.f_flags & FILTEROP_MPSAFE != 0 {
        match fop.f_modify {
            Some(f_modify) => f_modify(kev, kn),
            None => kpanic(format_args!(
                "filter_modify: knote {:p} has no f_modify",
                kn
            )),
        }
    } else {
        let _k = KernelHold::take();
        match fop.f_modify {
            Some(f_modify) => f_modify(kev, kn),
            None => {
                let _s = hal::intr::splhigh();
                knote_modify(kev, kn)
            }
        }
    }
}

/// `filter_process(kn, kev)`.
fn filter_process<G: ProcGlue>(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
    let fop = kn.fop();

    if fop.f_flags & FILTEROP_MPSAFE != 0 {
        match fop.f_process {
            Some(f_process) => f_process(kn, kev),
            None => kpanic(format_args!(
                "filter_process: knote {:p} has no f_process",
                kn
            )),
        }
    } else {
        let _k = KernelHold::take();
        match fop.f_process {
            Some(f_process) => f_process(kn, kev),
            None => {
                let _s = hal::intr::splhigh();
                knote_process(kn, kev)
            }
        }
    }
}

/// `p->p_kq`: the thread's poll kqueue, which `kqpoll_init` made.
pub fn thread_kq<G: ProcGlue>(p: &Thread<G>) -> &KqRef<G> {
    match p.p_kq.get() {
        Some(kq) => kq,
        None => kpanic(format_args!("thread {}: no p_kq", p.p_tid.get())),
    }
}

/// `p->p_kq == kq`.
fn is_thread_kq<G: ProcGlue>(p: &Thread<G>, kq: &Kqueue<G>) -> bool {
    p.p_kq.get().is_some_and(|k| ptr::eq(&**k, kq))
}

/// `LIST_INSERT_HEAD(&fdp->fd_kqlist, kq, kq_next)`, under the list's mutex (the C's
/// `fdplock`): a new kqueue on its table's list, where `knote_fdclose` finds it.
pub fn fd_kqlist_insert<G: ProcGlue>(kq: &KqRef<G>) {
    let fdp = &kq.fdp;
    let mut g = fdp.kqlist_mtx.lock();
    fdp.kqlist.insert_head(&mut g, kq.0.as_pin());
}

/// `kqpoll_init(num)`: initialize the thread `p` for poll/select system call. `num`
/// indicates the number of serials that the system call may utilize. After this function,
/// the valid range of serials is `p_kq_serial <= x < p_kq_serial + num`.
pub fn kqpoll_init<G: ProcGlue>(p: &Thread<G>, num: u32) -> Result<(), Errno> {
    if p.p_kq.get().is_none() {
        let kq = kqueue_alloc::<G>(p.process().fd_ref())?;
        p.p_kq_serial.set(u64::from(hal::random::arc4random()));
        fd_kqlist_insert(&kq);
        if p.p_kq.set(kq).is_err() {
            kpanic(format_args!("kqpoll_init: p_kq set twice"));
        }
    }

    let serial = p.p_kq_serial.get();
    if serial.wrapping_add(u64::from(num)) < serial {
        // Serial is about to wrap. Clear all attached knotes.
        kqueue_purge(Some(p), thread_kq(p));
        p.p_kq_serial.set(0);
    }
    Ok(())
}

/// `kqpoll_done(num)`: finish poll/select system call. `num` must have the same value that
/// was used with `kqpoll_init()`.
pub fn kqpoll_done<G: ProcGlue>(p: &Thread<G>, num: u32) {
    let kq = thread_kq(p);

    let serial = p.p_kq_serial.get();
    kassert!(serial.wrapping_add(u64::from(num)) >= serial);

    p.p_kq_serial.set(serial.wrapping_add(u64::from(num)));

    // Because of kn_pollid key, a thread can in principle allocate up to O(maxfiles^2)
    // knotes by calling poll(2) repeatedly with suitably varying pollfd arrays. Prevent such
    // a large allocation by clearing knotes eagerly if there are too many of them.
    //
    // A small multiple of kq_knlistsize should give enough margin that eager clearing is
    // infrequent, or does not happen at all, with normal programs. A single pollfd entry can
    // use up to three knotes. Typically there is no significant overlap of fd and events
    // between different entries in the pollfd array.
    let (nknotes, knlistsize) = {
        let g = kq.lock.lock();
        (g.nknotes, g.knlistsize)
    };
    if u64::from(nknotes) > 4 * knlistsize as u64 {
        kqueue_purge(Some(p), kq);
    }
}

/// `kqpoll_exit()`: the thread's poll kqueue goes with the thread: purged and terminated
/// now; its memory when the thread's (the module's deviations).
pub fn kqpoll_exit<G: ProcGlue>(p: &Thread<G>) {
    let Some(kq) = p.p_kq.get() else {
        return;
    };

    kqueue_purge(Some(p), kq);
    kqueue_terminate(Some(p), kq);
    // The C asserts `kq_refcnt == 1` here. A sibling thread's knote_fdclose may hold a
    // counted reference to this kqueue for the moment it walks fd_kqlist (one kqueue at a
    // time, with no table lock held: try_from_ref), so the count can be 2 without anything
    // being wrong; what the C's assertion protects, an empty and terminated queue, is
    // asserted instead.
    kassert!(kq.lock.lock().nknotes == 0);
}

/// `kqueue_alloc(fdp)`: a new kqueue on `fdp` with one reference; `ENOMEM` when the pool
/// or `malloc(9)` gives nothing (see the module's deviations).
pub fn kqueue_alloc<G: ProcGlue>(fdp: &FdRef<Files<G>>) -> Result<KqRef<G>, Errno> {
    let task = MBox::new(M_KEVENT, M_WAITOK, Task::unset()).ok_or(Errno::ENOMEM)?;
    G::tables()
        .kq
        .kqueuepl
        .get_homed(PR_WAITOK, || Kqueue {
            lock: Mutexed::new(IPL_HIGH, KqState::new()),
            head: PinList::new(),
            count: AtomicI32::new(0),
            state: AtomicI32::new(0),
            klist: Klist::owner(),
            fdp: fdp.clone(),
            next: PinEntry::new(),
            task,
            home: PoolHome::new(),
        })
        .map(KqRef)
        .ok_or(Errno::ENOMEM)
}

/// `dokqueue(p, flags, retval)`: a new kqueue at the lowest free descriptor, which it
/// returns.
pub fn dokqueue<G: ProcGlue>(p: &Thread<G>, flags: i32) -> Result<i32, Errno> {
    let fdp = p.fd();

    let cloexec = if flags & O_CLOEXEC != 0 {
        UF_EXCLOSE
    } else {
        0
    };

    let kq = kqueue_alloc::<G>(p.process().fd_ref())?;

    let mut g = fdplock(fdp);
    // out: on an error the kqueue, on no list, goes with its reference.
    let (fp, fd) = crate::descrip::falloc::<G, Files<G>>(p, &mut g)?;
    fp.f_flag.store(
        (FREAD | FWRITE | (flags & FNONBLOCK)) as u32,
        Ordering::SeqCst,
    );
    fd_kqlist_insert(&kq);
    let (ops, data) = <Files<G> as FileGlue>::kqueue_file(kq);
    fp.install(DTYPE_KQUEUE, ops, data);
    fdinsert(&mut g, fd, cloexec, &fp);
    let _ = fp.frele(Some(p));
    drop(g);
    Ok(fd)
}

/// `kqueue(2)`.
pub fn sys_kqueue<G: ProcGlue>(p: &Thread<G>) -> Result<i32, Errno> {
    dokqueue(p, 0)
}

/// `kqueue1(2)`: `kqueue(2)` with `O_CLOEXEC` and `O_NONBLOCK`.
pub fn sys_kqueue1<G: ProcGlue>(p: &Thread<G>, flags: i32) -> Result<i32, Errno> {
    if flags & !(O_CLOEXEC | FNONBLOCK) != 0 {
        return Err(Errno::EINVAL);
    }
    dokqueue(p, flags)
}

/// `kevent(2)`: registers the `nchanges` changes at `changelist`, then collects at most
/// `nevents` pending events into `eventlist`, waiting at most the `struct timespec` at
/// `timeout` (0: for ever); the number of events (or of errors reported) it stored.
pub fn sys_kevent<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    changelist: UserPtr<Kevent>,
    nchanges: i32,
    eventlist: UserPtr<Kevent>,
    nevents: i32,
    timeout: usize,
) -> Result<i32, Errno> {
    let fdp = p.fd();
    let mut kev = [Kevent::default(); KQ_NEVENTS];

    let Some(fp) = fd_getfile(fdp, fd) else {
        return Err(Errno::EBADF);
    };

    let error: Result<i32, Errno> = 'done: {
        if fp.f_type() != DTYPE_KQUEUE {
            break 'done Err(Errno::EBADF);
        }

        let mut tsp: Option<Timespec> = None;
        if timeout != 0 {
            let ts = match UserPtr::<UserTimespec>::new(timeout).copyin() {
                Ok(ts) => ts,
                Err(error) => break 'done Err(error),
            };
            // KTRACE (ktrreltimespec): not configured.
            if ts.tv_sec < 0 || !(0..1_000_000_000).contains(&ts.tv_nsec) {
                break 'done Err(Errno::EINVAL);
            }
            tsp = Some(Timespec::new(ts.tv_sec, ts.tv_nsec));
        }

        let kq = fp_kqueue::<G>(&fp);
        let mut nerrors = 0i32;
        let mut changelist = changelist;
        let mut nchanges = nchanges;
        let mut eventlist = eventlist;
        let mut nevents = nevents;

        while nchanges > 0 {
            let n = (nchanges as usize).min(KQ_NEVENTS);
            if let Err(error) = changelist.copyin_slice(&mut kev[..n]) {
                break 'done Err(error);
            }
            // KTRACE (ktrevent): not configured.
            for kevp in &mut kev[..n] {
                kevp.flags &= !EV_SYSFLAGS;
                let error = kqueue_register(&kq, kevp, 0, Some(p));
                if error.is_err() || kevp.flags & EV_RECEIPT != 0 {
                    if nevents != 0 {
                        kevp.flags = EV_ERROR;
                        kevp.data = error.map_or_else(|e| i64::from(e.as_i32()), |()| 0);
                        let _ = eventlist.copyout(kevp);
                        eventlist = eventlist.checked_add(1).unwrap_or(eventlist);
                        nevents -= 1;
                        nerrors += 1;
                    } else {
                        break 'done error.map(|()| 0);
                    }
                }
            }
            nchanges -= n as i32;
            changelist = changelist.checked_add(n).unwrap_or(changelist);
        }
        if nerrors != 0 {
            break 'done Ok(nerrors);
        }

        let start = core::pin::pin!(Knote::marker(&kq));
        let end = core::pin::pin!(Knote::marker(&kq));
        let scan = kqueue_scan_setup(&kq, start.as_ref(), end.as_ref());
        let _ = fp.frele(Some(p));
        // Collect as many events as we can. The timeout on successive loops is disabled
        // (kqueue_scan() becomes non-blocking).
        let mut total = 0usize;
        let mut error = Ok(());
        loop {
            let n = (nevents.max(0) as usize)
                .saturating_sub(total)
                .min(KQ_NEVENTS);
            if n == 0 {
                break;
            }
            let (ready, e) = kqueue_scan(&scan, n, &mut kev, tsp.as_mut(), p);
            error = e;
            if ready == 0 {
                break;
            }
            error = match eventlist.checked_add(total) {
                Some(u) => u.copyout_slice(&kev[..ready]),
                None => Err(Errno::EFAULT),
            };
            // KTRACE (ktrevent): not configured.
            total += ready;
            if error.is_err() || ready < n {
                break;
            }
        }
        drop(scan);
        return error.map(|()| total as i32);
    };

    // done:
    let _ = fp.frele(Some(p));
    error
}

/// The errno a filter left in `kev->data` with `EV_ERROR`.
fn errno_of(data: i64) -> Errno {
    Errno::from_raw(data as i32).unwrap_or(Errno::EINVAL)
}

/// `kqueue_register(kq, kev, pollid, p)`: applies one change (`EV_ADD`, `EV_DELETE`,
/// `EV_ENABLE`, `EV_DISABLE`, a user trigger) to the kqueue.
pub fn kqueue_register<G: ProcGlue>(
    kq: &KqRef<G>,
    kev: &mut Kevent,
    pollid: u32,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let fdp = &kq.fdp;

    kassert!(pollid == 0 || p.is_some_and(|p| is_thread_kq(p, kq)));

    let fops = if kev.filter < 0 {
        if i32::from(kev.filter) + EVFILT_SYSCOUNT < 0 {
            return Err(Errno::EINVAL);
        }
        sysfilt_ops::<G>((!kev.filter) as usize) // to 0-base index
    } else {
        None
    };

    let Some(fops) = fops else {
        // XXX filter attach routine is responsible for ensuring that the identifier can be
        // attached to it.
        return Err(Errno::EINVAL);
    };
    let isfd = fops.f_flags & FILTEROP_ISFD != 0;

    if isfd {
        // validate descriptor
        if kev.ident > i32::MAX as usize {
            return Err(Errno::EBADF);
        }
    }

    let mut newkn: Option<PoolBox<Knote<G>>> = None;
    if kev.flags & EV_ADD != 0 {
        newkn = Some(
            G::tables()
                .kq
                .knotepl
                .get(PR_WAITOK, || Knote::new(kq))
                .ok_or(Errno::ENOMEM)?,
        );
    }
    let mut fp: Option<FileRef<Files<G>>> = None;

    let error: Result<(), Errno> = 'done: loop {
        // again:
        if isfd {
            let Some(f) = fd_getfile(fdp, kev.ident as i32) else {
                break 'done Err(Errno::EBADF);
            };
            fp = Some(f);
        }
        let mut g = kq.lock.lock();
        if kev.flags & EV_ADD != 0 {
            let e;
            (g, e) = if isfd {
                kqueue_expand_list(kq, g, kev.ident as i32)
            } else {
                kqueue_expand_hash(kq, g)
            };
            if let Err(error) = e {
                break 'done Err(error);
            }
        }
        let found = g.slot(isfd, kev.ident).and_then(|list| {
            list.iter().find(|kn| {
                kev.filter == kn.kn_filter().get()
                    && kev.ident == kn.kn_id().get()
                    && pollid == kn.kn_pollid.get()
            })
        });
        let mut held = None;
        if let Some(found) = found {
            let h = KnHold::of(found);
            match knote_acquire(&mut g, h) {
                Some(h) => held = Some(h),
                None => {
                    // knote_acquire() has slept; the knote may be stale: again.
                    drop(g);
                    if let Some(f) = fp.take() {
                        let _ = f.frele(p);
                    }
                    continue;
                }
            }
        }

        // held now contains the matching knote, or None if no match.
        let h: KnHold<G> = match held {
            None if kev.flags & EV_ADD == 0 => {
                break 'done Err(Errno::ENOENT);
            }
            None => {
                let Some(kn) = newkn.take() else {
                    kpanic(format_args!("kqueue_register: EV_ADD without a new knote"));
                };
                *kn.status_mut(&mut g) = KN_PROCESSING;
                // apply reference count to knote structure, and do not release it at the
                // end of this routine. The slot's old value is empty.
                let old = kn.fp.lock().set(fp.take());
                drop(old);
                kn.kn_fop.set(Some(fops));

                kn.kn_sfflags.set(kev.fflags);
                kn.kn_sdata.set(kev.data);
                kev.fflags = 0;
                kev.data = 0;
                kn.kn_kevent.set(kev);
                kn.kn_pollid.set(pollid);

                let h = knote_attach(&mut g, kn);
                drop(g);

                if let Err(error) = filter_attach(h.kn()) {
                    knote_drop(kq, h, p);
                    break 'done Err(error);
                }

                // If this is a file descriptor filter, check if fd was closed while the knote
                // was being added. knote_fdclose() has missed kn if the function ran before
                // kn appeared in kq_knlist.
                if isfd && fd_checkclosed(fdp, kev.ident as i32, &h.kn().fp()) {
                    // Drop the knote silently without error because another thread might
                    // already have seen it. This corresponds to the insert happening in
                    // full before the close.
                    filter_detach(h.kn());
                    knote_drop(kq, h, p);
                    break 'done Ok(());
                }

                // Check if there is a pending event.
                let active = filter_process(h.kn(), None);
                g = kq.lock.lock();
                if active {
                    knote_activate(kq, &mut g, h.kn());
                }
                h
            }
            Some(h) if kev.flags & EV_ADD != 0 && h.kn().fop_is(FILTEROP_BADFD) => {
                // Nothing expects this badfd knote any longer. Drop it to make room for the
                // new knote and retry.
                kassert!(p.is_some_and(|p| is_thread_kq(p, kq)));
                drop(g);
                filter_detach(h.kn());
                knote_drop(kq, h, p);

                kassert!(fp.is_some());
                if let Some(f) = fp.take() {
                    let _ = f.frele(p);
                }

                continue;
            }
            Some(h)
                if kev.flags & EV_ADD != 0
                    || (kev.flags & EV_DELETE == 0 && h.kn().fop_is(FILTEROP_USER)) =>
            {
                // The user may change some filter values after the initial EV_ADD, but doing
                // so will not reset any filters which have already been triggered (EV_ADD);
                // f_modify allows NOTE_TRIGGER without EV_ADD (EVFILT_USER).
                drop(g);
                let active = filter_modify(kev, h.kn());
                g = kq.lock.lock();
                if active {
                    knote_activate(kq, &mut g, h.kn());
                }
                if kev.flags & EV_ERROR != 0 {
                    // release:
                    knote_release(&mut g, h);
                    break 'done Err(errno_of(kev.data));
                }
                h
            }
            Some(h) if kev.flags & EV_DELETE != 0 => {
                drop(g);
                filter_detach(h.kn());
                knote_drop(kq, h, p);
                break 'done Ok(());
            }
            Some(h) => h,
        };

        if kev.flags & EV_DISABLE != 0 && h.kn().status(&g) & KN_DISABLED == 0 {
            *h.kn().status_mut(&mut g) |= KN_DISABLED;
        }

        if kev.flags & EV_ENABLE != 0 && h.kn().status(&g) & KN_DISABLED != 0 {
            *h.kn().status_mut(&mut g) &= !KN_DISABLED;
            drop(g);
            // Check if there is a pending event.
            let active = filter_process(h.kn(), None);
            g = kq.lock.lock();
            if active {
                knote_activate(kq, &mut g, h.kn());
            }
        }

        // release:
        knote_release(&mut g, h);
        break 'done Ok(());
    };

    // done:
    if let Some(f) = fp {
        let _ = f.frele(p);
    }
    drop(newkn);
    error
}

/// `sysfilt_ops[i]`: the system filter of `~filter`.
fn sysfilt_ops<G: ProcGlue>(i: usize) -> Option<&'static Filterops<G>> {
    match i {
        0 | 1 | 3 | 7 | 8 => Some(&Kqueue::<G>::FILE_FILTOPS), // READ, WRITE, VNODE, DEVICE, EXCEPT
        2 => None,                                             // EVFILT_AIO (&aio_filtops)
        4 => Some(&Kqueue::<G>::PROC_FILTOPS),
        5 => Some(&Kqueue::<G>::SIG_FILTOPS),
        6 => Some(&Kqueue::<G>::TIMER_FILTOPS),
        9 => Some(&Kqueue::<G>::USER_FILTOPS),
        _ => None,
    }
}

/// `timespec_to_nsec(ts)`: the nanoseconds of `ts`, 0 for a negative one, saturated.
fn timespec_to_nsec(ts: &Timespec) -> u64 {
    if ts.tv_sec < 0 {
        return 0;
    }
    (ts.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(ts.tv_nsec as u64))
        .unwrap_or(u64::MAX)
}

/// `kqueue_sleep(kq, tsp)`: sleeps on the kqueue (`kq_lock` held, released), at most `tsp`,
/// which is reduced by the time slept.
fn kqueue_sleep<G: ProcGlue>(
    kq: &Kqueue<G>,
    mut g: KqGuard<'_, G>,
    tsp: Option<&mut Timespec>,
) -> Result<(), Errno> {
    let mut start = Timespec::ZERO;
    let nsecs = match &tsp {
        Some(ts) => {
            start = clock::getnanouptime();
            timespec_to_nsec(ts).min(MAXTSLP)
        }
        None => INFSLP,
    };
    let error = g.sleep_nsec(ptr::from_ref(kq), PSOCK | PCATCH, "kqread", nsecs);
    // PNORELOCK.
    drop(g);
    if let Some(ts) = tsp {
        let stop = clock::getnanouptime();
        let elapsed = stop.sub(start);
        *ts = ts.sub(elapsed);
        if ts.tv_sec < 0 {
            *ts = Timespec::ZERO;
        }
    }

    error
}

/// `kqueue_scan(scan, maxevents, kevp, tsp, p, errorp)`: scan the kqueue, blocking if
/// necessary until the target time is reached. If `tsp` is `None` we block indefinitely.
/// If `tsp`'s seconds and nanoseconds are both 0 we do not block at all. The events stored
/// at the start of `kevp`, and the scan's error.
pub fn kqueue_scan<G: ProcGlue>(
    scan: &KqueueScanState<'_, G>,
    maxevents: usize,
    kevp: &mut [Kevent],
    mut tsp: Option<&mut Timespec>,
    p: &Thread<G>,
) -> (usize, Result<(), Errno>) {
    let kq: &Kqueue<G> = &scan.kq;
    let start: &Knote<G> = &scan.start;
    let end: &Knote<G> = &scan.end;
    let mut nkev = 0usize;

    if maxevents == 0 {
        return (0, Ok(()));
    }
    // retry:
    loop {
        kassert!(nkev == 0);

        let mut reinserted = false;

        let mut g = kq.lock.lock();

        if kq.state() & KQ_DYING != 0 {
            return (0, Err(Errno::EBADF));
        }

        if kq.count() == 0 {
            // Successive loops are only necessary if there are more ready events to gather,
            // so they don't need to block.
            if tsp.as_ref().is_some_and(|ts| **ts == Timespec::ZERO) || scan.nevent.get() != 0 {
                return (0, Ok(()));
            }
            kq.state_or(&g, KQ_SLEEP);
            let error = kqueue_sleep(kq, g, tsp.as_deref_mut());
            // kqueue_sleep() has released kq_lock.
            return match error {
                Ok(()) | Err(Errno::EWOULDBLOCK) => continue,
                // don't restart after signals...
                Err(Errno::ERESTART) => (0, Err(Errno::EINTR)),
                Err(e) => (0, Err(e)),
            };
        }

        // Put the end marker in the queue to limit the scan to the events that are currently
        // active. This prevents events from being recollected if they reactivate during
        // scan.
        //
        // If a partial scan has been performed already but no events have been collected,
        // reposition the end marker to make any new events reachable.
        if !scan.queued.get() {
            kq.head.insert_tail(&mut g, scan.end);
            scan.queued.set(true);
        } else if scan.nevent.get() == 0 {
            kq.head.remove(&mut g, end);
            kq.head.insert_tail(&mut g, scan.end);
        }

        kq.head.insert_head(&mut g, scan.start);
        while nkev < maxevents {
            let Some(kn) = kq.head.next(&g, start) else {
                kpanic(format_args!("kqueue_scan: end marker missing"));
            };
            if kn.kn_filter().get() == EVFILT_MARKER {
                if ptr::eq(kn, end) {
                    break;
                }

                // Move start marker past another thread's marker.
                kq.head.step_over(&mut g, start);
                continue;
            }

            let h = KnHold::of(kn);
            let Some(h) = knote_acquire(&mut g, h) else {
                // knote_acquire() has slept and taken kq_lock again.
                continue;
            };
            let kn = h.kn();

            // kqueue_check(kq): KQUEUE_DEBUG, not configured.
            kq.head.remove(&mut g, kn);
            *kn.status_mut(&mut g) &= !KN_QUEUED;
            kq.count_add(&g, -1);

            if kn.status(&g) & KN_DISABLED != 0 {
                knote_release(&mut g, h);
                continue;
            }

            drop(g);

            // Drop expired kqpoll knotes.
            if is_thread_kq(p, kq) && p.p_kq_serial.get() > kn.kn_udata().get() as u64 {
                filter_detach(kn);
                knote_drop(&scan.kq, h, Some(p));
                g = kq.lock.lock();
                continue;
            }

            // Invalidate knotes whose vnodes have been revoked. This is a workaround; it is
            // tricky to clear existing knotes and prevent new ones from being registered
            // with the current revocation mechanism. The file read under the slot alone (no
            // reference taken: the knote holds one).
            let revoked =
                kn.fop().f_flags & FILTEROP_ISFD != 0
                    && kn.fp.lock().get().is_some_and(|fp| {
                        fp.data().is_some_and(<Files<G> as FileGlue>::vnode_revoked)
                    });
            if revoked && !kn.fop_is(FILTEROP_DEAD) {
                filter_detach(kn);
                kn.kn_fop.set(Some(&Kqueue::<G>::DEAD_FILTOPS));

                // Check if the event should be delivered. Use f_event directly because this
                // is a special situation.
                if !fop_event(kn, 0) {
                    filter_detach(kn);
                    knote_drop(&scan.kq, h, Some(p));
                    g = kq.lock.lock();
                    continue;
                }
            }

            kevp[nkev] = Kevent::default();
            if !filter_process(kn, Some(&mut kevp[nkev])) {
                g = kq.lock.lock();
                if kn.status(&g) & KN_QUEUED == 0 {
                    *kn.status_mut(&mut g) &= !KN_ACTIVE;
                }
                knote_release(&mut g, h);
                continue;
            }

            // Post-event action on the note
            let flags = kevp[nkev].flags;
            if flags & EV_ONESHOT != 0 {
                filter_detach(kn);
                knote_drop(&scan.kq, h, Some(p));
                g = kq.lock.lock();
            } else if flags & (EV_CLEAR | EV_DISPATCH) != 0 {
                g = kq.lock.lock();
                if flags & EV_DISPATCH != 0 {
                    *kn.status_mut(&mut g) |= KN_DISABLED;
                }
                if kn.status(&g) & KN_QUEUED == 0 {
                    *kn.status_mut(&mut g) &= !KN_ACTIVE;
                }
                knote_release(&mut g, h);
            } else {
                g = kq.lock.lock();
                if kn.status(&g) & KN_QUEUED == 0 {
                    kq.count_add(&g, 1);
                    *kn.status_mut(&mut g) |= KN_QUEUED;
                    kq.head.insert_tail(&mut g, kn.pinned());
                    // Wakeup is done after loop.
                    reinserted = true;
                }
                knote_release(&mut g, h);
            }

            nkev += 1;
            scan.nevent.set(scan.nevent.get() + 1);
        }
        kq.head.remove(&mut g, start);
        if reinserted && kq.count() != 0 {
            kqueue_wakeup(kq, &mut g);
        }
        drop(g);
        if scan.nevent.get() == 0 {
            continue;
        }
        return (nkev, Ok(()));
    }
}

/// `kqueue_scan_setup(scan, kq)`: a scan of `kq` over the markers `start` and `end`
/// ([`Knote::marker`] of `kq`, pinned by the caller), with a reference to the kqueue; the
/// scan ends when the state is dropped (`kqueue_scan_finish`).
pub fn kqueue_scan_setup<'m, G: ProcGlue>(
    kq: &KqRef<G>,
    start: Pin<&'m Knote<G>>,
    end: Pin<&'m Knote<G>>,
) -> KqueueScanState<'m, G> {
    kassert!(start.kn_filter().get() == EVFILT_MARKER && end.kn_filter().get() == EVFILT_MARKER);
    KqueueScanState {
        kq: kq.clone(),
        start,
        end,
        nevent: Cell::new(0),
        queued: Cell::new(false),
    }
}

/// `kqueue_read`, `kqueue_write`: XXX This could be expanded to call kqueue_scan, if
/// desired.
pub fn kqueue_rw() -> Result<(), Errno> {
    Err(Errno::ENXIO)
}

/// `kqueue_ioctl`.
pub fn kqueue_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `kqueue_stat`'s numbers: `st_size` is the pending count (an unlocked read, as the C),
/// `st_blksize` the size of a `struct kevent`; the mode is `S_IFIFO`.
pub fn kqueue_stat<G: ProcGlue>(fp: &File<Files<G>>) -> (i64, u32) {
    let kq = fp_kqueue::<G>(fp);

    (i64::from(kq.count()), size_of::<Kevent>() as u32)
}

/// `kqueue_purge(p, kq)`: drops every knote of the kqueue.
pub fn kqueue_purge<G: ProcGlue>(p: Option<&Thread<G>>, kq: &KqRef<G>) {
    let mut g = kq.lock.lock();
    let mut i = 0;
    while i < g.knlistsize as usize {
        g = knote_remove(p, kq, g, false, i, true);
        i += 1;
    }
    if g.knhashmask != 0 {
        let mut i = 0;
        while i <= g.knhashmask as usize {
            g = knote_remove(p, kq, g, true, i, true);
            i += 1;
        }
    }
}

/// `kqueue_terminate(p, kq)`: marks the kqueue dying, wakes its sleepers and waits for its
/// pending task, whose counted argument then goes (the module's redesign).
pub fn kqueue_terminate<G: ProcGlue>(_p: Option<&Thread<G>>, kq: &KqRef<G>) {
    let mut g = kq.lock.lock();

    // Any remaining entries should be scan markers. They are removed when the ongoing scans
    // finish.
    kassert!(kq.count() == 0);
    for kn in kq.head.iter(&g) {
        kassert!(kn.kn_filter().get() == EVFILT_MARKER);
    }

    kq.state_or(&g, KQ_DYING);
    let state = kq.state();
    kqueue_wakeup(kq, &mut g);
    drop(g);

    // Any knotes that were attached to this kqueue were deleted by knote_fdclose() when this
    // kqueue's file descriptor was closed.
    kassert!(kq.klist.is_empty());
    if state & KQ_TASK != 0 {
        taskq_del_barrier(&SYSTQMP, &kq.task);
        kq.task.clear();
    }
}

/// `kqueue_close`: the descriptor's last close; the file's reference goes with the file.
pub fn kqueue_close<G: ProcGlue>(fp: &File<Files<G>>, p: Option<&Thread<G>>) -> Result<(), Errno> {
    let kq = fp_kqueue::<G>(fp);

    kqueue_purge(p, &kq);
    kqueue_terminate(p, &kq);

    Ok(())
}

/// `kqueue_task(arg)`: activates the knotes of other kqueues watching this one.
fn kqueue_task<G: ProcGlue>(arg: &AnyArc) {
    let Some(kq) = arg.downcast_ref::<Kqueue<G>>() else {
        kpanic(format_args!("kqueue_task: not a kqueue"));
    };

    let g = KQUEUE_KLIST_LOCK.lock();
    kq.klist.knote_locked(&g, 0);
}

/// `kqueue_wakeup(kq)`: wakes the scan sleeping on the kqueue and schedules the activation
/// of the kqueues watching it (`kq_lock` held).
fn kqueue_wakeup<G: ProcGlue>(kq: &Kqueue<G>, g: &mut KqGuard<'_, G>) {
    if kq.state() & KQ_SLEEP != 0 {
        kq.state_clear(g, KQ_SLEEP);
        hal::sleep::wakeup(ptr::from_ref(kq));
    }
    if !kq.klist.is_empty() {
        // Defer activation to avoid recursion. The task's argument is the kqueue, counted,
        // from its first scheduling until kqueue_terminate.
        if kq.state() & KQ_TASK == 0
            && let Some(r) = KqRef::from_ref(kq)
        {
            kq.task.set_shared(kqueue_task::<G>, r.erase());
        }
        kq.state_or(g, KQ_TASK);
        task_add(&SYSTQMP, kq.task.as_pin());
    }
}

/// `kqueue_expand_hash(kq)`: allocates the hash of non-descriptor knotes the first time
/// (`kq_lock` held, released around the allocation); the guard back.
fn kqueue_expand_hash<'a, G: ProcGlue>(
    kq: &'a Kqueue<G>,
    g: KqGuard<'a, G>,
) -> (KqGuard<'a, G>, Result<(), Errno>) {
    if g.knhashmask != 0 {
        return (g, Ok(()));
    }
    drop(g);
    let hash = MArray::new(M_KEVENT, M_WAITOK, KN_HASHSIZE, |_| OwnedList::new());
    let mut g = kq.lock.lock();
    let Some(hash) = hash else {
        return (g, Err(Errno::ENOMEM));
    };
    if g.knhashmask == 0 {
        g.knhash = Some(hash);
        g.knhashmask = KN_HASHSIZE as u64 - 1;
    } else {
        // Another thread has allocated the hash.
        drop(g);
        drop(hash);
        g = kq.lock.lock();
    }
    (g, Ok(()))
}

/// `kqueue_expand_list(kq, fd)`: grows the descriptor table past `fd` by `KQEXTENT` slots at
/// a time (`kq_lock` held, released around the allocation); the guard back.
fn kqueue_expand_list<'a, G: ProcGlue>(
    kq: &'a Kqueue<G>,
    g: KqGuard<'a, G>,
    fd: i32,
) -> (KqGuard<'a, G>, Result<(), Errno>) {
    if g.knlistsize > fd {
        return (g, Ok(()));
    }
    let mut size = g.knlistsize;
    drop(g);
    while size <= fd {
        size += KQEXTENT;
    }
    let list = MArray::new(M_KEVENT, M_WAITOK, size as usize, |_| OwnedList::new());
    let mut g = kq.lock.lock();
    let Some(mut list) = list else {
        return (g, Err(Errno::ENOMEM));
    };
    if g.knlistsize <= fd {
        // The chains move to the new table: a chain's identity is not its address.
        let mut olist = g.knlist.take();
        if let Some(old) = olist.as_mut() {
            for (new, old) in list.iter_mut().zip(old.iter_mut()) {
                *new = core::mem::take(old);
            }
        }
        g.knlist = Some(list);
        g.knlistsize = size;
        drop(g);
        drop(olist);
    } else {
        // Another thread has expanded the list.
        drop(g);
        drop(list);
    }
    (kq.lock.lock(), Ok(()))
}

/// `knote_acquire(kn, NULL, 0)`'s test: the knote, now `KN_PROCESSING`, or, when another
/// thread processes it, `KN_WAITING` set and the channel to sleep on (the reference given
/// back: the knote may be stale once `kq_lock` is released).
fn knote_try_acquire<G: ProcGlue>(
    g: &mut KqGuard<'_, G>,
    h: KnHold<G>,
) -> Result<KnHold<G>, *const ()> {
    let kn = h.kn();
    kassert!(kn.kn_filter().get() != EVFILT_MARKER);

    let st = kn.status_mut(g);
    if *st & KN_PROCESSING != 0 {
        *st |= KN_WAITING;
        let chan = ptr::from_ref(kn).cast::<()>();
        drop(h);
        return Err(chan);
    }
    *st |= KN_PROCESSING;
    Ok(h)
}

/// `knote_acquire(kn, NULL, 0)`: acquire a knote, `None` on failure. If we cannot acquire
/// the knote we sleep (`kq_lock` released, taken again) and return `None`: the knote may be
/// stale and the caller must restart whatever loop it is in.
fn knote_acquire<G: ProcGlue>(g: &mut KqGuard<'_, G>, h: KnHold<G>) -> Option<KnHold<G>> {
    match knote_try_acquire(g, h) {
        Ok(h) => Some(h),
        Err(chan) => {
            // XXX Timeout resolves potential loss of wakeup.
            let _ = g.sleep_nsec(chan, 0, "kqepts", SEC_NSEC);
            // knote may be stale now
            None
        }
    }
}

/// `knote_release(kn)`: release an acquired knote, clearing `KN_PROCESSING`.
fn knote_release<G: ProcGlue>(g: &mut KqGuard<'_, G>, h: KnHold<G>) {
    let kn = h.kn();
    kassert!(kn.kn_filter().get() != EVFILT_MARKER);
    let st = kn.status_mut(g);
    kassert!(*st & KN_PROCESSING != 0);

    if *st & KN_WAITING != 0 {
        *st &= !KN_WAITING;
        hal::sleep::wakeup(ptr::from_ref(kn));
    }
    *st &= !KN_PROCESSING;
    // kn should not be accessed anymore
}

/// `knote_activate(kn)`: activate one knote (`kq_lock` held).
fn knote_activate<G: ProcGlue>(kq: &Kqueue<G>, g: &mut KqGuard<'_, G>, kn: &Knote<G>) {
    let st = {
        let st = kn.status_mut(g);
        *st |= KN_ACTIVE;
        *st
    };
    if st & (KN_QUEUED | KN_DISABLED) == 0 {
        knote_enqueue(kq, g, kn);
    }
}

/// `knote_remove(p, kq, plist, idx, purge)`: remove all knotes from a specified knlist: slot
/// `idx` of the hash (`hash`) or of the descriptor table. Without `purge`, the knotes of a
/// poll or select are rewired to report `EBADF`. Called and returns with `kq_lock` held.
fn knote_remove<'a, G: ProcGlue>(
    p: Option<&Thread<G>>,
    kq: &'a KqRef<G>,
    mut g: KqGuard<'a, G>,
    hash: bool,
    idx: usize,
    purge: bool,
) -> KqGuard<'a, G> {
    loop {
        // Always fetch array pointer as another thread can resize kq_knlist.
        let list = if hash {
            g.knhash.as_ref().and_then(|a| a.get(idx))
        } else {
            g.knlist.as_ref().and_then(|a| a.get(idx))
        };
        let Some(list) = list else {
            return g;
        };
        // Skip pending badfd knotes.
        let Some(kn) = list.iter().find(|kn| purge || !kn.fop_is(FILTEROP_BADFD)) else {
            return g;
        };
        kassert!(ptr::eq(kn.kq(), &**kq));

        let h = KnHold::of(kn);
        let Some(h) = knote_acquire(&mut g, h) else {
            // knote_acquire() has slept and taken kq_lock again.
            continue;
        };
        drop(g);
        let kn = h.kn();
        filter_detach(kn);

        // Notify poll(2) and select(2) when a monitored file descriptor is closed.
        //
        // This reuses the original knote for delivering the notification so as to avoid
        // allocating memory.
        let expired = p.is_some_and(|p| {
            is_thread_kq(p, kq) && p.p_kq_serial.get() > kn.kn_udata().get() as u64
        });
        if !purge && kn.has_flags(__EV_POLL | __EV_SELECT) && !expired && !kn.fop_is(FILTEROP_BADFD)
        {
            kassert!(kn.fop().f_flags & FILTEROP_ISFD != 0);
            let fp = kn.fp.lock().take();
            if let Some(fp) = fp {
                let _ = fp.frele(p);
            }

            kn.kn_fop.set(Some(&Kqueue::<G>::BADFD_FILTOPS));
            filter_event(kn, 0);
            g = kq.lock.lock();
            knote_activate(kq, &mut g, kn);
            knote_release(&mut g, h);
            continue;
        }

        knote_drop(kq, h, p);
        g = kq.lock.lock();
    }
}

/// `knote_fdclose(p, fd)`: remove all knotes referencing a specified fd, in every kqueue of
/// the thread's table (the table locked by the caller, unless it is being freed). The
/// kqueues are walked one counted reference at a time, so no list lock is held across the
/// knotes' removal (which sleeps, and may drop a kqueue's last reference).
pub fn knote_fdclose<G: ProcGlue>(p: &Thread<G>, fd: i32) {
    // The descriptors' tests see where their closes call it (the glue's hook before N4.4).
    #[cfg(any(test, feature = "doubles"))]
    if crate::testutil::file_log_on() {
        crate::testutil::file_log_push(std::format!("knote_fdclose {fd}"));
    }

    let fdp = p.fd();

    // fdplock can be ignored if the file descriptor table is being freed because no other
    // thread can access the fdp.
    if fdp.shares() != 0 {
        fdpassertlocked(fdp);
    }

    let mut cur: Option<KqRef<G>> = None;
    loop {
        let next = {
            let g = fdp.kqlist_mtx.lock();
            let mut n = match &cur {
                None => fdp.kqlist.first(&g),
                Some(c) => fdp.kqlist.next(&g, c),
            };
            loop {
                match n {
                    None => break None,
                    Some(k) => match KqRef::try_from_ref(k) {
                        Some(r) => break Some(r),
                        // Its last reference is gone: it is leaving the list.
                        None => n = fdp.kqlist.next(&g, k),
                    },
                }
            }
        };
        drop(cur);
        let Some(kq) = next else {
            return;
        };
        {
            let g = kq.lock.lock();
            if fd < g.knlistsize {
                drop(knote_remove(Some(p), &kq, g, false, fd as usize, false));
            }
        }
        cur = Some(kq);
    }
}

/// `knote(&pr->ps_klist, hint)` of a holder of `ps_mtx` (`w`, its guard): `ptsignal`'s
/// `NOTE_SIGNAL`.
pub fn knote_process_locked<G: ProcGlue, W: Witness + ?Sized>(pr: &Process<G>, w: &W, hint: i64) {
    pr.ps_klist.knote_locked(w, hint);
}

/// `knote_processexit(pr)`: handle a process exiting, including the triggering of
/// `NOTE_EXIT` notes. XXX this could be more efficient, doing a single pass down the klist
pub fn knote_processexit<G: ProcGlue>(pr: &Process<G>) {
    {
        // this needs both the ps_mtx and exclusive kqueue_ps_list_lock.
        let _w = KQUEUE_PS_LIST_LOCK.write();
        let mut g = pr.ps_mtx.lock();
        pr.ps_klist.knote_locked(&g, i64::from(NOTE_EXIT));
        // filt_proc's NOTE_EXIT knotes leave the list (the C unlinks each in the walk).
        loop {
            let h = pr
                .ps_klist
                .list
                .iter(&g)
                .find(|kn| kn.pmark.load(Ordering::Relaxed) & PMARK_UNLINK != 0)
                .map(KnHold::of);
            let Some(h) = h else {
                break;
            };
            h.kn().pmark.fetch_and(!PMARK_UNLINK, Ordering::Relaxed);
            pr.ps_klist.remove_locked(&mut g, h.kn());
        }
    }

    // remove other knotes hanging off the process
    pr.ps_klist.invalidate_with(|| pr.ps_mtx.lock());
}

/// `knote_processfork(pr, pid)`: `NOTE_FORK` with the child's pid; then the `NOTE_TRACK`
/// knotes' registrations on the child, with `ps_mtx` released (the C makes them inside the
/// walk, dropping `ps_mtx` there), each knote activated after its registration as the C's
/// `filt_proc` return does.
pub fn knote_processfork<G: ProcGlue>(pr: &Process<G>, pid: Pid) {
    // this needs both the ps_mtx and exclusive kqueue_ps_list_lock.
    let _w = KQUEUE_PS_LIST_LOCK.write();
    {
        let g = pr.ps_mtx.lock();
        pr.ps_klist
            .knote_locked(&g, i64::from(NOTE_FORK) | i64::from(pid));
    }
    loop {
        let h = {
            let g = pr.ps_mtx.lock();
            pr.ps_klist
                .list
                .iter(&g)
                .find(|kn| kn.pmark.load(Ordering::Relaxed) & PMARK_TRACK != 0)
                .map(KnHold::of)
        };
        let Some(h) = h else {
            break;
        };
        let kn = h.kn();
        kn.pmark.fetch_and(!PMARK_TRACK, Ordering::Relaxed);
        filt_proc_track(kn, pid);
        // filt_proc's answer, after the registration (the child's knote is queued first):
        // `knote_locked`'s activation of a knote whose filter returned true. The knote stays
        // linked: its detach waits on kqueue_ps_list_lock, held here.
        if kn.kn_fflags().get() != 0 {
            let kq = kn.kq();
            let mut g = kq.lock.lock();
            knote_activate(kq, &mut g, kn);
        }
    }
}

/// `knote(&pr->ps_klist, NOTE_EXEC)`: the process exec'd.
pub fn knote_processexec<G: ProcGlue>(pr: &Process<G>) {
    let g = pr.ps_mtx.lock();
    pr.ps_klist.knote_locked(&g, i64::from(NOTE_EXEC));
}

/// `knote_attach(kn)`: links the knote into its kqueue's descriptor table or hash, which
/// owns it from now on; the reference its registration goes on with.
fn knote_attach<G: ProcGlue>(g: &mut KqGuard<'_, G>, kn: PoolBox<Knote<G>>) -> KnHold<G> {
    kassert!(kn.status(&*g) & KN_PROCESSING != 0);

    let isfd = kn.fop().f_flags & FILTEROP_ISFD != 0;
    let ident = kn.kn_id().get();
    let st: &mut KqState<G> = g;
    st.nknotes += 1;
    let Some(list) = st.slot_mut(isfd, ident) else {
        kpanic(format_args!("knote_attach: no chain for {ident}"));
    };
    list.push(kn);
    match list.last() {
        Some(kn) => KnHold::of(kn),
        None => kpanic(format_args!("knote_attach: the chain is empty")),
    }
}

/// `knote_drop(kn, p)`: unlinks the knote from its kqueue (`knote_detach`), drops its file
/// reference and frees it. Should be called at spl == 0, since we don't want to hold spl
/// while calling `FRELE` and `pool_put`.
fn knote_drop<G: ProcGlue>(kq: &Kqueue<G>, h: KnHold<G>, p: Option<&Thread<G>>) {
    let mut g = kq.lock.lock();
    let kn = h.kn();
    kassert!(kn.kn_filter().get() != EVFILT_MARKER);
    kassert!(kn.status(&g) & KN_PROCESSING != 0);

    if kn.status(&g) & KN_QUEUED != 0 {
        knote_dequeue(kq, &mut g, kn);
    }
    let st = kn.status_mut(&mut g);
    if *st & KN_WAITING != 0 {
        *st &= !KN_WAITING;
        hal::sleep::wakeup(ptr::from_ref(kn));
    }
    let isfd = kn.fop().f_flags & FILTEROP_ISFD != 0;
    let ident = kn.kn_id().get();
    let b = {
        let st: &mut KqState<G> = &mut g;
        st.nknotes -= 1;
        match st.slot_mut(isfd, ident) {
            Some(list) => list.take(h.0),
            None => kpanic(format_args!("knote_drop: no chain for {ident}")),
        }
    };
    drop(g);

    if isfd {
        // The slot's guard ends with the statement: `frele` may sleep in the file's close.
        let fp = b.fp.lock().take();
        if let Some(fp) = fp {
            let _ = fp.frele(p);
        }
    }
    drop(b);
}

/// `knote_enqueue(kn)`: queues an active knote on its kqueue and wakes the kqueue.
fn knote_enqueue<G: ProcGlue>(kq: &Kqueue<G>, g: &mut KqGuard<'_, G>, kn: &Knote<G>) {
    kassert!(kn.kn_filter().get() != EVFILT_MARKER);
    kassert!(kn.status(&*g) & KN_QUEUED == 0);

    // kqueue_check(kq): KQUEUE_DEBUG, not configured.
    kq.head.insert_tail(g, kn.pinned());
    *kn.status_mut(g) |= KN_QUEUED;
    kq.count_add(g, 1);
    kqueue_wakeup(kq, g);
}

/// `knote_dequeue(kn)`.
fn knote_dequeue<G: ProcGlue>(kq: &Kqueue<G>, g: &mut KqGuard<'_, G>, kn: &Knote<G>) {
    kassert!(kn.kn_filter().get() != EVFILT_MARKER);
    kassert!(kn.status(&*g) & KN_QUEUED != 0);

    kq.head.remove(g, kn);
    *kn.status_mut(g) &= !KN_QUEUED;
    kq.count_add(g, -1);
}

/// `knote_assign(kev, kn)`: assign parameters to the knote. The knote's object lock must be
/// held.
pub fn knote_assign<G: ProcGlue>(kev: &Kevent, kn: &Knote<G>) {
    if kn.fop().f_flags & FILTEROP_MPSAFE == 0 {
        klock::kernel_assert_locked();
    }

    kn.kn_sfflags.set(kev.fflags);
    kn.kn_sdata.set(kev.data);
    kn.kn_udata().set(kev.udata);
}

/// `knote_submit(kn, kev)`: submit the knote's event for delivery. The knote's object lock
/// must be held.
pub fn knote_submit<G: ProcGlue>(kn: &Knote<G>, kev: Option<&mut Kevent>) {
    if kn.fop().f_flags & FILTEROP_MPSAFE == 0 {
        klock::kernel_assert_locked();
    }

    if let Some(kev) = kev {
        *kev = kn.kn_kevent.get();
        if kn.has_flags(EV_CLEAR) {
            kn.kn_fflags().set(0);
            kn.kn_data().set(0);
        }
    }
}

/// `knote_modify(kev, kn)`: assigns the user's parameters and re-runs the event function.
pub fn knote_modify<G: ProcGlue>(kev: &Kevent, kn: &Knote<G>) -> bool {
    knote_assign(kev, kn);
    fop_event(kn, 0)
}

/// `knote_process(kn, kev)`: whether the event is active, its state stored in `kev` if so.
pub fn knote_process<G: ProcGlue>(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
    // If called from kqueue_scan(), skip f_event when EV_ONESHOT is set, to preserve old
    // behaviour.
    let active = if kev.is_some() && kn.has_flags(EV_ONESHOT) {
        true
    } else {
        fop_event(kn, 0)
    };
    if active {
        knote_submit(kn, kev);
    }
    active
}

// A knote, a kqueue and a klist are shared between CPUs by composition (no `unsafe impl`):
// the kernel's own glue proves it below for the tests' `TestGlue`, `bsd` for its own.
/// The filterops of a [`test_knote`]: no file, no event, nothing to detach (the test unhooks
/// the knote itself).
#[cfg(any(test, feature = "doubles"))]
const fn test_filtops<G: ProcGlue>() -> Filterops<G> {
    Filterops {
        f_flags: FILTEROP_MPSAFE,
        f_attach: None,
        f_detach: None,
        f_event: None,
        f_modify: None,
        f_process: None,
    }
}

/// A knote of `kq` that its chain owns (the hash slot of `ident`, as `EVFILT_USER`'s),
/// attached to no object and held `KN_PROCESSING`, for host tests that hook it on a klist
/// themselves (the kernel's and `bsd`'s filter tests); [`test_knote_drop`] gives it back.
#[cfg(any(test, feature = "doubles"))]
pub fn test_knote<G: ProcGlue>(kq: &KqRef<G>, ident: usize) -> KnHold<G> {
    let Some(kn) = G::tables().kq.knotepl.get(PR_WAITOK, || Knote::new(kq)) else {
        kpanic(format_args!("test_knote: no knote"));
    };
    let (mut g, e) = kqueue_expand_hash(kq, kq.lock.lock());
    if e.is_err() {
        kpanic(format_args!("test_knote: no hash"));
    }
    *kn.status_mut(&mut g) = KN_PROCESSING;
    kn.kn_fop.set(Some(&Kqueue::<G>::TEST_FILTOPS));
    kn.kn_kevent.ident.set(ident);
    kn.kn_kevent.filter.set(EVFILT_USER);
    knote_attach(&mut g, kn)
}

/// Gives back a [`test_knote`], unhooked from every klist by the test (its filterops put
/// back first: they name its chain).
#[cfg(any(test, feature = "doubles"))]
pub fn test_knote_drop<G: ProcGlue>(kq: &KqRef<G>, h: KnHold<G>) {
    h.kn().kn_fop.set(Some(&Kqueue::<G>::TEST_FILTOPS));
    knote_drop(kq, h, None);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests for `kern_event.c` over the kernel's test glue: knotes registered on test
    // objects of each klist kind, activated, scanned and deleted; every `EV_*` action; the
    // system filters (proc, signal, timer through the wheel, user, kqueue on kqueue, dead,
    // badfd); a closed descriptor under a poll knote; the thread's poll kqueue; contention on
    // `KN_PROCESSING`; two scans crossing markers; the chains' growth.

    use core::sync::atomic::AtomicBool;
    use std::boxed::Box;
    use std::cell::RefCell;
    use std::vec::Vec;

    use super::*;
    use crate::cred::{CredRef, Ucred};
    use crate::descrip::{fdinit, fdrelease, filedesc_init};
    use crate::proc::{PS_PLEDGE, process_alloc, procinit, thread_alloc};
    use crate::resource::{Plimit, PlimitRef, RLIMIT_NOFILE};
    use crate::testutil::{self, TestFileData, TestStat};
    use crate::uio::Uio;

    crate::test_file_glue!(Kf, Kg);
    crate::test_proc_glue!(Kg, Kf);

    pub(crate) type T = &'static Thread<Kg>;

    /// A test object: its klist, whether its event is ready, its owner's lock (an `Owner`
    /// klist's).
    pub(crate) struct Obj {
        pub(crate) sel: crate::select::Selinfo<Kg>,
        pub(crate) ready: AtomicBool,
        lock: SharedMutex,
        /// A FIFO's: `EVFILT_WRITE` is `EPERM`, `EVFILT_EXCEPT` attaches (`filt_fifo*`).
        pub(crate) fifo: AtomicBool,
    }

    std::thread_local! {
        static OBJS: RefCell<Vec<&'static Obj>> = const { RefCell::new(Vec::new()) };
    }

    /// A new object whose klist is of `kind`, and its id (its files' object).
    pub(crate) fn obj(kind: KlistKind) -> (usize, &'static Obj) {
        let o: &'static Obj = Box::leak(Box::new(Obj {
            sel: crate::select::Selinfo {
                si_note: match kind {
                    KlistKind::Owner => Klist::owner(),
                    _ => Klist::new(),
                },
            },
            ready: AtomicBool::new(false),
            lock: Mutexed::new(IPL_HIGH, ()),
            fifo: AtomicBool::new(false),
        }));
        if kind == KlistKind::Own {
            o.sel.si_note.init_own();
        }
        OBJS.with(|v| {
            let mut v = v.borrow_mut();
            v.push(o);
            (v.len(), o)
        })
    }

    pub(crate) fn obj_of(fp: &File<Kf>) -> &'static Obj {
        let id = fp.data().map_or(0, |d| d.id);
        OBJS.with(|v| v.borrow()[id - 1])
    }

    /// The object posts an event (`knote`/`knote_locked` as its kind takes it).
    pub(crate) fn post(o: &Obj) {
        match o.sel.si_note.kind() {
            KlistKind::Owner => {
                let g = o.lock.lock();
                o.sel.si_note.knote_locked(&g, 0);
            }
            _ => o.sel.si_note.knote(0),
        }
    }

    fn filt_testdetach(kn: &Knote<Kg>) {
        let o = obj_of(&kn.fp());
        match o.sel.si_note.kind() {
            KlistKind::Owner => {
                let mut g = o.lock.lock();
                o.sel.si_note.remove_locked(&mut g, kn);
            }
            _ => o.sel.si_note.remove(kn),
        }
    }

    fn filt_test(kn: &Knote<Kg>, _hint: i64) -> bool {
        let ready = obj_of(&kn.fp()).ready.load(Ordering::Relaxed);
        kn.kn_data().set(i64::from(ready));
        ready
    }

    fn filt_testmodify(kev: &mut Kevent, kn: &Knote<Kg>) -> bool {
        knote_modify(kev, kn)
    }

    fn filt_testprocess(kn: &Knote<Kg>, kev: Option<&mut Kevent>) -> bool {
        knote_process(kn, kev)
    }

    static TEST_FILTOPS: Filterops<Kg> = Filterops {
        f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
        f_attach: None,
        f_detach: Some(filt_testdetach),
        f_event: Some(filt_test),
        f_modify: Some(filt_testmodify),
        f_process: Some(filt_testprocess),
    };

    /// The test files' operations: `EVFILT_READ` hooks the knote on its object's klist.
    struct TOps;

    impl FileOps<Kf> for TOps {
        fn read(&self, _: &File<Kf>, _: &mut Uio<'_, Kg>, _: i32) -> Result<(), Errno> {
            Ok(())
        }
        fn write(&self, _: &File<Kf>, _: &mut Uio<'_, Kg>, _: i32) -> Result<(), Errno> {
            Ok(())
        }
        fn ioctl(&self, _: &File<Kf>, _: u64, _: &mut [u8], _: &Thread<Kg>) -> Result<(), Errno> {
            Err(Errno::ENOTTY)
        }
        fn kqfilter(&self, fp: &File<Kf>, kn: &Knote<Kg>) -> Result<(), Errno> {
            let fifo = obj_of(fp).fifo.load(Ordering::Relaxed);
            match kn.kn_filter().get() {
                EVFILT_READ => {}
                EVFILT_WRITE if fifo => return Err(Errno::EPERM),
                EVFILT_EXCEPT if fifo => {}
                _ => return Err(Errno::EINVAL),
            }
            kn.kn_fop.set(Some(&TEST_FILTOPS));
            let o = obj_of(fp);
            match o.sel.si_note.kind() {
                KlistKind::Owner => {
                    let mut g = o.lock.lock();
                    o.sel.si_note.insert_locked(&mut g, kn);
                }
                _ => o.sel.si_note.insert(kn),
            }
            Ok(())
        }
        fn stat(&self, _: &File<Kf>, _: &mut TestStat, _: &Thread<Kg>) -> Result<(), Errno> {
            Ok(())
        }
        fn close(&self, _: &File<Kf>, _: Option<&Thread<Kg>>) -> Result<(), Errno> {
            Ok(())
        }
    }

    static TOPS: TOps = TOps;

    /// Fresh tables, pools and a thread of process `pid` on the process lists.
    pub(crate) fn setup(pid: i32) -> T {
        testutil::install();
        hal::klock::kernel_lock();
        procinit::<Kg>(64, 64);
        let _ = filedesc_init::<Kf>();
        kqueue_init::<Kg>().unwrap();
        testutil::set_test_maxfiles(4096);
        testutil::set_file_log(false);
        thread_of(pid)
    }

    /// A thread of a new process `pid` (on `allprocess` and its pid chain, never taken off:
    /// the tests' processes live for the run).
    pub(crate) fn thread_of(pid: i32) -> T {
        let pr = process_alloc::<Kg>(pid, ());
        drop(pr.set_ucred(CredRef::new(Ucred::new())));
        pr.set_fd(fdinit());
        let mut limits = Plimit::new();
        limits.pl_rlimit[RLIMIT_NOFILE].rlim_cur = 512;
        limits.pl_rlimit[RLIMIT_NOFILE].rlim_max = 512;
        drop(pr.set_limit(PlimitRef::new(limits)));
        {
            let t = Kg::tables();
            let mut w = klock::witness();
            t.allprocess.insert_head(&mut w, pr.as_pin());
            t.pidhash(pid).insert_head(&mut w, pr.as_pin());
        }
        let t = thread_alloc::<Kg>(&pr, (), ());
        drop(t.set_cred(pr.ucred()));
        core::mem::forget(pr);
        let t: &'static PoolBox<Thread<Kg>> = Box::leak(Box::new(t));
        t
    }

    /// A test file on object `oid` at the lowest free descriptor.
    pub(crate) fn open_obj(p: T, oid: usize) -> i32 {
        let mut g = fdplock(p.fd());
        let (fp, fd) = crate::descrip::falloc(p, &mut g).unwrap();
        fp.f_flag.store((FREAD | FWRITE) as u32, Ordering::SeqCst);
        fp.install(
            crate::file::DTYPE_PIPE,
            &TOPS,
            TestFileData {
                id: oid,
                ..TestFileData::default()
            },
        );
        fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        fp.frele(Some(p)).unwrap();
        fd
    }

    /// `close(2)` of `fd`.
    pub(crate) fn close_fd(p: T, fd: i32) {
        assert_eq!(fdrelease(p, fdplock(p.fd()), fd), Ok(()));
    }

    /// A kqueue made by `kqueue(2)`: its descriptor and a reference.
    pub(crate) fn new_kqueue(p: T) -> (i32, KqRef<Kg>) {
        let fd = sys_kqueue(p).unwrap();
        let fp = fd_getfile(p.fd(), fd).unwrap();
        let kq = fp_kqueue::<Kg>(&fp);
        fp.frele(Some(p)).unwrap();
        (fd, kq)
    }

    /// One non-blocking scan: the events, and the scan's error.
    pub(crate) fn scan(p: T, kq: &KqRef<Kg>, out: &mut [Kevent]) -> (usize, Result<(), Errno>) {
        let start = core::pin::pin!(Knote::marker(kq));
        let end = core::pin::pin!(Knote::marker(kq));
        let s = kqueue_scan_setup(kq, start.as_ref(), end.as_ref());
        let mut ts = Timespec::ZERO;
        let n = out.len();
        kqueue_scan(&s, n, out, Some(&mut ts), p)
    }

    /// Serialises the test that lowers the global `KQ_USEREVENTSMAX` (writer) with the tests
    /// that make user events or timers, which count against it (readers).
    static USEREVENTS: std::sync::RwLock<()> = std::sync::RwLock::new(());

    /// A reader of [`USEREVENTS`].
    fn userevents_shared() -> std::sync::RwLockReadGuard<'static, ()> {
        USEREVENTS
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn reg(p: T, kq: &KqRef<Kg>, kev: Kevent) -> Result<(), Errno> {
        let mut kev = kev;
        kqueue_register(kq, &mut kev, 0, Some(p))
    }

    #[test]
    fn a_knote_attaches_activates_and_detaches_on_every_klist_kind() {
        let p = setup(10);
        for kind in [KlistKind::Kernel, KlistKind::Own, KlistKind::Owner] {
            let (kfd, kq) = new_kqueue(p);
            let (oid, o) = obj(kind);
            let fd = open_obj(p, oid);
            let fp = fd_getfile(p.fd(), fd).unwrap();
            let mut out = [Kevent::default(); 4];

            assert_eq!(
                reg(
                    p,
                    &kq,
                    ev_set(fd as usize, EVFILT_READ, EV_ADD, 0, 0, 0x1234)
                ),
                Ok(())
            );
            assert!(
                !o.sel.si_note.is_empty(),
                "{kind:?}: fo_kqfilter hooked the knote"
            );
            assert_eq!(kq.nknotes(), 1);
            assert_eq!(
                fp.count(),
                3,
                "the slot, the test and the knote hold the file"
            );
            assert_eq!(scan(p, &kq, &mut out), (0, Ok(())), "not ready yet");

            // The object posts an event: the knote is queued, and stays (level-triggered).
            o.ready.store(true, Ordering::Relaxed);
            post(o);
            assert_eq!(kq.count(), 1);
            assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
            assert_eq!((out[0].ident, out[0].filter), (fd as usize, EVFILT_READ));
            assert_eq!((out[0].data, out[0].udata), (1, 0x1234));
            assert_eq!(scan(p, &kq, &mut out), (1, Ok(())), "still ready");

            // No longer ready: the scan drops it.
            o.ready.store(false, Ordering::Relaxed);
            assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));
            assert_eq!(kq.count(), 0);

            // EV_DISABLE keeps a posted event from being queued; EV_ENABLE rechecks.
            assert_eq!(
                reg(
                    p,
                    &kq,
                    ev_set(fd as usize, EVFILT_READ, EV_DISABLE, 0, 0, 0)
                ),
                Ok(())
            );
            o.ready.store(true, Ordering::Relaxed);
            post(o);
            assert_eq!(kq.count(), 0);
            assert_eq!(
                reg(p, &kq, ev_set(fd as usize, EVFILT_READ, EV_ENABLE, 0, 0, 0)),
                Ok(())
            );
            assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));

            // EV_DELETE detaches it from the object and drops the file reference.
            let del = ev_set(fd as usize, EVFILT_READ, EV_DELETE, 0, 0, 0);
            assert_eq!(reg(p, &kq, del), Ok(()));
            assert!(o.sel.si_note.is_empty());
            assert_eq!(kq.nknotes(), 0);
            assert_eq!(fp.count(), 2);
            assert_eq!(reg(p, &kq, del), Err(Errno::ENOENT), "deleted twice");

            fp.frele(Some(p)).unwrap();
            close_fd(p, fd);
            close_fd(p, kfd);
            assert_eq!(kq.refs(), 1, "only this test's reference is left");
        }
    }

    #[test]
    fn bad_filters_descriptors_and_identifiers_are_refused() {
        let _u = userevents_shared();
        let p = setup(11);
        let (_kfd, kq) = new_kqueue(p);
        let (oid, _o) = obj(KlistKind::Kernel);
        let fd = open_obj(p, oid);
        let cases = [
            (ev_set(99, EVFILT_READ, EV_ADD, 0, 0, 0), Err(Errno::EBADF)),
            (
                ev_set(1usize << 40, EVFILT_READ, EV_ADD, 0, 0, 0),
                Err(Errno::EBADF),
            ),
            (
                ev_set(fd as usize, EVFILT_WRITE, EV_ADD, 0, 0, 0),
                Err(Errno::EINVAL),
            ),
            (
                ev_set(fd as usize, -11, EV_ADD, 0, 0, 0),
                Err(Errno::EINVAL),
            ),
            (
                ev_set(fd as usize, EVFILT_AIO, EV_ADD, 0, 0, 0),
                Err(Errno::EINVAL),
            ),
            (ev_set(fd as usize, 3, EV_ADD, 0, 0, 0), Err(Errno::EINVAL)),
            (
                ev_set(fd as usize, EVFILT_READ, EV_DELETE, 0, 0, 0),
                Err(Errno::ENOENT),
            ),
            (
                ev_set(5, EVFILT_USER, EV_ENABLE, 0, 0, 0),
                Err(Errno::ENOENT),
            ),
            (
                ev_set(1, EVFILT_TIMER, EV_ADD, 0x100, 10, 0),
                Err(Errno::EINVAL),
            ),
            (
                ev_set(NSIG, EVFILT_SIGNAL, EV_ADD, 0, 0, 0),
                Err(Errno::EINVAL),
            ),
            (
                ev_set(99_999_999, EVFILT_PROC, EV_ADD, 0, 0, 0),
                Err(Errno::ESRCH),
            ),
            (
                ev_set(4242, EVFILT_PROC, EV_ADD, 0, 0, 0),
                Err(Errno::ESRCH),
            ),
        ];
        Kg::set_curproc(Some(p));
        for (i, (kev, want)) in cases.into_iter().enumerate() {
            assert_eq!(reg(p, &kq, kev), want, "case {i}");
        }
        Kg::set_curproc(None);
        assert_eq!(kq.nknotes(), 0);
    }

    #[test]
    fn oneshot_dispatch_and_clear_post_event_actions() {
        let p = setup(12);
        let (_kfd, kq) = new_kqueue(p);
        let (oid, o) = obj(KlistKind::Own);
        let fd = open_obj(p, oid) as usize;
        o.ready.store(true, Ordering::Relaxed);
        let mut out = [Kevent::default(); 2];

        // EV_ONESHOT: reported once, then dropped.
        assert_eq!(
            reg(
                p,
                &kq,
                ev_set(fd, EVFILT_READ, EV_ADD | EV_ONESHOT, 0, 0, 0)
            ),
            Ok(())
        );
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(kq.nknotes(), 0);
        assert!(o.sel.si_note.is_empty());
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));

        // EV_DISPATCH: reported once, then disabled until re-enabled.
        assert_eq!(
            reg(
                p,
                &kq,
                ev_set(fd, EVFILT_READ, EV_ADD | EV_DISPATCH, 0, 0, 0)
            ),
            Ok(())
        );
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        post(o);
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));
        assert_eq!(
            reg(p, &kq, ev_set(fd, EVFILT_READ, EV_ENABLE, 0, 0, 0)),
            Ok(())
        );
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(
            reg(p, &kq, ev_set(fd, EVFILT_READ, EV_DELETE, 0, 0, 0)),
            Ok(())
        );

        // EV_CLEAR: the state resets after it is reported; a new post reports again.
        assert_eq!(
            reg(p, &kq, ev_set(fd, EVFILT_READ, EV_ADD | EV_CLEAR, 0, 0, 0)),
            Ok(())
        );
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(out[0].data, 1);
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())), "cleared");
        post(o);
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));

        close_fd(p, fd as i32);
        assert!(o.sel.si_note.is_empty(), "knote_fdclose detached it");
        assert_eq!(kq.nknotes(), 0);
    }

    #[test]
    fn receipt_and_the_change_list_errors_come_back_through_kevent() {
        let _u = userevents_shared();
        let p = setup(13);
        let (kfd, _kq) = new_kqueue(p);
        // The user space of the double: a change list of three at 0x100, events at 0x400.
        let base = testutil::USER_BASE + 0x100;
        let changes = [
            ev_set(1, EVFILT_USER, EV_ADD | EV_RECEIPT, 0, 0, 7),
            ev_set(77, EVFILT_READ, EV_ADD, 0, 0, 8),
            ev_set(2, EVFILT_USER, EV_ADD, 0, 0, 9),
        ];
        UserPtr::<Kevent>::new(base)
            .copyout_slice(&changes)
            .unwrap();
        let events = UserPtr::<Kevent>::new(testutil::USER_BASE + 0x400);
        assert_eq!(
            sys_kevent(p, kfd, UserPtr::new(base), 3, events, 8, 0),
            Ok(2),
            "two errors reported: the receipt and the bad descriptor"
        );
        let mut got = [Kevent::default(); 2];
        events.copyin_slice(&mut got).unwrap();
        assert_eq!((got[0].ident, got[0].flags, got[0].data), (1, EV_ERROR, 0));
        assert_eq!(
            (got[1].ident, got[1].flags, got[1].data),
            (77, EV_ERROR, i64::from(Errno::EBADF.as_i32()))
        );
        // Without an event list the first error is the call's, and the rest is not applied.
        UserPtr::<Kevent>::new(base)
            .copyout_slice(&[ev_set(78, EVFILT_READ, EV_ADD, 0, 0, 0)])
            .unwrap();
        assert_eq!(
            sys_kevent(p, kfd, UserPtr::new(base), 1, events, 0, 0),
            Err(Errno::EBADF)
        );
        // A bad timeout, a descriptor that is not a kqueue, a closed one.
        UserPtr::<UserTimespec>::new(base)
            .copyout(&UserTimespec {
                tv_sec: 0,
                tv_nsec: 1_000_000_000,
            })
            .unwrap();
        assert_eq!(
            sys_kevent(p, kfd, UserPtr::new(0), 0, events, 8, base),
            Err(Errno::EINVAL)
        );
        let (oid, _o) = obj(KlistKind::Kernel);
        let fd = open_obj(p, oid);
        assert_eq!(
            sys_kevent(p, fd, UserPtr::new(0), 0, events, 8, 0),
            Err(Errno::EBADF)
        );
        assert_eq!(
            sys_kevent(p, 300, UserPtr::new(0), 0, events, 8, 0),
            Err(Errno::EBADF)
        );
        // A zero timeout collects what is pending: the user event 2 was added, not triggered.
        UserPtr::<UserTimespec>::new(base)
            .copyout(&UserTimespec::default())
            .unwrap();
        assert_eq!(
            sys_kevent(p, kfd, UserPtr::new(0), 0, events, 8, base),
            Ok(0)
        );
        assert_eq!(sys_kqueue1(p, 0x4000_0000), Err(Errno::EINVAL));
    }

    #[test]
    fn user_events_trigger_combine_fflags_and_count_against_the_limit() {
        // It lowers kq_usereventsmax, which every test making user events or timers reads.
        let _u = USEREVENTS
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let p = setup(14);
        let (_kfd, kq) = new_kqueue(p);
        let mut out = [Kevent::default(); 2];

        assert_eq!(
            reg(
                p,
                &kq,
                ev_set(1, EVFILT_USER, EV_ADD | EV_CLEAR, 0x0f, 7, 0)
            ),
            Ok(())
        );
        assert_eq!(p.fd().fd_nuserevents.load(Ordering::SeqCst), 1);
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())), "not triggered");

        // NOTE_TRIGGER without EV_ADD goes through f_modify; NOTE_FFOR merges the flags.
        let trig = ev_set(1, EVFILT_USER, 0, NOTE_TRIGGER | NOTE_FFOR | 0x30, 9, 0);
        assert_eq!(reg(p, &kq, trig), Ok(()));
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].filter), (1, EVFILT_USER));
        assert_eq!((out[0].fflags, out[0].data), (0x3f, 9));
        assert_eq!(
            scan(p, &kq, &mut out),
            (0, Ok(())),
            "EV_CLEAR resets the trigger"
        );
        for (ctl, want) in [
            (NOTE_FFAND | 0x3, 0x3),
            (NOTE_FFCOPY | 0x40, 0x40),
            (NOTE_FFNOP | 0x1, 0x40),
        ] {
            assert_eq!(
                reg(p, &kq, ev_set(1, EVFILT_USER, 0, NOTE_TRIGGER | ctl, 0, 0)),
                Ok(())
            );
            assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
            assert_eq!(out[0].fflags, want);
        }

        // kq_usereventsmax: one more user event than the limit is ENOMEM.
        KQ_USEREVENTSMAX.store(2, Ordering::Relaxed);
        assert_eq!(reg(p, &kq, ev_set(2, EVFILT_USER, EV_ADD, 0, 0, 0)), Ok(()));
        assert_eq!(
            reg(p, &kq, ev_set(3, EVFILT_USER, EV_ADD, 0, 0, 0)),
            Err(Errno::ENOMEM)
        );
        KQ_USEREVENTSMAX.store(1024, Ordering::Relaxed);
        assert_eq!(p.fd().fd_nuserevents.load(Ordering::SeqCst), 2);

        assert_eq!(
            reg(p, &kq, ev_set(1, EVFILT_USER, EV_DELETE, 0, 0, 0)),
            Ok(())
        );
        assert_eq!(
            reg(p, &kq, ev_set(2, EVFILT_USER, EV_DELETE, 0, 0, 0)),
            Ok(())
        );
        assert_eq!(p.fd().fd_nuserevents.load(Ordering::SeqCst), 0);
    }

    /// Serialises the tests that drive the timeout wheel (one wheel per process).
    static WHEEL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn wheel() -> std::sync::MutexGuard<'static, ()> {
        let g = WHEEL.lock().unwrap_or_else(|e| e.into_inner());
        crate::timeout::startup(100, 10_000, 10_000_000);
        g
    }

    #[test]
    fn timers_fire_through_the_wheel_count_expiries_and_reset_on_modify() {
        let _u = userevents_shared();
        let _w = wheel();
        let p = setup(15);
        let (_kfd, kq) = new_kqueue(p);
        let mut out = [Kevent::default(); 2];

        // A one-shot 20 ms timer fires once and is gone.
        assert_eq!(
            reg(
                p,
                &kq,
                ev_set(5, EVFILT_TIMER, EV_ADD | EV_ONESHOT, 0, 20, 0)
            ),
            Ok(())
        );
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));
        let mut ticks = 0;
        while kq.count() == 0 && ticks < 100 {
            crate::timeout::tick_once();
            ticks += 1;
        }
        assert!(ticks < 100, "the timer fired");
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(
            (out[0].ident, out[0].filter, out[0].data),
            (5, EVFILT_TIMER, 1)
        );
        assert_eq!(kq.nknotes(), 0, "EV_ONESHOT dropped it");
        assert_eq!(p.fd().fd_nuserevents.load(Ordering::SeqCst), 0);

        // A periodic timer counts its expirations between scans (EV_CLEAR is implied).
        assert_eq!(
            reg(p, &kq, ev_set(6, EVFILT_TIMER, EV_ADD, 0, 10, 0)),
            Ok(())
        );
        for _ in 0..50 {
            crate::timeout::tick_once();
        }
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert!(out[0].data > 1, "several expirations: {}", out[0].data);
        assert_eq!(out[0].flags & EV_CLEAR, EV_CLEAR);
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())), "cleared");

        // A modify discards what is pending and rearms; a bad unit comes back in the event.
        for _ in 0..5 {
            crate::timeout::tick_once();
        }
        assert_eq!(
            reg(
                p,
                &kq,
                ev_set(6, EVFILT_TIMER, EV_ADD, NOTE_SECONDS, 100, 0)
            ),
            Ok(())
        );
        assert_eq!(kq.count(), 0, "pending events discarded");
        assert_eq!(
            reg(p, &kq, ev_set(6, EVFILT_TIMER, EV_ADD, 0x100, 10, 0)),
            Err(Errno::EINVAL)
        );
        // An absolute time already past expires at once.
        let mut kev = ev_set(7, EVFILT_TIMER, EV_ADD, NOTE_ABSTIME | NOTE_SECONDS, 1, 0);
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].data), (7, 1));

        kqueue_purge(Some(p), &kq);
        assert_eq!(p.fd().fd_nuserevents.load(Ordering::SeqCst), 0);
        assert!(crate::timeout::wheel_is_empty() || kq.nknotes() == 0);
    }

    #[test]
    fn timer_units_validate_and_the_hash_mixes() {
        assert_eq!(filt_timervalidate(NOTE_SECONDS, 3), Ok(Timespec::new(3, 0)));
        assert_eq!(
            filt_timervalidate(NOTE_MSECONDS, 1500),
            Ok(Timespec::new(1, 500_000_000))
        );
        assert_eq!(
            filt_timervalidate(NOTE_USECONDS, 2_000_001),
            Ok(Timespec::new(2, 1000))
        );
        assert_eq!(
            filt_timervalidate(NOTE_NSECONDS, 7),
            Ok(Timespec::new(0, 7))
        );
        assert_eq!(
            filt_timervalidate(NOTE_SECONDS | NOTE_ABSTIME, 1),
            Ok(Timespec::new(1, 0))
        );
        assert_eq!(filt_timervalidate(0x20, 1), Err(Errno::EINVAL));
        assert_eq!(kn_hash(0x1234, 63), (0x1234 ^ 0x12) & 63);
        assert_eq!(timespec_to_nsec(&Timespec::new(-1, 5)), 0);
        assert_eq!(timespec_to_nsec(&Timespec::new(2, 5)), 2_000_000_005);
        assert_eq!(timespec_to_nsec(&Timespec::new(i64::MAX, 0)), u64::MAX);
    }

    #[test]
    fn closing_a_polled_descriptor_reports_ebadf_and_old_serials_are_dropped() {
        let p = setup(16);
        let (oid, o) = obj(KlistKind::Kernel);
        let fd = open_obj(p, oid);
        let mut out = [Kevent::default(); 2];

        // The thread's poll kqueue, as poll(2) registers on it.
        assert_eq!(kqpoll_init(p, 1), Ok(()));
        let kq = thread_kq(p).clone();
        let serial = p.p_kq_serial.get();
        let pollkev = ev_set(
            fd as usize,
            EVFILT_READ,
            EV_ADD | EV_ENABLE | __EV_POLL,
            0,
            0,
            serial as usize,
        );
        assert_eq!(reg(p, &kq, pollkev), Ok(()));
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));

        // knote_fdclose rewires the knote to badfd_filtops instead of dropping it.
        close_fd(p, fd);
        assert!(o.sel.si_note.is_empty());
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(out[0].flags & EV_ERROR, EV_ERROR);
        assert_eq!(out[0].data, i64::from(Errno::EBADF.as_i32()));
        assert_eq!(kq.nknotes(), 0, "EV_ONESHOT dropped it");
        kqpoll_done(p, 1);
        assert_eq!(p.p_kq_serial.get(), serial + 1);

        // A knote of an older serial is dropped by the scan, unreported.
        let fd = open_obj(p, oid);
        o.ready.store(true, Ordering::Relaxed);
        let mut kev = ev_set(
            fd as usize,
            EVFILT_READ,
            EV_ADD | EV_ENABLE | __EV_POLL,
            0,
            0,
            serial as usize,
        );
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert_eq!(kq.count(), 1);
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));
        assert_eq!(kq.nknotes(), 0);

        // A pending badfd knote is dropped by an EV_ADD on its slot, which retries.
        let mut kev = ev_set(
            fd as usize,
            EVFILT_READ,
            EV_ADD | __EV_POLL,
            0,
            0,
            p.p_kq_serial.get() as usize,
        );
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        close_fd(p, fd);
        let fd2 = open_obj(p, oid);
        assert_eq!(fd2, fd);
        let mut kev = ev_set(
            fd as usize,
            EVFILT_READ,
            EV_ADD | __EV_POLL,
            0,
            0,
            p.p_kq_serial.get() as usize,
        );
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert_eq!(kq.nknotes(), 1, "the badfd knote made room for the new one");
        assert!(!o.sel.si_note.is_empty());

        close_fd(p, fd);
        kqpoll_exit(p);
        assert_eq!(kq.nknotes(), 0);
        assert_eq!(kq.state() & KQ_DYING, KQ_DYING);
    }

    #[test]
    fn kqpoll_purges_past_four_knotes_a_slot_and_on_the_serial_wrap() {
        let p = setup(17);
        let (oid, o) = obj(KlistKind::Kernel);
        let fd = open_obj(p, oid) as usize;
        assert_eq!(kqpoll_init(p, 1), Ok(()));
        let kq = thread_kq(p).clone();
        // One slot table of KQEXTENT; more than 4 * 256 knotes on it (by pollid) purge.
        for id in 0..(4 * KQEXTENT as u32 + 1) {
            let mut kev = ev_set(
                fd,
                EVFILT_READ,
                EV_ADD | __EV_POLL,
                0,
                0,
                p.p_kq_serial.get() as usize,
            );
            assert_eq!(kqueue_register(&kq, &mut kev, id, Some(p)), Ok(()));
        }
        assert_eq!(kq.nknotes(), 4 * KQEXTENT as u32 + 1);
        kqpoll_done(p, 1);
        assert_eq!(kq.nknotes(), 0);
        assert!(o.sel.si_note.is_empty());
        // The serial about to wrap: everything attached is purged, the serial restarts at 0.
        p.p_kq_serial.set(u64::MAX - 1);
        let mut kev = ev_set(fd, EVFILT_READ, EV_ADD | __EV_POLL, 0, 0, 0);
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert_eq!(kqpoll_init(p, 4), Ok(()));
        assert_eq!(p.p_kq_serial.get(), 0);
        assert_eq!(kq.nknotes(), 0);
        kqpoll_exit(p);
    }

    #[test]
    fn the_descriptor_table_and_the_hash_grow_on_demand() {
        let _u = userevents_shared();
        let p = setup(18);
        let (_kfd, kq) = new_kqueue(p);
        let (oid, _o) = obj(KlistKind::Kernel);
        let mut fds = Vec::new();
        while fds.last().copied().unwrap_or(0) < 300 {
            fds.push(open_obj(p, oid));
        }
        let last = *fds.last().unwrap() as usize;
        assert_eq!(reg(p, &kq, ev_set(3, EVFILT_READ, EV_ADD, 0, 0, 0)), Ok(()));
        assert_eq!(kq.lock.lock().knlistsize, KQEXTENT);
        assert_eq!(
            reg(p, &kq, ev_set(last, EVFILT_READ, EV_ADD, 0, 0, 0)),
            Ok(())
        );
        assert_eq!(kq.lock.lock().knlistsize, 2 * KQEXTENT, "grown past {last}");
        // The knote of descriptor 3 moved with its chain.
        assert_eq!(
            reg(p, &kq, ev_set(3, EVFILT_READ, EV_DELETE, 0, 0, 0)),
            Ok(())
        );
        assert_eq!(kq.lock.lock().knhashmask, 0);
        for id in 0..200usize {
            assert_eq!(
                reg(p, &kq, ev_set(id * 257, EVFILT_USER, EV_ADD, 0, 0, 0)),
                Ok(())
            );
        }
        assert_eq!(kq.lock.lock().knhashmask, KN_HASHSIZE as u64 - 1);
        assert_eq!(kq.nknotes(), 201);
        for id in 0..200usize {
            assert_eq!(
                reg(p, &kq, ev_set(id * 257, EVFILT_USER, EV_DELETE, 0, 0, 0)),
                Ok(())
            );
        }
        kqueue_purge(Some(p), &kq);
        assert_eq!(kq.nknotes(), 0);
    }

    #[test]
    fn a_kqueue_is_readable_by_another_while_events_are_pending() {
        let _u = userevents_shared();
        let p = setup(19);
        crate::task::taskq_init_once();
        let (wfd, watched) = new_kqueue(p);
        let (_kfd, watcher) = new_kqueue(p);
        let mut out = [Kevent::default(); 2];

        assert_eq!(
            reg(
                p,
                &watched,
                ev_set(wfd as usize, EVFILT_WRITE, EV_ADD, 0, 0, 0)
            ),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            reg(
                p,
                &watcher,
                ev_set(wfd as usize, EVFILT_READ, EV_ADD, 0, 0, 0)
            ),
            Ok(())
        );
        assert!(!watched.klist.is_empty());
        assert_eq!(scan(p, &watcher, &mut out), (0, Ok(())));

        // An event on the watched kqueue: its task activates the watcher's knote.
        assert_eq!(
            reg(
                p,
                &watched,
                ev_set(1, EVFILT_USER, EV_ADD, NOTE_TRIGGER, 0, 0)
            ),
            Ok(())
        );
        assert_eq!(watched.count(), 1);
        assert_eq!(watched.state() & KQ_TASK, KQ_TASK);
        kqueue_task::<Kg>(&watched.clone().erase());
        let mut n = (0, Ok(()));
        for _ in 0..1000 {
            n = scan(p, &watcher, &mut out);
            if n.0 == 1 {
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(n, (1, Ok(())));
        assert_eq!(
            (out[0].ident, out[0].filter, out[0].data),
            (wfd as usize, EVFILT_READ, 1)
        );

        // kqueue_stat: the pending count and the size of a kevent.
        let fp = fd_getfile(p.fd(), wfd).unwrap();
        assert_eq!(kqueue_stat::<Kg>(&fp), (1, 32));
        fp.frele(Some(p)).unwrap();

        // Closing the watched kqueue's descriptor removes the watcher's knote on it first.
        close_fd(p, wfd);
        assert!(watched.klist.is_empty());
        assert_eq!(watcher.nknotes(), 0);
        assert_eq!(watched.state() & KQ_DYING, KQ_DYING);
        assert_eq!(
            scan(p, &watched, &mut out),
            (0, Err(Errno::EBADF)),
            "a dying kqueue"
        );
    }

    #[test]
    fn proc_knotes_report_exit_fork_tracking_and_exec() {
        let p = setup(20);
        let (_kfd, kq) = new_kqueue(p);
        let child = thread_of(21);
        let cpr = child.process_ref().clone();
        let mut out = [Kevent::default(); 4];
        Kg::set_curproc(Some(p));

        let watch = ev_set(
            21,
            EVFILT_PROC,
            EV_ADD,
            NOTE_EXIT | NOTE_FORK | NOTE_EXEC | NOTE_TRACK,
            0,
            5,
        );
        assert_eq!(reg(p, &kq, watch), Ok(()));
        assert!(!cpr.ps_klist.is_empty());

        // NOTE_EXEC.
        knote_processexec(&cpr);
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].fflags), (21, NOTE_EXEC));

        // NOTE_FORK with NOTE_TRACK: a knote on the grandchild with NOTE_CHILD and the parent,
        // queued before the parent's (the C registers inside filt_proc, which then returns).
        let _gc = thread_of(22);
        knote_processfork(&cpr, 22);
        assert_eq!(scan(p, &kq, &mut out), (2, Ok(())));
        assert_eq!(
            (out[0].ident, out[0].fflags, out[0].data, out[0].udata),
            (22, NOTE_CHILD, 21, 5)
        );
        assert_eq!((out[1].ident, out[1].fflags), (21, NOTE_FORK));
        assert_eq!(kq.nknotes(), 2);

        // A fork to a pid that does not exist: NOTE_TRACKERR.
        knote_processfork(&cpr, 23);
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(out[0].fflags, NOTE_FORK | NOTE_TRACKERR);

        // NOTE_TRACK without NOTE_FORK: no fork event, but a failed registration is reported.
        let other = thread_of(24);
        let opr = other.process_ref().clone();
        assert_eq!(
            reg(p, &kq, ev_set(24, EVFILT_PROC, EV_ADD, NOTE_TRACK, 0, 6)),
            Ok(())
        );
        knote_processfork(&opr, 25);
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].fflags), (24, NOTE_TRACKERR));
        assert_eq!(
            reg(p, &kq, ev_set(24, EVFILT_PROC, EV_DELETE, 0, 0, 0)),
            Ok(())
        );

        // NOTE_EXIT: the exit status, EV_EOF, and the knote is gone once reported.
        cpr.ps_xexit.set(3);
        knote_processexit(&cpr);
        assert!(cpr.ps_klist.is_empty());
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(out[0].fflags, NOTE_EXIT);
        assert_eq!(out[0].data, i64::from(w_exitcode(3, 0)));
        assert_eq!(out[0].flags & (EV_EOF | EV_ONESHOT), EV_EOF | EV_ONESHOT);
        assert_eq!(kq.nknotes(), 1, "the grandchild's knote remains");

        // An exiting process cannot be watched.
        cpr.ps_flags.fetch_or(PS_EXITING, Ordering::Relaxed);
        assert_eq!(
            reg(p, &kq, ev_set(21, EVFILT_PROC, EV_ADD, NOTE_EXIT, 0, 0)),
            Err(Errno::ESRCH)
        );

        // pledge without "proc": EPERM through pledge_fail.
        p.process().ps_flags.fetch_or(PS_PLEDGE, Ordering::Relaxed);
        assert_eq!(
            reg(p, &kq, ev_set(20, EVFILT_PROC, EV_ADD, NOTE_EXIT, 0, 0)),
            Err(Errno::EPERM)
        );
        p.process()
            .ps_flags
            .fetch_and(!PS_PLEDGE, Ordering::Relaxed);
        Kg::set_curproc(None);
        kqueue_purge(Some(p), &kq);
    }

    #[test]
    fn signal_knotes_count_deliveries_and_go_with_the_process() {
        let p = setup(30);
        let (_kfd, kq) = new_kqueue(p);
        let mut out = [Kevent::default(); 2];
        Kg::set_curproc(Some(p));
        assert_eq!(
            reg(p, &kq, ev_set(30, EVFILT_SIGNAL, EV_ADD, 0, 0, 0)),
            Ok(())
        );
        Kg::set_curproc(None);
        let pr = p.process();
        for sig in [30i64, 31, 30] {
            let g = pr.ps_mtx.lock();
            knote_process_locked(pr, &g, i64::from(NOTE_SIGNAL) | sig);
        }
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(
            (out[0].ident, out[0].filter, out[0].data),
            (30, EVFILT_SIGNAL, 2)
        );
        assert_eq!(out[0].flags & EV_CLEAR, EV_CLEAR);
        // The process exits: its klist is invalidated and the signal knote dropped.
        knote_processexit(pr);
        assert!(pr.ps_klist.is_empty());
        assert_eq!(kq.nknotes(), 0);
    }

    #[test]
    fn invalidate_rewires_descriptor_knotes_to_dead_on_every_kind() {
        let p = setup(40);
        let (_kfd, kq) = new_kqueue(p);
        let mut out = [Kevent::default(); 2];
        for kind in [KlistKind::Kernel, KlistKind::Own, KlistKind::Owner] {
            let (oid, o) = obj(kind);
            let fd = open_obj(p, oid);
            assert_eq!(
                reg(p, &kq, ev_set(fd as usize, EVFILT_READ, EV_ADD, 0, 0, 0)),
                Ok(())
            );
            match kind {
                KlistKind::Owner => o.sel.si_note.invalidate_with(|| o.lock.lock()),
                _ => o.sel.si_note.invalidate(),
            }
            assert!(o.sel.si_note.is_empty());
            assert_eq!(scan(p, &kq, &mut out), (1, Ok(())), "{kind:?}");
            assert_eq!(out[0].flags & (EV_EOF | EV_ONESHOT), EV_EOF | EV_ONESHOT);
            assert_eq!(kq.nknotes(), 0);
            close_fd(p, fd);
        }
    }

    #[test]
    fn a_revoked_vnode_s_knote_turns_dead_in_the_scan() {
        let p = setup(41);
        let (_kfd, kq) = new_kqueue(p);
        let (oid, o) = obj(KlistKind::Kernel);
        let fd = open_obj(p, oid);
        o.ready.store(true, Ordering::Relaxed);
        let mut out = [Kevent::default(); 2];
        assert_eq!(
            reg(p, &kq, ev_set(fd as usize, EVFILT_READ, EV_ADD, 0, 0, 0)),
            Ok(())
        );
        let fp = fd_getfile(p.fd(), fd).unwrap();
        fp.data().unwrap().revoked.store(true, Ordering::Relaxed);
        fp.frele(Some(p)).unwrap();
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(out[0].flags & EV_EOF, EV_EOF);
        assert!(o.sel.si_note.is_empty(), "detached from its object");
        assert_eq!(kq.nknotes(), 0, "EV_ONESHOT dropped it");
        close_fd(p, fd);
    }

    #[test]
    fn two_scans_step_over_each_other_s_markers() {
        let _u = userevents_shared();
        let p = setup(42);
        let (_kfd, kq) = new_kqueue(p);
        for id in 1..=3usize {
            assert_eq!(
                reg(p, &kq, ev_set(id, EVFILT_USER, EV_ADD, NOTE_TRIGGER, 0, 0)),
                Ok(())
            );
        }
        // An outer scan's markers in the queue while an inner scan runs past them.
        let ostart = core::pin::pin!(Knote::marker(&kq));
        let oend = core::pin::pin!(Knote::marker(&kq));
        let outer = kqueue_scan_setup(&kq, ostart.as_ref(), oend.as_ref());
        {
            let mut g = kq.lock.lock();
            kq.head.insert_head(&mut g, outer.start);
            kq.head.insert_tail(&mut g, outer.end);
            outer.queued.set(true);
        }
        let mut out = [Kevent::default(); 8];
        assert_eq!(scan(p, &kq, &mut out), (3, Ok(())));
        let mut ids: Vec<usize> = out[..3].iter().map(|k| k.ident).collect();
        ids.sort_unstable();
        assert_eq!(ids, [1, 2, 3]);
        {
            let mut g = kq.lock.lock();
            kq.head.remove(&mut g, ostart.as_ref().get_ref());
        }
        drop(outer);
        assert_eq!(kq.lock.lock().nknotes, 3);
        kqueue_purge(Some(p), &kq);
    }

    #[test]
    fn a_processing_knote_makes_the_others_wait() {
        let _u = userevents_shared();
        let p = setup(43);
        let (_kfd, kq) = new_kqueue(p);
        assert_eq!(reg(p, &kq, ev_set(1, EVFILT_USER, EV_ADD, 0, 0, 0)), Ok(()));
        // Take the knote as a filter call would, then let another thread delete it.
        let h = {
            let mut g = kq.lock.lock();
            let kn = g
                .slot(false, 1)
                .and_then(OwnedList::first)
                .map(KnHold::of)
                .unwrap();
            knote_acquire(&mut g, kn).unwrap()
        };
        let done = AtomicBool::new(false);
        std::thread::scope(|s| {
            s.spawn(|| {
                testutil::install();
                let mut kev = ev_set(1, EVFILT_USER, EV_DELETE, 0, 0, 0);
                assert_eq!(kqueue_register(&kq, &mut kev, 0, None), Ok(()));
                done.store(true, Ordering::SeqCst);
            });
            std::thread::sleep(std::time::Duration::from_millis(50));
            assert!(
                !done.load(Ordering::SeqCst),
                "the delete waits for KN_PROCESSING"
            );
            let mut g = kq.lock.lock();
            assert_eq!(h.kn().status(&g) & KN_WAITING, KN_WAITING);
            knote_release(&mut g, h);
        });
        assert!(done.load(Ordering::SeqCst));
        assert_eq!(kq.nknotes(), 0);
    }

    #[test]
    fn a_new_klist_is_the_kernel_lock_s_and_the_items_are_their_measured_sizes() {
        // A new klist is the kernel lock's, empty (the byte view of its all-zero value is
        // `bsd`'s test: `sys/sys/event.rs`).
        let k: Klist<Kg> = Klist::new();
        assert_eq!(k.kind(), KlistKind::Kernel);
        assert!(k.is_empty());
        // vmstat -m's item sizes of the three pools (64-bit), pinned (decision 1).
        let _p = setup(44);
        let pools = &Kg::tables().kq;
        let sizes = [
            pools.knotepl.item_size().unwrap(),
            pools.kqueuepl.item_size().unwrap(),
            pools.kqtimerpl.item_size().unwrap(),
        ];
        // The C's knote is 120 bytes, LZ's about 200; the kqueue grew with its chains'
        // owners and the knote with its three entries and its counted members (the user's
        // decision of 2026-10-22: accepted, pinned here).
        assert_eq!(sizes, [288, 328, 88], "knotepl, kqueuepl, kqtimerpl");
    }

    // A knote, a kqueue and a klist are shared between CPUs by composition (no `unsafe impl`).
    const _: () = {
        const fn send_sync<T: Send + Sync>() {}
        send_sync::<Knote<Kg>>();
        send_sync::<Kqueue<Kg>>();
        send_sync::<Klist<Kg>>();
        send_sync::<FiltTimer<Kg>>();
        send_sync::<KqRef<Kg>>();
    };
}
/* </TESTS> */
