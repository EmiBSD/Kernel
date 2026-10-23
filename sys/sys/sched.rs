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
/* </LICENSES> */

/* <CODE> */
//! `<sys/sched.h>`: the per-CPU scheduler state and the CPU-state statistics.
//!
//! Upstream: sys/sys/sched.h @ 3ce1f3f79392
//!
//! Status: `ported`. Milestone M5 (part a, the clocks) ports the `CP_*` states, `struct
//! cpustats`, the `SPCF_*` flags, `SCHED_NQS`/`SCHED_PPQ`/`NICE_WEIGHT`/`ESTCPULIM` and the
//! members of `struct schedstate_percpu` the clock interrupts use: the four clockintr
//! handles, `spc_cp_time` with its lock, `spc_schedticks`, `spc_spinning`, `spc_runtime`;
//! M11a the SMR members (`spc_deferred`, `spc_ndeferred`, `spc_smrdepth`, `spc_smrexpedite`,
//! `spc_smrgp`, used by `kern/kern_smr.rs`) and `cpu_is_idle`. N4.5a: the scheduler's members
//! (`spc_qs`, `spc_whichqs`, `spc_nrun`, `spc_idleproc`, `spc_deadproc`, `spc_schedflags`,
//! `spc_curpriority`) are `kernel::sched::SchedCpu`'s, reached by the CPU's number; the
//! functions it declares are `kernel::sched`'s, under their names in `kern/kern_sched.rs` and
//! `kern/sched_bsd.rs`.
//!
//! ## Deviations
//! - `spc_spinning` (`volatile`) is an `AtomicU32`, `spc_smrgp` (read by other CPUs,
//!   `READ_ONCE` in C) an `AtomicU8`.
//! - `spc_cp_time` (written by the CPU's `statclock`, read by sysctl under the `pc_lock`
//!   generation protocol) holds `AtomicU64`s; relaxed accesses compile to the C's plain loads
//!   and stores.

use core::cell::Cell;
use core::sync::atomic::{AtomicU8, AtomicU32, AtomicU64};

use crate::sys::pclock::PcLock;
use crate::sys::proc::Proc;
use crate::sys::queue::SimpleqHead;
use crate::sys::smr::SmrEntryList;
use crate::sys::time::Timespec;

/*
 * CPU states.
 * XXX Not really scheduler state, but no other good place to put
 * it right now, and it really is per-CPU.
 */

/// `CP_USER`.
pub const CP_USER: usize = 0;
/// `CP_NICE`.
pub const CP_NICE: usize = 1;
/// `CP_SYS`.
pub const CP_SYS: usize = 2;
/// `CP_SPIN`.
pub const CP_SPIN: usize = 3;
/// `CP_INTR`.
pub const CP_INTR: usize = 4;
/// `CP_IDLE`.
pub const CP_IDLE: usize = 5;
/// `CPUSTATES`.
pub const CPUSTATES: usize = 6;

/// `struct cpustats`: what `kern.cpustats` copies out (`#[repr(C)]`: ABI).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Cpustats {
    /// `cs_time`: CPU state statistics.
    pub cs_time: [u64; CPUSTATES],
    /// `cs_flags`: see below.
    pub cs_flags: u64,
}

/// `CPUSTATS_ONLINE`: CPU is schedulable.
pub const CPUSTATS_ONLINE: u64 = 0x0001;

/// `SCHED_NQS`: 32 run queues.
pub const SCHED_NQS: usize = 32;

/// `struct schedstate_percpu`: per-CPU scheduler state, the clock's and SMR's members (the
/// scheduler's are `kernel::sched::SchedCpu`'s).
///
/// - o: owned (modified only) by this CPU.
pub struct SchedstatePercpu {
    /// \[o\] `spc_runtime`: time curproc started running.
    pub spc_runtime: Cell<Timespec>,
    /// \[o\] `spc_schedticks`: ticks for `schedclock()`.
    pub spc_schedticks: Cell<u32>,
    /// `spc_cp_time_lock`.
    pub spc_cp_time_lock: PcLock,
    /// `spc_cp_time`: CPU state statistics, written by this CPU under `spc_cp_time_lock`.
    pub spc_cp_time: [AtomicU64; CPUSTATES],

    // `spc_itimer`, `spc_profclock`, `spc_roundrobin`, `spc_statclock`: the CPU's slots of
    // `kernel::clockintr`'s queue (`Clockintr::of(cpu, Slot::...)`, N4.8a).
    /// `spc_spinning` (volatile): this cpu is currently spinning.
    pub spc_spinning: AtomicU32,

    /// \[o\] `spc_deferred`: deferred smr calls, changed at `splhigh`.
    pub spc_deferred: SimpleqHead<SmrEntryList>,
    /// \[o\] `spc_ndeferred`: number of deferred smr calls.
    pub spc_ndeferred: Cell<u32>,
    /// \[o\] `spc_smrdepth`: level of smr nesting (`DIAGNOSTIC`).
    pub spc_smrdepth: Cell<u32>,
    /// \[o\] `spc_smrexpedite`: if set, dispatch smr entries without delay.
    pub spc_smrexpedite: Cell<u8>,
    /// `spc_smrgp`: this CPU's view of grace period (read by `smr_grace_wait` elsewhere).
    pub spc_smrgp: AtomicU8,
}

// SAFETY: one CPU's clock and SMR state. The `Cell` members are owned by that CPU ([o]: the
// clock handles under their queue's mutex, `spc_runtime` and `spc_schedticks` by its own
// clock and switch, the SMR queue at splhigh). Everything other CPUs read without a lock is
// atomic.
unsafe impl Sync for SchedstatePercpu {}

impl SchedstatePercpu {
    /// A CPU's scheduler state before `sched_init_cpu`: all zero, as the C's static.
    pub const fn new() -> Self {
        Self {
            spc_runtime: Cell::new(Timespec::new(0, 0)),
            spc_schedticks: Cell::new(0),
            spc_cp_time_lock: PcLock::new(),
            spc_cp_time: [const { AtomicU64::new(0) }; CPUSTATES],

            spc_spinning: AtomicU32::new(0),
            spc_deferred: SimpleqHead::new(),
            spc_ndeferred: Cell::new(0),
            spc_smrdepth: Cell::new(0),
            spc_smrexpedite: Cell::new(0),
            spc_smrgp: AtomicU8::new(0),
        }
    }
}

impl Default for SchedstatePercpu {
    fn default() -> Self {
        Self::new()
    }
}

/* spc_flags */

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

/// `SCHED_PPQ`: priorities per queue.
pub const SCHED_PPQ: u32 = 128 / SCHED_NQS as u32;
/// `NICE_WEIGHT`: priorities per nice level.
pub const NICE_WEIGHT: u32 = 2;
/// `PRIO_MAX` (`<sys/resource.h>`): the largest nice value.
const PRIO_MAX: u32 = 20;

/// `ESTCPULIM(e)`.
pub const fn estcpulim(e: u32) -> u32 {
    let lim = NICE_WEIGHT * PRIO_MAX - SCHED_PPQ;
    if e < lim { e } else { lim }
}

/// `CPUTYP_SMT`: SMT cpu.
pub const CPUTYP_SMT: i32 = 0x01;
/// `CPUTYP_P`: Performance core.
pub const CPUTYP_P: i32 = 0x02;
/// `CPUTYP_E`: Efficiency core.
pub const CPUTYP_E: i32 = 0x04;
/// `CPUTYP_L`: Lethargic, Low Power Efficiency core.
pub const CPUTYP_L: i32 = 0x08;

/// `cpu_is_idle(ci)`: nothing is queued on `ci` (a CPU without scheduler state yet has
/// nothing queued).
pub fn cpu_is_idle(ci: &crate::machine::cpu::CpuInfo) -> bool {
    let n = <crate::machine::Machine as crate::machine::cpu::Cpu>::cpu_info_unit(ci) as usize;
    crate::kern::sched_bsd::sched_cpu(n).is_none_or(|c| c.is_idle())
}

/// `scheduler_wait_hook(parent, child)`: chargeback parents for the sins of their children.
pub fn scheduler_wait_hook(parent: &Proc, child: &Proc) {
    parent
        .p_estcpu
        .set(estcpulim(parent.p_estcpu.get() + child.p_estcpu.get()));
}

/// `sched_pause(func)`: calls `func` (a yield) when the CPU's scheduler flags ask the
/// running thread to give the CPU up (`SPCF_SHOULDYIELD`).
pub fn sched_pause(func: fn()) {
    if crate::kern::sched_bsd::curcpu_schedflags() & SPCF_SHOULDYIELD != 0 {
        func();
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estcpulim_clamps() {
        assert_eq!(estcpulim(0), 0);
        assert_eq!(estcpulim(35), 35);
        assert_eq!(estcpulim(36), 36);
        assert_eq!(estcpulim(1000), 36);
    }
}
/* </TESTS> */
