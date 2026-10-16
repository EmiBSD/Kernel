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
//! **Before registration** (early boot, before `main()` registers the bridges, and the host
//! tests that register none): there are no interrupts yet on the one CPU that runs, so raising
//! the level has nothing to hold back. [`splraise`] then returns a guard that restores
//! nothing, and [`mpfloor`] is `IPL_NONE`.
//!
//! ## Redesign
//! - `s = splraise(ipl); ...; splx(s)` -> `let _s = splraise(ipl);`, the guard restoring on
//!   drop. The raw pair stays for the locks of this crate (`mutex.rs`), which store the old
//!   level in the lock as `mtx_oldipl` does.

use core::marker::PhantomData;

use crate::bridges;

/// `IPL_NONE`: no interrupt is blocked (0 on amd64 and arm64).
pub const IPL_NONE: i32 = 0;

/// A raised interrupt level; dropping it is `splx` of the level it replaced.
#[must_use = "dropping the guard at once restores the level"]
pub struct SplGuard {
    old: i32,
    /// `spl(9)` is per CPU: the guard stays on the CPU that raised the level.
    _not_send: PhantomData<*const ()>,
}

impl SplGuard {
    /// The level the guard restores.
    pub fn old_level(&self) -> i32 {
        self.old
    }
}

impl Drop for SplGuard {
    fn drop(&mut self) {
        splx(self.old);
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
}

/// `splraise(ipl)` as a guard: the level is at least `ipl` until the guard drops.
pub fn splraise(ipl: i32) -> SplGuard {
    SplGuard {
        old: splraise_raw(ipl),
        _not_send: PhantomData,
    }
}

/// `splraise(ipl)`: the old level (`IPL_NONE` before the bridge is registered).
pub(crate) fn splraise_raw(ipl: i32) -> i32 {
    bridges::intr_level().map_or(IPL_NONE, |b| b.splraise(ipl))
}

/// `splx(s)` (nothing before the bridge is registered).
pub(crate) fn splx(s: i32) {
    if let Some(b) = bridges::intr_level() {
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
}
/* </TESTS> */
