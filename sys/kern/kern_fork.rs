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
 *	@(#)kern_fork.c	8.6 (Berkeley) 4/8/94
 */
/* </LICENSES> */

/* <CODE> */
//! Creating processes and threads: `kern/kern_fork.c`.
//!
//! Upstream: sys/kern/kern_fork.c @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M5 (part b1) ports `nprocesses`/`nthreads`,
//! `process_initialize`, `fork_check_maxthread`, `alloctid`, `allocpid`, `ispidtaken` and
//! `freepid`; part b2 adds `forkstat`, `thread_new`, `process_new`, `fork_thread_start`,
//! `fork1` and `proc_trampoline_mi`, enough for kernel threads. M8 adds `fork_return`,
//! `sys_fork`, `sys_vfork`, `sys___tfork` and `thread_fork` (the file is complete); the
//! child returns to user mode through the machine's `child_return` (`machine::cpu`).
//!
//! ## Deviations
//! - `fork1` returns the new thread (`Result<&Proc, Errno>`) instead of an `int` plus the
//!   `retval`/`rnewprocp` out-pointers; `thread_new` and `process_new` write a whole
//!   `Proc::new()`/`Process::new()` into the pool item and then copy the `p_startcopy`/
//!   `ps_startcopy` fields from the parent, instead of `memset`/`memcpy` by field offset.
//! - `process_initialize` reports what its process does not have yet: `prof_fork` (M6);
//!   the `realitexpire` timeout is real since `kern_time.c` (M8) and `klist_init_mutex`
//!   since `kern_event.c`. `process_new` likewise reports `startprofclock` (M6);
//!   `sigactsinit` is real since `kern_sig.c`, `fdcopy`/`fdshare` since `kern_descrip.c`
//!   and `lim_fork` since the `plimit` port. `fork1` skips the `RLIMIT_NPROC` check for
//!   root as the C does, reports it for other users (the limits); `knote_processfork` is
//!   real since `kern_event.c`. The credentials (`crhold` in `thread_new` and
//!   `process_initialize`, the forking thread's real uid in `fork1`) are real since
//!   `kern_prot.c`.
//! - `sys_fork` never asks for `FORK_PTRACE`/`fork_return`: the `ptrace(2)` event mask
//!   (`ps_ptmask`, `sys_process.c`) is not ported, so no process wants fork reports.
//! - N4.2: the process and the thread are the kernel crate's (`kernel::proc`): `fork1` makes
//!   the process first (`process_alloc`, with the pid `allocpid` chose, so the pid is chosen
//!   before the item's `pool_get` rather than after it; the C's pid is not in the hash until
//!   `fork1` inserts it either), then its thread (`thread_alloc`, which needs the process),
//!   then fills the process (`process_new`). A process's klist lives in a store the glue
//!   allocates (`kern/proc_glue.rs`): when none can be had, `fork1` fails with `ENOMEM`
//!   before it commits, as for the u-area. A thread joins its process (`ps_threads`) through
//!   the kernel's `insert_thread_locked`, which hands back the box's address; `thread_join`
//!   turns it into the `&'static Proc` every thread reference here is (LZ's `curthread`
//!   convention). The child joins its parent's process group right after the parent
//!   (`proc_glue::fork_join_pgrp`, the C's `LIST_INSERT_AFTER`).

use core::ffi::c_void;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering, fence};

use libkern::StaticCell;

use crate::conf::param::{MAXPROCESS, MAXTHREAD};
use crate::dev::rnd::{arc4random, arc4random_uniform};
use crate::kassert;
use crate::kern::kern_descrip::{fdcopy, fdshare};
use crate::kern::kern_event::{klist_init_mutex, knote_processfork};
use crate::kern::kern_resource::{lim_fork, rucheck};
use crate::kern::kern_sched::{sched_choosecpu_fork, setrunqueue};
use crate::kern::kern_sig::{psignal, sigactsinit, sigstkinit};
use crate::kern::kern_synch::tsleep_nsec;
use crate::kern::kern_tc::nanouptime;
use crate::kern::kern_time::ratecheck;
use crate::kern::kern_timeout::timeout_set;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::proc_glue::{
    chgproccnt, fork_join_pgrp, pgfind, prfind, proc_tables, process_ref, store_alloc, store_free,
    tfind, zombiefind,
};
use crate::kern::sched_bsd::{sched_lock, sched_unlock};
use crate::kern::subr_prf::{panic, tablefull};
use crate::kern::vm_bsd::{uvm_uarea_alloc, uvmspace_fork_or_share};
use crate::machine::Machine;
use crate::machine::copy::{copyin, copyout};
use crate::machine::cpu::{Cpu, child_return};
use crate::machine::tcb::tcb_invalid;
use crate::sys::acct::AFORK;
use crate::sys::errno::Errno;
use crate::sys::param::PWAIT;
use crate::sys::proc::{
    BsdProcessG, BsdThreadG, FORK_FORK, FORK_IDLE, FORK_NOZOMBIE, FORK_PPWAIT, FORK_PTRACE,
    FORK_SHAREFILES, FORK_SHAREVM, FORK_SYSTEM, FORK_VFORK, P_CPUPEG, P_SUSPSIG, P_SUSPSINGLE,
    P_SYSTEM, P_THREAD, PID_MAX, PS_EMBRYO, PS_FLAGS_INHERITED_ON_FORK, PS_ISPWAIT, PS_NOZOMBIE,
    PS_PPWAIT, PS_PROFIL, PS_STOPPING, PS_SYSTEM, PS_TRACED, Proc, ProcExt, Process, ProcessRef,
    PsGuard, SIDL, THREAD_PID_OFFSET, TID_MASK,
};
use crate::sys::signal::{SIGSEGV, SIGTRAP};
use crate::sys::syscallargs::SysTforkArgs;
use crate::sys::systm::{INFSLP, SysArgs, sysargs};
use crate::sys::time::Timeval;
use crate::sys::types::{Pid, Register, Uid};
use crate::sys::unistd::Tfork;
use crate::sys::user::User;
use crate::sys::vmmeter::Forkstat;
use crate::unported;
use hal::klock;
use hal::pin::pin_static;
use hal::pool::PoolBox;
use hal::vm::uvmexp::UVMEXP;

/// `nprocesses`: process 0.
pub static NPROCESSES: AtomicI32 = AtomicI32::new(1);
/// \[a\] `nthreads`: proc 0.
pub static NTHREADS: AtomicI32 = AtomicI32::new(1);

/// `forkstat`: the fork statistics (`<sys/vmmeter.h>`).
pub static FORKSTAT: Forkstat = Forkstat::new();

/// The calling thread as the `&'static Proc` `fork1` and `thread_fork` keep (the child's
/// parent links and `cpu_fork`'s frame copy): its placed self (`ProcExt::as_static`).
fn curthread(p: &Proc) -> &'static Proc {
    p.as_static()
}

/// `fork_return`: the first thing a forked child runs when the parent asked `ptrace(2)` to
/// report forks: stop for the tracer, then return to user mode as `child_return` does.
pub fn fork_return(arg: *mut c_void) {
    // SAFETY: `fork1` passes the new thread itself as the argument.
    let p = unsafe { &*arg.cast::<Proc>() };

    if p.process().ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        psignal(p, SIGTRAP);
    }

    child_return(arg);
}

/// `fork(2)`.
pub fn sys_fork(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let func: fn(*mut c_void) = child_return;
    let flags = FORK_FORK;

    // ps_ptmask & PTRACE_FORK (flags |= FORK_PTRACE, func = fork_return): the ptrace event
    // mask is sys_process.c's, which is not ported, so no process asks for fork reports.

    let child = fork1(curthread(p), flags, func, ptr::null_mut())?;
    retval[0] = child.process().ps_pid as Register;
    Ok(())
}

/// `vfork(2)`: the parent sleeps until the child execs or exits (`FORK_PPWAIT`).
pub fn sys_vfork(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let child = fork1(
        curthread(p),
        FORK_VFORK | FORK_PPWAIT,
        child_return,
        ptr::null_mut(),
    )?;
    retval[0] = child.process().ps_pid as Register;
    Ok(())
}

/// `__tfork(2)`: a new thread in the calling process, with its own stack and TCB.
#[allow(non_snake_case)] // the C name: sys___tfork
pub fn sys___tfork(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysTforkArgs = sysargs(v);
    let psize = uap.psize.get();
    let mut raw = [0u8; size_of::<Tfork>()];

    if psize == 0 || psize > raw.len() {
        return Err(Errno::EINVAL);
    }
    copyin(uap.param.get() as usize, &mut raw[..psize])?;
    let word = |i: usize| {
        const W: usize = size_of::<usize>();
        let mut w = [0u8; W];
        w.copy_from_slice(&raw[i * W..(i + 1) * W]);
        usize::from_ne_bytes(w)
    };
    let param = Tfork {
        tf_tcb: word(0),
        tf_tid: word(1),
        tf_stack: word(2),
    };
    // KTRACE: not configured.
    if tcb_invalid(param.tf_tcb) {
        return Err(Errno::EINVAL);
    }

    thread_fork(
        curthread(p),
        param.tf_stack,
        param.tf_tcb,
        param.tf_tid,
        retval,
    )
}

/// `thread_new`: allocates a thread of `pr`, copying `parent`'s inheritable fields, with its
/// u-area at `uaddr`; its box, which the caller makes join the process ([`thread_join`]).
fn thread_new(parent: &Proc, pr: &ProcessRef, uaddr: NonNull<u8>) -> PoolBox<Proc> {
    let p = kernel::proc::thread_alloc(pr, BsdThreadG::new());

    p.set_stat(SIDL); // protect against others
    p.p_runpri.set(0);
    p.p_flag.store(0, Ordering::Relaxed);

    // Make a proc table entry for the new process. Start by zeroing the section of proc
    // that is zero-initialized (`thread_alloc` did), then copy the section that is copied
    // directly from the parent (p_startcopy .. p_endcopy).
    p.p_sigmask.set(parent.p_sigmask.get());
    p.set_name(&parent.name());
    p.p_slppri.set(parent.p_slppri.get());
    p.p_usrpri.set(parent.p_usrpri.get());
    p.p_estcpu.set(parent.p_estcpu.get());
    p.p_pledge_syscall.set(parent.p_pledge_syscall.get());
    p.p_pledge.set(parent.p_pledge.get());
    p.p_sigstk.set(parent.p_sigstk.get());
    p.p_prof_addr.set(parent.p_prof_addr.get());
    p.p_prof_ticks.set(parent.p_prof_ticks.get());

    // crhold(p->p_ucred): the thread's own copy of the parent's, and its reference.
    drop(p.set_cred(parent.cred_ref()));
    p.set_uarea(uaddr.as_ptr().cast::<User>());

    // Initialize the timeouts: `p_sleep_to` is named by the thread's tid, set once it has
    // one (`kernel::sched::sleep_timeout_init`, in `fork1` and `thread_fork`).

    p
}

/// The thread box `t` joins its process `pr` (`TAILQ_INSERT_TAIL(&pr->ps_threads, p,
/// p_thr_link)`); `then` runs in the same `ps_mtx` section (the counts). The thread placed,
/// as the `&'static Proc` every thread reference here is.
fn thread_join(
    pr: &Process,
    t: PoolBox<Proc>,
    then: impl FnOnce(&mut PsGuard<'_>, &Proc),
) -> &'static Proc {
    let mut g = pr.ps_mtx.lock();
    let at = pr.insert_thread_locked(&mut g, t);
    // SAFETY: `at` is the address of the thread box the process now owns
    // (`insert_thread_locked`): a `procpl` item, which stays at that address until `exit1`
    // takes the box back from the process and `proc_free` drops it (the reaper's, or
    // `process_zap`'s for the main thread), after the thread has left every list, hash and
    // queue and no other thread names it. Until then the item is a live, initialised thread
    // that nothing mutates but through its atomics and cells (shared references only), so a
    // shared reference to it is valid; `'static` is LZ's convention for a thread reference
    // (the C's `struct proc *` from `pool_get` to `pool_put`), the one `curproc()` and the
    // run queues rest on.
    let p: &'static Proc = unsafe { &*at };
    then(&mut g, p);
    drop(g);
    p.place();
    p
}

/// `process_initialize`: initialize common bits of a process structure, given the initial
/// thread (which has joined it).
pub fn process_initialize(pr: &Process, p: &'static Proc) {
    // initialize the thread links
    pr.ps_mainproc.set(p);
    pr.ps_threadcnt.set(1);

    // give the process the same creds as the initial thread
    drop(pr.set_ucred(p.cred_ref()));

    // prof_fork(pr): subr_prof.c (M6).
    let _ = unported!("process_initialize: prof_fork (M6)");

    // A new process's lists (children, orphans, sigio) are empty.
    kassert!(pr.ps_sigiolst.is_empty());

    // SAFETY: the klist's mutex is a member of the same store, alive as long as its klist
    // (a store is never freed: `kern/proc_glue.rs`).
    unsafe { klist_init_mutex(&pr.ps_klist, &pr.ps_klmtx) };

    crate::kern::kern_timeout::timeout_set_flags(
        &pr.ps_realit_to,
        crate::kern::kern_time::realitexpire,
        ptr::from_ref(pr).cast_mut().cast::<c_void>(),
        crate::sys::timeout::KCLOCK_UPTIME,
        0,
    );
    timeout_set(
        &pr.ps_rucheck_to,
        rucheck,
        ptr::from_ref(pr).cast_mut().cast::<c_void>(),
    );
}

/// `process_new`: initialize the new process `pr` (made by `fork1` with its pid, its main
/// thread `p` joined) from `parent`.
fn process_new(p: &'static Proc, pr: &ProcessRef, parent: &Process, flags: i32) {
    // Make a process structure for the new process. Start by zeroing the section of proc
    // that is zero-initialized (`process_alloc` did), then copy the section that is copied
    // directly from the parent (ps_startcopy .. ps_endcopy); ps_limit and ps_pgrp are set
    // by lim_fork and fork1.
    pr.set_comm(&parent.comm());
    pr.ps_strings.set(parent.ps_strings.get());
    pr.ps_auxinfo.set(parent.ps_auxinfo.get());
    pr.ps_timekeep.set(parent.ps_timekeep.get());
    pr.ps_sigcode.set(parent.ps_sigcode.get());
    pr.ps_sigcoderet.set(parent.ps_sigcoderet.get());
    pr.ps_sigcookie.set(parent.ps_sigcookie.get());
    pr.ps_rtableid.store(
        parent.ps_rtableid.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    pr.ps_iflags.set(parent.ps_iflags.get());
    pr.ps_nice.set(parent.ps_nice.get());
    pr.ps_acflag.set(parent.ps_acflag.get());
    pr.ps_pledge
        .store(parent.ps_pledge.load(Ordering::Relaxed), Ordering::Relaxed);
    pr.ps_execpledge.set(parent.ps_execpledge.get());
    pr.ps_kbind_cookie.set(parent.ps_kbind_cookie.get());
    pr.ps_kbind_addr.set(parent.ps_kbind_addr.get());

    process_initialize(pr, p);
    lim_fork(parent, pr);

    // post-copy fixups
    {
        let mut w = klock::witness();
        drop(pr.set_pptr(&mut w, process_ref(parent)));
    }
    // WITNESS_SETCHILD: not configured.

    // bump references to the text vnode (for sysctl)
    pr.ps_textvp.set(parent.ps_textvp.get());
    if let Some(vp) = pr.ps_textvp.get() {
        crate::kern::vfs_subr::vref(vp);
    }

    // copy unveil if unveil is active
    crate::kern::kern_unveil::unveil_copy(parent, pr);

    pr.ps_flags.store(
        parent.ps_flags.load(Ordering::Relaxed) & PS_FLAGS_INHERITED_ON_FORK,
        Ordering::Relaxed,
    );
    let ctty = {
        let r = klock::reader();
        parent
            .session(&r)
            .is_some_and(|s| s.s_ttyvp.get().is_some())
    };
    if ctty {
        pr.ps_flags.fetch_or(
            parent.ps_flags.load(Ordering::Relaxed) & crate::sys::proc::PS_CONTROLT,
            Ordering::Relaxed,
        );
    }

    pin_copy(&parent.ps_pin, &pr.ps_pin);
    pin_copy(&parent.ps_libcpin, &pr.ps_libcpin);

    // Duplicate sub-structures as needed. Increase reference counts on shared objects.
    if flags & FORK_SHAREFILES != 0 {
        pr.set_fd(fdshare(parent));
    } else {
        pr.set_fd(fdcopy(parent));
    }
    pr.ps_sigacts.set(Some(sigactsinit(parent)));
    uvmspace_fork_or_share(parent, pr, flags & FORK_SHAREVM != 0);

    if parent.ps_flags.load(Ordering::Relaxed) & PS_PROFIL != 0 {
        let _ = unported!("process_new: startprofclock (subr_prof.c, M6)");
    }
    if flags & FORK_PTRACE != 0 {
        pr.ps_flags.fetch_or(
            parent.ps_flags.load(Ordering::Relaxed) & PS_TRACED,
            Ordering::Relaxed,
        );
    }
    if flags & FORK_NOZOMBIE != 0 {
        pr.ps_flags.fetch_or(PS_NOZOMBIE, Ordering::Relaxed);
    }
    if flags & FORK_SYSTEM != 0 {
        pr.ps_flags.fetch_or(PS_SYSTEM, Ordering::Relaxed);
    }

    // mark as embryo to protect against others
    pr.ps_flags.fetch_or(PS_EMBRYO, Ordering::Relaxed);

    // Force visibility of all of the above changes
    fence(Ordering::Release); // membar_producer()

    // it's sufficiently inited to be globally visible
    let mut w = klock::witness();
    proc_tables().allprocess.insert_head(&mut w, pr.as_pin());
}

/// `process_new`'s copy of a pin table (`ps_pin`, `ps_libcpin`): the bounds come with the
/// copied area, the table itself is duplicated (`mallocarray(M_PINSYSCALL)` + `memcpy`).
fn pin_copy(from: &crate::sys::proc::Pinsyscall, to: &crate::sys::proc::Pinsyscall) {
    to.pn_start.set(from.pn_start.get());
    to.pn_end.set(from.pn_end.get());
    to.pn_npins.set(from.pn_npins.get());
    to.pn_pins.set(ptr::null_mut());
    let Some(src) = NonNull::new(from.pn_pins.get_mut()) else {
        return;
    };
    let n = from.pn_npins.get().max(0) as usize;
    let Some(mem) = crate::kern::kern_malloc::mallocarray(
        n,
        size_of::<u32>(),
        crate::sys::malloc::M_PINSYSCALL,
        crate::sys::malloc::M_WAITOK,
    ) else {
        // M_WAITOK cannot fail in C; here the child simply has no table (pin_check kills
        // it at its first system call, as it would a process without one).
        to.pn_npins.set(0);
        return;
    };
    let dst = mem.cast::<u32>();
    // SAFETY: `src` is the parent's table of `n` entries (alive while the parent forks),
    // `dst` a fresh allocation of `n` entries; they do not overlap.
    unsafe { ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), n) };
    to.pn_pins.set(dst.as_ptr());
}

/// `fork_tfmrate`: print the 'table full' message once per 10 seconds.
const FORK_TFMRATE: Timeval = Timeval::new(10, 0);

/// `fork_check_maxthread`: although process entries are dynamically created, we still keep
/// a global limit on the maximum number we will create. We reserve the last 5 processes to
/// root. The variable `nprocesses` is the current number of processes, `maxprocess` is the
/// limit. Similar rules for threads (struct proc): we reserve the last 5 to root; the
/// variable `nthreads` is the current number of procs, `maxthread` is the limit.
pub fn fork_check_maxthread(uid: Uid) -> Result<(), Errno> {
    static LASTTFM: StaticCell<Timeval> = StaticCell::new(Timeval::new(0, 0));

    let maxthread_local = MAXTHREAD.load(Ordering::Relaxed);
    let val = NTHREADS.fetch_add(1, Ordering::Relaxed) + 1;
    if (val > maxthread_local - 5 && uid != 0) || val > maxthread_local {
        // SAFETY: `lasttfm` is a rate limiter touched under the kernel lock.
        let lasttfm = unsafe { LASTTFM.get_mut() };
        if ratecheck(lasttfm, &FORK_TFMRATE) {
            tablefull("thread");
        }
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::EAGAIN);
    }

    Ok(())
}

/// `fork_thread_start`: puts the new thread on a run queue.
fn fork_thread_start(p: &Proc, parent: &Proc, flags: i32) {
    let mut h = sched_lock();
    let cpu = sched_choosecpu_fork(parent, flags);
    // TRACEPOINT(sched, fork, ...): dt(4), not configured.
    setrunqueue(&mut h, Some(cpu), p, p.p_usrpri.get());
    sched_unlock(h);
}

/// `fork1`: creates a new process (and its first thread) that starts in `func(arg)`
/// (`arg` null: the new thread itself). Returns the new thread (the C's `*rnewprocp`;
/// `*retval`, the child's pid, is `p.process().ps_pid`).
pub fn fork1(
    curp: &'static Proc,
    flags: i32,
    func: fn(*mut c_void),
    arg: *mut c_void,
) -> Result<&'static Proc, Errno> {
    static LASTTFM: StaticCell<Timeval> = StaticCell::new(Timeval::new(0, 0));

    let curpr: &'static Process = curp.process();
    let uid: Uid = curp.ucred().cr_ruid.get();

    kassert!(
        flags
            & !(FORK_FORK
                | FORK_VFORK
                | FORK_PPWAIT
                | FORK_PTRACE
                | FORK_IDLE
                | FORK_SHAREVM
                | FORK_SHAREFILES
                | FORK_NOZOMBIE
                | FORK_SYSTEM)
            == 0
    );

    fork_check_maxthread(uid)?;

    let maxprocess_local = MAXPROCESS.load(Ordering::Relaxed);
    let nprocesses = NPROCESSES.load(Ordering::Relaxed);
    if (nprocesses >= maxprocess_local - 5 && uid != 0) || nprocesses >= maxprocess_local {
        // SAFETY: `lasttfm` is a rate limiter touched under the kernel lock.
        let lasttfm = unsafe { LASTTFM.get_mut() };
        if ratecheck(lasttfm, &FORK_TFMRATE) {
            tablefull("process");
        }
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::EAGAIN);
    }
    NPROCESSES.fetch_add(1, Ordering::Relaxed);

    // Increment the count of processes running with this uid. Don't allow a nonprivileged
    // user to exceed their current limit.
    let _count = chgproccnt(uid, 1);
    if uid != 0 {
        // count > lim_cur(RLIMIT_NPROC): the limits (kern_resource.c, M6).
        let _ = unported!("fork1: lim_cur(RLIMIT_NPROC) (M6)");
    }

    // The new process's klist store (see the module's deviations).
    let store = match store_alloc() {
        Ok(store) => store,
        Err(error) => {
            chgproccnt(uid, -1);
            NPROCESSES.fetch_sub(1, Ordering::Relaxed);
            NTHREADS.fetch_sub(1, Ordering::Relaxed);
            return Err(error);
        }
    };
    let Some(uaddr) = uvm_uarea_alloc() else {
        store_free(store);
        chgproccnt(uid, -1);
        NPROCESSES.fetch_sub(1, Ordering::Relaxed);
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::ENOMEM);
    };

    // From now on, we're committed to the fork and cannot fail.
    let pr = kernel::proc::process_alloc(allocpid(), BsdProcessG::new(store));
    let p = thread_join(&pr, thread_new(curp, &pr, uaddr), |_, _| {});
    process_new(p, &pr, curpr, flags);

    // p_fd: the thread reads its process's table (`Thread::fd`).
    drop(p.set_vmspace(pr.vmspace_share()));
    if pr.ps_flags.load(Ordering::Relaxed) & PS_SYSTEM != 0 {
        p.p_flag.fetch_or(P_SYSTEM, Ordering::Relaxed);
    }

    if flags & FORK_PPWAIT != 0 {
        pr.ps_flags.fetch_or(PS_PPWAIT, Ordering::Relaxed);
        curpr.ps_flags.fetch_or(PS_ISPWAIT, Ordering::Relaxed);
    }

    // KTRACE: not configured.

    // Finish creating the child thread. cpu_fork() will copy and update the pcb and make
    // the child ready to run. If this is a normal user fork, the child will exit directly to
    // user mode via child_return() on its first time slice and will not return here. If
    // this is a kernel thread, the specified entry point will be executed.
    let arg = if arg.is_null() {
        ptr::from_ref(p).cast_mut().cast::<c_void>()
    } else {
        arg
    };
    Machine::cpu_fork(curp, p, ptr::null_mut(), ptr::null_mut(), func, arg);

    let vm = p.vmspace();
    let vmsize = (vm.vm_dsize.load(Ordering::Relaxed) + vm.vm_ssize.load(Ordering::Relaxed)) as u64;
    if flags & FORK_FORK != 0 {
        FORKSTAT.cntfork.fetch_add(1, Ordering::Relaxed);
        FORKSTAT.sizfork.fetch_add(vmsize, Ordering::Relaxed);
    } else if flags & FORK_VFORK != 0 {
        FORKSTAT.cntvfork.fetch_add(1, Ordering::Relaxed);
        FORKSTAT.sizvfork.fetch_add(vmsize, Ordering::Relaxed);
    } else {
        FORKSTAT.cntkthread.fetch_add(1, Ordering::Relaxed);
        FORKSTAT.sizkthread.fetch_add(vmsize, Ordering::Relaxed);
    }

    p.p_tid.set(alloctid());
    kernel::sched::sleep_timeout_init(p);
    {
        let mut w = klock::witness();
        let t = proc_tables();
        t.allproc.insert_head(&mut w, pin_static(p));
        t.tidhash(p.p_tid.get()).insert_head(&mut w, pin_static(p));
        t.pidhash(pr.ps_pid).insert_head(&mut w, pr.as_pin());

        fork_join_pgrp(&mut w, curpr, &pr);
        curpr.ps_children.insert_head(&mut w, pr.as_pin());
    }

    let g = pr.ps_mtx.lock();
    if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        // ps_opptr, process_reparent, the ptrace status: ptrace (M6).
        let _ = unported!("fork1: ptrace (process_reparent, ps_ptstat, M6)");
    }
    drop(g);

    // For new processes, set accounting bits and mark as complete.
    pr.ps_start.set(nanouptime().into());
    pr.ps_acflag.set(AFORK);
    pr.ps_flags.fetch_and(!PS_EMBRYO, Ordering::Relaxed);

    // Idle threads are just assigned to the CPU but not added to any runqueue (their
    // argument is the CPU's number, `sched_kthreads_create`).
    if flags & FORK_IDLE != 0 {
        kernel::sched::set_thread_cpu(p, arg.addr());
        // for consistency mark idle procs as pegged
        p.p_flag.fetch_or(P_CPUPEG, Ordering::Relaxed);
    } else {
        fork_thread_start(p, curp, flags);
    }

    // Notify any interested parties about the new process.
    knote_processfork(curpr, pr.ps_pid);

    // Update stats now that we know the fork was successful.
    UVMEXP.forks.fetch_add(1, Ordering::Relaxed);
    if flags & FORK_PPWAIT != 0 {
        UVMEXP.forks_ppwait.fetch_add(1, Ordering::Relaxed);
    }
    if flags & FORK_SHAREVM != 0 {
        UVMEXP.forks_sharevm.fetch_add(1, Ordering::Relaxed);
    }

    // Preserve synchronization semantics of vfork. If waiting for child to exec or exit,
    // set PS_PPWAIT on child and PS_ISPWAIT on ourselves, and sleep on our process for the
    // latter flag to go away. XXX Need to stop other rthreads in the parent
    if flags & FORK_PPWAIT != 0 {
        while curpr.ps_flags.load(Ordering::Relaxed) & PS_ISPWAIT != 0 {
            let _ = tsleep_nsec(ptr::from_ref(curpr), PWAIT, "ppwait", INFSLP);
        }
    }

    // If we're tracing the child, alert the parent too.
    if flags & FORK_PTRACE != 0 && curpr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        psignal(curp, SIGTRAP);
    }

    // Return child pid to parent process: the caller reads pr->ps_pid.
    Ok(p)
}

/// `thread_fork`: a new thread of `curp`'s process that returns to user mode on `stack`
/// with `tcb` as its TCB; its id (offset by `THREAD_PID_OFFSET`) is the return value and is
/// copied out to `tidptr` when that is not NULL.
pub fn thread_fork(
    curp: &'static Proc,
    stack: usize,
    tcb: usize,
    tidptr: usize,
    retval: &mut [Register; 2],
) -> Result<(), Errno> {
    let pr = curp.process();

    if stack == 0 {
        return Err(Errno::EINVAL);
    }

    fork_check_maxthread(curp.ucred().cr_ruid.get())?;

    let Some(uaddr) = uvm_uarea_alloc() else {
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::ENOMEM);
    };

    // From now on, we're committed to the fork and cannot fail.
    let p = thread_new(curp, curp.process_ref(), uaddr);
    p.p_flag.fetch_or(P_THREAD, Ordering::Relaxed);
    sigstkinit(&p.p_sigstk);
    p.set_name(b"");

    // other links: p_p, made with the thread (thread_alloc).

    // local copies (p_fd: the thread reads its process's table, `Thread::fd`)
    drop(p.set_vmspace(pr.vmspace_share()));

    // Finish creating the child thread. cpu_fork() will copy and update the pcb and make
    // the child ready to run. The child will exit directly to user mode via child_return()
    // on its first time slice and will not return here.
    Machine::cpu_fork(
        curp,
        &p,
        stack as *mut u8,
        tcb as *mut u8,
        child_return,
        PoolBox::as_ptr(&p).cast_mut().cast::<c_void>(),
    );

    p.p_tid.set(alloctid());
    kernel::sched::sleep_timeout_init::<BsdProcGlue>(&p);

    {
        let mut w = klock::witness();
        let t = proc_tables();
        t.allproc.insert_head(&mut w, p.as_pin());
        t.tidhash(p.p_tid.get()).insert_head(&mut w, p.as_pin());
    }

    let p = thread_join(pr, p, |_, p| {
        pr.ps_threadcnt.set(pr.ps_threadcnt.get() + 1);

        // if somebody else wants to take us to single threaded mode or suspend the
        // process, count ourselves in.
        if !pr.ps_single.get().is_null() || pr.ps_flags.load(Ordering::Relaxed) & PS_STOPPING != 0 {
            pr.ps_suspendcnt.set(pr.ps_suspendcnt.get() + 1);
            p.p_flag.fetch_or(
                curp.p_flag.load(Ordering::Relaxed) & (P_SUSPSINGLE | P_SUSPSIG),
                Ordering::Relaxed,
            );
        }
    });

    // Return tid to parent thread and copy it out to userspace
    let tid: Pid = p.p_tid.get() + THREAD_PID_OFFSET;
    retval[0] = tid as Register;
    if tidptr != 0 && copyout(&tid.to_ne_bytes(), tidptr).is_err() {
        psignal(curp, SIGSEGV);
    }

    fork_thread_start(p, curp, 0);

    // Update stats now that we know the fork was successful.
    FORKSTAT.cnttfork.fetch_add(1, Ordering::Relaxed);
    UVMEXP.forks.fetch_add(1, Ordering::Relaxed);
    UVMEXP.forks_sharevm.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

/// `alloctid`: find an unused tid.
pub fn alloctid() -> Pid {
    loop {
        // (0 .. TID_MASK+1]
        let tid = 1 + (arc4random() as Pid & TID_MASK);
        let r = klock::reader();
        if tfind(&r, tid).is_none() {
            return tid;
        }
    }
}

/// `oldpids`: the recently freed pids, not reused for a while.
static OLDPIDS: StaticCell<[Pid; 128]> = StaticCell::new([0; 128]);

/// `ispidtaken`: checks for current use of a pid, either as a pid or pgid.
pub fn ispidtaken(pid: Pid) -> bool {
    // SAFETY: written only by `freepid` under the kernel lock.
    if unsafe { OLDPIDS.get() }.contains(&pid) {
        return true;
    }

    let r = klock::reader();
    if prfind(&r, pid).is_some() {
        return true;
    }
    if pgfind(&r, pid).is_some() {
        return true;
    }
    if zombiefind(&r, pid).is_some() {
        return true;
    }
    false
}

/// `allocpid`: find an unused pid.
pub fn allocpid() -> Pid {
    static FIRST: AtomicI32 = AtomicI32::new(1);

    // The first PID allocated is always 1.
    if FIRST.swap(0, Ordering::Relaxed) == 1 {
        return 1;
    }

    // All subsequent PIDs are chosen randomly. We need to find an unused PID in the range
    // [2, PID_MAX].
    loop {
        let pid = 2 + arc4random_uniform((PID_MAX - 1) as u32) as Pid;
        if !ispidtaken(pid) {
            return pid;
        }
    }
}

/// `freepid`: remembers a freed pid so it is not reused right away.
pub fn freepid(pid: Pid) {
    static IDX: AtomicU32 = AtomicU32::new(0);

    let idx = IDX.fetch_add(1, Ordering::Relaxed) as usize % 128;
    // SAFETY: as for `ispidtaken`; one writer under the kernel lock.
    unsafe { OLDPIDS.get_mut()[idx] = pid };
}

/// How many distinct thread functions `cpu_fork` can start threads with ([`STARTS`]).
const NSTARTS: usize = 64;

/// The functions threads were started with, by number: a new thread's switch frame carries
/// the number (`hal::thread::thread_setup`'s start word, N4.5a), and its trampoline calls the
/// function again through [`thread_start`]. Filled on first use, never emptied (the C keeps
/// the function itself in the frame; here no code address crosses into `hal`).
static STARTS: [hal::once::OnceStatic<fn(*mut c_void)>; NSTARTS] =
    [const { hal::once::OnceStatic::new() }; NSTARTS];

/// The number [`thread_start`] knows `f` by: its slot, filled the first time. Panics when
/// more than [`NSTARTS`] functions are used.
pub fn thread_start_word(f: fn(*mut c_void)) -> usize {
    for (i, slot) in STARTS.iter().enumerate() {
        if slot.get().is_none() {
            // A racing claim of another function takes the slot first: look on.
            let _ = slot.set(f);
        }
        if slot.get().is_some_and(|g| core::ptr::fn_addr_eq(*g, f)) {
            return i;
        }
    }
    panic(format_args!(
        "cpu_fork: more than {NSTARTS} thread functions"
    ))
}

/// The start `hal::thread` calls on a new thread (registered by `hal_bridges_register`,
/// bridges.toml `ThreadStart`), with `hal`'s scheduler hold the switch to it kept and the
/// two words `cpu_fork` gave: `proc_trampoline_mi` (`kernel::sched::trampoline_mi`, which
/// releases the hold), then `func(arg)`.
pub fn thread_start(hold: hal::thread::SchedHold, start: usize, arg: usize) {
    kernel::sched::trampoline_mi::<BsdProcGlue>(hold);
    let Some(f) = STARTS.get(start).and_then(|s| s.get()).copied() else {
        panic(format_args!("proc_trampoline: no thread function {start}"));
    };
    f(ptr::with_exposed_provenance_mut(arg));
}
/* </CODE> */
