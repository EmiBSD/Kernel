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
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Mike Karels at Berkeley Software Design, Inc.
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
 *	@(#)kern_sysctl.c	8.4 (Berkeley) 4/14/94
 */
/* </LICENSES> */

/* <CODE> */
//! The sysctl tree's shape (`kern/kern_sysctl.c`): the [`SysctlNode`] a subsystem's
//! `*_sysctl` function is (the C's hand-written dispatch by `name[0]`), the request it
//! answers ([`SysctlReq`]) and the helpers every node is written with (`sysctl_int`,
//! `sysctl_rdint`, `sysctl_string`, `sysctl_rdstruct`, `sysctl_bounded_arr`, ...); the
//! `sysctl(2)` system call ([`sys_sysctl`]) with `sysctl_vslock`, and the `kern` and `hw`
//! trees: the nodes whose state is the kernel's (`kern.proc`, `kern.proc_args`,
//! `kern.proc_cwd`, `kern.securelevel`, `kern.hostname`, `kern.forkstat`, `hw.disknames`,
//! `hw.diskstats`, `hw.cpuspeed`, `hw.battery`, `hw.vendor`, ...) are served here, the others
//! by the [`SysctlNode`]s their subsystems register.
//!
//! Upstream: sys/kern/kern_sysctl.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_sysctl.rs@3c62ede68802
//!
//! ## Redesign
//! - Decision 46 (BRIEF-N4.7-AMEND-46, decision C): each `*_sysctl` the C dispatches by hand
//!   is a `SysctlNode` (`sysctl_tty` is `kernel::tty::TtyNode`), whose `sysctl` takes the
//!   name below the node and the request. The C's `oldp`/`oldlenp`/`newp`/`newlen` are a
//!   [`SysctlReq`] (moved here from `kernel::vm::meter`, which re-exports it): `*oldlenp` is
//!   in and out (a size query passes `oldp` 0 and gets the length back). No node of `kern`
//!   takes the process.
//! - The copy-out is `hal::user::UserPtr`'s (the `UserCopy` bridge), not `bsd`'s `copyout`.
//! - N4.6b: the dispatch of `sys_sysctl`, `kern_sysctl` and `hw_sysctl` is the C's `switch`es
//!   over a registry: the kernel serves the ids whose state is its own and a subsystem still
//!   in `bsd` registers a [`SysctlNode`] for each of its ids ([`register_top`],
//!   [`register_kern`], [`register_hw`]) and its `sysctl_bounded_arr` variables
//!   ([`register_kern_vars`], [`register_hw_vars`]); one `bridges.toml` row (`SysctlNode`,
//!   paid when the last subsystem moves in). An id nobody registered answers as the C does
//!   for a case its configuration compiled out (`EOPNOTSUPP` at the top and for a leaf,
//!   `ENOTDIR` for a `kern` directory), never a panic.
//! - The vslock rule is the dispatcher's data, not a node's property: which top-level trees
//!   are vslocked (`vm`, `vfs`, `machdep`, `ddb`; not `kern`, `hw`, `net`), which `kern`
//!   leaves and directories are served without the lock and which inside it (`kern_sysctl`
//!   and `kern_sysctl_dirs` versus `kern_sysctl_locked` and `kern_sysctl_dirs_locked`), which
//!   `hw` leaves take it. The error order is the C's: an unknown top-level id is `EOPNOTSUPP`
//!   before `oldlenp` is read; a `kern` directory outside the unlocked set is vslocked before
//!   it answers `ENOTDIR`.
//! - `sysctl_vslock`/`sysctl_vsunlock` are one value, [`VsLock`]: `sysctl_lock` taken for
//!   writing (its witness), the kernel lock and the wired buffer; its drop is
//!   `sysctl_vsunlock` with the length saved at lock time, on every path (the C's error paths
//!   call it by hand).
//! - `hostname` and `domainname` are `Rwlocked` buffers (the C's `sysctl_lock` alone guards
//!   them; a reader outside `sysctl(2)`, `gethostname`'s NFS and ISO 9660 users, takes the
//!   read side); `disknames` and `diskstats` are `MArray`s owned by the `Rwlocked` state of
//!   `sysctl_disklock` (LZ: two `AtomicPtr`s and their lengths, dereferenced in `unsafe`
//!   blocks), filled through the [`DiskExport`] bridge (the disk list is `bsd`'s, N7).
//! - `hw_vendor`, `hw_prod`, `hw_uuid`, `hw_serial`, `hw_ver` are [`HwString`]s (a `OnceStatic`) the machine
//!   sets once at attach (LZ: `StaticCell`s read in `unsafe`); `cpu_cpuspeed` is
//!   [`register_cpuspeed`]'s; the three battery setters are one [`BatteryCharge`] trait over
//!   the knob (decision 46: the C's three entries one driver fills).
//! - `fill_kproc` reads the kernel's process, thread, group, session and credentials; what
//!   `bsd` still owns comes through the glue: its vnode, the unveil state and a CPU's unit
//!   number. The terminal of a session is the kernel's (`s_ttyp`, `kernel::tty`, N4.7b),
//!   read through [`ProcGlue::session_tty`](crate::proc::ProcGlue::session_tty)'s default
//!   (a test glue stands in for a terminal it does not make). `ps_mainproc` is the
//!   process's main-thread slot.
//! - `kern.ttycount` is the kernel's (`kernel::tty::tty_count`, N4.7b) and `kern.tty` is
//!   `kernel::tty::TtyNode`, which `bsd` registers over its glue.
//! - `p_tsess` (`PTRTOINT64(tp->t_session)`) is written only when `show_pointers` (the
//!   caller is root), as the C: LZ wrote it for every caller, a kernel address `ps(1)` gave
//!   any user (N4.7 decision 8, the user's).
//! - `sysctl_proc_args` reads the victim through `uvm_io` into an `M_TEMP` `MArray` (LZ: a
//!   raw `malloc` and a slice over it in `unsafe`), the uio a [`UioMove`](crate::uio::UioMove)
//!   over that buffer; `sysctl_proc_cwd` builds the path through the descriptor glue
//!   (`FileGlue::getcwd`, `vfs_getcwd_common`, N5).
//!
//! ## Deviations
//! - Every node whose subsystem is not ported reports itself with `unported!` and fails with
//!   `ENOSYS`: `kern.proc_vmmap` after its checks (`fill_vmmap`); `hw.model` (`cpu_model`),
//!   `hw.setperf`/`hw.perfpolicy` (`sched_bsd.c`). `kern.proc_cwd` of a process without a
//!   current directory (none has one before a root file system is mounted) is `ENOENT`.
//! - Options this kernel does not configure are compiled out as in C: `DEBUG_SYSCTL`
//!   (`CTL_DEBUG` is `EOPNOTSUPP`), `SYSVMSG`/`SYSVSEM`/`SYSVSHM` (0), `NDT` (`kern.allowdt`),
//!   `GPROF`, `WITNESS`, `PTRACE` (`kern.global_ptrace`), `KTRACE` (the trace members of
//!   `kinfo_proc` stay zero). `SMALL_KERNEL` is not set.
//! - N4.2 (`kernel::proc`): `sysctl_doproc` reads a process's threads one at a time, by
//!   position, each under the process's `ps_mtx` (dropped for the copyout), where the C
//!   walks `ps_threads` under the kernel lock alone; a thread that joins or leaves between
//!   two reads shifts the positions, as one that exits during the C's copyout sleep would.
//! - `disknames`/`diskstats` are read holding `sysctl_disklock` for reading (the C reads them
//!   after `sysctl_diskinit` let go of it, under the kernel lock); `hw.diskstats` copies at
//!   most the bytes allocated; `sysctl_diskinit` fills at most as many entries as it
//!   allocated (the C trusts `disk_count` to match the list).
//! - `KERN_CPTIME` with no CPU online (which only a host double can produce) leaves the sums
//!   at zero instead of dividing by zero (`bsd`'s node); an empty name (never from
//!   `sys_sysctl`, which wants two components) is `EINVAL`.

use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::bridges::AlreadyRegistered;
use hal::klock::{self, KernelReader};
use hal::malloc::{M_WAITOK, MArray};
use hal::mutex::MutexedGuard;
use hal::once::OnceStatic;
use hal::rwlock::{RW_INTR, RawRwlock, Rwlocked, WriteWitness};
use hal::sys::errno::Errno;
use hal::sys::mman::{PROT_READ, PROT_WRITE};
use hal::sys::pledge::PLEDGE_UNVEIL;
use hal::sys::sysctl::{
    CTL_DDB, CTL_HW, CTL_KERN, CTL_MACHDEP, CTL_MAXID, CTL_MAXNAME, CTL_NET, CTL_VFS, CTL_VM,
    EPROC_CTTY, EPROC_LKUNVEIL, EPROC_UNVEIL, HW_ALLOWPOWERDOWN, HW_BATTERY, HW_BATTERY_CHARGEMODE,
    HW_BATTERY_CHARGESTART, HW_BATTERY_CHARGESTOP, HW_BLOCKCPU, HW_BYTEORDER, HW_CPUSPEED,
    HW_DISKNAMES, HW_DISKSTATS, HW_MAXID, HW_MODEL, HW_NCPUONLINE, HW_PAGESIZE, HW_PERFPOLICY,
    HW_PHYSMEM, HW_PHYSMEM64, HW_POWER, HW_PRODUCT, HW_SENSORS, HW_SERIALNO, HW_SETPERF, HW_SMT,
    HW_UCOMNAMES, HW_USERMEM, HW_USERMEM64, HW_UUID, HW_VENDOR, HW_VERSION, KERN_ALLOWKMEM,
    KERN_ARGMAX, KERN_AUDIO, KERN_AUTOCONF_SERIAL, KERN_BOOTTIME, KERN_CACHEPCT, KERN_CCPU,
    KERN_CLOCKINTR, KERN_CLOCKRATE, KERN_CONSBUF, KERN_CONSBUFSIZE, KERN_CONSDEV, KERN_CPTIME,
    KERN_CPTIME2, KERN_CPUSTATS, KERN_DOMAINNAME, KERN_EVCOUNT, KERN_FILE, KERN_FORKSTAT,
    KERN_FSCALE, KERN_FSYNC, KERN_GLOBAL_PTRACE, KERN_HOSTID, KERN_HOSTNAME, KERN_INTRCNT,
    KERN_JOB_CONTROL, KERN_MALLOCSTATS, KERN_MAXCLUSTERS, KERN_MAXFILES, KERN_MAXID,
    KERN_MAXPARTITIONS, KERN_MAXPROC, KERN_MAXTHREAD, KERN_MBSTAT, KERN_MSGBUF, KERN_MSGBUFSIZE,
    KERN_NCHSTATS, KERN_NETLIVELOCKS, KERN_NFILES, KERN_NGROUPS, KERN_NOSUIDCOREDUMP, KERN_NPROCS,
    KERN_NTHREADS, KERN_NUMVNODES, KERN_OSRELEASE, KERN_OSREV, KERN_OSTYPE, KERN_OSVERSION,
    KERN_PFSTATUS, KERN_POOL, KERN_POOL_DEBUG, KERN_POSIX1, KERN_PROC, KERN_PROC_ALL,
    KERN_PROC_ARGS, KERN_PROC_ARGV, KERN_PROC_CWD, KERN_PROC_ENV, KERN_PROC_KTHREAD,
    KERN_PROC_NARGV, KERN_PROC_NENV, KERN_PROC_NOBROADCASTKILL, KERN_PROC_PGRP, KERN_PROC_PID,
    KERN_PROC_RUID, KERN_PROC_SESSION, KERN_PROC_SHOW_THREADS, KERN_PROC_TTY, KERN_PROC_UID,
    KERN_PROC_VMMAP, KERN_RAWPARTITION, KERN_SAVED_IDS, KERN_SECURELVL, KERN_SOMAXCONN,
    KERN_SOMINCONN, KERN_STACKGAPRANDOM, KERN_SYSVMSG, KERN_SYSVSEM, KERN_SYSVSHM,
    KERN_TIMECOUNTER, KERN_TIMEOUT_STATS, KERN_TTY, KERN_TTYCOUNT, KERN_UTC_OFFSET, KERN_VERSION,
    KERN_WATCHDOG, KERN_WXABORT, KI_NOCPU, KinfoProc, KinfoVmentry, SYSCTL_INT_READONLY,
    SysctlBoundedArgs,
};
use hal::sys::syslimits::{ARG_MAX, NGROUPS_MAX, PATH_MAX};
use hal::sys::types::{Dev, Pid};
use hal::sys::unistd::_POSIX_VERSION;
use hal::user::{UserPtr, bytes_of};
use hal::vm::param::atop;
use hal::vm::uvmexp::UVMEXP;

use crate::assert::kpanic;
use crate::proc::{
    PS_CONTROLT, PS_EMBRYO, PS_EXITING, PS_INEXEC, PS_NOBROADCASTKILL, PS_PLEDGE, PS_SYSTEM,
    PS_ZOMBIE, ProcGlue, Process, PsMtx, SDEAD, SIDL, SONPROC, SRUN, SSLEEP, SSTOP,
    THREAD_PID_OFFSET, Thread, prfind, suser,
};
use crate::resource::{RLIMIT_RSS, TU_ITICKS, TU_STICKS, TU_UTICKS, Tusage};
use crate::uio::UioMove;
use crate::vm::map::{VmMap, VmspaceRef};

/// The user side of a sysctl request (`oldp`, `oldlenp`, `newp`, `newlen`).
pub struct SysctlReq<'a> {
    /// `oldp`: where the old value goes (0: nowhere).
    pub oldp: usize,
    /// `*oldlenp`: its room, then the value's length.
    pub oldlenp: &'a mut usize,
    /// `newp`: the new value (0: none).
    pub newp: usize,
    /// `newlen`.
    pub newlen: usize,
}

/// A node of the sysctl tree that answers for the names below it (a `*_sysctl` function of
/// the C: `sysctl_tty`, `sysctl_pty`, ...).
pub trait SysctlNode: Sync {
    /// The request `req` for the name `name` (the part below this node).
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno>;
}

/// `sysctl_rdquad(oldp, oldlenp, newp, val)`: a read-only 64-bit value.
pub fn rdquad(req: &mut SysctlReq<'_>, val: i64) -> Result<(), Errno> {
    rdstruct(req, &val.to_ne_bytes())
}

/// `sysctl_rdstruct(oldp, oldlenp, newp, sp, len)`: a read-only structure's bytes: `ENOMEM`
/// when the room is short, `EPERM` for a new value, the length always given back.
pub fn rdstruct(req: &mut SysctlReq<'_>, sp: &[u8]) -> Result<(), Errno> {
    let len = sp.len();
    if req.oldp != 0 && *req.oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 {
        return Err(Errno::EPERM);
    }
    *req.oldlenp = len;
    if req.oldp != 0 {
        UserPtr::<u8>::new(req.oldp).copyout_slice(sp)?;
    }
    Ok(())
}

/// `sizeof(int)`: the size of every integer leaf.
const INT: usize = size_of::<i32>();

/// The `int` at the user address `newp` (`copyin(newp, &v, sizeof(int))`).
fn copyin_int(newp: usize) -> Result<i32, Errno> {
    UserPtr::<i32>::new(newp).copyin()
}

/// `copyout(&v, oldp, sizeof(int))`.
fn copyout_int(v: i32, oldp: usize) -> Result<(), Errno> {
    UserPtr::<i32>::new(oldp).copyout(&v)
}

/// A read-only string leaf the registering subsystem names (`kern.ostype`, `hw.machine`):
/// [`rdstring`] of its bytes.
pub struct RdString(pub &'static [u8]);

impl SysctlNode for RdString {
    fn sysctl(&self, _name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        rdstring(req, self.0)
    }
}

/// `sysctl_int_lower`: reads, or writes that lower the value (the C's unsigned comparison:
/// a negative value is the largest).
pub fn int_lower(req: &mut SysctlReq<'_>, valp: &AtomicI32) -> Result<(), Errno> {
    if req.oldp != 0 && *req.oldlenp < INT {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 && req.newlen != INT {
        return Err(Errno::EINVAL);
    }
    *req.oldlenp = INT;

    if req.newp != 0 {
        let newval = copyin_int(req.newp)?;
        let mut oldval = valp.load(Ordering::Relaxed);
        loop {
            if (oldval as u32) < (newval as u32) {
                return Err(Errno::EPERM); // do not allow raising
            }
            match valp.compare_exchange(oldval, newval, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(current) => oldval = current,
            }
        }

        if req.oldp != 0 {
            // new value has been set although user gets error
            copyout_int(oldval, req.oldp)?;
        }
    } else if req.oldp != 0 {
        copyout_int(valp.load(Ordering::Relaxed), req.oldp)?;
    }

    Ok(())
}

/// `sysctl_int`: validates parameters and gets the old / sets the new value of an
/// integer-valued sysctl.
pub fn int(req: &mut SysctlReq<'_>, valp: &AtomicI32) -> Result<(), Errno> {
    int_bounded(req, valp, i32::MIN, i32::MAX)
}

/// `sysctl_rdint`: as [`int`], but read-only.
pub fn rdint(req: &mut SysctlReq<'_>, val: i32) -> Result<(), Errno> {
    if req.oldp != 0 && *req.oldlenp < INT {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 {
        return Err(Errno::EPERM);
    }
    *req.oldlenp = INT;
    if req.oldp != 0 {
        copyout_int(val, req.oldp)?;
    }
    Ok(())
}

/// `sysctl_securelevel`: `kern.securelevel`, which only init (pid 1, `pid` the caller's
/// process) may lower once raised.
pub fn sysctl_securelevel(req: &mut SysctlReq<'_>, pid: Pid) -> Result<(), Errno> {
    if req.oldp != 0 && *req.oldlenp < INT {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 && req.newlen != INT {
        return Err(Errno::EINVAL);
    }
    *req.oldlenp = INT;

    if req.newp != 0 {
        let newval = copyin_int(req.newp)?;
        let mut oldval = SECURELEVEL.load(Ordering::Relaxed);
        loop {
            if (oldval > 0 || newval < -1) && newval < oldval && pid != 1 {
                return Err(Errno::EPERM);
            }
            match SECURELEVEL.compare_exchange(oldval, newval, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(current) => oldval = current,
            }
        }

        if req.oldp != 0 {
            // new value has been set although user gets error
            copyout_int(oldval, req.oldp)?;
        }
    } else if req.oldp != 0 {
        copyout_int(SECURELEVEL.load(Ordering::Relaxed), req.oldp)?;
    }

    Ok(())
}

/// `sysctl_securelevel_int`: [`rdint`] or [`int`] according to `securelevel`.
pub fn securelevel_int(req: &mut SysctlReq<'_>, valp: &AtomicI32) -> Result<(), Errno> {
    if SECURELEVEL.load(Ordering::Relaxed) > 0 {
        return rdint(req, valp.load(Ordering::Relaxed));
    }
    int(req, valp)
}

/// `sysctl_int_bounded`: read-only or bounded integer values. `minimum > maximum` makes the
/// value read-only; both bounds are inclusive.
pub fn int_bounded(
    req: &mut SysctlReq<'_>,
    valp: &AtomicI32,
    minimum: i32,
    maximum: i32,
) -> Result<(), Errno> {
    // read only
    if req.newp != 0 && minimum > maximum {
        return Err(Errno::EPERM);
    }

    if req.oldp != 0 && *req.oldlenp < INT {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 && req.newlen != INT {
        return Err(Errno::EINVAL);
    }
    *req.oldlenp = INT;

    // copyin() may sleep, call it first
    let mut newval = 0;
    if req.newp != 0 {
        newval = copyin_int(req.newp)?;
        // outside limits
        if newval < minimum || maximum < newval {
            return Err(Errno::EINVAL);
        }
    }
    if req.oldp != 0 {
        let oldval = if req.newp != 0 {
            valp.swap(newval, Ordering::Relaxed)
        } else {
            valp.load(Ordering::Relaxed)
        };
        // new value has been set although user gets error
        copyout_int(oldval, req.oldp)?;
    } else if req.newp != 0 {
        valp.store(newval, Ordering::Relaxed);
    }

    Ok(())
}

/// `sysctl_bounded_arr`: an array of read-only or bounded integer values, looked up by the
/// one remaining name component.
pub fn bounded_arr(
    valpp: &[SysctlBoundedArgs],
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    let [mib] = name else {
        return Err(Errno::ENOTDIR);
    };
    match valpp.iter().find(|v| v.mib == *mib) {
        Some(v) => int_bounded(req, v.var, v.minimum, v.maximum),
        None => Err(Errno::EOPNOTSUPP),
    }
}

/// `sysctl_string`: validates parameters and gets the old / sets the new value of a
/// string-valued sysctl. `s` is the variable's whole buffer (`maxlen` in C).
pub fn string(req: &mut SysctlReq<'_>, s: &mut [u8]) -> Result<(), Errno> {
    string_(req, s, false)
}

/// `sysctl_tstring`: as [`string`], but a short `old` buffer gets a truncated,
/// NUL-terminated copy instead of `ENOMEM`.
pub fn tstring(req: &mut SysctlReq<'_>, s: &mut [u8]) -> Result<(), Errno> {
    string_(req, s, true)
}

/// `strlen(s)` of a buffer: up to its first NUL, or all of it.
fn cstrlen(s: &[u8]) -> usize {
    s.iter().position(|&c| c == 0).unwrap_or(s.len())
}

/// `sysctl__string`: the body of [`string`] and [`tstring`].
pub fn string_(req: &mut SysctlReq<'_>, s: &mut [u8], trunc: bool) -> Result<(), Errno> {
    let maxlen = s.len();
    let slen = cstrlen(s);
    let mut len = slen + 1;
    if req.oldp != 0 && *req.oldlenp < len && (!trunc || *req.oldlenp == 0) {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 && req.newlen >= maxlen {
        return Err(Errno::EINVAL);
    }
    let mut error = Ok(());
    if req.oldp != 0 {
        if trunc && *req.oldlenp < len {
            len = *req.oldlenp;
            error = UserPtr::<u8>::new(req.oldp).copyout_slice(&s[..len - 1]);
            if error.is_ok() {
                error = UserPtr::<u8>::new(req.oldp + len - 1).copyout_slice(&[0]);
            }
        } else {
            error = copyout_cstr(&s[..slen], req.oldp);
        }
    }
    *req.oldlenp = len;
    if error.is_ok() && req.newp != 0 {
        error = UserPtr::<u8>::new(req.newp).copyin_slice(&mut s[..req.newlen]);
        s[req.newlen] = 0;
    }
    error
}

/// `copyout(str, oldp, strlen(str) + 1)`: the bytes, then the NUL.
fn copyout_cstr(s: &[u8], uaddr: usize) -> Result<(), Errno> {
    UserPtr::<u8>::new(uaddr).copyout_slice(s)?;
    UserPtr::<u8>::new(uaddr + s.len()).copyout_slice(&[0])
}

/// `sysctl_rdstring`: as [`string`], but read-only. `s` ends at its first NUL or at its end.
pub fn rdstring(req: &mut SysctlReq<'_>, s: &[u8]) -> Result<(), Errno> {
    let slen = cstrlen(s);
    let len = slen + 1;
    if req.oldp != 0 && *req.oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 {
        return Err(Errno::EPERM);
    }
    *req.oldlenp = len;
    if req.oldp != 0 {
        copyout_cstr(&s[..slen], req.oldp)?;
    }
    Ok(())
}

/// `sysctl_struct`: validates parameters and gets the old / sets the new value of a
/// structure-valued sysctl. `sp` is the structure's bytes; a new value is copied in whole
/// (`len` bytes, as the C).
pub fn structure(req: &mut SysctlReq<'_>, sp: &mut [u8]) -> Result<(), Errno> {
    let len = sp.len();

    if req.oldp != 0 && *req.oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 && req.newlen > len {
        return Err(Errno::EINVAL);
    }
    let mut error = Ok(());
    if req.oldp != 0 {
        *req.oldlenp = len;
        error = UserPtr::<u8>::new(req.oldp).copyout_slice(sp);
    }
    if error.is_ok() && req.newp != 0 {
        error = UserPtr::<u8>::new(req.newp).copyin_slice(sp);
    }
    error
}

/// `PTRTOINT64(x)` (`<sys/sysctl.h>`): an address as `kinfo_proc` reports it.
fn ptrtoint64<T: ?Sized>(x: &T) -> u64 {
    ptr::from_ref(x).cast::<u8>().addr() as u64
}

/// `MAXHOSTNAMELEN` (`<sys/param.h>`): the size of `hostname` and `domainname`, the NUL
/// included.
pub const MAXHOSTNAMELEN: usize = 256;
/// `KERN_PROCSLOP`: try over estimating by 5 procs.
const KERN_PROCSLOP: usize = 5;
/// `MAXPHYS` (`<sys/param.h>`): the largest raw I/O transfer.
const MAXPHYS: usize = 64 * 1024;
/// `VMMAP_MAXLEN`: arbitrary but reasonable limit for one iteration.
const VMMAP_MAXLEN: usize = MAXPHYS;
/// `MAXPATHLEN` (`<sys/param.h>`).
const MAXPATHLEN: usize = PATH_MAX;
/// `NODEV` (`<sys/param.h>`): a non-existent device.
const NODEV: Dev = -1;
/// `M_TEMP` (`<sys/malloc.h>`): misc temporary data buffers.
const M_TEMP: i32 = 127;
/// `M_SYSCTL` (`<sys/malloc.h>`): sysctl buffers (persistent storage).
const M_SYSCTL: i32 = 11;
/// `sizeof(struct diskstats)` (`<sys/disk.h>`): the bytes of one `hw.diskstats` entry, which
/// the [`DiskExport`] hands over already laid out.
pub const DISKSTATS_SIZE: usize = 112;
/// `BYTE_ORDER` (`<machine/endian.h>`): `LITTLE_ENDIAN` (1234) or `BIG_ENDIAN` (4321).
const BYTE_ORDER_C: i32 = if cfg!(target_endian = "little") {
    1234
} else {
    4321
};

/// `sysctl_lock`: avoids too many processes vslocking a large amount of memory at the same
/// time (and serialises every vslocked node: `kern.hostname`, `kern.maxclusters`'s update,
/// the network's `MPSYSCTL`-less protocols).
pub static SYSCTL_LOCK: RawRwlock = RawRwlock::new("sysctllk");

/// `sysctl_disklock` and what it guards: `disknames` and `diskstats` (`M_SYSCTL`), rebuilt
/// by [`sysctl_diskinit`]; `None` until the first call.
static SYSCTL_DISKLOCK: Rwlocked<DiskExports> = Rwlocked::new(
    "sysctldlk",
    DiskExports {
        names: None,
        stats: None,
    },
);

/// The `hw.disknames` string and the `hw.diskstats` array of `sysctl_diskinit`.
struct DiskExports {
    /// `disknames` (`disknameslen` its length).
    names: Option<MArray<u8>>,
    /// `diskstats` (`diskstatslen` its length), [`DISKSTATS_SIZE`] bytes an entry.
    stats: Option<MArray<u8>>,
}

/// `hostname`. Protected by: its lock (and `sysctl_lock` for `kern.hostname`).
pub static HOSTNAME: Rwlocked<[u8; MAXHOSTNAMELEN]> =
    Rwlocked::new("hostname", [0; MAXHOSTNAMELEN]);
/// `hostnamelen`.
pub static HOSTNAMELEN: AtomicI32 = AtomicI32::new(0);
/// `domainname`. Protected by: its lock (and `sysctl_lock` for `kern.domainname`).
pub static DOMAINNAME: Rwlocked<[u8; MAXHOSTNAMELEN]> =
    Rwlocked::new("domainname", [0; MAXHOSTNAMELEN]);
/// `domainnamelen`.
pub static DOMAINNAMELEN: AtomicI32 = AtomicI32::new(0);
/// `hostid`.
pub static HOSTID: AtomicI32 = AtomicI32::new(0);
/// `securelevel`: the system security level (`<sys/systm.h>` explains the levels).
pub static SECURELEVEL: AtomicI32 = AtomicI32::new(0);
/// \[a\] `allowkmem`.
pub static ALLOWKMEM: AtomicI32 = AtomicI32::new(0);
/// `allowpowerdown`: allow the power button to shut the machine down.
pub static ALLOWPOWERDOWN: AtomicI32 = AtomicI32::new(1);
/// `hw_power`: the machine has wall power.
#[allow(non_upper_case_globals)] // the C's name; `HW_POWER` is the sysctl id
pub static hw_power: AtomicI32 = AtomicI32::new(1);

/// One of the machine's identity strings (`char *hw_vendor`, ...): set once at attach, read
/// by `hw.vendor` and its siblings and by the drivers that quirk on them. `write` is the C's
/// assignment (a `NULL` leaves the string unset, and a second one keeps the first: the C's
/// writers run once, at attach), `read` the C's read of the pointer.
pub struct HwString(OnceStatic<&'static [u8]>);

impl HwString {
    /// Unset (`NULL`).
    pub const fn new() -> Self {
        Self(OnceStatic::new())
    }

    /// `hw_x = s`.
    pub fn write(&self, s: Option<&'static [u8]>) {
        if let Some(s) = s {
            let _ = self.0.set(s);
        }
    }

    /// `hw_x`: the string, `None` for `NULL`.
    pub fn read(&self) -> Option<&'static [u8]> {
        self.0.get().copied()
    }
}

impl Default for HwString {
    fn default() -> Self {
        Self::new()
    }
}

/// `hw_vendor`: set by the machine when the firmware names it.
#[allow(non_upper_case_globals)] // the C's name; `HW_VENDOR` is the sysctl id
pub static hw_vendor: HwString = HwString::new();
/// `hw_prod`.
#[allow(non_upper_case_globals)] // the C's name, kept beside `hw_vendor`
pub static hw_prod: HwString = HwString::new();
/// `hw_uuid`.
#[allow(non_upper_case_globals)] // the C's name; `HW_UUID` is the sysctl id
pub static hw_uuid: HwString = HwString::new();
/// `hw_serial`.
#[allow(non_upper_case_globals)] // the C's name, kept beside `hw_vendor`
pub static hw_serial: HwString = HwString::new();
/// `hw_ver`.
#[allow(non_upper_case_globals)] // the C's name, kept beside `hw_vendor`
pub static hw_ver: HwString = HwString::new();

/// `hw_battery_chargemode`.
#[allow(non_upper_case_globals)] // the C's name; `HW_BATTERY_CHARGEMODE` is the sysctl id
pub static hw_battery_chargemode: AtomicI32 = AtomicI32::new(0);
/// `hw_battery_chargestart`.
#[allow(non_upper_case_globals)] // the C's name; `HW_BATTERY_CHARGESTART` is the sysctl id
pub static hw_battery_chargestart: AtomicI32 = AtomicI32::new(0);
/// `hw_battery_chargestop`.
#[allow(non_upper_case_globals)] // the C's name; `HW_BATTERY_CHARGESTOP` is the sysctl id
pub static hw_battery_chargestop: AtomicI32 = AtomicI32::new(0);

/// `int_zero`: morally const, reported by `sysctl_bounded_arr`.
static INT_ZERO: AtomicI32 = AtomicI32::new(0);
/// `int_one`.
static INT_ONE: AtomicI32 = AtomicI32::new(1);
/// `arg_max`.
static ARG_MAX_VAR: AtomicI32 = AtomicI32::new(ARG_MAX as i32);
/// `posix_version`.
static POSIX_VERSION: AtomicI32 = AtomicI32::new(_POSIX_VERSION as i32);
/// `ngroups_max`.
static NGROUPS_MAX_VAR: AtomicI32 = AtomicI32::new(NGROUPS_MAX as i32);
/// `ccpu`, read-only: the scheduler's constant.
static CCPU: AtomicI32 = AtomicI32::new(crate::sched::CCPU as i32);
/// `byte_order`.
static BYTE_ORDER: AtomicI32 = AtomicI32::new(BYTE_ORDER_C);

/// `kern_vars[]`, the kernel's entries: the `kern` integers `sysctl_bounded_arr` serves whose
/// variable is the kernel's. The others (`maxvnodes`, `maxproc`, `maxfiles`, `nfiles`,
/// `ttycount`, ...) are their subsystems' table ([`register_kern_vars`]), searched after this
/// one: the C's one array in two pieces, the same lookup.
static KERN_VARS: [SysctlBoundedArgs; 14] = [
    SysctlBoundedArgs::readonly(KERN_ARGMAX, &ARG_MAX_VAR),
    SysctlBoundedArgs::readonly(KERN_POSIX1, &POSIX_VERSION),
    SysctlBoundedArgs::readonly(KERN_NGROUPS, &NGROUPS_MAX_VAR),
    SysctlBoundedArgs::readonly(KERN_JOB_CONTROL, &INT_ONE),
    SysctlBoundedArgs::readonly(KERN_SAVED_IDS, &INT_ONE),
    SysctlBoundedArgs::readonly(KERN_NTHREADS, &crate::fork::NTHREADS),
    SysctlBoundedArgs::readonly(KERN_FSYNC, &INT_ONE),
    // SYSVMSG, SYSVSEM, SYSVSHM: not configured.
    SysctlBoundedArgs::readonly(KERN_SYSVMSG, &INT_ZERO),
    SysctlBoundedArgs::readonly(KERN_SYSVSEM, &INT_ZERO),
    SysctlBoundedArgs::readonly(KERN_SYSVSHM, &INT_ZERO),
    SysctlBoundedArgs::readonly(KERN_CCPU, &CCPU),
    SysctlBoundedArgs::readonly(KERN_NPROCS, &crate::fork::NPROCESSES),
    SysctlBoundedArgs::new(KERN_WXABORT, &crate::vm::mmap::UVM_WXABORT, 0, 1),
    SysctlBoundedArgs::readonly(KERN_NETLIVELOCKS, &INT_ZERO),
    // KERN_GLOBAL_PTRACE: PTRACE is not configured.
];

/// `hw_vars[]`, the kernel's entries; `ncpu`, `ncpufound` and `diskcount` are `bsd`'s table
/// ([`register_hw_vars`]).
static HW_VARS: [SysctlBoundedArgs; 3] = [
    SysctlBoundedArgs::readonly(HW_BYTEORDER, &BYTE_ORDER),
    SysctlBoundedArgs::readonly(HW_PAGESIZE, &UVMEXP.pagesize),
    SysctlBoundedArgs::readonly(HW_POWER, &hw_power),
];

/// A registered node's slot.
type NodeSlot = OnceStatic<&'static dyn SysctlNode>;

/// The top-level nodes of the subsystems still in `bsd` (`net_sysctl`, `vfs_sysctl`,
/// `cpu_sysctl`, `ddb_sysctl`), by `CTL_*` id.
static TOP: [NodeSlot; CTL_MAXID] = [const { OnceStatic::new() }; CTL_MAXID];
/// The `kern` nodes of the subsystems still in `bsd`, by `KERN_*` id (leaves and
/// directories alike; the dispatcher says which are locked).
static KERN: [NodeSlot; KERN_MAXID] = [const { OnceStatic::new() }; KERN_MAXID];
/// The `hw` nodes of the subsystems still in `bsd`, by `HW_*` id.
static HW: [NodeSlot; HW_MAXID] = [const { OnceStatic::new() }; HW_MAXID];
/// The `kern_vars[]` entries of `bsd`'s variables.
static KERN_VARS_EXT: OnceStatic<&'static [SysctlBoundedArgs]> = OnceStatic::new();
/// The `hw_vars[]` entries of `bsd`'s variables.
static HW_VARS_EXT: OnceStatic<&'static [SysctlBoundedArgs]> = OnceStatic::new();
/// `cpu_cpuspeed`: the machine's CPU frequency reader, if it has one.
static CPU_CPUSPEED: OnceStatic<CpuSpeedFn> = OnceStatic::new();
/// `int (*)(int *)`: the type of `cpu_cpuspeed`, which stores the CPU's frequency in MHz.
pub type CpuSpeedFn = fn(&mut i32) -> Result<(), Errno>;
/// The battery driver's setters (`hw_battery_set*`).
static BATTERY: OnceStatic<&'static dyn BatteryCharge> = OnceStatic::new();
/// The disk list's export (`bsd`'s `disklist`, N7).
static DISK_EXPORT: OnceStatic<&'static dyn DiskExport> = OnceStatic::new();
/// `physmem`: physical memory, in pages (the machine's `bsd` variable).
static PHYSMEM: OnceStatic<&'static core::sync::atomic::AtomicUsize> = OnceStatic::new();

/// The slot of `id` in `table`, `None` for an id outside it (negative or too large).
fn slot(table: &'static [NodeSlot], id: i32) -> Option<&'static NodeSlot> {
    usize::try_from(id).ok().and_then(|i| table.get(i))
}

/// The node registered for `id` in `table`.
fn node(table: &'static [NodeSlot], id: i32) -> Option<&'static dyn SysctlNode> {
    slot(table, id).and_then(|s| s.get().copied())
}

/// Registers in `table` the node of `id`, once.
fn register(
    table: &'static [NodeSlot],
    id: i32,
    n: &'static dyn SysctlNode,
    what: &'static str,
) -> Result<(), AlreadyRegistered> {
    match slot(table, id) {
        Some(s) => s.set(n).map_err(|_| AlreadyRegistered(what)),
        None => Err(AlreadyRegistered(what)),
    }
}

/// Registers the top-level node `ctl` (`CTL_NET`, `CTL_VFS`, `CTL_MACHDEP`, `CTL_DDB`), once;
/// a second registration of an id, or an id the dispatcher serves itself or never consults,
/// is refused.
pub fn register_top(ctl: i32, n: &'static dyn SysctlNode) -> Result<(), AlreadyRegistered> {
    if !matches!(ctl, CTL_NET | CTL_VFS | CTL_MACHDEP | CTL_DDB) {
        return Err(AlreadyRegistered("SysctlNode (top: not a registered tree)"));
    }
    register(&TOP, ctl, n, "SysctlNode (top)")
}

/// Registers the `kern` node `id` (a leaf or a directory of the dispatcher's registered
/// sets), once.
pub fn register_kern(id: i32, n: &'static dyn SysctlNode) -> Result<(), AlreadyRegistered> {
    if !kern_registrable(id) {
        return Err(AlreadyRegistered(
            "SysctlNode (kern: not a registered node)",
        ));
    }
    register(&KERN, id, n, "SysctlNode (kern)")
}

/// Registers the `hw` node `id` (`machine`, `sensors`, `ucomnames`, `smt`, `blockcpu`), once.
pub fn register_hw(id: i32, n: &'static dyn SysctlNode) -> Result<(), AlreadyRegistered> {
    if !HW_REGISTERED.contains(&id) {
        return Err(AlreadyRegistered("SysctlNode (hw: not a registered node)"));
    }
    register(&HW, id, n, "SysctlNode (hw)")
}

/// Registers the `kern_vars[]` entries whose variables are `bsd`'s, once.
pub fn register_kern_vars(t: &'static [SysctlBoundedArgs]) -> Result<(), AlreadyRegistered> {
    KERN_VARS_EXT
        .set(t)
        .map_err(|_| AlreadyRegistered("SysctlNode (kern_vars)"))
}

/// Registers the `hw_vars[]` entries whose variables are `bsd`'s, once.
pub fn register_hw_vars(t: &'static [SysctlBoundedArgs]) -> Result<(), AlreadyRegistered> {
    HW_VARS_EXT
        .set(t)
        .map_err(|_| AlreadyRegistered("SysctlNode (hw_vars)"))
}

/// `cpu_cpuspeed = f`: the machine's CPU frequency reader, once.
pub fn register_cpuspeed(f: CpuSpeedFn) -> Result<(), AlreadyRegistered> {
    CPU_CPUSPEED
        .set(f)
        .map_err(|_| AlreadyRegistered("cpu_cpuspeed"))
}

/// `physmem`'s variable (the machine's, in pages), once.
pub fn register_physmem(
    v: &'static core::sync::atomic::AtomicUsize,
) -> Result<(), AlreadyRegistered> {
    PHYSMEM.set(v).map_err(|_| AlreadyRegistered("physmem"))
}

/// One of the three battery knobs of `hw.battery`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryKnob {
    /// `chargemode` (-1 inhibit, 0 off, 1 auto).
    ChargeMode,
    /// `chargestart` (percent).
    ChargeStart,
    /// `chargestop` (percent).
    ChargeStop,
}

/// A battery driver's setters (`hw_battery_setchargemode`, `hw_battery_setchargestart`,
/// `hw_battery_setchargestop`): the C's three entries one driver fills, each optional
/// (`qcpas` sets start and stop only).
pub trait BatteryCharge: Sync {
    /// Whether the driver set `knob`'s setter (`hw_battery_set* != NULL`).
    fn has(&self, knob: BatteryKnob) -> bool;
    /// `hw_battery_set*(value)`.
    fn set(&self, knob: BatteryKnob, value: i32) -> Result<(), Errno>;
}

/// Registers the battery driver's setters, once.
pub fn register_battery(b: &'static dyn BatteryCharge) -> Result<(), AlreadyRegistered> {
    BATTERY
        .set(b)
        .map_err(|_| AlreadyRegistered("BatteryCharge"))
}

/// The disk list as `sysctl_diskinit` reads it (`bsd`'s `subr_disk`, N7: `disk_change`,
/// `disk_count`, `disklist` and each disk's statistics under its `dk_mtx`). A bridge
/// (`bridges.toml`).
pub trait DiskExport: Sync {
    /// `if (disk_change) disk_change = 0`: whether the list changed since the last call.
    fn take_change(&self) -> bool;
    /// `disk_count`.
    fn count(&self) -> usize;
    /// `TAILQ_FOREACH(dk, &disklist, dk_link)`: `f` on each disk's name (up to its NUL), its
    /// label's uid formatted (`None` when the label has none) and its `struct diskstats`
    /// (`DISKSTATS_SIZE` bytes, name and statistics filled under the disk's mutex).
    fn for_each(&self, f: &mut DiskVisit<'_>);
}

/// What [`DiskExport::for_each`] calls on each disk: its name, its label's uid formatted (or
/// `None`) and its `struct diskstats` bytes.
pub type DiskVisit<'a> = dyn FnMut(&[u8], Option<&[u8]>, &[u8]) + 'a;

/// The host tests' reset (`testing.md`'s rule: a global holding kernel memory): the disk
/// exports are forgotten, not freed, since they may be an earlier test's memory, which
/// `setup_real_memory` replaced.
#[cfg(any(test, feature = "doubles"))]
pub fn sysctl_test_reset() {
    let mut d = SYSCTL_DISKLOCK.write();
    core::mem::forget(d.names.take());
    core::mem::forget(d.stats.take());
}

/// Registers the disk list's export, once (`bsd`'s `main`).
pub fn install_disk_export(d: &'static dyn DiskExport) -> Result<(), AlreadyRegistered> {
    DISK_EXPORT
        .set(d)
        .map_err(|_| AlreadyRegistered("DiskExport"))
}

/// `f(hostname)`: the host's name (up to `hostnamelen` bytes are meaningful), under its lock.
pub fn with_hostname<R>(f: impl FnOnce(&[u8]) -> R) -> R {
    f(&*HOSTNAME.read())
}

/// `f(domainname)`, under its lock.
pub fn with_domainname<R>(f: impl FnOnce(&[u8]) -> R) -> R {
    f(&*DOMAINNAME.read())
}

/// `f(&mut hostname)` (`nfs_boot`'s answer from the server), under its lock.
pub fn with_hostname_mut<R>(f: impl FnOnce(&mut [u8; MAXHOSTNAMELEN]) -> R) -> R {
    f(&mut HOSTNAME.write())
}

/// `f(&mut domainname)`, under its lock.
pub fn with_domainname_mut<R>(f: impl FnOnce(&mut [u8; MAXHOSTNAMELEN]) -> R) -> R {
    f(&mut DOMAINNAME.write())
}

/// `sysctl_vslock`'s hold, `sysctl_vsunlock` its drop: `sysctl_lock` taken for writing, the
/// kernel lock, and the user buffer `[addr, addr + len)` wired (when `addr` is not NULL) so
/// that the copies under the lock do not sleep on a fault.
pub struct VsLock {
    /// The wired buffer: the address space, `addr`, the length saved at lock time.
    wired: Option<(VmspaceRef, usize, usize)>,
    /// `sysctl_lock`, released last (`rw_exit_write` after `KERNEL_UNLOCK`).
    _w: WriteWitness<'static>,
}

impl Drop for VsLock {
    /// `sysctl_vsunlock(addr, savelen)`.
    fn drop(&mut self) {
        klock::kernel_assert_locked();
        // The handle is curproc's address space as the C names it (`p->p_vmspace`), not a
        // reference: nothing is given back but the handle.
        if let Some((vm, addr, len)) = self.wired.take() {
            crate::vm::glue::uvm_vsunlock(&vm.vm_map, addr, len);
        }
        klock::kernel_unlock();
    }
}

/// `sysctl_vslock(addr, len)` for the thread `p` (`curproc`): `sysctl_lock` (interruptible),
/// the kernel lock, then the buffer wired unless it would pass `uvmexp.wiredmax` (`ENOMEM`)
/// or `uvm_vslock` fails; on an error nothing is held.
pub fn sysctl_vslock<G: ProcGlue>(p: &Thread<G>, addr: usize, len: usize) -> Result<VsLock, Errno> {
    let w = SYSCTL_LOCK.write_flags(RW_INTR)?;
    klock::kernel_lock();

    let mut wired = None;
    if addr != 0 {
        let held = i64::from(UVMEXP.wired.load(Ordering::Relaxed));
        let max = i64::from(UVMEXP.wiredmax.load(Ordering::Relaxed));
        let res = if atop(len) as i64 > max - held {
            Err(Errno::ENOMEM)
        } else {
            match p.vmspace_share() {
                Some(vm) => {
                    crate::vm::glue::uvm_vslock(&vm.vm_map, addr, len, PROT_READ | PROT_WRITE)
                        .map(|()| vm)
                }
                // A thread without an address space (a kernel thread) has no user buffer.
                None => Err(Errno::EFAULT),
            }
        };
        match res {
            Ok(vm) => wired = Some((vm, addr, len)),
            Err(e) => {
                klock::kernel_unlock();
                drop(w);
                return Err(e);
            }
        }
    }

    Ok(VsLock { wired, _w: w })
}

/// The top-level trees `sys_sysctl` dispatches (`switch (name[0])`) and whether it vslocks
/// the buffer around them (`dolock`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Top {
    /// `kern_sysctl`, unlocked (its leaves and directories lock themselves).
    Kern,
    /// `hw_sysctl`, unlocked.
    Hw,
    /// `net_sysctl`, unlocked (its protocols lock themselves).
    Net,
    /// `uvm_sysctl`, locked.
    Vm,
    /// `vfs_sysctl`, locked.
    Vfs,
    /// `cpu_sysctl`, locked.
    Machdep,
    /// `ddb_sysctl`, locked.
    Ddb,
}

impl Top {
    /// The tree of `ctl`; `None` for an id the C's `switch` has no case for (`CTL_DEBUG`
    /// without `DEBUG_SYSCTL`, the gaps), or whose subsystem registered no node (a tree
    /// compiled out).
    fn of(ctl: i32) -> Option<Self> {
        let t = match ctl {
            CTL_KERN => return Some(Self::Kern),
            CTL_HW => return Some(Self::Hw),
            CTL_VM => return Some(Self::Vm),
            CTL_NET => Self::Net,
            CTL_VFS => Self::Vfs,
            CTL_MACHDEP => Self::Machdep,
            CTL_DDB => Self::Ddb,
            _ => return None,
        };
        node(&TOP, ctl).map(|_| t)
    }

    /// `dolock`: the trees served under `sysctl_vslock` (all but `kern`, `hw` and `net`).
    fn dolock(self) -> bool {
        !matches!(self, Self::Kern | Self::Hw | Self::Net)
    }
}

/// `sysctl(2)`: reads and writes the variable `name` (`namelen` components at that user
/// address) names; `old` (with `*oldlenp` bytes of room) receives the old value, `new`
/// (`newlen` bytes) is the value to set. `p` is the calling thread.
pub fn sys_sysctl<G: ProcGlue>(
    p: &Thread<G>,
    name: UserPtr<i32>,
    namelen: u32,
    old: usize,
    oldlenp: UserPtr<usize>,
    new: usize,
    newlen: usize,
) -> Result<(), Errno> {
    if new != 0 {
        suser(p)?;
    }
    // all top-level sysctl names are non-terminal
    let namelen = namelen as usize;
    if !(2..=CTL_MAXNAME).contains(&namelen) {
        return Err(Errno::EINVAL);
    }
    let mut buf = [0i32; CTL_MAXNAME];
    name.copyin_slice(&mut buf[..namelen])?;
    let mib = &buf[..namelen];

    G::pledge_sysctl(p, mib, new)?;

    let Some(top) = Top::of(mib[0]) else {
        return Err(Errno::EOPNOTSUPP);
    };

    let mut oldlen: usize = 0;
    if !oldlenp.is_null() {
        oldlen = oldlenp.copyin()?;
    }

    let error = {
        let _vs = if top.dolock() {
            Some(sysctl_vslock(p, old, oldlen)?)
        } else {
            None
        };
        let mut req = SysctlReq {
            oldp: old,
            oldlenp: &mut oldlen,
            newp: new,
            newlen,
        };
        top_sysctl(p, top, mib[0], &mib[1..], &mut req)
        // `_vs` drops here: sysctl_vsunlock(old, savelen), on the error path too.
    };

    error?;
    if !oldlenp.is_null() {
        oldlenp.copyout(&oldlen)?;
    }
    Ok(())
}

/// The `(*fn)(&name[1], namelen - 1, ...)` call of `sys_sysctl`: the tree `top` (id `ctl`)
/// for the rest of the name.
fn top_sysctl<G: ProcGlue>(
    p: &Thread<G>,
    top: Top,
    ctl: i32,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    match top {
        Top::Kern => kern_sysctl(p, name, req),
        Top::Hw => hw_sysctl(p, name, req),
        Top::Vm => crate::vm::meter::uvm_sysctl::<G>(name, req),
        Top::Net | Top::Vfs | Top::Machdep | Top::Ddb => match node(&TOP, ctl) {
            Some(n) => n.sysctl(name, req),
            None => Err(Errno::EOPNOTSUPP),
        },
    }
}

/// The `kern` leaves `kern_sysctl` serves without the vslock whose subsystem registers them
/// (the C's first `switch`; the kernel's own are matched by [`kern_sysctl`] itself).
const KERN_LEAVES_UNLOCKED: [i32; 11] = [
    KERN_OSTYPE,
    KERN_OSRELEASE,
    KERN_OSVERSION,
    KERN_VERSION,
    KERN_CLOCKRATE,
    KERN_NUMVNODES,
    KERN_BOOTTIME,
    KERN_MAXCLUSTERS,
    KERN_MBSTAT,
    KERN_CPTIME,
    KERN_POOL_DEBUG,
];

/// The `kern` integers of `kern_vars[]` that `kern_sysctl` serves without the vslock (the
/// rest of the table, `maxvnodes`, `splassert`, `maxlocksperuid`, is served inside it, by
/// `kern_sysctl_locked`'s `default`).
const KERN_VARS_UNLOCKED: [i32; 28] = [
    KERN_MAXPROC,
    KERN_MAXFILES,
    KERN_NFILES,
    KERN_TTYCOUNT,
    KERN_ARGMAX,
    KERN_POSIX1,
    KERN_NGROUPS,
    KERN_JOB_CONTROL,
    KERN_SAVED_IDS,
    KERN_FSYNC,
    KERN_SYSVMSG,
    KERN_SYSVSEM,
    KERN_SYSVSHM,
    KERN_SOMAXCONN,
    KERN_SOMINCONN,
    KERN_NOSUIDCOREDUMP,
    KERN_WXABORT,
    KERN_NETLIVELOCKS,
    KERN_GLOBAL_PTRACE,
    KERN_AUTOCONF_SERIAL,
    KERN_OSREV,
    KERN_MAXPARTITIONS,
    KERN_RAWPARTITION,
    KERN_MAXTHREAD,
    KERN_NTHREADS,
    KERN_FSCALE,
    KERN_CCPU,
    KERN_NPROCS,
];

/// The `kern` leaves `kern_sysctl_locked` serves whose subsystem registers them.
const KERN_LEAVES_LOCKED: [i32; 6] = [
    KERN_NCHSTATS,
    KERN_STACKGAPRANDOM,
    KERN_CACHEPCT,
    KERN_PFSTATUS,
    KERN_CONSDEV,
    KERN_UTC_OFFSET,
];

/// The `kern` directories `kern_sysctl_dirs` serves without the vslock (all registered).
const KERN_DIRS_UNLOCKED: [i32; 6] = [
    KERN_FILE,
    KERN_MALLOCSTATS,
    KERN_CPTIME2,
    KERN_POOL,
    KERN_CPUSTATS,
    KERN_AUDIO,
];

/// The `kern` directories `kern_sysctl_dirs_locked` serves whose subsystem registers them
/// (`kern.proc`, `proc_args`, `proc_cwd`, `proc_nobroadcastkill` and `proc_vmmap` are the
/// kernel's).
const KERN_DIRS_LOCKED: [i32; 6] = [
    KERN_INTRCNT,
    KERN_WATCHDOG,
    KERN_EVCOUNT,
    KERN_CLOCKINTR,
    KERN_TTY,
    KERN_TIMECOUNTER,
];

/// The `hw` nodes whose subsystem registers them: `machine` and `sensors` served without the
/// vslock, `ucomnames`, `smt` and `blockcpu` inside it.
const HW_REGISTERED: [i32; 5] = [HW_MACHINE_ID, HW_SENSORS, HW_UCOMNAMES, HW_SMT, HW_BLOCKCPU];

/// `HW_MACHINE`.
const HW_MACHINE_ID: i32 = hal::sys::sysctl::HW_MACHINE;

/// Whether a subsystem may register the `kern` node `id`: the dispatcher consults only these.
fn kern_registrable(id: i32) -> bool {
    KERN_LEAVES_UNLOCKED.contains(&id)
        || KERN_LEAVES_LOCKED.contains(&id)
        || KERN_DIRS_UNLOCKED.contains(&id)
        || KERN_DIRS_LOCKED.contains(&id)
}

/// `sysctl_bounded_arr(table, ...)` over the kernel's entries of a table and the ones its
/// subsystems registered: the C's one array, found by `mib`.
fn vars_arr(
    own: &[SysctlBoundedArgs],
    ext: &OnceStatic<&'static [SysctlBoundedArgs]>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    let [mib] = name else {
        return Err(Errno::ENOTDIR);
    };
    let ext: &[SysctlBoundedArgs] = ext.get().copied().unwrap_or(&[]);
    match own.iter().chain(ext).find(|v| v.mib == *mib) {
        Some(v) => int_bounded(req, v.var, v.minimum, v.maximum),
        None => Err(Errno::EOPNOTSUPP),
    }
}

/// `sysctl_bounded_arr(kern_vars, nitems(kern_vars), ...)`.
fn kern_vars(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    vars_arr(&KERN_VARS, &KERN_VARS_EXT, name, req)
}

/// `sysctl_bounded_arr(hw_vars, nitems(hw_vars), ...)`.
fn hw_vars(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    vars_arr(&HW_VARS, &HW_VARS_EXT, name, req)
}

/// `kern_sysctl`: kernel related system variables. The non-terminal nodes first
/// ([`kern_sysctl_dirs`]); then the leaves served without the vslock; then, vslocked, the
/// others ([`kern_sysctl_locked`]).
pub fn kern_sysctl<G: ProcGlue>(
    p: &Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    let [top, rest @ ..] = name else {
        return Err(Errno::EINVAL);
    };

    // dispatch the non-terminal nodes first
    if !rest.is_empty() {
        return kern_sysctl_dirs(p, *top, rest, req);
    }

    if let Some(r) = kern_leaf_unlocked(p, *top, name, req) {
        return r;
    }

    let _vs = sysctl_vslock(p, req.oldp, *req.oldlenp)?;
    kern_sysctl_locked(p, name, req)
}

/// The first `switch` of `kern_sysctl`: the leaf `id` served without the vslock, or `None`
/// for the ones `kern_sysctl_locked` serves (and for a registered leaf nobody registered:
/// the C's case compiled out, which falls through to the vslocked `default`).
fn kern_leaf_unlocked<G: ProcGlue>(
    p: &Thread<G>,
    id: i32,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Option<Result<(), Errno>> {
    Some(match id {
        KERN_CONSBUF | KERN_MSGBUF => {
            if id == KERN_CONSBUF
                && let Err(e) = suser(p)
            {
                return Some(Err(e));
            }
            let which = if id == KERN_MSGBUF {
                crate::log::Buf::Msgbuf
            } else {
                crate::log::Buf::Consbuf
            };
            crate::log::sysctl_msgbuf(which, req.oldp, req.oldlenp, req.newp)
        }
        KERN_CONSBUFSIZE | KERN_MSGBUFSIZE => {
            let which = if id == KERN_MSGBUFSIZE {
                crate::log::Buf::Msgbuf
            } else {
                crate::log::Buf::Consbuf
            };
            // deal with cases where the message buffer has become corrupted (ENXIO).
            match crate::log::msgbuf_size(which) {
                Ok(bufs) => rdint(req, bufs as i32),
                Err(e) => Err(e),
            }
        }
        // KERN_ALLOWDT: NDT is 0.
        KERN_HOSTID => int(req, &HOSTID),
        KERN_ALLOWKMEM => securelevel_int(req, &ALLOWKMEM),
        KERN_TIMEOUT_STATS => timeout_stats(req),
        KERN_TTYCOUNT => {
            // `kern_vars[]`'s read-only `tty_count`, the kernel's terminals' count
            // (`kernel::tty`, N4.7b): the entry served over a copy of it.
            let count = AtomicI32::new(crate::tty::tty_count::<G>());
            int_bounded(req, &count, SYSCTL_INT_READONLY.0, SYSCTL_INT_READONLY.1)
        }
        _ if KERN_VARS_UNLOCKED.contains(&id) => kern_vars(name, req),
        _ if KERN_LEAVES_UNLOCKED.contains(&id) => node(&KERN, id)?.sysctl(&[], req),
        _ => return None,
    })
}

/// `timeout_sysctl`: `kern.timeout_stats`, `struct timeoutstat` (twelve `uint64_t` in the
/// C's order) copied under the wheel's lock.
fn timeout_stats(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let mut status = [[0u8; 8]; 12];
    for (b, v) in status.iter_mut().zip(crate::timeout::stats()) {
        *b = v.to_ne_bytes();
    }
    rdstruct(req, status.as_flattened())
}

/// `kern_sysctl_locked`: the `kern` leaves served under `sysctl_lock`.
fn kern_sysctl_locked<G: ProcGlue>(
    p: &Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    match name[0] {
        KERN_SECURELVL => sysctl_securelevel(req, p.process().ps_pid),
        KERN_HOSTNAME => {
            let error = tstring(req, &mut *HOSTNAME.write());
            if req.newp != 0 && error.is_ok() {
                HOSTNAMELEN.store(req.newlen as i32, Ordering::Relaxed);
            }
            error
        }
        KERN_DOMAINNAME => {
            let error = if SECURELEVEL.load(Ordering::Relaxed) >= 1
                && DOMAINNAMELEN.load(Ordering::Relaxed) != 0
                && req.newp != 0
            {
                Err(Errno::EPERM)
            } else {
                tstring(req, &mut *DOMAINNAME.write())
            };
            if req.newp != 0 && error.is_ok() {
                DOMAINNAMELEN.store(req.newlen as i32, Ordering::Relaxed);
            }
            error
        }
        KERN_FORKSTAT => rdstruct(req, &forkstat_bytes()),
        id if KERN_LEAVES_LOCKED.contains(&id) => match node(&KERN, id) {
            Some(n) => n.sysctl(&[], req),
            None => kern_vars(name, req),
        },
        _ => kern_vars(name, req),
    }
}

/// `struct forkstat` as the C lays it out: four `int`s, then four `uint64_t`.
fn forkstat_bytes() -> [u8; 48] {
    let f = &crate::fork::FORKSTAT;
    let cnt = [
        f.cntfork.load(Ordering::Relaxed),
        f.cntvfork.load(Ordering::Relaxed),
        f.cnttfork.load(Ordering::Relaxed),
        f.cntkthread.load(Ordering::Relaxed),
    ];
    let siz = [
        f.sizfork.load(Ordering::Relaxed),
        f.sizvfork.load(Ordering::Relaxed),
        f.siztfork.load(Ordering::Relaxed),
        f.sizkthread.load(Ordering::Relaxed),
    ];
    let mut fs = [0u8; 48];
    fs[..16].copy_from_slice(bytes_of(&cnt));
    fs[16..].copy_from_slice(bytes_of(&siz));
    fs
}

/// `kern_sysctl_dirs`: the non-terminal `kern` nodes that need no lock, then the others under
/// `sysctl_lock` (a directory id nobody serves is vslocked before it answers `ENOTDIR`).
fn kern_sysctl_dirs<G: ProcGlue>(
    p: &Thread<G>,
    top_name: i32,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    if KERN_DIRS_UNLOCKED.contains(&top_name)
        && let Some(n) = node(&KERN, top_name)
    {
        return n.sysctl(name, req);
    }

    let _vs = sysctl_vslock(p, req.oldp, *req.oldlenp)?;
    kern_sysctl_dirs_locked(p, top_name, name, req)
}

/// `kern_sysctl_dirs_locked`.
fn kern_sysctl_dirs_locked<G: ProcGlue>(
    p: &Thread<G>,
    top_name: i32,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    match top_name {
        KERN_PROC => sysctl_doproc(p, name, req),
        KERN_PROC_ARGS => sysctl_proc_args(p, name, req),
        KERN_PROC_CWD => sysctl_proc_cwd(p, name, req),
        KERN_PROC_NOBROADCASTKILL => sysctl_proc_nobroadcastkill(p, name, req),
        KERN_PROC_VMMAP => sysctl_proc_vmmap(p, name, req),
        // KERN_PROF: GPROF and DDBPROF are not configured. KERN_WITNESSWATCH, KERN_WITNESS:
        // WITNESS is not configured.
        id if KERN_DIRS_LOCKED.contains(&id) => match node(&KERN, id) {
            Some(n) => n.sysctl(name, req),
            None => Err(Errno::ENOTDIR),
        },
        _ => Err(Errno::ENOTDIR), // overloaded
    }
}

/// `physmem`, in pages (0 before the machine registered it).
fn physmem() -> i64 {
    PHYSMEM
        .get()
        .map_or(0, |v| v.load(Ordering::Relaxed) as i64)
}

/// `hw_sysctl`: hardware related system variables.
pub fn hw_sysctl<G: ProcGlue>(
    p: &Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    let Some(&top) = name.first() else {
        return Err(Errno::EINVAL);
    };

    // all sysctl names at this level except sensors and battery are terminal
    if top != HW_SENSORS && top != HW_BATTERY && name.len() != 1 {
        return Err(Errno::ENOTDIR); // overloaded
    }

    let page = hal::arch::current::param::PAGE_SIZE as i64;
    let wired = i64::from(UVMEXP.wired.load(Ordering::Relaxed));

    match top {
        HW_MACHINE_ID | HW_SENSORS => match node(&HW, top) {
            Some(n) => n.sysctl(&name[1..], req),
            // The C's case compiled out: the default's table.
            None => hw_vars(name, req),
        },
        HW_MODEL => Err(crate::prf::unported!(
            "hw.model: cpu_model (amd64 identcpu.c, arm64 cpu.c)"
        )),
        HW_NCPUONLINE => rdint(req, crate::sched::ncpuonline::<G>() as i32),
        // ptoa(physmem): the C's int.
        HW_PHYSMEM => rdint(req, (physmem() * page) as i32),
        HW_USERMEM => rdint(req, ((physmem() - wired) * page) as i32),
        HW_DISKNAMES | HW_DISKSTATS | HW_CPUSPEED | HW_SETPERF | HW_PERFPOLICY | HW_BATTERY
        | HW_ALLOWPOWERDOWN | HW_UCOMNAMES | HW_SMT | HW_BLOCKCPU => {
            let _vs = sysctl_vslock(p, req.oldp, *req.oldlenp)?;
            hw_sysctl_locked(name, req)
        }
        HW_VENDOR => hw_string(&hw_vendor, req),
        HW_PRODUCT => hw_string(&hw_prod, req),
        HW_VERSION => hw_string(&hw_ver, req),
        HW_SERIALNO => hw_string(&hw_serial, req),
        HW_UUID => hw_string(&hw_uuid, req),
        HW_PHYSMEM64 => rdquad(req, physmem() * page),
        HW_USERMEM64 => rdquad(req, (physmem() - wired) * page),
        _ => hw_vars(name, req),
    }
}

/// The `HW_VENDOR`.. `HW_UUID` cases of `hw_sysctl`: the string, or `EOPNOTSUPP` when the
/// machine recorded none.
fn hw_string(var: &HwString, req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    match var.read() {
        Some(s) => rdstring(req, s),
        None => Err(Errno::EOPNOTSUPP),
    }
}

/// `hw_sysctl_locked`: the `hw` nodes served under `sysctl_lock`.
fn hw_sysctl_locked(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    match name[0] {
        HW_DISKNAMES => {
            sysctl_diskinit(false)?;
            let d = SYSCTL_DISKLOCK.read();
            rdstring(req, d.names.as_deref().unwrap_or(&[]))
        }
        HW_DISKSTATS => {
            sysctl_diskinit(true)?;
            let d = SYSCTL_DISKLOCK.read();
            let stats = d.stats.as_deref().unwrap_or(&[]);
            let count = DISK_EXPORT.get().map_or(0, |e| e.count());
            let len = (count * DISKSTATS_SIZE).min(stats.len());
            rdstruct(req, &stats[..len])
        }
        HW_CPUSPEED => {
            let Some(cpu_cpuspeed) = CPU_CPUSPEED.get() else {
                return Err(Errno::EOPNOTSUPP);
            };
            let mut cpuspeed = 0;
            cpu_cpuspeed(&mut cpuspeed)?;
            rdint(req, cpuspeed)
        }
        HW_SETPERF => Err(crate::prf::unported!(
            "hw.setperf: sysctl_hwsetperf (sched_bsd.c)"
        )),
        HW_PERFPOLICY => Err(crate::prf::unported!(
            "hw.perfpolicy: sysctl_hwperfpolicy (sched_bsd.c)"
        )),
        HW_ALLOWPOWERDOWN => securelevel_int(req, &ALLOWPOWERDOWN),
        // NUCOM 0: the empty string.
        HW_UCOMNAMES => match node(&HW, HW_UCOMNAMES) {
            Some(n) => n.sysctl(&[], req),
            None => rdstring(req, b""),
        },
        id @ (HW_SMT | HW_BLOCKCPU) => match node(&HW, id) {
            Some(n) => n.sysctl(&[], req),
            None => Err(Errno::EOPNOTSUPP),
        },
        HW_BATTERY => sysctl_hwbattery(&name[1..], req),
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// The common body of `sysctl_hwchargemode`, `sysctl_hwchargestart` and
/// `sysctl_hwchargestop`: `EOPNOTSUPP` before anything is copied when the driver has no
/// setter for `knob`.
fn sysctl_hwcharge(
    knob: BatteryKnob,
    var: &AtomicI32,
    minimum: i32,
    maximum: i32,
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    let Some(b) = BATTERY.get().copied().filter(|b| b.has(knob)) else {
        return Err(Errno::EOPNOTSUPP);
    };
    let value = AtomicI32::new(var.load(Ordering::Relaxed));

    int_bounded(req, &value, minimum, maximum)?;

    if req.newp != 0 {
        b.set(knob, value.into_inner())?;
    }
    Ok(())
}

/// `sysctl_hwchargemode`.
pub fn sysctl_hwchargemode(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_hwcharge(BatteryKnob::ChargeMode, &hw_battery_chargemode, -1, 1, req)
}

/// `sysctl_hwchargestart`.
pub fn sysctl_hwchargestart(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_hwcharge(
        BatteryKnob::ChargeStart,
        &hw_battery_chargestart,
        0,
        100,
        req,
    )
}

/// `sysctl_hwchargestop`.
pub fn sysctl_hwchargestop(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_hwcharge(BatteryKnob::ChargeStop, &hw_battery_chargestop, 0, 100, req)
}

/// `sysctl_hwbattery`: the `hw.battery` node.
pub fn sysctl_hwbattery(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let [mib] = name else {
        return Err(Errno::ENOTDIR);
    };

    match *mib {
        HW_BATTERY_CHARGEMODE => sysctl_hwchargemode(req),
        HW_BATTERY_CHARGESTART => sysctl_hwchargestart(req),
        HW_BATTERY_CHARGESTOP => sysctl_hwchargestop(req),
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// One `snprintf(disknames + l, disknameslen - l, "%s:%s,", name, duid)` of
/// `sysctl_diskinit`, then `l += strlen(disknames + l)`: appends at `l` what fits of the
/// entry, NUL-terminated, and returns the new `l`.
fn diskname_append(buf: &mut [u8], l: usize, name: &[u8], duid: &[u8]) -> usize {
    let Some(avail) = buf.len().checked_sub(l).filter(|&a| a > 0) else {
        return l;
    };
    let mut n = 0;
    for &c in name.iter().chain(b":").chain(duid).chain(b",") {
        if n + 1 >= avail {
            break;
        }
        buf[l + n] = c;
        n += 1;
    }
    buf[l + n] = 0;
    l + n
}

/// `sysctl_diskinit`: initialises `disknames`/`diskstats` for export by sysctl. If `update`
/// is set, then we simply update the disk statistics information. Under the kernel lock (the
/// caller's vslock).
pub fn sysctl_diskinit(update: bool) -> Result<(), Errno> {
    klock::kernel_assert_locked();

    let mut d = SYSCTL_DISKLOCK.write_flags(RW_INTR)?;
    let Some(exp) = DISK_EXPORT.get().copied() else {
        // No disk list is exported (no `bsd`): no disk, nothing to build.
        return Ok(());
    };

    let mut changed = false;

    // Run in a loop, disks may change while malloc sleeps.
    while exp.take_change() {
        let mut tlen = 0;
        exp.for_each(&mut |name, _, _| {
            tlen += name.len();
            tlen += 18; // label uid + separators
        });
        tlen += 1;
        // disk_count may change when malloc sleeps
        let count = exp.count();

        // The sysctl_disklock ensures that no other process can allocate disknames and
        // diskstats while our malloc sleeps.
        d.names = None;
        d.stats = None;
        d.stats = MArray::new(M_SYSCTL, M_WAITOK, count * DISKSTATS_SIZE, |_| 0);
        // disknames[0] = '\0': zeroed.
        d.names = MArray::new(M_SYSCTL, M_WAITOK, tlen, |_| 0);
        changed = true;
    }

    let DiskExports { names, stats } = &mut *d;
    let stats: &mut [u8] = stats.as_deref_mut().unwrap_or(&mut []);
    let mut sdk = stats.as_chunks_mut::<DISKSTATS_SIZE>().0.iter_mut();
    if changed {
        let names: &mut [u8] = names.as_deref_mut().unwrap_or(&mut []);
        let mut l = 0;
        exp.for_each(&mut |name, duid, ds| {
            l = diskname_append(names, l, name, duid.unwrap_or(&[]));
            if let Some(dst) = sdk.next() {
                let n = ds.len().min(DISKSTATS_SIZE);
                dst[..n].copy_from_slice(&ds[..n]);
            }
        });

        // Eliminate trailing comma
        if l != 0 {
            names[l - 1] = 0;
        }
    } else if update {
        // Just update, number of drives hasn't changed
        exp.for_each(&mut |_, _, ds| {
            if let Some(dst) = sdk.next() {
                let n = ds.len().min(DISKSTATS_SIZE);
                dst[..n].copy_from_slice(&ds[..n]);
            }
        });
    }
    Ok(())
}

/// `sysctl_doproc`: the `kern.proc` node, an array of `kinfo_proc` (`name` is op, arg,
/// element size, element count), for the calling thread `p`; under the vslock (the kernel
/// lock: the process lists are read with its reader).
pub fn sysctl_doproc<G: ProcGlue>(
    p: &Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    let where_ = req.oldp;
    let mut dp = where_;
    let mut buflen = if where_ != 0 { *req.oldlenp } else { 0 };
    let mut needed: usize = 0;

    let [op, arg, elem_size, elem_count] = *name else {
        return Err(Errno::EINVAL);
    };
    if elem_size <= 0 || elem_count < 0 || elem_size as usize > size_of::<KinfoProc>() {
        return Err(Errno::EINVAL);
    }
    let elem_size = elem_size as usize;
    let mut elem_count = elem_count;

    let dothreads = op & KERN_PROC_SHOW_THREADS != 0;
    let op = op & !KERN_PROC_SHOW_THREADS;

    let show_pointers = suser(p).is_ok();

    let mut kproc = KinfoProc::zeroed();
    // One element's bytes copied out at `dp`.
    let copy = |kproc: &KinfoProc, dp: usize| {
        UserPtr::<u8>::new(dp).copyout_slice(&bytes_of(core::slice::from_ref(kproc))[..elem_size])
    };

    // The lists under the kernel lock's reader (`sysctl_vslock` took the lock and wired the
    // user buffer, so the copyouts do not sleep); a process's threads under its `ps_mtx`, one
    // at a time, its entry filled under the mutex and copied out after it.
    let r = klock::reader();
    let t = G::tables();
    for list in [&t.allprocess, &t.zombprocess] {
        for pr in list.iter(&r) {
            // XXX skip processes in the middle of being created or zapped
            if pr.pgrp(&r).is_none() {
                continue;
            }

            // Skip embryonic processes.
            if pr.ps_flags.load(Ordering::Relaxed) & PS_EMBRYO != 0 {
                continue;
            }

            if !doproc_matches(&r, pr, op, arg)? {
                continue;
            }

            if buflen >= elem_size && elem_count > 0 {
                {
                    let g = pr.ps_mtx.lock();
                    fill_kproc(&r, pr, &mut kproc, None, show_pointers, &g);
                }
                copy(&kproc, dp)?;
                dp += elem_size;
                buflen -= elem_size;
                elem_count -= 1;
            }
            needed += elem_size;

            // Skip per-thread entries if not required by op
            if !dothreads {
                continue;
            }

            let mut i = 0;
            loop {
                let more = {
                    let g = pr.ps_mtx.lock();
                    match pr.ps_threads.iter(&g).nth(i) {
                        None => false,
                        Some(t) => {
                            if buflen >= elem_size && elem_count > 0 {
                                fill_kproc(&r, pr, &mut kproc, Some(t), show_pointers, &g);
                            }
                            true
                        }
                    }
                };
                if !more {
                    break;
                }
                if buflen >= elem_size && elem_count > 0 {
                    copy(&kproc, dp)?;
                    dp += elem_size;
                    buflen -= elem_size;
                    elem_count -= 1;
                }
                needed += elem_size;
                i += 1;
            }
        }
    }

    if where_ != 0 {
        *req.oldlenp = dp - where_;
        if needed > *req.oldlenp {
            return Err(Errno::ENOMEM);
        }
    } else {
        needed += KERN_PROCSLOP * elem_size;
        *req.oldlenp = needed;
    }
    Ok(())
}

/// The `switch (op)` of `sysctl_doproc`: whether `pr` is selected; `EINVAL` for an unknown
/// op.
fn doproc_matches<G: ProcGlue>(
    r: &KernelReader,
    pr: &Process<G>,
    op: i32,
    arg: i32,
) -> Result<bool, Errno> {
    // A process in a list has its pgrp and session until process_zap, and the caller
    // skipped the ones without a pgrp.
    let sess = pr.session(r);
    let flags = pr.ps_flags.load(Ordering::Relaxed);
    Ok(match op {
        // could do this with just a lookup
        KERN_PROC_PID => pr.ps_pid == arg,
        // could do this by traversing pgrp
        KERN_PROC_PGRP => pr.pgid(r) == arg,
        KERN_PROC_SESSION => sess.and_then(|s| s.leader_pid()) == Some(arg),
        KERN_PROC_TTY => {
            flags & PS_CONTROLT != 0
                && sess
                    .and_then(G::session_tty)
                    .is_some_and(|tty| tty.dev == arg as Dev)
        }
        KERN_PROC_UID => pr.ucred().cr_uid.get() == arg as u32,
        KERN_PROC_RUID => pr.ucred().cr_ruid.get() == arg as u32,
        KERN_PROC_ALL => flags & PS_SYSTEM == 0,
        // no filtering
        KERN_PROC_KTHREAD => true,
        _ => return Err(Errno::EINVAL),
    })
}

/// `_getcompatprio` (`<sys/sysctl.h>`): the priority `ps(1)` shows.
fn getcompatprio<G: ProcGlue>(p: &Thread<G>) -> u8 {
    match p.stat() {
        SRUN => p.p_runpri.get(),
        SSLEEP => p.p_slppri.get(),
        _ => p.p_usrpri.get(),
    }
}

/// `W_EXITCODE` (`<sys/wait.h>`).
const fn w_exitcode(ret: u32, sig: i32) -> u32 {
    (ret << 8) | sig as u32
}

/// `fill_kproc`: fills in a `kinfo_proc` for process `pr`, or for its thread `p`, under the
/// kernel lock's reader (`r`) and the process's `ps_mtx` (`g`, the C's `PR_LOCK` taken for
/// the whole fill: the thread list is the mutex's). The kernel addresses (`p_paddr`,
/// `p_fd`, ..., `p_tsess`) only when `show_pointers` (the caller is root).
pub fn fill_kproc<G: ProcGlue>(
    r: &KernelReader,
    pr: &Process<G>,
    ki: &mut KinfoProc,
    p: Option<&Thread<G>>,
    show_pointers: bool,
    g: &MutexedGuard<'_, PsMtx<G>>,
) {
    let flags = pr.ps_flags.load(Ordering::Relaxed);
    // The callers skip processes without a pgrp; a group has a session.
    let pg = pr.pgrp(r);
    let s = pg.and_then(|pg| pg.session(r));

    // exiting/zombie process might no longer have VM space.
    let mut vm: Option<VmspaceRef> = None;
    if flags & PS_EXITING == 0 {
        // vm = pr->ps_vmspace; uvmspace_addref(vm): a process that is not exiting holds
        // its vmspace, and the reference keeps it while it is read. The handle is the C's
        // pointer, not a reference: the `uvmspace_free` below gives back the one taken here.
        vm = g.vmspace.clone();
        if let Some(vm) = &vm {
            crate::vm::map::uvmspace_addref(vm);
        }
    }

    let tu = Tusage::new();
    let isthread = p.is_some();
    let p: &Thread<G> = match p {
        Some(p) => {
            crate::resource::tuagg_get_proc(&tu, p);
            p
        }
        None => {
            // p = pr->ps_mainproc: the process's main thread, its slot's, under the
            // caller's ps_mtx (a process keeps it until process_zap).
            let Some(mainp) = pr.main_thread(g) else {
                kpanic(format_args!("fill_kproc: process without a main thread"));
            };
            // tuagg_get_process(&tu, pr), under the caller's ps_mtx.
            tu.clear();
            crate::resource::tuagg_sumup(&tu, &pr.ps_tu);
            for t in pr.ps_threads.iter(g) {
                crate::resource::tuagg_sumup(&tu, &t.p_tu);
            }
            mainp.get_ref()
        }
    };
    let uc = g.ucred();

    // FILL_KPROC(ki, strlcpy, p, pr, pr->ps_ucred, pr->ps_pgrp, p, pr, s, vm,
    //     pr->ps_limit, pr->ps_sigacts, &tu, isthread, show_pointers)
    *ki = KinfoProc::zeroed();

    if show_pointers {
        ki.p_paddr = ptrtoint64(p);
        ki.p_fd = pr.fd_opt().map_or(0, |fdp| fdp.addr() as u64);
        ki.p_limit = pr.limit_word() as u64;
        ki.p_vmspace = pr.vmspace_word() as u64;
        ki.p_sigacts = pr.sigacts_opt().map_or(0, ptrtoint64);
        ki.p_sess = s.map_or(0, ptrtoint64);
        ki.p_ru = pr
            .ps_ru
            .get(r)
            .as_ref()
            .map_or(0, |ru| ptrtoint64::<crate::resource::Rusage>(ru));
    }
    ki.p_stats = 0;
    ki.p_exitsig = 0;
    ki.p_flag = p.p_flag.load(Ordering::Relaxed);
    ki.p_pid = pr.ps_pid;
    ki.p_psflags = flags;

    ki.p__pgid = pg.map_or(0, |pg| pg.pg_id.get());

    ki.p_uid = uc.cr_uid.get();
    ki.p_ruid = uc.cr_ruid.get();
    ki.p_gid = uc.cr_gid.get();
    ki.p_rgid = uc.cr_rgid.get();
    ki.p_svuid = uc.cr_svuid.get();
    ki.p_svgid = uc.cr_svgid.get();

    for (dst, gid) in ki.p_groups.iter_mut().zip(&uc.cr_groups) {
        *dst = gid.get();
    }
    ki.p_ngroups = uc.cr_ngroups.get();

    ki.p_jobc = pg.map_or(0, |pg| pg.pg_jobc.get()) as i16;

    ki.p_estcpu = p.p_estcpu.get();
    if isthread {
        ki.p_tid = p.p_tid.get() + THREAD_PID_OFFSET;
        libkern::strlcpy(&mut ki.p_name, &p.name());
    } else {
        ki.p_tid = -1;
    }
    let runtime = tu.tu_runtime.get();
    ki.p_rtime_sec = runtime.tv_sec as u32;
    ki.p_rtime_usec = (runtime.tv_nsec / 1000) as u32;
    ki.p_uticks = tu.tu_ticks[TU_UTICKS].get();
    ki.p_sticks = tu.tu_ticks[TU_STICKS].get();
    ki.p_iticks = tu.tu_ticks[TU_ITICKS].get();
    ki.p_cpticks = p.p_cpticks.get() as i32;

    // p_tracep, p_traceflag: KTRACE is not configured.

    ki.p_siglist =
        (p.p_siglist.load(Ordering::Relaxed) | pr.ps_siglist.load(Ordering::Relaxed)) as i32;
    ki.p_sigmask = p.p_sigmask.get();

    // PR_LOCK(pr): the caller's.
    ki.p_ppid = pr.ps_ppid.get();
    if let Some(sa) = pr.sigacts_opt() {
        let m = sa.get(g);
        ki.p_sigignore = m.ps_sigignore;
        ki.p_sigcatch = m.ps_sigcatch;
    }

    // A process holds a reference to its limits; ps_mtx keeps them from changing.
    if let Some(lim) = g.limit.as_ref() {
        ki.p_rlim_rss_cur = lim.pl_rlimit[RLIMIT_RSS].rlim_cur;
    }
    // PR_UNLOCK(pr): the caller's.

    ki.p_stat = p.stat() as i8;
    ki.p_nice = pr.ps_nice.get();

    ki.p_xstat = w_exitcode(pr.ps_xexit.get(), pr.ps_xsig.get()) as u16;
    ki.p_acflag = pr.ps_acflag.get();
    ki.p_pledge = pr.ps_pledge.load(Ordering::Relaxed);

    libkern::strlcpy(&mut ki.p_emul, b"native");
    libkern::strlcpy(&mut ki.p_comm, &pr.comm());
    if let Some(s) = s {
        libkern::strlcpy(&mut ki.p_login, s.s_login.get(r));

        if G::session_has_tty_vnode(s) {
            ki.p_eflag |= EPROC_CTTY;
        }
    }
    let uv = G::unveil_state(pr);
    if uv.paths {
        ki.p_eflag |= EPROC_UNVEIL;
    }
    if uv.done
        || (flags & PS_PLEDGE != 0 && pr.ps_pledge.load(Ordering::Relaxed) & PLEDGE_UNVEIL == 0)
    {
        ki.p_eflag |= EPROC_LKUNVEIL;
    }

    if flags & (PS_EMBRYO | PS_ZOMBIE) == 0 {
        if let Some(vm) = vm.as_deref() {
            ki.p_vm_rssize = vm.vm_rssize.load(Ordering::Relaxed);
            ki.p_vm_tsize = vm.vm_tsize.load(Ordering::Relaxed);
            ki.p_vm_dsize = vm.vm_dused.load(Ordering::Relaxed);
            ki.p_vm_ssize = vm.vm_ssize.load(Ordering::Relaxed);
        }
        ki.p_stat = p.stat() as i8;
        ki.p_slptime = p.p_slptime.get();
        ki.p_holdcnt = 1;
        ki.p_priority = getcompatprio(p);
        ki.p_usrpri = p.p_usrpri.get();
        if p.p_wchan.get() != 0
            && let Some(wmesg) = p.p_wmesg.get()
        {
            libkern::strlcpy(&mut ki.p_wmesg, wmesg.as_bytes());
        }
        if show_pointers {
            ki.p_wchan = p.p_wchan.get() as u64;
            ki.p_addr = p.addr_word() as u64;
        }
    }

    if flags & PS_ZOMBIE == 0 {
        ki.p_uvalid = 1;

        let ru = &p.p_ru;
        ki.p_uru_maxrss = ru.ru_maxrss.get() as u64;
        ki.p_uru_ixrss = ru.ru_ixrss.get() as u64;
        ki.p_uru_idrss = ru.ru_idrss.get() as u64;
        ki.p_uru_isrss = ru.ru_isrss.get() as u64;
        ki.p_uru_minflt = ru.ru_minflt.get() as u64;
        ki.p_uru_majflt = ru.ru_majflt.get() as u64;
        ki.p_uru_nswap = ru.ru_nswap.get() as u64;
        ki.p_uru_inblock = ru.ru_inblock.get() as u64;
        ki.p_uru_oublock = ru.ru_oublock.get() as u64;
        ki.p_uru_msgsnd = ru.ru_msgsnd.get() as u64;
        ki.p_uru_msgrcv = ru.ru_msgrcv.get() as u64;
        ki.p_uru_nsignals = ru.ru_nsignals.get() as u64;
        ki.p_uru_nvcsw = ru.ru_nvcsw.get() as u64;
        ki.p_uru_nivcsw = ru.ru_nivcsw.get() as u64;

        let tv = pr.ps_cru.ru_utime.get().add(pr.ps_cru.ru_stime.get());
        ki.p_uctime_sec = tv.tv_sec as u32;
        ki.p_uctime_usec = tv.tv_usec as u32;
    }

    ki.p_cpuid = KI_NOCPU;
    ki.p_rtableid = pr.ps_rtableid.load(Ordering::Relaxed);
    // end of FILL_KPROC

    // stuff that's too painful to generalize into the macros
    if let Some(leader) = s.and_then(|s| s.leader_pid()) {
        ki.p_sid = leader;
    }

    match s.and_then(G::session_tty) {
        Some(tty) if flags & PS_CONTROLT != 0 => {
            ki.p_tdev = tty.dev as u32;
            ki.p_tpgid = tty.pgid.unwrap_or(-1);
            // A kernel address: root's only, as every other pointer here (the C's
            // `if (show_pointers)`; LZ wrote it for every caller).
            if show_pointers {
                ki.p_tsess = tty.session_addr as u64;
            }
        }
        _ => {
            ki.p_tdev = NODEV as u32;
            ki.p_tpgid = -1;
        }
    }

    // fixups that can only be done in the kernel
    if flags & PS_EXITING == 0 {
        if flags & PS_EMBRYO == 0
            && let Some(vm) = vm.as_deref()
        {
            ki.p_vm_rssize = vm.vm_map.pmap().resident_count() as i32;
        }
        let (ut, st, _) = crate::resource::calctsru(&tu, G::stathz());
        ki.p_uutime_sec = ut.tv_sec as u32;
        ki.p_uutime_usec = (ut.tv_nsec / 1000) as u32;
        ki.p_ustime_sec = st.tv_sec as u32;
        ki.p_ustime_usec = (st.tv_nsec / 1000) as u32;

        // Convert starting uptime to a starting UTC time.
        let utc = crate::clock::nanoboottime().add(pr.ps_start.get());
        ki.p_ustart_sec = utc.tv_sec as u64;
        ki.p_ustart_usec = (utc.tv_nsec / 1000) as u32;

        // if (p->p_cpu != NULL) ki->p_cpuid = CPU_INFO_UNIT(p->p_cpu)
        if let Some(unit) = crate::sched::thread_cpu(p).and_then(G::cpu_unit) {
            ki.p_cpuid = unit;
        }
    }

    if let Some(vm) = vm {
        crate::vm::map::uvmspace_free(vm);
    }

    // get %cpu and schedule state: just one thread or sum of all?
    if isthread {
        ki.p_pctcpu = p.p_pctcpu.load(Ordering::Relaxed);
        ki.p_stat = p.stat() as i8;
    } else {
        ki.p_pctcpu = 0;
        let mut stat = if flags & PS_EXITING != 0 { SDEAD } else { SIDL };
        for t in pr.ps_threads.iter(g) {
            ki.p_pctcpu += t.p_pctcpu.load(Ordering::Relaxed);
            // find best state: ONPROC > RUN > STOP > SLEEP > ..
            let ts = t.stat();
            if ts == SONPROC || stat == SONPROC {
                stat = SONPROC;
            } else if ts == SRUN || stat == SRUN {
                stat = SRUN;
            } else if ts == SSTOP || stat == SSTOP {
                stat = SSTOP;
            } else if ts == SSLEEP {
                stat = SSLEEP;
            }
        }
        ki.p_stat = stat as i8;
    }
}

/// `pr`, counted, found by `pid` under the kernel lock's reader (`prfind` with the
/// reference the C's callers keep across their sleeps).
fn prfind_ref<G: ProcGlue>(pid: Pid) -> Option<crate::proc::ProcessRef<G>> {
    let r = klock::reader();
    prfind::<G, _>(&r, pid).map(G::process_ref)
}

/// `sizeof(struct ps_strings)`: `ps_argvstr`, `ps_nargvstr`, `ps_envstr`, `ps_nenvstr`, each
/// in a pointer-sized slot.
const PS_STRINGS_SIZE: usize = 4 * size_of::<usize>();

/// `sysctl_proc_args`: `kern.procargs.<pid>.<op>`: the victim's argument or environment
/// strings (or their count), read from its address space with `uvm_io`, for the calling
/// thread `cp`.
pub fn sysctl_proc_args<G: ProcGlue>(
    cp: &Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    if name.len() > 2 {
        return Err(Errno::ENOTDIR);
    }
    let [pid, op] = *name else {
        return Err(Errno::EINVAL);
    };

    match op {
        KERN_PROC_ARGV | KERN_PROC_NARGV | KERN_PROC_ENV | KERN_PROC_NENV => {}
        _ => return Err(Errno::EOPNOTSUPP),
    }

    let Some(vpr) = prfind_ref::<G>(pid) else {
        return Err(Errno::ESRCH);
    };

    if req.oldp == 0 {
        *req.oldlenp = if op == KERN_PROC_NARGV || op == KERN_PROC_NENV {
            size_of::<i32>()
        } else {
            ARG_MAX // XXX XXX XXX
        };
        return Ok(());
    }

    let flags = vpr.ps_flags.load(Ordering::Relaxed);
    // Either system process or exiting/zombie
    if flags & (PS_SYSTEM | PS_EXITING) != 0 {
        return Err(Errno::EINVAL);
    }

    // Execing - danger.
    if flags & PS_INEXEC != 0 {
        return Err(Errno::EBUSY);
    }

    // Only owner or root can get env
    if (op == KERN_PROC_NENV || op == KERN_PROC_ENV)
        && vpr.ucred().cr_uid.get() != cp.ucred().cr_uid.get()
    {
        suser(cp)?;
    }

    let ps_strings = vpr.ps_strings.get();
    // vm = vpr->ps_vmspace; uvmspace_addref(vm) (gone already: exiting since the check);
    // the handle is the C's pointer, the reference the `uvmspace_free`s below give back.
    let Some(vm) = vpr.vmspace_share() else {
        return Err(Errno::EINVAL);
    };
    crate::vm::map::uvmspace_addref(&vm);
    drop(vpr);

    let Some(mut buf) = MArray::new(M_TEMP, M_WAITOK, PAGE_SIZE, |_| 0u8) else {
        crate::vm::map::uvmspace_free(vm);
        return Err(Errno::ENOMEM);
    };

    let error = proc_args_copy(&vm.vm_map, ps_strings, op, req, &mut buf);

    crate::vm::map::uvmspace_free(vm);
    drop(buf);
    error
}

/// `PAGE_SIZE` of the machine.
const PAGE_SIZE: usize = hal::arch::current::param::PAGE_SIZE;
/// `PAGE_MASK` of the machine.
const PAGE_MASK: usize = hal::arch::current::param::PAGE_MASK;

/// `uvm_io`'s uio for a read of another address space into a kernel buffer (the C's
/// `UIO_SYSSPACE`, `UIO_READ` uio over one iovec): `uiomove` fills the buffer in order.
struct BufRead<'a> {
    /// The buffer.
    buf: &'a mut [u8],
    /// The bytes moved so far.
    done: usize,
}

impl UioMove for BufRead<'_> {
    fn resid(&self) -> usize {
        self.buf.len() - self.done
    }

    fn uiomove(&mut self, src: &mut [u8]) -> Result<(), Errno> {
        let n = src.len().min(self.resid());
        self.buf[self.done..self.done + n].copy_from_slice(&src[..n]);
        self.done += n;
        Ok(())
    }

    fn nonblock(&self) -> bool {
        false
    }

    fn is_write(&self) -> bool {
        false
    }

    fn set_resid(&mut self, resid: usize) {
        // `uio_resid = resid`: the bytes still to fill are the buffer's last `resid`.
        self.done = self.buf.len().saturating_sub(resid);
    }
}

/// `dst.len()` bytes of the victim's address space `map` at `va` (`uvm_io` of a read).
fn victim_read(map: &VmMap, va: usize, dst: &mut [u8]) -> Result<(), Errno> {
    crate::vm::io::uvm_io(map, &mut BufRead { buf: dst, done: 0 }, va, 0)
}

/// The body of `sysctl_proc_args` past its checks (its `goto out` paths are the `?`s; the
/// caller drops the vmspace reference and the buffer): reads the victim's `ps_strings`, then
/// lays out an `argv`-style array of `cnt` pointers, a NULL and the strings in the reader's
/// buffer at `oldp`.
fn proc_args_copy(
    map: &VmMap,
    ps_strings: usize,
    op: i32,
    req: &mut SysctlReq<'_>,
    buf: &mut [u8],
) -> Result<(), Errno> {
    const PTR: usize = size_of::<usize>();
    let out = |b: &[u8], uaddr: usize| UserPtr::<u8>::new(uaddr).copyout_slice(b);

    let mut pss = [0u8; PS_STRINGS_SIZE];
    victim_read(map, ps_strings, &mut pss)?;
    let word = |b: &[u8]| {
        let mut w = [0u8; PTR];
        w.copy_from_slice(&b[..PTR]);
        usize::from_ne_bytes(w)
    };
    let int = |b: &[u8]| {
        let mut w = [0u8; 4];
        w.copy_from_slice(&b[..4]);
        i32::from_ne_bytes(w)
    };
    // struct ps_strings: ps_argvstr, ps_nargvstr, ps_envstr, ps_nenvstr.
    let (argvstr, nargvstr) = (word(&pss[0..]), int(&pss[PTR..]));
    let (envstr, nenvstr) = (word(&pss[2 * PTR..]), int(&pss[3 * PTR..]));

    // sysctl_rdint(oldp, oldlenp, NULL, n): the count, whatever new value came.
    if op == KERN_PROC_NARGV || op == KERN_PROC_NENV {
        let n = if op == KERN_PROC_NARGV {
            nargvstr
        } else {
            nenvstr
        };
        let mut count = SysctlReq {
            oldp: req.oldp,
            oldlenp: &mut *req.oldlenp,
            newp: 0,
            newlen: 0,
        };
        return rdint(&mut count, n);
    }

    let (cnt, mut vargv) = if op == KERN_PROC_ARGV {
        (nargvstr, argvstr)
    } else {
        (nenvstr, envstr)
    };

    // Clamp to avoid overflow, using ARG_MAX is only an approximation. It is not possible
    // to execve() with this many elements, so this only happens if a process has changed
    // its strings. A hard cap, so not ENOMEM: the caller cannot retry. (The C's count is
    // unsigned: a negative one is over the cap too.)
    if cnt < 0 || cnt as usize > ARG_MAX {
        return Err(Errno::EINVAL);
    }
    let mut cnt = cnt as usize;

    let oldlenp = &mut *req.oldlenp;
    // -1 to have space for a terminating NUL
    let limit = oldlenp.wrapping_sub(1);
    *oldlenp = 0;

    // *oldlenp: bytes copied out into the reader's buffer; limit: the most allowed there;
    // rarg: where the next string goes in the reader's buffer; rargv: where the next rarg
    // pointer goes; vargv: where the next argument pointer is read in the victim.
    let mut rargv = req.oldp;
    // space for cnt pointers and a NULL
    let mut rarg = rargv + (cnt + 1) * PTR;
    *oldlenp += (cnt + 1) * PTR;

    while cnt > 0 && *oldlenp < limit {
        // Write to the reader's argv.
        out(&rarg.to_ne_bytes(), rargv)?;

        // Read the victim's argv.
        let mut vargb = [0u8; PTR];
        victim_read(map, vargv, &mut vargb)?;
        let mut varg = usize::from_ne_bytes(vargb);
        if varg == 0 {
            break;
        }

        // Read the victim's string a page at a time, so as not to cross a page boundary
        // too much and return an error.
        loop {
            let len = PAGE_SIZE - (varg & PAGE_MASK);
            victim_read(map, varg, &mut buf[..len])?;
            let vstrlen = buf[..len].iter().position(|&c| c == 0).unwrap_or(len);

            // Don't overflow the reader's buffer.
            if *oldlenp + vstrlen + 1 >= limit {
                return Err(Errno::ENOMEM);
            }
            out(&buf[..vstrlen], rarg)?;
            *oldlenp += vstrlen;
            rarg += vstrlen;

            // The string didn't end in this page?
            if vstrlen == len {
                varg += vstrlen;
                continue;
            }
            break;
        }

        // End of string. Terminate it with a NUL.
        out(&[0], rarg)?;
        *oldlenp += 1;
        rarg += 1;

        vargv += PTR;
        rargv += PTR;
        cnt -= 1;
    }

    if *oldlenp >= limit {
        return Err(Errno::ENOMEM);
    }

    // Write the terminating null.
    out(&0usize.to_ne_bytes(), rargv)
}

/// `sysctl_proc_cwd`: `kern.proc_cwd.<pid>`, the process's current directory, for the
/// calling thread `cp`.
pub fn sysctl_proc_cwd<G: ProcGlue>(
    cp: &Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    if name.len() > 1 {
        return Err(Errno::ENOTDIR);
    }
    let [pid] = *name else {
        return Err(Errno::EINVAL);
    };

    let Some(findpr) = prfind_ref::<G>(pid) else {
        return Err(Errno::ESRCH);
    };

    if req.oldp == 0 {
        *req.oldlenp = MAXPATHLEN * 4;
        return Ok(());
    }

    // Either system process or exiting/zombie
    if findpr.ps_flags.load(Ordering::Relaxed) & (PS_SYSTEM | PS_EXITING) != 0 {
        return Err(Errno::EINVAL);
    }

    // Only owner or root can get cwd
    if findpr.ucred().cr_uid.get() != cp.ucred().cr_uid.get() {
        suser(cp)?;
    }

    let mut len = *req.oldlenp;
    if len > MAXPATHLEN * 4 {
        len = MAXPATHLEN * 4;
    } else if len < 2 {
        return Err(Errno::ERANGE);
    }
    *req.oldlenp = 0;

    // snag a reference to the vnode before we can sleep
    let Some(vp) = findpr.fd_opt().and_then(|fdp| fdp.cdir()) else {
        // No current directory before a root file system is mounted.
        return Err(Errno::ENOENT);
    };
    <G::FileGlue as crate::file::FileGlue>::vref(vp);
    drop(findpr);

    let Some(mut path) = MArray::new(M_TEMP, M_WAITOK, len, |_| 0u8) else {
        <G::FileGlue as crate::file::FileGlue>::vrele(vp);
        return Err(Errno::ENOMEM);
    };

    let bp = len - 1;
    path[bp] = 0;

    // Same as sys__getcwd
    let mut error =
        <G::FileGlue as crate::file::FileGlue>::getcwd(vp, &mut path, bp, (len / 2) as i32, cp);
    if let Ok(bp) = error {
        let lenused = len - bp;
        *req.oldlenp = lenused;
        error = UserPtr::<u8>::new(req.oldp)
            .copyout_slice(&path[bp..])
            .map(|()| bp);
    }

    <G::FileGlue as crate::file::FileGlue>::vrele(vp);
    drop(path);

    error.map(|_| ())
}

/// `sysctl_proc_nobroadcastkill`: `kern.proc_nobroadcastkill.<pid>`, the process's
/// `PS_NOBROADCASTKILL` flag, for the calling thread `cp`.
pub fn sysctl_proc_nobroadcastkill<G: ProcGlue>(
    cp: &Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    if name.len() > 1 {
        return Err(Errno::ENOTDIR);
    }
    let [pid] = *name else {
        return Err(Errno::EINVAL);
    };

    let Some(findpr) = prfind_ref::<G>(pid) else {
        return Err(Errno::ESRCH);
    };

    // Either system process or exiting/zombie
    if findpr.ps_flags.load(Ordering::Relaxed) & (PS_SYSTEM | PS_EXITING) != 0 {
        return Err(Errno::EINVAL);
    }

    // Only root can change PS_NOBROADCASTKILL
    if req.newp != 0 {
        suser(cp)?;
    }

    // get the PS_NOBROADCASTKILL flag
    let flag = AtomicI32::new(i32::from(
        findpr.ps_flags.load(Ordering::Relaxed) & PS_NOBROADCASTKILL != 0,
    ));

    let error = int(req, &flag);
    if error.is_ok() && req.newp != 0 {
        if flag.into_inner() != 0 {
            findpr
                .ps_flags
                .fetch_or(PS_NOBROADCASTKILL, Ordering::Relaxed);
        } else {
            findpr
                .ps_flags
                .fetch_and(!PS_NOBROADCASTKILL, Ordering::Relaxed);
        }
    }

    error
}

/// `sysctl_proc_vmmap`: `kern.proc_vmmap.<pid>`, the address space as `kinfo_vmentry`s,
/// for the calling thread `cp`. The checks are the C's; `fill_vmmap` is reported.
pub fn sysctl_proc_vmmap<G: ProcGlue>(
    cp: &Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    if name.len() > 1 {
        return Err(Errno::ENOTDIR);
    }
    let [pid] = *name else {
        return Err(Errno::EINVAL);
    };

    // Provide max buffer length as hint (oldlenp is never NULL here).
    if req.oldp == 0 {
        *req.oldlenp = VMMAP_MAXLEN;
        return Ok(());
    }

    if pid == cp.process().ps_pid {
        // Self process mapping.
    } else if pid > 0 {
        let Some(findpr) = prfind_ref::<G>(pid) else {
            return Err(Errno::ESRCH);
        };

        // Either system process or exiting/zombie
        if findpr.ps_flags.load(Ordering::Relaxed) & (PS_SYSTEM | PS_EXITING) != 0 {
            return Err(Errno::EINVAL);
        }

        // XXX Allow only root for now
        suser(cp)?;
    } else {
        // Only root can get kernel_map
        suser(cp)?;
    }

    // Check the given size.
    let oldlen = *req.oldlenp;
    if oldlen == 0 || !oldlen.is_multiple_of(size_of::<KinfoVmentry>()) {
        return Err(Errno::EINVAL);
    }

    // Deny huge allocation.
    if oldlen > VMMAP_MAXLEN {
        return Err(Errno::EINVAL);
    }

    // Iterate from the given address passed as the first element's kve_start via oldp.
    let _start: u64 = UserPtr::<u64>::new(req.oldp).copyin()?;

    Err(crate::prf::unported!(
        "kern.proc_vmmap: fill_vmmap (uvm_glue.c)"
    ))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::{Mutex as StdMutex, Once};
    use std::vec::Vec;

    use hal::pool::PoolBox;
    use hal::sys::sysctl::{
        CTL_DEBUG, HW_NCPU, KERN_PROC_SHOW_THREADS, KinfoFile, SYSCTL_INT_READONLY,
    };
    use hal::sys::types::Uid;

    use super::*;
    use crate::cred::{CredRef, Ucred};
    use crate::proc::{ProcessRef, SessionTty, UnveilState};
    use crate::testutil;

    /// A user buffer of the double's memory (its own range).
    const UBUF: usize = testutil::USER_BASE + 0x9000;

    /// This module's range of the user-copy double: 0x5000..0x7000.
    const U: usize = testutil::USER_BASE + 0x5000;

    #[test]
    fn rdquad_and_rdstruct_answer_as_the_c() {
        testutil::install();
        // A size query: no buffer, the length comes back.
        let mut len = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(rdquad(&mut req, 7), Ok(()));
        assert_eq!(len, 8);
        // Too little room.
        let mut len = 4;
        let mut req = SysctlReq {
            oldp: UBUF,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(rdquad(&mut req, 7), Err(Errno::ENOMEM));
        // A new value for a read-only leaf.
        let mut len = 8;
        let mut req = SysctlReq {
            oldp: UBUF,
            oldlenp: &mut len,
            newp: UBUF,
            newlen: 8,
        };
        assert_eq!(rdquad(&mut req, 7), Err(Errno::EPERM));
        // The value copied out.
        let mut len = 16;
        let mut req = SysctlReq {
            oldp: UBUF,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(rdstruct(&mut req, b"abc"), Ok(()));
        assert_eq!(len, 3);
        assert_eq!(testutil::user_bytes(UBUF, 3).unwrap(), b"abc");
    }

    /// A request: `old` (0 for none) with room `len`, `new` (0 for none) of `newlen` bytes.
    fn req(old: usize, len: &mut usize, new: usize, newlen: usize) -> SysctlReq<'_> {
        SysctlReq {
            oldp: old,
            oldlenp: len,
            newp: new,
            newlen,
        }
    }

    /// `bytes` put at the user address `at`.
    fn put(at: usize, bytes: &[u8]) {
        testutil::install();
        UserPtr::<u8>::new(at).copyout_slice(bytes).unwrap();
    }

    /// `n` bytes at the user address `at`.
    fn get(at: usize, n: usize) -> Vec<u8> {
        testutil::user_bytes(at, n).unwrap()
    }

    /// The `int` at the user address `at`.
    fn get_int(at: usize) -> i32 {
        UserPtr::<i32>::new(at).copyin().unwrap()
    }

    #[test]
    fn rdint_sizes_reads_and_refuses_writes() {
        let out = U;
        put(out, &[0; 4]);
        let mut len = 0;
        assert_eq!(rdint(&mut req(0, &mut len, 0, 0), 42), Ok(()));
        assert_eq!(len, 4);

        let mut len = 3;
        assert_eq!(rdint(&mut req(out, &mut len, 0, 0), 42), Err(Errno::ENOMEM));
        assert_eq!(len, 3);

        let mut len = 4;
        assert_eq!(rdint(&mut req(out, &mut len, 0, 0), 42), Ok(()));
        assert_eq!(get_int(out), 42);

        put(out + 8, &7i32.to_ne_bytes());
        assert_eq!(
            rdint(&mut req(0, &mut len, out + 8, 4), 42),
            Err(Errno::EPERM)
        );
    }

    #[test]
    fn int_swaps_the_old_value_out() {
        let (out, new) = (U + 0x40, U + 0x48);
        let var = AtomicI32::new(5);
        put(new, &(-9i32).to_ne_bytes());
        let mut len = 4;
        assert_eq!(int(&mut req(out, &mut len, new, 4), &var), Ok(()));
        assert_eq!((get_int(out), var.load(Ordering::Relaxed), len), (5, -9, 4));

        // A new value of the wrong size is EINVAL, and nothing changes.
        assert_eq!(int(&mut req(0, &mut len, new, 2), &var), Err(Errno::EINVAL));
        // No old buffer: the value is just stored.
        put(new, &11i32.to_ne_bytes());
        assert_eq!(int(&mut req(0, &mut len, new, 4), &var), Ok(()));
        assert_eq!(var.load(Ordering::Relaxed), 11);
    }

    #[test]
    fn int_bounded_checks_both_bounds_and_read_only() {
        let (too_big, too_small, fine, out) = (U + 0x80, U + 0x84, U + 0x88, U + 0x8c);
        put(too_big, &101i32.to_ne_bytes());
        put(too_small, &(-1i32).to_ne_bytes());
        put(fine, &100i32.to_ne_bytes());
        let var = AtomicI32::new(50);
        let mut len = 4;
        assert_eq!(
            int_bounded(&mut req(0, &mut len, too_big, 4), &var, 0, 100),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            int_bounded(&mut req(0, &mut len, too_small, 4), &var, 0, 100),
            Err(Errno::EINVAL)
        );
        assert_eq!(var.load(Ordering::Relaxed), 50);
        assert_eq!(
            int_bounded(&mut req(0, &mut len, fine, 4), &var, 0, 100),
            Ok(())
        );
        assert_eq!(var.load(Ordering::Relaxed), 100);

        // minimum > maximum: read-only, refused before anything is looked at.
        let (min, max) = SYSCTL_INT_READONLY;
        let mut len = 0;
        assert_eq!(
            int_bounded(&mut req(0, &mut len, fine, 4), &var, min, max),
            Err(Errno::EPERM)
        );
        assert_eq!(len, 0);
        let mut len = 4;
        assert_eq!(
            int_bounded(&mut req(out, &mut len, 0, 0), &var, min, max),
            Ok(())
        );
        assert_eq!(get_int(out), 100);
    }

    #[test]
    fn int_lower_never_raises() {
        let (higher, lower, out) = (U + 0xc0, U + 0xc4, U + 0xc8);
        put(higher, &4i32.to_ne_bytes());
        put(lower, &1i32.to_ne_bytes());
        let var = AtomicI32::new(3);
        let mut len = 4;
        assert_eq!(
            int_lower(&mut req(0, &mut len, higher, 4), &var),
            Err(Errno::EPERM)
        );
        assert_eq!(int_lower(&mut req(out, &mut len, lower, 4), &var), Ok(()));
        assert_eq!((get_int(out), var.load(Ordering::Relaxed)), (3, 1));
        // The C's unsigned comparison: -1 is the largest value, so nothing may raise to it,
        // and anything lowers from it.
        put(higher, &(-1i32).to_ne_bytes());
        assert_eq!(
            int_lower(&mut req(0, &mut len, higher, 4), &var),
            Err(Errno::EPERM)
        );
        let neg = AtomicI32::new(-1);
        assert_eq!(int_lower(&mut req(0, &mut len, lower, 4), &neg), Ok(()));
        assert_eq!(neg.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn rdquad_is_eight_bytes() {
        let out = U + 0x100;
        let mut len = 7;
        assert_eq!(
            rdquad(&mut req(out, &mut len, 0, 0), 1 << 40),
            Err(Errno::ENOMEM)
        );
        let mut len = 8;
        assert_eq!(rdquad(&mut req(out, &mut len, 0, 0), 1 << 40), Ok(()));
        assert_eq!(get(out, 8), (1i64 << 40).to_ne_bytes());
    }

    #[test]
    fn strings_count_the_nul_and_refuse_short_buffers() {
        let (out, new) = (U + 0x140, U + 0x160);
        put(out, &[0xff; 16]);
        put(new, b"x\0");
        let mut len = 0;
        assert_eq!(rdstring(&mut req(0, &mut len, 0, 0), b"EmiBSD"), Ok(()));
        assert_eq!(len, 7);

        let mut len = 6;
        assert_eq!(
            rdstring(&mut req(out, &mut len, 0, 0), b"EmiBSD"),
            Err(Errno::ENOMEM)
        );
        let mut len = 16;
        assert_eq!(
            rdstring(&mut req(out, &mut len, 0, 0), b"EmiBSD\0junk"),
            Ok(())
        );
        assert_eq!((len, get(out, 8)), (7, b"EmiBSD\0\xff".to_vec()));

        assert_eq!(
            rdstring(&mut req(0, &mut len, new, 2), b"EmiBSD"),
            Err(Errno::EPERM)
        );
    }

    #[test]
    fn string_reads_then_writes_and_terminates() {
        let (out, new) = (U + 0x180, U + 0x1a0);
        put(out, &[0xff; 8]);
        put(new, b"newname");
        let mut var = [0u8; 8];
        var[..3].copy_from_slice(b"old");
        let mut len = 8;
        assert_eq!(string(&mut req(out, &mut len, new, 4), &mut var), Ok(()));
        assert_eq!((len, get(out, 4)), (4, b"old\0".to_vec()));
        assert_eq!(&var[..5], b"newn\0");

        // newlen must leave room for the NUL.
        assert_eq!(
            string(&mut req(0, &mut len, new, 8), &mut var),
            Err(Errno::EINVAL)
        );
        // A short old buffer is ENOMEM for sysctl_string...
        let mut len = 2;
        assert_eq!(
            string(&mut req(out, &mut len, 0, 0), &mut var),
            Err(Errno::ENOMEM)
        );
    }

    #[test]
    fn tstring_truncates_with_a_nul() {
        let out = U + 0x1c0;
        put(out, &[0xff; 8]);
        let mut var = [0u8; 16];
        var[..8].copy_from_slice(b"hostname");
        let mut len = 5;
        assert_eq!(tstring(&mut req(out, &mut len, 0, 0), &mut var), Ok(()));
        assert_eq!((len, get(out, 6)), (5, b"host\0\xff".to_vec()));

        let mut len = 0;
        assert_eq!(
            tstring(&mut req(out, &mut len, 0, 0), &mut var),
            Err(Errno::ENOMEM)
        );
    }

    #[test]
    fn structs_copy_whole_and_rdstruct_is_read_only() {
        let (out, new) = (U + 0x200, U + 0x220);
        let tv: [u8; 16] = core::array::from_fn(|i| i as u8);
        let mut len = 15;
        assert_eq!(
            rdstruct(&mut req(out, &mut len, 0, 0), &tv),
            Err(Errno::ENOMEM)
        );
        let mut len = 16;
        assert_eq!(rdstruct(&mut req(out, &mut len, 0, 0), &tv), Ok(()));
        assert_eq!(get(out, 16), tv.to_vec());
        assert_eq!(
            rdstruct(&mut req(0, &mut len, out, 16), &tv),
            Err(Errno::EPERM)
        );

        // sysctl_struct: a new value larger than the structure is EINVAL; one that fits is
        // copied in whole (`len` bytes, as the C).
        let mut var = [0u8; 16];
        let newv: [u8; 16] = core::array::from_fn(|i| 100 + i as u8);
        put(new, &newv);
        let mut len = 0;
        assert_eq!(
            structure(&mut req(0, &mut len, new, 17), &mut var),
            Err(Errno::EINVAL)
        );
        assert_eq!(structure(&mut req(0, &mut len, new, 16), &mut var), Ok(()));
        assert_eq!(var, newv);
    }

    #[test]
    fn bounded_arr_finds_the_mib() {
        static A: AtomicI32 = AtomicI32::new(7);
        let out = U + 0x240;
        put(out, &[0; 4]);
        let table = [SysctlBoundedArgs::readonly(3, &A)];
        let mut len = 4;
        assert_eq!(
            bounded_arr(&table, &[3, 1], &mut req(0, &mut len, 0, 0)),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            bounded_arr(&table, &[4], &mut req(0, &mut len, 0, 0)),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            bounded_arr(&table, &[3], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(get_int(out), 7);
    }

    #[test]
    fn diskname_append_is_snprintf() {
        let mut buf = [0xffu8; 12];
        let l = diskname_append(&mut buf, 0, b"sd0", b"0123");
        assert_eq!((l, &buf[..l + 1]), (9, &b"sd0:0123,\0"[..]));
        // What does not fit is cut, NUL-terminated.
        let l = diskname_append(&mut buf, l, b"cd0", b"");
        assert_eq!((l, &buf[..]), (11, &b"sd0:0123,cd\0"[..]));
        // A full buffer takes nothing.
        assert_eq!(diskname_append(&mut buf, 12, b"wd0", b""), 12);
    }

    #[test]
    fn the_abi_structs_keep_their_sizes_and_offsets() {
        assert_eq!(size_of::<KinfoProc>(), 648);
        assert_eq!(size_of::<KinfoFile>(), 632);
        assert_eq!(size_of::<KinfoVmentry>(), 80);
        assert_eq!(core::mem::offset_of!(KinfoProc, p_tsess), 80);
        assert_eq!(core::mem::offset_of!(KinfoProc, p_tdev), 212);
        assert_eq!(DISKSTATS_SIZE, 112);
        // struct forkstat: four ints, four uint64_t.
        assert_eq!(forkstat_bytes().len(), 48);
        assert_eq!(PS_STRINGS_SIZE, 32);
    }

    // The processes: a test glue whose sessions have a terminal, so that `p_tsess` and
    // `KERN_PROC_TTY` are exercised.
    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files, {
        fn session_tty(sess: &crate::proc::Session<Self>) -> Option<SessionTty> {
            Some(SessionTty {
                dev: 0x0c01,
                pgid: Some(4242),
                session_addr: core::ptr::from_ref(sess).addr(),
            })
        }
        fn unveil_state(pr: &Process<Self>) -> UnveilState {
            UnveilState {
                paths: pr.ps_pid == 4343,
                done: false,
            }
        }
    });

    /// The test thread's tables, once, with the kernel lock held (the double counts it), as
    /// a system call under `sysctl_vslock` holds it.
    fn setup() {
        std::thread_local! {
            static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        testutil::install();
        if !MADE.with(|m| m.replace(true)) {
            hal::klock::kernel_lock();
            crate::proc::procinit::<Procs>(64, 64);
        }
    }

    /// A caller of `sysctl(2)`: a thread of a process `pid` of user `uid` (not on the lists).
    fn caller(pid: Pid, uid: Uid) -> (ProcessRef<Procs>, PoolBox<Thread<Procs>>) {
        setup();
        let pr = crate::proc::process_alloc::<Procs>(pid, ());
        let mut ids = Ucred::new();
        ids.cr_uid.set(uid);
        ids.cr_ruid.set(uid);
        drop(pr.set_ucred(CredRef::new(ids)));
        let t = crate::proc::thread_alloc::<Procs>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        (pr, t)
    }

    /// A process `pid` of user `uid` on `allprocess` and its pid chain, in a session of its
    /// own, with a main thread: what `fork1` leaves. Leaked (the lists keep it).
    fn listed(pid: Pid, uid: Uid) -> &'static Process<Procs> {
        let (pr, t) = caller(pid, uid);
        crate::proc::enternewpgrp(
            &pr,
            crate::proc::pgrp_alloc::<Procs>(()),
            Some(crate::proc::session_alloc::<Procs>(())),
        );
        pr.set_comm(b"test");
        {
            let tb = Procs::tables();
            let mut w = klock::witness();
            tb.allprocess.insert_head(&mut w, pr.as_pin());
            tb.pidhash(pid).insert_head(&mut w, pr.as_pin());
        }
        {
            let mut g = pr.ps_mtx.lock();
            pr.join_locked(&mut g, t.as_pin());
            pr.set_main_locked(&mut g, t);
        }
        let pr: &'static ProcessRef<Procs> = std::boxed::Box::leak(std::boxed::Box::new(pr));
        pr
    }

    /// `fill_kproc` of `pr` (or its main thread when `thread`), as `sysctl_doproc` calls it.
    fn kproc(pr: &Process<Procs>, thread: bool, show_pointers: bool) -> KinfoProc {
        let mut ki = KinfoProc::zeroed();
        let r = klock::reader();
        let g = pr.ps_mtx.lock();
        let main = pr.main_thread(&g).map(|p| p.get_ref());
        let p = if thread { main } else { None };
        fill_kproc(&r, pr, &mut ki, p, show_pointers, &g);
        ki
    }

    #[test]
    fn fill_kproc_reports_a_process_a_thread_and_a_zombie() {
        let pr = listed(4242, 1000);
        let ki = kproc(pr, false, false);
        assert_eq!(
            (ki.p_pid, ki.p__pgid, ki.p_sid, ki.p_tid),
            (4242, 4242, 4242, -1)
        );
        assert_eq!((ki.p_uid, ki.p_ruid), (1000, 1000));
        assert_eq!(&ki.p_comm[..5], b"test\0");
        assert_eq!(&ki.p_emul[..7], b"native\0");
        // No PS_CONTROLT: no terminal, whatever the session has.
        assert_eq!((ki.p_tdev, ki.p_tpgid, ki.p_tsess), (NODEV as u32, -1, 0));
        assert_eq!((ki.p_cpuid, ki.p_uvalid, ki.p_paddr), (KI_NOCPU, 1, 0));
        assert_eq!(ki.p_stat, SIDL as i8);
        assert_eq!(ki.p_eflag & EPROC_UNVEIL, 0);

        // As a thread: its tid and name.
        {
            let g = pr.ps_mtx.lock();
            let p = pr.main_thread(&g).unwrap();
            p.p_tid.set(7);
            p.set_name(b"worker");
        }
        let ki = kproc(pr, true, true);
        assert_eq!(ki.p_tid, 7 + THREAD_PID_OFFSET);
        assert_eq!(&ki.p_name[..7], b"worker\0");
        let addr = {
            let g = pr.ps_mtx.lock();
            ptr::from_ref(pr.main_thread(&g).unwrap().get_ref()).addr() as u64
        };
        assert_eq!(ki.p_paddr, addr);

        // A zombie: no resource usage, no VM fields, no start time.
        pr.ps_flags
            .fetch_or(PS_ZOMBIE | PS_EXITING, Ordering::Relaxed);
        let ki = kproc(pr, false, false);
        assert_eq!((ki.p_uvalid, ki.p_ustart_sec, ki.p_holdcnt), (0, 0, 0));
        assert_eq!(ki.p_stat, SDEAD as i8);
        pr.ps_flags
            .fetch_and(!(PS_ZOMBIE | PS_EXITING), Ordering::Relaxed);

        // The unveil state comes from the glue.
        let pr = listed(4343, 1000);
        assert_eq!(kproc(pr, false, false).p_eflag & EPROC_UNVEIL, EPROC_UNVEIL);
    }

    /// N4.7 decision 8 (the user's): `p_tsess` is a kernel address, given to root only, as
    /// every other pointer of `kinfo_proc` (kern_sysctl.c:2108-2109); the terminal's device and
    /// foreground group are everybody's.
    #[test]
    fn p_tsess_only_for_root() {
        let pr = listed(4444, 1000);
        pr.ps_flags.fetch_or(PS_CONTROLT, Ordering::Relaxed);
        let user = kproc(pr, false, false);
        assert_eq!((user.p_tdev, user.p_tpgid), (0x0c01, 4242));
        assert_eq!(user.p_tsess, 0, "a user sees no kernel address");
        assert_eq!((user.p_sess, user.p_paddr, user.p_wchan), (0, 0, 0));
        let root = kproc(pr, false, true);
        assert_eq!((root.p_tdev, root.p_tpgid), (0x0c01, 4242));
        let sess = {
            let r = klock::reader();
            ptr::from_ref(pr.session(&r).unwrap()).addr() as u64
        };
        assert_eq!(root.p_tsess, sess);
        assert_eq!(root.p_sess, sess);
    }

    #[test]
    fn doproc_sizes_the_answer_and_checks_its_arguments() {
        let (_pr, p) = caller(4500, 1000);
        let size = size_of::<KinfoProc>() as i32;

        let mut len = 0;
        assert_eq!(
            sysctl_doproc(&p, &[KERN_PROC_ALL, 0, size], &mut req(0, &mut len, 0, 0)),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_doproc(
                &p,
                &[KERN_PROC_ALL, 0, size + 1, 1],
                &mut req(0, &mut len, 0, 0)
            ),
            Err(Errno::EINVAL)
        );

        // No process has this pid: only the slop is asked for.
        assert_eq!(
            sysctl_doproc(
                &p,
                &[KERN_PROC_PID, 999_999, size, 10],
                &mut req(0, &mut len, 0, 0)
            ),
            Ok(())
        );
        assert_eq!(len, KERN_PROCSLOP * size as usize);

        // One process: its entry, and with KERN_PROC_SHOW_THREADS its thread's too.
        let pr = listed(4501, 1000);
        let mut len = 0;
        assert_eq!(
            sysctl_doproc(&p, &[99, 0, size, 1], &mut req(0, &mut len, 0, 0)),
            Err(Errno::EINVAL),
            "an unknown op, once a process is looked at"
        );
        let name = [KERN_PROC_PID, 4501, size, 10];
        assert_eq!(
            sysctl_doproc(&p, &name, &mut req(0, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, (1 + KERN_PROCSLOP) * size as usize);
        let name = [KERN_PROC_PID | KERN_PROC_SHOW_THREADS, 4501, size, 10];
        assert_eq!(
            sysctl_doproc(&p, &name, &mut req(0, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, (2 + KERN_PROCSLOP) * size as usize);

        // Copied out: the process, then its thread (a short element size cuts each one).
        let out = testutil::USER_BASE + 0x6000;
        let mut len = 2 * 128;
        let name = [KERN_PROC_PID | KERN_PROC_SHOW_THREADS, 4501, 128, 10];
        assert_eq!(
            sysctl_doproc(&p, &name, &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, 256);
        let pid = |at: usize| get_int(at + core::mem::offset_of!(KinfoProc, p_pid));
        assert_eq!((pid(out), pid(out + 128)), (4501, 4501));
        // Too small a buffer: what fits, then ENOMEM.
        let mut len = 128;
        assert_eq!(
            sysctl_doproc(&p, &name, &mut req(out, &mut len, 0, 0)),
            Err(Errno::ENOMEM)
        );
        assert_eq!(len, 128);

        // KERN_PROC_TTY matches the session's terminal under PS_CONTROLT only.
        let name = [KERN_PROC_TTY, 0x0c01, size, 10];
        assert_eq!(
            sysctl_doproc(&p, &name, &mut req(0, &mut len, 0, 0)),
            Ok(())
        );
        let before = len;
        pr.ps_flags.fetch_or(PS_CONTROLT, Ordering::Relaxed);
        assert_eq!(
            sysctl_doproc(&p, &name, &mut req(0, &mut len, 0, 0)),
            Ok(())
        );
        assert!(len > before);
    }

    #[test]
    fn kern_proc_args_sizes_and_refuses_an_exiting_process() {
        let (_cpr, cp) = caller(4600, 1000);
        let pr = listed(4601, 1000);
        let mut len = 0;
        // Too long a name, too short a name, an unknown op, no such process.
        assert_eq!(
            sysctl_proc_args(&cp, &[4601, KERN_PROC_ARGV, 1], &mut req(0, &mut len, 0, 0)),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            sysctl_proc_args(&cp, &[4601], &mut req(0, &mut len, 0, 0)),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_proc_args(&cp, &[4601, 99], &mut req(0, &mut len, 0, 0)),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            sysctl_proc_args(&cp, &[999_999, KERN_PROC_ARGV], &mut req(0, &mut len, 0, 0)),
            Err(Errno::ESRCH)
        );
        // A size query: an int for the counts, ARG_MAX for the strings.
        assert_eq!(
            sysctl_proc_args(&cp, &[4601, KERN_PROC_NARGV], &mut req(0, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, 4);
        assert_eq!(
            sysctl_proc_args(&cp, &[4601, KERN_PROC_ENV], &mut req(0, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, ARG_MAX);
        // An exiting process, a system one: EINVAL; one in exec: EBUSY.
        let out = testutil::USER_BASE + 0x6800;
        let mut len = 64;
        for (flag, error) in [
            (PS_EXITING, Errno::EINVAL),
            (PS_SYSTEM, Errno::EINVAL),
            (PS_INEXEC, Errno::EBUSY),
        ] {
            pr.ps_flags.fetch_or(flag, Ordering::Relaxed);
            assert_eq!(
                sysctl_proc_args(&cp, &[4601, KERN_PROC_ARGV], &mut req(out, &mut len, 0, 0)),
                Err(error)
            );
            pr.ps_flags.fetch_and(!flag, Ordering::Relaxed);
        }
        // Another user's environment needs root; the arguments do not (and a process
        // without an address space has none to read: EINVAL).
        let other = listed(4602, 2000);
        assert_eq!(
            sysctl_proc_args(&cp, &[4602, KERN_PROC_ENV], &mut req(out, &mut len, 0, 0)),
            Err(Errno::EPERM)
        );
        assert_eq!(
            sysctl_proc_args(&cp, &[4602, KERN_PROC_ARGV], &mut req(out, &mut len, 0, 0)),
            Err(Errno::EINVAL)
        );
        let _ = other;
    }

    #[test]
    fn proc_cwd_nobroadcastkill_and_vmmap_check_as_the_c() {
        let (_cpr, cp) = caller(4700, 1000);
        let pr = listed(4701, 1000);
        let out = testutil::USER_BASE + 0x6c00;
        let mut len = 0;
        // kern.proc_cwd: the size hint, then ERANGE for a room under 2, ENOENT without a
        // current directory (no root mounted).
        assert_eq!(
            sysctl_proc_cwd(&cp, &[4701], &mut req(0, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, MAXPATHLEN * 4);
        let mut len = 1;
        assert_eq!(
            sysctl_proc_cwd(&cp, &[4701], &mut req(out, &mut len, 0, 0)),
            Err(Errno::ERANGE)
        );
        let mut len = 64;
        assert_eq!(
            sysctl_proc_cwd(&cp, &[4701], &mut req(out, &mut len, 0, 0)),
            Err(Errno::ENOENT)
        );
        assert_eq!(
            sysctl_proc_cwd(&cp, &[4701, 1], &mut req(out, &mut len, 0, 0)),
            Err(Errno::ENOTDIR)
        );

        // kern.proc_nobroadcastkill: read by anyone, set by root only.
        let mut len = 4;
        assert_eq!(
            sysctl_proc_nobroadcastkill(&cp, &[4701], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(get_int(out), 0);
        put(out + 8, &1i32.to_ne_bytes());
        assert_eq!(
            sysctl_proc_nobroadcastkill(&cp, &[4701], &mut req(0, &mut len, out + 8, 4)),
            Err(Errno::EPERM)
        );
        let (_rpr, root) = caller(4702, 0);
        assert_eq!(
            sysctl_proc_nobroadcastkill(&root, &[4701], &mut req(0, &mut len, out + 8, 4)),
            Ok(())
        );
        assert_ne!(pr.ps_flags.load(Ordering::Relaxed) & PS_NOBROADCASTKILL, 0);

        // kern.proc_vmmap: the size hint, the size checks, then the unported fill.
        let mut len = 0;
        assert_eq!(
            sysctl_proc_vmmap(&cp, &[4700], &mut req(0, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, VMMAP_MAXLEN);
        let mut len = 81;
        assert_eq!(
            sysctl_proc_vmmap(&cp, &[4700], &mut req(out, &mut len, 0, 0)),
            Err(Errno::EINVAL)
        );
        let mut len = 80;
        assert_eq!(
            sysctl_proc_vmmap(&cp, &[4701], &mut req(out, &mut len, 0, 0)),
            Err(Errno::EPERM),
            "another process's map is root's"
        );
        assert_eq!(
            sysctl_proc_vmmap(&cp, &[4700], &mut req(out, &mut len, 0, 0)),
            Err(Errno::ENOSYS)
        );
    }

    /// A top-level node that records whether `sysctl_vslock`'s locks are held when it runs.
    struct LockProbe {
        /// `(sysctl_lock held, kernel lock held)` at the last call.
        seen: StdMutex<Option<(bool, bool)>>,
    }

    impl SysctlNode for LockProbe {
        fn sysctl(&self, _name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
            let held = (SYSCTL_LOCK.status() != 0, klock::kernel_lock_held());
            *self.seen.lock().unwrap() = Some(held);
            rdint(req, 1)
        }
    }

    static MACHDEP: LockProbe = LockProbe {
        seen: StdMutex::new(None),
    };
    static NET: LockProbe = LockProbe {
        seen: StdMutex::new(None),
    };

    /// The registrations of these tests, once per process.
    fn registered() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            register_top(CTL_MACHDEP, &MACHDEP).unwrap();
            register_top(CTL_NET, &NET).unwrap();
            static OSTYPE: RdString = RdString(b"EmiBSD");
            register_kern(KERN_OSTYPE, &OSTYPE).unwrap();
            install_disk_export(&DISKS).unwrap();
        });
    }

    /// `sysctl(2)` with the name `mib` (put at `at`), the old buffer `old` (room `len`, put
    /// at `at + 0x80`) and the new value `new`: the error and the length given back.
    fn sysctl(
        p: &Thread<Procs>,
        at: usize,
        mib: &[i32],
        old: usize,
        len: Option<usize>,
        new: usize,
        newlen: usize,
    ) -> (Result<(), Errno>, Option<usize>) {
        UserPtr::<i32>::new(at).copyout_slice(mib).unwrap();
        let lenp = at + 0x80;
        if let Some(len) = len {
            UserPtr::<usize>::new(lenp).copyout(&len).unwrap();
        }
        let oldlenp = if len.is_some() { lenp } else { 0 };
        let r = sys_sysctl(
            p,
            UserPtr::new(at),
            mib.len() as u32,
            old,
            UserPtr::new(oldlenp),
            new,
            newlen,
        );
        let back = len.map(|_| UserPtr::<usize>::new(lenp).copyin().unwrap());
        (r, back)
    }

    #[test]
    fn the_registry_answers_eopnotsupp_and_enotdir_in_the_c_s_order() {
        registered();
        let (_pr, p) = caller(4800, 0);
        let at = testutil::USER_BASE + 0x5800;
        let old = at + 0x100;

        // namelen 1 or past CTL_MAXNAME: EINVAL before the name is read.
        assert_eq!(
            sysctl(&p, at, &[CTL_KERN], 0, Some(0), 0, 0).0,
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl(&p, at, &[1; 13], 0, Some(0), 0, 0).0,
            Err(Errno::EINVAL)
        );
        // An unknown top-level id (CTL_DEBUG without DEBUG_SYSCTL, a gap) is EOPNOTSUPP
        // before oldlenp is read: an unmapped oldlenp is not EFAULT here.
        UserPtr::<i32>::new(at)
            .copyout_slice(&[CTL_DEBUG, 1])
            .unwrap();
        assert_eq!(
            sys_sysctl(&p, UserPtr::new(at), 2, 0, UserPtr::new(8), 0, 0),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            sysctl(&p, at, &[3, 1], 0, Some(0), 0, 0).0,
            Err(Errno::EOPNOTSUPP)
        );
        // A registered tree no subsystem registered (ddb in these tests) is the C's case
        // compiled out.
        assert_eq!(
            sysctl(&p, at, &[CTL_DDB, 1], 0, Some(0), 0, 0).0,
            Err(Errno::EOPNOTSUPP)
        );
        // A kern directory nobody serves: ENOTDIR, after the vslock (which fails first,
        // EFAULT, for a buffer this thread without an address space cannot wire).
        assert_eq!(
            sysctl(&p, at, &[CTL_KERN, 99_999, 1], 0, Some(0), 0, 0).0,
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            sysctl(&p, at, &[CTL_KERN, 99_999, 1], old, Some(4), 0, 0).0,
            Err(Errno::EFAULT)
        );
        // An unregistered unlocked directory (kern.file in these tests) is served locked:
        // ENOTDIR.
        assert_eq!(
            sysctl(&p, at, &[CTL_KERN, KERN_FILE, 1], 0, Some(0), 0, 0).0,
            Err(Errno::ENOTDIR)
        );
        // An unknown kern or hw leaf: EOPNOTSUPP (bounded_arr's).
        assert_eq!(
            sysctl(&p, at, &[CTL_KERN, 99_999], 0, Some(0), 0, 0).0,
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            sysctl(&p, at, &[CTL_HW, 99_999], 0, Some(0), 0, 0).0,
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            sysctl(&p, at, &[CTL_HW, HW_NCPU, 1], 0, Some(0), 0, 0).0,
            Err(Errno::ENOTDIR)
        );
        // A registered leaf, read through sysctl(2): kern.ostype.
        let (r, len) = sysctl(&p, at, &[CTL_KERN, KERN_OSTYPE], old, Some(32), 0, 0);
        assert_eq!((r, len), (Ok(()), Some(7)));
        assert_eq!(get(old, 7), b"EmiBSD\0".to_vec());
        // A size query of a kernel leaf, no oldlenp at all for another.
        assert_eq!(
            sysctl(&p, at, &[CTL_KERN, KERN_ARGMAX], 0, Some(0), 0, 0),
            (Ok(()), Some(4))
        );
        assert_eq!(
            sysctl(&p, at, &[CTL_KERN, KERN_ARGMAX], 0, None, 0, 0).0,
            Ok(())
        );

        // A new value from a user other than root: EPERM, first.
        let (_upr, user) = caller(4801, 1000);
        assert_eq!(
            sysctl(&user, at, &[CTL_KERN, KERN_HOSTNAME], 0, Some(0), old, 4).0,
            Err(Errno::EPERM)
        );
        assert_eq!(
            sysctl(&user, at, &[1], 0, Some(0), old, 4).0,
            Err(Errno::EPERM),
            "suser before the name's length"
        );
    }

    #[test]
    fn machdep_and_ddb_are_locked() {
        registered();
        let (_pr, p) = caller(4900, 0);
        let at = testutil::USER_BASE + 0x5c00;
        // machdep: vslocked (the C's dolock; LZ passed it unlocked).
        assert_eq!(
            sysctl(&p, at, &[CTL_MACHDEP, 1], 0, Some(0), 0, 0),
            (Ok(()), Some(4))
        );
        assert_eq!(*MACHDEP.seen.lock().unwrap(), Some((true, true)));
        assert_eq!(SYSCTL_LOCK.status(), 0, "released after");
        // net: not vslocked by the dispatcher (the protocols lock themselves).
        assert_eq!(
            sysctl(&p, at, &[CTL_NET, 1], 0, Some(0), 0, 0),
            (Ok(()), Some(4))
        );
        assert_eq!(MACHDEP.seen.lock().unwrap().map(|s| s.0), Some(true));
        assert_eq!(NET.seen.lock().unwrap().map(|s| s.0), Some(false));
        assert!(Top::Ddb.dolock() && Top::Vm.dolock() && Top::Vfs.dolock());
        assert!(!Top::Kern.dolock() && !Top::Hw.dolock() && !Top::Net.dolock());
    }

    #[test]
    fn the_registry_refuses_ids_the_dispatcher_does_not_consult() {
        static N: RdString = RdString(b"x");
        // The kernel's own leaves and trees are not registrable.
        assert!(register_kern(KERN_PROC, &N).is_err());
        assert!(register_kern(KERN_HOSTNAME, &N).is_err());
        assert!(register_kern(-1, &N).is_err());
        assert!(register_hw(HW_DISKNAMES, &N).is_err());
        assert!(register_top(CTL_KERN, &N).is_err());
        assert!(register_top(CTL_VM, &N).is_err());
        assert!(register_top(99, &N).is_err());
        // A registered id is registered once.
        registered();
        assert!(register_top(CTL_MACHDEP, &N).is_err());
    }

    #[test]
    fn hostname_round_trips_under_the_lock() {
        let (_pr, p) = caller(5000, 0);
        let buf = testutil::USER_BASE + 0x6400;
        put(buf, b"emibsd.local");
        let mut len = 0;
        assert_eq!(
            kern_sysctl_locked(&p, &[KERN_HOSTNAME], &mut req(0, &mut len, buf, 6)),
            Ok(())
        );
        assert_eq!(HOSTNAMELEN.load(Ordering::Relaxed), 6);
        assert_eq!(with_hostname(|h| h[..7].to_vec()), b"emibsd\0".to_vec());
        // Read back, truncated with a NUL into a short buffer (sysctl_tstring).
        let out = buf + 0x40;
        let mut len = 4;
        assert_eq!(
            kern_sysctl_locked(&p, &[KERN_HOSTNAME], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!((len, get(out, 4)), (4, b"emi\0".to_vec()));
        // A name as long as the buffer leaves no room for the NUL: EINVAL.
        assert_eq!(
            kern_sysctl_locked(
                &p,
                &[KERN_HOSTNAME],
                &mut req(0, &mut len, buf, MAXHOSTNAMELEN)
            ),
            Err(Errno::EINVAL)
        );
    }

    /// The disk list's double: `(name, duid, statistics)` per disk and the change flag.
    struct Disks {
        list: StdMutex<(bool, Vec<(&'static [u8], Option<&'static [u8]>, u8)>)>,
    }

    impl DiskExport for Disks {
        fn take_change(&self) -> bool {
            core::mem::replace(&mut self.list.lock().unwrap().0, false)
        }
        fn count(&self) -> usize {
            self.list.lock().unwrap().1.len()
        }
        fn for_each(&self, f: &mut DiskVisit<'_>) {
            let list = self.list.lock().unwrap().1.clone();
            for (name, duid, fill) in list {
                f(name, duid, &[fill; DISKSTATS_SIZE]);
            }
        }
    }

    static DISKS: Disks = Disks {
        list: StdMutex::new((false, Vec::new())),
    };

    #[test]
    fn hw_disk_nodes_follow_the_export() {
        registered();
        setup();
        let out = testutil::USER_BASE + 0x5e00;
        *DISKS.list.lock().unwrap() = (
            true,
            std::vec![
                (&b"sd0"[..], Some(&b"0123456789abcdef"[..]), 1),
                (&b"cd0"[..], None, 2)
            ],
        );
        let mut len = 64;
        assert_eq!(
            hw_sysctl_locked(&[HW_DISKNAMES], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(get(out, len), b"sd0:0123456789abcdef,cd0:\0".to_vec());

        // The statistics: both entries, refreshed on every read (the update path).
        let mut len = 2 * DISKSTATS_SIZE;
        assert_eq!(
            hw_sysctl_locked(&[HW_DISKSTATS], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, 2 * DISKSTATS_SIZE);
        let stats = get(out, len);
        assert!(stats[..DISKSTATS_SIZE].iter().all(|&b| b == 1));
        assert!(stats[DISKSTATS_SIZE..].iter().all(|&b| b == 2));
        DISKS.list.lock().unwrap().1[1].2 = 3;
        assert_eq!(
            hw_sysctl_locked(&[HW_DISKSTATS], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert!(get(out, len)[DISKSTATS_SIZE..].iter().all(|&b| b == 3));

        // A disk gone: the names are rebuilt.
        {
            let mut l = DISKS.list.lock().unwrap();
            l.0 = true;
            l.1.truncate(1);
        }
        let mut len = 64;
        assert_eq!(
            hw_sysctl_locked(&[HW_DISKNAMES], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(get(out, len), b"sd0:0123456789abcdef\0".to_vec());
    }

    #[test]
    fn hw_nodes_without_hardware() {
        registered();
        let (_pr, p) = caller(5100, 0);
        let out = testutil::USER_BASE + 0x5f00;
        let mut len = 4;
        assert_eq!(
            hw_sysctl(&p, &[HW_BYTEORDER], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(get_int(out), 1234);
        let mut len = 4;
        assert_eq!(
            hw_sysctl(&p, &[HW_PAGESIZE], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(get_int(out), UVMEXP.pagesize.load(Ordering::Relaxed));
        // No vendor recorded; no machine registered (the default's table); no cpuspeed.
        assert_eq!(
            hw_sysctl(&p, &[HW_VENDOR], &mut req(out, &mut len, 0, 0)),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            hw_sysctl(&p, &[HW_MACHINE_ID], &mut req(out, &mut len, 0, 0)),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            hw_sysctl_locked(&[HW_CPUSPEED], &mut req(out, &mut len, 0, 0)),
            Err(Errno::EOPNOTSUPP)
        );
        // No battery driver: every knob EOPNOTSUPP, before the room is looked at.
        let mut len = 0;
        assert_eq!(
            sysctl_hwbattery(&[HW_BATTERY_CHARGEMODE], &mut req(out, &mut len, 0, 0)),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            sysctl_hwbattery(&[HW_BATTERY_CHARGEMODE, 1], &mut req(out, &mut len, 0, 0)),
            Err(Errno::ENOTDIR)
        );
        // ucomnames without a ucom: the empty string.
        let mut len = 4;
        assert_eq!(
            hw_sysctl_locked(&[HW_UCOMNAMES], &mut req(out, &mut len, 0, 0)),
            Ok(())
        );
        assert_eq!(len, 1);
    }
}
/* </TESTS> */
