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
 * Copyright (c) 2017 Visa Hankala
 * Copyright (c) 2014 David Gwynne <dlg@openbsd.org>
 * Copyright (c) 2004 Artur Grabowski <art@openbsd.org>
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
 * Copyright (c) 2004 Niklas Hallqvist.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! The kernel lock: `kernel_lock`, the recursive ticket lock of `<sys/mplock.h>`
//! (`struct __mp_lock`, [`MpLock`]) and `kern_lock.c`'s functions over it, with
//! `KERNEL_LOCK()`, `KERNEL_UNLOCK()` and `KERNEL_ASSERT_LOCKED()` of `<sys/systm.h>`.
//!
//! Upstream: sys/kern/kern_lock.c @ 3ce1f3f79392
//! Upstream: sys/sys/mplock.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_lock.rs@3c62ede68802
//! LZ: sys/sys/mplock.rs@3c62ede68802
//!
//! The big lock serialises what is not yet `MPSAFE`. It is recursive: each CPU keeps its
//! depth and the ticket it drew when it first asked ([`MpLock::lock`]), and the lock serves
//! one ticket at a time; the last [`MpLock::unlock`] serves the next. A sleep or the switch
//! gives every level up at once ([`release_all`]) and takes the same number back
//! ([`acquire_count`]). The depth and ticket of a CPU change with its interrupts masked
//! (`hal::cpu`'s interrupt guard), as the C's `intr_disable`/`intr_restore` pairs, since an
//! interrupt handler on the same CPU takes the lock too.
//!
//! **The host.** Every test thread is a CPU of its own (`hal::cpu`'s host double), and tests
//! that take the kernel lock wait for each other's threads: a real ticket lock would
//! serialise or deadlock them. On the host the kernel lock is a per-thread depth
//! (`crate::arch::host::klock`), which keeps everything a test checks (held, depth, the
//! release checks) and excludes nothing; the ticket lock is tested on its own
//! ([`MpLock`] values of the tests).
//!
//! **Witnesses** (N4.2). The C keeps the process lists, the pid, tid and pgrp hashes and the
//! parent, child and group links under the kernel lock (`[K]` in `<sys/proc.h>`). The
//! kernel's cells and lists of that state are keyed to the kernel lock's identity
//! (`Guarded::new_kernel`, the pinned lists bound to it) and open through a witness of it, as
//! a `Mutexed`'s through its guard: [`witness`] gives the one exclusive [`KernelWitness`]
//! (changes), [`reader`] a shared [`KernelReader`] (walks, lookups). The big lock is
//! recursive, is taken by interrupt handlers on the CPU that holds it, and is released by every
//! sleep, so a witness is more than "the lock is held":
//! - it is made only while the running CPU holds the kernel lock ([`kernel_lock_held`]);
//! - per CPU, readers nest and the writer is alone: a witness asked for against the other
//!   kind already out on this CPU (a handler that interrupted its holder, a callee that takes
//!   its own) is `panic(9)`, as a `RefCell` refuses a second borrow; a writer raises the
//!   interrupt level to `IPL_VM`, above every handler that walks the lists (`ttyinput`'s
//!   `pgsignal` runs at `IPL_TTY` at most), so none of them meets one;
//! - the lock is not released while a witness is out on this CPU: the release paths (the
//!   switch's [`release_all`], the last [`kernel_unlock`]) call [`assert_unwitnessed`], so a
//!   holder never sleeps with one.
//!
//! So while a witness lives no other CPU holds the lock, and on this one only witnesses of
//! the same kind are out: the cells it opens see one writer or readers, never both.
//!
//! ## Deviations
//! - `__mp_lock_held` indexes the per-CPU holds by the CPU's number (`hal::cpu`'s), where the
//!   C uses `CPU_INFO_UNIT(ci)`; both are the CPU's number on every machine OpenBSD runs.
//! - `WITNESS` and `MP_LOCKDEBUG` (the spin-out into ddb, the "not held lock" check) are not
//!   configured, as in LZ.
//! - The host's kernel lock is a per-thread depth (above).
//!
//! ## Redesign
//! - N3.4c-N4.2: the kernel crate reached `bsd`'s `__mp_lock` through a `KernelLock` bridge,
//!   a no-op before registration. N4.5c: the ticket lock and `kernel_lock` are `hal`'s (the
//!   bridge is gone); `bsd`'s `kern_lock.rs` keeps the C's names over them. LZ's
//!   `intr_disable()`/`unsafe { intr_restore(s) }` pairs (4 `unsafe`) are the interrupt
//!   guard's scope.
//! - `kernel_lock`'s release checks (N4.2, LZ's `kernel_lock_release_check`, armed per host
//!   test thread in LZ) run on every machine and on the host double: the last
//!   [`kernel_unlock`] and [`release_all`] refuse a witness out, except past caring.
//! - N4.2: the C's `KERNEL_ASSERT_LOCKED()` in front of the `[K]` state becomes a witness the
//!   access needs ([`KernelWitness`], [`KernelReader`]), which checks the lock is held
//!   always, not only under `DIAGNOSTIC`.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicI64, AtomicU32, AtomicUsize, Ordering, fence};

use crate::cpu::{self, MAXCPUS};
use crate::intr::{IPL_VM, SplGuard, splraise};
use crate::percpu::PerCpu;
use crate::rwlock::LockId;

/// `struct __mp_lock_cpu`: one CPU's hold on an [`MpLock`].
struct MpLockCpu {
    /// `mplc_ticket`: the ticket this CPU drew when it first asked for the lock.
    ticket: AtomicU32,
    /// `mplc_depth`: how many times this CPU holds the lock.
    depth: AtomicU32,
}

impl MpLockCpu {
    /// A CPU that holds nothing.
    const fn new() -> Self {
        Self {
            ticket: AtomicU32::new(0),
            depth: AtomicU32::new(0),
        }
    }
}

/// `struct __mp_lock`: a recursive ticket lock, one hold per possible CPU (`kernel_lock`).
pub struct MpLock {
    /// `mpl_cpus[MAXCPUS]`, indexed by the CPU's number.
    cpus: [MpLockCpu; MAXCPUS],
    /// `mpl_ticket` (volatile): the ticket being served.
    ticket: AtomicU32,
    /// `mpl_users`: the last ticket handed out.
    users: AtomicU32,
}

impl MpLock {
    /// What `___mp_lock_init` leaves: nobody holds it and ticket 1 is served next.
    pub const fn new() -> Self {
        Self {
            cpus: [const { MpLockCpu::new() }; MAXCPUS],
            ticket: AtomicU32::new(1),
            users: AtomicU32::new(0),
        }
    }

    /// `___mp_lock_init` (`__mp_lock_init`): nobody holds the lock; ticket 1 is served next.
    pub fn init(&self) {
        for c in &self.cpus {
            c.ticket.store(0, Ordering::Relaxed);
            c.depth.store(0, Ordering::Relaxed);
        }
        self.users.store(0, Ordering::Relaxed);
        self.ticket.store(1, Ordering::Relaxed);
        // WITNESS: not configured.
    }

    /// CPU `n`'s hold (`n` below `MAXCPUS`: `hal::cpu`'s numbers are).
    fn cpu(&self, n: usize) -> &MpLockCpu {
        &self.cpus[n % MAXCPUS]
    }

    /// `__mp_lock_spin`: waits until ticket `me` is served, counted in the CPU's
    /// `spc_spinning`.
    #[inline]
    fn spin(&self, me: u32) {
        // MP_LOCKDEBUG (the spin-out into ddb): not configured.
        cpu::spinning(1);
        while self.ticket.load(Ordering::Relaxed) != me {
            core::hint::spin_loop(); // CPU_BUSY_CYCLE()
        }
        cpu::spinning(-1);
    }

    /// `__mp_lock`: takes the lock, recursively: a CPU that holds it only deepens its hold.
    pub fn lock(&self) {
        let c = self.cpu(cpu::cpu_number());
        // WITNESS_CHECKORDER: not configured.
        let masked = cpu::intr_disable();
        if c.depth.fetch_add(1, Ordering::Relaxed) == 0 {
            // atomic_inc_int_nv
            let ticket = self.users.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
            c.ticket.store(ticket, Ordering::Relaxed);
        }
        drop(masked);
        self.spin(c.ticket.load(Ordering::Relaxed));
        fence(Ordering::Acquire); // membar_enter_after_atomic()
        // WITNESS_LOCK: not configured.
    }

    /// `__mp_unlock`: drops one level of this CPU's hold; the last one serves the next
    /// ticket.
    pub fn unlock(&self) {
        let c = self.cpu(cpu::cpu_number());
        // MP_LOCKDEBUG (the "not held lock" check): not configured. WITNESS_UNLOCK: neither.
        let masked = cpu::intr_disable();
        if c.depth.fetch_sub(1, Ordering::Relaxed) == 1 {
            fence(Ordering::Release); // membar_exit()
            self.ticket.fetch_add(1, Ordering::Relaxed);
        }
        drop(masked);
    }

    /// `__mp_release_all`: drops every level of this CPU's hold; how many there were.
    pub fn release_all(&self) -> u32 {
        let c = self.cpu(cpu::cpu_number());
        let masked = cpu::intr_disable();
        let rv = c.depth.load(Ordering::Relaxed);
        // WITNESS_UNLOCK for each level: not configured.
        c.depth.store(0, Ordering::Relaxed);
        fence(Ordering::Release); // membar_exit()
        self.ticket.fetch_add(1, Ordering::Relaxed);
        drop(masked);
        rv
    }

    /// `__mp_acquire_count`: takes the lock `count` times (what `release_all` returned).
    pub fn acquire_count(&self, count: u32) {
        for _ in 0..count {
            self.lock();
        }
    }

    /// `__mp_lock_held(mpl, ci)`: CPU `n` holds the lock.
    pub fn held_by(&self, n: usize) -> bool {
        let c = self.cpu(n);
        c.ticket.load(Ordering::Relaxed) == self.ticket.load(Ordering::Relaxed)
            && c.depth.load(Ordering::Relaxed) > 0
    }

    /// CPU `n`'s depth in the lock.
    pub fn depth_of(&self, n: usize) -> u32 {
        self.cpu(n).depth.load(Ordering::Relaxed)
    }
}

impl Default for MpLock {
    fn default() -> Self {
        Self::new()
    }
}

/// `kernel_lock`: the big lock serialising whatever is not yet `MPSAFE`.
pub static KERNEL_LOCK: MpLock = MpLock::new();

/// The kernel lock on a machine: [`KERNEL_LOCK`] for the running CPU.
#[cfg(target_os = "none")]
mod backend {
    use super::KERNEL_LOCK;
    use crate::cpu::cpu_number;

    /// `KERNEL_LOCK_INIT()`.
    pub(super) fn init() {
        KERNEL_LOCK.init();
    }

    /// `__mp_lock(&kernel_lock)`.
    #[inline]
    pub(super) fn lock() {
        KERNEL_LOCK.lock();
    }

    /// `__mp_unlock(&kernel_lock)`.
    #[inline]
    pub(super) fn unlock() {
        KERNEL_LOCK.unlock();
    }

    /// `__mp_lock_held(&kernel_lock, curcpu())`.
    #[inline]
    pub(super) fn held() -> bool {
        KERNEL_LOCK.held_by(cpu_number())
    }

    /// The running CPU's depth.
    #[inline]
    pub(super) fn depth() -> u32 {
        KERNEL_LOCK.depth_of(cpu_number())
    }

    /// `__mp_release_all(&kernel_lock)`.
    pub(super) fn release_all() -> u32 {
        KERNEL_LOCK.release_all()
    }

    /// `__mp_acquire_count(&kernel_lock, count)`.
    pub(super) fn acquire_count(count: u32) {
        KERNEL_LOCK.acquire_count(count);
    }
}

/// The kernel lock of the host double: a per-thread depth (the module's docs).
#[cfg(not(target_os = "none"))]
use crate::arch::host::klock as backend;

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

/// `KERNEL_LOCK_INIT()` (`_kernel_lock_init`'s lock half), once, from `main`.
pub fn init() {
    backend::init();
}

/// `_kernel_lock_held()`: the running CPU holds the kernel lock (always after a panic or in
/// ddb, which hold it by fiat).
pub fn kernel_lock_held() -> bool {
    cpu::past_caring() || backend::held()
}

/// The running CPU's depth in the kernel lock (0: not held).
pub fn depth() -> u32 {
    backend::depth()
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
/// [`release_all`], the last [`kernel_unlock`]): `panic(9)` naming `what` when a witness of
/// it is out here, whose holder would go on reading cells another CPU may then change.
pub fn assert_unwitnessed(what: &str) {
    let n = witnesses().load(Ordering::Relaxed);
    if n != 0 {
        crate::assert::hal_panic(format_args!(
            "{what}: the kernel lock released with a witness out ({n})"
        ));
    }
}

/// Whether a release of the kernel lock checks that no witness of it is out: not when the
/// kernel is past caring (after a panic, in ddb, which hold the lock by fiat and may yield
/// from inside a witness section, where a second panic would hide the first).
fn release_check_due(past_caring: bool) -> bool {
    !past_caring
}

/// `_kernel_lock()` (`KERNEL_LOCK()`): acquire the kernel lock. Intended for use in the
/// scheduler and the lower half of the kernel; never under the scheduler lock.
pub fn kernel_lock() {
    crate::assert::kassert!(!crate::thread::sched_locked()); // SCHED_ASSERT_UNLOCKED()
    backend::lock();
}

/// `_kernel_unlock()` (`KERNEL_UNLOCK()`): release the kernel lock once. The last level
/// refuses a witness out on this CPU, except past caring.
pub fn kernel_unlock() {
    if backend::depth() == 1 && release_check_due(cpu::past_caring()) {
        assert_unwitnessed("KERNEL_UNLOCK");
    }
    backend::unlock();
}

/// `__mp_release_all(&kernel_lock)`: every hold of the running CPU released, their count
/// returned. A witness out here is `panic(9)` (its holder would go on reading cells another
/// CPU may change), except past caring.
pub fn release_all() -> u32 {
    if release_check_due(cpu::past_caring()) {
        assert_unwitnessed("__mp_release_all");
    }
    backend::release_all()
}

/// `__mp_acquire_count(&kernel_lock, count)`: the kernel lock taken back `count` times.
pub fn acquire_count(count: u32) {
    backend::acquire_count(count);
}

/// `KERNEL_ASSERT_LOCKED()`: `KASSERT(_kernel_lock_held())` (`DIAGNOSTIC`).
pub fn kernel_assert_locked() {
    crate::assert::kassert!(kernel_lock_held());
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use super::*;

    #[test]
    fn the_kernel_lock_is_recursive_and_counts_its_depth() {
        assert!(!backend::held());
        kernel_lock();
        kernel_lock();
        assert!(kernel_lock_held());
        assert_eq!(depth(), 2);
        kernel_assert_locked();
        kernel_unlock();
        assert_eq!(depth(), 1);
        // The switch: every level given up, then taken back.
        assert_eq!(release_all(), 1);
        assert_eq!(depth(), 0);
        acquire_count(1);
        assert_eq!(depth(), 1);
        kernel_unlock();
        assert_eq!(depth(), 0);
    }

    #[test]
    fn readers_nest_and_the_writer_is_alone() {
        kernel_lock();
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
        kernel_unlock();
    }

    #[test]
    fn a_witness_needs_the_lock() {
        let caught = std::panic::catch_unwind(|| {
            let _r = reader();
        });
        assert!(caught.is_err());
    }

    /// The last `KERNEL_UNLOCK` and `__mp_release_all` with a witness out are `panic(9)`
    /// (LZ's `kernel_lock_release_check`).
    #[test]
    fn a_release_with_a_witness_out_panics() {
        std::thread::spawn(|| {
            kernel_lock();
            let r = reader();
            let caught = std::panic::catch_unwind(kernel_unlock);
            assert!(caught.is_err());
            assert_eq!(depth(), 1);
            let caught = std::panic::catch_unwind(release_all);
            assert!(caught.is_err());
            drop(r);
            // Inner levels are not the release: no check.
            kernel_lock();
            let r = reader();
            kernel_unlock();
            drop(r);
            kernel_unlock();
        })
        .join()
        .unwrap();
    }

    /// `__mp_release_all`'s witness check runs except past caring (a panic, ddb), as
    /// `kern_lock.c`'s.
    #[test]
    fn the_release_check_is_skipped_past_caring() {
        assert!(release_check_due(false));
        assert!(!release_check_due(true));
    }

    /// The ticket lock itself (a machine's kernel lock), on a lock of the test's own: two
    /// threads (two CPUs of the host double) never hold it at once, and a holder's nested
    /// levels and whole release work as `__mp_lock`'s.
    #[test]
    fn the_ticket_lock_excludes_and_serves_in_order() {
        static L: MpLock = MpLock::new();
        static INSIDE: AtomicBool = AtomicBool::new(false);
        static COUNT: AtomicU32 = AtomicU32::new(0);
        L.init();
        std::thread::scope(|s| {
            for _ in 0..2 {
                s.spawn(|| {
                    for _ in 0..2000 {
                        L.lock();
                        L.lock();
                        assert!(L.held_by(cpu::cpu_number()));
                        assert!(!INSIDE.swap(true, Ordering::Relaxed));
                        COUNT.fetch_add(1, Ordering::Relaxed);
                        INSIDE.store(false, Ordering::Relaxed);
                        L.unlock();
                        assert_eq!(L.depth_of(cpu::cpu_number()), 1);
                        assert_eq!(L.release_all(), 1);
                        assert!(!L.held_by(cpu::cpu_number()));
                    }
                });
            }
        });
        assert_eq!(COUNT.load(Ordering::Relaxed), 4000);
        // Every ticket handed out was served: free, the next one is the next drawn.
        assert_eq!(
            L.ticket.load(Ordering::Relaxed),
            L.users.load(Ordering::Relaxed) + 1
        );
    }
}
/* </TESTS> */
