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
 * 4. This work was done expressly for inclusion into FreeBSD.  Other use
 *    is allowed if this notation is included.
 * 5. Modifications may be freely made to this file if the above conditions
 *    are met.
 */
/* </LICENSES> */

/* <CODE> */
//! pipe(2): a pair of circular buffers in pageable kernel memory, one per direction, that
//! share one lock. The high-performance replacement for the socket-based pipes scheme
//! originally used in FreeBSD/4.4Lite; it does not support all features of sockets, but does
//! do everything that pipes normally do.
//!
//! Upstream: sys/kern/sys_pipe.c @ 3ce1f3f79392
//! Upstream: sys/sys/pipe.h @ 3ce1f3f79392
//! LZ: sys/kern/sys_pipe.rs@3c62ede68802
//! LZ: sys/sys/pipe.rs@3c62ede68802
//!
//! `pipe(2)`/`pipe2(2)` take a [`PipePair`] from the pool `pipepl`: two pipes, `pp_rpipe`
//! ([`Dir::Rd`], read through `fds[0]`) and `pp_wpipe` ([`Dir::Wr`], read through
//! `fds[1]`), each with a `PIPE_SIZE`-byte buffer of pageable memory ([`KmBuf`]). Both
//! descriptors read and write (the C opens both `FREAD | FWRITE`): a file reads its own pipe's
//! buffer and writes into its peer's. One rwlock per pair guards the state of both pipes;
//! `PIPE_LOCK` (`pipe_iolock`) gives one reader or writer at a time a buffer, so the copy to
//! or from userland runs with the rwlock released. A write of at most `PIPE_BUF` bytes is
//! atomic; a write larger than `PIPE_SIZE` into an empty buffer grows it to `BIG_PIPE_SIZE`
//! (at most `LIMITBIGPIPES` such pipes). A write whose reader is gone fails with `EPIPE`, on
//! which `dofilewritev` posts `SIGPIPE`. Every sleep is on the pipe's own address, as the C's
//! `cpipe`: the I/O lock (`pipeiolk`), the reader (`piperd`), the writer (`pipewr`), all
//! `PRIBIO | PCATCH`, and `pipe_destroy`'s rundown (`pipecl`, `PRIBIO`).
//!
//! The kernel holds every decision here; what is still `bsd`'s (the file, the descriptor
//! table, the knotes and their klist, the sigio registration, the `uio`) is reached through
//! three traits that `bsd`'s glue implements (`sys/kern/pipe_glue.rs`, `bridges.toml`):
//! [`PipeGlue`] (one value per pipe, inside the pair: the klist, the sigio, the clock),
//! [`UioMove`] (the copy) and [`PipeFiles`] (`dopipe`'s descriptor work).
//!
//! ## Deviations
//! - A pair is allocated with `PR_WAITOK`, and `hal`'s pool answers `None` only where
//!   `pool_get(9)` fails (while `cold`, `sys/kern/subr_pool.rs`'s deviation): `pipe(2)` then
//!   answers `ENOMEM`, as for a buffer that cannot be allocated. LZ's note that `PR_WAITOK`
//!   could not sleep yet is stale (the pool sleeps after boot) and is dropped.
//! - The `rw_assert_wrlock`s of the filters and of the pipe's helpers are the guards' types,
//!   but for [`PipePair::filter`], which the filters call back with the lock held by their
//!   caller: there it is a `KASSERT` on `rw_status` under `diagnostic`, as the C's.
//! - `pipe_rundown` returns `bool` (the C's "non-zero if a rundown is ongoing").
//! - `KTRACE` is not configured (`ktrfds`), as in LZ.
//! - The `#ifndef PIPE_SIZE`/`BIG_PIPE_SIZE` overrides are not configurable; both are `usize`.
//!
//! ## Redesign
//! - LZ's pair is a raw pool item reached by raw pointers (`f_data`, `pipe_peer`,
//!   `pipe_pair`, `pipe_lock`), freed by the second `unsafe fn pipe_destroy`; here it is a
//!   `hal::pool::PoolArc<PipePair<G>>` of a `TypedPool`, and each pipe is owned by one
//!   [`PipeEnd`] (not `Clone`), which [`Pipes::create`] returns, one per descriptor. Dropping
//!   an end is `pipe_close`'s `pipe_destroy`, so it runs exactly once per pipe, and the pair
//!   goes back to the pool when the last handle goes (`pipe_pair_destroy` is
//!   `PipePair`'s `Drop`). A use after free or a second destroy cannot be written.
//! - The `[p]` state the C guards with `pipe_lock` lives inside the lock
//!   (`Rwlocked<PairState>`): the buffer's `in`/`out` and memory, `pipe_busy`, the times.
//!   Four words stay outside it, atomics stored only under the write guard (the setters take
//!   the guard as their witness): `pipe_state` (decisions 11 and 12: `kinfo_file` reads the
//!   word whole, without the lock, as the C does), `pipe_peer != NULL` (`peer_alive`), and
//!   the buffer's `cnt` and `size`, because the filters run under the lock but are called
//!   back by kqueue through the glue (`knote_locked`, `knote_process`), where no guard can
//!   be passed: what they read must be readable without one.
//! - The buffer is a [`KmBuf`] moved out of the lock while `uiomove` runs and put back before
//!   anything else (decision 11), so the three `unsafe` slices of LZ are safe slices; only
//!   the holder of `PIPE_LOCK` takes it.
//! - The kernel names nothing of `bsd`: the pair is generic over the glue type, `G:
//!   PipeGlue`, which `bsd` instantiates once (`Pipes<BsdPipeGlue>`): its klist and sigio sit
//!   in the pair as LZ's did, with no cast and no indirection. The ioctl commands and the
//!   kqueue filters arrive as [`PipeIoctl`] and [`PipeFilter`], translated by the glue from
//!   `bsd`'s numbers; `pipe_stat` returns a [`PipeStat`] and `kern_sysctl`'s fill a
//!   [`PipeKinfo`], which the glue writes into `bsd`'s structures.
//! - A knote's `kn_hook` is the [`Dir`] of the pipe whose klist holds it, not a pointer.
//! - `fp_pipe`, `kn_pipe`, `Pipe::lock`, `pipe_peer`'s raw link and the `int` ioctl argument
//!   helpers have no counterpart: the glue reaches the pair through its file's typed slot,
//!   the lock is the pair's, the peer is `Dir::other`.

use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use hal::kmbuf::KmBuf;
use hal::pool::{PR_WAITOK, PoolArc, TypedPool};
use hal::rwlock::{RW_WRITE, Rwlocked, WriteGuard};
use hal::sleep::{PCATCH, wakeup};
use hal::sys::errno::Errno;
use hal::user::UserPtr;

/// `PIPE_SIZE`: pipe buffer size, keep moderate in value, pipes take kva space.
pub const PIPE_SIZE: usize = 16384;

/// `BIG_PIPE_SIZE`: the size a pipe grows to for a large write.
pub const BIG_PIPE_SIZE: usize = 64 * 1024;

/// `PIPE_ASYNC`: async I/O.
pub const PIPE_ASYNC: u32 = 0x004;
/// `PIPE_WANTR`: reader wants some characters.
pub const PIPE_WANTR: u32 = 0x008;
/// `PIPE_WANTW`: writer wants space to put characters.
pub const PIPE_WANTW: u32 = 0x010;
/// `PIPE_WANTD`: pipe is wanted to be run-down.
pub const PIPE_WANTD: u32 = 0x020;
/// `PIPE_EOF`: pipe is in EOF condition.
pub const PIPE_EOF: u32 = 0x080;
/// `PIPE_LOCK`: thread has exclusive I/O access.
pub const PIPE_LOCK: u32 = 0x100;
/// `PIPE_LWANT`: thread wants exclusive I/O access.
pub const PIPE_LWANT: u32 = 0x200;

/// `MINPIPESIZE`: below this many buffered bytes a reader wakes a blocked writer (write
/// blocking hysteresis).
const MINPIPESIZE: usize = PIPE_SIZE / 3;

/// `LIMITBIGPIPES`: limit the number of "big" pipes.
const LIMITBIGPIPES: u32 = 32;

/// `PIPE_BUF` of `<sys/syslimits.h>`: max bytes for atomic pipe writes.
pub const PIPE_BUF: usize = 512;

/// `PRIBIO` of `<sys/param.h>`: the priority of the pipe's sleeps.
const PRIBIO: i32 = 16;

/// `O_CLOEXEC` of `<sys/fcntl.h>`: `pipe2(2)` sets `FD_CLOEXEC` on both descriptors.
pub const O_CLOEXEC: i32 = 0x10000;
/// `O_CLOFORK` of `<sys/fcntl.h>`: `pipe2(2)` sets `FD_CLOFORK` on both descriptors.
pub const O_CLOFORK: i32 = 0x40000;
/// `FNONBLOCK` (`O_NONBLOCK`) of `<sys/fcntl.h>`: both files are non-blocking.
pub const FNONBLOCK: i32 = 0x0004;

/// `KASSERT(9)`: under feature `diagnostic`, `panic(9)` with `__assert`'s message when `e` is
/// false; otherwise `e` is type-checked and never evaluated.
macro_rules! kassert {
    ($e:expr) => {
        #[cfg(feature = "diagnostic")]
        {
            if !$e {
                pipe_panic(format_args!(
                    "kernel diagnostic assertion \"{}\" failed: file \"{}\", line {}",
                    stringify!($e),
                    file!(),
                    line!()
                ));
            }
        }
        #[cfg(not(feature = "diagnostic"))]
        {
            let _ = || -> bool { $e };
        }
    };
}

/// The C's `#ifdef DIAGNOSTIC` `if (e) panic(msg)`: under feature `diagnostic`, `panic(9)`
/// with `msg` when `e` holds; otherwise `e` is type-checked and never evaluated.
macro_rules! diagnostic_panic {
    ($e:expr, $msg:literal) => {
        #[cfg(feature = "diagnostic")]
        {
            if $e {
                pipe_panic(format_args!($msg));
            }
        }
        #[cfg(not(feature = "diagnostic"))]
        {
            let _ = || -> bool { $e };
        }
    };
}

/// One of the pair's two pipes: which buffer a descriptor reads (it writes its peer's).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    /// `pp_rpipe`, the pipe `fds[0]` reads.
    Rd = 0,
    /// `pp_wpipe`, the pipe `fds[1]` reads.
    Wr = 1,
}

impl Dir {
    /// The other pipe of the pair (the C's `pipe_peer` before it is cleared).
    pub const fn other(self) -> Dir {
        match self {
            Dir::Rd => Dir::Wr,
            Dir::Wr => Dir::Rd,
        }
    }

    /// The index of this pipe in the pair (also the knote's `kn_hook`).
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The pipe of index `i`, if `i` names one.
    pub const fn from_index(i: usize) -> Option<Dir> {
        match i {
            0 => Some(Dir::Rd),
            1 => Some(Dir::Wr),
            _ => None,
        }
    }
}

/// `struct timespec` as the pipe keeps its times: seconds and nanoseconds.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Timespec {
    /// `tv_sec`.
    pub tv_sec: i64,
    /// `tv_nsec`.
    pub tv_nsec: i64,
}

/// `struct pipebuf` without `cnt` and `size` (atomics of [`Pipe`]): the in and out pointers
/// and the buffer's memory, which the holder of `PIPE_LOCK` takes out of the lock while it
/// copies.
struct Pipebuf {
    /// `in`: in pointer.
    r#in: u32,
    /// `out`: out pointer.
    out: u32,
    /// `buffer`: the pages, `None` once freed, or while the I/O lock's holder copies.
    mem: Option<KmBuf>,
}

/// The `[p]` members of `struct pipe` that live inside the pair's lock.
struct PipeState {
    /// `pipe_buffer`: data storage.
    buffer: Pipebuf,
    /// `pipe_busy`: # readers/writers.
    busy: i32,
    /// `pipe_atime`: time of last access.
    atime: Timespec,
    /// `pipe_mtime`: time of last modify.
    mtime: Timespec,
    /// `pipe_ctime` (`[I]` once created): time of status change.
    ctime: Timespec,
}

impl PipeState {
    const fn new() -> Self {
        Self {
            buffer: Pipebuf {
                r#in: 0,
                out: 0,
                mem: None,
            },
            busy: 0,
            atime: Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            mtime: Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            ctime: Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
        }
    }
}

/// What `pp_lock` guards: both pipes' state, indexed by [`Dir`].
struct PairState {
    ends: [PipeState; 2],
}

/// The write guard of a pair: the witness the atomics' setters take.
type Guard<'a> = WriteGuard<'a, PairState>;

/// `struct pipe` outside the lock: the words readable without it, and the glue's part.
struct Pipe<G> {
    /// `[p]` `pipe_state` (`PIPE_*`): stored only under the pair's write guard; read under
    /// it, and without it only by `kern_sysctl` ([`PipePair::kinfo`]), the C's racy read.
    state: AtomicU32,
    /// `[p]` `pipe_peer != NULL`: cleared by the peer's `pipe_destroy`, under the guard.
    peer_alive: AtomicBool,
    /// `[p]` `pipe_buffer.cnt`: the bytes in the buffer, stored only under the guard.
    cnt: AtomicU32,
    /// `[p]` `pipe_buffer.size`: the buffer's size, stored only under the guard.
    size: AtomicU32,
    /// The klist (`[p]`) and the sigio registration (`[S]`) of `bsd`'s glue.
    glue: G,
}

impl<G> Pipe<G> {
    fn new(glue: G) -> Self {
        Self {
            state: AtomicU32::new(0),
            peer_alive: AtomicBool::new(true),
            cnt: AtomicU32::new(0),
            size: AtomicU32::new(0),
            glue,
        }
    }

    /// `cpipe->pipe_state & bits`, as a test.
    fn has(&self, bits: u32) -> bool {
        self.state.load(Ordering::Relaxed) & bits != 0
    }

    /// `cpipe->pipe_state |= bits`, under the guard.
    fn set(&self, _w: &mut Guard<'_>, bits: u32) {
        self.state.fetch_or(bits, Ordering::Relaxed);
    }

    /// `cpipe->pipe_state &= ~bits`, under the guard.
    fn clear(&self, _w: &mut Guard<'_>, bits: u32) {
        self.state.fetch_and(!bits, Ordering::Relaxed);
    }

    fn peer_alive(&self) -> bool {
        self.peer_alive.load(Ordering::Relaxed)
    }

    fn set_peer_alive(&self, _w: &mut Guard<'_>, alive: bool) {
        self.peer_alive.store(alive, Ordering::Relaxed);
    }

    fn cnt(&self) -> usize {
        self.cnt.load(Ordering::Relaxed) as usize
    }

    fn set_cnt(&self, _w: &mut Guard<'_>, cnt: usize) {
        self.cnt.store(cnt as u32, Ordering::Relaxed);
    }

    fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed) as usize
    }

    fn set_size(&self, _w: &mut Guard<'_>, size: usize) {
        self.size.store(size as u32, Ordering::Relaxed);
    }
}

/// `struct pipe_pair`: both pipes and the lock they share, a pool item of [`Pipes`].
pub struct PipePair<G: PipeGlue> {
    /// The pipes' globals (the pool, `nbigpipe`, `amountpipekva`).
    pipes: &'static Pipes<G>,
    /// `pp_lock`: one lock is used per pipe pair in order to obtain exclusive access to the
    /// pipe pair.
    lock: Rwlocked<PairState>,
    /// `pp_rpipe` and `pp_wpipe`, indexed by [`Dir`].
    ends: [Pipe<G>; 2],
}

/// One pipe of a pair, owned by the file that reads it: dropping it is `pipe_close`'s
/// `pipe_destroy` of that pipe, and the pair goes back to the pool with its last handle.
pub struct PipeEnd<G: PipeGlue> {
    pair: PoolArc<PipePair<G>>,
    dir: Dir,
}

impl<G: PipeGlue> PipeEnd<G> {
    /// The pair this end belongs to (cloned by whoever needs it beyond a borrow).
    pub fn pair(&self) -> &PoolArc<PipePair<G>> {
        &self.pair
    }

    /// Which pipe of the pair this end is.
    pub fn dir(&self) -> Dir {
        self.dir
    }

    /// `pipe_close`: destroys this pipe (what the drop does; this names it).
    pub fn close(self) {
        drop(self);
    }
}

/// `fo_ioctl`'s commands as the pipe knows them; the glue maps `bsd`'s numbers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PipeIoctl {
    /// `FIOASYNC` with its `int` argument, non-zero to set.
    Async(bool),
    /// `FIONREAD`.
    Nread,
    /// `FIOSETOWN`, `SIOCSPGRP`, `TIOCSPGRP`: the command (which `sigio_setown` reads) and
    /// its `int` argument.
    SetOwn {
        /// The command.
        cmd: u64,
        /// Its argument.
        arg: i32,
    },
    /// `FIOGETOWN`, `SIOCGPGRP`, `TIOCGPGRP`.
    GetOwn {
        /// The command.
        cmd: u64,
    },
    /// Anything else (`ENOTTY`).
    Other,
}

/// The kqueue filters a pipe supports; the glue maps `EVFILT_*`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PipeFilter {
    /// `EVFILT_READ` (`pipe_rfiltops`).
    Read,
    /// `EVFILT_WRITE` (`pipe_wfiltops`).
    Write,
    /// `EVFILT_EXCEPT` (`pipe_efiltops`), poll's hang-up only.
    Except,
}

/// What a filter decided: what the glue writes into the knote.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct FilterAnswer {
    /// The event is active.
    pub active: bool,
    /// The new `kn_data`, when the filter sets it.
    pub data: Option<i64>,
    /// Set `EV_EOF` in `kn_flags`.
    pub eof: bool,
    /// Set `__EV_HUP` in `kn_flags`.
    pub hup: bool,
}

/// The fields `pipe_stat` fills from the pipe; the glue adds `S_IFIFO` and the credentials.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PipeStat {
    /// `st_blksize`: the buffer's size.
    pub blksize: i32,
    /// `st_size`: the bytes buffered.
    pub size: i64,
    /// `st_blocks`.
    pub blocks: i64,
    /// `st_atim`.
    pub atime: Timespec,
    /// `st_mtim`.
    pub mtime: Timespec,
    /// `st_ctim`.
    pub ctime: Timespec,
}

/// What `kern_sysctl`'s `fill_file` reports of a pipe (`kinfo_file`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PipeKinfo {
    /// `f_data`: this pipe's address.
    pub addr: usize,
    /// `pipe_peer`: the peer's address, 0 once the peer is destroyed.
    pub peer: usize,
    /// `pipe_state`, the word whole.
    pub state: u32,
}

/// `pipe2(2)`'s flags, as `dopipe` uses them.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PipeFlags {
    /// `O_CLOEXEC`: `UF_EXCLOSE` on both descriptors.
    pub cloexec: bool,
    /// `O_CLOFORK`: `UF_FORKCLOSE` on both descriptors.
    pub clofork: bool,
    /// `FNONBLOCK`: both files non-blocking.
    pub nonblock: bool,
}

/// The pipes' globals: the pool of pairs, `nbigpipe` and `amountpipekva`. `bsd` keeps one
/// `static` of it over its glue.
pub struct Pipes<G: PipeGlue> {
    /// `pipe_pair_pool`.
    pool: TypedPool<PipePair<G>>,
    /// `nbigpipe`: the number of pipes whose buffer is `BIG_PIPE_SIZE`.
    nbigpipe: AtomicU32,
    /// `amountpipekva`: the kernel memory held by pipe buffers.
    amountpipekva: AtomicU32,
}

impl<G: PipeGlue> Pipes<G> {
    /// The globals before `pipe_init`.
    pub const fn new() -> Self {
        Self {
            pool: TypedPool::new(),
            nbigpipe: AtomicU32::new(0),
            amountpipekva: AtomicU32::new(0),
        }
    }

    /// `nbigpipe`.
    pub fn nbigpipe(&self) -> u32 {
        self.nbigpipe.load(Ordering::Relaxed)
    }

    /// `amountpipekva`.
    pub fn amountpipekva(&self) -> u32 {
        self.amountpipekva.load(Ordering::Relaxed)
    }

    /// `pr_nout` of `pipepl`: the pairs out of the pool.
    pub fn nout(&self) -> u64 {
        self.pool.nout()
    }
}

impl<G: PipeGlue> Default for Pipes<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `bsd`'s part of one pipe (`sys/kern/pipe_glue.rs`): its klist and its sigio registration,
/// made by [`PipeGlue::new`] as `pipe_pair_create` and `pipe_create` set them up (an empty
/// klist, `sigio_init`), and given back by its drop. The kernel calls every method with the pair's write guard held, so one that
/// reaches a filter finds the lock taken, as the C's `knote_locked` does; but
/// [`PipeGlue::sigio_setown`] and [`PipeGlue::sigio_getown`] (the C's `[S]`, `sigio_lock`),
/// and [`PipeGlue::klist_free`], called by the pair's last drop, when nothing reaches it.
/// A bridge (`bridges.toml`): one call into `bsd` per method, no logic.
pub trait PipeGlue: Send + Sync + Sized + 'static {
    /// `struct knote`, which the kernel only passes through.
    type Knote: ?Sized;

    /// One pipe's klist and sigio registration, set up; `ENOMEM` when they cannot be had
    /// (`pipe(2)` then fails as for a pool item that cannot be had).
    fn new() -> Result<Self, Errno>;

    /// `getnanotime(9)`.
    fn nanotime() -> Timespec;
    /// `knote_locked(&pipe_klist, hint)`.
    fn knote(&self, hint: i64);
    /// `pipe_kqfilter`'s attach: `kn_fop` for `filter`, `kn_hook` the pipe `hook`, then
    /// `klist_insert_locked(&pipe_klist, kn)`.
    fn klist_insert(&self, kn: &Self::Knote, filter: PipeFilter, hook: Dir);
    /// `filt_pipedetach`'s `klist_remove(&pipe_klist, kn)`, the lock held.
    fn klist_remove(&self, kn: &Self::Knote);
    /// `klist_free(&pipe_klist)`: the list must be empty (`pipe_pair_destroy`).
    fn klist_free(&self);
    /// `pgsigio(&pipe_sigio, SIGIO, 0)`.
    fn pgsigio(&self);
    /// `sigio_free(&pipe_sigio)`.
    fn sigio_free(&self);
    /// `sigio_setown(&pipe_sigio, cmd, &arg)`.
    fn sigio_setown(&self, cmd: u64, arg: i32) -> Result<(), Errno>;
    /// `sigio_getown(&pipe_sigio, cmd, &pgid)`: the owner.
    fn sigio_getown(&self, cmd: u64) -> i32;
}

/// The `uio` of a read or a write, as the pipe moves bytes through it (`bsd`'s `Uio` and
/// `uiomove`, through the glue). A bridge (`bridges.toml`).
pub trait UioMove {
    /// `uio_resid`: the bytes still to move.
    fn resid(&self) -> usize;
    /// `uiomove(buf, buf.len(), uio)`: moves `buf` (at most `resid` bytes of it) in the
    /// uio's direction: out of `buf` for a read, into it for a write. It may fault and
    /// sleep, so the pipe calls it with its lock released.
    fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno>;
}

/// `dopipe`'s descriptor work, which stays `bsd`'s until descriptors move (N4): two
/// `falloc`s under `fdplock`, the files set up (`DTYPE_PIPE`, `pipeops`, `FREAD | FWRITE`
/// and `FNONBLOCK`), `fdinsert` of both, `copyout` of the two numbers, and on a failure the
/// C's error paths (`fdrelease` both after a fault; the ends dropped, `wpipe` first, when a
/// `falloc` fails). A bridge (`bridges.toml`).
pub trait PipeFiles<G: PipeGlue> {
    /// `dopipe(p, ufds, flags)` from the new pair on: `rpipe` for `fds[0]`, `wpipe` for
    /// `fds[1]`; `copyout` copies the two numbers to userland.
    fn dopipe(
        &self,
        rpipe: PipeEnd<G>,
        wpipe: PipeEnd<G>,
        flags: PipeFlags,
        copyout: &dyn Fn([i32; 2]) -> Result<(), Errno>,
    ) -> Result<(), Errno>;
}

impl<G: PipeGlue> Pipes<G> {
    /// The pipe system call for the `DTYPE_PIPE` type of pipes.
    pub fn sys_pipe(
        &'static self,
        fdp: UserPtr<i32>,
        files: &dyn PipeFiles<G>,
    ) -> Result<(), Errno> {
        self.dopipe(fdp, 0, files)
    }

    /// `pipe2(2)`: `pipe(2)` with `O_CLOEXEC`, `O_CLOFORK` and `O_NONBLOCK`; any other flag
    /// is `EINVAL`.
    pub fn sys_pipe2(
        &'static self,
        fdp: UserPtr<i32>,
        flags: i32,
        files: &dyn PipeFiles<G>,
    ) -> Result<(), Errno> {
        if flags & !(O_CLOEXEC | O_CLOFORK | FNONBLOCK) != 0 {
            return Err(Errno::EINVAL);
        }
        self.dopipe(fdp, flags, files)
    }

    /// `dopipe(p, ufds, flags)`: a new pair, its two files installed by the glue, the two
    /// descriptors copied out to the two `int`s at `ufds` (the C's `copyout(fds, ufds,
    /// sizeof(fds))`, the same 8 bytes).
    fn dopipe(
        &'static self,
        ufds: UserPtr<i32>,
        flags: i32,
        files: &dyn PipeFiles<G>,
    ) -> Result<(), Errno> {
        let pflags = PipeFlags {
            cloexec: flags & O_CLOEXEC != 0,
            clofork: flags & O_CLOFORK != 0,
            nonblock: flags & FNONBLOCK != 0,
        };
        let (rpipe, wpipe) = self.create()?;
        files.dopipe(rpipe, wpipe, pflags, &|fds| ufds.copyout_slice(&fds))
    }

    /// `pipe_init`: the pool of pipe pairs, `pipepl`, at `ipl` (`IPL_MPFLOOR`), `PR_WAITOK`.
    pub fn init(&'static self, ipl: i32) -> Result<(), Errno> {
        self.pool.init(ipl, PR_WAITOK, "pipepl")
    }

    /// `pipe_pair_create()`: a new pair with both buffers, as its two ends (`pp_rpipe`,
    /// `pp_wpipe`); `ENOMEM` when the pool, the glue or a buffer cannot be had.
    pub fn create(&'static self) -> Result<(PipeEnd<G>, PipeEnd<G>), Errno> {
        let glue = [G::new()?, G::new()?];
        let pair = self
            .pool
            .get_shared(PR_WAITOK, || PipePair::new(self, glue))
            .ok_or(Errno::ENOMEM)?;
        let wpipe = PipeEnd {
            pair: pair.clone(),
            dir: Dir::Wr,
        };
        let rpipe = PipeEnd { pair, dir: Dir::Rd };
        let made = {
            let pp = &rpipe.pair;
            let mut g = pp.lock.write();
            pp.pipe_create(&mut g, Dir::Wr)
                .and_then(|()| pp.pipe_create(&mut g, Dir::Rd))
        };
        match made {
            Ok(()) => Ok((rpipe, wpipe)),
            Err(error) => {
                // err: both pipes destroyed, wpipe first; the last handle frees the pair.
                drop(wpipe);
                drop(rpipe);
                Err(error)
            }
        }
    }
}

impl<G: PipeGlue> PipePair<G> {
    /// A pair as `pipe_pair_create` links it, before `pipe_create`: each pipe the other's
    /// peer, the lock `pipelk`, the glue's klists and sigio set up (`glue`, indexed by
    /// [`Dir`]).
    fn new(pipes: &'static Pipes<G>, glue: [G; 2]) -> Self {
        let [rglue, wglue] = glue;
        Self {
            pipes,
            lock: Rwlocked::new(
                "pipelk",
                PairState {
                    ends: [PipeState::new(), PipeState::new()],
                },
            ),
            ends: [Pipe::new(rglue), Pipe::new(wglue)],
        }
    }

    fn end(&self, dir: Dir) -> &Pipe<G> {
        &self.ends[dir.index()]
    }

    /// The glue's part of pipe `dir`.
    pub fn glue(&self, dir: Dir) -> &G {
        &self.end(dir).glue
    }

    /// Allocate kva for pipe circular buffer, the space is pageable. This routine will
    /// 'realloc' the size of a pipe safely, if it fails it will retain the old buffer. If it
    /// fails it will return `ENOMEM`.
    fn buffer_realloc(&self, g: &mut Guard<'_>, dir: Dir, size: usize) -> Result<(), Errno> {
        let cpipe = self.end(dir);
        let d = dir.index();

        // buffer uninitialized or pipe locked
        kassert!(g.ends[d].buffer.mem.is_none() || cpipe.has(PIPE_LOCK));
        // buffer should be empty
        kassert!(cpipe.cnt() == 0);

        let mem = KmBuf::alloc_pageable(size).ok_or(Errno::ENOMEM)?;

        // free old resources if we are resizing
        self.buffer_free(g, dir);

        g.ends[d].buffer = Pipebuf {
            r#in: 0,
            out: 0,
            mem: Some(mem),
        };
        cpipe.set_size(g, size);

        self.pipes
            .amountpipekva
            .fetch_add(size as u32, Ordering::SeqCst);
        Ok(())
    }

    /// Initialize and allocate VM and memory for pipe (`sigio_init` is `PipeGlue::new`).
    fn pipe_create(&self, g: &mut Guard<'_>, dir: Dir) -> Result<(), Errno> {
        self.buffer_realloc(g, dir, PIPE_SIZE)?;

        let now = G::nanotime();
        let st = &mut g.ends[dir.index()];
        st.ctime = now;
        st.atime = now;
        st.mtime = now;
        Ok(())
    }

    /// `pipe_peer(cpipe)`: the other pipe, or `None` once it is gone or at EOF. Read under
    /// the lock (any mode): the words it reads are stored only under the write guard.
    fn peer_of(&self, dir: Dir) -> Option<Dir> {
        if !self.end(dir).peer_alive() {
            return None;
        }
        let peer = dir.other();
        if self.end(peer).has(PIPE_EOF) {
            return None;
        }
        Some(peer)
    }

    /// Lock a pipe for exclusive I/O access.
    fn iolock(&self, g: &mut Guard<'_>, dir: Dir) -> Result<(), Errno> {
        let cpipe = self.end(dir);

        while cpipe.has(PIPE_LOCK) {
            cpipe.set(g, PIPE_LWANT);
            g.sleep(ptr::from_ref(cpipe), PRIBIO | PCATCH, "pipeiolk")?;
        }
        cpipe.set(g, PIPE_LOCK);
        Ok(())
    }

    /// Unlock a pipe I/O lock.
    fn iounlock(&self, g: &mut Guard<'_>, dir: Dir) {
        let cpipe = self.end(dir);
        kassert!(cpipe.has(PIPE_LOCK));

        cpipe.clear(g, PIPE_LOCK);
        if cpipe.has(PIPE_LWANT) {
            cpipe.clear(g, PIPE_LWANT);
            wakeup(ptr::from_ref(cpipe));
        }
    }

    /// Unlock the pipe I/O lock and go to sleep. Returns `Ok` on success and the I/O lock is
    /// relocked. Otherwise if a signal was caught, the error is returned and the I/O lock is
    /// not locked.
    ///
    /// Any caller must obtain a reference to the pipe by incrementing `pipe_busy` before
    /// calling this function in order ensure that the same pipe is not destroyed while
    /// sleeping.
    fn iosleep(&self, g: &mut Guard<'_>, dir: Dir, wmesg: &'static str) -> Result<(), Errno> {
        self.iounlock(g, dir);
        g.sleep(ptr::from_ref(self.end(dir)), PRIBIO | PCATCH, wmesg)?;
        self.iolock(g, dir)
    }

    /// `pipe_wakeup(cpipe)`: tells the pipe's waiters (knotes, and `SIGIO` for `FIOASYNC`)
    /// that its state changed.
    fn pipe_wakeup(&self, _g: &mut Guard<'_>, dir: Dir) {
        let cpipe = self.end(dir);

        cpipe.glue.knote(0);

        if cpipe.has(PIPE_ASYNC) {
            cpipe.glue.pgsigio();
        }
    }

    /// The buffer of pipe `d`, taken out of the lock by the holder of its `PIPE_LOCK`.
    fn take_mem(g: &mut Guard<'_>, d: usize) -> KmBuf {
        match g.ends[d].buffer.mem.take() {
            Some(mem) => mem,
            // Only the I/O lock's holder takes it, and it puts it back before it lets go: a
            // broken I/O lock protocol is panic(9), never a no-op.
            None => pipe_panic(format_args!("pipe: no buffer under the I/O lock")),
        }
    }

    /// `fo_read` of a pipe: copies out what pipe `dir` buffers, waiting for data unless
    /// `nonblock` (the file's `FNONBLOCK`); 0 bytes at EOF.
    pub fn read(&self, dir: Dir, uio: &mut dyn UioMove, nonblock: bool) -> Result<(), Errno> {
        let rpipe = self.end(dir);
        let d = dir.index();
        let mut nread: usize = 0;

        let mut g = self.lock.write();
        g.ends[d].busy += 1;
        if let Err(error) = self.iolock(&mut g, dir) {
            g.ends[d].busy -= 1;
            self.rundown(&mut g, dir);
            return Err(error);
        }

        let mut error = Ok(());
        // `false` when a sleep failed and the I/O lock is not held (the C's
        // `goto unlocked_error`).
        let iolocked = 'io: {
            while uio.resid() > 0 {
                let cnt = rpipe.cnt();
                if cnt > 0 {
                    // Normal pipe buffer receive.
                    let out = g.ends[d].buffer.out as usize;
                    let size = (rpipe.size() - out).min(cnt).min(uio.resid());
                    let mut mem = Self::take_mem(&mut g, d);
                    let result = g.unlocked(|| uio.uiomove(&mut mem[out..out + size]));
                    g.ends[d].buffer.mem = Some(mem);
                    if let Err(e) = result {
                        error = Err(e);
                        break;
                    }
                    let mut out = out + size;
                    if out >= rpipe.size() {
                        out = 0;
                    }
                    g.ends[d].buffer.out = out as u32;

                    let cnt = rpipe.cnt() - size;
                    rpipe.set_cnt(&mut g, cnt);
                    // If there is no more to read in the pipe, reset its pointers to the
                    // beginning. This improves cache hit stats.
                    if cnt == 0 {
                        g.ends[d].buffer.r#in = 0;
                        g.ends[d].buffer.out = 0;
                    }
                    nread += size;
                } else {
                    // detect EOF condition
                    // read returns 0 on EOF, no need to set error
                    if rpipe.has(PIPE_EOF) {
                        break;
                    }

                    // If the "write-side" has been blocked, wake it up.
                    if rpipe.has(PIPE_WANTW) {
                        rpipe.clear(&mut g, PIPE_WANTW);
                        wakeup(ptr::from_ref(rpipe));
                    }

                    // Break if some data was read.
                    if nread > 0 {
                        break;
                    }

                    // Handle non-blocking mode operation.
                    if nonblock {
                        error = Err(Errno::EAGAIN);
                        break;
                    }

                    // Wait for more data.
                    rpipe.set(&mut g, PIPE_WANTR);
                    if let Err(e) = self.iosleep(&mut g, dir, "piperd") {
                        error = Err(e);
                        break 'io false;
                    }
                }
            }
            true
        };
        if iolocked {
            self.iounlock(&mut g, dir);

            if error.is_ok() {
                g.ends[d].atime = G::nanotime();
            }
        }
        // unlocked_error:
        g.ends[d].busy -= 1;

        if !self.rundown(&mut g, dir) && rpipe.cnt() < MINPIPESIZE {
            // Handle write blocking hysteresis.
            if rpipe.has(PIPE_WANTW) {
                rpipe.clear(&mut g, PIPE_WANTW);
                wakeup(ptr::from_ref(rpipe));
            }
        }

        if rpipe.size() - rpipe.cnt() >= PIPE_BUF {
            self.pipe_wakeup(&mut g, dir);
        }

        drop(g);
        error
    }

    /// `fo_write` of a pipe: copies into the buffer of `dir`'s peer, waiting for space
    /// unless `nonblock`; `EPIPE` once the reader is gone. Writes of at most `PIPE_BUF` bytes
    /// are atomic.
    pub fn write(&self, dir: Dir, uio: &mut dyn UioMove, nonblock: bool) -> Result<(), Errno> {
        let mut g = self.lock.write();

        // Detect loss of pipe read side, issue SIGPIPE if lost.
        let Some(wdir) = self.peer_of(dir) else {
            return Err(Errno::EPIPE);
        };
        let wpipe = self.end(wdir);
        let w = wdir.index();

        g.ends[w].busy += 1;
        if let Err(error) = self.iolock(&mut g, wdir) {
            g.ends[w].busy -= 1;
            self.rundown(&mut g, wdir);
            return Err(error);
        }

        // If it is advantageous to resize the pipe buffer, do so.
        if uio.resid() > PIPE_SIZE && wpipe.size() <= PIPE_SIZE && wpipe.cnt() == 0 {
            let npipe = self.pipes.nbigpipe.fetch_add(1, Ordering::SeqCst) + 1;
            if npipe > LIMITBIGPIPES || self.buffer_realloc(&mut g, wdir, BIG_PIPE_SIZE).is_err() {
                self.pipes.nbigpipe.fetch_sub(1, Ordering::SeqCst);
            }
        }

        let orig_resid = uio.resid();

        let mut error = Ok(());
        // `false` when a sleep failed and the I/O lock is not held (the C's
        // `goto unlocked_error`).
        let iolocked = 'io: {
            while uio.resid() > 0 {
                if wpipe.has(PIPE_EOF) {
                    error = Err(Errno::EPIPE);
                    break;
                }

                let mut space = wpipe.size() - wpipe.cnt();

                // Writes of size <= PIPE_BUF must be atomic.
                if space < uio.resid() && orig_resid <= PIPE_BUF {
                    space = 0;
                }

                if space > 0 {
                    // Transfer size is minimum of uio transfer and free space in pipe
                    // buffer.
                    let size = space.min(uio.resid());
                    // First segment to transfer is minimum of transfer size and contiguous
                    // space in pipe buffer. If first segment to transfer is less than the
                    // transfer size, we've got a wraparound in the buffer.
                    let bufsize = wpipe.size();
                    let r#in = g.ends[w].buffer.r#in as usize;
                    let segsize = (bufsize - r#in).min(size);

                    // Transfer first segment
                    let mut mem = Self::take_mem(&mut g, w);
                    let mut result = g.unlocked(|| uio.uiomove(&mut mem[r#in..r#in + segsize]));

                    if result.is_ok() && segsize < size {
                        // Transfer remaining part now, to support atomic writes. Wraparound
                        // happened.
                        diagnostic_panic!(
                            g.ends[w].buffer.r#in as usize + segsize != wpipe.size(),
                            "Expected pipe buffer wraparound disappeared"
                        );

                        result = g.unlocked(|| uio.uiomove(&mut mem[..size - segsize]));
                    }
                    g.ends[w].buffer.mem = Some(mem);
                    if result.is_ok() {
                        let mut r#in = g.ends[w].buffer.r#in as usize + size;
                        if r#in >= wpipe.size() {
                            diagnostic_panic!(
                                r#in != size - segsize + wpipe.size(),
                                "Expected wraparound bad"
                            );
                            r#in = size - segsize;
                        }
                        g.ends[w].buffer.r#in = r#in as u32;

                        let cnt = wpipe.cnt() + size;
                        wpipe.set_cnt(&mut g, cnt);
                        diagnostic_panic!(wpipe.cnt() > wpipe.size(), "Pipe buffer overflow");
                    }
                    if let Err(e) = result {
                        error = Err(e);
                        break;
                    }
                } else {
                    // If the "read-side" has been blocked, wake it up.
                    if wpipe.has(PIPE_WANTR) {
                        wpipe.clear(&mut g, PIPE_WANTR);
                        wakeup(ptr::from_ref(wpipe));
                    }

                    // Don't block on non-blocking I/O.
                    if nonblock {
                        error = Err(Errno::EAGAIN);
                        break;
                    }

                    // We have no more space and have something to offer, wake up
                    // select/poll.
                    self.pipe_wakeup(&mut g, wdir);

                    wpipe.set(&mut g, PIPE_WANTW);
                    if let Err(e) = self.iosleep(&mut g, wdir, "pipewr") {
                        error = Err(e);
                        break 'io false;
                    }

                    // If read side wants to go away, we just issue a signal to ourselves.
                    if wpipe.has(PIPE_EOF) {
                        error = Err(Errno::EPIPE);
                        break;
                    }
                }
            }
            true
        };
        if iolocked {
            self.iounlock(&mut g, wdir);
        }

        // unlocked_error:
        g.ends[w].busy -= 1;

        if !self.rundown(&mut g, wdir) && wpipe.cnt() > 0 {
            // If we have put any characters in the buffer, we wake up the reader.
            if wpipe.has(PIPE_WANTR) {
                wpipe.clear(&mut g, PIPE_WANTR);
                wakeup(ptr::from_ref(wpipe));
            }
        }

        // Don't return EPIPE if I/O was successful.
        if wpipe.cnt() == 0 && uio.resid() == 0 && error == Err(Errno::EPIPE) {
            error = Ok(());
        }

        if error.is_ok() {
            g.ends[w].mtime = G::nanotime();
        }
        // We have something to offer, wake up select/poll.
        if wpipe.cnt() != 0 {
            self.pipe_wakeup(&mut g, wdir);
        }

        drop(g);
        error
    }

    /// We implement a very minimal set of ioctls for compatibility with sockets: the `int`
    /// answer of `FIONREAD` and the get-owner commands, `None` for the others.
    pub fn ioctl(&self, dir: Dir, cmd: PipeIoctl) -> Result<Option<i32>, Errno> {
        let mpipe = self.end(dir);

        match cmd {
            PipeIoctl::Async(on) => {
                let mut g = self.lock.write();
                if on {
                    mpipe.set(&mut g, PIPE_ASYNC);
                } else {
                    mpipe.clear(&mut g, PIPE_ASYNC);
                }
                Ok(None)
            }
            PipeIoctl::Nread => {
                let _g = self.lock.read();
                Ok(Some(mpipe.cnt() as i32))
            }
            PipeIoctl::SetOwn { cmd, arg } => mpipe.glue.sigio_setown(cmd, arg).map(|()| None),
            PipeIoctl::GetOwn { cmd } => Ok(Some(mpipe.glue.sigio_getown(cmd))),
            PipeIoctl::Other => Err(Errno::ENOTTY),
        }
    }

    /// `fo_stat` of a pipe: what is buffered, the buffer's size and the times, under the
    /// lock shared. Left as 0 (by the glue): `st_dev`, `st_ino`, `st_nlink`, `st_rdev`,
    /// `st_flags`, `st_gen`; (`st_dev`, `st_ino`) should be unique, as the C says.
    pub fn stat(&self, dir: Dir) -> PipeStat {
        let pipe = self.end(dir);
        let g = self.lock.read();
        let st = &g.ends[dir.index()];
        let blksize = pipe.size() as i32;
        let size = pipe.cnt() as i64;
        PipeStat {
            blksize,
            size,
            blocks: (size + i64::from(blksize) - 1)
                .checked_div(i64::from(blksize))
                .unwrap_or(0),
            atime: st.atime,
            mtime: st.mtime,
            ctime: st.ctime,
        }
    }

    /// Shutdown the pipe, and free resources: [`PipeEnd`]'s drop, once per pipe.
    fn destroy(&self, dir: Dir) {
        let cpipe = self.end(dir);
        let d = dir.index();

        let mut g = self.lock.write();

        self.pipe_wakeup(&mut g, dir);
        cpipe.glue.sigio_free();

        // If the other side is blocked, wake it up saying that we want to close it down.
        cpipe.set(&mut g, PIPE_EOF);
        while g.ends[d].busy != 0 {
            wakeup(ptr::from_ref(cpipe));
            cpipe.set(&mut g, PIPE_WANTD);
            let _ = g.sleep(ptr::from_ref(cpipe), PRIBIO, "pipecl");
        }

        // Disconnect from peer.
        if cpipe.peer_alive() {
            let pdir = dir.other();
            let ppipe = self.end(pdir);
            self.pipe_wakeup(&mut g, pdir);

            ppipe.set(&mut g, PIPE_EOF);
            wakeup(ptr::from_ref(ppipe));
            ppipe.set_peer_alive(&mut g, false);
        }

        self.buffer_free(&mut g, dir);
        // The pair goes back to the pool with its last handle (`pipe_pair_destroy`).
    }

    /// Free kva for pipe circular buffer.
    fn buffer_free(&self, g: &mut Guard<'_>, dir: Dir) {
        let Some(mem) = g.ends[dir.index()].buffer.mem.take() else {
            return;
        };

        let size = self.end(dir).size();

        drop(mem);

        self.pipes
            .amountpipekva
            .fetch_sub(size as u32, Ordering::SeqCst);
        if size > PIPE_SIZE {
            self.pipes.nbigpipe.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Returns `true` if a rundown is currently ongoing.
    fn rundown(&self, g: &mut Guard<'_>, dir: Dir) -> bool {
        let cpipe = self.end(dir);

        if g.ends[dir.index()].busy > 0 || !cpipe.has(PIPE_WANTD) {
            return false;
        }

        // Only wakeup pipe_destroy() once the pipe is no longer busy.
        cpipe.clear(g, PIPE_WANTD | PIPE_WANTR | PIPE_WANTW);
        wakeup(ptr::from_ref(cpipe));
        true
    }

    /// `fo_kqfilter` of a pipe: attaches a knote on the file's pipe `dir`; `filter` is
    /// `None` for one a pipe does not support (`EINVAL`), `select` and `poll` are the
    /// knote's `__EV_SELECT` and `__EV_POLL`.
    pub fn kqfilter(
        &self,
        dir: Dir,
        filter: Option<PipeFilter>,
        select: bool,
        poll: bool,
        kn: &G::Knote,
    ) -> Result<(), Errno> {
        let _g = self.lock.write();
        let wdir = self.peer_of(dir);

        let attach = |which: PipeFilter, hook: Dir| -> Result<(), Errno> {
            self.end(hook).glue.klist_insert(kn, which, hook);
            Ok(())
        };
        match filter {
            Some(PipeFilter::Read) => attach(PipeFilter::Read, dir),
            // The other end of the pipe has been closed. Since the filter now always
            // indicates a pending event, attach the knote to the current side to proceed
            // with the registration.
            Some(PipeFilter::Write) => attach(PipeFilter::Write, wdir.unwrap_or(dir)),
            // Prevent triggering exceptfds.
            Some(PipeFilter::Except) if select => Err(Errno::EPERM),
            // Disallow usage through kevent(2).
            Some(PipeFilter::Except) if !poll => Err(Errno::EINVAL),
            Some(PipeFilter::Except) => attach(PipeFilter::Except, dir),
            None => Err(Errno::EINVAL),
        }
    }

    /// `filt_pipedetach(kn)`: unhooks the knote from the klist of pipe `hook` (its
    /// `kn_hook`), under the pair's lock.
    pub fn detach(&self, hook: Dir, kn: &G::Knote) {
        let _g = self.lock.write();
        self.end(hook).glue.klist_remove(kn);
    }

    /// `filt_piperead`, `filt_pipewrite`, `filt_pipeexcept` for a knote on the file of pipe
    /// `dir` (`poll`: its `__EV_POLL`). The caller holds the pair's write lock: the filter
    /// is called back by kqueue from [`PipeGlue::knote`] or under [`PipePair::with_lock`].
    pub fn filter(&self, dir: Dir, which: PipeFilter, poll: bool) -> FilterAnswer {
        kassert!(self.lock.status() == RW_WRITE);

        let rpipe = self.end(dir);
        let wdir = self.peer_of(dir);
        let gone = rpipe.has(PIPE_EOF) || wdir.is_none();

        match which {
            PipeFilter::Read => {
                let data = rpipe.cnt() as i64;
                if gone {
                    return FilterAnswer {
                        active: true,
                        data: Some(data),
                        eof: true,
                        hup: poll,
                    };
                }
                FilterAnswer {
                    active: data > 0,
                    data: Some(data),
                    ..FilterAnswer::default()
                }
            }
            PipeFilter::Write => match wdir {
                None => FilterAnswer {
                    active: true,
                    data: Some(0),
                    eof: true,
                    hup: poll,
                },
                Some(w) => {
                    let wpipe = self.end(w);
                    let data = (wpipe.size() - wpipe.cnt()) as i64;
                    FilterAnswer {
                        active: data >= PIPE_BUF as i64,
                        data: Some(data),
                        ..FilterAnswer::default()
                    }
                }
            },
            PipeFilter::Except => {
                let hangup = poll && gone;
                FilterAnswer {
                    active: hangup,
                    hup: hangup,
                    ..FilterAnswer::default()
                }
            }
        }
    }

    /// `filt_pipemodify`, `filt_pipeprocess`: `f` (`knote_modify`, `knote_process`) under the
    /// pair's write lock.
    pub fn with_lock<R>(&self, f: impl FnOnce() -> R) -> R {
        let _g = self.lock.write();
        f()
    }

    /// `kern_sysctl`'s `fill_file` of a pipe file: the pipe's address, its peer's (0 once the
    /// peer is destroyed) and `pipe_state`, read without the lock, as the C reads them.
    pub fn kinfo(&self, dir: Dir) -> PipeKinfo {
        let pipe = self.end(dir);
        let peer = if pipe.peer_alive() {
            ptr::from_ref(self.end(dir.other())).addr()
        } else {
            0
        };
        PipeKinfo {
            addr: ptr::from_ref(pipe).addr(),
            peer,
            state: pipe.state.load(Ordering::Relaxed),
        }
    }
}

/// `panic(9)` for this module: every kernel image's panic handler is `panic(9)` with the
/// message (`sys/hal/assert.rs`), and the host tests' is the unwinding panic.
#[cold]
#[allow(clippy::panic)] // KASSERT(9), the C's DIAGNOSTIC checks and a broken I/O lock: panic(9)
fn pipe_panic(msg: core::fmt::Arguments<'_>) -> ! {
    panic!("{msg}")
}

impl<G: PipeGlue> Drop for PipeEnd<G> {
    fn drop(&mut self) {
        self.pair.destroy(self.dir);
    }
}

impl<G: PipeGlue> Drop for PipePair<G> {
    /// `pipe_pair_destroy(pp)`: both klists must be empty; the item goes back to the pool
    /// after this (`PoolArc`'s last drop).
    fn drop(&mut self) {
        for end in &self.ends {
            end.glue.klist_free();
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for pipes over `hal`'s runtime and this crate's doubles of its bridges
    // (`crate::testutil`): every pair comes from `Pipes::create`, its buffers are
    // `PIPE_SIZE` bytes of the page double, and `TestGlue` records what the kernel asked of
    // the glue (knotes, SIGIOs, the klist). A test that counts the pool's items or the big
    // pipes uses a `Pipes` of its own, since the tests run in parallel.

    use std::sync::Mutex;
    use std::sync::atomic::{AtomicI32, AtomicI64};
    use std::thread;
    use std::time::Duration;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use super::*;
    use crate::testutil;

    /// A knote as the tests hand it to the glue: a name.
    struct TestKnote(usize);

    /// The glue of the tests: counts what the kernel asks of it.
    #[derive(Default)]
    struct TestGlue {
        knotes: AtomicU32,
        sigios: AtomicU32,
        sigio_freed: AtomicU32,
        owner: AtomicI32,
        klist: Mutex<Vec<(usize, PipeFilter, Dir)>>,
    }

    impl TestGlue {
        fn klist(&self) -> Vec<(usize, PipeFilter, Dir)> {
            self.klist.lock().unwrap().clone()
        }
    }

    /// The clock of the tests: one second later at every call.
    static CLOCK: AtomicI64 = AtomicI64::new(0);

    impl PipeGlue for TestGlue {
        type Knote = TestKnote;

        fn new() -> Result<Self, Errno> {
            Ok(Self::default())
        }

        fn nanotime() -> Timespec {
            Timespec {
                tv_sec: CLOCK.fetch_add(1, Ordering::Relaxed) + 1,
                tv_nsec: 7,
            }
        }

        fn knote(&self, _hint: i64) {
            self.knotes.fetch_add(1, Ordering::Relaxed);
        }

        fn klist_insert(&self, kn: &TestKnote, filter: PipeFilter, hook: Dir) {
            self.klist.lock().unwrap().push((kn.0, filter, hook));
        }

        fn klist_remove(&self, kn: &TestKnote) {
            self.klist.lock().unwrap().retain(|e| e.0 != kn.0);
        }

        fn klist_free(&self) {
            assert!(
                self.klist.lock().unwrap().is_empty(),
                "klist_free: not empty"
            );
        }

        fn pgsigio(&self) {
            self.sigios.fetch_add(1, Ordering::Relaxed);
        }

        fn sigio_free(&self) {
            self.sigio_freed.fetch_add(1, Ordering::Relaxed);
        }

        fn sigio_setown(&self, _cmd: u64, arg: i32) -> Result<(), Errno> {
            self.owner.store(arg, Ordering::Relaxed);
            Ok(())
        }

        fn sigio_getown(&self, _cmd: u64) -> i32 {
            self.owner.load(Ordering::Relaxed)
        }
    }

    /// The pipes most tests share.
    static PIPES: Pipes<TestGlue> = Pipes::new();

    /// Registers the doubles and makes `pipes`'s pool (once).
    fn setup(pipes: &'static Pipes<TestGlue>) -> &'static Pipes<TestGlue> {
        testutil::install();
        match pipes.init(9) {
            Ok(()) | Err(Errno::EBUSY) => pipes,
            Err(e) => panic!("pipe_init: {e:?}"),
        }
    }

    /// A new pair of the shared pipes: (`fds[0]`'s end, `fds[1]`'s end, the pair).
    fn pair() -> (
        PipeEnd<TestGlue>,
        PipeEnd<TestGlue>,
        PoolArc<PipePair<TestGlue>>,
    ) {
        let (r, w) = setup(&PIPES).create().unwrap();
        let pp = r.pair().clone();
        (r, w, pp)
    }

    /// A user buffer: the source of a write or the destination of a read; it faults (`EFAULT`)
    /// once `fault_at` bytes have moved.
    struct Buf {
        data: Vec<u8>,
        pos: usize,
        source: bool,
        fault_at: usize,
    }

    impl UioMove for Buf {
        fn resid(&self) -> usize {
            self.data.len() - self.pos
        }

        fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
            let n = buf.len().min(self.resid());
            if self.pos + n > self.fault_at {
                return Err(Errno::EFAULT);
            }
            let user = &mut self.data[self.pos..self.pos + n];
            if self.source {
                buf[..n].copy_from_slice(user);
            } else {
                user.copy_from_slice(&buf[..n]);
            }
            self.pos += n;
            Ok(())
        }
    }

    /// `write(2)` of `bytes` on the file of pipe `dir`: the result and how much went in.
    fn write(
        pp: &PipePair<TestGlue>,
        dir: Dir,
        bytes: &[u8],
        nb: bool,
    ) -> (Result<(), Errno>, usize) {
        let mut uio = Buf {
            data: bytes.to_vec(),
            pos: 0,
            source: true,
            fault_at: usize::MAX,
        };
        let error = pp.write(dir, &mut uio, nb);
        (error, uio.pos)
    }

    /// `read(2)` of at most `n` bytes on the file of pipe `dir`.
    fn read(pp: &PipePair<TestGlue>, dir: Dir, n: usize, nb: bool) -> (Result<(), Errno>, Vec<u8>) {
        let mut uio = Buf {
            data: vec![0; n],
            pos: 0,
            source: false,
            fault_at: usize::MAX,
        };
        let error = pp.read(dir, &mut uio, nb);
        uio.data.truncate(uio.pos);
        (error, uio.data)
    }

    /// `n` bytes counting from `start`.
    fn pattern(start: usize, n: usize) -> Vec<u8> {
        (start..start + n).map(|i| (i % 251) as u8).collect()
    }

    /// The buffer pointers of pipe `dir`: (`cnt`, `in`, `out`).
    fn pointers(pp: &PipePair<TestGlue>, dir: Dir) -> (usize, u32, u32) {
        let g = pp.lock.read();
        let b = &g.ends[dir.index()].buffer;
        (pp.end(dir).cnt(), b.r#in, b.out)
    }

    #[test]
    fn create_links_two_pipes_and_the_last_close_frees_the_pair_once() {
        static P: Pipes<TestGlue> = Pipes::new();
        let pipes = setup(&P);
        let pages = testutil::pages_out();
        for reader_first in [true, false] {
            let (r, w) = pipes.create().unwrap();
            assert_eq!((r.dir(), w.dir()), (Dir::Rd, Dir::Wr));
            assert!(PoolArc::ptr_eq(r.pair(), w.pair()));
            assert_eq!(pipes.nout(), 1);
            assert_eq!(pipes.amountpipekva(), 2 * PIPE_SIZE as u32);
            assert_eq!(testutil::pages_out(), pages + 2 * PIPE_SIZE as i64);
            let pp = r.pair().clone();
            for d in [Dir::Rd, Dir::Wr] {
                assert_eq!(pp.end(d).size(), PIPE_SIZE);
                assert!(pp.end(d).peer_alive());
                assert_eq!(pp.end(d).state.load(Ordering::Relaxed), 0);
            }
            assert_eq!(pp.stat(Dir::Rd).ctime, pp.stat(Dir::Rd).mtime);
            drop(pp);
            if reader_first {
                r.close();
                assert_eq!(pipes.nout(), 1, "the writer's file still holds the pair");
                w.close();
            } else {
                w.close();
                assert_eq!(pipes.nout(), 1);
                r.close();
            }
            assert_eq!(pipes.nout(), 0, "the pair went back to the pool once");
            assert_eq!(pipes.amountpipekva(), 0);
            assert_eq!(testutil::pages_out(), pages);
        }
    }

    #[test]
    fn create_fails_with_enomem_before_pipe_init() {
        static P: Pipes<TestGlue> = Pipes::new();
        testutil::install();
        assert_eq!(P.create().err(), Some(Errno::ENOMEM));
        assert_eq!(P.nout(), 0);
    }

    #[test]
    fn write_then_read_in_pieces() {
        let (r, w, pp) = pair();

        assert_eq!(write(&pp, Dir::Wr, b"hello", false), (Ok(()), 5));
        assert_eq!(pp.end(Dir::Rd).cnt(), 5);
        assert_eq!(read(&pp, Dir::Rd, 3, false), (Ok(()), b"hel".to_vec()));
        // Some data was read: the read returns what there was instead of waiting for more.
        assert_eq!(read(&pp, Dir::Rd, 10, false), (Ok(()), b"lo".to_vec()));
        // An empty buffer resets its pointers.
        assert_eq!(pointers(&pp, Dir::Rd), (0, 0, 0));

        assert_eq!(
            read(&pp, Dir::Rd, 1, true),
            (Err(Errno::EAGAIN), Vec::new())
        );
        assert!(!pp.end(Dir::Rd).has(PIPE_LOCK));
        assert_eq!(pp.lock.read().ends[0].busy, 0);

        // The other direction is independent.
        assert_eq!(write(&pp, Dir::Rd, b"back", false), (Ok(()), 4));
        assert_eq!(read(&pp, Dir::Wr, 8, false), (Ok(()), b"back".to_vec()));

        drop(pp);
        w.close();
        r.close();
    }

    #[test]
    fn write_wraps_around_the_buffer() {
        let (r, w, pp) = pair();

        let first = pattern(0, 10_000);
        assert_eq!(write(&pp, Dir::Wr, &first, false), (Ok(()), 10_000));
        assert_eq!(read(&pp, Dir::Rd, 4_000, false).1, first[..4_000]);
        assert_eq!(pointers(&pp, Dir::Rd), (6_000, 10_000, 4_000));
        // 6,384 bytes fit before the end, 1,616 go to the start.
        let second = pattern(10_000, 8_000);
        assert_eq!(write(&pp, Dir::Wr, &second, false), (Ok(()), 8_000));
        assert_eq!(pointers(&pp, Dir::Rd), (14_000, 1_616, 4_000));
        // The read stops at the end of the buffer, then continues at its start.
        let (error, got) = read(&pp, Dir::Rd, 14_000, false);
        assert_eq!(error, Ok(()));
        assert_eq!(got, pattern(4_000, 14_000));
        assert_eq!(pointers(&pp, Dir::Rd), (0, 0, 0));

        drop(pp);
        r.close();
        w.close();
    }

    #[test]
    fn small_writes_are_atomic_even_across_the_wraparound() {
        let (r, w, pp) = pair();

        // Six bytes of room: eight bytes (at most PIPE_BUF) do not fit, nothing is written.
        assert_eq!(
            write(&pp, Dir::Wr, &pattern(0, PIPE_SIZE - 6), true).1,
            PIPE_SIZE - 6
        );
        assert_eq!(
            write(&pp, Dir::Wr, b"abcdefgh", true),
            (Err(Errno::EAGAIN), 0)
        );
        assert_eq!(pp.end(Dir::Rd).cnt(), PIPE_SIZE - 6);
        // A write larger than PIPE_BUF is not atomic: it fills the room, then would block.
        assert_eq!(
            write(&pp, Dir::Wr, &[b'x'; PIPE_BUF + 88], true),
            (Err(Errno::EAGAIN), 6)
        );
        assert_eq!(pp.end(Dir::Rd).cnt(), PIPE_SIZE);
        assert_eq!(pp.lock.read().ends[0].busy, 0);
        assert_eq!(read(&pp, Dir::Rd, PIPE_SIZE, false).1.len(), PIPE_SIZE);

        // 100 bytes left at the end of the buffer, then a PIPE_BUF write: one write, two
        // segments, the bytes in order.
        assert_eq!(write(&pp, Dir::Wr, &pattern(0, 16_000), false).1, 16_000);
        assert_eq!(read(&pp, Dir::Rd, 15_900, false).1.len(), 15_900);
        assert_eq!(
            write(&pp, Dir::Wr, &pattern(16_000, PIPE_BUF), true),
            (Ok(()), PIPE_BUF)
        );
        assert_eq!(pointers(&pp, Dir::Rd), (100 + PIPE_BUF, 128, 15_900));
        assert_eq!(
            read(&pp, Dir::Rd, 1_000, false).1,
            pattern(15_900, 100 + PIPE_BUF)
        );

        drop(pp);
        r.close();
        w.close();
    }

    #[test]
    fn eof_after_the_writer_closes() {
        let (r, w, pp) = pair();

        assert_eq!(write(&pp, Dir::Wr, b"bye", false), (Ok(()), 3));
        w.close();
        assert!(pp.end(Dir::Rd).has(PIPE_EOF));
        assert!(!pp.end(Dir::Rd).peer_alive());
        assert_eq!(pp.glue(Dir::Wr).sigio_freed.load(Ordering::Relaxed), 1);

        // What was written is still read, then EOF: 0 bytes, no error, no waiting.
        assert_eq!(read(&pp, Dir::Rd, 8, false), (Ok(()), b"bye".to_vec()));
        assert_eq!(read(&pp, Dir::Rd, 8, false), (Ok(()), Vec::new()));
        // Nobody reads what rf would write.
        assert_eq!(write(&pp, Dir::Rd, b"x", false), (Err(Errno::EPIPE), 0));

        drop(pp);
        r.close();
    }

    #[test]
    fn epipe_after_the_reader_closes() {
        let (r, w, pp) = pair();

        r.close();
        assert!(pp.end(Dir::Wr).has(PIPE_EOF));
        assert_eq!(write(&pp, Dir::Wr, b"lost", false), (Err(Errno::EPIPE), 0));
        // The reverse direction sees EOF.
        assert_eq!(read(&pp, Dir::Wr, 4, false), (Ok(()), Vec::new()));

        drop(pp);
        w.close();
    }

    #[test]
    fn ioctl_and_stat() {
        let (r, w, pp) = pair();

        assert_eq!(write(&pp, Dir::Wr, b"abc", false), (Ok(()), 3));
        assert_eq!(pp.ioctl(Dir::Rd, PipeIoctl::Nread), Ok(Some(3)));

        assert_eq!(pp.ioctl(Dir::Rd, PipeIoctl::Async(true)), Ok(None));
        assert!(pp.end(Dir::Rd).has(PIPE_ASYNC));
        // An async pipe's reader gets SIGIO when data arrives.
        assert_eq!(write(&pp, Dir::Wr, b"d", false), (Ok(()), 1));
        assert_eq!(pp.glue(Dir::Rd).sigios.load(Ordering::Relaxed), 1);
        assert_eq!(pp.ioctl(Dir::Rd, PipeIoctl::Async(false)), Ok(None));
        assert!(!pp.end(Dir::Rd).has(PIPE_ASYNC));

        let set = PipeIoctl::SetOwn { cmd: 1, arg: -42 };
        assert_eq!(pp.ioctl(Dir::Rd, set), Ok(None));
        assert_eq!(
            pp.ioctl(Dir::Rd, PipeIoctl::GetOwn { cmd: 2 }),
            Ok(Some(-42))
        );
        // sys_ioctl handles FIONBIO itself; the pipe does not.
        assert_eq!(pp.ioctl(Dir::Rd, PipeIoctl::Other), Err(Errno::ENOTTY));

        let st = pp.stat(Dir::Rd);
        assert_eq!((st.size, st.blksize, st.blocks), (4, PIPE_SIZE as i32, 1));
        assert!(st.mtime.tv_sec >= st.ctime.tv_sec);
        let empty = pp.stat(Dir::Wr);
        assert_eq!((empty.size, empty.blocks), (0, 0));

        drop(pp);
        r.close();
        w.close();
    }

    #[test]
    fn rundown_and_filters() {
        let (r, w, pp) = pair();

        assert_eq!(write(&pp, Dir::Wr, b"abc", false), (Ok(()), 3));

        pp.with_lock(|| {
            // Readable with 3 bytes; the write side has room for PIPE_BUF bytes.
            let rd = pp.filter(Dir::Rd, PipeFilter::Read, false);
            assert_eq!((rd.active, rd.data, rd.eof), (true, Some(3), false));
            let wr = pp.filter(Dir::Wr, PipeFilter::Write, false);
            assert_eq!((wr.active, wr.data), (true, Some(PIPE_SIZE as i64 - 3)));
            let ex = pp.filter(Dir::Rd, PipeFilter::Except, true);
            assert_eq!((ex.active, ex.hup), (false, false));
            // The empty direction is not readable.
            assert!(!pp.filter(Dir::Wr, PipeFilter::Read, false).active);
        });

        {
            let mut g = pp.lock.write();
            let wp = pp.end(Dir::Wr);
            g.ends[1].busy = 1;
            wp.set(&mut g, PIPE_WANTD | PIPE_WANTR);
            assert!(!pp.rundown(&mut g, Dir::Wr));
            g.ends[1].busy = 0;
            assert!(pp.rundown(&mut g, Dir::Wr));
            assert!(!wp.has(PIPE_WANTD | PIPE_WANTR));
        }

        r.close();
        pp.with_lock(|| {
            // The reader is gone: writable "forever", with EOF and, for poll, a hang-up.
            let wr = pp.filter(Dir::Wr, PipeFilter::Write, true);
            assert_eq!(
                wr,
                FilterAnswer {
                    active: true,
                    data: Some(0),
                    eof: true,
                    hup: true
                }
            );
            let ex = pp.filter(Dir::Wr, PipeFilter::Except, true);
            assert!(ex.active && ex.hup);
            assert!(!pp.filter(Dir::Wr, PipeFilter::Except, false).active);
            let rd = pp.filter(Dir::Wr, PipeFilter::Read, false);
            assert_eq!((rd.active, rd.eof, rd.hup), (true, true, false));
        });

        drop(pp);
        w.close();
    }

    #[test]
    fn kqfilter_hooks_each_filter_where_the_c_does() {
        let (r, w, pp) = pair();

        // EVFILT_READ on rf: rf's own pipe.
        assert_eq!(
            pp.kqfilter(Dir::Rd, Some(PipeFilter::Read), false, false, &TestKnote(1)),
            Ok(())
        );
        // EVFILT_WRITE on wf: the pipe wf writes into, its peer.
        assert_eq!(
            pp.kqfilter(
                Dir::Wr,
                Some(PipeFilter::Write),
                false,
                false,
                &TestKnote(2)
            ),
            Ok(())
        );
        assert_eq!(
            pp.glue(Dir::Rd).klist(),
            vec![
                (1, PipeFilter::Read, Dir::Rd),
                (2, PipeFilter::Write, Dir::Rd)
            ]
        );
        // EVFILT_EXCEPT: poll's only.
        let except = Some(PipeFilter::Except);
        assert_eq!(
            pp.kqfilter(Dir::Rd, except, true, true, &TestKnote(3)),
            Err(Errno::EPERM)
        );
        assert_eq!(
            pp.kqfilter(Dir::Rd, except, false, false, &TestKnote(3)),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            pp.kqfilter(Dir::Rd, except, false, true, &TestKnote(3)),
            Ok(())
        );
        assert_eq!(
            pp.kqfilter(Dir::Rd, None, false, false, &TestKnote(4)),
            Err(Errno::EINVAL)
        );
        assert_eq!(pp.glue(Dir::Rd).klist().len(), 3);

        // A write wakes the reader's knotes (pipe_wakeup's knote_locked).
        let before = pp.glue(Dir::Rd).knotes.load(Ordering::Relaxed);
        assert_eq!(write(&pp, Dir::Wr, b"x", false), (Ok(()), 1));
        assert!(pp.glue(Dir::Rd).knotes.load(Ordering::Relaxed) > before);

        for kn in [1, 2, 3] {
            pp.detach(Dir::Rd, &TestKnote(kn));
        }
        assert!(pp.glue(Dir::Rd).klist().is_empty());

        // The reader gone, EVFILT_WRITE hooks on wf's own pipe.
        r.close();
        assert_eq!(
            pp.kqfilter(
                Dir::Wr,
                Some(PipeFilter::Write),
                false,
                false,
                &TestKnote(5)
            ),
            Ok(())
        );
        assert_eq!(
            pp.glue(Dir::Wr).klist(),
            vec![(5, PipeFilter::Write, Dir::Wr)]
        );
        pp.detach(Dir::Wr, &TestKnote(5));

        drop(pp);
        w.close();
    }

    #[test]
    fn kinfo_shows_one_address_for_both_ends() {
        let (r, w, pp) = pair();

        let rk = pp.kinfo(Dir::Rd);
        let wk = pp.kinfo(Dir::Wr);
        assert_ne!(rk.addr, 0);
        assert_eq!((rk.peer, wk.peer), (wk.addr, rk.addr));
        // fstat(1) prints MAXIMUM(f_data, pipe_peer): the same for both ends.
        assert_eq!(rk.addr.max(rk.peer), wk.addr.max(wk.peer));

        assert_eq!(pp.ioctl(Dir::Rd, PipeIoctl::Async(true)), Ok(None));
        assert_eq!(pp.kinfo(Dir::Rd).state, PIPE_ASYNC);

        w.close();
        let rk = pp.kinfo(Dir::Rd);
        assert_eq!(rk.peer, 0, "pipe_peer is 0 once the peer is destroyed");
        assert_eq!(rk.state, PIPE_ASYNC | PIPE_EOF, "the word whole");

        drop(pp);
        r.close();
    }

    #[test]
    fn a_big_write_grows_an_empty_buffer() {
        static P: Pipes<TestGlue> = Pipes::new();
        let (r, w) = setup(&P).create().unwrap();
        let pp = r.pair().clone();

        let big = pattern(0, PIPE_SIZE + 1);
        assert_eq!(write(&pp, Dir::Wr, &big, true), (Ok(()), PIPE_SIZE + 1));
        assert_eq!(pp.end(Dir::Rd).size(), BIG_PIPE_SIZE);
        assert_eq!(P.nbigpipe(), 1);
        assert_eq!(P.amountpipekva(), (BIG_PIPE_SIZE + PIPE_SIZE) as u32);
        assert_eq!(read(&pp, Dir::Rd, BIG_PIPE_SIZE, false).1, big);

        drop(pp);
        r.close();
        assert_eq!(P.nbigpipe(), 0, "freeing a big buffer gives its slot back");
        w.close();
        assert_eq!(P.amountpipekva(), 0);
    }

    #[test]
    fn a_fault_stops_the_copy_and_keeps_what_moved() {
        let (r, w, pp) = pair();

        // The source faults after 3 bytes: the segment is not counted.
        let mut uio = Buf {
            data: b"abcdef".to_vec(),
            pos: 0,
            source: true,
            fault_at: 3,
        };
        assert_eq!(pp.write(Dir::Wr, &mut uio, false), Err(Errno::EFAULT));
        assert_eq!(pp.end(Dir::Rd).cnt(), 0);
        assert!(!pp.end(Dir::Rd).has(PIPE_LOCK));

        assert_eq!(write(&pp, Dir::Wr, b"abcdef", false), (Ok(()), 6));
        let mut uio = Buf {
            data: vec![0; 6],
            pos: 0,
            source: false,
            fault_at: 2,
        };
        assert_eq!(pp.read(Dir::Rd, &mut uio, false), Err(Errno::EFAULT));
        assert_eq!(pp.end(Dir::Rd).cnt(), 6, "nothing consumed");
        assert_eq!(
            pp.lock.read().ends[0].buffer.mem.as_ref().map(|m| m.len()),
            Some(PIPE_SIZE)
        );

        drop(pp);
        r.close();
        w.close();
    }

    #[test]
    fn a_blocked_reader_is_woken_by_a_write_and_by_the_writers_close() {
        let (r, w, pp) = pair();

        let reader = {
            let pp = pp.clone();
            thread::spawn(move || read(&pp, Dir::Rd, 16, false))
        };
        // Wait until the reader sleeps for data (PIPE_WANTR), then write.
        while !pp.end(Dir::Rd).has(PIPE_WANTR) {
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(write(&pp, Dir::Wr, b"wake", false), (Ok(()), 4));
        assert_eq!(reader.join().unwrap(), (Ok(()), b"wake".to_vec()));

        let reader = {
            let pp = pp.clone();
            thread::spawn(move || read(&pp, Dir::Rd, 16, false))
        };
        while !pp.end(Dir::Rd).has(PIPE_WANTR) {
            thread::sleep(Duration::from_millis(1));
        }
        w.close();
        assert_eq!(reader.join().unwrap(), (Ok(()), Vec::new()), "EOF");

        drop(pp);
        r.close();
    }

    #[test]
    fn a_blocked_writer_is_woken_by_a_read() {
        let (r, w, pp) = pair();

        assert_eq!(
            write(&pp, Dir::Wr, &pattern(0, PIPE_SIZE), false).1,
            PIPE_SIZE
        );
        let writer = {
            let pp = pp.clone();
            thread::spawn(move || write(&pp, Dir::Wr, &pattern(PIPE_SIZE, 100), false))
        };
        while !pp.end(Dir::Rd).has(PIPE_WANTW) {
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            read(&pp, Dir::Rd, PIPE_SIZE, false).1,
            pattern(0, PIPE_SIZE)
        );
        assert_eq!(writer.join().unwrap(), (Ok(()), 100));
        assert_eq!(read(&pp, Dir::Rd, 200, false).1, pattern(PIPE_SIZE, 100));

        drop(pp);
        r.close();
        w.close();
    }

    /// `dopipe`'s descriptor work in the tests: keeps the ends, copies out 3 and 4.
    struct TestFiles {
        ends: Mutex<Vec<(PipeEnd<TestGlue>, PipeFlags)>>,
    }

    impl PipeFiles<TestGlue> for TestFiles {
        fn dopipe(
            &self,
            rpipe: PipeEnd<TestGlue>,
            wpipe: PipeEnd<TestGlue>,
            flags: PipeFlags,
            copyout: &dyn Fn([i32; 2]) -> Result<(), Errno>,
        ) -> Result<(), Errno> {
            // On a fault the C releases both descriptors, which closes both pipes.
            copyout([3, 4])?;
            let mut ends = self.ends.lock().unwrap();
            ends.push((rpipe, flags));
            ends.push((wpipe, flags));
            Ok(())
        }
    }

    #[test]
    fn pipe2_checks_its_flags_and_copies_out_both_descriptors() {
        static P: Pipes<TestGlue> = Pipes::new();
        let pipes = setup(&P);
        let files = TestFiles {
            ends: Mutex::new(Vec::new()),
        };
        let fdp = UserPtr::new(testutil::USER_BASE + 64);

        assert_eq!(
            pipes.sys_pipe2(fdp, 0x8000_0000_u32 as i32, &files),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            pipes.sys_pipe2(fdp, O_CLOEXEC | 0x1, &files),
            Err(Errno::EINVAL)
        );
        assert_eq!(pipes.nout(), 0, "no pair for bad flags");

        assert_eq!(pipes.sys_pipe2(fdp, O_CLOEXEC | FNONBLOCK, &files), Ok(()));
        let mut expect = 3i32.to_ne_bytes().to_vec();
        expect.extend_from_slice(&4i32.to_ne_bytes());
        assert_eq!(
            testutil::user_bytes(testutil::USER_BASE + 64, 8),
            Some(expect)
        );
        {
            let ends = files.ends.lock().unwrap();
            assert_eq!(ends.len(), 2);
            assert_eq!((ends[0].0.dir(), ends[1].0.dir()), (Dir::Rd, Dir::Wr));
            let flags = PipeFlags {
                cloexec: true,
                clofork: false,
                nonblock: true,
            };
            assert_eq!((ends[0].1, ends[1].1), (flags, flags));
        }
        assert_eq!(pipes.sys_pipe(fdp, &files), Ok(()));
        assert_eq!(files.ends.lock().unwrap()[2].1, PipeFlags::default());
        assert_eq!(pipes.nout(), 2);

        // A fault on the copy: the pair goes back with its ends.
        let bad = UserPtr::new(testutil::USER_BASE + testutil::USER_LEN - 4);
        assert_eq!(pipes.sys_pipe2(bad, O_CLOFORK, &files), Err(Errno::EFAULT));
        assert_eq!(pipes.nout(), 2);

        files.ends.lock().unwrap().clear();
        assert_eq!(pipes.nout(), 0);
    }

    #[test]
    fn dir_names_the_peer() {
        assert_eq!(Dir::Rd.other(), Dir::Wr);
        assert_eq!(Dir::Wr.other(), Dir::Rd);
        for d in [Dir::Rd, Dir::Wr] {
            assert_eq!(Dir::from_index(d.index()), Some(d));
        }
        assert_eq!(Dir::from_index(2), None);
    }

    /// The `#define NAME value` lines of a header under `$OPENBSD_SRC`, the values as numbers
    /// (decimal, hexadecimal, a product in parentheses, or another name of the same file).
    fn defines(path: &str) -> std::collections::BTreeMap<std::string::String, i64> {
        use std::string::{String, ToString};
        let root = std::env::var("OPENBSD_SRC").expect("OPENBSD_SRC (just test-ref)");
        let text = std::fs::read_to_string(std::format!("{root}/{path}")).unwrap();
        let mut raw: Vec<(String, String)> = Vec::new();
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("#define") else {
                continue;
            };
            let mut words = rest.split_whitespace();
            let (Some(name), Some(value)) = (words.next(), words.next()) else {
                continue;
            };
            raw.push((name.to_string(), value.to_string()));
        }
        let number = |v: &str| -> Option<i64> {
            let v = v.trim_matches(|c| c == '(' || c == ')');
            let mut product = 1i64;
            for f in v.split('*') {
                let f = f.trim();
                product *= match f.strip_prefix("0x") {
                    Some(h) => i64::from_str_radix(h, 16).ok()?,
                    None => f.parse().ok()?,
                };
            }
            Some(product)
        };
        let mut out = std::collections::BTreeMap::new();
        for (name, value) in &raw {
            if let Some(n) = number(value) {
                out.insert(name.clone(), n);
            }
        }
        for (name, value) in &raw {
            if let Some(&n) = out.get(value) {
                out.entry(name.clone()).or_insert(n);
            }
        }
        out
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_headers() {
        let pipe = defines("sys/sys/pipe.h");
        for (name, ours) in [
            ("PIPE_SIZE", PIPE_SIZE as i64),
            ("BIG_PIPE_SIZE", BIG_PIPE_SIZE as i64),
            ("PIPE_ASYNC", i64::from(PIPE_ASYNC)),
            ("PIPE_WANTR", i64::from(PIPE_WANTR)),
            ("PIPE_WANTW", i64::from(PIPE_WANTW)),
            ("PIPE_WANTD", i64::from(PIPE_WANTD)),
            ("PIPE_EOF", i64::from(PIPE_EOF)),
            ("PIPE_LOCK", i64::from(PIPE_LOCK)),
            ("PIPE_LWANT", i64::from(PIPE_LWANT)),
        ] {
            assert_eq!(pipe.get(name), Some(&ours), "{name}");
        }
        // Every PIPE_* of the header is here.
        assert_eq!(pipe.keys().filter(|k| k.starts_with("PIPE_")).count(), 8);
        assert_eq!(
            defines("sys/sys/syslimits.h").get("PIPE_BUF"),
            Some(&(PIPE_BUF as i64))
        );
        assert_eq!(
            defines("sys/sys/param.h").get("PRIBIO"),
            Some(&i64::from(PRIBIO))
        );
        let fcntl = defines("sys/sys/fcntl.h");
        assert_eq!(fcntl.get("O_CLOEXEC"), Some(&i64::from(O_CLOEXEC)));
        assert_eq!(fcntl.get("O_CLOFORK"), Some(&i64::from(O_CLOFORK)));
        assert_eq!(fcntl.get("FNONBLOCK"), Some(&i64::from(FNONBLOCK)));
    }
}
/* </TESTS> */
