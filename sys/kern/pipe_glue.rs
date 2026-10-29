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
 * Copyright (c) 1996 John S. Dyson
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice immediately at the beginning of the file, without modification,
 *    this list of conditions, and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Absolutely no warranty of function or purpose is made by the author
 *    John S. Dyson.
 * 4. Modifications may be freely made to this file if the above conditions
 *    are met.
 */
/* </LICENSES> */

/* <CODE> */
//! The pipe's glue to what is still `bsd`'s: the call-site adapter of `kernel::pipe`
//! (`sys/kernel/pipe.rs`), with 0 `unsafe`, while the file's operations (`struct fileops`,
//! `FileGlue::Ops`) and object (`FileGlue::Data`) are `bsd`'s (N8).
//!
//! Upstream: sys/kern/sys_pipe.c @ 3ce1f3f79392
//! LZ: sys/kern/sys_pipe.rs@3c62ede68802
//!
//! Every decision pipe(2) makes is the kernel crate's; this file holds the `struct fileops`
//! (`PIPEOPS`) of a pipe file, which turns `bsd`'s `File`, `Uio`, ioctl numbers and `Stat`
//! into the kernel's types; the system call entries `bsd`'s `sysent[]` keeps (the `Legacy`
//! path, which calls the same kernel function as the module's `Native` entry); and
//! `pipe_init`. N5.0: the clock and the three `struct filterops` are the kernel's
//! (`kernel::pipe::PipeRead`, `PipeWrite`, `PipeExcept`, which find the pair through
//! `BsdFileGlue::pipe_of`), and so is `dopipe` (`kernel::pipe::Pipes::dopipe` over the
//! kernel's descriptor table, a file set up by `BsdFileGlue::pipe_file` with [`PIPEOPS`]);
//! the `PipeGlue` and `PipeFiles` bridges are gone.
//!
//! A pipe file's object is its pipe (`BsdFileData::Pipe`): the [`PipeEnd`] the kernel made
//! for it. Every file operation clones the pair's handle and reads which pipe it is
//! ([`pipe_of`]), never calling into the pair while it reads the file's object. `fo_close` takes the end out under the mutex, releases it, and
//! only then drops the end, whose drop is `pipe_destroy` (it sleeps on `PIPE_WANTD`, which a
//! spinning mutex must not be held across).
//!
//! A pipe's klist is the kernel's, by value in the pair (`kernel::pipe`, N4.4), and so is its
//! sigio registration (`kernel::sigio::SigioRef`, N4.5b: one atomic word, `Send` and `Sync`).
//!
//! ## Deviations
//! - `fo_ioctl`'s `data` is the kernel copy of the argument (`sys_ioctl`), at least an `int`
//!   wide; the `int` is read and written in native byte order (LZ's).
//! - `KTRACE` is not configured (`ktrfds`), as in LZ.
//!
//! ## Redesign
//! - A call-site adapter, listed as such (docs/ROADMAP.md N2): LZ's `sys_pipe.rs` minus every
//!   decision, which moved to `sys/kernel/pipe.rs`. The `fileops` keep their names and
//!   signatures; each finds the pair through [`pipe_of`] (LZ's `fp_pipe`, a cast of `f_data`)
//!   and calls one kernel method. The `filterops` and their functions are the kernel's since
//!   N5.0 (`kernel::pipe`, the same names).
//! - A knote's `kn_hook` holds the index of the pipe whose klist it is on (`Dir::index`, as
//!   an address without provenance, set by the kernel's `kqfilter`), not a pointer (LZ's
//!   `kn_pipe` cast it back; the kernel's `filt_pipedetach` reads it).
//! - `pipe_stat`'s `st_uid`/`st_gid` are the file's `f_cred` (`File::cred`, N5.0), as the
//!   C's; N4 copied them into the pipe's slot when `dopipe` made the file.

use hal::pool::PoolArc;
use hal::user::UserPtr;
use kernel::pipe::{Dir, PipeEnd, PipeFilter, PipeIoctl, PipeKinfo, PipePair, Pipes, UioMove};

use crate::kern::init_sysent::SYSENT;
use crate::kern::kern_descrip::BsdFileData;
use crate::kern::kern_subr::uiomove;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::machine::cpu::curproc;
use crate::machine::intr::IPL_MPFLOOR;
use crate::sys::errno::Errno;
use crate::sys::event::{__EV_POLL, __EV_SELECT, EVFILT_EXCEPT, EVFILT_READ, EVFILT_WRITE, Knote};
use crate::sys::fcntl::FNONBLOCK;
use crate::sys::file::{File, Fileops};
use crate::sys::filio::{FIOASYNC, FIOGETOWN, FIONREAD, FIOSETOWN};
use crate::sys::proc::Proc;
use crate::sys::sockio::{SIOCGPGRP, SIOCSPGRP};
use crate::sys::stat::{S_IFIFO, Stat};
use crate::sys::syscall::{SYS_pipe, SYS_pipe2};
use crate::sys::syscall_mi::mi_syscall;
use crate::sys::syscallargs::{SysPipe2Args, SysPipeArgs};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::time::Timespec;
use crate::sys::ttycom::{TIOCGPGRP, TIOCSPGRP};
use crate::sys::types::Register;
use crate::sys::uio::Uio;

/// A `Uio` as the kernel's pipe moves bytes through it, with the file it is on (whose
/// `FNONBLOCK` the pipe reads at every check).
struct UioAdapter<'a, 'b> {
    uio: &'a mut Uio<'b>,
    fp: &'a File,
}

/// `pipeops`: the interfaces to the outside world.
pub static PIPEOPS: Fileops = Fileops {
    fo_read: pipe_read,
    fo_write: pipe_write,
    fo_ioctl: pipe_ioctl,
    fo_kqfilter: pipe_kqfilter,
    fo_stat: pipe_stat,
    fo_close: pipe_close,
    fo_seek: None,
};

/// The pipe buffers' pages: the kernel's `km_alloc` (`kernel::vm::km`, `kv_any` with
/// `kp_pageable`).
#[cfg(not(test))]
const PIPE_PAGES: &dyn hal::kmbuf::PageMemory = kernel::vm::km::page_memory();
/// The host tests' pages: the kernel crate's double (the host has no MMU, so `km_alloc`
/// refuses pageable memory there, as LZ's did).
#[cfg(test)]
const PIPE_PAGES: &dyn hal::kmbuf::PageMemory = kernel::testutil::page_memory();

/// The pipes: `pipe_pair_pool`, `nbigpipe`, `amountpipekva`, over this glue.
pub static PIPES: Pipes<BsdProcGlue> = Pipes::new(PIPE_PAGES);

impl UioMove for UioAdapter<'_, '_> {
    fn resid(&self) -> usize {
        self.uio.uio_resid
    }

    fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
        uiomove(buf, self.uio)
    }

    fn nonblock(&self) -> bool {
        self.fp.flag() & FNONBLOCK != 0
    }

    fn is_write(&self) -> bool {
        self.uio.uio_rw == crate::sys::uio::UioRw::UIO_WRITE
    }

    fn set_resid(&mut self, resid: usize) {
        self.uio.uio_resid = resid;
    }
}

/// A pipe file's object (`f_data`): its end.
fn pipe_file_of(fp: &File) -> Option<&PipeEnd<BsdProcGlue>> {
    match fp.data() {
        Some(BsdFileData::Pipe(end)) => Some(end),
        _ => None,
    }
}

/// The pair and the pipe of a pipe file (the pair counted again: a filter's file is a
/// temporary handle).
pub fn pipe_of(fp: &File) -> (PoolArc<PipePair<BsdProcGlue>>, Dir) {
    match pipe_file_of(fp) {
        Some(end) => (end.pair().clone(), end.dir()),
        None => panic(format_args!("file {:#x}: not an open pipe", fp.addr())),
    }
}

/// `kern_sysctl`'s `fill_file` of a pipe file.
pub fn pipe_kinfo(fp: &File) -> Option<PipeKinfo> {
    pipe_file_of(fp).map(|end| end.pair().kinfo(end.dir()))
}

/// The int an `ioctl` argument holds.
fn int_arg(data: &[u8]) -> i32 {
    data.first_chunk::<4>()
        .map_or(0, |b| i32::from_ne_bytes(*b))
}

/// Stores an int into an `ioctl` argument.
fn set_int_arg(data: &mut [u8], v: i32) {
    if let Some(b) = data.first_chunk_mut::<4>() {
        *b = v.to_ne_bytes();
    }
}

/// The pipe system call for the `DTYPE_PIPE` type of pipes (`sysent[]`'s entry).
pub fn sys_pipe(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPipeArgs = sysargs(v);

    PIPES.sys_pipe(p, UserPtr::new(uap.fdp.get() as usize))
}

/// `pipe2(2)`: `pipe(2)` with `O_CLOEXEC`, `O_CLOFORK` and `O_NONBLOCK` (`sysent[]`'s
/// entry).
pub fn sys_pipe2(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPipe2Args = sysargs(v);

    PIPES.sys_pipe2(p, UserPtr::new(uap.fdp.get() as usize), uap.flags.get())
}

/// `KernelApi::pipe2`, the module's `Native` pipe(2) and pipe2(2): the thread on this CPU
/// runs `mi_syscall` on its own `sysent[]` row, so the system call prologue that is still
/// `bsd`'s (`refreshcreds`, the pinned syscalls' check, pledge, the kernel lock rule) runs
/// exactly as on the `Legacy` path, then the row's [`sys_pipe`] or [`sys_pipe2`], which call
/// the kernel crate. The two paths cannot diverge. A number that is neither is `ENOSYS`.
pub fn native_pipe2(nr: Register, fdp: usize, flags: Register) -> Result<(), i32> {
    let Some(p) = curproc() else {
        panic(format_args!("pipe2: no curproc"));
    };
    if nr != SYS_pipe as Register && nr != SYS_pipe2 as Register {
        return Err(Errno::ENOSYS.as_i32());
    }
    let args: SysArgs = [fdp as Register, flags, 0, 0, 0, 0];
    let mut rval: [Register; 2] = [0, 0];
    mi_syscall(p, nr, &SYSENT[nr as usize], &args, &mut rval).map_err(Errno::as_i32)
}

/// `fo_read` of a pipe.
pub fn pipe_read(fp: &File, uio: &mut Uio<'_>, _fflags: i32) -> Result<(), Errno> {
    let (pair, dir) = pipe_of(fp);
    pair.read(dir, &mut UioAdapter { uio, fp })
}

/// `fo_write` of a pipe.
pub fn pipe_write(fp: &File, uio: &mut Uio<'_>, _fflags: i32) -> Result<(), Errno> {
    let (pair, dir) = pipe_of(fp);
    pair.write(dir, &mut UioAdapter { uio, fp })
}

/// We implement a very minimal set of ioctls for compatibility with sockets: `bsd`'s
/// numbers as the kernel's commands.
pub fn pipe_ioctl(fp: &File, cmd: u64, data: &mut [u8], _p: &Proc) -> Result<(), Errno> {
    let (pair, dir) = pipe_of(fp);
    let what = match cmd {
        FIOASYNC => PipeIoctl::Async(int_arg(data) != 0),
        FIONREAD => PipeIoctl::Nread,
        FIOSETOWN | SIOCSPGRP | TIOCSPGRP => PipeIoctl::SetOwn {
            cmd,
            arg: int_arg(data),
        },
        FIOGETOWN | SIOCGPGRP | TIOCGPGRP => PipeIoctl::GetOwn { cmd },
        _ => PipeIoctl::Other,
    };
    if let Some(v) = pair.ioctl(dir, what)? {
        set_int_arg(data, v);
    }
    Ok(())
}

/// `fo_stat` of a pipe: a FIFO whose size is what is buffered.
pub fn pipe_stat(fp: &File, ub: &mut Stat, _p: &Proc) -> Result<(), Errno> {
    let (pair, dir) = pipe_of(fp);
    let cred = fp.cred();
    let st = pair.stat(dir);

    *ub = Stat::default();
    ub.st_mode = S_IFIFO;
    ub.st_blksize = st.blksize;
    ub.st_size = st.size;
    ub.st_blocks = st.blocks;
    ub.st_atim = Timespec::new(st.atime.tv_sec, st.atime.tv_nsec);
    ub.st_mtim = Timespec::new(st.mtime.tv_sec, st.mtime.tv_nsec);
    ub.st_ctim = Timespec::new(st.ctime.tv_sec, st.ctime.tv_nsec);
    ub.st_uid = cred.cr_uid.get();
    ub.st_gid = cred.cr_gid.get();
    // Left as 0: st_dev, st_ino, st_nlink, st_rdev, st_flags, st_gen.
    // XXX (st_dev, st_ino) should be unique.
    Ok(())
}

/// `fo_close` of a pipe: the file's last reference is gone. The end is the file's object,
/// dropped with the file's item (`pipe_destroy`), after `numfiles` falls: nothing to do here.
pub fn pipe_close(_fp: &File, _p: Option<&Proc>) -> Result<(), Errno> {
    Ok(())
}

/// `fo_kqfilter` of a pipe: `bsd`'s filter and flags as the kernel's.
pub fn pipe_kqfilter(_fp: &File, kn: &Knote) -> Result<(), Errno> {
    let (pair, dir) = pipe_of(&kn.fp());
    let filter = match kn.kn_filter().get() {
        EVFILT_READ => Some(PipeFilter::Read),
        EVFILT_WRITE => Some(PipeFilter::Write),
        EVFILT_EXCEPT => Some(PipeFilter::Except),
        _ => None,
    };
    pair.kqfilter(
        dir,
        filter,
        kn.has_flags(__EV_SELECT),
        kn.has_flags(__EV_POLL),
        kn,
    )
}

/// `pipe_init`: the pool of pipe pairs (`pipepl`, `IPL_MPFLOOR`, `PR_WAITOK`).
pub fn pipe_init() {
    if let Err(error) = PIPES.init(IPL_MPFLOOR) {
        panic(format_args!("pipe_init: {:?}", error));
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the glue: the fileops and filterops over real files, knotes and a real
    // kqueue (`kern_event.c`'s helpers), the pair from the kernel crate's `Pipes::create` over
    // its doubles of hal's bridges (`kernel::testutil`, feature `doubles`, registered once in
    // this test process; the real bridges never are here). A kqueue seeing a pipe become
    // readable; poll and select on pipes through the kernel functions they run; the ioctls and
    // `fo_stat`; what `kern_sysctl` reports; the sigio stores recycled.

    use core::ptr;
    use core::sync::atomic::Ordering;
    use std::sync::MutexGuard;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use kernel::pipe::{PIPE_EOF, PIPE_SIZE};

    use super::*;
    use crate::kern::kern_descrip::fnew;
    use crate::kern::kern_event::tests::thread as fd_thread;
    use crate::kern::kern_event::tests::{
        close_fd, close_kqueue, install, new_kqueue, poll_once, scan, select_once,
    };
    use crate::kern::kern_event::{kqpoll_exit, kqueue_register};
    use crate::sys::event::{EV_ADD, EV_EOF, EV_ONESHOT, Kevent, ev_set};
    use crate::sys::fcntl::{FREAD, FWRITE};
    use crate::sys::filio::FIONBIO;
    use crate::sys::poll::{POLLHUP, POLLIN, POLLNVAL, POLLOUT, Pollfd};
    use crate::sys::uio::{Iovec, UioRw, UioSeg};

    /// Real memory, the process, file and kqueue pools, the kernel crate's doubles and the
    /// pipe pool.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = crate::kern::kern_event::tests::setup();
        kernel::testutil::install();
        match PIPES.init(IPL_MPFLOOR) {
            Ok(()) | Err(Errno::EBUSY) => {}
            Err(e) => panic!("pipe_init: {e:?}"),
        }
        guard
    }

    /// A new pair whose files are descriptors 3 (`fds[0]`, `pp_rpipe`) and 4 (`fds[1]`,
    /// `pp_wpipe`) of the thread `p`, set up as `dopipe` sets them up.
    fn installed_pair(p: &Proc) -> PoolArc<PipePair<BsdProcGlue>> {
        let (r, w) = PIPES.create().unwrap();
        let pair = r.pair().clone();
        for (fd, end) in [(3, r), (4, w)] {
            let fp = fnew(p).unwrap();
            fp.f_flag.store((FREAD | FWRITE) as u32, Ordering::SeqCst);
            let (ops, data) =
                <crate::kern::kern_descrip::BsdFileGlue as kernel::file::FileGlue>::pipe_file(end);
            fp.install(crate::sys::file::DTYPE_PIPE, ops, data);
            install(p, fd, &fp);
        }
        pair
    }

    /// `write(2)` of `bytes` through `pipe_write`: the result and how much went in.
    fn write(fp: &File, bytes: &[u8]) -> (Result<(), Errno>, usize) {
        let mut iov = [Iovec::new()];
        iov[0].iov_base = bytes.as_ptr().cast_mut().cast();
        iov[0].iov_len = bytes.len();
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: bytes.len(),
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_WRITE,
            uio_procp: None,
        };
        let error = pipe_write(fp, &mut uio, 0);
        (error, bytes.len() - uio.uio_resid)
    }

    /// `read(2)` of at most `n` bytes through `pipe_read`.
    fn read(fp: &File, n: usize) -> (Result<(), Errno>, Vec<u8>) {
        let mut buf = vec![0u8; n];
        let mut iov = [Iovec::new()];
        iov[0].iov_base = buf.as_mut_ptr().cast();
        iov[0].iov_len = n;
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: n,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: None,
        };
        let error = pipe_read(fp, &mut uio, 0);
        let got = n - uio.uio_resid;
        buf.truncate(got);
        (error, buf)
    }

    #[test]
    fn the_fileops_move_bytes_and_answer_ioctl_and_stat() {
        let _g = setup();
        let p = fd_thread();
        let pair = installed_pair(p);
        let rf_ref = p.fd().ofile(3).unwrap();
        let rf: &File = &rf_ref;
        let wf_ref = p.fd().ofile(4).unwrap();
        let wf: &File = &wf_ref;
        assert_eq!(rf.flag() & (FREAD | FWRITE), FREAD | FWRITE);
        assert!(
            matches!(rf.data(), Some(BsdFileData::Pipe(_))),
            "a pipe's object is its end"
        );

        assert_eq!(write(wf, b"abc"), (Ok(()), 3));
        let mut data = [0u8; 8];
        assert_eq!(pipe_ioctl(rf, FIONREAD, &mut data, p), Ok(()));
        assert_eq!(int_arg(&data), 3);
        set_int_arg(&mut data, 1);
        assert_eq!(pipe_ioctl(rf, FIOASYNC, &mut data, p), Ok(()));
        assert_eq!(pipe_kinfo(rf).unwrap().state & 0x004, 0x004, "PIPE_ASYNC");
        set_int_arg(&mut data, 0);
        assert_eq!(pipe_ioctl(rf, FIOASYNC, &mut data, p), Ok(()));
        // sys_ioctl handles FIONBIO itself; the pipe does not.
        assert_eq!(pipe_ioctl(rf, FIONBIO, &mut data, p), Err(Errno::ENOTTY));

        let mut st = Stat::default();
        assert_eq!(pipe_stat(rf, &mut st, p), Ok(()));
        assert_eq!(st.st_mode, S_IFIFO);
        assert_eq!(
            (st.st_size, st.st_blksize, st.st_blocks),
            (3, PIPE_SIZE as i32, 1)
        );
        let cred = p.ucred();
        assert_eq!(
            (st.st_uid, st.st_gid, st.st_nlink),
            (cred.cr_uid.get(), cred.cr_gid.get(), 0)
        );

        assert_eq!(read(rf, 8), (Ok(()), b"abc".to_vec()));
        drop(pair);
        close_fd(p, 4);
        close_fd(p, 3);
    }

    #[test]
    fn kinfo_reports_the_pipes_as_the_c_does() {
        let _g = setup();
        let p = fd_thread();
        let pair = installed_pair(p);
        let rf_ref = p.fd().ofile(3).unwrap();
        let rf: &File = &rf_ref;
        let wf_ref = p.fd().ofile(4).unwrap();
        let wf: &File = &wf_ref;

        let rk = pipe_kinfo(rf).unwrap();
        let wk = pipe_kinfo(wf).unwrap();
        assert_eq!((rk.peer, wk.peer), (wk.addr, rk.addr));
        assert_eq!(rk.addr.max(rk.peer), wk.addr.max(wk.peer));

        // The test's reference goes first: the close is the last one.
        drop(wf_ref);
        close_fd(p, 4);
        let rk = pipe_kinfo(rf).unwrap();
        assert_eq!(rk.peer, 0);
        assert_eq!(rk.state & PIPE_EOF, PIPE_EOF);

        drop(pair);
        close_fd(p, 3);
    }

    #[test]
    fn kevent_sees_a_pipe_become_readable() {
        let _g = setup();
        let p = fd_thread();
        let pair = installed_pair(p);
        let kq = new_kqueue(p);
        let wf_ref = p.fd().ofile(4).unwrap();
        let wf: &File = &wf_ref;
        let rf_ref = p.fd().ofile(3).unwrap();
        let rf: &File = &rf_ref;
        let mut out = [Kevent::default(); 4];

        let mut kev = ev_set(3, EVFILT_READ, EV_ADD, 0, 0, 7);
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert!(!pair.klist_empty(Dir::Rd), "hooked on the read side");
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())), "empty");

        // The write wakes the reader's knote (pipe_wakeup's knote_locked).
        assert_eq!(write(wf, b"hello"), (Ok(()), 5));
        assert_eq!(kq.count(), 1);
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].filter), (3, EVFILT_READ));
        assert_eq!(
            (out[0].data, out[0].udata, out[0].flags & EV_EOF),
            (5, 7, 0)
        );

        // Drained: the next scan finds nothing.
        assert_eq!(read(rf, 5).1, b"hello");
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));

        // EVFILT_WRITE on the write end: room for the whole buffer, hooked on its peer.
        let mut kev = ev_set(4, EVFILT_WRITE, EV_ADD | EV_ONESHOT, 0, 0, 8);
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].data), (4, PIPE_SIZE as i64));
        // EVFILT_EXCEPT is poll's only.
        let mut kev = ev_set(3, EVFILT_EXCEPT, EV_ADD, 0, 0, 0);
        assert_eq!(
            kqueue_register(&kq, &mut kev, 0, Some(p)),
            Err(Errno::EINVAL)
        );

        // The writer goes away: EOF on the reader's knote.
        drop(wf_ref);
        close_fd(p, 4);
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!(out[0].flags & EV_EOF, EV_EOF);

        drop(pair);
        drop(rf_ref);
        close_fd(p, 3);
        assert_eq!(kq.nknotes(), 0);
        close_kqueue(p, kq);
    }

    #[test]
    fn poll_and_select_on_a_pipe() {
        let _g = setup();
        let p = fd_thread();
        let pair = installed_pair(p);
        let wf_ref = p.fd().ofile(4).unwrap();
        let wf: &File = &wf_ref;
        let pfd = |fd, events| Pollfd {
            fd,
            events,
            revents: 0,
        };

        // Empty: only the write end is ready.
        let mut pl = [pfd(3, POLLIN), pfd(4, POLLOUT)];
        assert_eq!(poll_once(p, &mut pl), 1);
        assert_eq!((pl[0].revents, pl[1].revents), (0, POLLOUT));
        assert_eq!(select_once(p, 5, 1 << 3, 1 << 4), Ok([0, 1 << 4, 0]));

        // Data in the pipe: both are ready (the knotes of the earlier calls are reused).
        assert_eq!(write(wf, b"ping"), (Ok(()), 4));
        let mut pl = [pfd(3, POLLIN), pfd(4, POLLOUT)];
        assert_eq!(poll_once(p, &mut pl), 2);
        assert_eq!((pl[0].revents, pl[1].revents), (POLLIN, POLLOUT));
        assert_eq!(select_once(p, 5, 1 << 3, 1 << 4), Ok([1 << 3, 1 << 4, 0]));

        // The writer is gone: the reader sees EOF as POLLIN | POLLHUP, and select as readable.
        drop(wf_ref);
        close_fd(p, 4);
        let mut pl = [pfd(3, POLLIN)];
        assert_eq!(poll_once(p, &mut pl), 1);
        assert_eq!(pl[0].revents, POLLIN | POLLHUP);
        assert_eq!(select_once(p, 5, 1 << 3, 0), Ok([1 << 3, 0, 0]));
        // A closed descriptor: POLLNVAL for poll, EBADF for select.
        let mut pl = [pfd(4, POLLIN)];
        assert_eq!(poll_once(p, &mut pl), 1);
        assert_eq!(pl[0].revents, POLLNVAL);
        assert_eq!(select_once(p, 5, 1 << 4, 0), Err(Errno::EBADF));

        drop(pair);
        close_fd(p, 3);
        kqpoll_exit(p);
    }

    #[test]
    fn the_hook_is_the_pipes_index() {
        for d in [Dir::Rd, Dir::Wr] {
            let hook: *mut core::ffi::c_void = ptr::without_provenance_mut(d.index());
            assert_eq!(Dir::from_index(hook.addr()), Some(d));
        }
        let mut data = [0u8; 4];
        set_int_arg(&mut data, -5);
        assert_eq!(int_arg(&data), -5);
        assert_eq!(int_arg(&[1, 2]), 0, "too short an argument reads as 0");
    }
}
/* </TESTS> */
