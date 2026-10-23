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
//! Clock handling routines: `kern/kern_clock.c`.
//!
//! This code is written to operate with two timers that run independently of each other.
//! The main clock, running hz times per second, is used to keep track of real time. The
//! second timer handles kernel and user profiling, and does resource use estimation. If the
//! second timer is programmable, it is randomized to avoid aliasing between the two clocks.
//! For example, the randomization prevents an adversary from always giving up the cpu just
//! before its quantum expires. Otherwise, it would never accumulate cpu ticks. The mean
//! frequency of the second timer is stathz.
//!
//! If no second timer exists, stathz will be zero; in this case we drive profiling and
//! statistics off the main clock. This WILL NOT be accurate; do not do it unless absolutely
//! necessary.
//!
//! The statistics clock may (or may not) be run at a higher rate while profiling. This
//! profile clock runs at profhz. We require that profhz be an integral multiple of stathz.
//!
//! If the statistics clock is running fast, it must be divided by the ratio profhz/stathz
//! for statistics. (For profiling, every tick counts.)
//!
//! Upstream: sys/kern/kern_clock.c @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M5 ports the globals, `initclocks`, `hardclock`, `tvtohz`,
//! `tstohz` and `statclock`; M7b completes `statclock`'s per-thread accounting
//! (`p_cpticks`, the `tusage` ticks and sizes, `schedclock`). `startprofclock` and
//! `stopprofclock` (profiling) are not ported; `sysctl_clockrate` came with `kern_sysctl.c`.
//!
//! ## Deviations
//! - None beyond the module's status: the profiling half of the file is missing.

use core::sync::atomic::Ordering;

// The clock's statics, `hardclock`, `initclocks` and `sysctl_clockrate` live in the kernel
// crate (N4.1: `ticks`; N4.8a: the rest, `kernel::clock`); re-exported under their names for
// the readers of this path.
pub use kernel::clock::{
    HARDCLOCK_PERIOD, JIFFIES, PROFHZ, PROFPROCS, STATCLOCK_AVG, STATCLOCK_IS_RANDOMIZED,
    STATCLOCK_MASK, STATCLOCK_MIN, STATHZ, TICKS, hardclock, hardclock_period, initclocks, ticks,
};

use crate::conf::param::{HZ, TICK};
use crate::kern::kern_clockintr::{clockrequest_advance, clockrequest_advance_random};
use crate::kern::kern_lock::{pc_sprod_enter, pc_sprod_leave};
use crate::kern::sched_bsd::schedclock;
use crate::machine::Machine;
use crate::machine::cpu::{Cpu, curcpu};
use crate::sys::clockintr::Clockrequest;
use crate::sys::errno::Errno;
use crate::sys::param::{NZERO, PAGE_SHIFT};
use crate::sys::proc::ProcExt;
use crate::sys::proc::{P_SYSTEM, TU_ITICKS, TU_STICKS, TU_UTICKS, tu_enter, tu_leave};
use crate::sys::sched::{CP_IDLE, CP_INTR, CP_NICE, CP_SPIN, CP_SYS, CP_USER};
use crate::sys::time::{Timespec, Timeval, timespec_to_timeval};
use hal::clock::ClockFrame;

/// `tvtohz`: compute number of hz in the specified amount of time.
pub fn tvtohz(tv: &Timeval) -> i32 {
    // If the number of usecs in the whole seconds part of the time fits in a long, then the
    // total number of usecs will fit in an unsigned long. Compute the total and convert it to
    // ticks, rounding up and adding 1 to allow for the current tick to expire. Rounding also
    // depends on unsigned long arithmetic to avoid overflow.
    //
    // Otherwise, if the number of ticks in the whole seconds part of the time fits in a
    // long, then convert the parts to ticks separately and add, using similar rounding
    // methods and overflow avoidance. This method would work in the previous case but it is
    // slightly slower and assumes that hz is integral.
    //
    // Otherwise, round the time down to the maximum representable value.
    //
    // If ints have 32 bits, then the maximum value for any timeout in 10ms ticks is 248 days.
    let sec = tv.tv_sec;
    let usec = tv.tv_usec;
    let tick = TICK.load(Ordering::Relaxed) as u64;
    let hz = HZ.load(Ordering::Relaxed) as i64;
    let nticks: u64 = if sec < 0 || (sec == 0 && usec <= 0) {
        0
    } else if sec <= i64::MAX / 1_000_000 {
        (sec as u64)
            .wrapping_mul(1_000_000)
            .wrapping_add(usec as u64)
            .div_ceil(tick)
            + 1
    } else if sec <= i64::MAX / hz {
        (sec as u64).wrapping_mul(hz as u64) + (usec as u64).div_ceil(tick) + 1
    } else {
        i64::MAX as u64
    };
    nticks.min(i32::MAX as u64) as i32
}

/// `tstohz`.
pub fn tstohz(ts: &Timespec) -> i32 {
    let mut tv = timespec_to_timeval(ts);

    // Round up.
    if ts.tv_nsec % 1000 != 0 {
        tv.tv_usec += 1;
        if tv.tv_usec >= 1_000_000 {
            tv.tv_usec -= 1_000_000;
            tv.tv_sec += 1;
        }
    }

    tvtohz(&tv)
}

// startprofclock, stopprofclock: struct process (M5-b).

/// `statclock`: statistics clock. Grab profile sample, and if divider reaches 0, do process
/// and kernel statistics.
pub fn statclock(cr: &Clockrequest, frame: &ClockFrame) {
    let ci = curcpu();
    let spc = Machine::ci_schedstate(ci);
    let p = Machine::ci_curproc(ci);

    let count = if STATCLOCK_IS_RANDOMIZED.load(Ordering::Relaxed) {
        clockrequest_advance_random(
            cr,
            STATCLOCK_MIN.load(Ordering::Relaxed),
            STATCLOCK_MASK.load(Ordering::Relaxed),
        )
    } else {
        clockrequest_advance(cr, STATCLOCK_AVG.load(Ordering::Relaxed))
    };

    // SAFETY: `ci_curproc` names the thread on this CPU, hence alive.
    let p = unsafe { p.as_ref() };
    let usermode = frame.usermode();
    let mut tu_tick: Option<usize> = None;
    let cp_time = if let (true, Some(p)) = (usermode, p) {
        let pr = p.process();
        // Came from user mode; CPU was in user state. If this process is being profiled
        // record the tick.
        tu_tick = Some(TU_UTICKS);
        if i32::from(pr.ps_nice.get()) > NZERO {
            CP_NICE
        } else {
            CP_USER
        }
    } else {
        // Came from kernel mode, so we were:
        // - spinning on a lock
        // - handling an interrupt,
        // - doing syscall or trap work on behalf of the current user process, or
        // - spinning in the idle loop.
        // Whichever it is, charge the time as appropriate. Note that we charge interrupts to
        // the current process, regardless of whether they are ``for'' that process, so that we
        // know how much of its real time was spent in ``non-process'' (i.e., interrupt) work.
        let mut cp_time = if frame.intr() {
            tu_tick = Some(TU_ITICKS);
            CP_INTR
        } else if p.is_some_and(|p| !kernel::sched::is_idle_thread(p)) {
            tu_tick = Some(TU_STICKS);
            CP_SYS
        } else {
            CP_IDLE
        };

        if spc.spc_spinning.load(Ordering::Relaxed) != 0 || hal::cpu::curcpu().spinning() != 0 {
            cp_time = CP_SPIN;
        }
        cp_time
    };

    let generation = pc_sprod_enter(&spc.spc_cp_time_lock);
    let t = &spc.spc_cp_time[cp_time]; // this CPU is the only writer
    t.store(t.load(Ordering::Relaxed) + count, Ordering::Relaxed);
    pc_sprod_leave(&spc.spc_cp_time_lock, generation);

    if let Some(p) = p {
        p.p_cpticks
            .set(p.p_cpticks.get().wrapping_add(count as u32));

        if p.p_flag.load(Ordering::Relaxed) & P_SYSTEM == 0
            && let Some(tu_tick) = tu_tick
        {
            let vm = p.vmspace();
            let tu = &p.p_tu;

            let generation = tu_enter(tu);
            tu.tu_ticks[tu_tick].set(tu.tu_ticks[tu_tick].get().wrapping_add(count));

            // maxrss is handled by uvm
            if tu_tick != TU_ITICKS {
                let kb = |pages: i32| ((pages as u64) << (PAGE_SHIFT - 10)).wrapping_mul(count);
                tu.tu_ixrss.set(
                    tu.tu_ixrss
                        .get()
                        .wrapping_add(kb(vm.vm_tsize.load(Ordering::Relaxed))),
                );
                tu.tu_idrss.set(
                    tu.tu_idrss
                        .get()
                        .wrapping_add(kb(vm.vm_dused.load(Ordering::Relaxed))),
                );
                tu.tu_isrss.set(
                    tu.tu_isrss
                        .get()
                        .wrapping_add(kb(vm.vm_ssize.load(Ordering::Relaxed))),
                );
            }
            tu_leave(tu, generation);
        }

        // schedclock() runs every fourth statclock().
        for _ in 0..count {
            let ticks = spc.spc_schedticks.get().wrapping_add(1);
            spc.spc_schedticks.set(ticks);
            if ticks & 3 == 0 {
                schedclock(p);
            }
        }
    }
}

/// `sysctl_clockrate`: `kern.clockrate`, with `kern_sysctl.c`'s arguments
/// (`kernel::clock::sysctl_clockrate`).
pub fn sysctl_clockrate(where_: usize, sizep: &mut usize, newp: usize) -> Result<(), Errno> {
    let mut req = kernel::sysctl::SysctlReq {
        oldp: where_,
        oldlenp: sizep,
        newp,
        newlen: 0,
    };
    kernel::clock::sysctl_clockrate(&mut req)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tvtohz_rounds_up_and_adds_one() {
        // hz = 100, tick = 10000 us.
        assert_eq!(tvtohz(&Timeval::new(0, 0)), 0);
        assert_eq!(tvtohz(&Timeval::new(-1, 0)), 0);
        assert_eq!(tvtohz(&Timeval::new(0, 1)), 2);
        assert_eq!(tvtohz(&Timeval::new(0, 10_000)), 2);
        assert_eq!(tvtohz(&Timeval::new(0, 10_001)), 3);
        assert_eq!(tvtohz(&Timeval::new(1, 0)), 101);
        assert_eq!(tvtohz(&Timeval::new(i64::MAX, 0)), i32::MAX);
        assert_eq!(tstohz(&Timespec::new(0, 1)), 2);
        assert_eq!(tstohz(&Timespec::new(0, 10_000_001)), 3);
        // Rounds up to a whole second: 100 ticks plus the one for the current tick.
        assert_eq!(tstohz(&Timespec::new(0, 999_999_999)), 101);
    }
}
/* </TESTS> */
