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
//! ([`do_pending_intr`], which runs `crate::cpu::softintr::dispatch`). Which soft levels a level
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
//! - N7.2a1: `IPL_AUDIO`, `IPL_STATCLOCK`, `IPL_IPI`, `IPL_WAKEUP` and arm64's `IPL_IRQMASK`
//!   and `IPL_FLAGMASK` join the levels, for the interrupt core in the kernel
//!   (`kernel::dev::arm64::intr`) and the controllers after it.
//! - N7.2a2: the controller's registration is this module's ([`install`], the C's
//!   `arm_set_intr_handler`, the `cpu::trap::install` shape), made by the controller that
//!   attaches (`kernel::dev::arm64::ampintc`; until N7.2a3 also `bsd`'s `arm_set_intr_handler`
//!   for the legacy GICv3) instead of by `initarm` over `bsd`'s table: before one attaches, the
//!   defaults run, as the C's table starts. The dispatch takes the clock frame (`ClockFrame`, the machine's trap frame on
//!   arm64), which the kernel's handlers read; [`unmasked`] is the dispatch's
//!   `intr_enable()`/`intr_disable()` pair around its handlers, its premise a type
//!   ([`InIrq`], the dispatch's token). `arm_splassert_check` is this module's
//!   ([`splassert_check`], the trait's default and the check before a controller attaches;
//!   the report through `hal::prf::splassert_fail`), so every controller checks the same.
//! - N7.2a3: the table's last two entries, `enable_wakeup` and `disable_wakeup`, are the
//!   trait's ([`IntrHardware::enable_wakeup`], [`IntrHardware::disable_wakeup`], the C's NULL
//!   as the default), so the GICv3 in the kernel fills the whole table; `bsd`'s copy of it
//!   (`ArmIntrHardware` over `arm_intr_func`) is gone, and `hal::bridges` no longer forwards
//!   a registration here.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::bridges::AlreadyRegistered;
use crate::cpu::curcpu;
use crate::sync::once::OnceStatic;
use sysdefs::dev::softintr::{NSOFTINTR, SOFTINTR_CLOCK, SOFTINTR_NET, SOFTINTR_TTY};

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
    /// `IPL_NET` (N6.1: the mbuf pools and queues).
    pub const IPL_NET: i32 = 0x4;
    /// `IPL_SOFTTTY`.
    pub const IPL_SOFTTTY: i32 = 0x8;
    /// `IPL_TTY`.
    pub const IPL_TTY: i32 = 0x9;
    /// `IPL_VM`.
    pub const IPL_VM: i32 = 0xa;
    /// `IPL_AUDIO`.
    pub const IPL_AUDIO: i32 = 0xb;
    /// `IPL_SCHED` (`IPL_CLOCK`).
    pub const IPL_SCHED: i32 = 0xc;
    /// `IPL_HIGH`.
    pub const IPL_HIGH: i32 = 0xd;
    /// `IPL_IPI`: inter-processor interrupts.
    pub const IPL_IPI: i32 = 0xe;
    /// `IPL_MPFLOOR`: `IPL_TTY`.
    pub const IPL_MPFLOOR: i32 = 0x9;
    /// `NIPL`: the levels' count.
    pub const NIPL: usize = 16;
    /// `IPL_IRQMASK`: the level part of a level and its flags (arm64's name; the host
    /// compiles arm64's interrupt core for its tests).
    pub const IPL_IRQMASK: i32 = 0xf;
    /// `IPL_FLAGMASK`: the flags part (arm64's name, as `IPL_IRQMASK`).
    pub const IPL_FLAGMASK: i32 = 0xf00;
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
    /// `IPL_NET` (N6.1: the mbuf pools and queues).
    pub const IPL_NET: i32 = 6;
    /// `IPL_TTY`.
    pub const IPL_TTY: i32 = 7;
    /// `IPL_VM`.
    pub const IPL_VM: i32 = 8;
    /// `IPL_AUDIO`.
    pub const IPL_AUDIO: i32 = 9;
    /// `IPL_SCHED` (`IPL_CLOCK`).
    pub const IPL_SCHED: i32 = 10;
    /// `IPL_HIGH`.
    pub const IPL_HIGH: i32 = 11;
    /// `IPL_IPI`: inter-processor interrupts.
    pub const IPL_IPI: i32 = 12;
    /// `IPL_MPFLOOR`: `IPL_TTY`.
    pub const IPL_MPFLOOR: i32 = 7;
    /// `NIPL`: the levels' count.
    pub const NIPL: usize = 13;
    /// `IPL_IRQMASK`: the level part of a level and its flags.
    pub const IPL_IRQMASK: i32 = 0xf;
    /// `IPL_FLAGMASK`: the flags part.
    pub const IPL_FLAGMASK: i32 = 0xf00;
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
    /// `IPL_NET` (`<machine/intrdefs.h>`'s value; the boundary designed for both, decision 44).
    pub const IPL_NET: i32 = 0x4;
    /// `IPL_SOFTTTY`.
    pub const IPL_SOFTTTY: i32 = 0x8;
    /// `IPL_TTY`.
    pub const IPL_TTY: i32 = 0x9;
    /// `IPL_VM`.
    pub const IPL_VM: i32 = 0xa;
    /// `IPL_AUDIO`.
    pub const IPL_AUDIO: i32 = 0xb;
    /// `IPL_SCHED` (`IPL_CLOCK`).
    pub const IPL_SCHED: i32 = 0xc;
    /// `IPL_HIGH`.
    pub const IPL_HIGH: i32 = 0xd;
    /// `IPL_IPI`: inter-processor interrupts.
    pub const IPL_IPI: i32 = 0xe;
    /// `IPL_MPFLOOR`: `IPL_TTY`.
    pub const IPL_MPFLOOR: i32 = 0x9;
    /// `NIPL`: the levels' count.
    pub const NIPL: usize = 16;
}

pub use levels::{
    IPL_AUDIO, IPL_BIO, IPL_HIGH, IPL_IPI, IPL_MPFLOOR, IPL_NET, IPL_SCHED, IPL_SOFTCLOCK,
    IPL_SOFTNET, IPL_SOFTTTY, IPL_TTY, IPL_VM, NIPL,
};
// `<machine/intrdefs.h>` (amd64) has no `IPL_IRQMASK` or `IPL_FLAGMASK`: they are arm64's
// (`<machine/intr.h>`), and the host's for the tests of arm64's interrupt core.
#[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
pub use levels::{IPL_FLAGMASK, IPL_IRQMASK};

/// `IPL_CLOCK`: the clock interrupt's level, `IPL_SCHED` on both machines (`<machine/intr.h>`).
pub const IPL_CLOCK: i32 = IPL_SCHED;

/// `IPL_STATCLOCK`: the statistics clock's level, `IPL_CLOCK` on both machines.
pub const IPL_STATCLOCK: i32 = IPL_CLOCK;

/// `IPL_MPSAFE`: an "mpsafe" interrupt or soft interrupt, run without the kernel lock (a flag
/// beside the level, both machines).
pub const IPL_MPSAFE: i32 = 0x100;

/// `IPL_WAKEUP`: an interrupt that may wake the machine from suspend (a flag beside the level,
/// both machines).
pub const IPL_WAKEUP: i32 = 0x200;

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

/// `arm_intr_func`: the attached controller, set once by its attach ([`install`]).
static HARDWARE: OnceStatic<&'static dyn IntrHardware> = OnceStatic::new();

/// The machine's interrupt controller (`struct arm_intr_func` and `arm_set_intr_handler`'s
/// table), as the controller's attach installs it ([`install`]: the kernel's GIC driver
/// itself, `kernel::dev::arm64::ampintc` or `agintc`). Every method's default is the C's
/// `arm_dflt_*` (or its NULL entry), what runs before a controller attaches.
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
    /// `arm_splassert_check(wantipl, func)` behind `splassert`: reports a caller of `func`
    /// running below `wantipl` ([`splassert_check`], the C's generic check, which every
    /// controller shares).
    fn splassert(&self, wantipl: i32, func: &str) {
        splassert_check(wantipl, func);
    }
    /// `intr_enable_wakeup()` (`arm_intr_func.enable_wakeup`): before the machine sleeps,
    /// every interrupt no handler established with `IPL_WAKEUP` is masked; the default is the
    /// C's NULL hook, nothing.
    fn enable_wakeup(&self) {}
    /// `intr_disable_wakeup()` (`arm_intr_func.disable_wakeup`): after the resume; the default
    /// nothing.
    fn disable_wakeup(&self) {}
    /// `(*arm_irq_dispatch)(frame)`: the controller's IRQ dispatch, which `arm_cpu_irq` (the
    /// machine's trap code) calls with `ci_idepth` raised and this CPU's interrupts masked
    /// (`irq`, the proof [`unmasked`] asks for); the default is `arm_dflt_irq`, whose
    /// `panic(9)` is what an interrupt before a controller attached meets.
    #[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
    fn dispatch_irq(&self, frame: &mut crate::cpu::clock::ClockFrame, irq: &InIrq) {
        let _ = (frame, irq);
        crate::assert::hal_panic(format_args!("arm_dflt_irq"));
    }
    /// `(*arm_fiq_dispatch)(frame)`, as [`IntrHardware::dispatch_irq`] (`arm_dflt_fiq`).
    #[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
    fn dispatch_fiq(&self, frame: &mut crate::cpu::clock::ClockFrame) {
        let _ = frame;
        crate::assert::hal_panic(format_args!("arm_dflt_fiq"));
    }
}

/// The IRQ dispatch under way on this CPU, with its interrupts masked by the exception entry:
/// made only by `hal`'s IRQ entry ([`dispatch_irq`]) and lent to the controller's
/// [`IntrHardware::dispatch_irq`] for the call, so [`unmasked`]'s premise (entered masked, in
/// a dispatch) is a type. Not `Send`, not `Clone`, and a `'static` one cannot be had (it
/// lives on the entry's frame). The host double has no interrupts: its constructor is public
/// there ([`InIrq::host`]), for the tests of a controller's dispatch.
pub struct InIrq {
    _not_send: PhantomData<*const ()>,
}

impl InIrq {
    /// The token of a dispatch `hal`'s IRQ entry makes.
    #[cfg_attr(
        not(target_os = "none"),
        allow(dead_code, reason = "the machine's IRQ entry alone makes one")
    )]
    fn entry() -> InIrq {
        InIrq {
            _not_send: PhantomData,
        }
    }

    /// The host double's token (a host thread takes no interrupt, so nothing is masked or
    /// unmasked): for a test that runs a controller's dispatch by hand.
    #[cfg(not(target_os = "none"))]
    pub fn host() -> InIrq {
        InIrq::entry()
    }
}

/// `arm_set_intr_handler(...)`: the attaching controller becomes the machine's, once; a
/// second call keeps the first and names it (one controller drives the machine's
/// interrupts, as the C's single table).
pub fn install(hw: &'static dyn IntrHardware) -> Result<(), AlreadyRegistered> {
    HARDWARE
        .set(hw)
        .map_err(|_| AlreadyRegistered("IntrHardware"))
}

/// The installed controller, `None` before one attached (the C's table still holding the
/// `arm_dflt_*` entries).
#[inline]
pub fn hardware() -> Option<&'static dyn IntrHardware> {
    HARDWARE.get().copied()
}

/// `intr_enable(); f(); intr_disable();`: the IRQ dispatch's pair around its handlers
/// (`ampintc_irq_handler`, `agintc_irq_handler`), inside a dispatch (`irq`), entered with
/// this CPU's interrupts masked (the exception entry's state): unmasked while `f` runs (a
/// higher-priority interrupt may nest), masked again after it, as the dispatch found them.
/// Not a guard: a guard's drop would restore the state it saved, and the dispatch must
/// return masked.
#[inline]
pub fn unmasked<R>(irq: &InIrq, f: impl FnOnce() -> R) -> R {
    let _ = irq;
    crate::cpu::intr_enable();
    let r = f();
    let _ = crate::cpu::intr_disable_raw();
    r
}

/// `arm_cpu_irq`'s dispatch: through the registered controller, `arm_dflt_irq`'s `panic(9)`
/// before one.
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[cfg_attr(
    not(target_os = "none"),
    allow(dead_code, reason = "the machine's IRQ entry alone calls it")
)]
pub(crate) fn dispatch_irq(frame: &mut crate::cpu::clock::ClockFrame) {
    match hardware() {
        Some(hw) => hw.dispatch_irq(frame, &InIrq::entry()),
        None => crate::assert::hal_panic(format_args!("arm_dflt_irq")),
    }
}

/// `arm_cpu_fiq`'s dispatch, as [`dispatch_irq`].
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[cfg_attr(
    not(target_os = "none"),
    allow(dead_code, reason = "the machine's FIQ entry alone calls it")
)]
pub(crate) fn dispatch_fiq(frame: &mut crate::cpu::clock::ClockFrame) {
    match hardware() {
        Some(hw) => hw.dispatch_fiq(frame),
        None => crate::assert::hal_panic(format_args!("arm_dflt_fiq")),
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
    match hardware() {
        Some(hw) => hw.setipl(ipl),
        None => dflt_setipl(ipl),
    }
}

/// `arm_do_pending_intr(pcpl)`: runs the soft interrupts pending on this CPU that `pcpl`
/// leaves unmasked, highest first, each at its level with the machine's interrupts on, until
/// none is left; then the level is `pcpl`. Each pass runs what one snapshot of `ci_ipending`
/// holds, as the C's: a soft interrupt a handler raises during the pass runs in the next
/// one (a NET raised by a TTY handler after the pass's CLOCK).
pub fn do_pending_intr(pcpl: i32) {
    let c = curcpu();
    let mut masked = Some(crate::cpu::intr_disable());

    loop {
        let ipending = c.ipending() & smask(pcpl);
        for (si, ipl) in pass(ipending) {
            c.clear_pending(si_to_irqbit(si));
            setipl(ipl);
            drop(masked.take());
            crate::cpu::softintr::dispatch(si);
            masked = Some(crate::cpu::intr_disable());
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

/// One pass of [`do_pending_intr`] over the snapshot `ipending`: the soft interrupts it holds,
/// with the level each runs at, highest first (`do_softint`'s order: TTY, NET, CLOCK).
fn pass(ipending: u64) -> impl Iterator<Item = (usize, i32)> {
    [
        (SOFTINTR_TTY as usize, IPL_SOFTTTY),
        (SOFTINTR_NET as usize, IPL_SOFTNET),
        (SOFTINTR_CLOCK as usize, IPL_SOFTCLOCK),
    ]
    .into_iter()
    .filter(move |&(si, _)| ipending & si_to_irqbit(si) != 0)
}

/// `splraise(ipl)`: raises this CPU's level to at least `ipl`; the old level (`bsd`'s
/// `splraise`, through the controller as the C's `arm_intr_func.raise`).
#[inline]
pub fn splraise_raw(ipl: i32) -> i32 {
    match hardware() {
        Some(hw) => hw.splraise(ipl),
        None => dflt_splraise(ipl),
    }
}

/// `spllower(ipl)`: through the controller.
#[inline]
pub fn spllower(ipl: i32) -> i32 {
    match hardware() {
        Some(hw) => hw.spllower(ipl),
        None => dflt_spllower(ipl),
    }
}

/// `splx(ipl)`: through the controller.
#[inline]
pub fn splx(ipl: i32) {
    match hardware() {
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

/// `splassert(wantipl)`: the machine's check that the running code is at `wantipl` or above
/// (named `func` in the report), through the controller's registration; before one, the
/// generic check on the machine ([`splassert_check`], with the default `setipl`, as LZ's
/// table did from `initarm` on), nothing on the host double without one.
pub fn splassert(wantipl: i32, func: &str) {
    match hardware() {
        Some(hw) => hw.splassert(wantipl, func),
        #[cfg(target_os = "none")]
        None => splassert_check(wantipl, func),
        #[cfg(not(target_os = "none"))]
        None => {}
    }
}

/// `arm_splassert_check(wantipl, func)`: a level below `wantipl` reported
/// (`splassert_fail`, `hal::prf`'s, the kernel's `splassert_ctl` policy behind it) and, if
/// that returns, the level raised to `wantipl` in a feeble attempt to reduce damage; and
/// `IPL_NONE` asked inside an interrupt reported (`-1` and the depth).
pub fn splassert_check(wantipl: i32, func: &str) {
    let c = curcpu();
    let oldipl = c.cpl();

    if oldipl < wantipl {
        crate::prf::splassert_fail(wantipl, oldipl, func);
        // If the splassert_ctl is set to not panic, raise the ipl in a feeble attempt to
        // reduce damage.
        setipl(wantipl);
    }

    if wantipl == IPL_NONE && c.idepth() != 0 {
        crate::prf::splassert_fail(-1, c.idepth() as i32, func);
    }
}

/// `splraise(ipl)` for this crate's locks, which keep the level to restore in their guards:
/// always `Some` (the level is `hal`'s own since N4.5c; the `Option` stays the guards' shape).
#[inline]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the guards' shape: restore takes what raise gave"
)]
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

    struct Second;

    impl IntrHardware for Second {}

    static SECOND: Second = Second;

    #[test]
    fn one_controller_is_installed_and_the_dispatch_pair_returns_what_its_body_does() {
        testutil::install();
        assert!(hardware().is_some());
        assert_eq!(install(&SECOND), Err(AlreadyRegistered("IntrHardware")));
        let mut ran = false;
        assert_eq!(
            unmasked(&InIrq::host(), || {
                ran = true;
                7
            }),
            7
        );
        assert!(ran);
    }

    #[test]
    fn the_generic_splassert_check_raises_a_level_found_below_its_want() {
        testutil::install();
        let c = curcpu();
        let before = c.cpl();
        c.set_cpl(IPL_NONE);
        splassert_check(IPL_NONE, "fine");
        assert_eq!(c.cpl(), IPL_NONE, "at the level wanted: nothing to do");
        splassert_check(IPL_BIO, "low");
        assert_eq!(
            c.cpl(),
            IPL_BIO,
            "reported, then raised to the level wanted"
        );
        c.set_cpl(before);
    }

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
        let si =
            crate::cpu::softintr::establish(IPL_SOFTNET | IPL_MPSAFE, count, core::ptr::null_mut())
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

    /// A pass runs what its snapshot holds, highest first; a level raised during it waits
    /// for the next pass (review finding 3: the C reads `ci_ipending` once per pass).
    #[test]
    fn a_pass_runs_its_snapshot_highest_first() {
        let bit = |si: i32| si_to_irqbit(si as usize);
        let order = |p: u64| pass(p).map(|(si, _)| si).collect::<std::vec::Vec<_>>();
        let (tty, net, clock) = (
            SOFTINTR_TTY as usize,
            SOFTINTR_NET as usize,
            SOFTINTR_CLOCK as usize,
        );
        // A TTY handler that raises NET: the first pass's snapshot is TTY and CLOCK, so NET
        // runs after CLOCK, in the second pass.
        assert_eq!(order(bit(SOFTINTR_TTY) | bit(SOFTINTR_CLOCK)), [tty, clock]);
        assert_eq!(order(bit(SOFTINTR_NET)), [net]);
        assert_eq!(
            order(bit(SOFTINTR_CLOCK) | bit(SOFTINTR_NET) | bit(SOFTINTR_TTY)),
            [tty, net, clock]
        );
        assert!(order(0).is_empty());
        let levels: std::vec::Vec<_> = pass(u64::MAX).map(|(_, ipl)| ipl).collect();
        assert_eq!(levels, [IPL_SOFTTTY, IPL_SOFTNET, IPL_SOFTCLOCK]);
    }
}
/* </TESTS> */
