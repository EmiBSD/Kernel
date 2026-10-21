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
//! The interrupt-level guard: `spl(9)` as a value whose drop restores the level.
//!
//! The C raises the level with `s = splraise(ipl)` (or `splnet()`, `splbio()`, ...) and must
//! remember to `splx(s)` on every path out; LZ keeps that shape (`sys/machine/intr.rs`). Here
//! [`splraise`] returns an [`SplGuard`] and its drop is the `splx`: a path cannot leave the
//! level raised, and the guard cannot leave the CPU that raised it (it is not `Send`).
//!
//! The levels themselves are the machine's (`IPL_NONE` is 0 on both architectures; the
//! others differ between amd64 and arm64), so they are passed in as the C's numbers. The
//! machine's `splraise`/`splx` reach the runtime through the [`IntrLevel`] bridge, registered
//! by `bsd` at boot over `sys/machine/intr.rs` (bridges.toml; paid by N4, when the hardware
//! is `hal`'s own and the bridge goes).
//!
//! **Before registration** (early boot, before the boot path registers the bridges, and the
//! host tests that register none): there are no interrupts yet on the one CPU that runs, so
//! raising the level has nothing to hold back. A guard taken then raised nothing and so
//! restores nothing, even if the bridge is registered before it drops: a guard restores only
//! a level it raised (the reviewer's finding 5; restoring `IPL_NONE` there would lower the
//! real level). [`mpfloor`] is `IPL_NONE` before registration.
//!
//! ## Redesign
//! - `s = splraise(ipl); ...; splx(s)` -> `let _s = splraise(ipl);`, the guard restoring on
//!   drop. The raw pair (`raise`/`restore`, crate-private) carries the same "raised or not"
//!   for the locks of this crate (`mutex.rs`), which keep it in their guards.

use core::marker::PhantomData;

use crate::bridges;

/// `IPL_NONE`: no interrupt is blocked (0 on amd64 and arm64).
pub const IPL_NONE: i32 = 0;

/// `IPL_SOFTCLOCK`: soft clock interrupts, the level timeout(9)'s `softclock` runs at and its
/// `softclock` thread raises to (`splsoftclock`). The machine's number, until the interrupt
/// tables enter `hal` with their owner (N4.5): the host double's is amd64's numbering, as for
/// the other levels here (`<machine/intrdefs.h>` 0x1); amd64's kernel gains its own at N10
/// (decision 44). `bsd`'s `sys/machine/intr.rs` asserts that it equals the machine's.
#[cfg(not(target_os = "none"))]
pub const IPL_SOFTCLOCK: i32 = 0x1;
/// `IPL_SOFTCLOCK`: soft clock interrupts (arm64 `<machine/intr.h>`), until the interrupt
/// tables enter `hal` with their owner (N4.5); `bsd`'s `sys/machine/intr.rs` asserts that it
/// equals the machine's.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub const IPL_SOFTCLOCK: i32 = 2;

/// `IPL_BIO`: block I/O, the level of the page system's per-CPU caches (the buffer flipper
/// allocates and frees pages from interrupt context, `uvm_pmemrange.c`). The machine's number
/// (amd64 `<machine/intrdefs.h>` 0x3; the host double uses amd64's), until the interrupt
/// tables enter `hal` with their owner (N4); `bsd`'s `sys/machine/intr.rs` asserts that it
/// equals the machine's.
#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub const IPL_BIO: i32 = 0x3;
/// `IPL_BIO`: block I/O, the level of the page system's per-CPU caches. The machine's number
/// (arm64 `<machine/intr.h>`), until the interrupt tables enter `hal` with their owner (N4);
/// `bsd`'s `sys/machine/intr.rs` asserts that it equals the machine's.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub const IPL_BIO: i32 = 5;
/// `IPL_VM`: memory allocation, the level of the pmap's locks (`pm_mtx`, `pv_mtx`). The
/// machine's number (amd64 `<machine/intrdefs.h>` 0xa, arm64 `<machine/intr.h>` 8; the host
/// double uses amd64's), until the interrupt tables enter `hal` with their owner (N4); `bsd`'s
/// `sys/machine/intr.rs` asserts that it equals the machine's.
#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub const IPL_VM: i32 = 0xa;
/// `IPL_VM`: memory allocation, the level of the pmap's locks (`pm_mtx`, `pv_mtx`). The
/// machine's number (arm64 `<machine/intr.h>`), until the interrupt tables enter `hal` with
/// their owner (N4); `bsd`'s `sys/machine/intr.rs` asserts that it equals the machine's.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub const IPL_VM: i32 = 8;

/// `IPL_HIGH`: everything (arm64's `pmap_asid_mtx`). The machine's number (amd64 0xd, the
/// host double's too); asserted equal to the machine's by `bsd`'s `sys/machine/intr.rs`.
#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub const IPL_HIGH: i32 = 0xd;
/// `IPL_HIGH`: everything (arm64's `pmap_asid_mtx`). The machine's number (arm64 11);
/// asserted equal to the machine's by `bsd`'s `sys/machine/intr.rs`.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub const IPL_HIGH: i32 = 11;

/// A raised interrupt level; dropping it is `splx` of the level it replaced.
#[must_use = "dropping the guard at once restores the level"]
pub struct SplGuard {
    /// The level to restore, `None` when nothing was raised (no bridge yet).
    old: Option<i32>,
    /// `spl(9)` is per CPU: the guard stays on the CPU that raised the level.
    _not_send: PhantomData<*const ()>,
}

impl SplGuard {
    /// The level the guard restores (`IPL_NONE` when it raised nothing).
    pub fn old_level(&self) -> i32 {
        self.old.unwrap_or(IPL_NONE)
    }
}

impl Drop for SplGuard {
    fn drop(&mut self) {
        restore(self.old);
    }
}

/// The machine's `spl(9)`, as `bsd` registers it (`sys/machine/intr.rs`).
pub trait IntrLevel: Sync {
    /// `splraise(ipl)`: raises the current level to at least `ipl`; returns the old level.
    fn splraise(&self, ipl: i32) -> i32;
    /// `splx(s)`: restores a level `splraise` returned.
    fn splx(&self, s: i32);
    /// `IPL_MPFLOOR`: the lowest level a mutex raises to under `MULTIPROCESSOR`.
    fn mpfloor(&self) -> i32;
    /// `IPL_HIGH`: the level that blocks every interrupt (`splhigh()`).
    fn ipl_high(&self) -> i32;
    /// `splassert(wantipl)` (`DIAGNOSTIC`, while `splassert_ctl` is on): reports, through
    /// the machine's `splassert_fail`, a caller of `func` running below `wantipl`.
    fn splassert(&self, wantipl: i32, func: &str);
}

/// `splraise(ipl)` as a guard: the level is at least `ipl` until the guard drops.
pub fn splraise(ipl: i32) -> SplGuard {
    SplGuard {
        old: raise(ipl),
        _not_send: PhantomData,
    }
}

/// `splhigh()` as a guard (nothing raised before the bridge is registered).
pub fn splhigh() -> SplGuard {
    SplGuard {
        old: bridges::intr_level().map(|b| b.splraise(b.ipl_high())),
        _not_send: PhantomData,
    }
}

/// `splassert(wantipl)`: the machine's `DIAGNOSTIC` check that the running code is at
/// `wantipl` or above (named `func` in the report); nothing before the bridge is registered
/// (no interrupts yet: every level holds).
pub fn splassert(wantipl: i32, func: &str) {
    if let Some(b) = bridges::intr_level() {
        b.splassert(wantipl, func);
    }
}

/// `splraise(ipl)`: `Some(old level)` when the level was raised through the bridge, `None`
/// when there is no bridge yet (nothing raised, so nothing to restore).
pub(crate) fn raise(ipl: i32) -> Option<i32> {
    bridges::intr_level().map(|b| b.splraise(ipl))
}

/// `splx(s)` of what [`raise`] returned: nothing when it raised nothing. The bridge is set once,
/// so a level raised through it is restored through the same implementation.
pub(crate) fn restore(old: Option<i32>) {
    if let (Some(s), Some(b)) = (old, bridges::intr_level()) {
        b.splx(s);
    }
}

/// `IPL_MPFLOOR` (`IPL_NONE` before the bridge is registered: no interrupts yet).
pub(crate) fn mpfloor() -> i32 {
    bridges::intr_level().map_or(IPL_NONE, |b| b.mpfloor())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn the_guard_restores_the_level_it_replaced() {
        testutil::install();
        let base = testutil::spl_level();
        {
            let g = splraise(base + 3);
            assert_eq!(g.old_level(), base);
            assert_eq!(testutil::spl_level(), base + 3);
            {
                let inner = splraise(base + 1);
                // `splraise` never lowers.
                assert_eq!(testutil::spl_level(), base + 3);
                drop(inner);
            }
            assert_eq!(testutil::spl_level(), base + 3);
        }
        assert_eq!(testutil::spl_level(), base);
    }

    #[test]
    fn a_guard_that_raised_nothing_restores_nothing() {
        testutil::install();
        // As a guard taken before the bridge was registered and dropped after it.
        let raised = splraise(7);
        let before = testutil::spl_level();
        drop(SplGuard {
            old: None,
            _not_send: PhantomData,
        });
        assert_eq!(testutil::spl_level(), before);
        drop(raised);
    }

    #[test]
    fn splhigh_raises_to_ipl_high() {
        testutil::install();
        let base = testutil::spl_level();
        {
            let _s = splhigh();
            assert_eq!(testutil::spl_level(), base.max(testutil::IPL_HIGH));
        }
        assert_eq!(testutil::spl_level(), base);
    }
}
/* </TESTS> */
