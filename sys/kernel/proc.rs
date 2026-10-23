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
 * Copyright (c) 1982, 1986, 1989, 1991, 1993
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
 *	@(#)kern_proc.c	8.4 (Berkeley) 1/4/94
 */

/*-
 * Copyright (c) 1986, 1989, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)proc.h	8.8 (Berkeley) 1/21/94
 */

/*
 * Copyright (c) 1982, 1986, 1989, 1991, 1993
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
 *	@(#)user.h	8.2 (Berkeley) 9/23/93
 */
/* </LICENSES> */

/* <CODE> */
//! Processes and threads: `struct process`, `struct proc`, `struct pgrp`, `struct session`
//! and `struct uidinfo` (`<sys/proc.h>`), the u-area's size (`<sys/user.h>`), and
//! `kern/kern_proc.c`: the process lists and hash tables, the process groups and sessions,
//! the per-user process counts.
//!
//! Upstream: sys/kern/kern_proc.c @ 3ce1f3f79392
//! Upstream: sys/sys/proc.h @ 3ce1f3f79392
//! Upstream: sys/sys/user.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_proc.rs@3c62ede68802
//! LZ: sys/sys/proc.rs@3c62ede68802
//! LZ: sys/sys/user.rs@3c62ede68802
//!
//! A process is a counted item of `processpl` ([`Process`], shared through
//! [`ProcRef`]s: its threads, its children's parent link, a waiter), a thread a boxed item of
//! `procpl` its process owns ([`Thread`], in the process's thread tree), a process group an
//! item of `pgrppl` its members hold, a session one of `sessionpl` its groups hold. Process 0
//! and its thread, group and session are `static`s (`process0`, `proc0`, `pgrp0`, `session0`
//! of `init_main.c`), so a reference to one of these is a [`StaticOrPooled`].
//!
//! The members that belong to subsystems still in `bsd` (the signal actions, the descriptors,
//! the kqueues, the klist, the timeouts, the tty) are the glue's: one value per object of a
//! type `bsd` chooses ([`ProcGlue`]: `ProcessG`, `ThreadG`, `PgrpG`, `SessionG`), which each
//! kernel type dereferences to, so `pr.ps_sigacts` names the glue's member as LZ named its
//! own. The run-queue link `p_runq` is a member of the thread itself (a `PinEntry` of the
//! scheduler's queues, `kernel::sched`, N4.5a).
//!
//! Locks, as the C's annotations: `[K]` the kernel lock (the lists, the hashes, the parent,
//! child and group links: [`hal::klock`]'s witnesses), `[m]` the process's `ps_mtx` (its thread
//! list and thread tree, its credentials, address space and limits: [`PsMtx`]), `[S]`
//! `sched_lock` (still `bsd`'s, decision D5: the scheduler's members are atomic words written
//! under it), `[a]` atomics, `[I]` set before the object is reachable. A thread's own `[o]`
//! members are atomic words, and its credentials an embedded copy ([`Thread::ucred`]).
//!
//! ## Deviations
//! - The thread's credentials are a copy of its process's (the user's decision of
//!   2026-10-21, shape A): `dorefreshcreds` copies the ids where the C swaps a pointer, and
//!   `procpl`'s item is about 100 bytes larger.
//! - `uid_find`'s allocation is `malloc(M_PROC, M_WAITOK)` of a value written whole (the C's
//!   `M_ZERO` and the stores after it), never freed (as the C's).
//! - `proc_printit`, `db_kill_cmd`, `db_stop_cmd`, `db_show_all_procs` (the real ddb) and
//!   `pgrpdump` (`DEBUG`) are not ported (LZ's gaps, kept).
//!
//! ## Redesign
//! - The lists and hash chains (`LIST_ENTRY`/`TAILQ_ENTRY` with raw links under an `unsafe
//!   impl Sync` saying "kernel lock") are `hal::pinlist::PinList`s of pinned elements bound to
//!   the kernel lock's identity and changed only with its exclusive witness
//!   (`hal::klock::witness`); lookups borrow a witness (`prfind(&r, pid)`), and a caller that
//!   keeps the process takes a handle (`PoolArc::from_ref`, `processpl` being recorded). A
//!   process or thread dropped while linked stops the kernel (the C's "unlink before
//!   `pool_put`" made a check). The adapters are `bsd`'s (`proc_glue_links!`), because the
//!   types are generic over the glue.
//! - The hash tables of `hashinit` are `MArray<PinList<_>>`s made by [`procinit`], in the
//!   glue's [`ProcTables`] (one per kernel; a host test thread's own), `uihashtbl` among them,
//!   under the `uidinfo` reader-writer lock ([`with_uidinfo`]).
//! - `struct process *ps_pptr`, `struct pgrp *ps_pgrp`, `struct session *pg_session` are counted
//!   handles in kernel-lock cells (a parent outlives no reference to it; a group lives while a
//!   member holds it, a session while a group does: `s_count` is that count); `s_leader` is the
//!   leader's address, compared, never followed.
//! - A thread's process link `p_p` is a counted handle (or process 0's `static`); a process
//!   owns its main thread's box in a slot (`PsMtx.main`, [`MainThread`], N4.6a: `fork1` sets
//!   it, `process_zap` takes it; the C's `ps_mainproc`, read under `ps_mtx` with
//!   [`Process::main_thread`]) and its other threads' boxes in a tree under `ps_mtx`
//!   (`hal::owned::OwnedTree` keyed by the order they joined, each taken back by its `exit1`),
//!   so a thread's memory goes with its box, never behind a raw `pool_put`; `ps_threads` is
//!   the list of the living ones, apart from the boxes.
//! - The pin tables (`ps_pin`, `ps_libcpin`) are owned [`PinTable`]s in
//!   `hal::slot::ReadMostly` cells (N4.6a), `ps_opptr` a counted handle under `ps_mtx`,
//!   `initprocess` the tables' (`ProcTables::initprocess`): no raw word left to dereference.
//! - `ps_ucred`, `ps_vmspace`, `ps_limit` are counted handles under `ps_mtx`, each with its
//!   address in a word for the C's unlocked compares (`refreshcreds`, `lim_read_enter`,
//!   `kinfo_proc`'s pointer fields).
//! - The members the C keeps as plain words are atomic words with LZ's `get()`/`set()`
//!   (`ps_xexit`, `p_estcpu`, ...); `p_wmesg` is a `hal::seqlock::StrSlot`; `p_comm`/`p_name`
//!   are three atomic words ([`NameWord`]).
//! - `p_stat`, `p_cpu`, `p_addr` and `p_md` are the thread's [`MachineSlot`], first in the
//!   `#[repr(C)]` thread, where both machines' assembly reaches them (decision P1).

use core::ops::Deref;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{
    AtomicI16, AtomicI32, AtomicI64, AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
    Ordering,
};

use hal::intr::{IPL_HIGH, IPL_NONE};
use hal::klock::{self, KernelWitness};
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray, MBox};
use hal::mutex::Mutexed;
use hal::once::OnceStatic;
use hal::owned::{OwnedAdapter, OwnedEntry, OwnedTree};
use hal::pin::pin_static;
use hal::pinlist::{PinAdapter, PinEntry, PinList};
use hal::pool::{BoxPool, PR_WAITOK, PoolArc, PoolBox, PoolHome, TypedPool};
use hal::refcnt::Refcnt;
use hal::rwlock::{Guarded, RawRwlock, Witness};
use hal::seqlock::StrSlot;
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::types::{Pid, Uid};
use hal::thread::MachineSlot;

use crate::assert::kpanic;
use crate::cred::{CredRef, Ucred};
use crate::descrip::{FdRef, Filedesc};
use crate::event::{Klist, Knote, KqPools, KqRef, Kqueue};
use crate::file::FileGlue;
use crate::resource::{LimitCache, PlimitRef, Rusage, TimespecWord, Tusage};
use crate::sig::{Sigacts, SigactsRef, SigaltstackWord, SigvalWord};
use crate::vm::map::{InentryCache, VmspaceRef};

/// `_MAXCOMLEN`: the command and thread names, NUL included.
pub const _MAXCOMLEN: usize = 24;
/// `LOGIN_NAME_MAX`: a session's login name, NUL included.
pub const LOGIN_NAME_MAX: usize = 32;
/// `M_PROC`: the malloc type of the hash tables and the uid entries.
pub const M_PROC: i32 = 41;

/// `SIGHUP`, sent by `orphanpg`.
const SIGHUP: i32 = 1;
/// `SIGCONT`, sent by `orphanpg`.
const SIGCONT: i32 = 19;

/*
 * These flags are kept in ps_flags.
 *
 * When adding a new flag, carefully consider whether it should be
 * added to PS_FLAGS_INHERITED_ON_FORK.
 */

/// `PS_CONTROLT`: has a controlling terminal.
pub const PS_CONTROLT: u32 = 0x0000_0001;
/// `PS_EXEC`: process called exec.
pub const PS_EXEC: u32 = 0x0000_0002;
/// `PS_INEXEC`: process is doing an exec right now.
pub const PS_INEXEC: u32 = 0x0000_0004;
/// `PS_EXITING`: process is exiting.
pub const PS_EXITING: u32 = 0x0000_0008;
/// `PS_SUGID`: had set id privs since last exec.
pub const PS_SUGID: u32 = 0x0000_0010;
/// `PS_SUGIDEXEC`: last execve() was set[ug]id.
pub const PS_SUGIDEXEC: u32 = 0x0000_0020;
/// `PS_PPWAIT`: parent waits for exec/exit.
pub const PS_PPWAIT: u32 = 0x0000_0040;
/// `PS_ISPWAIT`: is parent of PPWAIT child.
pub const PS_ISPWAIT: u32 = 0x0000_0080;
/// `PS_PROFIL`: has started profiling.
pub const PS_PROFIL: u32 = 0x0000_0100;
/// `PS_TRACED`: being ptraced.
pub const PS_TRACED: u32 = 0x0000_0200;
/// `PS_WAITED`: stopped proc was waited for.
pub const PS_WAITED: u32 = 0x0000_0400;
/// `PS_COREDUMP`: busy coredumping.
pub const PS_COREDUMP: u32 = 0x0000_0800;
/// `PS_SINGLEEXIT`: other threads must die.
pub const PS_SINGLEEXIT: u32 = 0x0000_1000;
/// `PS_SINGLEUNWIND`: other threads must unwind.
pub const PS_SINGLEUNWIND: u32 = 0x0000_2000;
/// `PS_NOZOMBIE`: no signal or zombie at exit.
pub const PS_NOZOMBIE: u32 = 0x0000_4000;
/// `PS_STOPPING`: just stopped, need sig to parent.
pub const PS_STOPPING: u32 = 0x0000_8000;
/// `PS_SYSTEM`: no sigs, stats or swapping.
pub const PS_SYSTEM: u32 = 0x0001_0000;
/// `PS_EMBRYO`: new process, not yet fledged.
pub const PS_EMBRYO: u32 = 0x0002_0000;
/// `PS_ZOMBIE`: dead and ready to be waited for.
pub const PS_ZOMBIE: u32 = 0x0004_0000;
/// `PS_NOBROADCASTKILL`: process excluded from kill -1.
pub const PS_NOBROADCASTKILL: u32 = 0x0008_0000;
/// `PS_PLEDGE`: has called pledge(2).
pub const PS_PLEDGE: u32 = 0x0010_0000;
/// `PS_EXECPLEDGE`: has exec pledges.
pub const PS_EXECPLEDGE: u32 = 0x0040_0000;
/// `PSI_WXNEEDED`: process allowed to violate W^X (`ps_iflags`).
pub const PSI_WXNEEDED: u16 = 0x0001;
/// `PSI_NOBTCFI`: no Branch Target CFI.
pub const PSI_NOBTCFI: u16 = 0x0002;
/// `PSI_PROFILE`: linked with -pg: allow profile(2).
pub const PSI_PROFILE: u16 = 0x0004;

/// `BOGO_PC`: an address that can't be in userspace or kernelspace (`ps_kbind_addr`).
pub const BOGO_PC: usize = usize::MAX;

/// `PS_ORPHAN`: process is on an orphan list.
pub const PS_ORPHAN: u32 = 0x0080_0000;
/// `PS_CHROOT`: process is chrooted.
pub const PS_CHROOT: u32 = 0x0100_0000;
/// `PS_ITIMER`: virtual interval timers running.
pub const PS_ITIMER: u32 = 0x0400_0000;
/// `PS_WAITEVENT`: wait(2) event pending.
pub const PS_WAITEVENT: u32 = 0x1000_0000;
/// `PS_CONTINUED`: continued proc not yet waited for.
pub const PS_CONTINUED: u32 = 0x2000_0000;
/// `PS_STOPPED`: stopped process.
pub const PS_STOPPED: u32 = 0x4000_0000;
/// `PS_TRAPPED`: stopped due to tracing event.
pub const PS_TRAPPED: u32 = 0x8000_0000;

/// `PS_BITS`: the `%b` format of `ps_flags`.
pub const PS_BITS: &[u8] = b"\x20\x01CONTROLT\x02EXEC\x03INEXEC\x04EXITING\x05SUGID\x06SUGIDEXEC\x07PPWAIT\x08ISPWAIT\x09PROFIL\x0aTRACED\x0bWAITED\x0cCOREDUMP\x0dSINGLEEXIT\x0eSINGLEUNWIND\x0fNOZOMBIE\x10STOPPING\x11SYSTEM\x12EMBRYO\x13ZOMBIE\x14NOBROADCASTKILL\x15PLEDGE\x17EXECPLEDGE\x18ORPHAN\x19CHROOT\x1bITIMER\x1dWAITEVENT\x1eCONTINUED\x1fSTOPPED\x20TRAPPED";

/// `PS_FLAGS_INHERITED_ON_FORK`.
pub const PS_FLAGS_INHERITED_ON_FORK: u32 =
    PS_SUGID | PS_SUGIDEXEC | PS_PLEDGE | PS_EXECPLEDGE | PS_CHROOT;

/* Status values. */

/// `SIDL`: thread being created by fork.
pub const SIDL: u8 = 1;
/// `SRUN`: currently runnable.
pub const SRUN: u8 = 2;
/// `SSLEEP`: sleeping on an address.
pub const SSLEEP: u8 = 3;
/// `SSTOP`: debugging or suspension.
pub const SSTOP: u8 = 4;
/// `SZOMB`: unused.
pub const SZOMB: u8 = 5;
/// `SDEAD`: thread is almost gone.
pub const SDEAD: u8 = 6;
/// `SONPROC`: thread is currently on a CPU.
pub const SONPROC: u8 = 7;

// The switch stores `hal::thread`'s `SONPROC` into the new thread's slot.
const _: () = assert!(hal::thread::SONPROC == SONPROC);

/*
 * These flags are per-thread and kept in p_flag
 */

/// `P_INKTR`: in a ktrace op, don't recurse.
pub const P_INKTR: i32 = 0x0000_0001;
/// `P_PROFPEND`: SIGPROF needs to be posted.
pub const P_PROFPEND: i32 = 0x0000_0002;
/// `P_ALRMPEND`: SIGVTALRM needs to be posted.
pub const P_ALRMPEND: i32 = 0x0000_0004;
/// `P_SIGSUSPEND`: need to restore before-suspend mask.
pub const P_SIGSUSPEND: i32 = 0x0000_0008;
/// `P_CANTSLEEP`: insomniac thread.
pub const P_CANTSLEEP: i32 = 0x0000_0010;
/// `P_INSCHED`: switching scheduler state.
pub const P_INSCHED: i32 = 0x0000_0020;
/// `P_SINTR`: sleep is interruptible.
pub const P_SINTR: i32 = 0x0000_0080;
/// `P_SYSTEM`: no sigs, stats or swapping.
pub const P_SYSTEM: i32 = 0x0000_0200;
/// `P_TIMEOUT`: timing out during sleep.
pub const P_TIMEOUT: i32 = 0x0000_0400;
/// `P_TIMEOUTRAN`: timeout handler has finished.
pub const P_TIMEOUTRAN: i32 = 0x0000_0800;
/// `P_TRACESINGLE`: ptrace: keep single threaded.
pub const P_TRACESINGLE: i32 = 0x0000_1000;
/// `P_WEXIT`: working on exiting.
pub const P_WEXIT: i32 = 0x0000_2000;
/// `P_OWEUPC`: owe proc an addupc() at next ast.
pub const P_OWEUPC: i32 = 0x0000_8000;
/// `P_SUSPSINGLE`: need to stop for single threading.
pub const P_SUSPSINGLE: i32 = 0x0008_0000;
/// `P_THREAD`: only a thread, not a real process.
pub const P_THREAD: i32 = 0x0400_0000;
/// `P_SUSPSIG`: stopped from signal.
pub const P_SUSPSIG: i32 = 0x0800_0000;
/// `P_CPUPEG`: do not move to another cpu.
pub const P_CPUPEG: i32 = 0x4000_0000;

/// `P_BITS`: the `%b` format of `p_flag`.
pub const P_BITS: &[u8] = b"\x20\x01INKTR\x02PROFPEND\x03ALRMPEND\x04SIGSUSPEND\x05CANTSLEEP\x06INSCHED\x08SINTR\x0aSYSTEM\x0bTIMEOUT\x0cTIMEOUTRAN\x0dTRACESINGLE\x0eWEXIT\x10OWEUPC\x14SUSPSINGLE\x1bTHREAD\x1cSUSPSIG\x1fCPUPEG";

/// `THREAD_PID_OFFSET`.
pub const THREAD_PID_OFFSET: Pid = 100_000;

/*
 * We use process IDs <= PID_MAX; PID_MAX + 1 must also fit in a pid_t,
 * as it is used to represent "no process group".
 * We set PID_MAX to 99999 to keep it in 5 columns in ps
 * When exposed to userspace, thread IDs have THREAD_PID_OFFSET
 * added to keep them from overlapping the PID range.  For them,
 * we use a * a (0 .. 2^n] range for cheapness, picking 'n' such
 * that 2^n + THREAD_PID_OFFSET and THREAD_PID_OFFSET have
 * the same number of columns when printed.
 */

/// `PID_MAX`.
pub const PID_MAX: Pid = 99_999;
/// `TID_MASK`.
pub const TID_MASK: Pid = 0x7ffff;

/// `NO_PID`.
pub const NO_PID: Pid = PID_MAX + 1;

/*
 * Flags to fork1().
 */

/// `FORK_FORK`.
pub const FORK_FORK: i32 = 0x0000_0001;
/// `FORK_VFORK`.
pub const FORK_VFORK: i32 = 0x0000_0002;
/// `FORK_IDLE`.
pub const FORK_IDLE: i32 = 0x0000_0004;
/// `FORK_PPWAIT`.
pub const FORK_PPWAIT: i32 = 0x0000_0008;
/// `FORK_SHAREFILES`.
pub const FORK_SHAREFILES: i32 = 0x0000_0010;
/// `FORK_SYSTEM`.
pub const FORK_SYSTEM: i32 = 0x0000_0020;
/// `FORK_NOZOMBIE`.
pub const FORK_NOZOMBIE: i32 = 0x0000_0040;
/// `FORK_SHAREVM`.
pub const FORK_SHAREVM: i32 = 0x0000_0080;
/// `FORK_PTRACE`.
pub const FORK_PTRACE: i32 = 0x0000_0400;

/// `EXIT_NORMAL`.
pub const EXIT_NORMAL: i32 = 0x0000_0001;
/// `EXIT_THREAD`.
pub const EXIT_THREAD: i32 = 0x0000_0002;
/// `EXIT_THREAD_NOCHECK`.
pub const EXIT_THREAD_NOCHECK: i32 = 0x0000_0003;

/// `SINGLE_SUSPEND`: other threads to stop wherever they are.
pub const SINGLE_SUSPEND: i32 = 0x01;
/// `SINGLE_UNWIND`: other threads to unwind and stop.
pub const SINGLE_UNWIND: i32 = 0x02;
/// `SINGLE_EXIT`: other threads to unwind and then exit.
pub const SINGLE_EXIT: i32 = 0x03;
/// `SINGLE_MASK`.
pub const SINGLE_MASK: i32 = 0x0f;
/// `SINGLE_DEEP`: call is in deep (extra flag for `single_thread_set`).
pub const SINGLE_DEEP: i32 = 0x10;

/// Declares a word member: a relaxed atomic with LZ's `get()`/`set()`, for a member the C
/// keeps as a plain word written under the lock its annotation names (or by its owner) and
/// read where no lock is held (decision 12).
macro_rules! word {
    ($(#[$m:meta])* $name:ident, $atomic:ty, $t:ty) => {
        $(#[$m])*
        #[derive(Debug, Default)]
        #[repr(transparent)]
        pub struct $name($atomic);

        impl $name {
            /// `v`.
            pub const fn new(v: $t) -> Self {
                Self(<$atomic>::new(v))
            }

            /// The value.
            #[inline]
            pub fn get(&self) -> $t {
                self.0.load(Ordering::Relaxed)
            }

            /// Stores `v`.
            #[inline]
            pub fn set(&self, v: $t) {
                self.0.store(v, Ordering::Relaxed);
            }
        }
    };
}

word!(
    /// A `u_char` member.
    U8Word,
    AtomicU8,
    u8
);
word!(
    /// A `u_short` member.
    U16Word,
    AtomicU16,
    u16
);
word!(
    /// A `short` member.
    I16Word,
    AtomicI16,
    i16
);
word!(
    /// A `u_int` member.
    U32Word,
    AtomicU32,
    u32
);
word!(
    /// An `int` member (`pid_t` included).
    I32Word,
    AtomicI32,
    i32
);
word!(
    /// A `uint64_t` member.
    U64Word,
    AtomicU64,
    u64
);
word!(
    /// An `int64_t`/`long` member.
    I64Word,
    AtomicI64,
    i64
);
word!(
    /// A `vaddr_t`/`size_t` member, or an address kept as a number.
    UsizeWord,
    AtomicUsize,
    usize
);

/// A command or thread name (`ps_comm`, `p_name`: `char [_MAXCOMLEN]`), read as a copy that
/// dereferences to its bytes up to the NUL.
#[derive(Clone, Copy)]
pub struct Name {
    bytes: [u8; _MAXCOMLEN],
}

impl Deref for Name {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        let n = self
            .bytes
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.bytes.len());
        &self.bytes[..n]
    }
}

/// `char ps_comm[_MAXCOMLEN]`: three words, written whole by `set` (its owner, under the kernel
/// lock as the C) and read as a copy by anyone.
#[derive(Debug, Default)]
pub struct NameWord([AtomicU64; 3]);

impl NameWord {
    /// The empty name.
    pub const fn new() -> Self {
        Self([const { AtomicU64::new(0) }; 3])
    }

    /// The name.
    pub fn get(&self) -> Name {
        let mut bytes = [0u8; _MAXCOMLEN];
        for (i, w) in self.0.iter().enumerate() {
            bytes[i * 8..i * 8 + 8].copy_from_slice(&w.load(Ordering::Relaxed).to_ne_bytes());
        }
        Name { bytes }
    }

    /// `strlcpy(name, s, _MAXCOMLEN)` (`memset` for an empty `s`).
    pub fn set(&self, s: &[u8]) {
        let mut bytes = [0u8; _MAXCOMLEN];
        let n = s.len().min(_MAXCOMLEN - 1);
        bytes[..n].copy_from_slice(&s[..n]);
        for (i, w) in self.0.iter().enumerate() {
            let mut b = [0u8; 8];
            b.copy_from_slice(&bytes[i * 8..i * 8 + 8]);
            w.store(u64::from_ne_bytes(b), Ordering::Relaxed);
        }
    }
}

/// `struct p_inentry p_spinentry`: the thread's own cache for `uvm_map_inentry`, three words
/// read and written whole by `get()`/`set()` by the thread itself.
#[derive(Debug, Default)]
pub struct InentryWord {
    start: AtomicUsize,
    end: AtomicUsize,
    serial: AtomicU64,
}

impl InentryWord {
    /// An empty range.
    pub const fn new() -> Self {
        Self {
            start: AtomicUsize::new(0),
            end: AtomicUsize::new(0),
            serial: AtomicU64::new(0),
        }
    }

    /// The cache.
    #[inline]
    pub fn get(&self) -> InentryCache {
        InentryCache {
            start: self.start.load(Ordering::Relaxed),
            end: self.end.load(Ordering::Relaxed),
            serial: self.serial.load(Ordering::Relaxed),
        }
    }

    /// Stores `c`.
    #[inline]
    pub fn set(&self, c: InentryCache) {
        self.start.store(c.start, Ordering::Relaxed);
        self.end.store(c.end, Ordering::Relaxed);
        self.serial.store(c.serial, Ordering::Relaxed);
    }
}

/// A counted reference to a process, group or session, or to one of process 0's `static`s
/// (`process0`, `pgrp0`, `session0` of `init_main.c`), which no pool owns.
pub enum StaticOrPooled<T: Send + Sync + 'static> {
    /// One of process 0's.
    Static(&'static T),
    /// A pool item, counted.
    Pooled(PoolArc<T>),
}

impl<T: Send + Sync + 'static> StaticOrPooled<T> {
    /// The value, pinned: a `static` never moves, a pool item stays until its last handle.
    pub fn as_pin(&self) -> Pin<&T> {
        match self {
            Self::Static(r) => pin_static(r),
            Self::Pooled(a) => a.as_pin(),
        }
    }

    /// The value's address, the C's pointer (compared, printed by `kinfo_proc`).
    pub fn addr(&self) -> usize {
        ptr::from_ref::<T>(&**self).addr()
    }

    /// Whether two references name the same object.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.addr() == b.addr()
    }
}

impl<T: Send + Sync + 'static> Clone for StaticOrPooled<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Static(r) => Self::Static(r),
            Self::Pooled(a) => Self::Pooled(a.clone()),
        }
    }
}

impl<T: Send + Sync + 'static> Deref for StaticOrPooled<T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Self::Static(r) => r,
            Self::Pooled(a) => a,
        }
    }
}

/// The main thread of a process (`ps_mainproc`), owned by its process: process 0's `static`
/// thread, or the box of the thread `fork1` made, which the process keeps from `fork1` until
/// `process_zap` takes it (`proc_free`). The process's other threads are owned by its tree
/// ([`PsMtx`]), each until its own `exit1`.
pub enum MainThread<G: ProcGlue> {
    /// None yet (a process being made), or none any more (`process_zap` took it).
    None,
    /// Process 0's `static` thread (`proc0`).
    Static(Pin<&'static Thread<G>>),
    /// A `procpl` box.
    Owned(PoolBox<Thread<G>>),
}

impl<G: ProcGlue> MainThread<G> {
    /// The thread, pinned (a `static`, or a pool item that stays until its box goes).
    pub fn get(&self) -> Option<Pin<&Thread<G>>> {
        match self {
            Self::None => None,
            Self::Static(t) => Some(*t),
            Self::Owned(b) => Some(b.as_pin()),
        }
    }
}

/// `struct pinsyscall`: the system call pin table of a text region (`ps_pin`, the program's
/// or ld.so's from `exec`; `ps_libcpin`, libc's from `pinsyscalls(2)`): `[start, end)` and
/// the offset of each system call's entry, owned (`M_PINSYSCALL`).
pub struct PinTable {
    /// `pn_start`.
    pub start: usize,
    /// `pn_end`.
    pub end: usize,
    /// `pn_pins` and `pn_npins`: the offsets by system call number, none for no table.
    pub pins: Option<MArray<u32>>,
}

impl PinTable {
    /// No table (`pn_pins == NULL`, the bounds 0).
    pub const NONE: PinTable = PinTable {
        start: 0,
        end: 0,
        pins: None,
    };

    /// `pn_npins`.
    pub fn npins(&self) -> usize {
        self.pins.as_ref().map_or(0, |p| p.len())
    }

    /// `process_new`'s copy (`mallocarray(npins, sizeof(u_int), M_PINSYSCALL, M_WAITOK)` and
    /// `memcpy`) of the table in `cell`: the same bounds and a table of its own; the bounds
    /// alone when the copy cannot be had (the C's `M_WAITOK` cannot fail; a child left without
    /// a table is killed by `pin_check` at its first system call, as a process without one
    /// would be). The allocation, which may sleep, is made with no borrow of the cell held
    /// (a reader of a `ReadMostly` does not sleep); the copy is made under a borrow, again if
    /// the table changed size meanwhile.
    pub fn duplicate(cell: &ReadMostly<PinTable>) -> PinTable {
        loop {
            let n = cell.read().pins.as_ref().map(|p| p.len());
            let mut dst =
                n.and_then(|n| MArray::new(crate::vm::mmap::M_PINSYSCALL, M_WAITOK, n, |_| 0u32));
            let src = cell.read();
            if src.pins.as_ref().map(|p| p.len()) != n {
                continue;
            }
            if let (Some(d), Some(s)) = (dst.as_mut(), src.pins.as_ref()) {
                d.copy_from_slice(s);
            }
            return PinTable {
                start: src.start,
                end: src.end,
                pins: dst,
            };
        }
    }
}

/// A counted reference to a process (`struct process *` with a reference).
pub type ProcessRef<G> = StaticOrPooled<Process<G>>;
/// A counted reference to a process group.
pub type PgrpRef<G> = StaticOrPooled<Pgrp<G>>;
/// A counted reference to a session (`s_count`).
pub type SessionRef<G> = StaticOrPooled<Session<G>>;

/// What the processes need of the subsystems still in `bsd` (decision 9): the member types
/// each kernel type carries for them (and dereferences to), the list adapters (which `bsd`
/// declares with [`proc_glue_links!`], the types being generic), the tables, and the calls
/// `kern_proc.c` makes into them. One instantiation, `bsd`'s (`sys/kern/proc_glue.rs`); a
/// bridge of `bridges.toml` while `bsd` implements it.
pub trait ProcGlue: Sized + Send + Sync + 'static {
    /// The members of `struct process` of `bsd`'s subsystems (`ps_textvp`, the unveil table,
    /// the timeouts, the interval timers); `Default` is a new process's (`fork1`).
    type ProcessG: Send + Sync + Default;
    /// The members of `struct proc` of `bsd`'s subsystems; `Default` is a new thread's.
    type ThreadG: Send + Sync + Default;
    /// The members of `struct pgrp` of `bsd`'s (none since N4.5b: `pg_sigiolst` is
    /// `kernel::sigio`'s registry); `Default` is a new group's (`setpgid(2)`, `setsid(2)`).
    type PgrpG: Send + Sync + Default;
    /// The members of `struct session` of `bsd`'s (`s_ttyvp`, `s_ttyp`, `s_verauthto`);
    /// `Default` is a new session's (`setsid(2)`).
    type SessionG: Send + Sync + Default;
    /// `p_runq`: the scheduler's queues (`kernel::sched`).
    type ThreadRunq: PinAdapter<Elem = Thread<Self>>;
    /// `ps_list`: `allprocess`, `zombprocess`.
    type ProcessList: PinAdapter<Elem = Process<Self>>;
    /// `ps_hash`: the pid hash chains.
    type ProcessHash: PinAdapter<Elem = Process<Self>>;
    /// `ps_pglist`: a group's members.
    type ProcessPglist: PinAdapter<Elem = Process<Self>>;
    /// `ps_sibling`: a parent's children.
    type ProcessSibling: PinAdapter<Elem = Process<Self>>;
    /// `ps_orphan`: a debugger's orphans.
    type ProcessOrphan: PinAdapter<Elem = Process<Self>>;
    /// `p_list`: `allproc`.
    type ThreadList: PinAdapter<Elem = Thread<Self>>;
    /// `p_hash`: the tid hash chains.
    type ThreadHash: PinAdapter<Elem = Thread<Self>>;
    /// `p_thr_link`: a process's threads, in the order they joined.
    type ThreadLink: PinAdapter<Elem = Thread<Self>>;
    /// The owner of a process's thread boxes, by the order they joined.
    type ThreadOwned: OwnedAdapter<Elem = Thread<Self>>;
    /// `pg_hash`: the pgrp hash chains.
    type PgrpHash: PinAdapter<Elem = Pgrp<Self>>;
    /// The descriptors of this glue's processes (`ps_fd`, `kernel::descrip`).
    type FileGlue: FileGlue<Proc = Self>;
    /// `kn_link`: the chain of a kqueue that owns a knote (`kernel::event`).
    type KnLink: OwnedAdapter<Elem = Knote<Self>>;
    /// `kn_tqe`: a kqueue's pending queue.
    type KnTqe: PinAdapter<Elem = Knote<Self>>;
    /// `kn_selnext`: an object's klist.
    type KnSelnext: PinAdapter<Elem = Knote<Self>>;
    /// `kq_next`: a table's `fd_kqlist`.
    type KqNext: PinAdapter<Elem = Kqueue<Self>>;

    /// The lists, hashes and pools of this glue's processes (one `static` of `bsd`'s).
    fn tables() -> &'static ProcTables<Self>;
    /// The scheduler's queues and CPUs of this glue's threads (one `static` of `bsd`'s).
    fn sched() -> &'static crate::sched::SchedTables<Self>;

    // The scheduler's calls into what is still `bsd`'s (`kernel::sched`). The defaults are
    // what a glue without that subsystem does (the kernel's own tests): nothing, or the
    // answer of a machine with nothing else running.

    /// `cold || panicstr`: a sleep returns at once.
    fn cold() -> bool {
        false
    }
    /// That early return's work: `splhigh`, `splx(safepri)`, the kernel lock bounced.
    fn cold_yield() {}
    /// `tuagg_add_runtime()` (`kern_resource.c`): the running thread's run time accounted.
    fn tuagg_add_runtime() {}
    /// The optional clock interrupts of the running CPU stopped (`clockintr_cancel` of
    /// `spc_itimer` and `spc_profclock`, as asked).
    fn clocks_stop(_itimer: bool, _profclock: bool) {}
    /// The optional clock interrupts of the running thread started (`clockintr_advance`, as
    /// asked) and its start time recorded (`spc_runtime = nanouptime()`).
    fn clocks_start(_itimer: bool, _profclock: bool) {}
    /// `assertwaitok()` (`subr_xxx.c`).
    fn assertwaitok() {}
    /// `smr_idle()` (`kern_smr.c`): this CPU passes a quiescent state.
    fn smr_idle() {}
    /// `spl0()`.
    fn spl0() {}
    /// `exit2`'s accounting of a dead thread (`tuagg_add_process` under `ps_mtx`).
    fn exit_account(_p: &Thread<Self>) {}
    /// `cpu_unidle(ci)`: CPU `n`, idle, is kicked out of its wait.
    fn cpu_unidle(_n: usize) {}
    /// `need_resched(ci)`: CPU `n` reschedules at its next chance (its AST, a kick).
    fn need_resched(_n: usize) {}
    /// `cpu_idle_enter()`.
    fn idle_enter() {}
    /// `cpu_idle_cycle()`: waits for an interrupt.
    fn idle_cycle() {
        core::hint::spin_loop();
    }
    /// `cpu_idle_leave()`.
    fn idle_leave() {}
    /// `CPU_IS_PRIMARY(ci)` of CPU `n`.
    fn cpu_is_primary(n: usize) -> bool {
        n == 0
    }
    /// `CPU_IS_RUNNING(ci)` of CPU `n`.
    fn cpu_is_running(_n: usize) -> bool {
        true
    }
    /// `ci->ci_cputype` of CPU `n` (`CPUTYP_*`, 0: no topology known).
    fn cputype(_n: usize) -> i32 {
        0
    }
    /// `pmap_resident_count(p->p_vmspace->vm_map.pmap)`, read without `ps_mtx` (the
    /// scheduler's cost estimate holds `SCHED_LOCK`, which `ps_mtx` comes before).
    fn resident_count(_p: &Thread<Self>) -> Option<i64> {
        None
    }
    /// `stathz`.
    fn stathz() -> i32 {
        100
    }
    /// `pgdelete`'s call out (after `kernel::sigio`'s `sigio_freelist`): when the session's
    /// terminal names this group, `tp->t_pgrp = NULL`.
    fn pgrp_destroy(pg: &Pgrp<Self>);
    /// `killjobc`'s controlling-terminal part, for a session leader whose session has a
    /// terminal vnode: the foreground group hung up, the terminal drained and revoked, the
    /// vnode released and `s_ttyvp` cleared. It may sleep: no witness is out.
    fn session_hangup(sess: &Session<Self>);
    /// A session placed in its pool item: `timeout_set(&s_verauthto, zapverauth, sess)`.
    fn session_placed(sess: &Session<Self>);
    /// The last reference to a session dropped (`SESSRELE`'s `timeout_del(&s_verauthto)`).
    fn session_destroy(sess: &Session<Self>);
    /// `curproc`, lent to `f`: the thread running on this CPU (`None` before the first one,
    /// and on a host test thread that set none). The kernel's glue reads hal's
    /// (`hal::thread::try_with_current`, the thread type `kernel::sched` switches); the test
    /// glues of other thread types keep their own, since hal admits one thread type per
    /// process. The reference lives for `f` alone: nothing claims the thread past it.
    fn with_curproc<R>(f: impl FnOnce(Option<&Thread<Self>>) -> R) -> R;
    /// `pledge_fail(p, error, code)`: reports a pledge violation of `p` and returns the
    /// error the system call fails with (`kern_pledge`'s, N4.7).
    fn pledge_fail(
        p: &Thread<Self>,
        error: hal::sys::errno::Errno,
        code: u64,
    ) -> hal::sys::errno::Errno;
    /// A counted reference to `pr` (process 0's `static`, or a `processpl` item), for a
    /// holder that keeps a process found under a witness past it (`filt_procattach`).
    fn process_ref(pr: &Process<Self>) -> ProcessRef<Self>;

    // The signals' calls into what is still `bsd`'s (`kernel::sig`, N4.5b). The defaults are
    // what a glue without user mode does (the kernel's own tests).

    /// `exit1(p, xexit, xsig, flags)` (`kern_exit.c`, N4.6): the thread (or its process)
    /// exits; never returns.
    fn exit1(p: &Thread<Self>, xexit: i32, xsig: i32, flags: i32) -> ! {
        crate::exit::exit1(p, xexit, xsig, flags)
    }
    /// `coredump(p)` (the file systems, N5): whether a core was written.
    fn coredump(_p: &Thread<Self>) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
    /// `sendsig(catcher, sig, mask, ksip, info, onstack)` (`machdep.c`, N4.8): the signal
    /// frame of `p`, the running thread, built on its user stack for `catcher`.
    #[allow(
        clippy::too_many_arguments,
        reason = "the C's six arguments and the thread"
    )]
    fn sendsig(
        _p: &Thread<Self>,
        _catcher: hal::sys::signal::Sig,
        _sig: i32,
        _mask: hal::sys::signal::Sigset,
        _ksip: &hal::sys::siginfo::Siginfo,
        _info: bool,
        _onstack: bool,
    ) -> Result<(), Errno> {
        Ok(())
    }
    /// `signotify(p)` (`<machine/cpu.h>`, N4.8): the running thread `p` notices its signal at
    /// its next return to user mode (an AST, and a kick of its CPU).
    fn signotify(_p: &Thread<Self>) {}
    /// `PROC_STACK(p)` (N4.8): the user stack pointer of `p`'s trap frame.
    fn proc_stack(_p: &Thread<Self>) -> usize {
        0
    }
    /// `TCB_GET(p)` (N4.8): `p`'s thread control block address.
    fn tcb_get(_p: &Thread<Self>) -> usize {
        0
    }
    // The process code's calls into what is still `bsd`'s (`kernel::fork`, `kernel::exit`,
    // N4.6a). The defaults are what a glue without that subsystem does (the kernel's tests).

    /// `maxprocess` (`conf/param.c`, `kern.maxproc`).
    fn maxprocess() -> i32 {
        i32::MAX
    }
    /// `maxthread` (`kern.maxthread`).
    fn maxthread() -> i32 {
        i32::MAX
    }
    /// `log(level, fmt, ...)` (`subr_log.c`): `tablefull`'s and accounting's messages.
    fn log(_level: i32, _args: core::fmt::Arguments<'_>) {}
    /// A new thread is linked (`fork1`, `thread_fork`): `bsd`'s record of its `&'static`,
    /// which its run, sleep and signal adapters lend (N4.8 retires it with `curproc`'s).
    fn thread_placed(_p: Pin<&Thread<Self>>) {}
    /// `process_initialize`'s timeouts of `bsd`'s (`ps_realit_to`, `kern_time.c`, N4.8a).
    fn process_initialized(_pr: &Process<Self>) {}
    /// `process_new`'s `bsd` members: the text vnode's reference (`vref`, N5) and the
    /// unveils (`unveil_copy`, N8).
    fn process_forked(_parent: &Process<Self>, _child: &Process<Self>) {}
    /// `sess->s_ttyvp != NULL` (the session's controlling terminal vnode, N4.7b).
    fn session_has_tty_vnode(_sess: &Session<Self>) -> bool {
        false
    }
    /// `sess->s_ttyp->t_dev` (accounting's `ac_tty`, N4.7b), `None` without a terminal.
    fn session_ttydev(_sess: &Session<Self>) -> Option<hal::sys::types::Dev> {
        None
    }
    /// `pmap_activate(p)` of the machine (`cpu_fork`'s, for the child).
    fn pmap_activate(_p: &Thread<Self>) {}
    /// `pmap_deactivate(p)` of the machine (`exit1`'s, on the running thread).
    fn pmap_deactivate(_p: &Thread<Self>) {}
    /// `pmap_purge(p)` of the machine (`uvm_purge`'s, `__HAVE_PMAP_PURGE`).
    fn pmap_purge(_p: &Thread<Self>) {}
    /// `child_return(p)` of the machine (N4.8): the forked child's return values set, the
    /// kernel lock its trampoline took released, `mi_child_return`.
    fn child_return(_p: &Thread<Self>) {}
    /// `start_init(p)` (`init_main.c`, N9.1): process 1 execs init.
    fn start_init(_p: &Thread<Self>) {}
    /// `TCB_INVALID(tcb)` of the machine (`__tfork(2)`, N4.8).
    fn tcb_invalid(_tcb: usize) -> bool {
        false
    }
    /// init's `exit1` (pid 1, `EXIT_NORMAL`): the C panics ("init died"); under QEMU `bsd`
    /// ends the run instead.
    fn init_exited(xexit: i32, xsig: i32) -> ! {
        kpanic(format_args!("init died (signal {xsig}, exit {xexit})"))
    }
    /// `cancel_all_itimers()` (`kern_time.c`, N4.8a): the exiting process's interval timers.
    fn cancel_all_itimers() {}
    /// `unveil_destroy(pr)` (`kern_unveil.c`, N8).
    fn unveil_destroy(_pr: &Process<Self>) {}
    /// `process_zap`'s `vrele` of the text vnode (`ps_textvp`, N5).
    fn textvp_release(_pr: &Process<Self>) {}
    /// `TCB_SET(p, tcb)` of the machine (`__set_tcb(2)`, N4.8).
    fn tcb_set(_p: &Thread<Self>, _tcb: usize) {}
    /// `rtable_exists(id)` (`net/rtable.c`, N6: `setrtable(2)`).
    fn rtable_exists(id: u32) -> bool {
        id == 0
    }
    /// A counted reference to the group `pg` (process 0's `static`, or a `pgrppl` item), for
    /// a holder that keeps a group found under a witness past it (`setpgid(2)`).
    fn pgrp_ref(pg: &Pgrp<Self>) -> PgrpRef<Self> {
        match PoolArc::from_ref(pg) {
            Some(a) => StaticOrPooled::Pooled(a),
            None => kpanic(format_args!("pgrp_ref: not a pooled group")),
        }
    }
}

/// Declares the list adapters of the glue `$glue` (`hal::pin_adapter!` and
/// `hal::owned_adapter!` of every list of [`ProcGlue`]), in the module that implements it:
/// `kernel::proc_glue_links!(BsdProcGlue);` then `kernel::proc_glue_link_types!();` inside
/// the `impl ProcGlue`.
#[macro_export]
macro_rules! proc_glue_links {
    ($glue:ty) => {
        hal::pin_adapter!(
            /// `ps_list`.
            pub ProcessListLink: $crate::proc::Process<$glue>, ps_list
        );
        hal::pin_adapter!(
            /// `ps_hash`.
            pub ProcessHashLink: $crate::proc::Process<$glue>, ps_hash
        );
        hal::pin_adapter!(
            /// `ps_pglist`.
            pub ProcessPglistLink: $crate::proc::Process<$glue>, ps_pglist
        );
        hal::pin_adapter!(
            /// `ps_sibling`.
            pub ProcessSiblingLink: $crate::proc::Process<$glue>, ps_sibling
        );
        hal::pin_adapter!(
            /// `ps_orphan`.
            pub ProcessOrphanLink: $crate::proc::Process<$glue>, ps_orphan
        );
        hal::pin_adapter!(
            /// `p_list`.
            pub ThreadListLink: $crate::proc::Thread<$glue>, p_list
        );
        hal::pin_adapter!(
            /// `p_hash`.
            pub ThreadHashLink: $crate::proc::Thread<$glue>, p_hash
        );
        hal::pin_adapter!(
            /// `p_thr_link`.
            pub ThreadThrLink: $crate::proc::Thread<$glue>, p_thr_link
        );
        hal::pin_adapter!(
            /// `p_runq`: the scheduler's queues.
            pub ThreadRunqLink: $crate::proc::Thread<$glue>, p_runq
        );
        hal::owned_adapter!(
            /// A process's thread boxes, by the order they joined.
            pub ThreadOwnedLink: $crate::proc::Thread<$glue>,
            p_owned,
            |a, b| a.p_seq.get().cmp(&b.p_seq.get())
        );
        hal::pin_adapter!(
            /// `pg_hash`.
            pub PgrpHashLink: $crate::proc::Pgrp<$glue>, pg_hash
        );
        hal::owned_adapter!(
            /// `kn_link`: a kqueue's chains own their knotes (a list: the order is unused).
            pub KnLinkLink: $crate::event::Knote<$glue>,
            link,
            |_a, _b| ::core::cmp::Ordering::Equal
        );
        hal::pin_adapter!(
            /// `kn_tqe`: a kqueue's pending queue.
            pub KnTqeLink: $crate::event::Knote<$glue>, tqe
        );
        hal::pin_adapter!(
            /// `kn_selnext`: an object's klist.
            pub KnSelnextLink: $crate::event::Knote<$glue>, selnext
        );
        hal::pin_adapter!(
            /// `kq_next`: a table's `fd_kqlist`.
            pub KqNextLink: $crate::event::Kqueue<$glue>, next
        );
    };
}

/// The adapter types [`proc_glue_links!`] declared, inside `impl ProcGlue for ..`.
#[macro_export]
macro_rules! proc_glue_link_types {
    () => {
        type ProcessList = ProcessListLink;
        type ProcessHash = ProcessHashLink;
        type ProcessPglist = ProcessPglistLink;
        type ProcessSibling = ProcessSiblingLink;
        type ProcessOrphan = ProcessOrphanLink;
        type ThreadList = ThreadListLink;
        type ThreadHash = ThreadHashLink;
        type ThreadLink = ThreadThrLink;
        type ThreadRunq = ThreadRunqLink;
        type ThreadOwned = ThreadOwnedLink;
        type PgrpHash = PgrpHashLink;
        type KnLink = KnLinkLink;
        type KnTqe = KnTqeLink;
        type KnSelnext = KnSelnextLink;
        type KqNext = KqNextLink;
    };
}

/// The process lists, hash tables and pools of one glue's processes (the C's globals of
/// `kern_proc.c`): `[K]` lists, kernel-lock witnesses.
pub struct ProcTables<G: ProcGlue> {
    /// `allprocess`: list of all processes.
    pub allprocess: PinList<G::ProcessList>,
    /// `zombprocess`: list of zombie processes.
    pub zombprocess: PinList<G::ProcessList>,
    /// `allproc`: list of all threads.
    pub allproc: PinList<G::ThreadList>,
    /// `pidhashtbl`, made by [`procinit`].
    pidhash: OnceStatic<MArray<PinList<G::ProcessHash>>>,
    /// `tidhashtbl`.
    tidhash: OnceStatic<MArray<PinList<G::ThreadHash>>>,
    /// `pgrphashtbl`.
    pgrphash: OnceStatic<MArray<PinList<G::PgrpHash>>>,
    /// `uihashtbl` (`[U]`): the users' process and lock counts.
    uihash: OnceStatic<MArray<PinList<UidinfoHash>>>,
    /// `proc_pool`, `procpl`.
    pub procpl: BoxPool<Thread<G>>,
    /// `process_pool`, `processpl` (recorded: a lent `&Process` can be counted again).
    pub processpl: TypedPool<Process<G>>,
    /// `pgrp_pool`, `pgrppl` (recorded).
    pub pgrppl: TypedPool<Pgrp<G>>,
    /// `session_pool`, `sessionpl` (recorded).
    pub sessionpl: TypedPool<Session<G>>,
    /// `kqueue_pool`, `knote_pool` and the timers' pool (`kernel::event::kqueue_init`).
    pub kq: KqPools<G>,
    /// `initprocess`: init(8)'s process, set once by `main` when it forked it.
    pub initprocess: OnceStatic<ProcessRef<G>>,
    /// `process0`, the swapper: the glue's `static`, recorded by `main`
    /// ([`ProcTables::set_process0`]); the orphans' parent until init exists.
    process0: OnceStatic<&'static Process<G>>,
    /// `proc0`: process 0's thread, which `kthread_create` forks from.
    proc0: OnceStatic<Pin<&'static Thread<G>>>,
}

impl<G: ProcGlue> ProcTables<G> {
    /// Empty lists, no hash table and no pool yet (`procinit` makes them).
    pub const fn new() -> Self {
        Self {
            allprocess: PinList::new(),
            zombprocess: PinList::new(),
            allproc: PinList::new(),
            pidhash: OnceStatic::new(),
            tidhash: OnceStatic::new(),
            pgrphash: OnceStatic::new(),
            uihash: OnceStatic::new(),
            procpl: BoxPool::new(),
            processpl: TypedPool::new(),
            pgrppl: TypedPool::new(),
            sessionpl: TypedPool::new(),
            kq: KqPools::new(),
            initprocess: OnceStatic::new(),
            process0: OnceStatic::new(),
            proc0: OnceStatic::new(),
        }
    }

    /// Records process 0 and its thread (`main`'s `process0`, `proc0`), once (`panic(9)` on
    /// a second).
    pub fn set_process0(&self, pr: &'static Process<G>, p: Pin<&'static Thread<G>>) {
        if self.process0.set(pr).is_err() || self.proc0.set(p).is_err() {
            kpanic(format_args!("process0 recorded twice"));
        }
    }

    /// `proc0` (`panic(9)` before `main` recorded it).
    pub fn proc0(&self) -> Pin<&'static Thread<G>> {
        match self.proc0.get() {
            Some(p) => *p,
            None => kpanic(format_args!("proc0 not recorded")),
        }
    }

    /// `process0` (`panic(9)` before `main` recorded it).
    pub fn process0(&self) -> ProcessRef<G> {
        match self.process0.get() {
            Some(pr) => StaticOrPooled::Static(pr),
            None => kpanic(format_args!("process0 not recorded")),
        }
    }

    /// `initprocess`, or process 0 while init does not exist yet (the orphans' parent in
    /// `exit1` and `process_untrace` before `main` forked init; the C's is never NULL there).
    pub fn initprocess_or_process0(&self) -> ProcessRef<G> {
        match self.initprocess.get() {
            Some(pr) => pr.clone(),
            None => self.process0(),
        }
    }

    /// The address of `initprocess`, 0 before it exists: compared, never followed
    /// (`setsigvec`'s `SAS_NOCLDWAIT`).
    pub fn initprocess_addr(&self) -> usize {
        self.initprocess.get().map_or(0, StaticOrPooled::addr)
    }

    /// `PIDHASH(pid)`.
    pub fn pidhash(&self, pid: Pid) -> &PinList<G::ProcessHash> {
        chain(&self.pidhash, pid)
    }

    /// `TIDHASH(tid)`.
    pub fn tidhash(&self, tid: Pid) -> &PinList<G::ThreadHash> {
        chain(&self.tidhash, tid)
    }

    /// `PGRPHASH(pgid)`.
    pub fn pgrphash(&self, pgid: Pid) -> &PinList<G::PgrpHash> {
        chain(&self.pgrphash, pgid)
    }
}

impl<G: ProcGlue> Default for ProcTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The chain for `key` of a table `procinit` made (`&tbl[key & mask]`).
fn chain<A: PinAdapter>(tbl: &OnceStatic<MArray<PinList<A>>>, key: i32) -> &PinList<A> {
    let Some(t) = tbl.get() else {
        kpanic(format_args!("procinit: hash table used before procinit"));
    };
    &t[(key as usize) & (t.len() - 1)]
}

/// `hashinit(elements, M_PROC, M_NOWAIT, &mask)`: a power of two of empty chains, at least
/// `elements`.
fn hashinit<A: PinAdapter>(elements: i32) -> MArray<PinList<A>> {
    if elements <= 0 {
        kpanic(format_args!("hashinit: bad cnt"));
    }
    let n = (elements as usize).next_power_of_two();
    match MArray::new(M_PROC, M_NOWAIT, n, |_| PinList::new()) {
        Some(t) => t,
        None => kpanic(format_args!("procinit: malloc")),
    }
}

/// `rusage_pool`, `zombiepl`: an exited process's usage (`ps_ru`).
static RUSAGE_POOL: BoxPool<Rusage> = BoxPool::new();

/// `uidinfolk`.
static UIDINFOLK: RawRwlock = RawRwlock::new("uidinfo");

/// `procinit()`: the hash tables (`maxthread / 4` tid chains, `maxprocess / 4` pid and pgrp
/// chains, `maxprocess / 16` uid chains) and the pools `procpl`, `processpl`, `zombiepl`,
/// `pgrppl`, `sessionpl` (`ucredpl` is `kernel::cred`'s, made at its first use). Once per
/// glue at boot (one glue per kernel; a host test thread's fresh tables, each).
pub fn procinit<G: ProcGlue>(maxthread: i32, maxprocess: i32) {
    let t = G::tables();
    let made = t.tidhash.set(hashinit(maxthread / 4)).is_ok()
        && t.pidhash.set(hashinit(maxprocess / 4)).is_ok()
        && t.pgrphash.set(hashinit(maxprocess / 4)).is_ok()
        && t.uihash.set(hashinit(maxprocess / 16)).is_ok();
    if !made {
        kpanic(format_args!("procinit: called twice"));
    }
    let pools = t
        .procpl
        .init_checked(
            IPL_NONE,
            PR_WAITOK,
            "procpl",
            crate::sched::thread_releasable::<G>,
        )
        .is_ok()
        && t.processpl
            .init_recorded(IPL_NONE, PR_WAITOK, "processpl")
            .is_ok()
        && t.pgrppl
            .init_recorded(IPL_NONE, PR_WAITOK, "pgrppl")
            .is_ok()
        && t.sessionpl
            .init_recorded(IPL_NONE, PR_WAITOK, "sessionpl")
            .is_ok();
    if !pools {
        kpanic(format_args!("procinit: pool_init"));
    }
    // One per kernel (a second glue's procinit, the host tests', finds it made).
    let _ = RUSAGE_POOL.init(IPL_NONE, PR_WAITOK, "zombiepl");
}

/// `pool_get(&rusage_pool, PR_WAITOK)` for `exit1`: a zeroed usage box.
pub fn rusage_alloc() -> PoolBox<Rusage> {
    match RUSAGE_POOL.get(PR_WAITOK, Rusage::new) {
        Some(b) => b,
        None => kpanic(format_args!("exit1: zombiepl is empty")),
    }
}

/// `zombiepl`'s item size (`pr_size`, what `vmstat -m` shows); `None` before [`procinit`].
pub fn rusage_item_size() -> Option<usize> {
    RUSAGE_POOL.item_size()
}

/// `struct uidinfo`: a user's process and lock counts.
pub struct Uidinfo {
    /// \[U\] `ui_hash`.
    ui_hash: PinEntry<Uidinfo>,
    /// \[I\] `ui_uid`.
    pub ui_uid: Uid,
    /// \[U\] `ui_proccnt`: proc structs.
    pub ui_proccnt: I64Word,
    /// \[U\] `ui_lockcnt`: lockf structs.
    pub ui_lockcnt: I64Word,
}

hal::pin_adapter!(
    /// `LIST_ENTRY(uidinfo) ui_hash`.
    UidinfoHash: Uidinfo, ui_hash
);

impl Uidinfo {
    /// The entry of `uid`, no counts.
    pub const fn new(uid: Uid) -> Self {
        Self {
            ui_hash: PinEntry::new(),
            ui_uid: uid,
            ui_proccnt: I64Word::new(0),
            ui_lockcnt: I64Word::new(0),
        }
    }
}

/// `struct session`: one per session.
pub struct Session<G: ProcGlue> {
    home: PoolHome<Session<G>>,
    /// `s_leader`: the session leader's address (compared with a process's, never followed),
    /// 0 when it has none.
    pub s_leader: UsizeWord,
    /// `s_leader->ps_pid`, kept with the address ([`Session::set_leader`]): what `getsid(2)`
    /// answers, read without following the address.
    s_leaderpid: I32Word,
    /// \[K\] `s_login`: setlogin() name.
    pub s_login: Guarded<[u8; LOGIN_NAME_MAX]>,
    /// `s_verauthppid`.
    pub s_verauthppid: I32Word,
    /// `s_verauthuid`.
    pub s_verauthuid: U32Word,
    /// The members of `bsd`'s subsystems.
    pub g: G::SessionG,
}

hal::pool_home!(impl[G: ProcGlue] Session<G>, home);

impl<G: ProcGlue> Session<G> {
    /// A session with no leader and no login name (`session0`, or a new one).
    pub const fn new(g: G::SessionG) -> Self {
        Self {
            home: PoolHome::new(),
            s_leader: UsizeWord::new(0),
            s_leaderpid: I32Word::new(0),
            s_login: Guarded::new_kernel([0; LOGIN_NAME_MAX]),
            s_verauthppid: I32Word::new(0),
            s_verauthuid: U32Word::new(0),
            g,
        }
    }

    /// Whether `pr` leads the session (`SESS_LEADER`).
    pub fn led_by(&self, pr: &Process<G>) -> bool {
        self.s_leader.get() == ptr::from_ref(pr).addr()
    }

    /// `sess->s_leader = pr` (`None`: `NULL`, its leader gone).
    pub fn set_leader(&self, pr: Option<&Process<G>>) {
        self.s_leaderpid.set(pr.map_or(0, |p| p.ps_pid));
        self.s_leader.set(pr.map_or(0, |p| ptr::from_ref(p).addr()));
    }

    /// `sess->s_leader->ps_pid`, `None` once the leader is gone (`getsid`'s `ESRCH`).
    pub fn leader_pid(&self) -> Option<Pid> {
        (self.s_leader.get() != 0).then(|| self.s_leaderpid.get())
    }
}

impl<G: ProcGlue> Deref for Session<G> {
    type Target = G::SessionG;

    fn deref(&self) -> &G::SessionG {
        &self.g
    }
}

impl<G: ProcGlue> Drop for Session<G> {
    /// `SESSRELE`'s last reference: `timeout_del(&s_verauthto)`, then the item goes back.
    fn drop(&mut self) {
        G::session_destroy(self);
    }
}

/// `pool_get(&session_pool, PR_WAITOK)` of a new session, placed and announced to the glue
/// (`timeout_set` of its `s_verauthto`); filled by [`enternewpgrp`].
pub fn session_alloc<G: ProcGlue>(g: G::SessionG) -> SessionRef<G> {
    let Some(s) = G::tables()
        .sessionpl
        .get_homed(PR_WAITOK, || Session::new(g))
    else {
        kpanic(format_args!("setsid: session_pool is empty"));
    };
    G::session_placed(&s);
    StaticOrPooled::Pooled(s)
}

/// `struct pgrp`: one per process group.
pub struct Pgrp<G: ProcGlue> {
    /// \[K\] `pg_hash`.
    pub pg_hash: PinEntry<Pgrp<G>>,
    /// \[K\] `pg_members`.
    pub pg_members: PinList<G::ProcessPglist>,
    home: PoolHome<Pgrp<G>>,
    /// \[K\] `pg_session`, set by [`enternewpgrp`].
    pub pg_session: Guarded<Option<SessionRef<G>>>,
    /// `pg_id`: pgrp id, set by [`enternewpgrp`].
    pub pg_id: I32Word,
    /// `pg_jobc`: # procs qualifying pgrp for job control.
    pub pg_jobc: I32Word,
    /// The members of `bsd`'s subsystems (none since N4.5b).
    pub g: G::PgrpG,
}

hal::pool_home!(impl[G: ProcGlue] Pgrp<G>, home);

impl<G: ProcGlue> Pgrp<G> {
    /// An empty group (`pgrp0`, or a new one).
    pub const fn new(g: G::PgrpG) -> Self {
        Self {
            pg_hash: PinEntry::new(),
            pg_members: PinList::new(),
            home: PoolHome::new(),
            pg_session: Guarded::new_kernel(None),
            pg_id: I32Word::new(0),
            pg_jobc: I32Word::new(0),
            g,
        }
    }

    /// `pg_session`, for a holder of the kernel lock.
    pub fn session<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a Session<G>> {
        self.pg_session.get(w).as_deref()
    }

    /// The session's address (0 for none), compared by `fixjobc`.
    fn session_addr<W: Witness + ?Sized>(&self, w: &W) -> usize {
        self.pg_session
            .get(w)
            .as_ref()
            .map_or(0, StaticOrPooled::addr)
    }
}

impl<G: ProcGlue> Deref for Pgrp<G> {
    type Target = G::PgrpG;

    fn deref(&self) -> &G::PgrpG {
        &self.g
    }
}

/// `pool_get(&pgrp_pool, PR_WAITOK)` of a new group, filled by [`enternewpgrp`].
pub fn pgrp_alloc<G: ProcGlue>(g: G::PgrpG) -> PgrpRef<G> {
    match G::tables().pgrppl.get_homed(PR_WAITOK, || Pgrp::new(g)) {
        Some(p) => StaticOrPooled::Pooled(p),
        None => kpanic(format_args!("setpgid: pgrp_pool is empty")),
    }
}

/// The members of `struct process` under its `ps_mtx` (`[m]`) that are counted handles or
/// owners: the credentials, the address space, the limits, the threads' boxes.
pub struct PsMtx<G: ProcGlue> {
    /// `ps_ucred`: the process's credentials (set by `fork1`, the set*id calls and exec).
    pub cred: Option<CredRef>,
    /// `ps_vmspace`: the address space (set by `fork1`, exec and `main`, taken by `uvm_exit`).
    pub vmspace: Option<VmspaceRef>,
    /// `ps_limit` (`[m,R]`): the limits, written under `rlimit_lock` too.
    pub limit: Option<PlimitRef>,
    /// `ps_pptr` (`[K|m]`) for a holder of `ps_mtx` alone: the parent the kernel lock's cell
    /// holds, set with it ([`Process::set_pptr_locked`]).
    pptr: Option<ProcessRef<G>>,
    /// `ps_pgrp` (`[K|m]`) for a holder of `ps_mtx` alone, set with the kernel lock's cell.
    pgrp: Option<PgrpRef<G>>,
    /// The boxes of the process's threads (not process 0's `static` one), by joining order.
    threads: OwnedTree<G::ThreadOwned>,
    /// `ps_mainproc`: the main thread, owned (see [`MainThread`]).
    main: MainThread<G>,
    /// \[K|m\] `ps_opptr`: the old parent while `ptrace(2)` attached (none: ptrace is not
    /// ported, so nothing sets it).
    opptr: Option<ProcessRef<G>>,
    /// The order number the next thread to join takes.
    next_seq: u64,
}

impl<G: ProcGlue> PsMtx<G> {
    /// Nothing held.
    pub const fn new() -> Self {
        Self {
            cred: None,
            vmspace: None,
            limit: None,
            pptr: None,
            pgrp: None,
            threads: OwnedTree::new(),
            main: MainThread::None,
            opptr: None,
            next_seq: 0,
        }
    }

    /// `pr->ps_ucred` read inside a `ps_mtx` section (`panic(9)` before the process has
    /// any): what a holder of the mutex reads, since [`Process::ucred`] takes it again.
    pub fn ucred(&self) -> &CredRef {
        match &self.cred {
            Some(c) => c,
            None => kpanic(format_args!("ps_ucred: no credentials")),
        }
    }
}

impl<G: ProcGlue> Default for PsMtx<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The `[K]` links of a process that are counted handles.
pub struct Tree<G: ProcGlue> {
    /// `ps_pptr`: the parent (none for process 0).
    pub pptr: Option<ProcessRef<G>>,
    /// `ps_pgrp`: the process group.
    pub pgrp: Option<PgrpRef<G>>,
}

/// `struct process`: what the threads of a process share.
pub struct Process<G: ProcGlue> {
    // The links first: dropped first, a linked one stops the kernel before anything else goes.
    /// \[K\] `ps_list`: `allprocess` or `zombprocess`.
    pub ps_list: PinEntry<Process<G>>,
    /// \[K\] `ps_hash`.
    pub ps_hash: PinEntry<Process<G>>,
    /// \[K\] `ps_pglist`.
    pub ps_pglist: PinEntry<Process<G>>,
    /// \[K\] `ps_sibling`.
    pub ps_sibling: PinEntry<Process<G>>,
    /// \[K\] `ps_orphan`.
    pub ps_orphan: PinEntry<Process<G>>,
    /// \[K\] `ps_children`.
    pub ps_children: PinList<G::ProcessSibling>,
    /// \[K\] `ps_orphans`.
    pub ps_orphans: PinList<G::ProcessOrphan>,
    /// \[m\] `ps_threads`: the threads, in the order they joined (bound to `ps_mtx`).
    pub ps_threads: PinList<G::ThreadLink>,
    home: PoolHome<Process<G>>,
    /// `ps_refcnt`: `kern.file`'s readers, waited for by `exit1` ("psdtor").
    pub ps_refcnt: Refcnt,
    /// \[I\] `ps_pid`.
    pub ps_pid: Pid,
    /// `ps_mtx`.
    pub ps_mtx: Mutexed<PsMtx<G>>,
    /// The address of the credentials `ps_mtx` holds (`refreshcreds`' unlocked compare).
    cred_word: AtomicUsize,
    /// The address of the address space `ps_mtx` holds (`kinfo_proc`'s `p_vmspace`).
    vmspace_word: AtomicUsize,
    /// The address of the limits `ps_mtx` holds (`lim_read_enter`'s unlocked compare).
    limit_word: AtomicUsize,
    /// `ps_lock`: per-process rwlock (initialised only, as the C's).
    pub ps_lock: RawRwlock,
    /// \[a\] `ps_flags`.
    pub ps_flags: AtomicU32,
    /// \[a\] `ps_siglist`.
    pub ps_siglist: AtomicU32,
    /// \[m\] `ps_single`: thread for single-threading.
    pub ps_single: hal::link::PtrWord<Thread<G>>,
    /// \[m\] `ps_trapped`: thread trapped for ptrace.
    pub ps_trapped: hal::link::PtrWord<Thread<G>>,
    /// \[m\] `ps_suspendcnt`.
    pub ps_suspendcnt: U32Word,
    /// \[m\] `ps_exitcnt`.
    pub ps_exitcnt: U32Word,
    /// \[m\] `ps_threadcnt`.
    pub ps_threadcnt: U32Word,
    /// `ps_xexit`.
    pub ps_xexit: U32Word,
    /// `ps_xsig`.
    pub ps_xsig: I32Word,
    /// \[K|m\] `ps_ppid`: the parent's pid (`getppid` reads it without a lock).
    pub ps_ppid: I32Word,
    /// \[K\] `ps_pptr`, `ps_pgrp`.
    tree: Guarded<Tree<G>>,
    /// \[K\] `ps_ru`: sum of stats for dead threads.
    pub ps_ru: Guarded<Option<PoolBox<Rusage>>>,
    /// \[m\] `ps_tu`.
    pub ps_tu: Tusage,
    /// `ps_cru`.
    pub ps_cru: Rusage,
    /// `ps_nextxcpu`.
    pub ps_nextxcpu: I64Word,
    /// `ps_wxcounter`.
    pub ps_wxcounter: U64Word,
    /// `ps_comm`.
    ps_comm: NameWord,
    /// `ps_strings`.
    pub ps_strings: UsizeWord,
    /// `ps_auxinfo`.
    pub ps_auxinfo: UsizeWord,
    /// `ps_timekeep`.
    pub ps_timekeep: UsizeWord,
    /// \[I\] `ps_sigcode`.
    pub ps_sigcode: UsizeWord,
    /// \[I\] `ps_sigcoderet`.
    pub ps_sigcoderet: UsizeWord,
    /// \[I\] `ps_sigcookie`.
    pub ps_sigcookie: U64Word,
    /// \[a\] `ps_rtableid`.
    pub ps_rtableid: AtomicU32,
    /// \[I\] `ps_iflags`.
    pub ps_iflags: U16Word,
    /// `ps_nice`.
    pub ps_nice: U8Word,
    /// `ps_acflag`.
    pub ps_acflag: U32Word,
    /// \[m\] `ps_pledge` (read without the lock by `pledge_syscall`).
    pub ps_pledge: AtomicU64,
    /// \[m\] `ps_execpledge`.
    pub ps_execpledge: U64Word,
    /// \[m\] `ps_kbind_cookie`.
    pub ps_kbind_cookie: I64Word,
    /// \[m\] `ps_kbind_addr`.
    pub ps_kbind_addr: UsizeWord,
    /// `ps_start`: starting uptime.
    pub ps_start: TimespecWord,
    /// `ps_fd`: the descriptor table, set once before the process's first thread runs
    /// (`fork1`, `main` for process 0) and never taken back (`fdfree` empties it).
    fd: OnceStatic<FdRef<G::FileGlue>>,
    /// \[Q,m\] `ps_klist`: knotes attached to this process, locked by `ps_mtx` (with
    /// `kqueue_ps_list_lock`, `kernel::event`).
    pub ps_klist: Klist<G>,
    /// \[I\] `ps_sigacts`: signal actions, state (`sigactsinit`'s, or `sigacts0`).
    ps_sigacts: OnceStatic<SigactsRef>,
    /// `ps_pin`: the system call pins of the program or ld.so (`exec`), read at every system
    /// call (`pin_check`), replaced by `exec` single-threaded and freed by the last `exit1`.
    pub ps_pin: ReadMostly<PinTable>,
    /// `ps_libcpin`: libc.so's pins, from `pinsyscalls(2)`; as `ps_pin`.
    pub ps_libcpin: ReadMostly<PinTable>,
    /// `ps_rucheck_to`: the resource limit check timer (`kernel::resource::rucheck`, named by
    /// the process's pid).
    pub ps_rucheck_to: crate::timeout::Timeout,
    /// The members of `bsd`'s subsystems.
    pub g: G::ProcessG,
}

hal::pool_home!(impl[G: ProcGlue] Process<G>, home);

impl<G: ProcGlue> Process<G> {
    /// A process with pid `pid`, holding nothing yet (`process0`, or `process_new`'s before
    /// `fork1` fills it).
    pub const fn new(pid: Pid, g: G::ProcessG) -> Self {
        Self {
            ps_list: PinEntry::new(),
            ps_hash: PinEntry::new(),
            ps_pglist: PinEntry::new(),
            ps_sibling: PinEntry::new(),
            ps_orphan: PinEntry::new(),
            ps_children: PinList::new(),
            ps_orphans: PinList::new(),
            ps_threads: PinList::new(),
            home: PoolHome::new(),
            ps_refcnt: Refcnt::new(),
            ps_pid: pid,
            ps_mtx: Mutexed::new(IPL_HIGH, PsMtx::new()),
            cred_word: AtomicUsize::new(0),
            vmspace_word: AtomicUsize::new(0),
            limit_word: AtomicUsize::new(0),
            ps_lock: RawRwlock::new("pslock"),
            ps_flags: AtomicU32::new(0),
            ps_siglist: AtomicU32::new(0),
            ps_single: hal::link::PtrWord::new(),
            ps_trapped: hal::link::PtrWord::new(),
            ps_suspendcnt: U32Word::new(0),
            ps_exitcnt: U32Word::new(0),
            ps_threadcnt: U32Word::new(0),
            ps_xexit: U32Word::new(0),
            ps_xsig: I32Word::new(0),
            ps_ppid: I32Word::new(0),
            tree: Guarded::new_kernel(Tree {
                pptr: None,
                pgrp: None,
            }),
            ps_ru: Guarded::new_kernel(None),
            ps_tu: Tusage::new(),
            ps_cru: Rusage::new(),
            ps_nextxcpu: I64Word::new(0),
            ps_wxcounter: U64Word::new(0),
            ps_comm: NameWord::new(),
            ps_strings: UsizeWord::new(0),
            ps_auxinfo: UsizeWord::new(0),
            ps_timekeep: UsizeWord::new(0),
            ps_sigcode: UsizeWord::new(0),
            ps_sigcoderet: UsizeWord::new(0),
            ps_sigcookie: U64Word::new(0),
            ps_rtableid: AtomicU32::new(0),
            ps_iflags: U16Word::new(0),
            ps_nice: U8Word::new(0),
            ps_acflag: U32Word::new(0),
            ps_pledge: AtomicU64::new(0),
            ps_execpledge: U64Word::new(0),
            ps_kbind_cookie: I64Word::new(0),
            ps_kbind_addr: UsizeWord::new(0),
            ps_start: TimespecWord::new(),
            fd: OnceStatic::new(),
            ps_klist: Klist::owner(),
            ps_sigacts: OnceStatic::new(),
            ps_pin: ReadMostly::new(PinTable::NONE),
            ps_libcpin: ReadMostly::new(PinTable::NONE),
            ps_rucheck_to: crate::timeout::Timeout::unset(),
            g,
        }
    }

    /// `pr->ps_sigacts`: the process's signal actions, which it holds from `process_new`
    /// (`sigacts0` for process 0) until it goes (`panic(9)` before it has them).
    pub fn sigacts(&self) -> &Sigacts {
        match self.ps_sigacts.get() {
            Some(ps) => ps,
            None => kpanic(format_args!("ps_sigacts: none")),
        }
    }

    /// `pr->ps_sigacts`, `None` before `process_new` gave it (`fill_kproc` of an embryo).
    pub fn sigacts_opt(&self) -> Option<&Sigacts> {
        self.ps_sigacts.get().map(|ps| &**ps)
    }

    /// `pr->ps_sigacts = ps`, once, before the process is visible (`panic(9)` on a second).
    pub fn set_sigacts(&self, ps: SigactsRef) {
        if self.ps_sigacts.set(ps).is_err() {
            kpanic(format_args!("ps_sigacts: set twice"));
        }
    }

    /// `pr->ps_fd`: the process's descriptor table (`panic(9)` before it has one).
    pub fn fd(&self) -> &Filedesc<G::FileGlue> {
        self.fd_ref()
    }

    /// `pr->ps_fd`, `None` before the process has one.
    pub fn fd_opt(&self) -> Option<&Filedesc<G::FileGlue>> {
        self.fd.get().map(|f| &**f)
    }

    /// `pr->ps_fd`'s counted handle (`fdshare`'s clone, `kinfo_proc.p_fd`'s address).
    pub fn fd_ref(&self) -> &FdRef<G::FileGlue> {
        match self.fd.get() {
            Some(f) => f,
            None => kpanic(format_args!("process {}: no ps_fd", self.ps_pid)),
        }
    }

    /// `pr->ps_fd = fdp`: once, before a thread of the process runs (`panic(9)` twice).
    pub fn set_fd(&self, fdp: FdRef<G::FileGlue>) {
        if self.fd.set(fdp).is_err() {
            kpanic(format_args!("process {}: ps_fd set twice", self.ps_pid));
        }
    }

    /// `ps_comm`.
    pub fn comm(&self) -> Name {
        self.ps_comm.get()
    }

    /// `strlcpy(pr->ps_comm, name, sizeof pr->ps_comm)`.
    pub fn set_comm(&self, name: &[u8]) {
        self.ps_comm.set(name);
    }

    /// `crhold(pr->ps_ucred)`: the process's credentials (`panic(9)` before it has any).
    pub fn ucred(&self) -> CredRef {
        self.ps_mtx.lock().ucred().clone()
    }

    /// `pr->ps_ucred = new` (the set*id calls, exec, `process_initialize`): the old
    /// reference, which the caller drops outside the lock (`crfree`).
    pub fn set_ucred(&self, new: CredRef) -> Option<CredRef> {
        let addr = CredRef::as_ptr(&new).addr();
        let old = self.ps_mtx.lock().cred.replace(new);
        self.cred_word.store(addr, Ordering::Release);
        old
    }

    /// The address of the process's credentials, read without the lock (`refreshcreds`).
    pub fn ucred_word(&self) -> usize {
        self.cred_word.load(Ordering::Acquire)
    }

    /// `vm = pr->ps_vmspace; uvmspace_addref(vm)`: a handle to the address space, `None` once
    /// `uvm_exit` took it (LZ's `vmspace_share`).
    pub fn vmspace_share(&self) -> Option<VmspaceRef> {
        self.ps_mtx.lock().vmspace.clone()
    }

    /// `pr->ps_vmspace = vm`: the previous handle. The word keeps the address exposed
    /// (`bsd`'s readers of `p_vmspace` until N4.5).
    pub fn set_vmspace(&self, vm: Option<VmspaceRef>) -> Option<VmspaceRef> {
        let addr = vm
            .as_ref()
            .map_or(0, |v| PoolArc::as_ptr(v).expose_provenance());
        let old = core::mem::replace(&mut self.ps_mtx.lock().vmspace, vm);
        self.vmspace_word.store(addr, Ordering::Release);
        old
    }

    /// The address of the address space (`kinfo_proc`'s `p_vmspace`, a thread's copy), 0 for
    /// none.
    pub fn vmspace_word(&self) -> usize {
        self.vmspace_word.load(Ordering::Acquire)
    }

    /// `pr->ps_limit`, a reference (`lim_fork`'s `refcnt_take`).
    pub fn limit(&self) -> Option<PlimitRef> {
        self.ps_mtx.lock().limit.clone()
    }

    /// `pr->ps_limit = limit` under `ps_mtx` (`lim_write_commit`, `fork1`): the old one.
    pub fn set_limit(&self, limit: PlimitRef) -> Option<PlimitRef> {
        let addr = PlimitRef::addr(&limit);
        let old = self.ps_mtx.lock().limit.replace(limit);
        self.limit_word.store(addr, Ordering::Release);
        old
    }

    /// The address of the limits, read without the lock (`lim_read_enter`).
    pub fn limit_word(&self) -> usize {
        self.limit_word.load(Ordering::Acquire)
    }

    /// `ps_pptr`, for a holder of the kernel lock.
    pub fn pptr<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a ProcessRef<G>> {
        self.tree.get(w).pptr.as_ref()
    }

    /// `pr->ps_pptr = parent; pr->ps_ppid = parent->ps_pid` (`fork1`, `process_reparent`):
    /// the old parent's reference. Takes `ps_mtx` (the C's `[K|m]`).
    pub fn set_pptr(&self, w: &mut KernelWitness, parent: ProcessRef<G>) -> Option<ProcessRef<G>> {
        let mut g = self.ps_mtx.lock();
        self.set_pptr_locked(w, &mut g, parent)
    }

    /// [`Process::set_pptr`] under the caller's guard of this process's `ps_mtx`
    /// (`process_reparent`'s `MUTEX_ASSERT_LOCKED`); a guard of another mutex is refused
    /// (`panic(9)`) once the thread list is bound to its own (`process_alloc`).
    pub fn set_pptr_locked(
        &self,
        w: &mut KernelWitness,
        g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>,
        parent: ProcessRef<G>,
    ) -> Option<ProcessRef<G>> {
        // The thread list checks the guard is this process's.
        let _ = self.ps_threads.is_empty(g);
        self.ps_ppid.set(parent.ps_pid);
        g.pptr = Some(parent.clone());
        self.tree.get_mut(w).pptr.replace(parent)
    }

    /// `ps_pptr`, for a holder of this process's `ps_mtx` (`[K|m]`) who may not hold the
    /// kernel lock; a guard of another mutex is refused (as for `set_pptr_locked`).
    pub fn pptr_locked<'a>(
        &self,
        g: &'a hal::mutex::MutexedGuard<'_, PsMtx<G>>,
    ) -> Option<&'a ProcessRef<G>> {
        let _ = self.ps_threads.is_empty(g);
        g.pptr.as_ref()
    }

    /// `ps_pgrp`, for a holder of this process's `ps_mtx` (`[K|m]`) who may not hold the
    /// kernel lock; a guard of another mutex is refused (as for `set_pptr_locked`).
    pub fn pgrp_locked<'a>(
        &self,
        g: &'a hal::mutex::MutexedGuard<'_, PsMtx<G>>,
    ) -> Option<&'a PgrpRef<G>> {
        let _ = self.ps_threads.is_empty(g);
        g.pgrp.as_ref()
    }

    /// `ps_pgrp`, for a holder of the kernel lock.
    pub fn pgrp<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a PgrpRef<G>> {
        self.tree.get(w).pgrp.as_ref()
    }

    /// `pr->ps_pgrp = pgrp` (under `ps_mtx` too, as the C): the old group's reference.
    pub fn set_pgrp(&self, w: &mut KernelWitness, pgrp: Option<PgrpRef<G>>) -> Option<PgrpRef<G>> {
        let mut m = self.ps_mtx.lock();
        // The mirror's old reference goes after the lock (a group's last drop may sleep).
        let mirror = core::mem::replace(&mut m.pgrp, pgrp.clone());
        let old = core::mem::replace(&mut self.tree.get_mut(w).pgrp, pgrp);
        drop(m);
        drop(mirror);
        old
    }

    /// `pr->ps_pgrp->pg_id` (0 without a group).
    pub fn pgid<W: Witness + ?Sized>(&self, w: &W) -> Pid {
        self.pgrp(w).map_or(0, |pg| pg.pg_id.get())
    }

    /// `pr->ps_session` (`ps_pgrp->pg_session`).
    pub fn session<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a Session<G>> {
        self.pgrp(w)?.session(w)
    }

    /// `SESS_LEADER(pr)`.
    pub fn sess_leader<W: Witness + ?Sized>(&self, w: &W) -> bool {
        self.session(w).is_some_and(|s| s.led_by(self))
    }

    /// `ps_mainproc`, for a holder of this process's `ps_mtx` (a guard of another mutex is
    /// refused, as for `set_pptr_locked`): the main thread from `fork1` (or `main` for process
    /// 0) until `process_zap`.
    pub fn main_thread<'g>(
        &self,
        g: &'g hal::mutex::MutexedGuard<'_, PsMtx<G>>,
    ) -> Option<Pin<&'g Thread<G>>> {
        let _ = self.ps_threads.is_empty(g);
        g.main.get()
    }

    /// `pr->ps_mainproc = p` of `fork1` (`process_initialize`): the process takes its main
    /// thread's box (`panic(9)` for a second one).
    pub fn set_main_locked(
        &self,
        g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>,
        t: PoolBox<Thread<G>>,
    ) {
        let _ = self.ps_threads.is_empty(g);
        if !matches!(g.main, MainThread::None) {
            kpanic(format_args!(
                "process {}: a second main thread",
                self.ps_pid
            ));
        }
        g.main = MainThread::Owned(t);
    }

    /// Process 0's `static` main thread (`main`'s `process_initialize(&process0, &proc0)`).
    pub fn set_main_static(&self, t: Pin<&'static Thread<G>>) {
        let mut g = self.ps_mtx.lock();
        if !matches!(g.main, MainThread::None) {
            kpanic(format_args!(
                "process {}: a second main thread",
                self.ps_pid
            ));
        }
        g.main = MainThread::Static(t);
    }

    /// `process_zap`'s `proc_free(pr->ps_mainproc)`: the main thread's box taken back (`None`
    /// for process 0's `static` one, or once taken).
    pub fn take_main_locked(
        &self,
        g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>,
    ) -> Option<PoolBox<Thread<G>>> {
        let _ = self.ps_threads.is_empty(g);
        match core::mem::replace(&mut g.main, MainThread::None) {
            MainThread::Owned(b) => Some(b),
            s @ MainThread::Static(_) => {
                g.main = s;
                None
            }
            MainThread::None => None,
        }
    }

    /// The thread joins `ps_threads` (`TAILQ_INSERT_TAIL`), last, with its joining order;
    /// its box stays the caller's (the main thread's goes to [`Process::set_main_locked`], a
    /// secondary thread's to [`Process::insert_box_locked`]).
    pub fn join_locked(&self, g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>, t: Pin<&Thread<G>>) {
        // The list first: it checks the guard is this process's.
        self.ps_threads.insert_tail(g, t);
        let seq = g.next_seq;
        g.next_seq += 1;
        t.p_seq.set(seq);
    }

    /// A secondary thread's box (`P_THREAD`), kept by the process's tree until its `exit1`
    /// ([`Process::take_box_locked`]).
    pub fn insert_box_locked(
        &self,
        g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>,
        t: PoolBox<Thread<G>>,
    ) {
        // The tree is the guard's own data (no other lock's): the guard's identity is checked
        // by the `join_locked` this pairs with, in the same section (one check, decision 43).
        if g.threads.insert(t).is_some() {
            kpanic(format_args!("insert_box: a thread joined twice"));
        }
    }

    /// The box of the secondary thread whose joining order is `seq`, taken back (`exit1`);
    /// `None` when the tree has none (the main thread's is the slot's).
    pub fn take_box_locked(
        &self,
        g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>,
        seq: u64,
    ) -> Option<PoolBox<Thread<G>>> {
        // As `insert_box_locked`: the `leave_locked` this pairs with checks the guard.
        g.threads.take_by(|e| seq.cmp(&e.p_seq.get()))
    }

    /// `TAILQ_REMOVE(&pr->ps_threads, p, p_thr_link)`: the thread leaves the list; its box
    /// stays where it is (the slot or the tree), which the caller deals with.
    pub fn leave_locked(&self, g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>, t: &Thread<G>) {
        self.ps_threads.remove(g, t);
    }

    /// The thread of this process at address `addr` (`ps_single`, `ps_trapped`: the C's
    /// pointers, kept as words), found on `ps_threads` under the caller's guard; `None` when
    /// none is there (cold: the walk is the C's dereference made checkable).
    pub fn thread_by_addr<'a>(
        &'a self,
        g: &'a hal::mutex::MutexedGuard<'_, PsMtx<G>>,
        addr: usize,
    ) -> Option<Pin<&'a Thread<G>>> {
        if addr == 0 {
            return None;
        }
        self.ps_threads
            .iter_pinned(g)
            .find(|t| ptr::from_ref::<Thread<G>>(t).addr() == addr)
    }

    /// `ps_opptr`, for a holder of this process's `ps_mtx` (`[K|m]`).
    pub fn opptr_locked<'a>(
        &self,
        g: &'a hal::mutex::MutexedGuard<'_, PsMtx<G>>,
    ) -> Option<&'a ProcessRef<G>> {
        let _ = self.ps_threads.is_empty(g);
        g.opptr.as_ref()
    }

    /// `pr->ps_opptr = op` under this process's `ps_mtx`: the old one.
    pub fn set_opptr_locked(
        &self,
        g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>,
        op: Option<ProcessRef<G>>,
    ) -> Option<ProcessRef<G>> {
        let _ = self.ps_threads.is_empty(g);
        core::mem::replace(&mut g.opptr, op)
    }
}

impl<G: ProcGlue> Deref for Process<G> {
    type Target = G::ProcessG;

    fn deref(&self) -> &G::ProcessG {
        &self.g
    }
}

/// `process_new`'s `pool_get(&process_pool, PR_WAITOK)`: a process with pid `pid`, placed
/// in `processpl`, counted once.
pub fn process_alloc<G: ProcGlue>(pid: Pid, g: G::ProcessG) -> ProcessRef<G> {
    match G::tables()
        .processpl
        .get_homed(PR_WAITOK, || Process::new(pid, g))
    {
        Some(p) => {
            // The thread list bound to the process's own `ps_mtx` from the start, so a guard
            // of another mutex is refused by every `_locked` method.
            let _ = p.ps_threads.is_empty(&p.ps_mtx.lock());
            StaticOrPooled::Pooled(p)
        }
        None => kpanic(format_args!("process_new: process_pool is empty")),
    }
}

/// `struct proc`: a thread. `#[repr(C)]` with its [`MachineSlot`] first (decision P1).
#[repr(C)]
pub struct Thread<G: ProcGlue> {
    /// `p_stat`, `p_cpu`, `p_addr`, `p_md`: what the machine reaches by offset.
    pub machine: MachineSlot,
    /// \[S\] The number of the CPU `p_cpu` names (`kernel::sched`'s `set_thread_cpu`, which
    /// writes both), -1 before the thread first ran or was queued: `p_cpu`'s CPU in O(1), beside the slot
    /// (one cache line with `p_cpu`).
    pub p_cpuno: I32Word,
    /// \[K\] `p_list`: `allproc`.
    pub p_list: PinEntry<Thread<G>>,
    /// \[K\] `p_hash`.
    pub p_hash: PinEntry<Thread<G>>,
    /// \[m\] `p_thr_link`.
    pub p_thr_link: PinEntry<Thread<G>>,
    /// \[m\] The thread's place in its process's tree of boxes.
    pub p_owned: OwnedEntry<Thread<G>>,
    /// \[m\] The thread's joining order.
    pub p_seq: U64Word,
    /// \[S\] `p_runq`: the run, sleep and idle queues' link (`kernel::sched`), and the dead
    /// threads' (`deadproc_mutex`).
    pub p_runq: PinEntry<Thread<G>>,
    /// `p_sleep_to`: the timeout of a timed sleep (`endtsleep`, named by the thread's tid:
    /// `kernel::sched::sleep_timeout_init`).
    pub p_sleep_to: crate::timeout::Timeout,
    /// \[I\] `p_p`.
    p_p: ProcessRef<G>,
    /// `p_tid`.
    pub p_tid: I32Word,
    /// `p_flag`.
    pub p_flag: AtomicI32,
    /// \[S\] `p_runpri`.
    pub p_runpri: U8Word,
    /// `p_descfd`.
    pub p_descfd: U8Word,
    /// `p_dupfd`.
    pub p_dupfd: I32Word,
    /// \[o\] `p_cpticks`.
    pub p_cpticks: U32Word,
    /// \[K\] `p_cpticks2`.
    pub p_cpticks2: U32Word,
    /// \[S\] `p_wchan`: the sleep address, as a number.
    pub p_wchan: UsizeWord,
    /// \[S\] `p_wmesg`.
    pub p_wmesg: StrSlot,
    /// \[a\] `p_pctcpu`.
    pub p_pctcpu: AtomicU32,
    /// \[S\] `p_slptime`.
    pub p_slptime: U32Word,
    /// `p_ru`.
    pub p_ru: Rusage,
    /// \[o\] `p_tu`.
    pub p_tu: Tusage,
    /// \[l\] `p_limit`: the thread's copy of its process's limits.
    pub p_limit: LimitCache,
    /// \[o\] `p_kq_serial`.
    pub p_kq_serial: U64Word,
    /// \[o\] `p_kq`: for select/poll (`kernel::event::kqpoll_init`), set at the first call
    /// and kept until the thread's memory goes (`kqpoll_exit` empties it).
    pub p_kq: OnceStatic<KqRef<G>>,
    /// \[a\] `p_siglist`.
    pub p_siglist: AtomicU32,
    /// \[o\] `p_sigmask`.
    pub p_sigmask: U32Word,
    /// \[o\] `p_sigstk`: sp & on stack state variable.
    pub p_sigstk: SigaltstackWord,
    /// `p_sigval`: for core dump/debugger XXX.
    pub p_sigval: SigvalWord,
    /// `p_name`.
    p_name: NameWord,
    /// \[S\] `p_slppri`.
    pub p_slppri: U8Word,
    /// \[S\] `p_usrpri`.
    pub p_usrpri: U8Word,
    /// \[S\] `p_estcpu`.
    pub p_estcpu: U32Word,
    /// `p_pledge_syscall`.
    pub p_pledge_syscall: I32Word,
    /// \[o\] `p_pledge`.
    pub p_pledge: U64Word,
    /// `p_prof_addr`.
    pub p_prof_addr: U64Word,
    /// `p_prof_ticks`.
    pub p_prof_ticks: U64Word,
    /// \[o\] `p_oldmask`.
    pub p_oldmask: U32Word,
    /// `p_sisig`.
    pub p_sisig: I32Word,
    /// `p_sitrapno`.
    pub p_sitrapno: U64Word,
    /// `p_sicode`.
    pub p_sicode: I32Word,
    /// \[o\] `p_spinentry`.
    pub p_spinentry: InentryWord,
    /// \[o\] `p_ucred`'s ids: the thread's own copy of its process's credentials.
    cred: Ucred,
    /// The address of the credentials [`Thread::cred`] was copied from.
    cred_from: AtomicUsize,
    /// \[m\] The reference the copy came from (`crhold(p->p_ucred)` takes it).
    cred_ref: Guarded<Option<CredRef>>,
    /// \[m\] `p_vmspace`'s handle.
    vmspace: Guarded<Option<VmspaceRef>>,
    /// The address of `vmspace` (`p->p_vmspace`), read without the lock.
    vmspace_word: AtomicUsize,
    /// A secondary thread's own box, from its `exit1`, where its process gives it back,
    /// until the reaper takes it (`proc_free`): the C's pool item, alive until `pool_put`.
    /// Not the C's.
    pub exited: Mutexed<Option<PoolBox<Thread<G>>>>,
    /// The members of `bsd`'s subsystems.
    pub g: G::ThreadG,
}

impl<G: ProcGlue> Thread<G> {
    /// Process 0's `static` thread (`proc0`), of `pr` (`process0`).
    pub const fn new_static(pr: &'static Process<G>, g: G::ThreadG) -> Self {
        Self::make(
            StaticOrPooled::Static(pr),
            Guarded::new_static_mutexed(&pr.ps_mtx, None),
            Guarded::new_static_mutexed(&pr.ps_mtx, None),
            g,
        )
    }

    /// The members, keyed to `pr`'s `ps_mtx` by the caller.
    const fn make(
        p_p: ProcessRef<G>,
        cred_ref: Guarded<Option<CredRef>>,
        vmspace: Guarded<Option<VmspaceRef>>,
        g: G::ThreadG,
    ) -> Self {
        Self {
            machine: MachineSlot::new(),
            p_cpuno: I32Word::new(-1),
            p_list: PinEntry::new(),
            p_hash: PinEntry::new(),
            p_thr_link: PinEntry::new(),
            p_owned: OwnedEntry::new(),
            p_seq: U64Word::new(0),

            p_runq: PinEntry::new(),
            p_sleep_to: crate::timeout::Timeout::unset(),
            p_p,
            p_tid: I32Word::new(0),
            p_flag: AtomicI32::new(0),
            p_runpri: U8Word::new(0),
            p_descfd: U8Word::new(0),
            p_dupfd: I32Word::new(0),
            p_cpticks: U32Word::new(0),
            p_cpticks2: U32Word::new(0),
            p_wchan: UsizeWord::new(0),
            p_wmesg: StrSlot::new(),
            p_pctcpu: AtomicU32::new(0),
            p_slptime: U32Word::new(0),
            p_ru: Rusage::new(),
            p_tu: Tusage::new(),
            p_limit: LimitCache::new(),
            p_kq_serial: U64Word::new(0),
            p_kq: OnceStatic::new(),
            p_siglist: AtomicU32::new(0),
            p_sigmask: U32Word::new(0),
            p_sigstk: SigaltstackWord::new(),
            p_sigval: SigvalWord::new(),
            p_name: NameWord::new(),
            p_slppri: U8Word::new(0),
            p_usrpri: U8Word::new(0),
            p_estcpu: U32Word::new(0),
            p_pledge_syscall: I32Word::new(0),
            p_pledge: U64Word::new(0),
            p_prof_addr: U64Word::new(0),
            p_prof_ticks: U64Word::new(0),
            p_oldmask: U32Word::new(0),
            p_sisig: I32Word::new(0),
            p_sitrapno: U64Word::new(0),
            p_sicode: I32Word::new(0),
            p_spinentry: InentryWord::new(),
            cred: Ucred::new(),
            cred_from: AtomicUsize::new(0),
            cred_ref,
            vmspace,
            vmspace_word: AtomicUsize::new(0),
            exited: Mutexed::new(IPL_NONE, None),
            g,
        }
    }

    /// `p->p_p`.
    pub fn process(&self) -> &Process<G> {
        &self.p_p
    }

    /// `p->p_p`, a counted reference (what pins the process for its lists).
    pub fn process_ref(&self) -> &ProcessRef<G> {
        &self.p_p
    }

    /// `p->p_fd`: the descriptor table of the thread's process (the C's per-thread copy of
    /// `ps_fd`, which every thread of a process shares).
    pub fn fd(&self) -> &Filedesc<G::FileGlue> {
        self.p_p.fd()
    }

    /// `p_stat`.
    #[inline]
    pub fn stat(&self) -> u8 {
        self.machine.stat()
    }

    /// `p->p_stat = s`.
    #[inline]
    pub fn set_stat(&self, s: u8) {
        self.machine.set_stat(s);
    }

    /// `p_cpu`: the `cpu_info` word of the CPU the thread runs or last ran on.
    #[inline]
    pub fn cpu_word(&self) -> usize {
        self.machine.cpu_word()
    }

    /// `p->p_cpu = ci` (`fork1`, `sched_choosecpu`).
    #[inline]
    pub fn set_cpu_word(&self, ci: usize) {
        self.machine.set_cpu_word(ci);
    }

    /// `p_addr`: the u-area's address, 0 before `cpu_fork` (set and cleared through the
    /// slot alone: `hal::thread::MachineSlot::install_uarea`, `take_uarea`, E4).
    #[inline]
    pub fn addr_word(&self) -> usize {
        self.machine.addr_word()
    }

    /// `p->p_addr = uaddr` of `thread_new`: the thread takes the u-area `u` (`hal`'s E4).
    pub fn install_uarea(&self, u: hal::thread::UArea) {
        self.machine.install_uarea(u);
    }

    /// The reaper's `uvm_uarea_free(p)`: the u-area given back, for a thread no CPU runs.
    pub fn take_uarea(&self) -> Option<hal::thread::UArea> {
        self.machine.take_uarea()
    }

    /// `p_name` up to its NUL.
    pub fn name(&self) -> Name {
        self.p_name.get()
    }

    /// `strlcpy(p->p_name, name, sizeof p->p_name)`.
    pub fn set_name(&self, name: &[u8]) {
        self.p_name.set(name);
    }

    /// `p->p_ucred`: the thread's credentials, its own copy (no load of a pointer, no lock).
    #[inline]
    pub fn ucred(&self) -> &Ucred {
        &self.cred
    }

    /// The address of the thread's credentials, for `bsd`'s VFS, which passes `struct ucred *`
    /// (with its `NOCRED`/`FSCRED` sentinels) until N5: valid while the thread is, and changed
    /// only by the thread itself ([`Thread::refresh_cred`], [`Thread::set_cred`]).
    #[inline]
    pub fn ucred_ptr(&self) -> *const Ucred {
        &raw const self.cred
    }

    /// `crhold(p->p_ucred)`: the reference the thread's copy came from (`falloc`, sockets,
    /// the signal owner, `fork1`).
    pub fn cred_ref(&self) -> CredRef {
        let g = self.p_p.ps_mtx.lock();
        match self.cred_ref.get(&g) {
            Some(c) => c.clone(),
            None => kpanic(format_args!("p_ucred: no credentials")),
        }
    }

    /// Whether the process's credentials changed since the thread copied them
    /// (`refreshcreds`: `pr->ps_ucred != p->p_ucred`, without a lock, as the C).
    #[inline]
    pub fn cred_stale(&self) -> bool {
        self.p_p.ucred_word() != self.cred_from.load(Ordering::Relaxed)
    }

    /// `dorefreshcreds`: the thread takes its process's credentials (copied under `ps_mtx`, the
    /// old reference dropped after).
    pub fn refresh_cred(&self) {
        let old = {
            let mut g = self.p_p.ps_mtx.lock();
            let Some(new) = g.cred.clone() else {
                return;
            };
            self.install_cred(&mut g, new)
        };
        drop(old);
    }

    /// `p->p_ucred = cr` for the thread alone (`thread_new`'s `crhold`, exec's, coredump's,
    /// `access(2)`'s): the old reference.
    pub fn set_cred(&self, cr: CredRef) -> Option<CredRef> {
        let mut g = self.p_p.ps_mtx.lock();
        self.install_cred(&mut g, cr)
    }

    /// The copy and the reference, under the process's `ps_mtx`.
    fn install_cred(
        &self,
        g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>,
        cr: CredRef,
    ) -> Option<CredRef> {
        self.cred.copy_from(&cr);
        self.cred_from
            .store(CredRef::as_ptr(&cr).addr(), Ordering::Relaxed);
        self.cred_ref.get_mut(g).replace(cr)
    }

    /// `p->p_vmspace`'s handle (`None` after the reaper took it).
    pub fn vmspace_share(&self) -> Option<VmspaceRef> {
        let g = self.p_p.ps_mtx.lock();
        self.vmspace.get(&g).clone()
    }

    /// `p->p_vmspace = vm` (`fork1`'s copy of the process's, exec's, the reaper's `NULL`): the
    /// old handle. The word keeps the address exposed: `bsd`'s `vmspace()` turns it back into
    /// a reference until N4.5.
    pub fn set_vmspace(&self, vm: Option<VmspaceRef>) -> Option<VmspaceRef> {
        let addr = vm
            .as_ref()
            .map_or(0, |v| PoolArc::as_ptr(v).expose_provenance());
        let mut g = self.p_p.ps_mtx.lock();
        let old = core::mem::replace(self.vmspace.get_mut(&mut g), vm);
        self.vmspace_word.store(addr, Ordering::Release);
        old
    }

    /// `p->p_vmspace = None` under the caller's guard of its process's `ps_mtx` (the reaper's
    /// zap of a dead main thread's copy, which its process keeps): the old handle, which the
    /// caller drops outside the lock.
    pub fn take_vmspace_locked(
        &self,
        g: &mut hal::mutex::MutexedGuard<'_, PsMtx<G>>,
    ) -> Option<VmspaceRef> {
        let old = self.vmspace.get_mut(g).take();
        self.vmspace_word.store(0, Ordering::Release);
        old
    }

    /// `p->p_vmspace` as an address, read without the lock (the fault and syscall paths' read
    /// through `bsd`'s `vmspace()` until N4.5), 0 for none.
    #[inline]
    pub fn vmspace_word(&self) -> usize {
        self.vmspace_word.load(Ordering::Acquire)
    }

    /// `P_HASSIBLING(p)`.
    pub fn has_sibling(&self) -> bool {
        self.p_p.ps_threadcnt.get() > 1
    }

    /// `lim_read_enter`'s refresh: the thread's copy follows its process's limits; the
    /// limit `which`'s `rlim_cur` (`lim_cur`).
    pub fn lim_cur(&self, which: usize) -> hal::sys::types::Rlim {
        let pr = &self.p_p;
        if !self.p_limit.is_from(pr.limit_word()) {
            let g = pr.ps_mtx.lock();
            if let Some(l) = g.limit.as_ref() {
                self.p_limit.fill(PlimitRef::addr(l), l);
            }
        }
        self.p_limit.cur(which)
    }
}

impl<G: ProcGlue> Deref for Thread<G> {
    type Target = G::ThreadG;

    fn deref(&self) -> &G::ThreadG {
        &self.g
    }
}

/// What `hal::thread`'s switch reaches of a thread: its slot, first in the `#[repr(C)]`
/// thread (decision P1), so the thread's address, `ci_curproc`, is the slot's for the
/// assembly that reads the slot by `ci_curproc` plus an offset (arm64's `exception.S`).
impl<G: ProcGlue> hal::thread::ThreadLayout for Thread<G> {
    #[inline]
    fn machine(&self) -> &MachineSlot {
        &self.machine
    }
}

/// `thread_new`'s `pool_get(&proc_pool, PR_WAITOK)`: a thread of `pr`, its box (the caller
/// gives it to the process, [`Process::insert_thread`]).
pub fn thread_alloc<G: ProcGlue>(pr: &ProcessRef<G>, g: G::ThreadG) -> PoolBox<Thread<G>> {
    let make = || {
        Thread::make(
            pr.clone(),
            Guarded::new_mutexed(&pr.ps_mtx, None),
            Guarded::new_mutexed(&pr.ps_mtx, None),
            g,
        )
    };
    match G::tables().procpl.get(PR_WAITOK, make) {
        Some(t) => t,
        None => kpanic(format_args!("thread_new: proc_pool is empty")),
    }
}

/// `suser(p)`: `Ok` for a thread with root's credentials, `EPERM` otherwise.
pub fn suser<G: ProcGlue>(p: &Thread<G>) -> Result<(), hal::sys::errno::Errno> {
    crate::cred::suser_ucred(p.ucred())
}

/// `uid_find(uid)` ... `uid_release(uip)`: `f` runs on the entry of `uid` in the glue's
/// table (made on first need) with `uidinfolk` held for writing, as between the C's pair.
pub fn with_uidinfo<G: ProcGlue, R>(uid: Uid, f: impl FnOnce(&Uidinfo) -> R) -> R {
    let Some(tbl) = G::tables().uihash.get() else {
        kpanic(format_args!("uid_find: before procinit"));
    };
    let chain = &tbl[(uid as usize) & (tbl.len() - 1)];
    {
        let w = UIDINFOLK.write();
        if let Some(uip) = chain.iter(&w).find(|u| u.ui_uid == uid) {
            return f(uip);
        }
    }
    // malloc(M_WAITOK) may sleep: not under uidinfolk.
    let Some(nuip) = MBox::new(M_PROC, M_WAITOK, Uidinfo::new(uid)) else {
        kpanic(format_args!("uid_find: no memory"));
    };
    let mut w = UIDINFOLK.write();
    if let Some(uip) = chain.iter(&w).find(|u| u.ui_uid == uid) {
        // Made meanwhile: `nuip` is in no list, freed.
        return f(uip);
    }
    let nuip: &'static Uidinfo = MBox::leak(nuip);
    chain.insert_head(&mut w, pin_static(nuip));
    f(nuip)
}

/// `chgproccnt(uid, diff)`: the threads `uid` has, changed by `diff`; `panic(9)` below 0.
pub fn chgproccnt<G: ProcGlue>(uid: Uid, diff: i64) -> i64 {
    let count = with_uidinfo::<G, _>(uid, |uip| {
        let c = uip.ui_proccnt.get() + diff;
        uip.ui_proccnt.set(c);
        c
    });
    if count < 0 {
        kpanic(format_args!("chgproccnt: procs < 0"));
    }
    count
}

/// `inferior(pr, parent)`: whether `pr` descends from `parent` (itself included).
pub fn inferior<G: ProcGlue, W: Witness + ?Sized>(
    w: &W,
    pr: &Process<G>,
    parent: &Process<G>,
) -> bool {
    let mut p = pr;
    while !ptr::eq(p, parent) {
        if p.ps_pid == 0 || p.ps_pid == 1 {
            return false;
        }
        match p.pptr(w) {
            Some(pp) => p = pp,
            None => return false,
        }
    }
    true
}

/// `tfind(tid)`: the thread `tid`.
pub fn tfind<G: ProcGlue, W: Witness + ?Sized>(w: &W, tid: Pid) -> Option<&Thread<G>> {
    G::tables()
        .tidhash(tid)
        .iter(w)
        .find(|p| p.p_tid.get() == tid)
}

/// `tfind_user(tid, pr)`: the thread of `pr` userland names `tid` (`THREAD_PID_OFFSET` added).
pub fn tfind_user<'a, G: ProcGlue, W: Witness + ?Sized>(
    w: &'a W,
    tid: Pid,
    pr: &Process<G>,
) -> Option<&'a Thread<G>> {
    if tid < THREAD_PID_OFFSET {
        return None;
    }
    let p = tfind::<G, W>(w, tid - THREAD_PID_OFFSET)?;
    // verify we found a thread in the correct process
    ptr::eq(p.process(), pr).then_some(p)
}

/// [`tfind_user`], the thread pinned (as its hash chain links it): for a caller that makes
/// it runnable (`thrkill`).
pub fn tfind_user_pinned<'a, G: ProcGlue, W: Witness + ?Sized>(
    w: &'a W,
    tid: Pid,
    pr: &Process<G>,
) -> Option<Pin<&'a Thread<G>>> {
    if tid < THREAD_PID_OFFSET {
        return None;
    }
    let tid = tid - THREAD_PID_OFFSET;
    G::tables()
        .tidhash(tid)
        .iter_pinned(w)
        .find(|p| p.p_tid.get() == tid)
        .filter(|p| ptr::eq(p.process(), pr))
}

/// `prfind(pid)`: the process `pid` (a caller that keeps it takes `PoolArc::from_ref`).
pub fn prfind<G: ProcGlue, W: Witness + ?Sized>(w: &W, pid: Pid) -> Option<&Process<G>> {
    G::tables().pidhash(pid).iter(w).find(|pr| pr.ps_pid == pid)
}

/// `pgfind(pgid)`: the process group `pgid`.
pub fn pgfind<G: ProcGlue, W: Witness + ?Sized>(w: &W, pgid: Pid) -> Option<&Pgrp<G>> {
    G::tables()
        .pgrphash(pgid)
        .iter(w)
        .find(|pg| pg.pg_id.get() == pgid)
}

/// `zombiefind(pid)`: the zombie `pid`.
pub fn zombiefind<G: ProcGlue, W: Witness + ?Sized>(w: &W, pid: Pid) -> Option<&Process<G>> {
    G::tables().zombprocess.iter(w).find(|pr| pr.ps_pid == pid)
}

/// The session at address `addr` among the sessions of the groups in `pgrphash`, for a
/// callback that was handed the session's address as its argument (`s_verauthto`'s
/// `zapverauth`, where the C casts its `void *`): `None` once no group holds it, when no
/// process could read what the callback would change.
pub fn session_find<G: ProcGlue, W: Witness + ?Sized>(w: &W, addr: usize) -> Option<&Session<G>> {
    let tbl = G::tables().pgrphash.get()?;
    tbl.iter()
        .flat_map(|chain| chain.iter(w))
        .filter_map(|pg| pg.session(w))
        .find(|s| ptr::from_ref::<Session<G>>(s).addr() == addr)
}

/// `enternewpgrp(pr, pgrp, newsess)`: `pr` moves to the new group `pgrp` (from
/// [`pgrp_alloc`], made before the caller's checks as the C's), of the new session `newsess`
/// (from [`session_alloc`]; `pr` leads it) or of its own session. Takes the kernel lock's
/// witnesses itself: the caller holds the lock, none of its witnesses.
pub fn enternewpgrp<G: ProcGlue>(
    pr: &ProcessRef<G>,
    pgrp: PgrpRef<G>,
    newsess: Option<SessionRef<G>>,
) {
    #[cfg(feature = "diagnostic")]
    {
        let r = klock::reader();
        if pr.sess_leader(&r) {
            kpanic(format_args!(
                "enternewpgrp: session leader attempted setpgrp"
            ));
        }
    }
    let pid = pr.ps_pid;
    {
        let mut w = klock::witness();
        let sess = match newsess {
            Some(s) => {
                // New session. Initialize it completely.
                s.set_leader(Some(pr));
                let login = pr.session(&w).map(|old| *old.s_login.get(&w));
                if let Some(login) = login {
                    *s.s_login.get_mut(&mut w) = login;
                }
                pr.ps_flags.fetch_and(!PS_CONTROLT, Ordering::SeqCst);
                s
            }
            None => match pr.pgrp(&w).and_then(|pg| pg.pg_session.get(&w).clone()) {
                Some(s) => s,
                None => kpanic(format_args!("enternewpgrp: no session")),
            },
        };
        *pgrp.pg_session.get_mut(&mut w) = Some(sess);
        pgrp.pg_id.set(pid);
        pgrp.pg_jobc.set(0);
        G::tables().pgrphash(pid).insert_head(&mut w, pgrp.as_pin());
    }
    enterthispgrp(pr, pgrp);
}

/// `enterthispgrp(pr, pgrp)`: `pr` moves to the existing group `pgrp`; its old group goes
/// when it was the last member. `fixjobc` runs under a reader (it may signal), the moves
/// under the writer.
pub fn enterthispgrp<G: ProcGlue>(pr: &ProcessRef<G>, pgrp: PgrpRef<G>) {
    let savepgrp = {
        let r = klock::reader();
        let save = pr.pgrp(&r).cloned();
        // Adjust eligibility of affected pgrps to participate in job control. Increment
        // eligibility counts before decrementing, otherwise we could reach 0 spuriously
        // during the first call.
        fixjobc(&r, pr, &pgrp, true);
        if let Some(save) = &save {
            fixjobc(&r, pr, save, false);
        }
        save
    };
    let (old, empty) = {
        let mut w = klock::witness();
        if let Some(save) = &savepgrp {
            save.pg_members.remove(&mut w, pr);
        }
        let old = pr.set_pgrp(&mut w, Some(pgrp.clone()));
        pgrp.pg_members.insert_head(&mut w, pr.as_pin());
        let empty = savepgrp.as_ref().is_some_and(|s| s.pg_members.is_empty(&w));
        (old, empty)
    };
    drop(old);
    if let Some(save) = savepgrp
        && empty
    {
        pgdelete(save);
    }
}

/// `leavepgrp(pr)`: `pr` leaves its group (`process_zap`); the group goes with its last
/// member.
pub fn leavepgrp<G: ProcGlue>(pr: &ProcessRef<G>) {
    let Some(savepgrp) = ({
        let r = klock::reader();
        if let Some(sess) = pr.session(&r)
            && sess.s_verauthppid.get() == pr.ps_pid
        {
            zapverauth(sess);
        }
        pr.pgrp(&r).cloned()
    }) else {
        return;
    };
    let (old, empty) = {
        let mut w = klock::witness();
        let old = pr.set_pgrp(&mut w, None);
        savepgrp.pg_members.remove(&mut w, pr);
        (old, savepgrp.pg_members.is_empty(&w))
    };
    drop(old);
    if empty {
        pgdelete(savepgrp);
    }
}

/// `pgdelete(pgrp)`: a group without members leaves the hash; its sigio list and its
/// terminal's pointer to it go (the glue), and its session reference drops (`SESSRELE`)
/// after the lock's witness, with the group's last reference.
pub fn pgdelete<G: ProcGlue>(pgrp: PgrpRef<G>) {
    crate::sigio::sigio_freelist_pgrp(&pgrp);
    G::pgrp_destroy(&pgrp);
    let sess = {
        let mut w = klock::witness();
        G::tables().pgrphash(pgrp.pg_id.get()).remove(&mut w, &pgrp);
        pgrp.pg_session.get_mut(&mut w).take()
    };
    drop(sess);
    drop(pgrp);
}

/// `zapverauth(sess)`: the session's verified authentication forgotten (`s_verauthto`).
pub fn zapverauth<G: ProcGlue>(sess: &Session<G>) {
    sess.s_verauthuid.set(0);
    sess.s_verauthppid.set(0);
}

/// `fixjobc(pr, pgrp, entering)`: the job-control counts of `pgrp` and of `pr`'s children's
/// groups as `pr` enters (`true`) or leaves `pgrp`: a process qualifies its group when its
/// parent is in another group of the same session; a group whose count reaches 0 is orphaned
/// ([`orphanpg`]). Under a reader of the kernel lock (it may signal).
pub fn fixjobc<G: ProcGlue, W: Witness + ?Sized>(
    w: &W,
    pr: &Process<G>,
    pgrp: &Pgrp<G>,
    entering: bool,
) {
    let mysession = pgrp.session_addr(w);

    // Check pr's parent to see whether pr qualifies its own process group; if so, adjust
    // count for pr's process group.
    if let Some(parent) = pr.pptr(w)
        && let Some(hispgrp) = parent.pgrp(w)
        && !ptr::eq::<Pgrp<G>>(&**hispgrp, pgrp)
        && hispgrp.session_addr(w) == mysession
    {
        jobc_adjust(w, pgrp, entering);
    }

    // Check this process' children to see whether they qualify their process groups; if
    // so, adjust counts for children's process groups.
    for child in pr.ps_children.iter(w) {
        if let Some(hispgrp) = child.pgrp(w)
            && !ptr::eq::<Pgrp<G>>(&**hispgrp, pgrp)
            && hispgrp.session_addr(w) == mysession
            && child.ps_flags.load(Ordering::Relaxed) & PS_ZOMBIE == 0
        {
            jobc_adjust(w, hispgrp, entering);
        }
    }
}

/// `pg_jobc++`, or `pg_jobc--` and [`orphanpg`] at 0.
fn jobc_adjust<G: ProcGlue, W: Witness + ?Sized>(w: &W, pg: &Pgrp<G>, entering: bool) {
    if entering {
        pg.pg_jobc.set(pg.pg_jobc.get() + 1);
    } else {
        let n = pg.pg_jobc.get() - 1;
        pg.pg_jobc.set(n);
        if n == 0 {
            orphanpg(w, pg);
        }
    }
}

/// `killjobc(pr)`: an exiting process: as a session leader, its controlling terminal's
/// foreground group hung up and the terminal revoked (the glue: it may sleep, no witness is
/// out), the session left without a leader; then it leaves job control.
pub fn killjobc<G: ProcGlue>(pr: &ProcessRef<G>) {
    let sess = {
        let r = klock::reader();
        if pr.sess_leader(&r) {
            pr.pgrp(&r).and_then(|pg| pg.pg_session.get(&r).clone())
        } else {
            None
        }
    };
    if let Some(sp) = sess {
        G::session_hangup(&sp);
        sp.set_leader(None);
    }
    let r = klock::reader();
    if let Some(pg) = pr.pgrp(&r) {
        fixjobc(&r, pr, pg, false);
    }
}

/// `orphanpg(pg)`: an orphaned group with a stopped member: every member hung up and
/// continued.
fn orphanpg<G: ProcGlue, W: Witness + ?Sized>(w: &W, pg: &Pgrp<G>) {
    if pg
        .pg_members
        .iter(w)
        .any(|pr| pr.ps_flags.load(Ordering::Relaxed) & PS_STOPPED != 0)
    {
        for pr in pg.pg_members.iter(w) {
            crate::sig::prsignal(pr, SIGHUP);
            crate::sig::prsignal(pr, SIGCONT);
        }
    }
}

// proc_printit, db_kill_cmd, db_stop_cmd, db_show_all_procs: the real ddb. pgrpdump: DEBUG.
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;

    use super::*;
    use crate::cred::{CredRef, Ucred};
    use crate::testutil;

    /// The kernel's own glue for its tests: no member of `bsd`'s, the signals recorded.
    pub struct TestGlue;

    crate::proc_glue_links!(TestGlue);
    crate::test_file_glue!(TestFiles, TestGlue);

    std::thread_local! {
        static TABLES: &'static ProcTables<TestGlue> = Box::leak(Box::new(ProcTables::new()));
        /// Tables a test puts in place of the thread's own for a while.
        static TABLES_OVERRIDE: core::cell::Cell<Option<&'static ProcTables<TestGlue>>> =
            const { core::cell::Cell::new(None) };
    }

    impl ProcGlue for TestGlue {
        type ProcessG = ();
        type ThreadG = ();
        type PgrpG = ();
        type SessionG = ();

        type FileGlue = TestFiles;
        crate::proc_glue_link_types!();

        fn tables() -> &'static ProcTables<Self> {
            TABLES_OVERRIDE
                .with(core::cell::Cell::get)
                .unwrap_or_else(|| TABLES.with(|t| *t))
        }

        fn sched() -> &'static crate::sched::SchedTables<Self> {
            static SCHED: crate::sched::SchedTables<TestGlue> = crate::sched::SchedTables::new();
            &SCHED
        }

        fn pgrp_destroy(_pg: &Pgrp<Self>) {}
        fn session_hangup(_sess: &Session<Self>) {}
        fn session_placed(_sess: &Session<Self>) {}
        fn session_destroy(_sess: &Session<Self>) {}
        fn with_curproc<R>(f: impl FnOnce(Option<&Thread<Self>>) -> R) -> R {
            f(None)
        }
        fn exit1(_p: &Thread<Self>, _xexit: i32, xsig: i32, _flags: i32) -> ! {
            std::panic!("exit1 of a test thread (signal {xsig})")
        }
        fn pledge_fail(
            _p: &Thread<Self>,
            error: hal::sys::errno::Errno,
            _code: u64,
        ) -> hal::sys::errno::Errno {
            error
        }
        fn process_ref(pr: &Process<Self>) -> ProcessRef<Self> {
            StaticOrPooled::Pooled(PoolArc::from_ref(pr).unwrap())
        }
    }

    type P = ProcessRef<TestGlue>;

    /// Fresh tables for this test thread, the kernel lock held (the double counts it).
    fn setup() {
        testutil::install();
        hal::klock::kernel_lock();
        procinit::<TestGlue>(64, 64);
    }

    /// A process `pid` on `allprocess` and its pid chain.
    fn process(pid: Pid) -> P {
        let pr = process_alloc::<TestGlue>(pid, ());
        let t = TestGlue::tables();
        let mut w = klock::witness();
        t.allprocess.insert_head(&mut w, pr.as_pin());
        t.pidhash(pid).insert_head(&mut w, pr.as_pin());
        pr
    }

    /// `pr` off its lists (the process lists, its parent's children).
    fn unlink(pr: &P) {
        let t = TestGlue::tables();
        let mut w = klock::witness();
        t.allprocess.remove(&mut w, pr);
        t.pidhash(pr.ps_pid).remove(&mut w, pr);
        if let Some(parent) = pr.pptr(&w).cloned() {
            parent.ps_children.remove(&mut w, pr);
        }
    }

    #[test]
    fn a_process_is_found_by_pid_and_kept_by_a_handle() {
        setup();
        let pr = process(10);
        let kept = {
            let r = klock::reader();
            let found = prfind::<TestGlue, _>(&r, 10).unwrap();
            assert!(ptr::eq(found, &*pr));
            assert!(prfind::<TestGlue, _>(&r, 11).is_none());
            PoolArc::from_ref(found).unwrap()
        };
        let StaticOrPooled::Pooled(h) = &pr else {
            panic!("a pooled process");
        };
        assert_eq!(PoolArc::count(h), 2);
        unlink(&pr);
        drop(pr);
        assert_eq!(kept.ps_pid, 10);
    }

    #[test]
    #[should_panic]
    fn a_process_dropped_while_linked_stops_the_kernel() {
        setup();
        // On one list only: a second linked entry would panic again during the unwind.
        let pr = process_alloc::<TestGlue>(12, ());
        {
            let mut w = klock::witness();
            TestGlue::tables()
                .allprocess
                .insert_head(&mut w, pr.as_pin());
        }
        drop(pr);
    }

    #[test]
    fn a_thread_joins_its_process_and_leaves_it() {
        setup();
        let pr = process(20);
        let t = thread_alloc::<TestGlue>(&pr, ());
        t.p_tid.set(5000);
        {
            let mut w = klock::witness();
            let tb = TestGlue::tables();
            tb.allproc.insert_head(&mut w, t.as_pin());
            tb.tidhash(5000).insert_head(&mut w, t.as_pin());
        }
        pr.ps_threadcnt.set(1);
        let at = PoolBox::as_ptr(&t);
        {
            let mut g = pr.ps_mtx.lock();
            pr.join_locked(&mut g, t.as_pin());
            pr.insert_box_locked(&mut g, t);
        }
        {
            let r = klock::reader();
            let found = tfind::<TestGlue, _>(&r, 5000).unwrap();
            assert!(ptr::eq(found, at));
            assert!(tfind_user::<TestGlue, _>(&r, 5000 + THREAD_PID_OFFSET, &pr).is_some());
            assert!(tfind_user::<TestGlue, _>(&r, 5000, &pr).is_none());
            assert!(!found.has_sibling());
            assert_eq!(found.process().ps_pid, 20);
        }
        {
            let g = pr.ps_mtx.lock();
            assert_eq!(pr.ps_threads.iter(&g).count(), 1);
        }
        let seq = {
            let g = pr.ps_mtx.lock();
            pr.ps_threads.iter(&g).next().unwrap().p_seq.get()
        };
        let b = {
            let mut g = pr.ps_mtx.lock();
            let b = pr.take_box_locked(&mut g, seq).unwrap();
            pr.leave_locked(&mut g, &b);
            b
        };
        assert_eq!(b.p_tid.get(), 5000);
        {
            let mut w = klock::witness();
            let tb = TestGlue::tables();
            tb.allproc.remove(&mut w, &b);
            tb.tidhash(5000).remove(&mut w, &b);
        }
        drop(b);
        assert!(pr.take_box_locked(&mut pr.ps_mtx.lock(), seq).is_none());
        unlink(&pr);
    }

    #[test]
    fn a_thread_s_credentials_are_its_own_copy_refreshed_on_change() {
        setup();
        let pr = process(30);
        let mut ids = Ucred::new();
        ids.cr_uid.set(1000);
        let first = CredRef::new(ids);
        drop(pr.set_ucred(first.clone()));
        let t = thread_alloc::<TestGlue>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        assert_eq!(t.ucred().cr_uid.get(), 1000);
        assert!(!t.cred_stale());
        assert!(CredRef::ptr_eq(&t.cred_ref(), &first));

        let mut ids = Ucred::new();
        ids.cr_uid.set(0);
        drop(pr.set_ucred(CredRef::new(ids)));
        assert!(t.cred_stale());
        // The copy is the thread's until it refreshes.
        assert_eq!(t.ucred().cr_uid.get(), 1000);
        t.refresh_cred();
        assert!(!t.cred_stale());
        assert_eq!(t.ucred().cr_uid.get(), 0);
        assert_eq!(suser(&*t), Ok(()));
        // The first credentials are held by `first` alone now.
        assert_eq!(CredRef::count(&first), 1);
        drop(t);
        unlink(&pr);
    }

    #[test]
    fn a_thread_s_sleep_message_and_names() {
        setup();
        let pr = process(31);
        let t = thread_alloc::<TestGlue>(&pr, ());
        assert_eq!(t.p_wmesg.get(), None);
        t.p_wmesg.set(Some("nanoslp"));
        assert_eq!(t.p_wmesg.get(), Some("nanoslp"));
        t.set_name(b"worker");
        assert_eq!(&*t.name(), b"worker");
        pr.set_comm(b"a very long command name that does not fit");
        assert_eq!(pr.comm().len(), _MAXCOMLEN - 1);
        assert_eq!(t.stat(), 0);
        t.set_stat(SONPROC);
        assert_eq!(t.machine.stat(), SONPROC);
        assert_eq!(core::mem::offset_of!(Thread<TestGlue>, machine), 0);
        drop(t);
        unlink(&pr);
    }

    #[test]
    fn groups_sessions_and_job_control() {
        setup();
        // A leads a new session and its group; B, A's child, starts in A's group.
        let a = process(40);
        enternewpgrp(
            &a,
            pgrp_alloc::<TestGlue>(()),
            Some(session_alloc::<TestGlue>(())),
        );
        let b = process(41);
        {
            let mut w = klock::witness();
            drop(b.set_pptr(&mut w, a.clone()));
            a.ps_children.insert_head(&mut w, b.as_pin());
        }
        let g1 = {
            let r = klock::reader();
            assert!(a.sess_leader(&r));
            assert_eq!(a.pgid(&r), 40);
            assert_eq!(b.ps_ppid.get(), 40);
            pgfind::<TestGlue, _>(&r, 40).unwrap();
            a.pgrp(&r).unwrap().clone()
        };
        enterthispgrp(&b, g1.clone());
        assert_eq!(g1.pg_jobc.get(), 0);
        // B moves to a group of its own in the same session: A, its parent, is in another
        // group of the session, so B qualifies it.
        enternewpgrp(&b, pgrp_alloc::<TestGlue>(()), None);
        let g2 = {
            let r = klock::reader();
            assert_eq!(b.pgid(&r), 41);
            assert!(!b.sess_leader(&r));
            assert!(ptr::eq(b.session(&r).unwrap(), a.session(&r).unwrap()));
            assert!(inferior(&r, &b, &a));
            assert!(!inferior(&r, &a, &b));
            b.pgrp(&r).unwrap().clone()
        };
        assert_eq!(g2.pg_jobc.get(), 1);
        // The group is orphaned with a stopped member: hung up and continued (`prsignal` of
        // both to B, whose one thread is not running: both stay pending on the process).
        crate::sig::test_sigacts(&b);
        {
            let t = thread_alloc::<TestGlue>(&b, ());
            let mut g = b.ps_mtx.lock();
            b.join_locked(&mut g, t.as_pin());
            b.insert_box_locked(&mut g, t);
        }
        b.ps_flags.fetch_or(PS_STOPPED, Ordering::Relaxed);
        {
            let r = klock::reader();
            fixjobc(&r, &b, &g2, false);
        }
        assert_eq!(g2.pg_jobc.get(), 0);
        assert_eq!(
            b.ps_siglist.load(Ordering::Relaxed),
            hal::sys::signal::sigmask(SIGHUP) | hal::sys::signal::sigmask(SIGCONT)
        );
        let seq = {
            let g = b.ps_mtx.lock();
            b.ps_threads.first(&g).unwrap().p_seq.get()
        };
        {
            let mut g = b.ps_mtx.lock();
            let t = b.take_box_locked(&mut g, seq).unwrap();
            b.leave_locked(&mut g, &t);
            drop(g);
            drop(t);
        }
        // Leaving: the groups go with their last members, the session with its last group.
        drop(g2);
        drop(g1);
        leavepgrp(&b);
        leavepgrp(&a);
        {
            let r = klock::reader();
            assert!(pgfind::<TestGlue, _>(&r, 41).is_none());
            assert!(pgfind::<TestGlue, _>(&r, 40).is_none());
        }
        unlink(&b);
        unlink(&a);
    }

    #[test]
    fn a_user_s_process_count() {
        setup();
        assert_eq!(chgproccnt::<TestGlue>(64_001, 1), 1);
        assert_eq!(chgproccnt::<TestGlue>(64_001, 2), 3);
        assert_eq!(chgproccnt::<TestGlue>(64_001, -3), 0);
        with_uidinfo::<TestGlue, _>(64_001, |u| {
            assert_eq!(u.ui_uid, 64_001);
            u.ui_lockcnt.set(u.ui_lockcnt.get() + 1);
        });
        assert_eq!(
            with_uidinfo::<TestGlue, _>(64_001, |u| u.ui_lockcnt.get()),
            1
        );
    }

    #[test]
    fn fresh_tables_count_every_user_from_zero() {
        // The uid table is the glue's (a test thread's own): a count left by earlier tables
        // (another test, before a reset) is not seen by fresh ones.
        setup();
        assert_eq!(chgproccnt::<TestGlue>(64_003, 2), 2);
        let fresh: &'static ProcTables<TestGlue> = Box::leak(Box::new(ProcTables::new()));
        TABLES_OVERRIDE.with(|o| o.set(Some(fresh)));
        procinit::<TestGlue>(64, 64);
        assert_eq!(chgproccnt::<TestGlue>(64_003, 1), 1);
        TABLES_OVERRIDE.with(|o| o.set(None));
        assert_eq!(chgproccnt::<TestGlue>(64_003, 1), 3);
    }

    #[test]
    #[should_panic(expected = "procs < 0")]
    fn a_negative_count_is_refused() {
        setup();
        chgproccnt::<TestGlue>(64_002, -1);
    }

    #[test]
    fn a_session_s_leader_and_its_lookup_by_address() {
        setup();
        let a = process(50);
        let sess = session_alloc::<TestGlue>(());
        enternewpgrp(&a, pgrp_alloc::<TestGlue>(()), Some(sess.clone()));
        assert_eq!(sess.leader_pid(), Some(50));
        assert!(sess.led_by(&a));
        {
            let r = klock::reader();
            let found = session_find::<TestGlue, _>(&r, sess.addr()).unwrap();
            assert!(ptr::eq(found, &*sess));
            assert!(session_find::<TestGlue, _>(&r, sess.addr() + 1).is_none());
        }
        // The leader gone (exit1's `sp->s_leader = NULL`): getsid's ESRCH.
        sess.set_leader(None);
        assert_eq!(sess.leader_pid(), None);
        assert!(!sess.led_by(&a));
        drop(sess);
        leavepgrp(&a);
        unlink(&a);
    }

    #[test]
    fn the_parent_and_group_read_under_ps_mtx_follow_their_changes() {
        setup();
        let a = process(60);
        let b = process(61);
        enternewpgrp(
            &a,
            pgrp_alloc::<TestGlue>(()),
            Some(session_alloc::<TestGlue>(())),
        );
        {
            let mut w = klock::witness();
            drop(b.set_pptr(&mut w, a.clone()));
            a.ps_children.insert_head(&mut w, b.as_pin());
        }
        let group = {
            let r = klock::reader();
            a.pgrp(&r).unwrap().clone()
        };
        enterthispgrp(&b, group.clone());
        {
            let g = b.ps_mtx.lock();
            assert!(StaticOrPooled::ptr_eq(b.pptr_locked(&g).unwrap(), &a));
            assert!(StaticOrPooled::ptr_eq(b.pgrp_locked(&g).unwrap(), &group));
        }
        drop(group);
        leavepgrp(&b);
        assert!(b.pgrp_locked(&b.ps_mtx.lock()).is_none());
        leavepgrp(&a);
        unlink(&b);
        unlink(&a);
    }

    #[test]
    #[should_panic]
    fn a_guard_of_another_process_s_mutex_is_refused() {
        setup();
        // On no list: nothing panics again while the test unwinds.
        let a = process_alloc::<TestGlue>(70, ());
        let b = process_alloc::<TestGlue>(71, ());
        let g = a.ps_mtx.lock();
        let _ = b.pptr_locked(&g);
    }

    #[test]
    fn a_thread_joins_and_leaves_under_the_caller_s_guard() {
        setup();
        let pr = process(80);
        let t = thread_alloc::<TestGlue>(&pr, ());
        t.p_tid.set(6000);
        let mut g = pr.ps_mtx.lock();
        let at = PoolBox::as_ptr(&t);
        pr.join_locked(&mut g, t.as_pin());
        pr.insert_box_locked(&mut g, t);
        assert_eq!(pr.ps_threads.iter(&g).count(), 1);
        let seq = pr.ps_threads.iter(&g).next().unwrap().p_seq.get();
        assert!(pr.take_box_locked(&mut g, seq + 1).is_none());
        let b = pr.take_box_locked(&mut g, seq).unwrap();
        pr.leave_locked(&mut g, &b);
        assert!(ptr::eq(PoolBox::as_ptr(&b), at));
        assert_eq!(pr.ps_threads.iter(&g).count(), 0);
        drop(g);
        drop(b);
        unlink(&pr);
    }

    /// A process owns its main thread's box in its slot and a secondary thread's in its
    /// tree; both are on `ps_threads`, found by address while they are, and each box comes
    /// back exactly once (the main one at `process_zap`, a secondary one at its `exit1`).
    #[test]
    fn the_main_thread_is_the_slot_s_and_a_secondary_one_the_tree_s() {
        setup();
        let pr = process(30);
        let main = thread_alloc::<TestGlue>(&pr, ());
        let second = thread_alloc::<TestGlue>(&pr, ());
        let (main_at, second_at) = (
            ptr::from_ref::<Thread<TestGlue>>(&main).addr(),
            ptr::from_ref::<Thread<TestGlue>>(&second).addr(),
        );
        {
            let mut g = pr.ps_mtx.lock();
            assert!(pr.main_thread(&g).is_none());
            pr.join_locked(&mut g, main.as_pin());
            pr.set_main_locked(&mut g, main);
            pr.join_locked(&mut g, second.as_pin());
            let seq = second.p_seq.get();
            pr.insert_box_locked(&mut g, second);
            let m = pr.main_thread(&g).unwrap();
            assert_eq!(ptr::from_ref::<Thread<TestGlue>>(&m).addr(), main_at);
            assert!(pr.thread_by_addr(&g, second_at).is_some());
            assert!(pr.thread_by_addr(&g, 0).is_none());
            assert!(pr.thread_by_addr(&g, main_at + 8).is_none());
            // The secondary thread leaves: off the list, its box back.
            let b = pr.take_box_locked(&mut g, seq).unwrap();
            pr.leave_locked(&mut g, &b);
            assert!(pr.thread_by_addr(&g, second_at).is_none());
            assert!(pr.take_box_locked(&mut g, seq).is_none());
            drop(g);
            drop(b);
        }
        let mut g = pr.ps_mtx.lock();
        let m = pr
            .main_thread(&g)
            .map(|m| ptr::from_ref::<Thread<TestGlue>>(&m).addr());
        assert_eq!(m, Some(main_at));
        // The main thread leaves the list at its exit1; its box stays for process_zap.
        let at = pr
            .thread_by_addr(&g, main_at)
            .map(|t| ptr::from_ref::<Thread<TestGlue>>(&t));
        assert!(at.is_some());
        let main = pr.take_main_locked(&mut g).unwrap();
        pr.leave_locked(&mut g, &main);
        assert!(pr.main_thread(&g).is_none());
        assert!(pr.take_main_locked(&mut g).is_none());
        drop(g);
        drop(main);
        unlink(&pr);
    }

    /// `process_new`'s copy of a pin table is a table of its own with the same bounds; a
    /// replacement hands the old table back.
    #[test]
    fn a_pin_table_is_duplicated_and_replaced() {
        testutil::install();
        let pins =
            MArray::new(crate::vm::mmap::M_PINSYSCALL, M_WAITOK, 4, |i| i as u32 * 8).unwrap();
        let t = PinTable {
            start: 0x1000,
            end: 0x2000,
            pins: Some(pins),
        };
        let at = t.pins.as_ref().unwrap().as_ptr();
        let cell = ReadMostly::new(t);
        let d = PinTable::duplicate(&cell);
        assert_eq!((d.start, d.end, d.npins()), (0x1000, 0x2000, 4));
        assert_eq!(&**d.pins.as_ref().unwrap(), &[0, 8, 16, 24]);
        assert_ne!(d.pins.as_ref().unwrap().as_ptr(), at);
        let old = cell.replace(PinTable::NONE);
        assert_eq!(old.npins(), 4);
        assert_eq!(cell.read().npins(), 0);
        assert_eq!(PinTable::duplicate(&cell).npins(), 0);
    }
}
/* </TESTS> */
