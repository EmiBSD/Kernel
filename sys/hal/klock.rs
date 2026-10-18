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
//! ## Redesign
//! - Not a port: the C calls `_kernel_lock()` and its kin directly; the kernel crate reaches
//!   them through one trait object until the lock moves in (docs/ZERO_UNSAFE.md section 0.3.2).

use crate::bridges;

/// The kernel lock, as `bsd` registers it (one call each into `kern_lock.rs`).
pub trait KernelLock: Sync {
    /// `KERNEL_LOCK()`: takes the kernel lock (recursively, as `__mp_lock` does).
    fn lock(&self);
    /// `KERNEL_UNLOCK()`: releases it once.
    fn unlock(&self);
    /// `KERNEL_ASSERT_LOCKED()`: panics unless the running CPU holds it (`DIAGNOSTIC`).
    fn assert_locked(&self);
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
}
/* </TESTS> */
