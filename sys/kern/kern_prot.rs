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
//! System calls related to processes and protection: `bsd`'s `sysent[]` rows and names over
//! `kernel::prot` and `kernel::cred`.
//!
//! Upstream: sys/kern/kern_prot.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_prot.rs@3c62ede68802
//!
//! An adapter (N4.6a): every function of the C file is `kernel::prot`'s (the system calls,
//! with typed arguments, `sugid_change`, `proc_cansugid`, `dorefreshcreds`) or
//! `kernel::cred`'s (`crget`, `crhold`, `crfree`, `crset`, `crcopy`, `crdup`,
//! `crfromxucred`, `groupmember`, `suser_ucred`); this file decodes `sys_*`'s arguments for
//! `bsd`'s `sysent[]` rows (and the ABI module's `Native` entries, which run those rows),
//! and keeps the names its callers import.
//!
//! ## Redesign
//! - LZ's credentials were `*const Ucred` with a manual `cr_refcnt`, written in place by the
//!   set*id calls after `crget` (N4.2): they are the kernel crate's `CredRef`s, never written
//!   once shared (`kernel::prot`'s `sugid_change`).
//! - N4.6a: the system calls' bodies moved to `kernel::prot`, which takes typed arguments
//!   (`UserPtr<T>` for a user pointer) and returns what the C puts in `retval[0]`.

use hal::user::UserPtr;

use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::errno::Errno;
use crate::sys::proc::{Proc, Process};
use crate::sys::syscallargs::{
    SysGetgroupsArgs, SysGetloginRArgs, SysGetpgidArgs, SysGetresgidArgs, SysGetresuidArgs,
    SysGetsidArgs, SysGetthrnameArgs, SysSetTcbArgs, SysSetegidArgs, SysSeteuidArgs, SysSetgidArgs,
    SysSetgroupsArgs, SysSetloginArgs, SysSetpgidArgs, SysSetregidArgs, SysSetresgidArgs,
    SysSetresuidArgs, SysSetreuidArgs, SysSetthrnameArgs, SysSetuidArgs,
};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::Register;

pub use kernel::cred::{
    crcopy, crdup, crfree, crfromxucred, crget, crhold, crset, groupmember, suser_ucred,
};

/// `getpid(2)`.
pub fn sys_getpid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_getpid(p) as Register;
    Ok(())
}

/// `getthrid(2)`.
pub fn sys_getthrid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_getthrid(p) as Register;
    Ok(())
}

/// `getppid(2)`.
pub fn sys_getppid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_getppid(p) as Register;
    Ok(())
}

/// `getpgrp(2)`.
pub fn sys_getpgrp(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_getpgrp(p) as Register;
    Ok(())
}

/// `getpgid(2)`.
pub fn sys_getpgid(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetpgidArgs = sysargs(v);
    retval[0] = kernel::prot::sys_getpgid(curp, uap.pid.get())? as Register;
    Ok(())
}

/// `getsid(2)`.
pub fn sys_getsid(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetsidArgs = sysargs(v);
    retval[0] = kernel::prot::sys_getsid(curp, uap.pid.get())? as Register;
    Ok(())
}

/// `getuid(2)`.
pub fn sys_getuid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_getuid(p) as Register;
    Ok(())
}

/// `geteuid(2)`.
pub fn sys_geteuid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_geteuid(p) as Register;
    Ok(())
}

/// `issetugid(2)`.
pub fn sys_issetugid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_issetugid(p) as Register;
    Ok(())
}

/// `getgid(2)`.
pub fn sys_getgid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_getgid(p) as Register;
    Ok(())
}

/// `getegid(2)`.
pub fn sys_getegid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_getegid(p) as Register;
    Ok(())
}

/// `getgroups(2)`.
pub fn sys_getgroups(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetgroupsArgs = sysargs(v);
    retval[0] = kernel::prot::sys_getgroups(
        p,
        uap.gidsetsize.get(),
        UserPtr::new(uap.gidset.get() as usize),
    )? as Register;
    Ok(())
}

/// `setsid(2)`.
pub fn sys_setsid(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_setsid(p)? as Register;
    Ok(())
}

/// `setpgid(2)`.
pub fn sys_setpgid(curp: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetpgidArgs = sysargs(v);
    kernel::prot::sys_setpgid(curp, uap.pid.get(), uap.pgid.get())
}

/// `getresuid(2)`.
pub fn sys_getresuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetresuidArgs = sysargs(v);
    kernel::prot::sys_getresuid(
        p,
        UserPtr::new(uap.ruid.get() as usize),
        UserPtr::new(uap.euid.get() as usize),
        UserPtr::new(uap.suid.get() as usize),
    )
}

/// `setresuid(2)`.
pub fn sys_setresuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetresuidArgs = sysargs(v);
    kernel::prot::sys_setresuid(p, uap.ruid.get(), uap.euid.get(), uap.suid.get())
}

/// `getresgid(2)`.
pub fn sys_getresgid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetresgidArgs = sysargs(v);
    kernel::prot::sys_getresgid(
        p,
        UserPtr::new(uap.rgid.get() as usize),
        UserPtr::new(uap.egid.get() as usize),
        UserPtr::new(uap.sgid.get() as usize),
    )
}

/// `setresgid(2)`.
pub fn sys_setresgid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetresgidArgs = sysargs(v);
    kernel::prot::sys_setresgid(p, uap.rgid.get(), uap.egid.get(), uap.sgid.get())
}

/// `setregid(2)`.
pub fn sys_setregid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetregidArgs = sysargs(v);
    kernel::prot::sys_setregid(p, uap.rgid.get(), uap.egid.get())
}

/// `setreuid(2)`.
pub fn sys_setreuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetreuidArgs = sysargs(v);
    kernel::prot::sys_setreuid(p, uap.ruid.get(), uap.euid.get())
}

/// `setuid(2)`.
pub fn sys_setuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetuidArgs = sysargs(v);
    kernel::prot::sys_setuid(p, uap.uid.get())
}

/// `seteuid(2)`.
pub fn sys_seteuid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSeteuidArgs = sysargs(v);
    kernel::prot::sys_seteuid(p, uap.euid.get())
}

/// `setgid(2)`.
pub fn sys_setgid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetgidArgs = sysargs(v);
    kernel::prot::sys_setgid(p, uap.gid.get())
}

/// `setegid(2)`.
pub fn sys_setegid(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetegidArgs = sysargs(v);
    kernel::prot::sys_setegid(p, uap.egid.get())
}

/// `setgroups(2)`.
pub fn sys_setgroups(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetgroupsArgs = sysargs(v);
    kernel::prot::sys_setgroups(
        p,
        uap.gidsetsize.get(),
        UserPtr::new(uap.gidset.get() as usize),
    )
}

/// `suser`: test whether this process has special user powers.
pub fn suser(p: &Proc) -> Result<(), Errno> {
    kernel::prot::suser(p)
}

/// `getlogin_r(2)`. The error is the return value, as in C.
pub fn sys_getlogin_r(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetloginRArgs = sysargs(v);
    retval[0] = kernel::prot::sys_getlogin_r(
        p,
        UserPtr::new(uap.namebuf.get() as usize),
        uap.namelen.get(),
    ) as Register;
    Ok(())
}

/// `setlogin(2)`.
pub fn sys_setlogin(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetloginArgs = sysargs(v);
    kernel::prot::sys_setlogin(p, UserPtr::new(uap.namebuf.get() as usize))
}

/// `proc_cansugid`: check if a process is allowed to raise its privileges.
pub fn proc_cansugid(p: &Proc) -> bool {
    kernel::prot::proc_cansugid(p)
}

/// `__set_tcb(2)`.
#[allow(non_snake_case)] // the C name, verbatim: the syscall table finds it by name
pub fn sys___set_tcb(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetTcbArgs = sysargs(v);
    kernel::prot::sys___set_tcb(p, uap.tcb.get() as usize)
}

/// `__get_tcb(2)`.
#[allow(non_snake_case)] // the C name, verbatim: the syscall table finds it by name
pub fn sys___get_tcb(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys___get_tcb(p) as Register;
    Ok(())
}

/// `getthrname(2)`. The error is the return value, as in C.
pub fn sys_getthrname(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetthrnameArgs = sysargs(v);
    retval[0] = kernel::prot::sys_getthrname(
        curp,
        uap.tid.get(),
        UserPtr::new(uap.name.get() as usize),
        uap.len.get(),
    )? as Register;
    Ok(())
}

/// `setthrname(2)`. The error is the return value, as in C; a name too long is truncated.
pub fn sys_setthrname(curp: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetthrnameArgs = sysargs(v);
    retval[0] =
        kernel::prot::sys_setthrname(curp, uap.tid.get(), UserPtr::new(uap.name.get() as usize))?
            as Register;
    Ok(())
}

/// `dorefreshcreds`: refresh the thread's copy of the process's credentials.
pub fn dorefreshcreds(_pr: &Process, p: &Proc) {
    kernel::prot::dorefreshcreds::<BsdProcGlue>(p);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the credentials and the id system calls: the reference counting of
    // `crget`/`crhold`/`crfree`/`crcopy`/`crdup`, `groupmember`, `suser`, and the permission
    // rules of the set*id calls against threads built by hand.

    use core::sync::atomic::Ordering;
    use std::sync::MutexGuard;
    use std::{assert, assert_eq, assert_ne};

    use super::*;
    use crate::kern::proc_glue::chgproccnt;
    use crate::kern::proc_glue::{procinit, test_process, test_thread};
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::proc::refreshcreds;
    use crate::sys::proc::{_MAXCOMLEN, PS_SUGID};
    use crate::sys::syslimits::NGROUPS_MAX;
    use crate::sys::systm::{kernel_lock, kernel_unlock};
    use crate::sys::types::{Gid, Uid};
    use crate::sys::ucred::Xucred;
    use crate::sys::ucred::{CredRef, Ucred};

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
        let Some(u) = hal::thread::UArea::alloc() else {
            std::panic!("no test u-area");
        };
        p.install_uarea(u);
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
            _pad0: 0,
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

    /// The user memory of these tests: the user-copy double's, a range of their own.
    const UMEM: usize = kernel::testutil::USER_BASE + 0xb000;

    /// `bytes` at user address `at`.
    fn uput(at: usize, bytes: &[u8]) {
        UserPtr::<u8>::new(at).copyout_slice(bytes).unwrap();
    }

    /// `len` bytes at user address `at`.
    fn uget(at: usize, len: usize) -> std::vec::Vec<u8> {
        let mut v = std::vec![0u8; len];
        UserPtr::<u8>::new(at).copyin_slice(&mut v).unwrap();
        v
    }

    /// The ids in user memory at `at`, `n` of them.
    fn uids_at(at: usize, n: usize) -> std::vec::Vec<u32> {
        uget(at, 4 * n)
            .chunks(4)
            .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    #[test]
    fn the_get_calls_copy_out_and_the_groups_round_trip() {
        let _g = setup();
        let mut rv: [Register; 2] = [0; 2];

        let mut v = ids(1000, 100);
        v.cr_svuid.set(7);
        let p = thread(cred(0, 0));
        // A root thread installs cr's groups, then reads them back.
        let groups = UMEM;
        let back = UMEM + 0x100;
        let resid = UMEM + 0x200;
        let mut g3 = std::vec::Vec::new();
        for gid in [100u32, 20, 30] {
            g3.extend_from_slice(&gid.to_ne_bytes());
        }
        uput(groups, &g3);
        assert_eq!(
            sys_setgroups(p, &args(&[3, groups as Register]), &mut rv),
            Ok(())
        );
        refreshcreds(p);
        assert_eq!(sys_getgroups(p, &args(&[0, 0]), &mut rv), Ok(()));
        assert_eq!(rv[0], 3);
        uput(back, &[0u8; 16]);
        assert_eq!(
            sys_getgroups(p, &args(&[2, back as Register]), &mut rv),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sys_getgroups(p, &args(&[4, back as Register]), &mut rv),
            Ok(())
        );
        assert_eq!((rv[0], uids_at(back, 4)), (3, std::vec![100, 20, 30, 0]));

        let q = thread(CredRef::new(v));
        uput(resid, &[9u8, 0, 0, 0, 9, 0, 0, 0, 9, 0, 0, 0]);
        assert_eq!(
            sys_getresuid(
                q,
                &args(&[resid as Register, 0, (resid + 8) as Register]),
                &mut rv
            ),
            Ok(())
        );
        assert_eq!(uids_at(resid, 3), std::vec![1000, 9, 7]);
        assert_eq!(
            sys_setgroups(q, &args(&[1, groups as Register]), &mut rv),
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
        let uname = UMEM + 0x300;
        let out = UMEM + 0x400;
        uput(uname, name);
        assert_eq!(
            sys_setthrname(p, &args(&[0, uname as Register]), &mut rv),
            Ok(())
        );
        assert_eq!((rv[0], &*p.name()), (0, &b"worker"[..]));
        uput(out, &[0xffu8; _MAXCOMLEN]);
        assert_eq!(
            sys_getthrname(p, &args(&[0, out as Register, 4]), &mut rv),
            Ok(())
        );
        assert_eq!(rv[0], Errno::ERANGE as i32 as Register);
        assert_eq!(
            sys_getthrname(
                p,
                &args(&[0, out as Register, _MAXCOMLEN as Register]),
                &mut rv
            ),
            Ok(())
        );
        assert_eq!((rv[0], uget(out, 7)), (0, name.to_vec()));
        // A tid lookup walks the tid hash: under the kernel lock, as a system call runs.
        kernel_lock();
        let r = sys_getthrname(p, &args(&[12345, out as Register, 4]), &mut rv);
        kernel_unlock();
        assert_eq!(r, Err(Errno::ESRCH));
    }
}
/* </TESTS> */
