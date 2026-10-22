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
 *	@(#)kern_descrip.c	8.6 (Berkeley) 4/19/94
 */

/*
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
//! Descriptor management: `kern_descrip.c` (the per-process descriptor tables, descriptor
//! allocation, the open files' life and the descriptor system calls) and the generic file
//! system calls of `sys_generic.c` (`read`, `write`, their vector and positioned forms,
//! `ioctl`, `utrace`), over `<sys/filedesc.h>`'s table.
//!
//! Upstream: sys/kern/kern_descrip.c @ 3ce1f3f79392
//! Upstream: sys/sys/filedesc.h @ 3ce1f3f79392
//! Upstream: sys/kern/sys_generic.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_descrip.rs@3c62ede68802
//! LZ: sys/sys/filedesc.rs@3c62ede68802
//! LZ: sys/kern/sys_generic.rs@3c62ede68802
//!
//! A descriptor table ([`Filedesc`]) is an item of the pool `fdescpl`, held by its processes
//! ([`FdRef`], `kinfo_proc.p_fd` prints its address). Its slots (`fd_ofiles`, the flags) sit
//! inside `fd_fplock` (a mutex at `IPL_MPFLOOR`), so `fd_getfile` reads a slot without the
//! table's lock; the free-descriptor bitmaps and the high-water marks sit inside `fd_lock`
//! (the sleeping rwlock "fdlock"), which every writer of the table takes ([`fdplock`], an
//! [`FdGuard`]). `fdrelease` and `finishdup` take the guard and return with the table
//! unlocked, as the C's return unlocked. A slot holds a counted [`FileRef`]; a close takes
//! the slot's reference out ([`fdremove`]) and drops it after the unlock ([`closef`]).
//!
//! What a table needs of the subsystems still in `bsd` (the vnode layer's record locks and
//! `pathconf`, kqueue's `knote_fdclose`, pledge, `SIGPIPE`) is [`FileGlue`]'s.
//!
//! ## Deviations
//! - Out-parameters are return values: `fdalloc` returns the descriptor, `falloc` the file
//!   and the descriptor, `fd_getfile` an `Option<FileRef>` (the C's NULL is `None`), and the
//!   system calls their `retval`.
//! - `fdexpand` returns `ENOMEM` when `malloc(9)` answers nothing (LZ's: the C's `M_WAITOK`
//!   cannot fail), and its callers pass the error up; `fnew` answers `None` (`ENFILE` from
//!   `falloc`) when `filepl` gives no item; `sys_ioctl`'s buffer past 128 bytes likewise
//!   (`ENOMEM`). `fdinit` and `fdcopy`, whose callers cannot fail, `panic(9)`.
//! - `find_next_zero` reads the words past the end of a bitmap as full, where the C reads
//!   past the array; the C rejects whatever it finds there (`i < last`), so the result is the
//!   same.
//! - `fd_nfiles` is the length of the slots, read under `fd_fplock` by `fd_getfile` (the C
//!   reads it before taking the mutex); the C's other unlocked reads stay unlocked
//!   (`fd_openfd`, `fd_cmask`, `fd_flags`: atomics).
//! - `fdfree` of a table's last process closes every descriptor and empties the table; the
//!   item goes back to `fdescpl` with the last handle (the process's), so a zombie's emptied
//!   table stays allocated until `wait4` reaps it (`vmstat -m fdescpl` only;
//!   `kinfo_proc.p_fd` prints the same address). The C's `p->p_fd = NULL` after `fdfree` is
//!   that emptied table: a `closef` after it finds no lock to release.
//! - `KTRACE` is not configured: `utrace(2)` succeeds and records nothing (the C's `#else`),
//!   and the read and write paths trace nothing. `KASSERTMSG` is `kassert!`.
//!
//! ## Redesign
//! - LZ's `struct filedesc` kept `fd_ofiles`, `fd_ofileflags`, `fd_himap` and `fd_lomap` as
//!   raw pointers into a `struct filedesc0` (its own arrays) or into `malloc` blocks, read
//!   and written by bounds-checked `unsafe` accessors, `fdexpand` copying them with
//!   `ptr::copy_nonoverlapping`, and the table itself a raw `fdesc_pool` item freed by
//!   `pool_put` from `fdfree`; `fd_lock` and `fd_fplock` were locks beside the data they
//!   guard. Now the data is inside its lock: the slots (`Option<FileRef>`) and their flags in
//!   `fd_fplock`'s `Mutexed`, the bitmaps and the marks in `fd_lock`'s `Rwlocked`, each array
//!   the internal one of `filedesc0` or an owned `MArray` (`M_FILEDESC`), swapped by
//!   `fdexpand` under both locks and freed by its drop; the table is a `TypedPool` item behind
//!   [`FdRef`]. A slot cannot name a freed file, and a writer cannot forget the lock.
//! - `FREF`/`FRELE`/`f_count` are the handles' count ([`crate::file`]): `fd_getfile` clones
//!   the slot's handle under the mutex; `closef`'s "count is at least 2, drop one" is the
//!   slot's handle, which [`fdremove`] hands its caller to drop after the unlock;
//!   `fd_iterfile`'s compare-and-swap from a non-zero count is `FileRef::try_from_ref` under
//!   `fhdlk`.
//! - `p->p_fd`/`ps_fd` are the process's set-once [`FdRef`] (`kernel::proc`), never taken
//!   back: `fd_refcnt` is the count of processes sharing the table (`fdshare`), and the last
//!   `fdfree` empties it (see the deviations).
//! - `dofilewritev`'s unlocked read of `fd_ofileflags[fd]` (`UF_PLEDGEOPEN`) is made with
//!   the reference, under the mutex ([`fd_getfile_flags`]).
//! - `iovec_copyin`'s array is an owned buffer (`crate::uio::IovBuf`): `iovec_free` is its
//!   drop. `sys_ioctl`'s `M_IOCTLOPS` buffer is an `MArray<u8>`, freed by its drop.
//! - `fdalloc` takes the limit (`lim_cur(RLIMIT_NOFILE)`, `maxfiles`) once per call; the C
//!   re-reads it when it restarts the search, with nothing in between that could change it.

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hal::link::Link;
use hal::malloc::{M_WAITOK, MArray};
use hal::mutex::Mutexed;
use hal::pool::{PR_WAITOK, PoolArc};
use hal::rwlock::{RW_WRITE, Rwlocked, WriteGuard};
use hal::sys::errno::Errno;
use hal::sys::fcntl::{
    F_DUPFD, F_DUPFD_CLOEXEC, F_DUPFD_CLOFORK, F_FLOCK, F_GETFD, F_GETFL, F_GETLK, F_GETOWN,
    F_ISATTY, F_POSIX, F_RDLCK, F_SETFD, F_SETFL, F_SETLK, F_SETLKW, F_SETOWN, F_UNLCK, F_WAIT,
    F_WRLCK, FASYNC, FCNTLFLAGS, FD_CLOEXEC, FD_CLOFORK, FNONBLOCK, FREAD, FWRITE, Flock, LOCK_EX,
    LOCK_NB, LOCK_SH, LOCK_UN, O_CLOEXEC, O_CLOFORK, fflags, oflags,
};
use hal::sys::filio::{FIOASYNC, FIOCLEX, FIOGETOWN, FIONBIO, FIONCLEX, FIOSETOWN};
use hal::sys::ioccom::{IOC_IN, IOC_OUT, IOC_VOID, IOCPARM_MAX, iocparm_len};
use hal::sys::limits::SSIZE_MAX;
use hal::sys::syslimits::PIPE_BUF;
use hal::sys::types::{Off, Register};
use hal::sys::unistd::{_PC_PIPE_BUF, SEEK_CUR, SEEK_SET};
use hal::user::UserPtr;

use crate::assert::{kassert, kpanic};
use crate::file::{
    DTYPE_KQUEUE, DTYPE_PIPE, DTYPE_SOCKET, DTYPE_VNODE, FDUP_MAX_COUNT, FIF_HASLOCK, FIF_INSERTED,
    FO_POSITION, File, FileGlue, FileOps, FileRef, FileSlot, LockOwner, VFS_STALLING, fnew,
    foffset, vfs_stall_barrier,
};
use crate::proc::{PS_PLEDGE, PS_SUGID, PS_SUGIDEXEC, ProcGlue, Process, Thread, suser};
use crate::resource::RLIMIT_NOFILE;
use crate::uio::{IovBuf, Iovec, Uio, UioRw, UioSeg, iovec_copyin};

/// `NDFILE`: descriptors in the initial table.
pub const NDFILE: usize = 20;
/// `NDEXTENT`: the first expansion (250 bytes in 256-byte alloc).
pub const NDEXTENT: usize = 50;
/// `NDENTRIES`: 32 fds per entry.
pub const NDENTRIES: usize = 32;
/// `NDENTRYMASK`.
pub const NDENTRYMASK: usize = NDENTRIES - 1;
/// `NDENTRYSHIFT`: bits per entry.
pub const NDENTRYSHIFT: usize = 5;

/// `NDREDUCE(x)`: the number of 32-bit words `x` bits need.
pub const fn ndreduce(x: usize) -> usize {
    (x + NDENTRIES - 1) >> NDENTRYSHIFT
}

/// `NDHISLOTS(x)`: the words of `fd_himap` for `x` descriptors.
pub const fn ndhislots(x: usize) -> usize {
    ndreduce(ndreduce(x))
}

/// `NDLOSLOTS(x)`: the words of `fd_lomap` for `x` descriptors.
pub const fn ndloslots(x: usize) -> usize {
    ndhislots(x) << NDENTRYSHIFT
}

/// `UF_EXCLOSE`: auto-close on exec.
pub const UF_EXCLOSE: u8 = 0x01;
/// `UF_PLEDGED`: opened after `pledge(2)`.
pub const UF_PLEDGED: u8 = 0x02;
/// `UF_FORKCLOSE`: auto-close on fork.
pub const UF_FORKCLOSE: u8 = 0x04;
/// `UF_PLEDGEOPEN`: opened with `__pledge_open()`.
pub const UF_PLEDGEOPEN: u8 = 0x08;

/// `FD_ADVLOCK`: may hold a POSIX adv. lock (`fd_flags`).
pub const FD_ADVLOCK: i32 = 0x01;

/// `M_FILEDESC`: open file descriptor tables.
pub const M_FILEDESC: i32 = 39;
/// `M_IOCTLOPS`: ioctl data buffers.
pub const M_IOCTLOPS: i32 = 14;

/// `S_IWGRP | S_IWOTH`: a new table's `fd_cmask`.
const CMASK_DEFAULT: u32 = 0o020 | 0o002;

/// `DUPF_CLOEXEC`.
const DUPF_CLOEXEC: i32 = 0x01;
/// `DUPF_DUP2`.
const DUPF_DUP2: i32 = 0x02;
/// `DUPF_CLOFORK`.
const DUPF_CLOFORK: i32 = 0x04;

/// `STK_PARAMS`: the ioctl argument bytes `sys_ioctl` keeps on its stack.
const STK_PARAMS: usize = 128;

/// An array of a table: `filedesc0`'s internal one of `N` entries, or an `M_FILEDESC` block
/// past it.
enum Store<T, const N: usize> {
    Inline([T; N]),
    Heap(MArray<T>),
}

impl<T, const N: usize> Store<T, N> {
    fn as_slice(&self) -> &[T] {
        match self {
            Self::Inline(a) => a,
            Self::Heap(m) => m,
        }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        match self {
            Self::Inline(a) => a,
            Self::Heap(m) => m,
        }
    }
}

/// `fd_dhimap`'s words (`NDHISLOTS(NDFILE)`).
const NDHI0: usize = ndhislots(NDFILE);
/// `fd_dlomap`'s words (`NDLOSLOTS(NDFILE)`).
const NDLO0: usize = ndloslots(NDFILE);

/// \[f\] What `fd_lock` guards: the free-descriptor bitmaps and the marks.
pub struct FdState {
    /// `fd_himap`: the words of `fd_lomap` that are full.
    himap: Store<u32, NDHI0>,
    /// `fd_lomap`: the descriptors in use.
    lomap: Store<u32, NDLO0>,
    /// `fd_nfiles`: the number of slots (and of bits the maps cover).
    nfiles: usize,
    /// `fd_lastfile`: high-water mark of `fd_ofiles`.
    lastfile: i32,
    /// `fd_freefile`: approx. next free file.
    freefile: i32,
}

impl FdState {
    fn new() -> Self {
        Self {
            himap: Store::Inline([0; NDHI0]),
            lomap: Store::Inline([0; NDLO0]),
            nfiles: NDFILE,
            lastfile: 0,
            freefile: 0,
        }
    }

    /// `fd_lastfile`.
    pub fn lastfile(&self) -> i32 {
        self.lastfile
    }

    /// `fd_freefile`.
    pub fn freefile(&self) -> i32 {
        self.freefile
    }

    /// `fd_nfiles`.
    pub fn nfiles(&self) -> usize {
        self.nfiles
    }

    /// `fdp->fd_lomap[off]` (0 past the map: no descriptor there is in use).
    fn lomap(&self, off: usize) -> u32 {
        self.lomap.as_slice().get(off).copied().unwrap_or(0)
    }

    /// `fd_inuse(fdp, fd)`: whether descriptor `fd` is allocated.
    fn fd_inuse(&self, fd: i32) -> bool {
        let off = (fd as u32 >> NDENTRYSHIFT) as usize;
        self.lomap(off) & (1u32 << (fd as u32 & NDENTRYMASK as u32)) != 0
    }

    /// `fd_used(fdp, fd)`: marks descriptor `fd` allocated (`openfd` is `fd_openfd`).
    fn fd_used(&mut self, fd: i32, openfd: &AtomicI32) {
        let off = (fd as u32 >> NDENTRYSHIFT) as usize;
        let lo = self.lomap.as_mut_slice();
        lo[off] |= 1u32 << (fd as u32 & NDENTRYMASK as u32);
        if lo[off] == !0 {
            let hi = off >> NDENTRYSHIFT;
            self.himap.as_mut_slice()[hi] |= 1u32 << (off & NDENTRYMASK);
        }
        if fd > self.lastfile {
            self.lastfile = fd;
        }
        openfd.fetch_add(1, Ordering::Relaxed);
    }

    /// `fd_unused(fdp, fd)`: marks descriptor `fd` free.
    fn fd_unused(&mut self, fd: i32, openfd: &AtomicI32) {
        let off = (fd as u32 >> NDENTRYSHIFT) as usize;
        if fd < self.freefile {
            self.freefile = fd;
        }
        if self.lomap.as_slice()[off] == !0 {
            let hi = off >> NDENTRYSHIFT;
            self.himap.as_mut_slice()[hi] &= !(1u32 << (off & NDENTRYMASK));
        }
        self.lomap.as_mut_slice()[off] &= !(1u32 << (fd as u32 & NDENTRYMASK as u32));
        if cfg!(feature = "diagnostic") && fd > self.lastfile {
            kpanic(format_args!("fd_unused: fd_lastfile inconsistent"));
        }
        if fd == self.lastfile {
            self.lastfile = find_last_set(self, fd);
        }
        openfd.fetch_sub(1, Ordering::Relaxed);
    }
}

/// \[m\] What `fd_fplock` guards: `fd_ofiles` and `fd_ofileflags`.
struct Slots<G: FileGlue> {
    /// `fd_ofiles`: the file at each descriptor, with the reference the slot holds.
    files: Store<FileSlot<G>, NDFILE>,
    /// `fd_ofileflags`: the `UF_*` flags of each descriptor.
    flags: Store<u8, NDFILE>,
}

impl<G: FileGlue> Slots<G> {
    fn new() -> Self {
        Self {
            files: Store::Inline([const { FileSlot::new() }; NDFILE]),
            flags: Store::Inline([0; NDFILE]),
        }
    }

    fn file(&self, fd: usize) -> Option<&FileRef<G>> {
        self.files.as_slice().get(fd).and_then(FileSlot::get)
    }

    fn flags(&self, fd: usize) -> u8 {
        self.flags.as_slice().get(fd).copied().unwrap_or(0)
    }

    fn set_flags(&mut self, fd: usize, f: u8) {
        match self.flags.as_mut_slice().get_mut(fd) {
            Some(s) => *s = f,
            None => kpanic(format_args!("filedesc: fd_ofileflags[{fd}] out of range")),
        }
    }

    fn slot(&mut self, fd: usize) -> &mut FileSlot<G> {
        match self.files.as_mut_slice().get_mut(fd) {
            Some(s) => s,
            None => kpanic(format_args!("filedesc: fd_ofiles[{fd}] out of range")),
        }
    }
}

/// `struct filedesc`: a descriptor table, shared by the processes of `fdshare`.
///
/// Locks: \[a\] atomic operations, \[f\] `fd_lock` ([`FdState`]), \[m\] `fd_fplock` (the
/// slots), \[K\] kernel lock (`fd_cdir`, `fd_rdir`, links the C reads under it).
pub struct Filedesc<G: FileGlue> {
    /// `fd_lock`: "fdlock", the writers' lock, and the bitmaps it guards.
    lock: Rwlocked<FdState>,
    /// `fd_fplock`: the slots and their flags, read by `fd_getfile` without `fd_lock`.
    slots: Mutexed<Slots<G>>,
    /// \[K\] `fd_refcnt`: the processes sharing this table.
    shares: AtomicU32,
    /// \[f\] `fd_openfd`: number of files currently open (read unlocked).
    pub fd_openfd: AtomicI32,
    /// \[a\] `fd_flags`: flags on this filedesc (`FD_ADVLOCK`).
    pub fd_flags: AtomicI32,
    /// \[f/w\] `fd_cmask`: mask for file creation (read unlocked by `open`, as in C).
    cmask: AtomicU32,
    /// \[K\] `fd_cdir`: current directory.
    cdir: Link<G::Vnode>,
    /// \[K\] `fd_rdir`: root directory.
    rdir: Link<G::Vnode>,
    /// `fd_kqlist`, `fd_nuserevents`: kqueue's members (`bsd`'s until N4.4).
    kq: G::FdKq,
}

impl<G: FileGlue> Filedesc<G> {
    /// `fdinit`'s table: `NDFILE` slots, one process, `S_IWGRP | S_IWOTH`.
    fn new() -> Self {
        Self::with(FdState::new(), Slots::new())
    }

    fn with(state: FdState, slots: Slots<G>) -> Self {
        Self {
            lock: Rwlocked::new("fdlock", state),
            slots: Mutexed::new(hal::intr::mpfloor(), slots),
            shares: AtomicU32::new(1),
            fd_openfd: AtomicI32::new(0),
            fd_flags: AtomicI32::new(0),
            cmask: AtomicU32::new(CMASK_DEFAULT),
            cdir: Link::new(),
            rdir: Link::new(),
            kq: G::FdKq::default(),
        }
    }

    /// The table's address (`kinfo_proc.p_fd`, the POSIX lock owner).
    pub fn addr(&self) -> usize {
        core::ptr::from_ref(self).addr()
    }

    /// `fd_refcnt`: the processes sharing the table (0 once the last `fdfree` ran).
    pub fn shares(&self) -> u32 {
        self.shares.load(Ordering::Acquire)
    }

    /// `fd_nfiles`, read now.
    pub fn nfiles(&self) -> usize {
        self.slots.lock().files.as_slice().len()
    }

    /// `fd_ofiles[fd]`, read now with a reference taken (no `vfs_stall_barrier`: a walk of
    /// the table, `kinfo_file`'s); `None` for an empty slot or past the table.
    pub fn ofile(&self, fd: usize) -> Option<FileRef<G>> {
        self.slots.lock().file(fd).cloned()
    }

    /// `fd_ofileflags[fd]`, read now (0 past the table).
    pub fn ofileflags(&self, fd: i32) -> u8 {
        self.slots.lock().flags(fd as u32 as usize)
    }

    /// `fd_cmask`.
    pub fn cmask(&self) -> u32 {
        self.cmask.load(Ordering::Relaxed)
    }

    /// `fd_cmask = mask` (`umask(2)`, the table locked by the caller as in C).
    pub fn set_cmask(&self, mask: u32) {
        self.cmask.store(mask, Ordering::Relaxed);
    }

    /// `fd_cdir`.
    pub fn cdir(&self) -> Option<&'static G::Vnode> {
        self.cdir.get()
    }

    /// `fd_cdir = vp` (the caller holds the use count it gives, `vref`).
    pub fn set_cdir(&self, vp: Option<&'static G::Vnode>) {
        self.cdir.set(vp);
    }

    /// `fd_cdir = vp`, the old value back (its use count the caller's).
    pub fn replace_cdir(&self, vp: Option<&'static G::Vnode>) -> Option<&'static G::Vnode> {
        let old = self.cdir.get();
        self.cdir.set(vp);
        old
    }

    /// `fd_rdir`.
    pub fn rdir(&self) -> Option<&'static G::Vnode> {
        self.rdir.get()
    }

    /// `fd_rdir = vp` (as [`Filedesc::set_cdir`]).
    pub fn set_rdir(&self, vp: Option<&'static G::Vnode>) {
        self.rdir.set(vp);
    }

    /// `fd_kqlist`, `fd_nuserevents`.
    pub fn kq(&self) -> &G::FdKq {
        &self.kq
    }

    /// `rw_status(&fdp->fd_lock)`: `RW_WRITE` when the running thread holds the table.
    pub fn lock_status(&self) -> i32 {
        self.lock.status()
    }

    /// The table's state, read under its lock taken for reading (`kinfo`'s snapshot).
    pub fn with_state<R>(&self, f: impl FnOnce(&FdState) -> R) -> R {
        f(&self.lock.read())
    }
}

/// A counted reference to a descriptor table (`struct filedesc *` with `fd_refcnt`'s share).
pub struct FdRef<G: FileGlue>(PoolArc<Filedesc<G>>);

impl<G: FileGlue> FdRef<G> {
    /// The table's address (`kinfo_proc.p_fd`).
    pub fn addr(&self) -> usize {
        self.0.addr()
    }
}

impl<G: FileGlue> Clone for FdRef<G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<G: FileGlue> core::ops::Deref for FdRef<G> {
    type Target = Filedesc<G>;

    fn deref(&self) -> &Filedesc<G> {
        &self.0
    }
}

/// `fdplock(fdp)` held: the table's lock taken for writing, and its bitmaps.
pub struct FdGuard<'a, G: FileGlue> {
    fdp: &'a Filedesc<G>,
    st: WriteGuard<'a, FdState>,
}

impl<'a, G: FileGlue> FdGuard<'a, G> {
    /// The table this guard holds.
    pub fn fdp(&self) -> &'a Filedesc<G> {
        self.fdp
    }

    /// The state under the lock (`fd_lastfile`, `fd_freefile`, `fd_nfiles`).
    pub fn state(&self) -> &FdState {
        &self.st
    }

    /// `fd_ofileflags[fd]`.
    pub fn ofileflags(&self, fd: i32) -> u8 {
        self.fdp.slots.lock().flags(fd as u32 as usize)
    }

    /// `fd_ofileflags[fd] = flags`.
    pub fn set_ofileflags(&mut self, fd: i32, flags: u8) {
        self.fdp.slots.lock().set_flags(fd as u32 as usize, flags);
    }

    /// `fd_ofiles[fd]`, a reference taken (no `vfs_stall_barrier`: the table's own read).
    pub fn ofile(&self, fd: i32) -> Option<FileRef<G>> {
        self.fdp.slots.lock().file(fd as u32 as usize).cloned()
    }

    /// `fd_ofiles[fd] = NULL; fd_ofileflags[fd] = 0` under `fd_fplock`, the descriptor's
    /// number kept allocated: the slot's reference and flags come back (diskmap's swap of a
    /// descriptor's file, which `fdinsert` refills).
    pub fn take_ofile(&mut self, fd: i32) -> (Option<FileRef<G>>, u8) {
        let mut s = self.fdp.slots.lock();
        let fp = s.slot(fd as usize).take();
        let flags = s.flags(fd as usize);
        s.set_flags(fd as usize, 0);
        (fp, flags)
    }

    /// `fdp->fd_ofiles[fd] = fp` under `fd_fplock` of an allocated, empty descriptor, the
    /// slot taking the caller's reference, and its flags (`unp_externalize`'s placement of a
    /// received file, already in `filehead`).
    pub fn place_ofile(&mut self, fd: i32, fp: FileRef<G>, flags: u8) {
        kassert!(fp.f_iflags.load(Ordering::Relaxed) & FIF_INSERTED != 0);
        let mut s = self.fdp.slots.lock();
        s.slot(fd as usize).fill(fp);
        s.set_flags(fd as usize, flags);
    }

    /// `fd_inuse(fdp, fd)`.
    pub fn fd_inuse(&self, fd: i32) -> bool {
        self.st.fd_inuse(fd)
    }

    /// `fd_used(fdp, fd)`: marks descriptor `fd` allocated.
    pub fn fd_used(&mut self, fd: i32) {
        self.st.fd_used(fd, &self.fdp.fd_openfd);
    }

    /// `fd_unused(fdp, fd)`: marks descriptor `fd` free.
    pub fn fd_unused(&mut self, fd: i32) {
        self.st.fd_unused(fd, &self.fdp.fd_openfd);
    }
}

/// `fdplock(fdp)`: takes the table's lock for writing (`NET_ASSERT_UNLOCKED()`: the network
/// lock is `bsd`'s; a holder that sleeps here under it is caught there).
pub fn fdplock<G: FileGlue>(fdp: &Filedesc<G>) -> FdGuard<'_, G> {
    FdGuard {
        fdp,
        st: fdp.lock.write(),
    }
}

/// `fdpunlock(fdp)`.
pub fn fdpunlock<G: FileGlue>(g: FdGuard<'_, G>) {
    drop(g);
}

/// `fdpassertlocked(fdp)`: the running thread holds the table for writing.
pub fn fdpassertlocked<G: FileGlue>(fdp: &Filedesc<G>) {
    if fdp.lock.status() != RW_WRITE {
        kpanic(format_args!("fdpassertlocked: fdlock not held"));
    }
}

/// The first clear bit at or after bit `want` in the first `NDLOSLOTS(bits)` words of the
/// map `word` reads (`find_next_zero(bitmap, want, bits)`), or -1.
fn find_next_zero(word: impl Fn(usize) -> u32, want: i32, bits: u32) -> i32 {
    if want as u32 > bits {
        return -1;
    }
    let mut off = (want as u32 >> NDENTRYSHIFT) as usize;
    let i = want as u32 & NDENTRYMASK as u32;
    let sub = 'found: {
        if i != 0 {
            let sub = word(off) | (!0u32 >> (NDENTRIES as u32 - i));
            if sub != !0 {
                break 'found sub;
            }
            off += 1;
        }
        let maxoff = ndloslots(bits as usize);
        while off < maxoff {
            let sub = word(off);
            if sub != !0 {
                break 'found sub;
            }
            off += 1;
        }
        return -1;
    };
    ((off << NDENTRYSHIFT) as u32 + (!sub).trailing_zeros()) as i32
}

/// A map's word, one past its end reading as full (see the deviations).
fn word_or_full(map: &[u32], off: usize) -> u32 {
    map.get(off).copied().unwrap_or(!0)
}

/// `find_last_set(fd, last)`: the highest descriptor below `last` that is in use, or 0.
fn find_last_set(st: &FdState, last: i32) -> i32 {
    let mut off = (last - 1) >> NDENTRYSHIFT;
    while off >= 0 && st.lomap(off as usize) == 0 {
        off -= 1;
    }
    if off < 0 {
        return 0;
    }
    let mut i = ((off + 1) << NDENTRYSHIFT) - 1;
    if i >= last {
        i = last - 1;
    }
    while i > 0 && !st.fd_inuse(i) {
        i -= 1;
    }
    i
}

/// `fd_iterfile(fp, p)`: the open file after `prev` (the first one when `None`) in
/// `filehead`, with a reference taken; the reference on `prev` is dropped. A file whose last
/// reference is gone (its close under way) is skipped, as the C's `f_count == 0`.
pub fn fd_iterfile<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    prev: Option<FileRef<G>>,
    p: &Thread<P>,
) -> Option<FileRef<G>> {
    let t = G::tables();
    let next = {
        let g = t.fhdlk.lock();
        let mut n = match &prev {
            None => t.filehead.first(&g),
            Some(fp) => t.filehead.next(&g, fp),
        };
        loop {
            match n {
                None => break None,
                Some(f) => match FileRef::try_from_ref(f) {
                    Some(r) => break Some(r),
                    None => n = t.filehead.next(&g, f),
                },
            }
        }
    };
    if let Some(fp) = prev {
        let _ = fp.frele(Some(p));
    }
    next
}

/// `fd_getfile(fdp, fd)`: the file at descriptor `fd`, with a reference taken, or `None`.
pub fn fd_getfile<G: FileGlue>(fdp: &Filedesc<G>, fd: i32) -> Option<FileRef<G>> {
    vfs_stall_barrier::<G>();
    fdp.slots.lock().file(fd as u32 as usize).cloned()
}

/// `fd_getfile` and the descriptor's flags, read together under `fd_fplock`
/// (`dofilewritev`'s `UF_PLEDGEOPEN`).
pub fn fd_getfile_flags<G: FileGlue>(fdp: &Filedesc<G>, fd: i32) -> Option<(FileRef<G>, u8)> {
    vfs_stall_barrier::<G>();
    let s = fdp.slots.lock();
    let i = fd as u32 as usize;
    s.file(i).cloned().map(|fp| (fp, s.flags(i)))
}

/// `fd_getfile_mode(fdp, fd, mode)`: as [`fd_getfile`], `None` also when the file is not open
/// for `mode` (`FREAD`, `FWRITE`); the reference goes back with `p`, as the C's `FRELE(fp,
/// curproc)`.
pub fn fd_getfile_mode<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    mode: i32,
) -> Option<FileRef<G>> {
    kassert!(mode != 0);
    let fp = fd_getfile(p.fd(), fd)?;
    if fp.flag() & mode == 0 {
        let _ = fp.frele(Some(p));
        return None;
    }
    Some(fp)
}

/// `fd_checkclosed(fdp, fd, fp)`: whether descriptor `fd` no longer refers to `fp`.
pub fn fd_checkclosed<G: FileGlue>(fdp: &Filedesc<G>, fd: i32, fp: &File<G>) -> bool {
    let s = fdp.slots.lock();
    kassert!((fd as u32 as usize) < s.files.as_slice().len());
    !s.file(fd as u32 as usize)
        .is_some_and(|f| core::ptr::eq(&**f, fp))
}

/// Duplicate a file descriptor (`dup(2)`): the new descriptor.
pub fn sys_dup<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    old: i32,
) -> Result<i32, Errno> {
    let fdp = p.fd();
    loop {
        let fp = fd_getfile(fdp, old).ok_or(Errno::EBADF)?;
        let mut g = fdplock(fdp);
        match fdalloc(p, &mut g, 0) {
            Err(Errno::ENOSPC) => {
                let expanded = fdexpand(p, &mut g);
                drop(g);
                let _ = fp.frele(Some(p));
                expanded?;
                // restart
            }
            Err(error) => {
                drop(g);
                let _ = fp.frele(Some(p));
                return Err(error);
            }
            // No need for FRELE(), finishdup() uses current ref.
            Ok(new) => return finishdup(p, g, fp, old, new, 0),
        }
    }
}

/// Duplicate a file descriptor to a particular value (`dup2(2)`).
pub fn sys_dup2<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    from: i32,
    to: i32,
) -> Result<i32, Errno> {
    dodup3(p, from, to, 0)
}

/// `dup3(2)`: `dup2` with `O_CLOEXEC`/`O_CLOFORK`, refusing `from == to`.
pub fn sys_dup3<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    from: i32,
    to: i32,
    flags: i32,
) -> Result<i32, Errno> {
    if from == to {
        return Err(Errno::EINVAL);
    }
    if flags & !(O_CLOEXEC | O_CLOFORK) != 0 {
        return Err(Errno::EINVAL);
    }
    dodup3(p, from, to, flags)
}

/// `dodup3(p, old, new, flags)`: the common part of `dup2` and `dup3`.
pub fn dodup3<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    old: i32,
    new: i32,
    flags: i32,
) -> Result<i32, Errno> {
    let fdp = p.fd();
    loop {
        let fp = fd_getfile(fdp, old).ok_or(Errno::EBADF)?;
        if old == new {
            // NOTE! This doesn't clear the close-on-exec flag. This might or might not be
            // the intended behavior from the start, but this is what everyone else does.
            let _ = fp.frele(Some(p));
            return Ok(new);
        }
        if u64::from(new as u32) >= p.lim_cur(RLIMIT_NOFILE) || new as u32 >= G::maxfiles() as u32 {
            let _ = fp.frele(Some(p));
            return Err(Errno::EBADF);
        }
        let mut g = fdplock(fdp);
        if new as usize >= g.st.nfiles {
            match fdalloc(p, &mut g, new) {
                Err(Errno::ENOSPC) => {
                    let expanded = fdexpand(p, &mut g);
                    drop(g);
                    let _ = fp.frele(Some(p));
                    expanded?;
                    continue; // restart
                }
                Err(error) => {
                    drop(g);
                    let _ = fp.frele(Some(p));
                    return Err(error);
                }
                Ok(i) => {
                    if new != i {
                        kpanic(format_args!("dup2: fdalloc"));
                    }
                    g.fd_unused(new);
                }
            }
        }

        let mut dupflags = DUPF_DUP2;
        if flags & O_CLOEXEC != 0 {
            dupflags |= DUPF_CLOEXEC;
        }
        if flags & O_CLOFORK != 0 {
            dupflags |= DUPF_CLOFORK;
        }

        // No need for FRELE(), finishdup() uses current ref.
        return finishdup(p, g, fp, old, new, dupflags);
    }
}

/// The `l_whence == SEEK_CUR` adjustment `fcntl`'s record locks make against the offset.
fn flock_seek_cur<G: FileGlue>(fp: &File<G>, fl: &mut Flock) {
    if i32::from(fl.l_whence) == SEEK_CUR {
        let offset = foffset(fp);
        if fl.l_start == 0 && fl.l_len < 0 {
            // lockf(3) compliance hack
            fl.l_len = -fl.l_len;
            fl.l_start = offset - fl.l_len;
        } else {
            fl.l_start += offset;
        }
    }
}

/// `(*fp->f_ops->fo_ioctl)(fp, com, (caddr_t)&tmp, p)`: the error and `tmp` as the ioctl
/// left it.
fn fo_ioctl_int<G: FileGlue>(
    fp: &File<G>,
    com: u64,
    tmp: i32,
    p: &Thread<G::Proc>,
) -> (Result<(), Errno>, i32) {
    let mut data = tmp.to_ne_bytes();
    let error = fp.ops().ioctl(fp, com, &mut data, p);
    (error, i32::from_ne_bytes(data))
}

/// A vnode file's object (`fp->f_data`): the file is installed.
fn data_of<G: FileGlue>(fp: &File<G>) -> &G::Data {
    match fp.data() {
        Some(d) => d,
        None => kpanic(format_args!("file {:#x}: no f_data", fp.addr())),
    }
}

/// `struct flock` from userland.
fn flock_copyin(arg: usize) -> Result<Flock, Errno> {
    let bytes = UserPtr::<[u8; Flock::SIZE]>::new(arg).copyin()?;
    Ok(Flock::from_bytes(&bytes))
}

/// The file control system call (`fcntl(2)`): `retval`.
pub fn sys_fcntl<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    cmd: i32,
    arg: usize,
) -> Result<Register, Errno> {
    let fdp = p.fd();
    let mut flg = F_POSIX;

    G::pledge_fcntl(p, cmd)?;

    'restart: loop {
        let fp = fd_getfile(fdp, fd).ok_or(Errno::EBADF)?;
        let mut retval: Register = 0;
        let error: Result<(), Errno> = 'out: {
            match cmd {
                F_DUPFD | F_DUPFD_CLOEXEC | F_DUPFD_CLOFORK => {
                    let newmin = arg as isize as i32;
                    if u64::from(newmin as u32) >= p.lim_cur(RLIMIT_NOFILE)
                        || newmin as u32 >= G::maxfiles() as u32
                    {
                        break 'out Err(Errno::EINVAL);
                    }
                    let mut g = fdplock(fdp);
                    match fdalloc(p, &mut g, newmin) {
                        Err(Errno::ENOSPC) => {
                            let expanded = fdexpand(p, &mut g);
                            drop(g);
                            let _ = fp.frele(Some(p));
                            expanded?;
                            continue 'restart;
                        }
                        Err(error) => {
                            drop(g);
                            let _ = fp.frele(Some(p));
                            return Err(error);
                        }
                        Ok(i) => {
                            let mut dupflags = 0;
                            if cmd == F_DUPFD_CLOEXEC {
                                dupflags |= DUPF_CLOEXEC;
                            }
                            if cmd == F_DUPFD_CLOFORK {
                                dupflags |= DUPF_CLOFORK;
                            }
                            // No need for FRELE(), finishdup() uses current ref.
                            return finishdup(p, g, fp, fd, i, dupflags).map(|fd| fd as Register);
                        }
                    }
                }

                F_GETFD => {
                    let g = fdplock(fdp);
                    let flags = g.ofileflags(fd);
                    retval = (if flags & UF_EXCLOSE != 0 {
                        FD_CLOEXEC
                    } else {
                        0
                    } | if flags & UF_FORKCLOSE != 0 {
                        FD_CLOFORK
                    } else {
                        0
                    }) as Register;
                    drop(g);
                    Ok(())
                }

                F_SETFD => {
                    let mut g = fdplock(fdp);
                    let arg = arg as isize;
                    let i = (if arg & FD_CLOEXEC as isize != 0 {
                        UF_EXCLOSE
                    } else {
                        0
                    }) | (if arg & FD_CLOFORK as isize != 0 {
                        UF_FORKCLOSE
                    } else {
                        0
                    });
                    let flags = (g.ofileflags(fd) & !(UF_EXCLOSE | UF_FORKCLOSE)) | i;
                    g.set_ofileflags(fd, flags);
                    drop(g);
                    Ok(())
                }

                F_GETFL => {
                    retval = oflags(fp.flag()) as Register;
                    Ok(())
                }

                F_ISATTY => {
                    if fp.f_type() == DTYPE_VNODE && G::isatty(data_of(&fp)) {
                        retval = 1;
                        Ok(())
                    } else {
                        retval = 0;
                        Err(Errno::ENOTTY)
                    }
                }

                F_SETFL => {
                    let mut prev = fp.f_flag.load(Ordering::SeqCst);
                    loop {
                        let tmp = (prev & !(FCNTLFLAGS as u32))
                            | (fflags(arg as isize as i32) & FCNTLFLAGS) as u32;
                        match fp.f_flag.compare_exchange(
                            prev,
                            tmp,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            Ok(_) => break,
                            Err(cur) => prev = cur,
                        }
                    }
                    let tmp = fp.flag() & FASYNC;
                    fo_ioctl_int(&fp, FIOASYNC, tmp, p).0
                }

                F_GETOWN => {
                    let (error, tmp) = fo_ioctl_int(&fp, FIOGETOWN, 0, p);
                    retval = tmp as Register;
                    error
                }

                F_SETOWN => fo_ioctl_int(&fp, FIOSETOWN, arg as isize as i32, p).0,

                F_SETLKW | F_SETLK => {
                    if cmd == F_SETLKW {
                        flg |= F_WAIT;
                    }
                    if let Err(error) = G::pledge_flock(p) {
                        break 'out Err(error);
                    }
                    if fp.f_type() != DTYPE_VNODE {
                        break 'out Err(Errno::EINVAL);
                    }
                    let vd = data_of(&fp);
                    // Copy in the lock structure
                    let mut fl = match flock_copyin(arg) {
                        Ok(fl) => fl,
                        Err(e) => break 'out Err(e),
                    };
                    flock_seek_cur(&fp, &mut fl);
                    let id = LockOwner::Table(fdp.addr());
                    let error = match fl.l_type {
                        F_RDLCK => {
                            if fp.flag() & FREAD == 0 {
                                break 'out Err(Errno::EBADF);
                            }
                            fdp.fd_flags.fetch_or(FD_ADVLOCK, Ordering::SeqCst);
                            G::advlock(vd, id, F_SETLK, &mut fl, flg)
                        }
                        F_WRLCK => {
                            if fp.flag() & FWRITE == 0 {
                                break 'out Err(Errno::EBADF);
                            }
                            fdp.fd_flags.fetch_or(FD_ADVLOCK, Ordering::SeqCst);
                            G::advlock(vd, id, F_SETLK, &mut fl, flg)
                        }
                        F_UNLCK => {
                            break 'out G::advlock(vd, id, i32::from(F_UNLCK), &mut fl, F_POSIX);
                        }
                        _ => break 'out Err(Errno::EINVAL),
                    };

                    if fd_checkclosed(fdp, fd, &fp) {
                        // We have lost the race with close() or dup2(); unlock, pretend that
                        // we've won the race and that lock had been removed by close()
                        fl.l_whence = SEEK_SET as i16;
                        fl.l_start = 0;
                        fl.l_len = 0;
                        let _ = G::advlock(vd, id, i32::from(F_UNLCK), &mut fl, F_POSIX);
                        // fl.l_type = F_UNLCK: F_SETLK copies nothing out, so the C's
                        // assignment is never read.
                    }
                    error
                }

                F_GETLK => {
                    if let Err(error) = G::pledge_flock(p) {
                        break 'out Err(error);
                    }
                    if fp.f_type() != DTYPE_VNODE {
                        break 'out Err(Errno::EINVAL);
                    }
                    let vd = data_of(&fp);
                    // Copy in the lock structure
                    let mut fl = match flock_copyin(arg) {
                        Ok(fl) => fl,
                        Err(e) => break 'out Err(e),
                    };
                    flock_seek_cur(&fp, &mut fl);
                    if fl.l_type != F_RDLCK
                        && fl.l_type != F_WRLCK
                        && fl.l_type != F_UNLCK
                        && fl.l_type != 0
                    {
                        break 'out Err(Errno::EINVAL);
                    }
                    let id = LockOwner::Table(fdp.addr());
                    if let Err(e) = G::advlock(vd, id, F_GETLK, &mut fl, F_POSIX) {
                        break 'out Err(e);
                    }
                    UserPtr::<[u8; Flock::SIZE]>::new(arg).copyout(&fl.to_bytes())
                }

                _ => Err(Errno::EINVAL),
            }
        };
        let _ = fp.frele(Some(p));
        return error.map(|()| retval);
    }
}

/// Common code for `dup`, `dup2`, and `fcntl(F_DUPFD)`: `fp` (whose reference it takes) at
/// descriptor `new`, which it returns. Called with the table locked; returns with it
/// unlocked; a file `new` held is closed after the unlock.
pub fn finishdup<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    mut g: FdGuard<'_, G>,
    fp: FileRef<G>,
    old: i32,
    new: i32,
    dupflags: i32,
) -> Result<i32, Errno> {
    kassert!(fp.f_iflags.load(Ordering::Relaxed) & FIF_INSERTED != 0);

    if fp.count() >= FDUP_MAX_COUNT {
        drop(g);
        let _ = fp.frele(Some(p));
        return Err(Errno::EDEADLK);
    }

    // Under `fd_fplock`, so that fd_getfile() no longer creates a new reference to the old
    // file: the slot's reference to it comes out here.
    let fdp = g.fdp;
    let oldfp = {
        let mut s = fdp.slots.lock();
        if dupflags & DUPF_DUP2 != 0 && s.file(new as usize).is_none() {
            if g.fd_inuse(new) {
                drop(s);
                drop(g);
                let _ = fp.frele(Some(p));
                return Err(Errno::EBUSY);
            }
            g.fd_used(new);
        }
        let oldfp = s.slot(new as usize).replace(fp);
        let mut flags = s.flags(old as usize) & !(UF_EXCLOSE | UF_FORKCLOSE);
        if dupflags & DUPF_CLOEXEC != 0 {
            flags |= UF_EXCLOSE;
        }
        if dupflags & DUPF_CLOFORK != 0 {
            flags |= UF_FORKCLOSE;
        }
        s.set_flags(new as usize, flags);
        oldfp
    };

    match oldfp {
        Some(oldfp) => {
            G::knote_fdclose(p, new);
            drop(g);
            let _ = closef(oldfp, Some(p));
        }
        None => drop(g),
    }
    Ok(new)
}

/// `fdinsert(fdp, fd, flags, fp)`: installs `fp` at descriptor `fd` (allocated by `fdalloc`),
/// the slot taking a reference, and links it in `filehead` the first time (after the file at
/// descriptor 0, where the C puts it).
pub fn fdinsert<G: FileGlue>(g: &mut FdGuard<'_, G>, fd: i32, flags: u8, fp: &FileRef<G>) {
    let t = G::tables();
    // One section of `fd_fplock` for the whole insertion: descriptor 0's file is read in
    // place (no reference taken: the slot keeps it while the mutex is held) and `fhdlk` is
    // taken inside it. That nesting is the only one of the two mutexes (`filehead`'s walkers
    // never take `fd_fplock`, and no file is closed with either held), so it cannot invert.
    let mut s = g.fdp.slots.lock();
    {
        let mut h = t.fhdlk.lock();
        if fp.f_iflags.load(Ordering::SeqCst) & FIF_INSERTED == 0 {
            fp.f_iflags.fetch_or(FIF_INSERTED, Ordering::SeqCst);
            match s.file(0) {
                Some(fq) => t.filehead.insert_after(&mut h, fq, fp.as_pin()),
                None => t.filehead.insert_head(&mut h, fp.as_pin()),
            }
        }
    }

    s.slot(fd as usize).fill(fp.clone());
    let f = s.flags(fd as usize) | (flags & (UF_EXCLOSE | UF_FORKCLOSE | UF_PLEDGEOPEN));
    s.set_flags(fd as usize, f);
}

/// `fdremove(fdp, fd)`: empties descriptor `fd` and frees its number; the slot's reference
/// comes back, for the caller to drop after the unlock (`closef`).
pub fn fdremove<G: FileGlue>(g: &mut FdGuard<'_, G>, fd: i32) -> Option<FileRef<G>> {
    // Under `fd_fplock`, so that fd_getfile() no longer creates a new reference to the file.
    let fp = {
        let mut s = g.fdp.slots.lock();
        let fp = s.slot(fd as usize).take();
        s.set_flags(fd as usize, 0);
        fp
    };
    g.fd_unused(fd);
    fp
}

/// `fdrelease(p, fd)`: closes descriptor `fd`. Called with the table locked; returns with it
/// unlocked.
pub fn fdrelease<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    mut g: FdGuard<'_, G>,
    fd: i32,
) -> Result<(), Errno> {
    vfs_stall_barrier::<G>();
    // fd_getfile's check and fdremove's slot, in one section of `fd_fplock`.
    let fp = {
        let mut s = g.fdp.slots.lock();
        let i = fd as u32 as usize;
        if s.file(i).is_none() {
            None
        } else {
            let fp = s.slot(i).take();
            s.set_flags(i, 0);
            fp
        }
    };
    let Some(fp) = fp else {
        drop(g);
        return Err(Errno::EBADF);
    };
    g.fd_unused(fd);
    G::knote_fdclose(p, fd);
    drop(g);
    closef(fp, Some(p))
}

/// Close a file descriptor (`close(2)`).
pub fn sys_close<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
) -> Result<(), Errno> {
    let g = fdplock(p.fd());
    // fdrelease unlocks fdp.
    fdrelease(p, g, fd)
}

/// Return status information about a file descriptor (`fstat(2)`).
pub fn sys_fstat<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    sb: usize,
) -> Result<(), Errno> {
    let fp = fd_getfile(p.fd(), fd).ok_or(Errno::EBADF)?;
    let mut ub = G::Stat::default();
    let error = fp.ops().stat(&fp, &mut ub, p);
    let _ = fp.frele(Some(p));
    error?;
    // Don't let non-root see generation numbers (for NFS security)
    if suser(p).is_err() {
        G::stat_hide_gen(&mut ub);
    }
    G::stat_copyout(&ub, sb)
}

/// Return pathconf information about a file descriptor (`fpathconf(2)`).
pub fn sys_fpathconf<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    name: i32,
) -> Result<Register, Errno> {
    let fp = fd_getfile(p.fd(), fd).ok_or(Errno::EBADF)?;
    let error = match fp.f_type() {
        DTYPE_PIPE | DTYPE_SOCKET => {
            if name != _PC_PIPE_BUF {
                Err(Errno::EINVAL)
            } else {
                Ok(PIPE_BUF as Register)
            }
        }
        DTYPE_VNODE => G::pathconf(data_of(&fp), name),
        _ => Err(Errno::EOPNOTSUPP),
    };
    let _ = fp.frele(Some(p));
    error
}

/// Allocate a file descriptor for the process: the lowest free one at or above `want`.
/// `ENOSPC` asks the caller to `fdexpand` and retry; `EMFILE` is the limit. A pledged
/// process's new descriptor is `UF_PLEDGED`.
pub fn fdalloc<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    g: &mut FdGuard<'_, G>,
    want: i32,
) -> Result<i32, Errno> {
    let lim = (p.lim_cur(RLIMIT_NOFILE) as i32).min(G::maxfiles());
    let pledged = p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0;
    fdalloc_search(g, want, lim, pledged)
}

/// The search of `fdalloc` once the limit is known.
fn fdalloc_search<G: FileGlue>(
    g: &mut FdGuard<'_, G>,
    want: i32,
    lim: i32,
    pledged: bool,
) -> Result<i32, Errno> {
    let mut want = want;
    // Search for a free descriptor starting at the higher of want or fd_freefile. If that
    // fails, consider expanding the ofile array.
    loop {
        let nfiles = g.st.nfiles as i32;
        let last = nfiles.min(lim);
        let mut i = want;
        if i < g.st.freefile {
            i = g.st.freefile;
        }
        let off = i >> NDENTRYSHIFT;
        let new = {
            let hi = g.st.himap.as_slice();
            find_next_zero(
                |o| word_or_full(hi, o),
                off,
                ((last + NDENTRIES as i32 - 1) >> NDENTRYSHIFT) as u32,
            )
        };
        if new != -1 {
            let j = {
                let lo = g.st.lomap.as_slice();
                find_next_zero(
                    |o| word_or_full(lo, new as usize + o),
                    if new > off { 0 } else { i & NDENTRYMASK as i32 },
                    NDENTRIES as u32,
                )
            };
            if j == -1 {
                // Free file descriptor in this block was below want, try again with higher
                // want.
                want = (new + 1) << NDENTRYSHIFT;
                continue;
            }
            let i = j + (new << NDENTRYSHIFT);
            if i < last {
                g.fd_used(i);
                if want <= g.st.freefile {
                    g.st.freefile = i;
                }
                // A section of `fd_fplock`, where the C stores `fd_ofileflags[i]` plainly
                // under fdplock: the flags live with the slots inside that mutex because the
                // C also reads them with no lock at all (`UF_PLEDGEOPEN` in `dofilewritev`,
                // `fchmod`, `fchown`, `futimens`, `ftruncate`; `unp_internalize` reads them
                // under fdplock), and those readers take the mutex here instead of racing.
                // Under fdlock alone they would have to take that sleeping lock, a new sleep
                // point on the write path.
                g.set_ofileflags(i, if pledged { UF_PLEDGED } else { 0 });
                return Ok(i);
            }
        }
        if nfiles >= lim {
            return Err(Errno::EMFILE);
        }
        return Err(Errno::ENOSPC);
    }
}

/// `fdexpand(p)`: grows the table to `NDEXTENT` descriptors, then doubles it. The new arrays
/// are made first (the allocation may sleep, the table locked, its arrays still valid),
/// swapped in under `fd_fplock`, and the old ones freed after. `ENOMEM` when `malloc(9)`
/// answers nothing (see the deviations).
pub fn fdexpand<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    _p: &Thread<P>,
    g: &mut FdGuard<'_, G>,
) -> Result<(), Errno> {
    let oldnfiles = g.st.nfiles;
    // No space in current array.
    let nfiles = if oldnfiles < NDEXTENT {
        NDEXTENT
    } else {
        2 * oldnfiles
    };

    let newofile: MArray<FileSlot<G>> =
        MArray::new(M_FILEDESC, M_WAITOK, nfiles, |_| FileSlot::new()).ok_or(Errno::ENOMEM)?;
    let newflags: MArray<u8> =
        MArray::new(M_FILEDESC, M_WAITOK, nfiles, |_| 0).ok_or(Errno::ENOMEM)?;
    // Allocate all required chunks before the swap, so that the arrays stay valid if we go
    // to sleep.
    let maps = if ndhislots(nfiles) > ndhislots(oldnfiles) {
        let hi = g.st.himap.as_slice();
        let lo = g.st.lomap.as_slice();
        let newhi = MArray::new(M_FILEDESC, M_WAITOK, ndhislots(nfiles), |i| {
            hi.get(i).copied().unwrap_or(0)
        })
        .ok_or(Errno::ENOMEM)?;
        let newlo = MArray::new(M_FILEDESC, M_WAITOK, ndloslots(nfiles), |i| {
            lo.get(i).copied().unwrap_or(0)
        })
        .ok_or(Errno::ENOMEM)?;
        Some((newhi, newlo))
    } else {
        None
    };

    // Copy the existing ofile and ofileflags arrays (the new portion is empty) and swap them
    // in under `fd_fplock`; the old ones are freed once it is released.
    let (oldfiles, oldflags) = {
        let mut s = g.fdp.slots.lock();
        let mut newofile = newofile;
        let mut newflags = newflags;
        for (to, from) in newofile.iter_mut().zip(s.files.as_mut_slice().iter_mut()) {
            core::mem::swap(to, from);
        }
        newflags[..oldnfiles].copy_from_slice(&s.flags.as_slice()[..oldnfiles]);
        (
            core::mem::replace(&mut s.files, Store::Heap(newofile)),
            core::mem::replace(&mut s.flags, Store::Heap(newflags)),
        )
    };
    if let Some((newhi, newlo)) = maps {
        g.st.himap = Store::Heap(newhi);
        g.st.lomap = Store::Heap(newlo);
    }
    g.st.nfiles = nfiles;
    drop(oldfiles);
    drop(oldflags);
    Ok(())
}

/// Create a new open file structure and allocate a file descriptor for the process that
/// refers to it: the file, with the reference the caller holds (the table takes its own at
/// `fdinsert`), and the descriptor. Called with the table locked.
pub fn falloc<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    g: &mut FdGuard<'_, G>,
) -> Result<(FileRef<G>, i32), Errno> {
    let i = loop {
        match fdalloc(p, g, 0) {
            Ok(i) => break i,
            Err(Errno::ENOSPC) => fdexpand(p, g)?,
            Err(error) => return Err(error),
        }
    };
    match fnew(p) {
        Some(fp) => Ok((fp, i)),
        None => {
            g.fd_unused(i);
            Err(Errno::ENFILE)
        }
    }
}

/// `filedesc_init`: the pools of open files (`filepl`, `IPL_MPFLOOR`, recorded) and of
/// descriptor tables (`fdescpl`, `IPL_NONE`), both `PR_WAITOK`.
pub fn filedesc_init<G: FileGlue>() -> Result<(), Errno> {
    let t = G::tables();
    t.filepl
        .init_recorded(hal::intr::mpfloor(), PR_WAITOK, "filepl")?;
    t.fdescpl.init(hal::intr::IPL_NONE, PR_WAITOK, "fdescpl")
}

/// Build a new filedesc structure (`fdinit`).
pub fn fdinit<G: FileGlue>() -> FdRef<G> {
    fd_pooled(Filedesc::new())
}

/// The table in a `fdescpl` item.
fn fd_pooled<G: FileGlue>(fdp: Filedesc<G>) -> FdRef<G> {
    match G::tables().fdescpl.get_shared(PR_WAITOK, move || fdp) {
        Some(a) => FdRef(a),
        None => kpanic(format_args!("fdinit: fdesc_pool is empty")),
    }
}

/// Share a filedesc structure (`fdshare`): one more process on `pr`'s table.
pub fn fdshare<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(pr: &Process<P>) -> FdRef<G> {
    let fdp = pr.fd_ref();
    fdp.shares.fetch_add(1, Ordering::AcqRel);
    fdp.clone()
}

/// Copy a filedesc structure (`fdcopy`): `pr`'s table without its `UF_FORKCLOSE`
/// descriptors, its kqueues, and the files referenced too often to duplicate.
pub fn fdcopy<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(pr: &Process<P>) -> FdRef<G> {
    let fdp = pr.fd();
    let g = fdplock(fdp);

    let mut st = FdState::new();
    let mut slots = Slots::<G>::new();
    let lastfile = g.st.lastfile;
    // If the number of open files fits in the internal arrays of the open file structure,
    // use them, otherwise allocate additional memory for the number of descriptors
    // currently in use.
    if lastfile >= NDFILE as i32 {
        // Compute the smallest multiple of NDEXTENT needed for the file descriptors
        // currently in use, allowing the table to shrink.
        let mut i = g.st.nfiles;
        while i >= 2 * NDEXTENT && i > lastfile as usize * 2 {
            i /= 2;
        }
        let (Some(files), Some(flags)) = (
            MArray::new(M_FILEDESC, M_WAITOK, i, |_| FileSlot::new()),
            MArray::new(M_FILEDESC, M_WAITOK, i, |_| 0u8),
        ) else {
            kpanic(format_args!("fdcopy: no memory for {i} descriptors"));
        };
        slots.files = Store::Heap(files);
        slots.flags = Store::Heap(flags);
        st.nfiles = i;
    }
    if ndhislots(st.nfiles) > ndhislots(NDFILE) {
        let n = st.nfiles;
        let (Some(hi), Some(lo)) = (
            MArray::new(M_FILEDESC, M_WAITOK, ndhislots(n), |_| 0u32),
            MArray::new(M_FILEDESC, M_WAITOK, ndloslots(n), |_| 0u32),
        ) else {
            kpanic(format_args!(
                "fdcopy: no memory for the maps of {n} descriptors"
            ));
        };
        st.himap = Store::Heap(hi);
        st.lomap = Store::Heap(lo);
    }
    st.freefile = g.st.freefile;

    // The new table is ours alone until it is returned: its state and slots are filled
    // before it exists, with no lock.
    let openfd = AtomicI32::new(0);
    let mut src = fdp.slots.lock();
    for i in 0..=lastfile {
        // FREF's vfs_stall_barrier: one load; the wait, if any, with the mutex let go (the
        // slots cannot change: the table's lock is held).
        if VFS_STALLING.load(Ordering::Relaxed) != 0 {
            drop(src);
            vfs_stall_barrier::<G>();
            src = fdp.slots.lock();
        }
        let (fp, fileflags) = match src.file(i as usize) {
            Some(fp) => (fp.clone(), src.flags(i as usize)),
            None => continue,
        };
        // If the UF_FORKCLOSE flag is set, skip the fd. XXX Gruesome hack. If count gets too
        // high, fail to copy an fd, since fdcopy()'s callers do not permit it to indicate
        // failure yet. Meanwhile, kqueue files have to be tied to the process that opened
        // them to enforce their internal consistency, so close them here. (The count read
        // includes the reference just taken: the C's `>=` before its `FREF`.)
        if fp.count() > FDUP_MAX_COUNT
            || fileflags & UF_FORKCLOSE != 0
            || fp.f_type() == DTYPE_KQUEUE
        {
            if i < st.freefile {
                st.freefile = i;
            }
            drop(fp);
            continue;
        }
        slots.slot(i as usize).fill(fp);
        slots.set_flags(i as usize, fileflags);
        st.fd_used(i, &openfd);
    }
    drop(src);

    let newfdp = Filedesc::with(st, slots);
    newfdp
        .fd_openfd
        .store(openfd.load(Ordering::Relaxed), Ordering::Relaxed);
    if let Some(cdir) = fdp.cdir() {
        G::vref(cdir);
        newfdp.cdir.set(Some(cdir));
    }
    if let Some(rdir) = fdp.rdir() {
        G::vref(rdir);
        newfdp.rdir.set(Some(rdir));
    }
    newfdp
        .fd_flags
        .store(fdp.fd_flags.load(Ordering::SeqCst), Ordering::SeqCst);
    newfdp.cmask.store(fdp.cmask(), Ordering::Relaxed);
    drop(g);

    fd_pooled(newfdp)
}

/// Release a filedesc structure (`fdfree`): the last process sharing `p`'s table closes its
/// descriptors (`knote_fdclose` first) and releases its directories; the table itself goes
/// back with the last handle (see the deviations).
pub fn fdfree<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(p: &Thread<P>) {
    let fdp = p.fd();
    if fdp.shares.fetch_sub(1, Ordering::AcqRel) != 1 {
        return;
    }
    let lastfile = fdp.with_state(|st| st.lastfile);
    for fd in 0..=lastfile {
        let fp = fdp.slots.lock().slot(fd as usize).take();
        if let Some(fp) = fp {
            G::knote_fdclose(p, fd);
            let _ = closef(fp, Some(p));
        }
    }
    // The emptied table: its arrays go back once both locks are released.
    let (old_st, old_slots) = {
        let mut g = fdplock(fdp);
        let old_st = core::mem::replace(&mut *g.st, FdState::new());
        let old_slots = core::mem::replace(&mut *g.fdp.slots.lock(), Slots::new());
        (old_st, old_slots)
    };
    drop(old_st);
    drop(old_slots);
    fdp.fd_openfd.store(0, Ordering::Relaxed);
    if let Some(cdir) = fdp.cdir.get() {
        fdp.cdir.set(None);
        G::vrele(cdir);
    }
    if let Some(rdir) = fdp.rdir.get() {
        fdp.rdir.set(None);
        G::vrele(rdir);
    }
    G::fdkq_free(&fdp.kq);
}

/// Internal form of close (`closef`): the reference a slot (or a message) held. POSIX record
/// locking dictates that any close releases ALL locks owned by this process: a thread's close
/// of a vnode file whose table may hold one unlocks the table's POSIX locks (not `flock(2)`'s,
/// owned by the file); a file closed with no thread (it was in a message) does not. Then the
/// reference goes (`FRELE`).
pub fn closef<G: FileGlue>(fp: FileRef<G>, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
    if let Some(p) = p
        && let Some(fdp) = p.process().fd_opt()
        && fdp.fd_flags.load(Ordering::SeqCst) & FD_ADVLOCK != 0
        && fp.f_type() == DTYPE_VNODE
    {
        let mut lf = Flock {
            l_whence: SEEK_SET as i16,
            l_start: 0,
            l_len: 0,
            l_type: F_UNLCK,
            ..Flock::default()
        };
        let id = LockOwner::Table(fdp.addr());
        let _ = G::advlock(data_of(&fp), id, i32::from(F_UNLCK), &mut lf, F_POSIX);
    }
    fp.frele(p)
}

/// Apply an advisory lock on a file descriptor (`flock(2)`): a record lock of the requested
/// type on the entire file, owned by the file.
pub fn sys_flock<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    how: i32,
) -> Result<(), Errno> {
    let fp = fd_getfile(p.fd(), fd).ok_or(Errno::EBADF)?;
    let error = 'out: {
        if fp.f_type() != DTYPE_VNODE {
            break 'out Err(Errno::EOPNOTSUPP);
        }
        let vd = data_of(&fp);
        let id = LockOwner::File(fp.addr());
        let mut lf = Flock {
            l_whence: SEEK_SET as i16,
            l_start: 0,
            l_len: 0,
            ..Flock::default()
        };
        if how & LOCK_UN != 0 {
            lf.l_type = F_UNLCK;
            fp.f_iflags.fetch_and(!FIF_HASLOCK, Ordering::SeqCst);
            break 'out G::advlock(vd, id, i32::from(F_UNLCK), &mut lf, F_FLOCK);
        }
        if how & LOCK_EX != 0 {
            lf.l_type = F_WRLCK;
        } else if how & LOCK_SH != 0 {
            lf.l_type = F_RDLCK;
        } else {
            break 'out Err(Errno::EINVAL);
        }
        fp.f_iflags.fetch_or(FIF_HASLOCK, Ordering::SeqCst);
        if how & LOCK_NB != 0 {
            G::advlock(vd, id, F_SETLK, &mut lf, F_FLOCK)
        } else {
            G::advlock(vd, id, F_SETLK, &mut lf, F_FLOCK | F_WAIT)
        }
    };
    let _ = fp.frele(Some(p));
    error
}

/// Duplicate the descriptor `p_dupfd` names into the free descriptor `indx` (an open of
/// `/dev/fd/N`, whose driver set `p_dupfd` and failed with `ENODEV`). Called with the table
/// locked.
pub fn dupfdopen<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    g: &mut FdGuard<'_, G>,
    indx: i32,
    mode: i32,
) -> Result<(), Errno> {
    let dupfd = p.p_dupfd.get();

    // Assume that the filename was user-specified; applications do not tend to open
    // /dev/fd/# when they can just call dup()
    if p.process().ps_flags.load(Ordering::Relaxed) & (PS_SUGIDEXEC | PS_SUGID) != 0 {
        if p.p_descfd.get() == 255 {
            return Err(Errno::EPERM);
        }
        if i32::from(p.p_descfd.get()) != dupfd {
            return Err(Errno::EPERM);
        }
    }

    // If the to-be-dup'd fd number is greater than the allowed number of file descriptors,
    // or the fd to be dup'd has already been closed, reject. Note, there is no need to
    // check for new == old because fd_getfile will return NULL if the file at indx is newly
    // created by falloc.
    let wfp = fd_getfile(g.fdp, dupfd).ok_or(Errno::EBADF)?;

    // Check that the mode the file is being opened for is a subset of the mode of the
    // existing descriptor.
    if ((mode & (FREAD | FWRITE)) | wfp.flag()) != wfp.flag() {
        let _ = wfp.frele(Some(p));
        return Err(Errno::EACCES);
    }
    if wfp.count() >= FDUP_MAX_COUNT {
        let _ = wfp.frele(Some(p));
        return Err(Errno::EDEADLK);
    }

    kassert!(wfp.f_iflags.load(Ordering::SeqCst) & FIF_INSERTED != 0);

    // The reference fd_getfile took becomes the slot's.
    let mut s = g.fdp.slots.lock();
    s.slot(indx as usize).fill(wfp);
    let f = (s.flags(indx as usize) & (UF_EXCLOSE | UF_FORKCLOSE))
        | (s.flags(dupfd as usize) & !(UF_EXCLOSE | UF_FORKCLOSE));
    s.set_flags(indx as usize, f);
    Ok(())
}

/// Doing an exec, so handle fd flags: do close-on-exec and clear pledged and close-on-fork
/// (`fdprepforexec`).
pub fn fdprepforexec<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(p: &Thread<P>) {
    let fdp = p.fd();
    let mut g = fdplock(fdp);
    let mut fd = 0;
    loop {
        // The flags of the descriptors up to the next close-on-exec one, under one section
        // of `fd_fplock` (`fd_lastfile` read under the table's lock, as the C's loop).
        let last = g.st.lastfile;
        let close = {
            let mut s = fdp.slots.lock();
            let flags = s.flags.as_mut_slice();
            // `fd_lastfile` is below `fd_nfiles` (-1 for an empty table).
            let end = usize::try_from(last.saturating_add(1))
                .unwrap_or(0)
                .min(flags.len());
            let start = usize::try_from(fd).unwrap_or(0).min(end);
            flags[start..end]
                .iter_mut()
                .position(|f| {
                    *f &= !(UF_PLEDGED | UF_FORKCLOSE);
                    *f & UF_EXCLOSE != 0
                })
                .map(|k| (start + k) as i32)
        };
        let Some(cfd) = close else {
            break;
        };
        // fdrelease() unlocks fdp.
        let _ = fdrelease(p, g, cfd);
        g = fdplock(fdp);
        fd = cfd + 1;
    }
    drop(g);
}

/// `closefrom(2)`: closes every descriptor from `fd` on.
pub fn sys_closefrom<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
) -> Result<(), Errno> {
    let fdp = p.fd();
    let startfd = fd as u32;
    let mut g = fdplock(fdp);

    if startfd > g.st.lastfile as u32 {
        drop(g);
        return Err(Errno::EBADF);
    }

    let mut i = startfd;
    while i <= g.st.lastfile as u32 {
        // fdrelease() unlocks fdp.
        let _ = fdrelease(p, g, i as i32);
        g = fdplock(fdp);
        i += 1;
    }
    drop(g);
    Ok(())
}

/// `getdtablecount(2)`: the number of descriptors open.
pub fn sys_getdtablecount<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(p: &Thread<P>) -> i32 {
    p.fd().fd_openfd.load(Ordering::Relaxed)
}

/// The positioned-I/O checks of `dofilereadv`/`dofilewritev` (`FO_POSITION`).
fn position_check<G: FileGlue>(fp: &File<G>, offset: Off) -> Result<(), Errno> {
    if fp.f_type() != DTYPE_VNODE {
        return Err(Errno::ESPIPE);
    }
    G::position_check(data_of(fp), offset)
}

/// The errors that come after some data was moved are dropped: `ERESTART`, `EINTR` and
/// `EWOULDBLOCK` when `uio_resid` is no longer `cnt`.
fn partial_ok(error: Result<(), Errno>, resid: usize, cnt: usize) -> Result<(), Errno> {
    match error {
        Err(Errno::ERESTART | Errno::EINTR | Errno::EAGAIN) if resid != cnt => Ok(()),
        e => e,
    }
}

/// One user buffer as a uio's iovec (`read(2)`, `write(2)`, `pread(2)`, `pwrite(2)`), the
/// length bounded by `SSIZE_MAX`.
fn one_iovec(buf: usize, nbyte: usize) -> Result<[Iovec; 1], Errno> {
    if nbyte > SSIZE_MAX.unsigned_abs() {
        return Err(Errno::EINVAL);
    }
    Ok([Iovec {
        iov_base: core::ptr::without_provenance_mut(buf),
        iov_len: nbyte,
    }])
}

/// A user transfer over `iov` for `p`, `resid` bytes from `offset`.
fn user_uio<'a, G: FileGlue>(
    p: &'a Thread<G::Proc>,
    iov: &'a mut [Iovec],
    resid: usize,
    offset: Off,
    rw: UioRw,
) -> Uio<'a, G::Proc> {
    Uio {
        uio_iov: iov,
        uio_offset: offset,
        uio_resid: resid,
        uio_segflg: UioSeg::UIO_USERSPACE,
        uio_rw: rw,
        uio_procp: Some(p),
    }
}

/// `dofilereadv(p, fd, uio, flags, retval)`: reads descriptor `fd` into the user iovecs of
/// `uio`; the byte count.
pub fn dofilereadv<'a, P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &'a Thread<P>,
    fd: i32,
    uio: &mut Uio<'a, G::Proc>,
    flags: i32,
) -> Result<usize, Errno> {
    kassert!(uio.uio_iovcnt() > 0);

    let fp = fd_getfile_mode(p, fd, FREAD).ok_or(Errno::EBADF)?;
    let error = 'done: {
        // Checks for positioned read.
        if flags & FO_POSITION != 0
            && let Err(e) = position_check(&fp, uio.uio_offset)
        {
            break 'done Err(e);
        }
        uio.uio_rw = UioRw::UIO_READ;
        uio.uio_segflg = UioSeg::UIO_USERSPACE;
        uio.uio_procp = Some(p);
        let cnt = uio.uio_resid;
        let error = fp.ops().read(&fp, uio, flags);
        let error = partial_ok(error, uio.uio_resid, cnt);
        let cnt = cnt - uio.uio_resid;
        {
            let mut s = fp.stats();
            s.f_rxfer += 1;
            s.f_rbytes += cnt as u64;
        }
        error.map(|()| cnt)
    };
    let _ = fp.frele(Some(p));
    error
}

/// `dofilewritev(p, fd, uio, flags, retval)`: writes the user iovecs of `uio` to descriptor
/// `fd`; the byte count. A write to a broken pipe or socket (`EPIPE`) signals the thread.
pub fn dofilewritev<'a, P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &'a Thread<P>,
    fd: i32,
    uio: &mut Uio<'a, G::Proc>,
    flags: i32,
) -> Result<usize, Errno> {
    kassert!(uio.uio_iovcnt() > 0);

    let (fp, fdflags) = fd_getfile_flags(p.fd(), fd).ok_or(Errno::EBADF)?;
    if fp.flag() & FWRITE == 0 {
        let _ = fp.frele(Some(p));
        return Err(Errno::EBADF);
    }
    let error = 'done: {
        if fdflags & UF_PLEDGEOPEN != 0 {
            break 'done Err(Errno::EPERM);
        }
        // Checks for positioned write.
        if flags & FO_POSITION != 0
            && let Err(e) = position_check(&fp, uio.uio_offset)
        {
            break 'done Err(e);
        }
        uio.uio_rw = UioRw::UIO_WRITE;
        uio.uio_segflg = UioSeg::UIO_USERSPACE;
        uio.uio_procp = Some(p);
        let cnt = uio.uio_resid;
        let error = fp.ops().write(&fp, uio, flags);
        let error = partial_ok(error, uio.uio_resid, cnt);
        if error == Err(Errno::EPIPE) {
            G::sigpipe(p);
        }
        let cnt = cnt - uio.uio_resid;
        {
            let mut s = fp.stats();
            s.f_wxfer += 1;
            s.f_wbytes += cnt as u64;
        }
        error.map(|()| cnt)
    };
    let _ = fp.frele(Some(p));
    error
}

/// Read system call (`read(2)`).
pub fn sys_read<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    buf: usize,
    nbyte: usize,
) -> Result<usize, Errno> {
    let mut iov = one_iovec(buf, nbyte)?;
    let mut auio = user_uio::<G>(p, &mut iov, nbyte, 0, UioRw::UIO_READ);
    dofilereadv(p, fd, &mut auio, 0)
}

/// Scatter read system call (`readv(2)`).
pub fn sys_readv<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    iovp: UserPtr<Iovec>,
    iovcnt: i32,
) -> Result<usize, Errno> {
    let mut buf = IovBuf::new();
    let (iov, resid) = iovec_copyin(iovp, &mut buf, iovcnt as u32)?;
    let mut auio = user_uio::<G>(p, iov, resid, 0, UioRw::UIO_READ);
    dofilereadv(p, fd, &mut auio, 0)
}

/// Positional read system call (`pread(2)`).
pub fn sys_pread<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    buf: usize,
    nbyte: usize,
    offset: Off,
) -> Result<usize, Errno> {
    let mut iov = one_iovec(buf, nbyte)?;
    let mut auio = user_uio::<G>(p, &mut iov, nbyte, offset, UioRw::UIO_READ);
    dofilereadv(p, fd, &mut auio, FO_POSITION)
}

/// Positional scatter read system call (`preadv(2)`).
pub fn sys_preadv<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    iovp: UserPtr<Iovec>,
    iovcnt: i32,
    offset: Off,
) -> Result<usize, Errno> {
    let mut buf = IovBuf::new();
    let (iov, resid) = iovec_copyin(iovp, &mut buf, iovcnt as u32)?;
    let mut auio = user_uio::<G>(p, iov, resid, offset, UioRw::UIO_READ);
    dofilereadv(p, fd, &mut auio, FO_POSITION)
}

/// Write system call (`write(2)`).
pub fn sys_write<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    buf: usize,
    nbyte: usize,
) -> Result<usize, Errno> {
    let mut iov = one_iovec(buf, nbyte)?;
    let mut auio = user_uio::<G>(p, &mut iov, nbyte, 0, UioRw::UIO_WRITE);
    dofilewritev(p, fd, &mut auio, 0)
}

/// Gather write system call (`writev(2)`).
pub fn sys_writev<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    iovp: UserPtr<Iovec>,
    iovcnt: i32,
) -> Result<usize, Errno> {
    let mut buf = IovBuf::new();
    let (iov, resid) = iovec_copyin(iovp, &mut buf, iovcnt as u32)?;
    let mut auio = user_uio::<G>(p, iov, resid, 0, UioRw::UIO_WRITE);
    dofilewritev(p, fd, &mut auio, 0)
}

/// Positional write system call (`pwrite(2)`).
pub fn sys_pwrite<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    buf: usize,
    nbyte: usize,
    offset: Off,
) -> Result<usize, Errno> {
    let mut iov = one_iovec(buf, nbyte)?;
    let mut auio = user_uio::<G>(p, &mut iov, nbyte, offset, UioRw::UIO_WRITE);
    dofilewritev(p, fd, &mut auio, FO_POSITION)
}

/// Positional gather write system call (`pwritev(2)`).
pub fn sys_pwritev<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    iovp: UserPtr<Iovec>,
    iovcnt: i32,
    offset: Off,
) -> Result<usize, Errno> {
    let mut buf = IovBuf::new();
    let (iov, resid) = iovec_copyin(iovp, &mut buf, iovcnt as u32)?;
    let mut auio = user_uio::<G>(p, iov, resid, offset, UioRw::UIO_WRITE);
    dofilewritev(p, fd, &mut auio, FO_POSITION)
}

/// Ioctl system call (`ioctl(2)`): the argument copied in and out by the direction and size
/// `com` encodes (at most `IOCPARM_MAX` bytes; the stack's 128, or an `M_IOCTLOPS` buffer).
pub fn sys_ioctl<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    com: u64,
    udata: usize,
) -> Result<(), Errno> {
    let fdp = p.fd();
    let fp = fd_getfile_mode(p, fd, FREAD | FWRITE).ok_or(Errno::EBADF)?;
    let mut stkbuf = [0u8; STK_PARAMS];
    let mut heap: Option<MArray<u8>> = None;

    let error = 'out: {
        if fp.f_type() == DTYPE_SOCKET && G::dns_socket(data_of(&fp)) {
            break 'out Err(Errno::EINVAL);
        }

        if let Err(error) = G::pledge_ioctl(p, com, &fp) {
            break 'out Err(error);
        }

        if com == FIONCLEX || com == FIOCLEX {
            let mut g = fdplock(fdp);
            let flags = g.ofileflags(fd);
            g.set_ofileflags(
                fd,
                if com == FIONCLEX {
                    flags & !UF_EXCLOSE
                } else {
                    flags | UF_EXCLOSE
                },
            );
            drop(g);
            break 'out Ok(());
        }

        // Interpret high order word to find amount of data to be copied to/from the user's
        // address space.
        let size = iocparm_len(com) as usize;
        if size > IOCPARM_MAX {
            break 'out Err(Errno::ENOTTY);
        }
        let data: &mut [u8] = if size > STK_PARAMS {
            match MArray::new(M_IOCTLOPS, M_WAITOK, size, |_| 0u8) {
                Some(m) => &mut heap.insert(m)[..],
                None => break 'out Err(Errno::ENOMEM),
            }
        } else {
            &mut stkbuf[..size.max(size_of::<usize>())]
        };
        let udata_bytes = udata.to_ne_bytes();
        if com & IOC_IN != 0 {
            if size != 0 {
                if let Err(e) = UserPtr::<u8>::new(udata).copyin_slice(&mut data[..size]) {
                    break 'out Err(e);
                }
            } else {
                data[..udata_bytes.len()].copy_from_slice(&udata_bytes);
            }
        } else if com & IOC_OUT != 0 && size != 0 {
            // Zero the buffer so the user always gets back something deterministic.
            data[..size].fill(0);
        } else if com & IOC_VOID != 0 {
            data[..udata_bytes.len()].copy_from_slice(&udata_bytes);
        }

        let int_arg = |data: &[u8]| i32::from_ne_bytes([data[0], data[1], data[2], data[3]]);
        let error = match com {
            FIONBIO => {
                if int_arg(data) != 0 {
                    fp.f_flag.fetch_or(FNONBLOCK as u32, Ordering::SeqCst);
                } else {
                    fp.f_flag.fetch_and(!(FNONBLOCK as u32), Ordering::SeqCst);
                }
                Ok(())
            }
            FIOASYNC => {
                let tmp = int_arg(data);
                if tmp != 0 {
                    fp.f_flag.fetch_or(FASYNC as u32, Ordering::SeqCst);
                } else {
                    fp.f_flag.fetch_and(!(FASYNC as u32), Ordering::SeqCst);
                }
                let mut tmp = tmp.to_ne_bytes();
                fp.ops().ioctl(&fp, FIOASYNC, &mut tmp, p)
            }
            _ => fp.ops().ioctl(&fp, com, data, p),
        };
        // Copy any data to user, size was already set and checked above.
        if error.is_ok() && com & IOC_OUT != 0 && size != 0 {
            UserPtr::<u8>::new(udata).copyout_slice(&data[..size])
        } else {
            error
        }
    };
    let _ = fp.frele(Some(p));
    drop(heap);
    error
}

/// `utrace(2)`: `KTRACE` is not configured, so it succeeds and records nothing (the C's
/// `#else`).
pub fn sys_utrace<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    _p: &Thread<P>,
) -> Result<(), Errno> {
    Ok(())
}

const _: () = {
    assert!(ndhislots(NDFILE) == NDENTRIES >> NDENTRYSHIFT);
    assert!(ndloslots(NDFILE) == NDENTRIES);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::format;
    use std::string::String;
    use std::vec;
    use std::vec::Vec;

    use hal::pool::PoolBox;
    use hal::sys::ioccom::{_ior, _iow, _iowr};

    use super::*;
    use crate::cred::{CredRef, Ucred};
    use crate::file::FileTables;
    use crate::proc::{ProcessRef, process_alloc, procinit, thread_alloc};
    use crate::resource::{Plimit, PlimitRef};
    use crate::testutil::{self, TestFileData, TestStat, TestVnode, file_log};

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    type P = PoolBox<Thread<Procs>>;

    /// The operations of the tests' files, by object id: a read moves up to 5 bytes, then
    /// object 2 fails with `EINTR` (after data) and object 3 before any; a write to object 4
    /// fails with `EPIPE`; an `IOC_OUT` ioctl writes 0xab, any other `ENOTTY`; the close is
    /// recorded.
    struct TOps;

    fn id(fp: &File<Files>) -> usize {
        fp.data().map_or(0, |d| d.id)
    }

    impl FileOps<Files> for TOps {
        fn read(&self, fp: &File<Files>, uio: &mut Uio<'_, Procs>, _: i32) -> Result<(), Errno> {
            if id(fp) == 3 {
                return Err(Errno::EINTR);
            }
            uio.uio_resid -= uio.uio_resid.min(5);
            if id(fp) == 2 {
                Err(Errno::EINTR)
            } else {
                Ok(())
            }
        }
        fn write(&self, fp: &File<Files>, uio: &mut Uio<'_, Procs>, _: i32) -> Result<(), Errno> {
            if id(fp) == 4 {
                return Err(Errno::EPIPE);
            }
            uio.uio_resid -= uio.uio_resid.min(5);
            Ok(())
        }
        fn ioctl(
            &self,
            fp: &File<Files>,
            com: u64,
            data: &mut [u8],
            _: &Thread<Procs>,
        ) -> Result<(), Errno> {
            testutil::file_log_push(format!("ioctl {} {com:#x} {}", id(fp), data.len()));
            if com == FIOASYNC {
                return Ok(());
            }
            if com & IOC_OUT != 0 {
                data[0] = 0xab;
                return Ok(());
            }
            Err(Errno::ENOTTY)
        }
        fn kqfilter(&self, _: &File<Files>, _: &()) -> Result<(), Errno> {
            Err(Errno::EINVAL)
        }
        fn stat(&self, _: &File<Files>, st: &mut TestStat, _: &Thread<Procs>) -> Result<(), Errno> {
            st.st_gen = 7;
            st.st_size = 11;
            Ok(())
        }
        fn close(&self, fp: &File<Files>, p: Option<&Thread<Procs>>) -> Result<(), Errno> {
            testutil::file_log_push(format!("close {} {}", id(fp), p.is_some()));
            Ok(())
        }
    }

    static TOPS: TOps = TOps;

    /// A thread of a new process with `uid`'s credentials, a new table and a file limit of
    /// `lim`.
    fn thread_as(pid: i32, uid: u32, lim: u64) -> (ProcessRef<Procs>, P) {
        std::thread_local! {
            static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        testutil::install();
        if !MADE.with(|m| m.replace(true)) {
            procinit::<Procs>(64, 64);
            let _ = filedesc_init::<Files>();
        }
        testutil::set_test_maxfiles(4096);
        let pr = process_alloc::<Procs>(pid, ());
        let mut ids = Ucred::new();
        ids.cr_uid.set(uid);
        drop(pr.set_ucred(CredRef::new(ids)));
        pr.set_fd(fdinit());
        let mut limits = Plimit::new();
        limits.pl_rlimit[RLIMIT_NOFILE].rlim_cur = lim;
        limits.pl_rlimit[RLIMIT_NOFILE].rlim_max = lim;
        drop(pr.set_limit(PlimitRef::new(limits)));
        let t = thread_alloc::<Procs>(&pr, (), ());
        drop(t.set_cred(pr.ucred()));
        (pr, t)
    }

    fn thread(pid: i32) -> (ProcessRef<Procs>, P) {
        thread_as(pid, 0, 512)
    }

    /// `falloc` + install + `fdinsert`: a new file of type `ty` (open `flag`) at the lowest
    /// free descriptor, object `oid`; the caller's reference dropped.
    fn open_as(p: &Thread<Procs>, oid: usize, ty: i32, flag: i32) -> i32 {
        open_with(
            p,
            ty,
            flag,
            TestFileData {
                id: oid,
                ..TestFileData::default()
            },
        )
    }

    fn open_with(p: &Thread<Procs>, ty: i32, flag: i32, data: TestFileData) -> i32 {
        let mut g = fdplock(p.fd());
        let (fp, fd) = falloc(p, &mut g).unwrap();
        fp.f_flag.store(flag as u32, Ordering::SeqCst);
        fp.install(ty, &TOPS, data);
        fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        fp.frele(Some(p)).unwrap();
        fd
    }

    fn open(p: &Thread<Procs>, oid: usize) -> i32 {
        open_as(p, oid, DTYPE_PIPE, FREAD | FWRITE)
    }

    fn tables() -> &'static FileTables<Files> {
        Files::tables()
    }

    #[test]
    fn find_next_zero_scans_words_from_want() {
        let map = [!0u32, 0xffff_00ff, 0];
        let at = |o: usize| word_or_full(&map, o);
        assert_eq!(find_next_zero(at, 0, 96), 40);
        assert_eq!(find_next_zero(at, 41, 96), 41);
        assert_eq!(find_next_zero(at, 48, 96), 64);
        assert_eq!(find_next_zero(at, 97, 96), -1);
        let full = |_| !0u32;
        assert_eq!(find_next_zero(full, 0, 32), -1);
        // bits rounds up to whole high-map words: NDLOSLOTS(1) is 32 words.
        let one = |o: usize| if o == 31 { 0xfffe_ffff } else { !0 };
        assert_eq!(find_next_zero(one, 0, 1), 31 * 32 + 16);
    }

    #[test]
    fn fdinit_builds_an_empty_table() {
        let (pr, p) = thread(1);
        let fdp = p.fd();
        assert_eq!(fdp.nfiles(), NDFILE);
        assert_eq!(fdp.shares(), 1);
        assert_eq!(fdp.cmask(), 0o022);
        assert_eq!(fdp.fd_openfd.load(Ordering::Relaxed), 0);
        assert!((0..NDFILE as i32).all(|fd| fd_getfile(fdp, fd).is_none()));
        assert!(fd_getfile(fdp, -1).is_none());
        assert_eq!(pr.fd_ref().addr(), fdp.addr());
    }

    #[test]
    fn fdalloc_takes_the_lowest_free_descriptor() {
        let (_pr, p) = thread(2);
        let fds: Vec<i32> = (0..5).map(|i| open(&p, 100 + i)).collect();
        assert_eq!(fds, vec![0, 1, 2, 3, 4]);
        assert_eq!(sys_close(&p, 1), Ok(()));
        assert_eq!(sys_close(&p, 3), Ok(()));
        assert_eq!(open(&p, 200), 1);
        assert_eq!(open(&p, 201), 3);
        assert_eq!(open(&p, 202), 5);
        // fcntl(F_DUPFD) takes the lowest at or above its argument.
        assert_eq!(sys_fcntl(&p, 0, F_DUPFD, 10), Ok(10));
        assert_eq!(sys_fcntl(&p, 0, F_DUPFD, 2), Ok(6));
        assert_eq!(sys_getdtablecount(&p), 8);
        // The limit: EMFILE past it.
        let (_pr, q) = thread_as(3, 0, 3);
        for want in 0..3 {
            assert_eq!(open(&q, 300 + want as usize), want);
        }
        let mut g = fdplock(q.fd());
        assert_eq!(fdalloc(&q, &mut g, 0), Err(Errno::EMFILE));
    }

    #[test]
    fn fdexpand_grows_the_table_and_the_maps_past_1024() {
        let (_pr, p) = thread_as(4, 0, 4096);
        let fdp = p.fd();
        let mut sizes = vec![fdp.nfiles()];
        let mut g = fdplock(fdp);
        while g.st.nfiles < 1600 {
            fdexpand(&p, &mut g).unwrap();
            sizes.push(g.st.nfiles);
        }
        drop(g);
        assert_eq!(sizes, vec![20, 50, 100, 200, 400, 800, 1600]);
        // The files of before the expansions are still there, and a descriptor past the
        // internal maps' 1024 is allocated through the bigger ones.
        let _ = open(&p, 1);
        assert_eq!(sys_fcntl(&p, 0, F_DUPFD, 1500), Ok(1500));
        assert_eq!(sys_fcntl(&p, 0, F_DUPFD, 1023), Ok(1023));
        assert_eq!(sys_fcntl(&p, 0, F_DUPFD, 1023), Ok(1024));
        let fp = fd_getfile(fdp, 1500).unwrap();
        assert_eq!(id(&fp), 1);
        assert_eq!(fp.count(), 5);
        drop(fp);
        assert_eq!(fdp.with_state(|s| s.lastfile()), 1500);
        assert_eq!(sys_close(&p, 1500), Ok(()));
        assert_eq!(fdp.with_state(|s| s.lastfile()), 1024);
    }

    #[test]
    fn a_new_file_follows_descriptor_zero_s_in_filehead() {
        let (_pr, p) = thread(5);
        for oid in [10, 11, 12] {
            let _ = open(&p, oid);
        }
        // fdinsert links a new file after the file of descriptor 0 (the C's order).
        let mut order = Vec::new();
        let mut fp = fd_iterfile(None, &p);
        while let Some(f) = fp {
            order.push(id(&f));
            fp = fd_iterfile(Some(f), &p);
        }
        assert_eq!(order, vec![10, 12, 11]);
        assert_eq!(tables().numfiles.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn close_closes_after_knote_fdclose_and_the_unlock() {
        let (_pr, p) = thread(6);
        let fd = open(&p, 20);
        let _ = file_log();
        assert_eq!(sys_close(&p, fd), Ok(()));
        assert_eq!(
            file_log(),
            vec![
                String::from("knote_fdclose 0"),
                String::from("close 20 true")
            ]
        );
        assert_eq!(sys_close(&p, fd), Err(Errno::EBADF));
        assert_eq!(sys_close(&p, -1), Err(Errno::EBADF));
        assert_eq!(sys_close(&p, 1000), Err(Errno::EBADF));
        // A file a thread still holds is closed by its last reference.
        let fd = open(&p, 21);
        let held = fd_getfile(p.fd(), fd).unwrap();
        assert_eq!(sys_close(&p, fd), Ok(()));
        assert_eq!(file_log(), vec![String::from("knote_fdclose 0")]);
        held.frele(Some(&p)).unwrap();
        assert_eq!(file_log(), vec![String::from("close 21 true")]);
    }

    #[test]
    fn the_dup_family_answers_the_c_s_errnos() {
        let (_pr, p) = thread(7);
        let a = open(&p, 30);
        let b = open(&p, 31);
        let cases: [(&str, Result<i32, Errno>); 11] = [
            ("dup bad", sys_dup(&p, 9)),
            ("dup2 bad", sys_dup2(&p, 9, 3)),
            ("dup2 past the limit", sys_dup2(&p, a, 512)),
            ("dup2 negative", sys_dup2(&p, a, -1)),
            ("dup2 same", sys_dup2(&p, a, a)),
            ("dup3 same", sys_dup3(&p, a, a, 0)),
            ("dup3 flags", sys_dup3(&p, a, 5, 0x1000_0000)),
            ("dup", sys_dup(&p, a)),
            ("dup2", sys_dup2(&p, a, 7)),
            ("dup3 cloexec", sys_dup3(&p, b, 8, O_CLOEXEC)),
            ("dup2 past the table", sys_dup2(&p, a, 100)),
        ];
        let want: [Result<i32, Errno>; 11] = [
            Err(Errno::EBADF),
            Err(Errno::EBADF),
            Err(Errno::EBADF),
            Err(Errno::EBADF),
            Ok(a),
            Err(Errno::EINVAL),
            Err(Errno::EINVAL),
            Ok(2),
            Ok(7),
            Ok(8),
            Ok(100),
        ];
        for ((name, got), want) in cases.iter().zip(want.iter()) {
            assert_eq!(got, want, "{name}");
        }
        assert_eq!(sys_fcntl(&p, 8, F_GETFD, 0), Ok(FD_CLOEXEC as Register));
        assert_eq!(sys_fcntl(&p, 7, F_GETFD, 0), Ok(0));
        assert_eq!(p.fd().nfiles(), 200);
        // dup2 onto an open descriptor closes the displaced file after the unlock.
        let _ = file_log();
        assert_eq!(sys_dup2(&p, a, b), Ok(b));
        assert_eq!(
            file_log(),
            vec![String::from("knote_fdclose 1")],
            "file 31 is still at 8"
        );
        assert_eq!(sys_dup2(&p, a, 8), Ok(8));
        assert_eq!(
            file_log(),
            vec![
                String::from("knote_fdclose 8"),
                String::from("close 31 true")
            ]
        );
        assert_eq!(fd_getfile(p.fd(), 8).map(|f| id(&f)), Some(30));
    }

    #[test]
    fn fcntl_answers_the_c_s_values_and_errnos() {
        let (_pr, p) = thread(8);
        let fd = open(&p, 40);
        assert_eq!(sys_fcntl(&p, 9, F_GETFD, 0), Err(Errno::EBADF));
        assert_eq!(sys_fcntl(&p, fd, 99, 0), Err(Errno::EINVAL));
        assert_eq!(sys_fcntl(&p, fd, F_DUPFD, 512), Err(Errno::EINVAL));
        assert_eq!(sys_fcntl(&p, fd, F_DUPFD, usize::MAX), Err(Errno::EINVAL));
        assert_eq!(
            sys_fcntl(&p, fd, F_SETFD, (FD_CLOEXEC | FD_CLOFORK) as usize),
            Ok(0)
        );
        assert_eq!(
            sys_fcntl(&p, fd, F_GETFD, 0),
            Ok((FD_CLOEXEC | FD_CLOFORK) as Register)
        );
        assert_eq!(p.fd().ofileflags(fd), UF_EXCLOSE | UF_FORKCLOSE);
        assert_eq!(
            sys_fcntl(&p, fd, F_GETFL, 0),
            Ok(oflags(FREAD | FWRITE) as Register)
        );
        let _ = file_log();
        assert_eq!(sys_fcntl(&p, fd, F_SETFL, FNONBLOCK as usize), Ok(0));
        let fp = fd_getfile(p.fd(), fd).unwrap();
        assert_eq!(fp.flag(), FREAD | FWRITE | FNONBLOCK);
        assert_eq!(file_log(), vec![format!("ioctl 40 {FIOASYNC:#x} 4")]);
        assert_eq!(sys_fcntl(&p, fd, F_DUPFD_CLOFORK, 0), Ok(1));
        assert_eq!(p.fd().ofileflags(1), UF_FORKCLOSE);
        // Not a tty, not a vnode: no record locks.
        assert_eq!(sys_fcntl(&p, fd, F_ISATTY, 0), Err(Errno::ENOTTY));
        assert_eq!(sys_fcntl(&p, fd, F_GETLK, 0), Err(Errno::EINVAL));
        let vn = open_as(&p, 41, DTYPE_VNODE, FREAD);
        assert_eq!(sys_fcntl(&p, vn, F_ISATTY, 0), Err(Errno::ENOTTY));
        let tty = open_with(
            &p,
            DTYPE_VNODE,
            FREAD,
            TestFileData {
                id: 42,
                tty: true,
                ..TestFileData::default()
            },
        );
        assert_eq!(sys_fcntl(&p, tty, F_ISATTY, 0), Ok(1));
        // A DNS socket refuses every ioctl.
        let dns = open_with(
            &p,
            DTYPE_SOCKET,
            FREAD | FWRITE,
            TestFileData {
                id: 43,
                dns: true,
                ..TestFileData::default()
            },
        );
        assert_eq!(sys_ioctl(&p, dns, FIONBIO, 0), Err(Errno::EINVAL));
    }

    #[test]
    fn record_locks_are_the_table_s_and_flock_s_the_file_s() {
        let (_pr, p) = thread(9);
        let fd = open_as(&p, 50, DTYPE_VNODE, FREAD);
        let fdp_addr = p.fd().addr();
        let fl_at = testutil::USER_BASE + 0x3000;
        let mut fl = Flock {
            l_type: F_WRLCK,
            ..Flock::default()
        };
        UserPtr::<[u8; Flock::SIZE]>::new(fl_at)
            .copyout(&fl.to_bytes())
            .unwrap();
        let _ = file_log();
        // A write lock on a file open for reading only.
        assert_eq!(sys_fcntl(&p, fd, F_SETLK, fl_at), Err(Errno::EBADF));
        fl.l_type = F_RDLCK;
        UserPtr::<[u8; Flock::SIZE]>::new(fl_at)
            .copyout(&fl.to_bytes())
            .unwrap();
        assert_eq!(sys_fcntl(&p, fd, F_SETLK, fl_at), Ok(0));
        assert_eq!(
            file_log(),
            vec![format!(
                "advlock 50 Table({fdp_addr}) {F_SETLK} {F_RDLCK} 0 {F_POSIX:#x}"
            )]
        );
        assert_ne!(p.fd().fd_flags.load(Ordering::Relaxed) & FD_ADVLOCK, 0);
        // flock(2): the file is the owner.
        let fp_addr = fd_getfile(p.fd(), fd).unwrap().addr();
        assert_eq!(sys_flock(&p, fd, LOCK_SH), Ok(()));
        assert_eq!(
            file_log(),
            vec![format!(
                "advlock 50 File({fp_addr}) {F_SETLK} {F_RDLCK} 0 {:#x}",
                F_FLOCK | F_WAIT
            )]
        );
        assert_eq!(sys_flock(&p, fd, 0), Err(Errno::EINVAL));
        let _ = file_log();
        let pipe = open(&p, 51);
        assert_eq!(sys_flock(&p, pipe, LOCK_EX), Err(Errno::EOPNOTSUPP));
        // The close of a vnode file releases the table's POSIX locks, as their owner.
        assert_eq!(sys_close(&p, fd), Ok(()));
        let log = file_log();
        assert_eq!(log[0], format!("knote_fdclose {fd}"));
        assert_eq!(
            log[1],
            format!(
                "advlock 50 Table({fdp_addr}) {} {F_UNLCK} 0 {F_POSIX:#x}",
                i32::from(F_UNLCK)
            )
        );
        assert_eq!(log[2], "close 50 true");
    }

    #[test]
    fn fdcopy_skips_forkclose_and_kqueues_and_shrinks_the_table() {
        let (pr, p) = thread_as(10, 0, 4096);
        let a = open(&p, 60);
        let b = open(&p, 61);
        let k = open_as(&p, 62, DTYPE_KQUEUE, FREAD | FWRITE);
        assert_eq!(sys_fcntl(&p, b, F_SETFD, FD_CLOFORK as usize), Ok(0));
        assert_eq!(sys_fcntl(&p, a, F_SETFD, FD_CLOEXEC as usize), Ok(0));
        assert_eq!(sys_fcntl(&p, a, F_DUPFD, 30), Ok(30));
        {
            let mut g = fdplock(p.fd());
            while g.st.nfiles < 200 {
                fdexpand(&p, &mut g).unwrap();
            }
        }
        let vn: &'static TestVnode = std::boxed::Box::leak(std::boxed::Box::default());
        Files::vref(vn);
        p.fd().set_cdir(Some(vn));
        let copy = fdcopy(&pr);
        assert_eq!(vn.refs.load(Ordering::Relaxed), 2);
        assert_eq!(copy.nfiles(), 50);
        assert_eq!(fd_getfile(&copy, a).map(|f| id(&f)), Some(60));
        assert_eq!(copy.ofileflags(a), UF_EXCLOSE);
        assert!(fd_getfile(&copy, b).is_none());
        assert!(fd_getfile(&copy, k).is_none());
        assert_eq!(fd_getfile(&copy, 30).map(|f| id(&f)), Some(60));
        assert_eq!(copy.fd_openfd.load(Ordering::Relaxed), 2);
        assert_eq!(copy.with_state(|s| (s.lastfile(), s.freefile())), (30, 1));
        assert_eq!(copy.cmask(), p.fd().cmask());
        // The shared file counts both tables' slots.
        assert_eq!(fd_getfile(p.fd(), a).unwrap().count(), 5);
    }

    #[test]
    fn the_last_share_s_fdfree_closes_every_descriptor() {
        let (pr, p) = thread(11);
        let a = open(&p, 70);
        let _b = open_as(&p, 71, DTYPE_VNODE, FREAD);
        let vn: &'static TestVnode = std::boxed::Box::leak(std::boxed::Box::default());
        Files::vref(vn);
        p.fd().set_rdir(Some(vn));
        let shared = fdshare(&pr);
        assert_eq!(p.fd().shares(), 2);
        let _ = file_log();
        fdfree(&p);
        assert!(file_log().is_empty());
        assert_eq!(p.fd().shares(), 1);
        fdfree(&p);
        assert_eq!(
            file_log(),
            vec![
                String::from("knote_fdclose 0"),
                String::from("close 70 true"),
                String::from("knote_fdclose 1"),
                String::from("close 71 true"),
            ]
        );
        assert_eq!(vn.refs.load(Ordering::Relaxed), 0);
        assert!(shared.rdir().is_none());
        assert_eq!(shared.fd_openfd.load(Ordering::Relaxed), 0);
        assert!(fd_getfile(&shared, a).is_none());
        assert_eq!(tables().numfiles.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn closefrom_and_prepforexec_close_what_the_c_s_do() {
        let (_pr, p) = thread(12);
        for oid in 80..86 {
            let _ = open(&p, oid);
        }
        assert_eq!(sys_closefrom(&p, 9), Err(Errno::EBADF));
        assert_eq!(sys_closefrom(&p, -1), Err(Errno::EBADF));
        let _ = file_log();
        assert_eq!(sys_closefrom(&p, 4), Ok(()));
        assert_eq!(sys_getdtablecount(&p), 4);
        let log = file_log();
        assert_eq!(log.iter().filter(|l| l.starts_with("close")).count(), 2);
        assert_eq!(sys_fcntl(&p, 1, F_SETFD, FD_CLOEXEC as usize), Ok(0));
        assert_eq!(sys_fcntl(&p, 2, F_SETFD, FD_CLOFORK as usize), Ok(0));
        let _ = file_log();
        fdprepforexec(&p);
        assert_eq!(
            file_log(),
            vec![
                String::from("knote_fdclose 1"),
                String::from("close 81 true")
            ]
        );
        assert_eq!(p.fd().ofileflags(2), 0);
        assert_eq!(sys_getdtablecount(&p), 3);
    }

    /// A user buffer of `n` bytes for the read and write tests.
    const BUF: usize = testutil::USER_BASE + 0x2000;

    #[test]
    fn read_and_write_keep_the_partial_error_rule_and_the_counters() {
        let (_pr, p) = thread(13);
        let ok = open(&p, 1);
        let late = open(&p, 2);
        let early = open(&p, 3);
        let pipe = open(&p, 4);
        assert_eq!(sys_read(&p, ok, BUF, 8), Ok(5));
        // EINTR after data: the bytes moved count; before any: the error.
        assert_eq!(sys_read(&p, late, BUF, 8), Ok(5));
        assert_eq!(sys_read(&p, early, BUF, 8), Err(Errno::EINTR));
        assert_eq!(sys_read(&p, 9, BUF, 8), Err(Errno::EBADF));
        assert_eq!(sys_read(&p, ok, BUF, usize::MAX), Err(Errno::EINVAL));
        let fp = fd_getfile(p.fd(), ok).unwrap();
        let s = *fp.stats();
        assert_eq!((s.f_rxfer, s.f_rbytes), (1, 5));
        drop(fp);
        assert_eq!(sys_write(&p, ok, BUF, 3), Ok(3));
        let _ = file_log();
        assert_eq!(sys_write(&p, pipe, BUF, 3), Err(Errno::EPIPE));
        assert_eq!(file_log(), vec![String::from("SIGPIPE")]);
        let fp = fd_getfile(p.fd(), ok).unwrap();
        let s = *fp.stats();
        assert_eq!((s.f_wxfer, s.f_wbytes), (1, 3));
        drop(fp);
        // Positioned I/O is a vnode's: ESPIPE elsewhere, the vnode's checks there.
        assert_eq!(sys_pread(&p, ok, BUF, 8, 0), Err(Errno::ESPIPE));
        let vn = open_as(&p, 5, DTYPE_VNODE, FREAD);
        assert_eq!(sys_pread(&p, vn, BUF, 8, 4), Ok(5));
        assert_eq!(sys_pread(&p, vn, BUF, 8, -1), Err(Errno::EINVAL));
        // Open for reading only: no write.
        assert_eq!(sys_write(&p, vn, BUF, 3), Err(Errno::EBADF));
        // A descriptor opened with __pledge_open() is not written.
        {
            let mut g = fdplock(p.fd());
            let f = g.ofileflags(ok) | UF_PLEDGEOPEN;
            g.set_ofileflags(ok, f);
        }
        assert_eq!(sys_write(&p, ok, BUF, 3), Err(Errno::EPERM));
    }

    #[test]
    fn readv_and_writev_take_their_iovecs_from_userland() {
        let (_pr, p) = thread(14);
        let fd = open(&p, 1);
        let iov_at = testutil::USER_BASE + 0x2800;
        let mut words = [[0u8; Iovec::SIZE]; 2];
        for (i, w) in words.iter_mut().enumerate() {
            w[..8].copy_from_slice(&(BUF + i * 4).to_ne_bytes());
            w[8..].copy_from_slice(&4usize.to_ne_bytes());
        }
        UserPtr::<[u8; Iovec::SIZE]>::new(iov_at)
            .copyout_slice(&words)
            .unwrap();
        assert_eq!(sys_readv(&p, fd, UserPtr::new(iov_at), 2), Ok(5));
        assert_eq!(sys_writev(&p, fd, UserPtr::new(iov_at), 2), Ok(5));
        assert_eq!(
            sys_readv(&p, fd, UserPtr::new(iov_at), 0),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sys_preadv(&p, fd, UserPtr::new(iov_at), 2, 0),
            Err(Errno::ESPIPE)
        );
    }

    #[test]
    fn ioctl_copies_by_the_command_s_direction_and_size() {
        let (_pr, p) = thread(15);
        let fd = open(&p, 90);
        let at = testutil::USER_BASE + 0x3800;
        UserPtr::<u8>::new(at).copyout_slice(&[0x55; 8]).unwrap();
        let _ = file_log();
        // IOC_OUT: a zeroed buffer, written by the file, copied out.
        let out = _ior::<[u8; 8]>(b't', 1);
        assert_eq!(sys_ioctl(&p, fd, out, at), Ok(()));
        let mut got = [0u8; 8];
        UserPtr::<u8>::new(at).copyin_slice(&mut got).unwrap();
        assert_eq!(got, [0xab, 0, 0, 0, 0, 0, 0, 0]);
        // A buffer past the stack's 128 bytes.
        let big = _iowr::<[u8; 200]>(b't', 2);
        assert_eq!(sys_ioctl(&p, fd, big, at), Ok(()));
        assert_eq!(
            file_log(),
            vec![
                format!("ioctl 90 {out:#x} 8"),
                format!("ioctl 90 {big:#x} 200")
            ]
        );
        // IOC_IN that faults, the command the file does not know, and FIOCLEX.
        let inward = _iow::<[u8; 8]>(b't', 3);
        assert_eq!(sys_ioctl(&p, fd, inward, 0), Err(Errno::EFAULT));
        assert_eq!(sys_ioctl(&p, fd, inward, at), Err(Errno::ENOTTY));
        assert_eq!(sys_ioctl(&p, fd, FIOCLEX, 0), Ok(()));
        assert_eq!(p.fd().ofileflags(fd), UF_EXCLOSE);
        assert_eq!(sys_ioctl(&p, fd, FIONCLEX, 0), Ok(()));
        assert_eq!(p.fd().ofileflags(fd), 0);
        // FIONBIO is the descriptor layer's own.
        UserPtr::<u8>::new(at)
            .copyout_slice(&1i32.to_ne_bytes())
            .unwrap();
        assert_eq!(sys_ioctl(&p, fd, FIONBIO, at), Ok(()));
        assert_ne!(fd_getfile(p.fd(), fd).unwrap().flag() & FNONBLOCK, 0);
        assert_eq!(sys_ioctl(&p, 9, FIONBIO, at), Err(Errno::EBADF));
        // A length past IOCPARM_MAX.
        let huge = IOC_IN | ((IOCPARM_MAX as u64 + 1) << 16) | (u64::from(b't') << 8) | 9;
        assert_eq!(sys_ioctl(&p, fd, huge, at), Err(Errno::ENOTTY));
    }

    #[test]
    fn fstat_hides_the_generation_from_non_root_and_fpathconf_answers_by_type() {
        let (_pr, root) = thread(16);
        let fd = open(&root, 1);
        let sb = testutil::USER_BASE + 0x3c00;
        assert_eq!(sys_fstat(&root, fd, sb), Ok(()));
        let st = UserPtr::<TestStat>::new(sb).copyin().unwrap();
        assert_eq!((st.st_size, st.st_gen), (11, 7));
        let (_qr, user) = thread_as(17, 1000, 512);
        let ufd = open(&user, 2);
        assert_eq!(sys_fstat(&user, ufd, sb), Ok(()));
        let st = UserPtr::<TestStat>::new(sb).copyin().unwrap();
        assert_eq!((st.st_size, st.st_gen), (11, 0));
        assert_eq!(sys_fstat(&user, 9, sb), Err(Errno::EBADF));

        assert_eq!(
            sys_fpathconf(&root, fd, _PC_PIPE_BUF),
            Ok(PIPE_BUF as Register)
        );
        assert_eq!(sys_fpathconf(&root, fd, 1), Err(Errno::EINVAL));
        let vn = open_as(&root, 3, DTYPE_VNODE, FREAD);
        assert_eq!(sys_fpathconf(&root, vn, 2), Ok(1002));
        let kq = open_as(&root, 4, DTYPE_KQUEUE, FREAD);
        assert_eq!(sys_fpathconf(&root, kq, 2), Err(Errno::EOPNOTSUPP));
    }

    #[test]
    fn dupfdopen_duplicates_p_dupfd_with_its_flags() {
        let (_pr, p) = thread(18);
        let src = open_as(&p, 1, DTYPE_PIPE, FREAD);
        assert_eq!(sys_fcntl(&p, src, F_SETFD, FD_CLOFORK as usize), Ok(0));
        p.p_dupfd.set(src);
        let mut g = fdplock(p.fd());
        let indx = fdalloc(&p, &mut g, 0).unwrap();
        assert_eq!(
            dupfdopen(&p, &mut g, indx, FREAD | FWRITE),
            Err(Errno::EACCES)
        );
        assert_eq!(dupfdopen(&p, &mut g, indx, FREAD), Ok(()));
        drop(g);
        assert_eq!(fd_getfile(p.fd(), indx).map(|f| id(&f)), Some(1));
        // The slot's flags but the close-on flags, which the new descriptor keeps its own.
        assert_eq!(p.fd().ofileflags(indx), 0);
        p.p_dupfd.set(9);
        let mut g = fdplock(p.fd());
        let indx = fdalloc(&p, &mut g, 0).unwrap();
        assert_eq!(dupfdopen(&p, &mut g, indx, FREAD), Err(Errno::EBADF));
    }

    #[test]
    #[ignore = "reads reference/openbsd-src (just test-ref)"]
    fn constants_match_the_c() {
        let defs = crate::reftest::defines("sys/sys/filedesc.h");
        let _ = crate::reftest::assert_defines!(defs;
            NDFILE, NDEXTENT, NDENTRIES, NDENTRYSHIFT, UF_EXCLOSE, UF_PLEDGED, UF_FORKCLOSE,
            UF_PLEDGEOPEN, FD_ADVLOCK);
        let defs = crate::reftest::defines("sys/sys/malloc.h");
        let _ = crate::reftest::assert_defines!(defs; M_FILEDESC, M_IOCTLOPS);
        let defs = crate::reftest::defines("sys/sys/stat.h");
        let w = crate::reftest::int(&defs, "S_IWGRP").unwrap()
            | crate::reftest::int(&defs, "S_IWOTH").unwrap();
        assert_eq!(w, i64::from(CMASK_DEFAULT));
    }
}
/* </TESTS> */
