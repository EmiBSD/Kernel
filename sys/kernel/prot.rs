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
//! System calls related to processes and protection (`kern/kern_prot.c`): the process,
//! group and session ids, the user and group ids, the login name, the thread control block,
//! the thread names, the routing table (`getrtable`/`setrtable`, only those two of
//! `uipc_syscalls.c`, whose LZ file stays `bsd`'s), and
//! the checks over credentials (`suser`, `proc_cansugid`, `dorefreshcreds`). The credentials
//! themselves are `kernel::cred`'s (`crget`, `crset`, `groupmember`, ...).
//!
//! Upstream: sys/kern/kern_prot.c @ 3ce1f3f79392
//! Upstream: sys/kern/uipc_syscalls.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_prot.rs@3c62ede68802
//!
//! Each system call is a function of the thread and its typed arguments (`UserPtr<T>` for a
//! user pointer), returning what the C puts in `retval[0]`; `bsd`'s `sysent[]` rows and the
//! `abi` module's `Native` entries call them. The machine's TCB (`TCB_SET`, `TCB_GET`,
//! `TCB_INVALID`, N4.8) and the routing tables (`rtable_exists`, N6) are reached through the
//! process glue.
//!
//! ## Deviations
//! - Credentials are `CredRef`s (N4.2): a shared credential is never written. The set*id
//!   calls build the new credentials as a value from the process's (`crset` into a fresh
//!   copy), place it (`crget`'s `pool_get`, which may sleep) and install it only if the
//!   process's credentials did not change during that sleep; otherwise they build it again
//!   from the new ones, which is what the C's `crset` after `crget` copies. The permission
//!   checks run before, against the thread's credentials and the process's, as the C's.
//! - A thread's `p_ucred` is its own copy (`Thread::ucred`); `dorefreshcreds` copies the
//!   process's under `ps_mtx` where the C swaps a pointer.
//! - `(uid_t)-1`/`(gid_t)-1` ("leave unchanged") are `Uid::MAX`/`Gid::MAX`.
//! - `getsid(2)` reads the leader's pid the session keeps (`Session::leader_pid`) instead of
//!   following `s_leader`.
//! - `setsid(2)` makes the DIAGNOSTIC check of `enternewpgrp`'s new-session branch ("mksession
//!   but not curproc") itself, since `kernel::proc::enternewpgrp` has no curproc.
//!
//! ## Redesign
//! - LZ's system calls took `&SysArgs` and wrote `retval` (bsd's `sysent[]` signature); here
//!   they take their arguments typed and return the value, so `bsd`'s rows and the ABI
//!   module translate once (decision 25) and the kernel never reads a raw register.
//! - The group and session lookups run under the kernel lock's reader or writer witness
//!   (`hal::klock`), the C's `[K]` made a type; `getpgid`/`getsid`'s "same session" rule is
//!   one helper (`sameses_target`).

use core::sync::atomic::Ordering;

use hal::klock;
use hal::sys::errno::Errno;
use hal::sys::syslimits::NGROUPS_MAX;
use hal::sys::types::{Gid, Pid, Uid};
use hal::user::UserPtr;

use crate::cred::{CredRef, Ucred, crset};
use crate::proc::{
    _MAXCOMLEN, LOGIN_NAME_MAX, PS_EXEC, PS_SUGID, PS_SUGIDEXEC, PS_TRACED, ProcGlue, Process,
    THREAD_PID_OFFSET, Thread,
};

pub use crate::proc::suser;

/// `getpid(2)`.
pub fn sys_getpid<G: ProcGlue>(p: &Thread<G>) -> Pid {
    p.process().ps_pid
}

/// `getthrid(2)`.
pub fn sys_getthrid<G: ProcGlue>(p: &Thread<G>) -> Pid {
    p.p_tid.get() + THREAD_PID_OFFSET
}

/// `getppid(2)`.
pub fn sys_getppid<G: ProcGlue>(p: &Thread<G>) -> Pid {
    let pr = p.process();
    let g = pr.ps_mtx.lock();
    let ppid = pr.ps_ppid.get();
    drop(g);
    ppid
}

/// `getpgrp(2)`: get process group ID; note that POSIX getpgrp takes no parameter.
pub fn sys_getpgrp<G: ProcGlue>(p: &Thread<G>) -> Pid {
    let r = klock::reader();
    p.process().pgid(&r)
}

/// The address of `pr`'s session (`pr->ps_session`), 0 without one: what the C compares.
fn session_addr<G: ProcGlue, W: hal::rwlock::Witness + ?Sized>(w: &W, pr: &Process<G>) -> usize {
    pr.session(w).map_or(0, |s| core::ptr::from_ref(s).addr())
}

/// The process `pid` names for `getpgid`/`getsid` (the caller's own for 0 or its pid,
/// otherwise one in the caller's session: `EPERM` outside it, `ESRCH` for none), given to `f`
/// under the kernel lock's reader.
fn sameses_target<G: ProcGlue, R>(
    curp: &Thread<G>,
    pid: Pid,
    f: impl FnOnce(&klock::KernelReader, &Process<G>) -> R,
) -> Result<R, Errno> {
    let r = klock::reader();
    let curpr = curp.process();
    if pid == 0 || pid == curpr.ps_pid {
        return Ok(f(&r, curpr));
    }
    let targpr = crate::proc::prfind::<G, _>(&r, pid).ok_or(Errno::ESRCH)?;
    if session_addr(&r, targpr) != session_addr(&r, curpr) {
        return Err(Errno::EPERM);
    }
    Ok(f(&r, targpr))
}

/// `getpgid(2)`: SysVR.4 compatible getpgid().
pub fn sys_getpgid<G: ProcGlue>(curp: &Thread<G>, pid: Pid) -> Result<Pid, Errno> {
    sameses_target(curp, pid, |r, targpr| targpr.pgid(r))
}

/// `getsid(2)`: the session leader's pid; `ESRCH` when the leader is gone (exiting).
pub fn sys_getsid<G: ProcGlue>(curp: &Thread<G>, pid: Pid) -> Result<Pid, Errno> {
    let leader = sameses_target(curp, pid, |r, targpr| {
        targpr.session(r).and_then(super::proc::Session::leader_pid)
    })?;
    // Skip exiting processes
    leader.ok_or(Errno::ESRCH)
}

/// `getuid(2)`.
pub fn sys_getuid<G: ProcGlue>(p: &Thread<G>) -> Uid {
    p.ucred().cr_ruid.get()
}

/// `geteuid(2)`.
pub fn sys_geteuid<G: ProcGlue>(p: &Thread<G>) -> Uid {
    p.ucred().cr_uid.get()
}

/// `issetugid(2)`.
pub fn sys_issetugid<G: ProcGlue>(p: &Thread<G>) -> i32 {
    i32::from(p.process().ps_flags.load(Ordering::Relaxed) & PS_SUGIDEXEC != 0)
}

/// `getgid(2)`.
pub fn sys_getgid<G: ProcGlue>(p: &Thread<G>) -> Gid {
    p.ucred().cr_rgid.get()
}

/// `getegid(2)`: get effective group ID. The "egid" is groups\[0\], and could be obtained
/// via getgroups. This syscall exists because it is somewhat painful to do correctly in a
/// library function.
pub fn sys_getegid<G: ProcGlue>(p: &Thread<G>) -> Gid {
    p.ucred().cr_gid.get()
}

/// `getgroups(2)`: the number of groups (for `gidsetsize` 0), or the groups copied out to
/// `gidset` (`EINVAL` when `gidsetsize` is too small).
pub fn sys_getgroups<G: ProcGlue>(
    p: &Thread<G>,
    gidsetsize: i32,
    gidset: UserPtr<Gid>,
) -> Result<i32, Errno> {
    let uc = p.ucred();
    let ngroups = i32::from(uc.cr_ngroups.get());
    if gidsetsize == 0 {
        return Ok(ngroups);
    }
    if gidsetsize < ngroups {
        return Err(Errno::EINVAL);
    }
    let mut groups = [0 as Gid; NGROUPS_MAX];
    let n = usize::try_from(ngroups).unwrap_or(0).min(NGROUPS_MAX);
    for (g, gid) in groups.iter_mut().zip(&uc.cr_groups) {
        *g = gid.get();
    }
    gidset.copyout_slice(&groups[..n])?;
    Ok(ngroups)
}

/// `setsid(2)`: a new session and group led by the caller's process; its pid (`EPERM` when
/// it already leads a group, or a group has its pid).
pub fn sys_setsid<G: ProcGlue>(p: &Thread<G>) -> Result<Pid, Errno> {
    let pr = p.process();
    let pid = pr.ps_pid;

    let newsess = crate::proc::session_alloc::<G>(G::SessionG::default());
    let newpgrp = crate::proc::pgrp_alloc::<G>(G::PgrpG::default());

    let taken = {
        let r = klock::reader();
        pr.pgid(&r) == pid || crate::proc::pgfind::<G, _>(&r, pid).is_some()
    };
    if taken {
        // The new group and session go with their only references.
        drop(newpgrp);
        drop(newsess);
        return Err(Errno::EPERM);
    }
    // DIAGNOSTIC: a session is made only for the running process (the C's check in
    // enternewpgrp's mksession branch; setsid(2) is its only caller with a new session).
    #[cfg(feature = "diagnostic")]
    if !G::with_curproc(|c| c.is_some_and(|c| core::ptr::eq(c.process(), pr))) {
        crate::assert::kpanic(format_args!("enternewpgrp: mksession but not curproc"));
    }
    crate::proc::enternewpgrp(p.process_ref(), newpgrp, Some(newsess));
    Ok(pid)
}

/// `setpgid(2)`: set process group (setpgid/old setpgrp). The caller does
/// `setpgid(targpid, targpgid)`:
/// - `pid` must be the caller or a child of the caller (`ESRCH`); if a child, it must be in
///   the same session (`EPERM`) and must not have done an exec (`EACCES`);
/// - if `pgid != pid`, there must exist some process in the same session having `pgid`
///   (`EPERM`);
/// - `pid` must not be a session leader (`EPERM`).
pub fn sys_setpgid<G: ProcGlue>(curp: &Thread<G>, pid: Pid, pgid: Pid) -> Result<(), Errno> {
    /// What the checks under the reader decided.
    enum Move<G: ProcGlue> {
        /// Nothing to do.
        None,
        /// Into a new group of id `pgid`.
        New(crate::proc::ProcessRef<G>),
        /// Into the existing group.
        This(crate::proc::ProcessRef<G>, crate::proc::PgrpRef<G>),
    }

    let curpr = curp.process();
    let mut pgid = pgid;

    if pgid < 0 {
        return Err(Errno::EINVAL);
    }

    let newpgrp = crate::proc::pgrp_alloc::<G>(G::PgrpG::default());

    let decided: Result<Move<G>, Errno> = 'out: {
        let r = klock::reader();
        let targpr: &Process<G> = if pid != 0 && pid != curpr.ps_pid {
            let Some(targpr) = crate::proc::prfind::<G, _>(&r, pid)
                .filter(|t| crate::proc::inferior(&r, t, curpr))
            else {
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

        match crate::proc::pgfind::<G, _>(&r, pgid) {
            None => {
                // can only create a new process group with pgid == pid
                if pgid == targpr.ps_pid {
                    Ok(Move::New(G::process_ref(targpr)))
                } else {
                    Err(Errno::EPERM)
                }
            }
            // anything to do?
            Some(pgrp)
                if !targpr.pgrp(&r).is_some_and(|cur| {
                    core::ptr::eq::<crate::proc::Pgrp<G>>(&raw const **cur, pgrp)
                }) =>
            {
                let pgsess = pgrp
                    .session(&r)
                    .map_or(0, |s| core::ptr::from_ref(s).addr());
                if pgid != targpr.ps_pid && pgsess != session_addr(&r, curpr) {
                    Err(Errno::EPERM)
                } else {
                    Ok(Move::This(G::process_ref(targpr), pgrp_ref(pgrp)))
                }
            }
            Some(_) => Ok(Move::None),
        }
    };

    // The moves take the kernel lock's writer themselves: the reader is gone.
    match decided? {
        Move::None => {}
        Move::New(targpr) => crate::proc::enternewpgrp(&targpr, newpgrp, None),
        Move::This(targpr, pgrp) => crate::proc::enterthispgrp(&targpr, pgrp),
    }
    // out: an unused new group goes with its only reference.
    Ok(())
}

/// A counted reference to the group `pg` found under a witness (a pool item: groups other
/// than process 0's are `pgrppl`'s, and process 0's never moves).
fn pgrp_ref<G: ProcGlue>(pg: &crate::proc::Pgrp<G>) -> crate::proc::PgrpRef<G> {
    match hal::pool::PoolArc::from_ref(pg) {
        Some(a) => crate::proc::StaticOrPooled::Pooled(a),
        None => G::pgrp_ref(pg),
    }
}

/// The set*id calls' change: the process's credentials copied (`crget` + `crset`), changed
/// by `change(pruc, newcred)` and installed as `pr->ps_ucred` with `PS_SUGID`; the
/// credentials replaced are returned, for the caller's process count and `crfree`. When the
/// process's credentials changed during the placement (another thread's set*id or exec), the
/// copy is made again from the new ones (see the module's deviations).
fn sugid_change<G: ProcGlue>(pr: &Process<G>, change: impl Fn(&Ucred, &mut Ucred)) -> CredRef {
    loop {
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

/// `copyout` of one id to a user pointer, skipped when the pointer is NULL.
fn copyout_id(id: u32, uaddr: UserPtr<u32>) -> Result<(), Errno> {
    if uaddr.is_null() {
        return Ok(());
    }
    uaddr.copyout(&id)
}

/// `getresuid(2)`: the real, effective and saved uids to the three pointers (each when not
/// NULL); the first error of the three.
pub fn sys_getresuid<G: ProcGlue>(
    p: &Thread<G>,
    ruid: UserPtr<Uid>,
    euid: UserPtr<Uid>,
    suid: UserPtr<Uid>,
) -> Result<(), Errno> {
    let uc = p.ucred();
    let error1 = copyout_id(uc.cr_ruid.get(), ruid);
    let error2 = copyout_id(uc.cr_uid.get(), euid);
    let error3 = copyout_id(uc.cr_svuid.get(), suid);
    error1.and(error2).and(error3)
}

/// `setresuid(2)`.
pub fn sys_setresuid<G: ProcGlue>(
    p: &Thread<G>,
    ruid: Uid,
    euid: Uid,
    suid: Uid,
) -> Result<(), Errno> {
    let pr = p.process();
    let uc = p.ucred();

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
        crate::proc::chgproccnt::<G>(pruc.cr_ruid.get(), -1);
        crate::proc::chgproccnt::<G>(ruid, 1);
    }
    drop(pruc);
    Ok(())
}

/// `getresgid(2)`.
pub fn sys_getresgid<G: ProcGlue>(
    p: &Thread<G>,
    rgid: UserPtr<Gid>,
    egid: UserPtr<Gid>,
    sgid: UserPtr<Gid>,
) -> Result<(), Errno> {
    let uc = p.ucred();
    let error1 = copyout_id(uc.cr_rgid.get(), rgid);
    let error2 = copyout_id(uc.cr_gid.get(), egid);
    let error3 = copyout_id(uc.cr_svgid.get(), sgid);
    error1.and(error2).and(error3)
}

/// `setresgid(2)`.
pub fn sys_setresgid<G: ProcGlue>(
    p: &Thread<G>,
    rgid: Gid,
    egid: Gid,
    sgid: Gid,
) -> Result<(), Errno> {
    let pr = p.process();
    let uc = p.ucred();

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
    drop(pruc);
    Ok(())
}

/// `setregid(2)`.
pub fn sys_setregid<G: ProcGlue>(p: &Thread<G>, rgid: Gid, egid: Gid) -> Result<(), Errno> {
    let pr = p.process();
    let uc = p.ucred();

    // make permission checks against the thread's ucred, but the actual changes will be to
    // the process's ucred
    //
    // The saved gid check here is complicated: we reset the saved gid to the real gid if the
    // real gid is specified *and* either it's changing _or_ the saved gid won't equal the
    // effective gid. So, the svgid *won't* change when the rgid isn't specified or when the
    // rgid isn't changing and the svgid equals the requested egid.
    let new_egid = |cr: &Ucred| {
        if egid == Gid::MAX {
            cr.cr_gid.get()
        } else {
            egid
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
    drop(pruc);
    Ok(())
}

/// `setreuid(2)`.
pub fn sys_setreuid<G: ProcGlue>(p: &Thread<G>, ruid: Uid, euid: Uid) -> Result<(), Errno> {
    let pr = p.process();
    let uc = p.ucred();

    // make permission checks against the thread's ucred, but the actual changes will be to
    // the process's ucred
    //
    // The saved uid check here is complicated: we reset the saved uid to the real uid if the
    // real uid is specified *and* either it's changing _or_ the saved uid won't equal the
    // effective uid. So, the svuid *won't* change when the ruid isn't specified or when the
    // ruid isn't changing and the svuid equals the requested euid.
    let new_euid = |cr: &Ucred| {
        if euid == Uid::MAX {
            cr.cr_uid.get()
        } else {
            euid
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
        crate::proc::chgproccnt::<G>(pruc.cr_ruid.get(), -1);
        crate::proc::chgproccnt::<G>(ruid, 1);
    }
    drop(pruc);
    Ok(())
}

/// `setuid(2)`.
pub fn sys_setuid<G: ProcGlue>(p: &Thread<G>, uid: Uid) -> Result<(), Errno> {
    let pr = p.process();
    let uc = p.ucred();

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
        crate::proc::chgproccnt::<G>(pruc.cr_ruid.get(), -1);
        crate::proc::chgproccnt::<G>(uid, 1);
    }
    drop(pruc);
    Ok(())
}

/// `seteuid(2)`.
pub fn sys_seteuid<G: ProcGlue>(p: &Thread<G>, euid: Uid) -> Result<(), Errno> {
    let pr = p.process();
    let uc = p.ucred();

    if pr.ucred().cr_uid.get() == euid {
        return Ok(());
    }

    if euid != uc.cr_ruid.get() && euid != uc.cr_svuid.get() {
        suser(p)?;
    }

    drop(sugid_change(pr, |_, newcred| newcred.cr_uid.set(euid)));
    Ok(())
}

/// `setgid(2)`.
pub fn sys_setgid<G: ProcGlue>(p: &Thread<G>, gid: Gid) -> Result<(), Errno> {
    let pr = p.process();
    let uc = p.ucred();

    let pruc = pr.ucred();
    if pruc.cr_gid.get() == gid && pruc.cr_rgid.get() == gid && pruc.cr_svgid.get() == gid {
        return Ok(());
    }
    drop(pruc);

    if gid != uc.cr_rgid.get() && gid != uc.cr_svgid.get() && gid != uc.cr_gid.get() {
        suser(p)?;
    }

    let root = suser(p).is_ok();
    drop(sugid_change(pr, |pruc, newcred| {
        if gid == pruc.cr_gid.get() || root {
            newcred.cr_rgid.set(gid);
            newcred.cr_svgid.set(gid);
        }
        newcred.cr_gid.set(gid);
    }));
    Ok(())
}

/// `setegid(2)`.
pub fn sys_setegid<G: ProcGlue>(p: &Thread<G>, egid: Gid) -> Result<(), Errno> {
    let pr = p.process();
    let uc = p.ucred();

    if pr.ucred().cr_gid.get() == egid {
        return Ok(());
    }

    if egid != uc.cr_rgid.get() && egid != uc.cr_svgid.get() {
        suser(p)?;
    }

    drop(sugid_change(pr, |_, newcred| newcred.cr_gid.set(egid)));
    Ok(())
}

/// `setgroups(2)`: root only; `EINVAL` for a negative count or more than `NGROUPS_MAX`.
pub fn sys_setgroups<G: ProcGlue>(
    p: &Thread<G>,
    gidsetsize: i32,
    gidset: UserPtr<Gid>,
) -> Result<(), Errno> {
    let pr = p.process();

    suser(p)?;
    let Ok(ngrp) = usize::try_from(gidsetsize) else {
        return Err(Errno::EINVAL);
    };
    if ngrp > NGROUPS_MAX {
        return Err(Errno::EINVAL);
    }
    let mut groups = [0 as Gid; NGROUPS_MAX];
    gidset.copyin_slice(&mut groups[..ngrp])?;

    drop(sugid_change(pr, |_, newcred| {
        for (gid, &g) in newcred.cr_groups.iter_mut().zip(&groups[..ngrp]) {
            gid.set(g);
        }
        newcred.cr_ngroups.set(ngrp as i16);
    }));
    Ok(())
}

/// `getlogin_r(2)`: the session's login name copied out to `namebuf` (at most `namelen`
/// bytes); the error is the return value, as in C (`ERANGE` for a buffer too short).
pub fn sys_getlogin_r<G: ProcGlue>(p: &Thread<G>, namebuf: UserPtr<u8>, namelen: usize) -> i32 {
    let mut buf = [0u8; LOGIN_NAME_MAX];

    let namelen = namelen.min(LOGIN_NAME_MAX);
    {
        let r = klock::reader();
        if let Some(s) = p.process().session(&r) {
            strlcpy(&mut buf, s.s_login.get(&r));
        }
    }
    match namebuf.copyoutstr(&buf[..namelen]) {
        Ok(_) => 0,
        Err(Errno::ENAMETOOLONG) => Errno::ERANGE as i32,
        Err(e) => e as i32,
    }
}

/// `setlogin(2)`: root only; `EINVAL` for a name too long.
pub fn sys_setlogin<G: ProcGlue>(p: &Thread<G>, namebuf: UserPtr<u8>) -> Result<(), Errno> {
    let mut buf = [0u8; LOGIN_NAME_MAX];

    suser(p)?;
    match namebuf.copyinstr(&mut buf) {
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

/// `strlcpy(dst, src, sizeof dst)`: `src` up to its NUL, truncated to leave room for the
/// NUL `dst` always ends with.
fn strlcpy(dst: &mut [u8], src: &[u8]) {
    let Some(room) = dst.len().checked_sub(1) else {
        return;
    };
    let n = src
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(src.len())
        .min(room);
    dst[..n].copy_from_slice(&src[..n]);
    dst[n] = 0;
}

/// `proc_cansugid`: check if a process is allowed to raise its privileges.
pub fn proc_cansugid<G: ProcGlue>(p: &Thread<G>) -> bool {
    // ptrace(2)d processes shouldn't.
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_TRACED != 0 {
        return false;
    }

    // processes with shared filedescriptors shouldn't.
    if p.fd().shares() > 1 {
        return false;
    }

    // Allow.
    true
}

/// `__set_tcb(2)`: set address of the proc's thread-control-block (`EINVAL` for an address
/// the machine refuses).
#[allow(non_snake_case, reason = "the C name, sys___set_tcb")]
pub fn sys___set_tcb<G: ProcGlue>(p: &Thread<G>, tcb: usize) -> Result<(), Errno> {
    if hal::thread::tcb_invalid(tcb) {
        return Err(Errno::EINVAL);
    }
    hal::thread::tcb_set(p, tcb);
    Ok(())
}

/// `__get_tcb(2)`: get address of the proc's thread-control-block.
#[allow(non_snake_case, reason = "the C name, sys___get_tcb")]
pub fn sys___get_tcb<G: ProcGlue>(p: &Thread<G>) -> usize {
    hal::thread::tcb_get(p)
}

/// `getthrname(2)`: the name of thread `tid` of the caller's process (the caller for 0)
/// copied out to `name` (at most `len` bytes); `ESRCH` for no such thread; the copy's error
/// is the return value, as in C (`ERANGE` for a buffer too short).
pub fn sys_getthrname<G: ProcGlue>(
    curp: &Thread<G>,
    tid: Pid,
    name: UserPtr<u8>,
    len: usize,
) -> Result<i32, Errno> {
    let mut buf = [0u8; _MAXCOMLEN];

    let n = if tid != 0 {
        let r = klock::reader();
        crate::proc::tfind_user::<G, _>(&r, tid, curp.process()).map(Thread::name)
    } else {
        Some(curp.name())
    };
    let Some(n) = n else {
        return Err(Errno::ESRCH);
    };
    strlcpy(&mut buf, &n);

    let len = len.min(buf.len());
    Ok(match name.copyoutstr(&buf[..len]) {
        Ok(_) => 0,
        Err(Errno::ENAMETOOLONG) => Errno::ERANGE as i32,
        Err(e) => e as i32,
    })
}

/// `setthrname(2)`: thread `tid`'s name (the caller's for 0) from `name`, truncated when too
/// long; `ESRCH` for no such thread; the copy's error is the return value, as in C.
pub fn sys_setthrname<G: ProcGlue>(
    curp: &Thread<G>,
    tid: Pid,
    name: UserPtr<u8>,
) -> Result<i32, Errno> {
    let mut buf = [0u8; _MAXCOMLEN];

    let error = match name.copyinstr(&mut buf) {
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
            let Some(p) = crate::proc::tfind_user::<G, _>(&r, tid, curp.process()) else {
                return Err(Errno::ESRCH);
            };
            p.set_name(&buf[..n]);
        } else {
            curp.set_name(&buf[..n]);
        }
    }
    Ok(error)
}

/// `dorefreshcreds`: refresh the thread's copy of the process's credentials.
pub fn dorefreshcreds<G: ProcGlue>(p: &Thread<G>) {
    klock::kernel_lock(); // KERNEL_LOCK() (XXX should be PROCESS_RLOCK(pr))
    if p.cred_stale() {
        p.refresh_cred();
    }
    klock::kernel_unlock(); // KERNEL_UNLOCK()
}

/// `setrtable(2)` (`uipc_syscalls.c`): moves the process to routing table `rtableid`; only
/// root may leave a table other than 0; `EINVAL` for a table that does not exist.
pub fn sys_setrtable<G: ProcGlue>(p: &Thread<G>, rtableid: i32) -> Result<(), Errno> {
    let ps_rtableid = p.process().ps_rtableid.load(Ordering::Relaxed);

    if i64::from(ps_rtableid) == i64::from(rtableid) {
        return Ok(());
    }
    if ps_rtableid != 0 {
        suser(p)?;
    }
    if rtableid < 0 || !G::rtable_exists(rtableid as u32) {
        return Err(Errno::EINVAL);
    }

    p.process()
        .ps_rtableid
        .store(rtableid as u32, Ordering::Relaxed);
    Ok(())
}

/// `getrtable(2)` (`uipc_syscalls.c`): the process's routing table.
pub fn sys_getrtable<G: ProcGlue>(p: &Thread<G>) -> i32 {
    p.process().ps_rtableid.load(Ordering::Relaxed) as i32
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use hal::pool::PoolBox;

    use super::*;
    use crate::proc::ProcessRef;

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// The tests' region of the user-copy double.
    const UGIDS: usize = crate::testutil::USER_BASE + 0xf000;

    /// A thread of a new process `pid` whose real, effective and saved uids are `(r, e, s)`
    /// (the real uid counted once, as `fork1` would) and whose gids are `(rg, eg, sg)`.
    fn thread_as(
        pid: Pid,
        (r, e, s): (Uid, Uid, Uid),
        (rg, eg, sg): (Gid, Gid, Gid),
    ) -> (ProcessRef<Procs>, PoolBox<Thread<Procs>>) {
        std::thread_local! {
            static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        crate::testutil::install();
        if !MADE.with(|m| m.replace(true)) {
            crate::proc::procinit::<Procs>(64, 64);
        }
        let pr = crate::proc::process_alloc::<Procs>(pid, ());
        let mut ids = Ucred::new();
        ids.cr_ruid.set(r);
        ids.cr_uid.set(e);
        ids.cr_svuid.set(s);
        ids.cr_rgid.set(rg);
        ids.cr_gid.set(eg);
        ids.cr_svgid.set(sg);
        drop(pr.set_ucred(CredRef::new(ids)));
        crate::proc::chgproccnt::<Procs>(r, 1);
        let t = crate::proc::thread_alloc::<Procs>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        (pr, t)
    }

    fn uids(pr: &Process<Procs>) -> (Uid, Uid, Uid) {
        let c = pr.ucred();
        (c.cr_ruid.get(), c.cr_uid.get(), c.cr_svuid.get())
    }

    fn gids(pr: &Process<Procs>) -> (Gid, Gid, Gid) {
        let c = pr.ucred();
        (c.cr_rgid.get(), c.cr_gid.get(), c.cr_svgid.get())
    }

    fn sugid(pr: &Process<Procs>) -> bool {
        pr.ps_flags.load(Ordering::Relaxed) & PS_SUGID != 0
    }

    /// setuid(2) (kern_prot.c:696-751): a user may take one of its three uids, and sets the
    /// real and saved ones only when the target is its effective uid; root sets all three;
    /// anything else is `EPERM`; a change sets `PS_SUGID`, the same ids change nothing.
    #[test]
    fn setuid_takes_a_held_uid_and_root_any() {
        let u = (1000, 1001, 1002);
        // (ids, target, result, ids after)
        let rows: [((Uid, Uid, Uid), Uid, Result<(), Errno>, (Uid, Uid, Uid)); 7] = [
            (u, 1000, Ok(()), (1000, 1000, 1002)),
            (u, 1001, Ok(()), (1001, 1001, 1001)),
            (u, 1002, Ok(()), (1000, 1002, 1002)),
            (u, 1003, Err(Errno::EPERM), u),
            (u, 0, Err(Errno::EPERM), u),
            ((0, 0, 0), 1000, Ok(()), (1000, 1000, 1000)),
            ((7, 7, 7), 7, Ok(()), (7, 7, 7)),
        ];
        for (i, &(ids, target, result, after)) in rows.iter().enumerate() {
            let (pr, t) = thread_as(300 + i as Pid, ids, (10, 10, 10));
            assert_eq!(sys_setuid(&t, target), result, "row {i}");
            assert_eq!(uids(&pr), after, "row {i}");
            assert_eq!(sugid(&pr), after != ids, "row {i}");
        }
    }

    /// setuid(2) moves the process count to the new real uid (kern_prot.c:744-746).
    #[test]
    fn setuid_moves_the_process_count_with_the_real_uid() {
        let (pr, t) = thread_as(320, (0, 0, 0), (0, 0, 0));
        let before0 = crate::proc::chgproccnt::<Procs>(0, 0);
        let before = crate::proc::chgproccnt::<Procs>(4321, 0);
        assert_eq!(sys_setuid(&t, 4321), Ok(()));
        assert_eq!(uids(&pr), (4321, 4321, 4321));
        assert_eq!(crate::proc::chgproccnt::<Procs>(0, 0), before0 - 1);
        assert_eq!(crate::proc::chgproccnt::<Procs>(4321, 0), before + 1);
    }

    /// seteuid(2) (kern_prot.c:754-785): the real or saved uid, or anything for root.
    #[test]
    fn seteuid_takes_the_real_or_saved_uid() {
        let u = (1000, 1001, 1002);
        let rows = [
            (u, 1000, Ok(()), (1000, 1000, 1002)),
            (u, 1002, Ok(()), (1000, 1002, 1002)),
            (u, 1001, Ok(()), u),
            (u, 1003, Err(Errno::EPERM), u),
            ((0, 0, 0), 55, Ok(()), (0, 55, 0)),
        ];
        for (i, &(ids, target, result, after)) in rows.iter().enumerate() {
            let (pr, t) = thread_as(330 + i as Pid, ids, (10, 10, 10));
            assert_eq!(sys_seteuid(&t, target), result, "row {i}");
            assert_eq!(uids(&pr), after, "row {i}");
            assert_eq!(sugid(&pr), after != ids, "row {i}");
        }
    }

    /// setresuid(2) (kern_prot.c:346-426): each id independently, to one of the three held
    /// (`-1` leaves it); root is not limited; no change sets nothing.
    #[test]
    fn setresuid_takes_held_uids_each_on_its_own() {
        let u = (1000, 1001, 1002);
        let m = Uid::MAX;
        let rows = [
            (u, (1002, m, 1000), Ok(()), (1002, 1001, 1000)),
            (u, (m, 1000, m), Ok(()), (1000, 1000, 1002)),
            (u, (m, m, m), Ok(()), u),
            (u, (1000, 1001, 1002), Ok(()), u),
            (u, (5, m, m), Err(Errno::EPERM), u),
            (u, (m, m, 5), Err(Errno::EPERM), u),
            ((0, 0, 0), (1, 2, 3), Ok(()), (1, 2, 3)),
        ];
        for (i, &(ids, (r, e, s), result, after)) in rows.iter().enumerate() {
            let (pr, t) = thread_as(340 + i as Pid, ids, (10, 10, 10));
            assert_eq!(sys_setresuid(&t, r, e, s), result, "row {i}");
            assert_eq!(uids(&pr), after, "row {i}");
            assert_eq!(sugid(&pr), after != ids, "row {i}");
        }
    }

    /// setgid(2): the gid analogue of setuid(2), over the thread's credentials.
    #[test]
    fn setgid_takes_a_held_gid_and_root_any() {
        let g = (20, 21, 22);
        let rows = [
            ((1000, 1000, 1000), 21, Ok(()), (21, 21, 21)),
            ((1000, 1000, 1000), 20, Ok(()), (20, 20, 22)),
            ((1000, 1000, 1000), 23, Err(Errno::EPERM), g),
            ((0, 0, 0), 23, Ok(()), (23, 23, 23)),
        ];
        for (i, &(ids, target, result, after)) in rows.iter().enumerate() {
            let (pr, t) = thread_as(350 + i as Pid, ids, g);
            assert_eq!(sys_setgid(&t, target), result, "row {i}");
            assert_eq!(gids(&pr), after, "row {i}");
            assert_eq!(sugid(&pr), after != g, "row {i}");
        }
    }

    /// setgroups(2) (kern_prot.c:866-895): root only, checked first; a count below 0 or over
    /// `NGROUPS_MAX` is `EINVAL`; the groups copied in become the process's, `PS_SUGID` set.
    #[test]
    fn setgroups_is_root_s_and_bounded() {
        let (pr, t) = thread_as(360, (1000, 1000, 1000), (10, 10, 10));
        let set = UserPtr::<Gid>::new(UGIDS);
        assert_eq!(sys_setgroups(&t, 1, set), Err(Errno::EPERM));
        assert_eq!(sys_setgroups(&t, -1, set), Err(Errno::EPERM));
        assert_eq!(
            sys_setgroups(&t, NGROUPS_MAX as i32 + 1, set),
            Err(Errno::EPERM)
        );
        assert!(!sugid(&pr));

        let (pr, t) = thread_as(361, (0, 0, 0), (0, 0, 0));
        assert_eq!(sys_setgroups(&t, -1, set), Err(Errno::EINVAL));
        assert_eq!(sys_setgroups(&t, i32::MIN, set), Err(Errno::EINVAL));
        assert_eq!(
            sys_setgroups(&t, NGROUPS_MAX as i32 + 1, set),
            Err(Errno::EINVAL)
        );
        assert!(!sugid(&pr));

        let groups: [Gid; NGROUPS_MAX] = core::array::from_fn(|i| 100 + i as Gid);
        set.copyout_slice(&groups).unwrap();
        assert_eq!(sys_setgroups(&t, NGROUPS_MAX as i32, set), Ok(()));
        let c = pr.ucred();
        assert_eq!(c.cr_ngroups.get(), NGROUPS_MAX as i16);
        for (i, g) in c.cr_groups.iter().enumerate() {
            assert_eq!(g.get(), 100 + i as Gid);
        }
        assert!(sugid(&pr));
        drop(c);

        assert_eq!(sys_setgroups(&t, 0, set), Ok(()));
        assert_eq!(pr.ucred().cr_ngroups.get(), 0);
    }
}
/* </TESTS> */
