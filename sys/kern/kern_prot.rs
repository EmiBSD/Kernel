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
 * Copyright (c) 1982, 1986, 1989, 1990, 1991, 1993
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
 *	@(#)kern_prot.c	8.6 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! System calls related to processes and protection: the process, group and session ids,
//! the user and group ids, the credentials (`crget`/`crhold`/`crfree`, the kernel crate's),
//! `suser`, the login name, the thread control block and the thread names.
//!
//! Upstream: sys/kern/kern_prot.c @ 3ce1f3f79392
//!
//! Status: `ported`. Every function of the C file is here or, for the credentials
//! themselves, in `kernel::cred` (`crget`, `crhold`, `crfree`, `crset`, `crcopy`, `crdup`,
//! `crfromxucred`, `groupmember`, `suser_ucred`), re-exported.
//!
//! ## Deviations
//! - Credentials are the kernel crate's `CredRef`s (N4.2): `crhold` is a clone, `crfree` a
//!   drop, and a shared credential is never written. So the set*id calls build the new
//!   credentials as a value from the process's (`crset` into a fresh copy), place it
//!   (`crget`'s `pool_get`, which may sleep) and install it with `Process::set_ucred`; when
//!   the process's credentials changed during that sleep (another thread's set*id or exec),
//!   the value is built again from the new ones, which is what the C's `crset` after `crget`
//!   copies. The permission checks run before, against the thread's credentials and the
//!   process's, as the C's.
//! - A thread's `p_ucred` is its own copy (`kernel::proc::Thread::ucred`); `dorefreshcreds`
//!   copies the process's under `ps_mtx` where the C swaps a pointer.
//! - `suser`, `suser_ucred` and `crfromxucred` return `Result<(), Errno>`; `groupmember` and
//!   `proc_cansugid` return `bool`. `(uid_t)-1`/`(gid_t)-1` ("leave unchanged") are
//!   `Uid::MAX`/`Gid::MAX`.
//! - `getsid(2)` reads the leader's pid the session keeps with its address
//!   (`Session::leader_pid`) instead of following `s_leader`.
//! - `sys___set_tcb`/`sys___get_tcb` go through `machine::tcb` (`TCB_SET`, `TCB_GET`,
//!   `TCB_INVALID`); the TCB is a `usize`.

use core::sync::atomic::Ordering;

use hal::klock;
use libkern::strlcpy;

use crate::kern::proc_glue::{
    chgproccnt, inferior, pgfind, pgrp_ref, prfind, process_ref, tfind_user,
};
use crate::machine::copy::{copyin, copyinstr, copyout, copyoutstr};
use crate::machine::tcb::{tcb_get, tcb_invalid, tcb_set};
use crate::sys::errno::Errno;
use crate::sys::proc::{
    _MAXCOMLEN, BsdPgrpG, BsdSessionG, PS_EXEC, PS_SUGID, PS_SUGIDEXEC, PS_TRACED, Proc, Process,
    THREAD_PID_OFFSET,
};
use crate::sys::syscallargs::{
    SysGetgroupsArgs, SysGetloginRArgs, SysGetpgidArgs, SysGetresgidArgs, SysGetresuidArgs,
    SysGetsidArgs, SysGetthrnameArgs, SysSetTcbArgs, SysSetegidArgs, SysSeteuidArgs, SysSetgidArgs,
    SysSetgroupsArgs, SysSetloginArgs, SysSetpgidArgs, SysSetregidArgs, SysSetresgidArgs,
    SysSetresuidArgs, SysSetreuidArgs, SysSetthrnameArgs, SysSetuidArgs,
};
use crate::sys::syslimits::{LOGIN_NAME_MAX, NGROUPS_MAX};
use crate::sys::systm::{SysArgs, kernel_lock, kernel_unlock, sysargs};
use crate::sys::types::{Gid, Pid, Register, Uid};
use crate::sys::ucred::{CredRef, Ucred};

pub use kernel::cred::{
    crcopy, crdup, crfree, crfromxucred, crget, crhold, crset, groupmember, suser_ucred,
};

/// `getpid(2)`.
pub fn sys_getpid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = p.process().ps_pid as Register;
    Ok(())
}

/// `getthrid(2)`.
pub fn sys_getthrid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = (p.p_tid.get() + THREAD_PID_OFFSET) as Register;
    Ok(())
}

/// `getppid(2)`.
pub fn sys_getppid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let pr = p.process();
    let g = pr.ps_mtx.lock();
    retval[0] = pr.ps_ppid.get() as Register;
    drop(g);
    Ok(())
}

/// `getpgrp(2)`: get process group ID; note that POSIX getpgrp takes no parameter.
pub fn sys_getpgrp(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let r = klock::reader();
    retval[0] = p.process().pgid(&r) as Register;
    Ok(())
}

/// The address of `pr`'s session (`pr->ps_session`), 0 without one: what the C compares.
fn session_addr<W: hal::rwlock::Witness + ?Sized>(w: &W, pr: &Process) -> usize {
    pr.session(w).map_or(0, |s| core::ptr::from_ref(s).addr())
}

/// The process `pid` names for `getpgid`/`getsid` (the caller's own for 0 or its pid,
/// otherwise one in the caller's session), given to `f` under the kernel lock's reader.
fn sameses_target<R>(
    curp: &Proc,
    pid: Pid,
    f: impl FnOnce(&klock::KernelReader, &Process) -> R,
) -> Result<R, Errno> {
    let r = klock::reader();
    let curpr = curp.process();
    if pid == 0 || pid == curpr.ps_pid {
        return Ok(f(&r, curpr));
    }
    let targpr = prfind(&r, pid).ok_or(Errno::ESRCH)?;
    if session_addr(&r, targpr) != session_addr(&r, curpr) {
        return Err(Errno::EPERM);
    }
    Ok(f(&r, targpr))
}

/// `getpgid(2)`: SysVR.4 compatible getpgid().
pub fn sys_getpgid(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetpgidArgs = sysargs(v);

    retval[0] = sameses_target(curp, uap.pid.get(), |r, targpr| targpr.pgid(r))? as Register;
    Ok(())
}

/// `getsid(2)`.
pub fn sys_getsid(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetsidArgs = sysargs(v);

    let leader = sameses_target(curp, uap.pid.get(), |r, targpr| {
        targpr.session(r).and_then(|s| s.leader_pid())
    })?;
    // Skip exiting processes
    let Some(leader) = leader else {
        return Err(Errno::ESRCH);
    };
    retval[0] = leader as Register;
    Ok(())
}

/// `getuid(2)`.
pub fn sys_getuid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = p.ucred().cr_ruid.get() as Register;
    Ok(())
}

/// `geteuid(2)`.
pub fn sys_geteuid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = p.ucred().cr_uid.get() as Register;
    Ok(())
}

/// `issetugid(2)`.
pub fn sys_issetugid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = Register::from(p.process().ps_flags.load(Ordering::Relaxed) & PS_SUGIDEXEC != 0);
    Ok(())
}

/// `getgid(2)`.
pub fn sys_getgid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = p.ucred().cr_rgid.get() as Register;
    Ok(())
}

/// `getegid(2)`: get effective group ID. The "egid" is groups\[0\], and could be obtained
/// via getgroups. This syscall exists because it is somewhat painful to do correctly in a
/// library function.
pub fn sys_getegid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = p.ucred().cr_gid.get() as Register;
    Ok(())
}

/// `getgroups(2)`.
pub fn sys_getgroups(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetgroupsArgs = sysargs(v);
    let uc = p.ucred();

    let ngrp = uap.gidsetsize.get();
    if ngrp == 0 {
        retval[0] = Register::from(uc.cr_ngroups.get());
        return Ok(());
    }
    if ngrp < i32::from(uc.cr_ngroups.get()) {
        return Err(Errno::EINVAL);
    }
    let ngrp = usize::try_from(uc.cr_ngroups.get()).unwrap_or(0);
    let mut groups = [[0u8; size_of::<Gid>()]; NGROUPS_MAX];
    for (bytes, gid) in groups.iter_mut().zip(&uc.cr_groups) {
        *bytes = gid.get().to_ne_bytes();
    }
    copyout(groups[..ngrp].as_flattened(), uap.gidset.get() as usize)?;
    retval[0] = ngrp as Register;
    Ok(())
}

/// `setsid(2)`.
pub fn sys_setsid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let pr = p.process();
    let pid = pr.ps_pid;

    let newsess = kernel::proc::session_alloc(BsdSessionG::new());
    let newpgrp = kernel::proc::pgrp_alloc(BsdPgrpG::new());

    let taken = {
        let r = klock::reader();
        pr.pgid(&r) == pid || pgfind(&r, pid).is_some()
    };
    if taken {
        // The new group and session go with their only references.
        drop(newpgrp);
        drop(newsess);
        Err(Errno::EPERM)
    } else {
        kernel::proc::enternewpgrp(p.process_ref(), newpgrp, Some(newsess));
        retval[0] = pid as Register;
        Ok(())
    }
}

/// `setpgid(2)`: set process group (setpgid/old setpgrp).
///
/// The caller does `setpgid(targpid, targpgid)`:
/// - `pid` must be the caller or a child of the caller (`ESRCH`); if a child, it must be in
///   the same session (`EPERM`) and must not have done an exec (`EACCES`);
/// - if `pgid != pid`, there must exist some process in the same session having `pgid`
///   (`EPERM`);
/// - `pid` must not be a session leader (`EPERM`).
pub fn sys_setpgid(curp: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetpgidArgs = sysargs(v);
    let curpr = curp.process();

    let pid = uap.pid.get();
    let mut pgid = uap.pgid.get();

    if pgid < 0 {
        return Err(Errno::EINVAL);
    }

    let newpgrp = kernel::proc::pgrp_alloc(BsdPgrpG::new());

    /// What the checks under the reader decided.
    enum Move {
        /// Nothing to do.
        None,
        /// Into a new group of id `pgid`.
        New(crate::sys::proc::ProcessRef),
        /// Into the existing group.
        This(crate::sys::proc::ProcessRef, crate::sys::proc::PgrpRef),
    }

    let decided: Result<Move, Errno> = 'out: {
        let r = klock::reader();
        let targpr: &Process = if pid != 0 && pid != curpr.ps_pid {
            let Some(targpr) = prfind(&r, pid).filter(|t| inferior(&r, t, curpr)) else {
                break 'out Err(Errno::ESRCH);
            };
            if session_addr(&r, targpr) != session_addr(&r, curpr) {
                break 'out Err(Errno::EPERM);
            }
            if targpr.ps_flags.load(Ordering::Relaxed) & PS_EXEC != 0 {
                break 'out Err(Errno::EACCES);
            }
            targpr
        } else {
            curpr
        };
        if targpr.sess_leader(&r) {
            break 'out Err(Errno::EPERM);
        }
        if pgid == 0 {
            pgid = targpr.ps_pid;
        }

        match pgfind(&r, pgid) {
            None => {
                // can only create a new process group with pgid == pid
                if pgid != targpr.ps_pid {
                    Err(Errno::EPERM)
                } else {
                    Ok(Move::New(process_ref(targpr)))
                }
            }
            // anything to do?
            Some(pgrp)
                if !targpr
                    .pgrp(&r)
                    .is_some_and(|cur| core::ptr::eq::<crate::sys::proc::Pgrp>(&**cur, pgrp)) =>
            {
                let pgsess = pgrp
                    .session(&r)
                    .map_or(0, |s| core::ptr::from_ref(s).addr());
                if pgid != targpr.ps_pid && pgsess != session_addr(&r, curpr) {
                    Err(Errno::EPERM)
                } else {
                    Ok(Move::This(process_ref(targpr), pgrp_ref(pgrp)))
                }
            }
            Some(_) => Ok(Move::None),
        }
    };

    // The moves take the kernel lock's writer themselves: the reader is gone.
    match decided? {
        Move::None => {}
        Move::New(targpr) => kernel::proc::enternewpgrp(&targpr, newpgrp, None),
        Move::This(targpr, pgrp) => kernel::proc::enterthispgrp(&targpr, pgrp),
    }
    // out: an unused new group goes with its only reference.
    Ok(())
}

/// `copyout` of one id to a user pointer, skipped when the pointer is NULL.
fn copyout_id(id: u32, uaddr: *mut u32) -> Result<(), Errno> {
    if uaddr.is_null() {
        return Ok(());
    }
    copyout(&id.to_ne_bytes(), uaddr as usize)
}

/// The set*id calls' change: the process's credentials copied (`crget` + `crset`), changed
/// by `change(pruc, newcred)` and installed as `pr->ps_ucred` with `PS_SUGID`; the
/// credentials replaced are returned, for the caller's process count and `crfree`.
///
/// `crget`'s `pool_get` may sleep: when the process's credentials changed meanwhile, the
/// copy is made again from the new ones (the C copies after `crget`, see the module's
/// deviations).
fn sugid_change(pr: &Process, change: impl Fn(&Ucred, &mut Ucred)) -> CredRef {
    loop {
        // Copy credentials so other references do not see our changes. ps_ucred may change
        // during the crget().
        let pruc = pr.ucred();
        let mut newcred = Ucred::new();
        crset(&mut newcred, &pruc);
        change(&pruc, &mut newcred);
        let newcred = CredRef::new(newcred);
        if pr.ucred_word() != CredRef::as_ptr(&pruc).addr() {
            continue;
        }
        drop(pr.set_ucred(newcred));
        pr.ps_flags.fetch_or(PS_SUGID, Ordering::Relaxed);
        return pruc;
    }
}

/// `getresuid(2)`.
pub fn sys_getresuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetresuidArgs = sysargs(v);
    let uc = p.ucred();

    let error1 = copyout_id(uc.cr_ruid.get(), uap.ruid.get());
    let error2 = copyout_id(uc.cr_uid.get(), uap.euid.get());
    let error3 = copyout_id(uc.cr_svuid.get(), uap.suid.get());

    error1.and(error2).and(error3)
}

/// `setresuid(2)`.
pub fn sys_setresuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetresuidArgs = sysargs(v);
    let pr = p.process();
    let uc = p.ucred();

    let ruid = uap.ruid.get();
    let euid = uap.euid.get();
    let suid = uap.suid.get();

    // make permission checks against the thread's ucred, but the actual changes will be to
    // the process's ucred
    let pruc = pr.ucred();
    if (ruid == Uid::MAX || ruid == pruc.cr_ruid.get())
        && (euid == Uid::MAX || euid == pruc.cr_uid.get())
        && (suid == Uid::MAX || suid == pruc.cr_svuid.get())
    {
        return Ok(()); // no change
    }
    drop(pruc);

    // Any of the real, effective, and saved uids may be changed to the current value of one
    // of the three (root is not limited).
    let held = |id: Uid| id == uc.cr_ruid.get() || id == uc.cr_uid.get() || id == uc.cr_svuid.get();
    if ruid != Uid::MAX && !held(ruid) {
        suser(p)?;
    }
    if euid != Uid::MAX && !held(euid) {
        suser(p)?;
    }
    if suid != Uid::MAX && !held(suid) {
        suser(p)?;
    }

    // Note that unlike the other set*uid() calls, each uid type is set independently of the
    // others.
    let pruc = sugid_change(pr, |_, newcred| {
        if ruid != Uid::MAX {
            newcred.cr_ruid.set(ruid);
        }
        if euid != Uid::MAX {
            newcred.cr_uid.set(euid);
        }
        if suid != Uid::MAX {
            newcred.cr_svuid.set(suid);
        }
    });

    // now that we can sleep, transfer proc count to new user
    if ruid != Uid::MAX && ruid != pruc.cr_ruid.get() {
        chgproccnt(pruc.cr_ruid.get(), -1);
        chgproccnt(ruid, 1);
    }
    crfree(pruc);

    Ok(())
}

/// `getresgid(2)`.
pub fn sys_getresgid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetresgidArgs = sysargs(v);
    let uc = p.ucred();

    let error1 = copyout_id(uc.cr_rgid.get(), uap.rgid.get());
    let error2 = copyout_id(uc.cr_gid.get(), uap.egid.get());
    let error3 = copyout_id(uc.cr_svgid.get(), uap.sgid.get());

    error1.and(error2).and(error3)
}

/// `setresgid(2)`.
pub fn sys_setresgid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetresgidArgs = sysargs(v);
    let pr = p.process();
    let uc = p.ucred();

    let rgid = uap.rgid.get();
    let egid = uap.egid.get();
    let sgid = uap.sgid.get();

    // make permission checks against the thread's ucred, but the actual changes will be to
    // the process's ucred
    let pruc = pr.ucred();
    if (rgid == Gid::MAX || rgid == pruc.cr_rgid.get())
        && (egid == Gid::MAX || egid == pruc.cr_gid.get())
        && (sgid == Gid::MAX || sgid == pruc.cr_svgid.get())
    {
        return Ok(()); // no change
    }
    drop(pruc);

    // Any of the real, effective, and saved gids may be changed to the current value of one
    // of the three (root is not limited).
    let held = |id: Gid| id == uc.cr_rgid.get() || id == uc.cr_gid.get() || id == uc.cr_svgid.get();
    if rgid != Gid::MAX && !held(rgid) {
        suser(p)?;
    }
    if egid != Gid::MAX && !held(egid) {
        suser(p)?;
    }
    if sgid != Gid::MAX && !held(sgid) {
        suser(p)?;
    }

    // Note that unlike the other set*gid() calls, each gid type is set independently of the
    // others.
    let pruc = sugid_change(pr, |_, newcred| {
        if rgid != Gid::MAX {
            newcred.cr_rgid.set(rgid);
        }
        if egid != Gid::MAX {
            newcred.cr_gid.set(egid);
        }
        if sgid != Gid::MAX {
            newcred.cr_svgid.set(sgid);
        }
    });
    crfree(pruc);
    Ok(())
}

/// `setregid(2)`.
pub fn sys_setregid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetregidArgs = sysargs(v);
    let pr = p.process();
    let uc = p.ucred();

    let rgid = uap.rgid.get();
    let egid = uap.egid.get();

    // make permission checks against the thread's ucred, but the actual changes will be to
    // the process's ucred
    //
    // The saved gid check here is complicated: we reset the saved gid to the real gid if the
    // real gid is specified *and* either it's changing _or_ the saved gid won't equal the
    // effective gid. So, the svgid *won't* change when the rgid isn't specified or when the
    // rgid isn't changing and the svgid equals the requested egid.
    let new_egid = |cr: &Ucred| {
        if egid != Gid::MAX {
            egid
        } else {
            cr.cr_gid.get()
        }
    };
    let pruc = pr.ucred();
    if (rgid == Gid::MAX || rgid == pruc.cr_rgid.get())
        && (egid == Gid::MAX || egid == pruc.cr_gid.get())
        && (rgid == Gid::MAX
            || (rgid == pruc.cr_rgid.get() && pruc.cr_svgid.get() == new_egid(&pruc)))
    {
        return Ok(()); // no change
    }
    drop(pruc);

    // Any of the real, effective, and saved gids may be changed to the current value of one
    // of the three (root is not limited).
    let held = |id: Gid| id == uc.cr_rgid.get() || id == uc.cr_gid.get() || id == uc.cr_svgid.get();
    if rgid != Gid::MAX && !held(rgid) {
        suser(p)?;
    }
    if egid != Gid::MAX && !held(egid) {
        suser(p)?;
    }

    let pruc = sugid_change(pr, |pruc, newcred| {
        if rgid != Gid::MAX {
            newcred.cr_rgid.set(rgid);
        }
        if egid != Gid::MAX {
            newcred.cr_gid.set(egid);
        }

        // The saved gid presents a bit of a dilemma, as it did not exist when setregid(2)
        // was conceived. We only set the saved gid when the real gid is specified and either
        // its value would change, or where the saved and effective gids are different.
        if rgid != Gid::MAX && (rgid != pruc.cr_rgid.get() || pruc.cr_svgid.get() != new_egid(pruc))
        {
            newcred.cr_svgid.set(rgid);
        }
    });
    crfree(pruc);
    Ok(())
}

/// `setreuid(2)`.
pub fn sys_setreuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetreuidArgs = sysargs(v);
    let pr = p.process();
    let uc = p.ucred();

    let ruid = uap.ruid.get();
    let euid = uap.euid.get();

    // make permission checks against the thread's ucred, but the actual changes will be to
    // the process's ucred
    //
    // The saved uid check here is complicated: we reset the saved uid to the real uid if the
    // real uid is specified *and* either it's changing _or_ the saved uid won't equal the
    // effective uid. So, the svuid *won't* change when the ruid isn't specified or when the
    // ruid isn't changing and the svuid equals the requested euid.
    let new_euid = |cr: &Ucred| {
        if euid != Uid::MAX {
            euid
        } else {
            cr.cr_uid.get()
        }
    };
    let pruc = pr.ucred();
    if (ruid == Uid::MAX || ruid == pruc.cr_ruid.get())
        && (euid == Uid::MAX || euid == pruc.cr_uid.get())
        && (ruid == Uid::MAX
            || (ruid == pruc.cr_ruid.get() && pruc.cr_svuid.get() == new_euid(&pruc)))
    {
        return Ok(()); // no change
    }
    drop(pruc);

    // Any of the real, effective, and saved uids may be changed to the current value of one
    // of the three (root is not limited).
    let held = |id: Uid| id == uc.cr_ruid.get() || id == uc.cr_uid.get() || id == uc.cr_svuid.get();
    if ruid != Uid::MAX && !held(ruid) {
        suser(p)?;
    }
    if euid != Uid::MAX && !held(euid) {
        suser(p)?;
    }

    let pruc = sugid_change(pr, |pruc, newcred| {
        if ruid != Uid::MAX {
            newcred.cr_ruid.set(ruid);
        }
        if euid != Uid::MAX {
            newcred.cr_uid.set(euid);
        }

        // The saved uid presents a bit of a dilemma, as it did not exist when setreuid(2)
        // was conceived. We only set the saved uid when the real uid is specified and either
        // its value would change, or where the saved and effective uids are different.
        if ruid != Uid::MAX && (ruid != pruc.cr_ruid.get() || pruc.cr_svuid.get() != new_euid(pruc))
        {
            newcred.cr_svuid.set(ruid);
        }
    });

    // now that we can sleep, transfer proc count to new user
    if ruid != Uid::MAX && ruid != pruc.cr_ruid.get() {
        chgproccnt(pruc.cr_ruid.get(), -1);
        chgproccnt(ruid, 1);
    }
    crfree(pruc);

    Ok(())
}

/// `setuid(2)`.
pub fn sys_setuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetuidArgs = sysargs(v);
    let pr = p.process();
    let uc = p.ucred();

    let uid = uap.uid.get();

    let pruc = pr.ucred();
    if pruc.cr_uid.get() == uid && pruc.cr_ruid.get() == uid && pruc.cr_svuid.get() == uid {
        return Ok(());
    }
    drop(pruc);

    if uid != uc.cr_ruid.get() && uid != uc.cr_svuid.get() && uid != uc.cr_uid.get() {
        suser(p)?;
    }

    // Everything's okay, do it.
    let root = suser(p).is_ok();
    let did_real = core::cell::Cell::new(false);
    let pruc = sugid_change(pr, |pruc, newcred| {
        did_real.set(uid == pruc.cr_uid.get() || root);
        if did_real.get() {
            newcred.cr_ruid.set(uid);
            newcred.cr_svuid.set(uid);
        }
        newcred.cr_uid.set(uid);
    });

    // Transfer proc count to new user.
    if did_real.get() && uid != pruc.cr_ruid.get() {
        chgproccnt(pruc.cr_ruid.get(), -1);
        chgproccnt(uid, 1);
    }
    crfree(pruc);

    Ok(())
}

/// `seteuid(2)`.
pub fn sys_seteuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSeteuidArgs = sysargs(v);
    let pr = p.process();
    let uc = p.ucred();

    let euid = uap.euid.get();

    if pr.ucred().cr_uid.get() == euid {
        return Ok(());
    }

    if euid != uc.cr_ruid.get() && euid != uc.cr_svuid.get() {
        suser(p)?;
    }

    let pruc = sugid_change(pr, |_, newcred| newcred.cr_uid.set(euid));
    crfree(pruc);
    Ok(())
}

/// `setgid(2)`.
pub fn sys_setgid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetgidArgs = sysargs(v);
    let pr = p.process();
    let uc = p.ucred();

    let gid = uap.gid.get();

    let pruc = pr.ucred();
    if pruc.cr_gid.get() == gid && pruc.cr_rgid.get() == gid && pruc.cr_svgid.get() == gid {
        return Ok(());
    }
    drop(pruc);

    if gid != uc.cr_rgid.get() && gid != uc.cr_svgid.get() && gid != uc.cr_gid.get() {
        suser(p)?;
    }

    let root = suser(p).is_ok();
    let pruc = sugid_change(pr, |pruc, newcred| {
        if gid == pruc.cr_gid.get() || root {
            newcred.cr_rgid.set(gid);
            newcred.cr_svgid.set(gid);
        }
        newcred.cr_gid.set(gid);
    });
    crfree(pruc);
    Ok(())
}

/// `setegid(2)`.
pub fn sys_setegid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetegidArgs = sysargs(v);
    let pr = p.process();
    let uc = p.ucred();

    let egid = uap.egid.get();

    if pr.ucred().cr_gid.get() == egid {
        return Ok(());
    }

    if egid != uc.cr_rgid.get() && egid != uc.cr_svgid.get() {
        suser(p)?;
    }

    let pruc = sugid_change(pr, |_, newcred| newcred.cr_gid.set(egid));
    crfree(pruc);
    Ok(())
}

/// `setgroups(2)`.
pub fn sys_setgroups(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetgroupsArgs = sysargs(v);
    let pr = p.process();

    suser(p)?;
    let ngrp = uap.gidsetsize.get();
    let Ok(ngrp) = usize::try_from(ngrp) else {
        return Err(Errno::EINVAL);
    };
    if ngrp > NGROUPS_MAX {
        return Err(Errno::EINVAL);
    }
    let mut groups = [[0u8; size_of::<Gid>()]; NGROUPS_MAX];
    copyin(uap.gidset.get() as usize, groups[..ngrp].as_flattened_mut())?;

    let pruc = sugid_change(pr, |_, newcred| {
        for (gid, &bytes) in newcred.cr_groups.iter_mut().zip(&groups[..ngrp]) {
            gid.set(Gid::from_ne_bytes(bytes));
        }
        newcred.cr_ngroups.set(ngrp as i16);
    });
    crfree(pruc);
    Ok(())
}

/// `suser`: test whether this process has special user powers.
pub fn suser(p: &Proc) -> Result<(), Errno> {
    let cred = p.ucred();

    if cred.cr_uid.get() == 0 {
        return Ok(());
    }
    Err(Errno::EPERM)
}

/// `getlogin_r(2)`: get login name, if available. The error is the return value, as in C.
pub fn sys_getlogin_r(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetloginRArgs = sysargs(v);
    let mut buf = [0u8; LOGIN_NAME_MAX];

    let namelen = uap.namelen.get().min(LOGIN_NAME_MAX);
    {
        let r = klock::reader();
        if let Some(s) = p.process().session(&r) {
            strlcpy(&mut buf, s.s_login.get(&r));
        }
    }
    let error = match copyoutstr(&buf[..namelen], uap.namebuf.get() as usize) {
        Ok(_) => 0,
        Err(Errno::ENAMETOOLONG) => Errno::ERANGE as i32,
        Err(e) => e as i32,
    };
    retval[0] = error as Register;
    Ok(())
}

/// `setlogin(2)`: set login name.
pub fn sys_setlogin(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetloginArgs = sysargs(v);
    let mut buf = [0u8; LOGIN_NAME_MAX];

    suser(p)?;
    match copyinstr(uap.namebuf.get() as usize, &mut buf) {
        Ok(_) => {
            // The session, counted, so the writer below may open its name.
            let sess = {
                let r = klock::reader();
                p.process()
                    .pgrp(&r)
                    .and_then(|pg| pg.pg_session.get(&r).clone())
            };
            if let Some(s) = sess {
                let mut w = klock::witness();
                strlcpy(s.s_login.get_mut(&mut w), &buf);
            }
            Ok(())
        }
        Err(Errno::ENAMETOOLONG) => Err(Errno::EINVAL),
        Err(e) => Err(e),
    }
}

/// `proc_cansugid`: check if a process is allowed to raise its privileges.
pub fn proc_cansugid(p: &Proc) -> bool {
    // ptrace(2)d processes shouldn't.
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        return false;
    }

    // processes with shared filedescriptors shouldn't.
    if p.fd().fd_refcnt.get() > 1 {
        return false;
    }

    // Allow.
    true
}

/// `__set_tcb(2)`: set address of the proc's thread-control-block.
#[allow(non_snake_case)] // the C name, verbatim: the syscall table finds it by name
pub fn sys___set_tcb(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetTcbArgs = sysargs(v);
    let tcb = uap.tcb.get() as usize;

    if tcb_invalid(tcb) {
        return Err(Errno::EINVAL);
    }
    tcb_set(p, tcb);
    Ok(())
}

/// `__get_tcb(2)`: get address of the proc's thread-control-block.
#[allow(non_snake_case)] // the C name, verbatim: the syscall table finds it by name
pub fn sys___get_tcb(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = tcb_get(p) as Register;
    Ok(())
}

/// `getthrname(2)`. The error is the return value, as in C.
pub fn sys_getthrname(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetthrnameArgs = sysargs(v);
    let mut buf = [0u8; _MAXCOMLEN];
    let tid = uap.tid.get();

    let name = if tid != 0 {
        let r = klock::reader();
        tfind_user(&r, tid, curp.process()).map(|p| p.name())
    } else {
        Some(curp.name())
    };
    let Some(name) = name else {
        return Err(Errno::ESRCH);
    };
    strlcpy(&mut buf, &name);

    let len = uap.len.get().min(buf.len());
    let error = match copyoutstr(&buf[..len], uap.name.get() as usize) {
        Ok(_) => 0,
        Err(Errno::ENAMETOOLONG) => Errno::ERANGE as i32,
        Err(e) => e as i32,
    };
    retval[0] = error as Register;
    Ok(())
}

/// `setthrname(2)`. The error is the return value, as in C; a name too long is truncated.
pub fn sys_setthrname(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetthrnameArgs = sysargs(v);
    let mut buf = [0u8; _MAXCOMLEN];
    let tid = uap.tid.get();

    let error = match copyinstr(uap.name.get() as usize, &mut buf) {
        Ok(_) => 0,
        Err(Errno::ENAMETOOLONG) => {
            buf[_MAXCOMLEN - 1] = 0;
            0
        }
        Err(e) => e as i32,
    };
    if error == 0 {
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        if tid != 0 {
            let r = klock::reader();
            let Some(p) = tfind_user(&r, tid, curp.process()) else {
                return Err(Errno::ESRCH);
            };
            p.set_name(&buf[..n]);
        } else {
            curp.set_name(&buf[..n]);
        }
    }
    retval[0] = error as Register;
    Ok(())
}

/// `dorefreshcreds`: refresh the thread's copy of the process's credentials.
pub fn dorefreshcreds(_pr: &Process, p: &Proc) {
    kernel_lock(); // KERNEL_LOCK() (XXX should be PROCESS_RLOCK(pr))
    if p.cred_stale() {
        p.refresh_cred();
    }
    kernel_unlock(); // KERNEL_UNLOCK()
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the credentials and the id system calls: the reference counting of
    // `crget`/`crhold`/`crfree`/`crcopy`/`crdup`, `groupmember`, `suser`, and the permission
    // rules of the set*id calls against threads built by hand.

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::{assert, assert_eq, assert_ne};

    use super::*;
    use crate::kern::proc_glue::{procinit, test_process, test_thread};
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::proc::{ProcExt, refreshcreds};
    use crate::sys::ucred::Xucred;
    use crate::sys::user::User;

    /// Real memory and the process pools and hashes.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = setup_real_memory();
        crate::machine::cons::consinit();
        procinit();
        guard
    }

    /// Credentials whose real, effective and saved ids are `uid`/`gid`.
    fn ids(uid: Uid, gid: Gid) -> Ucred {
        let mut cr = Ucred::new();
        cr.cr_uid.set(uid);
        cr.cr_ruid.set(uid);
        cr.cr_svuid.set(uid);
        cr.cr_gid.set(gid);
        cr.cr_rgid.set(gid);
        cr.cr_svgid.set(gid);
        cr
    }

    /// Placed credentials with one reference whose ids are `uid`/`gid`.
    fn cred(uid: Uid, gid: Gid) -> CredRef {
        CredRef::new(ids(uid, gid))
    }

    /// A thread of a fresh process, both holding `cr`, with a u-area for the TCB, charged to
    /// its real uid as `fork1` does.
    fn thread(cr: CredRef) -> &'static Proc {
        let pr = test_process(1000);
        let p = test_thread(pr);
        p.set_uarea(Box::leak(Box::new(User::new())));
        chgproccnt(cr.cr_ruid.get(), 1);
        drop(p.set_cred(cr.clone()));
        drop(pr.set_ucred(cr));
        p
    }

    /// The argument registers of a system call.
    fn args(a: &[Register]) -> SysArgs {
        let mut v: SysArgs = [0; 6];
        v[..a.len()].copy_from_slice(a);
        v
    }

    /// `(uid_t)-1` in an argument register.
    const NONE: Register = Uid::MAX as Register;

    /// The real, effective and saved uids of `cr`.
    fn uids(cr: &Ucred) -> (Uid, Uid, Uid) {
        (cr.cr_ruid.get(), cr.cr_uid.get(), cr.cr_svuid.get())
    }

    #[test]
    fn crget_crhold_crfree_count_references() {
        let _g = setup();
        let nout = kernel::cred::ucred_nout();

        let cr = crget();
        assert_eq!(CredRef::count(&cr), 1);
        assert_eq!(uids(&cr), (0, 0, 0));
        assert_eq!(cr.cr_ngroups.get(), 0);
        assert_eq!(kernel::cred::ucred_nout(), nout + 1);

        let held = crhold(&cr);
        assert!(CredRef::ptr_eq(&held, &cr));
        assert_eq!(CredRef::count(&cr), 2);
        crfree(held);
        assert_eq!(CredRef::count(&cr), 1);
        assert_eq!(kernel::cred::ucred_nout(), nout + 1);
        crfree(cr);
        assert_eq!(kernel::cred::ucred_nout(), nout);
    }

    #[test]
    fn crcopy_and_crdup_give_private_copies() {
        let _g = setup();

        let mut v = ids(1000, 100);
        v.cr_ngroups.set(2);
        v.cr_groups[0].set(100);
        v.cr_groups[1].set(20);
        let cr = CredRef::new(v);

        let other = crhold(&cr);
        let copy = crcopy(other);
        assert_eq!(CredRef::count(&cr), 1);
        assert_eq!(uids(&copy), (1000, 1000, 1000));
        assert_eq!(copy.cr_svgid.get(), 100);
        assert_eq!(copy.cr_ngroups.get(), 2);
        assert_eq!(copy.cr_groups[1].get(), 20);

        let dup = crdup(&cr);
        assert!(!CredRef::ptr_eq(&dup, &cr));
        assert_eq!(CredRef::count(&cr), 1);
        assert_eq!(CredRef::count(&dup), 1);
        assert_eq!(uids(&dup), (1000, 1000, 1000));
    }

    #[test]
    fn groupmember_suser_and_crfromxucred() {
        let _g = setup();

        let mut cr = ids(1000, 100);
        cr.cr_ngroups.set(2);
        cr.cr_groups[0].set(100);
        cr.cr_groups[1].set(20);
        cr.cr_groups[2].set(30); // beyond cr_ngroups
        assert!(groupmember(100, &cr));
        assert!(groupmember(20, &cr));
        assert!(!groupmember(30, &cr));
        assert_eq!(suser_ucred(&cr), Err(Errno::EPERM));
        cr.cr_uid.set(0);
        assert_eq!(suser_ucred(&cr), Ok(()));

        let mut x = Xucred {
            cr_uid: 5,
            cr_gid: 6,
            cr_ngroups: 1,
            cr_groups: [7; NGROUPS_MAX],
        };
        let mut k = Ucred::new();
        assert_eq!(
            crfromxucred(&mut k, x.cr_uid, x.cr_gid, x.cr_ngroups, &x.cr_groups),
            Ok(())
        );
        assert_eq!(
            (k.cr_uid.get(), k.cr_gid.get(), k.cr_ngroups.get()),
            (5, 6, 1)
        );
        assert_eq!((k.cr_groups[0].get(), k.cr_groups[1].get()), (7, 0));
        x.cr_ngroups = NGROUPS_MAX as i16 + 1;
        assert_eq!(
            crfromxucred(&mut k, x.cr_uid, x.cr_gid, x.cr_ngroups, &x.cr_groups),
            Err(Errno::EINVAL)
        );
        x.cr_ngroups = -1;
        assert_eq!(
            crfromxucred(&mut k, x.cr_uid, x.cr_gid, x.cr_ngroups, &x.cr_groups),
            Err(Errno::EINVAL)
        );
    }

    #[test]
    fn setresuid_allows_the_current_ids_and_root_anything() {
        let _g = setup();
        let mut rv: [Register; 2] = [0; 2];

        // A user whose saved uid is still root may become root again, nothing else.
        let mut v = ids(1000, 100);
        v.cr_svuid.set(0);
        let cr = CredRef::new(v);
        let p = thread(cr.clone());
        let pr = p.process();
        assert_eq!(
            sys_setresuid(p, &args(&[NONE, NONE, NONE]), &mut rv),
            Ok(())
        );
        assert_eq!(pr.ps_flags.load(Ordering::Relaxed) & PS_SUGID, 0);
        assert_eq!(
            sys_setresuid(p, &args(&[NONE, 2000, NONE]), &mut rv),
            Err(Errno::EPERM)
        );
        assert!(CredRef::ptr_eq(&pr.ucred(), &cr));
        assert_eq!(sys_setresuid(p, &args(&[NONE, 0, NONE]), &mut rv), Ok(()));

        // The process has new credentials; the thread keeps its own until it refreshes them.
        assert!(!CredRef::ptr_eq(&pr.ucred(), &cr));
        assert_eq!(uids(&pr.ucred()), (1000, 0, 0));
        assert_ne!(pr.ps_flags.load(Ordering::Relaxed) & PS_SUGID, 0);
        assert_eq!(uids(p.ucred()), (1000, 1000, 0));
        assert!(p.cred_stale());
        refreshcreds(p);
        assert!(!p.cred_stale());
        assert_eq!(uids(p.ucred()), (1000, 0, 0));
        assert!(CredRef::ptr_eq(&p.cred_ref(), &pr.ucred()));

        // Now root: any uid at all, and the process count moves to the new real uid.
        assert_eq!(
            sys_setresuid(p, &args(&[3000, 3000, 3000]), &mut rv),
            Ok(())
        );
        refreshcreds(p);
        assert_eq!(uids(p.ucred()), (3000, 3000, 3000));
        assert_eq!(
            sys_setresuid(p, &args(&[0, NONE, NONE]), &mut rv),
            Err(Errno::EPERM)
        );
    }

    #[test]
    fn setreuid_resets_the_saved_uid_with_the_real_one() {
        let _g = setup();
        let mut rv: [Register; 2] = [0; 2];

        let p = thread(cred(0, 0));
        let pr = p.process();
        // Only the effective uid: the saved uid stays.
        assert_eq!(sys_setreuid(p, &args(&[NONE, 1000]), &mut rv), Ok(()));
        assert_eq!(uids(&pr.ucred()), (0, 1000, 0));
        refreshcreds(p);
        // The real uid changes: the saved one follows it.
        assert_eq!(sys_setreuid(p, &args(&[1000, NONE]), &mut rv), Ok(()));
        assert_eq!(uids(&pr.ucred()), (1000, 1000, 1000));
        refreshcreds(p);
        assert_eq!(
            sys_setreuid(p, &args(&[0, NONE]), &mut rv),
            Err(Errno::EPERM)
        );
    }

    #[test]
    fn setuid_and_seteuid_of_a_user_with_a_root_saved_uid() {
        let _g = setup();
        let mut rv: [Register; 2] = [0; 2];

        let mut v = ids(1000, 100);
        v.cr_svuid.set(0);
        let p = thread(CredRef::new(v));
        let pr = p.process();
        // setuid(0) is allowed (0 is the saved uid) but, not being root, only the effective
        // uid changes.
        assert_eq!(sys_setuid(p, &args(&[0]), &mut rv), Ok(()));
        assert_eq!(uids(&pr.ucred()), (1000, 0, 0));
        assert_eq!(sys_seteuid(p, &args(&[2000]), &mut rv), Err(Errno::EPERM));
        assert_eq!(sys_seteuid(p, &args(&[1000]), &mut rv), Ok(()));
        assert_eq!(uids(&pr.ucred()), (1000, 1000, 0));
        refreshcreds(p);
        // setgid to a group the thread does not have needs root.
        assert_eq!(sys_setgid(p, &args(&[200]), &mut rv), Err(Errno::EPERM));
        assert_eq!(sys_setegid(p, &args(&[100]), &mut rv), Ok(()));
    }

    #[test]
    fn the_get_calls_copy_out_and_the_groups_round_trip() {
        let _g = setup();
        let mut rv: [Register; 2] = [0; 2];

        let mut v = ids(1000, 100);
        v.cr_svuid.set(7);
        let p = thread(cred(0, 0));
        let (mut r, e, mut s): (Uid, Uid, Uid) = (9, 9, 9);
        let ptrs = |a: &mut Uid| a as *mut Uid as Register;
        // A root thread installs cr's groups, then reads them back.
        let groups: [Gid; 3] = [100, 20, 30];
        assert_eq!(
            sys_setgroups(p, &args(&[3, groups.as_ptr() as Register]), &mut rv),
            Ok(())
        );
        refreshcreds(p);
        assert_eq!(sys_getgroups(p, &args(&[0, 0]), &mut rv), Ok(()));
        assert_eq!(rv[0], 3);
        let mut back: [Gid; 4] = [0; 4];
        assert_eq!(
            sys_getgroups(p, &args(&[2, back.as_mut_ptr() as Register]), &mut rv),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sys_getgroups(p, &args(&[4, back.as_mut_ptr() as Register]), &mut rv),
            Ok(())
        );
        assert_eq!((rv[0], back), (3, [100, 20, 30, 0]));

        let q = thread(CredRef::new(v));
        assert_eq!(
            sys_getresuid(q, &args(&[ptrs(&mut r), 0, ptrs(&mut s)]), &mut rv),
            Ok(())
        );
        assert_eq!((r, e, s), (1000, 9, 7));
        assert_eq!(
            sys_setgroups(q, &args(&[1, groups.as_ptr() as Register]), &mut rv),
            Err(Errno::EPERM)
        );
    }

    #[test]
    fn the_tcb_and_the_thread_name_round_trip() {
        let _g = setup();
        let mut rv: [Register; 2] = [0; 2];

        let p = thread(cred(0, 0));
        assert_eq!(sys___set_tcb(p, &args(&[0x1234_5000]), &mut rv), Ok(()));
        assert_eq!(sys___get_tcb(p, &args(&[]), &mut rv), Ok(()));
        assert_eq!(rv[0], 0x1234_5000);

        let name = b"worker\0";
        assert_eq!(
            sys_setthrname(p, &args(&[0, name.as_ptr() as Register]), &mut rv),
            Ok(())
        );
        assert_eq!((rv[0], &*p.name()), (0, &b"worker"[..]));
        let mut out = [0xffu8; 4];
        assert_eq!(
            sys_getthrname(p, &args(&[0, out.as_mut_ptr() as Register, 4]), &mut rv),
            Ok(())
        );
        assert_eq!(rv[0], Errno::ERANGE as i32 as Register);
        let mut out = [0xffu8; _MAXCOMLEN];
        assert_eq!(
            sys_getthrname(
                p,
                &args(&[0, out.as_mut_ptr() as Register, _MAXCOMLEN as Register]),
                &mut rv
            ),
            Ok(())
        );
        assert_eq!((rv[0], &out[..7]), (0, &name[..]));
        // A tid lookup walks the tid hash: under the kernel lock, as a system call runs.
        kernel_lock();
        let r = sys_getthrname(p, &args(&[12345, out.as_mut_ptr() as Register, 4]), &mut rv);
        kernel_unlock();
        assert_eq!(r, Err(Errno::ESRCH));
    }
}
/* </TESTS> */
