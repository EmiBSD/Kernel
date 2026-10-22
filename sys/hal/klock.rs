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
/* </LICENSES> */

/* <CODE> */
//! The kernel lock (`KERNEL_LOCK()`, `KERNEL_UNLOCK()`, `KERNEL_ASSERT_LOCKED()` of
//! `<sys/systm.h>`), as the kernel crate needs it while the big lock is `bsd`'s.
//!
//! The lock itself is `bsd`'s `__mp_lock` (`sys/kern/kern_lock.rs`), reached through the
//! [`KernelLock`] bridge, registered by `bsd` at boot (bridges.toml; paid by N4, when
//! `kern_lock` moves). The vnode pager (`kernel::vm::vnode`) takes it around the vnode layer,
//! and the object functions that the buffer cache calls under it assert it.
//!
//! **Before registration** (the boot CPU before `main()` registers the bridges, the host
//! tests that register none): one CPU runs and nothing else could hold the lock; taking and
//! releasing it do nothing, and the assertion holds.
//!
//! **Witnesses** (N4.2). The C keeps the process lists, the pid, tid and pgrp hashes and the
//! parent, child and group links under the kernel lock (`[K]` in `<sys/proc.h>`). The
//! kernel's cells and lists of that state are keyed to the kernel lock's identity
//! (`Guarded::new_kernel`, the pinned lists bound to it) and open through a witness of it, as
//! a `Mutexed`'s through its guard: [`witness`] gives the one exclusive [`KernelWitness`]
//! (changes), [`reader`] a shared [`KernelReader`] (walks, lookups). The big lock is
//! recursive, is taken by interrupt handlers on the CPU that holds it, and is released by every
//! sleep, so a witness is more than "the lock is held":
//! - it is made only while the running CPU holds the kernel lock ([`KernelLock::held`]);
//! - per CPU, readers nest and the writer is alone: a witness asked for against the other
//!   kind already out on this CPU (a handler that interrupted its holder, a callee that takes
//!   its own) is `panic(9)`, as a `RefCell` refuses a second borrow; a writer raises the
//!   interrupt level to `IPL_VM`, above every handler that walks the lists (`ttyinput`'s
//!   `pgsignal` runs at `IPL_TTY` at most), so none of them meets one;
//! - the lock is not released while a witness is out on this CPU: `bsd`'s release paths (the
//!   switch's `__mp_release_all`, the last `KERNEL_UNLOCK`) call [`assert_unwitnessed`], the
//!   bridge's contract, so a holder never sleeps with one.
//!
//! So while a witness lives no other CPU holds the lock, and on this one only witnesses of
//! the same kind are out: the cells it opens see one writer or readers, never both.
//!
//! ## Redesign
//! - Not a port: the C calls `_kernel_lock()` and its kin directly; the kernel crate reaches
//!   them through one trait object until the lock moves in (docs/ZERO_UNSAFE.md section 0.3.2).
//! - N4.2: the C's `KERNEL_ASSERT_LOCKED()` in front of the `[K]` state becomes a witness the
//!   access needs ([`KernelWitness`], [`KernelReader`]); `KernelLock` gains `held` (the C's
//!   `_kernel_lock_held()`), which the witness checks always, not only under `DIAGNOSTIC`.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

use crate::bridges;
use crate::intr::{IPL_VM, SplGuard, splraise};
use crate::percpu::PerCpu;
use crate::rwlock::LockId;

/// The kernel lock, as `bsd` registers it (one call each into `kern_lock.rs`).
pub trait KernelLock: Sync {
    /// `KERNEL_LOCK()`: takes the kernel lock (recursively, as `__mp_lock` does).
    fn lock(&self);
    /// `KERNEL_UNLOCK()`: releases it once.
    fn unlock(&self);
    /// `KERNEL_ASSERT_LOCKED()`: panics unless the running CPU holds it (`DIAGNOSTIC`).
    fn assert_locked(&self);
    /// `_kernel_lock_held()`: whether the running CPU holds it.
    fn held(&self) -> bool;
}

/// The identity of the kernel lock's cells and lists: a word no lock owns, compared and never
/// dereferenced (`LockId`'s rule: no two live locks share one).
pub(crate) static KERNEL_LOCK_ID: AtomicUsize = AtomicUsize::new(0);

/// The kernel lock's identity ([`crate::rwlock::Guarded::new_kernel`] keys its cells to it).
pub fn kernel_lock_id() -> LockId {
    LockId::of_mutex(&KERNEL_LOCK_ID)
}

/// Per CPU, the witnesses out: 0 none, `n > 0` that many [`KernelReader`]s, -1 the
/// [`KernelWitness`].
static WITNESSES: PerCpu<AtomicI64> = PerCpu::new();

/// The exclusive witness of the kernel lock ([`witness`]): opens its cells mutably and
/// changes its lists. Not `Send`: it stays on the CPU that holds the lock; it holds the
/// interrupt level at `IPL_VM` or above until it drops.
#[must_use = "a witness opens the kernel lock's cells only while it lives"]
pub struct KernelWitness {
    _spl: SplGuard,
    _not_send: PhantomData<*const ()>,
}

/// A shared witness of the kernel lock ([`reader`]): reads its cells and walks its lists.
/// Not `Send`.
#[must_use = "a witness opens the kernel lock's cells only while it lives"]
pub struct KernelReader {
    _not_send: PhantomData<*const ()>,
}

/// The running CPU's witness count.
fn witnesses() -> &'static AtomicI64 {
    WITNESSES.get()
}

/// Whether the running CPU holds the kernel lock (always, before the bridge is registered:
/// one CPU runs).
pub fn kernel_lock_held() -> bool {
    bridges::kernel_lock().is_none_or(|b| b.held())
}

/// `KERNEL_ASSERT_LOCKED()` that holds always: `panic(9)` naming `what` unless the running
/// CPU holds the kernel lock.
fn require_held(what: &str) {
    if !kernel_lock_held() {
        crate::assert::hal_panic(format_args!("{what}: the kernel lock is not held"));
    }
}

/// The exclusive witness of the kernel lock, for a holder of it: `panic(9)` when the running
/// CPU does not hold the lock or has a witness of it out already (a reader or the writer:
/// the caller passes its own down instead).
pub fn witness() -> KernelWitness {
    require_held("klock::witness");
    let spl = splraise(IPL_VM);
    if witnesses()
        .compare_exchange(0, -1, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        crate::assert::hal_panic(format_args!(
            "klock::witness: a kernel lock witness is out on this CPU"
        ));
    }
    KernelWitness {
        _spl: spl,
        _not_send: PhantomData,
    }
}

/// A shared witness of the kernel lock, for a holder of it: `panic(9)` when the running CPU
/// does not hold the lock or has its exclusive witness out.
pub fn reader() -> KernelReader {
    require_held("klock::reader");
    if witnesses()
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n >= 0).then_some(n + 1)
        })
        .is_err()
    {
        crate::assert::hal_panic(format_args!(
            "klock::reader: the kernel lock's writer is out on this CPU"
        ));
    }
    KernelReader {
        _not_send: PhantomData,
    }
}

impl Drop for KernelWitness {
    fn drop(&mut self) {
        // The interrupt level is restored after this, by the field's drop.
        witnesses().store(0, Ordering::Relaxed);
    }
}

impl Drop for KernelReader {
    fn drop(&mut self) {
        witnesses().fetch_sub(1, Ordering::Relaxed);
    }
}

/// The kernel lock is about to be released by the running CPU (the switch's
/// `__mp_release_all`, the last `KERNEL_UNLOCK`): `panic(9)` naming `what` when a witness of it
/// is out here, whose holder would go on reading cells another CPU may then change.
pub fn assert_unwitnessed(what: &str) {
    let n = witnesses().load(Ordering::Relaxed);
    if n != 0 {
        crate::assert::hal_panic(format_args!(
            "{what}: the kernel lock released with a witness out ({n})"
        ));
    }
}

/// `KERNEL_LOCK()` (nothing before the bridge is registered).
pub fn kernel_lock() {
    if let Some(b) = bridges::kernel_lock() {
        b.lock();
    }
}

/// `KERNEL_UNLOCK()` (nothing before the bridge is registered).
pub fn kernel_unlock() {
    if let Some(b) = bridges::kernel_lock() {
        b.unlock();
    }
}

/// `KERNEL_ASSERT_LOCKED()` (holds before the bridge is registered).
pub fn kernel_assert_locked() {
    if let Some(b) = bridges::kernel_lock() {
        b.assert_locked();
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn before_registration_the_lock_is_a_no_op() {
        // hal's own tests register no kernel lock: one CPU, nothing to exclude.
        kernel_lock();
        kernel_assert_locked();
        kernel_unlock();
    }

    #[test]
    fn readers_nest_and_the_writer_is_alone() {
        // No bridge here: the lock counts as held and every thread is CPU 0; this is hal's
        // only test that takes a witness, so no other test thread shares the word.
        let r1 = reader();
        let r2 = reader();
        assert_eq!(witnesses().load(Ordering::Relaxed), 2);
        drop(r1);
        drop(r2);
        assert_unwitnessed("test");
        let w = witness();
        assert_eq!(witnesses().load(Ordering::Relaxed), -1);
        let caught = std::panic::catch_unwind(|| {
            let _r = reader();
        });
        assert!(caught.is_err());
        let caught = std::panic::catch_unwind(|| assert_unwitnessed("switch"));
        assert!(caught.is_err());
        drop(w);
        assert_unwitnessed("test");
        assert_eq!(kernel_lock_id(), kernel_lock_id());
    }
}
/* </TESTS> */
