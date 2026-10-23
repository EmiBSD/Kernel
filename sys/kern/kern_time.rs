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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)kern_time.c	8.4 (Berkeley) 5/26/95
 */
/* </LICENSES> */

/* <CODE> */
//! Time-related system calls and the interval timers: `kern/kern_time.c`.
//!
//! Upstream: sys/kern/kern_time.c @ 3ce1f3f79392
//!
//! Status: `ported`. Time of day and interval timer support: `settime`, `clock_gettime`,
//! `sys_clock_gettime`, `sys_clock_settime`, `sys_clock_getres`, `sys_nanosleep`,
//! `sys_gettimeofday`, `sys_settimeofday`, `sys_adjfreq`, `sys_adjtime`, `setitimer`,
//! `cancel_all_itimers`, `sys_getitimer`, `sys_setitimer`, `realitexpire`, `itimerfix`,
//! `itimerdecr`, `itimer_update`, `process_reset_itimer_flag`, `ratecheck`, `ppsratecheck`,
//! `inittodr`, `resettodr`, `todr_attach` and the periodic `resettodr`.
//!
//! Since N4.8's a6 the ten system calls, `settime`, `clock_gettime` and `itimerfix` are
//! `kernel::time`'s: the `sys_*` here are the `sysent[]` rows that call them, and
//! `setitimer` is reached from there through the process glue (`ProcGlue::setitimer`).
//!
//! These routines provide the kernel entry points to get and set the time-of-day and
//! per-process interval timers. Subroutines here provide support for adding and
//! subtracting timeval structures and decrementing interval timers, optionally reloading
//! the interval timers when they expire.
//!
//! ## Deviations
//! - No time-of-day chip driver is ported (`todr_attach` has no caller): `inittodr` sets the
//!   clock from the file system's time and warns, `resettodr` has nothing to write.
//! - `itimerdecr` returns `bool` (`true` where the C returns 1: the timer is still
//!   running).

use core::ffi::c_void;
use core::sync::atomic::Ordering;

use crate::dev::clock_subr::TodrChipHandle;
use crate::kern::kern_clock::hardclock_period;
use crate::kern::kern_clockintr::clockrequest_advance;
use crate::kern::kern_lock::{mtx_enter, mtx_leave};
use crate::kern::kern_sig::prsignal;
use crate::kern::kern_tc::nanouptime;
use crate::kern::kern_timeout::{timeout_abs_ts, timeout_del};
use crate::machine::Machine;
use crate::machine::cpu::{Cpu, curcpu, curproc, need_resched};
use crate::machine::intr::IPL_CLOCK;
use crate::sys::_time::Itimerspec;
use crate::sys::clockintr::Clockrequest;
use crate::sys::errno::Errno;
use crate::sys::mutex::Mutex;
use crate::sys::proc::{
    P_ALRMPEND, P_PROFPEND, P_SYSTEM, P_WEXIT, PS_EXITING, PS_ITIMER, Proc, Process,
};
use crate::sys::signal::SIGALRM;
use crate::sys::syscallargs::{
    SysAdjfreqArgs, SysAdjtimeArgs, SysClockGetresArgs, SysClockGettimeArgs, SysClockSettimeArgs,
    SysGetitimerArgs, SysGettimeofdayArgs, SysNanosleepArgs, SysSetitimerArgs, SysSettimeofdayArgs,
};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::time::{
    ITIMER_PROF, ITIMER_REAL, ITIMER_VIRTUAL, Itimerval, Timespec, Timeval, nsec_to_timespec,
    timespec_to_timeval, timespecadd, timespecsub, timeval_to_timespec,
};
use crate::sys::types::{Register, Time};
use hal::clock::ClockFrame;
use hal::user::UserPtr;

/// `itimer_mtx`: the virtual and profiling interval timers.
pub static ITIMER_MTX: Mutex = Mutex::new(IPL_CLOCK);

/// `clock_gettime(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_clock_gettime`]).
pub fn sys_clock_gettime(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysClockGettimeArgs = sysargs(v);
    kernel::time::sys_clock_gettime(p, uap.clock_id.get(), UserPtr::new(uap.tp.get() as usize))
}

/// `clock_settime(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_clock_settime`]).
pub fn sys_clock_settime(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysClockSettimeArgs = sysargs(v);
    kernel::time::sys_clock_settime(p, uap.clock_id.get(), UserPtr::new(uap.tp.get() as usize))
}

/// `clock_getres(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_clock_getres`]).
pub fn sys_clock_getres(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysClockGetresArgs = sysargs(v);
    kernel::time::sys_clock_getres(p, uap.clock_id.get(), UserPtr::new(uap.tp.get() as usize))
}

/// `nanosleep(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_nanosleep`]).
pub fn sys_nanosleep(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysNanosleepArgs = sysargs(v);
    kernel::time::sys_nanosleep(
        p,
        UserPtr::new(uap.rqtp.get() as usize),
        UserPtr::new(uap.rmtp.get() as usize),
    )
}

/// `gettimeofday(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_gettimeofday`]).
pub fn sys_gettimeofday(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGettimeofdayArgs = sysargs(v);
    kernel::time::sys_gettimeofday(
        p,
        UserPtr::new(uap.tp.get() as usize),
        UserPtr::new(uap.tzp.get() as usize),
    )
}

/// `settimeofday(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_settimeofday`]).
pub fn sys_settimeofday(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSettimeofdayArgs = sysargs(v);
    kernel::time::sys_settimeofday(
        p,
        UserPtr::new(uap.tv.get() as usize),
        UserPtr::new(uap.tzp.get() as usize),
    )
}

/// `adjfreq(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_adjfreq`]).
pub fn sys_adjfreq(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysAdjfreqArgs = sysargs(v);
    kernel::time::sys_adjfreq(
        p,
        UserPtr::new(uap.freq.get() as usize),
        UserPtr::new(uap.oldfreq.get() as usize),
    )
}

/// `adjtime(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_adjtime`]).
pub fn sys_adjtime(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysAdjtimeArgs = sysargs(v);
    kernel::time::sys_adjtime(
        p,
        UserPtr::new(uap.delta.get() as usize),
        UserPtr::new(uap.olddelta.get() as usize),
    )
}

/// Get or set value of an interval timer. The process virtual and profiling virtual time
/// timers are kept internally in the way they are specified externally: in time until they
/// expire.
///
/// The real time interval timer's `it_value`, in contrast, is kept as an absolute time
/// rather than as a delta, so that it is easy to keep periodic real-time signals from
/// drifting.
///
/// Virtual time timers are processed in the hardclock() routine of kern_clock.c. The real
/// time timer is processed by a timeout routine, called from the softclock() routine.
/// Since a callout may be delayed in real time due to interrupt processing in the system,
/// it is possible for the real time timeout routine (realitexpire, given below), to be
/// delayed in real time past when it is supposed to occur. It does not suffice, therefore,
/// to reload the real timer .it_value from the real time timers .it_interval. Rather, we
/// compute the next time in absolute time the timer should go off.
pub fn setitimer(which: i32, itv: Option<&Itimerval>, olditv: Option<&mut Itimerval>) {
    crate::kassert!((ITIMER_REAL..=ITIMER_PROF).contains(&which));

    let Some(p) = curproc() else {
        return;
    };
    let pr = p.process();
    let itimer = &pr.ps_timer[which as usize];
    let mut now = Timespec::new(0, 0);

    let its = itv.map(|itv| Itimerspec {
        it_value: timeval_to_timespec(&itv.it_value),
        it_interval: timeval_to_timespec(&itv.it_interval),
    });

    let g_pr = if which == ITIMER_REAL {
        let g = pr.ps_mtx.lock();
        now = nanouptime();
        Some(g)
    } else {
        mtx_enter(&ITIMER_MTX);
        None
    };

    let mut oldits = Itimerspec::new();
    if olditv.is_some() {
        oldits = itimer.get();
    }
    if let Some(mut its) = its {
        if which == ITIMER_REAL {
            if its.it_value.is_set() {
                its.it_value = timespecadd(&its.it_value, &now);
                timeout_abs_ts(&pr.ps_realit_to, &its.it_value);
            } else {
                timeout_del(&pr.ps_realit_to);
            }
        }
        itimer.set(its);
        if which == ITIMER_VIRTUAL || which == ITIMER_PROF {
            process_reset_itimer_flag(pr);
            need_resched(curcpu());
        }
    }

    if which == ITIMER_REAL {
        drop(g_pr);
    } else {
        mtx_leave(&ITIMER_MTX);
    }

    if let Some(olditv) = olditv {
        if which == ITIMER_REAL && oldits.it_value.is_set() {
            if oldits.it_value < now {
                oldits.it_value = Timespec::new(0, 0);
            } else {
                oldits.it_value = timespecsub(&oldits.it_value, &now);
            }
        }
        olditv.it_value = timespec_to_timeval(&oldits.it_value);
        olditv.it_interval = timespec_to_timeval(&oldits.it_interval);
    }
}

/// `cancel_all_itimers`: stop the calling process's three interval timers.
pub fn cancel_all_itimers() {
    let itv = Itimerval::default();

    for i in ITIMER_REAL..=ITIMER_PROF {
        setitimer(i, Some(&itv), None);
    }
}

/// `getitimer(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_getitimer`]), over
/// [`setitimer`] (the glue's).
pub fn sys_getitimer(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetitimerArgs = sysargs(v);
    kernel::time::sys_getitimer(p, uap.which.get(), UserPtr::new(uap.itv.get() as usize))
}

/// `setitimer(2)`'s `sysent[]` row: the kernel's ([`kernel::time::sys_setitimer`]), over
/// [`setitimer`] (the glue's).
pub fn sys_setitimer(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetitimerArgs = sysargs(v);
    kernel::time::sys_setitimer(
        p,
        uap.which.get(),
        UserPtr::new(uap.itv.get() as usize),
        UserPtr::new(uap.oitv.get() as usize),
    )
}

/// Real interval timer expired: send process whose timer expired an alarm signal. If time
/// is not set up to reload, then just return. Else compute next time timer should go off
/// which is > current time. This is where delay in processing this timeout causes multiple
/// SIGALRM calls to be compressed into one.
pub fn realitexpire(arg: *mut c_void) {
    // SAFETY: `process_initialize` hands the process itself, alive while its timeout can
    // fire (`exit1` cancels it).
    let pr: &Process = unsafe { &*arg.cast::<Process>() };
    let tp = &pr.ps_timer[ITIMER_REAL as usize];
    let mut need_signal = false;

    let g_pr = pr.ps_mtx.lock();

    'out: {
        let mut t = tp.get();
        // Do nothing if the timer was cancelled or rescheduled while we were entering the
        // mutex.
        if !t.it_value.is_set() || crate::sys::timeout::timeout_pending(&pr.ps_realit_to) {
            break 'out;
        }

        // The timer expired. We need to send the signal.
        need_signal = true;

        // One-shot timers are not reloaded.
        if !t.it_interval.is_set() {
            t.it_value = Timespec::new(0, 0);
            tp.set(t);
            break 'out;
        }

        // Find the nearest future expiration point and restart the timeout.
        let cts = nanouptime();
        while t.it_value <= cts {
            t.it_value = timespecadd(&t.it_value, &t.it_interval);
        }
        tp.set(t);
        if pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING == 0 {
            timeout_abs_ts(&pr.ps_realit_to, &t.it_value);
        }
    }

    drop(g_pr);

    if need_signal {
        prsignal(pr, SIGALRM);
    }
}

/// Decrement an interval timer by the given duration: the kernel's
/// (`kernel::time::itimerdecr`).
pub fn itimerdecr(itp: &mut Itimerspec, decrement: &Timespec) -> bool {
    kernel::time::itimerdecr(itp, decrement)
}

/// `itimer_update`: the per-CPU clock interrupt that decrements the running process's
/// virtual and profiling interval timers.
pub fn itimer_update(cr: &Clockrequest, frame: &ClockFrame) {
    let Some(p) = curproc() else {
        return;
    };
    if p.p_flag.load(Ordering::Relaxed) & (P_SYSTEM | P_WEXIT) != 0 {
        return;
    }

    let pr = p.process();
    if pr.ps_flags.load(Ordering::Relaxed) & PS_ITIMER == 0 {
        return;
    }

    let nsecs = clockrequest_advance(cr, hardclock_period()) * hardclock_period();
    let elapsed = nsec_to_timespec(nsecs);

    let usermode = frame.usermode();

    mtx_enter(&ITIMER_MTX);
    let mut virt = pr.ps_timer[ITIMER_VIRTUAL as usize].get();
    if usermode && virt.it_value.is_set() && !itimerdecr(&mut virt, &elapsed) {
        pr.ps_timer[ITIMER_VIRTUAL as usize].set(virt);
        process_reset_itimer_flag(pr);
        p.p_flag.fetch_or(P_ALRMPEND, Ordering::Relaxed);
        Machine::need_proftick(p);
    } else {
        pr.ps_timer[ITIMER_VIRTUAL as usize].set(virt);
    }
    let mut prof = pr.ps_timer[ITIMER_PROF as usize].get();
    if prof.it_value.is_set() && !itimerdecr(&mut prof, &elapsed) {
        pr.ps_timer[ITIMER_PROF as usize].set(prof);
        process_reset_itimer_flag(pr);
        p.p_flag.fetch_or(P_PROFPEND, Ordering::Relaxed);
        Machine::need_proftick(p);
    } else {
        pr.ps_timer[ITIMER_PROF as usize].set(prof);
    }
    mtx_leave(&ITIMER_MTX);
}

/// `process_reset_itimer_flag`: `PS_ITIMER` says whether a virtual or profiling timer runs.
pub fn process_reset_itimer_flag(ps: &Process) {
    if ps.ps_timer[ITIMER_VIRTUAL as usize].get().it_value.is_set()
        || ps.ps_timer[ITIMER_PROF as usize].get().it_value.is_set()
    {
        ps.ps_flags.fetch_or(PS_ITIMER, Ordering::Relaxed);
    } else {
        ps.ps_flags.fetch_and(!PS_ITIMER, Ordering::Relaxed);
    }
}

/// `ratecheck(9)`: the kernel's (`kernel::time::ratecheck`).
pub fn ratecheck(lasttime: &mut Timeval, mininterval: &Timeval) -> bool {
    kernel::time::ratecheck(lasttime, mininterval)
}

/// `ppsratecheck(9)`: the kernel's (`kernel::time::ppsratecheck`).
pub fn ppsratecheck(lasttime: &mut Timeval, curpps: &mut i32, maxpps: i32) -> bool {
    kernel::time::ppsratecheck(lasttime, curpps, maxpps)
}

/// `ppsratecheck()` on counters shared between CPUs (the network's rate limiters, which
/// softnet threads reach at once): the pointers are dereferenced only inside
/// `ppsratecheck_mtx` (`kernel::time::ppsratecheck_locked`, whose guard is the proof), which
/// is what orders every CPU's accesses in C.
///
/// # Safety
/// `lasttime` and `curpps` are valid for reads and writes for the whole call, and every
/// other access to them, from any CPU, also goes through `ppsratecheck` (inside the mutex).
pub unsafe fn ppsratecheck_shared(lasttime: *mut Timeval, curpps: *mut i32, maxpps: i32) -> bool {
    kernel::time::ppsratecheck_locked(|g| {
        // SAFETY: the caller's contract; the mutex is held (the guard `g`) until the
        // references' last use, inside this closure.
        let (lasttime, curpps) = unsafe { (&mut *lasttime, &mut *curpps) };
        kernel::time::ppsratecheck_held(g, lasttime, curpps, maxpps)
    })
}

/// inittodr: the kernel's (`kernel::time::inittodr`).
pub fn inittodr(base: Time) {
    kernel::time::inittodr(base);
}

/// resettodr: the kernel's (`kernel::time::resettodr`).
pub fn resettodr() {
    kernel::time::resettodr();
}

/// `todr_attach`: the chip of `bsd`'s drivers (efi(4), plrtc(4)) to the kernel's
/// (`kernel::time::todr_attach`, the better quality winning).
pub fn todr_attach(todr: &'static TodrChipHandle) {
    kernel::time::todr_attach(todr);
}

/// `start_periodic_resettodr`: the kernel's.
pub fn start_periodic_resettodr() {
    kernel::time::start_periodic_resettodr();
}

/// `stop_periodic_resettodr`: the kernel's.
pub fn stop_periodic_resettodr() {
    kernel::time::stop_periodic_resettodr();
}

const _: () = {
    // The interval timers are indexed by ITIMER_REAL..=ITIMER_PROF.
    assert!(ITIMER_REAL == 0 && ITIMER_PROF == 2);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::types::Suseconds;
    use kernel::time::itimerfix;

    #[test]
    fn itimerdecr_reloads_without_drift() {
        let mut it = Itimerspec {
            it_interval: Timespec::new(1, 0),
            it_value: Timespec::new(0, 500_000_000),
        };
        assert!(itimerdecr(&mut it, &Timespec::new(0, 200_000_000)));
        assert_eq!(it.it_value, Timespec::new(0, 300_000_000));
        // expires, overruns by 0.2 s: the next period is shortened by the overrun
        assert!(!itimerdecr(&mut it, &Timespec::new(0, 500_000_000)));
        assert_eq!(it.it_value, Timespec::new(0, 800_000_000));
        // a one-shot timer clears
        let mut one = Itimerspec {
            it_interval: Timespec::new(0, 0),
            it_value: Timespec::new(0, 1),
        };
        assert!(!itimerdecr(&mut one, &Timespec::new(1, 0)));
        assert!(!one.it_value.is_set());
    }

    #[test]
    fn itimerfix_validates_and_rounds() {
        let tick = crate::conf::param::TICK.load(Ordering::Relaxed) as Suseconds;
        let mut itv = Itimerval {
            it_interval: Timeval::new(0, 1),
            it_value: Timeval::new(1, 0),
        };
        assert_eq!(itimerfix(&mut itv), Ok(()));
        assert_eq!(itv.it_interval, Timeval::new(0, tick));
        let mut zero = Itimerval {
            it_interval: Timeval::new(5, 0),
            it_value: Timeval::new(0, 0),
        };
        assert_eq!(itimerfix(&mut zero), Ok(()));
        assert!(!zero.it_interval.is_set());
        let mut bad = Itimerval {
            it_interval: Timeval::new(0, 0),
            it_value: Timeval::new(-1, 0),
        };
        assert_eq!(itimerfix(&mut bad), Err(Errno::EINVAL));
        let mut big = Itimerval {
            it_interval: Timeval::new(0, 0),
            it_value: Timeval::new(i64::from(u32::MAX) + 1, 0),
        };
        assert_eq!(itimerfix(&mut big), Err(Errno::EINVAL));
    }
}
/* </TESTS> */
