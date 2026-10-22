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

/*
 * Copyright (c) 1989, 1993
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
 *	@(#)ucred.h	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! Credentials: `struct ucred` (`<sys/ucred.h>`) and the functions of `kern/kern_prot.c` that
//! make, share and test them (`crget`, `crhold`, `crfree`, `crset`, `crcopy`, `crdup`,
//! `crfromxucred`, `groupmember`, `suser_ucred`). `suser(p)` reads a thread's credentials and
//! is `kernel::proc`'s; the set*id system calls stay in `bsd`'s `kern_prot.rs` until N4.6.
//!
//! Upstream: sys/kern/kern_prot.c @ 3ce1f3f79392
//! Upstream: sys/sys/ucred.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_prot.rs@3c62ede68802
//! LZ: sys/sys/ucred.rs@3c62ede68802
//!
//! A process holds a reference to its credentials, each of its threads one to a copy it
//! refreshes on the way back to user mode (`refreshcreds`), and files, sockets, mounts and
//! nfs nodes keep references of their own. Credentials are never changed once shared: the
//! set*id calls make new ones (`crget` + `crset` + the changes) and swap the process's
//! pointer, so a reader of a shared `struct ucred` needs no lock.
//!
//! ## Deviations
//! - `crcopy` of credentials nobody else holds copies them too, where the C hands back the
//!   same structure to be changed in place: the pool `ucredpl` is a recorded pool (a `&Ucred`
//!   a caller holds can be counted again, [`CredRef::from_ref`]), which never gives a value
//!   mutably. One more pool item per such call; the credentials' contents and every id
//!   userland reads are the same.
//! - `crfromxucred` takes the fields of `struct xucred` (which stays in `bsd`'s `ucred.rs`
//!   with the other structures userland shares until N4.6) and fills a `&mut Ucred`.
//!
//! ## Redesign
//! - `struct ucred *` handed around as LZ's `&'static Ucred` (`crget`/`crhold` return it,
//!   `crfree` gives it back by convention) is a [`CredRef`], a counted handle of the pool
//!   `ucredpl`: `Clone` is `crhold`, `Drop` is `crfree`, and the item goes back with the last
//!   handle, so a credential freed while held cannot be written. `cr_refcnt` is the handle's
//!   count.
//! - The members, LZ's `Cell`s written through the shared pointer while "unshared", are
//!   plain data read by `get()` ([`Ro`], decision 10: the 60 files that read `cr.cr_uid.get()`
//!   compile unchanged) and written only through `&mut Ucred`: a credential is built as a
//!   value (`crset` into a copy, the ids changed) and then placed ([`CredRef::new`]), so a
//!   shared credential is immutable by type, not by convention.
//! - The `NOCRED`/`FSCRED` sentinels stay `bsd`'s pointer constants while the VFS passes
//!   `*const Ucred` (N5).

use core::ops::Deref;
use core::sync::atomic::{AtomicBool, Ordering};

use hal::pool::{PR_WAITOK, PoolArc, PoolHome, TypedPool};
use hal::rwlock::Rwlocked;
use hal::sys::errno::Errno;
use hal::sys::types::{Gid, Uid};

use crate::assert::kpanic;

/// `NGROUPS_MAX`: the most supplementary groups a credential holds (`<sys/syslimits.h>`).
pub const NGROUPS_MAX: usize = 16;

/// A member read with `get()`, written only through `&mut` (decision 10): the reader of
/// LZ's `Cell` member, for data that is never changed while shared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Ro<T>(T);

impl<T: Copy> Ro<T> {
    /// `v`.
    pub const fn new(v: T) -> Self {
        Self(v)
    }

    /// The value.
    #[inline]
    pub const fn get(&self) -> T {
        self.0
    }

    /// Stores `v`: only through the exclusive reference of a value being built.
    #[inline]
    pub fn set(&mut self, v: T) {
        self.0 = v;
    }
}

/// `struct ucred`: the ids of a process or of what it opened. Plain data, built as a value
/// and shared through [`CredRef`]s once placed.
pub struct Ucred {
    // The following fields are all copied by crset() (cr_startcopy = cr_uid).
    /// `cr_uid`: effective user id.
    pub cr_uid: Ro<Uid>,
    /// `cr_ruid`: real user id.
    pub cr_ruid: Ro<Uid>,
    /// `cr_svuid`: saved effective user id.
    pub cr_svuid: Ro<Uid>,
    /// `cr_gid`: effective group id.
    pub cr_gid: Ro<Gid>,
    /// `cr_rgid`: real group id.
    pub cr_rgid: Ro<Gid>,
    /// `cr_svgid`: saved effective group id.
    pub cr_svgid: Ro<Gid>,
    /// `cr_ngroups`: number of groups.
    pub cr_ngroups: Ro<i16>,
    /// `cr_groups`: groups.
    pub cr_groups: [Ro<Gid>; NGROUPS_MAX],
    /// Where the pool placed it ([`CredRef::from_ref`]); empty for a value of a caller's own
    /// (nfs's per-request credential, an export's anonymous one).
    home: PoolHome<Ucred>,
}

impl Ucred {
    /// Root's credentials, no groups (what `crget` returns zeroed).
    pub const fn new() -> Self {
        Self {
            cr_uid: Ro(0),
            cr_ruid: Ro(0),
            cr_svuid: Ro(0),
            cr_gid: Ro(0),
            cr_rgid: Ro(0),
            cr_svgid: Ro(0),
            cr_ngroups: Ro(0),
            cr_groups: [Ro(0); NGROUPS_MAX],
            home: PoolHome::new(),
        }
    }

    /// The supplementary groups in use (`cr_groups[0..cr_ngroups]`); a count outside
    /// `0..=NGROUPS_MAX` reads as none.
    pub fn groups(&self) -> &[Ro<Gid>] {
        let n = usize::try_from(self.cr_ngroups.get()).unwrap_or(0);
        self.cr_groups.get(..n).unwrap_or(&[])
    }
}

impl Default for Ucred {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Ucred {
    /// `*newcr = *cr` of `crcopy`/`crdup`: every id, a fresh (empty) home.
    fn clone(&self) -> Self {
        let mut c = Ucred::new();
        crset(&mut c, self);
        c
    }
}

hal::pool_home!(Ucred, home);

/// `ucred_pool`, `ucredpl`: a recorded pool, so a lent `&Ucred` can be counted again.
static UCRED_POOL: TypedPool<Ucred> = TypedPool::new();
/// Whether [`UCRED_POOL`] is made.
static UCRED_POOL_MADE: AtomicBool = AtomicBool::new(false);
/// Serialises the making of [`UCRED_POOL`] (`pool_init` may sleep), once.
static UCRED_POOL_INIT: Rwlocked<()> = Rwlocked::new("ucredpl", ());

/// [`UCRED_POOL`], made at its first need (`procinit`, or a credential before it: the host
/// tests make credentials without `main`), at `IPL_MPFLOOR` as the C's `pool_init`.
fn ucred_pool() -> &'static TypedPool<Ucred> {
    if !UCRED_POOL_MADE.load(Ordering::Acquire) {
        let _w = UCRED_POOL_INIT.write();
        if !UCRED_POOL_MADE.load(Ordering::Relaxed) {
            if UCRED_POOL
                .init_recorded(hal::intr::mpfloor(), 0, "ucredpl")
                .is_err()
            {
                kpanic(format_args!("crget: cannot make ucredpl"));
            }
            UCRED_POOL_MADE.store(true, Ordering::Release);
        }
    }
    &UCRED_POOL
}

/// `ucredpl`'s items out (`pr_nout`).
pub fn ucred_nout() -> u64 {
    UCRED_POOL.nout()
}

/// A counted reference to placed credentials (`struct ucred *` with its `cr_refcnt`
/// reference): `Clone` is `crhold`, `Drop` is `crfree`; `Deref<Target = Ucred>`.
#[derive(Clone)]
pub struct CredRef(PoolArc<Ucred>);

impl CredRef {
    /// `crget()` + the fields: `cr` placed in a `ucredpl` item with one reference, the
    /// caller's. `pool_get(PR_WAITOK)` sleeps for an item, as the C's.
    pub fn new(cr: Ucred) -> Self {
        match ucred_pool().get_homed(PR_WAITOK, || cr) {
            Some(a) => CredRef(a),
            None => kpanic(format_args!("crget: ucred_pool is empty")),
        }
    }

    /// `crhold(cr)` of credentials a caller only borrows (a `struct ucred *` argument): one
    /// more reference, `None` for credentials no pool placed (a caller's own value).
    pub fn from_ref(cr: &Ucred) -> Option<Self> {
        PoolArc::from_ref(cr).map(CredRef)
    }

    /// Whether two references name the same credentials (the C's pointer compare,
    /// `uc != pr->ps_ucred`).
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        PoolArc::ptr_eq(&a.0, &b.0)
    }

    /// The references alive now (`cr_refcnt.r_refs`, a snapshot).
    pub fn count(this: &Self) -> usize {
        PoolArc::count(&this.0)
    }

    /// The credentials' address, for what prints or compares it as a number (`kinfo_proc`'s
    /// `p_ucred` slot, the VFS's `*const Ucred` arguments); never dereferenced here.
    pub fn as_ptr(this: &Self) -> *const Ucred {
        PoolArc::as_ptr(&this.0)
    }
}

impl Deref for CredRef {
    type Target = Ucred;

    fn deref(&self) -> &Ucred {
        &self.0
    }
}

/// `crset(newcr, cr)`: every member after the reference count of `cr` into `newcr`.
#[inline]
pub fn crset(newcr: &mut Ucred, cr: &Ucred) {
    newcr.cr_uid = cr.cr_uid;
    newcr.cr_ruid = cr.cr_ruid;
    newcr.cr_svuid = cr.cr_svuid;
    newcr.cr_gid = cr.cr_gid;
    newcr.cr_rgid = cr.cr_rgid;
    newcr.cr_svgid = cr.cr_svgid;
    newcr.cr_ngroups = cr.cr_ngroups;
    newcr.cr_groups = cr.cr_groups;
}

/// `groupmember(gid, cred)`: whether `gid` is the effective group or one of the groups.
pub fn groupmember(gid: Gid, cred: &Ucred) -> bool {
    cred.cr_gid.get() == gid || cred.groups().iter().any(|g| g.get() == gid)
}

/// `suser_ucred(cred)`: `Ok` for root's credentials, `EPERM` otherwise.
pub fn suser_ucred(cred: &Ucred) -> Result<(), Errno> {
    if cred.cr_uid.get() == 0 {
        Ok(())
    } else {
        Err(Errno::EPERM)
    }
}

/// `crget()`: root's credentials, no groups, with one reference.
pub fn crget() -> CredRef {
    CredRef::new(Ucred::new())
}

/// `crhold(cr)`: one more reference to the same credentials.
#[inline]
pub fn crhold(cr: &CredRef) -> CredRef {
    cr.clone()
}

/// `crfree(cr)`: the caller's reference given back; the item goes with the last one.
#[inline]
pub fn crfree(cr: CredRef) {
    drop(cr);
}

/// `crcopy(cr)`: the caller's reference given up for a private copy to change: the value,
/// to be placed by [`CredRef::new`] once changed (see the module's deviations).
pub fn crcopy(cr: CredRef) -> Ucred {
    let copy = (*cr).clone();
    crfree(cr);
    copy
}

/// `crdup(cr)`: new credentials, held once, with `cr`'s ids.
pub fn crdup(cr: &Ucred) -> CredRef {
    CredRef::new(cr.clone())
}

/// `crfromxucred(cr, xcr)`: `cr`'s user, group and groups from those of a `struct xucred`;
/// `EINVAL` for a group count outside `0..=NGROUPS_MAX`, nothing changed then.
pub fn crfromxucred(
    cr: &mut Ucred,
    uid: Uid,
    gid: Gid,
    ngroups: i16,
    groups: &[Gid; NGROUPS_MAX],
) -> Result<(), Errno> {
    let n = usize::try_from(ngroups).map_err(|_| Errno::EINVAL)?;
    if n > NGROUPS_MAX {
        return Err(Errno::EINVAL);
    }
    cr.cr_uid.set(uid);
    cr.cr_gid.set(gid);
    cr.cr_ngroups.set(ngroups);
    for (to, &from) in cr.cr_groups.iter_mut().zip(&groups[..n]) {
        to.set(from);
    }
    Ok(())
}

const _: () = {
    assert!(size_of::<Ro<u32>>() == size_of::<u32>());
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

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

    fn uids(cr: &Ucred) -> (Uid, Uid, Uid) {
        (cr.cr_ruid.get(), cr.cr_uid.get(), cr.cr_svuid.get())
    }

    #[test]
    fn crget_crhold_crfree_count_references() {
        testutil::install();
        let cr = crget();
        assert_eq!(CredRef::count(&cr), 1);
        assert_eq!(uids(&cr), (0, 0, 0));
        assert_eq!(cr.cr_ngroups.get(), 0);

        let two = crhold(&cr);
        assert!(CredRef::ptr_eq(&cr, &two));
        assert_eq!(CredRef::count(&cr), 2);
        crfree(two);
        assert_eq!(CredRef::count(&cr), 1);
        crfree(cr);
    }

    #[test]
    fn a_borrowed_credential_is_held_again_only_when_placed() {
        testutil::install();
        let cr = CredRef::new(ids(1000, 100));
        let lent: &Ucred = &cr;
        let again = CredRef::from_ref(lent).unwrap();
        assert!(CredRef::ptr_eq(&cr, &again));
        assert_eq!(CredRef::count(&cr), 2);
        drop(cr);
        assert_eq!(uids(&again), (1000, 1000, 1000));
        // A caller's own value (nfs's per-request credential) is not held.
        let own = ids(5, 5);
        assert!(CredRef::from_ref(&own).is_none());
        // Its copy, placed, is.
        let placed = crdup(&own);
        assert!(CredRef::from_ref(&placed).is_some());
    }

    #[test]
    fn crcopy_and_crdup_copy_every_id() {
        testutil::install();
        let mut v = ids(1000, 100);
        v.cr_ngroups.set(2);
        v.cr_groups[0].set(100);
        v.cr_groups[1].set(20);
        let cr = CredRef::new(v);
        let held = crhold(&cr);

        let mut copy = crcopy(cr);
        assert_eq!(CredRef::count(&held), 1);
        assert_eq!(uids(&copy), (1000, 1000, 1000));
        assert_eq!(copy.cr_svgid.get(), 100);
        assert_eq!(copy.cr_ngroups.get(), 2);
        assert_eq!(copy.cr_groups[1].get(), 20);
        copy.cr_uid.set(0);
        let changed = CredRef::new(copy);
        assert_eq!(changed.cr_uid.get(), 0);
        assert_eq!(held.cr_uid.get(), 1000);

        let dup = crdup(&held);
        assert!(!CredRef::ptr_eq(&dup, &held));
        assert_eq!(CredRef::count(&dup), 1);
        assert_eq!(uids(&dup), uids(&held));
        assert_eq!(dup.groups().len(), 2);
    }

    #[test]
    fn groupmember_suser_and_crfromxucred() {
        let mut cr = ids(1000, 100);
        cr.cr_ngroups.set(2);
        cr.cr_groups[0].set(100);
        cr.cr_groups[1].set(20);
        assert!(groupmember(100, &cr));
        assert!(groupmember(20, &cr));
        assert!(!groupmember(30, &cr));
        // A group past cr_ngroups is not a member.
        cr.cr_groups[2].set(30);
        assert!(!groupmember(30, &cr));
        assert_eq!(suser_ucred(&cr), Err(Errno::EPERM));
        assert_eq!(suser_ucred(&ids(0, 0)), Ok(()));

        let mut groups = [0; NGROUPS_MAX];
        groups[..3].copy_from_slice(&[7, 8, 9]);
        let mut x = Ucred::new();
        assert_eq!(crfromxucred(&mut x, 42, 7, 3, &groups), Ok(()));
        assert_eq!(
            (x.cr_uid.get(), x.cr_gid.get(), x.cr_ngroups.get()),
            (42, 7, 3)
        );
        assert!(groupmember(9, &x));
        let before = (x.cr_uid.get(), x.cr_ngroups.get());
        assert_eq!(crfromxucred(&mut x, 1, 1, -1, &groups), Err(Errno::EINVAL));
        assert_eq!(
            crfromxucred(&mut x, 1, 1, NGROUPS_MAX as i16 + 1, &groups),
            Err(Errno::EINVAL)
        );
        assert_eq!((x.cr_uid.get(), x.cr_ngroups.get()), before);
        assert_eq!(
            crfromxucred(&mut x, 1, 1, NGROUPS_MAX as i16, &groups),
            Ok(())
        );
    }
}
/* </TESTS> */
