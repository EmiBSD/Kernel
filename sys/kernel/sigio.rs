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

/*-
 * Copyright (c) 1990, 1993
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
 *	@(#)filedesc.h	8.1 (Berkeley) 6/2/93
 * $FreeBSD: head/sys/sys/sigio.h 326023 2017-11-20 19:43:44Z pfg $
 */
/* </LICENSES> */

/* <CODE> */
//! The `SIGIO`/`SIGURG` registrations (`<sys/sigio.h>` and the `sigio_*` functions and
//! `pgsigio` of `kern/kern_sig.c`): a file, socket or device that a process or process group
//! asked to be signalled about (`fcntl(F_SETOWN)`, `FIOSETOWN`, `TIOCSPGRP`) holds a
//! [`SigioRef`]; `pgsigio` signals its owner with the credentials of the process that
//! registered it.
//!
//! Upstream: sys/kern/kern_sig.c @ 3ce1f3f79392
//! Upstream: sys/sys/sigio.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_sig.rs@3c62ede68802
//! LZ: sys/sys/sigio.rs@3c62ede68802
//!
//! ## Redesign
//! - The C links a `malloc(9)`ed `struct sigio` from two sides: the registering object's
//!   `sir_sigio` points at it, and it sits on its owner's `ps_sigiolst`/`pg_sigiolst`
//!   with `sio_myref` pointing back at the `sigio_ref`, so the owner's death can clear the
//!   object's pointer (LZ: 16 `unsafe` dereferences of raw `Cell` links, under conventions
//!   "the owner empties its list before it goes", "a registered sigio stays allocated").
//!   Here `sigio_lock` owns every registration: [`SIGIO_LOCK`] is a mutex at `IPL_HIGH`
//!   whose data is the chain of [`MBox`]es of `M_SIGIO` (each registration its own
//!   `malloc(9)` block, so `vmstat -m`'s `sigio` row counts what the C's does). Both sides
//!   name a registration without a pointer: the object's [`SigioRef`] holds its address as
//!   a number (`sir_sigio`, compared, never followed, valid only while the chain holds a
//!   registration at that address whose `sio_myref` is the object's), and the owner is its
//!   `sio_pgid` (a process while positive, a group while negative), found with
//!   `prfind`/`pgfind` under the kernel lock when a signal is sent. An owner's death
//!   (`sigio_freelist_process` from `exit1`, `sigio_freelist_pgrp` from `pgdelete`) takes its
//!   registrations out of the chain and frees them, as the C's `sigio_freelist`; the
//!   objects' stale numbers find nothing afterwards, which is the C's `sir_sigio = NULL`.
//!   No list head remains in `struct process` or `struct pgrp`.
//! - A registration's credentials are its own counted reference, dropped with its box after
//!   `sigio_lock` is left (the C's `sigio_del` after `mtx_leave`).
//! - A [`SigioRef`] is one atomic word, valid all zero: the softcs and sockets that embed
//!   one (`M_ZERO`, `PR_ZERO`) hold an empty registration, as in C.
//!
//! ## Deviations
//! - `pgsigio` and `sigio_getown` find a registration by a walk of every registration
//!   (system-wide, under `sigio_lock`), and `pgsigio` its owner by a hash lookup under the
//!   kernel lock (which the C holds there), where the C follows two pointers. There are as
//!   many registrations as descriptors in `FIOASYNC` mode; neither call is on a hot path.
//! - `sigio_freelist`'s unlocked "nothing to do" test reads the number of registrations
//!   system-wide where the C reads its own list's emptiness: the same answer when no
//!   registration exists, one locked walk when some do.

use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

use hal::intr::IPL_HIGH;
use hal::klock;
use hal::malloc::{M_WAITOK, MBox};
use hal::mutex::Mutexed;
use hal::sys::errno::Errno;
use hal::sys::ioccom::{_ior, _iow};
use hal::sys::types::Pid;

use crate::assert::kpanic;
use crate::cred::CredRef;
use crate::proc::{PS_CONTROLT, PS_EXITING, Pgrp, ProcGlue, Process, pgfind, prfind};
use crate::sig::{cansigio, prsignal};

/// `M_SIGIO` (`<sys/malloc.h>`): sigio structures.
pub const M_SIGIO: i32 = 40;
/// `TIOCGPGRP` (`<sys/ttycom.h>`): get pgrp of tty.
pub const TIOCGPGRP: u64 = _ior::<i32>(b't', 119);
/// `TIOCSPGRP` (`<sys/ttycom.h>`): set pgrp of tty.
pub const TIOCSPGRP: u64 = _iow::<i32>(b't', 118);

/// `struct sigio_ref`: sigio registration.
///
/// Locking: S: `sigio_lock`.
#[derive(Debug)]
pub struct SigioRef {
    /// \[S\] `sir_sigio`: the address of the associated registration, 0 for none (see the
    /// module's redesign).
    sir_sigio: AtomicUsize,
}

impl SigioRef {
    /// `sigio_init`: an empty registration.
    pub const fn new() -> Self {
        Self {
            sir_sigio: AtomicUsize::new(0),
        }
    }

    /// `sir_sigio` as ddb prints it (`%p`): the address of the registration, 0 for none (a
    /// stale number after its owner's death reads as the C's pointer before it was cleared).
    pub fn sigio_addr(&self) -> usize {
        self.sir_sigio.load(Ordering::Relaxed)
    }

    /// The address of this registration slot, which a registration records (`sio_myref`).
    fn addr(&self) -> usize {
        ptr::from_ref(self).addr()
    }
}

impl Default for SigioRef {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct sigio`: this structure holds the information needed to send a SIGIO or a SIGURG
/// signal to a process or process group when new data arrives on a device or socket. It is
/// kept in [`SIGIO_LOCK`]'s chain, so that the entire set of an owner may be revoked when the
/// process exits or the process group disappears.
///
/// Locking: I: immutable after creation; S: `sigio_lock`.
struct Sigio {
    /// \[S\] The next registration of the chain.
    sio_next: Option<MBox<Sigio>>,
    /// \[I\] `sio_myref`: the address of the [`SigioRef`] that holds this registration.
    sio_myref: usize,
    /// \[I\] `sio_ucred`: the registrant's credentials (`None` only while being made).
    sio_ucred: Option<CredRef>,
    /// \[I\] `sio_pgid`: pgid for signals: the process while positive, the group (negated)
    /// while negative.
    sio_pgid: Pid,
}

impl Sigio {
    /// The address of the registration, which its slot records.
    fn addr(&self) -> usize {
        ptr::from_ref(self).addr()
    }
}

/// `sigio_lock`, and the registrations it keeps: the chain of every `struct sigio`.
static SIGIO_LOCK: Mutexed<Option<MBox<Sigio>>> = Mutexed::new(IPL_HIGH, None);

/// The number of registrations in [`SIGIO_LOCK`]'s chain, changed under the lock: the
/// unlocked test of `sigio_freelist`.
static SIGIO_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Frees the registrations of a chain taken out of [`SIGIO_LOCK`] (`sigio_del`): their
/// credentials go with their boxes, one at a time (no recursion down the chain).
fn sigio_del(mut rmlist: Option<MBox<Sigio>>) {
    while let Some(mut sigio) = rmlist {
        rmlist = sigio.sio_next.take();
        drop(sigio);
    }
}

/// The registration `sir` holds, in the chain `chain`: one at `sir`'s recorded address whose
/// `sio_myref` is `sir`.
fn sigio_of<'a>(chain: &'a Option<MBox<Sigio>>, sir: &SigioRef) -> Option<&'a Sigio> {
    let at = sir.sir_sigio.load(Ordering::Relaxed);
    if at == 0 {
        return None;
    }
    let mut cur = chain.as_deref();
    while let Some(s) = cur {
        if s.addr() == at && s.sio_myref == sir.addr() {
            return Some(s);
        }
        cur = s.sio_next.as_deref();
    }
    None
}

/// Takes the first registration `pred` accepts out of `chain`.
fn chain_take(
    chain: &mut Option<MBox<Sigio>>,
    mut pred: impl FnMut(&Sigio) -> bool,
) -> Option<MBox<Sigio>> {
    // Find the position with a shared walk, then step to it mutably.
    let mut at = 0usize;
    let mut cur = chain.as_deref();
    loop {
        let s = cur?;
        if pred(s) {
            break;
        }
        at += 1;
        cur = s.sio_next.as_deref();
    }
    let mut slot = chain;
    for _ in 0..at {
        slot = &mut slot.as_mut()?.sio_next;
    }
    let mut node = slot.take()?;
    *slot = node.sio_next.take();
    SIGIO_COUNT.fetch_sub(1, Ordering::Relaxed);
    Some(node)
}

/// Puts `node` at the head of `chain` (`LIST_INSERT_HEAD`).
fn chain_push(chain: &mut Option<MBox<Sigio>>, mut node: MBox<Sigio>) {
    node.sio_next = chain.take();
    *chain = Some(node);
    SIGIO_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// `sigio_unlink`: detaches the registration of `sir` from the chain (under `sigio_lock`):
/// the box, which the caller frees after leaving the lock. A stale number is cleared.
fn sigio_unlink(chain: &mut Option<MBox<Sigio>>, sir: &SigioRef) -> Option<MBox<Sigio>> {
    let at = sir.sir_sigio.swap(0, Ordering::Relaxed);
    if at == 0 {
        return None;
    }
    let me = sir.addr();
    chain_take(chain, |s| s.addr() == at && s.sio_myref == me)
}

/// `sigio_init(sir)`.
pub fn sigio_init(sir: &SigioRef) {
    sir.sir_sigio.store(0, Ordering::Relaxed);
}

/// `pgsigio`: send a SIGIO or SIGURG signal to a process or process group using stored
/// credentials rather than those of the current process.
pub fn pgsigio<G: ProcGlue>(sir: &SigioRef, sig: i32, checkctty: bool) {
    if sir.sir_sigio.load(Ordering::Relaxed) == 0 {
        return;
    }

    klock::kernel_lock();
    {
        let chain = SIGIO_LOCK.lock();
        if let Some(sigio) = sigio_of(&chain, sir)
            && let Some(cred) = sigio.sio_ucred.as_ref()
        {
            let r = klock::reader();
            if sigio.sio_pgid > 0 {
                if let Some(pr) = prfind::<G, _>(&r, sigio.sio_pgid)
                    && cansigio(cred, pr)
                {
                    prsignal(pr, sig);
                }
            } else if sigio.sio_pgid < 0
                && let Some(pg) = pgfind::<G, _>(&r, -sigio.sio_pgid)
            {
                for pr in pg.pg_members.iter(&r) {
                    if cansigio(cred, pr)
                        && (!checkctty || pr.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT != 0)
                    {
                        prsignal(pr, sig);
                    }
                }
            }
        }
    }
    klock::kernel_unlock();
}

/// `sigio_free`: unregisters `sir`.
pub fn sigio_free(sir: &SigioRef) {
    if sir.sir_sigio.load(Ordering::Relaxed) == 0 {
        return;
    }

    let rm = {
        let mut chain = SIGIO_LOCK.lock();
        sigio_unlink(&mut chain, sir)
    };
    sigio_del(rm);
}

/// `sigio_freelist(&pgid's list)`: unregisters every registration whose owner is `pgid` (a
/// process's pid, or a group's negated id).
fn sigio_freelist(pgid: Pid) {
    if SIGIO_COUNT.load(Ordering::Relaxed) == 0 {
        return;
    }

    let mut rmlist: Option<MBox<Sigio>> = None;
    {
        let mut chain = SIGIO_LOCK.lock();
        while let Some(mut sigio) = chain_take(&mut chain, |s| s.sio_pgid == pgid) {
            sigio.sio_next = rmlist.take();
            rmlist = Some(sigio);
        }
    }
    sigio_del(rmlist);
}

/// `sigio_freelist(&pr->ps_sigiolst)`: the process `pr` is exiting.
pub fn sigio_freelist_process<G: ProcGlue>(pr: &Process<G>) {
    sigio_freelist(pr.ps_pid);
}

/// `sigio_freelist(&pgrp->pg_sigiolst)`: the group `pg` goes (`pgdelete`).
pub fn sigio_freelist_pgrp<G: ProcGlue>(pg: &Pgrp<G>) {
    sigio_freelist(-pg.pg_id.get());
}

/// `sigio_setown`: registers the process (`pgid` > 0) or process group (< 0, or with
/// `TIOCSPGRP`) to receive `SIGIO`/`SIGURG` through `sir`; 0 unregisters.
pub fn sigio_setown<G: ProcGlue>(sir: &SigioRef, cmd: u64, pgid: Pid) -> Result<(), Errno> {
    let mut pgid = pgid;

    if pgid == 0 {
        sigio_free(sir);
        return Ok(());
    }

    if cmd == TIOCSPGRP {
        if pgid < 0 {
            return Err(Errno::EINVAL);
        }
        pgid = -pgid;
    }

    G::with_curproc(|p| {
        let Some(p) = p else {
            return Err(Errno::ESRCH);
        };
        let Some(sigio) = MBox::new(
            M_SIGIO,
            M_WAITOK,
            Sigio {
                sio_next: None,
                sio_myref: sir.addr(),
                sio_ucred: Some(p.cred_ref()),
                sio_pgid: pgid,
            },
        ) else {
            kpanic(format_args!("sigio_setown: malloc"));
        };

        // The kernel lock, and not sleeping between prfind()/pgfind() and linking of the
        // sigio ensure that the process or process group does not disappear unexpectedly.
        klock::kernel_lock();
        let mut chain = SIGIO_LOCK.lock();

        let checked = {
            let r = klock::reader();
            let mine = session_addr(&r, p.process());
            if pgid > 0 {
                match prfind::<G, _>(&r, pgid) {
                    None => Err(Errno::ESRCH),
                    // Policy - Don't allow a process to FSETOWN a process in another
                    // session.
                    //
                    // Remove this test to allow maximum flexibility or restrict FSETOWN to
                    // the current process or process group for maximum safety.
                    Some(pr) if session_addr(&r, pr) != mine => Err(Errno::EPERM),
                    Some(pr) if pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING != 0 => {
                        Err(Errno::ESRCH)
                    }
                    Some(_) => Ok(()),
                }
            } else {
                match pgfind::<G, _>(&r, -pgid) {
                    None => Err(Errno::ESRCH),
                    // Policy - Don't allow a process to FSETOWN a process in another
                    // session (as above).
                    Some(pg) if pg.session(&r).map_or(0, |s| ptr::from_ref(s).addr()) != mine => {
                        Err(Errno::EPERM)
                    }
                    Some(_) => Ok(()),
                }
            }
        };

        if let Err(error) = checked {
            // fail:
            drop(chain);
            klock::kernel_unlock();
            // The registration and its reference to the credentials, freed.
            drop(sigio);
            return Err(error);
        }

        let at = sigio.addr();
        let rm = sigio_unlink(&mut chain, sir);
        chain_push(&mut chain, sigio);
        sir.sir_sigio.store(at, Ordering::Relaxed);

        drop(chain);
        klock::kernel_unlock();

        sigio_del(rm);

        Ok(())
    })
}

/// `sigio_getown`: the pgid registered through `sir` (negated for `TIOCGPGRP`), 0 for none.
pub fn sigio_getown(sir: &SigioRef, cmd: u64) -> Pid {
    let pgid = {
        let chain = SIGIO_LOCK.lock();
        sigio_of(&chain, sir).map_or(0, |s| s.sio_pgid)
    };

    if cmd == TIOCGPGRP { -pgid } else { pgid }
}

/// `sigio_copy`: registers the owner of `src` in `dst` too.
pub fn sigio_copy(dst: &SigioRef, src: &SigioRef) {
    sigio_free(dst);

    if src.sir_sigio.load(Ordering::Relaxed) == 0 {
        return;
    }

    let Some(mut newsigio) = MBox::new(
        M_SIGIO,
        M_WAITOK,
        Sigio {
            sio_next: None,
            sio_myref: dst.addr(),
            sio_ucred: None,
            sio_pgid: 0,
        },
    ) else {
        kpanic(format_args!("sigio_copy: malloc"));
    };

    let rm = {
        let mut chain = SIGIO_LOCK.lock();

        let Some(sigio) = sigio_of(&chain, src) else {
            drop(chain);
            drop(newsigio);
            return;
        };

        newsigio.sio_pgid = sigio.sio_pgid;
        // The source holds a reference to its credentials: one more for the copy.
        newsigio.sio_ucred = sigio.sio_ucred.clone();

        let at = newsigio.addr();
        let rm = sigio_unlink(&mut chain, dst);
        chain_push(&mut chain, newsigio);
        dst.sir_sigio.store(at, Ordering::Relaxed);
        rm
    };

    sigio_del(rm);
}

/// The address of `pr`'s session (`pr->ps_session`), 0 without one: what the C compares.
fn session_addr<G: ProcGlue, W: hal::rwlock::Witness + ?Sized>(w: &W, pr: &Process<G>) -> usize {
    pr.session(w).map_or(0, |s| ptr::from_ref(s).addr())
}

/// The registrations forgotten (not freed: an earlier test's memory), for the host tests,
/// which share the chain (`testing.md`).
#[cfg(any(test, feature = "doubles"))]
pub fn sigio_test_reset() {
    let mut chain = SIGIO_LOCK.lock();
    if let Some(old) = chain.take() {
        core::mem::forget(old);
    }
    SIGIO_COUNT.store(0, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;

    use hal::pin::pin_static;
    use hal::pool::PoolBox;
    use hal::sys::signal::{SIGIO, SIGURG, sigmask};

    use super::*;
    use crate::cred::Ucred;
    use crate::proc::{
        ProcessRef, Thread, enternewpgrp, pgrp_alloc, process_alloc, procinit, session_alloc,
        thread_alloc,
    };
    use crate::sig::{siginit, test_sigacts};
    use crate::testutil;

    crate::test_file_glue!(Of, Og);
    crate::test_proc_glue!(Og, Of);

    /// A process `pid` with its actions and one thread, on the process lists, for the run.
    fn process(pid: Pid) -> &'static ProcessRef<Og> {
        let pr = process_alloc::<Og>(pid, ());
        drop(pr.set_ucred(CredRef::new(Ucred::new())));
        {
            let t = Og::tables();
            let mut w = klock::witness();
            t.allprocess.insert_head(&mut w, pr.as_pin());
            t.pidhash(pid).insert_head(&mut w, pr.as_pin());
        }
        test_sigacts(&pr);
        siginit(&pr);
        let t = thread_alloc::<Og>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        let t: &'static PoolBox<Thread<Og>> = Box::leak(Box::new(t));
        {
            let mut g = pr.ps_mtx.lock();
            pr.ps_threads
                .insert_tail(&mut g, pin_static::<Thread<Og>>(t));
        }
        pr.ps_threadcnt.set(1);
        Og::set_curproc(Some(t));
        Box::leak(Box::new(pr))
    }

    fn setup() {
        testutil::install();
        klock::kernel_lock();
        procinit::<Og>(64, 64);
    }

    #[test]
    fn a_registration_is_one_word_and_empty_at_zero() {
        assert_eq!(size_of::<SigioRef>(), size_of::<usize>());
        let sir = SigioRef::default();
        assert_eq!(sigio_getown(&sir, 0), 0);
        sigio_init(&sir);
        sigio_free(&sir);
    }

    #[test]
    fn a_process_owner_gets_sigio_until_unregistered() {
        setup();
        let pr = process(201);
        let sir = SigioRef::new();
        assert_eq!(sigio_setown::<Og>(&sir, 0, 201), Ok(()));
        assert_eq!(sigio_getown(&sir, 0), 201);
        assert_eq!(sigio_getown(&sir, TIOCGPGRP), -201);

        pgsigio::<Og>(&sir, SIGIO, false);
        // SIGIO is ignored by default: siginit's process forgets it; SIGURG likewise. Catch
        // SIGIO to see it pending.
        assert_eq!(pr.ps_siglist.load(Ordering::Relaxed), 0);
        let mut sa = hal::sys::signal::Sigaction {
            sa_handler: 0x4000,
            ..Default::default()
        };
        Og::with_curproc(|p| crate::sig::setsigvec(p.unwrap(), SIGIO, &mut sa));
        pgsigio::<Og>(&sir, SIGIO, false);
        assert_eq!(pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGIO));

        // 0 unregisters; nothing more is sent.
        assert_eq!(sigio_setown::<Og>(&sir, 0, 0), Ok(()));
        assert_eq!(sigio_getown(&sir, 0), 0);
        pr.ps_siglist.store(0, Ordering::Relaxed);
        pgsigio::<Og>(&sir, SIGIO, false);
        assert_eq!(pr.ps_siglist.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn setown_checks_its_owner() {
        setup();
        let _pr = process(202);
        let sir = SigioRef::new();
        assert_eq!(sigio_setown::<Og>(&sir, 0, 99_997), Err(Errno::ESRCH));
        assert_eq!(sigio_setown::<Og>(&sir, 0, -99_997), Err(Errno::ESRCH));
        assert_eq!(sigio_setown::<Og>(&sir, TIOCSPGRP, -5), Err(Errno::EINVAL));
        assert_eq!(sigio_getown(&sir, 0), 0);

        // A process in another session (the caller is in none) is refused.
        let other = process(203);
        enternewpgrp(&other, pgrp_alloc::<Og>(()), Some(session_alloc::<Og>(())));
        let _me = process(204);
        assert_eq!(sigio_setown::<Og>(&sir, 0, 203), Err(Errno::EPERM));
        // An exiting process is refused.
        let gone = process(205);
        gone.ps_flags.fetch_or(PS_EXITING, Ordering::Relaxed);
        assert_eq!(sigio_setown::<Og>(&sir, 0, 205), Err(Errno::ESRCH));
    }

    #[test]
    fn the_owner_s_exit_takes_its_registrations() {
        setup();
        let pr = process(206);
        let a = SigioRef::new();
        let b = SigioRef::new();
        assert_eq!(sigio_setown::<Og>(&a, 0, 206), Ok(()));
        sigio_copy(&b, &a);
        assert_eq!(sigio_getown(&b, 0), 206);
        // A new registration of `a` replaces its old one.
        assert_eq!(sigio_setown::<Og>(&a, 0, 206), Ok(()));
        assert_eq!(sigio_getown(&a, 0), 206);

        sigio_freelist_process(&**pr);
        assert_eq!(sigio_getown(&a, 0), 0);
        assert_eq!(sigio_getown(&b, 0), 0);
        // The objects' stale numbers find nothing; freeing them is harmless.
        sigio_free(&a);
        sigio_free(&b);
        // A copy of nothing is nothing.
        sigio_copy(&b, &a);
        assert_eq!(sigio_getown(&b, 0), 0);
    }

    #[test]
    fn a_group_owner_signals_its_members() {
        setup();
        let pr = process(207);
        enternewpgrp(&pr, pgrp_alloc::<Og>(()), Some(session_alloc::<Og>(())));
        let mut sa = hal::sys::signal::Sigaction {
            sa_handler: 0x4000,
            ..Default::default()
        };
        Og::with_curproc(|p| crate::sig::setsigvec(p.unwrap(), SIGURG, &mut sa));
        let sir = SigioRef::new();
        // TIOCSPGRP names the group by a positive id.
        assert_eq!(sigio_setown::<Og>(&sir, TIOCSPGRP, 207), Ok(()));
        assert_eq!(sigio_getown(&sir, 0), -207);
        assert_eq!(sigio_getown(&sir, TIOCGPGRP), 207);
        pgsigio::<Og>(&sir, SIGURG, false);
        assert_eq!(pr.ps_siglist.load(Ordering::Relaxed), sigmask(SIGURG));
        // checkctty: only members with a controlling terminal.
        pr.ps_siglist.store(0, Ordering::Relaxed);
        pgsigio::<Og>(&sir, SIGURG, true);
        assert_eq!(pr.ps_siglist.load(Ordering::Relaxed), 0);
        // The group goes: its registrations with it.
        let pg = {
            let r = klock::reader();
            pr.pgrp(&r).unwrap().pg_id.get()
        };
        {
            let r = klock::reader();
            sigio_freelist_pgrp(pgfind::<Og, _>(&r, pg).unwrap());
        }
        assert_eq!(sigio_getown(&sir, 0), 0);
    }
}
/* </TESTS> */
