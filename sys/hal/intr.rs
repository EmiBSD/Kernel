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
 * Copyright (c) 2011 Dale Rahn <drahn@openbsd.org>
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
//! `spl(9)`'s generic half: the CPU's interrupt priority level (`ci_cpl`), the levels'
//! defaults (`arm_dflt_splraise`, `arm_dflt_spllower`, `arm_dflt_splx`, `arm_dflt_setipl`),
//! the soft interrupts pending on a CPU (`ci_ipending`, `softintr()`, `arm_smask`) and their
//! run when the level drops below them (`arm_do_pending_intr`); the interrupt controller's
//! part is a registration ([`IntrHardware`], the C's `struct arm_intr_func` table). And the
//! interrupt-level guard: `spl(9)` as a value whose drop restores the level.
//!
//! Upstream: sys/arch/arm64/arm64/intr.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/intr.rs@3c62ede68802
//!
//! The C raises the level with `s = splraise(ipl)` (or `splnet()`, `splbio()`, ...) and must
//! remember to `splx(s)` on every path out; LZ keeps that shape (`sys/machine/intr.rs`).
//! Here [`splraise`] returns an [`SplGuard`] and its drop is the `splx`: a path cannot leave
//! the level raised, and the guard cannot leave the CPU that raised it (it is not `Send`).
//! The raw forms ([`splraise_raw`], [`spllower`], [`splx`]) are `bsd`'s `spl*()` (the
//! machine's `Intr` of `sys/machine/intr.rs` forwards to them, 136 files' `splx`).
//!
//! The levels are the machine's numbers (`IPL_NONE` is 0 on both machines; the others
//! differ between amd64 and arm64; the host double uses amd64's). The level lives in the
//! running CPU's core ([`crate::cpu::CpuCore::cpl`]); the controller writes its own mask
//! when it registered one (`agintc_setipl` writes `ICC_PMR`), each `spl*` going through
//! the registration as the C goes through `arm_intr_func`, the defaults otherwise (before a
//! controller attaches, on the host).
//!
//! **Soft interrupts.** `softintr(si)` marks `si` pending on this CPU; a level lowered below
//! a soft level (`splx`, `spllower`) runs the pending soft interrupts that level leaves
//! unmasked, highest first, each at its own level with the machine's interrupts enabled
//! ([`do_pending_intr`], which runs `crate::softintr::dispatch`). Which soft levels a level
//! leaves unmasked is [`smask`], the table `arm_init_smask` fills once
//! ([`init_smask`], called by the controller's attach as in the C): until then no level
//! unmasks a soft interrupt, as the C's zeroed `arm_smask`. The host double never fills it:
//! a host test's soft interrupts run when it dispatches them itself.
//!
//! ## Deviations
//! - amd64's `spl(9)` (`Xspllower`, `ci_isources`) is N10's (decision 44): this module is
//!   arm64's and the host's, and amd64's `hal` has no `intr_disable` yet.
//!
//! ## Redesign
//! - `s = splraise(ipl); ...; splx(s)` -> `let _s = splraise(ipl);`, the guard restoring on
//!   drop.
//! - LZ's `ci_cpl`, `ci_ipending` (`Cell`s of `bsd`'s arm64 `cpu_info`) and `arm_smask`, a
//!   `StaticCell` written under "boot CPU, before interrupts", are the core's atomics and a
//!   table of atomics; `struct arm_intr_func`, a `StaticCell` of function pointers, is the
//!   [`IntrHardware`] trait (decision 46), whose default methods are the `arm_dflt_*`
//!   functions, registered once by the machine (arm64's `initarm`) and answering through
//!   what the controller gave `arm_set_intr_handler`. `arm_do_pending_intr`'s
//!   `disable_irq_daif_ret`/`restore_daif` pair is `hal::cpu`'s interrupt guard.
//! - N4.5c: the `IntrLevel` bridge (`bsd`'s `splraise`/`splx` behind every `hal` lock) is
//!   gone: the level is `hal`'s own, read and written here.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::bridges;
use crate::cpu::curcpu;
use crate::sys::softintr::{NSOFTINTR, SOFTINTR_CLOCK, SOFTINTR_NET, SOFTINTR_TTY};

/// `IPL_NONE`: no interrupt is blocked (0 on amd64 and arm64).
pub const IPL_NONE: i32 = 0;

/// The levels of the host double: amd64's numbering (`<machine/intrdefs.h>`).
#[cfg(not(target_os = "none"))]
mod levels {
    /// `IPL_SOFTCLOCK`.
    pub const IPL_SOFTCLOCK: i32 = 0x1;
    /// `IPL_SOFTNET`.
    pub const IPL_SOFTNET: i32 = 0x2;
    /// `IPL_BIO`.
    pub const IPL_BIO: i32 = 0x3;
    /// `IPL_SOFTTTY`.
    pub const IPL_SOFTTTY: i32 = 0x8;
    /// `IPL_TTY`.
    pub const IPL_TTY: i32 = 0x9;
    /// `IPL_VM`.
    pub const IPL_VM: i32 = 0xa;
    /// `IPL_SCHED` (`IPL_CLOCK`).
    pub const IPL_SCHED: i32 = 0xc;
    /// `IPL_HIGH`.
    pub const IPL_HIGH: i32 = 0xd;
    /// `IPL_MPFLOOR`: `IPL_TTY`.
    pub const IPL_MPFLOOR: i32 = 0x9;
    /// `NIPL`: the levels' count.
    pub const NIPL: usize = 16;
}

/// The levels of arm64 (`<machine/intr.h>`).
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
mod levels {
    /// `IPL_SOFTCLOCK`.
    pub const IPL_SOFTCLOCK: i32 = 2;
    /// `IPL_SOFTNET`.
    pub const IPL_SOFTNET: i32 = 3;
    /// `IPL_SOFTTTY`.
    pub const IPL_SOFTTTY: i32 = 4;
    /// `IPL_BIO`.
    pub const IPL_BIO: i32 = 5;
    /// `IPL_TTY`.
    pub const IPL_TTY: i32 = 7;
    /// `IPL_VM`.
    pub const IPL_VM: i32 = 8;
    /// `IPL_SCHED` (`IPL_CLOCK`).
    pub const IPL_SCHED: i32 = 10;
    /// `IPL_HIGH`.
    pub const IPL_HIGH: i32 = 11;
    /// `IPL_MPFLOOR`: `IPL_TTY`.
    pub const IPL_MPFLOOR: i32 = 7;
    /// `NIPL`: the levels' count.
    pub const NIPL: usize = 13;
}

/// The levels of amd64 (`<machine/intrdefs.h>`): its `spl(9)` is N10's.
#[cfg(all(target_os = "none", target_arch = "x86_64"))]
mod levels {
    /// `IPL_SOFTCLOCK`.
    pub const IPL_SOFTCLOCK: i32 = 0x1;
    /// `IPL_SOFTNET`.
    pub const IPL_SOFTNET: i32 = 0x2;
    /// `IPL_BIO`.
    pub const IPL_BIO: i32 = 0x3;
    /// `IPL_SOFTTTY`.
    pub const IPL_SOFTTTY: i32 = 0x8;
    /// `IPL_TTY`.
    pub const IPL_TTY: i32 = 0x9;
    /// `IPL_VM`.
    pub const IPL_VM: i32 = 0xa;
    /// `IPL_SCHED` (`IPL_CLOCK`).
    pub const IPL_SCHED: i32 = 0xc;
    /// `IPL_HIGH`.
    pub const IPL_HIGH: i32 = 0xd;
    /// `IPL_MPFLOOR`: `IPL_TTY`.
    pub const IPL_MPFLOOR: i32 = 0x9;
    /// `NIPL`: the levels' count.
    pub const NIPL: usize = 16;
}

pub use levels::{
    IPL_BIO, IPL_HIGH, IPL_MPFLOOR, IPL_SCHED, IPL_SOFTCLOCK, IPL_SOFTNET, IPL_SOFTTTY, IPL_TTY,
    IPL_VM, NIPL,
};

/// `IPL_MPSAFE`: an "mpsafe" interrupt or soft interrupt, run without the kernel lock (a flag
/// beside the level, both machines).
pub const IPL_MPSAFE: i32 = 0x100;

/// `SI_TO_IRQBIT(si)`: the `ci_ipending` bit of soft interrupt `si`.
const fn si_to_irqbit(si: usize) -> u64 {
    1 << si
}

/// A raised interrupt level; dropping it is `splx` of the level it replaced.
#[must_use = "dropping the guard at once restores the level"]
pub struct SplGuard {
    /// The level to restore.
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

/// `arm_smask[NIPL]`: the soft interrupts each level leaves unmasked, filled once by
/// [`init_smask`]; all zero before (nothing pending ever runs).
static SMASK: [AtomicU64; 16] = [const { AtomicU64::new(0) }; 16];

/// The machine's interrupt controller (`struct arm_intr_func` and `arm_set_intr_handler`'s
/// table), as `bsd` registers it (`sys/kern/hal_bridges.rs`, over what a controller's attach
/// gave `arm_set_intr_handler`; the GIC drivers are `bsd`'s until N7). Every method's
/// default is the C's `arm_dflt_*`, what runs before a controller attaches.
pub trait IntrHardware: Sync {
    /// `splraise(ipl)`: raises this CPU's level to at least `ipl`; returns the old level.
    fn splraise(&self, ipl: i32) -> i32 {
        dflt_splraise(ipl)
    }
    /// `spllower(ipl)`: the level set to `ipl`, the pending soft interrupts it unmasks run;
    /// returns the old level.
    fn spllower(&self, ipl: i32) -> i32 {
        dflt_spllower(ipl)
    }
    /// `splx(ipl)`: the level restored to `ipl`, the pending soft interrupts it unmasks run.
    fn splx(&self, ipl: i32) {
        dflt_splx(ipl);
    }
    /// `setipl(ipl)`: the level set to `ipl` in this CPU's core (`CpuCore::set_cpl`) and in
    /// the controller's mask, with this CPU's interrupts kept off while both change.
    fn setipl(&self, ipl: i32) {
        dflt_setipl(ipl);
    }
    /// `arm_splassert_check(wantipl, func)` behind `splassert` (`DIAGNOSTIC`, while
    /// `splassert_ctl` is on): reports a caller of `func` running below `wantipl`.
    fn splassert(&self, wantipl: i32, func: &str) {
        let _ = (wantipl, func);
    }
}

/// `arm_dflt_splraise`: raise the level in `ci_cpl`.
pub fn dflt_splraise(newcpl: i32) -> i32 {
    let c = curcpu();
    let oldcpl = c.cpl();
    c.set_cpl(newcpl.max(oldcpl));
    oldcpl
}

/// `arm_dflt_spllower`: lower the level, running what becomes unmasked.
pub fn dflt_spllower(newcpl: i32) -> i32 {
    let oldcpl = curcpu().cpl();
    splx(newcpl);
    oldcpl
}

/// `arm_dflt_splx`: restore the level, running the pending soft interrupts it unmasks.
pub fn dflt_splx(newcpl: i32) {
    let c = curcpu();
    if c.ipending() & smask(newcpl) != 0 {
        do_pending_intr(newcpl);
    }
    c.set_cpl(newcpl);
}

/// `arm_dflt_setipl`: set the level.
pub fn dflt_setipl(newcpl: i32) {
    curcpu().set_cpl(newcpl);
}

/// `arm_init_smask`: which soft interrupts each level leaves unmasked, once (a second call
/// changes nothing: the table is a function of the levels).
pub fn init_smask() {
    for (i, mask) in SMASK.iter().enumerate().take(IPL_HIGH as usize + 1) {
        let i = i as i32;
        let mut m = 0;
        if i < IPL_SOFTCLOCK {
            m |= si_to_irqbit(SOFTINTR_CLOCK as usize);
            // NXCALL > 0: SOFTINTR_XCALL; NXCALL is 0 on arm64 (no xcall(4) in GENERIC).
        }
        if i < IPL_SOFTNET {
            m |= si_to_irqbit(SOFTINTR_NET as usize);
        }
        if i < IPL_SOFTTTY {
            m |= si_to_irqbit(SOFTINTR_TTY as usize);
        }
        mask.store(m, Ordering::Relaxed);
    }
}

/// `arm_smask[level]`: the soft interrupts `level` leaves unmasked (0 for a level past the
/// table, or before [`init_smask`]).
#[inline]
pub fn smask(level: i32) -> u64 {
    usize::try_from(level)
        .ok()
        .and_then(|l| SMASK.get(l))
        .map_or(0, |m| m.load(Ordering::Relaxed))
}

/// `setipl(ipl)` through the registered controller (`arm_intr_func.setipl`), the default
/// before one.
#[inline]
pub fn setipl(ipl: i32) {
    match bridges::intr_hardware() {
        Some(hw) => hw.setipl(ipl),
        None => dflt_setipl(ipl),
    }
}

/// `arm_do_pending_intr(pcpl)`: runs the soft interrupts pending on this CPU that `pcpl`
/// leaves unmasked, highest first, each at its level with the machine's interrupts on, until
/// none is left; then the level is `pcpl`.
pub fn do_pending_intr(pcpl: i32) {
    let c = curcpu();
    let mut masked = Some(crate::cpu::intr_disable());

    loop {
        for (si, ipl) in [
            (SOFTINTR_TTY, IPL_SOFTTTY),
            (SOFTINTR_NET, IPL_SOFTNET),
            (SOFTINTR_CLOCK, IPL_SOFTCLOCK),
        ] {
            let bit = si_to_irqbit(si as usize);
            if c.ipending() & smask(pcpl) & bit != 0 {
                c.clear_pending(bit);
                setipl(ipl);
                drop(masked.take());
                crate::softintr::dispatch(si as usize);
                masked = Some(crate::cpu::intr_disable());
            }
        }
        // NXCALL > 0: SOFTINTR_XCALL; NXCALL is 0 on arm64 (no xcall(4) in GENERIC).
        if c.ipending() & smask(pcpl) == 0 {
            break;
        }
    }

    // Don't use splx... we are here already!
    setipl(pcpl);
    drop(masked);
}

/// `splraise(ipl)`: raises this CPU's level to at least `ipl`; the old level (`bsd`'s
/// `splraise`, through the controller as the C's `arm_intr_func.raise`).
#[inline]
pub fn splraise_raw(ipl: i32) -> i32 {
    match bridges::intr_hardware() {
        Some(hw) => hw.splraise(ipl),
        None => dflt_splraise(ipl),
    }
}

/// `spllower(ipl)`: through the controller.
#[inline]
pub fn spllower(ipl: i32) -> i32 {
    match bridges::intr_hardware() {
        Some(hw) => hw.spllower(ipl),
        None => dflt_spllower(ipl),
    }
}

/// `splx(ipl)`: through the controller.
#[inline]
pub fn splx(ipl: i32) {
    match bridges::intr_hardware() {
        Some(hw) => hw.splx(ipl),
        None => dflt_splx(ipl),
    }
}

/// `softintr(si)`: marks soft interrupt `si` (`SOFTINTR_*`) pending on this CPU.
#[inline]
pub fn softintr(si: usize) {
    if si < NSOFTINTR {
        curcpu().mark_pending(si_to_irqbit(si));
    }
}

/// `splraise(ipl)` as a guard: the level is at least `ipl` until the guard drops.
pub fn splraise(ipl: i32) -> SplGuard {
    SplGuard {
        old: splraise_raw(ipl),
        _not_send: PhantomData,
    }
}

/// `splhigh()` as a guard.
pub fn splhigh() -> SplGuard {
    splraise(IPL_HIGH)
}

/// `splassert(wantipl)`: the machine's `DIAGNOSTIC` check that the running code is at
/// `wantipl` or above (named `func` in the report), through the controller's registration
/// (`bsd`'s `splassert_ctl` and `splassert_fail`); nothing before it.
pub fn splassert(wantipl: i32, func: &str) {
    if let Some(hw) = bridges::intr_hardware() {
        hw.splassert(wantipl, func);
    }
}

/// `splraise(ipl)` for this crate's locks, which keep the level to restore in their guards:
/// always `Some` (the level is `hal`'s own since N4.5c; the `Option` stays the guards' shape).
#[inline]
pub(crate) fn raise(ipl: i32) -> Option<i32> {
    Some(splraise_raw(ipl))
}

/// `splx(s)` of what [`raise`] returned.
#[inline]
pub(crate) fn restore(old: Option<i32>) {
    if let Some(s) = old {
        splx(s);
    }
}

/// `IPL_MPFLOOR`: the lowest level a mutex raises to (the C's compile-time `__MUTEX_IPL`).
#[inline]
pub fn mpfloor() -> i32 {
    IPL_MPFLOOR
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicU32;

    use super::*;
    use crate::testutil;

    fn level() -> i32 {
        curcpu().cpl()
    }

    #[test]
    fn the_guard_restores_the_level_it_replaced() {
        testutil::install();
        let base = level();
        {
            let g = splraise(base + 3);
            assert_eq!(g.old_level(), base);
            assert_eq!(level(), base + 3);
            {
                let inner = splraise(base + 1);
                // `splraise` never lowers.
                assert_eq!(level(), base + 3);
                drop(inner);
            }
            assert_eq!(level(), base + 3);
        }
        assert_eq!(level(), base);
    }

    #[test]
    fn splhigh_raises_to_ipl_high_and_spllower_lowers() {
        testutil::install();
        let base = level();
        {
            let _s = splhigh();
            assert_eq!(level(), base.max(IPL_HIGH));
            assert_eq!(spllower(IPL_NONE), base.max(IPL_HIGH));
            assert_eq!(level(), IPL_NONE);
        }
        assert_eq!(level(), base);
    }

    #[test]
    fn each_level_unmasks_the_soft_levels_above_it() {
        init_smask();
        let all = si_to_irqbit(SOFTINTR_CLOCK as usize)
            | si_to_irqbit(SOFTINTR_NET as usize)
            | si_to_irqbit(SOFTINTR_TTY as usize);
        assert_eq!(smask(IPL_NONE), all);
        assert_eq!(
            smask(IPL_SOFTCLOCK) & si_to_irqbit(SOFTINTR_CLOCK as usize),
            0
        );
        assert_ne!(
            smask(IPL_SOFTCLOCK) & si_to_irqbit(SOFTINTR_NET as usize),
            0
        );
        assert_eq!(smask(IPL_SOFTTTY), 0);
        assert_eq!(smask(IPL_HIGH), 0);
        // Past the table: nothing.
        assert_eq!(smask(99), 0);
        assert_eq!(smask(-1), 0);
    }

    static RAN: AtomicU32 = AtomicU32::new(0);

    fn count(_arg: *mut core::ffi::c_void) {
        RAN.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn lowering_the_level_runs_a_pending_soft_interrupt() {
        testutil::install();
        init_smask();
        let si = crate::softintr::establish(IPL_SOFTNET | IPL_MPSAFE, count, core::ptr::null_mut())
            .unwrap();
        let before = RAN.load(Ordering::Relaxed);
        let s = splraise_raw(IPL_HIGH);
        si.schedule();
        assert_ne!(curcpu().ipending() & si_to_irqbit(SOFTINTR_NET as usize), 0);
        // Still masked at IPL_HIGH and at IPL_SOFTNET itself.
        splx(IPL_SOFTNET);
        assert_eq!(RAN.load(Ordering::Relaxed), before);
        // Below it: run once, then gone.
        splx(IPL_NONE);
        assert_eq!(RAN.load(Ordering::Relaxed), before + 1);
        assert_eq!(curcpu().ipending() & si_to_irqbit(SOFTINTR_NET as usize), 0);
        assert_eq!(level(), IPL_NONE);
        splx(s);
        assert_eq!(si.disestablish(), None);
        si.release();
    }
}
/* </TESTS> */
