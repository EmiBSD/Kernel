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
use crate::kern::kern_clock::{STATHZ, hardclock_period};
use crate::kern::kern_clockintr::clockrequest_advance;
use crate::kern::kern_lock::{mtx_enter, mtx_leave};
use crate::kern::kern_pledge::pledge_adjtime;
use crate::kern::kern_prot::suser;
use crate::kern::kern_resource::{tuagg_get_proc, tuagg_get_process};
use crate::kern::kern_sig::prsignal;
use crate::kern::kern_synch::{nowake, tsleep_nsec};
use crate::kern::kern_sysctl::SECURELEVEL;
use crate::kern::kern_tc::{
    TC_LOCK, getnanouptime, microtime, nanoruntime, nanotime, nanouptime, tc_adjfreq, tc_adjtime,
};
use crate::kern::kern_timeout::{timeout_abs_ts, timeout_del};
use crate::kern::proc_glue::tfind_user;
use crate::machine::Machine;
use crate::machine::copy::{copyin_obj, copyout, copyout_obj};
use crate::machine::cpu::{Cpu, curcpu, curproc, need_resched};
use crate::machine::intr::IPL_CLOCK;
use crate::sys::_time::{
    CLOCK_BOOTTIME, CLOCK_MONOTONIC, CLOCK_PROCESS_CPUTIME_ID, CLOCK_REALTIME,
    CLOCK_THREAD_CPUTIME_ID, CLOCK_UPTIME, Itimerspec, clock_ptid, clock_type,
};
use crate::sys::clockintr::Clockrequest;
use crate::sys::errno::Errno;
use crate::sys::mutex::Mutex;
use crate::sys::param::{PCATCH, PWAIT};
use crate::sys::proc::{
    P_ALRMPEND, P_PROFPEND, P_SYSTEM, P_WEXIT, PS_EXITING, PS_ITIMER, Proc, Process, Tusage,
};
use crate::sys::signal::SIGALRM;
use crate::sys::syscallargs::{
    SysAdjfreqArgs, SysAdjtimeArgs, SysClockGetresArgs, SysClockGettimeArgs, SysClockSettimeArgs,
    SysGetitimerArgs, SysGettimeofdayArgs, SysNanosleepArgs, SysSetitimerArgs, SysSettimeofdayArgs,
};
use crate::sys::systm::{MAXTSLP, SysArgs, kernel_lock, kernel_unlock, sysargs};
use crate::sys::time::{
    ITIMER_PROF, ITIMER_REAL, ITIMER_VIRTUAL, Itimerval, Timespec, Timeval, Timezone,
    nsec_to_timespec, timespec_to_nsec, timespec_to_timeval, timespecadd, timespecsub,
    timeval_to_timespec,
};
use crate::sys::types::{Clockid, Register, Suseconds, Time};
use hal::clock::ClockFrame;
use hal::rwlock::RwMode;

/// `itimer_mtx`: the virtual and profiling interval timers.
pub static ITIMER_MTX: Mutex = Mutex::new(IPL_CLOCK);

/// This function is used by clock_settime and settimeofday: the kernel's
/// (`kernel::time::settime`), with `kern.securelevel`.
pub fn settime(ts: &Timespec) -> Result<(), Errno> {
    kernel::time::settime(ts, SECURELEVEL.load(Ordering::Relaxed))
}

/// `clock_gettime`: the time of `clock_id` as `p` sees it.
pub fn clock_gettime(p: &Proc, clock_id: Clockid) -> Result<Timespec, Errno> {
    let spc_runtime = || Machine::ci_schedstate(curcpu()).spc_runtime.get();
    match clock_id {
        CLOCK_REALTIME => Ok(nanotime()),
        CLOCK_UPTIME => Ok(nanoruntime()),
        CLOCK_MONOTONIC | CLOCK_BOOTTIME => Ok(nanouptime()),
        CLOCK_PROCESS_CPUTIME_ID => {
            let tu = Tusage::new();
            let tp = nanouptime();
            tuagg_get_process(&tu, p.process());
            let tp = timespecsub(&tp, &spc_runtime());
            Ok(timespecadd(&tp, &tu.tu_runtime.get()))
        }
        CLOCK_THREAD_CPUTIME_ID => {
            let tu = Tusage::new();
            let tp = nanouptime();
            tuagg_get_proc(&tu, p);
            let tp = timespecsub(&tp, &spc_runtime());
            Ok(timespecadd(&tp, &tu.tu_runtime.get()))
        }
        _ => {
            // check for clock from pthread_getcpuclockid()
            if clock_type(clock_id) == CLOCK_THREAD_CPUTIME_ID {
                kernel_lock(); // KERNEL_LOCK()
                let error = {
                    let r = hal::klock::reader();
                    match tfind_user(&r, clock_ptid(clock_id), p.process()) {
                        None => Err(Errno::ESRCH),
                        Some(q) => {
                            let tu = Tusage::new();
                            tuagg_get_proc(&tu, q);
                            Ok(tu.tu_runtime.get())
                        }
                    }
                };
                kernel_unlock(); // KERNEL_UNLOCK()
                error
            } else {
                Err(Errno::EINVAL)
            }
        }
    }
}

/// `clock_gettime(2)`.
pub fn sys_clock_gettime(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysClockGettimeArgs = sysargs(v);

    let ats = clock_gettime(p, uap.clock_id.get())?;

    copyout_obj(&ats, uap.tp.get() as usize)
    // KTRACE: not configured.
}

/// `clock_settime(2)`: root only; only `CLOCK_REALTIME` can be set.
pub fn sys_clock_settime(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysClockSettimeArgs = sysargs(v);

    suser(p)?;

    let ats: Timespec = copyin_obj(uap.tp.get() as usize)?;

    match uap.clock_id.get() {
        CLOCK_REALTIME => {
            if !ats.is_valid() {
                return Err(Errno::EINVAL);
            }
            settime(&ats)?;
        }
        // Other clocks are read-only
        _ => return Err(Errno::EINVAL),
    }

    Ok(())
}

/// `clock_getres(2)`.
pub fn sys_clock_getres(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysClockGetresArgs = sysargs(v);
    let mut ts = Timespec::new(0, 0);
    let clock_id = uap.clock_id.get();
    let stathz = || i64::from(STATHZ.load(Ordering::Relaxed).max(1));

    let error = match clock_id {
        CLOCK_REALTIME | CLOCK_MONOTONIC | CLOCK_BOOTTIME | CLOCK_UPTIME => {
            ts = kernel::time::tc_resolution();
            Ok(())
        }
        CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => {
            ts.tv_nsec = 1_000_000_000 / stathz();
            Ok(())
        }
        _ => {
            // check for clock from pthread_getcpuclockid()
            if clock_type(clock_id) == CLOCK_THREAD_CPUTIME_ID {
                kernel_lock(); // KERNEL_LOCK()
                let found = {
                    let r = hal::klock::reader();
                    tfind_user(&r, clock_ptid(clock_id), p.process()).is_some()
                };
                let error = if found {
                    ts.tv_nsec = 1_000_000_000 / stathz();
                    Ok(())
                } else {
                    Err(Errno::ESRCH)
                };
                kernel_unlock(); // KERNEL_UNLOCK()
                error
            } else {
                Err(Errno::EINVAL)
            }
        }
    };

    let tp = uap.tp.get() as usize;
    if error.is_ok() && tp != 0 {
        ts.tv_nsec = ts.tv_nsec.max(1);
        copyout_obj(&ts, tp)?;
        // KTRACE: not configured.
    }

    error
}

/// `nanosleep(2)`: sleeps for the requested time, or until a signal; the time left is
/// copied out to `rmtp`.
pub fn sys_nanosleep(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysNanosleepArgs = sysargs(v);
    let _ = p;

    let rmtp = uap.rmtp.get() as usize;
    let mut request: Timespec = copyin_obj(uap.rqtp.get() as usize)?;
    // KTRACE: not configured.

    if request.tv_sec < 0 || !request.is_valid() {
        return Err(Errno::EINVAL);
    }

    let mut error;
    loop {
        let start = getnanouptime();
        let nsecs = timespec_to_nsec(&request).clamp(1, MAXTSLP);
        error = tsleep_nsec(nowake(), PWAIT | PCATCH, "nanoslp", nsecs);
        let stop = getnanouptime();
        let elapsed = timespecsub(&stop, &start);
        request = timespecsub(&request, &elapsed);
        if request.tv_sec < 0 {
            request = Timespec::new(0, 0);
        }
        if error != Err(Errno::EWOULDBLOCK) {
            break;
        }
        if !request.is_set() {
            break;
        }
    }

    if error == Err(Errno::ERESTART) {
        error = Err(Errno::EINTR);
    }
    if error == Err(Errno::EWOULDBLOCK) {
        error = Ok(());
    }

    if rmtp != 0 {
        let remainder = request;
        if let Err(copyout_error) = copyout_obj(&remainder, rmtp) {
            error = Err(copyout_error);
        }
        // KTRACE: not configured.
    }

    error
}

/// `gettimeofday(2)`: the time of day; the time zone is always zero.
pub fn sys_gettimeofday(_p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGettimeofdayArgs = sysargs(v);
    let zerotz = Timezone::default();

    let tp = uap.tp.get() as usize;
    let tzp = uap.tzp.get() as usize;

    if tp != 0 {
        let atv = microtime();
        copyout_obj(&atv, tp)?;
        // KTRACE: not configured.
    }
    if tzp != 0 {
        copyout_obj(&zerotz, tzp)?;
    }
    Ok(())
}

/// `settimeofday(2)`: root only; the time zone is checked for readability and ignored.
pub fn sys_settimeofday(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSettimeofdayArgs = sysargs(v);

    let tv = uap.tv.get() as usize;
    let tzp = uap.tzp.get() as usize;

    suser(p)?;
    // Verify all parameters before changing time.
    let atv: Option<Timeval> = if tv != 0 { Some(copyin_obj(tv)?) } else { None };
    if tzp != 0 {
        let _atz: Timezone = copyin_obj(tzp)?;
    }
    if let Some(atv) = atv {
        // KTRACE: not configured.
        if !atv.is_valid() {
            return Err(Errno::EINVAL);
        }
        let ts = timeval_to_timespec(&atv);
        settime(&ts)?;
    }

    Ok(())
}

/// `ADJFREQ_MAX`.
const ADJFREQ_MAX: i64 = 500_000_000i64 << 32;
/// `ADJFREQ_MIN`.
const ADJFREQ_MIN: i64 = -ADJFREQ_MAX;

/// `adjfreq(2)`: reads and (root only) sets the timecounter's frequency adjustment.
pub fn sys_adjfreq(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysAdjfreqArgs = sysargs(v);
    let freq = uap.freq.get() as usize;
    let oldfreq = uap.oldfreq.get() as usize;
    let mut f = 0i64;

    if freq != 0 {
        suser(p)?;
        let mut b = [0u8; 8];
        crate::machine::copy::copyin(freq, &mut b)?;
        f = i64::from_ne_bytes(b);
        if !(ADJFREQ_MIN..=ADJFREQ_MAX).contains(&f) {
            return Err(Errno::EINVAL);
        }
    }

    let w = TC_LOCK.enter(if freq == 0 {
        RwMode::Read
    } else {
        RwMode::Write
    });
    let error = 'out: {
        if oldfreq != 0 {
            let mut oldf = 0i64;
            tc_adjfreq(&w, Some(&mut oldf), None);
            if let Err(e) = copyout(&oldf.to_ne_bytes(), oldfreq) {
                break 'out Err(e);
            }
        }
        if freq != 0 {
            tc_adjfreq(&w, None, Some(f));
        }
        Ok(())
    };
    drop(w);
    error
}

/// `adjtime(2)`: reads and (root only) sets the remaining adjustment of the clock.
pub fn sys_adjtime(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysAdjtimeArgs = sysargs(v);
    let delta = uap.delta.get() as usize;
    let olddelta = uap.olddelta.get() as usize;
    let mut adjustment = 0i64;

    pledge_adjtime(p, delta)?;

    if delta != 0 {
        suser(p)?;
        let atv: Timeval = copyin_obj(delta)?;
        // KTRACE: not configured.
        if !atv.is_valid() {
            return Err(Errno::EINVAL);
        }

        if atv.tv_sec > i64::MAX / 1_000_000 {
            return Err(Errno::EINVAL);
        }
        if atv.tv_sec < i64::MIN / 1_000_000 {
            return Err(Errno::EINVAL);
        }
        adjustment = atv.tv_sec * 1_000_000;
        if adjustment > i64::MAX - atv.tv_usec {
            return Err(Errno::EINVAL);
        }
        adjustment += atv.tv_usec;
    }
    let w = (delta != 0).then(|| TC_LOCK.write());

    let error = 'out: {
        if olddelta != 0 {
            let mut remaining = 0i64;
            tc_adjtime(w.as_ref(), Some(&mut remaining), None);
            let mut atv = Timeval::new(remaining / 1_000_000, (remaining % 1_000_000) as Suseconds);
            if atv.tv_usec < 0 {
                atv.tv_usec += 1_000_000;
                atv.tv_sec -= 1;
            }

            if let Err(e) = copyout_obj(&atv, olddelta) {
                break 'out Err(e);
            }
        }

        if delta != 0 {
            tc_adjtime(w.as_ref(), None, Some(adjustment));
        }
        Ok(())
    };
    drop(w);
    error
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

/// `getitimer(2)`.
pub fn sys_getitimer(_p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetitimerArgs = sysargs(v);

    let which = uap.which.get();
    if !(ITIMER_REAL..=ITIMER_PROF).contains(&which) {
        return Err(Errno::EINVAL);
    }

    let mut aitv = Itimerval::default();

    setitimer(which, None, Some(&mut aitv));

    copyout_obj(&aitv, uap.itv.get() as usize)
    // KTRACE: not configured.
}

/// `setitimer(2)`.
pub fn sys_setitimer(_p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetitimerArgs = sysargs(v);

    let which = uap.which.get();
    if !(ITIMER_REAL..=ITIMER_PROF).contains(&which) {
        return Err(Errno::EINVAL);
    }

    let uitv = uap.itv.get() as usize;
    let uoitv = uap.oitv.get() as usize;
    let mut aitv = Itimerval::default();
    let mut olditv = Itimerval::default();
    let mut newitv = false;
    if uitv != 0 {
        aitv = copyin_obj(uitv)?;
        // KTRACE: not configured.
        itimerfix(&mut aitv)?;
        newitv = true;
    }
    if !newitv && uoitv == 0 {
        return Ok(());
    }

    setitimer(
        which,
        if newitv { Some(&aitv) } else { None },
        if uoitv != 0 { Some(&mut olditv) } else { None },
    );

    if uoitv != 0 {
        return copyout_obj(&olditv, uoitv);
        // KTRACE: not configured.
    }

    Ok(())
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

/// Check if the given setitimer(2) input is valid: the kernel's (`kernel::time::itimerfix`).
pub fn itimerfix(itv: &mut Itimerval) -> Result<(), Errno> {
    kernel::time::itimerfix(itv)
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
