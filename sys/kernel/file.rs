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
//! - `f_mtx` holds the counters it guards ([`FileStats`]); `f_offset` is an atomic beside
//!   them, written under `f_mtx` as the C writes it ([`File::set_offset`],
//!   [`File::advance_offset`]) and read without it ([`File::offset`], `foffset`): the C's
//!   reads under the vnode lock alone (`vn_read`, `vn_write`, `vn_seek`'s `SEEK_CUR`) take no
//!   `f_mtx` section, and `foffset`'s `f_mtx` (a 64-bit read that cannot tear) is the atomic
//!   load.
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
//!   without one (`(void)FRELE(fp, NULL)`) closes it the same way with no thread, in
//!   `FileRef`'s own drop, before the value's: the close (the unlink from `filehead`, the
//!   object's `fo_close`) always runs on the item at count 0 and before the value is
//!   dropped, never inside it (review S2 of N4.3: a close inside `File`'s drop ran while
//!   `filehead`'s walkers could still reach the value). The item cannot be freed while a
//!   handle exists, and a handle cannot outlive the item. A holder that may be empty (a
//!   descriptor's slot, a knote's `kn_fp`, nfsd's `ns_fp`) is a [`FileSlot`], one word as the
//!   C's pointer.
//! - `f_data` is installed once with its type ([`Installed`]), so it is read by reference
//!   (no cast; the pipe needs no slot of its own) and a close sees the object by reference
//!   (`fo_close(fp, p)`), which drops with the item after `numfiles` falls.
//! - `filehead` is a `hal::pinlist::PinList` under `fhdlk` ([`FileTables`]), the file's
//!   `f_list` a `PinEntry`: a file is unlinked by its close before its memory goes, checked.
//! - The descriptor in flight in a unix-domain message (a raw `struct file *` in an mbuf) is a
//!   parked reference ([`FileRef::forget`]/[`FileRef::adopt`]), counted by `f_count` as the
//!   C counts it, adopted once.

use core::pin::Pin;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, Ordering};

use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::pinlist::{PinAdapter, PinEntry, PinList};
use hal::pool::{Last, PoolArc, PoolHome, TypedPool};
use hal::sys::errno::Errno;
use hal::sys::fcntl::Flock;
use hal::sys::types::{Off, Register};

use crate::assert::kpanic;
use crate::cred::CredRef;
use crate::descrip::Filedesc;
use crate::event::{Knote, KqRef};
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

/// `vfs_stalling`: whether `vfs_stall(9)` is stalling the file systems (non-zero), read by
/// every `FREF`, counted by `bsd`'s `vfs_stall`; while it is, `FREF` waits
/// ([`FileGlue::stall_wait`]).
pub static VFS_STALLING: AtomicU32 = AtomicU32::new(0);

/// `vfs_stall_barrier()`: one relaxed load, and a wait while the file systems are stalled.
pub fn vfs_stall_barrier<G: FileGlue>() {
    if VFS_STALLING.load(Ordering::Relaxed) != 0 {
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

/// \[f\] The counters `f_mtx` guards (`f_offset`, written under it, is [`File::offset`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FileStats {
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
    /// \[f,v\] `f_offset`: written with `f_mtx` held, read without it.
    f_offset: AtomicI64,
    /// `f_mtx` and the counters it guards.
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
            f_offset: AtomicI64::new(0),
            // We need to block interrupts as long as `f_mtx` is being taken with and without
            // the kernel lock.
            stats: Mutexed::new(hal::intr::mpfloor(), FileStats::default()),
        }
    }

    /// `fp->f_flag` as the `int` the `fcntl.h` constants are.
    pub fn flag(&self) -> i32 {
        self.f_flag.load(Ordering::Relaxed) as i32
    }

    /// `fp->f_count` read through the file itself, with no reference taken: the references
    /// alive now (a snapshot), 0 once the last is gone. For a holder that names the file
    /// without a reference and must not take one (`unp_gc`, under a lock the file's last
    /// close would take again).
    pub fn f_count(&self) -> u32 {
        PoolArc::count_of(self) as u32
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

    /// `f_mtx` held, and the counters it guards.
    pub fn stats(&self) -> MutexedGuard<'_, FileStats> {
        self.stats.lock()
    }

    /// `fp->f_offset`, read without `f_mtx` (one atomic load: the C's reads under the vnode
    /// lock, and `foffset`'s).
    pub fn offset(&self) -> Off {
        self.f_offset.load(Ordering::Relaxed)
    }

    /// `fp->f_offset = off` under `f_mtx` (`vn_seek`, `vn_write`'s append, `getdents`).
    pub fn set_offset(&self, off: Off) {
        let _f_mtx = self.stats.lock();
        self.f_offset.store(off, Ordering::Relaxed);
    }

    /// `fp->f_offset += n` under `f_mtx` (`vn_read`, `vn_write`): the writers are serialised
    /// by the mutex, so the load and the store are one update.
    pub fn advance_offset(&self, n: Off) {
        let _f_mtx = self.stats.lock();
        let off = self.f_offset.load(Ordering::Relaxed);
        self.f_offset.store(off.wrapping_add(n), Ordering::Relaxed);
    }

    /// The file's address (`kinfo_file.f_fileaddr`, `flock(2)`'s lock owner).
    pub fn addr(&self) -> usize {
        core::ptr::from_ref(self).addr()
    }

    /// `fdrop`'s work, on the item at count 0 ([`fdrop`] alone calls it, once per item):
    /// unlink from `filehead`, `fo_close` of an installed file, `numfiles--`. The item itself
    /// goes back after, credentials and object with it (`crfree`, the object's drop).
    fn close(&self, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
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

/// A counted reference to an open file (`struct file *` with a reference: `f_count`). Its
/// handle is `None` only inside an empty [`FileSlot`] and between a release and the end of
/// the reference's own drop; no `FileRef` a caller holds is ever empty.
pub struct FileRef<G: FileGlue>(Option<PoolArc<File<G>>>);

impl<G: FileGlue> FileRef<G> {
    /// The handle: present in every `FileRef` but an empty slot's (see the type).
    fn arc(&self) -> &PoolArc<File<G>> {
        match &self.0 {
            Some(a) => a,
            None => kpanic(format_args!("FileRef: an empty reference read")),
        }
    }

    /// `FREF(fp)`: one more reference, after `vfs_stall_barrier()`.
    pub fn fref(&self) -> Self {
        vfs_stall_barrier::<G>();
        self.clone()
    }

    /// `FRELE(fp, p)`: gives the reference back; the last one closes the file (`fdrop`) and
    /// returns the close's error.
    pub fn frele(mut self, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
        match self.0.take().and_then(PoolArc::try_last) {
            Some(last) => fdrop(last, p),
            None => Ok(()),
        }
    }

    /// `fp->f_count`: the references alive now (slots, holders, knotes, messages in flight).
    pub fn count(&self) -> u32 {
        PoolArc::count(self.arc()) as u32
    }

    /// The file's address (`kinfo_file.f_fileaddr`).
    pub fn addr(&self) -> usize {
        self.arc().addr()
    }

    /// Whether two references name the same file.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        PoolArc::ptr_eq(a.arc(), b.arc())
    }

    /// The file, pinned (for `filehead`).
    pub fn as_pin(&self) -> Pin<&File<G>> {
        self.arc().as_pin()
    }

    /// Gives the reference up as a word in legacy memory (a descriptor in flight in a
    /// message, `unp_internalize`'s `FREF`): still counted, taken back by one
    /// [`FileRef::adopt`].
    pub fn forget(mut self) {
        if let Some(a) = self.0.take() {
            PoolArc::forget(a);
        }
    }

    /// The reference a [`FileRef::forget`] parked on `fp` (a message received or discarded).
    /// `None` when none is parked: a word adopted twice.
    pub fn adopt(fp: &File<G>) -> Option<Self> {
        PoolArc::adopt(fp).map(|a| Self(Some(a)))
    }

    /// One more reference to a file a list lends (`fd_iterfile`'s CAS from a non-zero
    /// count): `None` for a file whose last reference is gone, its close under way.
    pub fn try_from_ref(fp: &File<G>) -> Option<Self> {
        PoolArc::try_from_ref(fp).map(|a| Self(Some(a)))
    }
}

impl<G: FileGlue> Drop for FileRef<G> {
    /// A reference dropped without [`FileRef::frele`] (the C's `(void)FRELE(fp, NULL)`:
    /// `unp_discard`, a knote's, a table freed with its slots): the last one closes the file
    /// with no thread (`fdrop`), its error dropped as the C drops it.
    #[inline(always)]
    fn drop(&mut self) {
        // An empty reference (every empty slot of a table being dropped) is a load and a
        // compare inline, no call.
        if self.0.is_some() {
            drop_slow(self);
        }
    }
}

/// A held reference's drop, out of line: the decrement, and the close when it was the last.
#[inline(never)]
fn drop_slow<G: FileGlue>(r: &mut FileRef<G>) {
    if let Some(last) = r.0.take().and_then(PoolArc::try_last) {
        let _ = fdrop(last, None);
    }
}

impl<G: FileGlue> Clone for FileRef<G> {
    /// One more reference without `vfs_stall_barrier()` (a knote's slot read, a table's copy
    /// of a reference the caller holds).
    fn clone(&self) -> Self {
        Self(Some(self.arc().clone()))
    }
}

impl<G: FileGlue> core::ops::Deref for FileRef<G> {
    type Target = File<G>;

    fn deref(&self) -> &File<G> {
        self.arc()
    }
}

/// A holder of at most one reference to an open file, one word as the C's `struct file *`
/// (`fd_ofiles[fd]`, a knote's `kn_fp`, nfsd's `ns_fp`): empty, or a [`FileRef`] whose drop
/// closes the file when it is the last. `Option<FileRef>` is two words (the reference's
/// own handle is optional), which would grow `fdescpl`, the knotes and `nfssvc_sock`.
pub struct FileSlot<G: FileGlue>(FileRef<G>);

impl<G: FileGlue> FileSlot<G> {
    /// An empty slot (the C's NULL).
    pub const fn new() -> Self {
        Self(FileRef(None))
    }

    /// A slot holding `fp`, or empty.
    pub fn with(fp: Option<FileRef<G>>) -> Self {
        fp.map_or_else(Self::new, Self)
    }

    /// The reference held, if any.
    pub fn get(&self) -> Option<&FileRef<G>> {
        self.0.0.is_some().then_some(&self.0)
    }

    /// Whether the slot holds a file.
    pub fn is_some(&self) -> bool {
        self.0.0.is_some()
    }

    /// Whether the slot is empty.
    pub fn is_none(&self) -> bool {
        self.0.0.is_none()
    }

    /// The reference held, the slot left empty.
    pub fn take(&mut self) -> Option<FileRef<G>> {
        let fp = core::mem::replace(&mut self.0, FileRef(None));
        fp.0.is_some().then_some(fp)
    }

    /// `fp` put in a slot known to be empty (`fdinsert`, a placed or duplicated descriptor,
    /// `fdcopy`'s new table): no old reference to drop, so no close can run here (under
    /// `fd_fplock`); an occupied slot stops the kernel, the C's `KASSERT(fd_ofiles[fd] ==
    /// NULL)` made unconditional.
    pub fn fill(&mut self, fp: FileRef<G>) {
        if self.is_some() {
            kpanic(format_args!("FileSlot::fill: the slot holds a file"));
        }
        // The old value is empty, as checked: its drop is the inline compare alone.
        drop(core::mem::replace(&mut self.0, fp));
    }

    /// `fp` put in the slot; the reference it held before, if any.
    pub fn replace(&mut self, fp: FileRef<G>) -> Option<FileRef<G>> {
        let old = core::mem::replace(&mut self.0, fp);
        old.0.is_some().then_some(old)
    }

    /// `fp` (or nothing) put in the slot; the reference it held before, if any.
    pub fn set(&mut self, fp: Option<FileRef<G>>) -> Option<FileRef<G>> {
        match fp {
            Some(fp) => self.replace(fp),
            None => self.take(),
        }
    }
}

impl<G: FileGlue> Default for FileSlot<G> {
    fn default() -> Self {
        Self::new()
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
    /// `numfiles`: actual number of open files (the glue's static: `kern.nfiles` reads it).
    pub numfiles: &'static AtomicI32,
}

impl<G: FileGlue> FileTables<G> {
    /// Empty tables whose `fhdlk` is at `ipl` (`IPL_MPFLOOR`), counting the open files in
    /// `numfiles`; `filedesc_init` makes the pools.
    pub const fn new(ipl: i32, numfiles: &'static AtomicI32) -> Self {
        Self {
            fhdlk: Mutexed::new(ipl, ()),
            filehead: PinList::new(),
            filepl: TypedPool::new(),
            fdescpl: TypedPool::new(),
            numfiles,
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
    fn kqfilter(&self, fp: &File<G>, kn: &Knote<G::Proc>) -> Result<(), Errno>;
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
/// file holds and the operations on it, the vnodes of `fd_cdir`/`fd_rdir`, the tables
/// themselves, the kqueue file's operations and object (N4.4: kqueue is the kernel's, the
/// `struct fileops` table the glue's), and the calls `kern_descrip.c` and `sys_generic.c`
/// make into the vnode layer, the sockets, signals and pledge. One instantiation, `bsd`'s
/// (`sys/kern/kern_descrip.rs`); a bridge of `bridges.toml` while `bsd` implements it.
pub trait FileGlue: Send + Sync + Sized + 'static {
    /// The threads whose tables these are.
    type Proc: ProcGlue<FileGlue = Self>;
    /// `struct fileops`: the operations a file is installed with.
    type Ops: ?Sized + FileOps<Self> + 'static;
    /// `f_data`: the object of a file (a vnode, a socket, a pipe, a kqueue).
    type Data: Send + Sync + 'static;
    /// `struct stat`, as `fo_stat` fills it.
    type Stat: Default;
    /// `struct vnode` (`fd_cdir`, `fd_rdir`).
    type Vnode: Sync + 'static;
    /// `f_list`: `filehead`'s adapter ([`file_glue_links!`](crate::file_glue_links)).
    type FileList: PinAdapter<Elem = File<Self>>;
    /// The members of `struct vnode` of `bsd`'s types: the name cache's heads (`v_nc_tree`,
    /// `v_cache_dst`, N5.3), `v_socket` and a device's advisory locks (`si_lockf`, N5.3);
    /// `Default` is a new vnode's (`kernel::fs::vnode`).
    type VnodeG: Default + Sync + 'static;
    /// `struct componentname` (`namei`'s, `bsd`'s until N5.3): what `VOP_LOOKUP` and the
    /// directory operations receive.
    type Componentname;
    /// `struct nameidata` (`bsd`'s until N5.3): what `VFS_MOUNT` receives.
    type Nameidata<'a>;

    /// The open files, `fhdlk` and the pools (one `static` of `bsd`'s).
    fn tables() -> &'static FileTables<Self>;
    /// `maxfiles` (`kern.maxfiles`).
    fn maxfiles() -> i32;
    /// `tablefull(tab)`: the "table is full" message.
    fn tablefull(tab: &str);
    /// `vfs_stall_barrier()`'s wait while [`VFS_STALLING`] is set (`rw_enter_read` and
    /// `rw_exit_read` of `vfs_stall_lock`).
    fn stall_wait();
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
    /// `kqueueops` and the object of a new kqueue file (`dokqueue`'s `f_ops`, `f_data`).
    fn kqueue_file(kq: KqRef<Self::Proc>) -> (&'static Self::Ops, Self::Data);
    /// The kqueue a kqueue file's object is (`fp->f_data` of a `DTYPE_KQUEUE` file); `None`
    /// for another object.
    fn kqueue_of(d: &Self::Data) -> Option<&KqRef<Self::Proc>>;
    /// Whether a vnode file's vnode was revoked (its operations are deadfs's:
    /// `kqueue_scan`'s `vp->v_op == &dead_vops`); `false` for any other object.
    fn vnode_revoked(d: &Self::Data) -> bool;
    /// The vnode a `DTYPE_VNODE` file's object is (`(struct vnode *)fp->f_data`), as the
    /// pager and `mmap(2)` see it (`VnodeOps`, `bsd`'s vnodes until N5); `None` for another
    /// object.
    fn vnode_ops(d: &Self::Data) -> Option<&'static dyn crate::vm::vnode::VnodeOps>;
    /// The pipe a `DTYPE_PIPE` file's object holds (`fp->f_data`, the pipe the file reads
    /// and whose peer it writes), for the pipe's filters (`kernel::pipe`'s `filt_pipe*`);
    /// `None` for another object.
    fn pipe_of(d: &Self::Data) -> Option<&crate::pipe::PipeEnd<Self::Proc>>;
    /// `pipeops` and the object of a new pipe file (`dopipe`'s `f_ops` and `f_data`): the
    /// object holds `end`, the file's pipe, which goes with the file's last reference
    /// (`pipe_close`'s `pipe_destroy`).
    fn pipe_file(end: crate::pipe::PipeEnd<Self::Proc>) -> (&'static Self::Ops, Self::Data);
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
    /// `copyout(&ub, sb, sizeof(ub))` of `fstat(2)`: the structure's user bytes at `sb`.
    fn stat_copyout(st: &Self::Stat, sb: usize) -> Result<(), Errno>;
    /// `vfs_getcwd_common(vp, NULL, &bp, path, limit, GETCWD_CHECK_ACCESS, p)`
    /// (`kern.proc_cwd`, N4.6b): the path of the directory `vp` written backwards into `path`
    /// from `bp` (where the caller put the terminating NUL), the new `bp` returned. The
    /// default is a glue without file systems (the kernel's tests): no path (`ENOENT`).
    fn getcwd(
        _vp: &'static Self::Vnode,
        _path: &mut [u8],
        _bp: usize,
        _limit: i32,
        _p: &Thread<Self::Proc>,
    ) -> Result<usize, Errno> {
        Err(Errno::ENOENT)
    }
    /// `uiomove(buf, len, uio)` (`kern_subr.c`, `bsd`'s until the device switch moves, N7):
    /// moves `buf` (at most `uio_resid` bytes of it) in the uio's direction, a special file's
    /// block-device read or write through the buffer cache.
    fn uiomove(buf: &mut [u8], uio: &mut Uio<'_, Self::Proc>) -> Result<(), Errno>;
    /// The vnode and mount tables of this glue's processes (one `static` of `bsd`'s).
    fn vfs() -> &'static crate::fs::subr::VfsTables<Self::Proc>;
    /// A new vnode's `bsd` members made ready at their place (`getnewvnode`'s
    /// `cache_tree_init` and `TAILQ_INIT(&vp->v_cache_dst)`).
    fn vnode_g_init(vp: &'static crate::fs::vnode::Vnode<Self::Proc>);
    /// A recycled vnode's `bsd` members cleared (`getnewvnode`'s `vp->v_socket = NULL`).
    fn vnode_recycle(vp: &'static crate::fs::vnode::Vnode<Self::Proc>);
    /// `cache_purge(vp)`: the name cache forgets the vnode, which gets a new capability
    /// (`v_id`). Required: a glue that forgot it would leave stale names.
    fn cache_purge(vp: &'static crate::fs::vnode::Vnode<Self::Proc>);
    /// `lf_purgelocks(&vp->v_specinfo->si_lockf)` (`vgonel` of a device vnode).
    fn lf_purgelocks(vp: &'static crate::fs::vnode::Vnode<Self::Proc>);
    /// `lf_advlock(&vp->v_speclockf, 0, id, op, fl, flags)` (`spec_advlock`).
    fn spec_advlock(
        vp: &'static crate::fs::vnode::Vnode<Self::Proc>,
        id: *const core::ffi::c_void,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno>;
    /// `vop_generic_abortop`'s `pool_put(&namei_pool, cnp->cn_pnbuf)` when the lookup left a
    /// buffer (`HASBUF`) and was not asked to keep it (`SAVESTART`).
    fn abortop(cnp: &mut Self::Componentname);
    /// `dounmount(mp, flags, p)` (`vop_generic_revoke`'s forced unmount of a revoked block
    /// device's file system; `bsd`'s until N5.4).
    fn dounmount(
        mp: &'static crate::fs::mount::Mount<Self::Proc>,
        flags: i32,
        p: &Thread<Self::Proc>,
    ) -> Result<(), Errno>;
    /// `spec_close`'s release of the session's controlling terminal: when the closing
    /// thread's session's `s_ttyvp` is `vp`, it is cleared and `true` returned (the caller
    /// `vrele`s the vnode); `false` otherwise.
    fn session_ttyvp_take(
        p: &Thread<Self::Proc>,
        vp: &'static crate::fs::vnode::Vnode<Self::Proc>,
    ) -> bool;
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
        Some(fp) => Some(FileRef(Some(fp))),
        None => {
            t.numfiles.fetch_sub(1, Ordering::AcqRel);
            None
        }
    }
}

/// `fdrop(fp, p)`: the last reference is gone: unlink the file, close it and free it, the
/// close's error returned.
pub fn fdrop<G: FileGlue>(last: Last<File<G>>, p: Option<&Thread<G::Proc>>) -> Result<(), Errno> {
    let error = last.close(p);
    drop(last);
    error
}

/// `foffset(fp)`: the file's offset (the C's read under `f_mtx`; here one atomic load).
pub fn foffset<G: FileGlue>(fp: &File<G>) -> Off {
    fp.offset()
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
        fn kqfilter(&self, _: &File<Files>, _: &crate::event::Knote<Procs>) -> Result<(), Errno> {
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
        let t = thread_alloc::<Procs>(&pr, ());
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

    /// `f_count` through the file (unp_gc's read): the references, never one more, and the
    /// read is never the release that closes the file.
    #[test]
    fn f_count_reads_the_references_without_taking_one() {
        let (_pr, p) = thread(15);
        let fp = fnew(&p).unwrap();
        install(&fp, 8);
        let file: &File<Files> = &fp;
        assert_eq!(file.f_count(), 1);
        let two = fp.fref();
        assert_eq!(file.f_count(), 2);
        assert_eq!(two.frele(Some(&p)), Ok(()));
        assert_eq!(fp.f_count(), 1);
        assert!(file_log().is_empty());
        assert_eq!(fp.frele(Some(&p)), Ok(()));
        assert_eq!(file_log(), vec![String::from("close 8 true")]);
    }

    /// A slot is one word, empty or holding a reference; the reference it drops closes the
    /// file when it is the last, before the value's drop (review S2: no close inside
    /// `File`'s drop).
    #[test]
    fn a_slot_is_one_word_and_its_last_reference_closes_the_file() {
        assert_eq!(size_of::<FileSlot<Files>>(), size_of::<usize>());
        assert_eq!(size_of::<FileRef<Files>>(), size_of::<usize>());
        let (_pr, p) = thread(16);
        let mut slot = FileSlot::<Files>::new();
        assert!(slot.is_none() && slot.get().is_none() && slot.take().is_none());
        let fp = fnew(&p).unwrap();
        install(&fp, 6);
        assert!(slot.replace(fp.clone()).is_none());
        assert!(slot.is_some());
        assert_eq!(fp.count(), 2);
        assert!(FileRef::ptr_eq(slot.get().unwrap(), &fp));
        let back = slot.take().unwrap();
        assert!(slot.is_none());
        assert!(slot.set(Some(back)).is_none());
        assert_eq!(fp.frele(Some(&p)), Ok(()));
        assert!(file_log().is_empty());
        assert_eq!(numfiles(), 1);
        drop(slot);
        assert_eq!(file_log(), vec![String::from("close 6 false")]);
        assert_eq!(numfiles(), 0);
    }

    /// `fill` takes a known-empty slot and never drops an old reference: an occupied slot
    /// stops the kernel instead of closing a file under the caller's lock.
    #[test]
    #[should_panic(expected = "the slot holds a file")]
    fn filling_an_occupied_slot_stops_the_kernel() {
        let (_pr, p) = thread(17);
        let fp = fnew(&p).unwrap();
        let mut slot = FileSlot::<Files>::new();
        slot.fill(fp.clone());
        assert_eq!(fp.count(), 2);
        slot.fill(fp);
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
    fn the_stats_are_under_f_mtx_and_the_offset_beside_them() {
        let (_pr, p) = thread(16);
        let fp = fnew(&p).unwrap();
        fp.stats().f_rxfer += 1;
        fp.set_offset(40);
        fp.advance_offset(2);
        assert_eq!(foffset(&fp), 42);
        assert_eq!(fp.offset(), 42);
        // The counters' guard does not cover the offset's read (no f_mtx in foffset).
        {
            let s = fp.stats();
            assert_eq!(foffset(&fp), 42);
            assert_eq!(s.f_rxfer, 1);
        }
        assert_eq!(fp.stats().f_rxfer, 1);
        assert!(CredRef::ptr_eq(fp.cred(), &p.cred_ref()));
    }
}
/* </TESTS> */
