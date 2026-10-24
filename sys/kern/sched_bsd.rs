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
/* </LICENSES> */

/* <CODE> */
//! The 4.4BSD scheduler: `kern/sched_bsd.c`, as `bsd`'s adapter of `kernel::sched`.
//!
//! Upstream: sys/kern/sched_bsd.c @ 3ce1f3f79392
//! LZ: sys/kern/sched_bsd.rs@3c62ede68802
//!
//! The scheduler is `kernel::sched` (N4.5a): `mi_switch`, `setrunnable`, `setpriority`,
//! `schedclock`, `yield`, `preempt`, the load average, `schedcpu` and `roundrobin`'s work
//! live there, over `hal::thread`'s `SCHED_LOCK`. This module keeps LZ's names for `bsd`'s
//! callers: the lock's `SCHED_LOCK()`/`SCHED_UNLOCK()` pair as a hold
//! ([`sched_lock`]/[`sched_unlock`]), the clock interrupt `roundrobin` with the clockintr
//! signature, the running thread found by `hal::thread::with_current`, `struct cpu_info` read
//! as the CPU's number, and `averunnable` as `struct loadavg`. The CPU throttling
//! (`cpu_setperf`, `perflevel`, the `perfpolicy_*` knobs, `setperf_auto`,
//! `sysctl_hwsetperf`, `sysctl_hwperfpolicy`) needs sysctl and the power sensors (M7).
//!
//! ## Deviations
//! - `yield` is a Rust keyword: the function is the raw identifier `r#yield`.
//!
//! ## Redesign
//! - LZ's `SCHED_LOCK` was a `mutex(9)` taken by `sched_lock()` and released by
//!   `sched_unlock()` in any frame (`mi_switch`'s caller locks, `mi_switch` or the new
//!   thread's `proc_trampoline_mi` unlocks). Here [`sched_lock`] returns the hold of
//!   `hal::thread::SCHED_LOCK`, the witness the scheduler's functions take (`&SchedHold` to
//!   read, `&mut SchedHold` to change a queue), and [`sched_unlock`] or [`mi_switch`] consume
//!   it; `SCHED_LOCK_INIT()` has nothing left to do (the lock is a static made at its level).
//! - The scheduler's own state left `struct schedstate_percpu` for `kernel::sched::SchedCpu`
//!   ([`sched_cpu`], [`curcpu_schedflags`]).

use core::pin::Pin;
use core::sync::atomic::Ordering;

use hal::thread::SchedHold;
use libkern::StaticCell;

use crate::kern::kern_clockintr::clockrequest_advance;
use crate::kern::proc_glue::BsdProcGlue;
use crate::machine::Machine;
use crate::machine::cpu::{Cpu, curcpu};
use crate::sys::clockintr::Clockrequest;
use crate::sys::param::FSCALE;
use crate::sys::proc::{Proc, ProcExt};
use crate::sys::resource::Loadavg;

pub use kernel::sched::{AVERUNNABLE, CCPU, ROUNDROBIN_PERIOD, decay_aftersleep};

/// The glue every scheduler call here is made for.
type G = BsdProcGlue;

/// `cpu_setperf`: the machine's CPU performance setter, `None` until a driver installs one
/// (acpicpu(4)'s `acpicpu_setperf`, M16e). Written while cold by the attaching driver;
/// `hw.setperf` and the perfpolicy timeout that call it are not ported (see above).
pub static CPU_SETPERF: StaticCell<Option<fn(i32)>> = StaticCell::new(None);

/// `averunnable` as the C struct, for sysctl and the like.
pub fn averunnable() -> Loadavg {
    Loadavg {
        ldavg: kernel::sched::averunnable(),
        fscale: i64::from(FSCALE),
    }
}

/// `roundrobin_period`.
pub fn roundrobin_period() -> u64 {
    ROUNDROBIN_PERIOD.load(Ordering::Relaxed)
}

/// `SCHED_LOCK()`: the scheduler lock's hold, the witness of the run and sleep queues.
#[inline]
pub fn sched_lock() -> SchedHold {
    hal::thread::sched_lock()
}

/// `SCHED_UNLOCK()`: the hold given back.
#[inline]
pub fn sched_unlock(h: SchedHold) {
    drop(h);
}

/// `SCHED_ASSERT_LOCKED()`.
pub fn sched_assert_locked() {
    crate::kassert!(hal::thread::sched_locked());
}

/// `SCHED_ASSERT_UNLOCKED()`.
pub fn sched_assert_unlocked() {
    crate::kassert!(!hal::thread::sched_locked());
}

/// CPU `n`'s scheduler state (`ci->ci_schedstate`'s scheduler members), once it joined.
pub fn sched_cpu(n: usize) -> Option<&'static kernel::sched::SchedCpu<G>> {
    <G as kernel::proc::ProcGlue>::sched().cpu(n)
}

/// `curcpu()->ci_schedstate.spc_schedflags` (0 before the CPU joined).
pub fn curcpu_schedflags() -> i32 {
    sched_cpu(hal::cpu::cpu_number()).map_or(0, |c| c.schedflags())
}

/// `roundrobin`: force switch among equal priority processes every 100ms.
pub fn roundrobin(cr: &Clockrequest, _frame: &hal::clock::ClockFrame) {
    let count = clockrequest_advance(cr, roundrobin_period());
    kernel::sched::roundrobin::<G>(count);
}

/// `yield`: general yield call. Puts the current process back on its run queue and performs
/// a voluntary context switch.
pub fn r#yield() {
    kernel::sched::r#yield::<G>();
}

/// `preempt`: general preemption call. Puts the current process back on its run queue and
/// performs an involuntary context switch.
pub fn preempt() {
    kernel::sched::preempt::<G>();
}

/// `mi_switch`: the machine-independent half of a context switch, for the running thread,
/// whose `p_stat` its caller already changed from `SONPROC` under the hold `h`; returns once
/// it is scheduled again, the hold released.
pub fn mi_switch(h: SchedHold) {
    hal::thread::with_current(|p: Pin<&Proc>| kernel::sched::mi_switch(p, h));
}

/// `setrunnable`: change process state to be runnable, placing it on the run queue.
pub fn setrunnable(h: &mut SchedHold, p: &Proc) {
    kernel::sched::setrunnable(h, hal::pin::pin_static(p.as_static()));
}

/// `setpriority`: compute the priority of a process.
pub fn setpriority(h: &SchedHold, p: &Proc, newcpu: u32, nice: u8) {
    kernel::sched::setpriority(h, p, newcpu, nice);
}

/// `schedclock`: adjust the priority of the current process as it accumulates CPU time
/// (`kernel::sched::schedclock`); a spinning CPU's thread is not charged.
pub fn schedclock(p: &Proc) {
    let spc = Machine::ci_schedstate(curcpu());
    let spinning =
        spc.spc_spinning.load(Ordering::Relaxed) != 0 || hal::cpu::curcpu().spinning() != 0;
    kernel::sched::schedclock::<G>(p, spinning);
}

/// `scheduler_start`: start the scheduler's periodic timeouts.
pub fn scheduler_start() {
    kernel::sched::scheduler_start::<G>();
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn averunnable_is_the_kernels_load_in_fixed_point() {
        AVERUNNABLE[1].store(FSCALE / 2, Ordering::Relaxed);
        let l = averunnable();
        assert_eq!(l.ldavg[1], FSCALE / 2);
        assert_eq!(l.fscale, i64::from(FSCALE));
        AVERUNNABLE[1].store(0, Ordering::Relaxed);
    }

    #[test]
    fn the_hold_is_the_lock() {
        sched_assert_unlocked();
        let h = sched_lock();
        sched_assert_locked();
        sched_unlock(h);
        sched_assert_unlocked();
    }
}
/* </TESTS> */
