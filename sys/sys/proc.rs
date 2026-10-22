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
//! - `struct pinsyscall`'s members are atomic words (`pn_pins` a `hal::link::PtrWord`), so a
//!   process's pins can be members of the kernel's pooled `Process`.
//! - `SESSHOLD`/`SESSRELE` are the kernel's counted `SessionRef`s (a clone, a drop); a
//!   session goes with its last reference.
//! - A thread's `p_wchan` is the sleep address as a number (`kernel::proc`'s word), compared,
//!   never followed.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::AtomicU32;

use hal::link::PtrWord;
use hal::malloc::MArray;
use kernel::proc::{I32Word, UsizeWord};
use kernel::vm::map::Vmspace;

use crate::kassert;
use crate::kern::proc_glue::BsdProcGlue;
use crate::machine::cpu::{CpuInfo, MAXCPUS};
use crate::machine::proc::Pcb;
use crate::sys::time::Timeval;
use crate::sys::user::User;

pub use crate::kern::proc_glue::{
    BsdPgrpG, BsdProcessG, BsdSessionG, BsdThreadG, PgrpHashLink as PgrpHash, ProcStore,
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

hal::list_adapter!(
    /// `TAILQ_ENTRY(proc) p_runq`: the run, sleep and dead-thread queues.
    pub ProcRunq: Proc, p_runq_legacy
);

/// `struct pinsyscall`: the system call pin table of a text region.
#[derive(Debug, Default)]
pub struct Pinsyscall {
    /// `pn_start`.
    pub pn_start: UsizeWord,
    /// `pn_end`.
    pub pn_end: UsizeWord,
    /// `pn_pins`: array of offsets indexed by syscall#.
    pub pn_pins: PtrWord<u32>,
    /// `pn_npins`: number of entries in table.
    pub pn_npins: I32Word,
}

impl Pinsyscall {
    /// No table.
    pub const fn new() -> Self {
        Self {
            pn_start: UsizeWord::new(0),
            pn_end: UsizeWord::new(0),
            pn_pins: PtrWord::new(),
            pn_npins: I32Word::new(0),
        }
    }

    /// `pinsyscalls(2)`'s table for `[base, end)` (the kernel's `sys_pinsyscalls` built it,
    /// `mallocarray(npins, sizeof(u_int), M_PINSYSCALL)`): the block is the process's from now
    /// on, read by `pin_check` at every system call and freed by `exit`/`exec`'s `pin_free`
    /// (`free(pn_pins, M_PINSYSCALL, npins * sizeof(u_int))`), as the C's. The fields are
    /// stored in the C's order.
    pub fn install(&self, base: usize, end: usize, pins: MArray<u32>) {
        let (table, npins) = MArray::into_raw(pins);
        self.pn_start.set(base);
        self.pn_end.set(end);
        self.pn_pins.set(table.as_ptr());
        // sys_pinsyscalls refuses a table of more than SYS_MAXSYSCALL entries (E2BIG).
        let npins = i32::try_from(npins).ok();
        kassert!(npins.is_some());
        self.pn_npins.set(npins.unwrap_or(0));
    }
}

/// What `bsd` reads of a thread through its words: LZ's accessors that turn an address the
/// thread holds into a reference, each one `unsafe` deref until N4.5's `current()`.
pub trait ProcExt {
    /// `p->p_vmspace`: the thread's copy of its process's address space.
    fn vmspace(&self) -> &'static Vmspace;
    /// `p->p_cpu`.
    fn cpu(&self) -> Option<&'static CpuInfo>;
    /// `p->p_addr->u_pcb`: the thread's process control block.
    fn pcb(&self) -> &Pcb;
    /// `p->p_cpu` as the C's pointer (compared, never followed here).
    fn cpu_ptr(&self) -> *const CpuInfo;
    /// `p->p_cpu = ci`.
    fn set_cpu(&self, ci: *const CpuInfo);
    /// `p->p_addr` as the C's pointer.
    fn uarea(&self) -> *const User;
    /// `p->p_addr = uaddr`.
    fn set_uarea(&self, uaddr: *const User);
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
        // SAFETY: `p_cpu` is 0 or the exposed address of a static `cpu_info` (`set_cpu`'s
        // stores, the switch's `ci_self`), which lives for the whole run.
        unsafe { ptr::with_exposed_provenance::<CpuInfo>(self.cpu_word()).as_ref() }
    }

    fn pcb(&self) -> &Pcb {
        let u = ptr::with_exposed_provenance::<User>(self.addr_word());
        kassert!(!u.is_null());
        // SAFETY: `p_addr` is the exposed address of the thread's u-area (`set_uarea` in
        // `thread_new`, `proc0paddr` for `proc0`), allocated before the thread exists and
        // freed by the reaper's `uvm_uarea_free` after the thread is dead; the borrow is
        // the thread's.
        unsafe { &(*u).u_pcb }
    }

    fn cpu_ptr(&self) -> *const CpuInfo {
        ptr::with_exposed_provenance(self.cpu_word())
    }

    fn set_cpu(&self, ci: *const CpuInfo) {
        self.set_cpu_word(ci.expose_provenance());
    }

    fn uarea(&self) -> *const User {
        ptr::with_exposed_provenance(self.addr_word())
    }

    fn set_uarea(&self, uaddr: *const User) {
        self.set_addr_word(uaddr.expose_provenance());
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
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpusets_and_pins_start_empty() {
        assert_eq!(cpuset_asize(1), 1);
        assert_eq!(cpuset_asize(33), 2);
        let pin = Pinsyscall::new();
        assert!(pin.pn_pins.get().is_null());
        assert_eq!(pin.pn_npins.get(), 0);
        let c = Cpuset::new();
        assert!(
            c.cs_set
                .iter()
                .all(|w| w.load(core::sync::atomic::Ordering::Relaxed) == 0)
        );
    }
}
/* </TESTS> */
