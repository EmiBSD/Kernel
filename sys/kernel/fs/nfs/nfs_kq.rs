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
 * Copyright (c) 2002 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Jaromir Dolecek.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! kqueue(2) on NFS files: the filters (`nfs_kqfilter`, `filt_nfsread`, `filt_nfswrite`,
//! `filt_nfsvnode`, `filt_nfsdetach`) and the poller thread `nfs_kqpoll`, which notices
//! changes made on the server to the watched files (`nfs_kqwatch`, `nfs_kqunwatch`).
//!
//! Upstream: sys/nfs/nfs_kq.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_kq.rs@e8249449d905
//!
//! The poller periodically checks for server changes of any of the watched files every
//! `NFS_MINATTRTIMO/2` seconds. Only changes in size, modification time, change time and
//! nlinks are checked, everything else is ignored. It calls `VOP_GETATTR()` only when it is
//! likely to get new data, i.e. when the vnode expires from the attribute cache: the same
//! result as running stat(2) periodically from userland, with little CPU and network use,
//! and still proper kevent semantics. The thread is created when the first vnode is added
//! to the watch list and exits when the list is empty.
//!
//! ## Deviations
//! - `struct kevq` entries ([`Kevq`]) are items of a pool of their own ("nfskevqpl", made by
//!   `nfs_vfs_init`) where the C `malloc`s them (`M_KEVENT`): the watch list owns them
//!   (`hal::owned::OwnedList`), and an owning list takes pool items (`kqueue`'s timers'
//!   `kqtimerpl` precedent). `vmstat -m` shows the pool, and `M_KEVENT`'s row no longer counts
//!   a watched NFS vnode.
//! - `pnfskq` is the poller's thread id; the channel it sleeps on (and `nfs_kqwatch` wakes)
//!   is the watch list's cell's address, where the C's is the poller's `struct proc`.
//!   `nfs_kqpoll` calls `VOP_GETATTR` as itself (the running thread), which the C reads
//!   from `pnfskq`.
//! - The `DEBUG` `printf`s are not configured.
//!
//! ## Redesign
//! - LZ's `KEVLIST` was a static `SlistHead` of `malloc`ed `Kevq`s with `Cell` members behind
//!   an `unsafe impl Sync`, linked and unlinked through `unsafe`, `pnfskq` an `AtomicPtr`, and
//!   `kn_hook` a raw word cast back to the vnode. Here the list and `pnfskq` are one value
//!   in a `Rwlocked` (`nfskevq_lock`, the C's rwlock: `NfsTables::kevq`) that owns its
//!   entries; the poller keeps the entry it works on with the lock dropped by a counted
//!   reference (`hal::owned::SecondaryRef`, `kernel::event`'s `KnHold` shape: the C's
//!   `KEVQ_BUSY` stays the flag `nfs_kqunwatch` waits on, and the list could not give the
//!   entry back while the reference lives), and walks on from it with `next(e)`, O(1) a
//!   step as the C's `SLIST_FOREACH`; the reference is taken only for an entry whose
//!   attributes expired, where the C makes its `VOP_GETATTR` call. `nfs_kqunwatch` keeps the
//!   entry it waits for the same way across its sleep (`rwsleep`, the C's exit, `tsleep` and
//!   enter, in one step) and frees it through the reference (`OwnedList::take`).
//! - The filters are `kernel::event::Filter` impls ([`NfsRead`], [`NfsWrite`], [`NfsVnode`])
//!   whose tables are made at compile time; `kn_hook` is the knote's typed vnode hook
//!   (`Knote::set_hook_vnode`), ufs's form. `filt_nfsread` reaches the node only past its
//!   `NOTE_REVOKE` test (the vnode has none any more then; the C loads the NULL `v_data` and
//!   does not follow it).
//! - Cost against the C: per watched vnode and poll, `vtonfs` and `vfstonfs` (a count pair
//!   each, the C's casts) for `nfs_getattrcache`, and the reference's count pair for an
//!   expired one; `nfs_kqwatch`'s and `nfs_kqunwatch`'s searches are the C's O(n) walks.

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

use hal::owned::{OwnedAdapter, OwnedEntry, OwnedList, SecondaryRef};
use hal::pool::PR_WAITOK;
use hal::rwlock::Rwlocked;
use hal::sleep::{INFSLP, tsleep_nsec, wakeup};
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::types::{Nlink, Pid};

use super::nfs_subs::nfs_getattrcache;
use super::nfsmount::vfstonfs;
use super::nfsnode::{nfs_invalidate_attrcache, vtonfs};
use crate::assert::kpanic;
use crate::clock::{Timespec, sec_to_nsec};
use crate::event::{
    __EV_POLL, __EV_SELECT, EV_EOF, EV_ONESHOT, EVFILT_READ, EVFILT_VNODE, EVFILT_WRITE,
    FILTEROP_ISFD, Filter, Filterops, Knote, NOTE_ATTRIB, NOTE_DELETE, NOTE_EOF, NOTE_EXTEND,
    NOTE_LINK, NOTE_REVOKE, NOTE_TRUNCATE, NOTE_WRITE,
};
use crate::file::foffset;
use crate::fs::subr::vfs;
use crate::fs::vnode::{VN_KNOTE, Vattr, Vnode};
use crate::fs::vops::{Cred, vop_getattr};
use crate::net::sockbuf::PSOCK;
use crate::proc::{ProcGlue, Thread};
use crate::sys::nfs_nfs::NFS_MINATTRTIMO;

/// `KEVQ_BUSY`: currently being processed.
const KEVQ_BUSY: u32 = 0x01;
/// `KEVQ_WANT`: want to change this entry.
const KEVQ_WANT: u32 = 0x02;

/// What the poller last saw of a watched vnode (`omtime`, `octime`, `onlink`).
#[derive(Clone, Copy)]
struct KevqSeen {
    /// `omtime`: old modification time.
    omtime: Timespec,
    /// `octime`: old change time.
    octime: Timespec,
    /// `onlink`: old number of references to file.
    onlink: Nlink,
}

/// `struct kevq`: a watched vnode and what the poller last saw of it, an item of
/// "nfskevqpl" the watch list owns.
pub struct Kevq<G: ProcGlue> {
    /// `kev_link`.
    kev_link: OwnedEntry<Kevq<G>>,
    /// `vp`.
    vp: &'static Vnode<G>,
    /// `usecount`: the knotes watching `vp` (changed under `nfskevq_lock`).
    usecount: AtomicU32,
    /// `flags`: `KEVQ_*` (changed under `nfskevq_lock`).
    flags: AtomicU32,
    /// The times and the link count, written by the poller while the entry is `KEVQ_BUSY`
    /// (the lock dropped, as the C writes them).
    seen: ReadMostly<KevqSeen>,
}

/// `SLIST_HEAD(kevqlist, kevq)`: the watch list, through `kev_link`, owning its entries.
pub struct ByKevq<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> OwnedAdapter for ByKevq<G> {
    type Elem = Kevq<G>;
    fn entry(e: &Kevq<G>) -> &OwnedEntry<Kevq<G>> {
        &e.kev_link
    }
    /// A list has no order of its own.
    fn cmp(_a: &Kevq<G>, _b: &Kevq<G>) -> core::cmp::Ordering {
        core::cmp::Ordering::Equal
    }
}

/// `kevlist` and `pnfskq`, under `nfskevq_lock` (`NfsTables::kevq`).
pub struct NfsKevq<G: ProcGlue> {
    /// `kevlist`: the watched vnodes.
    kevlist: OwnedList<ByKevq<G>>,
    /// `pnfskq`: the poller's thread, `None` when none runs.
    pnfskq: Option<Pid>,
}

impl<G: ProcGlue> NfsKevq<G> {
    /// `SLIST_HEAD_INITIALIZER(kevlist)`, no poller (`nfskevq_lock`'s cell's value).
    pub const fn new() -> Self {
        Self {
            kevlist: OwnedList::new(),
            pnfskq: None,
        }
    }
}

impl<G: ProcGlue> Default for NfsKevq<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `filt_nfsread`'s filter: the bytes past the file offset; always ready for poll and
/// select.
pub struct NfsRead;

/// `filt_nfswrite`'s filter: an NFS file is always writable.
pub struct NfsWrite;

/// `filt_nfsvnode`'s filter: records the vnode events (`NOTE_*`) the user asked for.
pub struct NfsVnode;

/// The tables of the three filters (`kn_fop`'s targets), made at compile time.
struct NfsFiltops<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> NfsFiltops<G> {
    /// `nfsread_filtops`.
    const READ: Filterops<G> = Filterops::of::<NfsRead>();
    /// `nfswrite_filtops`.
    const WRITE: Filterops<G> = Filterops::of::<NfsWrite>();
    /// `nfsvnode_filtops`.
    const VNODE: Filterops<G> = Filterops::of::<NfsVnode>();
}

/// `nfskevq_lock`'s cell, the glue's.
fn kevq<G: ProcGlue>() -> &'static Rwlocked<NfsKevq<G>> {
    &vfs::<G>().nfs.kevq
}

/// The channel the poller sleeps on between its passes and `nfs_kqwatch` wakes (the C's
/// `pnfskq`): the watch list's cell's address.
fn pnfskq_chan<G: ProcGlue>() -> *const () {
    ptr::from_ref(kevq::<G>()).cast()
}

/// `nfs_kqpoll(arg)`: the poller thread's body. It checks the watched files whose cached
/// attributes expired, posts the events their changes imply, and exits when nothing is
/// watched any more.
pub fn nfs_kqpoll<G: ProcGlue>() -> ! {
    hal::thread::with_current::<Thread<G>, _>(|p| {
        while nfs_kqpoll_pass(&p) {
            // wait a while before checking for changes again
            let _ = tsleep_nsec(
                pnfskq_chan::<G>(),
                PSOCK,
                "nfskqpw",
                sec_to_nsec(NFS_MINATTRTIMO as u64) / 2,
            );
        }
    });
    crate::thread::kthread_exit::<G>(0)
}

/// One pass of [`nfs_kqpoll`]'s loop over the watch list, as the thread `p`: `true` while
/// something is watched; `false` (and `pnfskq` cleared, under the lock, as the C's exit)
/// once the list is empty.
pub(crate) fn nfs_kqpoll_pass<G: ProcGlue>(p: &Thread<G>) -> bool {
    let mut g = kevq::<G>().write();
    let mut cur = g.kevlist.first();
    while let Some(ke) = cur {
        let vp = ke.vp;
        let np = vtonfs(vp);
        let mut attr = Vattr::new();

        // skip if still in attrcache
        let cached = match vp.mount() {
            Some(mp) => nfs_getattrcache(&np, &vfstonfs(mp), &mut attr) != Err(Errno::ENOENT),
            None => kpanic(format_args!("nfs_kqpoll: vnode {vp:p} has no mount")),
        };
        if cached {
            cur = g.kevlist.next(ke);
            continue;
        }

        // Mark entry busy, release lock and check for changes.
        ke.flags.fetch_or(KEVQ_BUSY, Ordering::Relaxed);
        let mut hold = SecondaryRef::<ByKevq<G>>::new();
        hold.set(Some(ke));
        g.unlocked(|| nfs_kqpoll_check(p, vp, &np, &hold));

        // next:
        let Some(ke) = hold.get() else {
            kpanic(format_args!("nfs_kqpoll: a held entry is gone"));
        };
        ke.flags.fetch_and(!KEVQ_BUSY, Ordering::Relaxed);
        if ke.flags.load(Ordering::Relaxed) & KEVQ_WANT != 0 {
            ke.flags.fetch_and(!KEVQ_WANT, Ordering::Relaxed);
            wakeup(ptr::from_ref(ke));
        }
        cur = g.kevlist.next(ke);
        drop(hold);
    }

    if g.kevlist.is_empty() {
        // Nothing more to watch, exit
        g.pnfskq = None;
        return false;
    }
    true
}

/// The unlocked part of a pass over a watched vnode whose attributes expired: its
/// attributes fetched again and each change posted.
fn nfs_kqpoll_check<G: ProcGlue>(
    p: &Thread<G>,
    vp: &'static Vnode<G>,
    np: &super::nfsnode::NfsNode<G>,
    hold: &SecondaryRef<ByKevq<G>>,
) {
    let Some(ke) = hold.get() else {
        kpanic(format_args!("nfs_kqpoll: a held entry is gone"));
    };
    let mut attr = Vattr::new();
    // save v_size, nfs_getattr() updates it
    let osize = np.n_size.load(Ordering::Relaxed);

    if vop_getattr(vp, &mut attr, Cred::real(p.ucred()), p) == Err(Errno::ESTALE) {
        nfs_invalidate_attrcache(np);
        VN_KNOTE(vp, NOTE_DELETE);
        return;
    }

    let mut seen = *ke.seen.read();
    // following is a bit fragile, but about best we can get
    if attr.va_size != osize {
        let mut flags = NOTE_WRITE;

        if attr.va_size > osize {
            flags |= NOTE_EXTEND;
        } else {
            flags |= NOTE_TRUNCATE;
        }

        VN_KNOTE(vp, flags);
        seen.omtime = attr.va_mtime;
    } else if attr.va_mtime != seen.omtime {
        VN_KNOTE(vp, NOTE_WRITE);
        seen.omtime = attr.va_mtime;
    }

    if attr.va_ctime != seen.octime {
        VN_KNOTE(vp, NOTE_ATTRIB);
        seen.octime = attr.va_ctime;
    }

    if attr.va_nlink != seen.onlink {
        VN_KNOTE(vp, NOTE_LINK);
        seen.onlink = attr.va_nlink;
    }
    let _ = ke.seen.replace(seen);
}

/// `kn->kn_hook` of an NFS knote: its vnode.
fn kn_vnode<G: ProcGlue>(kn: &Knote<G>) -> &'static Vnode<G> {
    match kn.hook_vnode() {
        Some(vp) => vp,
        None => kpanic(format_args!("knote {kn:p}: no vnode")),
    }
}

/// `filt_nfsdetach`: unhook the knote from the vnode and stop watching the vnode for it.
fn filt_nfsdetach<G: ProcGlue>(kn: &Knote<G>) {
    let vp = kn_vnode(kn);

    vp.klist.remove_legacy(kn);

    // Remove the vnode from watch list
    if !kn.has_flags(__EV_POLL | __EV_SELECT) {
        nfs_kqunwatch(vp);
    }
}

/// `nfs_kqunwatch(vp)`: drop one watcher of `vp`; the last one takes it off the list.
pub fn nfs_kqunwatch<G: ProcGlue>(vp: &Vnode<G>) {
    let mut g = kevq::<G>().write();
    let mut hold = SecondaryRef::<ByKevq<G>>::new();
    hold.set(g.kevlist.iter().find(|ke| ptr::eq(ke.vp, vp)));
    loop {
        let Some(ke) = hold.get() else {
            return;
        };
        if ke.flags.load(Ordering::Relaxed) & KEVQ_BUSY == 0 {
            break;
        }
        ke.flags.fetch_or(KEVQ_WANT, Ordering::Relaxed);
        let chan = ptr::from_ref(ke);
        let _ = g.sleep_nsec(chan, PSOCK, "nfskqdet", INFSLP);
    }

    let Some(ke) = hold.get() else {
        return;
    };
    let users = ke.usecount.load(Ordering::Relaxed);
    if users > 1 {
        // keep, other kevents need this
        ke.usecount.store(users - 1, Ordering::Relaxed);
    } else {
        // last user, g/c
        drop(g.kevlist.take(hold));
    }
}

impl<G: ProcGlue> Filter<G> for NfsRead {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_nfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_nfsdetach(kn);
    }

    /// `filt_nfsread`.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        let vp = kn_vnode(kn);

        // filesystem is gone, so set the EOF flag and schedule the knote for deletion.
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }
        let np = vtonfs(vp);

        // `np->n_size - foffset(kn->kn_fp)`: the C subtracts in `u_quad_t` and keeps the
        // `int64_t`.
        let size = np.n_size.load(Ordering::Relaxed) as i64;
        kn.kn_data().set(size.wrapping_sub(foffset(&kn.fp())));
        if kn.kn_data().get() == 0 && kn.kn_sfflags.get() & NOTE_EOF != 0 {
            kn.kn_fflags().set(kn.kn_fflags().get() | NOTE_EOF);
            return true;
        }

        if kn.has_flags(__EV_POLL | __EV_SELECT) {
            return true;
        }

        kn.kn_data().get() != 0
    }
}

impl<G: ProcGlue> Filter<G> for NfsWrite {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_nfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_nfsdetach(kn);
    }

    /// `filt_nfswrite`.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        // filesystem is gone, so set the EOF flag and schedule the knote for deletion.
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }

        kn.kn_data().set(0);
        true
    }
}

impl<G: ProcGlue> Filter<G> for NfsVnode {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_nfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_nfsdetach(kn);
    }

    /// `filt_nfsvnode`.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        // `kn_sfflags & hint` of the C's `long` hint: its low 32 bits.
        let hint32 = hint as u32;
        if kn.kn_sfflags.get() & hint32 != 0 {
            kn.kn_fflags().set(kn.kn_fflags().get() | hint32);
        }
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF);
            return true;
        }
        kn.kn_fflags().get() != 0
    }
}

/// `nfs_kqfilter` (`vop_kqfilter`): attach a knote to an NFS vnode and, unless it is for
/// poll(2) or select(2), put the vnode on the poller's watch list.
pub fn nfs_kqfilter<G: ProcGlue>(vp: &'static Vnode<G>, kn: &Knote<G>) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&NfsFiltops::<G>::READ)),
        EVFILT_WRITE => kn.kn_fop.set(Some(&NfsFiltops::<G>::WRITE)),
        EVFILT_VNODE => kn.kn_fop.set(Some(&NfsFiltops::<G>::VNODE)),
        _ => return Err(Errno::EINVAL),
    }

    // kn->kn_hook = vp: set once, as ufs's (a knote reaches VOP_KQFILTER once).
    let _ = kn.set_hook_vnode(vp);

    // Put the vnode to watched list.
    if !kn.has_flags(__EV_POLL | __EV_SELECT) {
        nfs_kqwatch(vp)?;
    }

    vp.klist.insert_legacy(kn);

    Ok(())
}

/// `nfs_kqwatch(vp)`: watch `vp`, starting the poller thread if it is not running.
pub fn nfs_kqwatch<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    // Fetch current attributes. It's only needed when the vnode is not watched yet, but we
    // need to do this without lock held. This is likely cheap due to attrcache, so do it now.
    let mut attr = Vattr::new();
    G::with_curproc(|p| match p {
        Some(p) => {
            let _ = vop_getattr(vp, &mut attr, Cred::real(p.ucred()), p);
        }
        // XXX
        None => kpanic(format_args!("nfs_kqwatch: no thread")),
    });

    let mut g = kevq::<G>().write();

    // ensure the poller is running
    if g.pnfskq.is_none() {
        let tid = crate::thread::kthread_create::<G>(b"nfskqpoll", crate::fork::Start::NfsKqPoll)?;
        g.pnfskq = Some(tid);
    }

    if let Some(ke) = g.kevlist.iter().find(|ke| ptr::eq(ke.vp, vp)) {
        // already watched, so just bump usecount
        ke.usecount.fetch_add(1, Ordering::Relaxed);
    } else {
        // need a new one
        let seen = KevqSeen {
            omtime: attr.va_mtime,
            octime: attr.va_ctime,
            onlink: attr.va_nlink,
        };
        let Some(ke) = vfs::<G>().nfs.kevqpl.get(PR_WAITOK, || Kevq {
            kev_link: OwnedEntry::new(),
            vp,
            usecount: AtomicU32::new(1),
            flags: AtomicU32::new(0),
            seen: ReadMostly::new(seen),
        }) else {
            kpanic(format_args!("nfs_kqwatch: pool_get"));
        };
        // SLIST_INSERT_HEAD(&kevlist, ke, kev_link): before the first entry, which a
        // counted reference names while the list is changed.
        let mut at = SecondaryRef::<ByKevq<G>>::new();
        at.set(g.kevlist.first());
        match at.get() {
            Some(first) => g.kevlist.insert_before(first, ke),
            None => g.kevlist.push(ke),
        }
    }

    // kick the poller
    wakeup(pnfskq_chan::<G>());
    Ok(())
}

/// `nfs_test_reset`'s part: forgets the watch list of a test glue's tables, and its poller,
/// without freeing the entries (the memory behind them is gone with the last test's).
#[cfg(any(test, feature = "doubles"))]
pub(crate) fn nfs_kq_test_reset<G: ProcGlue>() {
    let mut g = kevq::<G>().write();
    core::mem::forget(core::mem::replace(&mut g.kevlist, OwnedList::new()));
    g.pnfskq = None;
}
/* </CODE> */
