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
//! Creating processes and threads: `kern/kern_fork.c`: `fork1` and its parts
//! (`thread_new`, `process_new`, `process_initialize`, `fork_thread_start`), `thread_fork`,
//! the pid and tid allocators (`allocpid`, `alloctid`, `ispidtaken`, `freepid`), the limits
//! (`fork_check_maxthread`, `nprocesses`, `nthreads`, `forkstat`), the system calls
//! `fork(2)`, `vfork(2)`, `__tfork(2)`, and the start of every new thread (the C's
//! `proc_trampoline` calling `func(arg)`).
//!
//! Upstream: sys/kern/kern_fork.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_fork.rs@3c62ede68802
//!
//! What is still `bsd`'s is reached through the process glue ([`ProcGlue`]): the text vnode
//! and the unveils of a new process (`process_forked`, N5/N8), the interval timer's timeout
//! (`process_initialized`, N4.8a), the machine's pmap and `child_return` (N4.8, N10), the
//! session's terminal vnode (`session_has_tty_vnode`, N4.7b), `init`'s start (N9.1), the
//! limits `maxprocess`/`maxthread` (`conf/param.rs`), `log(9)`.
//!
//! ## Deviations
//! - `ptrace(2)` is not ported (`sys_process.c`): no process asks for fork reports
//!   (`ps_ptmask`), so `fork(2)` never starts a child with [`Start::ForkReturn`]; a traced
//!   child's reparenting and report (`ps_opptr`, `ps_ptstat`) are reported once.
//! - `prof_fork` and `startprofclock` (`subr_prof.c`) are reported once.
//! - The pid is chosen before the process is made (`process_alloc(allocpid())`): the C's
//!   is not in the pid hash until `fork1` inserts it either.
//!
//! ## Redesign
//! - The thread's start (`void (*func)(void *)` and `void *arg`, which `cpu_fork` writes in
//!   the switch frame and `proc_trampoline` calls) is a [`Start`], two machine words for
//!   `hal::thread::thread_setup`: fixed tags for the kernel's starts (`child_return`,
//!   `fork_return`, an idle thread, a ticket of `kernel::thread`, `init`, the reaper,
//!   accounting) and, for a function of `bsd`'s, a tag naming its slot in a 64-slot table
//!   filed at first use (LZ's `STARTS`, moved here) with the argument's exposed address. No
//!   code address crosses into `hal`, nothing is allocated on `fork(2)`'s path, and the C's
//!   `arg ? arg : p` (a NULL argument is the new thread) is the argument word 0 replaced by the
//!   thread's address. [`thread_start`] is registered with `hal` once (the `ThreadStart`
//!   bridge of `bsd`'s fork is gone).
//! - The new thread's box stays `fork1`'s while it is made (linked on its process's thread
//!   list, `allproc` and the tid hash through its pin) and goes to its process's slot
//!   (`set_main_locked`) or tree (`insert_box_locked`) in the same `ps_mtx` section that puts
//!   it on a run queue: the process owns it before anything can run it, and no raw pointer
//!   names it (LZ's `thread_join` made the box's address a `&'static Proc`).
//! - `oldpids[128]` is a ring of atomics with an atomic index (the C's racy ring under no lock,
//!   kept racy: a heuristic that delays a pid's reuse); the two `lasttfm` rate limiters are
//!   `Mutexed<Timespec>`s (cold).
//! - `fork1` returns the child's pid, first thread's tid and process ([`ForkChild`]) for the
//!   C's `retval` and `rnewprocp`.
//! - `forkstat.sizkthread` is not accumulated, as the C's (LZ added the kernel threads'
//!   address-space size; the user's decision 13 of N4.6, `vmstat -f` changes for kernel
//!   threads only).
//! - `RLIMIT_NPROC` is enforced for users other than root (`EAGAIN`, as kern_fork.c:386-393;
//!   LZ reported it unported; the user's decision 12 of N4.6).

use core::ffi::c_void;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering, fence};

use hal::intr::IPL_NONE;
use hal::klock;
use hal::mutex::Mutexed;
use hal::once::OnceStatic;
use hal::pool::PoolBox;
use hal::sys::errno::Errno;
use hal::sys::types::{Pid, Uid};
use hal::sys::vmmeter::Forkstat;
use hal::thread::{SchedHold, UArea};
use hal::user::UserPtr;
use hal::vm::uvmexp::UVMEXP;

use crate::assert::{kassert, kpanic};
use crate::clock::{self, Timespec};
use crate::proc::{
    FORK_FORK, FORK_IDLE, FORK_NOZOMBIE, FORK_PPWAIT, FORK_PTRACE, FORK_SHAREFILES, FORK_SHAREVM,
    FORK_SYSTEM, FORK_VFORK, P_CPUPEG, P_SUSPSIG, P_SUSPSINGLE, P_SYSTEM, P_THREAD, PID_MAX,
    PS_CONTROLT, PS_EMBRYO, PS_FLAGS_INHERITED_ON_FORK, PS_ISPWAIT, PS_NOZOMBIE, PS_PPWAIT,
    PS_PROFIL, PS_STOPPING, PS_SYSTEM, PS_TRACED, PinTable, Placed, ProcGlue, Process, ProcessRef,
    SIDL, THREAD_PID_OFFSET, TID_MASK, Thread,
};
use crate::resource::RLIMIT_NPROC;

/// `SIGSEGV`.
const SIGSEGV: i32 = 11;
/// `SIGTRAP`.
const SIGTRAP: i32 = 5;
/// `PWAIT` (`<sys/param.h>`).
const PWAIT: i32 = 32;
/// `LOG_ERR` (`<sys/syslog.h>`).
pub const LOG_ERR: i32 = 3;

/// `nprocesses`: process 0.
pub static NPROCESSES: AtomicI32 = AtomicI32::new(1);
/// \[a\] `nthreads`: proc 0.
pub static NTHREADS: AtomicI32 = AtomicI32::new(1);
/// `forkstat`: the fork statistics (`kern.forkstat`, `vmstat -f`).
pub static FORKSTAT: Forkstat = Forkstat::new();

/// `fork_tfmrate`: print the 'table full' message once per 10 seconds.
const FORK_TFMRATE: Timespec = Timespec::new(10, 0);
/// `fork_check_maxthread`'s `lasttfm`.
static LASTTFM_THREAD: Mutexed<Timespec> = Mutexed::new(IPL_NONE, Timespec::ZERO);
/// `fork1`'s `lasttfm`.
static LASTTFM_PROCESS: Mutexed<Timespec> = Mutexed::new(IPL_NONE, Timespec::ZERO);

/// `oldpids`: the recently freed pids, not reused for a while.
static OLDPIDS: [AtomicI32; 128] = [const { AtomicI32::new(0) }; 128];
/// `freepid`'s `idx`.
static OLDPIDS_IDX: AtomicU32 = AtomicU32::new(0);
/// `allocpid`'s `first`: the first pid allocated is always 1.
static FIRSTPID: AtomicI32 = AtomicI32::new(1);

/// How many distinct `bsd` thread functions a [`Start::Legacy`] can name.
const NLEGACY: usize = 64;
/// The `bsd` thread functions started so far, by slot (filled at first use, never emptied).
static LEGACY: [OnceStatic<fn(*mut c_void)>; NLEGACY] = [const { OnceStatic::new() }; NLEGACY];

/// The start words' tags (`thread_setup`'s `start`).
const TAG_CHILD_RETURN: usize = 1;
/// [`Start::ForkReturn`].
const TAG_FORK_RETURN: usize = 2;
/// [`Start::Idle`].
const TAG_IDLE: usize = 3;
/// [`Start::Kernel`].
const TAG_KERNEL: usize = 4;
/// [`Start::Init`].
const TAG_INIT: usize = 5;
/// [`Start::Reaper`].
const TAG_REAPER: usize = 6;
/// [`Start::Acct`].
const TAG_ACCT: usize = 7;
/// [`Start::Legacy`]: the slot is added.
const TAG_LEGACY: usize = 0x100;

/// What a new thread runs first (`fork1`'s `func(arg)`, the switch frame's two words).
#[derive(Clone, Copy)]
pub enum Start {
    /// `child_return(p)`: a forked child returns to user mode.
    ChildReturn,
    /// `fork_return(p)`: as `ChildReturn`, after stopping for a tracer (`ptrace(2)`).
    ForkReturn,
    /// `sched_idle(ci)`: the idle thread of CPU `n`.
    Idle(usize),
    /// `kernel::thread`'s start filed under a ticket.
    Kernel(usize),
    /// `start_init(p)`: process 1.
    Init,
    /// `reaper()`: the dead threads' reaper.
    Reaper,
    /// `acct_thread()`: accounting's free-space watcher.
    Acct,
    /// A function of `bsd`'s and its argument's exposed address (0: the new thread's own).
    Legacy(fn(*mut c_void), usize),
}

impl Start {
    /// The two words `thread_setup` stores (`start`, `arg`); a `Legacy` function is filed in
    /// its slot first (`panic(9)` past [`NLEGACY`] distinct functions). `me` is the new
    /// thread's address, which a `Legacy` argument of 0 becomes.
    pub fn words(self, me: usize) -> (usize, usize) {
        match self {
            Start::ChildReturn => (TAG_CHILD_RETURN, 0),
            Start::ForkReturn => (TAG_FORK_RETURN, 0),
            Start::Idle(n) => (TAG_IDLE, n),
            Start::Kernel(t) => (TAG_KERNEL, t),
            Start::Init => (TAG_INIT, 0),
            Start::Reaper => (TAG_REAPER, 0),
            Start::Acct => (TAG_ACCT, 0),
            Start::Legacy(f, arg) => {
                let arg = if arg == 0 { me } else { arg };
                (TAG_LEGACY + legacy_slot(f), arg)
            }
        }
    }

    /// The start two words name (`None` for words no [`Start::words`] made).
    pub fn from_words(start: usize, arg: usize) -> Option<Start> {
        Some(match start {
            TAG_CHILD_RETURN => Start::ChildReturn,
            TAG_FORK_RETURN => Start::ForkReturn,
            TAG_IDLE => Start::Idle(arg),
            TAG_KERNEL => Start::Kernel(arg),
            TAG_INIT => Start::Init,
            TAG_REAPER => Start::Reaper,
            TAG_ACCT => Start::Acct,
            s if s >= TAG_LEGACY => {
                let f = *LEGACY.get(s - TAG_LEGACY)?.get()?;
                Start::Legacy(f, arg)
            }
            _ => return None,
        })
    }
}

/// The slot `f` is filed under: the first that holds it, or the first free one. Two pointers
/// to one function may compare unequal (copies in two codegen units): such a function takes
/// two slots, both of which call it, so the table only grows; `bsd` has about two dozen
/// thread functions, under the 64 slots even if each site's pointer were its own.
fn legacy_slot(f: fn(*mut c_void)) -> usize {
    for (i, slot) in LEGACY.iter().enumerate() {
        if slot.get().is_none() {
            // A racing claim of another function takes the slot first: look on.
            let _ = slot.set(f);
        }
        if slot.get().is_some_and(|g| ptr::fn_addr_eq(*g, f)) {
            return i;
        }
    }
    kpanic(format_args!(
        "cpu_fork: more than {NLEGACY} thread functions"
    ))
}

/// What `fork1` made (the C's `*retval` and `*rnewprocp`).
pub struct ForkChild<G: ProcGlue> {
    /// The new process's pid.
    pub pid: Pid,
    /// Its first thread's tid (without `THREAD_PID_OFFSET`).
    pub tid: Pid,
    /// The new process, counted.
    pub process: ProcessRef<G>,
}

/// `struct __tfork`: `__tfork(2)`'s parameters, as userland lays them out.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tfork {
    /// `tf_tcb`: the TCB address.
    pub tf_tcb: usize,
    /// `tf_tid`: where the new thread's id is copied out.
    pub tf_tid: usize,
    /// `tf_stack`: the new thread's stack.
    pub tf_stack: usize,
}

hal::abi_pod!(Tfork {
    tf_tcb: usize,
    tf_tid: usize,
    tf_stack: usize
});

/// `kernel::fork`'s registration: [`thread_start`] is the start of every new thread
/// (`hal::thread::register_start`), and this glue makes the kernel's own threads
/// (`kernel::thread`'s timeout and task threads), once, before the first fork (`main`, before
/// `procinit`).
pub fn init<G: ProcGlue>() -> Result<(), hal::bridges::AlreadyRegistered> {
    crate::thread::install_ops::<G>();
    hal::thread::register_start(thread_start::<G>)
}

/// The start `hal::thread` calls on a new thread with the scheduler's hold its switch kept
/// and the two words `fork1` gave: `proc_trampoline_mi` (`kernel::sched::trampoline_mi`,
/// which releases the hold), then what [`Start`] names. Returns to the trampoline when the
/// thread is to return to user mode (`child_return`).
fn thread_start<G: ProcGlue>(hold: SchedHold, start: usize, arg: usize) {
    crate::sched::trampoline_mi::<G>(hold);
    let Some(s) = Start::from_words(start, arg) else {
        kpanic(format_args!("proc_trampoline: no thread start {start:#x}"));
    };
    match s {
        Start::ChildReturn => G::with_curproc(|p| match p {
            Some(p) => G::child_return(p),
            None => kpanic(format_args!("child_return: no curproc")),
        }),
        Start::ForkReturn => fork_return::<G>(),
        Start::Idle(n) => crate::sched::sched_idle::<G>(n),
        Start::Kernel(t) => crate::thread::run(t),
        Start::Init => G::with_curproc(|p| match p {
            Some(p) => G::start_init(p),
            None => kpanic(format_args!("start_init: no curproc")),
        }),
        Start::Reaper => crate::exit::reaper::<G>(),
        Start::Acct => crate::acct::acct_thread::<G>(),
        Start::Legacy(f, arg) => f(ptr::with_exposed_provenance_mut(arg)),
    }
}

/// `fork_return`: the first thing a forked child runs when the parent asked `ptrace(2)` to
/// report forks: stop for the tracer, then return to user mode as `child_return` does.
fn fork_return<G: ProcGlue>() {
    hal::thread::with_current::<Thread<G>, _>(|p| {
        if p.process().ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
            crate::sig::psignal(p, SIGTRAP);
        }
        G::child_return(&p);
    });
}

/// `fork(2)`: a new process; its pid.
pub fn sys_fork<G: ProcGlue>(p: Pin<&Thread<G>>) -> Result<Pid, Errno> {
    // ps_ptmask & PTRACE_FORK (flags |= FORK_PTRACE, start = ForkReturn): the ptrace event
    // mask is sys_process.c's, which is not ported, so no process asks for fork reports.
    fork1(p, FORK_FORK, Start::ChildReturn).map(|c| c.pid)
}

/// `vfork(2)`: the parent sleeps until the child execs or exits (`FORK_PPWAIT`).
pub fn sys_vfork<G: ProcGlue>(p: Pin<&Thread<G>>) -> Result<Pid, Errno> {
    fork1(p, FORK_VFORK | FORK_PPWAIT, Start::ChildReturn).map(|c| c.pid)
}

/// `__tfork(2)`: a new thread in the calling process with its own stack and TCB, from the
/// `psize` bytes of the parameters at `param` (the C reads `min(psize, sizeof)`, `EINVAL`
/// for 0 or more); its id.
#[allow(non_snake_case, reason = "the C name, sys___tfork")]
pub fn sys___tfork<G: ProcGlue>(
    p: Pin<&Thread<G>>,
    param: usize,
    psize: usize,
) -> Result<Pid, Errno> {
    let mut raw = [0u8; size_of::<Tfork>()];
    if psize == 0 || psize > raw.len() {
        return Err(Errno::EINVAL);
    }
    UserPtr::<u8>::new(param).copyin_slice(&mut raw[..psize])?;
    let tf = tfork_from_bytes(&raw);
    // KTRACE: not configured.
    if G::tcb_invalid(tf.tf_tcb) {
        return Err(Errno::EINVAL);
    }
    thread_fork(p, tf.tf_stack, tf.tf_tcb, UserPtr::new(tf.tf_tid))
}

/// The parameters of `__tfork(2)` from their bytes (a short copy leaves the rest zero).
fn tfork_from_bytes(raw: &[u8; size_of::<Tfork>()]) -> Tfork {
    let words: [usize; 3] = core::array::from_fn(|i| {
        const W: usize = size_of::<usize>();
        let mut w = [0u8; W];
        w.copy_from_slice(&raw[i * W..(i + 1) * W]);
        usize::from_ne_bytes(w)
    });
    Tfork {
        tf_tcb: words[0],
        tf_tid: words[1],
        tf_stack: words[2],
    }
}

/// `thread_new`: a thread of `pr` copying `parent`'s inheritable members, with the u-area `u`;
/// its box, which the caller keeps until the thread joins its process.
fn thread_new<G: ProcGlue>(parent: &Thread<G>, pr: &ProcessRef<G>, u: UArea) -> PoolBox<Thread<G>> {
    let p = crate::proc::thread_alloc::<G>(pr, G::ThreadG::default());

    p.set_stat(SIDL); // protect against others
    p.p_runpri.set(0);
    p.p_flag.store(0, Ordering::Relaxed);

    // The zeroed section is `thread_alloc`'s; the section copied from the parent
    // (p_startcopy .. p_endcopy):
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
    p.install_uarea(u);

    // p_sleep_to is named by the thread's tid, set once it has one (sleep_timeout_init).
    p
}

/// `process_initialize`: the parts of a process common to process 0 and a forked one, given
/// its first thread `p`: `p` joins `ps_threads`, the process takes its credentials, the
/// timeouts are set. The caller makes `p` the process's main thread (`set_main_static` for
/// process 0, `set_main_locked` at the end of `fork1`).
pub fn process_initialize<G: ProcGlue>(pr: &Process<G>, p: Pin<&Thread<G>>) {
    {
        let mut g = pr.ps_mtx.lock();
        pr.join_locked(&mut g, p);
    }
    pr.ps_threadcnt.set(1);

    // give the process the same creds as the initial thread
    drop(pr.set_ucred(p.cred_ref()));

    // prof_fork(pr): subr_prof.c.
    let _ = crate::prf::unported!("process_initialize: prof_fork (M6)");

    // A new process's lists (children, orphans) are empty; no sigio registration names its
    // pid yet (kernel::sigio's registry).
    G::process_initialized(pr);
    crate::resource::rucheck_init(pr);
}

/// `process_new`: the new process `pr` (made by `fork1` with its pid, `p` its first thread)
/// filled from `parent`, and placed on `allprocess`.
fn process_new<G: ProcGlue>(
    p: Pin<&Thread<G>>,
    pr: &ProcessRef<G>,
    parent: &ProcessRef<G>,
    flags: i32,
) {
    // The zeroed section is `process_alloc`'s; the copied one (ps_startcopy .. ps_endcopy):
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
    crate::resource::lim_fork(parent, pr);

    // post-copy fixups
    {
        let mut w = klock::witness();
        drop(pr.set_pptr(&mut w, parent.clone()));
    }
    // WITNESS_SETCHILD: not configured.

    // bump references to the text vnode (for sysctl); copy unveil if unveil is active
    G::process_forked(parent, pr);

    pr.ps_flags.store(
        parent.ps_flags.load(Ordering::Relaxed) & PS_FLAGS_INHERITED_ON_FORK,
        Ordering::Relaxed,
    );
    let ctty = {
        let r = klock::reader();
        parent.session(&r).is_some_and(G::session_has_tty_vnode)
    };
    if ctty {
        pr.ps_flags.fetch_or(
            parent.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT,
            Ordering::Relaxed,
        );
    }

    drop(pr.ps_pin.replace(PinTable::duplicate(&parent.ps_pin)));
    drop(
        pr.ps_libcpin
            .replace(PinTable::duplicate(&parent.ps_libcpin)),
    );

    // Duplicate sub-structures as needed. Increase reference counts on shared objects.
    if flags & FORK_SHAREFILES != 0 {
        pr.set_fd(crate::descrip::fdshare(parent));
    } else {
        pr.set_fd(crate::descrip::fdcopy(parent));
    }
    crate::sig::sigactsinit(pr, parent);
    let Some(pvm) = parent.vmspace_share() else {
        kpanic(format_args!("process_new: the parent has no vmspace"));
    };
    let vm = if flags & FORK_SHAREVM != 0 {
        crate::vm::map::uvmspace_share(&pvm)
    } else {
        crate::vm::map::uvmspace_fork(&pvm)
    };
    drop(pvm);
    let prev = pr.set_vmspace(Some(vm));
    kassert!(prev.is_none());

    if parent.ps_flags.load(Ordering::Relaxed) & PS_PROFIL != 0 {
        let _ = crate::prf::unported!("process_new: startprofclock (subr_prof.c, M6)");
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
    G::tables().allprocess.insert_head(&mut w, pr.as_pin());
}

/// `ratecheck(lasttime, mininterval)` (`kern_time.c`) over the uptime of the last tick: true
/// when `mininterval` passed since `*last` (or `*last` is zero), which then becomes now.
pub fn ratecheck(last: &mut Timespec, mininterval: &Timespec) -> bool {
    let now = clock::getnanouptime();
    let delta = now.sub(*last);
    if delta >= *mininterval || *last == Timespec::ZERO {
        *last = now;
        return true;
    }
    false
}

/// `tablefull(tab)`: logs that a table is full.
fn tablefull<G: ProcGlue>(tab: &str) {
    G::log(LOG_ERR, format_args!("{tab}: table is full\n"));
}

/// `fork_check_maxthread`: one more thread for user `uid`, within `maxthread` (the last five
/// are root's); `EAGAIN` otherwise (and a "table is full" log, at most once per 10 seconds).
pub fn fork_check_maxthread<G: ProcGlue>(uid: Uid) -> Result<(), Errno> {
    let maxthread_local = G::maxthread();
    let val = NTHREADS.fetch_add(1, Ordering::Relaxed) + 1;
    if (val > maxthread_local - 5 && uid != 0) || val > maxthread_local {
        if ratecheck(&mut LASTTFM_THREAD.lock(), &FORK_TFMRATE) {
            tablefull::<G>("thread");
        }
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::EAGAIN);
    }
    Ok(())
}

/// `fork1`'s statistics (`kern.forkstat`): a fork or a vfork counts with the size of the new
/// address space (data plus stack); anything else is a kernel thread, counted without one:
/// `sizkthread` is never added to, as the C's (kern_fork.c:440-449; decision 13 of N4.6).
fn forkstat_add(fs: &Forkstat, flags: i32, vmsize: u64) {
    if flags & FORK_FORK != 0 {
        fs.cntfork.fetch_add(1, Ordering::Relaxed);
        fs.sizfork.fetch_add(vmsize, Ordering::Relaxed);
    } else if flags & FORK_VFORK != 0 {
        fs.cntvfork.fetch_add(1, Ordering::Relaxed);
        fs.sizvfork.fetch_add(vmsize, Ordering::Relaxed);
    } else {
        fs.cntkthread.fetch_add(1, Ordering::Relaxed);
    }
}

/// `fork_thread_start`'s choice: the CPU a new thread starts on and its priority.
fn fork_cpu<G: ProcGlue>() -> usize {
    crate::sched::sched_choosecpu_fork::<G>()
}

/// `fork1`: a new process whose first thread starts with `start`, made on behalf of `curp`
/// (the running thread for `fork(2)`/`vfork(2)`, process 0's for a kernel thread).
pub fn fork1<G: ProcGlue>(
    curp: Pin<&Thread<G>>,
    flags: i32,
    start: Start,
) -> Result<ForkChild<G>, Errno> {
    fork1_with(curp, flags, start, &mut |_| {})
}

/// [`fork1`] that lends the new thread to `child` once it is linked (and placed by the glue)
/// and before anything can run it: the C's `*rnewprocp` for a caller that keeps the thread.
pub fn fork1_with<G: ProcGlue>(
    curp: Pin<&Thread<G>>,
    flags: i32,
    start: Start,
    child: &mut dyn FnMut(Pin<&Thread<G>>),
) -> Result<ForkChild<G>, Errno> {
    let curpr = curp.process_ref().clone();
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

    fork_check_maxthread::<G>(uid)?;

    let maxprocess_local = G::maxprocess();
    let nprocesses = NPROCESSES.load(Ordering::Relaxed);
    if (nprocesses >= maxprocess_local - 5 && uid != 0) || nprocesses >= maxprocess_local {
        if ratecheck(&mut LASTTFM_PROCESS.lock(), &FORK_TFMRATE) {
            tablefull::<G>("process");
        }
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::EAGAIN);
    }
    NPROCESSES.fetch_add(1, Ordering::Relaxed);

    // Increment the count of processes running with this uid. Don't allow a nonprivileged
    // user to exceed their current limit.
    let count = crate::proc::chgproccnt::<G>(uid, 1);
    if uid != 0 && count as u64 > curp.lim_cur(RLIMIT_NPROC) {
        crate::proc::chgproccnt::<G>(uid, -1);
        NPROCESSES.fetch_sub(1, Ordering::Relaxed);
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::EAGAIN);
    }

    let Some(uaddr) = UArea::alloc() else {
        crate::proc::chgproccnt::<G>(uid, -1);
        NPROCESSES.fetch_sub(1, Ordering::Relaxed);
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::ENOMEM);
    };

    // From now on, we're committed to the fork and cannot fail.
    let pr = crate::proc::process_alloc::<G>(allocpid::<G>(), G::ProcessG::default());
    let t = thread_new(&curp, &pr, uaddr);
    let p = t.as_pin();
    process_new(p, &pr, &curpr, flags);

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

    // Finish creating the child thread: cpu_fork copies and updates the pcb and makes the
    // child ready to run. A user fork exits to user mode via child_return on its first time
    // slice; a kernel thread runs its entry point.
    let me = ptr::from_ref::<Thread<G>>(&p).expose_provenance();
    cpu_fork(curp, p, 0, 0, start.words(me));

    let vmsize = pr.vmspace_share().map_or(0, |vm| {
        (vm.vm_dsize.load(Ordering::Relaxed) as i64 + vm.vm_ssize.load(Ordering::Relaxed) as i64)
            as u64
    });
    forkstat_add(&FORKSTAT, flags, vmsize);

    let tid = alloctid::<G>();
    p.p_tid.set(tid);
    crate::sched::sleep_timeout_init(&p);
    {
        let mut w = klock::witness();
        let tb = G::tables();
        tb.allproc.insert_head(&mut w, p);
        tb.tidhash(tid).insert_head(&mut w, p);
        tb.pidhash(pr.ps_pid).insert_head(&mut w, pr.as_pin());

        fork_join_pgrp(&mut w, &curpr, &pr);
        curpr.ps_children.insert_head(&mut w, pr.as_pin());
    }
    G::thread_placed(Placed::new(p));
    child(p);

    if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        // ps_opptr, process_reparent, the ptrace status: sys_process.c.
        let _ = crate::prf::unported!("fork1: ptrace (process_reparent, ps_ptstat, M6)");
    }

    // For new processes, set accounting bits and mark as complete.
    pr.ps_start.set(clock::nanouptime());
    pr.ps_acflag.set(crate::acct::AFORK);
    pr.ps_flags.fetch_and(!PS_EMBRYO, Ordering::Relaxed);

    // The process owns its main thread from here on; an idle thread is only assigned to its
    // CPU (and pegged), any other goes on a run queue in the same section, so nothing runs
    // the thread before its process has its box.
    {
        let mut g = pr.ps_mtx.lock();
        if let Start::Idle(n) = start {
            kassert!(flags & FORK_IDLE != 0);
            crate::sched::set_thread_cpu(&p, n);
            // for consistency mark idle procs as pegged
            p.p_flag.fetch_or(P_CPUPEG, Ordering::Relaxed);
        } else {
            fork_thread_start::<G>(p);
        }
        pr.set_main_locked(&mut g, t);
    }

    // Notify any interested parties about the new process.
    crate::event::knote_processfork(&curpr, pr.ps_pid);

    // Update stats now that we know the fork was successful.
    UVMEXP.forks.fetch_add(1, Ordering::Relaxed);
    if flags & FORK_PPWAIT != 0 {
        UVMEXP.forks_ppwait.fetch_add(1, Ordering::Relaxed);
    }
    if flags & FORK_SHAREVM != 0 {
        UVMEXP.forks_sharevm.fetch_add(1, Ordering::Relaxed);
    }

    // Preserve synchronization semantics of vfork: sleep on our process until the child
    // execs or exits (PS_ISPWAIT cleared). XXX Need to stop other rthreads in the parent
    if flags & FORK_PPWAIT != 0 {
        while curpr.ps_flags.load(Ordering::Relaxed) & PS_ISPWAIT != 0 {
            let _ = crate::sched::tsleep_nsec::<G>(
                ptr::from_ref::<Process<G>>(&curpr).addr(),
                PWAIT,
                "ppwait",
                hal::sleep::INFSLP,
            );
        }
    }

    // If we're tracing the child, alert the parent too.
    if flags & FORK_PTRACE != 0 && curpr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        crate::sig::psignal(curp, SIGTRAP);
    }

    Ok(ForkChild {
        pid: pr.ps_pid,
        tid,
        process: pr,
    })
}

/// `cpu_fork(parent, child, stack, tcb, func, arg)`: the child's address space activated
/// (the machine's `pmap_activate`) and its frames built by `hal` (the parent's pcb and trap
/// frame copied, the user stack and TCB when not 0, the trampoline with the start's words).
/// `cpu_exit` is empty on both machines (amd64 and arm64 vm_machdep.c:116-118) and has no
/// counterpart.
fn cpu_fork<G: ProcGlue>(
    parent: Pin<&Thread<G>>,
    child: Pin<&Thread<G>>,
    stack: usize,
    tcb: usize,
    (start, arg): (usize, usize),
) {
    G::pmap_activate(&child);
    let h = hal::thread::sched_lock();
    hal::thread::thread_setup(&h, parent, child, stack, tcb, start, arg);
    drop(h);
}

/// `fork_thread_start`: puts the new thread on a run queue (the caller holds its process's
/// `ps_mtx`, which comes before the scheduler's lock).
fn fork_thread_start<G: ProcGlue>(p: Pin<&Thread<G>>) {
    let cpu = fork_cpu::<G>();
    let mut h = hal::thread::sched_lock();
    // TRACEPOINT(sched, fork, ...): dt(4), not configured.
    crate::sched::setrunqueue(&mut h, Some(cpu), p, p.p_usrpri.get());
    drop(h);
}

/// `pr->ps_pgrp = curpr->ps_pgrp; LIST_INSERT_AFTER(curpr, pr, ps_pglist)`: the child `pr`
/// joins its parent's group right after the parent among the members (the order
/// `setpriority(PRIO_PGRP)` and `ttyinfo` read); a parent in no group leaves it in none.
pub fn fork_join_pgrp<G: ProcGlue>(
    w: &mut klock::KernelWitness,
    curpr: &Process<G>,
    pr: &ProcessRef<G>,
) {
    let pgrp = curpr.pgrp(&*w).cloned();
    if let Some(pg) = &pgrp {
        pg.pg_members.insert_after(w, curpr, pr.as_pin());
    }
    drop(pr.set_pgrp(w, pgrp));
}

/// `thread_fork`: a new thread of `curp`'s process that returns to user mode on `stack` with
/// `tcb` as its TCB; its id (offset by `THREAD_PID_OFFSET`), copied out to `tidptr` too when
/// that is not NULL (a fault there is a `SIGSEGV` to the caller, the thread is made).
pub fn thread_fork<G: ProcGlue>(
    curp: Pin<&Thread<G>>,
    stack: usize,
    tcb: usize,
    tidptr: UserPtr<Pid>,
) -> Result<Pid, Errno> {
    let pr = curp.process_ref().clone();

    if stack == 0 {
        return Err(Errno::EINVAL);
    }

    fork_check_maxthread::<G>(curp.ucred().cr_ruid.get())?;

    let Some(uaddr) = UArea::alloc() else {
        NTHREADS.fetch_sub(1, Ordering::Relaxed);
        return Err(Errno::ENOMEM);
    };

    // From now on, we're committed to the fork and cannot fail.
    let t = thread_new(&curp, &pr, uaddr);
    let p = t.as_pin();
    p.p_flag.fetch_or(P_THREAD, Ordering::Relaxed);
    crate::sig::sigstkinit(&p.p_sigstk);
    p.set_name(b"");

    // other links: p_p, made with the thread (thread_alloc).

    // local copies (p_fd: the thread reads its process's table, `Thread::fd`)
    drop(p.set_vmspace(pr.vmspace_share()));

    // Finish creating the child thread: it returns to user mode via child_return on its
    // first time slice.
    let me = ptr::from_ref::<Thread<G>>(&p).expose_provenance();
    cpu_fork(curp, p, stack, tcb, Start::ChildReturn.words(me));

    let tid = alloctid::<G>();
    p.p_tid.set(tid);
    crate::sched::sleep_timeout_init(&p);

    {
        let mut w = klock::witness();
        let tb = G::tables();
        tb.allproc.insert_head(&mut w, p);
        tb.tidhash(tid).insert_head(&mut w, p);
    }
    G::thread_placed(Placed::new(p));

    {
        let mut g = pr.ps_mtx.lock();
        pr.join_locked(&mut g, p);
        pr.ps_threadcnt.set(pr.ps_threadcnt.get() + 1);

        // if somebody else wants to take us to single threaded mode or suspend the process,
        // count ourselves in.
        if !pr.ps_single.get().is_null() || pr.ps_flags.load(Ordering::Relaxed) & PS_STOPPING != 0 {
            pr.ps_suspendcnt.set(pr.ps_suspendcnt.get() + 1);
            p.p_flag.fetch_or(
                curp.p_flag.load(Ordering::Relaxed) & (P_SUSPSINGLE | P_SUSPSIG),
                Ordering::Relaxed,
            );
        }
    }

    // Return tid to parent thread and copy it out to userspace
    let utid: Pid = tid + THREAD_PID_OFFSET;
    if !tidptr.is_null() && tidptr.copyout(&utid).is_err() {
        crate::sig::psignal(curp, SIGSEGV);
    }

    // The process owns the thread's box from here on; it goes on a run queue in the same
    // section (see `fork1`).
    {
        let mut g = pr.ps_mtx.lock();
        fork_thread_start::<G>(p);
        pr.insert_box_locked(&mut g, t);
    }

    // Update stats now that we know the fork was successful.
    FORKSTAT.cnttfork.fetch_add(1, Ordering::Relaxed);
    UVMEXP.forks.fetch_add(1, Ordering::Relaxed);
    UVMEXP.forks_sharevm.fetch_add(1, Ordering::Relaxed);

    Ok(utid)
}

/// `alloctid`: an unused tid, at random in `[1, TID_MASK + 1]`.
pub fn alloctid<G: ProcGlue>() -> Pid {
    loop {
        let tid = 1 + (hal::random::arc4random() as Pid & TID_MASK);
        let r = klock::reader();
        if crate::proc::tfind::<G, _>(&r, tid).is_none() {
            return tid;
        }
    }
}

/// `ispidtaken`: whether `pid` is in use as a pid, a process group's id or a zombie's, or
/// was freed recently (`oldpids`).
pub fn ispidtaken<G: ProcGlue>(pid: Pid) -> bool {
    if OLDPIDS.iter().any(|o| o.load(Ordering::Relaxed) == pid) {
        return true;
    }
    let r = klock::reader();
    crate::proc::prfind::<G, _>(&r, pid).is_some()
        || crate::proc::pgfind::<G, _>(&r, pid).is_some()
        || crate::proc::zombiefind::<G, _>(&r, pid).is_some()
}

/// `allocpid`: an unused pid: 1 first, then at random in `[2, PID_MAX]`.
pub fn allocpid<G: ProcGlue>() -> Pid {
    // The first PID allocated is always 1.
    if FIRSTPID.swap(0, Ordering::Relaxed) == 1 {
        return 1;
    }
    loop {
        let pid = 2 + hal::random::arc4random_uniform((PID_MAX - 1) as u32) as Pid;
        if !ispidtaken::<G>(pid) {
            return pid;
        }
    }
}

/// `freepid`: remembers a freed pid so that it is not reused right away.
pub fn freepid(pid: Pid) {
    let idx = OLDPIDS_IDX.fetch_add(1, Ordering::Relaxed) as usize % OLDPIDS.len();
    OLDPIDS[idx].store(pid, Ordering::Relaxed);
}

/// The host tests' reset: `allocpid`'s first pid again, the ring and the rate limiters
/// emptied (`testing.md`'s rule: a global the tests touch).
#[cfg(any(test, feature = "doubles"))]
pub fn fork_test_reset() {
    FIRSTPID.store(1, Ordering::Relaxed);
    for o in &OLDPIDS {
        o.store(0, Ordering::Relaxed);
    }
    *LASTTFM_THREAD.lock() = Timespec::ZERO;
    *LASTTFM_PROCESS.lock() = Timespec::ZERO;
}

const _: () = {
    assert!(size_of::<Tfork>() == 3 * size_of::<usize>());
    assert!(TAG_LEGACY > TAG_ACCT);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_a(_arg: *mut c_void) {}
    fn legacy_b(_arg: *mut c_void) {}

    #[test]
    fn start_words_round_trip_every_variant() {
        let me = 0xdead_0000;
        for s in [
            Start::ChildReturn,
            Start::ForkReturn,
            Start::Idle(3),
            Start::Kernel(7),
            Start::Init,
            Start::Reaper,
            Start::Acct,
        ] {
            let (w0, w1) = s.words(me);
            let back = Start::from_words(w0, w1).unwrap();
            assert_eq!(back.words(me), (w0, w1));
        }
        assert!(Start::from_words(0, 0).is_none());
        assert!(Start::from_words(TAG_LEGACY + NLEGACY, 0).is_none());
    }

    #[test]
    fn a_legacy_start_with_arg_0_gets_the_thread_s_address() {
        let (wa, arg) = Start::Legacy(legacy_a, 0).words(0x1234_5000);
        assert_eq!(arg, 0x1234_5000);
        let (wb, arg) = Start::Legacy(legacy_b, 42).words(0x1234_5000);
        assert_eq!(arg, 42);
        assert_ne!(wa, wb);
        // the same function keeps its slot
        assert_eq!(Start::Legacy(legacy_a, 9).words(1).0, wa);
        match Start::from_words(wb, 42) {
            Some(Start::Legacy(f, 42)) => {
                assert!(ptr::fn_addr_eq(f, legacy_b as fn(*mut c_void)));
            }
            _ => panic!("not legacy_b"),
        }
    }

    #[test]
    fn tfork_parameters_are_read_by_psize() {
        let mut raw = [0u8; size_of::<Tfork>()];
        raw[..8].copy_from_slice(&0x1000usize.to_ne_bytes());
        raw[16..].copy_from_slice(&0x3000usize.to_ne_bytes());
        let tf = tfork_from_bytes(&raw);
        assert_eq!((tf.tf_tcb, tf.tf_tid, tf.tf_stack), (0x1000, 0, 0x3000));
    }

    #[test]
    fn freed_pids_stay_taken_for_a_while() {
        // The ring alone (no lookup: the pid is in it).
        freepid(77_777);
        assert!(OLDPIDS.iter().any(|o| o.load(Ordering::Relaxed) == 77_777));
        for i in 0..OLDPIDS.len() as Pid {
            freepid(80_000 + i);
        }
        assert!(!OLDPIDS.iter().any(|o| o.load(Ordering::Relaxed) == 77_777));
    }

    #[test]
    fn ratecheck_lets_one_through_per_interval() {
        crate::testutil::install();
        let mut last = Timespec::ZERO;
        // zero: the first message is always seen
        assert!(ratecheck(&mut last, &Timespec::new(3600, 0)));
        let after = last;
        if after != Timespec::ZERO {
            assert!(!ratecheck(&mut last, &Timespec::new(3600, 0)));
            assert_eq!(last, after);
        }
        assert!(ratecheck(&mut last, &Timespec::ZERO));
    }

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// A thread of a new process `pid` whose real and effective uid are `uid`, with an
    /// `RLIMIT_NPROC` of `nproc`.
    fn thread_as(pid: Pid, uid: Uid, nproc: u64) -> (ProcessRef<Procs>, PoolBox<Thread<Procs>>) {
        std::thread_local! {
            static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        crate::testutil::install();
        if !MADE.with(|m| m.replace(true)) {
            crate::proc::procinit::<Procs>(64, 64);
        }
        let pr = crate::proc::process_alloc::<Procs>(pid, ());
        let mut ids = crate::cred::Ucred::new();
        ids.cr_uid.set(uid);
        ids.cr_ruid.set(uid);
        drop(pr.set_ucred(crate::cred::CredRef::new(ids)));
        let mut limits = crate::resource::Plimit::new();
        limits.pl_rlimit[RLIMIT_NPROC].rlim_cur = nproc;
        limits.pl_rlimit[RLIMIT_NPROC].rlim_max = nproc;
        drop(pr.set_limit(crate::resource::PlimitRef::new(limits)));
        let t = crate::proc::thread_alloc::<Procs>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        (pr, t)
    }

    /// The user's decision 12 of N4.6 (kern_fork.c:386-393): a user other than root over
    /// `RLIMIT_NPROC` gets `EAGAIN` from `fork1`, and every count it took is given back (the
    /// uid's processes, `nprocesses`, `nthreads`), whatever the flags.
    #[test]
    fn a_user_over_rlimit_nproc_gets_eagain_and_the_counts_roll_back() {
        // (processes the uid already has, its limit, the flags)
        let rows = [
            (0, 0, FORK_FORK),
            (1, 1, FORK_FORK),
            (3, 2, FORK_FORK),
            (2, 2, FORK_VFORK | FORK_PPWAIT),
            (0, 0, FORK_FORK | FORK_SHAREVM),
        ];
        for (i, &(already, nproc, flags)) in rows.iter().enumerate() {
            let uid: Uid = 1000 + i as Uid;
            let (_pr, t) = thread_as(500 + i as Pid, uid, nproc);
            for _ in 0..already {
                crate::proc::chgproccnt::<Procs>(uid, 1);
            }
            let processes = NPROCESSES.load(Ordering::Relaxed);
            let threads = NTHREADS.load(Ordering::Relaxed);
            let r = fork1::<Procs>(t.as_pin(), flags, Start::ChildReturn);
            assert_eq!(r.err(), Some(Errno::EAGAIN), "row {i}");
            assert_eq!(NPROCESSES.load(Ordering::Relaxed), processes, "row {i}");
            assert_eq!(NTHREADS.load(Ordering::Relaxed), threads, "row {i}");
            assert_eq!(crate::proc::chgproccnt::<Procs>(uid, 0), already, "row {i}");
        }
    }

    /// The user's decision 13 of N4.6 (kern_fork.c:440-449): a fork and a vfork count with
    /// their address space's size, a kernel thread (any other flags) without one:
    /// `sizkthread` stays 0.
    #[test]
    fn forkstat_counts_kernel_threads_without_their_size() {
        let fs = Forkstat::new();
        forkstat_add(&fs, FORK_FORK, 10);
        forkstat_add(&fs, FORK_FORK | FORK_SHAREVM, 5);
        forkstat_add(&fs, FORK_VFORK | FORK_PPWAIT, 7);
        forkstat_add(
            &fs,
            FORK_SHAREVM | FORK_SHAREFILES | FORK_NOZOMBIE | FORK_SYSTEM,
            99,
        );
        forkstat_add(&fs, FORK_IDLE | FORK_SHAREVM | FORK_SYSTEM, 99);
        let get32 = |a: &AtomicU32| a.load(Ordering::Relaxed);
        let get64 = |a: &core::sync::atomic::AtomicU64| a.load(Ordering::Relaxed);
        assert_eq!((get32(&fs.cntfork), get64(&fs.sizfork)), (2, 15));
        assert_eq!((get32(&fs.cntvfork), get64(&fs.sizvfork)), (1, 7));
        assert_eq!((get32(&fs.cntkthread), get64(&fs.sizkthread)), (2, 0));
        assert_eq!((get32(&fs.cnttfork), get64(&fs.siztfork)), (0, 0));
    }
}
/* </TESTS> */
