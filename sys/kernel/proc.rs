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
//! own. The run-queue link `p_runq` is a member of the thread itself (`G::RunqEntry`): `bsd`'s
//! list adapters take its offset in the thread.
//!
//! Locks, as the C's annotations: `[K]` the kernel lock (the lists, the hashes, the parent,
//! child and group links: [`hal::klock`]'s witnesses), `[m]` the process's `ps_mtx` (its thread
//! list and thread tree, its credentials, address space and limits: [`PsMtx`]), `[S]`
//! `sched_lock` (still `bsd`'s, decision D5: the scheduler's members are atomic words written
//! under it), `[a]` atomics, `[I]` set before the object is reachable. A thread's own `[o]`
//! members are atomic words, and its credentials an embedded copy ([`Thread::ucred`]).
//!
//! ## Deviations
//! - `fork1`'s `LIST_INSERT_AFTER(curpr, pr, ps_pglist)` becomes an insert at the head of the
//!   group's members (the pinned lists have no insert-after): the order in which `pgsignal`
//!   reaches a group's members differs; no answer userland reads depends on it.
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
//! - The hash tables of `hashinit` are `MArray<PinList<_>>`s made by [`procinit`]; `uihashtbl`
//!   is a static's, under the `uidinfo` reader-writer lock whose witness [`uid_find`] returns.
//! - `struct process *ps_pptr`, `struct pgrp *ps_pgrp`, `struct session *pg_session` are counted
//!   handles in kernel-lock cells (a parent outlives no reference to it; a group lives while a
//!   member holds it, a session while a group does: `s_count` is that count); `s_leader` is the
//!   leader's address, compared, never followed.
//! - A thread's process link `p_p` is a counted handle (or process 0's `static`); a process's
//!   threads are owned by its tree under `ps_mtx` (`hal::owned::OwnedTree` keyed by the order
//!   they joined), so a thread's memory goes with its box, never behind a raw `pool_put`.
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
    AtomicI32, AtomicI64, AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize, Ordering,
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
use hal::sys::types::{Pid, Uid};
use hal::thread::MachineSlot;

use crate::assert::kpanic;
use crate::cred::{CredRef, Ucred};
use crate::resource::{LimitCache, PlimitRef, Rusage, TimespecWord, Tusage};
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
    /// The members of `struct process` of `bsd`'s subsystems (`ps_klist`, `ps_sigacts`,
    /// `ps_fd`, `ps_textvp`, the unveil table, the pins, the timeouts, the interval timers).
    type ProcessG: Send + Sync;
    /// The members of `struct proc` of `bsd`'s subsystems (`p_fd`, `p_kq`, `p_sleep_to`,
    /// `p_sigstk`, `p_sigval`).
    type ThreadG: Send + Sync;
    /// The members of `struct pgrp` of `bsd`'s (`pg_sigiolst`).
    type PgrpG: Send + Sync;
    /// The members of `struct session` of `bsd`'s (`s_ttyvp`, `s_ttyp`, `s_verauthto`).
    type SessionG: Send + Sync;
    /// `TAILQ_ENTRY(proc) p_runq`: the run and sleep queues' link, `bsd`'s until N4.5.
    type RunqEntry: Send + Sync;
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

    /// The lists, hashes and pools of this glue's processes (one `static` of `bsd`'s).
    fn tables() -> &'static ProcTables<Self>;
    /// `pgsignal(pg, sig, checkctty)`.
    fn pgsignal(pg: &Pgrp<Self>, sig: i32, checkctty: bool);
    /// `prsignal(pr, sig)`.
    fn prsignal(pr: &Process<Self>, sig: i32);
    /// `pgdelete`'s calls out: `sigio_freelist(&pgrp->pg_sigiolst)` and, when the session's
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
        type ThreadOwned = ThreadOwnedLink;
        type PgrpHash = PgrpHashLink;
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
    /// `proc_pool`, `procpl`.
    pub procpl: BoxPool<Thread<G>>,
    /// `process_pool`, `processpl` (recorded: a lent `&Process` can be counted again).
    pub processpl: TypedPool<Process<G>>,
    /// `pgrp_pool`, `pgrppl` (recorded).
    pub pgrppl: TypedPool<Pgrp<G>>,
    /// `session_pool`, `sessionpl` (recorded).
    pub sessionpl: TypedPool<Session<G>>,
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
            procpl: BoxPool::new(),
            processpl: TypedPool::new(),
            pgrppl: TypedPool::new(),
            sessionpl: TypedPool::new(),
        }
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
/// `uihashtbl` (`[U]`), made by the first [`procinit`].
static UIHASH: OnceStatic<MArray<PinList<UidinfoHash>>> = OnceStatic::new();

/// `procinit()`: the hash tables (`maxthread / 4` tid chains, `maxprocess / 4` pid and pgrp
/// chains, `maxprocess / 16` uid chains) and the pools `procpl`, `processpl`, `zombiepl`,
/// `pgrppl`, `sessionpl` (`ucredpl` is `kernel::cred`'s, made at its first use). Once per
/// glue at boot; the uid table once per kernel.
pub fn procinit<G: ProcGlue>(maxthread: i32, maxprocess: i32) {
    let t = G::tables();
    let made = t.tidhash.set(hashinit(maxthread / 4)).is_ok()
        && t.pidhash.set(hashinit(maxprocess / 4)).is_ok()
        && t.pgrphash.set(hashinit(maxprocess / 4)).is_ok();
    if !made {
        kpanic(format_args!("procinit: called twice"));
    }
    if !UIHASH.is_set() {
        let _ = UIHASH.set(hashinit(maxprocess / 16));
    }
    let pools = t.procpl.init(IPL_NONE, PR_WAITOK, "procpl").is_ok()
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
    /// The members of `bsd`'s subsystems (`pg_sigiolst`).
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
    /// The boxes of the process's threads (not process 0's `static` one), by joining order.
    threads: OwnedTree<G::ThreadOwned>,
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
            threads: OwnedTree::new(),
            next_seq: 0,
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
    /// `ps_mainproc`: the original thread's address.
    pub ps_mainproc: hal::link::PtrWord<Thread<G>>,
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
            ps_mainproc: hal::link::PtrWord::new(),
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
            g,
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
        match &self.ps_mtx.lock().cred {
            Some(c) => c.clone(),
            None => kpanic(format_args!("ps_ucred: no credentials")),
        }
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
    /// `uvm_exit` took it.
    pub fn vmspace(&self) -> Option<VmspaceRef> {
        self.ps_mtx.lock().vmspace.clone()
    }

    /// `pr->ps_vmspace = vm`: the previous handle.
    pub fn set_vmspace(&self, vm: Option<VmspaceRef>) -> Option<VmspaceRef> {
        let addr = vm.as_ref().map_or(0, |v| PoolArc::as_ptr(v).addr());
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
    /// the old parent's reference.
    pub fn set_pptr(&self, w: &mut KernelWitness, parent: ProcessRef<G>) -> Option<ProcessRef<G>> {
        self.ps_ppid.set(parent.ps_pid);
        self.tree.get_mut(w).pptr.replace(parent)
    }

    /// `ps_pgrp`, for a holder of the kernel lock.
    pub fn pgrp<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a PgrpRef<G>> {
        self.tree.get(w).pgrp.as_ref()
    }

    /// `pr->ps_pgrp = pgrp` (under `ps_mtx` too, as the C): the old group's reference.
    pub fn set_pgrp(&self, w: &mut KernelWitness, pgrp: Option<PgrpRef<G>>) -> Option<PgrpRef<G>> {
        let _m = self.ps_mtx.lock();
        core::mem::replace(&mut self.tree.get_mut(w).pgrp, pgrp)
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

    /// The thread's box joins the process: last in `ps_threads`, kept by the process. The
    /// thread's address, which `bsd`'s lists still take (its scheduler's queues).
    pub fn insert_thread(&self, t: PoolBox<Thread<G>>) -> *const Thread<G> {
        let mut g = self.ps_mtx.lock();
        let seq = g.next_seq;
        g.next_seq += 1;
        t.p_seq.set(seq);
        self.ps_threads.insert_tail(&mut g, t.as_pin());
        let at = PoolBox::as_ptr(&t);
        if g.threads.insert(t).is_some() {
            kpanic(format_args!("insert_thread: a thread joined twice"));
        }
        at
    }

    /// Process 0's `static` thread joins it (`main`).
    pub fn insert_static_thread(&self, t: &'static Thread<G>) {
        let mut g = self.ps_mtx.lock();
        self.ps_threads.insert_tail(&mut g, pin_static(t));
    }

    /// `TAILQ_REMOVE(&pr->ps_threads, p, p_thr_link)` of the thread `tid` and its box given
    /// back (`None` when no box of the process has that tid); the caller unlinks the thread
    /// from `allproc` and its tid chain with the box in hand, then drops it (`proc_free`).
    pub fn remove_thread(&self, tid: Pid) -> Option<PoolBox<Thread<G>>> {
        let mut g = self.ps_mtx.lock();
        let seq = g.threads.iter().find(|t| t.p_tid.get() == tid)?.p_seq.get();
        let b = g.threads.take_by(|e| seq.cmp(&e.p_seq.get()))?;
        self.ps_threads.remove(&mut g, &b);
        Some(b)
    }

    /// `TAILQ_REMOVE` of process 0's `static` thread.
    pub fn remove_static_thread(&self, t: &'static Thread<G>) {
        let mut g = self.ps_mtx.lock();
        self.ps_threads.remove(&mut g, t);
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
        Some(p) => StaticOrPooled::Pooled(p),
        None => kpanic(format_args!("process_new: process_pool is empty")),
    }
}

/// `struct proc`: a thread. `#[repr(C)]` with its [`MachineSlot`] first (decision P1).
#[repr(C)]
pub struct Thread<G: ProcGlue> {
    /// `p_stat`, `p_cpu`, `p_addr`, `p_md`: what the machine reaches by offset.
    pub machine: MachineSlot,
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
    /// \[S\] `p_runq`: the run and sleep queues' link (`bsd`'s until N4.5).
    pub p_runq: G::RunqEntry,
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
    /// \[a\] `p_siglist`.
    pub p_siglist: AtomicU32,
    /// \[o\] `p_sigmask`.
    pub p_sigmask: U32Word,
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
    /// The members of `bsd`'s subsystems.
    pub g: G::ThreadG,
}

impl<G: ProcGlue> Thread<G> {
    /// Process 0's `static` thread (`proc0`), of `pr` (`process0`).
    pub const fn new_static(pr: &'static Process<G>, runq: G::RunqEntry, g: G::ThreadG) -> Self {
        Self::make(
            StaticOrPooled::Static(pr),
            Guarded::new_static_mutexed(&pr.ps_mtx, None),
            Guarded::new_static_mutexed(&pr.ps_mtx, None),
            runq,
            g,
        )
    }

    /// The members, keyed to `pr`'s `ps_mtx` by the caller.
    const fn make(
        p_p: ProcessRef<G>,
        cred_ref: Guarded<Option<CredRef>>,
        vmspace: Guarded<Option<VmspaceRef>>,
        runq: G::RunqEntry,
        g: G::ThreadG,
    ) -> Self {
        Self {
            machine: MachineSlot::new(),
            p_list: PinEntry::new(),
            p_hash: PinEntry::new(),
            p_thr_link: PinEntry::new(),
            p_owned: OwnedEntry::new(),
            p_seq: U64Word::new(0),
            p_runq: runq,
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
            p_siglist: AtomicU32::new(0),
            p_sigmask: U32Word::new(0),
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

    /// `p_stat`.
    #[inline]
    pub fn stat(&self) -> u8 {
        self.machine.stat.load(Ordering::Relaxed)
    }

    /// `p->p_stat = s`.
    #[inline]
    pub fn set_stat(&self, s: u8) {
        self.machine.stat.store(s, Ordering::Relaxed);
    }

    /// `p_cpu`: the `cpu_info` word of the CPU the thread runs or last ran on.
    #[inline]
    pub fn cpu_word(&self) -> usize {
        self.machine.cpu.load(Ordering::Relaxed)
    }

    /// `p->p_cpu = ci` (`fork1`, `sched_choosecpu`).
    #[inline]
    pub fn set_cpu_word(&self, ci: usize) {
        self.machine.cpu.store(ci, Ordering::Relaxed);
    }

    /// `p_addr`: the u-area's address, 0 before `cpu_fork`.
    #[inline]
    pub fn addr_word(&self) -> usize {
        self.machine.addr.load(Ordering::Relaxed)
    }

    /// `p->p_addr = uaddr`.
    #[inline]
    pub fn set_addr_word(&self, uaddr: usize) {
        self.machine.addr.store(uaddr, Ordering::Relaxed);
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
    pub fn vmspace(&self) -> Option<VmspaceRef> {
        let g = self.p_p.ps_mtx.lock();
        self.vmspace.get(&g).clone()
    }

    /// `p->p_vmspace = vm` (`fork1`'s copy of the process's, exec's, the reaper's `NULL`): the
    /// old handle.
    pub fn set_vmspace(&self, vm: Option<VmspaceRef>) -> Option<VmspaceRef> {
        let addr = vm.as_ref().map_or(0, |v| PoolArc::as_ptr(v).addr());
        let mut g = self.p_p.ps_mtx.lock();
        let old = core::mem::replace(self.vmspace.get_mut(&mut g), vm);
        self.vmspace_word.store(addr, Ordering::Release);
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

/// `thread_new`'s `pool_get(&proc_pool, PR_WAITOK)`: a thread of `pr`, its box (the caller
/// gives it to the process, [`Process::insert_thread`]).
pub fn thread_alloc<G: ProcGlue>(
    pr: &ProcessRef<G>,
    runq: G::RunqEntry,
    g: G::ThreadG,
) -> PoolBox<Thread<G>> {
    let make = || {
        Thread::make(
            pr.clone(),
            Guarded::new_mutexed(&pr.ps_mtx, None),
            Guarded::new_mutexed(&pr.ps_mtx, None),
            runq,
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

/// `uid_find(uid)` ... `uid_release(uip)`: `f` runs on the entry of `uid` (made on first need)
/// with `uidinfolk` held for writing, as between the C's pair.
pub fn with_uidinfo<R>(uid: Uid, f: impl FnOnce(&Uidinfo) -> R) -> R {
    let Some(tbl) = UIHASH.get() else {
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
pub fn chgproccnt(uid: Uid, diff: i64) -> i64 {
    let count = with_uidinfo(uid, |uip| {
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
                s.s_leader.set(pr.addr());
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
        sp.s_leader.set(0);
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
            G::prsignal(pr, SIGHUP);
            G::prsignal(pr, SIGCONT);
        }
    }
}

// proc_printit, db_kill_cmd, db_stop_cmd, db_show_all_procs: the real ddb. pgrpdump: DEBUG.
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::cell::RefCell;
    use std::vec::Vec;

    use super::*;
    use crate::cred::{CredRef, Ucred};
    use crate::testutil;

    /// The kernel's own glue for its tests: no member of `bsd`'s, the signals recorded.
    pub struct TestGlue;

    crate::proc_glue_links!(TestGlue);

    std::thread_local! {
        static TABLES: &'static ProcTables<TestGlue> = Box::leak(Box::new(ProcTables::new()));
        static SIGNALS: RefCell<Vec<(Pid, i32)>> = const { RefCell::new(Vec::new()) };
    }

    impl ProcGlue for TestGlue {
        type ProcessG = ();
        type ThreadG = ();
        type PgrpG = ();
        type SessionG = ();
        type RunqEntry = ();
        crate::proc_glue_link_types!();

        fn tables() -> &'static ProcTables<Self> {
            TABLES.with(|t| *t)
        }

        fn pgsignal(_pg: &Pgrp<Self>, _sig: i32, _checkctty: bool) {}

        fn prsignal(pr: &Process<Self>, sig: i32) {
            // A signal path that walks the lists takes its own reader: allowed here.
            let _r = klock::reader();
            SIGNALS.with(|s| s.borrow_mut().push((pr.ps_pid, sig)));
        }

        fn pgrp_destroy(_pg: &Pgrp<Self>) {}
        fn session_hangup(_sess: &Session<Self>) {}
        fn session_placed(_sess: &Session<Self>) {}
        fn session_destroy(_sess: &Session<Self>) {}
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
        let t = thread_alloc::<TestGlue>(&pr, (), ());
        t.p_tid.set(5000);
        {
            let mut w = klock::witness();
            let tb = TestGlue::tables();
            tb.allproc.insert_head(&mut w, t.as_pin());
            tb.tidhash(5000).insert_head(&mut w, t.as_pin());
        }
        pr.ps_threadcnt.set(1);
        let at = pr.insert_thread(t);
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
        let b = pr.remove_thread(5000).unwrap();
        {
            let mut w = klock::witness();
            let tb = TestGlue::tables();
            tb.allproc.remove(&mut w, &b);
            tb.tidhash(5000).remove(&mut w, &b);
        }
        drop(b);
        assert!(pr.remove_thread(5000).is_none());
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
        let t = thread_alloc::<TestGlue>(&pr, (), ());
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
        let t = thread_alloc::<TestGlue>(&pr, (), ());
        assert_eq!(t.p_wmesg.get(), None);
        t.p_wmesg.set(Some("nanoslp"));
        assert_eq!(t.p_wmesg.get(), Some("nanoslp"));
        t.set_name(b"worker");
        assert_eq!(&*t.name(), b"worker");
        pr.set_comm(b"a very long command name that does not fit");
        assert_eq!(pr.comm().len(), _MAXCOMLEN - 1);
        assert_eq!(t.stat(), 0);
        t.set_stat(SONPROC);
        assert_eq!(t.machine.stat.load(Ordering::Relaxed), SONPROC);
        assert_eq!(core::mem::offset_of!(Thread<TestGlue>, machine), 0);
        drop(t);
        unlink(&pr);
    }

    #[test]
    fn groups_sessions_and_job_control() {
        setup();
        SIGNALS.with(|s| s.borrow_mut().clear());
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
        // The group is orphaned with a stopped member: hung up and continued.
        b.ps_flags.fetch_or(PS_STOPPED, Ordering::Relaxed);
        {
            let r = klock::reader();
            fixjobc(&r, &b, &g2, false);
        }
        assert_eq!(g2.pg_jobc.get(), 0);
        let sigs = SIGNALS.with(|s| s.borrow().clone());
        assert_eq!(sigs, [(41, SIGHUP), (41, SIGCONT)]);
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
        assert_eq!(chgproccnt(64_001, 1), 1);
        assert_eq!(chgproccnt(64_001, 2), 3);
        assert_eq!(chgproccnt(64_001, -3), 0);
        with_uidinfo(64_001, |u| {
            assert_eq!(u.ui_uid, 64_001);
            u.ui_lockcnt.set(u.ui_lockcnt.get() + 1);
        });
        assert_eq!(with_uidinfo(64_001, |u| u.ui_lockcnt.get()), 1);
    }

    #[test]
    #[should_panic(expected = "procs < 0")]
    fn a_negative_count_is_refused() {
        setup();
        chgproccnt(64_002, -1);
    }
}
/* </TESTS> */
