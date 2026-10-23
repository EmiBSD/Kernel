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
/* </LICENSES> */

/* <CODE> */
//! `<sys/proc.h>`: description of a process.
//!
//! These structures contain the information needed to manage a thread of control, known in
//! UN*X as a process; it has references to substructures containing descriptions of things
//! that the process uses, but may share with related processes.
//!
//! `struct process` is the higher level process containing information shared by all threads
//! in a process, while `struct proc` contains the run-time information needed by threads.
//!
//! Upstream: sys/sys/proc.h @ 3ce1f3f79392
//!
//! Status: an adapter (N4.2). The structures are the kernel crate's (`kernel::proc`:
//! `Thread`, `Process`, `Pgrp`, `Session`, `Uidinfo`, the `S*`, `P_*`, `PS_*`, `FORK_*`,
//! `EXIT_*`, `SINGLE_*` constants) instantiated with `bsd`'s glue (`kern/proc_glue.rs`,
//! whose members a thread or process dereferences to); this header keeps their LZ names
//! (`Proc`, `Process`, ...), `struct pinsyscall`, `struct cond`, `struct cpuset`,
//! `tu_enter`/`tu_leave`, `refreshcreds`, and [`ProcExt`]: the three reads of a thread's
//! words that `bsd` still turns into references (`vmspace()`, `cpu()`, `pcb()`), each LZ's
//! `unsafe` deref, until N4.5's `current()` retires them with `curproc()`'s.
//!
//! ## Deviations
//! - `SESSHOLD`/`SESSRELE` are the kernel's counted `SessionRef`s (a clone, a drop); a
//!   session goes with its last reference.
//! - A thread's `p_wchan` is the sleep address as a number (`kernel::proc`'s word), compared,
//!   never followed.

use core::ffi::c_void;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::AtomicU32;

use kernel::vm::map::Vmspace;

use crate::kassert;
use crate::kern::proc_glue::BsdProcGlue;
use crate::machine::cpu::{CpuInfo, MAXCPUS};
use crate::machine::proc::Pcb;
use crate::sys::time::Timeval;
use crate::sys::user::User;

pub use crate::kern::proc_glue::{
    BsdPgrpG, BsdProcessG, BsdSessionG, BsdThreadG, PgrpHashLink as PgrpHash,
    ProcessHashLink as ProcessHash, ProcessListLink as ProcessList,
    ProcessOrphanLink as ProcessOrphan, ProcessPglistLink as ProcessPglist,
    ProcessSiblingLink as ProcessSibling, ThreadHashLink as ProcHash, ThreadListLink as ProcList,
    ThreadThrLink as ProcThrLink,
};
pub use kernel::proc::{
    _MAXCOMLEN, BOGO_PC, EXIT_NORMAL, EXIT_THREAD, EXIT_THREAD_NOCHECK, FORK_FORK, FORK_IDLE,
    FORK_NOZOMBIE, FORK_PPWAIT, FORK_PTRACE, FORK_SHAREFILES, FORK_SHAREVM, FORK_SYSTEM,
    FORK_VFORK, NO_PID, P_ALRMPEND, P_BITS, P_CANTSLEEP, P_CPUPEG, P_INKTR, P_INSCHED, P_OWEUPC,
    P_PROFPEND, P_SIGSUSPEND, P_SINTR, P_SUSPSIG, P_SUSPSINGLE, P_SYSTEM, P_THREAD, P_TIMEOUT,
    P_TIMEOUTRAN, P_TRACESINGLE, P_WEXIT, PID_MAX, PS_BITS, PS_CHROOT, PS_CONTINUED, PS_CONTROLT,
    PS_COREDUMP, PS_EMBRYO, PS_EXEC, PS_EXECPLEDGE, PS_EXITING, PS_FLAGS_INHERITED_ON_FORK,
    PS_INEXEC, PS_ISPWAIT, PS_ITIMER, PS_NOBROADCASTKILL, PS_NOZOMBIE, PS_ORPHAN, PS_PLEDGE,
    PS_PPWAIT, PS_PROFIL, PS_SINGLEEXIT, PS_SINGLEUNWIND, PS_STOPPED, PS_STOPPING, PS_SUGID,
    PS_SUGIDEXEC, PS_SYSTEM, PS_TRACED, PS_TRAPPED, PS_WAITED, PS_WAITEVENT, PS_ZOMBIE,
    PSI_NOBTCFI, PSI_PROFILE, PSI_WXNEEDED, PsMtx, SDEAD, SIDL, SINGLE_DEEP, SINGLE_EXIT,
    SINGLE_MASK, SINGLE_SUSPEND, SINGLE_UNWIND, SONPROC, SRUN, SSLEEP, SSTOP, SZOMB,
    THREAD_PID_OFFSET, TID_MASK, Uidinfo,
};
pub use kernel::resource::{TU_ITICKS, TU_STICKS, TU_TICKS_COUNT, TU_UTICKS, Tusage};

/// `struct p_inentry`: the map's own cache type (`kernel::vm::map::InentryCache`, which
/// `uvm_map_inentry` fills: `ie_start`, `ie_end`, `ie_serial`).
pub use kernel::vm::map::InentryCache as PInentry;

/// `struct proc`: a thread (`kernel::proc::Thread` of `bsd`'s glue).
pub type Proc = kernel::proc::Thread<BsdProcGlue>;
/// `struct process` (`kernel::proc::Process` of `bsd`'s glue).
pub type Process = kernel::proc::Process<BsdProcGlue>;
/// `struct pgrp`.
pub type Pgrp = kernel::proc::Pgrp<BsdProcGlue>;
/// `struct session`.
pub type Session = kernel::proc::Session<BsdProcGlue>;
/// A counted reference to a process.
pub type ProcessRef = kernel::proc::ProcessRef<BsdProcGlue>;
/// A counted reference to a process group.
pub type PgrpRef = kernel::proc::PgrpRef<BsdProcGlue>;
/// A counted reference to a session (`SESSHOLD`).
pub type SessionRef = kernel::proc::SessionRef<BsdProcGlue>;
/// The guard of a process's `ps_mtx`.
pub type PsGuard<'a> = hal::mutex::MutexedGuard<'a, PsMtx<BsdProcGlue>>;

/// What `bsd` reads of a thread through its words: LZ's accessors that turn an address the
/// thread holds into a reference, each one `unsafe` deref until N4.5's `current()`.
pub trait ProcExt {
    /// `p->p_vmspace`: the thread's copy of its process's address space.
    fn vmspace(&self) -> &'static Vmspace;
    /// `p->p_cpu`: the `cpu_info` of the CPU the thread runs or last ran on (its number in
    /// the slot's word, `kernel::sched::thread_cpu`).
    fn cpu(&self) -> Option<&'static CpuInfo>;
    /// `p->p_addr->u_pcb`: the thread's process control block.
    fn pcb(&self) -> &Pcb;
    /// `p->p_addr` as the C's pointer.
    fn uarea(&self) -> *const User;
    /// `p->p_wchan` as the C's pointer (compared, never followed).
    fn wchan(&self) -> *const c_void;
    /// `p->p_wchan = ident`.
    fn set_wchan(&self, ident: *const c_void);
    /// Records the thread's own `&'static` (`p_self`), once, where it is placed:
    /// `thread_new` for a pool item, `main` for `proc0`, a test for a leaked one.
    fn place(&'static self);
    /// The thread as the `&'static` the run, sleep and dead-thread queues hold (`p_self`);
    /// panics for a thread never placed.
    fn as_static(&self) -> &'static Proc;
}

impl ProcExt for Proc {
    fn vmspace(&self) -> &'static Vmspace {
        let vm = ptr::with_exposed_provenance::<Vmspace>(self.vmspace_word());
        kassert!(!vm.is_null());
        // SAFETY: this is called on a live thread (`curproc` in a system call, a fault or a
        // selftest), whose word is the address of the vmspace its handle holds
        // (`Thread::set_vmspace` stores the handle and its exposed address together, under
        // the process's `ps_mtx`): `fork1` and `main` give it its process's, `uvmspace_exec`
        // sets the exec'ing thread's before it replaces the process's (its other threads are
        // dead by then); so the vmspace lives while the thread holds the handle, until the
        // reaper's `set_vmspace(None)`, which runs on a dead thread nothing calls this on.
        // The selftests that lend a thread another vmspace keep that handle until they set
        // the copy back. `'static` is the handle's lifetime, the thread's to keep.
        unsafe { &*vm }
    }

    fn cpu(&self) -> Option<&'static CpuInfo> {
        kernel::sched::thread_cpu(self).and_then(crate::kern::kern_sched::cpu_info_by_number)
    }

    fn pcb(&self) -> &Pcb {
        let u = ptr::with_exposed_provenance::<User>(self.addr_word());
        kassert!(!u.is_null());
        // SAFETY: `p_addr` is the exposed address of the u-area the thread's slot owns
        // (`hal::thread`'s E4: a `UArea` installed by `thread_new`, `hal`'s static one for
        // `proc0`), zeroed memory whose bottom is a `struct user` (`u_pcb` first); it is given
        // back only by the reaper's `uvm_uarea_free` (`take_uarea`), which `hal` refuses while
        // the thread's context lives (E6: only once it switched away for good, dead); the
        // borrow is the thread's, so the thread is alive and its u-area is too.
        unsafe { &(*u).u_pcb }
    }

    fn uarea(&self) -> *const User {
        ptr::with_exposed_provenance(self.addr_word())
    }

    fn wchan(&self) -> *const c_void {
        ptr::without_provenance(self.p_wchan.get())
    }

    fn set_wchan(&self, ident: *const c_void) {
        self.p_wchan.set(ident.addr());
    }

    fn place(&'static self) {
        self.p_self.set(Some(self));
    }

    fn as_static(&self) -> &'static Proc {
        match self.p_self.get() {
            Some(s) if ptr::eq(s, self) => s,
            _ => {
                crate::kern::subr_prf::panic(format_args!("thread {} not placed", self.p_tid.get()))
            }
        }
    }
}

/// The glue's `thread_placed` (`kernel::fork`'s `fork1` and `thread_fork`, N4.6a): records
/// the new thread's own `&'static` (`ProcExt::place`), which `bsd`'s run, sleep and signal
/// adapters lend the kernel (`as_static`, until N4.8 retires them with `curproc()`'s).
pub fn place_thread(p: Pin<&Proc>) {
    let at = ptr::from_ref::<Proc>(p.get_ref());
    // SAFETY: `kernel::fork` calls this once per thread, right after linking it, with the pin
    // of a `procpl` box it then gives to the thread's process (its slot for the main thread,
    // its tree for another), which keeps it until `exit1` hands a secondary thread's box to
    // the thread itself and the reaper or `process_zap` frees it (`proc_free`), after the
    // thread has left every list, hash and queue and no other thread names it: the pool item
    // stays at this address, initialised, until then (a pinned item is not moved), and nothing
    // mutates it but through its atomics and cells (shared references only), so a shared
    // reference to it is valid for the thread's life. `'static` is LZ's convention for a
    // thread reference (the C's `struct proc *` from `pool_get` to `pool_put`), the one
    // `curproc()` and the run queues rest on (this is LZ's `thread_join` deref, moved here
    // from `kern_fork.rs` with the creation).
    let p: &'static Proc = unsafe { &*at };
    p.place();
}

/// `P_HASSIBLING(p)`.
pub fn p_hassibling(p: &Proc) -> bool {
    p.has_sibling()
}

/// `struct cond`.
pub struct Cond {
    /// \[a\] `c_wait`: initialized and waiting.
    pub c_wait: AtomicU32,
}

impl Cond {
    /// `COND_INITIALIZER()`.
    pub const fn new() -> Self {
        Self {
            c_wait: AtomicU32::new(1),
        }
    }
}

impl Default for Cond {
    fn default() -> Self {
        Self::new()
    }
}

/// `cond_signal_handler` (`kern_synch.c`): the timeout/task handler that signals a `cond`,
/// whose address is its argument.
pub fn cond_signal_handler(arg: *mut c_void) {
    // SAFETY: `arg` is the `cond` its waiter armed the handler with (`cond_signal`, the
    // barrier tasks and timeouts of `smr_barrier` and `ifq_barrier`), which the waiter keeps
    // on its frame until `cond_wait` sees `c_wait` cleared here; nothing reads it after the
    // store but `wakeup_one`, which only compares the address.
    let c = unsafe { &*arg.cast::<Cond>() };

    c.c_wait.store(0, core::sync::atomic::Ordering::Relaxed);

    crate::kern::kern_synch::wakeup_one(ptr::from_ref(c));
}

/*
 * functions to handle sets of cpus.
 *
 * For now we keep the cpus in ints so that we can use the generic
 * atomic ops.
 */

/// `CPUSET_ASIZE(x)`.
pub const fn cpuset_asize(x: u32) -> usize {
    ((x as usize) - 1) / 32 + 1
}

/// `CPUSET_SSIZE`.
pub const CPUSET_SSIZE: usize = cpuset_asize(MAXCPUS);

/// `struct cpuset`.
pub struct Cpuset {
    /// `cs_set`.
    pub cs_set: [AtomicU32; CPUSET_SSIZE],
}

impl Cpuset {
    /// An empty set.
    pub const fn new() -> Self {
        Self {
            cs_set: [const { AtomicU32::new(0) }; CPUSET_SSIZE],
        }
    }
}

impl Default for Cpuset {
    fn default() -> Self {
        Self::new()
    }
}

/// `tu_enter(tu)`.
#[inline]
pub fn tu_enter(tu: &Tusage) -> u32 {
    tu.enter()
}

/// `tu_leave(tu, gen)`.
#[inline]
pub fn tu_leave(tu: &Tusage, generation: u32) {
    tu.leave(generation)
}

/// `refreshcreds(p)`: refresh the thread's cache of the process's creds. This is an unlocked
/// access to `ps_ucred`, but the result is benign.
#[inline]
pub fn refreshcreds(p: &Proc) {
    if p.cred_stale() {
        crate::kern::kern_prot::dorefreshcreds(p.process(), p);
    }
}

/// A `Timeval`-typed helper the resource code shares: the zero interval.
pub const ZERO_TIMEVAL: Timeval = Timeval::new(0, 0);
const _: () = assert!(
    core::mem::offset_of!(Proc, machine) == 0,
    "the machine's slot is the thread's first member: ci_curproc names both (hal::thread)"
);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpusets_start_empty() {
        assert_eq!(cpuset_asize(1), 1);
        assert_eq!(cpuset_asize(33), 2);
        let c = Cpuset::new();
        assert!(
            c.cs_set
                .iter()
                .all(|w| w.load(core::sync::atomic::Ordering::Relaxed) == 0)
        );
    }
}
/* </TESTS> */
