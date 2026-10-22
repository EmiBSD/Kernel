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
 *	@(#)kern_resource.c	8.5 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! Resource limits and usage: `kern/kern_resource.c`.
//!
//! Upstream: sys/kern/kern_resource.c @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M5 (part b2) ports the thread usage aggregation the scheduler
//! needs: `tuagg_sumup`, `tuagg_get_proc`, `tuagg_get_process`, `tuagg_add_process` and
//! `tuagg_add_runtime`; M6-b adds `calctsru`, `calcru` and `ruadd` for `exit1`. M7a adds the
//! `plimit` management (`lim_startup`, `lim_copy`, `lim_free`, `lim_fork`,
//! `lim_write_begin`/`lim_write_commit`, `lim_read_enter`, `lim_cur_proc`), `rucheck`,
//! `dosetrlimit`, `sys_setrlimit`, `sys_getrlimit`, `sys_getrusage` and `dogetrusage`, and
//! with the credentials the priority syscalls (`sys_getpriority`, `sys_setpriority`,
//! `donice`). The file is complete but for the KTRACE hooks, which are not configured. N4.2:
//! the limits, the usage counters and their arithmetic are the kernel crate's
//! (`kernel::resource`: `Plimit`/`PlimitRef`, `Rusage`, `Tusage`, `tuagg_sumup`, `calctsru`,
//! `calcru`, `ruadd`, `lim_startup`, `lim_copy`, `lim_free`), re-exported here.
//!
//! ## Deviations
//! - `struct plimit` is plain data shared through counted `PlimitRef`s, never written once
//!   placed: `lim_write_begin` hands out a copy (always: a placed structure is never
//!   writable, where the C writes in place when it is not shared) and `lim_write_commit`
//!   places it. A thread's read reference `p_limit` is its own copy of the values
//!   (`kernel::resource::LimitCache`), refreshed by `lim_read_enter` when the process's
//!   limits changed.
//! - The walks over a process's threads (`tuagg_get_process`, `dogetrusage`, `rucheck`,
//!   `donice`) hold its `ps_mtx`, whose guard the thread list asks for; the C's
//!   `dogetrusage` walks it under the kernel lock alone.
//!   KTRACE is not configured.

use core::ffi::c_void;
use core::sync::atomic::Ordering;

use hal::klock;

use crate::conf::param::{MAXFILES, MAXPROCESS};
use crate::kassert;
use crate::kern::kern_clock::STATHZ;
use crate::kern::kern_prot::suser;
use crate::kern::kern_rwlock::{rw_assert_wrlock, rw_enter_write, rw_exit_write};
use crate::kern::kern_sig::prsignal;
use crate::kern::kern_tc::nanouptime;
use crate::kern::kern_timeout::timeout_add_msec;
use crate::kern::proc_glue::{pgfind, prfind, proc_tables};
use crate::kern::sched_bsd::{sched_lock, sched_unlock, setpriority};
use crate::machine::Machine;
use crate::machine::copy::{copyin, copyout};
use crate::machine::cpu::{Cpu, curcpu, curproc};
use crate::machine::{Machine as Md, VmParam};
use crate::sys::errno::Errno;
use crate::sys::mman::{PROT_NONE, PROT_READ, PROT_WRITE};
use crate::sys::param::NZERO;
use crate::sys::proc::{Proc, ProcExt, Process, SDEAD, Tusage, tu_enter, tu_leave};
use crate::sys::resource::{PRIO_MAX, PRIO_MIN, PRIO_PGRP, PRIO_PROCESS, PRIO_USER};
use crate::sys::resource::{
    RLIM_INFINITY, RLIM_NLIMITS, RLIMIT_CPU, RLIMIT_DATA, RLIMIT_NOFILE, RLIMIT_NPROC,
    RLIMIT_STACK, RUSAGE_CHILDREN, RUSAGE_SELF, RUSAGE_THREAD, Rlimit, Rusage,
};
use crate::sys::resourcevar::{LimitCache, Plimit, PlimitRef, lim_read_leave};
use crate::sys::rwlock::Rwlock;
use crate::sys::signal::{SIGKILL, SIGXCPU};
use crate::sys::syscallargs::{
    SysGetpriorityArgs, SysGetrlimitArgs, SysGetrusageArgs, SysSetpriorityArgs, SysSetrlimitArgs,
};
use crate::sys::systm::{SysArgs, kernel_assert_locked, sysargs};
use crate::sys::time::{Timespec, timespecsub};
use crate::sys::types::Pid;
use crate::sys::types::{Register, Rlim};
use hal::vm::param::{round_page, trunc_page};
use kernel::vm::map::UVM_ET_STACK;
use kernel::vm::map::uvm_map_protect;

pub use kernel::resource::{lim_copy, lim_free, ruadd, tuagg_sumup};

/// `tuagg_get_proc`: the usage of one thread.
pub fn tuagg_get_proc(tu: &Tusage, p: &Proc) {
    tu.clear();
    tuagg_sumup(tu, &p.p_tu);
}

/// `tuagg_get_process`: the usage of a process: its own plus all living threads.
pub fn tuagg_get_process(tu: &Tusage, pr: &Process) {
    tu.clear();

    let g = pr.ps_mtx.lock();
    tuagg_sumup(tu, &pr.ps_tu);
    // add on all living threads
    for thread in pr.ps_threads.iter(&g) {
        tuagg_sumup(tu, &thread.p_tu);
    }
    drop(g);
}

/// `tuagg_add_process`: update the process `ps_tu` usage with the values from proc `p`
/// while doing so the times for proc `p` are reset. This requires that `p` is either
/// `curproc` or `SDEAD` and that the IPL is higher than `IPL_STATCLOCK`. `ps_mtx` uses
/// `IPL_HIGH` so this should always be the case.
pub fn tuagg_add_process(pr: &Process, p: &Proc) {
    kassert!(pr.ps_mtx.owned());
    kassert!(curproc().is_some_and(|cur| core::ptr::eq(cur, p)) || p.stat() == SDEAD);

    let generation = tu_enter(&pr.ps_tu);
    tuagg_sumup(&pr.ps_tu, &p.p_tu);
    tu_leave(&pr.ps_tu, generation);

    // Now reset CPU time usage for the thread.
    p.p_tu.clear();
}

/// `tuagg_add_runtime`: compute the amount of time during which the current process was
/// running, and add that to its total so far.
pub fn tuagg_add_runtime() {
    let spc = Machine::ci_schedstate(curcpu());
    let Some(p) = curproc() else {
        return;
    };

    let ts = nanouptime();
    let delta = if ts < spc.spc_runtime.get() {
        // uptime is not monotonic (the C prints this under #if 0)
        Timespec::new(0, 0)
    } else {
        timespecsub(&ts, &spc.spc_runtime.get())
    };
    // update spc_runtime
    spc.spc_runtime.set(ts);
    p.p_tu.add_runtime(delta.into());
}

/// `calctsru`: transform the running time and tick information in a struct tusage into
/// user, system, and interrupt time usage, at the statistics clock's rate.
pub fn calctsru(tup: &Tusage) -> (Timespec, Timespec, Timespec) {
    let (u, s, i) = kernel::resource::calctsru(tup, STATHZ.load(Ordering::Relaxed));
    (u.into(), s.into(), i.into())
}

/// `calcru`: `calctsru` in `timeval`s: (user, system, interrupt), as `struct rusage` keeps
/// them.
pub fn calcru(
    tup: &Tusage,
) -> (
    kernel::resource::Timeval,
    kernel::resource::Timeval,
    kernel::resource::Timeval,
) {
    kernel::resource::calcru(tup, STATHZ.load(Ordering::Relaxed))
}

/// `RUCHECK_INTERVAL`: resource usage check interval in msec.
const RUCHECK_INTERVAL: u64 = 1000;

/// `SIGXCPU_INTERVAL`: SIGXCPU interval in seconds of process runtime.
const SIGXCPU_INTERVAL: i64 = 5;

/// `maxdmap`: patchable maximum data limit.
pub static MAXDMAP: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(<Md as VmParam>::MAXDSIZ as u64);
/// `maxsmap`: patchable maximum stack limit.
pub static MAXSMAP: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(<Md as VmParam>::MAXSSIZ as u64);

/// `rlimit_lock`: serializes resource limit updates. This lock has to be held together with
/// `ps_mtx` when updating the process' `ps_limit`.
pub static RLIMIT_LOCK: Rwlock = Rwlock::new("rlimitlk");

/// `sys_getpriority`.
pub fn sys_getpriority(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetpriorityArgs = sysargs(v);
    let mut low = NZERO + PRIO_MAX + 1;
    let who = uap.who.get();
    let r = klock::reader();

    match uap.which.get() {
        PRIO_PROCESS => {
            let pr = if who == 0 {
                Some(curp.process())
            } else {
                prfind(&r, who as Pid)
            };
            if let Some(pr) = pr
                && i32::from(pr.ps_nice.get()) < low
            {
                low = i32::from(pr.ps_nice.get());
            }
        }

        PRIO_PGRP => {
            let pg = if who == 0 {
                curp.process().pgrp(&r).map(|pg| &**pg)
            } else {
                pgfind(&r, who as Pid)
            };
            if let Some(pg) = pg {
                for pr in pg.pg_members.iter(&r) {
                    if i32::from(pr.ps_nice.get()) < low {
                        low = i32::from(pr.ps_nice.get());
                    }
                }
            }
        }

        PRIO_USER => {
            let who = if who == 0 {
                curp.ucred().cr_uid.get()
            } else {
                who
            };
            for pr in proc_tables().allprocess.iter(&r) {
                if pr.ucred().cr_uid.get() == who && i32::from(pr.ps_nice.get()) < low {
                    low = i32::from(pr.ps_nice.get());
                }
            }
        }

        _ => return Err(Errno::EINVAL),
    }
    if low == NZERO + PRIO_MAX + 1 {
        return Err(Errno::ESRCH);
    }
    retval[0] = (low - NZERO) as Register;
    Ok(())
}

/// `sys_setpriority`.
pub fn sys_setpriority(curp: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetpriorityArgs = sysargs(v);
    let who = uap.who.get();
    let prio = uap.prio.get();
    let mut found = false;
    let mut error = Ok(());
    let r = klock::reader();

    match uap.which.get() {
        PRIO_PROCESS => {
            let pr = if who == 0 {
                Some(curp.process())
            } else {
                prfind(&r, who as Pid)
            };
            if let Some(pr) = pr {
                error = donice(curp, pr, prio);
                found = true;
            }
        }

        PRIO_PGRP => {
            let pg = if who == 0 {
                curp.process().pgrp(&r).map(|pg| &**pg)
            } else {
                pgfind(&r, who as Pid)
            };
            if let Some(pg) = pg {
                for pr in pg.pg_members.iter(&r) {
                    error = donice(curp, pr, prio);
                    found = true;
                }
            }
        }

        PRIO_USER => {
            let who = if who == 0 {
                curp.ucred().cr_uid.get()
            } else {
                who
            };
            for pr in proc_tables().allprocess.iter(&r) {
                if pr.ucred().cr_uid.get() == who {
                    error = donice(curp, pr, prio);
                    found = true;
                }
            }
        }

        _ => return Err(Errno::EINVAL),
    }
    if !found {
        return Err(Errno::ESRCH);
    }
    error
}

/// `donice`: sets the nice value of `chgpr` to `n` on behalf of `curp`.
pub fn donice(curp: &Proc, chgpr: &Process, n: i32) -> Result<(), Errno> {
    let ucred = curp.ucred();
    let chguid = chgpr.ucred().cr_uid.get();

    if ucred.cr_uid.get() != 0
        && ucred.cr_ruid.get() != 0
        && ucred.cr_uid.get() != chguid
        && ucred.cr_ruid.get() != chguid
    {
        return Err(Errno::EPERM);
    }
    let n = n.clamp(PRIO_MIN, PRIO_MAX) + NZERO;
    if n < i32::from(chgpr.ps_nice.get()) && suser(curp).is_err() {
        return Err(Errno::EACCES);
    }
    chgpr.ps_nice.set(n as u8);
    let g = chgpr.ps_mtx.lock();
    sched_lock();
    for p in chgpr.ps_threads.iter(&g) {
        setpriority(p, p.p_estcpu.get(), n as u8);
    }
    sched_unlock();
    drop(g);
    Ok(())
}

/// `sys_setrlimit`.
pub fn sys_setrlimit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetrlimitArgs = sysargs(v);
    let mut buf = [0u8; size_of::<Rlimit>()];
    copyin(uap.rlp.get() as usize, &mut buf)?;
    let mut alim = rlimit_from_bytes(&buf);
    // KTRACE: not configured.
    dosetrlimit(p, uap.which.get() as u32, &mut alim)
}

/// The process's limits (`pr->ps_limit`), which every process has from `fork1` (`limit0`
/// for process 0).
fn ps_limit(pr: &Process) -> PlimitRef {
    match pr.limit() {
        Some(l) => l,
        None => crate::kern::subr_prf::panic(format_args!("ps_limit: none")),
    }
}

/// `dosetrlimit`: sets limit `which` of `p`'s process to `limp`, clamped to the system's
/// maximum.
pub fn dosetrlimit(p: &Proc, which: u32, limp: &mut Rlimit) -> Result<(), Errno> {
    let which = which as usize;
    if which >= RLIM_NLIMITS || limp.rlim_cur > limp.rlim_max {
        return Err(Errno::EINVAL);
    }

    rw_enter_write(&RLIMIT_LOCK);

    let alimp = ps_limit(p.process()).pl_rlimit[which];
    if limp.rlim_max > alimp.rlim_max
        && let Err(error) = suser(p)
    {
        rw_exit_write(&RLIMIT_LOCK);
        return Err(error);
    }

    // Get exclusive write access to the limit structure.
    let mut limit = lim_write_begin();
    let alimp = limit.pl_rlimit[which];

    let maxlim: Rlim = match which {
        RLIMIT_DATA => MAXDMAP.load(Ordering::Relaxed),
        RLIMIT_STACK => MAXSMAP.load(Ordering::Relaxed),
        RLIMIT_NOFILE => MAXFILES.load(Ordering::Relaxed) as Rlim,
        RLIMIT_NPROC => MAXPROCESS.load(Ordering::Relaxed) as Rlim,
        _ => RLIM_INFINITY,
    };

    if limp.rlim_max > maxlim {
        limp.rlim_max = maxlim;
    }
    if limp.rlim_cur > limp.rlim_max {
        limp.rlim_cur = limp.rlim_max;
    }

    if which == RLIMIT_CPU && limp.rlim_cur != RLIM_INFINITY && alimp.rlim_cur == RLIM_INFINITY {
        timeout_add_msec(&p.process().ps_rucheck_to, RUCHECK_INTERVAL);
    }

    if which == RLIMIT_STACK {
        // Stack is allocated to the max at exec time with only "rlim_cur" bytes accessible.
        // If stack limit is going up make more accessible, if going down make inaccessible.
        if limp.rlim_cur != alimp.rlim_cur {
            let vm = p.vmspace();
            // MACHINE_STACK_GROWS_UP: neither amd64 nor arm64.
            let (prot, size, addr) = if limp.rlim_cur > alimp.rlim_cur {
                (
                    PROT_READ | PROT_WRITE,
                    (limp.rlim_cur - alimp.rlim_cur) as usize,
                    vm.vm_minsaddr.load(Ordering::Relaxed) - limp.rlim_cur as usize,
                )
            } else {
                (
                    PROT_NONE,
                    (alimp.rlim_cur - limp.rlim_cur) as usize,
                    vm.vm_minsaddr.load(Ordering::Relaxed) - alimp.rlim_cur as usize,
                )
            };
            let addr = trunc_page(addr);
            let size = round_page(size);
            let _ = uvm_map_protect(
                &vm.vm_map,
                addr,
                addr + size,
                prot,
                UVM_ET_STACK,
                false,
                false,
            );
        }
    }

    limit.pl_rlimit[which] = *limp;

    lim_write_commit(limit);
    rw_exit_write(&RLIMIT_LOCK);

    Ok(())
}

/// `sys_getrlimit`.
pub fn sys_getrlimit(_p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetrlimitArgs = sysargs(v);
    let which = uap.which.get();
    if which < 0 || which as usize >= RLIM_NLIMITS {
        return Err(Errno::EINVAL);
    }
    let limit = lim_read_enter();
    let alimp = limit.get(which as usize);
    lim_read_leave(limit);
    // KTRACE: not configured.
    copyout(&rlimit_to_bytes(&alimp), uap.rlp.get() as usize)
}

/// `struct rlimit` as the user sees it: `rlim_cur` then `rlim_max`, native endian.
fn rlimit_to_bytes(r: &Rlimit) -> [u8; size_of::<Rlimit>()] {
    let mut b = [0u8; size_of::<Rlimit>()];
    b[..8].copy_from_slice(&r.rlim_cur.to_ne_bytes());
    b[8..].copy_from_slice(&r.rlim_max.to_ne_bytes());
    b
}

/// The inverse of [`rlimit_to_bytes`].
fn rlimit_from_bytes(b: &[u8; size_of::<Rlimit>()]) -> Rlimit {
    let mut cur = [0u8; 8];
    let mut max = [0u8; 8];
    cur.copy_from_slice(&b[..8]);
    max.copy_from_slice(&b[8..]);
    Rlimit {
        rlim_cur: Rlim::from_ne_bytes(cur),
        rlim_max: Rlim::from_ne_bytes(max),
    }
}

/// `sys_getrusage`.
pub fn sys_getrusage(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetrusageArgs = sysargs(v);
    let ru = Rusage::default();
    dogetrusage(p, uap.who.get(), &ru)?;
    // KTRACE: not configured.
    copyout(&ru.to_bytes(), uap.rusage.get() as usize)
}

/// `dogetrusage`: the resource usage `who` (`RUSAGE_SELF`, `RUSAGE_THREAD`,
/// `RUSAGE_CHILDREN`) of `p`, into `rup`.
pub fn dogetrusage(p: &Proc, who: i32, rup: &Rusage) -> Result<(), Errno> {
    let pr = p.process();
    let tu = Tusage::new();

    kernel_assert_locked(); // KERNEL_ASSERT_LOCKED()

    match who {
        RUSAGE_SELF => {
            // start with the sum of dead threads, if any
            {
                let r = klock::reader();
                match pr.ps_ru.get(&r) {
                    Some(ru) => rup.copy_from(ru),
                    None => rup.copy_from(&Rusage::default()),
                }
            }
            tuagg_sumup(&tu, &pr.ps_tu);

            // add on all living threads
            let g = pr.ps_mtx.lock();
            for thread in pr.ps_threads.iter(&g) {
                ruadd(rup, &thread.p_ru);
                tuagg_sumup(&tu, &thread.p_tu);
            }
            drop(g);

            let (ut, st, _) = calcru(&tu);
            rup.ru_utime.set(ut);
            rup.ru_stime.set(st);

            rup.ru_ixrss.set(tu.tu_ixrss.get() as i64);
            rup.ru_idrss.set(tu.tu_idrss.get() as i64);
            rup.ru_isrss.set(tu.tu_isrss.get() as i64);
        }
        RUSAGE_THREAD => {
            rup.copy_from(&p.p_ru);
            let (ut, st, _) = calcru(&p.p_tu);
            rup.ru_utime.set(ut);
            rup.ru_stime.set(st);
        }
        RUSAGE_CHILDREN => rup.copy_from(&pr.ps_cru),
        _ => return Err(Errno::EINVAL),
    }
    Ok(())
}

/// `rucheck`: check if the process exceeds its cpu resource allocation. If over max, kill
/// it.
pub fn rucheck(arg: *mut c_void) {
    // SAFETY: the timeout's argument is the process (`process_initialize`), which outlives
    // its timeouts (they are deleted in `exit1`).
    let pr = unsafe { &*(arg as *const Process) };
    let tu = Tusage::new();

    kernel_assert_locked(); // KERNEL_ASSERT_LOCKED()

    let g = pr.ps_mtx.lock();
    let rlim = g
        .limit
        .as_ref()
        .map_or(Rlimit::default(), |l| l.pl_rlimit[RLIMIT_CPU]);
    tuagg_sumup(&tu, &pr.ps_tu);
    for thread in pr.ps_threads.iter(&g) {
        tuagg_sumup(&tu, &thread.p_tu);
    }
    drop(g);

    let runtime = tu.tu_runtime.get().tv_sec;

    if runtime as Rlim >= rlim.rlim_cur {
        if runtime as Rlim >= rlim.rlim_max {
            prsignal(pr, SIGKILL);
        } else if runtime >= pr.ps_nextxcpu.get() {
            prsignal(pr, SIGXCPU);
            pr.ps_nextxcpu.set(runtime + SIGXCPU_INTERVAL);
        }
    }

    timeout_add_msec(&pr.ps_rucheck_to, RUCHECK_INTERVAL);
}

/// `lim_startup`: the pool (the kernel crate's, made at its first need), and the limits of
/// process 0 into `limit0`, which the caller places.
pub fn lim_startup(limit0: &mut Plimit) {
    let maxfiles = MAXFILES.load(Ordering::Relaxed) as usize;
    let free =
        hal::vm::param::ptoa(hal::vm::uvmexp::UVMEXP.free.load(Ordering::Relaxed) as usize) as Rlim;
    *limit0 = kernel::resource::lim_startup(maxfiles, free, crate::sys::param::PAGE_SIZE);
}

/// `lim_fork`: `child` shares `parent`'s limits.
pub fn lim_fork(parent: &Process, child: &Process) {
    let limit = ps_limit(parent);
    let cpu = limit.pl_rlimit[RLIMIT_CPU].rlim_cur;
    drop(child.set_limit(limit));

    if cpu != RLIM_INFINITY {
        timeout_add_msec(&child.ps_rucheck_to, RUCHECK_INTERVAL);
    }
}

/// `lim_write_begin`: an exclusive copy of the process' resource limits to change; the
/// caller has to place it by calling `lim_write_commit()` (see the module's deviations).
///
/// This invalidates any plimit read reference held by the calling thread.
pub fn lim_write_begin() -> Plimit {
    let p = curproc_ref();

    rw_assert_wrlock(&RLIMIT_LOCK);

    p.p_limit.clear();

    // It is safe to access ps_limit here without holding ps_mtx because rlimit_lock excludes
    // other writers.
    *ps_limit(p.process())
}

/// `lim_write_commit`: finish exclusive write access to the plimit structure. This makes the
/// structure visible to other threads in the process.
pub fn lim_write_commit(limit: Plimit) {
    let p = curproc_ref();

    rw_assert_wrlock(&RLIMIT_LOCK);

    let pr = p.process();
    if *ps_limit(pr) != limit {
        let olimit = pr.set_limit(PlimitRef::new(limit));
        drop(olimit);
    }
}

/// `lim_read_enter`: begin read access to the process' resource limits: the calling
/// thread's copy, refreshed when the process's changed. The access has to be finished by
/// calling `lim_read_leave()`.
///
/// Sections denoted by `lim_read_enter()` and `lim_read_leave()` cannot nest.
pub fn lim_read_enter() -> &'static LimitCache {
    let p = curproc_ref();

    // This thread might not observe the latest value of ps_limit if another thread updated
    // the limits very recently on another CPU. However, the anomaly should disappear quickly,
    // especially if there is any synchronization activity between the threads (or the CPUs).
    let _ = p.lim_cur(RLIMIT_CPU);
    &p.p_limit
}

/// `lim_cur_proc`: get the value of the resource limit in given process.
pub fn lim_cur_proc(p: &Proc, which: usize) -> Rlim {
    kassert!(which < RLIM_NLIMITS);

    ps_limit(p.process()).pl_rlimit[which].rlim_cur
}

/// `curproc`, which a limit operation always has.
fn curproc_ref() -> &'static Proc {
    match curproc() {
        Some(p) => p,
        None => crate::kern::subr_prf::panic(format_args!("kern_resource: no curproc")),
    }
}
/* </CODE> */
