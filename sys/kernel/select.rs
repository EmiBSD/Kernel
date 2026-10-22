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
 * Copyright (c) 1996 Theo de Raadt
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)sys_generic.c	8.5 (Berkeley) 1/21/94
 */

/*-
 * Copyright (c) 1992, 1993
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
 *	@(#)select.h	8.2 (Berkeley) 1/4/94
 */

/*
 * Copyright (c) 1996 Theo de Raadt
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `select(2)`, `pselect(2)`, `poll(2)` and `ppoll(2)` over the thread's poll kqueue
//! (`sys_generic.c`'s second half), `selwakeup(9)`, `struct selinfo` and `struct pollfd`.
//!
//! Upstream: sys/kern/sys_generic.c @ 3ce1f3f79392
//! Upstream: sys/sys/selinfo.h @ 3ce1f3f79392
//! Upstream: sys/sys/poll.h @ 3ce1f3f79392
//! LZ: sys/kern/sys_generic.rs@3c62ede68802
//! LZ: sys/sys/selinfo.rs@3c62ede68802
//! LZ: sys/sys/poll.rs@3c62ede68802
//!
//! Each descriptor of the sets or the array becomes up to three kevents on the thread's poll
//! kqueue (`kqpoll_init`), tagged with the call's serial (`p_kq_serial`, plus the pollfd's
//! index for poll); the scan's events come back as bits or `revents`, and the knotes of
//! older calls are dropped lazily by later scans.
//!
//! ## Deviations
//! - `KTRACE` is not configured (`ktrfdset`, `ktrpollfd`, `ktrreltimespec`).
//! - `kqpoll_init` can fail (`ENOMEM`: the kqueue pool or `malloc(9)` gives nothing), which
//!   fails the call where the C cannot fail (LZ's).
//! - The signal mask of `pselect(2)`/`ppoll(2)` is installed by the caller's closure
//!   (`dosigsuspend`, `kern_sig`'s until N4.5), at the C's point.
//!
//! ## Redesign
//! - LZ's `POLL_LASTERR` was a `StaticCell` written under the kernel lock with an `unsafe`
//!   `get_mut`; here it is a `Mutexed` word pair, the C's unlocked static made a lock.
//! - The fd sets are `fd_mask` words copied in and out as words (`UserPtr<FdMask>`), the
//!   `pollfd` array a slice of `abi_pod!` records (`UserPtr<Pollfd>`): LZ copied bytes and
//!   rebuilt each value by hand. Up to one word per set and four pollfds stay on the stack,
//!   the rest is a `malloc(M_TEMP)` array, as the C's.
//! - `scan` state and markers: `kernel::event`'s pinned stack markers.

use core::ptr;

use hal::malloc::{M_CANFAIL, M_WAITOK, MArray};
use hal::mutex::Mutexed;
use hal::sleep::{INFSLP, PCATCH};
use hal::sys::errno::Errno;
use hal::user::UserPtr;

use crate::assert::{kassert, kpanic};
use crate::clock::{self, Timespec};
use crate::event::{
    __EV_HUP, __EV_POLL, __EV_SELECT, EV_ADD, EV_ENABLE, EV_ERROR, EVFILT_EXCEPT, EVFILT_READ,
    EVFILT_WRITE, KQ_NEVENTS, Kevent, Klist, Knote, NOTE_OOB, NOTE_SUBMIT, ev_set, kqpoll_done,
    kqpoll_init, kqueue_register, kqueue_scan, kqueue_scan_setup, thread_kq,
};
use crate::file::FileGlue;
use crate::proc::{ProcGlue, Thread};
use crate::resource::RLIMIT_NOFILE;

/// `POLLIN`.
pub const POLLIN: i16 = 0x0001;
/// `POLLPRI`.
pub const POLLPRI: i16 = 0x0002;
/// `POLLOUT`.
pub const POLLOUT: i16 = 0x0004;
/// `POLLERR`.
pub const POLLERR: i16 = 0x0008;
/// `POLLHUP`.
pub const POLLHUP: i16 = 0x0010;
/// `POLLNVAL`.
pub const POLLNVAL: i16 = 0x0020;
/// `POLLRDNORM`.
pub const POLLRDNORM: i16 = 0x0040;
/// `POLLNORM`.
pub const POLLNORM: i16 = POLLRDNORM;
/// `POLLWRNORM`.
pub const POLLWRNORM: i16 = POLLOUT;
/// `POLLRDBAND`.
pub const POLLRDBAND: i16 = 0x0080;
/// `POLLWRBAND`.
pub const POLLWRBAND: i16 = 0x0100;
/// `POLL_NOHUP`: internal use only.
pub const POLL_NOHUP: i16 = 0x1000;
/// `INFTIM`: an infinite `poll(2)` timeout.
pub const INFTIM: i32 = -1;

/// `fd_mask`: a word of an `fd_set`.
pub type FdMask = u32;
/// `NFDBITS`: bits per mask.
pub const NFDBITS: usize = 32;
/// `nfds_t`.
pub type Nfds = u32;

/// `M_TEMP`: the `malloc(9)` type of the fd sets and the pollfd array.
const M_TEMP: i32 = 127;
/// `PSOCK`: the priority of the sleep for nothing.
const PSOCK: i32 = 24;
/// `MAXTSLP`: the longest sleep that is not `INFSLP`.
const MAXTSLP: u64 = u64::MAX - 1;

/// `struct pollfd` (`pollfd_t`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pollfd {
    /// `fd`.
    pub fd: i32,
    /// `events`.
    pub events: i16,
    /// `revents`.
    pub revents: i16,
}

hal::abi_pod!(Pollfd {
    fd: i32,
    events: i16,
    revents: i16
});

/// `struct selinfo`: the knotes of the threads that wish to be notified when I/O becomes
/// possible on one of a driver's objects (`select(2)`, `poll(2)` or `kevent(2)`); its klist is
/// the kernel lock's.
pub struct Selinfo<G: ProcGlue> {
    /// `si_note`: kernel note list.
    pub si_note: Klist<G>,
}

impl<G: ProcGlue> Selinfo<G> {
    /// A zeroed record: no knotes, the list locked by the kernel lock.
    pub const fn new() -> Self {
        Self {
            si_note: Klist::new(),
        }
    }
}

impl<G: ProcGlue> Default for Selinfo<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `howmany(x, y)`.
pub const fn howmany(x: usize, y: usize) -> usize {
    x.div_ceil(y)
}

/// `FD_SET(fd, set)` over the words of one set.
fn fd_set(fd: usize, set: &mut [FdMask]) {
    if let Some(w) = set.get_mut(fd / NFDBITS) {
        *w |= 1 << (fd % NFDBITS);
    }
}

/// `nowake`: the channel of a sleep no wakeup ends (the poll/select family's for nothing).
static NOWAKE: u8 = 0;

/// The `select(2)` sets of one call: `ni` bytes each, three in and three out, one word each on
/// the stack (the C's `bits[6]`), or a `malloc(M_TEMP)` array past it.
enum Bits {
    /// Up to `NFDBITS` descriptors.
    Stack([FdMask; 6]),
    /// More: `6 * words` words.
    Heap(MArray<FdMask>),
}

impl Bits {
    fn words(&mut self) -> &mut [FdMask] {
        match self {
            Self::Stack(b) => b,
            Self::Heap(m) => m,
        }
    }
}

/// `select(2)`: `nd` descriptors of the sets at `sets` (in, out, except; `NULL` ones
/// skipped), waiting at most the `struct timeval` at `tv` (`NULL`: for ever); the number of
/// descriptors ready.
pub fn sys_select<G: ProcGlue>(
    p: &Thread<G>,
    nd: i32,
    sets: [UserPtr<FdMask>; 3],
    tv: UserPtr<[i64; 2]>,
) -> Result<i32, Errno> {
    let mut tsp = None;
    if !tv.is_null() {
        let [sec, usec] = tv.copyin()?;
        // KTRACE: not configured.
        if sec < 0 || !(0..1_000_000).contains(&usec) {
            return Err(Errno::EINVAL);
        }
        tsp = Some(Timespec::new(sec, usec * 1000));
    }

    dopselect(p, nd, sets, tsp, None)
}

/// `pselect(2)`: [`sys_select`] with a `struct timespec` timeout and, while it waits, the
/// signal mask `sigsuspend` installs (the caller's `dosigsuspend` of the mask it copied in).
pub fn sys_pselect<G: ProcGlue>(
    p: &Thread<G>,
    nd: i32,
    sets: [UserPtr<FdMask>; 3],
    ts: UserPtr<[i64; 2]>,
    sigsuspend: Option<&dyn Fn()>,
) -> Result<i32, Errno> {
    let tsp = copyin_timespec(ts)?;
    dopselect(p, nd, sets, tsp, sigsuspend)
}

/// A `struct timespec` timeout from userland: `None` for `NULL`, `EINVAL` for a negative or
/// unnormalised one (`pselect(2)` and `ppoll(2)` check it before they copy the mask in).
pub fn copyin_timespec(ts: UserPtr<[i64; 2]>) -> Result<Option<Timespec>, Errno> {
    if ts.is_null() {
        return Ok(None);
    }
    let [sec, nsec] = ts.copyin()?;
    // KTRACE: not configured.
    if sec < 0 || !(0..1_000_000_000).contains(&nsec) {
        return Err(Errno::EINVAL);
    }
    Ok(Some(Timespec::new(sec, nsec)))
}

/// `TIMESPEC_TO_NSEC(ts)` bounded to `[1, MAXTSLP]`: the sleep of a timeout that is set.
fn sleep_nsecs(ts: &Timespec) -> u64 {
    let n = (ts.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(ts.tv_nsec as u64))
        .unwrap_or(u64::MAX);
    n.clamp(1, MAXTSLP)
}

/// The sleep of a call that registered nothing and collected nothing: none for a zero
/// timeout, else on `nowake` until the timeout (`select` and `poll` are not restarted after
/// signals; the timeout's end is no error).
fn sleep_for_nothing(timeout: Option<&Timespec>, wmesg: &'static str) -> Result<(), Errno> {
    let mut nsecs = INFSLP;
    if let Some(ts) = timeout {
        if *ts == Timespec::ZERO {
            return Ok(());
        }
        nsecs = sleep_nsecs(ts);
    }
    match hal::sleep::tsleep_nsec(ptr::from_ref(&NOWAKE), PSOCK | PCATCH, wmesg, nsecs) {
        Err(Errno::ERESTART) => Err(Errno::EINTR),
        Err(Errno::EWOULDBLOCK) => Ok(()),
        other => other,
    }
}

/// `dopselect`: the body of `select(2)` and `pselect(2)`.
pub fn dopselect<G: ProcGlue>(
    p: &Thread<G>,
    nd: i32,
    sets: [UserPtr<FdMask>; 3],
    timeout: Option<Timespec>,
    sigsuspend: Option<&dyn Fn()>,
) -> Result<i32, Errno> {
    let mut timeout = timeout;
    let mut ncollected = 0usize;
    let mut retval = 0i32;

    if nd < 0 {
        return Err(Errno::EINVAL);
    }

    let nfiles = p.fd().nfiles() as i32;
    let nd = nd.min(nfiles) as usize;

    let words = howmany(nd, NFDBITS);
    let mut bits = if words > 1 {
        match MArray::new(M_TEMP, M_WAITOK, 6 * words, |_| 0) {
            Some(m) => Bits::Heap(m),
            None => return Err(Errno::ENOMEM),
        }
    } else {
        Bits::Stack([0; 6])
    };
    let (pibits, pobits) = bits.words().split_at_mut(3 * words);

    kqpoll_init(p, nd as u32)?;

    let error = 'done: {
        // getbits
        for (x, name) in sets.iter().enumerate() {
            if !name.is_null()
                && let Err(e) = name.copyin_slice(&mut pibits[x * words..(x + 1) * words])
            {
                break 'done Err(e);
            }
        }
        // KTRACE: not configured.

        if let Some(sigsuspend) = sigsuspend {
            sigsuspend();
        }

        // Register kqueue events
        let mut nevents = match pselregister(p, pibits, words, nd) {
            Ok(n) => n,
            Err(e) => break 'done Err(e),
        };

        // The poll/select family of syscalls has been designed to block when file
        // descriptors are not available, even if there's nothing to wait for.
        if nevents == 0 && ncollected == 0 {
            break 'done sleep_for_nothing(timeout.as_ref(), "kqsel");
        }

        // Do not block if registering found pending events.
        if ncollected > 0 {
            timeout = Some(Timespec::ZERO);
        }

        // Collect at most `nevents' possibly waiting in kqueue_scan()
        let kq = thread_kq(p);
        let start = core::pin::pin!(Knote::marker(kq));
        let end = core::pin::pin!(Knote::marker(kq));
        let scan = kqueue_scan_setup(kq, start.as_ref(), end.as_ref());
        let mut error = Ok(());
        while nevents > 0 {
            let mut kev = [Kevent::default(); KQ_NEVENTS];
            // Maximum number of events per iteration
            let count = kev.len().min(nevents);
            let (ready, e) = kqueue_scan(&scan, count, &mut kev, timeout.as_mut(), p);
            error = e;

            // Convert back events that are ready.
            for k in &kev[..ready] {
                if error.is_err() {
                    break;
                }
                error = pselcollect(p, k, pobits, words, &mut ncollected);
            }
            // Stop if there was an error or if we had enough space to collect all events
            // that were ready.
            if error.is_err() || ready < count {
                break;
            }

            nevents -= ready;
        }
        drop(scan);
        retval = ncollected as i32;
        error
    };

    // done: putbits
    let mut error = error;
    if error.is_ok() {
        for (x, name) in sets.iter().enumerate() {
            if !name.is_null()
                && let Err(error2) = name.copyout_slice(&pobits[x * words..(x + 1) * words])
            {
                error = Err(error2);
            }
        }
        // KTRACE: not configured.
    }

    kqpoll_done(p, nd as u32);

    error.map(|()| retval)
}

/// Convert fd_set into kqueue events and register them on the per-thread queue; returns how
/// many were registered. `pibits` holds the three input sets, `words` masks each.
pub fn pselregister<G: ProcGlue>(
    p: &Thread<G>,
    pibits: &[FdMask],
    words: usize,
    nfd: usize,
) -> Result<usize, Errno> {
    const EVF: [i16; 3] = [EVFILT_READ, EVFILT_WRITE, EVFILT_EXCEPT];
    const EVFF: [u32; 3] = [0, 0, NOTE_OOB];
    let kq = thread_kq(p);
    let mut nevents = 0;

    for msk in 0..3 {
        let mut i = 0;
        while i < nfd {
            let mut bits = pibits[msk * words + i / NFDBITS];
            while bits != 0 {
                let j = bits.trailing_zeros() as usize;
                let fd = i + j;
                if fd >= nfd {
                    break;
                }
                bits &= !(1 << j);

                let mut kev = ev_set(
                    fd,
                    EVF[msk],
                    EV_ADD | EV_ENABLE | __EV_SELECT,
                    EVFF[msk],
                    0,
                    p.p_kq_serial.get() as usize,
                );
                match kqueue_register(kq, &mut kev, 0, Some(p)) {
                    Ok(()) => nevents += 1,
                    // No underlying kqfilter, unimplemented filter, specific to FIFO and
                    // __EV_SELECT
                    Err(Errno::EOPNOTSUPP | Errno::EINVAL | Errno::EPERM) => {}
                    // Device has been detached, and the rest
                    Err(e) => return Err(e),
                }
            }
            i += NFDBITS;
        }
    }

    Ok(nevents)
}

/// Convert given kqueue event into corresponding select(2) bit.
pub fn pselcollect<G: ProcGlue>(
    p: &Thread<G>,
    kevp: &Kevent,
    pobits: &mut [FdMask],
    words: usize,
    ncollected: &mut usize,
) -> Result<(), Errno> {
    if kevp.udata as u64 != p.p_kq_serial.get() {
        kpanic(format_args!(
            "pselcollect: spurious kevp fd {} udata {:#x} serial {:#x}",
            kevp.ident,
            kevp.udata,
            p.p_kq_serial.get()
        ));
    }

    if kevp.flags & EV_ERROR != 0 {
        return Err(Errno::from_raw(kevp.data as i32).unwrap_or(Errno::EINVAL));
    }

    let set = match kevp.filter {
        EVFILT_READ => 0,
        EVFILT_WRITE => 1,
        EVFILT_EXCEPT => 2,
        _ => {
            kassert!(false);
            return Ok(());
        }
    };
    fd_set(kevp.ident, &mut pobits[set * words..(set + 1) * words]);
    *ncollected += 1;

    Ok(())
}

/// `selwakeup(sip)`: do a wakeup when a selectable event occurs.
pub fn selwakeup<G: ProcGlue>(sip: &Selinfo<G>) {
    // No knote on it (nothing ever polled the object): no kernel lock to take.
    if sip.si_note.is_empty() {
        return;
    }
    hal::klock::kernel_lock();
    sip.si_note.knote_legacy(i64::from(NOTE_SUBMIT));
    hal::klock::kernel_unlock();
}

/// Only copyout the revents field.
pub fn pollout(pl: &[Pollfd], upl: UserPtr<Pollfd>) -> Result<(), Errno> {
    for (i, pfd) in pl.iter().enumerate() {
        let at = upl
            .checked_add(i)
            .and_then(|u| u.addr().checked_add(core::mem::offset_of!(Pollfd, revents)))
            .ok_or(Errno::EFAULT)?;
        UserPtr::<i16>::new(at).copyout(&pfd.revents)?;
    }
    Ok(())
}

/// `poll(2)`: the `nfds` pollfds at `fds`, waiting at most `msec` milliseconds (`INFTIM`:
/// for ever); the number of pollfds with events.
pub fn sys_poll<G: ProcGlue>(
    p: &Thread<G>,
    fds: UserPtr<Pollfd>,
    nfds: Nfds,
    msec: i32,
) -> Result<i32, Errno> {
    let mut tsp = None;

    if msec != INFTIM {
        if msec < 0 {
            return Err(Errno::EINVAL);
        }
        let sec = i64::from(msec / 1000);
        tsp = Some(Timespec::new(
            sec,
            (i64::from(msec) - sec * 1000) * 1_000_000,
        ));
    }

    doppoll(p, fds, nfds, tsp, None)
}

/// `ppoll(2)`: [`sys_poll`] with a `struct timespec` timeout and, while it waits, the
/// signal mask `sigsuspend` installs.
pub fn sys_ppoll<G: ProcGlue>(
    p: &Thread<G>,
    fds: UserPtr<Pollfd>,
    nfds: Nfds,
    ts: UserPtr<[i64; 2]>,
    sigsuspend: Option<&dyn Fn()>,
) -> Result<i32, Errno> {
    let tsp = copyin_timespec(ts)?;
    doppoll(p, fds, nfds, tsp, sigsuspend)
}

/// The pollfds of one call: four on the stack (the C's `pfds[4]`), or a `malloc(M_TEMP)`
/// array past them.
enum Pollfds {
    /// Up to four.
    Stack([Pollfd; 4], usize),
    /// More.
    Heap(MArray<Pollfd>),
}

impl Pollfds {
    fn slice(&mut self) -> &mut [Pollfd] {
        match self {
            Self::Stack(a, n) => &mut a[..*n],
            Self::Heap(m) => m,
        }
    }
}

/// `doppoll`: the body of `poll(2)` and `ppoll(2)` over the user array `fds`.
pub fn doppoll<G: ProcGlue>(
    p: &Thread<G>,
    fds: UserPtr<Pollfd>,
    nfds: Nfds,
    timeout: Option<Timespec>,
    sigsuspend: Option<&dyn Fn()>,
) -> Result<i32, Errno> {
    let mut timeout = timeout;
    let mut ncollected = 0usize;
    let mut nevents = 0usize;
    let mut retval = 0i32;

    // Standards say no more than MAX_OPEN; this is possibly better.
    let limit = (p.lim_cur(RLIMIT_NOFILE) as i64)
        .min(i64::from(<G::FileGlue as FileGlue>::maxfiles()))
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    if i64::from(nfds) > i64::from(limit) {
        return Err(Errno::EINVAL);
    }

    // optimize for the default case, of a small nfds value
    let n = nfds as usize;
    let mut pfds = if n > 4 {
        match MArray::new(M_TEMP, M_WAITOK | M_CANFAIL, n, |_| Pollfd::default()) {
            Some(m) => Pollfds::Heap(m),
            None => return Err(Errno::EINVAL),
        }
    } else {
        Pollfds::Stack([Pollfd::default(); 4], n)
    };
    let pl = pfds.slice();

    kqpoll_init(p, nfds)?;

    if let Err(error) = fds.copyin_slice(pl) {
        // bad:
        kqpoll_done(p, nfds);
        return Err(error);
    }

    if let Some(sigsuspend) = sigsuspend {
        sigsuspend();
    }

    // Register kqueue events
    ppollregister(p, pl, &mut nevents, &mut ncollected);

    let error = 'done: {
        // The poll/select family of syscalls has been designed to block when file
        // descriptors are not available, even if there's nothing to wait for.
        if nevents == 0 && ncollected == 0 {
            break 'done sleep_for_nothing(timeout.as_ref(), "kqpoll");
        }

        // Do not block if registering found pending events.
        if ncollected > 0 {
            timeout = Some(Timespec::ZERO);
        }

        // Collect at most `nevents' possibly waiting in kqueue_scan()
        let kq = thread_kq(p);
        let start = core::pin::pin!(Knote::marker(kq));
        let end = core::pin::pin!(Knote::marker(kq));
        let scan = kqueue_scan_setup(kq, start.as_ref(), end.as_ref());
        let mut error = Ok(());
        while nevents > 0 {
            let mut kev = [Kevent::default(); KQ_NEVENTS];
            // Maximum number of events per iteration
            let count = kev.len().min(nevents);
            let (ready, e) = kqueue_scan(&scan, count, &mut kev, timeout.as_mut(), p);
            error = e;

            // Convert back events that are ready.
            for k in &kev[..ready] {
                ncollected += ppollcollect(p, k, pl);
            }

            // Stop if there was an error or if we had enough place to collect all events
            // that were ready.
            if error.is_err() || ready < count {
                break;
            }

            nevents -= ready;
        }
        drop(scan);
        retval = ncollected as i32;
        error
    };

    // done: NOTE: poll(2) is not restarted after a signal and EWOULDBLOCK is ignored (since
    // the whole point is to see what would block).
    let error = match error {
        Err(Errno::EINTR) => pollout(pl, fds).and(Err(Errno::EINTR)),
        Err(Errno::EWOULDBLOCK) | Ok(()) => pollout(pl, fds),
        other => other,
    };
    // KTRACE: not configured.

    kqpoll_done(p, nfds);

    error.map(|()| retval)
}

/// `ppollregister_evts`: registers the `kev` events of one pollfd; failures become its
/// `revents`. Returns how many were registered.
pub fn ppollregister_evts<G: ProcGlue>(
    p: &Thread<G>,
    kev: &mut [Kevent],
    pl: &mut Pollfd,
    pollid: u32,
) -> usize {
    let kq = thread_kq(p);
    let mut nevents = 0;
    let nkev = kev.len();

    kassert!(pl.revents == 0);

    for kevp in kev.iter_mut() {
        loop {
            match kqueue_register(kq, kevp, pollid, Some(p)) {
                Ok(()) => nevents += 1,
                // No underlying kqfilter, unimplemented filter
                Err(Errno::EOPNOTSUPP | Errno::EINVAL) => {}
                // Bad file descriptor
                Err(Errno::EBADF) => pl.revents |= POLLNVAL,
                // Specific to FIFO
                Err(Errno::EPERM) => {
                    kassert!(kevp.filter == EVFILT_WRITE);
                    if nkev == 1 {
                        // If this is the only filter make sure POLLHUP is passed to
                        // userland.
                        kevp.filter = EVFILT_EXCEPT;
                        continue;
                    }
                }
                // Device has been detached, and the rest
                Err(_) => pl.revents |= POLLERR,
            }
            break;
        }
    }

    nevents
}

/// Convert pollfd into kqueue events and register them on the per-thread queue. At most 3
/// events can correspond to a single pollfd.
pub fn ppollregister<G: ProcGlue>(
    p: &Thread<G>,
    pl: &mut [Pollfd],
    nregistered: &mut usize,
    ncollected: &mut usize,
) {
    for (i, pfd) in pl.iter_mut().enumerate() {
        pfd.events &= !POLL_NOHUP;
        pfd.revents = 0;

        if pfd.fd < 0 {
            continue;
        }

        // POLLHUP checking is implicit in the event filters. However, the checking must be
        // even if no events are requested.
        let forcehup = pfd.events & !POLLHUP == 0;

        let mut kev = [Kevent::default(); 3];
        let mut nkev = 0;
        let udata = p.p_kq_serial.get().wrapping_add(i as u64) as usize;
        let flags = EV_ADD | EV_ENABLE | __EV_POLL;
        if pfd.events & (POLLIN | POLLRDNORM) != 0 {
            kev[nkev] = ev_set(pfd.fd as usize, EVFILT_READ, flags, 0, 0, udata);
            nkev += 1;
        }
        if pfd.events & (POLLOUT | POLLWRNORM) != 0 {
            kev[nkev] = ev_set(pfd.fd as usize, EVFILT_WRITE, flags, 0, 0, udata);
            nkev += 1;
        }
        if pfd.events & (POLLPRI | POLLRDBAND) != 0 || forcehup {
            let evff = if forcehup { 0 } else { NOTE_OOB };

            kev[nkev] = ev_set(pfd.fd as usize, EVFILT_EXCEPT, flags, evff, 0, udata);
            nkev += 1;
        }

        if nkev == 0 {
            continue;
        }

        *nregistered += ppollregister_evts(p, &mut kev[..nkev], pfd, i as u32);

        if pfd.revents != 0 {
            *ncollected += 1;
        }
    }
}

/// `poll_lasterr`: the last report of an unclaimed event (`ratecheck`'s `(sec, usec)`).
static POLL_LASTERR: Mutexed<(i64, i64)> = Mutexed::new(hal::intr::IPL_NONE, (0, 0));
/// `poll_errintvl`: at most one report per five seconds.
const POLL_ERRINTVL: (i64, i64) = (5, 0);

/// `ratecheck(lasttime, mininterval)` (`kern_time.c`): true, and `lasttime` set to now (the
/// uptime as of the last tick, in `(tv_sec, tv_usec)`), when at least `mininterval` has
/// passed since `lasttime` or `lasttime` is (0, 0) (so the message is seen at least once).
fn ratecheck(lasttime: &Mutexed<(i64, i64)>, mininterval: (i64, i64)) -> bool {
    let now = clock::getnanouptime();
    let tv = (now.tv_sec, now.tv_nsec / 1000);

    let mut last = lasttime.lock();
    let mut delta = (tv.0 - last.0, tv.1 - last.1);
    if delta.1 < 0 {
        delta = (delta.0 - 1, delta.1 + 1_000_000);
    }
    if delta >= mininterval || *last == (0, 0) {
        *last = tv;
        return true;
    }
    false
}

/// A command name as `%s` prints it: its bytes up to the NUL.
struct Comm<'a>(&'a [u8]);

impl core::fmt::Display for Comm<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for &b in self.0 {
            core::fmt::Write::write_char(f, char::from(b))?;
        }
        Ok(())
    }
}

/// Convert given kqueue event into corresponding poll(2) revents bit; 1 when it is the
/// first event of its pollfd.
pub fn ppollcollect<G: ProcGlue>(p: &Thread<G>, kevp: &Kevent, pl: &mut [Pollfd]) -> usize {
    // Extract poll array index
    let serial = p.p_kq_serial.get();
    let i = (kevp.udata as u64).wrapping_sub(serial) as usize;
    let nfds = pl.len();

    if i >= nfds {
        kpanic(format_args!(
            "ppollcollect: spurious kevp nfds {nfds} udata {:#x} serial {serial:#x}",
            kevp.udata
        ));
    }
    if kevp.ident as i32 != pl[i].fd {
        kpanic(format_args!(
            "ppollcollect: kevp {}/{} mismatch fd {}!={} serial {serial:#x}",
            i + 1,
            nfds,
            kevp.ident as i32,
            pl[i].fd
        ));
    }

    // A given descriptor may already have generated an error against another filter during
    // kqueue_register().
    //
    // Make sure to set the appropriate flags but do not increment `*retval' more than once.
    let already_seen = pl[i].revents != 0;
    let pfd = &mut pl[i];

    'done: {
        // POLLNVAL preempts other events.
        if kevp.flags & EV_ERROR != 0 && kevp.data == i64::from(Errno::EBADF.as_i32()) {
            pfd.revents = POLLNVAL;
            break 'done;
        } else if pfd.revents & POLLNVAL != 0 {
            break 'done;
        }

        match kevp.filter {
            EVFILT_READ => {
                if kevp.flags & __EV_HUP != 0 {
                    pfd.revents |= POLLHUP;
                }
                if pfd.events & (POLLIN | POLLRDNORM) != 0 {
                    pfd.revents |= pfd.events & (POLLIN | POLLRDNORM);
                }
            }
            EVFILT_WRITE => {
                // POLLHUP and POLLOUT/POLLWRNORM are mutually exclusive
                if kevp.flags & __EV_HUP != 0 {
                    pfd.revents |= POLLHUP;
                } else if pfd.events & (POLLOUT | POLLWRNORM) != 0 {
                    pfd.revents |= pfd.events & (POLLOUT | POLLWRNORM);
                }
            }
            EVFILT_EXCEPT => {
                if kevp.flags & __EV_HUP != 0 {
                    pfd.revents |= POLLHUP;
                    break 'done;
                }
                if pfd.events & (POLLPRI | POLLRDBAND) != 0 {
                    pfd.revents |= pfd.events & (POLLPRI | POLLRDBAND);
                }
            }
            _ => {
                kassert!(false);
            }
        }
    }

    // Make noise about unclaimed events as they might indicate a bug and can result in
    // spurious-looking wakeups of poll(2).
    //
    // Live-locking within the system call should not happen because the scan loop in
    // doppoll() has an upper limit for the number of events to process.
    if pfd.revents == 0 && ratecheck(&POLL_LASTERR, POLL_ERRINTVL) {
        let comm = p.process().comm();
        crate::prf::printf(format_args!(
            "{}[{}]: poll index {} fd {} events 0x{:x} filter {}/0x{:x} unclaimed\n",
            Comm(&comm),
            p.p_tid.get(),
            i,
            pfd.fd,
            pfd.events,
            kevp.filter,
            kevp.flags
        ));
    }

    usize::from(!already_seen && pfd.revents != 0)
}

const _: () = {
    assert!(size_of::<Pollfd>() == 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `sys_generic.c`'s select and poll half over `kernel::event`'s test glue:
    // the kevents a pollfd or an fd set bit turns into and what a failed registration leaves,
    // the events turned back into `revents` and bits, the calls end to end over the user-copy
    // double (ready objects, a FIFO's write turned into the hang-up check, the errnos),
    // `pollout` copying only `revents`, and `selwakeup`.

    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::event::KlistKind;
    use crate::event::tests::{close_fd, obj, open_obj, setup};
    use crate::testutil;

    fn pfd(fd: i32, events: i16) -> Pollfd {
        Pollfd {
            fd,
            events,
            revents: 0,
        }
    }

    /// A region of the user-copy double of this test's own (`k`-th of 64 bytes... 4 KiB).
    fn user(k: usize) -> usize {
        testutil::USER_BASE + 0x8000 + k * 0x1000
    }

    #[test]
    fn ppollregister_reports_failed_registrations() {
        let p = setup(50);
        assert_eq!(kqpoll_init(p, 3), Ok(()));
        // No descriptor is open: every registration fails with EBADF, which poll(2) reports as
        // POLLNVAL; a negative fd is skipped.
        let mut pl = vec![pfd(7, POLLIN), pfd(-1, POLLIN), pfd(8, 0)];
        let (mut nregistered, mut ncollected) = (0, 0);
        ppollregister(p, &mut pl, &mut nregistered, &mut ncollected);
        assert_eq!(nregistered, 0);
        assert_eq!(ncollected, 2);
        assert_eq!(pl[0].revents, POLLNVAL);
        assert_eq!(pl[1].revents, 0);
        assert_eq!(pl[2].revents, POLLNVAL, "POLLHUP is always checked");
        kqpoll_done(p, 3);
    }

    #[test]
    fn ppollcollect_converts_events() {
        let p = setup(51);
        let s = p.p_kq_serial.get() as usize;
        let mut pl = vec![pfd(3, POLLIN | POLLOUT), pfd(4, POLLPRI)];
        let read = ev_set(3, EVFILT_READ, 0, 0, 0, s);
        assert_eq!(ppollcollect(p, &read, &mut pl), 1);
        assert_eq!(pl[0].revents, POLLIN);
        // a second event on the same pollfd is not counted again
        let write = ev_set(3, EVFILT_WRITE, 0, 0, 0, s);
        assert_eq!(ppollcollect(p, &write, &mut pl), 0);
        assert_eq!(pl[0].revents, POLLIN | POLLOUT);
        // hang-up on the except filter, and on the write filter instead of POLLOUT
        let hup = ev_set(4, EVFILT_EXCEPT, __EV_HUP, 0, 0, s + 1);
        assert_eq!(ppollcollect(p, &hup, &mut pl), 1);
        assert_eq!(pl[1].revents, POLLHUP);
        let mut pl2 = vec![pfd(5, POLLOUT)];
        let whup = ev_set(5, EVFILT_WRITE, __EV_HUP, 0, 0, s);
        assert_eq!(ppollcollect(p, &whup, &mut pl2), 1);
        assert_eq!(pl2[0].revents, POLLHUP);
        // EBADF preempts everything
        let bad = ev_set(
            4,
            EVFILT_EXCEPT,
            EV_ERROR,
            0,
            i64::from(Errno::EBADF.as_i32()),
            s + 1,
        );
        assert_eq!(ppollcollect(p, &bad, &mut pl), 0);
        assert_eq!(pl[1].revents, POLLNVAL);
        // and stays: a later event leaves POLLNVAL alone
        assert_eq!(
            ppollcollect(p, &ev_set(4, EVFILT_EXCEPT, 0, 0, 0, s + 1), &mut pl),
            0
        );
        assert_eq!(pl[1].revents, POLLNVAL);
    }

    #[test]
    fn pselcollect_sets_bits_and_returns_errors() {
        let p = setup(52);
        let s = p.p_kq_serial.get() as usize;
        let words = howmany(40, NFDBITS);
        let mut pobits = vec![0 as FdMask; 3 * words];
        let mut n = 0;
        let w = ev_set(33, EVFILT_WRITE, 0, 0, 0, s);
        assert_eq!(pselcollect(p, &w, &mut pobits, words, &mut n), Ok(()));
        assert_eq!(n, 1);
        assert_eq!(pobits[words + 1], 1 << 1);
        let x = ev_set(0, EVFILT_EXCEPT, 0, 0, 0, s);
        assert_eq!(pselcollect(p, &x, &mut pobits, words, &mut n), Ok(()));
        assert_eq!(pobits[2 * words], 1);
        let err = ev_set(
            2,
            EVFILT_READ,
            EV_ERROR,
            0,
            i64::from(Errno::EBADF.as_i32()),
            s,
        );
        assert_eq!(
            pselcollect(p, &err, &mut pobits, words, &mut n),
            Err(Errno::EBADF)
        );
        assert_eq!(n, 2);
    }

    #[test]
    fn pollout_copies_only_revents() {
        let _p = setup(53);
        let upl = UserPtr::<Pollfd>::new(user(0));
        upl.copyout_slice(&[pfd(7, POLLIN), pfd(8, POLLOUT)])
            .unwrap();
        let mut kernel = [pfd(7, POLLIN), pfd(99, POLLOUT)];
        kernel[0].revents = POLLIN;
        kernel[1].revents = POLLHUP;
        assert_eq!(pollout(&kernel, upl), Ok(()));
        let mut back = [Pollfd::default(); 2];
        upl.copyin_slice(&mut back).unwrap();
        assert_eq!(
            back,
            [
                Pollfd {
                    fd: 7,
                    events: POLLIN,
                    revents: POLLIN
                },
                Pollfd {
                    fd: 8,
                    events: POLLOUT,
                    revents: POLLHUP
                }
            ]
        );
    }

    #[test]
    fn poll_reports_ready_objects_and_a_fifo_s_write_as_its_hang_up_check() {
        let p = setup(54);
        let (a, oa) = obj(KlistKind::Kernel);
        let (b, ob) = obj(KlistKind::Own);
        let (c, oc) = obj(KlistKind::Kernel);
        oc.fifo.store(true, core::sync::atomic::Ordering::Relaxed);
        let (fa, fb, fc) = (open_obj(p, a), open_obj(p, b), open_obj(p, c));
        oa.ready.store(true, core::sync::atomic::Ordering::Relaxed);
        let upl = UserPtr::<Pollfd>::new(user(1));
        let set = [
            pfd(fa, POLLIN),
            pfd(fb, POLLIN),
            pfd(fc, POLLOUT),
            pfd(-1, POLLIN),
            pfd(250, POLLIN),
        ];
        upl.copyout_slice(&set).unwrap();
        // fa is ready; fb is not; fc's write is EPERM, retried as EXCEPT (registered, not
        // ready); -1 skipped; 250 is not open: POLLNVAL.
        assert_eq!(sys_poll(p, upl, 5, 0), Ok(2));
        let mut back = [Pollfd::default(); 5];
        upl.copyin_slice(&mut back).unwrap();
        let revents: Vec<i16> = back.iter().map(|x| x.revents).collect();
        assert_eq!(revents, [POLLIN, 0, 0, 0, POLLNVAL]);
        // Ready, then posted: the knote of an earlier call is reused by pollid, not doubled.
        ob.ready.store(true, core::sync::atomic::Ordering::Relaxed);
        assert_eq!(sys_poll(p, upl, 5, 0), Ok(3));
        // Errnos: a negative timeout; more pollfds than the descriptor limit.
        assert_eq!(sys_poll(p, upl, 1, -2), Err(Errno::EINVAL));
        assert_eq!(sys_poll(p, upl, 513, 0), Err(Errno::EINVAL));
        // Nothing to register and nothing collected: a zero timeout returns at once.
        upl.copyout_slice(&[pfd(-1, POLLIN)]).unwrap();
        assert_eq!(sys_poll(p, upl, 1, 0), Ok(0));
        // A user array that is not mapped: EFAULT.
        assert_eq!(sys_poll(p, UserPtr::new(16), 1, 0), Err(Errno::EFAULT));
        for fd in [fa, fb, fc] {
            close_fd(p, fd);
        }
        crate::event::kqpoll_exit(p);
    }

    #[test]
    fn select_reports_ready_descriptors_and_its_errnos() {
        let p = setup(55);
        let (a, oa) = obj(KlistKind::Kernel);
        let (b, _ob) = obj(KlistKind::Kernel);
        let fa = open_obj(p, a);
        let fb = open_obj(p, b);
        oa.ready.store(true, core::sync::atomic::Ordering::Relaxed);
        let r = UserPtr::<FdMask>::new(user(2));
        let w = UserPtr::<FdMask>::new(user(2) + 64);
        let nd = fb.max(fa) + 1;
        r.copyout(&((1 << fa) | (1 << fb))).unwrap();
        w.copyout(&0).unwrap();
        let none = UserPtr::new(0);
        let ts = UserPtr::<[i64; 2]>::new(user(2) + 128);
        ts.copyout(&[0, 0]).unwrap();
        assert_eq!(sys_pselect(p, nd, [r, w, none], ts, None), Ok(1));
        assert_eq!(r.copyin(), Ok(1 << fa), "only the ready one stays set");
        assert_eq!(w.copyin(), Ok(0));
        // A signal mask is installed at the C's point, before the registrations.
        let installed = core::cell::Cell::new(false);
        r.copyout(&(1 << fa)).unwrap();
        let mask = || installed.set(true);
        assert_eq!(sys_pselect(p, nd, [r, none, none], ts, Some(&mask)), Ok(1));
        assert!(installed.get());
        // Errnos: nd < 0, a bad timeval, a bad timespec, an unmapped set; a closed
        // descriptor in the set is EBADF.
        assert_eq!(dopselect(p, -1, [none; 3], None, None), Err(Errno::EINVAL));
        let tv = UserPtr::<[i64; 2]>::new(user(2) + 192);
        tv.copyout(&[0, 1_000_000]).unwrap();
        assert_eq!(sys_select(p, nd, [r, none, none], tv), Err(Errno::EINVAL));
        ts.copyout(&[-1, 0]).unwrap();
        assert_eq!(
            sys_pselect(p, nd, [r, none, none], ts, None),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            dopselect(
                p,
                nd,
                [UserPtr::new(16), none, none],
                Some(Timespec::ZERO),
                None
            ),
            Err(Errno::EFAULT)
        );
        r.copyout(&(1 << 9)).unwrap();
        assert_eq!(
            dopselect(p, 10, [r, none, none], Some(Timespec::ZERO), None),
            Err(Errno::EBADF)
        );
        close_fd(p, fa);
        close_fd(p, fb);
        crate::event::kqpoll_exit(p);
    }

    #[test]
    fn selwakeup_activates_the_knotes_of_a_selinfo() {
        let p = setup(56);
        let (a, oa) = obj(KlistKind::Kernel);
        let fa = open_obj(p, a);
        let (_kfd, kq) = crate::event::tests::new_kqueue(p);
        let mut kev = ev_set(fa as usize, EVFILT_READ, EV_ADD, 0, 0, 0);
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert_eq!(kq.count(), 0);
        oa.ready.store(true, core::sync::atomic::Ordering::Relaxed);
        selwakeup(&oa.sel);
        assert_eq!(kq.count(), 1);
        close_fd(p, fa);
        assert_eq!(kq.count(), 0, "knote_fdclose dropped the queued knote");
    }
}
/* </TESTS> */
