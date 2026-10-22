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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)file.h	8.2 (Berkeley) 8/20/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/file.h>`: the open file, `struct file`, its operations vector, the `DTYPE_*` types,
//! the `FIF_*` internal flags and the reference macros `FREF`/`FRELE`.
//!
//! Upstream: sys/sys/file.h @ 3ce1f3f79392
//! LZ: sys/sys/file.rs@3c62ede68802
//!
//! A [`File`] is an item of the pool `filepl`, held by counted handles ([`FileRef`]): every
//! descriptor slot holding it, every thread using it (`fd_getfile`'s reference), every knote
//! on it and every message carrying it in flight holds one, and the handle count is the C's
//! `f_count`. The last [`FileRef::frele`] calls the file's close (`fdrop`) and frees the item.
//! What the file is (`f_ops`, `f_data`, `f_type`) is installed once, between `fnew` and the
//! file's first `fdinsert` ([`File::install`]); a file that was never installed (an open that
//! failed) is freed without a close, as the C's `if (fp->f_ops)`.
//!
//! The kernel holds the file; what it points at (a vnode, a socket, a pipe, a kqueue) and the
//! operations on it are `bsd`'s until their subsystems move, reached through [`FileGlue`] (a
//! bridge of `bridges.toml`): one instantiation, `bsd`'s (`sys/kern/descrip_glue.rs`).
//!
//! ## Deviations
//! - `f_data` is the glue's value ([`FileGlue::Data`], `bsd`'s enum of the four objects) and
//!   `f_ops` a reference to the glue's operations ([`FileGlue::Ops`]), both written once
//!   ([`File::install`]); the C writes them as plain members. `kinfo_file` prints the
//!   object's address ([`FileGlue::data_addr`]) and the operations' address as before.
//! - `f_count` is the handle count ([`FileRef::count`]): the slots, the holders, the knotes
//!   and the messages in flight (parked references, [`FileRef::forget`]).
//! - `f_mtx` holds the members it guards ([`FileStats`], `f_offset` and the counters).
//!
//! ## Redesign
//! - LZ's `struct file` was a raw `file_pool` item handed out as `&'static File`, its
//!   `f_count` an `AtomicU32` that `FREF`/`FRELE` moved by hand, `fdrop` giving the item back
//!   with `pool_put` (a use after free one forgotten `FREF` away); `f_data` a
//!   `Cell<*mut c_void>` cast back by each object's accessor (`File::vnode`, `fp_socket`,
//!   `fp_kqueue`), and a pipe in a typed slot of its own beside it. Now the file is a
//!   `TypedPool` item behind [`FileRef`], a `PoolArc`: `FREF` is a clone, `FRELE` is
//!   [`FileRef::frele`] (`PoolArc::try_last`: the last one gets the item at count 0 and
//!   closes it, returning the close's error, which a `Drop` could not), and a handle dropped
//!   without one (`(void)FRELE(fp, NULL)`) closes it the same way with no thread. The item
//!   cannot be freed while a handle exists, and a handle cannot outlive the item.
//! - `f_data` is installed once with its type ([`Installed`]), so it is read by reference
//!   (no cast; the pipe needs no slot of its own) and a close sees the object by reference
//!   (`fo_close(fp, p)`), which drops with the item after `numfiles` falls.
//! - `filehead` is a `hal::pinlist::PinList` under `fhdlk` ([`FileTables`]), the file's
//!   `f_list` a `PinEntry`: a file is unlinked by its close before its memory goes, checked.
//! - The descriptor in flight in a unix-domain message (a raw `struct file *` in an mbuf) is a
//!   parked reference ([`FileRef::forget`]/[`FileRef::adopt`]), counted by `f_count` as the
//!   C counts it, adopted once.

use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::pinlist::{PinAdapter, PinEntry, PinList};
use hal::pool::{Last, PoolArc, PoolHome, TypedPool};
use hal::sys::errno::Errno;
use hal::sys::fcntl::Flock;
use hal::sys::types::{Off, Register};
use hal::user::AbiPod;

use crate::assert::kpanic;
use crate::cred::CredRef;
use crate::descrip::Filedesc;
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;

/// `DTYPE_VNODE`: file.
pub const DTYPE_VNODE: i32 = 1;
/// `DTYPE_SOCKET`: communications endpoint.
pub const DTYPE_SOCKET: i32 = 2;
/// `DTYPE_PIPE`: pipe.
pub const DTYPE_PIPE: i32 = 3;
/// `DTYPE_KQUEUE`: event queue.
pub const DTYPE_KQUEUE: i32 = 4;
/// `DTYPE_DMABUF`: DMA buffer (for DRM).
pub const DTYPE_DMABUF: i32 = 5;
/// `DTYPE_SYNC`: sync file (for DRM).
pub const DTYPE_SYNC: i32 = 6;
/// `DTYPE_VMM`: `vmm(4)` virtual machine.
pub const DTYPE_VMM: i32 = 7;

/// `FO_POSITION`: positioned read/write.
pub const FO_POSITION: i32 = 0x0000_0001;

/// `FIF_HASLOCK`: descriptor holds advisory lock.
pub const FIF_HASLOCK: u32 = 0x01;
/// `FIF_INSERTED`: present in `filehead`.
pub const FIF_INSERTED: u32 = 0x80;

/// `FDUP_MAX_COUNT`: the reference count past which a file is no longer duplicated (below
/// `hal`'s handle limit, so a duplication is refused before a clone could panic).
pub const FDUP_MAX_COUNT: u32 = u32::MAX - 2 * hal::cpu::MAXCPUS as u32;

/// Whether `vfs_stall(9)` is stalling the file systems (`vfs_stalling`): read by every
/// `FREF`, set by `bsd`'s `vfs_stall`; while it is, `FREF` waits ([`FileGlue::stall_wait`]).
pub static VFS_STALLING: AtomicBool = AtomicBool::new(false);

/// `vfs_stall_barrier()`: one relaxed load, and a wait while the file systems are stalled.
pub fn vfs_stall_barrier<G: FileGlue>() {
    if VFS_STALLING.load(Ordering::Relaxed) {
        G::stall_wait();
    }
}

/// The owner of a record lock (`VOP_ADVLOCK`'s `id`): a descriptor table for the POSIX locks
/// of `fcntl(2)` and `closef`, a file for `flock(2)`'s; by address, as lockf compares them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockOwner {
    /// A descriptor table (`struct filedesc *`).
    Table(usize),
    /// A file (`struct file *`).
    File(usize),
}

/// What a file is, written once by [`File::install`]: its operations and its object.
pub struct Installed<G: FileGlue> {
    /// `f_ops`.
    pub ops: &'static G::Ops,
    /// `f_data`.
    pub data: G::Data,
}

/// \[f\] The members `f_mtx` guards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FileStats {
    /// \[f,v\] `f_offset`.
    pub f_offset: Off,
    /// `f_rxfer`: total number of read transfers.
    pub f_rxfer: u64,
    /// `f_wxfer`: total number of write transfers.
    pub f_wxfer: u64,
    /// `f_seek`: total independent seek operations.
    pub f_seek: u64,
    /// `f_rbytes`: total bytes read.
    pub f_rbytes: u64,
    /// `f_wbytes`: total bytes written.
    pub f_wbytes: u64,
}

/// `struct file`: an open file. Locks: \[I\] immutable after creation, \[F\] `fhdlk`, \[a\]
/// atomic operations, \[f\] `f_mtx` ([`File::stats`]), \[v\] vnode lock.
pub struct File<G: FileGlue> {
    /// \[F\] `f_list`: list of active files (`filehead`).
    pub f_list: PinEntry<File<G>>,
    home: PoolHome<File<G>>,
    /// \[a\] `f_flag`: see `fcntl.h` (`FREAD`, `FWRITE`, `FNONBLOCK`, ...).
    pub f_flag: AtomicU32,
    /// \[a\] `f_iflags`: internal flags (`FIF_*`).
    pub f_iflags: AtomicU32,
    /// \[I\] `f_type`: descriptor type (`DTYPE_*`), 0 until installed.
    f_type: AtomicI32,
    /// \[I\] `f_cred`: the opener's credentials (`fnew`'s `crhold`), dropped with the item.
    cred: CredRef,
    /// \[I\] `f_ops`, `f_data`: set once between `fnew` and the first `fdinsert`.
    installed: OnceStatic<Installed<G>>,
    /// Whether `fdrop`'s close ran (the last handle's, or its drop's).
    closed: AtomicBool,
    /// `f_mtx` and what it guards.
    stats: Mutexed<FileStats>,
}

hal::pool_home!(impl[G: FileGlue] File<G>, home);

impl<G: FileGlue> File<G> {
    /// `fnew`'s new file: no type yet, the given credentials.
    fn new(cred: CredRef) -> Self {
        Self {
            f_list: PinEntry::new(),
            home: PoolHome::new(),
            f_flag: AtomicU32::new(0),
            f_iflags: AtomicU32::new(0),
            f_type: AtomicI32::new(0),
            cred,
            installed: OnceStatic::new(),
            closed: AtomicBool::new(false),
            // We need to block interrupts as long as `f_mtx` is being taken with and without
            // the kernel lock.
            stats: Mutexed::new(hal::intr::mpfloor(), FileStats::default()),
        }
    }

    /// `fp->f_flag` as the `int` the `fcntl.h` constants are.
    pub fn flag(&self) -> i32 {
        self.f_flag.load(Ordering::Relaxed) as i32
    }

    /// `fp->f_type` (0 before [`File::install`]).
    pub fn f_type(&self) -> i32 {
        self.f_type.load(Ordering::Relaxed)
    }

    /// `fp->f_cred`.
    pub fn cred(&self) -> &CredRef {
        &self.cred
    }

    /// Installs what the file is: `f_type`, `f_ops` and `f_data`, once, before the file is
    /// inserted in a table. A second installation is `panic(9)`.
    pub fn install(&self, ty: i32, ops: &'static G::Ops, data: G::Data) {
        if self.installed.set(Installed { ops, data }).is_err() {
            kpanic(format_args!("file {:#x}: installed twice", self.addr()));
        }
        self.f_type.store(ty, Ordering::Release);
    }

    /// What [`File::install`] installed, `None` before.
    pub fn installed(&self) -> Option<&Installed<G>> {
        self.installed.get()
    }

    /// `fp->f_ops`, which every installed file has.
    pub fn ops(&self) -> &'static G::Ops {
        match self.installed.get() {
            Some(i) => i.ops,
            None => kpanic(format_args!("file {:#x}: no f_ops", self.addr())),
        }
    }

    /// `fp->f_data`, `None` before [`File::install`].
    pub fn data(&self) -> Option<&G::Data> {
        self.installed.get().map(|i| &i.data)
    }

    /// `f_mtx` held, and the members it guards.
    pub fn stats(&self) -> MutexedGuard<'_, FileStats> {
        self.stats.lock()
    }

    /// The file's address (`kinfo_file.f_fileaddr`, `flock(2)`'s lock owner).
    pub fn addr(&self) -> usize {
        core::ptr::from_ref(self).addr()
    }

    /// `fdrop`'s work, once: unlink from `filehead`, `fo_close` of an installed file,
    /// `numfiles--`. The item itself goes back with the last handle, credentials and object
    /// with it (`crfree`, the object's drop).
    fn close_once(&self, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
        if self.closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let t = G::tables();
        if self.f_iflags.load(Ordering::Acquire) & FIF_INSERTED != 0 {
            let mut g = t.fhdlk.lock();
            t.filehead.remove(&mut g, self);
        }
        let error = match self.installed.get() {
            Some(i) => i.ops.close(self, p),
            None => Ok(()),
        };
        t.numfiles.fetch_sub(1, Ordering::AcqRel);
        error
    }
}

impl<G: FileGlue> Drop for File<G> {
    /// A file whose last handle was dropped without [`FileRef::frele`] (the C's `(void)FRELE`
    /// of a file with no thread: `unp_discard`, a knote's): its close, with no thread.
    fn drop(&mut self) {
        let _ = self.close_once(None);
    }
}

/// A counted reference to an open file (`struct file *` with a reference: `f_count`).
pub struct FileRef<G: FileGlue>(PoolArc<File<G>>);

impl<G: FileGlue> FileRef<G> {
    /// `FREF(fp)`: one more reference, after `vfs_stall_barrier()`.
    pub fn fref(&self) -> Self {
        vfs_stall_barrier::<G>();
        self.clone()
    }

    /// `FRELE(fp, p)`: gives the reference back; the last one closes the file (`fdrop`) and
    /// returns the close's error.
    pub fn frele(self, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
        match PoolArc::try_last(self.0) {
            Some(last) => fdrop(last, p),
            None => Ok(()),
        }
    }

    /// `fp->f_count`: the references alive now (slots, holders, knotes, messages in flight).
    pub fn count(&self) -> u32 {
        PoolArc::count(&self.0) as u32
    }

    /// The file's address (`kinfo_file.f_fileaddr`).
    pub fn addr(&self) -> usize {
        self.0.addr()
    }

    /// Whether two references name the same file.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        PoolArc::ptr_eq(&a.0, &b.0)
    }

    /// The file, pinned (for `filehead`).
    pub fn as_pin(&self) -> Pin<&File<G>> {
        self.0.as_pin()
    }

    /// Gives the reference up as a word in legacy memory (a descriptor in flight in a
    /// message, `unp_internalize`'s `FREF`): still counted, taken back by one
    /// [`FileRef::adopt`].
    pub fn forget(self) {
        PoolArc::forget(self.0);
    }

    /// The reference a [`FileRef::forget`] parked on `fp` (a message received or discarded).
    /// `None` when none is parked: a word adopted twice.
    pub fn adopt(fp: &File<G>) -> Option<Self> {
        PoolArc::adopt(fp).map(Self)
    }

    /// One more reference to a file a list lends (`fd_iterfile`'s CAS from a non-zero
    /// count): `None` for a file whose last reference is gone, its close under way.
    pub fn try_from_ref(fp: &File<G>) -> Option<Self> {
        PoolArc::try_from_ref(fp).map(Self)
    }
}

impl<G: FileGlue> Clone for FileRef<G> {
    /// One more reference without `vfs_stall_barrier()` (a knote's slot read, a table's copy
    /// of a reference the caller holds).
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<G: FileGlue> core::ops::Deref for FileRef<G> {
    type Target = File<G>;

    fn deref(&self) -> &File<G> {
        &self.0
    }
}

/// The open files of one glue (the C's globals of `kern_descrip.c`): `filehead` under
/// `fhdlk`, the pools and `numfiles`.
pub struct FileTables<G: FileGlue> {
    /// `fhdlk`: protects `filehead`. Interrupts are blocked as long as it is held, with and
    /// without the kernel lock (`IPL_MPFLOOR`).
    pub fhdlk: Mutexed<()>,
    /// `filehead`: head of list of open files.
    pub filehead: PinList<G::FileList>,
    /// `file_pool`, `filepl` (recorded: `fd_iterfile` and the messages count a lent file
    /// again).
    pub filepl: TypedPool<File<G>>,
    /// `fdesc_pool`, `fdescpl`.
    pub fdescpl: TypedPool<Filedesc<G>>,
    /// `numfiles`: actual number of open files.
    pub numfiles: AtomicI32,
}

impl<G: FileGlue> FileTables<G> {
    /// Empty tables whose `fhdlk` is at `ipl` (`IPL_MPFLOOR`); `filedesc_init` makes the
    /// pools.
    pub const fn new(ipl: i32) -> Self {
        Self {
            fhdlk: Mutexed::new(ipl, ()),
            filehead: PinList::new(),
            filepl: TypedPool::new(),
            fdescpl: TypedPool::new(),
            numfiles: AtomicI32::new(0),
        }
    }
}

/// `struct fileops`: the operations on a file of one kind. `read`, `write` and `close` may be
/// called without the kernel lock held.
pub trait FileOps<G: FileGlue>: Sync {
    /// `fo_read(fp, uio, fflags)`.
    fn read(&self, fp: &File<G>, uio: &mut Uio<'_, G::Proc>, fflags: i32) -> Result<(), Errno>;
    /// `fo_write(fp, uio, fflags)`.
    fn write(&self, fp: &File<G>, uio: &mut Uio<'_, G::Proc>, fflags: i32) -> Result<(), Errno>;
    /// `fo_ioctl(fp, com, data, p)`: `data` is the kernel copy of the argument.
    fn ioctl(
        &self,
        fp: &File<G>,
        com: u64,
        data: &mut [u8],
        p: &Thread<G::Proc>,
    ) -> Result<(), Errno>;
    /// `fo_kqfilter(fp, kn)`: attaches the knote to the file's object.
    fn kqfilter(&self, fp: &File<G>, kn: &G::Knote) -> Result<(), Errno>;
    /// `fo_stat(fp, ub, p)`.
    fn stat(&self, fp: &File<G>, st: &mut G::Stat, p: &Thread<G::Proc>) -> Result<(), Errno>;
    /// `fo_close(fp, p)`: `p` is `None` for a file closed with no thread (one that was in a
    /// message). It may sleep: no spin lock is held.
    fn close(&self, fp: &File<G>, p: Option<&Thread<G::Proc>>) -> Result<(), Errno>;
    /// `fo_seek(fp, offset, whence, p)`; the C's NULL is `ESPIPE`.
    fn seek(
        &self,
        _fp: &File<G>,
        _off: &mut Off,
        _whence: i32,
        _p: &Thread<G::Proc>,
    ) -> Result<(), Errno> {
        Err(Errno::ESPIPE)
    }
}

/// What the descriptors need of the subsystems still in `bsd` (decision 9): the object a
/// file holds and the operations on it, the vnodes of `fd_cdir`/`fd_rdir`, the kqueue members
/// of a table, the tables themselves, and the calls `kern_descrip.c` and `sys_generic.c` make
/// into the vnode layer, the sockets, kqueue, signals and pledge. One instantiation, `bsd`'s
/// (`sys/kern/descrip_glue.rs`); a bridge of `bridges.toml` while `bsd` implements it.
pub trait FileGlue: Send + Sync + Sized + 'static {
    /// The threads whose tables these are.
    type Proc: ProcGlue<FileGlue = Self>;
    /// `struct fileops`: the operations a file is installed with.
    type Ops: ?Sized + FileOps<Self> + 'static;
    /// `f_data`: the object of a file (a vnode, a socket, a pipe, a kqueue).
    type Data: Send + Sync + 'static;
    /// `struct knote`, for [`FileOps::kqfilter`].
    type Knote: ?Sized;
    /// `struct stat`, as `fstat(2)` copies it out.
    type Stat: AbiPod + Default;
    /// `struct vnode` (`fd_cdir`, `fd_rdir`).
    type Vnode: Sync + 'static;
    /// `f_list`: `filehead`'s adapter ([`file_glue_links!`](crate::file_glue_links)).
    type FileList: PinAdapter<Elem = File<Self>>;
    /// `fd_kqlist`, `fd_nuserevents`: a table's kqueue members (kqueue's, N4.4).
    type FdKq: Send + Sync + Default;

    /// The open files, `fhdlk` and the pools (one `static` of `bsd`'s).
    fn tables() -> &'static FileTables<Self>;
    /// `maxfiles` (`kern.maxfiles`).
    fn maxfiles() -> i32;
    /// `tablefull(tab)`: the "table is full" message.
    fn tablefull(tab: &str);
    /// `vfs_stall_barrier()`'s wait while [`VFS_STALLING`] is set (`rw_enter_read` and
    /// `rw_exit_read` of `vfs_stall_lock`).
    fn stall_wait();
    /// The object's address, as `kinfo_file.f_data` prints it.
    fn data_addr(d: &Self::Data) -> usize;
    /// `vref(vp)`.
    fn vref(v: &'static Self::Vnode);
    /// `vrele(vp)`.
    fn vrele(v: &'static Self::Vnode);
    /// `VOP_ADVLOCK(vp, id, op, fl, flags)` of a vnode file's vnode.
    fn advlock(
        d: &Self::Data,
        owner: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno>;
    /// `vn_lock`, `VOP_PATHCONF(vp, name, &retval)`, `VOP_UNLOCK` of a vnode file's vnode.
    fn pathconf(d: &Self::Data, name: i32) -> Result<Register, Errno>;
    /// `vp->v_flag & VISTTY` of a vnode file's vnode.
    fn isatty(d: &Self::Data) -> bool;
    /// The positioned-I/O checks of a vnode file (`ESPIPE` for a fifo or a tty, `EINVAL` for
    /// a negative offset but on a character device).
    fn position_check(d: &Self::Data, off: Off) -> Result<(), Errno>;
    /// Whether a socket file's socket is a DNS one (`SS_DNS`: `ioctl(2)` refuses it).
    fn dns_socket(d: &Self::Data) -> bool;
    /// `knote_fdclose(p, fd)`.
    fn knote_fdclose(p: &Thread<Self::Proc>, fd: i32);
    /// `ptsignal(p, SIGPIPE, STHREAD)`.
    fn sigpipe(p: &Thread<Self::Proc>);
    /// `pledge_fcntl(p, cmd)`.
    fn pledge_fcntl(p: &Thread<Self::Proc>, cmd: i32) -> Result<(), Errno>;
    /// `pledge_flock(p)`.
    fn pledge_flock(p: &Thread<Self::Proc>) -> Result<(), Errno>;
    /// `pledge_ioctl(p, com, fp)`.
    fn pledge_ioctl(p: &Thread<Self::Proc>, com: u64, fp: &File<Self>) -> Result<(), Errno>;
    /// `fstat(2)` for a caller that is not root: `ub.st_gen = 0`.
    fn stat_hide_gen(st: &mut Self::Stat);
    /// `fdfree`'s check of a table's kqueue members once its descriptors are closed
    /// (`fd_nuserevents == 0`).
    fn fdkq_free(kq: &Self::FdKq);
}

/// Declares the `filehead` adapter of the glue `$glue` (`hal::pin_adapter!` of `f_list`), in
/// the module that implements it: `kernel::file_glue_links!(BsdFileGlue);` then
/// `type FileList = FileListLink;` inside the `impl FileGlue`.
#[macro_export]
macro_rules! file_glue_links {
    ($glue:ty) => {
        hal::pin_adapter!(
            /// `f_list`: `filehead`.
            pub FileListLink: $crate::file::File<$glue>, f_list
        );
    };
}

/// `fnew(p)`: a new open file with one reference and the thread's credentials, or `None` when
/// `maxfiles` is reached (`tablefull("file")`) or the pool cannot give an item.
pub fn fnew<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(p: &Thread<P>) -> Option<FileRef<G>> {
    let t = G::tables();
    let nfiles = t.numfiles.fetch_add(1, Ordering::AcqRel) + 1;
    if nfiles > G::maxfiles() {
        t.numfiles.fetch_sub(1, Ordering::AcqRel);
        G::tablefull("file");
        return None;
    }
    let cred = p.cred_ref();
    match t.filepl.get_homed(hal::pool::PR_WAITOK, || File::new(cred)) {
        Some(fp) => Some(FileRef(fp)),
        None => {
            t.numfiles.fetch_sub(1, Ordering::AcqRel);
            None
        }
    }
}

/// `fdrop(fp, p)`: the last reference is gone: unlink the file, close it and free it, the
/// close's error returned.
pub fn fdrop<G: FileGlue>(last: Last<File<G>>, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
    let error = last.close_once(p);
    drop(last);
    error
}

/// `foffset(fp)`: the file's offset, read under `f_mtx`.
pub fn foffset<G: FileGlue>(fp: &File<G>) -> Off {
    fp.stats().f_offset
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::format;
    use std::string::String;
    use std::vec;

    use hal::pool::PoolBox;

    use super::*;
    use crate::cred::crget;
    use crate::descrip::{fdinit, filedesc_init};
    use crate::proc::{ProcessRef, process_alloc, procinit, thread_alloc};
    use crate::testutil::{self, TestFileData, file_log};

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// The operations of the tests' files: the close is recorded, and fails for object 99.
    struct TOps;

    impl FileOps<Files> for TOps {
        fn read(&self, _: &File<Files>, _: &mut Uio<'_, Procs>, _: i32) -> Result<(), Errno> {
            Ok(())
        }
        fn write(&self, _: &File<Files>, _: &mut Uio<'_, Procs>, _: i32) -> Result<(), Errno> {
            Ok(())
        }
        fn ioctl(
            &self,
            _: &File<Files>,
            _: u64,
            _: &mut [u8],
            _: &Thread<Procs>,
        ) -> Result<(), Errno> {
            Err(Errno::ENOTTY)
        }
        fn kqfilter(&self, _: &File<Files>, _: &()) -> Result<(), Errno> {
            Err(Errno::EINVAL)
        }
        fn stat(
            &self,
            _: &File<Files>,
            _: &mut testutil::TestStat,
            _: &Thread<Procs>,
        ) -> Result<(), Errno> {
            Ok(())
        }
        fn close(&self, fp: &File<Files>, p: Option<&Thread<Procs>>) -> Result<(), Errno> {
            let id = fp.data().map_or(0, |d| d.id);
            testutil::file_log_push(format!("close {id} {}", p.is_some()));
            if id == 99 { Err(Errno::EIO) } else { Ok(()) }
        }
    }

    static TOPS: TOps = TOps;

    /// A thread of a new process with root's credentials and a table, its tables made.
    fn thread(pid: i32) -> (ProcessRef<Procs>, PoolBox<Thread<Procs>>) {
        testutil::install();
        procinit::<Procs>(64, 64);
        let _ = filedesc_init::<Files>();
        let pr = process_alloc::<Procs>(pid, ());
        drop(pr.set_ucred(crget()));
        pr.set_fd(fdinit());
        let t = thread_alloc::<Procs>(&pr, (), ());
        drop(t.set_cred(pr.ucred()));
        (pr, t)
    }

    fn numfiles() -> i32 {
        Files::tables().numfiles.load(Ordering::Relaxed)
    }

    fn install(fp: &FileRef<Files>, id: usize) {
        fp.install(
            DTYPE_PIPE,
            &TOPS,
            TestFileData {
                id,
                ..TestFileData::default()
            },
        );
    }

    #[test]
    fn the_last_release_closes_the_file_with_its_thread_and_frees_it() {
        let (_pr, p) = thread(10);
        let fp = fnew(&p).unwrap();
        assert_eq!(numfiles(), 1);
        assert_eq!(fp.count(), 1);
        assert_eq!(fp.f_type(), 0);
        install(&fp, 7);
        assert_eq!(fp.f_type(), DTYPE_PIPE);
        let two = fp.fref();
        assert_eq!(fp.count(), 2);
        assert_eq!(two.frele(Some(&p)), Ok(()));
        assert!(file_log().is_empty());
        assert_eq!(fp.frele(Some(&p)), Ok(()));
        assert_eq!(file_log(), vec![String::from("close 7 true")]);
        assert_eq!(numfiles(), 0);
        assert_eq!(Files::tables().filepl.nout(), 0);
    }

    #[test]
    fn a_close_error_comes_back_from_the_last_release_and_a_plain_drop_closes_too() {
        let (_pr, p) = thread(11);
        let fp = fnew(&p).unwrap();
        install(&fp, 99);
        assert_eq!(fp.frele(Some(&p)), Err(Errno::EIO));
        assert_eq!(file_log(), vec![String::from("close 99 true")]);

        // `(void)FRELE(fp, NULL)`: the last handle dropped closes with no thread.
        let fp = fnew(&p).unwrap();
        install(&fp, 5);
        drop(fp);
        assert_eq!(file_log(), vec![String::from("close 5 false")]);
        assert_eq!(numfiles(), 0);
    }

    #[test]
    fn a_file_never_installed_is_freed_without_a_close() {
        let (_pr, p) = thread(12);
        let fp = fnew(&p).unwrap();
        assert!(fp.data().is_none());
        assert_eq!(fp.frele(Some(&p)), Ok(()));
        assert!(file_log().is_empty());
        assert_eq!(numfiles(), 0);
    }

    #[test]
    #[should_panic(expected = "installed twice")]
    fn a_file_is_installed_once() {
        let (_pr, p) = thread(13);
        let fp = fnew(&p).unwrap();
        install(&fp, 1);
        install(&fp, 2);
    }

    #[test]
    fn past_maxfiles_no_file_is_made() {
        let (_pr, p) = thread(14);
        testutil::set_test_maxfiles(2);
        let a = fnew(&p).unwrap();
        let b = fnew(&p).unwrap();
        assert!(fnew(&p).is_none());
        assert_eq!(file_log(), vec![String::from("tablefull file")]);
        assert_eq!(numfiles(), 2);
        drop((a, b));
        assert_eq!(numfiles(), 0);
        testutil::set_test_maxfiles(1000);
    }

    #[test]
    fn a_file_in_flight_counts_as_the_c_s_f_count_and_is_adopted_once() {
        let (_pr, p) = thread(15);
        let fp = fnew(&p).unwrap();
        install(&fp, 3);
        // unp_internalize: FREF into the message.
        let word: &File<Files> = &fp;
        let addr = word.addr();
        fp.fref().forget();
        assert_eq!(fp.count(), 2);
        // The sender closes its descriptor; the message holds the file.
        let held = fp.clone();
        assert_eq!(fp.frele(Some(&p)), Ok(()));
        assert_eq!(held.count(), 2);
        // unp_externalize: the receiver adopts the message's reference.
        let received = FileRef::adopt(&held).unwrap();
        assert_eq!(received.addr(), addr);
        assert!(FileRef::adopt(&held).is_none());
        assert_eq!(held.count(), 2);
        assert_eq!(held.frele(Some(&p)), Ok(()));
        assert!(file_log().is_empty());
        assert_eq!(received.frele(Some(&p)), Ok(()));
        assert_eq!(file_log(), vec![String::from("close 3 true")]);
    }

    #[test]
    fn the_stats_and_the_offset_are_under_f_mtx() {
        let (_pr, p) = thread(16);
        let fp = fnew(&p).unwrap();
        {
            let mut s = fp.stats();
            s.f_offset = 42;
            s.f_rxfer += 1;
        }
        assert_eq!(foffset(&fp), 42);
        assert_eq!(fp.stats().f_rxfer, 1);
        assert!(CredRef::ptr_eq(fp.cred(), &p.cred_ref()));
    }
}
/* </TESTS> */
