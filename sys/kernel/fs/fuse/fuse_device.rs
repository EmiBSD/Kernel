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
 * Copyright (c) 2012-2013 Sylvestre Gallon <ccna.syl@gmail.com>
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
/* </LICENSES> */

/* <CODE> */
//! The fuse(4) device: `/dev/fuse0`, the character device (`pseudo-device fuse`, major 92,
//! cloning, no ioctl) through which the userland file system daemon reads the kernel's
//! requests and writes its replies.
//!
//! Upstream: sys/miscfs/fuse/fuse_device.c @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fuse_device.rs@3c62ede68802
//!
//! Each open of the device makes a `struct fuse_d` for its (clone) minor. A request
//! (`fusebuf`) queued by `fb_queue` waits on `fd_fbufs_in` until the daemon reads it
//! (`fuseread`), then on `fd_fbufs_wait` until the daemon writes the reply with the same ID
//! (`fusewrite`), which wakes the requesting thread. kqueue's `EVFILT_READ` tells the daemon
//! that a request is ready.
//!
//! Locks used to protect struct members and global data: \[l\] `fd_lock`; the device list,
//! the wait queue and the requests under the kernel lock, which every entry point holds
//! (`vn_open`, `vn_read`, `vn_write`, `vn_closefile`, `vn_kqfilter` take it), as the C's.
//!
//! The entry points carry the `cdevsw` signatures over the glue (`bsd`'s `Uio`, `Proc` and
//! `Knote` are the kernel's over `BsdProcGlue`), so N5.9f2's switch names
//! `fuseopen::<BsdProcGlue>` and the others in `cdev_fuse_init` with no adapter (D-N5.9-7).
//!
//! ## Deviations
//! - `NFUSE` (config(8)'s `fuse.h`) is 1, GENERIC's `pseudo-device fuse` (LZ's).
//! - `fuse_device_cleanup` empties each queue by removing its head until it is empty. The C
//!   walks the queue keeping the removed element as `lprev` and calls
//!   `SIMPLEQ_REMOVE_AFTER(lprev)` on it, which unlinks nothing from the queue once two or
//!   more fusebufs are queued (the head keeps pointing at the second); the comment says
//!   every message is cleared, which the port does (LZ's fix, kept).
//! - A cleared `FUSE_FORGET` or `FUSE_INIT` fusebuf, which nobody waits for, is freed with
//!   the queue's reference (and so is one queued on a device that is not open), where the C
//!   (and LZ) leak it: `vmstat -m`'s `fmsg` `InUse` is lower than OpenBSD's after a cleanup,
//!   its requests equal (D-N5.9-15).
//! - `fuseclose` does not wait for the device's other users (the C's `refcnt_finalize(..,
//!   "fusedfd")`): the device is a counted block, and the memory goes with its last handle,
//!   so `vmstat -m`'s `M_DEVBUF` block may be freed a moment after `close(2)` returns
//!   (`InUse` equal at rest; D-N5.9-9).
//! - `fuse_lookup` matches the whole `dev_t` a unit was opened with, where the C matches the
//!   minor alone: the same for every node of `cdevsw[92]`; a `VCHR` of another major with the
//!   minor of an open unit names no fuse device (N5.9f2's `fusefs_mount` refuses it `ENXIO`,
//!   the user's D-N5.9-U1).
//! - `fusewrite` returns `ENODEV` (and the requester gets `ENXIO`) when the device has no
//!   mount any more where the C reads `fd_fmp->max_read` or `fd_fmp->max_write` through the
//!   NULL pointer; that only happens if the unmount ran between the read and the reply (LZ's).
//! - `fusewrite` negates the reply's error with `wrapping_neg` (`INT_MIN` stays `INT_MIN`,
//!   the C's overflow); the reply's data length is computed with wrapping arithmetic as the
//!   C's `size_t` expression is, so a short `len` fails the `max_read` check (LZ's).
//! - `fusewrite` refuses `EINVAL` a reply whose bytes after the header are fewer than the
//!   operation's output structure (`op_out_len`), and the requester gets `EIO`: the C's
//!   `uiomove(&fbuf->op, op_out_len, uio)` copies what the daemon sent and leaves the rest of
//!   the union holding the request's own `op.in` bytes, which are then read as the answer
//!   (an `entry.nodeid`, a `write.size`); a correct daemon always sends whole structures
//!   (the user's D-N5.9-U2, its third bound).
//! - `fusewrite` takes a request off the wait queue only while it is still there: a cleanup
//!   that ran while the reply's bytes were being copied (the daemon closed its device from
//!   another thread) took it off, failed and woke its requester already; the C's
//!   `SIMPLEQ_REMOVE_AFTER` then unlinks through a stale `lastfbuf`.
//!
//! Kept as the C, named: a request queued through a device handle taken before `fuseclose`'s
//! cleanup sleeps for ever (the C's `refcnt_finalize` window, D-N5.9-9); the daemon reads and
//! writes without `PCATCH` on the requester's side (`fb_queue`).
//!
//! ## Redesign
//! - `struct fuse_d` ([`FuseD`]) is a counted block of `malloc(M_DEVBUF)` ([`FuseDRef`],
//!   `MArc`) where LZ wrote a raw block through `unsafe` and shared it under an `unsafe impl
//!   Sync`: the C's `fd_refcnt` is the block's count (`fuse_lookup`'s `refcnt_take` a clone,
//!   `refcnt_rele_wake` its drop), the device list's place a reference the device parks on
//!   itself (`fd_self`, taken at `fuseclose`), the C's `refcnt_finalize` wait unneeded
//!   (docs/IDIOMS.md, the row of a C `refcnt` on a malloc'd object).
//! - The queues are `PinList`s of the requests' pool items, each request queued holding one
//!   parked reference (`PoolArc::forget` at the insert, `PoolArc::adopt` at the removal):
//!   `fd_fbufs_in` under `fd_lock` (a `RawRwlock`, the list's witness), `fd_fbufs_wait`
//!   under the kernel lock, which `fusewrite` holds where the C takes no lock: its walk takes
//!   `klock::reader()` (no interrupt level), its insertion (`fuseread`) and removal
//!   (`fusewrite`, the cleanup) `klock::witness()`, an spl pair each the C lacks: two per
//!   request, docs/ROADMAP.md N12's row (D-N5.3-2's class, as the vnode hashes'). The device
//!   list is the same: `klock::reader()` for `fuse_lookup`'s walk (the C's `LIST_FOREACH`,
//!   O(1) a step, O(n) over the open devices as the C's), the witness at open and close.
//! - The read filter's `kn_hook` is the device, a counted handle in the knote
//!   (`Knote::set_hook_shared`), as the C's `kn_hook = fd` and the tty's: the filters are
//!   `FILTEROP_MPSAFE` and run their `f_modify`, `f_process` and `f_detach` without the
//!   kernel lock, so a unit looked up again per event under `klock::reader()` (N5.9's plan)
//!   would panic there (N5.9f1's D-f1-3). `fd_rklist` is a klist of its owner's lock
//!   (`Klist::owner`), `fd_lock`, as the C's `klist_init_rwlock(&fd_rklist, &fd_lock)`; the
//!   filter reads the input queue's length from `fd_nin`, a word written under `fd_lock` with
//!   each change of the queue (`filt_fuse_read` is given no witness).
//! - The copies are the C's, through `bsd`'s `uiomove` (the `FileGlue::uiomove` bridge, N7,
//!   for a slice): `fuseread` copies the header and the input structure out of stack
//!   copies (40 and at most 128 bytes, LZ's for the header), and the data out of the request's
//!   block, moved out of its cell for the copy and freed after it as the C's; `fusewrite`
//!   copies the output structure into a stack array then the union's words (at most 128
//!   bytes, one copy more than the C's), the data into a fresh `M_FUSEFS` block (zeroed by
//!   `MArray::new`, a pass over the bytes the C's `malloc(M_WAITOK)` does not make: N12's
//!   row) installed after the copy.

use core::ptr;
use core::sync::atomic::Ordering;

use hal::klock;
use hal::malloc::{M_WAITOK, MArc, MArray};
use hal::pinlist::{PinEntry, PinList};
use hal::pool::PoolArc;
use hal::rwlock::{RW_WRITE, RawRwlock};
use hal::sleep::{INFSLP, PCATCH, PWAIT, wakeup, wakeup_one};
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::fcntl::O_EXCL;
use hal::sys::types::{Dev, minor};
use hal::user::bytes_of_mut;

use crate::assert::{kassert, kpanic};
use crate::event::{
    EVFILT_READ, EVFILT_WRITE, FILTEROP_ISFD, FILTEROP_MPSAFE, Filter, Filterops, Kevent, Klist,
    Knote, knote_modify, knote_process, seltrue_kqfilter,
};
use crate::file::FileGlue;
use crate::fs::fuse::fusebuf::FusebufRef;
use crate::fs::fuse::fusebuf_h::{
    FUSE_FORGET, FUSE_INIT, FUSE_KERNEL_VERSION, FUSEBUF_OP_SIZE, FUSEBUFMAXSIZE, FbLink,
    FuseInHeader, FuseInitOut, FuseOutHeader, Fusebuf,
};
use crate::fs::fuse::fusefs::{FuseTables, FusefsMntRef, M_FUSEFS};
use crate::fs::subr::vfs;
use crate::fs::vnode::IO_NDELAY;
use crate::fs::word::U32Word;
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;

/// `NFUSE`: the number of fuse pseudo-devices (`pseudo-device fuse` in GENERIC: 1).
pub const NFUSE: i32 = 1;

/// `M_DEVBUF` (`<sys/malloc.h>`): device driver memory (the device's block).
const M_DEVBUF: i32 = 2;

/// `struct fuse_d`: an open fuse(4) device.
pub struct FuseD<G: ProcGlue> {
    /// `fd_list`: the link on `fuse_d_list`.
    pub fd_link: PinEntry<FuseD<G>>,
    /// The reference the device list's place holds, parked on the device by `fuseopen` and
    /// taken by `fuseclose` (the module's `## Redesign`).
    fd_self: ReadMostly<Option<FuseDRef<G>>>,
    /// `fd_lock`.
    pub fd_lock: RawRwlock,
    /// `fd_fmp`: the mount the device serves, `None` before the mount and after the unmount.
    pub fd_fmp: ReadMostly<Option<FusefsMntRef<G>>>,
    /// `fd_unit`: the minor number.
    pub fd_unit: i32,
    /// The device the unit was opened as (the module's deviations).
    pub fd_dev: Dev,
    /// \[l\] `fd_fbufs_in`: the requests the daemon has not read yet.
    pub fd_fbufs_in: PinList<FbLink>,
    /// \[l\] The length of `fd_fbufs_in`, for the read filter.
    fd_nin: U32Word,
    /// `fd_fbufs_wait`: the requests the daemon read and has not answered yet (under the
    /// kernel lock).
    pub fd_fbufs_wait: PinList<FbLink>,
    /// \[l\] `fd_rklist`: kq fields.
    pub fd_rklist: Klist<G>,
}

hal::pin_adapter!(
    /// `fd_list`: the link of a device on `fuse_d_list`.
    impl[G: ProcGlue] pub FuseDLink<G>: FuseD<G>, fd_link
);

/// `struct fuse_d *` held: a counted handle to a device's block of `malloc(M_DEVBUF)`, which
/// the last handle frees.
pub type FuseDRef<G> = MArc<FuseD<G>>;

impl<G: ProcGlue> FuseD<G> {
    /// The device of the unit `unit` opened as `dev`, as `fuseopen` initialises its
    /// `malloc(M_ZERO)` block: empty queues, no mount.
    fn new(dev: Dev, unit: i32) -> Self {
        Self {
            fd_link: PinEntry::new(),
            fd_self: ReadMostly::new(None),
            fd_lock: RawRwlock::new("fusedlk"),
            fd_fmp: ReadMostly::new(None),
            fd_unit: unit,
            fd_dev: dev,
            fd_fbufs_in: PinList::new(),
            fd_nin: U32Word::new(0),
            fd_fbufs_wait: PinList::new(),
            fd_rklist: Klist::owner(),
        }
    }

    /// Whether the device serves a mount (`fd->fd_fmp != NULL`).
    pub fn has_fmp(&self) -> bool {
        self.fd_fmp.read().is_some()
    }
}

/// `fuse_rd_filtops`: the read filter of a fuse(4) device.
pub struct FuseRead;

/// The erased table of the read filter (`kn_fop`'s target).
struct FuseFiltops<G>(core::marker::PhantomData<fn() -> G>);

impl<G: ProcGlue> FuseFiltops<G> {
    /// `&fuse_rd_filtops`.
    const READ: Filterops<G> = Filterops::of::<FuseRead>();
}

/// The glue's FUSE tables.
fn tables<G: ProcGlue>() -> &'static FuseTables<G> {
    &vfs::<G>().fuse
}

/// `uiomove(buf, n, uio)` of the kernel bytes `buf` (`bsd`'s, the `FileGlue::uiomove`
/// bridge).
fn uiomove<G: ProcGlue>(buf: &mut [u8], uio: &mut Uio<'_, G>) -> Result<(), Errno> {
    <G::FileGlue as FileGlue>::uiomove(buf, uio)
}

/// The queue's reference of a request it lends: `panic(9)` when none is parked there.
fn adopt(f: &Fusebuf) -> FusebufRef {
    match PoolArc::adopt(f) {
        Some(q) => q,
        None => kpanic(format_args!("fuse: fusebuf {f:p} queued with no reference")),
    }
}

/// `fuse_lookup(unit)`: the open device of `dev` (the module's deviations), with a reference
/// taken (its drop is the C's `refcnt_rele_wake`).
pub fn fuse_lookup<G: ProcGlue>(dev: Dev) -> Option<FuseDRef<G>> {
    let r = klock::reader();
    tables::<G>()
        .devices
        .iter(&r)
        .find(|fd| fd.fd_dev == dev)
        .and_then(|fd| fd.fd_self.read().clone())
}

/// Fails the request `f`, just taken off its queue, with `ENXIO` and wakes the VFS syscall
/// waiting on it.
fn fuse_device_fail(f: &Fusebuf) {
    f.set_fb_err(Errno::ENXIO as i32);
    // Wakeup up VFS syscall waiting on this fbuf, it will fail
    wakeup(ptr::from_ref(f));
}

/// `fuse_device_cleanup`'s body on the device: every request of both queues failed.
fn fuse_device_cleanup_fd<G: ProcGlue>(fd: &FuseD<G>) {
    let t = tables::<G>();

    // clear FIFO IN
    let mut w = fd.fd_lock.write();
    while let Some(q) = fd.fd_fbufs_in.first(&w).map(adopt) {
        fd.fd_fbufs_in.remove_first(&mut w);
        fd.fd_nin.set(fd.fd_nin.get() - 1);
        // DPRINTF("cleanup unprocessed msg in sc_fbufs_in\n");
        t.stat_fbufs_in.fetch_sub(1, Ordering::Relaxed);
        fuse_device_fail(&q);
        // The queue's reference goes: a request nobody waits for is freed.
        drop(q);
    }
    fd.fd_rklist.knote_locked(&w, 0);
    drop(w);

    // clear FIFO WAIT
    let mut kw = klock::witness();
    while let Some(q) = fd.fd_fbufs_wait.first(&kw).map(adopt) {
        fd.fd_fbufs_wait.remove_first(&mut kw);
        // DPRINTF("umount unprocessed msg in sc_fbufs_wait\n");
        t.stat_fbufs_wait.fetch_sub(1, Ordering::Relaxed);
        fuse_device_fail(&q);
        // The queue's reference goes: a request nobody waits for is freed.
        drop(q);
    }
}

/// `fuse_device_cleanup`: cleanup all msgs from `fd_fbufs_in` and `fd_fbufs_wait`.
pub fn fuse_device_cleanup<G: ProcGlue>(dev: Dev) {
    if let Some(fd) = fuse_lookup::<G>(dev) {
        fuse_device_cleanup_fd(&fd);
    }
}

/// `fuse_device_queue_fbuf`: puts a request on the device's input queue, which keeps the
/// reference `fbuf` until the request is taken off, and lets the daemon know. Nothing happens
/// when the device is not open (the reference goes).
pub fn fuse_device_queue_fbuf<G: ProcGlue>(dev: Dev, fbuf: FusebufRef) {
    let Some(fd) = fuse_lookup::<G>(dev) else {
        return;
    };

    let mut w = fd.fd_lock.write();
    fd.fd_fbufs_in.insert_tail(&mut w, fbuf.as_pin());
    PoolArc::forget(fbuf);
    fd.fd_nin.set(fd.fd_nin.get() + 1);
    fd.fd_rklist.knote_locked(&w, 0);
    drop(w);
    tables::<G>().stat_fbufs_in.fetch_add(1, Ordering::Relaxed);

    // Let file system daemons know there is a request ready to process
    wakeup_one(ptr::from_ref(&fd.fd_fbufs_in));
}

/// `fuse_device_set_fmp`'s body on the device.
fn fuse_device_set_fmp_fd<G: ProcGlue>(fd: &FuseD<G>, fmp: Option<FusefsMntRef<G>>) {
    let set = fmp.is_some();
    drop(fd.fd_fmp.replace(fmp));
    if !set {
        // Let file system daemons know the device is dead
        wakeup(ptr::from_ref(&fd.fd_fbufs_in));
    }
}

/// `fuse_device_set_fmp`: attaches the mount to its device (`set`) or detaches it.
pub fn fuse_device_set_fmp<G: ProcGlue>(fmp: &FusefsMntRef<G>, set: bool) {
    if let Some(fd) = fuse_lookup::<G>(fmp.dev) {
        fuse_device_set_fmp_fd(&fd, set.then(|| fmp.clone()));
    }
}

/// `fuseattach`: the pseudo-device's attach function. The C's `LIST_INIT(&fuse_d_list)` has
/// nothing to do here: `FuseTables` makes the list empty with the glue's tables.
pub fn fuseattach<G: ProcGlue>(_num: i32) {}

/// `fuseopen`: makes the device of the minor; one open per minor (the device clones).
pub fn fuseopen<G: ProcGlue>(dev: Dev, flags: i32, _fmt: i32, _p: &Thread<G>) -> Result<(), Errno> {
    let unit = minor(dev) as i32;

    if flags & O_EXCL != 0 {
        return Err(Errno::EBUSY); // No exclusive opens
    }

    if fuse_lookup::<G>(dev).is_some() {
        return Err(Errno::EBUSY);
    }

    let Some(fd) = MArc::new(M_DEVBUF, M_WAITOK, FuseD::new(dev, unit)) else {
        kpanic(format_args!("fuseopen: malloc(M_WAITOK) failed"));
    };
    drop(fd.fd_self.replace(Some(fd.clone())));
    {
        let mut w = klock::witness();
        tables::<G>().devices.insert_head(&mut w, fd.as_pin());
    }

    tables::<G>()
        .stat_opened_fusedev
        .fetch_add(1, Ordering::Relaxed);
    Ok(())
}

/// `fuseclose`: fails the pending requests, detaches the mount and lets the device go.
pub fn fuseclose<G: ProcGlue>(
    dev: Dev,
    _flags: i32,
    _fmt: i32,
    _p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let Some(fd) = fuse_lookup::<G>(dev) else {
        return Err(Errno::EBADF);
    };

    fuse_device_cleanup_fd(&fd);

    // Let fusefs_unmount know the device is closed so it doesn't try and send FBT_DESTROY
    // to a dead file system daemon.
    let fmp = fd.fd_fmp.read().clone();
    if let Some(fmp) = fmp {
        fmp.sess_init.set(0);
        fuse_device_set_fmp_fd(&fd, None);
    }

    {
        let mut w = klock::witness();
        tables::<G>().devices.remove(&mut w, &fd);
    }
    drop(fd.fd_self.replace(None));
    tables::<G>()
        .stat_opened_fusedev
        .fetch_sub(1, Ordering::Relaxed);
    Ok(())
}

/// `fuseread`: the daemon reads the oldest request: its header, its input structure and its
/// data, all or nothing. Sleeps for one unless `IO_NDELAY`.
pub fn fuseread<G: ProcGlue>(dev: Dev, uio: &mut Uio<'_, G>, ioflag: i32) -> Result<(), Errno> {
    let Some(fd) = fuse_lookup::<G>(dev) else {
        return Err(Errno::ENODEV);
    };

    if !fd.has_fmp() {
        return Err(Errno::ENODEV);
    }

    let mut w = fd.fd_lock.write();

    // Loop to avoid a race condition with multithreaded daemons.
    while fd.fd_fbufs_in.is_empty(&w) {
        if ioflag & IO_NDELAY != 0 {
            return Err(Errno::EAGAIN);
        }

        let error = w.sleep_nsec(
            ptr::from_ref(&fd.fd_fbufs_in),
            PWAIT | PCATCH,
            "fusedr",
            INFSLP,
        );

        // check for unmount during sleep
        if !fd.has_fmp() {
            return Err(Errno::ENODEV);
        }
        if matches!(error, Err(Errno::EINTR | Errno::ERESTART)) {
            return Err(Errno::EINTR);
        }
    }
    // The queue's reference, taken back while the copies run (fd_lock keeps the request
    // first: every change of the queue takes it), parked again if they fail.
    let Some(fbuf) = fd.fd_fbufs_in.first(&w).map(adopt) else {
        kpanic(format_args!("fuseread: the queue emptied under fd_lock"));
    };

    // We get the whole fusebuf or nothing
    if (uio.uio_resid as u64) < (size_of::<FuseInHeader>() + fbuf.op_in_len) as u64 + fbuf.fb_len()
    {
        PoolArc::forget(fbuf);
        return Err(Errno::EINVAL);
    }

    if let Err(e) = fuseread_copy(&fbuf, uio) {
        PoolArc::forget(fbuf);
        return Err(e);
    }

    // Move the fbuf to the wait queue
    fd.fd_fbufs_in.remove_first(&mut w);
    fd.fd_nin.set(fd.fd_nin.get() - 1);
    let t = tables::<G>();
    t.stat_fbufs_in.fetch_sub(1, Ordering::Relaxed);

    // FUSE_FORGET has no response
    if fbuf.fb_type() == FUSE_FORGET {
        drop(fbuf);
        return Ok(());
    }

    {
        let mut kw = klock::witness();
        fd.fd_fbufs_wait.insert_tail(&mut kw, fbuf.as_pin());
    }
    PoolArc::forget(fbuf);
    t.stat_fbufs_wait.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

/// `fuseread`'s copies: the header, the input structure and the data (freed once copied, as
/// the C's `free(fbuf->fb_dat, ..)`; kept by the request when its copy fails).
fn fuseread_copy<G: ProcGlue>(fbuf: &Fusebuf, uio: &mut Uio<'_, G>) -> Result<(), Errno> {
    let mut hdr = fbuf.hdr;
    uiomove(bytes_of_mut(core::slice::from_mut(&mut hdr)), uio)?;
    let mut op = [0u8; FUSEBUF_OP_SIZE];
    let op = &mut op[..fbuf.op_in_len];
    fbuf.op.read_bytes(op);
    uiomove(op, uio)?;
    if fbuf.fb_len() > 0 {
        let mut dat = fbuf.take_dat();
        if let Err(e) = uiomove(dat.as_deref_mut().unwrap_or_default(), uio) {
            fbuf.set_dat(dat);
            return Err(e);
        }
    }
    fbuf.set_dat(None);
    Ok(())
}

/// `fusewrite`: the daemon answers a request: a `struct fuse_out_header` with the request's
/// ID, then the operation's output structure and its data. Wakes the requester (a
/// `FUSE_INIT` reply has none: it starts the session).
pub fn fusewrite<G: ProcGlue>(dev: Dev, uio: &mut Uio<'_, G>, _ioflag: i32) -> Result<(), Errno> {
    let Some(fd) = fuse_lookup::<G>(dev) else {
        return Err(Errno::ENODEV);
    };

    // Check for sanity - must receive at least the header
    if uio.uio_resid < size_of::<FuseOutHeader>() {
        return Err(Errno::EINVAL);
    }

    // Read the header
    let mut hdr = FuseOutHeader::default();
    uiomove(bytes_of_mut(core::slice::from_mut(&mut hdr)), uio)?;

    // A unique value of zero means daemon is notifying us and hdr.error contains
    // notification type. Currently unsupported.
    if hdr.unique == 0 {
        return Ok(());
    }

    // looking for uuid in fd_fbufs_wait
    let found = {
        let r = klock::reader();
        fd.fd_fbufs_wait
            .iter(&r)
            .find(|f| f.fb_uuid() == hdr.unique)
            .and_then(PoolArc::from_ref)
    };
    let Some(fbuf) = found else {
        return Err(Errno::ENOENT);
    };

    let error = fusewrite_reply(&fd, &fbuf, &hdr, uio);

    // Remove the fbuf from the wait queue
    let queued = {
        let mut kw = klock::witness();
        fbuf.fb_link.is_linked().then(|| {
            fd.fd_fbufs_wait.remove(&mut kw, &fbuf);
            adopt(&fbuf)
        })
    };
    if let Some(q) = queued {
        tables::<G>()
            .stat_fbufs_wait
            .fetch_sub(1, Ordering::Relaxed);

        // FBT_INIT doesn't expect a response. Otherwise let the VFS syscall that is
        // waiting on this fbuf know the reponse is ready.
        if fbuf.fb_type() != FUSE_INIT {
            wakeup(PoolArc::as_ptr(&q));
        }
    }

    error
}

/// The part of `fusewrite` between finding the request and its `end:` label: fills the
/// fusebuf from the reply.
fn fusewrite_reply<G: ProcGlue>(
    fd: &FuseD<G>,
    fbuf: &Fusebuf,
    hdr: &FuseOutHeader,
    uio: &mut Uio<'_, G>,
) -> Result<(), Errno> {
    // Update fb_hdr
    fbuf.set_fb_err(hdr.error.wrapping_neg());

    // Don't expect out struct or data if there was an error
    if fbuf.fb_err() != 0 {
        if uio.uio_resid > 0 {
            fbuf.set_fb_err(Errno::EIO as i32);
            return Err(Errno::EINVAL);
        }
        return Ok(());
    }

    // get operation output
    let op_out_len = fbuf.op_out_len;
    if op_out_len > 0 {
        // A reply shorter than the output structure would leave the request's own input
        // bytes in the union, read as the answer (the module's deviations, D-N5.9-U2).
        if uio.uio_resid < op_out_len {
            fbuf.set_fb_err(Errno::EIO as i32);
            return Err(Errno::EINVAL);
        }
        let mut op = [0u8; FUSEBUF_OP_SIZE];
        let op = &mut op[..op_out_len];
        if let Err(e) = uiomove(op, uio) {
            fbuf.set_fb_err(e as i32);
            return Err(e);
        }
        fbuf.op.write_bytes(op);
    }

    // Calculate the length of the data buffer to expect
    if fbuf.op_out_buf {
        let len = (hdr.len as usize)
            .wrapping_sub(size_of::<FuseOutHeader>())
            .wrapping_sub(op_out_len) as u64;
        fbuf.set_fb_len(len);
        let Some(max_read) = fd.fd_fmp.read().as_ref().map(|fmp| fmp.max_read) else {
            fbuf.set_fb_err(Errno::ENXIO as i32);
            return Err(Errno::ENODEV);
        };
        if len > max_read as u64 {
            // DPRINTF("invalid fusebuf read size: %llu opcode=%d\n", fb_len, fb_type);
            fbuf.set_fb_err(Errno::EIO as i32);
            return Err(Errno::EINVAL);
        }
    } else {
        fbuf.set_fb_len(0);
    }

    // validate remaining data
    if uio.uio_resid as u64 != fbuf.fb_len() {
        fbuf.set_fb_err(Errno::EIO as i32);
        return Err(Errno::EINVAL);
    }

    if fbuf.fb_len() > 0 {
        let len = fbuf.fb_len() as usize;
        let Some(mut dat) = MArray::new(M_FUSEFS, M_WAITOK, len, |_| 0u8) else {
            kpanic(format_args!("fusewrite: malloc(M_WAITOK) failed"));
        };
        if let Err(e) = uiomove(&mut dat, uio) {
            drop(dat);
            fbuf.set_fb_err(e as i32);
            return Err(e);
        }
        fbuf.set_dat(Some(dat));
    }

    if fbuf.fb_type() == FUSE_INIT && fbuf.fb_err() == 0 {
        let init: FuseInitOut = fbuf.op_get();
        // We don't support userspace with a smaller major version and it's up to userspace
        // implementations to fall back to our version if they are capable of a later
        // version.
        if init.major != FUSE_KERNEL_VERSION {
            // DPRINTF("unsupported major version: %d.%d\n", major, minor);
            return Err(Errno::EINVAL);
        }
        // If the major versions match then both shall use the smallest of the two minor
        // versions for communication. 7.9 is the smallest version less than what we support
        // where the ABI has not changed. Supporting an earlier version would require
        // conditional handling of some FUSE input arguments. If the daemon supports a later
        // version then it must fall back to ours.
        if init.minor < 9 {
            // DPRINTF("unsupported minor version: %d.%d\n", major, minor);
            return Err(Errno::EINVAL);
        }
        let fmp = fd.fd_fmp.read();
        let Some(fmp) = fmp.as_ref() else {
            return Err(Errno::ENODEV);
        };
        // max_write determines the size of buffer to send to the file system daemon when
        // writing so ensure that it's sane.
        let max_write = (init.max_write as usize).min(FUSEBUFMAXSIZE) as i32;
        fmp.max_write.set(if max_write == 0 {
            FUSEBUFMAXSIZE as i32
        } else {
            max_write
        });
        fmp.sess_init.set(1);
    }

    Ok(())
}

/// `fusekqfilter`: `EVFILT_READ` fires while a request waits to be read; the device is
/// always writable.
pub fn fusekqfilter<G: ProcGlue>(dev: Dev, kn: &Knote<G>) -> Result<(), Errno> {
    let Some(fd) = fuse_lookup::<G>(dev) else {
        return Err(Errno::EINVAL);
    };

    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&FuseFiltops::<G>::READ)),
        EVFILT_WRITE => return seltrue_kqfilter(kn),
        _ => return Err(Errno::EINVAL),
    }

    // kn->kn_hook = fd
    if kn.set_hook_shared(MArc::erase(fd.clone())).is_err() {
        kpanic(format_args!("fusekqfilter: knote {kn:p} has a hook"));
    }

    let mut w = fd.fd_lock.write();
    fd.fd_rklist.insert_locked(&mut w, kn);
    Ok(())
}

/// `kn->kn_hook` of a fuse(4) knote: its device.
fn kn_fused<G: ProcGlue>(kn: &Knote<G>) -> &FuseD<G> {
    match kn.hook_shared::<FuseD<G>>() {
        Some(fd) => fd,
        None => kpanic(format_args!("knote {kn:p}: no fuse device")),
    }
}

impl<G: ProcGlue> Filter<G> for FuseRead {
    const FLAGS: i32 = FILTEROP_ISFD | FILTEROP_MPSAFE;

    /// `filt_fuse_rdetach`.
    fn detach(kn: &Knote<G>) {
        let fd = kn_fused(kn);
        let mut w = fd.fd_lock.write();
        fd.fd_rklist.remove_locked(&mut w, kn);
    }

    /// `filt_fuse_read`: a request is waiting to be read.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let fd = kn_fused(kn);

        kassert!(fd.fd_lock.status() == RW_WRITE);

        fd.fd_nin.get() != 0
    }

    /// `filt_fuse_modify`.
    fn modify(kev: &mut Kevent, kn: &Knote<G>) -> bool {
        let fd = kn_fused(kn);
        let _w = fd.fd_lock.write();
        knote_modify(kev, kn)
    }

    /// `filt_fuse_process`.
    fn process(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
        let fd = kn_fused(kn);
        let _w = fd.fd_lock.write();
        knote_process(kn, kev)
    }
}

/// `fuse_test_reset`'s part: the counters of a test thread's tables back to 0. The device
/// list is not walked (its elements may live in an earlier test's memory, testing.md): every
/// test closes the devices it opens, which takes them off.
#[cfg(any(test, feature = "doubles"))]
pub(crate) fn fuse_device_test_reset<G: ProcGlue>() {
    let t = tables::<G>();
    t.stat_opened_fusedev.store(0, Ordering::Relaxed);
    t.stat_fbufs_in.store(0, Ordering::Relaxed);
    t.stat_fbufs_wait.store(0, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the fuse(4) device (LZ's moved, over the fixture's daemon side): opens
    // and closes, a request queued by the kernel read back by the daemon in libfuse's layout
    // (header, input structure, data), replies (data, errors, bad lengths, unknown IDs,
    // notifications, one shorter than the output structure: D-N5.9-U2), `FUSE_FORGET` and
    // `FUSE_INIT` (and their buffers freed once nobody needs them), `fb_queue` returning the
    // daemon's answer, the cleanup of both queues, close detaching the mount and the device
    // outliving its close while a handle holds it, and the read filter over a kqueue.

    use std::vec::Vec;
    use std::{assert, assert_eq};

    use hal::sys::fcntl::FREAD;
    use hal::user::bytes_of;

    use super::*;
    use crate::event::{EVFILT_VNODE, kqueue_alloc, kqueue_init, test_knote, test_knote_drop};
    use crate::fs::fuse::fusebuf::{fb_delete, fb_queue, fb_setup};
    use crate::fs::fuse::fusebuf_h::{
        FUSE_GETATTR, FUSE_LOOKUP, FUSE_OPEN, FUSE_READ, FuseAttrOut, FuseEntryOut, FuseForgetIn,
        FuseGetattrIn, FuseInitIn, FuseOpenOut, FuseReadIn,
    };
    use crate::fs::fuse::testfuse::{
        PENDING, dev_read, dev_write, fusedev, mount_fmp, open_dev, read_req, reply, set_daemon,
        setup, stats,
    };
    use crate::fs::testfs::TestGlue;
    use crate::proc::THREAD_PID_OFFSET;

    type G = TestGlue;

    /// The `"fmsg"` items out of the pool now.
    fn fmsg_out() -> u64 {
        vfs::<G>().fuse.fbuf_pool.nout()
    }

    /// The bytes of a protocol structure.
    fn b<T: hal::user::AbiPod>(v: &T) -> &[u8] {
        bytes_of(core::slice::from_ref(v))
    }

    #[test]
    fn open_is_exclusive_per_minor_and_close_lets_the_device_go() {
        let (_g, p) = setup();
        let dev0 = open_dev(p, 0);
        assert_eq!(stats().0, 1);
        assert_eq!(fuseopen::<G>(dev0, FREAD, 0, p), Err(Errno::EBUSY));
        assert_eq!(
            fuseopen::<G>(fusedev(1), FREAD | O_EXCL, 0, p),
            Err(Errno::EBUSY),
            "no exclusive opens"
        );
        let dev1 = open_dev(p, 1 << 8); // a clone's minor
        assert_eq!(stats().0, 2);
        let held = fuse_lookup::<G>(dev1).expect("open");
        assert_eq!(held.fd_unit, 1 << 8);
        assert_eq!(MArc::ty(&held), M_DEVBUF);

        fuseclose::<G>(dev0, FREAD, 0, Some(p)).expect("close");
        fuseclose::<G>(dev1, FREAD, 0, Some(p)).expect("close");
        assert_eq!(stats().0, 0);
        assert_eq!(fuseclose::<G>(dev0, FREAD, 0, Some(p)), Err(Errno::EBADF));
        assert_eq!(dev_read(dev0, &mut [0u8; 1], 0), Err(Errno::ENODEV));
        // The closed device is off the list, alive while a handle holds it (no
        // `refcnt_finalize` wait: the block goes with its last handle).
        assert!(fuse_lookup::<G>(dev1).is_none());
        assert!(!held.fd_link.is_linked());
        assert!(held.fd_self.read().is_none());
        drop(held);
    }

    #[test]
    fn read_needs_a_mount_and_honours_ndelay() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let mut buf = [0u8; 256];
        assert_eq!(dev_read(dev, &mut buf, IO_NDELAY), Err(Errno::ENODEV));
        let fmp = mount_fmp(dev, FUSEBUFMAXSIZE as i32);
        assert_eq!(dev_read(dev, &mut buf, IO_NDELAY), Err(Errno::EAGAIN));
        fuseclose::<G>(dev, 0, 0, Some(p)).expect("close");
        assert_eq!(
            fmp.sess_init.get(),
            0,
            "close tells the mount the daemon is gone"
        );
    }

    #[test]
    fn a_queued_request_is_read_in_libfuse_layout_and_answered() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let _fmp = mount_fmp(dev, FUSEBUFMAXSIZE as i32);
        let out0 = fmsg_out();

        let fbuf = fb_setup(6, 7, FUSE_LOOKUP, p);
        fbuf.dat_update(|d| d.copy_from_slice(b"hello\0"));
        fuse_device_queue_fbuf::<G>(dev, fbuf.clone());
        assert_eq!(stats().1, 1);

        // We get the whole fusebuf or nothing.
        let mut small = [0u8; 45];
        assert_eq!(dev_read(dev, &mut small, IO_NDELAY), Err(Errno::EINVAL));
        assert_eq!(stats().1, 1);

        let req = read_req(dev).expect("a request");
        assert_eq!(req.hdr.len, 46);
        assert_eq!(req.hdr.opcode, FUSE_LOOKUP);
        assert_eq!(req.hdr.unique, fbuf.fb_uuid());
        assert_eq!(req.hdr.nodeid, 7);
        assert_eq!(req.hdr.uid, p.ucred().cr_uid.get());
        assert_eq!(req.hdr.gid, p.ucred().cr_gid.get());
        assert_eq!(req.hdr.pid, (p.p_tid.get() + THREAD_PID_OFFSET) as u32);
        assert!(req.op.is_empty());
        assert_eq!(req.data, b"hello\0");
        assert!(fbuf.take_dat().is_none(), "the data is freed once read");
        assert_eq!(stats(), (1, 0, 1));
        assert!(read_req(dev).is_none());

        let mut entry = FuseEntryOut {
            nodeid: 9,
            ..FuseEntryOut::default()
        };
        entry.attr.mode = 0o100644;
        entry.attr.size = 13;
        reply(dev, req.hdr.unique, 0, b(&entry), &[]).expect("reply");
        assert_eq!(fbuf.fb_err(), 0);
        assert_eq!(fbuf.op_get::<FuseEntryOut>(), entry);
        assert_eq!(fbuf.fb_len(), 0);
        assert_eq!(stats().2, 0);
        // The reply's ID is not waited for any more.
        assert_eq!(
            reply(dev, req.hdr.unique, 0, b(&entry), &[]),
            Err(Errno::ENOENT)
        );
        fb_delete(fbuf);
        assert_eq!(fmsg_out(), out0, "the last handle put the item back");
        fuseclose::<G>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn replies_with_errors_bad_lengths_and_notifications() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let _fmp = mount_fmp(dev, 8);

        // Too short for a header; a notification (unique 0) is accepted and ignored.
        assert_eq!(dev_write(dev, &[&[0u8; 15]]), Err(Errno::EINVAL));
        reply(dev, 0, -1, &[], &[]).expect("notification");

        // An error reply carries nothing else.
        let a = fb_setup(0, 1, FUSE_GETATTR, p);
        let bb = fb_setup(0, 1, FUSE_GETATTR, p);
        fuse_device_queue_fbuf::<G>(dev, a.clone());
        fuse_device_queue_fbuf::<G>(dev, bb.clone());
        let ra = read_req(dev).expect("a");
        let rb = read_req(dev).expect("b");
        assert_eq!(ra.op.len(), size_of::<FuseGetattrIn>());
        // b is answered first: a is then the element before it on the wait queue.
        reply(dev, rb.hdr.unique, -(Errno::ENOENT as i32), &[], &[]).expect("error reply");
        assert_eq!(bb.fb_err(), Errno::ENOENT as i32);
        assert_eq!(
            reply(dev, ra.hdr.unique, -(Errno::EACCES as i32), &[], &[1, 2]),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            a.fb_err(),
            Errno::EIO as i32,
            "an error reply with data fails"
        );
        assert_eq!(stats().2, 0);
        fb_delete(a);
        fb_delete(bb);

        // A data reply: its length comes from hdr.len, bounded by max_read (8 here).
        let r = fb_setup(0, 2, FUSE_READ, p);
        r.op_set(&FuseReadIn {
            size: 8,
            ..FuseReadIn::default()
        });
        fuse_device_queue_fbuf::<G>(dev, r.clone());
        let rr = read_req(dev).expect("read");
        assert_eq!(rr.op::<FuseReadIn>().size, 8);
        reply(dev, rr.hdr.unique, 0, &[], b"abcdef").expect("data");
        assert_eq!(r.fb_len(), 6);
        r.dat_with(|d| assert_eq!(d, b"abcdef"));
        fb_delete(r);

        let r = fb_setup(0, 2, FUSE_READ, p);
        fuse_device_queue_fbuf::<G>(dev, r.clone());
        let rr = read_req(dev).expect("read");
        assert_eq!(
            reply(dev, rr.hdr.unique, 0, &[], b"123456789"),
            Err(Errno::EINVAL)
        );
        assert_eq!(r.fb_err(), Errno::EIO as i32, "more than max_read");
        fb_delete(r);

        // hdr.len and the bytes written disagree.
        let o = fb_setup(0, 2, FUSE_OPEN, p);
        fuse_device_queue_fbuf::<G>(dev, o.clone());
        let ro = read_req(dev).expect("open");
        let hdr = FuseOutHeader {
            len: 16,
            error: 0,
            unique: ro.hdr.unique,
        };
        let out = FuseOpenOut::default();
        assert_eq!(
            dev_write(dev, &[b(&hdr), b(&out), &[0]]),
            Err(Errno::EINVAL)
        );
        assert_eq!(o.fb_err(), Errno::EIO as i32);
        fb_delete(o);

        fuseclose::<G>(dev, 0, 0, Some(p)).expect("close");
    }

    /// D-N5.9-U2's third bound: a reply whose bytes after the header are fewer than the
    /// operation's output structure is refused `EINVAL` and the requester gets `EIO`; the
    /// union keeps the request's own input bytes, which the C would read as the answer.
    #[test]
    fn a_reply_shorter_than_the_output_structure_is_refused() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let _fmp = mount_fmp(dev, FUSEBUFMAXSIZE as i32);

        let g = fb_setup(0, 3, FUSE_GETATTR, p);
        g.op_set(&FuseGetattrIn {
            getattr_flags: 0x1234,
            dummy: 0,
            fh: 0x55,
        });
        fuse_device_queue_fbuf::<G>(dev, g.clone());
        let rg = read_req(dev).expect("getattr");
        let short = [0xffu8; 50]; // a FuseAttrOut is 104 bytes
        assert_eq!(
            reply(dev, rg.hdr.unique, 0, &short, &[]),
            Err(Errno::EINVAL)
        );
        assert_eq!(g.fb_err(), Errno::EIO as i32);
        assert_eq!(stats().2, 0, "taken off the wait queue all the same");
        let left: FuseGetattrIn = g.op_get();
        assert_eq!((left.getattr_flags, left.fh), (0x1234, 0x55));
        fb_delete(g);

        // A whole structure is taken.
        let g = fb_setup(0, 3, FUSE_GETATTR, p);
        fuse_device_queue_fbuf::<G>(dev, g.clone());
        let rg = read_req(dev).expect("getattr");
        let mut attr = FuseAttrOut::default();
        attr.attr.size = 99;
        reply(dev, rg.hdr.unique, 0, b(&attr), &[]).expect("reply");
        assert_eq!((g.fb_err(), g.op_get::<FuseAttrOut>()), (0, attr));
        fb_delete(g);

        fuseclose::<G>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn forget_and_init_need_no_requester() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let fmp = mount_fmp(dev, FUSEBUFMAXSIZE as i32);
        let out0 = fmsg_out();

        let f = fb_setup(0, 5, FUSE_FORGET, p);
        f.op_set(&FuseForgetIn { nlookup: 3 });
        fuse_device_queue_fbuf::<G>(dev, f);
        let rf = read_req(dev).expect("forget");
        assert_eq!(rf.hdr.opcode, FUSE_FORGET);
        assert_eq!(rf.op::<FuseForgetIn>().nlookup, 3);
        assert_eq!(stats().2, 0, "no reply is expected");
        assert_eq!(fmsg_out(), out0, "the forget is freed once read");

        // A daemon of an older major version is refused: the session stays pending.
        let i = fb_setup(0, 0, FUSE_INIT, p);
        i.op_set(&FuseInitIn {
            major: FUSE_KERNEL_VERSION,
            minor: 19,
            ..FuseInitIn::default()
        });
        fuse_device_queue_fbuf::<G>(dev, i);
        let ri = read_req(dev).expect("init");
        assert_eq!(ri.op::<FuseInitIn>().major, 7);
        let old = FuseInitOut {
            major: 6,
            minor: 30,
            ..FuseInitOut::default()
        };
        assert_eq!(
            reply(dev, ri.hdr.unique, 0, b(&old), &[]),
            Err(Errno::EINVAL)
        );
        assert_eq!(fmp.sess_init.get(), PENDING);
        assert_eq!(fmsg_out(), out0, "the init is freed with its reply");

        let i = fb_setup(0, 0, FUSE_INIT, p);
        fuse_device_queue_fbuf::<G>(dev, i);
        let ri = read_req(dev).expect("init");
        let ok = FuseInitOut {
            major: 7,
            minor: 26,
            max_write: 0,
            ..FuseInitOut::default()
        };
        reply(dev, ri.hdr.unique, 0, b(&ok), &[]).expect("init reply");
        assert_eq!(fmp.sess_init.get(), 1);
        assert_eq!(
            fmp.max_write.get(),
            FUSEBUFMAXSIZE as i32,
            "0 means the most"
        );
        assert_eq!(stats().2, 0);
        assert_eq!(fmsg_out(), out0);

        // A request for a device that is not open goes nowhere and is freed.
        let f = fb_setup(0, 5, FUSE_FORGET, p);
        fuse_device_queue_fbuf::<G>(fusedev(3), f);
        assert_eq!((stats().1, fmsg_out()), (0, out0));

        fuseclose::<G>(dev, 0, 0, Some(p)).expect("close");
    }

    /// A simulated daemon answering every queued request with `FuseOpenOut { fh: 77 }`, or
    /// with `EACCES` for `FUSE_GETATTR`.
    fn open_daemon(dev: Dev) {
        while let Some(req) = read_req(dev) {
            if req.hdr.opcode == FUSE_GETATTR {
                reply(dev, req.hdr.unique, -(Errno::EACCES as i32), &[], &[]).expect("reply");
            } else {
                let out = FuseOpenOut {
                    fh: 77,
                    ..FuseOpenOut::default()
                };
                reply(dev, req.hdr.unique, 0, b(&out), &[]).expect("reply");
            }
        }
    }

    #[test]
    fn fb_queue_returns_the_daemons_answer() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let _fmp = mount_fmp(dev, FUSEBUFMAXSIZE as i32);
        set_daemon(Some(open_daemon));

        let o = fb_setup(0, 3, FUSE_OPEN, p);
        assert_eq!(fb_queue::<G>(dev, &o), Ok(()));
        assert_eq!(o.op_get::<FuseOpenOut>().fh, 77);
        fb_delete(o);

        let g = fb_setup(0, 3, FUSE_GETATTR, p);
        assert_eq!(fb_queue::<G>(dev, &g), Err(Errno::EACCES));
        assert_eq!(g.op_get::<FuseAttrOut>(), FuseAttrOut::default());
        fb_delete(g);
        assert_eq!(stats(), (1, 0, 0));

        set_daemon(None);
        fuseclose::<G>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn cleanup_fails_every_queued_request() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let _fmp = mount_fmp(dev, FUSEBUFMAXSIZE as i32);
        let out0 = fmsg_out();

        let fbufs: Vec<FusebufRef> = (0..5).map(|i| fb_setup(0, i, FUSE_GETATTR, p)).collect();
        for f in &fbufs {
            fuse_device_queue_fbuf::<G>(dev, f.clone());
        }
        // Two of them read: on the wait queue.
        read_req(dev).expect("one");
        read_req(dev).expect("two");
        // A forget nobody waits for, unread.
        fuse_device_queue_fbuf::<G>(dev, fb_setup(0, 9, FUSE_FORGET, p));
        assert_eq!(stats(), (1, 4, 2));
        let fd = fuse_lookup::<G>(dev).expect("open");
        fuse_device_cleanup::<G>(dev);
        for f in &fbufs {
            assert_eq!(f.fb_err(), Errno::ENXIO as i32);
        }
        assert_eq!(stats(), (1, 0, 0));
        {
            let w = fd.fd_lock.write();
            assert!(fd.fd_fbufs_in.is_empty(&w));
            assert_eq!(fd.fd_nin.get(), 0);
        }
        assert!(fd.fd_fbufs_wait.is_empty(&klock::reader()));
        drop(fd);
        assert!(read_req(dev).is_none());
        drop(fbufs);
        assert_eq!(fmsg_out(), out0, "the forget was freed by the cleanup");
        fuseclose::<G>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn the_read_filter_fires_while_a_request_waits() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let _fmp = mount_fmp(dev, FUSEBUFMAXSIZE as i32);
        let _ = kqueue_init::<G>();
        let kq = kqueue_alloc::<G>(p.process().fd_ref()).expect("kqueue");

        let h = test_knote(&kq, 1);
        let kn = h.kn();
        kn.kn_kevent.filter.set(EVFILT_READ);
        fusekqfilter::<G>(dev, kn).expect("kqfilter");
        assert_eq!(kn.fop().f_flags, FILTEROP_ISFD | FILTEROP_MPSAFE);
        let fd = fuse_lookup::<G>(dev).expect("open");
        assert!(core::ptr::eq(kn_fused(kn), &*fd), "the hook is the device");
        let event = |kn: &Knote<G>| {
            let _w = fd.fd_lock.write();
            (kn.fop().f_event.unwrap())(kn, 0)
        };
        assert!(!event(kn));
        let f = fb_setup(0, 1, FUSE_GETATTR, p);
        fuse_device_queue_fbuf::<G>(dev, f.clone());
        assert!(event(kn), "a request waits");
        read_req(dev).expect("the request");
        assert!(!event(kn));
        (kn.fop().f_detach.unwrap())(kn);
        assert!(fd.fd_rklist.is_empty());
        reply(dev, f.fb_uuid(), 0, b(&FuseAttrOut::default()), &[]).expect("reply");
        fb_delete(f);
        test_knote_drop(&kq, h);

        let h = test_knote(&kq, 2);
        h.kn().kn_kevent.filter.set(EVFILT_WRITE);
        fusekqfilter::<G>(dev, h.kn()).expect("write filter: always true");
        test_knote_drop(&kq, h);
        let h = test_knote(&kq, 3);
        h.kn().kn_kevent.filter.set(EVFILT_VNODE);
        assert_eq!(fusekqfilter::<G>(dev, h.kn()), Err(Errno::EINVAL));
        test_knote_drop(&kq, h);
        let h = test_knote(&kq, 4);
        assert_eq!(fusekqfilter::<G>(fusedev(4), h.kn()), Err(Errno::EINVAL));
        test_knote_drop(&kq, h);

        drop(fd);
        drop(kq);
        fuseclose::<G>(dev, 0, 0, Some(p)).expect("close");
    }
}
/* </TESTS> */
