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
 * Copyright (c) 1997 Theo de Raadt. All rights reserved.
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
 *	@(#)kern_sig.c	8.7 (Berkeley) 4/18/94
 */

/*
 * Copyright (c) 1991, 1993
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
 *	@(#)signalvar.h	8.3 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! Signals: `kern/kern_sig.c` and `<sys/signalvar.h>`: the signal actions of a process
//! ([`Sigacts`]), the generation of signals (`psignal`, `prsignal`, `ptsignal`, `killpg1`,
//! `pgsignal`, `trapsignal`), their delivery (`cursig`, `postsig`, `userret`), the stops and
//! continues of job control and of the debugger (`process_stop`, `process_continue`,
//! `proc_trap`), single-threading (`single_thread_set`, `single_thread_clear`) and the
//! system calls' bodies (`sigaction(2)`, `sigprocmask(2)`, `sigsuspend(2)`,
//! `sigaltstack(2)`, `kill(2)`, `thrkill(2)`, `sigwait(3)`'s `__thrsigdivert`).
//!
//! Upstream: sys/kern/kern_sig.c @ 3ce1f3f79392
//! Upstream: sys/sys/signalvar.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_sig.rs@3c62ede68802
//! LZ: sys/sys/signalvar.rs@3c62ede68802
//!
//! What stays `bsd`'s is reached through [`ProcGlue`]: `exit1` (`kern_exit.c`, N4.6), the
//! core dump (the file systems, N5), and the machine's part (`sendsig`, the thread's stack
//! pointer and TCB, `signotify`'s AST and kick, N4.8). The system calls' argument decoding,
//! `pledge_kill` and `sys_sigreturn` are `bsd`'s `sysent[]` rows (`sys/kern/kern_sig.rs`);
//! the `sigio` registrations are [`crate::sigio`]'s.
//!
//! Locks, as the C's annotations: `[m]` the process's `ps_mtx` (the signal actions, the
//! thread list), `[S]` `SCHED_LOCK` (`hal::thread::sched_lock`: a thread's state, its sleep),
//! `[K]` the kernel lock (the process lists and groups, which `killpg1` and `pgsignal` walk),
//! `[a]` atomics (`ps_siglist`, `p_siglist`, `ps_sigflags`), `[o]` the thread itself
//! (`p_sigmask`, `p_sigstk`). The order is `ps_mtx` before `SCHED_LOCK` (`ptsignal_locked`
//! takes the latter inside the former), and a child's `ps_mtx` before its parent's
//! (`process_suspend_signal` signals the parent under the child's).
//!
//! ## Deviations
//! - `KTRACE`, `WITNESS`, dt(4)'s `TRACEPOINT` and `SMALL_KERNEL` are not configured.
//! - `sigactsinit` copies the parent's actions under the parent's `ps_mtx`, a lock the C's
//!   `memcpy` does without (the actions are `[m]`; holding it costs one uncontended
//!   mutex pair on `fork(2)`).
//! - `ptsignal_locked` reaches the running thread (`curproc`) it diverts a process signal to
//!   through its process's thread list (the pinned handle the scheduler needs), a walk the C
//!   does not make; a process has one thread unless it uses threads.
//!
//! ## Redesign
//! - `struct sigacts *ps_sigacts` (a `pool_get` written in place, `&'static` for ever, its
//!   `[m]` members `Cell`s under an `unsafe impl Sync` saying "`ps_mtx`") -> a
//!   [`Sigacts`] whose `[m]` members are one [`Guarded`] cell keyed to its process's
//!   `ps_mtx`, opened only with that mutex's guard, in a [`PoolBox`] of `sigapl`
//!   ([`SIGACTS_POOL`]) the process holds ([`SigactsRef`], `Process::ps_sigacts`; process
//!   0's is a `static`, `sigacts0`): `sigactsfree` is the box's drop with the process.
//! - Functions that make a thread runnable (`ptsignal_locked`, `process_continue`,
//!   `process_stop`, `thrkill`, `userret`, `trapsignal`) take the thread pinned
//!   (`Pin<&Thread>`, the scheduler's queues link it); the others a plain reference.
//! - The machine's and the other subsystems' calls (`sendsig`, `exit1`, `coredump`,
//!   `signotify`, `PROC_STACK`, `TCB_GET`, `initprocess`) are [`ProcGlue`] methods.
//! - `sleep_signal_check` (`kern_synch.c`) is `kernel::sched`'s, over this module.
//! - `p_sigstk` and `p_sigval` are the thread's own words ([`SigaltstackWord`],
//!   [`SigvalWord`]), members of `kernel::proc::Thread`.

use core::ops::Deref;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering, fence};

use hal::intr::IPL_NONE;
use hal::klock;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::pool::{BoxPool, PR_WAITOK, PoolBox};
use hal::rwlock::{Exclusive, Guarded, Witness};
use hal::sleep::{INFSLP, PCATCH, PWAIT};
use hal::sys::errno::Errno;
use hal::sys::siginfo::{ILL_BTCFI, SI_USER, Siginfo, Sigval};
use hal::sys::signal::{
    MINSIGSTKSZ, NSIG, SA_NOCLDSTOP, SA_NOCLDWAIT, SA_NODEFER, SA_ONSTACK, SA_RESETHAND,
    SA_RESTART, SA_SIGINFO, SIG_BLOCK, SIG_DFL, SIG_IGN, SIG_SETMASK, SIG_UNBLOCK, SIGABRT,
    SIGALRM, SIGBUS, SIGCHLD, SIGCONT, SIGFPE, SIGHUP, SIGILL, SIGINT, SIGKILL, SIGPROF, SIGSEGV,
    SIGSTOP, SIGSYS, SIGTERM, SIGTSTP, SIGTTIN, SIGTTOU, SIGUSR1, SIGUSR2, SIGVTALRM, SIGXFSZ,
    SS_DISABLE, SS_ONSTACK, Sig, Sigaction, Sigaltstack, Sigset, sigmask,
};
use hal::sys::types::Pid;
use hal::sys::wait::WCOREFLAG;
use hal::thread::{SchedHold, sched_lock};
use hal::user::UserPtr;

use crate::assert::{kassert, kpanic};
use crate::cred::{CredRef, Ucred};
use crate::event::{NOTE_SIGNAL, knote_process_locked};
use crate::proc::{
    EXIT_NORMAL, EXIT_THREAD_NOCHECK, P_ALRMPEND, P_INSCHED, P_PROFPEND, P_SIGSUSPEND, P_SINTR,
    P_SUSPSIG, P_SUSPSINGLE, P_TRACESINGLE, P_WEXIT, PS_CONTINUED, PS_CONTROLT, PS_COREDUMP,
    PS_EXITING, PS_NOBROADCASTKILL, PS_PPWAIT, PS_SINGLEEXIT, PS_SINGLEUNWIND, PS_STOPPED,
    PS_STOPPING, PS_SUGID, PS_SYSTEM, PS_TRACED, PS_TRAPPED, PS_WAITED, PS_WAITEVENT, Pgrp,
    ProcGlue, Process, PsMtx, SDEAD, SIDL, SINGLE_DEEP, SINGLE_EXIT, SINGLE_MASK, SINGLE_SUSPEND,
    SINGLE_UNWIND, SONPROC, SRUN, SSLEEP, SSTOP, Thread, pgfind, prfind, tfind_user_pinned,
    zombiefind,
};
use crate::sched::{self, NOWAKE, NZERO, PUSER};

/// `NSIG` as an array length.
const NSIG_LEN: usize = NSIG as usize;

/// `PPAUSE` (`<sys/param.h>`): the priority of a pause (`sigsuspend`, `sigwait`).
const PPAUSE: i32 = 40;

/// `AXSIG` (`<sys/acct.h>`): killed by a signal.
const AXSIG: u32 = 0x0000_0010;
/// `ATRAP` (`<sys/acct.h>`): memory access violation.
const ATRAP: u32 = 0x0000_0040;
/// `ABTCFI` (`<sys/acct.h>`): BT CFI violation.
const ABTCFI: u32 = 0x0000_0400;

/// `SAS_NOCLDSTOP`: No SIGCHLD when children stop.
pub const SAS_NOCLDSTOP: i32 = 0x01;
/// `SAS_NOCLDWAIT`: No zombies if child dies.
pub const SAS_NOCLDWAIT: i32 = 0x02;

/// `SIG_CATCH`: additional signal action value, used only temporarily/internally.
pub const SIG_CATCH: Sig = 2;
/// `SIG_HOLD`: additional signal action value, used only temporarily/internally.
pub const SIG_HOLD: Sig = 3;

/// `SA_KILL`: terminates process by default.
pub const SA_KILL: i32 = 0x01;
/// `SA_CORE`: ditto and coredumps.
pub const SA_CORE: i32 = 0x02;
/// `SA_STOP`: suspend process.
pub const SA_STOP: i32 = 0x04;
/// `SA_TTYSTOP`: ditto, from tty.
pub const SA_TTYSTOP: i32 = 0x08;
/// `SA_IGNORE`: ignore by default.
pub const SA_IGNORE: i32 = 0x10;
/// `SA_CONT`: continue if suspended.
pub const SA_CONT: i32 = 0x20;
/// `SA_CANTMASK`: non-maskable, catchable.
pub const SA_CANTMASK: i32 = 0x40;

/// `sigprop[NSIG]`: the array below categorizes the signals and their default actions.
pub const SIGPROP: [i32; NSIG_LEN] = [
    0,                    // unused
    SA_KILL,              // SIGHUP
    SA_KILL,              // SIGINT
    SA_KILL | SA_CORE,    // SIGQUIT
    SA_KILL | SA_CORE,    // SIGILL
    SA_KILL | SA_CORE,    // SIGTRAP
    SA_KILL | SA_CORE,    // SIGABRT
    SA_KILL | SA_CORE,    // SIGEMT
    SA_KILL | SA_CORE,    // SIGFPE
    SA_KILL,              // SIGKILL
    SA_KILL | SA_CORE,    // SIGBUS
    SA_KILL | SA_CORE,    // SIGSEGV
    SA_KILL | SA_CORE,    // SIGSYS
    SA_KILL,              // SIGPIPE
    SA_KILL,              // SIGALRM
    SA_KILL,              // SIGTERM
    SA_IGNORE,            // SIGURG
    SA_STOP,              // SIGSTOP
    SA_STOP | SA_TTYSTOP, // SIGTSTP
    SA_IGNORE | SA_CONT,  // SIGCONT
    SA_IGNORE,            // SIGCHLD
    SA_STOP | SA_TTYSTOP, // SIGTTIN
    SA_STOP | SA_TTYSTOP, // SIGTTOU
    SA_IGNORE,            // SIGIO
    SA_KILL,              // SIGXCPU
    SA_KILL,              // SIGXFSZ
    SA_KILL,              // SIGVTALRM
    SA_KILL,              // SIGPROF
    SA_IGNORE,            // SIGWINCH
    SA_IGNORE,            // SIGINFO
    SA_KILL,              // SIGUSR1
    SA_KILL,              // SIGUSR2
    SA_IGNORE,            // SIGTHR
];

/// `CONTSIGMASK`.
const CONTSIGMASK: Sigset = sigmask(SIGCONT);
/// `STOPSIGMASK`.
const STOPSIGMASK: Sigset =
    sigmask(SIGSTOP) | sigmask(SIGTSTP) | sigmask(SIGTTIN) | sigmask(SIGTTOU);

/// The guard of a process's `ps_mtx`: the witness of its `[m]` members.
pub type PsGuard<'a, G> = MutexedGuard<'a, PsMtx<G>>;

/// The `[m]` members of `struct sigacts`: process signal actions and state, needed only
/// within the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SigactsM {
    /// `ps_sigact`: disposition of signals.
    pub ps_sigact: [Sig; NSIG_LEN],
    /// `ps_catchmask`: signals to be blocked.
    pub ps_catchmask: [Sigset; NSIG_LEN],
    /// `ps_sigonstack`: signals to take on sigstack.
    pub ps_sigonstack: Sigset,
    /// `ps_sigintr`: signals interrupt syscalls.
    pub ps_sigintr: Sigset,
    /// `ps_sigreset`: signals that reset when caught.
    pub ps_sigreset: Sigset,
    /// `ps_siginfo`: signals that provide siginfo.
    pub ps_siginfo: Sigset,
    /// `ps_sigignore`: signals being ignored.
    pub ps_sigignore: Sigset,
    /// `ps_sigcatch`: signals being caught by user.
    pub ps_sigcatch: Sigset,
}

impl SigactsM {
    /// All zero: every action `SIG_DFL`, what `sigacts0` holds before `siginit`.
    pub const fn new() -> Self {
        Self {
            ps_sigact: [SIG_DFL; NSIG_LEN],
            ps_catchmask: [0; NSIG_LEN],
            ps_sigonstack: 0,
            ps_sigintr: 0,
            ps_sigreset: 0,
            ps_siginfo: 0,
            ps_sigignore: 0,
            ps_sigcatch: 0,
        }
    }
}

impl Default for SigactsM {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct sigacts`: the `[m]` members in a cell of the owning process's `ps_mtx`, and the
/// `[a]` flags.
pub struct Sigacts {
    /// \[m\] The actions and sets.
    m: Guarded<SigactsM>,
    /// \[a\] `ps_sigflags`: signal flags (`SAS_*`).
    pub ps_sigflags: AtomicI32,
}

impl Sigacts {
    /// `sigacts0`: process 0's, all zero, under `lock` (process 0's `ps_mtx`), made in a
    /// `const` context.
    pub const fn new_static<U>(lock: &'static Mutexed<U>) -> Self {
        Self {
            m: Guarded::new_static_mutexed(lock, SigactsM::new()),
            ps_sigflags: AtomicI32::new(0),
        }
    }

    /// Actions `m` and flags `flags` under `lock` (the new process's `ps_mtx`).
    fn new_under<U>(lock: &Mutexed<U>, m: SigactsM, flags: i32) -> Self {
        Self {
            m: Guarded::new_mutexed(lock, m),
            ps_sigflags: AtomicI32::new(flags),
        }
    }

    /// The `[m]` members, for a holder of the process's `ps_mtx` (`w`, its guard).
    pub fn get<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> &'a SigactsM {
        self.m.get(w)
    }

    /// The `[m]` members, mutably, for the holder of the process's `ps_mtx` (`w`).
    pub fn get_mut<'a, W: Exclusive + ?Sized>(&'a self, w: &'a mut W) -> &'a mut SigactsM {
        self.m.get_mut(w)
    }
}

/// `pr->ps_sigacts`: process 0's `static` `sigacts0`, or a `sigapl` item the process holds
/// until it goes (`sigactsfree` is the box's drop).
pub enum SigactsRef {
    /// `sigacts0`.
    Static(&'static Sigacts),
    /// `sigactsinit`'s.
    Pooled(PoolBox<Sigacts>),
}

impl Deref for SigactsRef {
    type Target = Sigacts;

    fn deref(&self) -> &Sigacts {
        match self {
            SigactsRef::Static(s) => s,
            SigactsRef::Pooled(b) => b,
        }
    }
}

/// `enum signal_type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalType {
    /// `SPROCESS`: process signal, can be diverted (sigwait()).
    SPROCESS,
    /// `STHREAD`: thread signal, but should be propagated if unhandled.
    STHREAD,
}

/// `struct sigctx`: the signal context `cursig` fills for `postsig` and `issignal`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sigctx {
    /// `sig_action`.
    pub sig_action: Sig,
    /// `sig_catchmask`.
    pub sig_catchmask: Sigset,
    /// `sig_onstack`.
    pub sig_onstack: bool,
    /// `sig_intr`.
    pub sig_intr: bool,
    /// `sig_reset`.
    pub sig_reset: bool,
    /// `sig_info`.
    pub sig_info: bool,
    /// `sig_ignore`.
    pub sig_ignore: bool,
    /// `sig_catch`.
    pub sig_catch: bool,
    /// `sig_stop`.
    pub sig_stop: bool,
}

/// `stack_t p_sigstk`: the thread's signal stack, three words written whole by the thread.
#[derive(Debug, Default)]
pub struct SigaltstackWord {
    sp: AtomicUsize,
    size: AtomicUsize,
    flags: AtomicI32,
}

impl SigaltstackWord {
    /// No stack.
    pub const fn new() -> Self {
        Self {
            sp: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
            flags: AtomicI32::new(0),
        }
    }

    /// The stack.
    pub fn get(&self) -> Sigaltstack {
        Sigaltstack {
            ss_sp: self.sp.load(Ordering::Relaxed),
            ss_size: self.size.load(Ordering::Relaxed),
            ss_flags: self.flags.load(Ordering::Relaxed),
            _pad: 0,
        }
    }

    /// Stores `ss`.
    pub fn set(&self, ss: Sigaltstack) {
        self.sp.store(ss.ss_sp, Ordering::Relaxed);
        self.size.store(ss.ss_size, Ordering::Relaxed);
        self.flags.store(ss.ss_flags, Ordering::Relaxed);
    }
}

/// `union sigval p_sigval`: one word.
#[derive(Debug, Default)]
pub struct SigvalWord(AtomicUsize);

impl SigvalWord {
    /// `sival_ptr` 0.
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    /// The value.
    pub fn get(&self) -> Sigval {
        Sigval::from_ptr(self.0.load(Ordering::Relaxed))
    }

    /// Stores `v`.
    pub fn set(&self, v: Sigval) {
        self.0.store(v.sival_ptr(), Ordering::Relaxed);
    }
}

/// `struct timespec` as `__thrsigdivert` reads it from userland.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct UserTimespec {
    /// `tv_sec`.
    pub tv_sec: i64,
    /// `tv_nsec`.
    pub tv_nsec: i64,
}

hal::abi_pod!(UserTimespec {
    tv_sec: i64,
    tv_nsec: i64
});

impl UserTimespec {
    /// `timespecisvalid(ts)`.
    pub const fn is_valid(&self) -> bool {
        self.tv_nsec >= 0 && self.tv_nsec < 1_000_000_000
    }

    /// `TIMESPEC_TO_NSEC(ts)`: saturating at `UINT64_MAX` (a negative `tv_sec` compares as
    /// the C's unsigned conversion does).
    pub const fn to_nsec(self) -> u64 {
        if self.tv_sec as u64 > (u64::MAX - self.tv_nsec as u64) / 1_000_000_000 {
            return u64::MAX;
        }
        (self.tv_sec as u64).wrapping_mul(1_000_000_000) + self.tv_nsec as u64
    }
}

/// `sigacts_pool`: memory pool for sigacts structures (`"sigapl"`).
pub static SIGACTS_POOL: BoxPool<Sigacts> = BoxPool::new();

/// `sigprop[signum]`.
pub fn sigprop(signum: i32) -> i32 {
    SIGPROP[signum as usize]
}

/// `ffs(mask)`: the lowest set bit, counting from 1; 0 for an empty set.
const fn ffs(mask: Sigset) -> i32 {
    if mask == 0 {
        0
    } else {
        mask.trailing_zeros() as i32 + 1
    }
}

/// `sigcantmask`: the signals that can be neither masked nor caught.
pub const fn sigcantmask() -> Sigset {
    sigmask(SIGKILL) | sigmask(SIGSTOP)
}

/// `SIGPENDING(p)`: check if process p has an unmasked signal pending. Return mask of
/// pending signals.
pub fn sigpending<G: ProcGlue>(p: &Thread<G>) -> Sigset {
    (p.p_siglist.load(Ordering::Relaxed) | p.process().ps_siglist.load(Ordering::Relaxed))
        & !p.p_sigmask.get()
}

/// `pr->ps_pgrp->pg_jobc == 0` for a holder of `pr`'s `ps_mtx`: whether the process's group
/// is orphaned (every process is in a group: one without counts as orphaned).
fn pgrp_orphaned<G: ProcGlue>(pr: &Process<G>, g: &PsGuard<'_, G>) -> bool {
    pr.pgrp_locked(g).is_none_or(|pg| pg.pg_jobc.get() == 0)
}

/// `pr->ps_pptr` for a holder of `pr`'s `ps_mtx`: process 0 is its own parent.
fn parent_locked<'a, G: ProcGlue>(pr: &'a Process<G>, g: &'a PsGuard<'_, G>) -> &'a Process<G> {
    match pr.pptr_locked(g) {
        Some(pp) => pp,
        None => pr,
    }
}

/// The address of `pr`'s session (`pr->ps_session`), 0 without one: what the C compares.
fn session_addr<G: ProcGlue, W: Witness + ?Sized>(w: &W, pr: &Process<G>) -> usize {
    pr.session(w).map_or(0, |s| ptr::from_ref(s).addr())
}

/// The address of `curproc`, 0 without one.
fn curproc_addr<G: ProcGlue>() -> usize {
    G::with_curproc(|c| c.map_or(0, |c| ptr::from_ref(c).addr()))
}

/// `p == curproc`.
fn is_curproc<G: ProcGlue>(p: &Thread<G>) -> bool {
    curproc_addr::<G>() == ptr::from_ref(p).addr()
}

/// `curproc`, if it is a thread of `pr`: its address.
fn curproc_in<G: ProcGlue>(pr: &Process<G>) -> Option<usize> {
    G::with_curproc(|c| {
        c.filter(|c| ptr::eq(c.process(), pr))
            .map(|c| ptr::from_ref(c).addr())
    })
}

/// `wakeup(chan)` of the address of `chan`.
fn wakeup_of<G: ProcGlue, T>(chan: &T) {
    sched::wakeup_n::<G>(ptr::from_ref(chan).addr(), -1);
}

/// `cansignal`: can thread `p` send the signal `signum` to process `qr`?
pub fn cansignal<G: ProcGlue>(p: &Thread<G>, qr: &Process<G>, signum: i32) -> bool {
    let pr = p.process();
    let uc = p.ucred();
    let quc = qr.ucred();

    if uc.cr_uid.get() == 0 {
        return true; // root can always signal
    }

    if ptr::eq(pr, qr) {
        return true; // process can always signal itself
    }

    // optimization: if the same creds then the tests below will pass
    if CredRef::ptr_eq(&p.cred_ref(), &quc) {
        return true;
    }

    if signum == SIGCONT {
        let r = klock::reader();
        if session_addr(&r, qr) == session_addr(&r, pr) {
            return true; // SIGCONT in session
        }
    }

    // Using kill(), only certain signals can be sent to setugid child processes
    if qr.ps_flags.load(Ordering::Relaxed) & PS_SUGID != 0 {
        return matches!(
            signum,
            0 | SIGKILL
                | SIGINT
                | SIGTERM
                | SIGALRM
                | SIGSTOP
                | SIGTTIN
                | SIGTTOU
                | SIGTSTP
                | SIGHUP
                | SIGUSR1
                | SIGUSR2
        ) && (uc.cr_ruid.get() == quc.cr_ruid.get()
            || uc.cr_uid.get() == quc.cr_ruid.get());
    }

    uc.cr_ruid.get() == quc.cr_ruid.get()
        || uc.cr_ruid.get() == quc.cr_svuid.get()
        || uc.cr_uid.get() == quc.cr_ruid.get()
        || uc.cr_uid.get() == quc.cr_svuid.get()
}

/// `signal_init`: initialize signal-related data structures (`sigapl`); a second call is a
/// host test's and does nothing.
pub fn signal_init() {
    match SIGACTS_POOL.init(IPL_NONE, PR_WAITOK, "sigapl") {
        Ok(()) | Err(Errno::EBUSY) => {}
        Err(e) => kpanic(format_args!("signal_init: sigapl: {e:?}")),
    }
}

/// `sigstkinit`: initialize a new sigaltstack structure.
pub fn sigstkinit(ss: &SigaltstackWord) {
    ss.set(Sigaltstack {
        ss_flags: SS_DISABLE,
        ss_size: 0,
        ss_sp: 0,
        _pad: 0,
    });
}

/// `sigactsinit(parent)`'s structure: an initial sigacts structure for the new process
/// `pr` (under its `ps_mtx`), using the same signal state as `parent`.
pub fn sigacts_copy<G: ProcGlue>(pr: &Process<G>, parent: &Process<G>) -> SigactsRef {
    let from = parent.sigacts();
    let (m, flags) = {
        let g = parent.ps_mtx.lock();
        (*from.get(&g), from.ps_sigflags.load(Ordering::Relaxed))
    };
    let Some(ps) = SIGACTS_POOL.get(PR_WAITOK, || Sigacts::new_under(&pr.ps_mtx, m, flags)) else {
        kpanic(format_args!("sigactsinit: sigacts_pool is empty"));
    };
    SigactsRef::Pooled(ps)
}

/// `pr->ps_sigacts = sigactsinit(parent)` (`process_new`): `pr` holds a copy of `parent`'s
/// actions from now on.
pub fn sigactsinit<G: ProcGlue>(pr: &Process<G>, parent: &Process<G>) {
    pr.set_sigacts(sigacts_copy(pr, parent));
}

/// `pr->ps_sigacts->ps_sigignore`, read under `pr`'s `ps_mtx` (the C reads it without:
/// `nfs_sigintr`).
pub fn sigignore<G: ProcGlue>(pr: &Process<G>) -> Sigset {
    let g = pr.ps_mtx.lock();
    pr.sigacts().get(&g).ps_sigignore
}

/// `sigaction(2)`'s body: examine and change a signal's action; the old one is copied out to
/// `osa`, the new one copied in from `nsa` (either may be `NULL`).
pub fn sigaction<G: ProcGlue>(
    p: &Thread<G>,
    signum: i32,
    nsa: UserPtr<Sigaction>,
    osa: UserPtr<Sigaction>,
) -> Result<(), Errno> {
    if signum <= 0 || signum >= NSIG || (!nsa.is_null() && (signum == SIGKILL || signum == SIGSTOP))
    {
        return Err(Errno::EINVAL);
    }
    let pr = p.process();
    let ps = pr.sigacts();
    let s = signum as usize;
    if !osa.is_null() {
        let bit = sigmask(signum);
        let g = pr.ps_mtx.lock();
        let m = ps.get(&g);
        let mut sa = Sigaction {
            sa_handler: m.ps_sigact[s],
            sa_mask: m.ps_catchmask[s],
            sa_flags: 0,
        };
        if m.ps_sigonstack & bit != 0 {
            sa.sa_flags |= SA_ONSTACK;
        }
        if m.ps_sigintr & bit == 0 {
            sa.sa_flags |= SA_RESTART;
        }
        if m.ps_sigreset & bit != 0 {
            sa.sa_flags |= SA_RESETHAND;
        }
        if m.ps_siginfo & bit != 0 {
            sa.sa_flags |= SA_SIGINFO;
        }
        if signum == SIGCHLD {
            let sigflags = ps.ps_sigflags.load(Ordering::Relaxed);
            if sigflags & SAS_NOCLDSTOP != 0 {
                sa.sa_flags |= SA_NOCLDSTOP;
            }
            if sigflags & SAS_NOCLDWAIT != 0 {
                sa.sa_flags |= SA_NOCLDWAIT;
            }
        }
        drop(g);
        if sa.sa_mask & bit == 0 {
            sa.sa_flags |= SA_NODEFER;
        }
        sa.sa_mask &= !bit;
        osa.copyout(&sa)?;
        // KTRACE: not configured.
    }
    if !nsa.is_null() {
        let mut sa = nsa.copyin()?;
        // KTRACE: not configured.
        setsigvec(p, signum, &mut sa);
    }
    Ok(())
}

/// `setsigvec`: installs `sa` as the action of `signum` (and adds `signum` to its mask unless
/// `SA_NODEFER`).
pub fn setsigvec<G: ProcGlue>(p: &Thread<G>, signum: i32, sa: &mut Sigaction) {
    let pr = p.process();
    let ps = pr.sigacts();
    let s = signum as usize;
    let bit = sigmask(signum);

    let mut g = pr.ps_mtx.lock();
    if sa.sa_flags & SA_NODEFER == 0 {
        sa.sa_mask |= bit;
    }
    if signum == SIGCHLD {
        if sa.sa_flags & SA_NOCLDSTOP != 0 {
            ps.ps_sigflags.fetch_or(SAS_NOCLDSTOP, Ordering::Relaxed);
        } else {
            ps.ps_sigflags.fetch_and(!SAS_NOCLDSTOP, Ordering::Relaxed);
        }
        // If the SA_NOCLDWAIT flag is set or the handler is SIG_IGN we reparent the dying
        // child to PID 1 (init) which will reap the zombie. Because we use init to do our
        // dirty work we never set SAS_NOCLDWAIT for PID 1.
        // XXX exit1 rework means this is unnecessary?
        let init_is_us = G::initprocess_addr() == ptr::from_ref(pr).addr();
        if !init_is_us && (sa.sa_flags & SA_NOCLDWAIT != 0 || sa.sa_handler == SIG_IGN) {
            ps.ps_sigflags.fetch_or(SAS_NOCLDWAIT, Ordering::Relaxed);
        } else {
            ps.ps_sigflags.fetch_and(!SAS_NOCLDWAIT, Ordering::Relaxed);
        }
    }
    let ignore =
        sa.sa_handler == SIG_IGN || (sigprop(signum) & SA_IGNORE != 0 && sa.sa_handler == SIG_DFL);
    if ignore {
        p.p_siglist.fetch_and(!bit, Ordering::Relaxed);
        pr.ps_siglist.fetch_and(!bit, Ordering::Relaxed);
    }
    let m = ps.get_mut(&mut g);
    let set = |v: &mut Sigset, on: bool| {
        if on {
            *v |= bit;
        } else {
            *v &= !bit;
        }
    };
    m.ps_sigact[s] = sa.sa_handler;
    m.ps_catchmask[s] = sa.sa_mask & !sigcantmask();
    set(&mut m.ps_sigreset, sa.sa_flags & SA_RESETHAND != 0);
    set(&mut m.ps_siginfo, sa.sa_flags & SA_SIGINFO != 0);
    set(&mut m.ps_sigintr, sa.sa_flags & SA_RESTART == 0);
    set(&mut m.ps_sigonstack, sa.sa_flags & SA_ONSTACK != 0);
    // Set bit in ps_sigignore for signals that are set to SIG_IGN, and for signals set to
    // SIG_DFL where the default is to ignore. However, don't put SIGCONT in ps_sigignore, as
    // we have to restart the process.
    if ignore {
        if signum != SIGCONT {
            set(&mut m.ps_sigignore, true); // easier in psignal
        }
        set(&mut m.ps_sigcatch, false);
    } else {
        set(&mut m.ps_sigignore, false);
        set(&mut m.ps_sigcatch, sa.sa_handler != SIG_DFL);
    }
    drop(g);
}

/// `siginit`: initialize signal state for process 0 (`pr`); set to ignore signals that are
/// ignored by default.
pub fn siginit<G: ProcGlue>(pr: &Process<G>) {
    let ps = pr.sigacts();
    let mut g = pr.ps_mtx.lock();
    let m = ps.get_mut(&mut g);
    for i in 0..NSIG {
        if sigprop(i) & SA_IGNORE != 0 && i != SIGCONT {
            m.ps_sigignore |= sigmask(i);
        }
    }
    ps.ps_sigflags
        .store(SAS_NOCLDWAIT | SAS_NOCLDSTOP, Ordering::Relaxed);
    drop(g);
}

/// `execsigs`: reset signals for an exec by the specified thread.
pub fn execsigs<G: ProcGlue>(p: &Thread<G>) {
    let pr = p.process();
    let ps = pr.sigacts();
    let mut g = pr.ps_mtx.lock();
    let m = ps.get_mut(&mut g);

    // Reset caught signals. Held signals remain held through p_sigmask (unless they were
    // caught, and are now ignored by default).
    while m.ps_sigcatch != 0 {
        let nc = ffs(m.ps_sigcatch);
        let mask = sigmask(nc);
        m.ps_sigcatch &= !mask;
        if sigprop(nc) & SA_IGNORE != 0 {
            if nc != SIGCONT {
                m.ps_sigignore |= mask;
            }
            p.p_siglist.fetch_and(!mask, Ordering::Relaxed);
            pr.ps_siglist.fetch_and(!mask, Ordering::Relaxed);
        }
        m.ps_sigact[nc as usize] = SIG_DFL;
    }
    // Reset stack state to the user stack. Clear set of signals caught on the signal stack.
    sigstkinit(&p.p_sigstk);
    m.ps_sigonstack = 0;
    ps.ps_sigflags.fetch_and(!SAS_NOCLDWAIT, Ordering::Relaxed);
    if m.ps_sigact[SIGCHLD as usize] == SIG_IGN {
        m.ps_sigact[SIGCHLD as usize] = SIG_DFL;
    }
    drop(g);
}

/// `sigprocmask(2)`'s body: manipulate signal mask; the old mask is the return value (the
/// library stub does the rest).
pub fn sigprocmask<G: ProcGlue>(p: &Thread<G>, how: i32, mask: Sigset) -> Result<Sigset, Errno> {
    kassert!(is_curproc(p));

    let old = p.p_sigmask.get();
    let mask = mask & !sigcantmask();

    match how {
        SIG_BLOCK => p.p_sigmask.set(old | mask),
        SIG_UNBLOCK => p.p_sigmask.set(old & !mask),
        SIG_SETMASK => p.p_sigmask.set(mask),
        _ => return Err(Errno::EINVAL),
    }
    Ok(old)
}

/// `sigpending(2)`'s body: the signals pending for the thread and its process.
pub fn sigpending_all<G: ProcGlue>(p: &Thread<G>) -> Sigset {
    p.p_siglist.load(Ordering::Relaxed) | p.process().ps_siglist.load(Ordering::Relaxed)
}

/// `dosigsuspend`: temporarily replace calling proc's signal mask for the duration of a
/// system call. Original signal mask will be restored by `userret()`.
pub fn dosigsuspend<G: ProcGlue>(p: &Thread<G>, newmask: Sigset) {
    kassert!(is_curproc(p));

    p.p_oldmask.set(p.p_sigmask.get());
    p.p_sigmask.set(newmask);
    p.p_flag.fetch_or(P_SIGSUSPEND, Ordering::Relaxed);
}

/// `sigsuspend(2)`'s body: suspend thread until signal, providing mask to be set in the
/// meantime; always `EINTR`.
pub fn sigsuspend<G: ProcGlue>(p: &Thread<G>, mask: Sigset) -> Errno {
    dosigsuspend(p, mask & !sigcantmask());
    let nowake = ptr::from_ref(&NOWAKE).addr();
    while sched::tsleep_nsec::<G>(nowake, PPAUSE | PCATCH, "sigsusp", INFSLP).is_ok() {
        continue;
    }
    // always return EINTR rather than ERESTART...
    Errno::EINTR
}

/// Whether the user stack pointer `stack` is on `p`'s alternate signal stack.
fn on_sigstack<G: ProcGlue>(p: &Thread<G>, stack: usize) -> bool {
    let ss = p.p_sigstk.get();
    if ss.ss_flags & SS_DISABLE != 0 {
        false
    } else {
        stack.wrapping_sub(ss.ss_sp) < ss.ss_size
    }
}

/// `sigonstack`: whether the user stack pointer `stack` is on curproc's alternate signal
/// stack.
pub fn sigonstack<G: ProcGlue>(stack: usize) -> bool {
    G::with_curproc(|p| p.is_some_and(|p| on_sigstack(p, stack)))
}

/// `sigaltstack(2)`'s body: set (from `nss`) and/or get (to `oss`) the alternate signal
/// stack.
pub fn sigaltstack<G: ProcGlue>(
    p: &Thread<G>,
    nss: UserPtr<Sigaltstack>,
    oss: UserPtr<Sigaltstack>,
) -> Result<(), Errno> {
    let onstack = on_sigstack(p, G::proc_stack(p));

    if !oss.is_null() {
        let mut ss = p.p_sigstk.get();
        if onstack {
            ss.ss_flags |= SS_ONSTACK;
        }
        oss.copyout(&ss)?;
    }
    if nss.is_null() {
        return Ok(());
    }
    let mut ss = nss.copyin()?;
    ss._pad = 0;
    if onstack {
        return Err(Errno::EPERM);
    }
    if ss.ss_flags & !SS_DISABLE != 0 {
        return Err(Errno::EINVAL);
    }
    if ss.ss_flags & SS_DISABLE != 0 {
        let mut cur = p.p_sigstk.get();
        cur.ss_flags = ss.ss_flags;
        p.p_sigstk.set(cur);
        return Ok(());
    }
    if ss.ss_size < MINSIGSTKSZ {
        return Err(Errno::ENOMEM);
    }

    let Some(vm) = p.vmspace_share() else {
        kpanic(format_args!(
            "sigaltstack: a thread without an address space"
        ));
    };
    crate::vm::map::uvm_map_remap_as_stack(&vm.vm_map, ss.ss_sp, ss.ss_size)?;
    drop(vm);

    p.p_sigstk.set(ss);
    Ok(())
}

/// `kill(2)`'s body, after `pledge_kill`: send `signum` to a process, a process group or
/// every process.
pub fn kill<G: ProcGlue>(cp: &Thread<G>, pid: Pid, signum: i32) -> Result<(), Errno> {
    if (signum as u32) >= NSIG as u32 {
        return Err(Errno::EINVAL);
    }
    if pid > 0 {
        let r = klock::reader();
        let (pr, zombie) = match prfind::<G, _>(&r, pid) {
            Some(pr) => (pr, false),
            None => match zombiefind::<G, _>(&r, pid) {
                Some(pr) => (pr, true),
                None => return Err(Errno::ESRCH),
            },
        };
        if !cansignal(cp, pr, signum) {
            return Err(Errno::EPERM);
        }

        // kill single process
        if signum != 0 && !zombie {
            prsignal(pr, signum);
        }
        return Ok(());
    }
    match pid {
        -1 => killpg1(cp, signum, 0, true),    // broadcast signal
        0 => killpg1(cp, signum, 0, false),    // signal own process group
        _ => killpg1(cp, signum, -pid, false), // negative explicit process group
    }
}

/// `thrkill(2)`'s body: send `signum` to the thread `tid` of the calling process (`cp`
/// itself for 0), which must have the TCB `tcb` unless it is 0.
pub fn thrkill<G: ProcGlue>(
    cp: Pin<&Thread<G>>,
    tid: Pid,
    signum: i32,
    tcb: usize,
) -> Result<(), Errno> {
    if (signum as u32) >= NSIG as u32 {
        return Err(Errno::EINVAL);
    }

    let r = klock::reader();
    let p: Pin<&Thread<G>> = if tid != 0 {
        match tfind_user_pinned::<G, _>(&r, tid, cp.process()) {
            Some(p) => p,
            None => return Err(Errno::ESRCH),
        }
    } else {
        cp
    };

    // optionally require the target thread to have the given tcb addr
    if tcb != 0 && tcb != G::tcb_get(&p) {
        return Err(Errno::ESRCH);
    }

    if signum != 0 {
        ptsignal(p, signum, SignalType::STHREAD);
    }
    Ok(())
}

/// `killpg1`: common code for kill process group/broadcast kill. `cp` is calling process.
pub fn killpg1<G: ProcGlue>(
    cp: &Thread<G>,
    signum: i32,
    pgid: Pid,
    all: bool,
) -> Result<(), Errno> {
    let mut nfound = 0;
    let r = klock::reader();

    if all {
        // broadcast
        for pr in G::tables().allprocess.iter(&r) {
            if pr.ps_pid <= 1
                || pr.ps_flags.load(Ordering::Relaxed) & (PS_SYSTEM | PS_NOBROADCASTKILL) != 0
                || ptr::eq(pr, cp.process())
                || !cansignal(cp, pr, signum)
            {
                continue;
            }
            nfound += 1;
            if signum != 0 {
                prsignal(pr, signum);
            }
        }
    } else {
        let pgrp: &Pgrp<G> = if pgid == 0 {
            // zero pgid means send to my process group.
            match cp.process().pgrp(&r) {
                Some(pg) => pg,
                None => return Err(Errno::ESRCH),
            }
        } else {
            match pgfind::<G, _>(&r, pgid) {
                Some(pg) => pg,
                None => return Err(Errno::ESRCH),
            }
        };
        for pr in pgrp.pg_members.iter(&r) {
            if pr.ps_pid <= 1
                || pr.ps_flags.load(Ordering::Relaxed) & PS_SYSTEM != 0
                || !cansignal(cp, pr, signum)
            {
                continue;
            }
            nfound += 1;
            if signum != 0 {
                prsignal(pr, signum);
            }
        }
    }
    if nfound != 0 {
        Ok(())
    } else {
        Err(Errno::ESRCH)
    }
}

/// `CANDELIVER(uid, euid, pr)`.
pub fn candeliver<G: ProcGlue>(uid: u32, euid: u32, pr: &Process<G>) -> bool {
    let cr = pr.ucred();
    euid == 0
        || uid == cr.cr_ruid.get()
        || uid == cr.cr_svuid.get()
        || uid == cr.cr_uid.get()
        || euid == cr.cr_ruid.get()
        || euid == cr.cr_svuid.get()
        || euid == cr.cr_uid.get()
}

/// `CANSIGIO(cr, pr)`.
pub fn cansigio<G: ProcGlue>(cr: &Ucred, pr: &Process<G>) -> bool {
    candeliver(cr.cr_ruid.get(), cr.cr_uid.get(), pr)
}

/// `pgsignal`: send a signal to a process group. If `checkctty` is set, limit to members
/// which have a controlling terminal.
pub fn pgsignal<G: ProcGlue>(pgrp: Option<&Pgrp<G>>, signum: i32, checkctty: bool) {
    if let Some(pgrp) = pgrp {
        let r = klock::reader();
        for pr in pgrp.pg_members.iter(&r) {
            if !checkctty || pr.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT != 0 {
                prsignal(pr, signum);
            }
        }
    }
}

/// `postsig_done`: recalculate the signal mask and reset the signal disposition after
/// usermode frame for delivery is formed.
pub fn postsig_done<G: ProcGlue>(p: &Thread<G>, signum: i32, catchmask: Sigset, reset: bool) {
    p.p_ru.ru_nsignals.set(p.p_ru.ru_nsignals.get() + 1);
    p.p_sigmask.set(p.p_sigmask.get() | catchmask);
    if reset {
        let mask = sigmask(signum);
        let pr = p.process();
        let ps = pr.sigacts();

        let mut g = pr.ps_mtx.lock();
        let m = ps.get_mut(&mut g);
        m.ps_sigcatch &= !mask;
        if signum != SIGCONT && sigprop(signum) & SA_IGNORE != 0 {
            m.ps_sigignore |= mask;
        }
        m.ps_sigact[signum as usize] = SIG_DFL;
        drop(g);
    }
}

/// `trapsignal`: send a signal caused by a trap to the current thread. If it will be caught
/// immediately, deliver it with correct code. Otherwise, post it normally.
pub fn trapsignal<G: ProcGlue>(
    p: Pin<&Thread<G>>,
    signum: i32,
    trapno: u64,
    code: i32,
    sigval: Sigval,
) {
    let pr = p.process();
    let mut signum = signum;
    let mut ctx = Sigctx::default();

    match signum {
        SIGILL if code == ILL_BTCFI => pr.ps_acflag.set(pr.ps_acflag.get() | ABTCFI),
        SIGILL | SIGBUS | SIGSEGV => pr.ps_acflag.set(pr.ps_acflag.get() | ATRAP),
        _ => {}
    }

    let mut mask = sigmask(signum);
    setsigctx(&p, signum, &mut ctx);
    if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED == 0
        && ctx.sig_catch
        && p.p_sigmask.get() & mask == 0
    {
        let si = initsiginfo(signum, trapno, code, sigval);
        // KTRACE: not configured.
        if G::sendsig(
            &p,
            ctx.sig_action,
            signum,
            p.p_sigmask.get(),
            &si,
            ctx.sig_info,
            ctx.sig_onstack,
        )
        .is_err()
        {
            klock::kernel_lock();
            sigexit(&p, SIGILL);
            // NOTREACHED
        }
        postsig_done(&p, signum, ctx.sig_catchmask, ctx.sig_reset);
    } else {
        p.p_sisig.set(signum);
        p.p_sitrapno.set(trapno); // XXX for core dump/debugger
        p.p_sicode.set(code);
        p.p_sigval.set(sigval);

        // If traced, stop if signal is masked, and stay stopped until released by the
        // debugger. If our parent process is waiting for us, don't hang as we could
        // deadlock.
        if pr.ps_flags.load(Ordering::Relaxed) & (PS_TRACED | PS_PPWAIT) == PS_TRACED
            && signum != SIGKILL
            && p.p_sigmask.get() & mask != 0
        {
            signum = proc_trap(&p, signum);

            // If we are no longer being traced, or the parent didn't give us a signal, skip
            // sending the signal.
            if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED == 0 || signum == 0 {
                return;
            }
            mask = sigmask(signum);
            setsigctx(&p, signum, &mut ctx);

            // update signal info
            p.p_sisig.set(signum);
        }

        // Signals like SIGBUS and SIGSEGV should not, when generated by the kernel, be
        // ignorable or blockable. If it is and we're not being traced, then just kill the
        // process. After vfs_shutdown(9), init(8) cannot receive signals because new code
        // pages of the signal handler cannot be mapped from halted storage. init(8) may not
        // die or the kernel panics. Better loop between signal handler and page fault trap
        // until the machine is halted.
        if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED == 0
            && sigprop(signum) & SA_KILL != 0
            && (p.p_sigmask.get() & mask != 0 || ctx.sig_ignore)
            && pr.ps_pid != 1
        {
            klock::kernel_lock();
            sigexit(&p, signum);
            // NOTREACHED
        }
        ptsignal(p, signum, SignalType::STHREAD);
    }
}

/// `psignal`: send the signal to the process. If the signal has an action, the action is
/// usually performed by the target process rather than the caller; we add the signal to the
/// set of pending signals for the process.
///
/// Exceptions:
/// - When a stop signal is sent to a sleeping process that takes the default action, the
///   process is stopped without awakening it.
/// - SIGCONT restarts stopped processes (or puts them back to sleep) regardless of the
///   signal action (eg, blocked or ignored).
///
/// Other ignored signals are discarded immediately.
pub fn psignal<G: ProcGlue>(p: Pin<&Thread<G>>, signum: i32) {
    ptsignal(p, signum, SignalType::SPROCESS);
}

/// `prsignal`: send the signal to the process `pr` (to its first thread, from where
/// `ptsignal_locked` picks the thread that takes it).
pub fn prsignal<G: ProcGlue>(pr: &Process<G>, signum: i32) {
    let g = pr.ps_mtx.lock();
    // Ignore signal if the target process is exiting
    if pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING != 0 {
        drop(g);
        return;
    }
    if let Some(p) = pr.ps_threads.first_pinned(&g) {
        ptsignal_locked(p, signum, SignalType::SPROCESS, &g);
    }
    drop(g);
}

/// `ptsignal`: `type_` = `SPROCESS`: process signal, can be diverted (sigwait());
/// `STHREAD`: thread signal, but should be propagated if unhandled.
pub fn ptsignal<G: ProcGlue>(p: Pin<&Thread<G>>, signum: i32, type_: SignalType) {
    let pr = p.process();

    let g = pr.ps_mtx.lock();
    ptsignal_locked(p, signum, type_, &g);
    drop(g);
}

/// `ptsignal_locked`: `ptsignal` with the process's `ps_mtx` held (`g`).
pub fn ptsignal_locked<G: ProcGlue>(
    p: Pin<&Thread<G>>,
    signum: i32,
    type_: SignalType,
    g: &PsGuard<'_, G>,
) {
    let pr = p.process();
    let mut p = p;
    let mut altaction = SIG_DFL;
    let mut wakeparent = false;

    kassert!(pr.ps_mtx.owned());

    #[cfg(feature = "diagnostic")]
    if (signum as u32) >= NSIG as u32 || signum == 0 {
        kpanic(format_args!("psignal signal number"));
    }

    // Ignore signal if the target process is exiting
    if pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING != 0 {
        return;
    }

    let mut mask = sigmask(signum);
    let mut sigmask_ = p.p_sigmask.get();

    if type_ == SignalType::SPROCESS {
        // Accept SIGKILL to coredumping processes
        if pr.ps_flags.load(Ordering::Relaxed) & PS_COREDUMP != 0 && signum == SIGKILL {
            pr.ps_siglist.fetch_or(mask, Ordering::Relaxed);
            return;
        }

        // If the current thread can process the signal immediately (it's unblocked) then
        // have it take it: its pinned handle is on its process's thread list.
        let cur = G::with_curproc(|c| {
            c.filter(|q| {
                ptr::eq(q.process(), pr)
                    && q.p_flag.load(Ordering::Relaxed) & P_WEXIT == 0
                    && q.p_sigmask.get() & mask == 0
            })
            .map(|q| ptr::from_ref(q).addr())
        });
        let cur = cur.and_then(|a| {
            pr.ps_threads
                .iter_pinned(g)
                .find(|q| ptr::from_ref::<Thread<G>>(q).addr() == a)
        });
        if let Some(q) = cur {
            p = q;
            sigmask_ = q.p_sigmask.get();
        } else {
            // A process-wide signal can be diverted to a different thread that's in
            // sigwait() for this signal. If there isn't such a thread, then pick a thread
            // that doesn't have it blocked so that the stop/kill consideration isn't
            // delayed. Otherwise, mark it pending on the main thread.
            for q in pr.ps_threads.iter_pinned(g) {
                // ignore exiting threads
                if q.p_flag.load(Ordering::Relaxed) & P_WEXIT != 0 {
                    continue;
                }

                // skip threads that have the signal blocked
                let tmpmask = q.p_sigmask.get();
                if tmpmask & mask != 0 {
                    continue;
                }

                // okay, could send to this thread
                p = q;
                sigmask_ = tmpmask;

                // sigsuspend, sigwait, ppoll/pselect, etc? Definitely go to this thread, as
                // it's already blocked in the kernel.
                if q.p_flag.load(Ordering::Relaxed) & P_SIGSUSPEND != 0 {
                    break;
                }
            }
        }
    }

    knote_process_locked(pr, g, i64::from(NOTE_SIGNAL) | i64::from(signum));

    let mut prop = sigprop(signum);
    let mut action;

    // If proc is traced, always give parent a chance.
    if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        action = SIG_DFL;
    } else {
        // If the signal is being ignored, then we forget about it immediately. (Note: we
        // don't set SIGCONT in ps_sigignore, and if it is set to SIG_IGN, action will be
        // SIG_DFL here.)
        let m = pr.sigacts().get(g);
        let sigignore = m.ps_sigignore;
        let sigcatch = m.ps_sigcatch;

        if sigignore & mask != 0 {
            return;
        }
        if sigmask_ & mask != 0 {
            action = SIG_HOLD;
            if sigcatch & mask != 0 {
                altaction = SIG_CATCH;
            }
        } else if sigcatch & mask != 0 {
            action = SIG_CATCH;
        } else {
            action = SIG_DFL;

            if prop & SA_KILL != 0 && u32::from(pr.ps_nice.get()) > NZERO {
                pr.ps_nice.set(NZERO as u8);
            }

            // If sending a tty stop signal to a member of an orphaned process group,
            // discard the signal here if the action is default; don't stop the process
            // below if sleeping, and don't clear any pending SIGCONT.
            if prop & SA_TTYSTOP != 0 && pgrp_orphaned(pr, g) {
                return;
            }
        }
    }
    // If delivered to process, mark as pending there. Continue and stop signals are always
    // marked at process level.
    let siglist: &AtomicU32 = if type_ == SignalType::SPROCESS || prop & (SA_CONT | SA_STOP) != 0 {
        &pr.ps_siglist
    } else {
        &p.p_siglist
    };

    let mut h = sched_lock();

    'out: {
        match p.stat() {
            SSTOP => {
                // If traced process is already stopped, then no further action is
                // necessary.
                if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
                    break 'out;
                }

                // Kill signal always sets processes running.
                if signum == SIGKILL {
                    p.p_flag.fetch_and(!P_SUSPSIG, Ordering::Relaxed);
                    // Raise priority to at least PUSER.
                    if u32::from(p.p_usrpri.get()) > PUSER {
                        p.p_usrpri.set(PUSER as u8);
                    }
                    sched::unsleep(&mut h, &p);
                    sched::setrunnable(&mut h, p);
                    break 'out;
                }

                if prop & SA_CONT != 0 {
                    // If SIGCONT is default (or ignored), we continue the process but don't
                    // leave the signal in p_siglist, as it has no further action. If SIGCONT
                    // is held, we continue the process and leave the signal in p_siglist. If
                    // the process catches SIGCONT, let it handle the signal itself. At the
                    // end continue the process.
                    pr.ps_flags.fetch_or(PS_CONTINUED, Ordering::Relaxed);
                    pr.ps_flags.fetch_and(
                        !(PS_WAITED | PS_STOPPED | PS_STOPPING | PS_TRAPPED),
                        Ordering::Relaxed,
                    );
                    if action == SIG_DFL {
                        mask = 0;
                    }
                    if action == SIG_CATCH {
                        // Raise priority to at least PUSER.
                        if u32::from(p.p_usrpri.get()) > PUSER {
                            p.p_usrpri.set(PUSER as u8);
                        }
                        sched::unsleep(&mut h, &p);
                    }

                    process_continue(pr, P_SUSPSIG, g, &mut h);
                    wakeparent = true;
                    break 'out;
                }

                // Defer further processing for signals which are held, except that stopped
                // processes must be continued by SIGCONT.
                if action == SIG_HOLD {
                    break 'out;
                }

                if prop & SA_STOP != 0 {
                    // Already stopped, don't need to stop again. (If we did the shell could
                    // get confused.)
                    mask = 0;
                    break 'out;
                }

                // If process is sleeping interruptibly, then simulate a wakeup so that when
                // it is continued, it will be made runnable and can look at the signal. But
                // don't make the process runnable, leave it stopped.
                if p.p_flag.load(Ordering::Relaxed) & P_SINTR != 0 {
                    sched::unsleep(&mut h, &p);
                }
            }

            SSLEEP => {
                // If process is sleeping uninterruptibly we can't interrupt the sleep... the
                // signal will be noticed when the process returns through trap() or
                // syscall().
                if p.p_flag.load(Ordering::Relaxed) & P_SINTR == 0 {
                    break 'out;
                }
                // Process is sleeping and traced... make it runnable so it can discover the
                // signal in cursig() and stop for the parent.
                if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
                    sched::unsleep(&mut h, &p);
                    sched::setrunnable(&mut h, p);
                    break 'out;
                }

                // Recheck sigmask before waking up the process, there is a chance that
                // while sending the signal the process changed sigmask and went to sleep.
                sigmask_ = p.p_sigmask.get();
                if sigmask_ & mask != 0 {
                    break 'out;
                } else if action == SIG_HOLD {
                    // signal got unmasked, get proper action
                    action = altaction;

                    if action == SIG_DFL {
                        if prop & SA_KILL != 0 && u32::from(pr.ps_nice.get()) > NZERO {
                            pr.ps_nice.set(NZERO as u8);
                        }

                        // Discard tty stop signals sent to an orphaned process group, see
                        // above.
                        if prop & SA_TTYSTOP != 0 && pgrp_orphaned(pr, g) {
                            mask = 0;
                            prop = 0;
                            break 'out;
                        }
                    }
                }

                // If SIGCONT is default (or ignored) and process is asleep, we are finished;
                // the process should not be awakened.
                if prop & SA_CONT != 0 && action == SIG_DFL {
                    mask = 0;
                    break 'out;
                }
                // When a sleeping process receives a stop signal, process immediately if
                // possible.
                if prop & SA_STOP != 0 && action == SIG_DFL {
                    // If a child holding parent blocked, stopping could cause deadlock.
                    if pr.ps_flags.load(Ordering::Relaxed) & PS_PPWAIT != 0 {
                        break 'out;
                    }
                    mask = 0;
                    pr.ps_xsig.set(signum);
                    pr.ps_flags.fetch_or(PS_STOPPING, Ordering::Relaxed);
                    process_stop(pr, P_SUSPSIG, SINGLE_SUSPEND, g, &mut h);
                    wakeparent = true;
                    break 'out;
                }
                // All other (caught or default) signals cause the process to run. Raise
                // priority to at least PUSER.
                if u32::from(p.p_usrpri.get()) > PUSER {
                    p.p_usrpri.set(PUSER as u8);
                }
                sched::unsleep(&mut h, &p);
                sched::setrunnable(&mut h, p);
            }

            SONPROC => {
                if action == SIG_HOLD {
                    break 'out;
                }

                // set siglist before issuing the ast
                siglist.fetch_or(mask, Ordering::Relaxed);
                mask = 0;
                G::signotify(&p);
                // FALLTHROUGH to the default: nothing more.
            }

            // SRUN, SIDL, SDEAD do nothing with the signal, other than kicking ourselves if
            // we are running. It will either never be noticed, or noticed very soon.
            _ => {}
        }
    }

    // out: finally adjust siglist
    if mask != 0 {
        siglist.fetch_or(mask, Ordering::Relaxed);
    }
    if prop & SA_CONT != 0 {
        siglist.fetch_and(!STOPSIGMASK, Ordering::Relaxed);
    }
    if prop & SA_STOP != 0 {
        siglist.fetch_and(!CONTSIGMASK, Ordering::Relaxed);
        pr.ps_flags.fetch_and(!PS_CONTINUED, Ordering::Relaxed);
    }

    drop(h);
    if wakeparent {
        if prop & SA_STOP != 0 {
            process_suspend_signal(pr, g);
        } else {
            let pptr = parent_locked(pr, g);
            pptr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
            wakeup_of::<G, _>(pptr);
        }
    }
}

/// `setsigctx`: fill the signal context which should be used by `postsig()` and
/// `issignal()`.
pub fn setsigctx<G: ProcGlue>(p: &Thread<G>, signum: i32, sctx: &mut Sigctx) {
    let pr = p.process();
    let s = signum as usize;

    let g = pr.ps_mtx.lock();
    let m = pr.sigacts().get(&g);
    let mask = sigmask(signum);
    sctx.sig_action = m.ps_sigact[s];
    sctx.sig_catchmask = m.ps_catchmask[s];
    sctx.sig_reset = m.ps_sigreset & mask != 0;
    sctx.sig_info = m.ps_siginfo & mask != 0;
    sctx.sig_intr = m.ps_sigintr & mask != 0;
    sctx.sig_onstack = m.ps_sigonstack & mask != 0;
    sctx.sig_ignore = m.ps_sigignore & mask != 0;
    sctx.sig_catch = m.ps_sigcatch & mask != 0;
    sctx.sig_stop = sigprop(signum) & SA_STOP != 0 && sctx.sig_action == SIG_DFL;
    if sctx.sig_stop {
        // If the process is a member of an orphaned process group, ignore tty stop signals.
        if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0
            || (pgrp_orphaned(pr, &g) && sigprop(signum) & SA_TTYSTOP != 0)
        {
            sctx.sig_stop = false;
            sctx.sig_ignore = true;
        }
    }
    drop(g);
}

/// `cursig`: determine signal that should be delivered to process `p`, the current process,
/// 0 if none.
///
/// If the current process has received a signal (should be caught or cause termination,
/// should interrupt current syscall), return the signal number. Stop signals with default
/// action are processed immediately, then cleared; they aren't returned. This is checked
/// after each entry to the system for a syscall or trap. The normal call sequence is
///
/// ```text
/// while (signum = cursig(curproc, &ctx, 0))
///     postsig(signum, &ctx);
/// ```
///
/// Assumes that if the `P_SINTR` flag is set, we're holding both the kernel and scheduler
/// locks.
pub fn cursig<G: ProcGlue>(p: &Thread<G>, sctx: &mut Sigctx, deep: bool) -> i32 {
    let pr = p.process();
    let mut keep: Sigset = 0;
    let mut signum;
    let mut mask;

    kassert!(is_curproc(p));

    loop {
        let ps_siglist = pr.ps_siglist.load(Ordering::Relaxed);
        fence(Ordering::Acquire); // membar_consumer()
        mask = sigpending(p);
        if pr.ps_flags.load(Ordering::Relaxed) & PS_PPWAIT != 0 {
            mask &= !STOPSIGMASK;
        }
        signum = ffs(mask);
        if signum == 0 {
            // no signal to send
            break; // goto keep
        }
        mask = sigmask(signum);

        // take the signal!
        if pr
            .ps_siglist
            .compare_exchange(
                ps_siglist,
                ps_siglist & !mask,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_err()
        {
            // lost race taking the process signal, restart
            continue;
        }
        p.p_siglist.fetch_and(!mask, Ordering::Relaxed);
        setsigctx(p, signum, sctx);

        // We should see pending but ignored signals only if PS_TRACED was on when they were
        // posted.
        if sctx.sig_ignore && pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED == 0 {
            continue;
        }

        // If cursig is called while going to sleep, abort now and stop the sleep. When the
        // call unwinded to userret cursig is called again and there the signal can be
        // handled cleanly.
        if deep {
            // Do not stop the thread here if multiple signals are pending and at least one
            // of them would force an unwind.
            //
            // ffs() favors low numbered signals and so stop signals may be picked up before
            // other pending signals.
            if sctx.sig_stop && sigpending(p) != 0 {
                keep |= mask;
                continue;
            }
            break; // goto keep
        }

        // If traced, always stop, and stay stopped until released by the debugger. If our
        // parent process is waiting for us, don't hang as we could deadlock.
        if pr.ps_flags.load(Ordering::Relaxed) & (PS_TRACED | PS_PPWAIT) == PS_TRACED
            && signum != SIGKILL
        {
            signum = proc_trap(p, signum);

            mask = sigmask(signum);
            setsigctx(p, signum, sctx);

            // If we are no longer being traced, or the parent didn't give us a signal, or
            // the signal is ignored, look for more signals.
            if pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED == 0
                || signum == 0
                || sctx.sig_ignore
            {
                continue;
            }

            // If the new signal is being masked, look for other signals but leave it for
            // later.
            if p.p_sigmask.get() & mask != 0 {
                p.p_siglist.fetch_or(mask, Ordering::Relaxed);
                continue;
            }
        }

        let prop = sigprop(signum);

        // Decide whether the signal should be returned. Return the signal's number, or fall
        // through to clear it from the pending mask.
        match sctx.sig_action {
            SIG_DFL => {
                // Don't take default actions on system processes.
                if pr.ps_pid <= 1 {
                    // Are you sure you want to ignore SIGSEGV in init? XXX
                    #[cfg(feature = "diagnostic")]
                    crate::prf::printf(format_args!(
                        "Process (pid {}) got signal {}\n",
                        pr.ps_pid, signum
                    ));
                    continue; // == ignore
                }
                // If there is a pending stop signal to process with default action, stop
                // here, then clear the signal.
                if sctx.sig_stop {
                    let g = pr.ps_mtx.lock();
                    pr.ps_xsig.set(signum);
                    pr.ps_flags.fetch_or(PS_STOPPING, Ordering::Relaxed);
                    let mut h = sched_lock();
                    process_stop(pr, P_SUSPSIG, SINGLE_SUSPEND, &g, &mut h);
                    p.p_flag.fetch_or(P_SUSPSIG, Ordering::Relaxed);
                    proc_stop_setup(p);
                    drop(h);
                    process_suspend_signal(pr, &g);
                    let g = proc_stop_finish(pr, p, g);
                    drop(g);
                    continue;
                } else if prop & SA_IGNORE != 0 {
                    // Except for SIGCONT, shouldn't get here. Default action is to ignore;
                    // drop it.
                    continue; // == ignore
                } else {
                    break; // goto keep
                }
            }
            SIG_IGN => {
                // Masking above should prevent us ever trying to take action on an ignored
                // signal other than SIGCONT, unless process is traced.
                if prop & SA_CONT == 0 && pr.ps_flags.load(Ordering::Relaxed) & PS_TRACED == 0 {
                    crate::prf::printf(format_args!("cursig\n"));
                }
                continue; // == ignore
            }
            _ => {
                // This signal has an action, let postsig() process it.
                break; // goto keep
            }
        }
    }

    // keep: if we stashed a stop signal but no other signal is pending anymore pick the stop
    // signal up again.
    if keep != 0 && signum == 0 {
        signum = ffs(keep);
        setsigctx(p, signum, sctx);
    }
    // move the signal to p_siglist for later
    p.p_siglist.fetch_or(mask | keep, Ordering::Relaxed);
    signum
}

/// `proc_trap`: stop the traced thread `p` for its debugger with `signum`; returns the
/// signal the debugger hands back (0 for none).
pub fn proc_trap<G: ProcGlue>(p: &Thread<G>, signum: i32) -> i32 {
    let pr = p.process();

    let g = pr.ps_mtx.lock();
    // Wait until any other suspend condition cleared, including other traps.
    let (g, _) = proc_suspend_check_locked(pr, p, g, false);

    pr.ps_flags
        .fetch_or(PS_STOPPING | PS_TRAPPED, Ordering::Relaxed);
    let mut h = sched_lock();
    process_stop(pr, P_SUSPSIG, SINGLE_SUSPEND, &g, &mut h);
    p.p_flag.fetch_or(P_SUSPSIG, Ordering::Relaxed);
    proc_stop_setup(p);
    drop(h);
    pr.ps_xsig.set(signum);
    pr.ps_trapped.set(ptr::from_ref(p));

    process_suspend_signal(pr, &g);
    let g = proc_stop_finish(pr, p, g);
    // Clear all flags for proc and process by hand here since ptrace just calls setrunnable
    // on the thread without clearing anything.
    p.p_flag.fetch_and(!P_SUSPSIG, Ordering::Relaxed);
    pr.ps_flags
        .fetch_and(!(PS_WAITED | PS_STOPPED | PS_TRAPPED), Ordering::Relaxed);

    let signum = pr.ps_xsig.get();
    pr.ps_xsig.set(0);
    pr.ps_trapped.set(ptr::null());

    if p.p_flag.load(Ordering::Relaxed) & P_TRACESINGLE == 0 {
        let mut h = sched_lock();
        process_continue(pr, P_SUSPSIG, &g, &mut h);
        drop(h);
    }
    p.p_flag.fetch_and(!P_TRACESINGLE, Ordering::Relaxed);
    drop(g);

    signum
}

/// `process_continue`: continue all threads of a process that were stopped because of
/// `flag`, under its `ps_mtx` (`g`) and `SCHED_LOCK` (`h`, the caller's).
pub fn process_continue<G: ProcGlue>(
    pr: &Process<G>,
    flag: i32,
    g: &PsGuard<'_, G>,
    h: &mut SchedHold,
) {
    kassert!(pr.ps_mtx.owned());

    // skip curproc if it is part of pr
    let me = curproc_in(pr);

    for q in pr.ps_threads.iter_pinned(g) {
        if me == Some(ptr::from_ref::<Thread<G>>(&q).addr()) {
            continue;
        }
        if q.p_flag.load(Ordering::Relaxed) & flag == 0 {
            continue;
        }
        q.p_flag.fetch_and(!flag, Ordering::Relaxed);

        // XXX in ptsignal the SCHED_LOCK is already held so we can't grab it here until
        // that is fixed: the caller's hold `h`.
        // Stopping a process is not an atomic operation so it is possible that some
        // threads are not stopped when process_continue is called. These threads need to
        // be skipped.
        //
        // Clearing either makes the thread runnable or puts it back into some sleep queue.
        if q.stat() == SSTOP && q.p_flag.load(Ordering::Relaxed) & (P_SUSPSIG | P_SUSPSINGLE) == 0 {
            if q.p_wchan.get() == 0 {
                sched::setrunnable(h, q);
            } else {
                q.set_stat(SSLEEP);
            }
        }
    }
}

/// `process_stop`: signal all but `p` threads of a process `pr` to stop because of `flag`,
/// under its `ps_mtx` (`g`) and `SCHED_LOCK` (`h`). Depending on `mode` stopped and sleeping
/// threads may be woken up.
pub fn process_stop<G: ProcGlue>(
    pr: &Process<G>,
    flag: i32,
    mode: i32,
    g: &PsGuard<'_, G>,
    h: &mut SchedHold,
) {
    kassert!(pr.ps_mtx.owned());

    // skip curproc if it is part of pr, caller takes care of that
    let me = curproc_in(pr);
    #[cfg(feature = "diagnostic")]
    if let Some(me) = me
        && let Some(c) = pr
            .ps_threads
            .iter(g)
            .find(|q| ptr::from_ref::<Thread<G>>(q).addr() == me)
    {
        kassert!(c.p_flag.load(Ordering::Relaxed) & (P_SUSPSIG | P_SUSPSINGLE) == 0);
    }

    pr.ps_suspendcnt.set(pr.ps_threadcnt.get());
    for q in pr.ps_threads.iter_pinned(g) {
        if me == Some(ptr::from_ref::<Thread<G>>(&q).addr()) {
            continue;
        }
        q.p_flag.fetch_or(flag, Ordering::Relaxed);

        // XXX in ptsignal the SCHED_LOCK is already held so we can't grab it here until
        // that is fixed: the caller's hold `h`.

        match q.stat() {
            SSTOP => {
                if mode == SINGLE_EXIT {
                    sched::unsleep(h, &q);
                    sched::setrunnable(h, q);
                } else {
                    pr.ps_suspendcnt.set(pr.ps_suspendcnt.get().wrapping_sub(1));
                }
            }
            SSLEEP => {
                // if it's not interruptible, then just have to wait
                if q.p_flag.load(Ordering::Relaxed) & P_SINTR != 0 {
                    // merely need to suspend? just stop it
                    if mode == SINGLE_SUSPEND {
                        q.set_stat(SSTOP);
                        pr.ps_suspendcnt.set(pr.ps_suspendcnt.get().wrapping_sub(1));
                    } else {
                        // need to unwind or exit, so wake it
                        sched::unsleep(h, &q);
                        sched::setrunnable(h, q);
                    }
                }
            }
            SONPROC => G::signotify(&q),
            SRUN | SIDL | SDEAD => {}
            _ => {}
        }
    }
}

/// `proc_stop_setup`: prepare a proc to be stopped.
pub fn proc_stop_setup<G: ProcGlue>(p: &Thread<G>) {
    kassert!(p.process().ps_mtx.owned());
    // XXX in ptsignal the SCHED_LOCK is already held so we can't grab it here until that is
    // fixed.
    kassert!(hal::thread::sched_locked());

    // TRACEPOINT(sched, stop, NULL): dt(4), not configured.

    p.p_flag.fetch_or(P_INSCHED, Ordering::Relaxed);
    p.set_stat(SSTOP);
}

/// `proc_stop_finish`: finish stopping a process if the condition still holds: `ps_mtx`
/// (`g`) is left around the switch and the guard of its reentry returned.
pub fn proc_stop_finish<'a, G: ProcGlue>(
    pr: &'a Process<G>,
    p: &Thread<G>,
    g: PsGuard<'a, G>,
) -> PsGuard<'a, G> {
    kassert!(pr.ps_mtx.owned());
    drop(g);
    let h = sched_lock();

    p.p_flag.fetch_and(!P_INSCHED, Ordering::Relaxed);
    if p.stat() == SSTOP {
        p.p_ru.ru_nvcsw.set(p.p_ru.ru_nvcsw.get() + 1);
        hal::thread::with_current::<Thread<G>, _>(|cp| sched::mi_switch(cp, h));
    } else {
        kassert!(p.stat() == SONPROC);
        drop(h);
    }
    pr.ps_mtx.lock()
}

/// `process_suspend_signal`: signal either the parent process or the `ps_single` thread
/// depending on the mode. Only do this if the suspendcnt dropped to 0. If curproc part of
/// the process count it out first. Under `pr`'s `ps_mtx` (`g`).
pub fn process_suspend_signal<G: ProcGlue>(pr: &Process<G>, g: &PsGuard<'_, G>) {
    kassert!(pr.ps_mtx.owned());

    // if part of the process, count us out
    if curproc_in(pr).is_some() {
        pr.ps_suspendcnt.set(pr.ps_suspendcnt.get().wrapping_sub(1));
    }

    if pr.ps_suspendcnt.get() != 0 {
        return;
    }

    if pr.ps_single.get().is_null() {
        pr.ps_flags
            .fetch_and(!(PS_STOPPING | PS_WAITED | PS_CONTINUED), Ordering::Relaxed);
        pr.ps_flags.fetch_or(PS_STOPPED, Ordering::Relaxed);

        let pptr = parent_locked(pr, g);
        if pptr.sigacts().ps_sigflags.load(Ordering::Relaxed) & SAS_NOCLDSTOP == 0 {
            prsignal(pptr, SIGCHLD);
        }
        pptr.ps_flags.fetch_or(PS_WAITEVENT, Ordering::Relaxed);
        wakeup_of::<G, _>(pptr);
    } else {
        wakeup_of::<G, _>(&pr.ps_suspendcnt);
    }
}

/// `postsig`: take the action for the specified signal from the current set of pending
/// signals.
pub fn postsig<G: ProcGlue>(p: &Thread<G>, signum: i32, sctx: &Sigctx) {
    kassert!(signum != 0);

    let mask = sigmask(signum);
    p.p_siglist.fetch_and(!mask, Ordering::Relaxed);

    let (trapno, code, sigval) = if p.p_sisig.get() != signum {
        (0, SI_USER, Sigval::from_ptr(0))
    } else {
        (p.p_sitrapno.get(), p.p_sicode.get(), p.p_sigval.get())
    };
    let si = initsiginfo(signum, trapno, code, sigval);

    // KTRACE: not configured.
    if sctx.sig_action == SIG_DFL {
        // Default action, where the default is to kill the process. (Other cases were
        // ignored above.)
        klock::kernel_lock();
        sigexit(p, signum);
        // NOTREACHED
    } else {
        // If we get here, the signal must be caught.
        #[cfg(feature = "diagnostic")]
        if sctx.sig_action == SIG_IGN || p.p_sigmask.get() & mask != 0 {
            kpanic(format_args!("postsig action"));
        }
        // Set the new mask value and also defer further occurrences of this signal.
        //
        // Special case: user has done a sigpause. Here the current mask is not of interest,
        // but rather the mask from before the sigpause is what we want restored after the
        // signal processing is completed.
        let returnmask = if p.p_flag.load(Ordering::Relaxed) & P_SIGSUSPEND != 0 {
            p.p_flag.fetch_and(!P_SIGSUSPEND, Ordering::Relaxed);
            p.p_oldmask.get()
        } else {
            p.p_sigmask.get()
        };
        if p.p_sisig.get() == signum {
            p.p_sisig.set(0);
            p.p_sitrapno.set(0);
            p.p_sicode.set(SI_USER);
            p.p_sigval.set(Sigval::from_ptr(0));
        }

        if G::sendsig(
            p,
            sctx.sig_action,
            signum,
            returnmask,
            &si,
            sctx.sig_info,
            sctx.sig_onstack,
        )
        .is_err()
        {
            klock::kernel_lock();
            sigexit(p, SIGILL);
            // NOTREACHED
        }
        postsig_done(p, signum, sctx.sig_catchmask, sctx.sig_reset);
    }
}

/// `sigexit`: force the current process to exit with the specified signal, dumping core if
/// appropriate. We bypass the normal tests for masked and caught signals, allowing
/// unrecoverable failures to terminate the process without changing signal state. Mark the
/// accounting record with the signal termination. If dumping core, save the signal number
/// for the debugger. Calls exit and does not return.
pub fn sigexit<G: ProcGlue>(p: &Thread<G>, signum: i32) -> ! {
    let mut signum = signum;

    // Mark process as going away
    p.p_flag.fetch_or(P_WEXIT, Ordering::Relaxed);

    let pr = p.process();
    pr.ps_acflag.set(pr.ps_acflag.get() | AXSIG);
    if sigprop(signum) & SA_CORE != 0 {
        p.p_sisig.set(signum);

        // if there are other threads, pause them
        if p.has_sibling() {
            let _ = single_thread_set(p, SINGLE_UNWIND);
        }

        if G::coredump(p).is_ok() {
            signum |= WCOREFLAG;
        }
    }
    G::exit1(p, 0, signum, EXIT_NORMAL)
    // NOTREACHED
}

/// `sigabort`: send uncatchable SIGABRT for coredump.
pub fn sigabort<G: ProcGlue>(p: Pin<&Thread<G>>) {
    kassert!(is_curproc(&p) || hal::cpu::past_caring());

    let mut sa = Sigaction {
        sa_handler: SIG_DFL,
        ..Sigaction::default()
    };
    setsigvec(&p, SIGABRT, &mut sa);
    p.p_sigmask.set(p.p_sigmask.get() & !sigmask(SIGABRT));
    psignal(p, SIGABRT);
}

/// `sigismasked`: return `true` if `sig`, a given signal, is ignored or masked for `p`, a
/// given thread, and `false` otherwise.
pub fn sigismasked<G: ProcGlue>(p: &Thread<G>, sig: i32) -> bool {
    let pr = p.process();

    kassert!(is_curproc(p));

    let g = pr.ps_mtx.lock();
    let rv = pr.sigacts().get(&g).ps_sigignore & sigmask(sig) != 0
        || p.p_sigmask.get() & sigmask(sig) != 0;
    drop(g);

    rv
}

/// `sys_nosys`'s body: nonexistent system call-- signal process (may want to handle it).
/// Flag error in case process won't see signal immediately (blocked or ignored).
pub fn nosys<G: ProcGlue>(p: Pin<&Thread<G>>) -> Errno {
    ptsignal(p, SIGSYS, SignalType::STHREAD);
    Errno::ENOSYS
}

/// `__thrsigdivert(2)`'s body (`sigwait(3)`): wait for one of the signals in `sigmask`,
/// which are taken out of the normal delivery, at most `timeout` (if not `NULL`); the
/// signal's number, its `siginfo_t` copied out to `info` (if not `NULL`).
pub fn thrsigdivert<G: ProcGlue>(
    p: &Thread<G>,
    sigmask_: Sigset,
    info: UserPtr<Siginfo>,
    timeout: UserPtr<UserTimespec>,
) -> Result<i32, Errno> {
    let mut ctx = Sigctx::default();
    let mask = sigmask_ & !sigcantmask();
    let mut si = Siginfo::zeroed();
    let mut nsecs = INFSLP;
    let mut timeinvalid = false;
    let mut error: Result<(), Errno> = Ok(());
    let nowake = ptr::from_ref(&NOWAKE).addr();

    if !timeout.is_null() {
        let ts = timeout.copyin()?;
        // KTRACE: not configured.
        if !ts.is_valid() {
            timeinvalid = true;
        } else {
            nsecs = ts.to_nsec();
        }
    }

    dosigsuspend(p, p.p_sigmask.get() & !mask);
    loop {
        si.si_signo = cursig(p, &mut ctx, false);
        if si.si_signo != 0 {
            let smask = sigmask(si.si_signo);
            if smask & mask != 0 {
                p.p_siglist.fetch_and(!smask, Ordering::Relaxed);
                error = Ok(());
                break;
            }
        }

        // per-POSIX, delay this error until after the above
        if timeinvalid {
            error = Err(Errno::EINVAL);
        }
        // per-POSIX, return immediately if timeout is zero-valued
        if nsecs == 0 {
            error = Err(Errno::EAGAIN);
        }

        if error.is_err() {
            break;
        }

        error = sched::tsleep_nsec::<G>(nowake, PPAUSE | PCATCH, "sigwait", nsecs);
    }

    match error {
        Ok(()) => {
            if !info.is_null() {
                info.copyout(&si)?;
                // KTRACE: not configured.
            }
            Ok(si.si_signo)
        }
        // Restarting is wrong if there's a timeout, as it'll be for the same interval again
        Err(Errno::ERESTART) if !timeout.is_null() => Err(Errno::EINTR),
        Err(e) => Err(e),
    }
}

/// `initsiginfo`: the `siginfo_t` of signal `sig` with `code`; `trapno` and `val` fill the
/// fault members for a kernel-generated `SIGSEGV`/`SIGILL`/`SIGBUS`/`SIGFPE`, `val` the
/// value of a user-generated one.
pub fn initsiginfo(sig: i32, trapno: u64, code: i32, val: Sigval) -> Siginfo {
    let mut si = Siginfo::zeroed();

    si.si_signo = sig;
    si.si_code = code;
    if code == SI_USER {
        si.set_si_value(val);
    } else {
        match sig {
            SIGSEGV | SIGILL | SIGBUS | SIGFPE => {
                si.set_si_addr(val.sival_ptr());
                si.set_si_trapno(trapno as i32);
            }
            SIGXFSZ => {}
            _ => {}
        }
    }
    si
}

/// `userret`: the last thing before a thread returns to user mode: pending suspensions and
/// signals, then the CPU's current priority.
pub fn userret<G: ProcGlue>(p: Pin<&Thread<G>>) {
    let mut ctx = Sigctx::default();

    if p.p_flag.load(Ordering::Relaxed) & (P_SUSPSINGLE | P_SUSPSIG) != 0 {
        let _ = proc_suspend_check(&p, false);
    }

    // send SIGPROF or SIGVTALRM if their timers interrupted this thread
    if p.p_flag.load(Ordering::Relaxed) & P_PROFPEND != 0 {
        p.p_flag.fetch_and(!P_PROFPEND, Ordering::Relaxed);
        psignal(p, SIGPROF);
    }
    if p.p_flag.load(Ordering::Relaxed) & P_ALRMPEND != 0 {
        p.p_flag.fetch_and(!P_ALRMPEND, Ordering::Relaxed);
        psignal(p, SIGVTALRM);
    }

    if sigpending(&p) != 0 {
        loop {
            let signum = cursig(&p, &mut ctx, false);
            if signum == 0 {
                break;
            }
            postsig(&p, signum, &ctx);
        }
    }

    // If P_SIGSUSPEND is still set here, then we still need to restore the original sigmask
    // before returning to userspace. Also, this might unmask some pending signals, so we
    // need to check a second time for signals to post.
    if p.p_flag.load(Ordering::Relaxed) & P_SIGSUSPEND != 0 {
        p.p_sigmask.set(p.p_oldmask.get());
        p.p_flag.fetch_and(!P_SIGSUSPEND, Ordering::Relaxed);

        loop {
            let signum = cursig(&p, &mut ctx, false);
            if signum == 0 {
                break;
            }
            postsig(&p, signum, &ctx);
        }
    }

    // WITNESS_WARN(WARN_PANIC, NULL, "userret: returning"): not configured.

    sched::set_curpriority(&p);
}

/// `proc_suspend_check_locked`: suspend (or, `deep`, report how to unwind) the thread `p`
/// while its process `pr` is being single-threaded or stopped; under `pr`'s `ps_mtx` (`g`),
/// which a suspension leaves and takes again: its guard is given back.
pub fn proc_suspend_check_locked<'a, G: ProcGlue>(
    pr: &'a Process<G>,
    p: &Thread<G>,
    g: PsGuard<'a, G>,
    deep: bool,
) -> (PsGuard<'a, G>, Result<(), Errno>) {
    let mut g = g;

    kassert!(pr.ps_mtx.owned());

    let single = pr.ps_single.get();
    if (single.is_null() || ptr::eq(single, p))
        && pr.ps_flags.load(Ordering::Relaxed) & PS_STOPPING == 0
    {
        return (g, Ok(()));
    }

    // if we're in deep, we need to unwind to the edge
    if deep {
        let mut err = Ok(());

        if pr.ps_flags.load(Ordering::Relaxed) & (PS_SINGLEUNWIND | PS_SINGLEEXIT) != 0 {
            return (g, Err(Errno::ERESTART));
        }
        let h = sched_lock();
        if p.stat() != SSTOP {
            err = Err(Errno::EWOULDBLOCK);
        }
        drop(h);
        return (g, err);
    }

    loop {
        if pr.ps_flags.load(Ordering::Relaxed) & PS_SINGLEEXIT != 0 {
            drop(g);
            klock::kernel_lock();
            G::exit1(p, 0, 0, EXIT_THREAD_NOCHECK);
            // NOTREACHED
        }

        let h = sched_lock();
        proc_stop_setup(p);
        drop(h);
        process_suspend_signal(pr, &g);

        // not exiting and don't need to unwind, so suspend
        g = proc_stop_finish(pr, p, g);

        if pr.ps_single.get().is_null() && pr.ps_flags.load(Ordering::Relaxed) & PS_STOPPING == 0 {
            break;
        }
    }

    (g, Ok(()))
}

/// `proc_suspend_check`: `proc_suspend_check_locked` under the process's `ps_mtx`.
pub fn proc_suspend_check<G: ProcGlue>(p: &Thread<G>, deep: bool) -> Result<(), Errno> {
    let pr = p.process();

    let g = pr.ps_mtx.lock();
    let (g, error) = proc_suspend_check_locked(pr, p, g, deep);
    drop(g);

    error
}

/// `single_thread_set`: stop other threads in the process. The mode controls how and where
/// the other threads should stop:
/// - `SINGLE_SUSPEND`: stop wherever they are, will later be released (via
///   `single_thread_clear()`)
/// - `SINGLE_UNWIND`: just unwind to kernel boundary, will be told to exit (by setting to
///   `SINGLE_EXIT`) or released as with `SINGLE_SUSPEND`
/// - `SINGLE_EXIT`: unwind to kernel boundary and exit
pub fn single_thread_set<G: ProcGlue>(p: &Thread<G>, flags: i32) -> Result<(), Errno> {
    let pr = p.process();
    let mode = flags & SINGLE_MASK;

    kassert!(is_curproc(p));

    let g = pr.ps_mtx.lock();
    let (mut g, checked) = proc_suspend_check_locked(pr, p, g, flags & SINGLE_DEEP != 0);
    if let Err(error) = checked {
        drop(g);
        return Err(error);
    }

    match mode {
        SINGLE_SUSPEND => {}
        SINGLE_UNWIND => {
            pr.ps_flags.fetch_or(PS_SINGLEUNWIND, Ordering::Relaxed);
        }
        SINGLE_EXIT => {
            pr.ps_flags.fetch_or(PS_SINGLEEXIT, Ordering::Relaxed);
            pr.ps_flags.fetch_and(!PS_SINGLEUNWIND, Ordering::Relaxed);
        }
        _ => {
            #[cfg(feature = "diagnostic")]
            kpanic(format_args!("single_thread_mode = {mode}"));
        }
    }
    pr.ps_single.set(ptr::from_ref(p));

    let mut h = sched_lock();
    process_stop(pr, P_SUSPSINGLE, mode, &g, &mut h);
    drop(h);

    // count ourself out
    pr.ps_suspendcnt.set(pr.ps_suspendcnt.get().wrapping_sub(1));

    // wait until all other threads suspended
    while pr.ps_suspendcnt.get() > 0 {
        let _ = g.sleep_nsec(ptr::addr_of!(pr.ps_suspendcnt), PWAIT, "suspend", INFSLP);
    }
    drop(g);
    kassert!(
        ptr::eq(pr.ps_single.get(), p) && p.p_flag.load(Ordering::Relaxed) & P_SUSPSINGLE == 0
    );
    Ok(())
}

/// `single_thread_clear`: release the threads `single_thread_set` stopped.
pub fn single_thread_clear<G: ProcGlue>(p: &Thread<G>) {
    let pr = p.process();

    kassert!(ptr::eq(pr.ps_single.get(), p));
    kassert!(is_curproc(p));

    let g = pr.ps_mtx.lock();
    pr.ps_single.set(ptr::null());
    pr.ps_flags
        .fetch_and(!(PS_SINGLEUNWIND | PS_SINGLEEXIT), Ordering::Relaxed);

    let mut h = sched_lock();
    process_continue(pr, P_SUSPSINGLE, &g, &mut h);
    drop(h);

    drop(g);
}
/// Fresh signal actions (all `SIG_DFL`) for a process of the host tests (`sigapl` made once
/// per run).
#[cfg(any(test, feature = "doubles"))]
pub fn test_sigacts<G: ProcGlue>(pr: &Process<G>) {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(signal_init);
    let Some(ps) = SIGACTS_POOL.get(PR_WAITOK, || {
        Sigacts::new_under(&pr.ps_mtx, SigactsM::new(), 0)
    }) else {
        kpanic(format_args!("test_sigacts: sigapl"));
    };
    pr.set_sigacts(SigactsRef::Pooled(ps));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;

    use hal::pin::pin_static;
    use hal::sys::signal::{SIGEMT, SIGINFO, SIGIO, SIGQUIT, SIGTHR, SIGTRAP, SIGURG, SIGWINCH};

    use super::*;
    use crate::cred::Ucred;
    use crate::proc::{ProcessRef, process_alloc, procinit, thread_alloc};
    use crate::testutil;

    crate::test_file_glue!(Sf, Sg);
    crate::test_proc_glue!(Sg, Sf);

    /// A process `pid` with its actions and one thread, on the process lists, all for the
    /// run (the thread is the test thread's `curproc`).
    struct World {
        pr: &'static Process<Sg>,
        p: Pin<&'static Thread<Sg>>,
    }

    fn world(pid: Pid) -> World {
        testutil::install();
        klock::kernel_lock();
        procinit::<Sg>(64, 64);
        let pr = process_alloc::<Sg>(pid, ());
        drop(pr.set_ucred(CredRef::new(Ucred::new())));
        {
            let t = Sg::tables();
            let mut w = klock::witness();
            t.allprocess.insert_head(&mut w, pr.as_pin());
            t.pidhash(pid).insert_head(&mut w, pr.as_pin());
        }
        test_sigacts(&pr);
        let t = thread_alloc::<Sg>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        let t: &'static PoolBox<Thread<Sg>> = Box::leak(Box::new(t));
        let p = pin_static::<Thread<Sg>>(t);
        {
            let mut g = pr.ps_mtx.lock();
            pr.ps_threads.insert_tail(&mut g, p);
        }
        pr.ps_threadcnt.set(1);
        let pr: &'static ProcessRef<Sg> = Box::leak(Box::new(pr));
        Sg::set_curproc(Some(t));
        World { pr, p }
    }

    #[test]
    fn sigprop_is_the_c_table() {
        assert_eq!(SIGPROP.len(), NSIG as usize);
        assert_eq!(SIGPROP[0], 0);
        for sig in [
            SIGQUIT, SIGILL, SIGTRAP, SIGABRT, SIGEMT, SIGFPE, SIGBUS, SIGSEGV, SIGSYS,
        ] {
            assert_eq!(SIGPROP[sig as usize], SA_KILL | SA_CORE, "signal {sig}");
        }
        for sig in [SIGURG, SIGCHLD, SIGIO, SIGWINCH, SIGINFO, SIGTHR] {
            assert_eq!(SIGPROP[sig as usize], SA_IGNORE, "signal {sig}");
        }
        assert_eq!(SIGPROP[SIGCONT as usize], SA_IGNORE | SA_CONT);
        assert_eq!(SIGPROP[SIGSTOP as usize], SA_STOP);
        for sig in [SIGTSTP, SIGTTIN, SIGTTOU] {
            assert_eq!(SIGPROP[sig as usize], SA_STOP | SA_TTYSTOP, "signal {sig}");
        }
        // The stop signals are exactly STOPSIGMASK; no signal is SA_CANTMASK in this table.
        let stops = (1..NSIG)
            .filter(|&s| SIGPROP[s as usize] & SA_STOP != 0)
            .fold(0, |m, s| m | sigmask(s));
        assert_eq!(stops, STOPSIGMASK);
        assert!(SIGPROP.iter().all(|&p| p & SA_CANTMASK == 0));
    }

    #[test]
    fn ffs_counts_from_one() {
        assert_eq!(ffs(0), 0);
        assert_eq!(ffs(1), 1);
        assert_eq!(ffs(sigmask(SIGUSR1) | sigmask(SIGUSR2)), SIGUSR1);
        assert_eq!(ffs(0x8000_0000), 32);
    }

    #[test]
    fn initsiginfo_fills_user_and_fault_members() {
        let si = initsiginfo(SIGUSR1, 0, SI_USER, Sigval::from_int(7));
        assert_eq!((si.si_signo, si.si_code), (SIGUSR1, SI_USER));
        assert_eq!(si.si_value().sival_int(), 7);

        let si = initsiginfo(SIGSEGV, 14, 2, Sigval::from_ptr(0x1000));
        assert_eq!((si.si_signo, si.si_code), (SIGSEGV, 2));
        assert_eq!(si.si_addr(), 0x1000);
        assert_eq!(si.si_trapno(), 14);

        // Kernel codes of other signals carry nothing more.
        let si = initsiginfo(SIGXFSZ, 3, 1, Sigval::from_ptr(0x1000));
        assert_eq!(si.si_addr(), 0);
    }

    #[test]
    fn a_user_timespec_converts_as_the_c_macros() {
        let ts = UserTimespec {
            tv_sec: 2,
            tv_nsec: 5,
        };
        assert!(ts.is_valid());
        assert_eq!(ts.to_nsec(), 2_000_000_005);
        assert!(
            !UserTimespec {
                tv_sec: 0,
                tv_nsec: 1_000_000_000
            }
            .is_valid()
        );
        // A negative second count is huge as the C's unsigned compare sees it.
        assert_eq!(
            UserTimespec {
                tv_sec: -1,
                tv_nsec: 0
            }
            .to_nsec(),
            u64::MAX
        );
    }

    #[test]
    fn siginit_ignores_what_is_ignored_by_default_but_sigcont() {
        let w = world(5);
        siginit(w.pr);
        let g = w.pr.ps_mtx.lock();
        let ignored = w.pr.sigacts().get(&g).ps_sigignore;
        assert_ne!(ignored & sigmask(SIGCHLD), 0);
        assert_ne!(ignored & sigmask(SIGURG), 0);
        assert_eq!(ignored & sigmask(SIGCONT), 0);
        assert_eq!(ignored & sigmask(SIGKILL), 0);
        drop(g);
        assert_eq!(
            w.pr.sigacts().ps_sigflags.load(Ordering::Relaxed),
            SAS_NOCLDWAIT | SAS_NOCLDSTOP
        );
    }

    #[test]
    fn setsigvec_records_the_action_and_execsigs_resets_it() {
        let w = world(6);
        let p = &w.p;

        let mut sa = Sigaction {
            sa_handler: 0x4000,
            sa_mask: sigmask(SIGHUP) | sigmask(SIGKILL),
            sa_flags: SA_RESTART | SA_SIGINFO,
        };
        setsigvec(p, SIGUSR1, &mut sa);
        let bit = sigmask(SIGUSR1);
        {
            let g = w.pr.ps_mtx.lock();
            let m = w.pr.sigacts().get(&g);
            assert_eq!(m.ps_sigact[SIGUSR1 as usize], 0x4000);
            // The signal blocks itself (no SA_NODEFER); SIGKILL can never be masked.
            assert_eq!(m.ps_catchmask[SIGUSR1 as usize], sigmask(SIGHUP) | bit);
            assert_ne!(m.ps_sigcatch & bit, 0);
            assert_ne!(m.ps_siginfo & bit, 0);
            assert_eq!(m.ps_sigintr & bit, 0);
            assert_eq!(m.ps_sigignore & bit, 0);
        }

        // SIG_IGN for SIGCHLD ignores it and asks for no zombies (init does not exist here).
        w.pr.ps_siglist.store(sigmask(SIGCHLD), Ordering::Relaxed);
        let mut ign = Sigaction {
            sa_handler: SIG_IGN,
            ..Sigaction::default()
        };
        setsigvec(p, SIGCHLD, &mut ign);
        {
            let g = w.pr.ps_mtx.lock();
            assert_ne!(w.pr.sigacts().get(&g).ps_sigignore & sigmask(SIGCHLD), 0);
        }
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), 0);
        let flags = || w.pr.sigacts().ps_sigflags.load(Ordering::Relaxed);
        assert_ne!(flags() & SAS_NOCLDWAIT, 0);

        execsigs(p);
        {
            let g = w.pr.ps_mtx.lock();
            let m = w.pr.sigacts().get(&g);
            assert_eq!(m.ps_sigact[SIGUSR1 as usize], SIG_DFL);
            assert_eq!(m.ps_sigcatch, 0);
            assert_eq!(m.ps_sigact[SIGCHLD as usize], SIG_DFL);
        }
        assert_eq!(flags() & SAS_NOCLDWAIT, 0);
        assert_eq!(p.p_sigstk.get().ss_flags, SS_DISABLE);
    }

    #[test]
    fn sigaction_copies_the_old_action_out_and_the_new_in() {
        let w = world(7);
        let nsa = UserPtr::<Sigaction>::new(testutil::USER_BASE + 4096);
        let osa = UserPtr::<Sigaction>::new(testutil::USER_BASE + 4096 + 64);
        nsa.copyout(&Sigaction {
            sa_handler: 0x5000,
            sa_mask: sigmask(SIGHUP),
            sa_flags: SA_NODEFER | SA_ONSTACK,
        })
        .unwrap();
        sigaction(&w.p, SIGUSR2, nsa, UserPtr::new(0)).unwrap();
        sigaction(&w.p, SIGUSR2, UserPtr::new(0), osa).unwrap();
        let old = osa.copyin().unwrap();
        assert_eq!(old.sa_handler, 0x5000);
        assert_eq!(old.sa_mask, sigmask(SIGHUP));
        // No SA_RESTART: the signal interrupts system calls; SA_NODEFER, since the mask
        // leaves the signal out.
        assert_eq!(old.sa_flags, SA_NODEFER | SA_ONSTACK);
        // SIGKILL's action cannot change; signal 0 and NSIG are no signals.
        assert_eq!(
            sigaction(&w.p, SIGKILL, nsa, UserPtr::new(0)),
            Err(Errno::EINVAL)
        );
        assert_eq!(sigaction(&w.p, 0, UserPtr::new(0), osa), Err(Errno::EINVAL));
        assert_eq!(
            sigaction(&w.p, NSIG, UserPtr::new(0), osa),
            Err(Errno::EINVAL)
        );
    }

    #[test]
    fn sigprocmask_blocks_unblocks_and_refuses_a_bad_how() {
        let w = world(8);
        let p = &w.p;
        assert_eq!(
            sigprocmask(p, SIG_BLOCK, sigmask(SIGHUP) | sigmask(SIGKILL)),
            Ok(0)
        );
        // SIGKILL and SIGSTOP are never masked.
        assert_eq!(p.p_sigmask.get(), sigmask(SIGHUP));
        assert_eq!(
            sigprocmask(p, SIG_SETMASK, sigmask(SIGUSR1)),
            Ok(sigmask(SIGHUP))
        );
        assert_eq!(
            sigprocmask(p, SIG_UNBLOCK, sigmask(SIGUSR1)),
            Ok(sigmask(SIGUSR1))
        );
        assert_eq!(p.p_sigmask.get(), 0);
        assert_eq!(sigprocmask(p, 4, 0), Err(Errno::EINVAL));
    }

    #[test]
    fn cursig_picks_the_lowest_deliverable_signal() {
        let w = world(9);
        siginit(w.pr);
        let mut ctx = Sigctx::default();

        // Nothing pending.
        assert_eq!(cursig(&w.p, &mut ctx, false), 0);

        // A caught SIGUSR2, a masked SIGHUP and an ignored-by-default SIGCHLD.
        let mut sa = Sigaction {
            sa_handler: 0x4000,
            ..Sigaction::default()
        };
        setsigvec(&w.p, SIGUSR2, &mut sa);
        w.p.p_sigmask.set(sigmask(SIGHUP));
        w.pr.ps_siglist.store(
            sigmask(SIGHUP) | sigmask(SIGUSR2) | sigmask(SIGCHLD),
            Ordering::Relaxed,
        );
        // SIGCHLD is pending although ignored (it was posted before), so cursig drops it and
        // returns the caught SIGUSR2; the masked SIGHUP stays pending.
        assert_eq!(cursig(&w.p, &mut ctx, false), SIGUSR2);
        assert_eq!(ctx.sig_action, 0x4000);
        assert!(ctx.sig_catch);
        // The taken signal moved to the thread's list for postsig.
        assert_eq!(w.p.p_siglist.load(Ordering::Relaxed), sigmask(SIGUSR2));
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGHUP));

        // Unmasking SIGHUP (default action: kill) makes it the next one, ahead of SIGUSR2.
        w.p.p_sigmask.set(0);
        assert_eq!(cursig(&w.p, &mut ctx, false), SIGHUP);
        assert_eq!(ctx.sig_action, SIG_DFL);
        assert_eq!(sigpending(&w.p), sigmask(SIGHUP) | sigmask(SIGUSR2));
    }

    #[test]
    fn cursig_ignores_default_actions_for_init() {
        let w = world(1);
        let mut ctx = Sigctx::default();
        w.pr.ps_siglist.store(sigmask(SIGTERM), Ordering::Relaxed);
        assert_eq!(cursig(&w.p, &mut ctx, false), 0);
        assert_eq!(sigpending(&w.p), 0);
    }

    #[test]
    fn a_signal_to_a_process_marks_it_pending_unless_ignored() {
        let w = world(10);
        siginit(w.pr);
        // The thread is not running: the signal waits on the process.
        prsignal(w.pr, SIGUSR1);
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGUSR1));
        // Ignored by default (SIGCHLD) or by action: forgotten at once.
        prsignal(w.pr, SIGCHLD);
        let mut ign = Sigaction {
            sa_handler: SIG_IGN,
            ..Sigaction::default()
        };
        setsigvec(&w.p, SIGUSR2, &mut ign);
        prsignal(w.pr, SIGUSR2);
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGUSR1));
        // A thread signal of a running thread is the thread's.
        w.p.set_stat(SONPROC);
        ptsignal(w.p, SIGHUP, SignalType::STHREAD);
        assert_eq!(w.p.p_siglist.load(Ordering::Relaxed), sigmask(SIGHUP));
        w.p.set_stat(0);
        // An exiting process takes nothing.
        w.pr.ps_flags.fetch_or(PS_EXITING, Ordering::Relaxed);
        prsignal(w.pr, SIGTERM);
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGUSR1));
    }

    #[test]
    fn a_tty_stop_signal_to_an_orphaned_group_is_dropped() {
        // A process in no group counts as orphaned: SIGTSTP's default action is discarded,
        // SIGSTOP's is not.
        let w = world(11);
        siginit(w.pr);
        prsignal(w.pr, SIGTSTP);
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), 0);
        prsignal(w.pr, SIGSTOP);
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGSTOP));
        // SIGCONT clears the pending stop signals.
        prsignal(w.pr, SIGCONT);
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGCONT));
    }

    #[test]
    fn kill_checks_the_signal_and_finds_the_process() {
        let w = world(12);
        siginit(w.pr);
        assert_eq!(kill(&w.p, 12, NSIG), Err(Errno::EINVAL));
        assert_eq!(kill(&w.p, 99_998, SIGUSR1), Err(Errno::ESRCH));
        // Signal 0 checks only.
        assert_eq!(kill(&w.p, 12, 0), Ok(()));
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), 0);
        assert_eq!(kill(&w.p, 12, SIGUSR1), Ok(()));
        assert_eq!(w.pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGUSR1));
        // No group: kill(0) finds none.
        assert_eq!(kill(&w.p, 0, SIGUSR1), Err(Errno::ESRCH));
        // A thread signal to the caller itself (tid 0).
        assert_eq!(thrkill(w.p, 0, SIGUSR2, 0), Ok(()));
        assert_eq!(w.p.p_siglist.load(Ordering::Relaxed), sigmask(SIGUSR2));
        assert_eq!(thrkill(w.p, 5, SIGUSR2, 0), Err(Errno::ESRCH));
    }

    #[test]
    fn sigaltstack_sets_reports_and_checks() {
        let w = world(13);
        sigstkinit(&w.p.p_sigstk);
        let nss = UserPtr::<Sigaltstack>::new(testutil::USER_BASE + 4096 + 128);
        let oss = UserPtr::<Sigaltstack>::new(testutil::USER_BASE + 4096 + 192);
        // Bad flags, a small stack.
        nss.copyout(&Sigaltstack {
            ss_sp: 0x1000,
            ss_size: MINSIGSTKSZ,
            ss_flags: SS_ONSTACK,
            _pad: 0,
        })
        .unwrap();
        assert_eq!(sigaltstack(&w.p, nss, oss), Err(Errno::EINVAL));
        assert_eq!(oss.copyin().unwrap().ss_flags, SS_DISABLE);
        nss.copyout(&Sigaltstack {
            ss_sp: 0x1000,
            ss_size: MINSIGSTKSZ - 1,
            ss_flags: 0,
            _pad: 0,
        })
        .unwrap();
        assert_eq!(sigaltstack(&w.p, nss, UserPtr::new(0)), Err(Errno::ENOMEM));
        // Disabling keeps the rest.
        nss.copyout(&Sigaltstack {
            ss_sp: 0,
            ss_size: 0,
            ss_flags: SS_DISABLE,
            _pad: 0,
        })
        .unwrap();
        assert_eq!(sigaltstack(&w.p, nss, UserPtr::new(0)), Ok(()));
        // The stack is on: a stack pointer inside it is "on the stack".
        w.p.p_sigstk.set(Sigaltstack {
            ss_sp: 0x10_0000,
            ss_size: 0x8000,
            ss_flags: 0,
            _pad: 0,
        });
        assert!(sigonstack::<Sg>(0x10_4000));
        assert!(!sigonstack::<Sg>(0x10_8000));
    }

    #[test]
    fn the_pools_item_is_the_actions() {
        let _w = world(14);
        // vmstat -m's sigapl: the actions and their lock key, then the flags (LZ: 424).
        assert_eq!(SIGACTS_POOL.item_size(), Some(size_of::<Sigacts>()));
        // 33 actions, 33 masks, six sets, and the tail padding to the actions' alignment.
        assert_eq!(size_of::<SigactsM>(), 33 * 8 + 33 * 4 + 6 * 4 + 4);
        assert_eq!(size_of::<Sigacts>(), 448);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn sigprop_matches_kern_sig_c() {
        let path = crate::reftest::openbsd_src().join("sys/kern/kern_sig.c");
        let text = std::fs::read_to_string(&path).expect("kern_sig.c");
        let start = text.find("const int sigprop[NSIG] = {").expect("sigprop");
        let body = &text[start..];
        let body = &body[body.find('{').expect("{") + 1..body.find("};").expect("};")];
        let rows: std::vec::Vec<i32> = body
            .lines()
            .filter_map(|l| {
                let v = l.split("/*").next()?.trim().trim_end_matches(',');
                if v.is_empty() {
                    return None;
                }
                Some(v.split('|').fold(0, |m, f| {
                    m | match f.trim() {
                        "0" => 0,
                        "SA_KILL" => SA_KILL,
                        "SA_CORE" => SA_CORE,
                        "SA_STOP" => SA_STOP,
                        "SA_TTYSTOP" => SA_TTYSTOP,
                        "SA_IGNORE" => SA_IGNORE,
                        "SA_CONT" => SA_CONT,
                        other => panic!("unknown property {other}"),
                    }
                }))
            })
            .collect();
        assert_eq!(rows.as_slice(), &SIGPROP[..]);
    }
}
/* </TESTS> */
