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
//! Process exit: `kern/kern_exit.c`: `exit1` and the system calls `exit(2)` and
//! `__threxit(2)`, the reaper, `wait4(2)` and `waitid(2)` (`dowait6`, `proc_finish_wait`),
//! the reparenting of children and orphans (`process_reparent`, `process_untrace`,
//! `process_clear_orphan`) and the end of a process (`process_zap`, `proc_free`).
//!
//! Upstream: sys/kern/kern_exit.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_exit.rs@3c62ede68802
//!
//! What is still `bsd`'s is reached through the process glue ([`ProcGlue`]): `init`'s death
//! (`init_exited`, the QEMU exit of the smokes), the interval timers (`cancel_all_itimers`,
//! N4.8a), the unveils (`unveil_destroy`, N8), the
//! machine's pmap (`pmap_deactivate`, `pmap_purge`, N4.8/N10).
//!
//! ## Deviations
//! - `stopprofclock`/`prof_write` (`subr_prof.c`) are reported once.
//! - `ptrace(2)` is not ported: nothing sets `ps_opptr` or `PS_TRACED`, so
//!   `proc_finish_wait` always takes the zombie's branch; the traced child's paths are kept.
//! - `deadproc`, `deadproc_mutex` and `exit2` are the scheduler's (`kernel::sched`, N4.5a).
//! - `initprocess` is process 0 until `main` forked init: the orphans `exit1` and
//!   `process_untrace` would hand to init go to process 0 then.
//! - Under QEMU (the glue's `init_exited`), init's exit ends the run (the smokes' criterion);
//!   the C panics.
//!
//! ## Redesign
//! - The thread's box: a secondary thread's (`P_THREAD`) is taken back from its process's
//!   tree by its `exit1` (one search by its joining order) and parked in the thread itself
//!   (`Thread::exited`) until the reaper takes it ([`Dead::Thread`]): the reaper owns it,
//!   frees its u-area and address space handle, and drops it (`proc_free`). The main
//!   thread's stays in its process's slot (`PsMtx.main`, insight G2): `exit1` only unlinks it
//!   from `ps_threads` (the C's `TAILQ_REMOVE`, no search); the reaper ([`Dead::Main`])
//!   frees its u-area and address space handle under `ps_mtx` without taking the box; and
//!   `process_zap` takes the box (`proc_free`) before the process's last reference goes. A
//!   thread's box has one owner at every instant (the slot or the tree while linked, `exited`
//!   or the slot while dead, a local while freed), and the main thread is reachable from its
//!   process under `ps_mtx` until it is zapped (no `ps_mainproc` word to dereference).
//! - `ps_single` (a word, compared) is turned back into its thread by a walk of
//!   `ps_threads` under `ps_mtx` (`Process::thread_by_addr`), and `ps_opptr` is a counted
//!   handle under `ps_mtx`: LZ dereferenced both words.
//! - `dowait6` takes its out-parameters as `Option<&mut>` and returns the pid (0 for
//!   `WNOHANG` with nothing to report).

use core::pin::Pin;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::klock;
use hal::pool::PoolBox;
use hal::sys::errno::Errno;
use hal::sys::siginfo::{
    CLD_CONTINUED, CLD_DUMPED, CLD_EXITED, CLD_KILLED, CLD_STOPPED, CLD_TRAPPED, Siginfo,
};
use hal::sys::types::Pid;
use hal::sys::wait::{
    _WCONTINUED, Idtype, P_ALL, P_PGID, P_PID, WAIT_ANY, WAIT_MYPGRP, WCONTINUED, WEXITED, WNOHANG,
    WNOWAIT, WSTOPPED, WTRAPPED, WUNTRACED, w_exitcode, w_stopcode, wcoredump, wstatus,
};
use hal::user::UserPtr;

use crate::assert::{kassert, kpanic};
use crate::fork::{NPROCESSES, NTHREADS, freepid};
use crate::proc::{
    EXIT_NORMAL, EXIT_THREAD, P_SYSTEM, P_THREAD, P_WEXIT, PS_CONTINUED, PS_EXITING, PS_ISPWAIT,
    PS_NOZOMBIE, PS_ORPHAN, PS_PPWAIT, PS_PROFIL, PS_STOPPED, PS_STOPPING, PS_TRACED, PS_TRAPPED,
    PS_WAITED, PS_WAITEVENT, PS_ZOMBIE, ProcGlue, Process, ProcessRef, PsMtx, SDEAD, SINGLE_EXIT,
    Thread,
};
use crate::resource::{Rusage, calcru, ruadd};
use crate::sig::{SAS_NOCLDWAIT, SignalType};

/// `SIGCHLD`.
const SIGCHLD: i32 = 20;
/// `SIGCONT`.
const SIGCONT: i32 = 19;
/// `SIGKILL`.
const SIGKILL: i32 = 9;
/// `SIGSEGV`.
const SIGSEGV: i32 = 11;
/// `PWAIT`.
const PWAIT: i32 = 32;
/// `PCATCH`.
const PCATCH: i32 = 0x100;

/// The guard of a process's `ps_mtx`.
type PsGuard<'a, G> = hal::mutex::MutexedGuard<'a, PsMtx<G>>;

/// `exit(2)`: death of process.
pub fn sys_exit<G: ProcGlue>(p: &Thread<G>, rval: i32) -> ! {
    G::exit1(p, rval, 0, EXIT_NORMAL)
}

/// `__threxit(2)`: the calling thread exits; the process goes on unless it was the last
/// thread. `*notdead` (when not NULL) is cleared first, which is how `pthread_join` learns
/// the thread is gone (a fault there is a `SIGSEGV`).
#[allow(non_snake_case, reason = "the C name, sys___threxit")]
pub fn sys___threxit<G: ProcGlue>(p: &Thread<G>, notdead: UserPtr<Pid>) -> ! {
    if !notdead.is_null() && notdead.copyout(&0).is_err() {
        hal::thread::with_current::<Thread<G>, _>(|cur| crate::sig::psignal(cur, SIGSEGV));
    }
    G::exit1(p, 0, 0, EXIT_THREAD)
}

/// `exit1`: the thread `p` (the running one) exits, and its process with it when it is the
/// last (`EXIT_NORMAL`, or the only thread): the address space and the other resources
/// released, the process made a zombie and unlinked from `allprocess` and its parent's
/// lists, its exit status and usage saved for `wait`, its children and orphans handed on.
/// Never returns: the thread ends in `sched_exit`.
pub fn exit1<G: ProcGlue>(p: &Thread<G>, xexit: i32, xsig: i32, flags: i32) -> ! {
    let mut flags = flags;

    p.p_flag.fetch_or(P_WEXIT, Ordering::Relaxed);

    let pr = p.process_ref().clone();
    let main = p.p_flag.load(Ordering::Relaxed) & P_THREAD == 0;

    // single-threaded?
    if !p.has_sibling() {
        flags = EXIT_NORMAL;
    } else if flags == EXIT_NORMAL {
        // nope, multi-threaded
        let _ = crate::sig::single_thread_set(p, SINGLE_EXIT);
    }

    if flags == EXIT_NORMAL && pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING == 0 {
        if pr.ps_pid == 1 {
            G::init_exited(xexit, xsig);
        }

        pr.ps_flags.fetch_or(PS_EXITING, Ordering::Relaxed);
        pr.ps_xexit.set(xexit as u32);
        pr.ps_xsig.set(xsig);

        // If parent is waiting for us to exit or exec, PS_PPWAIT is set; we wake up the
        // parent early to avoid deadlock.
        if pr.ps_flags.load(Ordering::Relaxed) & PS_PPWAIT != 0 {
            pr.ps_flags.fetch_and(!PS_PPWAIT, Ordering::Relaxed);
            let pptr = parent(&pr);
            pptr.ps_flags.fetch_and(!PS_ISPWAIT, Ordering::Relaxed);
            pptr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
            wakeup_process::<G>(&pptr);
        }

        // Wait for concurrent `allprocess' loops
        pr.ps_refcnt.finalize("psdtor");
    }

    // unlink ourselves from the active threads: the main thread's box stays its process's,
    // a secondary thread takes its own back until the reaper frees it.
    let mut g = pr.ps_mtx.lock();
    pr.leave_locked(&mut g, p);
    let mybox = if main {
        None
    } else {
        let b = pr.take_box_locked(&mut g, p.p_seq.get());
        kassert!(
            b.as_ref()
                .is_some_and(|b| ptr::eq::<Thread<G>>(&raw const **b, p))
        );
        b
    };
    pr.ps_threadcnt.set(pr.ps_threadcnt.get() - 1);
    pr.ps_exitcnt.set(pr.ps_exitcnt.get() + 1);

    // if somebody else wants to take us to single threaded mode or stop us, count ourselves
    // out.
    if !pr.ps_single.get().is_null() || pr.ps_flags.load(Ordering::Relaxed) & PS_STOPPING != 0 {
        crate::sig::process_suspend_signal(&pr, &g);
    }

    // proc is off ps_threads list so update accounting of process now
    G::tuagg_add_runtime();
    crate::resource::tuagg_add_process(&pr, p, &g);

    if main {
        // main thread gotta wait because it has the pid, et al
        while pr.ps_threadcnt.get() + pr.ps_exitcnt.get() > 1 {
            let _ = g.sleep_nsec(
                ptr::from_ref(&pr.ps_threads),
                PWAIT,
                "thrdeath",
                hal::sleep::INFSLP,
            );
        }
    }
    drop(g);
    if let Some(b) = mybox {
        drop(p.exited.lock().replace(b));
    }

    // The process's rusage (`ps_ru`), made by the first thread to exit.
    let need_ru = {
        let r = klock::reader();
        pr.ps_ru.get(&r).is_none()
    };
    if need_ru {
        let ru = crate::proc::rusage_alloc();
        let mut w = klock::witness();
        let slot = pr.ps_ru.get_mut(&mut w);
        if slot.is_none() {
            *slot = Some(ru);
        }
        // else another thread installed the process's rusage meanwhile; `ru` goes back
        // after the witness.
    }
    p.p_siglist.store(0, Ordering::Relaxed);
    if main {
        pr.ps_siglist.store(0, Ordering::Relaxed);
    }

    crate::event::kqpoll_exit(p);

    // kcov_exit(p): kcov is not configured.

    if main {
        if pr.ps_flags.load(Ordering::Relaxed) & PS_PROFIL != 0 {
            let _ = crate::prf::unported!("exit1: stopprofclock (M7)");
        }
        // prof_write(p): subr_prof.c.

        crate::sigio::sigio_freelist_process(&pr);

        // close open files and release open-file table
        crate::descrip::fdfree(p);

        G::cancel_all_itimers();

        pr.ps_rucheck_to.del();
        // SYSVSEM: not configured.
        crate::proc::killjobc(&pr);
        let _ = crate::acct::acct_process(p);
        // KTRACE: not configured.

        G::unveil_destroy(&pr);

        drop(pr.ps_pin.replace(crate::proc::PinTable::NONE));
        drop(pr.ps_libcpin.replace(crate::proc::PinTable::NONE));

        // If parent has the SAS_NOCLDWAIT flag set, we're not going to become a zombie.
        if parent(&pr).sigacts().ps_sigflags.load(Ordering::Relaxed) & SAS_NOCLDWAIT != 0 {
            pr.ps_flags.fetch_or(PS_NOZOMBIE, Ordering::Relaxed);
        }

        // Teardown the virtual address space.
        if p.p_flag.load(Ordering::Relaxed) & P_SYSTEM == 0 {
            // exit1() might be called with a lock count greater than one and the costly
            // teardown runs unlocked: release them all, since exit1() will not return.
            let _ = klock::release_all();
            uvm_purge(p);
            klock::kernel_lock();
        }
    }

    // p->p_fd = NULL: the thread's copy is its process's table, emptied by fdfree.

    // Release the thread's read reference of resource limit structure (its copy).
    p.p_limit.clear();

    // Remove proc from pidhash chain and allproc so looking it up won't work. The reaper
    // gets it from the deadproc list (p_runq). If this is the last thread of a process that
    // isn't PS_NOZOMBIE, the process goes on the zombprocess list below.
    //
    // NOTE: WE ARE NO LONGER ALLOWED TO SLEEP!
    p.set_stat(SDEAD);

    {
        let mut w = klock::witness();
        let t = G::tables();
        t.tidhash(p.p_tid.get()).remove(&mut w, p);
        t.allproc.remove(&mut w, p);
    }

    if main {
        {
            let mut w = klock::witness();
            let t = G::tables();
            t.pidhash(pr.ps_pid).remove(&mut w, &*pr);
            t.allprocess.remove(&mut w, &*pr);
            if pr.ps_flags.load(Ordering::Relaxed) & PS_NOZOMBIE == 0 {
                t.zombprocess.insert_head(&mut w, pr.as_pin());
            }
        }
        if pr.ps_flags.load(Ordering::Relaxed) & PS_NOZOMBIE != 0 {
            // Not going to be a zombie, so it's now off all the lists scanned by
            // ispidtaken(), so block fast reuse of the pid now.
            freepid(pr.ps_pid);
        }

        // Reparent children to their original parent, in case they were being traced, or to
        // init(8).
        let initprocess = G::tables().initprocess_or_process0();
        let any = {
            let r = klock::reader();
            !pr.ps_children.is_empty(&r)
        };
        if any {
            // only need this if any child is S_ZOMB
            wakeup_process::<G>(&initprocess);
        }
        // Each pass takes the first child off this process's list (reparenting moves it).
        while let Some(child) = {
            let r = klock::reader();
            pr.ps_children.first(&r).map(G::process_ref)
        } {
            // Traced processes are killed since their existence means someone is screwing
            // up.
            let mut cg = child.ps_mtx.lock();
            if child.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
                process_untrace(&child, &mut cg);
                if child.ps_flags.load(Ordering::Relaxed) & PS_EXITING == 0 {
                    // If single threading is active, direct the signal to the active thread
                    // to avoid deadlock.
                    let single = child.ps_single.get().addr();
                    let sent = match child.thread_by_addr(&cg, single) {
                        Some(t) => {
                            crate::sig::ptsignal_locked(t, SIGKILL, SignalType::STHREAD, &cg);
                            true
                        }
                        None => false,
                    };
                    drop(cg);
                    if !sent {
                        crate::sig::prsignal(&child, SIGKILL);
                    }
                } else {
                    drop(cg);
                }
            } else {
                process_reparent(&child, &mut cg, &initprocess);
                drop(cg);
            }
        }

        // Make sure orphans won't remember the exiting process.
        while let Some(orphan) = {
            let r = klock::reader();
            pr.ps_orphans.first(&r).map(G::process_ref)
        } {
            let mut og = orphan.ps_mtx.lock();
            kassert!(
                orphan
                    .opptr_locked(&og)
                    .is_some_and(|o| ptr::eq::<Process<G>>(&raw const **o, &raw const *pr))
            );
            drop(orphan.set_opptr_locked(&mut og, None));
            process_clear_orphan(&orphan, Some(&pr));
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

    if main {
        // Final thread has died, so add on our children's rusage and calculate the total
        // times.
        {
            let r = klock::reader();
            if let Some(rup) = pr.ps_ru.get(&r) {
                let (utime, stime, _) = calcru(&pr.ps_tu, G::stathz());
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
            let ppr = parent_locked(&pr, &g);
            process_reparent(&pr, &mut g, &G::tables().initprocess_or_process0());
            ppr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
            wakeup_process::<G>(&ppr);
        }
        drop(g);
    } else {
        // just a thread? check if last one standing.
        // scheduler_wait_hook(pr->ps_mainproc, p); XXX
        let g = pr.ps_mtx.lock();
        pr.ps_exitcnt.set(pr.ps_exitcnt.get() - 1);
        if pr.ps_threadcnt.get() + pr.ps_exitcnt.get() == 1 {
            crate::sched::wakeup_n::<G>(ptr::from_ref(&pr.ps_threads).addr(), -1);
        }
        drop(g);
    }

    // Other substructures are freed from reaper and wait().

    // cpu_exit(p): nothing on either machine (vm_machdep.c:116-118).

    // Deactivate the exiting address space before the vmspace is freed. We continue to run
    // on this vmspace's context until the switch to idle in sched_exit(); the reaper frees
    // the rest once we are off it.
    G::pmap_deactivate(p);
    drop(pr);
    hal::thread::with_current::<Thread<G>, _>(|cur| {
        kassert!(ptr::eq::<Thread<G>>(cur.get_ref(), p));
        crate::sched::sched_exit(cur)
    });
    kpanic(format_args!("sched_exit returned"))
}

/// `uvm_purge`: tear down the dying process's address space, from its last thread: the
/// machine's pmap first (`pmap_purge`, `__HAVE_PMAP_PURGE`), then the map's entries.
fn uvm_purge<G: ProcGlue>(p: &Thread<G>) {
    kassert!(!klock::kernel_lock_held());
    G::pmap_purge(p);
    if let Some(vm) = p.vmspace_share() {
        crate::vm::map::uvmspace_purge(&vm);
    }
}

/// `wakeup(pr)`: the channel a process's parent sleeps on in `wait` ("wait") and `vfork`
/// ("ppwait").
fn wakeup_process<G: ProcGlue>(pr: &Process<G>) {
    crate::sched::wakeup_n::<G>(ptr::from_ref(pr).addr(), -1);
}

/// `pr->ps_pptr`, counted, for a holder of the kernel lock: a process always has a parent
/// (process 0 is its own).
fn parent<G: ProcGlue>(pr: &Process<G>) -> ProcessRef<G> {
    let r = klock::reader();
    match pr.pptr(&r) {
        Some(pp) => pp.clone(),
        None => G::tables().process0(),
    }
}

/// `pr->ps_pptr`, counted, for a holder of its `ps_mtx` (`[K|m]`).
fn parent_locked<G: ProcGlue>(pr: &Process<G>, g: &PsGuard<'_, G>) -> ProcessRef<G> {
    match pr.pptr_locked(g) {
        Some(pp) => pp.clone(),
        None => G::tables().process0(),
    }
}

/// `proc_free`: a dead thread's box returns to its pool (its credentials' reference, its
/// address space handle and its process's reference with it).
pub fn proc_free<G: ProcGlue>(b: PoolBox<Thread<G>>) {
    NTHREADS.fetch_sub(1, Ordering::Relaxed);
    drop(b);
}

/// What the reaper took off the dead threads' queue.
pub enum Dead<G: ProcGlue> {
    /// A secondary thread, whose box the reaper now owns.
    Thread(PoolBox<Thread<G>>),
    /// A main thread, whose box stays its process's (until `process_zap`).
    Main(ProcessRef<G>),
}

/// `reaper`: process reaper, a kernel thread that frees the resources of dead threads. Once
/// the resources of a process's last thread are free, the process becomes a zombie, and
/// the parent is allowed to read the undead's status. Never returns.
pub fn reaper<G: ProcGlue>() -> ! {
    klock::kernel_unlock();

    kassert!(!hal::thread::sched_locked());

    loop {
        // The first dead thread, off the deadproc list (`deadproc_mutex` is the scheduler's,
        // which `exit2` and the idle threads fill).
        let dead = crate::sched::reaper_next::<G, _>(|p| {
            if p.p_flag.load(Ordering::Relaxed) & P_THREAD != 0 {
                match p.exited.lock().take() {
                    Some(b) => Dead::Thread(b),
                    None => kpanic(format_args!("reaper: thread {} has no box", p.p_tid.get())),
                }
            } else {
                Dead::Main(p.process_ref().clone())
            }
        });

        // WITNESS_THREAD_EXIT(p): not configured.

        match dead {
            Dead::Thread(b) => {
                // Free the VM resources we're still holding on to. We must do this from a
                // valid thread because doing so may block.
                drop(b.take_uarea());
                drop(b.set_vmspace(None)); // zap the thread's copy
                // Just a thread
                proc_free(b);
            }
            Dead::Main(pr) => {
                // The main thread's u-area and address space handle, taken under `ps_mtx`
                // (the box stays its process's) and freed outside it.
                let (uarea, vm) = {
                    let mut g = pr.ps_mtx.lock();
                    match pr.take_main_locked(&mut g) {
                        Some(b) => {
                            let vm = b.take_vmspace_locked(&mut g);
                            let u = b.take_uarea();
                            pr.set_main_locked(&mut g, b);
                            (u, vm)
                        }
                        None => kpanic(format_args!(
                            "reaper: process {} has no main thread",
                            pr.ps_pid
                        )),
                    }
                };
                drop(uarea);
                drop(vm);

                // Release the rest of the process's vmspace (uvm_exit).
                if let Some(vm) = pr.set_vmspace(None) {
                    crate::vm::map::uvmspace_free(vm);
                }

                klock::kernel_lock(); // KERNEL_LOCK()
                if pr.ps_flags.load(Ordering::Relaxed) & PS_NOZOMBIE == 0 {
                    // Process is now a true zombie.
                    pr.ps_flags.fetch_or(PS_ZOMBIE, Ordering::Relaxed);
                }

                // Notify listeners of our demise and clean up.
                crate::event::knote_processexit(&pr);

                if pr.ps_flags.load(Ordering::Relaxed) & PS_ZOMBIE != 0 {
                    // Post SIGCHLD and wake up parent.
                    let pptr = parent(&pr);
                    crate::sig::prsignal(&pptr, SIGCHLD);
                    pptr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
                    wakeup_process::<G>(&pptr);
                } else {
                    // No one will wait for us, just zap it.
                    process_zap(pr);
                }
                klock::kernel_unlock(); // KERNEL_UNLOCK()
            }
        }
    }
}

/// `start_reaper`: `main`'s `kthread_create(reaper, NULL, NULL, "reaper")`.
pub fn start_reaper<G: ProcGlue>() -> Result<Pid, Errno> {
    crate::thread::kthread_create::<G>(b"reaper", crate::fork::Start::Reaper)
}

/// The id `dowait6` compares a process with: its pid or its process group's id.
fn wait_matches<G: ProcGlue>(
    r: &klock::KernelReader,
    pr: &Process<G>,
    idtype: Idtype,
    id: i64,
) -> bool {
    !((idtype == P_PID && id != i64::from(pr.ps_pid))
        || (idtype == P_PGID && id != i64::from(pr.pgid(r))))
}

/// The `siginfo_t` `dowait6` fills for a child: who, as whom, and the `CLD_*` code; read
/// under the child's `ps_mtx` (`g`), which `dowait6` holds (the C reads
/// `pr->ps_ucred->cr_uid` in the same section).
fn wait_info<G: ProcGlue>(pr: &Process<G>, g: &PsGuard<'_, G>, code: i32, status: i32) -> Siginfo {
    let mut info = Siginfo::zeroed();
    info.set_si_pid(pr.ps_pid);
    info.set_si_uid(g.ucred().cr_uid.get());
    info.si_signo = SIGCHLD;
    info.si_code = code;
    info.set_si_status(status);
    info
}

/// What `dowait6` found among the children, decided under the kernel lock's reader.
enum Found<G: ProcGlue> {
    /// A child reported: its pid.
    Reported(Pid),
    /// A zombie to collect (`proc_finish_wait`), held past the reader: its pid.
    Zombie(ProcessRef<G>),
    /// Children matched, none had anything to report.
    Waiting,
    /// No child matched.
    None,
}

/// `dowait6`: the body of `wait4(2)` and `waitid(2)`: finds a child of `q` that matches
/// `idtype`/`id` and has something to report under `options` (exited, trapped, stopped or
/// continued), fills `statusp`, `rusage` and `info`, and returns its pid; sleeps for one
/// unless `WNOHANG` (then 0). `ECHILD` when no child matches.
pub fn dowait6<G: ProcGlue>(
    q: &Thread<G>,
    idtype: Idtype,
    id: i64,
    mut statusp: Option<&mut i32>,
    options: i32,
    rusage: Option<&Rusage>,
    mut info: Option<&mut Siginfo>,
) -> Result<Pid, Errno> {
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
                        break 'scan Found::Zombie(G::process_ref(pr));
                    }
                    break 'scan Found::Reported(pr.ps_pid);
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
                    if let Some(info) = info.as_deref_mut() {
                        *info = wait_info(pr, &g, CLD_TRAPPED, pr.ps_xsig.get());
                    }
                    if let Some(statusp) = statusp.as_deref_mut() {
                        *statusp = w_stopcode(pr.ps_xsig.get());
                    }
                    drop(g);
                    if let Some(rusage) = rusage {
                        rusage.copy_from(&Rusage::new());
                    }
                    break 'scan Found::Reported(pr.ps_pid);
                }
                if (flags & PS_TRACED != 0 || options & WUNTRACED != 0)
                    && flags & PS_WAITED == 0
                    && flags & PS_STOPPED != 0
                    && flags & PS_TRAPPED == 0
                {
                    if options & WNOWAIT == 0 {
                        pr.ps_flags.fetch_or(PS_WAITED, Ordering::Relaxed);
                    }
                    if let Some(info) = info.as_deref_mut() {
                        *info = wait_info(pr, &g, CLD_STOPPED, pr.ps_xsig.get());
                    }
                    if let Some(statusp) = statusp.as_deref_mut() {
                        *statusp = w_stopcode(pr.ps_xsig.get());
                    }
                    drop(g);
                    if let Some(rusage) = rusage {
                        rusage.copy_from(&Rusage::new());
                    }
                    break 'scan Found::Reported(pr.ps_pid);
                }
                if options & WCONTINUED != 0 && flags & PS_CONTINUED != 0 {
                    if options & WNOWAIT == 0 {
                        pr.ps_flags.fetch_and(!PS_CONTINUED, Ordering::Relaxed);
                    }
                    if let Some(info) = info.as_deref_mut() {
                        *info = wait_info(pr, &g, CLD_CONTINUED, SIGCONT);
                    }
                    drop(g);
                    if let Some(statusp) = statusp.as_deref_mut() {
                        *statusp = _WCONTINUED;
                    }
                    if let Some(rusage) = rusage {
                        rusage.copy_from(&Rusage::new());
                    }
                    break 'scan Found::Reported(pr.ps_pid);
                }
                drop(g);
            }
            // Look in the orphans list too, to allow the parent to collect its child's exit
            // status even if child is being debugged: the debugger's switch-over reparents the
            // child, and without this list wait4(2) would report no children.
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
            Found::Reported(pid) => return Ok(pid),
            Found::Zombie(pr) => {
                let pid = pr.ps_pid;
                proc_finish_wait(q, pr);
                return Ok(pid);
            }
            Found::None => return Err(Errno::ECHILD),
            Found::Waiting => {}
        }
        if options & WNOHANG != 0 {
            return Ok(0);
        }
        crate::sched::sleep_setup::<G>(ptr::from_ref(qr).addr(), PWAIT | PCATCH, "wait");
        crate::sched::sleep_finish::<G>(
            hal::sleep::INFSLP,
            qr.ps_flags.load(Ordering::Relaxed) & PS_WAITEVENT == 0,
        )?;
    }
}

/// `wait4(2)`: the child's pid (0 for `WNOHANG` with nothing to report); its status and
/// usage copied out where asked.
pub fn sys_wait4<G: ProcGlue>(
    q: &Thread<G>,
    pid: Pid,
    status: UserPtr<i32>,
    options: i32,
    rusage: UserPtr<u8>,
) -> Result<Pid, Errno> {
    let ru = Rusage::new();
    let mut st = 0i32;

    if options & !(WUNTRACED | WNOHANG | WCONTINUED) != 0 {
        return Err(Errno::EINVAL);
    }
    let options = options | WEXITED | WTRAPPED;

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

    let r = dowait6(
        q,
        idtype,
        id,
        (!status.is_null()).then_some(&mut st),
        options,
        (!rusage.is_null()).then_some(&ru),
        None,
    )?;
    if r > 0 && !status.is_null() {
        status.copyout(&st)?;
    }
    if r > 0 && !rusage.is_null() {
        rusage.copyout_slice(&ru.to_bytes())?;
        // KTRACE: not configured.
    }
    Ok(r)
}

/// `waitid(2)`: the child's `siginfo_t` copied out to `info` (zero when `WNOHANG` found
/// nothing).
pub fn sys_waitid<G: ProcGlue>(
    q: &Thread<G>,
    idtype: Idtype,
    id: i64,
    info: UserPtr<Siginfo>,
    options: i32,
) -> Result<(), Errno> {
    let mut si = Siginfo::zeroed();

    if options & !(WSTOPPED | WCONTINUED | WEXITED | WTRAPPED | WNOHANG | WNOWAIT) != 0 {
        return Err(Errno::EINVAL);
    }
    if options & (WSTOPPED | WCONTINUED | WEXITED | WTRAPPED) == 0 {
        return Err(Errno::EINVAL);
    }
    if idtype != P_ALL && idtype != P_PID && idtype != P_PGID {
        return Err(Errno::EINVAL);
    }

    dowait6(q, idtype, id, None, options, None, Some(&mut si))?;
    info.copyout(&si)
    // KTRACE: not configured.
}

/// `proc_finish_wait`: the waiter collected the zombie `pr`: give a ptrace-attached child
/// back to its old parent, or account for it and free it.
pub fn proc_finish_wait<G: ProcGlue>(waiter: &Thread<G>, pr: ProcessRef<G>) {
    // If we got the child via a ptrace 'attach', we need to give it back to the old parent.
    let mut g = pr.ps_mtx.lock();
    let pptr_addr = pr
        .pptr_locked(&g)
        .map_or(0, super::proc::StaticOrPooled::addr);
    let tr = pr
        .opptr_locked(&g)
        .filter(|tr| tr.addr() != pptr_addr)
        .cloned();
    if let Some(tr) = tr {
        drop(pr.set_opptr_locked(&mut g, None));
        pr.ps_flags.fetch_and(!PS_TRACED, Ordering::Relaxed);
        process_reparent(&pr, &mut g, &tr);
        drop(g);
        crate::sig::prsignal(&tr, SIGCHLD);
        tr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
        wakeup_process::<G>(&tr);
    } else {
        // The zombie keeps its main thread until process_zap; `g` held across the hook,
        // which reads p_estcpu only (insight G3).
        if let Some(child) = pr.main_thread(&g) {
            // scheduler_wait_hook(waiter, child)
            waiter.p_estcpu.set(crate::sched::estcpulim(
                waiter.p_estcpu.get() + child.p_estcpu.get(),
            ));
        }
        drop(g);
        let rup = &waiter.process().ps_cru;
        {
            let r = klock::reader();
            if let Some(ru) = pr.ps_ru.get(&r) {
                ruadd(rup, ru);
            }
        }
        {
            let mut w = klock::witness();
            G::tables().zombprocess.remove(&mut w, &*pr); // off zombprocess
        }
        freepid(pr.ps_pid);
        process_zap(pr);
    }
}

/// `process_untrace`: give process back to original parent or init(8) (the caller holds its
/// `ps_mtx`, `g`).
pub fn process_untrace<G: ProcGlue>(pr: &ProcessRef<G>, g: &mut PsGuard<'_, G>) {
    kassert!(pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0);

    let pptr_addr = pr
        .pptr_locked(g)
        .map_or(0, super::proc::StaticOrPooled::addr);
    let ppr = pr
        .opptr_locked(g)
        .filter(|op| op.addr() != pptr_addr)
        .cloned();

    // not being traced any more
    drop(pr.set_opptr_locked(g, None));
    pr.ps_flags.fetch_and(!PS_TRACED, Ordering::Relaxed);
    let to = ppr.unwrap_or_else(|| G::tables().initprocess_or_process0());
    process_reparent(pr, g, &to);
}

/// `process_clear_orphan`: an orphan `pr` (`PS_ORPHAN`) leaves the orphan list of `owner`,
/// its `ps_opptr` (the caller's read: under `pr`'s `ps_mtx`, which every caller holds or
/// takes).
pub fn process_clear_orphan<G: ProcGlue>(pr: &Process<G>, owner: Option<&ProcessRef<G>>) {
    if pr.ps_flags.load(Ordering::Relaxed) & PS_ORPHAN != 0 {
        if let Some(owner) = owner {
            let mut w = klock::witness();
            owner.ps_orphans.remove(&mut w, pr);
        }
        pr.ps_flags.fetch_and(!PS_ORPHAN, Ordering::Relaxed);
    }
}

/// `process_reparent`: make process `parent` the new parent of process `child`, whose
/// `ps_mtx` the caller holds (`g`).
pub fn process_reparent<G: ProcGlue>(
    child: &Process<G>,
    g: &mut PsGuard<'_, G>,
    parent: &ProcessRef<G>,
) {
    let old = child.pptr_locked(g).cloned();
    if old
        .as_ref()
        .is_some_and(|o| ptr::eq::<Process<G>>(&raw const **o, &raw const **parent))
    {
        return;
    }

    let opptr = child.opptr_locked(g).cloned();
    kassert!(
        opptr.is_none()
            || opptr.as_ref().map(super::proc::StaticOrPooled::addr)
                == old.as_ref().map(super::proc::StaticOrPooled::addr)
    );

    {
        let mut w = klock::witness();
        if let Some(old) = &old {
            old.ps_children.remove(&mut w, child);
        }
        parent
            .ps_children
            .insert_head(&mut w, G::process_ref(child).as_pin());
    }

    process_clear_orphan(child, opptr.as_ref());
    if child.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        child.ps_flags.fetch_or(PS_ORPHAN, Ordering::Relaxed);
        if let Some(old) = &old {
            let mut w = klock::witness();
            old.ps_orphans
                .insert_head(&mut w, G::process_ref(child).as_pin());
        }
    }

    let mut w = klock::witness();
    drop(child.set_pptr_locked(&mut w, g, parent.clone()));

    // WITNESS_SETCHILD: not configured.
}

/// `process_zap`: finally finished with old proc entry: unlinked from its process group and
/// its parent, uncharged, its main thread's box freed (`proc_free`) and the process with its
/// last reference (`pr`, the caller's), its credentials and limits with it.
pub fn process_zap<G: ProcGlue>(pr: ProcessRef<G>) {
    crate::proc::leavepgrp(&pr);
    {
        let mut w = klock::witness();
        let parent = pr.pptr(&w).cloned();
        if let Some(parent) = parent {
            parent.ps_children.remove(&mut w, &*pr);
        }
    }
    let opptr = pr.opptr_locked(&pr.ps_mtx.lock()).cloned();
    process_clear_orphan(&pr, opptr.as_ref());

    // Decrement the count of procs running with this uid.
    crate::proc::chgproccnt::<G>(pr.ucred().cr_ruid.get(), -1);

    // Release reference to text vnode
    if let Some(otvp) = pr.textvp.replace(None) {
        crate::fs::subr::vrele(otvp);
    }

    kassert!(pr.ps_threadcnt.get() == 0);
    kassert!(pr.ps_exitcnt.get() == 1);
    let ru = {
        let mut w = klock::witness();
        pr.ps_ru.get_mut(&mut w).take()
    };
    drop(ru);
    let main = {
        let mut g = pr.ps_mtx.lock();
        kassert!(pr.ps_threads.is_empty(&g));
        pr.take_main_locked(&mut g)
    };
    // sigactsfree(pr->ps_sigacts), lim_free(pr->ps_limit), crfree(pr->ps_ucred): the
    // process's references go with it.
    NPROCESSES.fetch_sub(1, Ordering::Relaxed);

    match main {
        Some(b) => proc_free(b),
        None => kpanic(format_args!("process_zap: no main thread")),
    }
    drop(pr);
}

/// `p` as the pinned running thread for `f` (`curproc`), checked to be `p`.
pub fn with_self<G: ProcGlue, R>(p: &Thread<G>, f: impl FnOnce(Pin<&Thread<G>>) -> R) -> R {
    hal::thread::with_current::<Thread<G>, _>(|cur| {
        kassert!(ptr::eq::<Thread<G>>(cur.get_ref(), p));
        f(cur)
    })
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// A thread of a new process `pid`, the tables made once per test thread.
    fn thread(pid: Pid) -> (ProcessRef<Procs>, PoolBox<Thread<Procs>>) {
        std::thread_local! {
            static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        crate::testutil::install();
        if !MADE.with(|m| m.replace(true)) {
            // As a system call runs: the kernel lock held (the double counts it per thread).
            hal::klock::kernel_lock();
            crate::proc::procinit::<Procs>(64, 64);
        }
        let pr = crate::proc::process_alloc::<Procs>(pid, ());
        drop(pr.set_ucred(crate::cred::CredRef::new(crate::cred::Ucred::new())));
        let t = crate::proc::thread_alloc::<Procs>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        (pr, t)
    }

    /// wait4(2)'s options (kern_exit.c:701-702): only `WUNTRACED`, `WNOHANG` and
    /// `WCONTINUED`; anything else is `EINVAL` before any child is looked at. A valid call
    /// with no child is `ECHILD`, with `WNOHANG` too (the C checks the children first).
    #[test]
    fn wait4_takes_three_options_and_wants_a_child() {
        let (_pr, q) = thread(700);
        let null = UserPtr::<i32>::new(0);
        let none = UserPtr::<u8>::new(0);
        for bad in [
            WEXITED,
            WNOWAIT,
            WTRAPPED,
            0x40,
            i32::MIN,
            WNOHANG | WNOWAIT,
        ] {
            assert_eq!(
                sys_wait4(&q, WAIT_ANY, null, bad, none),
                Err(Errno::EINVAL),
                "options {bad:#x}"
            );
        }
        for ok in [
            0,
            WNOHANG,
            WUNTRACED,
            WCONTINUED,
            WUNTRACED | WNOHANG | WCONTINUED,
        ] {
            for pid in [WAIT_ANY, 701, -700] {
                assert_eq!(
                    sys_wait4(&q, pid, null, ok, none),
                    Err(Errno::ECHILD),
                    "options {ok:#x}, pid {pid}"
                );
            }
        }
    }

    /// waitid(2)'s checks (kern_exit.c:749-754): no unknown option, at least one of the
    /// events, and an `idtype` of `P_ALL`, `P_PID` or `P_PGID`; each failure `EINVAL`.
    #[test]
    fn waitid_wants_known_options_an_event_and_an_idtype() {
        let (_pr, q) = thread(710);
        let info = UserPtr::<Siginfo>::new(0);
        let rows = [
            (P_ALL, 0x40 | WEXITED, Errno::EINVAL),
            (P_ALL, WUNTRACED << 8 | WEXITED, Errno::EINVAL),
            (P_ALL, 0, Errno::EINVAL),
            (P_ALL, WNOHANG, Errno::EINVAL),
            (P_ALL, WNOHANG | WNOWAIT, Errno::EINVAL),
            (3, WEXITED, Errno::EINVAL),
            (-1, WEXITED, Errno::EINVAL),
            (P_ALL, WEXITED, Errno::ECHILD),
            (P_PID, WSTOPPED | WNOHANG, Errno::ECHILD),
            (P_PGID, WCONTINUED, Errno::ECHILD),
            (P_ALL, WTRAPPED | WNOWAIT, Errno::ECHILD),
        ];
        for (i, &(idtype, options, e)) in rows.iter().enumerate() {
            assert_eq!(sys_waitid(&q, idtype, 0, info, options), Err(e), "row {i}");
        }
    }

    /// A live child with nothing to report: `WNOHANG` answers 0 (kern_exit.c:674-677), and
    /// `waitid` copies out a zeroed `siginfo_t`.
    #[test]
    fn wnohang_with_a_live_child_answers_zero() {
        let (pr, q) = thread(720);
        let (child, _ct) = thread(721);
        {
            let mut w = klock::witness();
            pr.ps_children.insert_head(&mut w, child.as_pin());
        }
        let null = UserPtr::<i32>::new(0);
        let none = UserPtr::<u8>::new(0);
        assert_eq!(sys_wait4(&q, WAIT_ANY, null, WNOHANG, none), Ok(0));
        assert_eq!(sys_wait4(&q, 721, null, WNOHANG, none), Ok(0));
        assert_eq!(sys_wait4(&q, 722, null, WNOHANG, none), Err(Errno::ECHILD));
        let mut si = Siginfo::zeroed();
        assert_eq!(
            dowait6(&q, P_PID, 721, None, WEXITED | WNOHANG, None, Some(&mut si)),
            Ok(0)
        );
        assert_eq!(si, Siginfo::zeroed());
        {
            let mut w = klock::witness();
            pr.ps_children.remove(&mut w, &child);
        }
    }
}
/* </TESTS> */
