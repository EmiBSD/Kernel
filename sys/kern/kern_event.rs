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
 * Copyright (c) 1999,2000,2001 Jonathan Lemon <jlemon@FreeBSD.org>
 * All rights reserved.
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * $FreeBSD: src/sys/kern/kern_event.c,v 1.22 2001/02/23 20:32:42 jlemon Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! `bsd`'s `kern_event.c` over the kernel's (`kernel::event`): the names `bsd`'s event sources
//! call (`klist_insert_locked`, `knote_locked`, `seltrue_kqfilter`, ...), the `struct
//! fileops` of a kqueue file (`kqueueops`, the `bsd` object a kqueue descriptor is), the
//! `sysent[]` rows of `kqueue(2)`, `kqueue1(2)` and `kevent(2)`, and the host tests' helpers
//! for the modules whose objects deliver events.
//!
//! Upstream: sys/kern/kern_event.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_event.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.4: every decision of `kern_event.c` is `kernel::event`'s (`sys/kernel/event.rs`
//!   says what changed shape). This module keeps LZ's path, which the filters of 18 files and
//!   their callers import (their `use` lines unchanged), with 0 `unsafe` (LZ: 70).
//! - `klist_insert_locked`, `klist_remove_locked` and `knote_locked` are the klist's legacy
//!   forms (`Klist::insert_legacy`, ...): the caller holds the list's lock as the C knows it
//!   (the kernel lock, or the legacy lock inside which an `Own` klist takes its mutex).
//!   `klist_init_mutex` and `klist_init_rwlock` give the klist its own mutex
//!   (`Klist::init_own`): the lock argument is no longer named, and the calls are safe.
//! - `kqueueops` is a `struct fileops` of this glue's, each operation the kernel's
//!   (`kqueue_kqfilter`, `kqueue_close`) or the C's constant answer; `kqueue_stat` fills
//!   `bsd`'s `struct stat` from the kernel's count.

use hal::user::UserPtr;
use kernel::api::{EventEntries, SysResult};
use kernel::event::Kqueue as KKqueue;

pub use kernel::event::{
    knote_processexec, knote_processexit, knote_processfork, kqpoll_done, kqpoll_exit, kqpoll_init,
    kqueue_register, kqueue_scan, kqueue_scan_setup,
};

use crate::kern::kern_descrip::{BsdFileData, native};
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::kern::sys_generic::{native_poll, native_ppoll, native_pselect, native_select};
use crate::sys::errno::Errno;
use crate::sys::event::{Filterops, Klist, Knote};
use crate::sys::eventvar::KqRef;
use crate::sys::file::{File, Fileops};
use crate::sys::mutex::Mutex;
use crate::sys::proc::Proc;
use crate::sys::rwlock::Rwlock;
use crate::sys::stat::{S_IFIFO, Stat};
use crate::sys::syscall::{SYS_kevent, SYS_kqueue, SYS_kqueue1};
use crate::sys::syscallargs::{SysKeventArgs, SysKqueue1Args};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::{Dev, Register};
use crate::sys::uio::Uio;

/// `kqueueops`: the file operations of a `kqueue(2)` descriptor.
pub static KQUEUEOPS: Fileops = Fileops {
    fo_read: kqueue_read,
    fo_write: kqueue_write,
    fo_ioctl: kqueue_ioctl,
    fo_kqfilter: kqueue_kqfilter,
    fo_stat: kqueue_stat,
    fo_close: kqueue_close,
    fo_seek: None,
};

/// `dead_filtops`: the filter of a knote whose object is gone (deadfs's `kqfilter`).
pub static DEAD_FILTOPS: Filterops = KKqueue::<BsdProcGlue>::DEAD_FILTOPS;

/// `kqueue_init`: the knote, kqueue and timer pools.
pub fn kqueue_init() {
    if kernel::event::kqueue_init::<BsdProcGlue>().is_err() {
        panic(format_args!("kqueue_init: cannot make the pools"));
    }
}

/// `kqueue_init_percpu`: the knote pool's per-CPU caches.
pub fn kqueue_init_percpu() {
    kernel::event::kqueue_init_percpu::<BsdProcGlue>();
}

/// The kqueue a kqueue file holds (`fp->f_data`), counted.
pub fn fp_kqueue(fp: &File) -> KqRef {
    match fp.data() {
        Some(BsdFileData::Kqueue(kq)) => kq.clone(),
        _ => panic(format_args!("file {:#x}: not a kqueue", fp.addr())),
    }
}

/// `kqueue_read`.
pub fn kqueue_read(_fp: &File, _uio: &mut Uio<'_>, _fflags: i32) -> Result<(), Errno> {
    kernel::event::kqueue_rw()
}

/// `kqueue_write`.
pub fn kqueue_write(_fp: &File, _uio: &mut Uio<'_>, _fflags: i32) -> Result<(), Errno> {
    kernel::event::kqueue_rw()
}

/// `kqueue_ioctl`.
pub fn kqueue_ioctl(_fp: &File, _com: u64, _data: &mut [u8], _p: &Proc) -> Result<(), Errno> {
    kernel::event::kqueue_ioctl()
}

/// `kqueue_kqfilter`: a kqueue is readable when it has pending events.
pub fn kqueue_kqfilter(fp: &File, kn: &Knote) -> Result<(), Errno> {
    kernel::event::kqueue_kqfilter::<BsdProcGlue>(fp, kn)
}

/// `kqueue_stat`: the pending count as the size, a FIFO.
pub fn kqueue_stat(fp: &File, st: &mut Stat, _p: &Proc) -> Result<(), Errno> {
    let (size, blksize) = kernel::event::kqueue_stat::<BsdProcGlue>(fp);

    *st = Stat::default();
    st.st_size = size; // unlocked read
    st.st_blksize = blksize as _;
    st.st_mode = S_IFIFO;
    Ok(())
}

/// `kqueue_close`: the descriptor's last close.
pub fn kqueue_close(fp: &File, p: Option<&Proc>) -> Result<(), Errno> {
    kernel::event::kqueue_close::<BsdProcGlue>(fp, p)
}

/// `seltrue_kqfilter(dev, kn)`: the kqfilter of a device that is always ready.
pub fn seltrue_kqfilter(_dev: Dev, kn: &Knote) -> Result<(), Errno> {
    kernel::event::seltrue_kqfilter(kn)
}

/// `knote(list, hint)`: takes the list's lock, which for a kernel-lock list is the kernel
/// lock. An `Own` list (`klist_init_mutex`/`klist_init_rwlock`) stands for its owner's lock,
/// which the C's `knote` takes and its filters read under: its caller takes that lock and
/// calls [`knote_locked`] instead; `knote` on one stops the kernel.
pub fn knote(list: &Klist, hint: i64) {
    if list.kind() == kernel::event::KlistKind::Own {
        panic(format_args!(
            "knote: klist {:p} is its owner's lock's: take it and call knote_locked",
            list
        ));
    }
    list.knote(hint);
}

/// `knote_locked(list, hint)`: the caller holds the list's lock.
pub fn knote_locked(list: &Klist, hint: i64) {
    list.knote_legacy(hint);
}

/// `klist_free(klist)`: the list must be empty.
pub fn klist_free(klist: &Klist) {
    klist.free();
}

/// `klist_insert(klist, kn)`: hooks a knote on the list, taking the list's lock.
pub fn klist_insert(klist: &Klist, kn: &Knote) {
    klist.insert(kn);
}

/// `klist_insert_locked(klist, kn)`: the caller holds the list's lock.
pub fn klist_insert_locked(klist: &Klist, kn: &Knote) {
    klist.insert_legacy(kn);
}

/// `klist_remove(klist, kn)`: unhooks a knote, taking the list's lock.
pub fn klist_remove(klist: &Klist, kn: &Knote) {
    klist.remove(kn);
}

/// `klist_remove_locked(klist, kn)`: the caller holds the list's lock.
pub fn klist_remove_locked(klist: &Klist, kn: &Knote) {
    klist.remove_legacy(kn);
}

/// `klist_invalidate(list)`: detach all knotes from klist; the descriptor knotes are
/// rewired to indicate EOF.
pub fn klist_invalidate(list: &Klist) {
    list.invalidate();
}

/// `klist_init_mutex(klist, mtx)`: a klist of an object `mtx` guards; the klist takes its own
/// mutex inside it (`Klist::init_own`).
pub fn klist_init_mutex(klist: &Klist, _mtx: &Mutex) {
    klist.init_own();
}

/// `klist_init_rwlock(klist, rwl)`: as [`klist_init_mutex`], for an object `rwl` guards.
pub fn klist_init_rwlock(klist: &Klist, _rwl: &Rwlock) {
    klist.init_own();
}

/// `kqueue(2)`.
pub fn sys_kqueue(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::event::sys_kqueue(p)? as Register;
    Ok(())
}

/// `kqueue1(2)`: `kqueue(2)` with `O_CLOEXEC` and `O_NONBLOCK`.
pub fn sys_kqueue1(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysKqueue1Args = sysargs(v);
    retval[0] = kernel::event::sys_kqueue1(p, uap.flags.get())? as Register;
    Ok(())
}

/// `kevent(2)`: registers the changes, then collects the pending events.
pub fn sys_kevent(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysKeventArgs = sysargs(v);
    retval[0] = kernel::event::sys_kevent(
        p,
        uap.fd.get(),
        UserPtr::new(uap.changelist.get().addr()),
        uap.nchanges.get(),
        UserPtr::new(uap.eventlist.get().addr()),
        uap.nevents.get(),
        uap.timeout.get().addr(),
    )? as Register;
    Ok(())
}

/// `KernelApi::event`: the `Native` entries of the event system calls (`kqueue(2)`,
/// `kqueue1(2)`, `kevent(2)` here; `select(2)`, `pselect(2)`, `poll(2)`, `ppoll(2)` in
/// `sys_generic.rs`), each running `mi_syscall` on its own `sysent[]` row.
pub const EVENT_ENTRIES: EventEntries = EventEntries {
    select: native_select,
    kevent: native_kevent,
    ppoll: native_ppoll,
    pselect: native_pselect,
    poll: native_poll,
    kqueue: native_kqueue,
    kqueue1: native_kqueue1,
};

/// `KernelApi::event.kevent`.
pub fn native_kevent(
    nr: Register,
    fd: i32,
    changes: (usize, i32),
    events: (usize, i32),
    timeout: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fd as Register,
        changes.0 as Register,
        changes.1 as Register,
        events.0 as Register,
        events.1 as Register,
        timeout as Register,
    ];
    native(nr, SYS_kevent, args, rval)
}

/// `KernelApi::event.kqueue`.
pub fn native_kqueue(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_kqueue, [0; 6], rval)
}

/// `KernelApi::event.kqueue1`.
pub fn native_kqueue1(nr: Register, flags: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_kqueue1, [flags as Register, 0, 0, 0, 0, 0], rval)
}

/// `kinfo_file`'s kqueue fields of a kqueue file: `kq_count`, `kq_state` (unlocked reads, as
/// the C's `fill_file`).
pub fn kqueue_kinfo(fp: &File) -> (u32, u32) {
    let kq = fp_kqueue(fp);
    (kq.count() as u32, kq.state() as u32)
}

const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<Knote>();
    send_sync::<Klist>();
    send_sync::<crate::sys::eventvar::Kqueue>();
    send_sync::<KqRef>();
};
/* </CODE> */

/* <TESTS> */
/// The host tests' helpers of the modules whose objects deliver events (pipe, tty, socket,
/// fuse): a thread with a table, kqueues, one scan, the descriptors placed and closed by
/// hand, one non-blocking `poll`/`select`.
#[cfg(test)]
pub(crate) mod tests {
    use std::sync::MutexGuard;

    use super::*;
    use crate::kern::kern_descrip::{fdinit, fdinsert, filedesc_init};
    use crate::kern::kern_prot::crget;
    use crate::kern::proc_glue::procinit;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::event::Kevent;
    use crate::sys::file::FileRef;
    use crate::sys::filedesc::fdplock;
    use crate::sys::poll::Pollfd;

    /// Real memory, the process, file and kqueue pools.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        let guard = setup_real_memory();
        crate::machine::cons::consinit();
        procinit();
        filedesc_init();
        kqueue_init();
        guard
    }

    /// A thread of a fresh process with credentials and a new descriptor table.
    pub(crate) fn thread() -> &'static Proc {
        let p = crate::kern::proc_glue::test_proc(1, crget());
        p.process().set_fd(fdinit());
        p
    }

    /// `dokqueue` without the descriptor: a kqueue on the thread's table.
    pub(crate) fn new_kqueue(p: &Proc) -> KqRef {
        let kq = kernel::event::kqueue_alloc::<BsdProcGlue>(p.process().fd_ref()).unwrap();
        kernel::event::fd_kqlist_insert(&kq);
        kq
    }

    /// `kqueue_close` of a kqueue made by `new_kqueue`.
    pub(crate) fn close_kqueue(p: &Proc, kq: KqRef) {
        kernel::event::kqueue_purge(Some(p), &kq);
        kernel::event::kqueue_terminate(Some(p), &kq);
        assert_eq!(kq.refs(), 1);
    }

    /// A knote a new kqueue of `p` owns, attached to no object, for a test that hooks it on
    /// an object's klist as a filter's attach does (the kqueue pools made if they are not):
    /// the kqueue and the knote, given back by [`owned_knote_drop`].
    pub(crate) fn owned_knote(p: &Proc) -> (KqRef, kernel::event::KnHold<BsdProcGlue>) {
        let _ = kernel::event::kqueue_init::<BsdProcGlue>();
        let kq = new_kqueue(p);
        let h = kernel::event::test_knote(&kq, 1);
        (kq, h)
    }

    /// Gives back an [`owned_knote`] (unhooked by the test) and its kqueue.
    pub(crate) fn owned_knote_drop(p: &Proc, kq: KqRef, h: kernel::event::KnHold<BsdProcGlue>) {
        kernel::event::test_knote_drop(&kq, h);
        close_kqueue(p, kq);
    }

    /// One non-blocking scan: the events, and the scan's error.
    pub(crate) fn scan(p: &Proc, kq: &KqRef, out: &mut [Kevent]) -> (usize, Result<(), Errno>) {
        let start = core::pin::pin!(Knote::marker(kq));
        let end = core::pin::pin!(Knote::marker(kq));
        let s = kqueue_scan_setup(kq, start.as_ref(), end.as_ref());
        let mut ts = kernel::clock::Timespec::ZERO;
        let n = out.len();
        kqueue_scan(&s, n, out, Some(&mut ts), p)
    }

    /// Places `fp` (fresh from `fnew`) at descriptor `fd`, as `falloc` and `fdinsert` do:
    /// the slot takes its own reference.
    pub(crate) fn install(p: &Proc, fd: i32, fp: &FileRef) {
        let mut g = fdplock(p.fd());
        g.fd_used(fd);
        fdinsert(&mut g, fd, 0, fp);
    }

    /// `close(2)` of `fd` (`fdrelease`: off the table, its knotes removed, closed).
    pub(crate) fn close_fd(p: &Proc, fd: i32) {
        let g = fdplock(p.fd());
        crate::kern::kern_descrip::fdrelease(p, g, fd).unwrap();
    }

    /// `doppoll` without the copies: register the pollfds on the thread's poll kqueue,
    /// collect what is ready without sleeping.
    pub(crate) fn poll_once(p: &Proc, pl: &mut [Pollfd]) -> usize {
        use kernel::select::{ppollcollect, ppollregister};
        let nfds = pl.len() as u32;
        assert_eq!(kqpoll_init(p, nfds), Ok(()));
        let (mut nevents, mut ncollected) = (0, 0);
        ppollregister(p, pl, &mut nevents, &mut ncollected);
        if nevents > 0 {
            let kq = kernel::event::thread_kq(p).clone();
            let mut kev = [Kevent::default(); crate::sys::eventvar::KQ_NEVENTS];
            let count = nevents.min(kev.len());
            let (ready, error) = scan(p, &kq, &mut kev[..count]);
            assert_eq!(error, Ok(()));
            for k in &kev[..ready] {
                ncollected += ppollcollect(p, k, pl);
            }
        }
        kqpoll_done(p, nfds);
        ncollected
    }

    /// `dopselect` likewise, over one word of each fd set: the three output words.
    pub(crate) fn select_once(
        p: &Proc,
        nd: usize,
        r: kernel::select::FdMask,
        w: kernel::select::FdMask,
    ) -> Result<[kernel::select::FdMask; 3], Errno> {
        use kernel::select::{FdMask, pselcollect, pselregister};
        assert_eq!(kqpoll_init(p, nd as u32), Ok(()));
        let pibits = [r, w, 0];
        let mut pobits = [0 as FdMask; 3];
        let mut ncollected = 0;
        let result = pselregister(p, &pibits, 1, nd).and_then(|nevents| {
            let kq = kernel::event::thread_kq(p).clone();
            let mut kev = [Kevent::default(); crate::sys::eventvar::KQ_NEVENTS];
            let count = nevents.min(kev.len());
            let (ready, mut error) = scan(p, &kq, &mut kev[..count]);
            for k in &kev[..ready] {
                if error.is_ok() {
                    error = pselcollect(p, k, &mut pobits, 1, &mut ncollected);
                }
            }
            error
        });
        kqpoll_done(p, nd as u32);
        result.map(|()| pobits)
    }

    #[test]
    fn a_kqueue_file_s_operations_answer_as_kqueueops() {
        use kernel::file::FileGlue;
        let _g = setup();
        let p = thread();
        // A kqueue file as `dokqueue` makes one: the glue's ops and object.
        let kq = new_kqueue(p);
        let fp = crate::kern::kern_descrip::fnew(p).unwrap();
        let (ops, data) = crate::kern::kern_descrip::BsdFileGlue::kqueue_file(kq.clone());
        fp.install(crate::sys::file::DTYPE_KQUEUE, ops, data);
        assert!(core::ptr::eq(ops, &KQUEUEOPS));
        let mut st = Stat::default();
        assert_eq!(kqueue_stat(&fp, &mut st, p), Ok(()));
        assert_eq!(
            (st.st_size, st.st_blksize as usize, st.st_mode),
            (0, 32, S_IFIFO)
        );
        assert_eq!(kqueue_ioctl(&fp, 0, &mut [], p), Err(Errno::ENOTTY));
        assert_eq!(kqueue_kinfo(&fp), (0, 0));
        assert_eq!(
            fp_kqueue(&fp).addr(),
            fp.data().map_or(0, |d| match d {
                BsdFileData::Kqueue(kq) => kq.addr(),
                _ => 0,
            })
        );
        // The file's last reference: kqueue_close terminates the queue.
        drop(kq);
        assert_eq!(fp.frele(Some(p)), Ok(()));
    }
}
/* </TESTS> */
