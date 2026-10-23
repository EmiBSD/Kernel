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
 * Copyright (c) 1986, 1989, 1991, 1993
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
 *	@(#)proc.h	8.8 (Berkeley) 1/21/94
 */

/*
 * Copyright (c) 1982, 1986, 1989, 1991, 1993
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
 *	@(#)kern_proc.c	8.4 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! The processes' glue to what is still `bsd`'s: the instantiation of `kernel::proc`
//! (`sys/kernel/proc.rs`) that `bsd` runs, with 0 `unsafe`, until signals, descriptors,
//! kqueue, the tty and the scheduler move into the kernel crate (N4.3-N4.8).
//!
//! Upstream: sys/sys/proc.h @ 3ce1f3f79392
//! Upstream: sys/kern/kern_proc.c @ 3ce1f3f79392
//! LZ: sys/sys/proc.rs@3c62ede68802
//! LZ: sys/kern/kern_proc.rs@3c62ede68802
//!
//! Every list, hash, group and session decision is the kernel crate's; this file holds what
//! N4 owns: the members of `struct process`, `struct proc`, `struct pgrp` and `struct
//! session` that belong to subsystems still in `bsd` (the glue's values [`BsdProcessG`],
//! [`BsdThreadG`], [`BsdPgrpG`], [`BsdSessionG`], which the kernel's types dereference to, so
//! `pr.ps_fd` names the glue's member as LZ named its own), the tables `bsd`'s processes live
//! in ([`proc_tables`]), and the calls `kern_proc.c` makes into `bsd` (`pgsignal`,
//! `prsignal`, `pgdelete`'s sigio and terminal, `killjobc`'s terminal, the session's
//! `s_verauthto`). The names `kern_proc.rs` exported (`prfind`, `pgfind`, `tfind`,
//! `procinit`, ...) are here, over the kernel's, with the kernel lock's witness the C's
//! `[K]` annotation asks for.
//!
//! ## Deviations
//! - The session's `s_verauthto` timeout finds its session by address among the sessions of
//!   the groups in `pgrphash` (`kernel::proc::session_find`) where the C casts its `void *`
//!   argument: a session without a group has no process that could read what the timeout
//!   clears.
//! - [`BsdThreadG::p_self`] records the thread's `&'static` where it is placed (`thread_new`,
//!   `proc0`): the run, sleep and dead-thread queues (`hal::list`, until N4.5) take one, and
//!   LZ's convention is that a thread reference lives until the reaper (`curproc()`,
//!   `curthread`).
//!
//! The switch-over's, in the callers it adapted (N4.2; none is visible to userland):
//! - `fork1` makes the process (its pid chosen before the item,
//!   `process_alloc(allocpid(), ..)`) before the thread, where the C makes the thread first.
//! - The orphan's owner (`ps_opptr`, ptrace) is looked up by address among the processes
//!   (`process_clear_orphan`), where the C follows the pointer.
//! - `sysctl_doproc` reads a process's threads one at a time by position under `ps_mtx`
//!   (dropped for the copyout), and `ttyinfo`/`process_sum` walk them under `ps_mtx`, where
//!   the C walks `ps_threads` under the kernel lock alone.
//! - `cansignal`'s "same credentials" test compares the counted references
//!   (`CredRef::ptr_eq` of the threads' `cred_ref()`), the C the `p_ucred` pointers.
//! - The credentials exec, `access(2)` and a core dump give the thread for a while are
//!   placed copies (`CredRef::new`), where the C `crdup`s and edits in place.
//! - ddb's `show proc /t` answers "kernel lock not held" when ddb was entered on a CPU that
//!   does not hold the kernel lock (the tid hash opens only through its reader).
//! - The `assert_unwitnessed` hooks of `kern_lock.rs` stand aside after a panic and in ddb
//!   (which hold the kernel lock by fiat). On the host they run only in a test thread that
//!   arms them (`kernel_lock_checks_arm`, its own depth): every host thread runs as CPU 0,
//!   whose one depth they all share.
//!
//! ## Redesign
//! - LZ's `Cell<*const T>` members of `struct process`/`struct proc` (`ps_fd`, `ps_opptr`,
//!   `p_fd`, `ps_uvpaths`) are `hal::link::PtrWord`s (an address, read and
//!   dereferenced by the legacy code as before); `ps_textvp` and `s_ttyvp` (and `s_ttyp`
//!   until N4.7b, when it became the kernel `Session`'s counted handle and `pgrp_destroy`
//!   went with it), whose targets the legacy code treats as `'static`, are
//!   `hal::link::Link`s, read without
//!   `unsafe`; the plain members are atomic words with LZ's `get()`/`set()` ([`ItimerWord`],
//!   [`IsizeWord`]), so every value is `Send` and `Sync` without a premise, as a pooled
//!   kernel object requires.
//! - N4.5b: `ps_sigacts`, `p_sigstk` and `p_sigval` are `kernel::proc`'s own members
//!   (`kernel::sig`); the glue gives the signals `exit1`, the core dump and the machine's
//!   `sendsig`, `signotify`, `PROC_STACK` and `TCB_GET`.
//! - `allprocess`, `zombprocess`, `allproc` and the hash tables are a
//!   `kernel::proc::ProcTables` (one `static`; each host test thread its own,
//!   [`proc_test_reset`]).

use core::ffi::c_void;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicI64, AtomicIsize, Ordering};

use hal::link::{Link, PtrWord};
use hal::pool::PoolArc;
use hal::rwlock::Witness;
use kernel::api::SysResult;
use kernel::proc::{
    I32Word, Placed, ProcGlue, ProcTables, StaticOrPooled, UsizeWord, pgfind as k_pgfind,
    prfind as k_prfind, tfind as k_tfind, tfind_user as k_tfind_user, zombiefind as k_zombiefind,
};

use crate::conf::param::{MAXPROCESS, MAXTHREAD};
use crate::kern::kern_descrip::native;
use crate::kern::kern_timeout::{timeout_del, timeout_set};
use crate::kern::vfs_subr::vrele;
use crate::kern::vfs_vops::VOP_REVOKE;
use crate::machine::Machine;
use crate::machine::cpu::{Cpu, curcpu};
use crate::sys::_time::Itimerspec;
use crate::sys::errno::Errno;
use crate::sys::proc::{Pgrp, PgrpRef, Proc, Process, ProcessRef, Session};
use crate::sys::siginfo::Siginfo;
use crate::sys::signal::{Sig, Sigset};
use crate::sys::syscall::{
    SYS___get_tcb, SYS___set_tcb, SYS___tfork, SYS___threxit, SYS_acct, SYS_exit, SYS_fork,
    SYS_getegid, SYS_geteuid, SYS_getgid, SYS_getgroups, SYS_getlogin_r, SYS_getpgid, SYS_getpgrp,
    SYS_getpid, SYS_getppid, SYS_getpriority, SYS_getresgid, SYS_getresuid, SYS_getrlimit,
    SYS_getrtable, SYS_getrusage, SYS_getsid, SYS_getthrid, SYS_getthrname, SYS_getuid,
    SYS_issetugid, SYS_setegid, SYS_seteuid, SYS_setgid, SYS_setgroups, SYS_setlogin, SYS_setpgid,
    SYS_setpriority, SYS_setregid, SYS_setresgid, SYS_setresuid, SYS_setreuid, SYS_setrlimit,
    SYS_setrtable, SYS_setsid, SYS_setthrname, SYS_setuid, SYS_vfork, SYS_wait4, SYS_waitid,
};
use crate::sys::syscallargs::{
    SysAcctArgs, SysExitArgs, SysTforkArgs, SysThrexitArgs, SysWait4Args, SysWaitidArgs,
};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::time::Timespec;
use crate::sys::timeout::Timeout;
use crate::sys::types::{Pid, Register};
use crate::sys::vnode::{REVOKEALL, Vnode};

pub use kernel::proc::{inferior, zapverauth};
pub use kernel::sig::{SigaltstackWord, SigvalWord};

/// `struct itimerspec ps_timer[i]`: four words, written whole under `itimer_mtx` (or the
/// process's `ps_mtx` for `ITIMER_REAL`) and read under it.
#[derive(Debug, Default)]
pub struct ItimerWord([AtomicI64; 4]);

impl ItimerWord {
    /// All zero.
    pub const fn new() -> Self {
        Self([const { AtomicI64::new(0) }; 4])
    }

    /// The timer.
    pub fn get(&self) -> Itimerspec {
        let w = |i: usize| self.0[i].load(Ordering::Relaxed);
        Itimerspec {
            it_interval: Timespec::new(w(0), w(1)),
            it_value: Timespec::new(w(2), w(3)),
        }
    }

    /// Stores `it`.
    pub fn set(&self, it: Itimerspec) {
        let v = [
            it.it_interval.tv_sec,
            it.it_interval.tv_nsec,
            it.it_value.tv_sec,
            it.it_value.tv_nsec,
        ];
        for (w, x) in self.0.iter().zip(v) {
            w.store(x, Ordering::Relaxed);
        }
    }
}

/// An `ssize_t` member (`ps_uvvcount`).
#[derive(Debug, Default)]
pub struct IsizeWord(AtomicIsize);

impl IsizeWord {
    /// `v`.
    pub const fn new(v: isize) -> Self {
        Self(AtomicIsize::new(v))
    }

    /// The value.
    pub fn get(&self) -> isize {
        self.0.load(Ordering::Relaxed)
    }

    /// Stores `v`.
    pub fn set(&self, v: isize) {
        self.0.store(v, Ordering::Relaxed);
    }
}

/// The members of `struct process` of `bsd`'s subsystems; a process dereferences to them.
pub struct BsdProcessG {
    /// `ps_textvp`: vnode of executable, referenced (none for the boot module image).
    pub ps_textvp: Link<Vnode>,
    /// \[m\] `ps_realit_to`: `ITIMER_REAL` timeout.
    pub ps_realit_to: Timeout,
    /// \[m\] `ps_timer[ITIMER_REAL]`, \[T\] `ps_timer[ITIMER_VIRTUAL, ITIMER_PROF]`.
    pub ps_timer: [ItimerWord; 3],
    /// `ps_uvpaths`: unveil vnodes and names (`kern_unveil.rs`'s table, null until the first
    /// `unveil(2)`).
    pub ps_uvpaths: PtrWord<crate::kern::kern_unveil::Unveil>,
    /// `ps_uvvcount`: count of unveil vnodes held.
    pub ps_uvvcount: IsizeWord,
    /// `ps_uvncount`: count of unveil names allocated.
    pub ps_uvncount: UsizeWord,
    /// `ps_uvdone`: no more unveil is permitted.
    pub ps_uvdone: I32Word,
}

impl BsdProcessG {
    /// The members of a new process.
    pub const fn new() -> Self {
        Self {
            ps_textvp: Link::new(),
            ps_realit_to: Timeout::zeroed(),
            ps_timer: [const { ItimerWord::new() }; 3],
            ps_uvpaths: PtrWord::new(),
            ps_uvvcount: IsizeWord::new(0),
            ps_uvncount: UsizeWord::new(0),
            ps_uvdone: I32Word::new(0),
        }
    }
}

impl Default for BsdProcessG {
    fn default() -> Self {
        Self::new()
    }
}

/// The members of `struct proc` of `bsd`'s subsystems; a thread dereferences to them.
pub struct BsdThreadG {
    /// \[I\] The thread's own `&'static`, recorded where it is placed (`thread_new`,
    /// `proc0`): what `ProcExt::as_static` lends the queues' inserts. Not the C's.
    pub p_self: Link<Proc>,
}

impl BsdThreadG {
    /// The members of a new thread.
    pub const fn new() -> Self {
        Self {
            p_self: Link::new(),
        }
    }
}

impl Default for BsdThreadG {
    fn default() -> Self {
        Self::new()
    }
}

/// The members of `struct pgrp` of `bsd`'s subsystems: none since N4.5b (`pg_sigiolst` is
/// `kernel::sigio`'s registry).
pub struct BsdPgrpG;

impl BsdPgrpG {
    /// The members of a new group.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for BsdPgrpG {
    fn default() -> Self {
        Self::new()
    }
}

/// The members of `struct session` of `bsd`'s subsystems.
pub struct BsdSessionG {
    /// `s_ttyvp`: vnode of controlling terminal, holding a use count (`vref` in `vn_ioctl`'s
    /// `TIOCSCTTY`).
    pub s_ttyvp: Link<Vnode>,
    /// `s_verauthto`.
    pub s_verauthto: Timeout,
}

impl BsdSessionG {
    /// No terminal.
    pub const fn new() -> Self {
        Self {
            s_ttyvp: Link::new(),
            s_verauthto: Timeout::zeroed(),
        }
    }
}

impl Default for BsdSessionG {
    fn default() -> Self {
        Self::new()
    }
}

/// `bsd`'s instantiation of `kernel::proc` (a `bridges.toml` row while `bsd` implements it).
pub struct BsdProcGlue;

kernel::proc_glue_links!(BsdProcGlue);

/// The scheduler's queues and CPUs of `bsd`'s threads (`kernel::sched`).
#[cfg(not(test))]
static SCHED_TABLES: kernel::sched::SchedTables<BsdProcGlue> = kernel::sched::SchedTables::new();

#[cfg(test)]
std::thread_local! {
    /// Each host test thread's own scheduler tables, made at its first use with the state of
    /// the thread's CPU (a host thread is a CPU of its own, `hal::cpu`).
    static TEST_SCHED: core::cell::Cell<Option<&'static kernel::sched::SchedTables<BsdProcGlue>>> =
        const { core::cell::Cell::new(None) };
}

/// The scheduler's tables of `bsd`'s threads.
fn sched_tables() -> &'static kernel::sched::SchedTables<BsdProcGlue> {
    #[cfg(not(test))]
    {
        &SCHED_TABLES
    }
    #[cfg(test)]
    {
        if let Some(t) = TEST_SCHED.with(core::cell::Cell::get) {
            return t;
        }
        let t: &'static kernel::sched::SchedTables<BsdProcGlue> =
            std::boxed::Box::leak(std::boxed::Box::new(kernel::sched::SchedTables::new()));
        TEST_SCHED.with(|c| c.set(Some(t)));
        kernel::sched::sched_init_cpu::<BsdProcGlue>(hal::cpu::cpu_number());
        t
    }
}

/// The process lists and hash tables of `bsd`'s processes.
#[cfg(not(test))]
static PROC_TABLES: ProcTables<BsdProcGlue> = ProcTables::new();

#[cfg(test)]
std::thread_local! {
    /// Each host test thread's own tables ([`proc_test_reset`] makes fresh ones).
    static TEST_TABLES: core::cell::Cell<&'static ProcTables<BsdProcGlue>> =
        core::cell::Cell::new(std::boxed::Box::leak(std::boxed::Box::new(ProcTables::new())));
}

/// The tables `bsd`'s processes live in.
pub fn proc_tables() -> &'static ProcTables<BsdProcGlue> {
    #[cfg(not(test))]
    {
        &PROC_TABLES
    }
    #[cfg(test)]
    {
        TEST_TABLES.with(core::cell::Cell::get)
    }
}

impl ProcGlue for BsdProcGlue {
    type ProcessG = BsdProcessG;
    type ThreadG = BsdThreadG;
    type PgrpG = BsdPgrpG;
    type SessionG = BsdSessionG;

    type FileGlue = crate::kern::kern_descrip::BsdFileGlue;
    kernel::proc_glue_link_types!();

    fn tables() -> &'static ProcTables<Self> {
        proc_tables()
    }

    fn sched() -> &'static kernel::sched::SchedTables<Self> {
        sched_tables()
    }

    fn cold() -> bool {
        crate::sys::systm::COLD.load(Ordering::Relaxed) || crate::kern::subr_prf::panicstr()
    }

    /// `splhigh`, `splx(safepri)`, the kernel lock's bounce, `splx` back.
    fn cold_yield() {
        let s = crate::machine::intr::splhigh();
        crate::machine::intr::splx(crate::sys::systm::SAFEPRI.load(Ordering::Relaxed));
        crate::kern::kern_synch::kernel_lock_bounce();
        crate::machine::intr::splx(s);
    }

    fn tuagg_add_runtime() {
        crate::kern::kern_resource::tuagg_add_runtime();
    }

    /// `clockintr_cancel` of the running CPU's `spc_itimer` and `spc_profclock`, as asked.
    fn clocks_stop(itimer: bool, profclock: bool) {
        let spc = Machine::ci_schedstate(curcpu());
        if itimer {
            crate::kern::kern_clockintr::clockintr_cancel(&spc.spc_itimer);
        }
        if profclock {
            crate::kern::kern_clockintr::clockintr_cancel(&spc.spc_profclock);
        }
    }

    /// `clockintr_advance` of the running CPU's `spc_itimer` and `spc_profclock`, as asked,
    /// and `spc_runtime = nanouptime()`.
    fn clocks_start(itimer: bool, profclock: bool) {
        let spc = Machine::ci_schedstate(curcpu());
        if itimer {
            crate::kern::kern_clockintr::clockintr_advance(
                &spc.spc_itimer,
                crate::kern::kern_clock::hardclock_period(),
            );
        }
        if profclock {
            crate::kern::kern_clockintr::clockintr_advance(
                &spc.spc_profclock,
                crate::kern::subr_prof::profclock_period(),
            );
        }
        spc.spc_runtime.set(crate::kern::kern_tc::nanouptime());
    }

    fn assertwaitok() {
        crate::kern::subr_xxx::assertwaitok();
    }

    fn smr_idle() {
        crate::kern::kern_smr::smr_idle();
    }

    fn spl0() {
        crate::machine::intr::spl0();
    }

    /// `exit2`'s `tuagg_add_process(p->p_p, p)` under `ps_mtx`: the remainder of the time
    /// spent in `exit1`.
    fn exit_account(p: &Proc) {
        let pr = p.process();
        let g = pr.ps_mtx.lock();
        kernel::resource::tuagg_add_process(pr, p, &g);
        drop(g);
    }

    fn cpu_unidle(n: usize) {
        if let Some(ci) = crate::kern::kern_sched::cpu_info_by_number(n) {
            Machine::cpu_unidle(ci);
        }
    }

    fn need_resched(n: usize) {
        if let Some(ci) = crate::kern::kern_sched::cpu_info_by_number(n) {
            Machine::need_resched(ci);
        }
    }

    fn idle_enter() {
        Machine::cpu_idle_enter();
    }

    fn idle_cycle() {
        Machine::cpu_idle_cycle();
    }

    fn idle_leave() {
        Machine::cpu_idle_leave();
    }

    fn cpu_is_primary(n: usize) -> bool {
        crate::kern::kern_sched::cpu_info_by_number(n).is_some_and(Machine::cpu_is_primary)
    }

    fn cpu_is_running(n: usize) -> bool {
        crate::kern::kern_sched::cpu_info_by_number(n).is_some_and(Machine::cpu_is_running)
    }

    fn cputype(n: usize) -> i32 {
        crate::kern::kern_sched::cpu_info_by_number(n).map_or(0, Machine::ci_cputype)
    }

    /// `pmap_resident_count(p->p_vmspace->vm_map.pmap)` of a thread with an address space.
    fn resident_count(p: &Proc) -> Option<i64> {
        use crate::sys::proc::ProcExt;
        (p.vmspace_word() != 0)
            .then(|| crate::machine::pmap::pmap_resident_count(p.vmspace().vm_map.pmap()))
    }

    fn stathz() -> i32 {
        crate::kern::kern_clock::STATHZ.load(Ordering::Relaxed)
    }

    /// `killjobc`'s controlling process: signal the foreground group, drain the controlling
    /// terminal (`kernel::tty::session_hangup_tty`) and revoke access to it; the vnode's use
    /// count dropped. `s_ttyp` is not zeroed: it says the session once had a controlling
    /// terminal (for logging and informational purposes).
    fn session_hangup(sp: &Session) {
        if sp.s_ttyvp.get().is_none() {
            return;
        }
        if kernel::tty::session_hangup_tty(sp) {
            // The tty could have been revoked if we blocked.
            if let Some(vp) = sp.s_ttyvp.get() {
                let _ = VOP_REVOKE(vp, REVOKEALL);
            }
        }
        let ovp = sp.s_ttyvp.get();
        sp.s_ttyvp.set(None);
        if let Some(ovp) = ovp {
            vrele(ovp);
        }
    }

    fn session_placed(sess: &Session) {
        timeout_set(
            &sess.s_verauthto,
            zapverauth_timeout,
            ptr::from_ref(sess).cast_mut().cast::<c_void>(),
        );
    }

    fn session_destroy(sess: &Session) {
        timeout_del(&sess.s_verauthto);
    }

    fn with_curproc<R>(f: impl FnOnce(Option<&Proc>) -> R) -> R {
        hal::thread::try_with_current::<Proc, R>(|p| f(p.map(core::pin::Pin::get_ref)))
    }

    fn pledge_fail(p: &Proc, error: Errno, code: u64) -> Errno {
        crate::kern::kern_pledge::pledge_fail(p, error, code)
    }

    fn process_ref(pr: &Process) -> ProcessRef {
        process_ref(pr)
    }

    fn coredump(p: &Proc) -> Result<(), Errno> {
        crate::kern::kern_sig::coredump(p)
    }

    /// The machine's `sendsig`, which builds the frame of the running thread (`p`).
    fn sendsig(
        _p: &Proc,
        catcher: Sig,
        sig: i32,
        mask: Sigset,
        ksip: &Siginfo,
        info: bool,
        onstack: bool,
    ) -> Result<(), Errno> {
        crate::machine::signal::sendsig(catcher, sig, mask, ksip, info, onstack)
    }

    fn signotify(p: &Proc) {
        Machine::signotify(p);
    }

    fn proc_stack(p: &Proc) -> usize {
        Machine::proc_stack(p)
    }

    fn tcb_get(p: &Proc) -> usize {
        crate::machine::tcb::tcb_get(p)
    }

    fn maxprocess() -> i32 {
        MAXPROCESS.load(Ordering::Relaxed)
    }

    fn maxthread() -> i32 {
        MAXTHREAD.load(Ordering::Relaxed)
    }

    fn log(level: i32, args: core::fmt::Arguments<'_>) {
        crate::kern::subr_prf::log(level, args);
    }

    /// The thread's `&'static`, which `bsd`'s run, sleep and signal adapters lend.
    fn thread_placed(p: Placed<'_, Self>) {
        crate::sys::proc::place_thread(p);
    }

    /// `timeout_set_flags(&pr->ps_realit_to, realitexpire, pr, KCLOCK_UPTIME, 0)`.
    fn process_initialized(pr: &Process) {
        crate::kern::kern_timeout::timeout_set_flags(
            &pr.ps_realit_to,
            crate::kern::kern_time::realitexpire,
            ptr::from_ref(pr).cast_mut().cast::<c_void>(),
            crate::sys::timeout::KCLOCK_UPTIME,
            0,
        );
    }

    /// The text vnode's reference (for sysctl) and the unveils copied.
    fn process_forked(parent: &Process, child: &Process) {
        child.ps_textvp.set(parent.ps_textvp.get());
        if let Some(vp) = child.ps_textvp.get() {
            crate::kern::vfs_subr::vref(vp);
        }
        crate::kern::kern_unveil::unveil_copy(parent, child);
    }

    fn session_has_tty_vnode(sess: &Session) -> bool {
        sess.s_ttyvp.get().is_some()
    }

    fn pmap_activate(p: &Proc) {
        crate::machine::pmap::pmap_activate(p);
    }

    fn pmap_deactivate(p: &Proc) {
        crate::machine::pmap::pmap_deactivate(p);
    }

    fn pmap_purge(p: &Proc) {
        crate::machine::pmap::pmap_purge(p);
    }

    /// The machine's `child_return`, which takes the thread as the C's `void *`.
    fn child_return(p: &Proc) {
        crate::machine::cpu::child_return(ptr::from_ref(p).cast_mut().cast::<c_void>());
    }

    fn start_init(p: &Proc) {
        crate::kern::init_main::start_init(p);
    }

    fn tcb_invalid(tcb: usize) -> bool {
        crate::machine::tcb::tcb_invalid(tcb)
    }

    fn tcb_set(p: &Proc, tcb: usize) {
        crate::machine::tcb::tcb_set(p, tcb);
    }

    fn rtable_exists(id: u32) -> bool {
        crate::net::rtable::rtable_exists(id)
    }

    fn pgrp_ref(pg: &Pgrp) -> PgrpRef {
        pgrp_ref(pg)
    }

    /// Under QEMU init's exit ends the run: the smokes' criterion. The C panics, as it has
    /// nothing to run.
    fn init_exited(xexit: i32, xsig: i32) -> ! {
        #[cfg(feature = "qemu")]
        {
            crate::kprintf!("init exited with status {xexit} (signal {xsig})\n");
            <crate::machine::Machine as crate::machine::Exit>::exit(if xexit == 0 && xsig == 0 {
                crate::machine::ExitStatus::Success
            } else {
                crate::machine::ExitStatus::Failure
            });
        }
        #[cfg(not(feature = "qemu"))]
        crate::kern::subr_prf::panic(format_args!("init died (signal {xsig}, exit {xexit})"));
    }

    fn cancel_all_itimers() {
        crate::kern::kern_time::cancel_all_itimers();
    }

    fn unveil_destroy(pr: &Process) {
        crate::kern::kern_unveil::unveil_destroy(pr);
    }

    /// `vrele(pr->ps_textvp)`, the reference dropped.
    fn textvp_release(pr: &Process) {
        let otvp = pr.ps_textvp.get();
        pr.ps_textvp.set(None);
        if let Some(otvp) = otvp {
            vrele(otvp);
        }
    }
}

/// `s_verauthto`'s function: `zapverauth` of the session at `arg` (a timeout without
/// `TIMEOUT_MPSAFE` runs under the kernel lock).
fn zapverauth_timeout(arg: *mut c_void) {
    let r = hal::klock::reader();
    if let Some(sess) = kernel::proc::session_find::<BsdProcGlue, _>(&r, arg.addr()) {
        zapverauth(sess);
    }
}

/// `procinit()`: the hash tables and pools (`maxthread / 4`, `maxprocess / 4` chains). A host
/// test calls it once per test: its thread gets fresh tables first.
pub fn procinit() {
    #[cfg(test)]
    proc_test_reset();
    kernel::proc::procinit::<BsdProcGlue>(
        MAXTHREAD.load(Ordering::Relaxed),
        MAXPROCESS.load(Ordering::Relaxed),
    );
}

/// `tfind(tid)`.
pub fn tfind<W: Witness + ?Sized>(w: &W, tid: Pid) -> Option<&Proc> {
    k_tfind::<BsdProcGlue, W>(w, tid)
}

/// `tfind_user(tid, pr)`.
pub fn tfind_user<'a, W: Witness + ?Sized>(w: &'a W, tid: Pid, pr: &Process) -> Option<&'a Proc> {
    k_tfind_user::<BsdProcGlue, W>(w, tid, pr)
}

/// `prfind(pid)`.
pub fn prfind<W: Witness + ?Sized>(w: &W, pid: Pid) -> Option<&Process> {
    k_prfind::<BsdProcGlue, W>(w, pid)
}

/// `pgfind(pgid)`.
pub fn pgfind<W: Witness + ?Sized>(w: &W, pgid: Pid) -> Option<&Pgrp> {
    k_pgfind::<BsdProcGlue, W>(w, pgid)
}

/// `zombiefind(pid)`.
pub fn zombiefind<W: Witness + ?Sized>(w: &W, pid: Pid) -> Option<&Process> {
    k_zombiefind::<BsdProcGlue, W>(w, pid)
}

/// A counted reference to `pr` (`process0`'s `static` or a `processpl` item), for a caller
/// that keeps a process found under a witness past it.
pub fn process_ref(pr: &Process) -> ProcessRef {
    if ptr::eq(pr, &crate::kern::init_main::PROCESS0) {
        return StaticOrPooled::Static(&crate::kern::init_main::PROCESS0);
    }
    match PoolArc::from_ref(pr) {
        Some(a) => StaticOrPooled::Pooled(a),
        None => crate::kern::subr_prf::panic(format_args!("process_ref: not a process")),
    }
}

/// `LIST_FOREACH(pr, &allprocess, ps_list)` for a holder of the kernel lock whose work on a
/// process may sleep (`vrele`): `pick` chooses under the lock's reader (it must not sleep),
/// `work` runs on the chosen process (counted) without it; the walk then starts again from
/// the head, so `pick` must refuse a process `work` already changed. Where the C keeps its
/// place in the list across the sleep, which another CPU may have changed meanwhile.
pub fn allprocess_rescan(mut pick: impl FnMut(&Process) -> bool, mut work: impl FnMut(&Process)) {
    loop {
        let next = {
            let r = hal::klock::reader();
            proc_tables()
                .allprocess
                .iter(&r)
                .find(|pr| pick(pr))
                .map(process_ref)
        };
        let Some(pr) = next else {
            return;
        };
        work(&pr);
    }
}

/// `prfind(pid)`, counted, for a holder of the kernel lock who keeps the process past the
/// lookup (a sleep, a copy, a signal).
pub fn prfind_ref(pid: Pid) -> Option<ProcessRef> {
    let r = hal::klock::reader();
    prfind(&r, pid).map(process_ref)
}

/// `pr->ps_pgrp`, counted, for a holder of the kernel lock who keeps it past the reader
/// (a sleep, a signal); `None` for a process in no group yet.
pub fn pgrp_of(pr: &Process) -> Option<PgrpRef> {
    let r = hal::klock::reader();
    pr.pgrp(&r).cloned()
}

/// `pr->ps_session`, counted, for a holder of the kernel lock who keeps it past the reader.
pub fn session_of(pr: &Process) -> Option<crate::sys::proc::SessionRef> {
    let r = hal::klock::reader();
    pr.pgrp(&r).and_then(|pg| pg.pg_session.get(&r).clone())
}

/// `sess->s_leader`, the process the session's leader pid names (`None` once the leader is
/// gone: `killjobc` cleared it before the process left the pid hash).
pub fn session_leader<'a, W: Witness + ?Sized>(w: &'a W, sess: &Session) -> Option<&'a Process> {
    prfind(w, sess.leader_pid()?)
}

/// A counted reference to `pg` (`pgrp0`'s `static` or a `pgrppl` item), for a caller that
/// keeps a group found under a witness past it.
pub fn pgrp_ref(pg: &Pgrp) -> PgrpRef {
    if ptr::eq(pg, &crate::kern::init_main::PGRP0) {
        return StaticOrPooled::Static(&crate::kern::init_main::PGRP0);
    }
    match PoolArc::from_ref(pg) {
        Some(a) => StaticOrPooled::Pooled(a),
        None => crate::kern::subr_prf::panic(format_args!("pgrp_ref: not a group")),
    }
}

/// `chgproccnt(uid, diff)` in `bsd`'s uid table ([`kernel::proc::chgproccnt`]).
pub fn chgproccnt(uid: crate::sys::types::Uid, diff: i64) -> i64 {
    kernel::proc::chgproccnt::<BsdProcGlue>(uid, diff)
}

/// `uid_find(uid)` ... `uid_release(uip)` in `bsd`'s uid table
/// ([`kernel::proc::with_uidinfo`]).
pub fn with_uidinfo<R>(
    uid: crate::sys::types::Uid,
    f: impl FnOnce(&kernel::proc::Uidinfo) -> R,
) -> R {
    kernel::proc::with_uidinfo::<BsdProcGlue, R>(uid, f)
}

/// The thread `p` of a system call, pinned (`bsd`'s threads are placed, `ProcExt::as_static`).
fn pinned(p: &Proc) -> Pin<&'static Proc> {
    hal::pin::pin_static(crate::sys::proc::ProcExt::as_static(p))
}

/// `fork(2)`.
pub fn sys_fork(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::fork::sys_fork(pinned(p))? as Register;
    Ok(())
}

/// `vfork(2)`.
pub fn sys_vfork(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::fork::sys_vfork(pinned(p))? as Register;
    Ok(())
}

/// `__tfork(2)`.
#[allow(non_snake_case)] // the C name, verbatim: the syscall table finds it by name
pub fn sys___tfork(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysTforkArgs = sysargs(v);
    retval[0] =
        kernel::fork::sys___tfork(pinned(p), uap.param.get().addr(), uap.psize.get())? as Register;
    Ok(())
}

/// `exit(2)`.
pub fn sys_exit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysExitArgs = sysargs(v);
    kernel::exit::sys_exit(p, uap.rval.get())
}

/// `__threxit(2)`.
#[allow(non_snake_case)] // the C name, verbatim: the syscall table finds it by name
pub fn sys___threxit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysThrexitArgs = sysargs(v);
    kernel::exit::sys___threxit(p, hal::user::UserPtr::new(uap.notdead.get().addr()))
}

/// `wait4(2)`.
pub fn sys_wait4(q: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysWait4Args = sysargs(v);
    retval[0] = kernel::exit::sys_wait4(
        q,
        uap.pid.get(),
        hal::user::UserPtr::new(uap.status.get().addr()),
        uap.options.get(),
        hal::user::UserPtr::new(uap.rusage.get().addr()),
    )? as Register;
    Ok(())
}

/// `waitid(2)`.
pub fn sys_waitid(q: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysWaitidArgs = sysargs(v);
    kernel::exit::sys_waitid(
        q,
        uap.idtype.get(),
        i64::from(uap.id.get()),
        hal::user::UserPtr::new(uap.info.get().addr()),
        uap.options.get(),
    )?;
    retval[0] = 0;
    Ok(())
}

/// `acct(2)`.
pub fn sys_acct(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysAcctArgs = sysargs(v);
    kernel::acct::sys_acct(p, hal::user::UserPtr::new(uap.path.get().addr()))
}

/// The accounting file of `kernel::acct` (bridges.toml `AcctFile`, N5): `vn_open` of a
/// regular file for `FWRITE | O_APPEND` with `UNVEIL_WRITE`, unlocked.
pub struct BsdAcctFile;

/// The registered [`BsdAcctFile`].
pub static BSD_ACCT_FILE: BsdAcctFile = BsdAcctFile;

/// An open accounting file: its vnode (held open by `vn_open` until `close`).
struct BsdAcctSink(&'static Vnode);

impl kernel::acct::AcctFile for BsdAcctFile {
    fn open(
        &self,
        path: hal::user::UserPtr<u8>,
    ) -> Result<alloc::boxed::Box<dyn kernel::acct::AcctSink>, Errno> {
        let Some(p) = crate::machine::cpu::curproc() else {
            crate::kern::subr_prf::panic(format_args!("acct: no curproc"));
        };
        let mut nd =
            crate::kern::vfs_lookup::ndinit(0, 0, crate::sys::namei::NiDirp::User(path.addr()), p);
        nd.ni_unveil = crate::sys::namei::UNVEIL_WRITE;
        crate::kern::vfs_vnops::vn_open(
            &mut nd,
            crate::sys::fcntl::FWRITE | crate::sys::fcntl::O_APPEND,
            0,
        )?;
        let Some(vp) = nd.ni_vp else {
            return Err(Errno::ENOENT);
        };
        let _ = crate::kern::vfs_vops::VOP_UNLOCK(vp);
        if vp.v_type.get() != crate::sys::vnode::VREG {
            let _ = crate::kern::vfs_vnops::vn_close(
                vp,
                crate::sys::fcntl::FWRITE,
                p.ucred_ptr(),
                Some(p),
            );
            return Err(Errno::EACCES);
        }
        Ok(alloc::boxed::Box::new(BsdAcctSink(vp)))
    }
}

impl kernel::acct::AcctSink for BsdAcctSink {
    fn write(&self, rec: &[u8]) -> Result<(), Errno> {
        let p = crate::machine::cpu::curproc();
        crate::kern::vfs_vnops::vn_rdwr(
            crate::sys::uio::UioRw::UIO_WRITE,
            self.0,
            rec.as_ptr().cast_mut().cast::<c_void>(),
            rec.len(),
            0,
            crate::sys::uio::UioSeg::UIO_SYSSPACE,
            crate::sys::vnode::IO_APPEND
                | crate::sys::vnode::IO_UNIT
                | crate::sys::vnode::IO_NOLIMIT,
            p.map_or(crate::sys::ucred::NOCRED, |p| p.ucred_ptr()),
            None,
            p,
        )
    }

    fn free_space(&self) -> Option<(i64, i64)> {
        let mp = self.0.v_mount.get()?;
        let p = crate::machine::cpu::curproc()?;
        let mut sb = crate::sys::mount::Statfs::default();
        let _ = crate::sys::mount::VFS_STATFS(mp, &mut sb, p);
        Some((sb.f_bavail, sb.f_blocks as i64))
    }

    fn is_bad(&self) -> bool {
        self.0.v_type.get() == crate::sys::vnode::VBAD
    }

    fn close(self: alloc::boxed::Box<Self>, by_curproc: bool) {
        let p = crate::machine::cpu::curproc();
        let cred = match p {
            Some(p) if by_curproc => p.ucred_ptr(),
            _ => crate::sys::ucred::NOCRED,
        };
        let _ = crate::kern::vfs_vnops::vn_close(self.0, crate::sys::fcntl::FWRITE, cred, p);
    }
}

/// `KernelApi::proc`: the `Native` entries of the process system calls (N4.6a), each running
/// `mi_syscall` on its own `sysent[]` row (whose function calls the kernel's).
pub const PROC_ENTRIES: hal::api::ProcEntries = hal::api::ProcEntries {
    fork: native_fork,
    vfork: native_vfork,
    tfork: native_tfork,
    exit: native_exit,
    threxit: native_threxit,
    wait4: native_wait4,
    waitid: native_waitid,
    getrusage: native_getrusage,
    getrlimit: native_getrlimit,
    setrlimit: native_setrlimit,
    getpriority: native_getpriority,
    setpriority: native_setpriority,
    acct: native_acct,
    getpid: native_getpid,
    getppid: native_getppid,
    getuid: native_getuid,
    geteuid: native_geteuid,
    getgid: native_getgid,
    getegid: native_getegid,
    getgroups: native_getgroups,
    setgroups: native_setgroups,
    getpgrp: native_getpgrp,
    getpgid: native_getpgid,
    getsid: native_getsid,
    setpgid: native_setpgid,
    setsid: native_setsid,
    setuid: native_setuid,
    seteuid: native_seteuid,
    setgid: native_setgid,
    setegid: native_setegid,
    setreuid: native_setreuid,
    setregid: native_setregid,
    setresuid: native_setresuid,
    setresgid: native_setresgid,
    getresuid: native_getresuid,
    getresgid: native_getresgid,
    issetugid: native_issetugid,
    getlogin_r: native_getlogin_r,
    setlogin: native_setlogin,
    set_tcb: native_set_tcb,
    get_tcb: native_get_tcb,
    getthrid: native_getthrid,
    getthrname: native_getthrname,
    setthrname: native_setthrname,
    getrtable: native_getrtable,
    setrtable: native_setrtable,
};

/// `KernelApi::proc.fork`: `fork(void)`.
pub fn native_fork(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_fork, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.vfork`: `vfork(void)`.
pub fn native_vfork(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_vfork, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.tfork`: `__tfork(const struct __tfork *param, size_t psize)`.
pub fn native_tfork(
    nr: Register,
    param: usize,
    psize: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS___tfork,
        [param as Register, psize as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.exit`: `exit(int rval)`.
pub fn native_exit(nr: Register, status: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_exit, [status as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.threxit`: `__threxit(pid_t *notdead)`.
pub fn native_threxit(nr: Register, notdead: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS___threxit,
        [notdead as Register, 0, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.wait4`: `wait4(pid_t pid, int *status, int options, struct rusage *rusage)`.
pub fn native_wait4(
    nr: Register,
    pid: i32,
    status: usize,
    options: i32,
    rusage: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_wait4,
        [
            pid as Register,
            status as Register,
            options as Register,
            rusage as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `KernelApi::proc.waitid`: `waitid(int idtype, id_t id, siginfo_t *info, int options)`.
pub fn native_waitid(
    nr: Register,
    idtype: i32,
    id: u32,
    info: usize,
    options: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_waitid,
        [
            idtype as Register,
            id as Register,
            info as Register,
            options as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `KernelApi::proc.getrusage`: `getrusage(int who, struct rusage *rusage)`.
pub fn native_getrusage(
    nr: Register,
    who: i32,
    rusage: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getrusage,
        [who as Register, rusage as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.getrlimit`: `getrlimit(int which, struct rlimit *rlp)`.
pub fn native_getrlimit(
    nr: Register,
    which: i32,
    rlp: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getrlimit,
        [which as Register, rlp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.setrlimit`: `setrlimit(int which, const struct rlimit *rlp)`.
pub fn native_setrlimit(
    nr: Register,
    which: i32,
    rlp: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_setrlimit,
        [which as Register, rlp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.getpriority`: `getpriority(int which, id_t who)`.
pub fn native_getpriority(
    nr: Register,
    which: i32,
    who: u32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getpriority,
        [which as Register, who as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.setpriority`: `setpriority(int which, id_t who, int prio)`.
pub fn native_setpriority(
    nr: Register,
    which: i32,
    who: u32,
    prio: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_setpriority,
        [
            which as Register,
            who as Register,
            prio as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `KernelApi::proc.acct`: `acct(const char *path)`.
pub fn native_acct(nr: Register, path: usize, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_acct, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getpid`: `getpid(void)`.
pub fn native_getpid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getpid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getppid`: `getppid(void)`.
pub fn native_getppid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getppid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getuid`: `getuid(void)`.
pub fn native_getuid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getuid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.geteuid`: `geteuid(void)`.
pub fn native_geteuid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_geteuid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getgid`: `getgid(void)`.
pub fn native_getgid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getgid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getegid`: `getegid(void)`.
pub fn native_getegid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getegid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getgroups`: `getgroups(int gidsetsize, gid_t *gidset)`.
pub fn native_getgroups(
    nr: Register,
    gidsetsize: i32,
    gidset: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getgroups,
        [gidsetsize as Register, gidset as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.setgroups`: `setgroups(int gidsetsize, const gid_t *gidset)`.
pub fn native_setgroups(
    nr: Register,
    gidsetsize: i32,
    gidset: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_setgroups,
        [gidsetsize as Register, gidset as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.getpgrp`: `getpgrp(void)`.
pub fn native_getpgrp(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getpgrp, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getpgid`: `getpgid(pid_t pid)`.
pub fn native_getpgid(nr: Register, pid: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getpgid, [pid as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getsid`: `getsid(pid_t pid)`.
pub fn native_getsid(nr: Register, pid: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getsid, [pid as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.setpgid`: `setpgid(pid_t pid, pid_t pgid)`.
pub fn native_setpgid(nr: Register, pid: i32, pgid: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_setpgid,
        [pid as Register, pgid as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.setsid`: `setsid(void)`.
pub fn native_setsid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_setsid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.setuid`: `setuid(uid_t uid)`.
pub fn native_setuid(nr: Register, uid: u32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_setuid, [uid as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.seteuid`: `seteuid(uid_t euid)`.
pub fn native_seteuid(nr: Register, euid: u32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_seteuid, [euid as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.setgid`: `setgid(gid_t gid)`.
pub fn native_setgid(nr: Register, gid: u32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_setgid, [gid as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.setegid`: `setegid(gid_t egid)`.
pub fn native_setegid(nr: Register, egid: u32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_setegid, [egid as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.setreuid`: `setreuid(uid_t ruid, uid_t euid)`.
pub fn native_setreuid(nr: Register, ruid: u32, euid: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_setreuid,
        [ruid as Register, euid as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.setregid`: `setregid(gid_t rgid, gid_t egid)`.
pub fn native_setregid(nr: Register, rgid: u32, egid: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_setregid,
        [rgid as Register, egid as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.setresuid`: `setresuid(uid_t ruid, uid_t euid, uid_t suid)`.
pub fn native_setresuid(
    nr: Register,
    ruid: u32,
    euid: u32,
    suid: u32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_setresuid,
        [
            ruid as Register,
            euid as Register,
            suid as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `KernelApi::proc.setresgid`: `setresgid(gid_t rgid, gid_t egid, gid_t sgid)`.
pub fn native_setresgid(
    nr: Register,
    rgid: u32,
    egid: u32,
    sgid: u32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_setresgid,
        [
            rgid as Register,
            egid as Register,
            sgid as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `KernelApi::proc.getresuid`: `getresuid(uid_t *ruid, uid_t *euid, uid_t *suid)`.
pub fn native_getresuid(
    nr: Register,
    ruid: usize,
    euid: usize,
    suid: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getresuid,
        [
            ruid as Register,
            euid as Register,
            suid as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `KernelApi::proc.getresgid`: `getresgid(gid_t *rgid, gid_t *egid, gid_t *sgid)`.
pub fn native_getresgid(
    nr: Register,
    rgid: usize,
    egid: usize,
    sgid: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getresgid,
        [
            rgid as Register,
            egid as Register,
            sgid as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `KernelApi::proc.issetugid`: `issetugid(void)`.
pub fn native_issetugid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_issetugid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getlogin_r`: `getlogin_r(char *namebuf, size_t namelen)`.
pub fn native_getlogin_r(
    nr: Register,
    namebuf: usize,
    namelen: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getlogin_r,
        [namebuf as Register, namelen as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.setlogin`: `setlogin(const char *namebuf)`.
pub fn native_setlogin(nr: Register, namebuf: usize, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_setlogin, [namebuf as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.set_tcb`: `__set_tcb(void *tcb)`.
pub fn native_set_tcb(nr: Register, tcb: usize, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS___set_tcb, [tcb as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.get_tcb`: `__get_tcb(void)`.
pub fn native_get_tcb(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS___get_tcb, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getthrid`: `getthrid(void)`.
pub fn native_getthrid(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getthrid, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.getthrname`: `getthrname(pid_t tid, char *name, size_t len)`.
pub fn native_getthrname(
    nr: Register,
    tid: i32,
    name: usize,
    len: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getthrname,
        [tid as Register, name as Register, len as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.setthrname`: `setthrname(pid_t tid, const char *name)`.
pub fn native_setthrname(
    nr: Register,
    tid: i32,
    name: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_setthrname,
        [tid as Register, name as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::proc.getrtable`: `getrtable(void)`.
pub fn native_getrtable(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getrtable, [0, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::proc.setrtable`: `setrtable(int rtableid)`.
pub fn native_setrtable(nr: Register, rtableid: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_setrtable,
        [rtableid as Register, 0, 0, 0, 0, 0],
        rval,
    )
}

/// A process of the host tests with pid `pid`, leaked (never freed: the tests' processes
/// live for the run).
#[cfg(test)]
pub(crate) fn test_process(pid: Pid) -> &'static Process {
    std::boxed::Box::leak(std::boxed::Box::new(Process::new(pid, BsdProcessG::new())))
}

/// A thread of the host tests of `pr`, placed, leaked, on none of the process's lists.
#[cfg(test)]
pub(crate) fn test_thread(pr: &'static Process) -> &'static Proc {
    let p: &'static Proc = std::boxed::Box::leak(std::boxed::Box::new(Proc::new_static(
        pr,
        BsdThreadG::new(),
    )));
    p.p_self.set(Some(p));
    p
}

/// A process of the host tests with pid `pid` placed in `processpl`, as `process_new` places
/// one, so the walks that count it (`process_ref`) take it; leaked, never freed. Needs
/// [`procinit`].
#[cfg(test)]
pub(crate) fn test_pooled_process(pid: Pid) -> &'static Process {
    match kernel::proc::process_alloc::<BsdProcGlue>(pid, BsdProcessG::new()) {
        StaticOrPooled::Pooled(a) => PoolArc::leak(a),
        StaticOrPooled::Static(s) => s,
    }
}

/// A thread of a fresh test process with pid `pid` ([`test_pooled_process`]), its main one,
/// both holding `cr` (as `fork1` leaves them). Needs [`procinit`].
#[cfg(test)]
pub(crate) fn test_proc(pid: Pid, cr: kernel::cred::CredRef) -> &'static Proc {
    let pr = test_pooled_process(pid);
    let p = test_thread(pr);
    pr.set_main_static(hal::pin::pin_static(p));
    drop(p.set_cred(cr.clone()));
    drop(pr.set_ucred(cr));
    p
}

/// The limits of the tests' processes: none but `RLIMIT_NOFILE`, `nofile` (what `fdalloc`
/// reads), for `p`'s process, and `p`'s copy forgotten so its next read takes them.
#[cfg(test)]
pub(crate) fn test_set_limits(p: &Proc, nofile: hal::sys::types::Rlim) {
    use crate::sys::resource::{RLIM_INFINITY, RLIMIT_NOFILE, Rlimit};
    let mut limit = crate::sys::resourcevar::Plimit::new();
    for l in &mut limit.pl_rlimit {
        *l = Rlimit {
            rlim_cur: RLIM_INFINITY,
            rlim_max: RLIM_INFINITY,
        };
    }
    limit.pl_rlimit[RLIMIT_NOFILE] = Rlimit {
        rlim_cur: nofile,
        rlim_max: nofile,
    };
    drop(
        p.process()
            .set_limit(kernel::resource::PlimitRef::new(limit)),
    );
    p.p_limit.clear();
}

/// `setsid` for a test process: a new session `pr` leads and a new group of its own (pgid
/// its pid), as `enternewpgrp` makes them, under the kernel lock as `setsid(2)` runs. Needs
/// [`procinit`].
#[cfg(test)]
pub(crate) fn test_setsid(pr: &'static Process) -> (PgrpRef, crate::sys::proc::SessionRef) {
    let sess = kernel::proc::session_alloc::<BsdProcGlue>(BsdSessionG::new());
    let pgrp = kernel::proc::pgrp_alloc::<BsdProcGlue>(BsdPgrpG::new());
    crate::sys::systm::kernel_lock();
    kernel::proc::enternewpgrp(
        &StaticOrPooled::Static(pr),
        pgrp.clone(),
        Some(sess.clone()),
    );
    crate::sys::systm::kernel_unlock();
    (pgrp, sess)
}

/// Fresh, empty tables for this host test thread, the uid table among them; the old ones
/// (and the processes and uid entries on them) are forgotten, never freed (`testing.md`, the
/// `*_test_reset` rule). The kernel's process-wide pools (`ucredpl`, `plimitpl`,
/// `zombiepl`) are made once per test process and kept: their items come from
/// `kernel::testutil`'s heap doubles, never from the memory `setup_real_memory` loads, which
/// is checked here.
#[cfg(test)]
pub(crate) fn proc_test_reset() {
    if kernel::testutil::memory_is_the_doubles() == Some(false) {
        crate::kern::subr_prf::panic(format_args!(
            "proc_test_reset: the kernel's pools are not the heap doubles"
        ));
    }
    let t: &'static ProcTables<BsdProcGlue> =
        std::boxed::Box::leak(std::boxed::Box::new(ProcTables::new()));
    TEST_TABLES.with(|c| c.set(t));
}

const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<Timeout>();
    send_sync::<BsdProcessG>();
    send_sync::<BsdThreadG>();
    send_sync::<BsdPgrpG>();
    send_sync::<BsdSessionG>();
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of bsd's side of kernel::proc: the pool items' sizes (vmstat -m), the
    // lookups and counted references, the walks that sleep, the stores' recycling and the
    // session's lookups.

    use core::pin::Pin;
    use std::sync::MutexGuard;
    use std::{assert, assert_eq};

    use super::*;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::systm::{kernel_lock, kernel_unlock};

    fn setup() -> MutexGuard<'static, ()> {
        let guard = setup_real_memory();
        procinit();
        guard
    }

    /// `pr` on `allprocess` and its pid chain (what `fork1` does), under the kernel lock.
    fn link(pr: &'static Process) {
        let mut w = hal::klock::witness();
        let t = proc_tables();
        t.allprocess.insert_head(&mut w, Pin::static_ref(pr));
        t.pidhash(pr.ps_pid)
            .insert_head(&mut w, Pin::static_ref(pr));
    }

    /// `pr` off them again.
    fn unlink(pr: &'static Process) {
        let mut w = hal::klock::witness();
        let t = proc_tables();
        t.allprocess.remove(&mut w, pr);
        t.pidhash(pr.ps_pid).remove(&mut w, pr);
    }

    #[test]
    fn the_pool_items_are_the_measured_sizes() {
        // vmstat -m's item sizes, which userland reads, as each pool reports them (`pr_size`,
        // the size it was made with: a counted pool's item holds the count and the pool's
        // address beside the value, 16 bytes more than the value): pinned at the sizes this
        // shape has (64-bit), so a change that moves one is seen and said in its commit. The
        // thread may move (its machine part and the kernel's words), below a bound. N4.2 moved
        // them from LZ's shape (smoke-diag's table of 2026-10-10, arm64): procpl 632 -> 1072
        // on arm64, 1064 here (the thread's own credential copy and limit cache, the machine
        // part), processpl 1048 -> 1288, pgrppl 48 -> 152 (the pinned lists and the
        // kernel-lock key), sessionpl 144 -> 144, filepl 120 -> 176, ucredpl 104 -> 120,
        // plimitpl 152 -> 160, zombiepl 144 -> 144; on arm64 (the kernel target, measured by
        // a compile-time probe of the values' sizes) the others are as pinned here. N4.3 moved
        // the descriptors into the kernel: filepl 176 -> 184 (kernel::file's File, its object
        // and stats inside, a counted item), fdescpl (LZ's struct filedesc0) -> 520 (the
        // kernel's table, its slots and maps inline), processpl 1288 -> 1296 (ps_fd a set-once
        // counted handle), procpl 1064 -> 1056 (p_fd gone: a thread reads its process's).
        // The user accepted these sizes (N4.2's and N4.3's decision 1, 2026-10-22). N4.5a
        // moved the run queues and the sleep timeout into the kernel: procpl 1056 -> 1128
        // (p_runq a PinEntry and p_sleep_to the kernel's Timeout in kernel::proc's Thread,
        // where bsd's list entry and timeout were), then 1136 (p_cpuno +8, the CPU number
        // beside the slot), pending the user's word as N4.2's were. N4.4 (kqueue into
        // kernel::event) gave processpl 1296 -> 1360 (ps_klist and its kqueue pools in
        // kernel::proc's Process) and fdescpl 520 -> 560; at the merge of N4.4 and N4.5a
        // procpl 1136 -> 1168: p_kq +8 (a set-once counted kqueue in the kernel's Thread where
        // bsd's word was) and p_sleep_to's ToState +24 (N4.4's kernel Timeout state). N4.5b (signals into kernel::sig): processpl 1360 -> 1376 (ps_sigacts a
        // set-once handle of process 0's static or a sigapl box in kernel::proc's Process,
        // where the glue's link was); procpl unchanged (p_sigstk and p_sigval moved from the
        // glue into the thread); sigapl 424 -> 448 (kernel::sig's test pins it). Then the
        // sigio registrations moved into kernel::sigio's registry: processpl 1376 -> 1360 and
        // pgrppl 152 -> 136 (ps_sigiolst and pg_sigiolst gone from the glue's members).
        // N4.7b (the terminal into kernel::tty): sessionpl 144 -> 160, s_ttyp a counted tty
        // handle under the kernel lock in kernel::proc's Session where the glue's link was.
        // N4.6a (the process core into kernel::{fork, exit}): processpl 1360 -> 1536 (+176,
        // the size pinned below; the parts are the fields' layouts): the pin tables are owned
        // tables in ReadMostly cells (2 x 64 where the glue's two `struct pinsyscall`s of
        // words were 2 x 32: +64), ps_rucheck_to is the kernel's Timeout (120 where bsd's was
        // 32: +88), PsMtx gains the main thread's slot (a tag and a pointer, 16) and
        // ps_opptr's counted handle (16): +32, and the glue's ps_mainproc word goes (-8);
        // procpl unchanged (p_exited moved from the glue into the kernel's Thread as
        // `exited`). Accepted under the user's rule for N4's sizes (reported, pinned here).
        let _g = setup();
        crate::kern::kern_descrip::filedesc_init();
        let t = proc_tables();
        let size =
            |s: Option<usize>, name: &str| s.unwrap_or_else(|| std::panic!("{name} is not made"));
        let proc = size(t.procpl.item_size(), "procpl");
        let sizes = [
            ("processpl", size(t.processpl.item_size(), "processpl")),
            ("pgrppl", size(t.pgrppl.item_size(), "pgrppl")),
            ("sessionpl", size(t.sessionpl.item_size(), "sessionpl")),
            (
                "filepl",
                size(
                    crate::kern::kern_descrip::file_tables().filepl.item_size(),
                    "filepl",
                ),
            ),
            (
                "fdescpl",
                size(
                    crate::kern::kern_descrip::file_tables().fdescpl.item_size(),
                    "fdescpl",
                ),
            ),
            ("ucredpl", size(kernel::cred::ucred_item_size(), "ucredpl")),
            (
                "plimitpl",
                size(kernel::resource::plimit_item_size(), "plimitpl"),
            ),
            (
                "zombiepl",
                size(kernel::proc::rusage_item_size(), "zombiepl"),
            ),
        ];
        std::println!("procpl {proc}, {sizes:?}");
        assert!(proc <= 1168, "procpl {proc}");
        assert_eq!(
            sizes,
            [
                ("processpl", 1536),
                ("pgrppl", 136),
                ("sessionpl", 160),
                ("filepl", 184),
                ("fdescpl", 560),
                ("ucredpl", 120),
                ("plimitpl", 160),
                ("zombiepl", 144),
            ]
        );
    }

    #[test]
    fn a_pooled_process_is_found_counted_and_walked() {
        let _g = setup();
        let pr = test_pooled_process(77);
        kernel_lock();
        link(pr);
        let found = prfind_ref(77).expect("on the pid hash");
        assert!(ptr::eq(&*found, pr));
        assert!(prfind_ref(78).is_none());
        assert!(StaticOrPooled::ptr_eq(&process_ref(pr), &found));
        // The walk that may sleep: `work` runs without the reader, `pick` refuses what
        // `work` changed.
        let seen = core::cell::Cell::new(0);
        allprocess_rescan(
            |p| p.ps_pid == 77 && seen.get() == 0,
            |p| {
                assert_eq!(p.ps_pid, 77);
                seen.set(seen.get() + 1);
            },
        );
        assert_eq!(seen.get(), 1);
        drop(found);
        unlink(pr);
        kernel_unlock();
    }

    #[test]
    fn a_session_is_reached_from_its_members() {
        let _g = setup();
        let pr = test_pooled_process(90);
        kernel_lock();
        link(pr);
        kernel_unlock();
        let (pg, sess) = test_setsid(pr);
        kernel_lock();
        assert!(StaticOrPooled::ptr_eq(&pgrp_of(pr).expect("a group"), &pg));
        assert!(StaticOrPooled::ptr_eq(
            &session_of(pr).expect("a session"),
            &sess
        ));
        {
            let r = hal::klock::reader();
            assert!(ptr::eq(session_leader(&r, &sess).expect("its leader"), pr));
            assert!(ptr::eq(pgfind(&r, 90).expect("the group"), &*pg));
            assert!(StaticOrPooled::ptr_eq(&pgrp_ref(&pg), &pg));
        }
        sess.set_leader(None);
        {
            let r = hal::klock::reader();
            assert!(session_leader(&r, &sess).is_none());
        }
        unlink(pr);
        kernel_unlock();
    }

    #[test]
    fn waitid_reports_who_and_as_whom_under_the_child_s_mutex() {
        // dowait6 fills the siginfo_t under the child's ps_mtx (the C's PR_LOCK section):
        // a zombie, a stopped and a continued child, each reported with its pid, its
        // effective uid and its CLD_* code, without taking the mutex a second time
        // (hal's mutex panics "locking against myself").
        use crate::sys::proc::{PS_CONTINUED, PS_STOPPED, PS_ZOMBIE};
        use crate::sys::siginfo::{CLD_CONTINUED, CLD_EXITED, CLD_STOPPED, Siginfo};
        use crate::sys::signal::{SIGCHLD, SIGCONT, SIGSTOP};
        use crate::sys::wait::{P_PID, WCONTINUED, WEXITED, WNOHANG, WNOWAIT, WUNTRACED};
        use kernel::cred::CredRef;
        use kernel::exit::dowait6;

        let _g = setup();
        let parent = test_pooled_process(500);
        let q = test_thread(parent);
        let child = |pid: Pid, uid: u32, flags: u32| {
            let pr = test_pooled_process(pid);
            let mut cr = crate::sys::ucred::Ucred::new();
            cr.cr_uid.set(uid);
            cr.cr_ruid.set(uid);
            drop(pr.set_ucred(CredRef::new(cr)));
            pr.ps_flags
                .fetch_or(flags, core::sync::atomic::Ordering::Relaxed);
            pr
        };
        let zombie = child(501, 1001, PS_ZOMBIE);
        zombie.ps_xexit.set(7);
        let stopped = child(502, 1002, PS_STOPPED);
        stopped.ps_xsig.set(SIGSTOP);
        let continued = child(503, 1003, PS_CONTINUED);
        kernel_lock();
        {
            let mut w = hal::klock::witness();
            for pr in [zombie, stopped, continued] {
                parent.ps_children.insert_head(&mut w, Pin::static_ref(pr));
            }
        }
        let cases = [
            (501, WEXITED, CLD_EXITED, 7),
            (502, WUNTRACED, CLD_STOPPED, SIGSTOP),
            (503, WCONTINUED, CLD_CONTINUED, SIGCONT),
        ];
        for (pid, opt, code, status) in cases {
            let mut info = Siginfo::zeroed();
            let r = dowait6(
                q,
                P_PID,
                i64::from(pid),
                None,
                opt | WNOHANG | WNOWAIT,
                None,
                Some(&mut info),
            );
            assert_eq!(r, Ok(pid), "pid {pid}");
            assert_eq!(info.si_signo, SIGCHLD);
            assert_eq!(info.si_code, code, "pid {pid}");
            assert_eq!(info.si_pid(), pid);
            assert_eq!(info.si_uid(), 1000 + pid as u32 - 500);
            assert_eq!(info.si_status(), status, "pid {pid}");
        }
        {
            let mut w = hal::klock::witness();
            for pr in [zombie, stopped, continued] {
                parent.ps_children.remove(&mut w, pr);
            }
        }
        kernel_unlock();
    }

    #[test]
    fn a_forked_child_follows_its_parent_among_the_group_s_members() {
        // fork1's LIST_INSERT_AFTER(curpr, pr, ps_pglist): after two forks of `parent` in a
        // group where `other` joined later (enterpgrp inserts at the head), the members are
        // other, parent, the second child, the first child: setpriority(PRIO_PGRP) returns
        // the first child's nice (the last member's), ttyinfo breaks ties in that order.
        let _g = setup();
        let parent = test_pooled_process(600);
        let other = test_pooled_process(601);
        let (pg, _sess) = test_setsid(parent);
        kernel_lock();
        {
            let mut w = hal::klock::witness();
            pg.pg_members.insert_head(&mut w, Pin::static_ref(other));
            drop(other.set_pgrp(&mut w, Some(pg.clone())));
        }
        let first = process_ref(test_pooled_process(602));
        let second = process_ref(test_pooled_process(603));
        {
            let mut w = hal::klock::witness();
            kernel::fork::fork_join_pgrp(&mut w, parent, &first);
            kernel::fork::fork_join_pgrp(&mut w, parent, &second);
        }
        {
            let r = hal::klock::reader();
            let pids: std::vec::Vec<Pid> = pg.pg_members.iter(&r).map(|p| p.ps_pid).collect();
            assert_eq!(pids, [601, 600, 603, 602]);
            assert!(StaticOrPooled::ptr_eq(
                first.pgrp(&r).expect("a group"),
                &pg
            ));
            assert!(StaticOrPooled::ptr_eq(
                second.pgrp(&r).expect("a group"),
                &pg
            ));
        }
        // The group stays linked with its members, all leaked, as the tests' processes.
        kernel_unlock();
    }

    #[test]
    fn procinit_gives_each_test_fresh_user_counts() {
        // The uid table lives in the tables proc_test_reset replaces: a count an earlier
        // test left is not seen after the next procinit. The kernel's process-wide pools are
        // kept, which is sound because they are the heap doubles (checked by the reset).
        let _g = setup();
        assert_eq!(chgproccnt(64_010, 2), 2);
        assert_eq!(with_uidinfo(64_010, |u| u.ui_proccnt.get()), 2);
        procinit();
        assert_eq!(chgproccnt(64_010, 1), 1);
        assert_eq!(kernel::testutil::memory_is_the_doubles(), Some(true));
    }
}
/* </TESTS> */
