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
/* </LICENSES> */

/* <CODE> */
//! `sys_generic.c`: the generic file system calls: `read`, `write`, `ioctl`, `select`,
//! `poll`.
//!
//! Upstream: sys/kern/sys_generic.c @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M6 (part b) ported `sys_write` for the console; M7b
//! (`kern_descrip.c`) made the read and write paths real. M8: `sys_select`, `sys_pselect`,
//! `dopselect`, `pselregister`, `pselcollect`, `selwakeup`, `pollout`, `sys_poll`,
//! `sys_ppoll`, `doppoll`, `ppollregister_evts`, `ppollregister`, `ppollcollect` and
//! `sys_utrace`. N4.3: `iovec_copyin`, `dofilereadv`, `dofilewritev`, the bodies of
//! `sys_read`, `sys_readv`, `sys_write`, `sys_writev`, `sys_ioctl` and `sys_utrace` moved
//! into `kernel::descrip` (and `kernel::uio`); the `sysent[]` rows here call them.
//! `select(2)`/`poll(2)` stay until kqueue moves (N4.4).
//!
//! ## Deviations
//! - `KTRACE` is not configured.
//! - `select(2)`/`poll(2)` are OpenBSD's, built on the thread's poll kqueue
//!   (`kern_event.rs`): `kqpoll_init` and `kqpoll_done` take the thread, and `kqpoll_init`
//!   can fail (`ENOMEM`, the kqueue pool cannot sleep yet), which fails the call. The fd
//!   sets and the pollfd array are one allocation each (the C keeps small ones on its
//!   stack).

use core::sync::atomic::Ordering;

use hal::user::UserPtr;

use crate::conf::param::MAXFILES;
use crate::kassert;
use crate::kern::kern_event::{
    knote_locked, kqpoll_done, kqpoll_init, kqueue_register, kqueue_scan, kqueue_scan_finish,
    kqueue_scan_setup,
};
use crate::kern::kern_sig::dosigsuspend;
use crate::kern::kern_synch::{nowake, tsleep_nsec};
use crate::kern::kern_time::ratecheck;
use crate::kern::subr_prf::Str;
use crate::kprintf;
use crate::machine::copy::copyin_obj;
use crate::machine::copy::{copyin, copyout};
use crate::sys::errno::Errno;
use crate::sys::event::{
    __EV_HUP, __EV_POLL, __EV_SELECT, EV_ADD, EV_ENABLE, EV_ERROR, EVFILT_EXCEPT, EVFILT_READ,
    EVFILT_WRITE, Kevent, KqueueScanState, NOTE_OOB, NOTE_SUBMIT, ev_set,
};
use crate::sys::eventvar::KQ_NEVENTS;
use crate::sys::param::{PCATCH, PSOCK};
use crate::sys::poll::{
    INFTIM, POLL_NOHUP, POLLERR, POLLHUP, POLLIN, POLLNVAL, POLLOUT, POLLPRI, POLLRDBAND,
    POLLRDNORM, POLLWRNORM, Pollfd,
};
use crate::sys::proc::Proc;
use crate::sys::resource::RLIMIT_NOFILE;
use crate::sys::resourcevar::lim_cur;
use crate::sys::select::{FdMask, NFDBITS, fd_set, howmany};
use crate::sys::selinfo::Selinfo;
use crate::sys::signal::Sigset;
use crate::sys::signalvar::sigcantmask;
use crate::sys::syscallargs::{
    SysIoctlArgs, SysReadArgs, SysReadvArgs, SysWriteArgs, SysWritevArgs,
};
use crate::sys::syscallargs::{SysPollArgs, SysPpollArgs, SysPselectArgs, SysSelectArgs};
use crate::sys::systm::{INFSLP, MAXTSLP};
use crate::sys::systm::{SysArgs, kernel_lock, kernel_unlock, sysargs};
use crate::sys::time::{Timespec, Timeval, timespec_to_nsec, timeval_to_timespec};
use crate::sys::types::Register;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::offset_of;
use libkern::StaticCell;

/// Read system call (`sysent[]`'s row: `kernel::descrip::sys_read`).
pub fn sys_read(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysReadArgs = sysargs(v);
    let n = kernel::descrip::sys_read(p, uap.fd.get(), uap.buf.get().addr(), uap.nbyte.get())?;
    retval[0] = n as Register;
    Ok(())
}

/// Scatter read system call (`sysent[]`'s row: `kernel::descrip::sys_readv`).
pub fn sys_readv(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysReadvArgs = sysargs(v);
    let n = kernel::descrip::sys_readv(
        p,
        uap.fd.get(),
        UserPtr::new(uap.iovp.get().addr()),
        uap.iovcnt.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Write system call (`sysent[]`'s row: `kernel::descrip::sys_write`).
pub fn sys_write(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysWriteArgs = sysargs(v);
    let n = kernel::descrip::sys_write(p, uap.fd.get(), uap.buf.get().addr(), uap.nbyte.get())?;
    retval[0] = n as Register;
    Ok(())
}

/// Gather write system call (`sysent[]`'s row: `kernel::descrip::sys_writev`).
pub fn sys_writev(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysWritevArgs = sysargs(v);
    let n = kernel::descrip::sys_writev(
        p,
        uap.fd.get(),
        UserPtr::new(uap.iovp.get().addr()),
        uap.iovcnt.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Ioctl system call (`sysent[]`'s row: `kernel::descrip::sys_ioctl`).
pub fn sys_ioctl(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysIoctlArgs = sysargs(v);
    kernel::descrip::sys_ioctl(p, uap.fd.get(), uap.com.get(), uap.data.get().addr())
}

/// Select system call.
pub fn sys_select(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSelectArgs = sysargs(v);

    let mut tsp = None;
    let utv = uap.tv.get() as usize;
    if utv != 0 {
        let tv: Timeval = copyin_obj(utv)?;
        // KTRACE: not configured.
        if tv.tv_sec < 0 || !tv.is_valid() {
            return Err(Errno::EINVAL);
        }
        tsp = Some(timeval_to_timespec(&tv));
    }

    dopselect(
        p,
        uap.nd.get(),
        [
            uap.r#in.get() as usize,
            uap.ou.get() as usize,
            uap.ex.get() as usize,
        ],
        tsp,
        None,
        retval,
    )
}

/// `pselect(2)`.
pub fn sys_pselect(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPselectArgs = sysargs(v);

    let mut tsp = None;
    let uts = uap.ts.get() as usize;
    if uts != 0 {
        let ts: Timespec = copyin_obj(uts)?;
        // KTRACE: not configured.
        if ts.tv_sec < 0 || !ts.is_valid() {
            return Err(Errno::EINVAL);
        }
        tsp = Some(ts);
    }
    let mut ssp = None;
    let umask = uap.mask.get() as usize;
    if umask != 0 {
        let mut b = [0u8; size_of::<Sigset>()];
        copyin(umask, &mut b)?;
        ssp = Some(Sigset::from_ne_bytes(b));
    }

    dopselect(
        p,
        uap.nd.get(),
        [
            uap.r#in.get() as usize,
            uap.ou.get() as usize,
            uap.ex.get() as usize,
        ],
        tsp,
        ssp,
        retval,
    )
}

/// `dopselect`: the body of `select(2)` and `pselect(2)` over the user fd sets `sets` (in,
/// out, except; 0 for NULL).
pub fn dopselect(
    p: &Proc,
    nd: i32,
    sets: [usize; 3],
    timeout: Option<Timespec>,
    sigmask: Option<Sigset>,
    retval: &mut [Register; 2],
) -> Result<(), Errno> {
    let mut timeout = timeout;
    let mut ncollected = 0usize;
    let mut nevents;

    if nd < 0 {
        return Err(Errno::EINVAL);
    }

    let nfiles = p.fd().nfiles() as i32;
    let nd = nd.min(nfiles) as usize;

    // ni bytes per set; the six sets (three in, three out) are one zeroed allocation (the C
    // keeps up to one mask each on its stack).
    let words = howmany(nd, NFDBITS);
    let ni = words * size_of::<FdMask>();
    let mut bits: Vec<FdMask> = vec![0; 6 * words];
    let (pibits, pobits) = bits.split_at_mut(3 * words);

    kqpoll_init(p, nd as u32)?;

    let error = 'done: {
        // getbits
        for (x, &name) in sets.iter().enumerate() {
            if name != 0 && ni != 0 {
                let mut raw = vec![0u8; ni];
                if let Err(e) = copyin(name, &mut raw) {
                    break 'done Err(e);
                }
                for (w, chunk) in pibits[x * words..(x + 1) * words]
                    .iter_mut()
                    .zip(raw.as_chunks::<{ size_of::<FdMask>() }>().0)
                {
                    *w = FdMask::from_ne_bytes(*chunk);
                }
            }
        }
        // KTRACE: not configured.

        if let Some(mask) = sigmask {
            dosigsuspend(p, mask & !sigcantmask());
        }

        // Register kqueue events
        match pselregister(p, pibits, words, nd) {
            Ok(n) => nevents = n,
            Err(e) => break 'done Err(e),
        }

        // The poll/select family of syscalls has been designed to block when file
        // descriptors are not available, even if there's nothing to wait for.
        if nevents == 0 && ncollected == 0 {
            let mut nsecs = INFSLP;

            if let Some(ts) = timeout {
                if !ts.is_set() {
                    break 'done Ok(());
                }
                nsecs = timespec_to_nsec(&ts).clamp(1, MAXTSLP);
            }
            let error = tsleep_nsec(nowake(), PSOCK | PCATCH, "kqsel", nsecs);
            // select is not restarted after signals...
            break 'done match error {
                Err(Errno::ERESTART) => Err(Errno::EINTR),
                Err(Errno::EWOULDBLOCK) => Ok(()),
                other => other,
            };
        }

        // Do not block if registering found pending events.
        if ncollected > 0 {
            timeout = Some(Timespec::new(0, 0));
        }

        // Collect at most `nevents' possibly waiting in kqueue_scan()
        let scan = KqueueScanState::new();
        // SAFETY: `scan` is a local that stays in place until `kqueue_scan_finish` below.
        unsafe { kqueue_scan_setup(&scan, p.kq()) };
        let mut error = Ok(());
        while nevents > 0 {
            let mut kev: [Kevent; KQ_NEVENTS] = [Kevent::default(); KQ_NEVENTS];
            // Maximum number of events per iteration
            let count = kev.len().min(nevents);
            let ready = kqueue_scan(&scan, count, &mut kev, timeout.as_mut(), p, &mut error);

            // Convert back events that are ready.
            let mut i = 0;
            while i < ready && error.is_ok() {
                error = pselcollect(p, &kev[i], pobits, words, &mut ncollected);
                i += 1;
            }
            // Stop if there was an error or if we had enough space to collect all events
            // that were ready.
            if error.is_err() || ready < count {
                break;
            }

            nevents -= ready;
        }
        kqueue_scan_finish(&scan);
        retval[0] = ncollected as Register;
        error
    };

    // done: putbits
    let mut error = error;
    if error.is_ok() {
        for (x, &name) in sets.iter().enumerate() {
            if name != 0 && ni != 0 {
                let mut raw = vec![0u8; ni];
                for (w, chunk) in pobits[x * words..(x + 1) * words]
                    .iter()
                    .zip(raw.as_chunks_mut::<{ size_of::<FdMask>() }>().0)
                {
                    chunk.copy_from_slice(&w.to_ne_bytes());
                }
                if let Err(error2) = copyout(&raw, name) {
                    error = Err(error2);
                }
            }
        }
        // KTRACE: not configured.
    }

    kqpoll_done(p, nd as u32);

    error
}

/// Convert fd_set into kqueue events and register them on the per-thread queue; returns how
/// many were registered. `pibits` holds the three input sets, `words` masks each.
pub fn pselregister(p: &Proc, pibits: &[FdMask], words: usize, nfd: usize) -> Result<usize, Errno> {
    const EVF: [i16; 3] = [EVFILT_READ, EVFILT_WRITE, EVFILT_EXCEPT];
    const EVFF: [u32; 3] = [0, 0, NOTE_OOB];
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
                match kqueue_register(p.kq(), &mut kev, 0, Some(p)) {
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
pub fn pselcollect(
    p: &Proc,
    kevp: &Kevent,
    pobits: &mut [FdMask],
    words: usize,
    ncollected: &mut usize,
) -> Result<(), Errno> {
    if kevp.udata as u64 != p.p_kq_serial.get() {
        crate::kern::subr_prf::panic(format_args!(
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

/// `selwakeup`: do a wakeup when a selectable event occurs.
pub fn selwakeup(sip: &Selinfo) {
    kernel_lock();
    knote_locked(&sip.si_note, i64::from(NOTE_SUBMIT));
    kernel_unlock();
}

/// Only copyout the revents field.
pub fn pollout(pl: &[Pollfd], upl: usize) -> Result<(), Errno> {
    for (i, pfd) in pl.iter().enumerate() {
        copyout(
            &pfd.revents.to_ne_bytes(),
            upl + i * size_of::<Pollfd>() + offset_of!(Pollfd, revents),
        )?;
    }
    Ok(())
}

/// We are using the same mechanism as select only we encode/decode args differently.
pub fn sys_poll(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPollArgs = sysargs(v);

    let mut tsp = None;
    let msec = uap.timeout.get();

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

    doppoll(p, uap.fds.get() as usize, uap.nfds.get(), tsp, None, retval)
}

/// `ppoll(2)`.
pub fn sys_ppoll(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPpollArgs = sysargs(v);

    let mut tsp = None;
    let uts = uap.ts.get() as usize;
    if uts != 0 {
        let ts: Timespec = copyin_obj(uts)?;
        // KTRACE: not configured.
        if ts.tv_sec < 0 || !ts.is_valid() {
            return Err(Errno::EINVAL);
        }
        tsp = Some(ts);
    }

    let mut ssp = None;
    let umask = uap.mask.get() as usize;
    if umask != 0 {
        let mut b = [0u8; size_of::<Sigset>()];
        copyin(umask, &mut b)?;
        ssp = Some(Sigset::from_ne_bytes(b));
    }

    doppoll(p, uap.fds.get() as usize, uap.nfds.get(), tsp, ssp, retval)
}

/// `doppoll`: the body of `poll(2)` and `ppoll(2)` over the user array `fds`.
pub fn doppoll(
    p: &Proc,
    fds: usize,
    nfds: u32,
    timeout: Option<Timespec>,
    sigmask: Option<Sigset>,
    retval: &mut [Register; 2],
) -> Result<(), Errno> {
    let mut timeout = timeout;
    let mut ncollected = 0usize;
    let mut nevents = 0usize;

    // Standards say no more than MAX_OPEN; this is possibly better.
    let limit = (lim_cur(RLIMIT_NOFILE) as i64).min(i64::from(MAXFILES.load(Ordering::Relaxed)));
    if i64::from(nfds) > limit {
        return Err(Errno::EINVAL);
    }

    // The array (the C keeps up to four on its stack and mallocs M_CANFAIL beyond).
    let mut pl: Vec<Pollfd> = vec![Pollfd::default(); nfds as usize];

    kqpoll_init(p, nfds)?;

    let sz = nfds as usize * size_of::<Pollfd>();
    let mut raw = vec![0u8; sz];
    if let Err(error) = copyin(fds, &mut raw) {
        // bad:
        kqpoll_done(p, nfds);
        return Err(error);
    }
    for (pfd, chunk) in pl
        .iter_mut()
        .zip(raw.as_chunks::<{ size_of::<Pollfd>() }>().0)
    {
        let word = |o: usize| {
            let mut b = [0u8; 2];
            b.copy_from_slice(&chunk[o..o + 2]);
            i16::from_ne_bytes(b)
        };
        let mut fd = [0u8; 4];
        fd.copy_from_slice(&chunk[0..4]);
        *pfd = Pollfd {
            fd: i32::from_ne_bytes(fd),
            events: word(offset_of!(Pollfd, events)),
            revents: word(offset_of!(Pollfd, revents)),
        };
    }

    if let Some(mask) = sigmask {
        dosigsuspend(p, mask & !sigcantmask());
    }

    // Register kqueue events
    ppollregister(p, &mut pl, &mut nevents, &mut ncollected);

    let error = 'done: {
        // The poll/select family of syscalls has been designed to block when file
        // descriptors are not available, even if there's nothing to wait for.
        if nevents == 0 && ncollected == 0 {
            let mut nsecs = INFSLP;

            if let Some(ts) = timeout {
                if !ts.is_set() {
                    break 'done Ok(());
                }
                nsecs = timespec_to_nsec(&ts).clamp(1, MAXTSLP);
            }

            let error = tsleep_nsec(nowake(), PSOCK | PCATCH, "kqpoll", nsecs);
            break 'done match error {
                Err(Errno::ERESTART) => Err(Errno::EINTR),
                Err(Errno::EWOULDBLOCK) => Ok(()),
                other => other,
            };
        }

        // Do not block if registering found pending events.
        if ncollected > 0 {
            timeout = Some(Timespec::new(0, 0));
        }

        // Collect at most `nevents' possibly waiting in kqueue_scan()
        let scan = KqueueScanState::new();
        // SAFETY: `scan` is a local that stays in place until `kqueue_scan_finish` below.
        unsafe { kqueue_scan_setup(&scan, p.kq()) };
        let mut error = Ok(());
        while nevents > 0 {
            let mut kev: [Kevent; KQ_NEVENTS] = [Kevent::default(); KQ_NEVENTS];
            // Maximum number of events per iteration
            let count = kev.len().min(nevents);
            let ready = kqueue_scan(&scan, count, &mut kev, timeout.as_mut(), p, &mut error);

            // Convert back events that are ready.
            for k in &kev[..ready] {
                ncollected += ppollcollect(p, k, &mut pl);
            }

            // Stop if there was an error or if we had enough place to collect all events
            // that were ready.
            if error.is_err() || ready < count {
                break;
            }

            nevents -= ready;
        }
        kqueue_scan_finish(&scan);
        retval[0] = ncollected as Register;
        error
    };

    // done: NOTE: poll(2) is not restarted after a signal and EWOULDBLOCK is ignored (since
    // the whole point is to see what would block).
    let error = match error {
        Err(Errno::EINTR) => pollout(&pl, fds).and(Err(Errno::EINTR)),
        Err(Errno::EWOULDBLOCK) | Ok(()) => pollout(&pl, fds),
        other => other,
    };
    // KTRACE: not configured.

    kqpoll_done(p, nfds);

    error
}

/// `ppollregister_evts`: registers the `kev` events of one pollfd; failures become its
/// `revents`. Returns how many were registered.
pub fn ppollregister_evts(p: &Proc, kev: &mut [Kevent], pl: &mut Pollfd, pollid: u32) -> usize {
    let mut nevents = 0;
    let nkev = kev.len();

    kassert!(pl.revents == 0);

    for kevp in kev.iter_mut() {
        loop {
            match kqueue_register(p.kq(), kevp, pollid, Some(p)) {
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
pub fn ppollregister(p: &Proc, pl: &mut [Pollfd], nregistered: &mut usize, ncollected: &mut usize) {
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
        let udata = (p.p_kq_serial.get() + i as u64) as usize;
        if pfd.events & (POLLIN | POLLRDNORM) != 0 {
            kev[nkev] = ev_set(
                pfd.fd as usize,
                EVFILT_READ,
                EV_ADD | EV_ENABLE | __EV_POLL,
                0,
                0,
                udata,
            );
            nkev += 1;
        }
        if pfd.events & (POLLOUT | POLLWRNORM) != 0 {
            kev[nkev] = ev_set(
                pfd.fd as usize,
                EVFILT_WRITE,
                EV_ADD | EV_ENABLE | __EV_POLL,
                0,
                0,
                udata,
            );
            nkev += 1;
        }
        if pfd.events & (POLLPRI | POLLRDBAND) != 0 || forcehup {
            let evff = if forcehup { 0 } else { NOTE_OOB };

            kev[nkev] = ev_set(
                pfd.fd as usize,
                EVFILT_EXCEPT,
                EV_ADD | EV_ENABLE | __EV_POLL,
                evff,
                0,
                udata,
            );
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

/// Convert given kqueue event into corresponding poll(2) revents bit; 1 when it is the
/// first event of its pollfd.
pub fn ppollcollect(p: &Proc, kevp: &Kevent, pl: &mut [Pollfd]) -> usize {
    static POLL_LASTERR: StaticCell<Timeval> = StaticCell::new(Timeval::new(0, 0));
    const POLL_ERRINTVL: Timeval = Timeval::new(5, 0);

    // Extract poll array index
    let serial = p.p_kq_serial.get();
    let i = (kevp.udata as u64).wrapping_sub(serial) as usize;
    let nfds = pl.len();

    if i >= nfds {
        crate::kern::subr_prf::panic(format_args!(
            "ppollcollect: spurious kevp nfds {nfds} udata {:#x} serial {serial:#x}",
            kevp.udata
        ));
    }
    if kevp.ident as i32 != pl[i].fd {
        crate::kern::subr_prf::panic(format_args!(
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
        if kevp.flags & EV_ERROR != 0 && kevp.data == Errno::EBADF as i64 {
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
    if pfd.revents == 0 && {
        // The C reads and writes the static without a lock (a benign race); here the kernel
        // lock serialises the rate limiter, which `SY_NOLOCK` callers reach concurrently.
        kernel_lock();
        // SAFETY: the rate limiter is touched only here, under the kernel lock.
        let noisy = ratecheck(unsafe { POLL_LASTERR.get_mut() }, &POLL_ERRINTVL);
        kernel_unlock();
        noisy
    } {
        kprintf!(
            "{}[{}]: poll index {} fd {} events 0x{:x} filter {}/0x{:x} unclaimed\n",
            Str(&p.process().comm()),
            p.p_tid.get(),
            i,
            pfd.fd,
            pfd.events,
            kevp.filter,
            kevp.flags
        );
    }

    usize::from(!already_seen && pfd.revents != 0)
}

/// `utrace(2)`: a user record for the process's `ktrace(1)` trace. `KTRACE` is not
/// configured, so there is no trace to add to and the call succeeds, as the C's `#else`.
pub fn sys_utrace(curp: &Proc, _v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    // KTRACE: ktruser(curp, label, addr, len) (`kernel::descrip::sys_utrace`).
    kernel::descrip::sys_utrace(curp)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the `select(2)`/`poll(2)` conversions of `sys_generic.c`: the kevents a
    // pollfd turns into and what a failed registration leaves in `revents`, the events turned
    // back into `revents` and fd set bits, and `pollout` copying only `revents` out.

    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use super::*;

    /// A thread for the conversions: they only read its name and id.
    fn test_proc() -> &'static Proc {
        crate::kern::proc_glue::test_thread(crate::kern::proc_glue::test_process(1))
    }

    fn pfd(fd: i32, events: i16) -> Pollfd {
        Pollfd {
            fd,
            events,
            revents: 0,
        }
    }

    #[test]
    fn ppollregister_reports_failed_registrations() {
        let _g = crate::kern::kern_event::tests::setup();
        let p = crate::kern::kern_event::tests::thread();
        assert_eq!(kqpoll_init(p, 3), Ok(()));
        // No descriptor is open: every registration fails with EBADF, which poll(2) reports as
        // POLLNVAL; a negative fd is skipped.
        let mut pl = vec![pfd(0, POLLIN), pfd(-1, POLLIN), pfd(1, 0)];
        let (mut nregistered, mut ncollected) = (0, 0);
        ppollregister(p, &mut pl, &mut nregistered, &mut ncollected);
        assert_eq!(nregistered, 0);
        assert_eq!(ncollected, 2);
        assert_eq!(pl[0].revents, POLLNVAL);
        assert_eq!(pl[1].revents, 0);
        assert_eq!(pl[2].revents, POLLNVAL, "POLLHUP is always checked");
        kqpoll_done(p, 3);
        crate::kern::kern_event::kqpoll_exit(p);
    }

    #[test]
    fn ppollcollect_converts_events() {
        let p = test_proc();
        let mut pl = vec![pfd(3, POLLIN | POLLOUT), pfd(4, POLLPRI)];
        let read = ev_set(3, EVFILT_READ, 0, 0, 0, 0);
        assert_eq!(ppollcollect(p, &read, &mut pl), 1);
        assert_eq!(pl[0].revents, POLLIN);
        // a second event on the same pollfd is not counted again
        let write = ev_set(3, EVFILT_WRITE, 0, 0, 0, 0);
        assert_eq!(ppollcollect(p, &write, &mut pl), 0);
        assert_eq!(pl[0].revents, POLLIN | POLLOUT);
        // hang-up on the except filter
        let hup = ev_set(4, EVFILT_EXCEPT, __EV_HUP, 0, 0, 1);
        assert_eq!(ppollcollect(p, &hup, &mut pl), 1);
        assert_eq!(pl[1].revents, POLLHUP);
        // EBADF preempts everything
        let bad = ev_set(4, EVFILT_EXCEPT, EV_ERROR, 0, Errno::EBADF as i64, 1);
        assert_eq!(ppollcollect(p, &bad, &mut pl), 0);
        assert_eq!(pl[1].revents, POLLNVAL);
    }

    #[test]
    fn pselcollect_sets_bits() {
        let p = test_proc();
        let words = howmany(40, NFDBITS);
        let mut pobits = vec![0 as FdMask; 3 * words];
        let mut n = 0;
        assert_eq!(
            pselcollect(
                p,
                &ev_set(33, EVFILT_WRITE, 0, 0, 0, 0),
                &mut pobits,
                words,
                &mut n
            ),
            Ok(())
        );
        assert_eq!(n, 1);
        assert_eq!(pobits[words + 1], 1 << 1);
        let err = ev_set(2, EVFILT_READ, EV_ERROR, 0, Errno::EBADF as i64, 0);
        assert_eq!(
            pselcollect(p, &err, &mut pobits, words, &mut n),
            Err(Errno::EBADF)
        );
    }

    #[test]
    fn pollout_copies_only_revents() {
        let mut user: Vec<Pollfd> = vec![pfd(7, POLLIN), pfd(8, POLLOUT)];
        let mut kernel = user.clone();
        kernel[0].revents = POLLIN;
        kernel[1].revents = POLLHUP;
        kernel[1].fd = 99; // not copied
        assert_eq!(pollout(&kernel, user.as_mut_ptr() as usize), Ok(()));
        assert_eq!(
            user[0],
            Pollfd {
                fd: 7,
                events: POLLIN,
                revents: POLLIN
            }
        );
        assert_eq!(
            user[1],
            Pollfd {
                fd: 8,
                events: POLLOUT,
                revents: POLLHUP
            }
        );
        assert!(size_of::<Pollfd>() == 8);
    }
}
/* </TESTS> */
