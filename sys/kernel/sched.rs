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
 * Copyright (c) 1982, 1986, 1990, 1991, 1993
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
 *	@(#)kern_synch.c	8.6 (Berkeley) 1/21/94
 */

/*
 * Copyright (c) 2007, 2008 Artur Grabowski <art@openbsd.org>
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
 * Copyright (c) 1982, 1986, 1990, 1991, 1993
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
 *	@(#)kern_synch.c	8.6 (Berkeley) 1/21/94
 */

/*-
 * Copyright (c) 1999 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Ross Harvey.
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

/*-
 * Copyright (c) 1982, 1986, 1991, 1993
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
 *	@(#)kern_clock.c	8.5 (Berkeley) 1/21/94
 */
/*
 * Copyright (c) 1982, 1986, 1989, 1991, 1993
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
 *	@(#)kern_exit.c	8.7 (Berkeley) 2/12/94
 */

/*
 * Copyright (c) 1982, 1986, 1989, 1991, 1993
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
 *	@(#)kern_fork.c	8.6 (Berkeley) 4/8/94
 */
/* </LICENSES> */

/* <CODE> */
//! The scheduler: the sleep queues and `tsleep(9)`/`wakeup(9)` (`kern_synch.c`), the run
//! queues, the CPU choice, the idle loop and the dead threads (`kern_sched.c`), and the
//! 4.4BSD priorities, the load average and `mi_switch` (`sched_bsd.c`), over `hal::thread`'s
//! switch and scheduler lock.
//!
//! Upstream: sys/kern/kern_synch.c @ 3ce1f3f79392
//! Upstream: sys/kern/kern_sched.c @ 3ce1f3f79392
//! Upstream: sys/kern/sched_bsd.c @ 3ce1f3f79392
//! Upstream: sys/sys/sched.h @ 3ce1f3f79392
//! Upstream: sys/kern/kern_exit.c @ 3ce1f3f79392
//! Upstream: sys/kern/kern_fork.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_synch.rs@3c62ede68802
//! LZ: sys/kern/kern_sched.rs@3c62ede68802
//! LZ: sys/kern/sched_bsd.rs@3c62ede68802
//! LZ: sys/sys/sched.rs@3c62ede68802
//! LZ: sys/kern/kern_exit.rs@3c62ede68802
//! LZ: sys/kern/kern_fork.rs@3c62ede68802
//!
//! **The lock.** `SCHED_LOCK` (`hal::thread::SCHED_LOCK`, at `IPL_SCHED`) keeps the sleep
//! queues, the run queues and each CPU's idle-thread queue, all `hal::pinlist::PinList`s of
//! threads linked through their `p_runq`, and the `[S]` members of the threads, which are
//! relaxed atomic words written under it (read without it by `ps`, the clock and the cost
//! estimates, as in the C). Its witness is the hold `hal::thread::sched_lock` returns: every
//! function that changes a queue takes `&mut SchedHold`, one that reads them `&SchedHold`.
//! The hold is held across the switch: [`mi_switch`] takes it by value and gives it to
//! `hal::thread::switch_to`, whose frame keeps it dormant until this thread runs again; it is
//! dropped right after, before `assertwaitok`, `smr_idle`, the clock interrupts and the
//! kernel lock's reacquisition, in the C's order (amendment A of the brief). A new thread
//! releases the lock with the hold its trampoline adopted ([`trampoline_mi`]).
//!
//! **References and queues.** A reference to a thread found on a queue borrows the hold:
//! nothing can relink the thread while it lives, and the thread, linked, cannot be freed
//! (its entry's drop panics while linked). So a thread is moved between queues by the
//! queues themselves (`hal::pinlist`'s `move_where`, `move_first_to`, `remove_where`), after
//! its new place was decided from what a shared look at it showed: [`wakeup_n`] moves its
//! sleepers to a local queue, then each to a run queue; [`sched_chooseproc`] makes the
//! chosen thread's token ([`Next`]) while it is still linked, then takes it off. A thread
//! named by its owner instead (the running one, one found under the kernel lock) is inserted
//! and removed directly.
//!
//! **The dead.** A dying thread puts itself on its CPU's `deadproc` and switches to the idle
//! thread for good ([`sched_exit`]); the idle thread moves it to the reaper's queue
//! ([`exit2`]) once the switch away is over; both queues are kept by `DEAD_LOCK`
//! (`deadproc_mutex`), which the reaper sleeps on ([`reaper_next`]).
//!
//! What is still `bsd`'s reaches the scheduler through the process glue (`ProcGlue`'s
//! scheduler hooks: the signal checks of a sleep, the clock interrupts and the CPU time
//! accounting, the machine's idle cycle and inter-processor kicks, `smr(9)`), until each moves
//! (N4.5b-c, N4.8).
//!
//! ## Deviations
//! - The `cold || panicstr` early return of `tsleep_nsec` is the glue's (`cold`, `cold_yield`),
//!   as `hal::sleep`'s bridge had it.
//! - `endtsleep` finds its thread by tid under the kernel lock (the softclock thread runs a
//!   timeout without `TIMEOUT_MPSAFE` with it), where the C's timeout argument is the thread:
//!   a hash lookup on expiry, never on the sleep path.
//! - `sched_barrier`'s task carries the CPU's number and signals a per-CPU generation the
//!   caller waits on, where the C hands the task a pointer to the caller's stack state.
//! - The CPU sets are kept by CPU number (`hal::cpu`'s), not by `struct cpu_info`.
//! - dt(4) is not configured: the `TRACEPOINT`s are comments, as in LZ.
//! - The CPUs' `spc_deadproc` lists share `deadproc_mutex` (`DEAD_LOCK`) with the reaper's
//!   `deadproc` (the C's per-CPU list is the CPU's own, touched without a lock), so the
//!   queues are bound to one lock; and `exit2`'s accounting of a dead thread
//!   (`tuagg_add_process` under `ps_mtx`, the glue's `exit_account`) runs while
//!   `deadproc_mutex` is held, where the C takes `ps_mtx` before it: the lock order is
//!   `deadproc_mutex` -> `ps_mtx`, and nothing takes them the other way (`sched_exit` inserts
//!   after `exit1` left `ps_mtx`; the reaper holds neither when it takes `deadproc_mutex`).
//!
//! ## Redesign
//! - LZ's three files keep the run queues in each `struct cpu_info`'s `schedstate_percpu` and
//!   the sleep queues in a static table, as `hal::list::List`s of `&'static Proc` (N4.2's
//!   interim), under the legacy `SCHED_LOCK` mutex taken and released by free functions in
//!   different frames (`mi_switch`'s caller locks, `mi_switch` or the new thread's
//!   `proc_trampoline_mi` unlocks). Here the queues are `PinList`s bound to `SCHED_LOCK`,
//!   changed only through its hold, and the per-CPU state is a [`SchedCpu`] the scheduler
//!   allocates when a CPU joins ([`sched_init_cpu`]), reached by the CPU's number; the
//!   clock-interrupt handles, the CPU time and `spc_runtime` stay with the clock (`bsd`'s
//!   `schedstate_percpu` until N4.8).
//! - The idle thread of a CPU is the one element of the CPU's idle queue, so the scheduler
//!   reaches it pinned under the hold like any queued thread (the C's `spc_idleproc`
//!   pointer); its address is kept apart for the unlocked compares (`schedclock`, SMR).

use core::ffi::c_void;
use core::marker::PhantomData;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use hal::cpu::MAXCPUS;
use hal::intr::IPL_NONE;
use hal::mutex::{Mutexed, SharedMutex};
use hal::once::OnceStatic;
use hal::pinlist::PinList;
use hal::sleep::{INFSLP, PCATCH, PRIMASK, PWAIT};
use hal::sys::errno::Errno;
use hal::thread::{Next, SchedHold, sched_lock};
use hal::vm::uvmexp::UVMEXP;

use crate::assert::{kassert, kpanic};
use crate::proc::{
    P_CPUPEG, P_INSCHED, P_SINTR, P_SUSPSIG, P_TIMEOUT, P_TIMEOUTRAN, P_WEXIT, PS_ITIMER,
    PS_PROFIL, PS_STOPPING, ProcGlue, SINGLE_SUSPEND, SONPROC, SRUN, SSLEEP, SSTOP, Thread,
};
use crate::sig::{self, Sigctx};
use crate::timeout::{TIMEOUT_PROC, Timeout};

/// `TABLESIZE`: we're only looking at 7 bits of the address; everything is aligned to 4,
/// lots of things are aligned to greater powers of 2. Shift right by 8, i.e. drop the bottom
/// 256 worth.
pub const TABLESIZE: usize = 128;

/// `SCHED_NQS`: 32 run queues.
pub const SCHED_NQS: usize = 32;
/// `SCHED_PPQ`: priorities per queue.
pub const SCHED_PPQ: u32 = 128 / SCHED_NQS as u32;
/// `NICE_WEIGHT`: priorities per nice level.
pub const NICE_WEIGHT: u32 = 2;
/// `PRIO_MAX` (`<sys/resource.h>`): the largest nice value.
const PRIO_MAX: u32 = 20;
/// `PUSER` (`<sys/param.h>`): the first user priority.
pub const PUSER: u32 = 50;
/// `MAXPRI` (`<sys/param.h>`): the worst priority.
pub const MAXPRI: u32 = 127;
/// `NZERO` (`<sys/param.h>`): the default nice.
pub const NZERO: u32 = 20;
/// `PZERO` (`<sys/param.h>`): no sleep with a signal below it.
pub const PZERO: i32 = 22;

/// `SPCF_SEENRR`: process has seen `roundrobin()`.
pub const SPCF_SEENRR: i32 = 0x0001;
/// `SPCF_SHOULDYIELD`: process should yield the CPU.
pub const SPCF_SHOULDYIELD: i32 = 0x0002;
/// `SPCF_SWITCHCLEAR`.
pub const SPCF_SWITCHCLEAR: i32 = SPCF_SEENRR | SPCF_SHOULDYIELD;
/// `SPCF_SHOULDHALT`: CPU should be vacated.
pub const SPCF_SHOULDHALT: i32 = 0x0004;
/// `SPCF_HALTED`: CPU has been halted.
pub const SPCF_HALTED: i32 = 0x0008;
/// `SPCF_PROFCLOCK`: `profclock()` was started.
pub const SPCF_PROFCLOCK: i32 = 0x0010;
/// `SPCF_ITIMER`: `itimer_update()` was started.
pub const SPCF_ITIMER: i32 = 0x0020;

/// `CPUTYP_SMT`: SMT cpu.
pub const CPUTYP_SMT: i32 = 0x01;
/// `CPUTYP_P`: Performance core.
pub const CPUTYP_P: i32 = 0x02;
/// `CPUTYP_E`: Efficiency core.
pub const CPUTYP_E: i32 = 0x04;
/// `CPUTYP_L`: Lethargic, Low Power Efficiency core.
pub const CPUTYP_L: i32 = 0x08;

/// `FSHIFT` (`<sys/param.h>`): bits to the right of the fixed binary point.
pub const FSHIFT: u32 = 11;
/// `FSCALE`.
pub const FSCALE: u32 = 1 << FSHIFT;

/// `ESTCPULIM(e)`.
pub const fn estcpulim(e: u32) -> u32 {
    let lim = NICE_WEIGHT * PRIO_MAX - SCHED_PPQ;
    if e < lim { e } else { lim }
}

/// `CPUSET_ASIZE(MAXCPUS)`: the words of a CPU set.
const CPUSET_WORDS: usize = MAXCPUS.div_ceil(32);

/// `struct cpuset`, by CPU number.
pub struct Cpuset([AtomicU32; CPUSET_WORDS]);

impl Cpuset {
    /// An empty set.
    pub const fn new() -> Self {
        Self([const { AtomicU32::new(0) }; CPUSET_WORDS])
    }

    /// `cpuset_add`.
    pub fn add(&self, n: usize) {
        self.0[n / 32].fetch_or(1 << (n % 32), Ordering::Relaxed);
    }

    /// `cpuset_del`.
    pub fn del(&self, n: usize) {
        self.0[n / 32].fetch_and(!(1 << (n % 32)), Ordering::Relaxed);
    }

    /// `cpuset_isset`.
    pub fn isset(&self, n: usize) -> bool {
        self.0[n / 32].load(Ordering::Relaxed) & (1 << (n % 32)) != 0
    }

    /// `cpuset_copy`.
    pub fn copy(&self) -> Cpuset {
        let to = Cpuset::new();
        for (t, f) in to.0.iter().zip(self.0.iter()) {
            t.store(f.load(Ordering::Relaxed), Ordering::Relaxed);
        }
        to
    }

    /// `cpuset_first`: the lowest numbered CPU of the set.
    pub fn first(&self) -> Option<usize> {
        self.0.iter().enumerate().find_map(|(i, w)| {
            let bits = w.load(Ordering::Relaxed);
            (bits != 0).then(|| i * 32 + bits.trailing_zeros() as usize)
        })
    }

    /// `cpuset_intersection(a, b)`: `a & b`.
    pub fn intersection(a: &Cpuset, b: &Cpuset) -> Cpuset {
        let to = Cpuset::new();
        for i in 0..CPUSET_WORDS {
            to.0[i].store(
                a.0[i].load(Ordering::Relaxed) & b.0[i].load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
        }
        to
    }

    /// `cpuset_complement(a, b)`: `b & ~a`.
    pub fn complement(a: &Cpuset, b: &Cpuset) -> Cpuset {
        let to = Cpuset::new();
        for i in 0..CPUSET_WORDS {
            to.0[i].store(
                b.0[i].load(Ordering::Relaxed) & !a.0[i].load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
        }
        to
    }

    /// `cpuset_cardinality`.
    pub fn cardinality(&self) -> u32 {
        self.0
            .iter()
            .map(|w| w.load(Ordering::Relaxed).count_ones())
            .sum()
    }
}

impl Default for Cpuset {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct schedstate_percpu`'s scheduler members: one CPU's run queues and state.
pub struct SchedCpu<G: ProcGlue> {
    /// The CPU's number (`hal::cpu`).
    number: usize,
    /// \[S\] `spc_qs`: the run queues, one per `SCHED_PPQ` priorities.
    qs: [PinList<G::ThreadRunq>; SCHED_NQS],
    /// \[S\] The CPU's idle thread (`spc_idleproc`), alone on it from `set_idleproc` on.
    idleq: PinList<G::ThreadRunq>,
    /// `spc_idleproc` as an address, for the unlocked compares (0 before it is set).
    idle_addr: AtomicUsize,
    /// \[D\] `spc_deadproc`: the dead threads of this CPU waiting for its idle thread.
    deadproc: PinList<G::ThreadRunq>,
    /// `spc_whichqs` (volatile): the queues that hold threads, a bit each.
    whichqs: AtomicU32,
    /// `spc_nrun`: procs on the run queues, changed under the hold.
    nrun: AtomicU32,
    /// `spc_curpriority` (volatile): usrpri of curproc.
    curpriority: AtomicU8,
    /// `spc_schedflags` (volatile): `SPCF_*`.
    schedflags: AtomicI32,
}

impl<G: ProcGlue> SchedCpu<G> {
    /// CPU `n`'s state before it runs: empty queues, no idle thread.
    fn new(n: usize) -> Self {
        Self {
            number: n,
            qs: [const { PinList::new() }; SCHED_NQS],
            idleq: PinList::new(),
            idle_addr: AtomicUsize::new(0),
            deadproc: PinList::new(),
            whichqs: AtomicU32::new(0),
            nrun: AtomicU32::new(0),
            curpriority: AtomicU8::new(0),
            schedflags: AtomicI32::new(0),
        }
    }

    /// The CPU's number.
    pub fn number(&self) -> usize {
        self.number
    }

    /// `spc_schedflags`.
    pub fn schedflags(&self) -> i32 {
        self.schedflags.load(Ordering::Relaxed)
    }

    /// `spc_schedflags |= bits`.
    pub fn set_schedflags(&self, bits: i32) {
        self.schedflags.fetch_or(bits, Ordering::Relaxed);
    }

    /// `spc_schedflags &= ~bits`.
    pub fn clear_schedflags(&self, bits: i32) {
        self.schedflags.fetch_and(!bits, Ordering::Relaxed);
    }

    /// `spc_nrun`.
    pub fn nrun(&self) -> u32 {
        self.nrun.load(Ordering::Relaxed)
    }

    /// `spc_whichqs`.
    pub fn whichqs(&self) -> u32 {
        self.whichqs.load(Ordering::Relaxed)
    }

    /// `spc_curpriority`.
    pub fn curpriority(&self) -> u8 {
        self.curpriority.load(Ordering::Relaxed)
    }

    /// `spc_curpriority = prio`.
    pub fn set_curpriority(&self, prio: u8) {
        self.curpriority.store(prio, Ordering::Relaxed);
    }

    /// `spc_idleproc` as an address (0 before it is set): compared, never followed.
    pub fn idle_addr(&self) -> usize {
        self.idle_addr.load(Ordering::Relaxed)
    }

    /// `cpu_is_idle(ci)`: nothing is queued here.
    pub fn is_idle(&self) -> bool {
        self.whichqs() == 0
    }
}

/// The scheduler's state of one glue's threads: the sleep queues, the CPUs' states, the CPU
/// sets and the reaper's queue. One `static` per glue (its `ProcGlue::sched`).
pub struct SchedTables<G: ProcGlue> {
    /// \[S\] `slpque`: the sleep queues, by `LOOKUP(ident)`.
    slpque: [PinList<G::ThreadRunq>; TABLESIZE],
    /// The CPUs' states, by number, made by [`sched_init_cpu`].
    cpus: [OnceStatic<&'static SchedCpu<G>>; MAXCPUS],
    /// \[D\] `deadproc`: the dead threads waiting for the reaper.
    deadproc: PinList<G::ThreadRunq>,
    /// `sched_idle_cpus`.
    pub idle_cpus: Cpuset,
    /// `sched_queued_cpus`.
    pub queued_cpus: Cpuset,
    /// `sched_all_cpus`.
    pub all_cpus: Cpuset,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> SchedTables<G> {
    /// No CPU, every queue empty.
    pub const fn new() -> Self {
        Self {
            slpque: [const { PinList::new() }; TABLESIZE],
            cpus: [const { OnceStatic::new() }; MAXCPUS],
            deadproc: PinList::new(),
            idle_cpus: Cpuset::new(),
            queued_cpus: Cpuset::new(),
            all_cpus: Cpuset::new(),
            _g: PhantomData,
        }
    }

    /// CPU `n`'s state, if it joined.
    pub fn cpu(&self, n: usize) -> Option<&'static SchedCpu<G>> {
        self.cpus.get(n).and_then(|c| c.get().copied())
    }

    /// The running CPU's state: `panic(9)` before [`sched_init_cpu`] made it.
    pub fn curcpu(&self) -> &'static SchedCpu<G> {
        let n = hal::cpu::cpu_number();
        match self.cpu(n) {
            Some(c) => c,
            None => kpanic(format_args!("sched: cpu{n} has no scheduler state")),
        }
    }
}

impl<G: ProcGlue> Default for SchedTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `deadproc_mutex` and the CPUs' `spc_deadproc`'s lock: the dead threads' queues, at the
/// C's `IPL_NONE` (it is never taken from an interrupt).
static DEAD_LOCK: SharedMutex = Mutexed::new(IPL_NONE, ());

/// `nowake`: global sleep channel for threads that do not want to receive wakeup(9)
/// broadcasts.
pub static NOWAKE: AtomicI32 = AtomicI32::new(0);

/// `sched_nmigrations`: Cpu migration counter.
pub static SCHED_NMIGRATIONS: AtomicU64 = AtomicU64::new(0);
/// `sched_nomigrations`: Cpu no migration counter.
pub static SCHED_NOMIGRATIONS: AtomicU64 = AtomicU64::new(0);
/// `sched_noidle`: Times we didn't pick the idle task.
pub static SCHED_NOIDLE: AtomicU64 = AtomicU64::new(0);
/// `sched_stolen`: Times we stole proc from other cpus.
pub static SCHED_STOLEN: AtomicU64 = AtomicU64::new(0);
/// `sched_choose`: Times we chose a cpu.
pub static SCHED_CHOOSE: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
std::thread_local! {
    /// The thread the last `sched_choosecpu` of this host thread chose a CPU for (its
    /// address): the tests' witness that a placement went through it (the global
    /// `sched_choose` counts every parallel test's).
    static CHOSE_FOR: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}
/// `sched_wasidle`: Times we came out of idle.
pub static SCHED_WASIDLE: AtomicU64 = AtomicU64::new(0);
/// `sched_blockcpu` (`__HAVE_CPU_TOPOLOGY`): types of cpu to not schedule on (`CPUTYP_*`).
pub static SCHED_BLOCKCPU: AtomicI32 = AtomicI32::new(0);

/// `sched_cost_priority`.
pub static SCHED_COST_PRIORITY: AtomicI32 = AtomicI32::new(1);
/// `sched_cost_runnable`.
pub static SCHED_COST_RUNNABLE: AtomicI32 = AtomicI32::new(3);
/// `sched_cost_resident`.
pub static SCHED_COST_RESIDENT: AtomicI32 = AtomicI32::new(1);

/// \[I\] `roundrobin_period`: roundrobin period (ns).
pub static ROUNDROBIN_PERIOD: AtomicU64 = AtomicU64::new(0);

/// `averunnable.ldavg`: the 1, 5 and 15 minute load averages, fixed point (`FSCALE`).
pub static AVERUNNABLE: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];

/// `cexp[3]`: constants for averages over 1, 5, and 15 minutes when sampling at 5 second
/// intervals: exp(-1/12), exp(-1/60), exp(-1/180), in `FSCALE` fixed point (the C's doubles
/// constant-folded).
const CEXP: [u32; 3] = [
    (0.920_044_414_629_323_2 * FSCALE as f64) as u32,
    (0.983_471_453_821_617_4 * FSCALE as f64) as u32,
    (0.994_459_848_004_896_7 * FSCALE as f64) as u32,
];

/// `ccpu`: decay 95% of `p_pctcpu` in 60 seconds, exp(-1/20).
pub const CCPU: u32 = (0.951_229_424_500_714 * FSCALE as f64) as u32;

/// `CCPU_SHIFT`: 1 - exp(-1/20) ~= 0.0487 ~= 0.0488 == 1 (fixed pt, *11* bits).
const CCPU_SHIFT: u32 = 11;

/// The highest barrier ticket whose task passed through each CPU, by CPU number.
static BARRIER_DONE: [AtomicU64; MAXCPUS] = [const { AtomicU64::new(0) }; MAXCPUS];
/// The last barrier ticket handed out for each CPU, by CPU number.
static BARRIER_TICKET: [AtomicU64; MAXCPUS] = [const { AtomicU64::new(0) }; MAXCPUS];

/// `schedcpu`'s and `update_loadavg`'s timeouts, set by [`scheduler_start`].
static SCHEDCPU_TO: Timeout = Timeout::unset();
/// `update_loadavg`'s timeout.
static LOADAVG_TO: Timeout = Timeout::unset();

/// `LOOKUP(x)`.
#[inline]
fn lookup(ident: usize) -> usize {
    (ident >> 8) & (TABLESIZE - 1)
}

/// The tables of `G`'s threads.
#[inline]
fn tables<G: ProcGlue>() -> &'static SchedTables<G> {
    G::sched()
}

/// The CPU state of CPU `n`, which must have joined.
#[inline]
fn cpu_of<G: ProcGlue>(n: usize) -> &'static SchedCpu<G> {
    match tables::<G>().cpu(n) {
        Some(c) => c,
        None => kpanic(format_args!("sched: cpu{n} has no scheduler state")),
    }
}

/// The number of the CPU `p` runs or last ran on (`p->p_cpu`), from the `cpu_info` word its
/// slot holds; `None` before it first ran or was queued.
#[inline]
pub fn thread_cpu<G: ProcGlue>(p: &Thread<G>) -> Option<usize> {
    usize::try_from(p.p_cpuno.get()).ok()
}

/// `p->p_cpu = ci` for CPU `n`: the word of CPU `n`'s core, an attached one or the running
/// CPU's (a host thread's core is its own, never attached), and its number beside it
/// ([`thread_cpu`]). A number that names no CPU is a scheduler bug (`kassert`).
#[inline]
pub fn set_thread_cpu<G: ProcGlue>(p: &Thread<G>, n: usize) {
    // The two are written together: a thread put back on its own CPU keeps both.
    if usize::try_from(p.p_cpuno.get()) == Ok(n) {
        return;
    }
    let core = hal::cpu::by_number(n).or_else(|| {
        let c = hal::cpu::curcpu();
        (c.number() == n).then_some(c)
    });
    kassert!(core.is_some());
    if let Some(core) = core {
        p.set_cpu_word(core.word());
        p.p_cpuno.set(n as i32);
    }
}

/// The address of `p`, compared, never followed.
#[inline]
fn addr_of<G: ProcGlue>(p: &Thread<G>) -> usize {
    ptr::from_ref(p).addr()
}

/// `curproc`, for `f`; the scheduler's callers always have one.
#[inline]
fn with_curproc<G: ProcGlue, R>(f: impl FnOnce(Pin<&Thread<G>>) -> R) -> R {
    hal::thread::with_current::<Thread<G>, R>(f)
}

/// `SCHED_ASSERT_LOCKED()`.
#[inline]
fn sched_assert_locked() {
    kassert!(hal::thread::sched_locked());
}

/// `SCHED_ASSERT_UNLOCKED()`.
#[inline]
fn sched_assert_unlocked() {
    kassert!(!hal::thread::sched_locked());
}

/// `sched_init`: setup the bare minimum to allow things like `setrunqueue()` to work even
/// before the scheduler is actually started: the boot CPU in `sched_all_cpus`.
pub fn sched_init<G: ProcGlue>() {
    tables::<G>().all_cpus.add(hal::cpu::cpu_number());
}

/// `sched_init_cpu(ci)`'s scheduler part: CPU `n`'s run queues (empty) made and published;
/// its clock interrupts and idle thread are its caller's (`bsd`'s `kern_sched.rs`). Panics
/// when `n` joined already.
pub fn sched_init_cpu<G: ProcGlue>(n: usize) {
    let c: &'static SchedCpu<G> = alloc::boxed::Box::leak(alloc::boxed::Box::new(SchedCpu::new(n)));
    if tables::<G>().cpus[n].set(c).is_err() {
        kpanic(format_args!("sched_init_cpu: cpu{n} twice"));
    }
}

/// `spc->spc_idleproc = p` (`sched_kthreads_create`): `p` is CPU `n`'s idle thread from now
/// on, for ever (it stays on the CPU's idle queue, so it is never freed).
pub fn set_idleproc<G: ProcGlue>(n: usize, p: Pin<&Thread<G>>) {
    let c = cpu_of::<G>(n);
    let mut h = sched_lock();
    if !c.idleq.is_empty(&h) {
        kpanic(format_args!(
            "sched_kthreads_create: cpu{n} has an idle thread"
        ));
    }
    c.idleq.insert_tail(&mut h, p);
    c.idle_addr.store(addr_of(p.get_ref()), Ordering::Relaxed);
    drop(h);
    set_thread_cpu(p.get_ref(), n);
}

/// `f` on CPU `n`'s idle thread, under the scheduler's hold `f` is lent
/// (`spc->spc_idleproc`, for the machine that re-creates its frames when the CPU hatches,
/// `hal::thread::thread_restart`); `None` before it is set.
pub fn with_idleproc<G: ProcGlue, R>(
    n: usize,
    f: impl FnOnce(&SchedHold, Pin<&Thread<G>>) -> R,
) -> Option<R> {
    let c = tables::<G>().cpu(n)?;
    let h = sched_lock();
    let r = c.idleq.first_pinned(&h).map(|p| f(&h, p));
    drop(h);
    r
}

/// The idle thread of the running CPU as a token, under `h`; `panic(9)` before it is set
/// ("no idleproc set on CPU%d").
fn idle_next<G: ProcGlue>(c: &SchedCpu<G>, h: &SchedHold) -> Next {
    match c.idleq.first_pinned(h) {
        Some(p) => {
            p.set_stat(SRUN);
            Next::of(h, p)
        }
        None => kpanic(format_args!("no idleproc set on CPU{}", c.number)),
    }
}

/// `sched_idle(ci)`: the idle thread of CPU `n`. It runs without the kernel lock and never
/// returns.
pub fn sched_idle<G: ProcGlue>(n: usize) -> ! {
    hal::klock::kernel_unlock();

    // The idle thread is setup in fork1(). When the CPU hatches we enter here for the first
    // time. The CPU is now ready to take work and so add it to sched_all_cpus when
    // appropriate. After that just go away and properly reenter once idle.
    let t = tables::<G>();
    if G::cputype(n) & SCHED_BLOCKCPU.load(Ordering::Relaxed) == 0 {
        t.all_cpus.add(n);
    }
    let spc = cpu_of::<G>(n);

    kassert!(n == hal::cpu::cpu_number());

    with_curproc::<G, _>(|p| {
        kassert!(addr_of(p.get_ref()) == spc.idle_addr());
        kassert!(thread_cpu(p.get_ref()) == Some(n)); // KASSERT(p->p_cpu == ci)

        let h = sched_lock();
        p.set_stat(SSLEEP);
        mi_switch(p, h);

        loop {
            while spc.whichqs() != 0 {
                let h = sched_lock();
                p.set_stat(SSLEEP);
                mi_switch(p, h);

                exit2::<G>(spc);
            }

            hal::intr::splassert(hal::intr::IPL_NONE, "sched_idle");

            G::smr_idle();

            t.idle_cpus.add(n);
            G::idle_enter();
            while spc.whichqs() == 0 {
                let flags = spc.schedflags();
                if flags & SPCF_SHOULDHALT != 0 && flags & SPCF_HALTED == 0 {
                    t.idle_cpus.del(n);
                    let h = sched_lock();
                    let halted = if spc.whichqs() != 0 { 0 } else { SPCF_HALTED };
                    spc.set_schedflags(halted);
                    drop(h);
                    wakeup(ptr::from_ref(spc).addr());
                }
                G::idle_cycle();
            }
            G::idle_leave();
            t.idle_cpus.del(n);
        }
    })
}

/// `exit2(p)` of every dead thread of `spc`, which only its idle thread drains: each moved
/// to the reaper's queue (its time accounted first, `G::exit_account`), and the reaper woken.
fn exit2<G: ProcGlue>(spc: &SchedCpu<G>) {
    let t = tables::<G>();
    let mut moved = false;
    loop {
        let mut g = DEAD_LOCK.lock();
        let Some(p) = spc.deadproc.first(&g) else {
            break;
        };
        // account the remainder of time spent in exit1()
        G::exit_account(p);
        spc.deadproc.move_first_to(&mut g, &t.deadproc);
        moved = true;
    }
    if moved {
        wakeup(ptr::from_ref(&t.deadproc).addr());
    }
}

/// The reaper's wait for a dead thread (`reaper`'s `msleep(&deadproc, &deadproc_mutex,
/// PVM, "reaper")` loop): `take` runs on the first dead thread, still on the queue under
/// `deadproc_mutex` (it must not sleep: the reaper takes the thread's own box out of it),
/// then the thread leaves the queue; its result is returned.
pub fn reaper_next<G: ProcGlue, R>(mut take: impl FnMut(Pin<&Thread<G>>) -> R) -> R {
    let t = tables::<G>();
    let mut g = DEAD_LOCK.lock();
    loop {
        if let Some(p) = t.deadproc.first_pinned(&g) {
            let r = take(p);
            t.deadproc.remove_first(&mut g);
            return r;
        }
        let _ = g.sleep_nsec(
            ptr::from_ref(&t.deadproc),
            hal::sleep::PVM,
            "reaper",
            INFSLP,
        );
    }
}

/// `sched_exit(p)`: to free our address space we have to jump through a few hoops. The
/// freeing is done by the reaper, but until we have one reaper per cpu, we have no way of
/// putting this proc on the deadproc list and waking up the reaper without risking having
/// our address space and stack torn from under us before we manage to switch to another
/// proc. Therefore we have a per-cpu list of dead processes where we put this proc and have
/// idle clean up that list and move it to the reaper list.
pub fn sched_exit<G: ProcGlue>(p: Pin<&Thread<G>>) -> ! {
    let spc = tables::<G>().curcpu();

    {
        let mut g = DEAD_LOCK.lock();
        spc.deadproc.insert_tail(&mut g, p);
    }

    G::tuagg_add_runtime();

    hal::klock::kernel_assert_locked();
    toidle(Some(p))
}

/// `sched_toidle()`: switches to the idle thread without saving the current context (no
/// thread ran on this CPU yet: a secondary CPU's start; a dying thread goes by
/// [`sched_exit`]).
pub fn sched_toidle<G: ProcGlue>() -> ! {
    toidle::<G>(None)
}

/// `sched_toidle`'s work, for the dead thread `old` (`sched_exit`'s), which `hal` marks dead
/// as it leaves it (its u-area can then go, `hal::thread`'s E6), or for none.
fn toidle<G: ProcGlue>(old: Option<Pin<&Thread<G>>>) -> ! {
    let spc = tables::<G>().curcpu();

    // This process no longer needs to hold the kernel lock.
    if hal::klock::kernel_lock_held() {
        let _ = hal::klock::release_all();
    }

    stop_clocks(spc);

    spc.clear_schedflags(SPCF_SWITCHCLEAR);

    let h = sched_lock();
    let next = idle_next(spc, &h);

    UVMEXP.swtch.fetch_add(1, Ordering::Relaxed);
    // TRACEPOINT(sched, off__cpu, ...): dt(4), not configured.
    hal::thread::exit_to_idle(h, old, next)
}

/// Stops the optional clock interrupts of the running CPU (`mi_switch`, `sched_toidle`).
fn stop_clocks<G: ProcGlue>(spc: &SchedCpu<G>) {
    let flags = spc.schedflags();
    let itimer = flags & SPCF_ITIMER != 0;
    let profclock = flags & SPCF_PROFCLOCK != 0;
    if itimer || profclock {
        spc.clear_schedflags(SPCF_ITIMER | SPCF_PROFCLOCK);
        G::clocks_stop(itimer, profclock);
    }
}

/// Starts the optional clock interrupts `p` needs (`mi_switch`, `proc_trampoline_mi`) and
/// records its start time on the running CPU.
fn start_clocks<G: ProcGlue>(spc: &SchedCpu<G>, p: &Thread<G>) {
    let ps_flags = p.process().ps_flags.load(Ordering::Relaxed);
    let itimer = ps_flags & PS_ITIMER != 0;
    let profclock = ps_flags & PS_PROFIL != 0;
    if itimer {
        spc.set_schedflags(SPCF_ITIMER);
    }
    if profclock {
        spc.set_schedflags(SPCF_PROFCLOCK);
    }
    G::clocks_start(itimer, profclock);
}

/// `setrunqueue(ci, p, prio)`: puts `p` on the run queue for `prio` of CPU `cpu` (`None`:
/// [`sched_choosecpu`]'s choice). `p` is on no queue.
pub fn setrunqueue<G: ProcGlue>(
    h: &mut SchedHold,
    cpu: Option<usize>,
    p: Pin<&Thread<G>>,
    prio: u8,
) {
    let spc = queue_place(h, cpu, p.get_ref(), prio);
    spc.qs[usize::from(prio >> 2)].insert_tail(h, p);
    queued(spc, prio);
}

/// `setrunqueue`'s look at `p` before it is linked: the CPU chosen, `p`'s `p_cpu`, `p_stat`
/// and `p_runpri` set, `spc_nrun` counted; the CPU's state.
fn queue_place<G: ProcGlue>(
    h: &SchedHold,
    cpu: Option<usize>,
    p: &Thread<G>,
    prio: u8,
) -> &'static SchedCpu<G> {
    let n = match cpu {
        Some(n) => n,
        None => sched_choosecpu(h, p),
    };

    sched_assert_locked();
    kassert!(p.p_wchan.get() == 0);
    kassert!(p.p_flag.load(Ordering::Relaxed) & P_INSCHED == 0);

    set_thread_cpu(p, n);
    p.set_stat(SRUN);
    p.p_runpri.set(prio);

    let spc = cpu_of::<G>(n);
    spc.nrun.fetch_add(1, Ordering::Relaxed);
    // TRACEPOINT(sched, enqueue, ...): dt(4), not configured.
    spc
}

/// `setrunqueue`'s bookkeeping once `p` is linked on `spc`'s queue for `prio`: the queue
/// flagged, the CPU queued, an idle CPU kicked or a busy one asked to reschedule.
fn queued<G: ProcGlue>(spc: &SchedCpu<G>, prio: u8) {
    let t = tables::<G>();
    let queue = usize::from(prio >> 2);
    spc.whichqs.fetch_or(1 << queue, Ordering::Relaxed);
    t.queued_cpus.add(spc.number);

    if t.idle_cpus.isset(spc.number) {
        G::cpu_unidle(spc.number);
    } else if prio < spc.curpriority() {
        G::need_resched(spc.number);
    }
}

/// `remrunqueue(p)`: takes `p` off its run queue.
pub fn remrunqueue<G: ProcGlue>(h: &mut SchedHold, p: &Thread<G>) {
    let queue = usize::from(p.p_runpri.get() >> 2);

    sched_assert_locked();
    let Some(n) = thread_cpu(p) else {
        kpanic(format_args!(
            "remrunqueue: thread {} has no CPU",
            p.p_tid.get()
        ));
    };
    let spc = cpu_of::<G>(n);
    spc.nrun.fetch_sub(1, Ordering::Relaxed);
    // TRACEPOINT(sched, dequeue, ...): dt(4), not configured.

    spc.qs[queue].remove(h, p);
    dequeued(h, spc, queue);
}

/// `remrunqueue`'s bookkeeping once a thread left `spc`'s `queue`.
fn dequeued<G: ProcGlue>(h: &SchedHold, spc: &SchedCpu<G>, queue: usize) {
    if spc.qs[queue].is_empty(h) {
        spc.whichqs.fetch_and(!(1 << queue), Ordering::Relaxed);
        if spc.whichqs() == 0 {
            tables::<G>().queued_cpus.del(spc.number);
        }
    }
}

/// The head of `spc`'s `queue` taken off it and put on the run queue for its priority of
/// the CPU [`sched_choosecpu`] picks (`sched_chooseproc`'s hand-over of a halting CPU's
/// threads): the CPU it went to. A thread pegged to this CPU stays here (`sched_choosecpu`
/// gives its own CPU back): the C's `remrunqueue` and `setrunqueue` of it on the same queue
/// move it from the head to the tail, through a local queue of the same lock.
fn requeue_first<G: ProcGlue>(h: &mut SchedHold, spc: &SchedCpu<G>, queue: usize) -> Option<usize> {
    let (n, prio) = {
        let p = spc.qs[queue].first(h)?;
        let prio = p.p_runpri.get();
        // remrunqueue's accounting, then setrunqueue's look, while `p` is still linked.
        spc.nrun.fetch_sub(1, Ordering::Relaxed);
        let ci = sched_choosecpu(h, p);
        if ci == spc.number {
            // KASSERT(p->p_flag & P_CPUPEG): only a pegged thread stays on a halting CPU.
            kassert!(p.p_flag.load(Ordering::Relaxed) & P_CPUPEG != 0);
        }
        set_thread_cpu(p, ci);
        p.set_stat(SRUN);
        p.p_runpri.set(prio);
        cpu_of::<G>(ci).nrun.fetch_add(1, Ordering::Relaxed);
        (ci, prio)
    };
    let dst = cpu_of::<G>(n);
    let dstq = &dst.qs[usize::from(prio >> 2)];
    if ptr::eq(dstq, &spc.qs[queue]) {
        let rotate: PinList<G::ThreadRunq> = PinList::new();
        spc.qs[queue].move_first_to(h, &rotate);
        rotate.move_first_to(h, dstq);
    } else {
        spc.qs[queue].move_first_to(h, dstq);
    }
    dequeued(h, spc, queue);
    queued(dst, prio);
    Some(n)
}

/// `sched_chooseproc()`: picks the next thread to run on this CPU: the head of the highest
/// priority non-empty run queue, else a stolen one, else idle. A CPU asked to halt
/// (`SPCF_SHOULDHALT`) first hands its queued threads to other CPUs.
pub fn sched_chooseproc<G: ProcGlue>(h: &mut SchedHold) -> Next {
    let spc = tables::<G>().curcpu();

    sched_assert_locked();

    if spc.schedflags() & SPCF_SHOULDHALT != 0 {
        let mut again = false;
        'queues: for queue in 0..SCHED_NQS {
            while spc.whichqs() & (1 << queue) != 0 {
                if requeue_first(h, spc, queue) == Some(spc.number) {
                    // Only a thread pegged here stays.
                    again = true; // goto again
                    break 'queues;
                }
            }
        }
        if !again {
            return idle_next(spc, h);
        }
    }
    // again:

    let whichqs = spc.whichqs();
    if whichqs != 0 {
        let queue = whichqs.trailing_zeros() as usize; // ffs() - 1
        let Some(p) = spc.qs[queue].first_pinned(h) else {
            kpanic(format_args!(
                "sched_chooseproc: queue {queue} flagged but empty"
            ));
        };
        if p.stat() != SRUN {
            kpanic(format_args!(
                "thread {} not in SRUN: {}",
                p.p_tid.get(),
                p.stat()
            ));
        }
        kassert!(p.p_wchan.get() == 0);
        kassert!(p.p_flag.load(Ordering::Relaxed) & P_INSCHED == 0);
        let next = Next::of(h, p);
        spc.nrun.fetch_sub(1, Ordering::Relaxed);
        spc.qs[queue].remove_first(h);
        dequeued(h, spc, queue);
        SCHED_NOIDLE.fetch_add(1, Ordering::Relaxed);
        next
    } else if let Some(next) = sched_steal_proc(h, spc) {
        next
    } else {
        idle_next(spc, h)
    }
}

/// `sched_choosecpu_fork(parent, flags)`: the CPU a new thread starts on: an idle CPU with
/// nothing queued, else the one with the fewest queued threads.
pub fn sched_choosecpu_fork<G: ProcGlue>() -> usize {
    let t = tables::<G>();
    // `#if 0` in the C: PPWAIT forks could take the parent's cpu, once exec can move a
    // thread to another cpu painlessly.

    // Look at all cpus that are currently idle and have nothing queued. If there are none,
    // pick the one with least queued procs first, then the one with lowest load average.
    let set = Cpuset::complement(&t.queued_cpus, &t.idle_cpus);
    let mut set = Cpuset::intersection(&set, &t.all_cpus);
    if set.first().is_none() {
        set = t.all_cpus.copy();
    }

    let mut choice: Option<usize> = None;
    let mut best_run = u32::MAX;
    while let Some(n) = set.first() {
        set.del(n);
        let run = t.cpu(n).map_or(0, |c| c.nrun());
        if choice.is_none() || run < best_run {
            choice = Some(n);
            best_run = run;
        }
    }
    choice.unwrap_or_else(hal::cpu::cpu_number)
}

/// `sched_choosecpu(p)`: the CPU to run `p` on.
pub fn sched_choosecpu<G: ProcGlue>(h: &SchedHold, p: &Thread<G>) -> usize {
    let t = tables::<G>();
    let _ = h;

    // If pegged to a cpu, don't allow it to move.
    if p.p_flag.load(Ordering::Relaxed) & P_CPUPEG != 0
        && let Some(n) = thread_cpu(p)
    {
        return n;
    }

    SCHED_CHOOSE.fetch_add(1, Ordering::Relaxed);
    #[cfg(test)]
    CHOSE_FOR.with(|c| c.set(ptr::from_ref(p).addr()));

    // Look at all cpus that are currently idle and have nothing queued. If there are none,
    // pick the cheapest of those. (idle + queued could mean that the cpu is handling an
    // interrupt at this moment and haven't had time to leave idle yet).
    let set = Cpuset::complement(&t.queued_cpus, &t.idle_cpus);
    let mut set = Cpuset::intersection(&set, &t.all_cpus);

    // First, just check if our current cpu is in that set, if it is, this is simple. Also,
    // our cpu might not be idle, but if it's the current cpu and it has nothing else queued
    // and we're curproc, take it.
    if let Some(pn) = thread_cpu(p)
        && let Some(pspc) = t.cpu(pn)
        && (set.isset(pn)
            || (pn == hal::cpu::cpu_number()
                && pspc.nrun() == 0
                && pspc.schedflags() & SPCF_SHOULDHALT == 0
                && hal::thread::current_addr() == addr_of(p)))
    {
        SCHED_WASIDLE.fetch_add(1, Ordering::Relaxed);
        return pn;
    }

    if set.first().is_none() {
        set = t.all_cpus.copy();
    }

    let mut choice: Option<usize> = None;
    let mut last_cost = i32::MAX;
    while let Some(n) = set.first() {
        let cost = sched_proc_to_cpu_cost(n, p);
        if choice.is_none() || cost < last_cost {
            choice = Some(n);
            last_cost = cost;
        }
        set.del(n);
    }

    if choice.is_some() && choice == thread_cpu(p) {
        SCHED_NOMIGRATIONS.fetch_add(1, Ordering::Relaxed);
    } else {
        SCHED_NMIGRATIONS.fetch_add(1, Ordering::Relaxed);
    }

    choice.unwrap_or_else(hal::cpu::cpu_number)
}

/// `sched_steal_proc(self)`: attempt to steal a proc from some cpu: the cheapest unpegged
/// thread at the head priority of a CPU with queued threads, taken off its queue for this CPU.
fn sched_steal_proc<G: ProcGlue>(h: &mut SchedHold, me: &SchedCpu<G>) -> Option<Next> {
    let t = tables::<G>();
    kassert!(me.schedflags() & SPCF_SHOULDHALT == 0);

    // Don't steal if we don't want to schedule processes in this CPU.
    if !t.all_cpus.isset(me.number) {
        return None;
    }

    let set = t.queued_cpus.copy();

    // The cheapest: its CPU, queue and address (compared, while the hold keeps it linked), and
    // its token, made while it is linked.
    let mut best: Option<(usize, usize, usize)> = None;
    let mut next: Option<Next> = None;
    let mut bestcost = i32::MAX;
    while let Some(n) = set.first() {
        set.del(n);
        let Some(spc) = t.cpu(n) else {
            continue;
        };
        let whichqs = spc.whichqs();
        if whichqs == 0 {
            continue;
        }
        let queue = whichqs.trailing_zeros() as usize; // ffs() - 1
        for p in spc.qs[queue].iter_pinned(h) {
            if p.p_flag.load(Ordering::Relaxed) & P_CPUPEG != 0 {
                continue;
            }
            let cost = sched_proc_to_cpu_cost(me.number, p.get_ref());
            if best.is_none() || cost < bestcost {
                best = Some((n, queue, addr_of(p.get_ref())));
                next = Some(Next::of(h, p));
                bestcost = cost;
            }
        }
    }
    let (n, queue, addr) = best?;
    let next = next?;

    // TRACEPOINT(sched, steal, ...): dt(4), not configured.

    // remrunqueue(best) and `best->p_cpu = self`, in the one walk that takes it off.
    let spc = cpu_of::<G>(n);
    spc.nrun.fetch_sub(1, Ordering::Relaxed);
    let took = spc.qs[queue].remove_where(h, |p| {
        let it = addr_of(p) == addr;
        if it {
            set_thread_cpu(p, me.number);
        }
        it
    });
    kassert!(took);
    dequeued(h, spc, queue);

    SCHED_STOLEN.fetch_add(1, Ordering::Relaxed);
    Some(next)
}

/// `log2`: base 2 logarithm of an int. returns 0 for 0 (yeye, I know).
fn log2(mut i: u32) -> i32 {
    let mut ret = 0;
    loop {
        i >>= 1;
        if i == 0 {
            break;
        }
        ret += 1;
    }
    ret
}

/// What `sched_proc_to_cpu_cost` weighs, read from the CPU and the thread.
#[derive(Clone, Copy, Debug)]
struct CpuCostInputs {
    /// The CPU is in `sched_idle_cpus`.
    idle: bool,
    /// The CPU is in `sched_queued_cpus`.
    queued: bool,
    /// `CPU_IS_PRIMARY(ci)`: it takes the hardware interrupts.
    primary: bool,
    /// `p->p_usrpri`.
    usrpri: u8,
    /// `spc->spc_curpriority`.
    curpriority: u8,
    /// `spc->spc_nrun`.
    nrun: u32,
    /// `pmap_resident_count` of the thread's map when it last ran on this CPU and has not
    /// slept since (`p->p_cpu == ci && p->p_slptime == 0`).
    resident: Option<i64>,
}

/*
 * Calculate the cost of moving the proc to this cpu.
 *
 * What we want is some guesstimate of how much "performance" it will
 * cost us to move the proc here. Not just for caches and TLBs and NUMA
 * memory, but also for the proc itself. A highly loaded cpu might not
 * be the best candidate for this proc since it won't get run.
 *
 * Just total guesstimates for now.
 */

/// The arithmetic of `sched_proc_to_cpu_cost`.
fn cpu_cost(c: CpuCostInputs) -> i32 {
    let priority = SCHED_COST_PRIORITY.load(Ordering::Relaxed);
    let runnable = SCHED_COST_RUNNABLE.load(Ordering::Relaxed);
    let resident = SCHED_COST_RESIDENT.load(Ordering::Relaxed);
    let mut cost = 0i32;

    // First, account for the priority of the proc we want to move. More willing to move,
    // the lower the priority of the destination and the higher the priority of the proc.
    if !c.idle {
        cost += (i32::from(c.usrpri) - i32::from(c.curpriority)) * priority;
        cost += runnable;
    }
    if c.queued {
        cost += c.nrun as i32 * runnable;
    }

    // Try to avoid the primary cpu as it handles hardware interrupts.
    //
    // XXX Needs to be revisited when we distribute interrupts over cpus.
    if c.primary {
        cost += runnable;
    }

    // If the proc is on this cpu already, lower the cost by how much it has been running and
    // an estimate of its footprint.
    if let Some(count) = c.resident {
        let l2resident = log2(count as u32);
        cost -= l2resident * resident;
    }

    cost
}

/// `sched_proc_to_cpu_cost(ci, p)`: calculate the cost of moving the proc to CPU `n`.
fn sched_proc_to_cpu_cost<G: ProcGlue>(n: usize, p: &Thread<G>) -> i32 {
    let t = tables::<G>();
    let (curpriority, nrun) = t.cpu(n).map_or((0, 0), |c| (c.curpriority(), c.nrun()));
    let resident = (thread_cpu(p) == Some(n) && p.p_slptime.get() == 0)
        .then(|| G::resident_count(p))
        .flatten();
    cpu_cost(CpuCostInputs {
        idle: t.idle_cpus.isset(n),
        queued: t.queued_cpus.isset(n),
        primary: G::cpu_is_primary(n),
        usrpri: p.p_usrpri.get(),
        curpriority,
        nrun,
        resident,
    })
}

/// `sched_peg_curproc(ci)`: peg the running thread to CPU `n` (it runs only there from now
/// on, and goes there now).
pub fn sched_peg_curproc<G: ProcGlue>(n: usize) {
    with_curproc::<G, _>(|p| {
        let mut h = sched_lock();
        p.p_flag.fetch_or(P_CPUPEG, Ordering::Relaxed);
        setrunqueue(&mut h, Some(n), p, p.p_usrpri.get());
        p.p_ru.ru_nvcsw.set(p.p_ru.ru_nvcsw.get() + 1);
        mi_switch(p, h);
    });
}

/// `sched_unpeg_curproc()`.
pub fn sched_unpeg_curproc<G: ProcGlue>() {
    with_curproc::<G, _>(|p| {
        p.p_flag.fetch_and(!P_CPUPEG, Ordering::Relaxed);
    });
}

/// `sched_start_secondary_cpus()`: lets the running secondary CPUs take threads again.
pub fn sched_start_secondary_cpus<G: ProcGlue>() {
    let t = tables::<G>();
    for core in hal::cpu::cores() {
        let n = core.number();
        let Some(spc) = t.cpu(n) else {
            continue;
        };
        if G::cpu_is_primary(n) || !G::cpu_is_running(n) {
            continue;
        }
        spc.clear_schedflags(SPCF_SHOULDHALT | SPCF_HALTED);
        if G::cputype(n) & SCHED_BLOCKCPU.load(Ordering::Relaxed) != 0 {
            continue;
        }
        t.all_cpus.add(n);
    }
}

/// `sched_stop_secondary_cpus()`: makes sure we stop the secondary CPUs: each hands its
/// threads over and parks in its idle loop (`SPCF_HALTED`); returns once all have.
pub fn sched_stop_secondary_cpus<G: ProcGlue>() {
    let t = tables::<G>();
    // Make sure we stop the secondary CPUs.
    for core in hal::cpu::cores() {
        let n = core.number();
        let Some(spc) = t.cpu(n) else {
            continue;
        };
        if G::cpu_is_primary(n) || !G::cpu_is_running(n) {
            continue;
        }
        t.all_cpus.del(n);
        spc.set_schedflags(SPCF_SHOULDHALT);
    }
    for core in hal::cpu::cores() {
        let n = core.number();
        let Some(spc) = t.cpu(n) else {
            continue;
        };
        if G::cpu_is_primary(n) || !G::cpu_is_running(n) {
            continue;
        }
        let halted = || spc.schedflags() & SPCF_HALTED != 0;
        while !halted() {
            sleep_setup::<G>(ptr::from_ref(spc).addr(), PZERO, "schedstate");
            let _ = sleep_finish::<G>(INFSLP, !halted());
        }
    }
}

/// `sched_barrier_task`: runs on `systqmp` with the CPU's number as its argument, moves
/// itself there, then signals the CPU's barrier generation.
fn sched_barrier_task<G: ProcGlue>(arg: *mut c_void) {
    let (n, ticket) = barrier_unpack(arg.addr());
    sched_peg_curproc::<G>(n);
    // cond_signal(&sb->cond): this caller's pass through CPU `n` is done.
    BARRIER_DONE[n].fetch_max(ticket, Ordering::Release);
    wakeup(ptr::from_ref(&BARRIER_DONE[n]).addr());
    sched_unpeg_curproc::<G>();
}

/// Whether the caller of `sched_barrier` with `ticket` on CPU `n` still waits: no pass at or
/// past its ticket through the CPU yet, or its own task `t` still queued.
fn barrier_waiting(n: usize, ticket: u64, t: &crate::task::Task) -> bool {
    BARRIER_DONE[n].load(Ordering::Acquire) < ticket || t.pending()
}

/// A barrier task's argument: CPU `n` and the caller's ticket.
fn barrier_pack(n: usize, ticket: u64) -> usize {
    (ticket as usize) << 9 | n
}

/// The CPU and the ticket a barrier task's argument names.
fn barrier_unpack(w: usize) -> (usize, u64) {
    (w & 0x1ff, (w >> 9) as u64)
}

/// `sched_barrier(ci)`: returns once CPU `n` (the primary CPU for `None`) has gone through
/// the scheduler, i.e. every interrupt handler it was running has finished.
pub fn sched_barrier<G: ProcGlue>(n: Option<usize>) {
    let n = match n {
        Some(n) => n,
        None => match hal::cpu::cores()
            .map(|c| c.number())
            .find(|&n| G::cpu_is_primary(n))
        {
            Some(n) => n,
            None => kpanic(format_args!("sched_barrier: no primary cpu")),
        },
    };

    if n == hal::cpu::cpu_number() {
        return;
    }

    // The C's per-caller `struct sched_barrier_state`: this caller's ticket, taken in call
    // order, which its own task sets as the CPU's done mark once it ran there; a later
    // caller's task passing first satisfies it too (it passed after this call).
    let ticket = BARRIER_TICKET[n].fetch_add(1, Ordering::AcqRel) + 1;
    let task = core::pin::pin!(crate::task::Task::new(
        sched_barrier_task::<G>,
        ptr::without_provenance_mut(barrier_pack(n, ticket))
    ));
    let _ = crate::task::task_add(&crate::task::SYSTQMP, task.as_ref());
    // Pinned on this frame: wait for this CPU's pass (the mark at or past this ticket) and
    // for this caller's own task to have left the queue, so the frame never goes with the
    // task linked (another caller's pass may come first: kern_sched.c keeps one cond per
    // caller).
    let done = &BARRIER_DONE[n];
    let waiting = |t: &crate::task::Task| barrier_waiting(n, ticket, t);
    while waiting(task.as_ref().get_ref()) {
        sleep_setup::<G>(ptr::from_ref(done).addr(), PWAIT, "sbar");
        let _ = sleep_finish::<G>(INFSLP, waiting(task.as_ref().get_ref()));
    }
}

/// `sysctl_hwncpuonline()`: the CPUs that schedule threads.
pub fn ncpuonline<G: ProcGlue>() -> u32 {
    tables::<G>().all_cpus.cardinality()
}

/// `cpu_is_online(ci)`.
pub fn cpu_is_online<G: ProcGlue>(n: usize) -> bool {
    tables::<G>().all_cpus.isset(n)
}

/// `sched_cpuadjust(newblockcpu)` (`__HAVE_CPU_TOPOLOGY`): blocks the secondary CPUs whose
/// type is in `newblockcpu` and lets the others schedule again.
pub fn sched_cpuadjust<G: ProcGlue>(newblockcpu: i32) {
    if newblockcpu == SCHED_BLOCKCPU.load(Ordering::Relaxed) {
        return;
    }
    SCHED_BLOCKCPU.store(newblockcpu, Ordering::Relaxed);
    let t = tables::<G>();
    for core in hal::cpu::cores() {
        let n = core.number();
        if G::cpu_is_primary(n) || !G::cpu_is_running(n) {
            continue;
        }
        let inset = t.all_cpus.isset(n);
        if G::cputype(n) & newblockcpu != 0 {
            if inset {
                t.all_cpus.del(n);
            }
        } else if !inset {
            t.all_cpus.add(n);
        }
    }
}

/// `roundrobin(cr, cf, arg)`'s work on the running CPU (its clock interrupt advanced `count`
/// periods): force switch among equal priority processes every 100ms.
pub fn roundrobin<G: ProcGlue>(count: u64) {
    let n = hal::cpu::cpu_number();
    let Some(spc) = tables::<G>().cpu(n) else {
        return;
    };

    if hal::thread::current_addr() != 0 {
        if spc.schedflags() & SPCF_SEENRR != 0 || count >= 2 {
            // The process has already been through a roundrobin without switching and may
            // be hogging the CPU. Indicate that the process should yield.
            spc.set_schedflags(SPCF_SEENRR | SPCF_SHOULDYIELD);
        } else {
            spc.set_schedflags(SPCF_SEENRR);
        }
    }

    if spc.nrun() != 0 || spc.schedflags() & SPCF_SHOULDYIELD != 0 {
        G::need_resched(n);
    }
}

/// `update_loadavg`: compute a tenex style load average of a quantity on 1, 5, and 15
/// minute intervals; again in 5 seconds.
fn update_loadavg<G: ProcGlue>(_unused: *mut c_void) {
    let t = tables::<G>();
    let set = Cpuset::complement(&t.idle_cpus, &t.all_cpus);
    let mut nrun = u64::from(set.cardinality());
    for core in hal::cpu::cores() {
        if let Some(c) = t.cpu(core.number()) {
            nrun += u64::from(c.nrun());
        }
    }

    for (ldavg, &cexp) in AVERUNNABLE.iter().zip(CEXP.iter()) {
        let old = u64::from(ldavg.load(Ordering::Relaxed));
        let new =
            (u64::from(cexp) * old + nrun * u64::from(FSCALE) * u64::from(FSCALE - cexp)) >> FSHIFT;
        ldavg.store(new as u32, Ordering::Relaxed);
    }

    let _ = hal::pin::pin_static(&LOADAVG_TO).add_sec(5);
}

/// `loadfactor(loadav)`: calculations for digital decay to forget 90% of usage in 5*loadav
/// sec.
const fn loadfactor(loadav: u32) -> u32 {
    2 * loadav
}

/// `decay_cpu(loadfac, cpu)`.
const fn decay_cpu(loadfac: u32, cpu: u32) -> u32 {
    ((loadfac as u64 * cpu as u64) / (loadfac as u64 + FSCALE as u64)) as u32
}

/// `schedcpu`: recompute process priorities, every second (a timeout without
/// `TIMEOUT_MPSAFE`, so under the kernel lock, which walks `allproc`).
fn schedcpu<G: ProcGlue>(_unused: *mut c_void) {
    let loadfac = loadfactor(AVERUNNABLE[0].load(Ordering::Relaxed));
    let stathz = G::stathz().max(1) as u32;

    let r = hal::klock::reader();
    for p in G::tables().allproc.iter_pinned(&r) {
        // Idle threads are never placed on the runqueue, therefore computing their priority
        // is pointless.
        if is_idle_thread(p.get_ref()) {
            continue;
        }
        // Increment sleep time (if sleeping). We ignore overflow.
        if p.stat() == SSLEEP || p.stat() == SSTOP {
            p.p_slptime.set(p.p_slptime.get().wrapping_add(1));
        }
        let mut pctcpu =
            ((u64::from(p.p_pctcpu.load(Ordering::Relaxed)) * u64::from(CCPU)) >> FSHIFT) as u32;
        // If the process has slept the entire second, stop recalculating its priority until
        // it wakes up.
        if p.p_slptime.get() > 1 {
            p.p_pctcpu.store(pctcpu, Ordering::Relaxed);
            continue;
        }
        let mut h = sched_lock();
        // p_pctcpu is only for diagnostic tools such as ps.
        let cpt = p.p_cpticks.get(); // READ_ONCE
        let dticks = cpt.wrapping_sub(p.p_cpticks2.get());
        // FSHIFT >= CCPU_SHIFT
        pctcpu = pctcpu.wrapping_add(if stathz == 100 {
            dticks << (FSHIFT - CCPU_SHIFT)
        } else {
            100u32.wrapping_mul(dticks << (FSHIFT - CCPU_SHIFT)) / stathz
        });
        p.p_pctcpu.store(pctcpu, Ordering::Relaxed);
        p.p_cpticks2.set(cpt);
        let newcpu = decay_cpu(loadfac, p.p_estcpu.get());
        setpriority(&h, p.get_ref(), newcpu, p.process().ps_nice.get());
        if p.stat() == SRUN
            && u32::from(p.p_runpri.get()) / SCHED_PPQ != u32::from(p.p_usrpri.get()) / SCHED_PPQ
        {
            remrunqueue(&mut h, p.get_ref());
            setrunqueue(&mut h, thread_cpu(p.get_ref()), p, p.p_usrpri.get());
        }
        drop(h);
    }
    drop(r);
    let _ = hal::pin::pin_static(&SCHEDCPU_TO).add_sec(1);
}

/// Whether `p` is a CPU's idle thread (`p == p->p_cpu->ci_schedstate.spc_idleproc`).
pub fn is_idle_thread<G: ProcGlue>(p: &Thread<G>) -> bool {
    let a = addr_of(p);
    thread_cpu(p)
        .and_then(|n| tables::<G>().cpu(n))
        .is_some_and(|c| c.idle_addr() == a)
}

/// `decay_aftersleep`: recalculate the priority of a process after it has slept for a
/// while. For all load averages >= 1 and max `p_estcpu` of 255, sleeping for at least six
/// times the loadfactor will decay `p_estcpu` to zero.
pub fn decay_aftersleep(estcpu: u32, slptime: u32) -> u32 {
    let loadfac = loadfactor(AVERUNNABLE[0].load(Ordering::Relaxed));

    if slptime > 5 * loadfac {
        return 0;
    }
    let mut newcpu = estcpu;
    let mut slptime = slptime.wrapping_sub(1); // the first time was done in schedcpu
    while newcpu != 0 {
        slptime = slptime.wrapping_sub(1);
        if slptime == 0 {
            break;
        }
        newcpu = decay_cpu(loadfac, newcpu);
    }
    newcpu
}

/// `yield`: general yield call. Puts the current process back on its run queue and performs
/// a voluntary context switch.
pub fn r#yield<G: ProcGlue>() {
    with_curproc::<G, _>(|p| {
        let mut h = sched_lock();
        setrunqueue(&mut h, thread_cpu(p.get_ref()), p, p.p_usrpri.get());
        p.p_ru.ru_nvcsw.set(p.p_ru.ru_nvcsw.get() + 1);
        mi_switch(p, h);
    });
}

/// `preempt`: general preemption call. Puts the current process back on its run queue and
/// performs an involuntary context switch.
pub fn preempt<G: ProcGlue>() {
    with_curproc::<G, _>(|p| {
        let mut h = sched_lock();
        setrunqueue(&mut h, thread_cpu(p.get_ref()), p, p.p_usrpri.get());
        p.p_ru.ru_nivcsw.set(p.p_ru.ru_nivcsw.get() + 1);
        mi_switch(p, h);
    });
}

/// `sched_pause(yield)`: [`r#yield`] when the running CPU's scheduler asks its thread to
/// give the CPU up (`SPCF_SHOULDYIELD`).
pub fn sched_pause<G: ProcGlue>() {
    let n = hal::cpu::cpu_number();
    if tables::<G>()
        .cpu(n)
        .is_some_and(|c| c.schedflags() & SPCF_SHOULDYIELD != 0)
    {
        r#yield::<G>();
    }
}

/// `sched_yield(2)`'s work for the thread `p` (the system call's): a thread of a
/// multi-threaded process drops to the lowest priority among its siblings so that they can
/// make some progress, then gives the CPU up.
pub fn sched_yield<G: ProcGlue>(p: Pin<&Thread<G>>) {
    // If one of the threads of a multi-threaded process called sched_yield(2), drop its
    // priority to ensure its siblings can make some progress.
    let pr = p.process();
    let mut newprio = p.p_usrpri.get();
    {
        let g = pr.ps_mtx.lock();
        for q in pr.ps_threads.iter(&g) {
            newprio = newprio.max(q.p_runpri.get());
        }
    }

    let mut h = sched_lock();
    setrunqueue(&mut h, thread_cpu(p.get_ref()), p, newprio);
    p.p_ru.ru_nvcsw.set(p.p_ru.ru_nvcsw.get() + 1);
    mi_switch(p, h);
}

/// `mi_switch()`: the machine-independent half of a context switch. Called with the
/// scheduler's hold and `p_stat` already changed from `SONPROC`; returns on the same thread
/// once it is scheduled again, the hold released (amendment A: right after the switch).
pub fn mi_switch<G: ProcGlue>(p: Pin<&Thread<G>>, mut h: SchedHold) {
    let spc = tables::<G>().curcpu();

    kassert!(p.stat() != SONPROC);
    sched_assert_locked();

    // Release the kernel_lock, as we are about to yield the CPU.
    let hold_count = if hal::klock::kernel_lock_held() {
        hal::klock::release_all()
    } else {
        0
    };

    // Update thread runtime
    G::tuagg_add_runtime();

    // Stop any optional clock interrupts.
    stop_clocks(spc);

    // Process is about to yield the CPU; clear the appropriate scheduling flags.
    spc.clear_schedflags(SPCF_SWITCHCLEAR);

    let next = sched_chooseproc::<G>(&mut h);

    if next.thread_addr() != addr_of(p.get_ref()) {
        UVMEXP.swtch.fetch_add(1, Ordering::Relaxed);
        // TRACEPOINT(sched, off__cpu, ...): dt(4), not configured.
        hal::thread::switch_to(&mut h, p, next);
        // TRACEPOINT(sched, on__cpu, NULL).
    } else {
        // TRACEPOINT(sched, remain__cpu, NULL).
        let _ = next;
        p.set_stat(SONPROC);
    }

    hal::cpu::curcpu().set_want_resched(0); // clear_resched(curcpu())

    sched_assert_locked();

    // The hold restores the level its taking replaced (the C's MUTEX_OLDIPL dance).
    drop(h);

    sched_assert_unlocked();

    G::assertwaitok();
    G::smr_idle();

    // We're running again; record our new start time. We might be running on a new CPU
    // now, so refetch the schedstate_percpu pointer.
    kassert!(thread_cpu(p.get_ref()) == Some(hal::cpu::cpu_number()));
    let spc = tables::<G>().curcpu();

    // Start any optional clock interrupts needed by the thread.
    start_clocks(spc, p.get_ref());

    // Reacquire the kernel_lock now. We do this after we've released the scheduler lock to
    // avoid deadlock, and before we reacquire the interlock and the scheduler lock.
    if hold_count != 0 {
        hal::klock::acquire_count(hold_count);
    }
}

/// `proc_trampoline_mi()`: do machine independent parts of switching to a new process: the
/// first thing a new thread runs (its trampoline adopted the hold the switch to it kept).
pub fn trampoline_mi<G: ProcGlue>(h: SchedHold) {
    let spc = tables::<G>().curcpu();

    sched_assert_locked();
    hal::cpu::curcpu().set_want_resched(0); // clear_resched(curcpu())
    drop(h);
    G::spl0();

    sched_assert_unlocked();
    kassert!(!hal::klock::kernel_lock_held()); // KERNEL_ASSERT_UNLOCKED()
    G::assertwaitok();
    G::smr_idle();

    // Start any optional clock interrupts needed by the thread.
    with_curproc::<G, _>(|p| start_clocks(spc, p.get_ref()));

    hal::klock::kernel_lock();
}

/// `setrunnable(p)`: change process state to be runnable, placing it on the run queue.
pub fn setrunnable<G: ProcGlue>(h: &mut SchedHold, p: Pin<&Thread<G>>) {
    if let Some(prio) = runnable_prio(h, p.get_ref()) {
        setrunqueue(h, None, p, prio);
        runnable_after(h, p.get_ref());
    }
}

/// `setrunnable`'s look at `p`: the priority to queue it at, `None` when it is not to be
/// queued (stopped or asleep, but not yet in the switch: `P_INSCHED`).
fn runnable_prio<G: ProcGlue>(h: &SchedHold, p: &Thread<G>) -> Option<u8> {
    let _ = h;
    sched_assert_locked();

    match p.stat() {
        SSTOP => {
            // TRACEPOINT(sched, unstop, ...): dt(4), not configured.
            // If not yet stopped or asleep, unstop but don't add to runq
            if p.p_flag.load(Ordering::Relaxed) & P_INSCHED != 0 {
                if p.p_wchan.get() != 0 {
                    p.set_stat(SSLEEP);
                } else {
                    p.set_stat(SONPROC);
                }
                return None;
            }
            Some(p.p_usrpri.get())
        }
        SSLEEP => {
            // TRACEPOINT(sched, wakeup, ...): dt(4), not configured.
            // if not yet asleep, don't add to runqueue
            if p.p_flag.load(Ordering::Relaxed) & P_INSCHED != 0 {
                return None;
            }
            Some(p.p_slppri.get())
        }
        // 0, SRUN, SONPROC, SDEAD, SIDL and anything else
        _ => kpanic(format_args!("setrunnable")),
    }
}

/// `setrunnable`'s work once `p` is queued: its priority decayed after a long sleep.
fn runnable_after<G: ProcGlue>(h: &SchedHold, p: &Thread<G>) {
    if p.p_slptime.get() > 1 {
        let newcpu = decay_aftersleep(p.p_estcpu.get(), p.p_slptime.get());
        setpriority(h, p, newcpu, p.process().ps_nice.get());
    }
    p.p_slptime.set(0);
}

/// `setpriority(p, newcpu, nice)`: compute the priority of a process.
pub fn setpriority<G: ProcGlue>(h: &SchedHold, p: &Thread<G>, newcpu: u32, nice: u8) {
    let _ = h;
    let newprio = (i64::from(PUSER)
        + i64::from(newcpu)
        + i64::from(NICE_WEIGHT) * (i64::from(nice) - i64::from(NZERO)))
    .min(i64::from(MAXPRI));

    sched_assert_locked();
    p.p_estcpu.set(newcpu);
    p.p_usrpri.set(newprio.clamp(0, 255) as u8);
}

/// `schedclock(p)`: we adjust the priority of the current process. The priority of a process
/// gets worse as it accumulates CPU time. The cpu usage estimator (`p_estcpu`) is increased
/// here. The formula for computing priorities will compute a different value each time
/// `p_estcpu` increases. This can cause a switch, but unless the priority crosses a PPQ
/// boundary the actual queue will not change. The cpu usage estimator ramps up quite quickly
/// when the process is running (linearly), and decays away exponentially, at a rate which is
/// proportionally slower when the system is busy. The basic principle is that the system
/// will 90% forget that the process used a lot of CPU time in 5 * loadav seconds. This
/// causes the system to favor processes which haven't run much recently, and to round-robin
/// among other processes.
pub fn schedclock<G: ProcGlue>(p: &Thread<G>, spinning: bool) {
    let n = hal::cpu::cpu_number();
    let Some(spc) = tables::<G>().cpu(n) else {
        return;
    };

    if addr_of(p) == spc.idle_addr() || spinning {
        return;
    }

    let h = sched_lock();
    let newcpu = estcpulim(p.p_estcpu.get() + 1);
    setpriority(&h, p, newcpu, p.process().ps_nice.get());
    drop(h);
}

/// `scheduler_start()`: start the scheduler's periodic timeouts.
pub fn scheduler_start<G: ProcGlue>() {
    SCHEDCPU_TO.set(
        Some(schedcpu::<G>),
        ptr::null_mut(),
        crate::timeout::KCLOCK_NONE,
        0,
    );
    LOADAVG_TO.set(
        Some(update_loadavg::<G>),
        ptr::null_mut(),
        crate::timeout::KCLOCK_NONE,
        0,
    );
    schedcpu::<G>(ptr::null_mut());
    update_loadavg::<G>(ptr::null_mut());

    // perfpolicy_dynamic() -> timeout_add_msec(&setperf_to, 200): CPU throttling (M7).
}

/// `averunnable` as the C's `struct loadavg` members: the three averages.
pub fn averunnable() -> [u32; 3] {
    [
        AVERUNNABLE[0].load(Ordering::Relaxed),
        AVERUNNABLE[1].load(Ordering::Relaxed),
        AVERUNNABLE[2].load(Ordering::Relaxed),
    ]
}

/// `thread_new`'s `timeout_set_flags(&p->p_sleep_to, endtsleep, p, KCLOCK_NONE,
/// TIMEOUT_PROC)`: the thread's sleep timeout, named by its tid (see the module's
/// deviations).
pub fn sleep_timeout_init<G: ProcGlue>(p: &Thread<G>) {
    p.p_sleep_to.set(
        Some(endtsleep::<G>),
        ptr::without_provenance_mut(p.p_tid.get() as usize),
        crate::timeout::KCLOCK_NONE,
        TIMEOUT_PROC,
    );
}

/// `tsleep_nsec(ident, priority, wmesg, nsecs)`: general sleep call. Suspends the current
/// process until a wakeup is performed on the specified identifier. The process will then be
/// made runnable with the specified priority. Sleeps at most `nsecs` (`INFSLP` means no
/// timeout). If `priority` includes the `PCATCH` flag, signals are checked before and after
/// sleeping, else signals are not checked. Returns `Ok` if awakened, `EWOULDBLOCK` if the
/// timeout expires. If `PCATCH` is set and a signal needs to be delivered, `ERESTART` is
/// returned if the current system call should be restarted if possible, and `EINTR` is
/// returned if the system call should be interrupted by the signal.
pub fn tsleep_nsec<G: ProcGlue>(
    ident: usize,
    priority: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let nowake = ptr::from_ref(&NOWAKE).addr();
    kassert!(priority & !(PRIMASK | PCATCH) == 0);
    kassert!(ident != nowake || priority & PCATCH != 0 || nsecs != INFSLP);
    kassert!(ident == nowake || nsecs != INFSLP || hal::klock::kernel_lock_held());

    if G::cold() {
        // After a panic, or during autoconfiguration, just give interrupts a chance, then just
        // return; don't run any other procs or panic below, in case this is the idle process
        // and already asleep.
        G::cold_yield();
        return Ok(());
    }

    sleep_setup::<G>(ident, priority, wmesg);
    sleep_finish::<G>(nsecs, true)
}

/// `sleep_setup(ident, prio, wmesg)`: puts the current thread on the sleep queue of
/// `ident`.
pub fn sleep_setup<G: ProcGlue>(ident: usize, prio: i32, wmesg: &'static str) {
    with_curproc::<G, _>(|p| {
        let mut prio = prio;

        #[cfg(feature = "diagnostic")]
        {
            if p.p_flag.load(Ordering::Relaxed) & crate::proc::P_CANTSLEEP != 0 {
                kpanic(format_args!(
                    "sleep: {} failed insomnia",
                    core::str::from_utf8(&p.process().comm()).unwrap_or("?")
                ));
            }
            if p.p_flag.load(Ordering::Relaxed) & P_SINTR != 0 {
                kpanic(format_args!("sleep: stale P_SINTR"));
            }
            if ident == 0 {
                kpanic(format_args!("sleep: no ident"));
            }
            if p.stat() != SONPROC {
                kpanic(format_args!("sleep: not SONPROC but {}", p.stat()));
            }
        }
        // exiting processes are not allowed to catch signals
        if p.p_flag.load(Ordering::Relaxed) & P_WEXIT != 0 {
            prio &= !PCATCH;
        }

        let mut h = sched_lock();

        // TRACEPOINT(sched, sleep, NULL): dt(4), not configured.

        p.p_wchan.set(ident);
        p.p_wmesg.set(Some(wmesg));
        p.p_slptime.set(0);
        p.p_slppri.set((prio & PRIMASK) as u8);
        p.p_flag.fetch_or(P_INSCHED, Ordering::Relaxed);
        tables::<G>().slpque[lookup(ident)].insert_tail(&mut h, p);
        if prio & PCATCH != 0 {
            p.p_flag.fetch_or(P_SINTR, Ordering::Relaxed);
        }
        p.set_stat(SSLEEP);

        drop(h);
    });
}

/// `p->p_cpu->ci_schedstate.spc_curpriority = p->p_usrpri` (`sleep_finish`, `userret`): the
/// priority the CPU `p` runs on is running at.
pub fn set_curpriority<G: ProcGlue>(p: &Thread<G>) {
    if let Some(spc) = thread_cpu(p).and_then(|n| tables::<G>().cpu(n)) {
        spc.set_curpriority(p.p_usrpri.get());
    }
}

/// `sleep_signal_check(p, after_sleep)`: the signals and stops of a `PCATCH` sleep of `p`
/// (the running thread): `EINTR` or `ERESTART` for a signal to deliver (as its action says),
/// `ERESTART` or the stop for a suspension; a stop asked before the sleep (`after_sleep`
/// false) is done here, the thread left `SSTOP` for the switch.
fn sleep_signal_check<G: ProcGlue>(p: &Thread<G>, after_sleep: bool) -> Result<(), Errno> {
    let pr = p.process();
    let mut ctx = Sigctx::default();

    if let Err(err) = sig::proc_suspend_check(p, true) {
        if err != Errno::EWOULDBLOCK {
            return Err(err);
        }

        // requested to stop
        if !after_sleep {
            let g = pr.ps_mtx.lock();
            sig::process_suspend_signal(pr, &g);

            let h = sched_lock();
            p.set_stat(SSTOP);
            drop(h);
            drop(g);
        }
    }

    let signo = sig::cursig(p, &mut ctx, true);
    if signo != 0 {
        if ctx.sig_stop {
            if !after_sleep {
                let g = pr.ps_mtx.lock();
                pr.ps_xsig.set(signo);
                // This is for stop signals delivered before sleep_setup() was called. We need
                // to do the full dance here before going to sleep.
                p.p_siglist
                    .fetch_and(!hal::sys::signal::sigmask(signo), Ordering::Relaxed);
                pr.ps_flags.fetch_or(PS_STOPPING, Ordering::Relaxed);
                let mut h = sched_lock();
                sig::process_stop(pr, P_SUSPSIG, SINGLE_SUSPEND, &g, &mut h);
                drop(h);
                p.p_flag.fetch_or(P_SUSPSIG, Ordering::Relaxed);
                sig::process_suspend_signal(pr, &g);
                let h = sched_lock();
                p.set_stat(SSTOP);
                drop(h);
                drop(g);
            }
        } else if ctx.sig_intr && !ctx.sig_ignore {
            return Err(Errno::EINTR);
        } else {
            return Err(Errno::ERESTART);
        }
    }

    Ok(())
}

/// `sleep_finish(nsecs, do_sleep)`: sleeps (or not) after `sleep_setup`; returns the
/// sleep's error.
pub fn sleep_finish<G: ProcGlue>(nsecs: u64, do_sleep: bool) -> Result<(), Errno> {
    with_curproc::<G, _>(|p| {
        let mut do_sleep = do_sleep;
        let mut error: Result<(), Errno> = Ok(());
        let mut error1: Result<(), Errno> = Ok(());

        #[cfg(feature = "diagnostic")]
        if nsecs == 0 {
            // log(LOG_WARNING, ...): the console's printf (the module's deviations).
            crate::prf::printf(format_args!(
                "sleep_finish: {}[{}]: {}: trying to sleep zero nanoseconds\n",
                core::str::from_utf8(&p.process().comm()).unwrap_or("?"),
                p.process().ps_pid,
                p.p_wmesg.get().unwrap_or("")
            ));
        }

        let sleep_to: Pin<&Timeout> = hal::pin::project(p, |t| &t.p_sleep_to);
        if nsecs != INFSLP {
            kassert!(p.p_flag.load(Ordering::Relaxed) & (P_TIMEOUT | P_TIMEOUTRAN) == 0);
            let _ = sleep_to.add_nsec(nsecs);
        }

        let mut catch = p.p_flag.load(Ordering::Relaxed) & P_SINTR != 0;
        if catch {
            error = sleep_signal_check(p.get_ref(), false);
            if error.is_err() {
                catch = false;
                do_sleep = false;
            }
        }

        let mut h = sched_lock();
        // A few checks need to happen before going to sleep:
        // - If the wakeup happens while going to sleep, p->p_wchan will be NULL. In that case
        //   unwind immediately but still check for possible signals and timeouts.
        // - If the sleep is aborted call unsleep and take us of the sleep queue.
        // - If requested to stop force a switch even if the sleep condition got cleared.
        if p.p_wchan.get() == 0 {
            do_sleep = false;
        }
        if !do_sleep {
            unsleep(&mut h, p.get_ref());
        }
        if p.stat() == SSTOP {
            do_sleep = true;
        }
        p.p_flag.fetch_and(!P_INSCHED, Ordering::Relaxed);

        if do_sleep {
            kassert!(p.stat() == SSLEEP || p.stat() == SSTOP);
            p.p_ru.ru_nvcsw.set(p.p_ru.ru_nvcsw.get() + 1);
            mi_switch(p, h);
        } else {
            kassert!(p.stat() == SONPROC || p.stat() == SSLEEP);
            p.set_stat(SONPROC);
            drop(h);
        }

        #[cfg(feature = "diagnostic")]
        if p.stat() != SONPROC {
            kpanic(format_args!("sleep_finish !SONPROC"));
        }

        set_curpriority(p.get_ref());

        // Even though this belongs to the signal handling part of sleep, we need to clear it
        // before the ktrace.
        p.p_flag.fetch_and(!P_SINTR, Ordering::Relaxed);

        // There are three situations to handle when cancelling the p_sleep_to timeout:
        //
        // 1. The timeout has not fired yet
        // 2. The timeout is running
        // 3. The timeout has run
        //
        // If timeout_del succeeds then the timeout won't run and situation 1 is dealt with.
        //
        // If timeout_del does not remove the timeout, then we're handling 2 or 3, but it
        // won't tell us which one. Instead, the P_TIMEOUTRAN flag is used to figure out when
        // we move from 2 to 3. endtsleep() (the p_sleep_to handler) sets the flag when it's
        // finished running, so we spin waiting for it.
        //
        // We spin instead of sleeping because endtsleep() takes the sched lock to do all it's
        // work. If we wanted to go to sleep to wait for endtsleep to run, we'd also have to
        // take the sched lock, so we'd be spinning against it anyway.
        if nsecs != INFSLP && !sleep_to.del() {
            // Wait for endtsleep timeout to finish running
            let mut flag;
            loop {
                flag = p.p_flag.load(Ordering::Relaxed);
                if flag & P_TIMEOUTRAN != 0 {
                    break;
                }
                core::hint::spin_loop(); // CPU_BUSY_CYCLE()
            }
            p.p_flag
                .fetch_and(!(P_TIMEOUT | P_TIMEOUTRAN), Ordering::Relaxed);

            if flag & P_TIMEOUT != 0 {
                error1 = Err(Errno::EWOULDBLOCK);
            }
        }

        // Check if thread was woken up because of a unwind or signal but ignore any pending
        // stop condition.
        if catch {
            error = sleep_signal_check(p.get_ref(), true);
        }

        // Signal errors are higher priority than timeouts.
        if error.is_ok() && error1.is_err() {
            error = error1;
        }

        error
    })
}

/// `wakeup_proc(p)`: if process hasn't been awakened (wchan non-zero), undo the sleep. If
/// proc is stopped, just unsleep so it will remain stopped.
pub fn wakeup_proc<G: ProcGlue>(h: &mut SchedHold, p: Pin<&Thread<G>>) -> bool {
    let mut awakened = false;

    sched_assert_locked();

    if p.p_wchan.get() != 0 {
        awakened = true;
        #[cfg(feature = "diagnostic")]
        if p.stat() != SSLEEP && p.stat() != SSTOP {
            kpanic(format_args!(
                "thread {} p_stat is {}",
                p.p_tid.get(),
                p.stat()
            ));
        }
        unsleep(h, p.get_ref());
        if p.stat() == SSLEEP {
            setrunnable(h, p);
        }
    }

    awakened
}

/// `endtsleep(arg)`: this is the timeout handler that wakes up procs that only want to
/// sleep for a period of time rather than forever (until they get a wakeup from somewhere
/// else). It is only scheduled and used by `sleep_finish()`, which coordinates with this
/// handler via the `P_TIMEOUT` and `P_TIMEOUTRAN` flags. Its argument is the thread's tid,
/// found under the kernel lock the softclock thread holds for it (the module's deviations).
pub(crate) fn endtsleep<G: ProcGlue>(arg: *mut c_void) {
    let tid = arg.addr() as i32;
    let r = hal::klock::reader();
    let Some(p) = G::tables()
        .tidhash(tid)
        .iter_pinned(&r)
        .find(|p| p.p_tid.get() == tid)
    else {
        kpanic(format_args!("endtsleep: no thread {tid}"));
    };

    let mut h = sched_lock();
    let awakened = wakeup_proc(&mut h, p);
    drop(h);

    let mut flags = P_TIMEOUTRAN;
    if awakened {
        flags |= P_TIMEOUT;
    }

    // Let sleep_finish() proceed.
    p.p_flag.fetch_or(flags, Ordering::Relaxed);
    // Do not alter the proc after this point.
    drop(r);
}

/// `unsleep(p)`: remove a process from its wait queue.
pub fn unsleep<G: ProcGlue>(h: &mut SchedHold, p: &Thread<G>) {
    sched_assert_locked();

    let wchan = p.p_wchan.get();
    if wchan != 0 {
        tables::<G>().slpque[lookup(wchan)].remove(h, p);
        p.p_wchan.set(0);
        p.p_wmesg.set(None);
        // TRACEPOINT(sched, unsleep, ...): dt(4), not configured.
    }
}

/// `wakeup_n(ident, n)`: make a number of processes sleeping on the specified identifier
/// runnable (every one for a negative `n`).
pub fn wakeup_n<G: ProcGlue>(ident: usize, n: i32) {
    let mut h = sched_lock();
    wakeup_n_locked::<G>(&mut h, ident, n);
    drop(h);
}

/// [`wakeup_n`] for a holder of `SCHED_LOCK`: `futex(2)`'s wakeup of each waiter it took off
/// a bucket, under the one hold the C's `futex_list_wakeup` takes for the batch (its
/// `wakeup_proc` of the thread asleep on the futex's own address).
pub fn wakeup_n_locked<G: ProcGlue>(h: &mut SchedHold, ident: usize, n: i32) {
    let t = tables::<G>();
    let wakeq: PinList<G::ThreadRunq> = PinList::new();
    let max = usize::try_from(n).unwrap_or(usize::MAX);

    let qp = &t.slpque[lookup(ident)];
    let _moved = qp.move_where(
        &mut *h,
        max,
        |p| {
            #[cfg(feature = "diagnostic")]
            if p.stat() != SSLEEP && p.stat() != SSTOP {
                kpanic(format_args!(
                    "thread {} p_stat is {}",
                    p.p_tid.get(),
                    p.stat()
                ));
            }
            kassert!(p.p_wchan.get() != 0);
            p.p_wchan.get() == ident
        },
        &wakeq,
    );
    // Each moved thread leaves its sleep, then goes to a run queue when it is asleep
    // (`setrunnable`, decided while it is still first on the local queue: its CPU and
    // priority, and the decay of a long sleep, which touch the thread's words alone).
    while let Some(place) = wakeq.first(&*h).map(|p| {
        p.p_wchan.set(0);
        p.p_wmesg.set(None);
        // TRACEPOINT(sched, unsleep, ...): dt(4), not configured.
        if p.stat() != SSLEEP {
            return None;
        }
        let prio = runnable_prio(&*h, p)?;
        let spc = queue_place(&*h, None, p, prio);
        runnable_after(&*h, p);
        Some((spc, prio))
    }) {
        match place {
            Some((spc, prio)) => {
                wakeq.move_first_to(&mut *h, &spc.qs[usize::from(prio >> 2)]);
                queued(spc, prio);
            }
            None => {
                wakeq.remove_first(&mut *h);
            }
        }
    }
}

/// `wakeup(chan)`: make all processes sleeping on the specified identifier runnable.
pub fn wakeup_all<G: ProcGlue>(chan: usize) {
    wakeup_n::<G>(chan, -1);
}

/// `wakeup(chan)` without a glue: through `hal::sleep`'s registration (the kernel's own
/// scheduler once it is registered), for the scheduler's own channels.
fn wakeup(chan: usize) {
    hal::sleep::wakeup(ptr::without_provenance::<u8>(chan));
}

/// `hal::sleep::Sleep` over this scheduler for the glue `G`: what the runtime's sleeping
/// locks and `hal::sleep`'s callers reach. The glue registers one `static` of it.
pub struct KernelSleep<G>(PhantomData<fn() -> G>);

impl<G> KernelSleep<G> {
    /// The implementation.
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<G> Default for KernelSleep<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: ProcGlue> hal::sleep::Sleep for KernelSleep<G> {
    fn curproc_word(&self) -> usize {
        hal::thread::current_addr()
    }

    fn sleep_setup(&self, ident: *const (), prio: i32, wmesg: &'static str) {
        sleep_setup::<G>(ident.addr(), prio, wmesg);
    }

    fn sleep_finish(&self, nsecs: u64, do_sleep: bool) -> Result<(), Errno> {
        sleep_finish::<G>(nsecs, do_sleep)
    }

    fn tsleep_nsec(
        &self,
        ident: *const (),
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        tsleep_nsec::<G>(ident.addr(), prio, wmesg, nsecs)
    }

    fn wakeup(&self, ident: *const ()) {
        wakeup_n::<G>(ident.addr(), -1);
    }

    fn wakeup_one(&self, ident: *const ()) {
        wakeup_n::<G>(ident.addr(), 1);
    }

    fn kernel_lock_held(&self) -> bool {
        hal::klock::kernel_lock_held()
    }

    fn cold(&self) -> bool {
        G::cold()
    }

    fn cold_yield(&self) {
        G::cold_yield();
    }

    fn sched_pause(&self) {
        sched_pause::<G>();
    }
}

/// The release check of the threads' pool (`procpl`, `hal::pool::BoxPool::init_checked`): a
/// thread is freed only when no CPU runs it (`hal::thread`'s E2) and it is on no queue.
pub fn thread_releasable<G: ProcGlue>(p: &Thread<G>) -> bool {
    !hal::thread::running_anywhere(p) && !p.p_runq.is_linked()
}

const _: () = assert!(
    SCHED_NQS == 32,
    "setrunqueue indexes the queues by prio >> 2"
);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    use std::boxed::Box;
    use std::sync::{Mutex, MutexGuard, Once, PoisonError};
    use std::time::Duration;
    use std::vec::Vec;

    use super::*;
    use crate::proc::{ProcTables, Process};

    /// The scheduler tests' glue: process tables of the process, the idle cycle a short
    /// sleep of the host thread.
    pub struct SchedGlue;

    crate::proc_glue_links!(SchedGlue);
    crate::test_file_glue!(SchedFiles, SchedGlue);

    pub(crate) static TABLES: ProcTables<SchedGlue> = ProcTables::new();
    static SCHED: SchedTables<SchedGlue> = SchedTables::new();

    impl ProcGlue for SchedGlue {
        type ProcessG = ();
        type ThreadG = ();
        type PgrpG = ();
        type SessionG = ();
        type FileGlue = SchedFiles;
        crate::proc_glue_link_types!();

        fn tables() -> &'static ProcTables<Self> {
            &TABLES
        }

        fn sched() -> &'static SchedTables<Self> {
            &SCHED
        }

        fn session_hangup(_sess: &crate::proc::Session<Self>) {}
        fn session_placed(_sess: &crate::proc::Session<Self>) {}
        fn session_destroy(_sess: &crate::proc::Session<Self>) {}

        fn with_curproc<R>(f: impl FnOnce(Option<&Thread<Self>>) -> R) -> R {
            hal::thread::try_with_current::<Thread<Self>, R>(|p| f(p.map(Pin::get_ref)))
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

        fn process_ref(pr: &Process<Self>) -> crate::proc::ProcessRef<Self> {
            match hal::pool::PoolArc::from_ref(pr) {
                Some(a) => crate::proc::StaticOrPooled::Pooled(a),
                None if core::ptr::eq(pr, &PR) => crate::proc::StaticOrPooled::Static(&PR),
                None => std::panic!("process_ref: not a process of the tests"),
            }
        }

        fn idle_cycle() {
            std::thread::sleep(Duration::from_micros(50));
        }
    }

    pub(crate) type T = Thread<SchedGlue>;

    /// The tests' process (process 1 of the glue).
    static PR: Process<SchedGlue> = Process::new(1, ());

    /// The virtual CPUs' numbers (high, away from the test threads' own cores' numbers).
    pub(crate) const VCPUS: [usize; 2] = [250, 251];

    /// The start words of the tests' threads.
    const START_IDLE: usize = 0;
    /// A thread that runs [`BODIES`]'s entry `arg`.
    const START_BODY: usize = 1;

    /// What a test thread started with `START_BODY` runs, by index.
    static BODIES: Mutex<Vec<fn()>> = Mutex::new(Vec::new());

    /// The tests run one at a time: one scheduler, one set of virtual CPUs.
    static SERIAL: Mutex<()> = Mutex::new(());

    /// A std mutex's guard, a panicking test's poison ignored.
    fn lock<V>(m: &Mutex<V>) -> MutexGuard<'_, V> {
        m.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The registered start: `proc_trampoline_mi`, then the idle loop or a body.
    fn test_start(hold: SchedHold, start: usize, arg: usize) {
        trampoline_mi::<SchedGlue>(hold);
        match start {
            START_IDLE => sched_idle::<SchedGlue>(arg),
            _ => {
                let f = lock(&BODIES)[arg];
                f();
                // A body ends as a dead thread does (its host thread parks once switched away).
                with_curproc::<SchedGlue, _>(|p| sched_exit(p));
            }
        }
    }

    /// A thread of the tests' process, leaked (it lives for the process), tid `tid`.
    pub(crate) fn thread(tid: i32, prio: u8) -> Pin<&'static T> {
        let t: &'static T = Box::leak(Box::new(Thread::new_static(&PR, ())));
        t.p_tid.set(tid);
        t.p_usrpri.set(prio);
        t.p_slppri.set(prio);
        hal::pin::pin_static(t)
    }

    /// The core of virtual CPU `n`.
    fn vcpu(n: usize) -> &'static hal::cpu::CpuCore {
        match hal::cpu::by_number(n) {
            Some(c) => c,
            None => panic!("vcpu {n} not attached"),
        }
    }

    /// Once per process: the doubles, the start, the virtual CPUs with their scheduler state
    /// and idle threads (each waiting for its first switch).
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        let g = lock(&SERIAL);
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            crate::testutil::install();
            // The tid hash `endtsleep` looks its thread up in.
            hal::klock::kernel_lock();
            crate::proc::procinit::<SchedGlue>(64, 64);
            hal::klock::kernel_unlock();
            let _ = hal::thread::register_start(test_start);
            for (i, &n) in VCPUS.iter().enumerate() {
                let core: &'static hal::cpu::CpuCore =
                    Box::leak(Box::new(hal::cpu::CpuCore::new()));
                hal::cpu::attach(core, n);
                sched_init_cpu::<SchedGlue>(n);
                let idle = thread(900 + i as i32, 127);
                set_idleproc(n, idle);
                let h = sched_lock();
                hal::thread::thread_restart(&h, idle, START_IDLE, n);
                drop(h);
                SCHED.all_cpus.add(n);
            }
        });
        g
    }

    /// The calling host thread becomes `p` running on virtual CPU `n`, pegged there
    /// (`P_CPUPEG`): woken, it resumes on `n`, so a test ends on the vCPU it started on and the
    /// next test's thread takes that vCPU over from a host thread that is gone.
    pub(crate) fn run_as(n: usize, p: Pin<&'static T>) {
        hal::cpu::set_current(vcpu(n));
        p.p_flag.fetch_or(P_CPUPEG, Ordering::Relaxed);
        p.set_stat(SONPROC);
        set_thread_cpu(p.get_ref(), n);
        hal::thread::set_current_static(Some(p));
    }

    #[test]
    fn the_cpu_sets_count_and_pick_the_lowest() {
        let s = Cpuset::new();
        assert_eq!(s.first(), None);
        s.add(40);
        s.add(3);
        assert!(s.isset(3) && s.isset(40) && !s.isset(4));
        assert_eq!(s.first(), Some(3));
        assert_eq!(s.cardinality(), 2);
        let all = s.copy();
        s.del(3);
        assert_eq!(Cpuset::complement(&s, &all).first(), Some(3));
        assert_eq!(Cpuset::intersection(&s, &all).first(), Some(40));
    }

    #[test]
    fn log2_rounds_down_and_maps_zero_to_zero() {
        assert_eq!(log2(0), 0);
        assert_eq!(log2(1), 0);
        assert_eq!(log2(2), 1);
        assert_eq!(log2(3), 1);
        assert_eq!(log2(1024), 10);
        assert_eq!(log2(u32::MAX), 31);
    }

    /// A busy, non-primary CPU running priority 50 with nothing queued.
    const BUSY: CpuCostInputs = CpuCostInputs {
        idle: false,
        queued: false,
        primary: false,
        usrpri: 50,
        curpriority: 50,
        nrun: 0,
        resident: None,
    };

    #[test]
    fn the_cost_of_a_move_is_the_c_s_guesstimate() {
        assert_eq!(cpu_cost(CpuCostInputs { idle: true, ..BUSY }), 0);
        assert_eq!(cpu_cost(BUSY), 3);
        assert_eq!(cpu_cost(CpuCostInputs { usrpri: 60, ..BUSY }), 13);
        assert_eq!(cpu_cost(CpuCostInputs { usrpri: 40, ..BUSY }), -7);
        let queued = CpuCostInputs {
            queued: true,
            nrun: 2,
            ..BUSY
        };
        assert_eq!(cpu_cost(queued), 3 + 2 * 3);
        assert_eq!(
            cpu_cost(CpuCostInputs {
                primary: true,
                ..BUSY
            }),
            3 + 3
        );
        let warm = CpuCostInputs {
            resident: Some(256),
            ..BUSY
        };
        assert_eq!(cpu_cost(warm), 3 - 8);
    }

    #[test]
    fn priorities_follow_the_estimator_and_nice() {
        let _g = setup();
        let p = thread(10, 50);
        let h = sched_lock();
        setpriority(&h, p.get_ref(), 0, NZERO as u8);
        assert_eq!(p.p_usrpri.get(), PUSER as u8);
        setpriority(&h, p.get_ref(), 10, NZERO as u8 + 5);
        assert_eq!(p.p_usrpri.get(), (PUSER + 10 + NICE_WEIGHT * 5) as u8);
        setpriority(&h, p.get_ref(), 500, 40);
        assert_eq!(p.p_usrpri.get(), MAXPRI as u8);
        drop(h);
        assert_eq!(estcpulim(1000), NICE_WEIGHT * PRIO_MAX - SCHED_PPQ);
        // No load: a long sleep forgets everything, a short one decays.
        AVERUNNABLE[0].store(FSCALE, Ordering::Relaxed);
        assert_eq!(decay_aftersleep(100, 1000), 0);
        assert!(decay_aftersleep(100, 3) < 100);
    }

    /// The run queues: a thread goes on its priority's queue, `whichqs` says which are full,
    /// the best queue's head is chosen first and leaves its queue.
    #[test]
    fn the_run_queues_hand_out_the_best_priority_first() {
        let _g = setup();
        let n = VCPUS[0];
        let me = thread(20, 50);
        run_as(n, me);
        let a = thread(21, 80);
        let b = thread(22, 20);
        let c = thread(23, 21);
        let spc = cpu_of::<SchedGlue>(n);
        let mut h = sched_lock();
        for p in [a, b, c] {
            setrunqueue(&mut h, Some(n), p, p.p_usrpri.get());
        }
        assert_eq!(spc.nrun(), 3);
        assert_eq!(spc.whichqs(), (1 << (80 >> 2)) | (1 << (20 >> 2)));
        assert!(SCHED.queued_cpus.isset(n));
        let next = sched_chooseproc::<SchedGlue>(&mut h);
        assert_eq!(next.thread_addr(), addr_of(b.get_ref()));
        let next = sched_chooseproc::<SchedGlue>(&mut h);
        assert_eq!(next.thread_addr(), addr_of(c.get_ref()));
        remrunqueue(&mut h, a.get_ref());
        assert_eq!(spc.nrun(), 0);
        assert_eq!(spc.whichqs(), 0);
        assert!(!SCHED.queued_cpus.isset(n));
        // Nothing left: the idle thread.
        let next = sched_chooseproc::<SchedGlue>(&mut h);
        assert_eq!(next.thread_addr(), spc.idle_addr());
        drop(h);
        hal::thread::set_current_static::<T>(None);
    }

    /// A wakeup between `sleep_setup` and `sleep_finish` leaves the thread on its CPU: the
    /// sleep returns without a switch (`p_wchan` is 0 at `sleep_finish`).
    #[test]
    fn a_wakeup_before_the_sleep_needs_no_switch() {
        let _g = setup();
        let n = VCPUS[0];
        let me = thread(30, 50);
        run_as(n, me);
        let chan = 0x1000_usize;
        sleep_setup::<SchedGlue>(chan, PWAIT, "testslp");
        assert_eq!(me.stat(), SSLEEP);
        assert_eq!(me.p_wchan.get(), chan);
        assert_eq!(me.p_wmesg.get(), Some("testslp"));
        wakeup_n::<SchedGlue>(chan, 1);
        // Taken off the sleep queue; still in the switch's window (P_INSCHED), not queued.
        assert_eq!(me.p_wchan.get(), 0);
        assert!(!me.p_runq.is_linked());
        assert_eq!(sleep_finish::<SchedGlue>(INFSLP, true), Ok(()));
        assert_eq!(me.stat(), SONPROC);
        assert_eq!(me.p_flag.load(Ordering::Relaxed) & P_INSCHED, 0);
        // `do_sleep` false takes the thread off its queue itself.
        sleep_setup::<SchedGlue>(chan, PWAIT, "testslp");
        assert!(me.p_runq.is_linked());
        assert_eq!(sleep_finish::<SchedGlue>(INFSLP, false), Ok(()));
        assert!(!me.p_runq.is_linked());
        assert_eq!(me.p_wchan.get(), 0);
        hal::thread::set_current_static::<T>(None);
    }

    /// `wakeup_n(1)` wakes one of three sleepers, `wakeup` the others; each goes to a run
    /// queue at its sleep priority.
    #[test]
    fn wakeup_n_moves_its_sleepers_to_the_run_queues() {
        let _g = setup();
        let n = VCPUS[1];
        let sleepers: Vec<_> = (0..3).map(|i| thread(40 + i, 60)).collect();
        let chan = 0x2000_usize;
        for &p in &sleepers {
            let mut h = sched_lock();
            p.p_wchan.set(chan);
            p.p_slppri.set(30);
            p.set_stat(SSLEEP);
            set_thread_cpu(p.get_ref(), n);
            SCHED.slpque[lookup(chan)].insert_tail(&mut h, p);
            drop(h);
        }
        wakeup_n::<SchedGlue>(chan, 1);
        let woken = sleepers.iter().filter(|p| p.stat() == SRUN).count();
        assert_eq!(woken, 1);
        assert_eq!(sleepers[0].stat(), SRUN);
        assert_eq!(sleepers[0].p_runpri.get(), 30);
        wakeup_n::<SchedGlue>(chan, -1);
        assert!(
            sleepers
                .iter()
                .all(|p| p.stat() == SRUN && p.p_wchan.get() == 0)
        );
        let mut h = sched_lock();
        for &p in &sleepers {
            remrunqueue(&mut h, p.get_ref());
        }
        drop(h);
    }

    /// A real switch: the running thread yields to a runnable one on the same CPU, which runs
    /// and yields back.
    #[test]
    fn yield_switches_to_a_runnable_thread_and_back() {
        let _g = setup();
        let n = VCPUS[0];
        static RAN: AtomicU32 = AtomicU32::new(0);
        fn body() {
            RAN.fetch_add(1, Ordering::Relaxed);
            r#yield::<SchedGlue>();
            RAN.fetch_add(1, Ordering::Relaxed);
        }
        let idx = {
            let mut b = lock(&BODIES);
            b.push(body);
            b.len() - 1
        };
        let me = thread(50, 50);
        run_as(n, me);
        let other = thread(51, 50);
        let mut h = sched_lock();
        hal::thread::thread_setup(&h, me, other, 0, 0, START_BODY, idx);
        setrunqueue(&mut h, Some(n), other, 50);
        drop(h);
        let before = RAN.load(Ordering::Relaxed);
        r#yield::<SchedGlue>();
        // Back here: the other thread ran once and yielded to this one.
        assert_eq!(RAN.load(Ordering::Relaxed), before + 1);
        assert_eq!(me.stat(), SONPROC);
        r#yield::<SchedGlue>();
        // The other thread finished its body and exited; this one runs again.
        assert_eq!(RAN.load(Ordering::Relaxed), before + 2);
        hal::thread::set_current_static::<T>(None);
    }

    /// A sleeper on one CPU is woken from another and resumed by its CPU's idle thread.
    #[test]
    fn a_sleeper_is_woken_from_another_cpu() {
        let _g = setup();
        let [a, b] = VCPUS;
        let me = thread(60, 50);
        run_as(a, me);
        let chan = ptr::from_ref(&NOWAKE).addr() + 0x100;
        let waker = std::thread::spawn(move || {
            let w = thread(61, 50);
            run_as(b, w);
            std::thread::sleep(Duration::from_millis(20));
            wakeup_n::<SchedGlue>(chan, -1);
            hal::thread::set_current_static::<T>(None);
        });
        hal::klock::kernel_lock();
        let r = tsleep_nsec::<SchedGlue>(chan, PWAIT, "xcpu", INFSLP);
        hal::klock::kernel_unlock();
        assert_eq!(r, Ok(()));
        assert_eq!(me.stat(), SONPROC);
        assert_eq!(me.p_wchan.get(), 0);
        waker.join().unwrap();
        hal::thread::set_current_static::<T>(None);
    }

    /// The load average's and `p_pctcpu`'s constants are the C's doubles in `FSCALE` fixed
    /// point (LZ's sched_bsd.rs test, carried with the code).
    #[test]
    fn decay_constants_match_the_c_doubles() {
        // 0.9200444146293232 * 2048, 0.9834714538216174 * 2048, 0.9944598480048967 * 2048
        assert_eq!(CEXP, [1884, 2014, 2036]);
        // 0.95122942450071400909 * 2048
        assert_eq!(CCPU, 1948);
    }

    /// `decay_cpu` forgets 90 % of `p_estcpu` in about 5 * loadav decays (LZ's sched_bsd.rs
    /// test, carried with the code).
    #[test]
    fn decay_cpu_forgets_ninety_percent_in_five_loadav() {
        // loadav 1.0: loadfac 2*FSCALE; after 5 decays 100 -> about 10.
        let loadfac = loadfactor(FSCALE);
        let mut cpu = 100;
        for _ in 0..5 {
            cpu = decay_cpu(loadfac, cpu);
        }
        assert!((8..=16).contains(&cpu), "{cpu}");
    }

    /// A halting CPU (`SPCF_SHOULDHALT`, out of `sched_all_cpus` as
    /// `sched_stop_secondary_cpus` leaves it) hands its queued threads to the other CPUs; a
    /// thread pegged to it stays, moved to its queue's tail, and the C's `goto again` picks
    /// from what is left: the pegged thread once nothing else is queued here.
    #[test]
    fn a_halting_cpu_keeps_its_pegged_thread() {
        let _g = setup();
        let (n, other) = (VCPUS[0], VCPUS[1]);
        let me = thread(40, 50);
        run_as(n, me);
        let spc = cpu_of::<SchedGlue>(n);
        let dst = cpu_of::<SchedGlue>(other);
        SCHED.all_cpus.del(n);
        spc.set_schedflags(SPCF_SHOULDHALT);

        // An unpegged thread, then a pegged one, on one queue: the first goes to the other
        // CPU, the pegged one stays and is chosen.
        let free = thread(41, 60);
        let pegged = thread(42, 60);
        pegged.p_flag.fetch_or(P_CPUPEG, Ordering::Relaxed);
        let mut h = sched_lock();
        setrunqueue(&mut h, Some(n), free, 60);
        setrunqueue(&mut h, Some(n), pegged, 60);
        let next = sched_chooseproc::<SchedGlue>(&mut h);
        assert_eq!(next.thread_addr(), addr_of(pegged.get_ref()));
        assert_eq!(thread_cpu(free.get_ref()), Some(other));
        assert_eq!(dst.nrun(), 1);
        assert_eq!(spc.nrun(), 0);
        assert_eq!(spc.whichqs(), 0);
        remrunqueue(&mut h, free.get_ref());

        // The pegged thread first: it goes to the queue's tail (the same list, no panic), and
        // `again` takes the head, the thread behind it, as the C does.
        let behind = thread(43, 60);
        let pegged2 = thread(44, 60);
        pegged2.p_flag.fetch_or(P_CPUPEG, Ordering::Relaxed);
        setrunqueue(&mut h, Some(n), pegged2, 60);
        setrunqueue(&mut h, Some(n), behind, 60);
        let next = sched_chooseproc::<SchedGlue>(&mut h);
        assert_eq!(thread_cpu(pegged2.get_ref()), Some(n));
        assert_eq!(next.thread_addr(), addr_of(behind.get_ref()));
        assert!(pegged2.p_runq.is_linked());
        for p in [behind, pegged2] {
            if p.p_runq.is_linked() {
                remrunqueue(&mut h, p.get_ref());
            }
        }
        drop(h);

        spc.clear_schedflags(SPCF_SHOULDHALT);
        SCHED.all_cpus.add(n);
        hal::thread::set_current_static::<T>(None);
    }

    /// Two barriers on one CPU: the later caller's pass satisfies the earlier one's mark, but
    /// the earlier caller still waits while its own task is queued (its frame keeps it), and
    /// goes once the task left the queue; tickets and the task's argument round-trip.
    #[test]
    fn two_barriers_each_wait_for_their_own_task() {
        static TQ: crate::task::Taskq = crate::task::Taskq::new(b"sbartest", 1, 0, 0);
        let _g = setup();
        let n = VCPUS[1];
        let a = BARRIER_TICKET[n].fetch_add(1, Ordering::AcqRel) + 1;
        let b = BARRIER_TICKET[n].fetch_add(1, Ordering::AcqRel) + 1;
        assert!(b > a);
        assert_eq!(barrier_unpack(barrier_pack(n, b)), (n, b));
        let ta = core::pin::pin!(crate::task::Task::new(|_| {}, ptr::null_mut()));
        let tb = core::pin::pin!(crate::task::Task::new(|_| {}, ptr::null_mut()));
        let _ = crate::task::task_add(&TQ, ta.as_ref());
        let _ = crate::task::task_add(&TQ, tb.as_ref());
        assert!(barrier_waiting(n, a, ta.as_ref().get_ref()));
        // B's task runs first: its pass marks the CPU past A's ticket.
        let _ = crate::task::task_del(&TQ, tb.as_ref().get_ref());
        BARRIER_DONE[n].fetch_max(b, Ordering::Release);
        assert!(!barrier_waiting(n, b, tb.as_ref().get_ref()));
        // A's mark is met, but its task is still queued: A waits (kern_sched.c's own cond).
        assert!(barrier_waiting(n, a, ta.as_ref().get_ref()));
        let _ = crate::task::task_del(&TQ, ta.as_ref().get_ref());
        assert!(!barrier_waiting(n, a, ta.as_ref().get_ref()));
    }

    /// A timed sleep that times out: the timeout's handler (`endtsleep`, found by tid under the
    /// kernel lock) wakes the sleeper from another vCPU, and `sleep_finish`, finding the
    /// timeout gone from the wheel, waits for `P_TIMEOUTRAN` and returns `EWOULDBLOCK`.
    #[test]
    fn a_timed_sleep_times_out_through_endtsleep() {
        let _g = setup();
        let [a, b] = VCPUS;
        let me = thread(70, 50);
        sleep_timeout_init(me.get_ref());
        run_as(a, me);
        {
            hal::klock::kernel_lock();
            let mut w = hal::klock::witness();
            TABLES.tidhash(70).insert_head(&mut w, me);
            drop(w);
            hal::klock::kernel_unlock();
        }
        let chan = ptr::from_ref(&NOWAKE).addr() + 0x300;
        let timer = std::thread::spawn(move || {
            let w = thread(71, 50);
            run_as(b, w);
            // Asleep: queued on the channel and out of `sleep_finish`'s window.
            while !(me.stat() == SSLEEP
                && me.p_wchan.get() == chan
                && me.p_flag.load(Ordering::Relaxed) & P_INSCHED == 0)
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            // The softclock thread took the timeout off the wheel and runs its handler.
            assert!(hal::pin::project(me, |t| &t.p_sleep_to).del());
            hal::klock::kernel_lock();
            endtsleep::<SchedGlue>(ptr::without_provenance_mut(70));
            hal::klock::kernel_unlock();
            hal::thread::set_current_static::<T>(None);
        });
        hal::klock::kernel_lock();
        let r = tsleep_nsec::<SchedGlue>(chan, PWAIT, "tmo", 3_600_000_000_000);
        hal::klock::kernel_unlock();
        assert_eq!(r, Err(Errno::EWOULDBLOCK));
        assert_eq!(
            me.p_flag.load(Ordering::Relaxed) & (P_TIMEOUT | P_TIMEOUTRAN),
            0
        );
        assert_eq!(me.p_wchan.get(), 0);
        timer.join().unwrap();
        hal::klock::kernel_lock();
        let mut w = hal::klock::witness();
        TABLES.tidhash(70).remove(&mut w, me.get_ref());
        drop(w);
        hal::klock::kernel_unlock();
        hal::thread::set_current_static::<T>(None);
    }

    /// A wakeup from another vCPU between `sleep_setup` and `sleep_finish`: the thread, still
    /// in `P_INSCHED`, leaves the sleep queue but is not queued to run, and `sleep_finish`
    /// returns at once (`p_wchan` is 0).
    #[test]
    fn a_wakeup_from_another_cpu_between_setup_and_finish() {
        let _g = setup();
        let [a, b] = VCPUS;
        let me = thread(72, 50);
        run_as(a, me);
        let chan = ptr::from_ref(&NOWAKE).addr() + 0x400;
        sleep_setup::<SchedGlue>(chan, PWAIT, "race");
        let waker = std::thread::spawn(move || {
            let w = thread(73, 50);
            run_as(b, w);
            wakeup_n::<SchedGlue>(chan, -1);
            hal::thread::set_current_static::<T>(None);
        });
        waker.join().unwrap();
        assert_eq!(me.p_wchan.get(), 0);
        assert_eq!(me.stat(), SSLEEP);
        assert!(!me.p_runq.is_linked());
        assert_eq!(sleep_finish::<SchedGlue>(INFSLP, true), Ok(()));
        assert_eq!(me.stat(), SONPROC);
        hal::thread::set_current_static::<T>(None);
    }

    /// `setrunnable` of a stopped thread: on a run queue at its user priority when it is out of
    /// the switch; back to its sleep (`SSLEEP`) or running (`SONPROC`) when it is still in it
    /// (`P_INSCHED`), not queued.
    #[test]
    fn setrunnable_of_a_stopped_thread() {
        let _g = setup();
        let n = VCPUS[0];
        let me = thread(74, 50);
        run_as(n, me);
        let stopped = thread(75, 40);
        stopped.set_stat(SSTOP);
        set_thread_cpu(stopped.get_ref(), n);
        let mut h = sched_lock();
        setrunnable(&mut h, stopped);
        assert_eq!(stopped.stat(), SRUN);
        assert_eq!(stopped.p_runpri.get(), 40);
        assert!(stopped.p_runq.is_linked());
        remrunqueue(&mut h, stopped.get_ref());

        let insched = thread(76, 40);
        insched.set_stat(SSTOP);
        insched.p_flag.fetch_or(P_INSCHED, Ordering::Relaxed);
        setrunnable(&mut h, insched);
        assert_eq!(insched.stat(), SONPROC);
        assert!(!insched.p_runq.is_linked());
        insched.set_stat(SSTOP);
        insched.p_wchan.set(0x5000);
        setrunnable(&mut h, insched);
        assert_eq!(insched.stat(), SSLEEP);
        assert!(!insched.p_runq.is_linked());
        drop(h);
        hal::thread::set_current_static::<T>(None);
    }

    /// The kthread ping-pong of the selftest on two vCPUs: two threads, one per vCPU, hand a
    /// turn to each other `ROUNDS` times through sleep and wakeup, while the vCPUs' first
    /// threads wait for both to finish.
    #[test]
    fn a_ping_pong_across_two_cpus() {
        const ROUNDS: u32 = 40;
        static TURN: AtomicU32 = AtomicU32::new(0);
        static PINGS: AtomicU32 = AtomicU32::new(0);
        static FINISHED: AtomicU32 = AtomicU32::new(0);
        fn chan(i: u32) -> usize {
            ptr::from_ref(&TURN).addr() + 0x500 + i as usize * 0x100
        }
        fn done_chan() -> usize {
            ptr::from_ref(&FINISHED).addr() + 0x800
        }
        fn player(mine: u32) {
            for _ in 0..ROUNDS {
                loop {
                    sleep_setup::<SchedGlue>(chan(mine), PWAIT, "ping");
                    let turn = TURN.load(Ordering::Acquire) == mine;
                    let _ = sleep_finish::<SchedGlue>(INFSLP, !turn);
                    if turn {
                        break;
                    }
                }
                PINGS.fetch_add(1, Ordering::Relaxed);
                TURN.store(1 - mine, Ordering::Release);
                wakeup_n::<SchedGlue>(chan(1 - mine), -1);
            }
            FINISHED.fetch_add(1, Ordering::Release);
            wakeup_n::<SchedGlue>(done_chan(), -1);
        }
        fn wait_done() {
            loop {
                sleep_setup::<SchedGlue>(done_chan(), PWAIT, "pdone");
                let done = FINISHED.load(Ordering::Acquire) == 2;
                let _ = sleep_finish::<SchedGlue>(INFSLP, !done);
                if done {
                    break;
                }
            }
        }
        fn ping() {
            player(0);
        }
        fn pong() {
            player(1);
        }
        let _g = setup();
        let [a, b] = VCPUS;
        let idx = {
            let mut bodies = lock(&BODIES);
            bodies.push(ping);
            bodies.push(pong);
            bodies.len() - 2
        };
        let me = thread(80, 50);
        run_as(a, me);
        let other = std::thread::spawn(move || {
            let w = thread(81, 50);
            run_as(b, w);
            wait_done();
            hal::thread::set_current_static::<T>(None);
        });
        let p0 = thread(82, 50);
        let p1 = thread(83, 50);
        let mut h = sched_lock();
        hal::thread::thread_setup(&h, me, p0, 0, 0, START_BODY, idx);
        hal::thread::thread_setup(&h, me, p1, 0, 0, START_BODY, idx + 1);
        setrunqueue(&mut h, Some(a), p0, 50);
        setrunqueue(&mut h, Some(b), p1, 50);
        drop(h);
        wait_done();
        other.join().unwrap();
        assert_eq!(PINGS.load(Ordering::Relaxed), 2 * ROUNDS);
        hal::thread::set_current_static::<T>(None);
    }

    /// An unpegged sleeper woken from another vCPU goes where `sched_choosecpu` sends it (the
    /// run_as threads are pegged; this one is not) and runs there.
    #[test]
    fn an_unpegged_sleeper_is_placed_by_choosecpu() {
        static ASLEEP: AtomicU32 = AtomicU32::new(0);
        static DONE: AtomicU32 = AtomicU32::new(0);
        fn chan() -> usize {
            ptr::from_ref(&ASLEEP).addr() + 0x900
        }
        fn done_chan() -> usize {
            ptr::from_ref(&DONE).addr() + 0xa00
        }
        fn sleeper() {
            loop {
                sleep_setup::<SchedGlue>(chan(), PWAIT, "unpeg");
                let _ = ASLEEP.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
                let go = ASLEEP.load(Ordering::Acquire) == 2;
                let _ = sleep_finish::<SchedGlue>(INFSLP, !go);
                if go {
                    break;
                }
            }
            DONE.store(1, Ordering::Release);
            wakeup_n::<SchedGlue>(done_chan(), -1);
        }
        fn wait_done() {
            loop {
                sleep_setup::<SchedGlue>(done_chan(), PWAIT, "udone");
                let done = DONE.load(Ordering::Acquire) == 1;
                let _ = sleep_finish::<SchedGlue>(INFSLP, !done);
                if done {
                    break;
                }
            }
        }
        let _g = setup();
        let [a, b] = VCPUS;
        let idx = {
            let mut bodies = lock(&BODIES);
            bodies.push(sleeper);
            bodies.len() - 1
        };
        let me = thread(90, 50);
        run_as(a, me);
        let helper = std::thread::spawn(move || {
            let w = thread(91, 50);
            run_as(b, w);
            wait_done();
            hal::thread::set_current_static::<T>(None);
        });
        let x = thread(92, 50);
        let mut h = sched_lock();
        hal::thread::thread_setup(&h, me, x, 0, 0, START_BODY, idx);
        setrunqueue(&mut h, Some(a), x, 50);
        drop(h);
        // Let it run (this thread yields to it) until it sleeps.
        // Asleep: on its channel and out of `sleep_finish`'s window (not `P_INSCHED`).
        while ASLEEP.load(Ordering::Acquire) != 1
            || x.stat() != SSLEEP
            || x.p_flag.load(Ordering::Relaxed) & P_INSCHED != 0
        {
            r#yield::<SchedGlue>();
        }
        assert_eq!(x.p_flag.load(Ordering::Relaxed) & P_CPUPEG, 0);
        CHOSE_FOR.with(|c| c.set(0));
        ASLEEP.store(2, Ordering::Release);
        wakeup_n::<SchedGlue>(chan(), -1);
        // The wakeup's setrunnable (this host thread's) went through sched_choosecpu for `x`
        // (no CPU named).
        assert_eq!(CHOSE_FOR.with(core::cell::Cell::get), addr_of(x.get_ref()));
        let placed = thread_cpu(x.get_ref());
        assert!(placed == Some(a) || placed == Some(b));
        wait_done();
        helper.join().unwrap();
        hal::thread::set_current_static::<T>(None);
    }

    /// The kernel's glue lends the thread hal names running (`with_curproc`, what the kqueue's
    /// `filt_procattach` and `filt_sigattach` read), and none once it is cleared.
    #[test]
    fn the_glue_lends_the_running_thread() {
        let _g = setup();
        let me = thread(93, 50);
        run_as(VCPUS[0], me);
        let seen = SchedGlue::with_curproc(|p| p.map(addr_of));
        assert_eq!(seen, Some(addr_of(me.get_ref())));
        hal::thread::set_current_static::<T>(None);
        assert_eq!(SchedGlue::with_curproc(|p| p.map(addr_of)), None);
    }
}
/* </TESTS> */
