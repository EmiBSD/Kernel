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
//! The network lock (`NET_LOCK()`, `NET_UNLOCK()` and `rw_status(&netlock)` of
//! `<sys/systm.h>`), as the kernel crate needs it while the network stack is `bsd`'s.
//!
//! The lock is `bsd`'s `netlock` (`sys/net/if.rs`), reached through the [`NetLock`] bridge,
//! registered by `bsd` at boot (bridges.toml; paid by N6, when the network stack moves). The
//! vnode pager (`kernel::vm::vnode`'s `uvn_io`) lets go of it around the vnode layer when the
//! faulting process holds it (a fault in `copyin()` or `copyout()` in the network stack).
//!
//! **Before registration** (early boot, the host tests that register none): there is no
//! network stack, so nobody holds the lock; taking and releasing it do nothing.
//!
//! ## Redesign
//! - Not a port: the C names `netlock` directly; the kernel crate reaches it through one trait
//!   object until the network stack moves in (docs/ZERO_UNSAFE.md section 0.3.2).

use crate::bridges;

/// The network lock, as `bsd` registers it (one call each into `if.c`'s `netlock`).
pub trait NetLock: Sync {
    /// `rw_status(&netlock) == RW_WRITE`: the running process holds it for writing.
    fn write_held(&self) -> bool;
    /// `NET_UNLOCK()`.
    fn unlock(&self);
    /// `NET_LOCK()`.
    fn lock(&self);
}

/// `rw_status(&netlock) == RW_WRITE` (false before the bridge is registered).
pub fn net_lock_write_held() -> bool {
    bridges::net_lock().is_some_and(|b| b.write_held())
}

/// `NET_UNLOCK()` (nothing before the bridge is registered).
pub fn net_unlock() {
    if let Some(b) = bridges::net_lock() {
        b.unlock();
    }
}

/// `NET_LOCK()` (nothing before the bridge is registered).
pub fn net_lock() {
    if let Some(b) = bridges::net_lock() {
        b.lock();
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn before_registration_nobody_holds_the_net_lock() {
        // hal's own tests register no network lock: there is no network stack.
        assert!(!net_lock_write_held());
        net_lock();
        net_unlock();
    }
}
/* </TESTS> */
