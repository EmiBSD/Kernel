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
//! (`sys/kernel/pipe.rs`), with 0 `unsafe`, until descriptors, kqueue and sigio move into the
//! kernel crate (N4).
//!
//! Upstream: sys/kern/sys_pipe.c @ 3ce1f3f79392
//! LZ: sys/kern/sys_pipe.rs@3c62ede68802
//!
//! Every decision pipe(2) makes is the kernel crate's; this file holds what N4 owns: the
//! `struct fileops` (`PIPEOPS`) and the three `struct filterops` of a pipe file, which turn
//! `bsd`'s `File`, `Uio`, `Knote`, ioctl numbers and `Stat` into the kernel's types; the
//! descriptor work of `dopipe` ([`PipeFiles`]); the per-pipe klist and sigio registration
//! ([`PipeGlue`]); the system call entries `bsd`'s `sysent[]` keeps (the `Legacy` path, which
//! calls the same kernel function as the module's `Native` entry); and `pipe_init`.
//!
//! A pipe file holds its pipe in its typed slot, `File::f_pipe` (`sys/sys/file.rs`): the
//! [`PipeEnd`] the kernel made for it, with the opener's uid and gid. `f_data` stays null for
//! a pipe. Every file operation locks the slot only to clone the pair's handle and read which
//! pipe it is ([`pipe_of`]), never sleeping and never calling into the pair under it; its
//! cost is one uncontended mutex and one atomic per pipe operation, gone at N4 with
//! `FileData` (decision 14). `fo_close` takes the end out under the mutex, releases it, and
//! only then drops the end, whose drop is `pipe_destroy` (it sleeps on `PIPE_WANTD`, which a
//! spinning mutex must not be held across).
//!
//! The klist and the sigio registration of a pipe live in a [`PipeKlists`] that the glue
//! allocates (`malloc(9)`, `M_TEMP`, through the global allocator) and recycles instead of
//! freeing: `bsd`'s `Klist` and `SigioRef` are `Sync` but not `Send`, so the pair (a
//! `PoolArc`, which needs its value `Send`) cannot hold them by value, and an `unsafe impl
//! Send` in `bsd` was refused (the coordinator's decision of 2026-10-16, decision 30's
//! pattern); a `&'static` to a never-freed object is `Send`. The klists are owner-locked
//! (`Klist::owned`, decision 13): the kernel calls every klist function in its `_locked` form
//! under the pair's write guard.
//!
//! ## Deviations
//! - The `PipeKlists` objects stay allocated at the peak number of pipes (recycled through a
//!   free list, never given back) and show under `M_TEMP` in `vmstat -m` instead of inside the
//!   `pipepl` items; no `diff-openbsd` scenario prints `vmstat -m`. Gone at N4, when `Klist`
//!   and `Sigio` are the kernel's and the pair holds them by value.
//! - `pipe_stat`'s `st_uid`/`st_gid` are the opener's credentials read when `dopipe` makes
//!   the files: the very `struct ucred` `fnew` stores in `f_cred`, which is never written once
//!   shared (`crcopy`), so the values are the C's; reading `f_cred` itself needs `unsafe`
//!   until descriptors move (N4, `CredRef`).
//! - `fo_ioctl`'s `data` is the kernel copy of the argument (`sys_ioctl`), at least an `int`
//!   wide; the `int` is read and written in native byte order (LZ's).
//! - A `PipeKlists` comes from the global allocator, `malloc(9)` with `M_NOWAIT`
//!   (`kern/rust_alloc.rs`): when no recycled one is free and memory is short, pipe(2) answers
//!   `ENOMEM` where the C's `pool_get(PR_WAITOK)` of the pair sleeps for memory (`hal`'s
//!   `MBox` has no safe way to a `&'static`; one would need `unsafe` in `hal`). Gone at N4.
//! - `KTRACE` is not configured (`ktrfds`), as in LZ.
//!
//! ## Redesign
//! - A call-site adapter, listed as such (docs/ROADMAP.md N2): LZ's `sys_pipe.rs` minus every
//!   decision, which moved to `sys/kernel/pipe.rs`. The `fileops` and `filterops` keep their
//!   names and signatures; each finds the pair through [`pipe_of`] (LZ's `fp_pipe`, a cast of
//!   `f_data`) and calls one kernel method.
//! - A knote's `kn_hook` holds the index of the pipe whose klist it is on (`Dir::index`, as
//!   an address without provenance), not a pointer (LZ's `kn_pipe` cast it back).
//! - `dopipe`'s descriptor work is [`BsdFiles`], the kernel's [`PipeFiles`]: the same
//!   `falloc`s under `fdplock`, the same error paths (the ends dropped where the C calls
//!   `pipe_destroy`), the same `copyout` of both numbers (the kernel's `UserPtr`), `fdrelease`
//!   of both on a fault.

use core::ptr;
use core::sync::atomic::Ordering;

#[cfg(feature = "alloc")]
use hal::mutex::Mutexed;
use hal::pool::PoolArc;
use hal::user::UserPtr;
use kernel::pipe::{
    Dir, PipeEnd, PipeFiles, PipeFilter, PipeFlags, PipeGlue, PipeIoctl, PipeKinfo, PipePair,
    Pipes, Timespec as PipeTime, UioMove,
};

use crate::kern::init_sysent::SYSENT;
use crate::kern::kern_descrip::{BsdFileData, closef, falloc, fdinsert, fdrelease, fdremove};
use crate::kern::kern_event::{klist_free, klist_insert_locked, klist_remove_locked, knote_locked};
use crate::kern::kern_sig::{pgsigio, sigio_free, sigio_getown, sigio_setown};
use crate::kern::kern_subr::uiomove;
use crate::kern::kern_tc::getnanotime;
use crate::kern::subr_prf::panic;
use crate::machine::cpu::curproc;
use crate::machine::intr::IPL_MPFLOOR;
use crate::sys::errno::Errno;
use crate::sys::event::{
    __EV_HUP, __EV_POLL, __EV_SELECT, EV_EOF, EVFILT_EXCEPT, EVFILT_READ, EVFILT_WRITE,
    FILTEROP_ISFD, FILTEROP_MPSAFE, Filterops, Kevent, Klist, Knote, knote_modify, knote_process,
};
use crate::sys::fcntl::{FNONBLOCK, FREAD, FWRITE};
use crate::sys::file::{DTYPE_PIPE, File, Fileops, frele};
use crate::sys::filedesc::{UF_EXCLOSE, UF_FORKCLOSE, fdplock};
use crate::sys::filio::{FIOASYNC, FIOGETOWN, FIONREAD, FIOSETOWN};
use crate::sys::proc::Proc;
use crate::sys::sigio::{SigioRef, sigio_init};
use crate::sys::signal::SIGIO;
use crate::sys::sockio::{SIOCGPGRP, SIOCSPGRP};
use crate::sys::stat::{S_IFIFO, Stat};
use crate::sys::syscall::{SYS_pipe, SYS_pipe2};
use crate::sys::syscall_mi::mi_syscall;
use crate::sys::syscallargs::{SysPipe2Args, SysPipeArgs};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::time::Timespec;
use crate::sys::ttycom::{TIOCGPGRP, TIOCSPGRP};
use crate::sys::types::{Gid, Register, Uid};
use crate::sys::uio::Uio;

/// One pipe's `pipe_klist` and `pipe_sigio`, allocated by the glue and recycled.
pub struct PipeKlists {
    /// `pipe_klist`, owner-locked: the pair's write guard is its lock.
    klist: Klist,
    /// `pipe_sigio`, under `sigio_lock` (`[S]`).
    sigio: SigioRef,
}

impl PipeKlists {
    const fn new() -> Self {
        Self {
            klist: Klist::owned(),
            sigio: SigioRef::new(),
        }
    }
}

/// `bsd`'s part of one pipe, the kernel's [`PipeGlue`]: its klist and sigio registration.
pub struct BsdPipeGlue {
    store: &'static PipeKlists,
}

/// What a pipe file's typed slot holds: its pipe, and the opener's credentials for
/// `pipe_stat`.
pub struct PipeFile {
    /// The pipe this file reads (and whose peer it writes).
    pub end: PipeEnd<BsdPipeGlue>,
    /// `f_cred->cr_uid` of the opener.
    pub uid: Uid,
    /// `f_cred->cr_gid` of the opener.
    pub gid: Gid,
}

/// `dopipe`'s descriptor work for the thread `p`.
pub struct BsdFiles<'a> {
    /// The thread that called `pipe(2)`.
    pub p: &'a Proc,
}

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

/// `pipe_rfiltops`: `EVFILT_READ` on a pipe.
pub static PIPE_RFILTOPS: Filterops = Filterops {
    f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
    f_attach: None,
    f_detach: Some(filt_pipedetach),
    f_event: Some(filt_piperead),
    f_modify: Some(filt_pipemodify),
    f_process: Some(filt_pipeprocess),
};

/// `pipe_wfiltops`: `EVFILT_WRITE` on a pipe.
pub static PIPE_WFILTOPS: Filterops = Filterops {
    f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
    f_attach: None,
    f_detach: Some(filt_pipedetach),
    f_event: Some(filt_pipewrite),
    f_modify: Some(filt_pipemodify),
    f_process: Some(filt_pipeprocess),
};

/// `pipe_efiltops`: `EVFILT_EXCEPT` on a pipe (poll's hang-up only).
pub static PIPE_EFILTOPS: Filterops = Filterops {
    f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
    f_attach: None,
    f_detach: Some(filt_pipedetach),
    f_event: Some(filt_pipeexcept),
    f_modify: Some(filt_pipemodify),
    f_process: Some(filt_pipeprocess),
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
pub static PIPES: Pipes<BsdPipeGlue> = Pipes::new(PIPE_PAGES);

/// The `PipeKlists` given back, for the next pipe. The mutex covers a pop or a push.
#[cfg(feature = "alloc")]
static FREE: Mutexed<alloc::vec::Vec<&'static PipeKlists>> =
    Mutexed::new(IPL_MPFLOOR, alloc::vec::Vec::new());

/// A `PipeKlists` given back by an earlier pipe, if there is one.
fn klists_recycled() -> Option<&'static PipeKlists> {
    #[cfg(feature = "alloc")]
    {
        FREE.lock().pop()
    }
    #[cfg(not(feature = "alloc"))]
    {
        None
    }
}

/// Gives `store` back for the next pipe; one that the list cannot take (its growth failed)
/// stays allocated, unused.
fn klists_recycle(store: &'static PipeKlists) {
    #[cfg(feature = "alloc")]
    {
        let mut free = FREE.lock();
        if free.try_reserve(1).is_ok() {
            free.push(store);
        }
    }
    #[cfg(not(feature = "alloc"))]
    {
        let _ = store;
    }
}

/// A fresh `PipeKlists` from `malloc(9)` (the global allocator), never freed: `ENOMEM` when
/// it cannot be had (`try_reserve`, never the allocator's panic), or without `alloc`.
fn klists_alloc() -> Result<&'static PipeKlists, Errno> {
    #[cfg(feature = "alloc")]
    {
        let mut v: alloc::vec::Vec<PipeKlists> = alloc::vec::Vec::new();
        v.try_reserve_exact(1).map_err(|_| Errno::ENOMEM)?;
        v.push(PipeKlists::new());
        let block: &'static [PipeKlists] = v.leak();
        let store = block.first().ok_or(Errno::ENOMEM)?;
        sigio_init(&store.sigio);
        Ok(store)
    }
    #[cfg(not(feature = "alloc"))]
    {
        Err(Errno::ENOMEM)
    }
}

impl PipeGlue for BsdPipeGlue {
    type Knote = Knote;

    /// `klist_init` and `sigio_init` of a pipe (`pipe_pair_create`, `pipe_create`): a
    /// recycled `PipeKlists`, or a new one.
    fn new() -> Result<Self, Errno> {
        let store = match klists_recycled() {
            Some(store) => store,
            None => klists_alloc()?,
        };
        Ok(Self { store })
    }

    fn nanotime() -> PipeTime {
        let ts = getnanotime();
        PipeTime {
            tv_sec: ts.tv_sec,
            tv_nsec: ts.tv_nsec,
        }
    }

    fn knote(&self, hint: i64) {
        knote_locked(&self.store.klist, hint);
    }

    fn klist_insert(&self, kn: &Knote, filter: PipeFilter, hook: Dir) {
        let fops: &'static Filterops = match filter {
            PipeFilter::Read => &PIPE_RFILTOPS,
            PipeFilter::Write => &PIPE_WFILTOPS,
            PipeFilter::Except => &PIPE_EFILTOPS,
        };
        kn.kn_fop.set(Some(fops));
        kn.kn_hook.set(ptr::without_provenance_mut(hook.index()));
        klist_insert_locked(&self.store.klist, kn);
    }

    fn klist_remove(&self, kn: &Knote) {
        klist_remove_locked(&self.store.klist, kn);
    }

    fn klist_free(&self) {
        klist_free(&self.store.klist);
    }

    fn pgsigio(&self) {
        pgsigio(&self.store.sigio, SIGIO, false);
    }

    fn sigio_free(&self) {
        sigio_free(&self.store.sigio);
    }

    fn sigio_setown(&self, cmd: u64, arg: i32) -> Result<(), Errno> {
        sigio_setown(&self.store.sigio, cmd, &arg)
    }

    fn sigio_getown(&self, cmd: u64) -> i32 {
        let mut pgid = 0;
        sigio_getown(&self.store.sigio, cmd, &mut pgid);
        pgid
    }
}

impl Drop for BsdPipeGlue {
    /// The pipe is gone (`pipe_pair_destroy`): its klist is empty, its sigio freed and set up
    /// again, and the object goes back on the free list; one that the list cannot take is
    /// left allocated (bounded: the list's growth failed).
    fn drop(&mut self) {
        klist_free(&self.store.klist);
        sigio_free(&self.store.sigio);
        sigio_init(&self.store.sigio);
        klists_recycle(self.store);
    }
}

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
}

impl PipeFiles<BsdPipeGlue> for BsdFiles<'_> {
    fn dopipe(
        &self,
        rpipe: PipeEnd<BsdPipeGlue>,
        wpipe: PipeEnd<BsdPipeGlue>,
        flags: PipeFlags,
        copyout: &dyn Fn([i32; 2]) -> Result<(), Errno>,
    ) -> Result<(), Errno> {
        dopipe(self.p, rpipe, wpipe, flags, copyout)
    }
}

/// A pipe file's object (`f_data`): its end and the opener's ids.
fn pipe_file_of(fp: &File) -> Option<&PipeFile> {
    match fp.data() {
        Some(BsdFileData::Pipe(pf)) => Some(pf),
        _ => None,
    }
}

/// The pair and the pipe of a pipe file (the pair counted again: a filter's file is a
/// temporary handle).
pub fn pipe_of(fp: &File) -> (PoolArc<PipePair<BsdPipeGlue>>, Dir) {
    match pipe_file_of(fp) {
        Some(pf) => (pf.end.pair().clone(), pf.end.dir()),
        None => panic(format_args!("file {:#x}: not an open pipe", fp.addr())),
    }
}

/// `kern_sysctl`'s `fill_file` of a pipe file.
pub fn pipe_kinfo(fp: &File) -> Option<PipeKinfo> {
    pipe_file_of(fp).map(|pf| pf.end.pair().kinfo(pf.end.dir()))
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

    PIPES.sys_pipe(UserPtr::new(uap.fdp.get() as usize), &BsdFiles { p })
}

/// `pipe2(2)`: `pipe(2)` with `O_CLOEXEC`, `O_CLOFORK` and `O_NONBLOCK` (`sysent[]`'s
/// entry).
pub fn sys_pipe2(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPipe2Args = sysargs(v);

    PIPES.sys_pipe2(
        UserPtr::new(uap.fdp.get() as usize),
        uap.flags.get(),
        &BsdFiles { p },
    )
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

/// Sets up a new pipe file: both ends are `FREAD | FWRITE`, as in the C.
fn pipe_file(fp: &File, end: PipeEnd<BsdPipeGlue>, flags: PipeFlags, p: &Proc) {
    let fflag = (FREAD | FWRITE | if flags.nonblock { FNONBLOCK } else { 0 }) as u32;
    let cred = p.ucred();
    fp.f_flag.store(fflag, Ordering::SeqCst);
    fp.install(
        DTYPE_PIPE,
        &PIPEOPS,
        BsdFileData::Pipe(PipeFile {
            end,
            uid: cred.cr_uid.get(),
            gid: cred.cr_gid.get(),
        }),
    );
}

/// `dopipe(p, ufds, flags)` from the new pair on: installs its two files at the lowest free
/// descriptors and copies them out.
pub fn dopipe(
    p: &Proc,
    rpipe: PipeEnd<BsdPipeGlue>,
    wpipe: PipeEnd<BsdPipeGlue>,
    flags: PipeFlags,
    copyout: &dyn Fn([i32; 2]) -> Result<(), Errno>,
) -> Result<(), Errno> {
    let fdp = p.fd();

    let fdflags = (if flags.cloexec { UF_EXCLOSE } else { 0 })
        | (if flags.clofork { UF_FORKCLOSE } else { 0 });

    let mut g = fdplock(fdp);

    let (rf, fd0) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(error) => {
            // free2:
            drop(g);
            drop(wpipe);
            drop(rpipe);
            return Err(error);
        }
    };
    pipe_file(&rf, rpipe, flags, p);

    let (wf, fd1) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(error) => {
            // free3: the descriptor (no slot holds the file yet) and the file, whose last
            // reference closes it, which destroys `rpipe`.
            let _ = fdremove(&mut g, fd0);
            let _ = closef(rf, p);
            // free2:
            drop(g);
            drop(wpipe);
            return Err(error);
        }
    };
    pipe_file(&wf, wpipe, flags, p);

    fdinsert(&mut g, fd0, fdflags, &rf);
    fdinsert(&mut g, fd1, fdflags, &wf);

    let error = copyout([fd0, fd1]);
    if error.is_ok() {
        drop(g);
        // KTRACE (ktrfds): not configured.
    } else {
        // fdrelease() unlocks fdp.
        let _ = fdrelease(p, g, fd0);
        let g = fdplock(fdp);
        let _ = fdrelease(p, g, fd1);
    }

    let _ = frele(rf, p);
    let _ = frele(wf, p);
    error
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
    let (uid, gid) = pipe_file_of(fp).map_or((0, 0), |pf| (pf.uid, pf.gid));
    let st = pair.stat(dir);

    *ub = Stat::default();
    ub.st_mode = S_IFIFO;
    ub.st_blksize = st.blksize;
    ub.st_size = st.size;
    ub.st_blocks = st.blocks;
    ub.st_atim = Timespec::new(st.atime.tv_sec, st.atime.tv_nsec);
    ub.st_mtim = Timespec::new(st.mtime.tv_sec, st.mtime.tv_nsec);
    ub.st_ctim = Timespec::new(st.ctime.tv_sec, st.ctime.tv_nsec);
    ub.st_uid = uid;
    ub.st_gid = gid;
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

/// `filt_pipedetach(kn)`: unhooks the knote from the klist `kn_hook` names.
pub fn filt_pipedetach(kn: &Knote) {
    let (pair, _) = pipe_of(&kn.fp());
    let Some(hook) = Dir::from_index(kn.kn_hook.get().addr()) else {
        panic(format_args!("knote {:p}: no pipe", kn));
    };
    pair.detach(hook, kn);
}

/// One filter's answer written into the knote.
fn filt_pipe(kn: &Knote, which: PipeFilter) -> bool {
    let (pair, dir) = pipe_of(&kn.fp());
    let answer = pair.filter(dir, which, kn.has_flags(__EV_POLL));
    if let Some(data) = answer.data {
        kn.kn_data().set(data);
    }
    if answer.eof {
        kn.set_flags(EV_EOF);
    }
    if answer.hup {
        kn.set_flags(__EV_HUP);
    }
    answer.active
}

/// `filt_piperead(kn, hint)`.
pub fn filt_piperead(kn: &Knote, _hint: i64) -> bool {
    filt_pipe(kn, PipeFilter::Read)
}

/// `filt_pipewrite(kn, hint)`.
pub fn filt_pipewrite(kn: &Knote, _hint: i64) -> bool {
    filt_pipe(kn, PipeFilter::Write)
}

/// `filt_pipeexcept(kn, hint)`.
pub fn filt_pipeexcept(kn: &Knote, _hint: i64) -> bool {
    filt_pipe(kn, PipeFilter::Except)
}

/// `filt_pipemodify(kev, kn)`: `knote_modify` under the pipe's lock.
pub fn filt_pipemodify(kev: &mut Kevent, kn: &Knote) -> bool {
    let (pair, _) = pipe_of(&kn.fp());
    pair.with_lock(|| knote_modify(kev, kn))
}

/// `filt_pipeprocess(kn, kev)`: `knote_process` under the pipe's lock.
pub fn filt_pipeprocess(kn: &Knote, kev: Option<&mut Kevent>) -> bool {
    let (pair, _) = pipe_of(&kn.fp());
    pair.with_lock(|| knote_process(kn, kev))
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
    // `fo_stat`; what `kern_sysctl` reports; the klists recycled.

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
    use crate::sys::event::{EV_ADD, EV_ONESHOT, ev_set, klist_empty};
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
    fn installed_pair(p: &Proc) -> PoolArc<PipePair<BsdPipeGlue>> {
        let (r, w) = PIPES.create().unwrap();
        let pair = r.pair().clone();
        for (fd, end) in [(3, r), (4, w)] {
            let fp = fnew(p).unwrap();
            pipe_file(&fp, end, PipeFlags::default(), p);
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
    #[cfg(feature = "alloc")]
    fn the_klists_are_recycled() {
        let _g = setup();
        let p = fd_thread();
        let pair = installed_pair(p);
        let free = FREE.lock().len();
        drop(pair);
        close_fd(p, 3);
        assert_eq!(
            FREE.lock().len(),
            free,
            "the writer's file still holds the pair"
        );
        close_fd(p, 4);
        assert_eq!(FREE.lock().len(), free + 2);
        // The next pair takes them back.
        let pair = installed_pair(p);
        assert_eq!(FREE.lock().len(), free);
        drop(pair);
        close_fd(p, 3);
        close_fd(p, 4);
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
        assert_eq!(kqueue_register(kq, &mut kev, 0, Some(p)), Ok(()));
        assert!(
            !klist_empty(&pair.glue(Dir::Rd).store.klist),
            "hooked on the read side"
        );
        assert_eq!(scan(p, kq, &mut out), (0, Ok(())), "empty");

        // The write wakes the reader's knote (pipe_wakeup's knote_locked).
        assert_eq!(write(wf, b"hello"), (Ok(()), 5));
        assert_eq!(kq.kq_count.get(), 1);
        assert_eq!(scan(p, kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].filter), (3, EVFILT_READ));
        assert_eq!(
            (out[0].data, out[0].udata, out[0].flags & EV_EOF),
            (5, 7, 0)
        );

        // Drained: the next scan finds nothing.
        assert_eq!(read(rf, 5).1, b"hello");
        assert_eq!(scan(p, kq, &mut out), (0, Ok(())));

        // EVFILT_WRITE on the write end: room for the whole buffer, hooked on its peer.
        let mut kev = ev_set(4, EVFILT_WRITE, EV_ADD | EV_ONESHOT, 0, 0, 8);
        assert_eq!(kqueue_register(kq, &mut kev, 0, Some(p)), Ok(()));
        assert_eq!(scan(p, kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].data), (4, PIPE_SIZE as i64));
        // EVFILT_EXCEPT is poll's only.
        let mut kev = ev_set(3, EVFILT_EXCEPT, EV_ADD, 0, 0, 0);
        assert_eq!(
            kqueue_register(kq, &mut kev, 0, Some(p)),
            Err(Errno::EINVAL)
        );

        // The writer goes away: EOF on the reader's knote.
        drop(wf_ref);
        close_fd(p, 4);
        assert_eq!(scan(p, kq, &mut out), (1, Ok(())));
        assert_eq!(out[0].flags & EV_EOF, EV_EOF);

        drop(pair);
        drop(rf_ref);
        close_fd(p, 3);
        assert_eq!(kq.kq_nknotes.get(), 0);
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
