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
 * Copyright (c) 1982, 1986, 1991, 1993
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
 *	@(#)kern_resource.c	8.5 (Berkeley) 1/21/94
 */

/*
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)resource.h	8.2 (Berkeley) 1/4/94
 */

/*
 * Copyright (c) 1991, 1993
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
 *	@(#)resourcevar.h	8.3 (Berkeley) 2/22/94
 */
/* </LICENSES> */

/* <CODE> */
//! Resource limits and usage: `struct rlimit`, `struct plimit`, `struct rusage` and `struct
//! tusage` (`<sys/resource.h>`, `<sys/resourcevar.h>`, `<sys/proc.h>`), and the functions of
//! `kern/kern_resource.c` over them that need no process: the limits shared copy-on-write
//! (`lim_startup`, `lim_copy`, a thread's read copy), the usage counters and their sums
//! (`tuagg_sumup`, `calctsru`, `calcru`, `ruadd`). The functions over a process (`lim_fork`,
//! `lim_write_begin`/`commit`, `lim_read_enter`, `lim_cur_proc`, `tuagg_get_process`,
//! `tuagg_add_process`, `rucheck`) come with `kernel::proc`; the five system calls stay in
//! `bsd`'s `kern_resource.rs` until N4.6.
//!
//! Upstream: sys/kern/kern_resource.c @ 3ce1f3f79392
//! Upstream: sys/sys/resource.h @ 3ce1f3f79392
//! Upstream: sys/sys/resourcevar.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_resource.rs@3c62ede68802
//! LZ: sys/sys/resource.rs@3c62ede68802
//! LZ: sys/sys/resourcevar.rs@3c62ede68802
//!
//! ## Deviations
//! - A thread's `p_limit`, a counted read reference to its process's `plimit` that the thread
//!   refreshes when the process's pointer changed (`lim_read_enter`), is a copy of the limits
//!   ([`LimitCache`]: the address of the `plimit` it was copied from and the values, atomics
//!   only the thread writes), so `lim_cur` reads the thread's words without a lock and without
//!   holding the structure; the process keeps the counted reference (`ps_limit`), which
//!   `kinfo_proc` prints.
//! - `lim_write_begin` always hands out a copy to change, where the C changes an unshared
//!   structure in place: the new limits are built as a value and placed by
//!   `lim_write_commit` ([`PlimitRef::new`]), so a published `plimit` is never written; one
//!   pool item per `setrlimit(2)` call, the same limits for userland.
//!
//! ## Redesign
//! - `struct plimit` handed around as LZ's `&'static Plimit` (`lim_copy` returns it,
//!   `lim_free` gives it back by convention), its limits `Cell`s written through the shared
//!   pointer by the exclusive copy, under an `unsafe impl Sync`: plain data ([`Plimit`]),
//!   placed in the pool `plimitpl` as a counted handle ([`PlimitRef`]: `Clone` is the
//!   `refcnt_take` of `lim_fork`, `Drop` is `lim_free`).
//! - `struct rusage`'s and `struct tusage`'s `Cell` counters under an `unsafe impl Sync` whose
//!   comment says "the owning thread": relaxed atomics read and written by `get()`/`set()`
//!   ([`Word`], [`TimevalWord`], [`TimespecWord`]), so the counters the owning thread bumps and
//!   `sysctl` reads from another CPU are never a data race and the 20 files that bump them keep
//!   their calls. `tu_enter`/`tu_leave` and `tuagg_sumup`'s consumer loop keep the C's
//!   `pc_lock` generation protocol over hal's `PcLock`.

use core::ffi::c_void;
use core::ops::Deref;
use core::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering, fence};

use hal::arch::current::vmparam::{MAXDSIZ, MAXSSIZ};
use hal::klock;
use hal::pool::{PR_WAITOK, PoolArc, TypedPool};
use hal::rwlock::{RawRwlock, Rwlocked};
use hal::sys::errno::Errno;
use hal::sys::mman::{PROT_NONE, PROT_READ, PROT_WRITE};
use hal::sys::pclock::PcLock;
use hal::sys::syslimits::NZERO;
use hal::sys::types::{Id, Pid, Rlim};
use hal::user::UserPtr;
use hal::vm::param::{round_page, trunc_page};

use crate::assert::{kassert, kpanic};
use crate::clock::Timespec;
use crate::proc::{ProcGlue, Process, ProcessRef, PsMtx, Thread};

/// `RLIMIT_CPU`: cpu time in milliseconds.
pub const RLIMIT_CPU: usize = 0;
/// `RLIMIT_FSIZE`: maximum file size.
pub const RLIMIT_FSIZE: usize = 1;
/// `RLIMIT_DATA`: data size.
pub const RLIMIT_DATA: usize = 2;
/// `RLIMIT_STACK`: stack size.
pub const RLIMIT_STACK: usize = 3;
/// `RLIMIT_CORE`: core file size.
pub const RLIMIT_CORE: usize = 4;
/// `RLIMIT_RSS`: resident set size.
pub const RLIMIT_RSS: usize = 5;
/// `RLIMIT_MEMLOCK`: locked-in-memory address space.
pub const RLIMIT_MEMLOCK: usize = 6;
/// `RLIMIT_NPROC`: number of processes.
pub const RLIMIT_NPROC: usize = 7;
/// `RLIMIT_NOFILE`: number of open files.
pub const RLIMIT_NOFILE: usize = 8;
/// `RLIM_NLIMITS`: number of resource limits.
pub const RLIM_NLIMITS: usize = 9;

/// `RLIM_INFINITY`.
pub const RLIM_INFINITY: Rlim = (1 << 63) - 1;
/// `RLIM_SAVED_MAX`.
pub const RLIM_SAVED_MAX: Rlim = RLIM_INFINITY;
/// `RLIM_SAVED_CUR`.
pub const RLIM_SAVED_CUR: Rlim = RLIM_INFINITY;

/// `NOFILE` (`OPEN_MAX`): the default soft limit of open files.
pub const NOFILE: usize = 64;
/// `NOFILE_MAX`: the most open files a process may have.
pub const NOFILE_MAX: usize = 1024;
/// `MAXUPRC` (`CHILD_MAX`): the default soft limit of processes per user.
pub const MAXUPRC: usize = 80;

/// `TU_UTICKS`: statclock hits in user mode.
pub const TU_UTICKS: usize = 0;
/// `TU_STICKS`: statclock hits in system mode.
pub const TU_STICKS: usize = 1;
/// `TU_ITICKS`: statclock hits processing intr.
pub const TU_ITICKS: usize = 2;
/// `TU_TICKS_COUNT`.
pub const TU_TICKS_COUNT: usize = 3;

/// `struct rlimit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rlimit {
    /// `rlim_cur`: current (soft) limit.
    pub rlim_cur: Rlim,
    /// `rlim_max`: maximum value for `rlim_cur`.
    pub rlim_max: Rlim,
}

/// `struct plimit`: the limits a process and its threads read, shared copy-on-write after
/// fork. Plain data: built as a value and placed ([`PlimitRef::new`]), never written once
/// shared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Plimit {
    /// `pl_rlimit`.
    pub pl_rlimit: [Rlimit; RLIM_NLIMITS],
}

impl Plimit {
    /// Every limit zero (what `limit0` holds before `lim_startup`).
    pub const fn new() -> Self {
        Self {
            pl_rlimit: [Rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            }; RLIM_NLIMITS],
        }
    }
}

/// `plimit_pool`, `plimitpl`.
static PLIMIT_POOL: TypedPool<Plimit> = TypedPool::new();
/// Whether [`PLIMIT_POOL`] is made.
static PLIMIT_POOL_MADE: AtomicBool = AtomicBool::new(false);
/// Serialises the making of [`PLIMIT_POOL`] (`pool_init` may sleep), once.
static PLIMIT_POOL_INIT: Rwlocked<()> = Rwlocked::new("plimitpl", ());

/// [`PLIMIT_POOL`], made at its first need (`lim_startup`, or a host test before it), at
/// `IPL_MPFLOOR` with `PR_WAITOK`, as the C's `pool_init`.
fn plimit_pool() -> &'static TypedPool<Plimit> {
    if !PLIMIT_POOL_MADE.load(Ordering::Acquire) {
        let _w = PLIMIT_POOL_INIT.write();
        if !PLIMIT_POOL_MADE.load(Ordering::Relaxed) {
            if PLIMIT_POOL
                .init(hal::intr::mpfloor(), PR_WAITOK, "plimitpl")
                .is_err()
            {
                kpanic(format_args!("lim_startup: cannot make plimitpl"));
            }
            PLIMIT_POOL_MADE.store(true, Ordering::Release);
        }
    }
    &PLIMIT_POOL
}

/// `plimitpl`'s items out (`pr_nout`).
pub fn plimit_nout() -> u64 {
    PLIMIT_POOL.nout()
}

/// `plimitpl`'s item size (`pr_size`, what `vmstat -m` shows), the pool made if it was not.
pub fn plimit_item_size() -> Option<usize> {
    plimit_pool().item_size()
}

/// A counted reference to placed limits (`struct plimit *` with a `pl_refcnt` reference):
/// `Clone` is `refcnt_take`, `Drop` is `lim_free`; `Deref<Target = Plimit>`.
#[derive(Clone)]
pub struct PlimitRef(PoolArc<Plimit>);

impl PlimitRef {
    /// `lim_copy`'s item: `limits` placed in a `plimitpl` item with one reference, the
    /// caller's. `pool_get(PR_WAITOK)` sleeps for an item, as the C's.
    pub fn new(limits: Plimit) -> Self {
        match plimit_pool().get_shared(PR_WAITOK, || limits) {
            Some(a) => PlimitRef(a),
            None => kpanic(format_args!("lim_copy: plimit_pool is empty")),
        }
    }

    /// Whether two references name the same structure (the C's pointer compare).
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        PoolArc::ptr_eq(&a.0, &b.0)
    }

    /// The references alive now (`pl_refcnt`, a snapshot).
    pub fn count(this: &Self) -> usize {
        PoolArc::count(&this.0)
    }

    /// The structure's address: `kinfo_proc`'s `p_limit` slot and the word a thread's read
    /// copy compares ([`LimitCache`]); never dereferenced.
    pub fn addr(this: &Self) -> usize {
        PoolArc::as_ptr(&this.0).addr()
    }
}

impl Deref for PlimitRef {
    type Target = Plimit;

    fn deref(&self) -> &Plimit {
        &self.0
    }
}

/// A thread's read copy of its process's limits (`p_limit`, `[l]`): the address of the
/// `plimit` it was copied from and every limit, atomics only the thread writes (see the
/// module's deviations).
pub struct LimitCache {
    /// The address of the structure the values came from, 0 before the first copy.
    from: AtomicUsize,
    /// `rlim_cur` of each limit.
    cur: [AtomicU64; RLIM_NLIMITS],
    /// `rlim_max` of each limit.
    max: [AtomicU64; RLIM_NLIMITS],
}

impl LimitCache {
    /// No copy yet (a thread being made).
    pub const fn new() -> Self {
        Self {
            from: AtomicUsize::new(0),
            cur: [const { AtomicU64::new(0) }; RLIM_NLIMITS],
            max: [const { AtomicU64::new(0) }; RLIM_NLIMITS],
        }
    }

    /// Whether the copy came from the structure at `addr` ([`PlimitRef::addr`]): the C's
    /// `p->p_limit == pr->ps_limit`.
    #[inline]
    pub fn is_from(&self, addr: usize) -> bool {
        self.from.load(Ordering::Relaxed) == addr
    }

    /// The copy of `lim` (at `addr`), by the thread itself (`lim_read_enter`'s refresh).
    pub fn fill(&self, addr: usize, lim: &Plimit) {
        for (i, r) in lim.pl_rlimit.iter().enumerate() {
            self.cur[i].store(r.rlim_cur, Ordering::Relaxed);
            self.max[i].store(r.rlim_max, Ordering::Relaxed);
        }
        self.from.store(addr, Ordering::Relaxed);
    }

    /// Forgets the copy (`lim_write_begin`'s `p->p_limit = NULL`).
    pub fn clear(&self) {
        self.from.store(0, Ordering::Relaxed);
    }

    /// Limit `which` of the copy.
    #[inline]
    pub fn get(&self, which: usize) -> Rlimit {
        Rlimit {
            rlim_cur: self.cur[which].load(Ordering::Relaxed),
            rlim_max: self.max[which].load(Ordering::Relaxed),
        }
    }

    /// `rlim_cur` of limit `which` of the copy.
    #[inline]
    pub fn cur(&self, which: usize) -> Rlim {
        self.cur[which].load(Ordering::Relaxed)
    }
}

impl Default for LimitCache {
    fn default() -> Self {
        Self::new()
    }
}

/// `lim_startup`'s limits of process 0 (`limit0`): every limit infinite but the open files
/// (`NOFILE`, at most `NOFILE_MAX` and `maxfiles - NOFILE`), the processes (`MAXUPRC`
/// soft), the resident set (`free_bytes`, the free memory, hard) and the locked memory (64k
/// pages hard, a third of it soft). The pool is made here, as the C's `pool_init`.
pub fn lim_startup(maxfiles: usize, free_bytes: Rlim, page_size: usize) -> Plimit {
    let _ = plimit_pool();
    let inf = Rlimit {
        rlim_cur: RLIM_INFINITY,
        rlim_max: RLIM_INFINITY,
    };
    let mut l = Plimit {
        pl_rlimit: [inf; RLIM_NLIMITS],
    };
    let spare = maxfiles.saturating_sub(NOFILE);
    l.pl_rlimit[RLIMIT_NOFILE] = Rlimit {
        rlim_cur: NOFILE as Rlim,
        rlim_max: NOFILE_MAX.min(if spare > NOFILE { spare } else { NOFILE }) as Rlim,
    };
    l.pl_rlimit[RLIMIT_NPROC].rlim_cur = MAXUPRC as Rlim;
    l.pl_rlimit[RLIMIT_RSS].rlim_max = free_bytes;
    let memlock = (64 * 1024 * page_size) as Rlim; // Default to very low
    l.pl_rlimit[RLIMIT_MEMLOCK].rlim_max = memlock;
    l.pl_rlimit[RLIMIT_MEMLOCK].rlim_cur = memlock / 3;
    l
}

/// `lim_copy(lim)`: new limits, held once, with `lim`'s values.
pub fn lim_copy(lim: &Plimit) -> PlimitRef {
    PlimitRef::new(*lim)
}

/// `lim_free(lim)`: the caller's reference given back; the item goes with the last one.
#[inline]
pub fn lim_free(lim: PlimitRef) {
    drop(lim);
}

/// A counter of `struct rusage` or `struct tusage` (`long`, `u_int64_t`): a relaxed atomic
/// read and written whole, by `get()`/`set()` as LZ's `Cell`.
#[derive(Default, Debug)]
pub struct Word(AtomicI64);

impl Word {
    /// `v`.
    pub const fn new(v: i64) -> Self {
        Self(AtomicI64::new(v))
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }

    /// Stores `v`.
    #[inline]
    pub fn set(&self, v: i64) {
        self.0.store(v, Ordering::Relaxed);
    }

    /// `x += v`, as the owning thread bumps its counters (one writer: a load and a store).
    #[inline]
    pub fn add(&self, v: i64) {
        self.set(self.get().wrapping_add(v));
    }
}

/// An unsigned counter of `struct tusage` (`u_int64_t`), as [`Word`].
#[derive(Default, Debug)]
pub struct UWord(AtomicU64);

impl UWord {
    /// `v`.
    pub const fn new(v: u64) -> Self {
        Self(AtomicU64::new(v))
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    /// Stores `v`.
    #[inline]
    pub fn set(&self, v: u64) {
        self.0.store(v, Ordering::Relaxed);
    }
}

/// `struct timeval`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timeval {
    /// `tv_sec`.
    pub tv_sec: i64,
    /// `tv_usec`.
    pub tv_usec: i64,
}

impl Timeval {
    /// `{ sec, usec }`.
    pub const fn new(tv_sec: i64, tv_usec: i64) -> Self {
        Self { tv_sec, tv_usec }
    }

    /// `timeradd(self, b, &res)`: the sum, the microseconds carried.
    pub const fn add(self, b: Timeval) -> Timeval {
        let mut sec = self.tv_sec.wrapping_add(b.tv_sec);
        let mut usec = self.tv_usec + b.tv_usec;
        if usec >= 1_000_000 {
            sec = sec.wrapping_add(1);
            usec -= 1_000_000;
        }
        Timeval::new(sec, usec)
    }

    /// `TIMESPEC_TO_TIMEVAL(&tv, &ts)`.
    pub const fn from_timespec(ts: Timespec) -> Self {
        Self::new(ts.tv_sec, ts.tv_nsec / 1000)
    }
}

/// A `struct timeval` member read and written whole by `get()`/`set()`: two relaxed words
/// (a reader racing a writer may see the two halves of two values, as the C's plain struct
/// copy; the counters are the owning thread's, read under `ps_mtx` or by `getrusage`).
#[derive(Default, Debug)]
pub struct TimevalWord {
    sec: AtomicI64,
    usec: AtomicI64,
}

impl TimevalWord {
    /// Zero.
    pub const fn new() -> Self {
        Self {
            sec: AtomicI64::new(0),
            usec: AtomicI64::new(0),
        }
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> Timeval {
        Timeval::new(
            self.sec.load(Ordering::Relaxed),
            self.usec.load(Ordering::Relaxed),
        )
    }

    /// Stores `v`.
    #[inline]
    pub fn set(&self, v: Timeval) {
        self.sec.store(v.tv_sec, Ordering::Relaxed);
        self.usec.store(v.tv_usec, Ordering::Relaxed);
    }
}

/// A `struct timespec` member read and written whole by `get()`/`set()`, as [`TimevalWord`].
#[derive(Default, Debug)]
pub struct TimespecWord {
    sec: AtomicI64,
    nsec: AtomicI64,
}

impl TimespecWord {
    /// Zero.
    pub const fn new() -> Self {
        Self {
            sec: AtomicI64::new(0),
            nsec: AtomicI64::new(0),
        }
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> Timespec {
        Timespec::new(
            self.sec.load(Ordering::Relaxed),
            self.nsec.load(Ordering::Relaxed),
        )
    }

    /// Stores `v`.
    #[inline]
    pub fn set(&self, v: Timespec) {
        self.sec.store(v.tv_sec, Ordering::Relaxed);
        self.nsec.store(v.tv_nsec, Ordering::Relaxed);
    }
}

/// `struct rusage`: resource utilization information.
#[derive(Default, Debug)]
pub struct Rusage {
    /// `ru_utime`: user time used.
    pub ru_utime: TimevalWord,
    /// `ru_stime`: system time used.
    pub ru_stime: TimevalWord,
    /// `ru_maxrss`: max resident set size.
    pub ru_maxrss: Word,
    /// `ru_ixrss`: integral shared text memory size.
    pub ru_ixrss: Word,
    /// `ru_idrss`: integral unshared data.
    pub ru_idrss: Word,
    /// `ru_isrss`: integral unshared stack.
    pub ru_isrss: Word,
    /// `ru_minflt`: page reclaims.
    pub ru_minflt: Word,
    /// `ru_majflt`: page faults.
    pub ru_majflt: Word,
    /// `ru_nswap`: swaps.
    pub ru_nswap: Word,
    /// `ru_inblock`: block input operations.
    pub ru_inblock: Word,
    /// `ru_oublock`: block output operations.
    pub ru_oublock: Word,
    /// `ru_msgsnd`: messages sent.
    pub ru_msgsnd: Word,
    /// `ru_msgrcv`: messages received.
    pub ru_msgrcv: Word,
    /// `ru_nsignals`: signals received.
    pub ru_nsignals: Word,
    /// `ru_nvcsw`: voluntary context switches.
    pub ru_nvcsw: Word,
    /// `ru_nivcsw`: involuntary context switches.
    pub ru_nivcsw: Word,
}

/// `sizeof(struct rusage)` on LP64: two `struct timeval` and fourteen `long`s.
pub const RUSAGE_SIZE: usize = 2 * 16 + 14 * 8;

impl Rusage {
    /// All zero.
    pub const fn new() -> Self {
        Self {
            ru_utime: TimevalWord::new(),
            ru_stime: TimevalWord::new(),
            ru_maxrss: Word::new(0),
            ru_ixrss: Word::new(0),
            ru_idrss: Word::new(0),
            ru_isrss: Word::new(0),
            ru_minflt: Word::new(0),
            ru_majflt: Word::new(0),
            ru_nswap: Word::new(0),
            ru_inblock: Word::new(0),
            ru_oublock: Word::new(0),
            ru_msgsnd: Word::new(0),
            ru_msgrcv: Word::new(0),
            ru_nsignals: Word::new(0),
            ru_nvcsw: Word::new(0),
            ru_nivcsw: Word::new(0),
        }
    }

    /// `ru_maxrss` through `ru_nivcsw` (`ru_first` is `ru_ixrss`), in declaration order.
    fn longs(&self) -> [&Word; 14] {
        [
            &self.ru_maxrss,
            &self.ru_ixrss,
            &self.ru_idrss,
            &self.ru_isrss,
            &self.ru_minflt,
            &self.ru_majflt,
            &self.ru_nswap,
            &self.ru_inblock,
            &self.ru_oublock,
            &self.ru_msgsnd,
            &self.ru_msgrcv,
            &self.ru_nsignals,
            &self.ru_nvcsw,
            &self.ru_nivcsw,
        ]
    }

    /// `*self = *other`: the struct assignment of the C.
    pub fn copy_from(&self, other: &Rusage) {
        self.ru_utime.set(other.ru_utime.get());
        self.ru_stime.set(other.ru_stime.get());
        for (a, b) in self.longs().iter().zip(other.longs().iter()) {
            a.set(b.get());
        }
    }

    /// The C layout of `struct rusage` (two `struct timeval`, then fourteen `long`s), native
    /// endian, for `copyout`.
    pub fn to_bytes(&self) -> [u8; RUSAGE_SIZE] {
        let mut b = [0u8; RUSAGE_SIZE];
        let mut off = 0;
        for tv in [self.ru_utime.get(), self.ru_stime.get()] {
            b[off..off + 8].copy_from_slice(&tv.tv_sec.to_ne_bytes());
            b[off + 8..off + 16].copy_from_slice(&tv.tv_usec.to_ne_bytes());
            off += 16;
        }
        for l in self.longs() {
            b[off..off + 8].copy_from_slice(&l.get().to_ne_bytes());
            off += 8;
        }
        b
    }
}

/// `struct tusage`: times accumulated in ticks and the run time. A thread's is written
/// lock-free by the thread itself between [`Tusage::enter`] and [`Tusage::leave`] (the C's
/// `tu_enter`/`tu_leave`, a single-producer `pc_lock`); a process's under its `ps_mtx`;
/// readers copy it with [`tuagg_sumup`]'s consumer loop.
#[derive(Default)]
pub struct Tusage {
    /// `tu_pcl`.
    pub tu_pcl: PcLock,
    /// `tu_ticks`: `tu_uticks`, `tu_sticks`, `tu_iticks`.
    pub tu_ticks: [UWord; TU_TICKS_COUNT],
    /// `tu_ixrss`.
    pub tu_ixrss: UWord,
    /// `tu_idrss`.
    pub tu_idrss: UWord,
    /// `tu_isrss`.
    pub tu_isrss: UWord,
    /// `tu_runtime`: realtime.
    pub tu_runtime: TimespecWord,
}

impl Tusage {
    /// All zero.
    pub const fn new() -> Self {
        Self {
            tu_pcl: PcLock::new(),
            tu_ticks: [const { UWord::new(0) }; TU_TICKS_COUNT],
            tu_ixrss: UWord::new(0),
            tu_idrss: UWord::new(0),
            tu_isrss: UWord::new(0),
            tu_runtime: TimespecWord::new(),
        }
    }

    /// `tu_enter(tu)` (`pc_sprod_enter`): the one producer starts a change; the generation
    /// goes odd.
    pub fn enter(&self) -> u32 {
        let g = self.tu_pcl.pcl_gen.load(Ordering::Relaxed).wrapping_add(1);
        self.tu_pcl.pcl_gen.store(g, Ordering::Relaxed);
        fence(Ordering::Release); // membar_producer()
        g
    }

    /// `tu_leave(tu, gen)` (`pc_sprod_leave`): the change is done; the generation goes even.
    pub fn leave(&self, generation: u32) {
        fence(Ordering::Release); // membar_producer()
        self.tu_pcl
            .pcl_gen
            .store(generation.wrapping_add(1), Ordering::Relaxed);
    }

    /// `tuagg_clear`: every counter to zero (the lock is left alone).
    pub fn clear(&self) {
        self.tu_runtime.set(Timespec::ZERO);
        for t in &self.tu_ticks {
            t.set(0);
        }
        self.tu_ixrss.set(0);
        self.tu_idrss.set(0);
        self.tu_isrss.set(0);
    }

    /// `tu_runtime += delta` by the thread itself (`tuagg_add_runtime`'s producer step).
    pub fn add_runtime(&self, delta: Timespec) {
        let g = self.enter();
        self.tu_runtime.set(self.tu_runtime.get().add(delta));
        self.leave(g);
    }
}

/// A consistent copy of a [`Tusage`]'s counters (what `tuagg_sumup` reads inside its
/// consumer loop).
#[derive(Clone, Copy)]
struct TuSnapshot {
    ticks: [u64; TU_TICKS_COUNT],
    ixrss: u64,
    idrss: u64,
    isrss: u64,
    runtime: Timespec,
}

/// `pc_cons_enter`: waits for an even generation of `pcl`; returns it.
fn cons_enter(pcl: &PcLock) -> u32 {
    let mut g = pcl.pcl_gen.load(Ordering::Relaxed);
    while g & 1 != 0 {
        core::hint::spin_loop(); // CPU_BUSY_CYCLE()
        g = pcl.pcl_gen.load(Ordering::Relaxed);
    }
    fence(Ordering::Acquire); // membar_consumer()
    g
}

/// `pc_cons_leave`: `false` when the read since `*g` was consistent; `true`, `*g` the new
/// even generation, when a producer intervened and the consumer must read again.
fn cons_leave(pcl: &PcLock, g: &mut u32) -> bool {
    fence(Ordering::Acquire); // membar_consumer()
    let mut now = pcl.pcl_gen.load(Ordering::Relaxed);
    while now & 1 != 0 {
        core::hint::spin_loop(); // CPU_BUSY_CYCLE()
        now = pcl.pcl_gen.load(Ordering::Relaxed);
    }
    if now == *g {
        return false;
    }
    fence(Ordering::Acquire);
    *g = now;
    true
}

/// `tuagg_sumup(tu, from)`: adds `from`'s counters, read consistently, to `tu`.
pub fn tuagg_sumup(tu: &Tusage, from: &Tusage) {
    let mut g = cons_enter(&from.tu_pcl);
    let s = loop {
        let s = TuSnapshot {
            ticks: [
                from.tu_ticks[TU_UTICKS].get(),
                from.tu_ticks[TU_STICKS].get(),
                from.tu_ticks[TU_ITICKS].get(),
            ],
            ixrss: from.tu_ixrss.get(),
            idrss: from.tu_idrss.get(),
            isrss: from.tu_isrss.get(),
            runtime: from.tu_runtime.get(),
        };
        if !cons_leave(&from.tu_pcl, &mut g) {
            break s;
        }
    };
    for (t, v) in tu.tu_ticks.iter().zip(s.ticks) {
        t.set(t.get().wrapping_add(v));
    }
    tu.tu_ixrss.set(tu.tu_ixrss.get().wrapping_add(s.ixrss));
    tu.tu_idrss.set(tu.tu_idrss.get().wrapping_add(s.idrss));
    tu.tu_isrss.set(tu.tu_isrss.get().wrapping_add(s.isrss));
    tu.tu_runtime.set(tu.tu_runtime.get().add(s.runtime));
}

/// `calctsru(tup, &up, &sp, &ip)`: the statclock ticks of `tup` turned into user, system
/// and interrupt time at `stathz` ticks a second (zero times for no ticks).
pub fn calctsru(tup: &Tusage, stathz: i32) -> (Timespec, Timespec, Timespec) {
    let st = tup.tu_ticks[TU_STICKS].get();
    let ut = tup.tu_ticks[TU_UTICKS].get();
    let it = tup.tu_ticks[TU_ITICKS].get();

    if st + ut + it == 0 {
        return (Timespec::ZERO, Timespec::ZERO, Timespec::ZERO);
    }
    let stathz = u64::try_from(stathz).unwrap_or(0).max(1);
    let to_ts = |ticks: u64| Timespec::from_nsec(ticks * 1_000_000_000 / stathz);
    (to_ts(ut), to_ts(st), to_ts(it))
}

/// `calcru(tup, &up, &sp, &ip)`: [`calctsru`] in `timeval`s: (user, system, interrupt).
pub fn calcru(tup: &Tusage, stathz: i32) -> (Timeval, Timeval, Timeval) {
    let (u, s, i) = calctsru(tup, stathz);
    (
        Timeval::from_timespec(u),
        Timeval::from_timespec(s),
        Timeval::from_timespec(i),
    )
}

/// `ruadd(ru, ru2)`: adds `ru2` into `ru`: the times, the larger `ru_maxrss`, the sums of
/// the rest.
pub fn ruadd(ru: &Rusage, ru2: &Rusage) {
    ru.ru_utime.set(ru.ru_utime.get().add(ru2.ru_utime.get()));
    ru.ru_stime.set(ru.ru_stime.get().add(ru2.ru_stime.get()));
    if ru.ru_maxrss.get() < ru2.ru_maxrss.get() {
        ru.ru_maxrss.set(ru2.ru_maxrss.get());
    }
    // ru_first .. ru_last: ru_ixrss through ru_nivcsw
    for (a, b) in ru.longs()[1..].iter().zip(&ru2.longs()[1..]) {
        a.add(b.get());
    }
}

const _: () = {
    assert!(size_of::<Rlimit>() == 16);
};
/// `PRIO_MIN`.
pub const PRIO_MIN: i32 = -20;
/// `PRIO_MAX`.
pub const PRIO_MAX: i32 = 20;
/// `PRIO_PROCESS`.
pub const PRIO_PROCESS: i32 = 0;
/// `PRIO_PGRP`.
pub const PRIO_PGRP: i32 = 1;
/// `PRIO_USER`.
pub const PRIO_USER: i32 = 2;
/// `RUSAGE_SELF`.
pub const RUSAGE_SELF: i32 = 0;
/// `RUSAGE_CHILDREN`.
pub const RUSAGE_CHILDREN: i32 = -1;
/// `RUSAGE_THREAD`.
pub const RUSAGE_THREAD: i32 = 1;

/// `RUCHECK_INTERVAL`: resource usage check interval in msec.
const RUCHECK_INTERVAL: u64 = 1000;
/// `SIGXCPU_INTERVAL`: SIGXCPU interval in seconds of process runtime.
const SIGXCPU_INTERVAL: i64 = 5;
/// `SIGKILL`.
const SIGKILL: i32 = 9;
/// `SIGXCPU`.
const SIGXCPU: i32 = 24;

/// `maxdmap`: patchable maximum data limit.
pub static MAXDMAP: AtomicU64 = AtomicU64::new(MAXDSIZ as u64);
/// `maxsmap`: patchable maximum stack limit.
pub static MAXSMAP: AtomicU64 = AtomicU64::new(MAXSSIZ as u64);

/// `rlimit_lock`: serializes resource limit updates; held together with `ps_mtx` when a
/// process's `ps_limit` changes.
pub static RLIMIT_LOCK: RawRwlock = RawRwlock::new("rlimitlk");

/// `struct rlimit` as userland lays it out: `rlim_cur` then `rlim_max`, native endian.
pub fn rlimit_to_bytes(r: &Rlimit) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&r.rlim_cur.to_ne_bytes());
    b[8..].copy_from_slice(&r.rlim_max.to_ne_bytes());
    b
}

/// The inverse of [`rlimit_to_bytes`].
pub fn rlimit_from_bytes(b: &[u8; 16]) -> Rlimit {
    let mut cur = [0u8; 8];
    let mut max = [0u8; 8];
    cur.copy_from_slice(&b[..8]);
    max.copy_from_slice(&b[8..]);
    Rlimit {
        rlim_cur: Rlim::from_ne_bytes(cur),
        rlim_max: Rlim::from_ne_bytes(max),
    }
}

/// `tuagg_get_proc`: the usage of one thread.
pub fn tuagg_get_proc<G: ProcGlue>(tu: &Tusage, p: &Thread<G>) {
    tu.clear();
    tuagg_sumup(tu, &p.p_tu);
}

/// `tuagg_get_process`: the usage of a process: its own plus all living threads.
pub fn tuagg_get_process<G: ProcGlue>(tu: &Tusage, pr: &Process<G>) {
    tu.clear();
    let g = pr.ps_mtx.lock();
    tuagg_sumup(tu, &pr.ps_tu);
    // add on all living threads
    for thread in pr.ps_threads.iter(&g) {
        tuagg_sumup(tu, &thread.p_tu);
    }
    drop(g);
}

/// `tuagg_add_process`: the process's `ps_tu` takes `p`'s usage, which is reset; `p` is the
/// running thread or `SDEAD`, under the process's `ps_mtx` (`g`, at `IPL_HIGH`, above
/// `IPL_STATCLOCK`).
pub fn tuagg_add_process<G: ProcGlue>(
    pr: &Process<G>,
    p: &Thread<G>,
    g: &hal::mutex::MutexedGuard<'_, PsMtx<G>>,
) {
    // The guard is this process's (the thread list checks it).
    let _ = pr.ps_threads.is_empty(g);
    let generation = pr.ps_tu.enter();
    tuagg_sumup(&pr.ps_tu, &p.p_tu);
    pr.ps_tu.leave(generation);

    // Now reset CPU time usage for the thread.
    p.p_tu.clear();
}

/// The process `ps_limit` names (every process has one from `fork1`, `limit0` for process 0).
fn ps_limit<G: ProcGlue>(pr: &Process<G>) -> PlimitRef {
    match pr.limit() {
        Some(l) => l,
        None => kpanic(format_args!("ps_limit: none")),
    }
}

/// `process_initialize`'s `timeout_set(&pr->ps_rucheck_to, rucheck, pr)`: the timeout is
/// named by the process's pid (as a thread's sleep timeout by its tid), which `rucheck`
/// looks up: no reference the process would hold to itself.
pub fn rucheck_init<G: ProcGlue>(pr: &Process<G>) {
    pr.ps_rucheck_to.set(
        Some(rucheck::<G>),
        core::ptr::without_provenance_mut(pr.ps_pid as usize),
        crate::timeout::KCLOCK_NONE,
        0,
    );
}

/// `timeout_add_msec(&pr->ps_rucheck_to, RUCHECK_INTERVAL)`.
pub fn rucheck_arm<G: ProcGlue>(pr: &ProcessRef<G>) {
    let _ = hal::pin::project(pr.as_pin(), |p| &p.ps_rucheck_to).add_msec(RUCHECK_INTERVAL);
}

/// `rucheck`: check if the process (named by its pid, [`rucheck_init`]) exceeds its cpu
/// resource allocation. If over max, kill it. Runs under the kernel lock (a timeout without
/// `TIMEOUT_MPSAFE`); a process gone meanwhile (its `exit1` deleted the timeout first) is
/// left alone.
fn rucheck<G: ProcGlue>(arg: *mut c_void) {
    klock::kernel_assert_locked(); // KERNEL_ASSERT_LOCKED()

    let pr = {
        let r = klock::reader();
        match crate::proc::prfind::<G, _>(&r, arg.addr() as Pid) {
            Some(pr) => G::process_ref(pr),
            None => return,
        }
    };
    let tu = Tusage::new();

    let g = pr.ps_mtx.lock();
    let rlim = g
        .limit
        .as_ref()
        .map_or(Rlimit::default(), |l| l.pl_rlimit[RLIMIT_CPU]);
    tuagg_sumup(&tu, &pr.ps_tu);
    for thread in pr.ps_threads.iter(&g) {
        tuagg_sumup(&tu, &thread.p_tu);
    }
    drop(g);

    let runtime = tu.tu_runtime.get().tv_sec;

    if runtime as Rlim >= rlim.rlim_cur {
        if runtime as Rlim >= rlim.rlim_max {
            crate::sig::prsignal(&pr, SIGKILL);
        } else if runtime >= pr.ps_nextxcpu.get() {
            crate::sig::prsignal(&pr, SIGXCPU);
            pr.ps_nextxcpu.set(runtime + SIGXCPU_INTERVAL);
        }
    }

    rucheck_arm(&pr);
}

/// `lim_fork`: `child` shares `parent`'s limits (and checks its cpu time when limited).
pub fn lim_fork<G: ProcGlue>(parent: &Process<G>, child: &ProcessRef<G>) {
    let limit = ps_limit(parent);
    let cpu = limit.pl_rlimit[RLIMIT_CPU].rlim_cur;
    drop(child.set_limit(limit));

    if cpu != RLIM_INFINITY {
        rucheck_arm(child);
    }
}

/// `lim_write_begin`: an exclusive copy of the process' resource limits to change (see the
/// module's deviations); invalidates the calling thread's read copy. Under `rlimit_lock`'s
/// writer (`w`), which excludes other writers, so `ps_limit` is read without `ps_mtx`.
pub fn lim_write_begin<G: ProcGlue>(p: &Thread<G>, _w: &hal::rwlock::WriteWitness<'_>) -> Plimit {
    p.p_limit.clear();
    *ps_limit(p.process())
}

/// `lim_write_commit`: the changed limits become the process's (when they changed), visible
/// to its other threads.
pub fn lim_write_commit<G: ProcGlue>(
    p: &Thread<G>,
    limit: Plimit,
    _w: &hal::rwlock::WriteWitness<'_>,
) {
    let pr = p.process();
    if *ps_limit(pr) != limit {
        drop(pr.set_limit(PlimitRef::new(limit)));
    }
}

/// `lim_cur_proc(p, which)`: the soft limit `which` of `p`'s process.
pub fn lim_cur_proc<G: ProcGlue>(p: &Thread<G>, which: usize) -> Rlim {
    kassert!(which < RLIM_NLIMITS);
    ps_limit(p.process()).pl_rlimit[which].rlim_cur
}

/// `getpriority(2)`: the lowest nice of the processes `which`/`who` names, as the system
/// call returns it (`NZERO` taken off); `ESRCH` when none matches.
pub fn sys_getpriority<G: ProcGlue>(curp: &Thread<G>, which: i32, who: Id) -> Result<i32, Errno> {
    let mut low = NZERO + PRIO_MAX + 1;
    let r = klock::reader();
    let mut take = |pr: &Process<G>| {
        if i32::from(pr.ps_nice.get()) < low {
            low = i32::from(pr.ps_nice.get());
        }
    };

    match which {
        PRIO_PROCESS => {
            let pr = if who == 0 {
                Some(curp.process())
            } else {
                crate::proc::prfind::<G, _>(&r, who as Pid)
            };
            if let Some(pr) = pr {
                take(pr);
            }
        }
        PRIO_PGRP => {
            let pg = if who == 0 {
                curp.process().pgrp(&r).map(|pg| &**pg)
            } else {
                crate::proc::pgfind::<G, _>(&r, who as Pid)
            };
            if let Some(pg) = pg {
                for pr in pg.pg_members.iter(&r) {
                    take(pr);
                }
            }
        }
        PRIO_USER => {
            let who = if who == 0 {
                curp.ucred().cr_uid.get()
            } else {
                who
            };
            for pr in G::tables().allprocess.iter(&r) {
                if pr.ucred().cr_uid.get() == who {
                    take(pr);
                }
            }
        }
        _ => return Err(Errno::EINVAL),
    }
    if low == NZERO + PRIO_MAX + 1 {
        return Err(Errno::ESRCH);
    }
    Ok(low - NZERO)
}

/// `setpriority(2)`: the nice of the processes `which`/`who` names set to `prio` (the last
/// error of `donice`); `ESRCH` when none matches.
pub fn sys_setpriority<G: ProcGlue>(
    curp: &Thread<G>,
    which: i32,
    who: Id,
    prio: i32,
) -> Result<(), Errno> {
    let mut found = false;
    let mut error = Ok(());
    let r = klock::reader();

    match which {
        PRIO_PROCESS => {
            let pr = if who == 0 {
                Some(curp.process())
            } else {
                crate::proc::prfind::<G, _>(&r, who as Pid)
            };
            if let Some(pr) = pr {
                error = donice(curp, pr, prio);
                found = true;
            }
        }
        PRIO_PGRP => {
            let pg = if who == 0 {
                curp.process().pgrp(&r).map(|pg| &**pg)
            } else {
                crate::proc::pgfind::<G, _>(&r, who as Pid)
            };
            if let Some(pg) = pg {
                for pr in pg.pg_members.iter(&r) {
                    error = donice(curp, pr, prio);
                    found = true;
                }
            }
        }
        PRIO_USER => {
            let who = if who == 0 {
                curp.ucred().cr_uid.get()
            } else {
                who
            };
            for pr in G::tables().allprocess.iter(&r) {
                if pr.ucred().cr_uid.get() == who {
                    error = donice(curp, pr, prio);
                    found = true;
                }
            }
        }
        _ => return Err(Errno::EINVAL),
    }
    if !found {
        return Err(Errno::ESRCH);
    }
    error
}

/// `donice`: the nice of `chgpr` set to `n` (clamped to `[PRIO_MIN, PRIO_MAX]`) on behalf of
/// `curp`: `EPERM` for another user's process unless root, `EACCES` to lower it unless
/// root; its threads' priorities recomputed under `ps_mtx` and the scheduler's lock.
pub fn donice<G: ProcGlue>(curp: &Thread<G>, chgpr: &Process<G>, n: i32) -> Result<(), Errno> {
    let ucred = curp.ucred();
    let chguid = chgpr.ucred().cr_uid.get();

    if ucred.cr_uid.get() != 0
        && ucred.cr_ruid.get() != 0
        && ucred.cr_uid.get() != chguid
        && ucred.cr_ruid.get() != chguid
    {
        return Err(Errno::EPERM);
    }
    let n = n.clamp(PRIO_MIN, PRIO_MAX) + NZERO;
    if n < i32::from(chgpr.ps_nice.get()) && crate::proc::suser(curp).is_err() {
        return Err(Errno::EACCES);
    }
    chgpr.ps_nice.set(n as u8);
    let g = chgpr.ps_mtx.lock();
    let h = hal::thread::sched_lock();
    for p in chgpr.ps_threads.iter(&g) {
        crate::sched::setpriority(&h, p, p.p_estcpu.get(), n as u8);
    }
    drop(h);
    drop(g);
    Ok(())
}

/// `setrlimit(2)`: the limit `which` from the `struct rlimit` at `rlp`.
pub fn sys_setrlimit<G: ProcGlue>(
    p: &Thread<G>,
    which: u32,
    rlp: UserPtr<u8>,
) -> Result<(), Errno> {
    let mut buf = [0u8; 16];
    rlp.copyin_slice(&mut buf)?;
    // KTRACE: not configured.
    dosetrlimit(p, which, rlimit_from_bytes(&buf))
}

/// `dosetrlimit`: limit `which` of `p`'s process set to `lim`, clamped to the system's
/// maximum: `EINVAL` for an unknown limit or a soft limit above the hard one, `suser`'s
/// error to raise a hard limit. A cpu limit arms the check; a stack limit changes what of the
/// stack is accessible.
pub fn dosetrlimit<G: ProcGlue>(p: &Thread<G>, which: u32, lim: Rlimit) -> Result<(), Errno> {
    let mut limp = lim;
    let which = which as usize;
    if which >= RLIM_NLIMITS || limp.rlim_cur > limp.rlim_max {
        return Err(Errno::EINVAL);
    }

    let w = RLIMIT_LOCK.write();

    let alimp = ps_limit(p.process()).pl_rlimit[which];
    if limp.rlim_max > alimp.rlim_max {
        crate::proc::suser(p)?;
    }

    // Get exclusive write access to the limit structure.
    let mut limit = lim_write_begin(p, &w);
    let alimp = limit.pl_rlimit[which];

    let maxlim: Rlim = match which {
        RLIMIT_DATA => MAXDMAP.load(Ordering::Relaxed),
        RLIMIT_STACK => MAXSMAP.load(Ordering::Relaxed),
        RLIMIT_NOFILE => <G::FileGlue as crate::file::FileGlue>::maxfiles() as Rlim,
        RLIMIT_NPROC => G::maxprocess() as Rlim,
        _ => RLIM_INFINITY,
    };

    if limp.rlim_max > maxlim {
        limp.rlim_max = maxlim;
    }
    if limp.rlim_cur > limp.rlim_max {
        limp.rlim_cur = limp.rlim_max;
    }

    if which == RLIMIT_CPU && limp.rlim_cur != RLIM_INFINITY && alimp.rlim_cur == RLIM_INFINITY {
        rucheck_arm(p.process_ref());
    }

    if which == RLIMIT_STACK && limp.rlim_cur != alimp.rlim_cur {
        // Stack is allocated to the max at exec time with only "rlim_cur" bytes accessible.
        // If stack limit is going up make more accessible, if going down make inaccessible.
        if let Some(vm) = p.vmspace_share() {
            let minsaddr = vm.vm_minsaddr.load(Ordering::Relaxed);
            // MACHINE_STACK_GROWS_UP: neither amd64 nor arm64.
            let (prot, size, addr) = if limp.rlim_cur > alimp.rlim_cur {
                (
                    PROT_READ | PROT_WRITE,
                    (limp.rlim_cur - alimp.rlim_cur) as usize,
                    minsaddr - limp.rlim_cur as usize,
                )
            } else {
                (
                    PROT_NONE,
                    (alimp.rlim_cur - limp.rlim_cur) as usize,
                    minsaddr - alimp.rlim_cur as usize,
                )
            };
            let addr = trunc_page(addr);
            let size = round_page(size);
            let _ = crate::vm::map::uvm_map_protect(
                &vm.vm_map,
                addr,
                addr + size,
                prot,
                crate::vm::map::UVM_ET_STACK,
                false,
                false,
            );
        }
    }

    limit.pl_rlimit[which] = limp;

    lim_write_commit(p, limit, &w);
    drop(w);

    Ok(())
}

/// `getrlimit(2)`: the limit `which` of the calling thread's copy, copied out to `rlp`.
pub fn sys_getrlimit<G: ProcGlue>(
    p: &Thread<G>,
    which: i32,
    rlp: UserPtr<u8>,
) -> Result<(), Errno> {
    if which < 0 || which as usize >= RLIM_NLIMITS {
        return Err(Errno::EINVAL);
    }
    // lim_read_enter: the thread's copy, refreshed when its process's limits changed.
    let _ = p.lim_cur(RLIMIT_CPU);
    let alimp = p.p_limit.get(which as usize);
    // KTRACE: not configured.
    rlp.copyout_slice(&rlimit_to_bytes(&alimp))
}

/// `getrusage(2)`: the usage `who` of `p`, copied out to `rusage`.
pub fn sys_getrusage<G: ProcGlue>(
    p: &Thread<G>,
    who: i32,
    rusage: UserPtr<u8>,
) -> Result<(), Errno> {
    let ru = Rusage::new();
    dogetrusage(p, who, &ru)?;
    // KTRACE: not configured.
    rusage.copyout_slice(&ru.to_bytes())
}

/// `dogetrusage`: the resource usage `who` (`RUSAGE_SELF`, `RUSAGE_THREAD`,
/// `RUSAGE_CHILDREN`) of `p`, into `rup`; `EINVAL` for another `who`.
pub fn dogetrusage<G: ProcGlue>(p: &Thread<G>, who: i32, rup: &Rusage) -> Result<(), Errno> {
    let pr = p.process();
    let tu = Tusage::new();

    klock::kernel_assert_locked(); // KERNEL_ASSERT_LOCKED()

    match who {
        RUSAGE_SELF => {
            // start with the sum of dead threads, if any
            {
                let r = klock::reader();
                match pr.ps_ru.get(&r) {
                    Some(ru) => rup.copy_from(ru),
                    None => rup.copy_from(&Rusage::new()),
                }
            }
            tuagg_sumup(&tu, &pr.ps_tu);

            // add on all living threads
            let g = pr.ps_mtx.lock();
            for thread in pr.ps_threads.iter(&g) {
                ruadd(rup, &thread.p_ru);
                tuagg_sumup(&tu, &thread.p_tu);
            }
            drop(g);

            let (ut, st, _) = calcru(&tu, G::stathz());
            rup.ru_utime.set(ut);
            rup.ru_stime.set(st);

            rup.ru_ixrss.set(tu.tu_ixrss.get() as i64);
            rup.ru_idrss.set(tu.tu_idrss.get() as i64);
            rup.ru_isrss.set(tu.tu_isrss.get() as i64);
        }
        RUSAGE_THREAD => {
            rup.copy_from(&p.p_ru);
            let (ut, st, _) = calcru(&p.p_tu, G::stathz());
            rup.ru_utime.set(ut);
            rup.ru_stime.set(st);
        }
        RUSAGE_CHILDREN => rup.copy_from(&pr.ps_cru),
        _ => return Err(Errno::EINVAL),
    }
    Ok(())
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn limit0_is_the_c_s() {
        testutil::install();
        let l = lim_startup(7030, 1 << 30, 4096);
        assert_eq!(l.pl_rlimit[RLIMIT_CPU].rlim_cur, RLIM_INFINITY);
        assert_eq!(
            l.pl_rlimit[RLIMIT_NOFILE],
            Rlimit {
                rlim_cur: 64,
                rlim_max: 1024
            }
        );
        // maxfiles - NOFILE below NOFILE_MAX
        let small = lim_startup(500, 1 << 30, 4096);
        assert_eq!(small.pl_rlimit[RLIMIT_NOFILE].rlim_max, 436);
        // maxfiles - NOFILE below NOFILE: NOFILE
        let tiny = lim_startup(100, 1 << 30, 4096);
        assert_eq!(tiny.pl_rlimit[RLIMIT_NOFILE].rlim_max, 64);
        assert_eq!(l.pl_rlimit[RLIMIT_NPROC].rlim_cur, 80);
        assert_eq!(l.pl_rlimit[RLIMIT_NPROC].rlim_max, RLIM_INFINITY);
        assert_eq!(l.pl_rlimit[RLIMIT_RSS].rlim_max, 1 << 30);
        assert_eq!(l.pl_rlimit[RLIMIT_MEMLOCK].rlim_max, 64 * 1024 * 4096);
        assert_eq!(l.pl_rlimit[RLIMIT_MEMLOCK].rlim_cur, 64 * 1024 * 4096 / 3);
    }

    #[test]
    fn limits_are_shared_and_copied_never_written() {
        testutil::install();
        let base = lim_startup(7030, 1 << 30, 4096);
        let parent = PlimitRef::new(base);
        let child = parent.clone(); // lim_fork
        assert!(PlimitRef::ptr_eq(&parent, &child));
        assert_eq!(PlimitRef::count(&parent), 2);
        // lim_write_begin: a copy to change; lim_write_commit: placed.
        let mut v = *child;
        v.pl_rlimit[RLIMIT_CORE].rlim_cur = 0;
        let changed = PlimitRef::new(v);
        lim_free(child);
        assert_eq!(PlimitRef::count(&parent), 1);
        assert_eq!(parent.pl_rlimit[RLIMIT_CORE].rlim_cur, RLIM_INFINITY);
        assert_eq!(changed.pl_rlimit[RLIMIT_CORE].rlim_cur, 0);
        let copy = lim_copy(&changed);
        assert!(!PlimitRef::ptr_eq(&copy, &changed));
        assert_eq!(*copy, *changed);
    }

    #[test]
    fn a_thread_s_copy_follows_the_structure_it_came_from() {
        testutil::install();
        let a = PlimitRef::new(lim_startup(7030, 1 << 30, 4096));
        let c = LimitCache::new();
        assert!(!c.is_from(PlimitRef::addr(&a)));
        c.fill(PlimitRef::addr(&a), &a);
        assert!(c.is_from(PlimitRef::addr(&a)));
        assert_eq!(c.cur(RLIMIT_NOFILE), 64);
        assert_eq!(c.get(RLIMIT_NOFILE).rlim_max, 1024);
        c.clear();
        assert!(!c.is_from(PlimitRef::addr(&a)));
    }

    #[test]
    fn ruadd_adds_times_sums_and_keeps_the_larger_maxrss() {
        let a = Rusage::new();
        let b = Rusage::new();
        a.ru_utime.set(Timeval::new(1, 700_000));
        b.ru_utime.set(Timeval::new(2, 400_000));
        a.ru_maxrss.set(10);
        b.ru_maxrss.set(30);
        a.ru_nvcsw.set(5);
        b.ru_nvcsw.set(7);
        b.ru_ixrss.set(3);
        ruadd(&a, &b);
        assert_eq!(a.ru_utime.get(), Timeval::new(4, 100_000));
        assert_eq!(a.ru_maxrss.get(), 30);
        assert_eq!(a.ru_nvcsw.get(), 12);
        assert_eq!(a.ru_ixrss.get(), 3);
        let bytes = a.to_bytes();
        assert_eq!(bytes.len(), 144);
        assert_eq!(i64::from_ne_bytes(bytes[0..8].try_into().unwrap()), 4);
        assert_eq!(i64::from_ne_bytes(bytes[32..40].try_into().unwrap()), 30);
        let c = Rusage::new();
        c.copy_from(&a);
        assert_eq!(c.to_bytes(), bytes);
    }

    #[test]
    fn tusage_sums_and_times() {
        let t = Tusage::new();
        let g = t.enter();
        assert_eq!(g & 1, 1);
        t.tu_ticks[TU_UTICKS].set(100);
        t.tu_ticks[TU_STICKS].set(50);
        t.leave(g);
        t.add_runtime(Timespec::new(1, 500_000_000));
        t.add_runtime(Timespec::new(0, 600_000_000));
        let sum = Tusage::new();
        tuagg_sumup(&sum, &t);
        tuagg_sumup(&sum, &t);
        assert_eq!(sum.tu_ticks[TU_UTICKS].get(), 200);
        assert_eq!(sum.tu_runtime.get(), Timespec::new(4, 200_000_000));
        let (u, s, i) = calcru(&t, 100);
        assert_eq!(u, Timeval::new(1, 0));
        assert_eq!(s, Timeval::new(0, 500_000));
        assert_eq!(i, Timeval::new(0, 0));
        t.clear();
        assert_eq!(
            calctsru(&t, 100),
            (Timespec::ZERO, Timespec::ZERO, Timespec::ZERO)
        );
    }
}
/* </TESTS> */
