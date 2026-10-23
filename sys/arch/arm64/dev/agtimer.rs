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
 * Copyright (c) 2013 Patrick Wildt <patrick@blueri.se>
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
//! The ARM generic timer: `arch/arm64/dev/agtimer.c`. The virtual counter (`CNTVCT_EL0`) is
//! the timecounter and the virtual timer (`CNTV_TVAL_EL0`) the interrupt clock, through the
//! PPI the device tree names.
//!
//! Upstream: sys/arch/arm64/dev/agtimer.c @ 3ce1f3f79392
//!
//! Status: `ported`. Milestone M5 ports the whole driver: `agtimer_readcnt64_{default,sun50i}`,
//! the register accessors, `agtimer_match`, `agtimer_attach`, `agtimer_get_timecount_*`,
//! `agtimer_rearm`, `agtimer_trigger`, `agtimer_intr`, `agtimer_set_clockrate`,
//! `agtimer_cpu_initclocks`, `agtimer_delay`, `agtimer_setstatclockrate`,
//! `agtimer_startclock` and `agtimer_init`. M11b checks it on every CPU: each application
//! processor routes the timer's PPI to itself in `agtimer_startclock` and runs its own
//! clock interrupt queue.
//!
//! ## Deviations
//! - One static softc (`AGTIMER`, the C's `agtimer_cd.cd_devs[0]`) holds what
//!   `struct agtimer_softc` adds to the device, so `agtimer_ca`'s `ca_devsize` is a bare
//!   `struct device`; mainbus attaches it from the device tree (`agtimer* at fdt?`).
//!   `agtimer_intrclock`'s cookie stays null and the callbacks read the softc directly.
//! - Not in the C: `agtimer_evtstrm_enable` turns on the timer's event stream
//!   (`CNTKCTL_EL1.EVNTEN`, counter bit 12 at QEMU's 62.5 MHz: a `wfe` wake-up every
//!   131 µs). Each application processor calls it first thing (`cpu_hatch_entry`), and
//!   `agtimer_startclock` calls it again after the C's `CNTKCTL_EL1` write, which clears it.
//!   QEMU's TCG can lose the `sev` that ends a `wfe` wait (its `sev` helper kicks the halted
//!   vCPU without the BQL, racing the vCPU thread's idle check); with no interrupt pending,
//!   the processor then sleeps forever. The event stream's wake-up comes from a QEMU timer
//!   and is reliable, so every `wfe` loop goes on checking its condition. On hardware it
//!   only costs a few spurious `wfe` returns; Linux keeps it on for the same reason.

use core::cell::Cell;
use core::ffi::c_void;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use hal::arch::arm64::timer;
use hal::clock::ClockFrame;
use kernel::clockintr::Intrclock;
use kernel::tc::{TcEntry, Timecounter};

use crate::arch::arm64::arm64::intr::{
    arm_clock_register, arm_intr_establish_fdt_idx, arm_intr_route,
};
use crate::arch::arm64::include::armreg::{
    CNTKCTL_EL0VCTEN, CNTKCTL_EVNTDIR, CNTKCTL_EVNTEN, CNTKCTL_EVNTI_MASK, CNTKCTL_EVNTI_SHIFT,
    CNTV_CTL_ENABLE, CNTV_CTL_IMASK, CURRENTEL_EL_EL2, CURRENTEL_EL_MASK,
};
use crate::arch::arm64::include::cpu::{cpu_is_primary, curcpu};
use crate::arch::arm64::include::fdt::FdtAttachArgs;
use crate::arch::arm64::include::intr::{IPL_CLOCK, IPL_MPSAFE, MachineIntrHandle};
use crate::arch::arm64::include::timetc::{TC_AGTIMER, TC_AGTIMER_SUN50I};
use crate::dev::ofw::openfirm::{OF_getindex, OF_getpropbool, OF_getpropint, OF_is_compatible};
use crate::kern::kern_clock::{PROFHZ, STATCLOCK_IS_RANDOMIZED, STATHZ};
use crate::kern::kern_clockintr::{clockintr_cpu_init, clockintr_dispatch, clockintr_trigger};
use crate::kprintf;
use crate::sys::device::{CfMatch, Cfattach, Cfdriver, DV_DULL, Device};

/// `TIMER_FREQUENCY`: ARM core clock.
const TIMER_FREQUENCY: i32 = 24 * 1000 * 1000;

/// `agtimer_frequency`.
pub static AGTIMER_FREQUENCY: AtomicI32 = AtomicI32::new(TIMER_FREQUENCY);

/// The generic timer's virtual counter as the kernel's timecounters read it: the hardware's
/// half of `agtimer_timecounter` (N4.8a: `kernel::tc::Timecounter`). The frequency is the
/// attach's (`clock-frequency`, `CNTFRQ_EL0`); the Allwinner A64's erratum switches the
/// reader (`hal`'s flag) and the `tc_user` userland's libc reads.
pub struct AgtimerTc {
    /// `tc_frequency`.
    freq: AtomicU64,
    /// `tc_get_timecount == agtimer_get_timecount_sun50i`, `tc_user == TC_AGTIMER_SUN50I`.
    sun50i: AtomicBool,
}

impl Timecounter for AgtimerTc {
    fn get_timecount(&self) -> u32 {
        // `hal`'s timecount reads around the A64 erratum once the attach chose it.
        agtimer_get_timecount_default()
    }

    fn counter_mask(&self) -> u32 {
        0xffff_ffff
    }

    fn frequency(&self) -> u64 {
        self.freq.load(Ordering::Relaxed)
    }

    fn name(&self) -> &'static str {
        "agtimer"
    }

    fn quality(&self) -> i32 {
        0
    }

    fn user(&self) -> i32 {
        if self.sun50i.load(Ordering::Relaxed) {
            TC_AGTIMER_SUN50I
        } else {
            TC_AGTIMER
        }
    }
}

/// The counter's hardware half.
static AGTIMER_TC: AgtimerTc = AgtimerTc {
    freq: AtomicU64::new(0),
    sun50i: AtomicBool::new(false),
};

/// `agtimer_timecounter`: the kernel's record of the counter.
static AGTIMER_TIMECOUNTER: TcEntry = TcEntry::new(&AGTIMER_TC);

/// `struct agtimer_softc`.
pub struct AgtimerSoftc {
    /// `sc_node`.
    pub sc_node: Cell<i32>,
    /// `sc_ticks_per_second`.
    pub sc_ticks_per_second: Cell<u32>,
    /// `sc_nsec_cycle_ratio`.
    pub sc_nsec_cycle_ratio: Cell<u64>,
    /// `sc_nsec_max`.
    pub sc_nsec_max: Cell<u64>,
    /// `sc_ih`.
    pub sc_ih: Cell<Option<NonNull<MachineIntrHandle>>>,
    /// Whether `agtimer_attach` ran (`agtimer_cd.cd_devs[0] != NULL`).
    pub sc_attached: AtomicBool,
}

// SAFETY: written at attach on the boot CPU; read by the clock paths afterwards.
unsafe impl Sync for AgtimerSoftc {}

/// `agtimer_ca`.
pub static AGTIMER_CA: Cfattach = Cfattach {
    ca_devsize: size_of::<Device>(),
    ca_match: Some(agtimer_match),
    ca_attach: agtimer_attach,
    ca_detach: None,
    ca_activate: None,
};

/// `agtimer_cd`.
pub static AGTIMER_CD: Cfdriver = Cfdriver::new(b"agtimer", DV_DULL, 0);

/// `agtimer_cd.cd_devs[0]`: the one generic timer.
static AGTIMER: AgtimerSoftc = AgtimerSoftc {
    sc_node: Cell::new(0),
    sc_ticks_per_second: Cell::new(0),
    sc_nsec_cycle_ratio: Cell::new(0),
    sc_nsec_max: Cell::new(0),
    sc_ih: Cell::new(None),
    sc_attached: AtomicBool::new(false),
};

/// `agtimer_intrclock`: the virtual timer as each CPU's interrupt clock (N4.8a:
/// `kernel::clockintr::Intrclock`; the C's null cookie: the callbacks read [`AGTIMER`]).
pub struct AgtimerIntrclock;

impl Intrclock for AgtimerIntrclock {
    fn rearm(&self, nsecs: u64) {
        agtimer_rearm(nsecs);
    }

    fn trigger(&self) {
        agtimer_trigger();
    }
}

/// `agtimer_intrclock`.
pub static AGTIMER_INTRCLOCK: AgtimerIntrclock = AgtimerIntrclock;

/// `agtimer_readcnt64_default`: reads `CNTVCT_EL0` (`hal`'s register access, around the
/// Cortex-A73 erratum 858921).
pub fn agtimer_readcnt64_default() -> u64 {
    timer::readcnt64_default()
}

/// `agtimer_readcnt64_sun50i`: reads the counter around the Allwinner A64 erratum.
pub fn agtimer_readcnt64_sun50i() -> u64 {
    timer::readcnt64_sun50i()
}

/// `agtimer_readcnt64()`: the counter through the reader the attach chose.
pub fn agtimer_readcnt64() -> u64 {
    timer::readcnt64()
}

/// `agtimer_get_freq`: `CNTFRQ_EL0`.
#[inline]
fn agtimer_get_freq() -> u64 {
    timer::cntfrq()
}

/// `agtimer_get_ctrl`: `CNTV_CTL_EL0`.
#[inline]
fn agtimer_get_ctrl() -> u32 {
    timer::cntv_ctl()
}

/// `agtimer_set_ctrl`: writes `CNTV_CTL_EL0`.
#[inline]
fn agtimer_set_ctrl(val: u32) {
    timer::set_cntv_ctl(val);
}

/// `agtimer_set_tval`: writes `CNTV_TVAL_EL0`.
#[inline]
fn agtimer_set_tval(val: u32) {
    timer::set_cntv_tval(val);
}

/// `agtimer_match`.
pub fn agtimer_match(_parent: Option<&Device>, _cfdata: &CfMatch, aux: *mut c_void) -> i32 {
    // SAFETY: `agtimer` attaches at `fdt`, whose buses hand over a `FdtAttachArgs`.
    let faa = unsafe { &*aux.cast::<FdtAttachArgs<'_>>() };
    i32::from(
        OF_is_compatible(faa.fa_node, b"arm,armv7-timer")
            || OF_is_compatible(faa.fa_node, b"arm,armv8-timer"),
    )
}

/// `agtimer_attach`.
pub fn agtimer_attach(_parent: Option<&Device>, _self: &Device, aux: *mut c_void) {
    let sc = &AGTIMER;
    // SAFETY: as in `agtimer_match`.
    let faa = unsafe { &*aux.cast::<FdtAttachArgs<'_>>() };

    sc.sc_node.set(faa.fa_node);

    if agtimer_get_freq() != 0 {
        AGTIMER_FREQUENCY.store(agtimer_get_freq() as i32, Ordering::Relaxed);
    }
    AGTIMER_FREQUENCY.store(
        OF_getpropint(
            sc.sc_node.get(),
            b"clock-frequency",
            AGTIMER_FREQUENCY.load(Ordering::Relaxed) as u32,
        ) as i32,
        Ordering::Relaxed,
    );
    sc.sc_ticks_per_second
        .set(AGTIMER_FREQUENCY.load(Ordering::Relaxed) as u32);
    sc.sc_nsec_cycle_ratio
        .set(u64::from(sc.sc_ticks_per_second.get()) * (1u64 << 32) / 1_000_000_000);
    sc.sc_nsec_max.set(u64::MAX / sc.sc_nsec_cycle_ratio.get());

    kprintf!(": {} kHz\n", sc.sc_ticks_per_second.get() / 1000);

    // The Allwinner A64 has an erratum where the bottom 9 bits of the counter register can't
    // be trusted if any of the higher bits are rolling over.
    if OF_getpropbool(sc.sc_node.get(), b"allwinner,erratum-unknown1") {
        timer::set_readcnt64_sun50i();
        AGTIMER_TC.sun50i.store(true, Ordering::Relaxed);
    }

    // private timer and interrupts not enabled until timer configures

    arm_clock_register(
        Some(agtimer_cpu_initclocks),
        agtimer_delay,
        Some(agtimer_setstatclockrate),
        Some(agtimer_startclock),
    );

    AGTIMER_TC
        .freq
        .store(u64::from(sc.sc_ticks_per_second.get()), Ordering::Relaxed);
    // tc_priv = sc: the counter reads no softc (the hardware half is `AGTIMER_TC`).
    kernel::tc::tc_init(&AGTIMER_TIMECOUNTER);

    // agtimer_intrclock.ic_cookie = sc: the callbacks read AGTIMER (see the deviations).
    sc.sc_attached.store(true, Ordering::Relaxed);
}

/// `agtimer_get_timecount_default`: the low 32 bits of the counter.
pub fn agtimer_get_timecount_default() -> u32 {
    timer::timecount()
}

/// `agtimer_get_timecount_sun50i`.
pub fn agtimer_get_timecount_sun50i() -> u32 {
    agtimer_readcnt64_sun50i() as u32
}

/// `agtimer_rearm`: fires the timer in `nsecs`.
pub fn agtimer_rearm(nsecs: u64) {
    let sc = &AGTIMER;

    let nsecs = nsecs.min(sc.sc_nsec_max.get());
    let cycles = (nsecs.wrapping_mul(sc.sc_nsec_cycle_ratio.get()) >> 32).min(i32::MAX as u64);
    agtimer_set_tval(cycles as u32);
}

/// `agtimer_trigger`: fires the timer now.
pub fn agtimer_trigger() {
    agtimer_set_tval(0);
}

/// `agtimer_intr`: the timer's interrupt handler.
pub fn agtimer_intr(frame: *mut c_void) -> i32 {
    // The C never dispatches without a frame; LZ read a null one as not from user mode,
    // which a kernel-address `tf_elr` keeps.
    const KERNEL_FRAME: ClockFrame = ClockFrame {
        tf_sp: 0,
        tf_lr: 0,
        tf_elr: 1 << 63,
        tf_spsr: 0,
        tf_x: [0; 30],
    };
    // SAFETY: the interrupt controller's dispatch hands its handlers the trap frame
    // `arm_cpu_irq` received from the IRQ entry (`exception.S`), saved on this CPU's stack
    // and alive for the handler's whole call (it is the entry's frame, popped only after the
    // dispatch returns); it is read here, never written, and only for the call.
    let frame = unsafe { frame.cast::<ClockFrame>().as_ref() }.unwrap_or(&KERNEL_FRAME);
    i32::from(clockintr_dispatch(frame))
}

/// `agtimer_set_clockrate`: a new counter frequency.
pub fn agtimer_set_clockrate(new_frequency: i32) {
    let sc = &AGTIMER;

    AGTIMER_FREQUENCY.store(new_frequency, Ordering::Relaxed);

    if !sc.sc_attached.load(Ordering::Relaxed) {
        return;
    }

    sc.sc_ticks_per_second
        .set(AGTIMER_FREQUENCY.load(Ordering::Relaxed) as u32);
    sc.sc_nsec_cycle_ratio
        .set(u64::from(sc.sc_ticks_per_second.get()) * (1u64 << 32) / 1_000_000_000);
    sc.sc_nsec_max.set(u64::MAX / sc.sc_nsec_cycle_ratio.get());

    AGTIMER_TC
        .freq
        .store(u64::from(sc.sc_ticks_per_second.get()), Ordering::Relaxed);
    AGTIMER_TIMECOUNTER.set_frequency(u64::from(sc.sc_ticks_per_second.get()));

    kprintf!(
        "agtimer0: adjusting clock: new tick rate {} kHz\n",
        sc.sc_ticks_per_second.get() / 1000
    );
}

/// `agtimer_cpu_initclocks`: `cpu_initclocks` on the generic timer.
pub fn agtimer_cpu_initclocks() {
    let sc = &AGTIMER;

    let hz = crate::conf::param::HZ.load(Ordering::Relaxed);
    STATHZ.store(hz, Ordering::Relaxed);
    PROFHZ.store(hz * 10, Ordering::Relaxed);
    STATCLOCK_IS_RANDOMIZED.store(true, Ordering::Relaxed);

    if sc.sc_ticks_per_second.get() != AGTIMER_FREQUENCY.load(Ordering::Relaxed) as u32 {
        agtimer_set_clockrate(AGTIMER_FREQUENCY.load(Ordering::Relaxed));
    }

    // Pick the correct PPI depending on the running EL.
    let el = timer::current_el() & CURRENTEL_EL_MASK;
    let mut idx = if el == CURRENTEL_EL_EL2 {
        OF_getindex(sc.sc_node.get(), Some(b"hyp-virt"), b"interrupt-names")
    } else {
        OF_getindex(sc.sc_node.get(), Some(b"virt"), b"interrupt-names")
    };
    if idx == -1 {
        idx = if el == CURRENTEL_EL_EL2 { 4 } else { 2 };
    }

    // configure virtual timer interrupt
    sc.sc_ih.set(arm_intr_establish_fdt_idx(
        sc.sc_node.get(),
        idx as usize,
        IPL_CLOCK | IPL_MPSAFE,
        agtimer_intr,
        ptr::null_mut(),
        "tick",
    ));
}

/// `agtimer_delay`: busy-waits `usecs` on the counter.
pub fn agtimer_delay(usecs: u32) {
    let start = agtimer_readcnt64();
    let cycles = u64::from(usecs) * AGTIMER_FREQUENCY.load(Ordering::Relaxed) as u64 / 1_000_000;
    while agtimer_readcnt64().wrapping_sub(start) < cycles {
        core::hint::spin_loop(); // CPU_BUSY_CYCLE()
    }
}

/// `agtimer_setstatclockrate`: the statclock shares the timer; nothing to change.
pub fn agtimer_setstatclockrate(_newhz: i32) {}

/// `agtimer_startclock`: starts the clock interrupts on this CPU.
pub fn agtimer_startclock() {
    let sc = &AGTIMER;

    if !cpu_is_primary(curcpu())
        && let Some(ih) = sc.sc_ih.get()
    {
        arm_intr_route(ih, true, curcpu());
    }

    clockintr_cpu_init(Some(&AGTIMER_INTRCLOCK));
    // The kernel's dispatch checks every uptime it reads under the smokes (feature qemu).
    kernel::clockintr::set_uptime_check(cfg!(feature = "qemu"));

    let mut reg = agtimer_get_ctrl();
    reg &= !CNTV_CTL_IMASK;
    reg |= CNTV_CTL_ENABLE;
    agtimer_set_tval(i32::MAX as u32);
    agtimer_set_ctrl(reg);

    clockintr_trigger();

    // enable userland access to virtual counter
    timer::set_cntkctl(CNTKCTL_EL0VCTEN);
    // Not in the C: the write above clears the event stream; keep it on (see the deviations).
    agtimer_evtstrm_enable();
}

/// `agtimer_init`: what `mainbus` does before the children attach: the counter's frequency
/// and `agtimer_delay` as `delay(9)`.
pub fn agtimer_init() {
    // XXX: Check for Generic Timer support.
    let cntfrq = agtimer_get_freq();

    if cntfrq != 0 {
        AGTIMER_FREQUENCY.store(cntfrq as i32, Ordering::Relaxed);
        arm_clock_register(None, agtimer_delay, None, None);
    }
}

/// Not in the C: turns on this CPU's event stream, a `wfe` wake-up about every 100 µs, so
/// that a `wfe` loop never depends on one `sev` alone (see the module's deviations). Reads
/// the counter frequency itself: an application processor calls it from its first kernel
/// instructions, before `agtimer_attach` may have run.
pub fn agtimer_evtstrm_enable() {
    let reg = timer::cntkctl() & !(CNTKCTL_EVNTI_MASK | CNTKCTL_EVNTDIR);
    let reg = reg | (agtimer_evtstrm_evnti(agtimer_get_freq()) << CNTKCTL_EVNTI_SHIFT);
    timer::set_cntkctl(reg | CNTKCTL_EVNTEN);
}

/// The event stream's trigger bit for a counter of `freq` Hz: the lowest counter bit whose
/// 0-to-1 transitions (every `2 << n` ticks) are at least 100 µs apart.
const fn agtimer_evtstrm_evnti(freq: u64) -> u64 {
    let ticks = freq / 10_000;
    let mut n = 0;
    while n < 15 && (2u64 << n) < ticks {
        n += 1;
    }
    n
}

// QEMU's 62.5 MHz counter: bit 12, an event every 131 µs; 24 MHz: bit 11, every 171 µs.
const _: () = {
    assert!(agtimer_evtstrm_evnti(62_500_000) == 12);
    assert!(agtimer_evtstrm_evnti(24_000_000) == 11);
    assert!(agtimer_evtstrm_evnti(0) == 0);
    assert!(agtimer_evtstrm_evnti(u64::MAX) == 15);
};
/* </CODE> */
