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
/* </LICENSES> */

/* <CODE> */
//! Process exit: `kern/kern_exit.c`.
//!
//! Upstream: sys/kern/kern_exit.c @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M6 (part b) ports `sys_exit`, `exit1`, `exit2`, `proc_free`,
//! `reaper`, `process_clear_orphan`, `process_reparent` and `process_zap`: enough for a
//! kernel thread (and soon `init`) to die and be reaped. M8 adds `sys___threxit`,
//! `dowait6`, `sys_wait4`, `sys_waitid`, `proc_finish_wait` and `process_untrace` (the file
//! is complete) and the `ACCOUNTING` record (`acct_process`) in `exit1`. M11e: the
//! `MULTIPROCESSOR` teardown as in C: `exit1` releases every hold of the kernel lock around
//! `uvm_purge`, and the reaper drops the lock its kthread starts with and takes it only
//! around the zombie's notification.
//!
//! ## Deviations
//! - What `exit1` tears down that does not exist yet is reported, each once:
//!   `stopprofclock`/`prof_write` (`cancel_all_itimers` is real since `kern_time.c`,
//!   `unveil_destroy` since `kern_unveil.c`, `kqpoll_exit` and the reaper's
//!   `knote_processexit` since `kern_event.c`), `process_untrace` (the `SIGKILL` to a
//!   traced child is sent); `process_zap` likewise `vrele`. `killjobc` and
//!   `leavepgrp` are real since the process group management of `kern_proc.c`. The signal side
//!   (`single_thread_set`, `process_suspend_signal`, `sigio_freelist`, `SAS_NOCLDWAIT`, the
//!   reaper's `SIGCHLD`, `sigactsfree`) is real since `kern_sig.c`, `fdfree` since
//!   `kern_descrip.c`, `lim_free` since the `plimit` port. The credentials (`crfree` in `proc_free`
//!   and `process_zap`, the real uid `process_zap` uncharges) are real since `kern_prot.c`.
//! - `initprocess` is null until `init` exists (M6-b): until then process 0 adopts the
//!   orphans `exit1` and `process_reparent` would hand to `init`.
//! - `deadproc` is a `hal::list::List` (N4.2, interim until N4.5): no `Sync` premise on its
//!   head, and `p_runq` is `Send + Sync`. The list's premise is elements never freed; the
//!   reaper takes a thread off it before `proc_free`, and the `&'static` it holds is the
//!   dead CPU list's own element.
//! - N4.2: the process and its threads are the kernel crate's (`kernel::proc`). A thread
//!   leaves `ps_threads` in `exit1` by taking its box back from the process
//!   (`remove_thread_locked`), which it keeps (`p_exited`) until `proc_free` drops it: the
//!   reaper's for a thread, `process_zap`'s for the main thread, as the C's `pool_put`s. A
//!   process goes with its last reference (`process_zap`'s, after the main thread's box),
//!   taking its credentials and limits with it. The lists move under the kernel lock's
//!   writer (`hal::klock::witness`), the walks run under its reader, and a process kept past
//!   one is counted (`process_ref`). `process_clear_orphan` names the orphan list's owner
//!   where `LIST_REMOVE` needs none: the caller's, or the process at `ps_opptr`'s address
//!   (unreachable while `ptrace(2)` is not ported: nothing sets `PS_ORPHAN`).
//! - `dowait6` takes its out-parameters as `Option<&mut>`; `ps_opptr` (ptrace's old parent)
//!   is always null, since `ptrace(2)` (`sys_process.c`) is not ported, so
//!   `proc_finish_wait` always takes the zombie's branch.

use core::ffi::c_void;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering;

use crate::kassert;
use crate::kern::init_main::{INITPROCESS, PROC0, PROCESS0};
use crate::kern::kern_descrip::fdfree;
use crate::kern::kern_event::{knote_processexit, kqpoll_exit};
use crate::kern::kern_fork::{NPROCESSES, NTHREADS, freepid};
use crate::kern::kern_lock::{mtx_enter, mtx_leave};
use crate::kern::kern_resource::{calcru, ruadd, tuagg_add_process, tuagg_add_runtime};
use crate::kern::kern_sched::sched_exit;
use crate::kern::kern_sig::{
    process_suspend_signal, prsignal, psignal, ptsignal, sigactsfree, sigio_freelist,
    single_thread_set,
};
use crate::kern::kern_synch::{msleep_nsec, sleep_finish, sleep_setup, wakeup};
use crate::kern::kern_timeout::timeout_del;
use crate::kern::proc_glue::{chgproccnt, proc_tables, process_ref};
use crate::kern::sched_bsd::sched_assert_unlocked;
use crate::kern::subr_prf::panic;
use crate::kern::vm_bsd::uvm_uarea_free;
use crate::kern::vm_bsd::{uvm_exit, uvm_purge};
#[cfg(feature = "qemu")]
use crate::kprintf;
use crate::machine::Machine;
use crate::machine::copy::{copyout, copyout_obj};
use crate::machine::cpu::Cpu;
use crate::machine::intr::IPL_NONE;
use crate::machine::pmap::pmap_deactivate;
#[cfg(feature = "qemu")]
use crate::machine::{Exit, ExitStatus};
use crate::sys::errno::Errno;
use crate::sys::mutex::{MTX_NOWITNESS, Mutex};
use crate::sys::param::{PCATCH, PVM, PWAIT};
use crate::sys::proc::{
    EXIT_NORMAL, EXIT_THREAD, P_SYSTEM, P_THREAD, P_WEXIT, PS_CONTINUED, PS_EXITING, PS_ISPWAIT,
    PS_NOZOMBIE, PS_ORPHAN, PS_PPWAIT, PS_PROFIL, PS_STOPPED, PS_STOPPING, PS_TRACED, PS_TRAPPED,
    PS_WAITED, PS_WAITEVENT, PS_ZOMBIE, Proc, ProcRunq, Process, ProcessRef, PsGuard, SDEAD,
    SINGLE_EXIT, p_hassibling,
};
use crate::sys::resource::Rusage;
use crate::sys::sched::scheduler_wait_hook;
use crate::sys::siginfo::{
    CLD_CONTINUED, CLD_DUMPED, CLD_EXITED, CLD_KILLED, CLD_STOPPED, CLD_TRAPPED, Siginfo,
};
use crate::sys::signal::{SIGCHLD, SIGCONT, SIGKILL, SIGSEGV};
use crate::sys::signalvar::{SAS_NOCLDWAIT, SignalType};
use crate::sys::syscallargs::{SysExitArgs, SysThrexitArgs, SysWait4Args, SysWaitidArgs};
use crate::sys::systm::{INFSLP, SysArgs, kernel_lock, kernel_unlock, sysargs};
use crate::sys::types::{Pid, Register};
use crate::sys::wait::{
    _WCONTINUED, Idtype, P_ALL, P_PGID, P_PID, WAIT_ANY, WAIT_MYPGRP, WCONTINUED, WEXITED, WNOHANG,
    WNOWAIT, WSTOPPED, WTRAPPED, WUNTRACED, w_exitcode, w_stopcode, wcoredump, wstatus,
};
use crate::unported;
use hal::klock;
use hal::list::List;

/// `sys_exit`: death of process.
pub fn sys_exit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysExitArgs = sysargs(v);

    exit1(p, uap.rval.get(), 0, EXIT_NORMAL)
    // NOTREACHED
}

/// `__threxit(2)`: the calling thread exits; the process goes on unless it was the last
/// thread. `*notdead` (when not NULL) is cleared first, which is how `pthread_join` learns
/// the thread is gone.
#[allow(non_snake_case)] // the C name: sys___threxit
pub fn sys___threxit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysThrexitArgs = sysargs(v);

    let notdead = uap.notdead.get() as usize;
    if notdead != 0 {
        let zero: Pid = 0;
        if copyout(&zero.to_ne_bytes(), notdead).is_err() {
            psignal(p, SIGSEGV);
        }
    }
    exit1(p, 0, 0, EXIT_THREAD)
    // NOTREACHED
}

/// `initprocess`, or process 0 while there is no init (see the module's deviations).
fn initprocess_or_process0() -> &'static Process {
    // SAFETY: a non-null `initprocess` is init's process, which never goes away.
    unsafe { INITPROCESS.load(Ordering::Relaxed).as_ref() }.unwrap_or(&PROCESS0)
}

/// `exit1`: deallocate address space and other resources, change proc state to zombie, and
/// unlink proc from allproc and parent's lists. Save exit status and rusage for wait().
/// Check for child processes and orphan them.
pub fn exit1(p: &Proc, xexit: i32, xsig: i32, flags: i32) -> ! {
    let mut flags = flags;

    p.p_flag.fetch_or(P_WEXIT, Ordering::Relaxed);

    let pr = p.process();

    // single-threaded?
    if !p_hassibling(p) {
        flags = EXIT_NORMAL;
    } else {
        // nope, multi-threaded
        if flags == EXIT_NORMAL {
            let _ = single_thread_set(p, SINGLE_EXIT);
        }
    }

    if flags == EXIT_NORMAL && pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING == 0 {
        if pr.ps_pid == 1 {
            // Under QEMU init's exit ends the run: the M6 exit criterion (see the module's
            // deviations). The C panics, as it has nothing to run.
            #[cfg(feature = "qemu")]
            {
                kprintf!("init exited with status {xexit} (signal {xsig})\n");
                Machine::exit(if xexit == 0 && xsig == 0 {
                    ExitStatus::Success
                } else {
                    ExitStatus::Failure
                });
            }
            #[cfg(not(feature = "qemu"))]
            panic(format_args!("init died (signal {xsig}, exit {xexit})"));
        }

        pr.ps_flags.fetch_or(PS_EXITING, Ordering::Relaxed);
        pr.ps_xexit.set(xexit as u32);
        pr.ps_xsig.set(xsig);

        // If parent is waiting for us to exit or exec, PS_PPWAIT is set; we wake up the
        // parent early to avoid deadlock.
        if pr.ps_flags.load(Ordering::Relaxed) & PS_PPWAIT != 0 {
            pr.ps_flags.fetch_and(!PS_PPWAIT, Ordering::Relaxed);
            let pptr = parent(pr);
            pptr.ps_flags.fetch_and(!PS_ISPWAIT, Ordering::Relaxed);
            pptr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
            wakeup(ptr::from_ref::<Process>(&pptr));
        }

        // Wait for concurrent `allprocess' loops
        pr.ps_refcnt.finalize("psdtor");
    }

    // unlink ourselves from the active threads
    let mut g = pr.ps_mtx.lock();
    let mybox = if ptr::eq(p, &PROC0) {
        pr.remove_static_thread_locked(&mut g, &PROC0);
        None
    } else {
        let b = pr.remove_thread_locked(&mut g, p.p_seq.get());
        kassert!(b.as_ref().is_some_and(|b| ptr::eq(&**b, p)));
        b
    };
    pr.ps_threadcnt.set(pr.ps_threadcnt.get() - 1);
    pr.ps_exitcnt.set(pr.ps_exitcnt.get() + 1);

    // if somebody else wants to take us to single threaded mode or stop us, count ourselves
    // out.
    if !pr.ps_single.get().is_null() || pr.ps_flags.load(Ordering::Relaxed) & PS_STOPPING != 0 {
        process_suspend_signal(pr, &g);
    }

    // proc is off ps_threads list so update accounting of process now
    tuagg_add_runtime();
    tuagg_add_process(pr, p);

    if p.p_flag.load(Ordering::Relaxed) & P_THREAD == 0 {
        // main thread gotta wait because it has the pid, et al
        while pr.ps_threadcnt.get() + pr.ps_exitcnt.get() > 1 {
            let _ = g.sleep_nsec(ptr::addr_of!(pr.ps_threads), PWAIT, "thrdeath", INFSLP);
        }
    }
    drop(g);
    // The thread keeps its own box until `proc_free` (see the module's deviations).
    if let Some(b) = mybox {
        drop(p.p_exited.lock().replace(b));
    }

    // The process's rusage (`ps_ru`), made by the first thread to exit.
    let need_ru = {
        let r = klock::reader();
        pr.ps_ru.get(&r).is_none()
    };
    if need_ru {
        let ru = kernel::proc::rusage_alloc();
        let mut w = klock::witness();
        let slot = pr.ps_ru.get_mut(&mut w);
        if slot.is_none() {
            *slot = Some(ru);
        }
        // else another thread installed the process's rusage meanwhile; `ru` goes back
        // after the witness.
    }
    p.p_siglist.store(0, Ordering::Relaxed);
    if p.p_flag.load(Ordering::Relaxed) & P_THREAD == 0 {
        pr.ps_siglist.store(0, Ordering::Relaxed);
    }

    kqpoll_exit(p);

    // kcov_exit(p): kcov is not configured.

    if p.p_flag.load(Ordering::Relaxed) & P_THREAD == 0 {
        if pr.ps_flags.load(Ordering::Relaxed) & PS_PROFIL != 0 {
            let _ = unported!("exit1: stopprofclock (M7)");
        }
        // prof_write(p): subr_prof.c (M7).

        sigio_freelist(&pr.ps_sigiolst);

        // close open files and release open-file table
        fdfree(p);

        crate::kern::kern_time::cancel_all_itimers();

        timeout_del(&pr.ps_rucheck_to);
        // SYSVSEM: not configured.
        kernel::proc::killjobc(p.process_ref());
        let _ = crate::kern::kern_acct::acct_process(p);
        // KTRACE: not configured.

        crate::kern::kern_unveil::unveil_destroy(pr);

        pin_free(&pr.ps_pin);
        pin_free(&pr.ps_libcpin);

        // If parent has the SAS_NOCLDWAIT flag set, we're not going to become a zombie.
        if parent(pr).sigacts().ps_sigflags.load(Ordering::Relaxed) & SAS_NOCLDWAIT != 0 {
            pr.ps_flags.fetch_or(PS_NOZOMBIE, Ordering::Relaxed);
        }

        // Teardown the virtual address space.
        if p.p_flag.load(Ordering::Relaxed) & P_SYSTEM == 0 {
            // exit1() might be called with a lock count greater than one and we want to
            // ensure the costly operation of tearing down the VM space is performed
            // unlocked. It is safe to release them all since exit1() will not return.
            #[cfg(feature = "multiprocessor")]
            let _ = crate::kern::kern_lock::__mp_release_all(&crate::kern::kern_lock::KERNEL_LOCK);
            uvm_purge();
            kernel_lock();
        }
    }

    // p->p_fd = NULL: the thread's copy is its process's table, emptied by fdfree.

    // Release the thread's read reference of resource limit structure (its copy).
    p.p_limit.clear();

    // Remove proc from pidhash chain and allproc so looking it up won't work. We will put
    // the proc on the deadproc list later (using the p_runq member), and wake up the reaper
    // when we do. If this is the last thread of a process that isn't PS_NOZOMBIE, we'll put
    // the process on the zombprocess list below.
    //
    // NOTE: WE ARE NO LONGER ALLOWED TO SLEEP!
    p.set_stat(SDEAD);

    {
        let mut w = klock::witness();
        let t = proc_tables();
        t.tidhash(p.p_tid.get()).remove(&mut w, p);
        t.allproc.remove(&mut w, p);
    }

    if p.p_flag.load(Ordering::Relaxed) & P_THREAD == 0 {
        {
            let mut w = klock::witness();
            let t = proc_tables();
            t.pidhash(pr.ps_pid).remove(&mut w, pr);
            t.allprocess.remove(&mut w, pr);
            if pr.ps_flags.load(Ordering::Relaxed) & PS_NOZOMBIE == 0 {
                t.zombprocess.insert_head(&mut w, p.process_ref().as_pin());
            }
        }
        if pr.ps_flags.load(Ordering::Relaxed) & PS_NOZOMBIE != 0 {
            // Not going to be a zombie, so it's now off all the lists scanned by
            // ispidtaken(), so block fast reuse of the pid now.
            freepid(pr.ps_pid);
        }

        // Reparent children to their original parent, in case they were being traced, or to
        // init(8).
        let initprocess = process_ref(initprocess_or_process0());
        let any = {
            let r = klock::reader();
            !pr.ps_children.is_empty(&r)
        };
        if any {
            // only need this if any child is S_ZOMB
            wakeup(ptr::from_ref::<Process>(&initprocess));
        }
        // Each pass takes the first child off this process's list (reparenting moves it).
        while let Some(child) = {
            let r = klock::reader();
            pr.ps_children.first(&r).map(process_ref)
        } {
            // Traced processes are killed since their existence means someone is screwing
            // up.
            let mut cg = child.ps_mtx.lock();
            if child.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
                process_untrace(&child, &mut cg);
                drop(cg);

                if child.ps_flags.load(Ordering::Relaxed) & PS_EXITING == 0 {
                    // If single threading is active, direct the signal to the active thread
                    // to avoid deadlock.
                    // SAFETY: a non-null `ps_single` is a live thread of `child`.
                    match unsafe { child.ps_single.get().as_ref() } {
                        Some(single) => ptsignal(single, SIGKILL, SignalType::STHREAD),
                        None => prsignal(&child, SIGKILL),
                    }
                }
            } else {
                process_reparent(&child, &mut cg, &initprocess);
                drop(cg);
            }
        }

        // Make sure orphans won't remember the exiting process.
        while let Some(orphan) = {
            let r = klock::reader();
            pr.ps_orphans.first(&r).map(process_ref)
        } {
            let og = orphan.ps_mtx.lock();
            // KASSERT(qr->ps_opptr == pr); qr->ps_opptr = NULL: ptrace (M7).
            process_clear_orphan(&orphan, Some(pr));
            drop(og);
        }
    }

    // add thread's accumulated rusage into the process's total
    {
        let r = klock::reader();
        if let Some(rup) = pr.ps_ru.get(&r) {
            ruadd(rup, &p.p_ru);
        }
    }

    // clear %cpu usage during swap
    p.p_pctcpu.store(0, Ordering::Relaxed);

    if p.p_flag.load(Ordering::Relaxed) & P_THREAD == 0 {
        // Final thread has died, so add on our children's rusage and calculate the total
        // times.
        {
            let r = klock::reader();
            if let Some(rup) = pr.ps_ru.get(&r) {
                let (utime, stime, _) = calcru(&pr.ps_tu);
                rup.ru_utime.set(utime);
                rup.ru_stime.set(stime);
                rup.ru_ixrss.set(pr.ps_tu.tu_ixrss.get() as i64);
                rup.ru_idrss.set(pr.ps_tu.tu_idrss.get() as i64);
                rup.ru_isrss.set(pr.ps_tu.tu_isrss.get() as i64);
                ruadd(rup, &pr.ps_cru);
            }
        }

        // Notify parent that we're gone. If we're not going to become a zombie, reparent to
        // process 1 (init) so that we can wake our original parent to possibly unblock
        // wait4() to return ECHILD.
        let mut g = pr.ps_mtx.lock();
        if pr.ps_flags.load(Ordering::Relaxed) & PS_NOZOMBIE != 0 {
            let ppr = parent_locked(pr, &g);
            process_reparent(pr, &mut g, &process_ref(initprocess_or_process0()));
            ppr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
            wakeup(ptr::from_ref::<Process>(&ppr));
        }
        drop(g);
    }

    // just a thread? check if last one standing.
    if p.p_flag.load(Ordering::Relaxed) & P_THREAD != 0 {
        // scheduler_wait_hook(pr->ps_mainproc, p); XXX
        let g = pr.ps_mtx.lock();
        pr.ps_exitcnt.set(pr.ps_exitcnt.get() - 1);
        if pr.ps_threadcnt.get() + pr.ps_exitcnt.get() == 1 {
            wakeup(ptr::addr_of!(pr.ps_threads));
        }
        drop(g);
    }

    // Other substructures are freed from reaper and wait().

    // Finally, call machine-dependent code.
    Machine::cpu_exit(p);

    // Deactivate the exiting address space before the vmspace is freed. Note that we will
    // continue to run on this vmspace's context until the switch to idle in sched_exit().
    //
    // Once we are no longer using the dead process's vmspace and stack, exit2() will be
    // called to schedule those resources to be released by the reaper thread.
    pmap_deactivate(p);
    sched_exit(p)
    // panic("sched_exit returned"): sched_exit never returns.
}

/// `pr->ps_pptr`, counted, for a holder of the kernel lock: a process always has a parent
/// (process 0 is its own).
fn parent(pr: &Process) -> ProcessRef {
    let r = klock::reader();
    match pr.pptr(&r) {
        Some(pp) => pp.clone(),
        None => process_ref(&PROCESS0),
    }
}

/// `pr->ps_pptr`, counted, for a holder of its `ps_mtx` (`[K|m]`).
fn parent_locked(pr: &Process, g: &PsGuard<'_>) -> ProcessRef {
    match pr.pptr_locked(g) {
        Some(pp) => pp.clone(),
        None => process_ref(&PROCESS0),
    }
}

/// `deadproc_mutex`: locking of this prochead is special; it's accessed in a critical
/// section of process exit, and thus locking it can't modify interrupt state. We use a simple
/// spin lock for this prochead. We use the `p_runq` member to linkup to deadproc.
static DEADPROC_MUTEX: Mutex = Mutex::new(IPL_NONE);

/// `deadproc`: the threads waiting for the reaper, touched under `deadproc_mutex`.
static DEADPROC: List<ProcRunq> = List::new();

/// `deadproc_mutex`'s `MUTEX_INITIALIZER_FLAGS(IPL_NONE, "deadproc", MTX_NOWITNESS)`: the
/// witness flag is kept for the record.
const DEADPROC_MUTEX_FLAGS: i32 = MTX_NOWITNESS;

/// `exit2`: we are called from `sched_idle()` once it is safe to schedule the dead process's
/// resources to be freed. So this is not allowed to sleep.
///
/// We lock the deadproc list, place the proc on that list (using the `p_runq` member), and
/// wake up the reaper.
pub fn exit2(p: &'static Proc) {
    // account the remainder of time spent in exit1()
    let pr = p.process();
    let g = pr.ps_mtx.lock();
    tuagg_add_process(pr, p);
    drop(g);

    mtx_enter(&DEADPROC_MUTEX);
    // `p` is SDEAD and on no queue (the idle thread took it off `spc_deadproc`).
    DEADPROC.insert_tail(p);
    mtx_leave(&DEADPROC_MUTEX);

    wakeup(ptr::addr_of!(DEADPROC));
    let _ = DEADPROC_MUTEX_FLAGS;
}

/// `proc_free`: returns a dead thread to the pool: the box it kept since `exit1` dropped
/// (its credentials' reference, its vmspace handle and its process's reference with it).
pub fn proc_free(p: &Proc) {
    let b = p.p_exited.lock().take();
    NTHREADS.fetch_sub(1, Ordering::Relaxed);
    drop(b);
}

/// `reaper`: process reaper. This is run by a kernel thread to free the resources of a
/// dead process. Once the resources are free, the process becomes a zombie, and the parent
/// is allowed to read the undead's status.
pub fn reaper(_arg: *mut c_void) {
    kernel_unlock();

    sched_assert_unlocked();

    loop {
        mtx_enter(&DEADPROC_MUTEX);
        let p = loop {
            if let Some(p) = DEADPROC.first() {
                break p;
            }
            let _ = msleep_nsec(
                ptr::addr_of!(DEADPROC),
                &DEADPROC_MUTEX,
                PVM,
                "reaper",
                INFSLP,
            );
        };

        // Remove us from the deadproc list.
        DEADPROC.remove(p);
        mtx_leave(&DEADPROC_MUTEX);

        // WITNESS_THREAD_EXIT(p): not configured.

        // Free the VM resources we're still holding on to. We must do this from a valid
        // thread because doing so may block.
        uvm_uarea_free(p);
        drop(p.set_vmspace(None)); // zap the thread's copy

        if p.p_flag.load(Ordering::Relaxed) & P_THREAD != 0 {
            // Just a thread
            proc_free(p);
        } else {
            let pr = process_ref(p.process());

            // Release the rest of the process's vmspace
            uvm_exit(&pr);

            kernel_lock(); // KERNEL_LOCK()
            if pr.ps_flags.load(Ordering::Relaxed) & PS_NOZOMBIE == 0 {
                // Process is now a true zombie.
                pr.ps_flags.fetch_or(PS_ZOMBIE, Ordering::Relaxed);
            }

            // Notify listeners of our demise and clean up.
            knote_processexit(&pr);

            if pr.ps_flags.load(Ordering::Relaxed) & PS_ZOMBIE != 0 {
                // Post SIGCHLD and wake up parent.
                let pptr = parent(&pr);
                prsignal(&pptr, SIGCHLD);
                pptr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
                wakeup(ptr::from_ref::<Process>(&pptr));
            } else {
                // No one will wait for us, just zap it.
                process_zap(pr);
            }
            kernel_unlock(); // KERNEL_UNLOCK()
        }
    }
}

/// The id `dowait6` compares a process with: its pid or its process group's id.
fn wait_matches(r: &klock::KernelReader, pr: &Process, idtype: Idtype, id: i64) -> bool {
    !((idtype == P_PID && id != i64::from(pr.ps_pid))
        || (idtype == P_PGID && id != i64::from(pr.pgid(r))))
}

/// The `siginfo_t` `dowait6` fills for a child: who, as whom, and the `CLD_*` code; read
/// under the child's `ps_mtx` (`g`), which `dowait6` holds (the C reads
/// `pr->ps_ucred->cr_uid` in the same section; `pr.ucred()` would take the mutex again).
fn wait_info(pr: &Process, g: &PsGuard<'_>, code: i32, status: i32) -> Siginfo {
    let mut info = Siginfo::zeroed();
    info.set_si_pid(pr.ps_pid);
    info.set_si_uid(g.ucred().cr_uid.get());
    info.si_signo = SIGCHLD;
    info.si_code = code;
    info.set_si_status(status);
    info
}

/// What `dowait6` found among the children, decided under the kernel lock's reader.
enum Found {
    /// A child reported: its pid in `retval`.
    Reported,
    /// A zombie to collect (`proc_finish_wait`), held past the reader.
    Zombie(ProcessRef),
    /// Children matched, none had anything to report.
    Waiting,
    /// No child matched.
    None,
}

/// `dowait6`: the body of `wait4(2)` and `waitid(2)`: finds a child of `q` that matches
/// `idtype`/`id` and has something to report under `options` (exited, trapped, stopped or
/// continued), fills `statusp`, `rusage` and `info`, and returns its pid in `retval`;
/// sleeps for one unless `WNOHANG`. `ECHILD` when no child matches.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn dowait6(
    q: &Proc,
    idtype: Idtype,
    id: i64,
    mut statusp: Option<&mut i32>,
    options: i32,
    rusage: Option<&Rusage>,
    mut info: Option<&mut Siginfo>,
    retval: &mut [Register; 2],
) -> Result<(), Errno> {
    let qr = q.process();

    if let Some(info) = info.as_deref_mut() {
        *info = Siginfo::zeroed();
    }

    loop {
        qr.ps_flags.fetch_and(!PS_WAITEVENT, Ordering::Relaxed);
        let found = 'scan: {
            let r = klock::reader();
            let mut nfound = 0;
            for pr in qr.ps_children.iter(&r) {
                let g = pr.ps_mtx.lock();
                let flags = pr.ps_flags.load(Ordering::Relaxed);
                if flags & PS_NOZOMBIE != 0 || !wait_matches(&r, pr, idtype, id) {
                    drop(g);
                    continue;
                }
                nfound += 1;
                if options & WEXITED != 0 && flags & PS_ZOMBIE != 0 {
                    retval[0] = pr.ps_pid as Register;
                    if let Some(info) = info.as_deref_mut() {
                        let xsig = pr.ps_xsig.get();
                        *info = if xsig == 0 {
                            wait_info(pr, &g, CLD_EXITED, pr.ps_xexit.get() as i32)
                        } else if wcoredump(xsig) {
                            wait_info(pr, &g, CLD_DUMPED, wstatus(xsig))
                        } else {
                            wait_info(pr, &g, CLD_KILLED, wstatus(xsig))
                        };
                    }

                    if let Some(statusp) = statusp.as_deref_mut() {
                        *statusp = w_exitcode(pr.ps_xexit.get() as i32, pr.ps_xsig.get());
                    }
                    if let Some(rusage) = rusage
                        && let Some(ru) = pr.ps_ru.get(&r)
                    {
                        rusage.copy_from(ru);
                    }
                    drop(g);
                    if options & WNOWAIT == 0 {
                        break 'scan Found::Zombie(process_ref(pr));
                    }
                    break 'scan Found::Reported;
                }
                if options & WTRAPPED != 0
                    && flags & PS_TRACED != 0
                    && flags & PS_WAITED == 0
                    && flags & PS_STOPPED != 0
                    && flags & PS_TRAPPED != 0
                {
                    if options & WNOWAIT == 0 {
                        pr.ps_flags.fetch_or(PS_WAITED, Ordering::Relaxed);
                    }

                    retval[0] = pr.ps_pid as Register;
                    if let Some(info) = info.as_deref_mut() {
                        *info = wait_info(pr, &g, CLD_TRAPPED, pr.ps_xsig.get());
                    }

                    if let Some(statusp) = statusp.as_deref_mut() {
                        *statusp = w_stopcode(pr.ps_xsig.get());
                    }
                    drop(g);
                    if let Some(rusage) = rusage {
                        rusage.copy_from(&Rusage::default());
                    }
                    break 'scan Found::Reported;
                }
                if (flags & PS_TRACED != 0 || options & WUNTRACED != 0)
                    && flags & PS_WAITED == 0
                    && flags & PS_STOPPED != 0
                    && flags & PS_TRAPPED == 0
                {
                    if options & WNOWAIT == 0 {
                        pr.ps_flags.fetch_or(PS_WAITED, Ordering::Relaxed);
                    }

                    retval[0] = pr.ps_pid as Register;
                    if let Some(info) = info.as_deref_mut() {
                        *info = wait_info(pr, &g, CLD_STOPPED, pr.ps_xsig.get());
                    }

                    if let Some(statusp) = statusp.as_deref_mut() {
                        *statusp = w_stopcode(pr.ps_xsig.get());
                    }
                    drop(g);
                    if let Some(rusage) = rusage {
                        rusage.copy_from(&Rusage::default());
                    }
                    break 'scan Found::Reported;
                }
                if options & WCONTINUED != 0 && flags & PS_CONTINUED != 0 {
                    if options & WNOWAIT == 0 {
                        pr.ps_flags.fetch_and(!PS_CONTINUED, Ordering::Relaxed);
                    }

                    retval[0] = pr.ps_pid as Register;
                    if let Some(info) = info.as_deref_mut() {
                        *info = wait_info(pr, &g, CLD_CONTINUED, SIGCONT);
                    }

                    drop(g);
                    if let Some(statusp) = statusp.as_deref_mut() {
                        *statusp = _WCONTINUED;
                    }
                    if let Some(rusage) = rusage {
                        rusage.copy_from(&Rusage::default());
                    }
                    break 'scan Found::Reported;
                }
                drop(g);
            }
            // Look in the orphans list too, to allow the parent to collect its child's exit
            // status even if child is being debugged.
            //
            // Debugger detaches from the parent upon successful switch-over from parent to
            // child. At this point due to re-parenting the parent loses the child to
            // debugger and a wait4(2) call would report that it has no children to wait
            // for. By maintaining a list of orphans we allow the parent to successfully wait
            // until the child becomes a zombie.
            if nfound == 0 {
                for pr in qr.ps_orphans.iter(&r) {
                    if pr.ps_flags.load(Ordering::Relaxed) & PS_NOZOMBIE != 0
                        || !wait_matches(&r, pr, idtype, id)
                    {
                        continue;
                    }
                    nfound += 1;
                    break;
                }
            }
            if nfound == 0 {
                Found::None
            } else {
                Found::Waiting
            }
        };
        match found {
            Found::Reported => return Ok(()),
            Found::Zombie(pr) => {
                proc_finish_wait(q, pr);
                return Ok(());
            }
            Found::None => return Err(Errno::ECHILD),
            Found::Waiting => {}
        }
        if options & WNOHANG != 0 {
            retval[0] = 0;
            return Ok(());
        }
        sleep_setup(ptr::from_ref(qr).cast(), PWAIT | PCATCH, "wait");
        sleep_finish(
            INFSLP,
            qr.ps_flags.load(Ordering::Relaxed) & PS_WAITEVENT == 0,
        )?;
    }
}
/// `wait4(2)`.
pub fn sys_wait4(q: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysWait4Args = sysargs(v);
    let ru = Rusage::default();
    let pid = uap.pid.get();
    let mut options = uap.options.get();
    let mut status = 0i32;

    if options & !(WUNTRACED | WNOHANG | WCONTINUED) != 0 {
        return Err(Errno::EINVAL);
    }
    options |= WEXITED | WTRAPPED;

    let (idtype, id) = if pid == WAIT_MYPGRP {
        let r = klock::reader();
        (P_PGID, i64::from(q.process().pgid(&r)))
    } else if pid == WAIT_ANY {
        (P_ALL, 0)
    } else if pid < 0 {
        (P_PGID, -i64::from(pid))
    } else {
        (P_PID, i64::from(pid))
    };

    let statusp = uap.status.get() as usize;
    let rusagep = uap.rusage.get() as usize;
    dowait6(
        q,
        idtype,
        id,
        if statusp != 0 {
            Some(&mut status)
        } else {
            None
        },
        options,
        if rusagep != 0 { Some(&ru) } else { None },
        None,
        retval,
    )?;
    if retval[0] > 0 && statusp != 0 {
        copyout(&status.to_ne_bytes(), statusp)?;
    }
    if retval[0] > 0 && rusagep != 0 {
        copyout(&ru.to_bytes(), rusagep)?;
        // KTRACE: not configured.
    }
    Ok(())
}

/// `waitid(2)`.
pub fn sys_waitid(q: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysWaitidArgs = sysargs(v);
    let mut info = Siginfo::zeroed();
    let idtype = uap.idtype.get();
    let options = uap.options.get();

    if options & !(WSTOPPED | WCONTINUED | WEXITED | WTRAPPED | WNOHANG | WNOWAIT) != 0 {
        return Err(Errno::EINVAL);
    }
    if options & (WSTOPPED | WCONTINUED | WEXITED | WTRAPPED) == 0 {
        return Err(Errno::EINVAL);
    }
    if idtype != P_ALL && idtype != P_PID && idtype != P_PGID {
        return Err(Errno::EINVAL);
    }

    dowait6(
        q,
        idtype,
        uap.id.get() as i64,
        None,
        options,
        None,
        Some(&mut info),
        retval,
    )?;
    copyout_obj(&info, uap.info.get() as usize)?;
    // KTRACE: not configured.
    retval[0] = 0;
    Ok(())
}

/// `proc_finish_wait`: the waiter collected `pr`: give a ptrace-attached child back to its
/// old parent, or account for it and free it.
pub fn proc_finish_wait(waiter: &Proc, pr: ProcessRef) {
    // If we got the child via a ptrace 'attach', we need to give it back to the old
    // parent.
    let mut g = pr.ps_mtx.lock();
    // SAFETY: a non-null `ps_opptr` is a live process (ptrace keeps it).
    let opptr = unsafe { pr.ps_opptr.get().as_ref() };
    let pptr_addr = pr
        .pptr_locked(&g)
        .map_or(0, |pp| ptr::from_ref::<Process>(pp).addr());
    match opptr {
        Some(tr) if ptr::from_ref(tr).addr() != pptr_addr => {
            pr.ps_opptr.set(ptr::null());
            pr.ps_flags.fetch_and(!PS_TRACED, Ordering::Relaxed);
            let tr = process_ref(tr);
            process_reparent(&pr, &mut g, &tr);
            drop(g);
            prsignal(&tr, SIGCHLD);
            tr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
            wakeup(ptr::from_ref::<Process>(&tr));
        }
        _ => {
            drop(g);
            // SAFETY: a zombie keeps its main thread until `process_zap`.
            if let Some(child) = unsafe { pr.ps_mainproc.get().as_ref() } {
                scheduler_wait_hook(waiter, child);
            }
            let rup = &waiter.process().ps_cru;
            {
                let r = klock::reader();
                if let Some(ru) = pr.ps_ru.get(&r) {
                    ruadd(rup, ru);
                }
            }
            {
                let mut w = klock::witness();
                proc_tables().zombprocess.remove(&mut w, &pr); // off zombprocess
            }
            freepid(pr.ps_pid);
            process_zap(pr);
        }
    }
}

/// `process_untrace`: give process back to original parent or init(8).
pub fn process_untrace(pr: &ProcessRef, g: &mut PsGuard<'_>) {
    kassert!(pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0);

    // SAFETY: as in `proc_finish_wait`.
    let opptr = unsafe { pr.ps_opptr.get().as_ref() };
    let pptr_addr = pr
        .pptr_locked(g)
        .map_or(0, |pp| ptr::from_ref::<Process>(pp).addr());
    let ppr = opptr
        .filter(|op| ptr::from_ref(*op).addr() != pptr_addr)
        .map(process_ref);

    // not being traced any more
    pr.ps_opptr.set(ptr::null());
    pr.ps_flags.fetch_and(!PS_TRACED, Ordering::Relaxed);
    let to = ppr.unwrap_or_else(|| process_ref(initprocess_or_process0()));
    process_reparent(pr, g, &to);
}

/// `process_clear_orphan`: `pr` leaves the orphan list it is on, which is `owner`'s when the
/// caller knows it, else the one of the process `ps_opptr` names (found by address: see the
/// module's deviations).
pub fn process_clear_orphan(pr: &Process, owner: Option<&Process>) {
    if pr.ps_flags.load(Ordering::Relaxed) & PS_ORPHAN != 0 {
        let owner: Option<ProcessRef> = match owner {
            Some(o) => Some(process_ref(o)),
            None => {
                let r = klock::reader();
                let at = pr.ps_opptr.get().addr();
                proc_tables()
                    .allprocess
                    .iter(&r)
                    .find(|q| ptr::from_ref::<Process>(q).addr() == at)
                    .map(process_ref)
            }
        };
        if let Some(owner) = owner {
            let mut w = klock::witness();
            owner.ps_orphans.remove(&mut w, pr);
        }
        pr.ps_flags.fetch_and(!PS_ORPHAN, Ordering::Relaxed);
    }
}

/// `process_reparent`: make process `parent` the new parent of process `child`, whose
/// `ps_mtx` the caller holds (`g`).
pub fn process_reparent(child: &Process, g: &mut PsGuard<'_>, parent: &ProcessRef) {
    let old = child.pptr_locked(g).cloned();
    if old
        .as_ref()
        .is_some_and(|o| ptr::eq::<Process>(&**o, &**parent))
    {
        return;
    }

    // KASSERT(child->ps_opptr == NULL || child->ps_opptr == child->ps_pptr): ptrace (M7).

    {
        let mut w = klock::witness();
        if let Some(old) = &old {
            old.ps_children.remove(&mut w, child);
        }
        parent
            .ps_children
            .insert_head(&mut w, process_ref(child).as_pin());
    }

    process_clear_orphan(child, None);
    if child.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        child.ps_flags.fetch_or(PS_ORPHAN, Ordering::Relaxed);
        if let Some(old) = &old {
            let mut w = klock::witness();
            old.ps_orphans
                .insert_head(&mut w, process_ref(child).as_pin());
        }
    }

    let mut w = klock::witness();
    drop(child.set_pptr_locked(&mut w, g, parent.clone()));

    // WITNESS_SETCHILD: not configured.
}

/// `free(pr->ps_pin.pn_pins, M_PINSYSCALL, ...)` (and `ps_libcpin`): gives a process's pin
/// table back.
pub fn pin_free(pin: &crate::sys::proc::Pinsyscall) {
    if let Some(pins) = NonNull::new(pin.pn_pins.get_mut()) {
        crate::kern::kern_malloc::free(
            pins.cast::<u8>(),
            crate::sys::malloc::M_PINSYSCALL,
            pin.pn_npins.get().max(0) as usize * size_of::<u32>(),
        );
    }
    pin.pn_pins.set(ptr::null());
}

/// `process_zap`: finally finished with old proc entry. Unlink it from its process group and
/// free it: the main thread's box goes with `proc_free`, the process with its last
/// reference (`pr`, the caller's), taking its credentials and limits with it.
pub fn process_zap(pr: ProcessRef) {
    // SAFETY: a process always has its main thread until it is zapped here.
    let Some(p) = (unsafe { pr.ps_mainproc.get().as_ref() }) else {
        panic(format_args!("process_zap: no main thread"));
    };

    kernel::proc::leavepgrp(&pr);
    {
        let mut w = klock::witness();
        let parent = pr.pptr(&w).cloned();
        if let Some(parent) = parent {
            parent.ps_children.remove(&mut w, &pr);
        }
    }
    process_clear_orphan(&pr, None);

    // Decrement the count of procs running with this uid.
    chgproccnt(pr.ucred().cr_ruid.get(), -1);

    // Release reference to text vnode
    let otvp = pr.ps_textvp.get();
    pr.ps_textvp.set(None);
    if let Some(otvp) = otvp {
        crate::kern::vfs_subr::vrele(otvp);
    }

    kassert!(pr.ps_threadcnt.get() == 0);
    kassert!(pr.ps_exitcnt.get() == 1);
    let ru = {
        let mut w = klock::witness();
        pr.ps_ru.get_mut(&mut w).take()
    };
    drop(ru);
    {
        let g = pr.ps_mtx.lock();
        kassert!(pr.ps_threads.is_empty(&g));
        drop(g);
    }
    sigactsfree(pr.sigacts());
    // lim_free(pr->ps_limit), crfree(pr->ps_ucred): the process's references go with it.
    NPROCESSES.fetch_sub(1, Ordering::Relaxed);

    proc_free(p);
    drop(pr);
}
/* </CODE> */
