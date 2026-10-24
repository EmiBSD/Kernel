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
//! Resource limits and usage: `kern/kern_resource.c`, `bsd`'s `sysent[]` rows and names
//! over `kernel::resource`.
//!
//! Upstream: sys/kern/kern_resource.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_resource.rs@3c62ede68802
//!
//! An adapter (N4.6a): the limits, the usage counters and their arithmetic (N4.2) and the
//! functions over a process and the five system calls (N4.6a) are the kernel crate's
//! (`kernel::resource`). What stays here: the rows of `sysent[]` (`getpriority`,
//! `setpriority`, `setrlimit`, `getrlimit`, `getrusage`, which decode their arguments for
//! the kernel's), `tuagg_add_runtime` over the CPU's `spc_runtime` (the clock's, N4.8a),
//! `calctsru`/`calcru` at the statistics clock's rate, `lim_startup` with the system's
//! `maxfiles` and free memory, and `lim_read_enter` of `curproc`.
//!
//! ## Deviations
//! - `struct plimit` is plain data shared through counted `PlimitRef`s, never written once
//!   placed (`kernel::resource`'s deviations). A thread's read reference `p_limit` is its own
//!   copy of the values (`kernel::resource::LimitCache`), refreshed by `lim_read_enter` when
//!   the process's limits changed.
//!
//! ## Redesign
//! - N4.6a: `dosetrlimit`, `dogetrusage`, `donice`, `rucheck`, `lim_fork`,
//!   `lim_write_begin`/`lim_write_commit`, `lim_cur_proc`, `tuagg_get_proc`,
//!   `tuagg_get_process`, `tuagg_add_process` and the system calls' bodies moved to
//!   `kernel::resource`; `rucheck`'s timeout names its process by pid (no cast of a `void *`
//!   back to a process).

use core::sync::atomic::Ordering;

use hal::user::UserPtr;

use crate::conf::param::MAXFILES;
use crate::kern::kern_clock::STATHZ;
use crate::kern::kern_tc::nanouptime;
use crate::machine::Machine;
use crate::machine::cpu::{Cpu, curcpu, curproc};
use crate::sys::errno::Errno;
use crate::sys::proc::{Proc, Tusage};
use crate::sys::resource::RLIMIT_CPU;
use crate::sys::resourcevar::{LimitCache, Plimit};
use crate::sys::syscallargs::{
    SysGetpriorityArgs, SysGetrlimitArgs, SysGetrusageArgs, SysSetpriorityArgs, SysSetrlimitArgs,
};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::time::{Timespec, timespecsub};
use crate::sys::types::{Register, Rlim};

pub use kernel::resource::{
    MAXDMAP, MAXSMAP, dogetrusage, donice, dosetrlimit, lim_copy, lim_cur_proc, lim_fork, lim_free,
    ruadd, tuagg_add_process, tuagg_get_proc, tuagg_get_process, tuagg_sumup,
};

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
    p.p_tu.add_runtime(delta);
}

/// `calctsru`: transform the running time and tick information in a struct tusage into
/// user, system, and interrupt time usage, at the statistics clock's rate.
pub fn calctsru(tup: &Tusage) -> (Timespec, Timespec, Timespec) {
    let (u, s, i) = kernel::resource::calctsru(tup, STATHZ.load(Ordering::Relaxed));
    (u, s, i)
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

/// `sys_getpriority`.
pub fn sys_getpriority(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetpriorityArgs = sysargs(v);
    retval[0] =
        kernel::resource::sys_getpriority(curp, uap.which.get(), uap.who.get())? as Register;
    Ok(())
}

/// `sys_setpriority`.
pub fn sys_setpriority(curp: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetpriorityArgs = sysargs(v);
    kernel::resource::sys_setpriority(curp, uap.which.get(), uap.who.get(), uap.prio.get())
}

/// `sys_setrlimit`.
pub fn sys_setrlimit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetrlimitArgs = sysargs(v);
    kernel::resource::sys_setrlimit(
        p,
        uap.which.get() as u32,
        UserPtr::new(uap.rlp.get() as usize),
    )
}

/// `sys_getrlimit`.
pub fn sys_getrlimit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetrlimitArgs = sysargs(v);
    kernel::resource::sys_getrlimit(p, uap.which.get(), UserPtr::new(uap.rlp.get() as usize))
}

/// `sys_getrusage`.
pub fn sys_getrusage(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetrusageArgs = sysargs(v);
    kernel::resource::sys_getrusage(p, uap.who.get(), UserPtr::new(uap.rusage.get() as usize))
}

/// `lim_startup`: the pool (the kernel crate's, made at its first need), and the limits of
/// process 0 into `limit0`, which the caller places.
pub fn lim_startup(limit0: &mut Plimit) {
    let maxfiles = MAXFILES.load(Ordering::Relaxed) as usize;
    let free =
        hal::vm::param::ptoa(hal::vm::uvmexp::UVMEXP.free.load(Ordering::Relaxed) as usize) as Rlim;
    *limit0 = kernel::resource::lim_startup(maxfiles, free, crate::sys::param::PAGE_SIZE);
}

/// `lim_read_enter`: begin read access to the process' resource limits: the calling
/// thread's copy, refreshed when the process's changed. The access has to be finished by
/// calling `lim_read_leave()`.
///
/// Sections denoted by `lim_read_enter()` and `lim_read_leave()` cannot nest.
pub fn lim_read_enter() -> &'static LimitCache {
    let Some(p) = curproc() else {
        crate::kern::subr_prf::panic(format_args!("kern_resource: no curproc"));
    };

    // This thread might not observe the latest value of ps_limit if another thread updated
    // the limits very recently on another CPU. However, the anomaly should disappear quickly,
    // especially if there is any synchronization activity between the threads (or the CPUs).
    let _ = p.lim_cur(RLIMIT_CPU);
    &p.p_limit
}
/* </CODE> */
