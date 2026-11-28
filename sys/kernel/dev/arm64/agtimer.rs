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
//! The ARM generic timer, `agtimer(4)`: the virtual counter (`CNTVCT_EL0`) is the
//! timecounter, the virtual timer (`CNTV_TVAL_EL0`) each CPU's interrupt clock through the PPI
//! the device tree names, and the counter is `delay(9)`'s busy-wait: arm64's clock hardware.
//!
//! Upstream: sys/arch/arm64/dev/agtimer.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/agtimer.rs@e8249449d905
//!
//! `mainbus(4)` calls [`agtimer_init`] before its children attach (`mainbus.c:107`): with a
//! frequency in `CNTFRQ_EL0` the counter becomes `delay(9)`'s. [`agtimer_attach`] reads the
//! node, prints the rate, registers the timecounter and makes the clock whole; the clock code
//! then calls `cpu_initclocks` (the PPI established at `IPL_CLOCK | IPL_MPSAFE`, `"tick"`)
//! and, on every CPU, `cpu_startclock` (the PPI routed to a secondary, the interrupt clock
//! installed, the timer enabled, the first interrupt triggered, the counter opened to EL0).
//! The registers are `hal::cpu::timer`'s (arm64's `arch/arm64/timer.rs`, a register file
//! per test thread on the host).
//!
//! ## Deviations
//! - One static softc (`AGTIMER_SC`, the C's `agtimer_cd.cd_devs[0]`) holds what
//!   `struct agtimer_softc` adds to the device, as LZ; the device's own softc is empty.
//! - A second timer node prints its line and changes nothing. The C's second attach
//!   (`agtimer.c:184-217`) sets `agtimer_frequency` again from `CNTFRQ_EL0` and the node's
//!   `clock-frequency`, may switch the counter to the sun50i reader, registers the clock
//!   functions again (ignored: the first registration wins) and calls `tc_init` on the same
//!   `agtimer_timecounter` once more. The kernel refuses that last step: a second `tc_init` of
//!   the same entry would insert it twice into the timecounter list (its one link). Machines
//!   have one timer node.
//! - Not in the C: [`agtimer_evtstrm_enable`] turns on the timer's event stream
//!   (`CNTKCTL_EL1.EVNTEN`, counter bit 12 at QEMU's 62.5 MHz: a `wfe` wake-up every
//!   131 µs). Each application processor calls it first thing (`cpu_hatch_entry`), and
//!   `agtimer_startclock` calls it again after the C's `CNTKCTL_EL1` write, which clears it.
//!   QEMU's TCG can lose the `sev` that ends a `wfe` wait (its `sev` helper kicks the halted
//!   vCPU without the BQL, racing the vCPU thread's idle check); with no interrupt pending,
//!   the processor then sleeps forever. The event stream's wake-up comes from a QEMU timer
//!   and is reliable, so every `wfe` loop goes on checking its condition. On hardware it
//!   only costs a few spurious `wfe` returns; Linux keeps it on for the same reason (LZ's).
//! - `agtimer_startclock` on a secondary whose PPI could not be established (no controller
//!   took it) routes nothing, where the C would dereference the null handle (LZ's).
//!
//! ## Redesign
//! - The driver was `bsd`'s (`arch/arm64/dev/agtimer.rs`), registered as LZ's
//!   `agtimer_ca`/`agtimer_cd` and `arm_clock_register`'s four functions in `bsd`'s clock
//!   table (`arm_clock_func`, a `StaticCell`). It is the kernel's: [`AGTIMER`] is a
//!   `DriverOps` (decision 46) whose probe reads the bus's `FdtArgs`, `cfdata[]`'s row 2 names
//!   it (`Cfdata::native`, the stub `agtimer_cd` in `ioconf.rs` for UKC's name).
//! - `struct agtimer_softc` (LZ: `Cell`s behind an `unsafe impl Sync`) is [`Agtimer`], all
//!   atomics read `Relaxed` (written at attach on the boot CPU, before any clock runs, as the
//!   C's plain fields), with `agtimer_frequency` (the C's file global, which `delay(9)` and
//!   `agtimer_init` use before the attach) and the A64 erratum's choice inside it. It is
//!   the C's `tc_priv` and `ic_cookie` at once: `Agtimer` implements the kernel's
//!   `Timecounter`, `Intrclock` and `ClockHardware` (P46): `arm_clock_register`'s
//!   `initclocks`, `startclock`, `setstatclockrate` and `delay`, installed by `agtimer_init`
//!   (with a frequency) and by the attach, the first installation winning as
//!   `arm_clock_register`'s first call; until the attach the three clock entries panic with
//!   `intr.c`'s messages (the C's table held `NULL` for them), and `delay` runs on the counter.
//!   Before either installation the kernel's `delay` is `arm_dflt_delay`'s loop.
//! - `sc_ih` is a set-once `IntrHandle` (the kernel's interrupt core); the handler is
//!   `IntrHandler::Frame(agtimer_intr)`, the C's `NULL` argument that hands it the frame.
//! - The `rearm`, `trigger`, `initclocks`, `startclock`, `setstatclockrate` and `delay`
//!   functions take the softc they act on (the C's cookie or `cd_devs[0]`), so the host tests
//!   run them on a softc of their own; the kernel's callers pass [`AGTIMER_SC`]. Two places
//!   name the static all the same: `ClockHardware::startclock` hands `&AGTIMER_SC` to
//!   `agtimer_startclock`, since the interrupt clock `clockintr_cpu_init` installs must be
//!   `'static` and the trait's `&self` is not (only that softc is ever installed); and the
//!   timecounter's entry is the softc's set-once `tc`, which the attach fills with
//!   `AGTIMER_TIMECOUNTER` before `tc_init` (the C's `tc_priv = sc`), so
//!   `agtimer_set_clockrate` updates the entry of the softc it is given (a test gives its
//!   softc an entry of its own, never registered).
//! - `agtimer_cpu_initclocks`' PPI choice is [`agtimer_ppi_index`], and the attach's
//!   frequency arithmetic `Agtimer::configure`, both tested on the host.
//! - The dispatch's backward-uptime check under the smokes (`set_uptime_check`, not in LZ:
//!   `bsd`'s N4.8a copy of this driver called it from `agtimer_startclock` with `bsd`'s
//!   feature `qemu`) is switched on by `bsd`'s `cpu_configure`, before the first clock starts.
//! - The `armreg.h` bits the driver writes are private constants of this module (the kernel
//!   names no machine header; `mainbus`'s `MPIDR_AFF` is the precedent).

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};

use hal::cpu::clock::ClockFrame;
use hal::cpu::intr::{IPL_CLOCK, IPL_MPSAFE};
use hal::cpu::timer;
use hal::mem::malloc::{M_NOWAIT, MArc};
use hal::mem::pool::AnyArc;
use hal::sync::once::OnceStatic;

use crate::assert::kpanic;
use crate::base::prf::printf;
use crate::dev::arm64::fdt::FdtArgs;
use crate::dev::arm64::intr::{IntrHandle, arm_intr_establish_fdt_idx, arm_intr_route};
use crate::dev::device::{
    Aux, DV_DULL, DevRef, Devclass, DriverOps, Ident, M_DEVBUF, Match, UnitTable,
};
use crate::dev::intr::IntrHandler;
use crate::dev::ofw::fdt::{OF_getindex, OF_getpropbool, OF_getpropint, OF_is_compatible};
use crate::time::clock::{
    ClockHardware, PROFHZ, STATCLOCK_IS_RANDOMIZED, STATHZ, install_clock_hardware,
};
use crate::time::clockintr::{
    Intrclock, clockintr_cpu_init, clockintr_dispatch, clockintr_trigger, cpu_is_primary,
};
use crate::time::tc::{TcEntry, Timecounter, tc_init};
use crate::time::timeout::hz;

/// `TIMER_FREQUENCY`: ARM core clock.
const TIMER_FREQUENCY: i32 = 24 * 1000 * 1000;

/// `CNTKCTL_EL0VCTEN` (`<machine/armreg.h>`): EL0 may read the virtual counter.
const CNTKCTL_EL0VCTEN: u64 = 1 << 1;
/// `CNTKCTL_EL1.EVNTEN`: the event stream is on (not in the C header).
const CNTKCTL_EVNTEN: u64 = 1 << 2;
/// `CNTKCTL_EL1.EVNTDIR`: the trigger bit's transition that makes an event (0: 0 to 1).
const CNTKCTL_EVNTDIR: u64 = 1 << 3;
/// `CNTKCTL_EL1.EVNTI`'s shift: the counter bit that triggers the event stream.
const CNTKCTL_EVNTI_SHIFT: u64 = 4;
/// `CNTKCTL_EL1.EVNTI`'s mask.
const CNTKCTL_EVNTI_MASK: u64 = 0xf << CNTKCTL_EVNTI_SHIFT;
/// `GTIMER_CNTV_CTL_ENABLE`: the virtual timer runs.
const CNTV_CTL_ENABLE: u32 = 1 << 0;
/// `GTIMER_CNTV_CTL_IMASK`: the virtual timer's interrupt is masked.
const CNTV_CTL_IMASK: u32 = 1 << 1;
/// `CURRENTEL_EL_MASK`: `CurrentEL`'s level field.
const CURRENTEL_EL_MASK: u64 = 0x3 << 2;
/// `CURRENTEL_EL_EL2`: running at EL2.
const CURRENTEL_EL_EL2: u64 = 0x2 << 2;

/// `struct agtimer_softc` with `agtimer_frequency`: the one generic timer (see the module's
/// redesign). Every member is written at attach (or by `agtimer_init`, `agtimer_set_clockrate`
/// and `agtimer_cpu_initclocks`) on the boot CPU before a clock runs, and read after.
pub struct Agtimer {
    /// `agtimer_frequency`: the counter's rate, `CNTFRQ_EL0` or the node's `clock-frequency`.
    frequency: AtomicI32,
    /// `sc_node`.
    node: AtomicI32,
    /// `sc_ticks_per_second`.
    ticks_per_second: AtomicU32,
    /// `sc_nsec_cycle_ratio`: counter cycles per nanosecond, 32.32 fixed point.
    nsec_cycle_ratio: AtomicU64,
    /// `sc_nsec_max`: the longest delay `rearm` can scale without overflow.
    nsec_max: AtomicU64,
    /// `sc_ih`: the virtual timer's PPI, established by `agtimer_cpu_initclocks`.
    ih: OnceStatic<IntrHandle>,
    /// `tc_get_timecount == agtimer_get_timecount_sun50i`, `tc_user == TC_AGTIMER_SUN50I`:
    /// the node names the Allwinner A64's erratum.
    sun50i: AtomicBool,
    /// `agtimer_cd.cd_devs[0] != NULL`: the attach ran.
    attached: AtomicBool,
    /// The timecounter's entry whose `tc_priv` this softc is (`agtimer_timecounter`), set by
    /// the attach before `tc_init`.
    tc: OnceStatic<&'static TcEntry>,
}

impl Agtimer {
    /// The softc before `agtimer_init`: the C's statics.
    pub const fn new() -> Self {
        Self {
            frequency: AtomicI32::new(TIMER_FREQUENCY),
            node: AtomicI32::new(0),
            ticks_per_second: AtomicU32::new(0),
            nsec_cycle_ratio: AtomicU64::new(0),
            nsec_max: AtomicU64::new(0),
            ih: OnceStatic::new(),
            sun50i: AtomicBool::new(false),
            attached: AtomicBool::new(false),
            tc: OnceStatic::new(),
        }
    }

    /// `agtimer_frequency`.
    pub fn frequency(&self) -> i32 {
        self.frequency.load(Ordering::Relaxed)
    }

    /// `sc_ticks_per_second`.
    pub fn ticks_per_second(&self) -> u32 {
        self.ticks_per_second.load(Ordering::Relaxed)
    }

    /// Whether the attach ran.
    pub fn attached(&self) -> bool {
        self.attached.load(Ordering::Relaxed)
    }

    /// The softc's rate from `agtimer_frequency`: `sc_ticks_per_second`, the ratio and the
    /// longest delay, as the attach and `agtimer_set_clockrate` compute them.
    fn set_ticks(&self) {
        let ticks = self.frequency() as u32;
        let ratio = u64::from(ticks) * (1u64 << 32) / 1_000_000_000;
        self.ticks_per_second.store(ticks, Ordering::Relaxed);
        self.nsec_cycle_ratio.store(ratio, Ordering::Relaxed);
        self.nsec_max.store(u64::MAX / ratio, Ordering::Relaxed);
    }

    /// `agtimer_attach`'s arithmetic for `node`: `agtimer_frequency` from a nonzero `cntfrq`
    /// (`CNTFRQ_EL0`), then the node's `clock-frequency` over it; the softc's rate. Returns
    /// `sc_ticks_per_second`.
    fn configure(&self, node: i32, cntfrq: u64) -> u32 {
        self.node.store(node, Ordering::Relaxed);
        if cntfrq != 0 {
            self.frequency.store(cntfrq as i32, Ordering::Relaxed);
        }
        let freq = OF_getpropint(node, b"clock-frequency", self.frequency() as u32);
        self.frequency.store(freq as i32, Ordering::Relaxed);
        self.set_ticks();
        self.ticks_per_second()
    }

    /// `agtimer_rearm`'s scaling: `nsecs` (at most `sc_nsec_max`) as counter cycles, at most
    /// `INT32_MAX`.
    fn cycles(&self, nsecs: u64) -> u32 {
        let nsecs = nsecs.min(self.nsec_max.load(Ordering::Relaxed));
        let cycles = nsecs.wrapping_mul(self.nsec_cycle_ratio.load(Ordering::Relaxed)) >> 32;
        cycles.min(i32::MAX as u64) as u32
    }
}

impl Default for Agtimer {
    fn default() -> Self {
        Self::new()
    }
}

/// The device's softc: the state is [`AGTIMER_SC`]'s.
pub struct AgtimerDev;

/// `agtimer_ca` with `agtimer_cd`'s constants.
pub struct AgtimerDriver;

/// The driver `cfdata[]`'s `agtimer* at fdt?` row names (`Cfdata::native`).
pub static AGTIMER: AgtimerDriver = AgtimerDriver;

/// `agtimer_cd`'s `cd_devs`.
static AGTIMER_UNITS: UnitTable = UnitTable::new();

/// `agtimer_cd.cd_devs[0]`: the one generic timer, `agtimer_timecounter`'s `tc_priv` and
/// `agtimer_intrclock`'s `ic_cookie`.
pub static AGTIMER_SC: Agtimer = Agtimer::new();

/// `agtimer_timecounter`: the kernel's record of the counter.
static AGTIMER_TIMECOUNTER: TcEntry = TcEntry::new(&AGTIMER_SC);

/// `agtimer_readcnt64_sun50i`: the counter around the Allwinner A64 erratum.
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

/// `agtimer_set_ctrl`: `CNTV_CTL_EL0 = val`, then `isb`.
#[inline]
fn agtimer_set_ctrl(val: u32) {
    timer::set_cntv_ctl(val);
}

/// `agtimer_set_tval`: `CNTV_TVAL_EL0 = val`, then `isb`.
#[inline]
fn agtimer_set_tval(val: u32) {
    timer::set_cntv_tval(val);
}

/// `agtimer_match`: `arm,armv7-timer` or `arm,armv8-timer`.
pub fn agtimer_match(fa: &FdtArgs) -> i32 {
    i32::from(
        OF_is_compatible(fa.node, b"arm,armv7-timer")
            || OF_is_compatible(fa.node, b"arm,armv8-timer"),
    )
}

impl DriverOps for AgtimerDriver {
    fn name(&self) -> &'static [u8] {
        b"agtimer"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &AGTIMER_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        match MArc::new(M_DEVBUF, M_NOWAIT, AgtimerDev) {
            Some(sc) => MArc::erase(sc),
            None => kpanic(format_args!(
                "config_make_softc: allocation for device softc failed"
            )),
        }
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        FdtArgs::of_aux(aux).map_or(0, agtimer_match)
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let Some(fa) = FdtArgs::of_aux(aux) else {
            kpanic(format_args!("{}: not a device-tree node", dev.xname()));
        };
        agtimer_attach(fa);
    }
}

/// `agtimer_attach`: the rate, the A64 erratum, the clock functions and the timecounter.
pub fn agtimer_attach(fa: &FdtArgs) {
    let sc = &AGTIMER_SC;
    if sc.attached() {
        // A second timer node: its line, nothing else (the module's deviations).
        let mut freq = agtimer_get_freq();
        if freq == 0 {
            freq = sc.frequency() as u64;
        }
        let ticks = OF_getpropint(fa.node, b"clock-frequency", freq as u32);
        printf(format_args!(": {} kHz\n", ticks / 1000));
        return;
    }

    let ticks = sc.configure(fa.node, agtimer_get_freq());
    printf(format_args!(": {} kHz\n", ticks / 1000));

    // The Allwinner A64 has an erratum where the bottom 9 bits of the counter register
    // cannot be trusted while any of the higher bits roll over.
    if OF_getpropbool(fa.node, b"allwinner,erratum-unknown1") {
        timer::set_readcnt64_sun50i();
        sc.sun50i.store(true, Ordering::Relaxed);
    }

    // The private timer and its interrupts wait for the clock's start (initclocks).
    sc.attached.store(true, Ordering::Relaxed);
    // arm_clock_register(agtimer_cpu_initclocks, agtimer_delay, agtimer_setstatclockrate,
    // agtimer_startclock): the first installation wins, and it is this same softc.
    let _ = install_clock_hardware(sc);

    // tc_frequency is the softc's rate (`Timecounter::frequency`), tc_priv the softc.
    let _ = sc.tc.set(&AGTIMER_TIMECOUNTER);
    tc_init(&AGTIMER_TIMECOUNTER);
    // agtimer_intrclock.ic_cookie = sc: the softc is the interrupt clock.
}

/// `agtimer_get_timecount_default`: the counter's low 32 bits (no need to work around the
/// Cortex-A73 erratum 858921 for the low half alone).
pub fn agtimer_get_timecount_default() -> u32 {
    timer::timecount()
}

/// `agtimer_get_timecount_sun50i`.
pub fn agtimer_get_timecount_sun50i() -> u32 {
    agtimer_readcnt64_sun50i() as u32
}

/// `agtimer_rearm`: the timer fires `nsecs` from now.
pub fn agtimer_rearm(sc: &Agtimer, nsecs: u64) {
    agtimer_set_tval(sc.cycles(nsecs));
}

/// `agtimer_trigger`: the timer fires now.
pub fn agtimer_trigger() {
    agtimer_set_tval(0);
}

/// `agtimer_intr`: the timer's interrupt, established with a `NULL` argument, so it reads
/// the interrupt's frame.
pub fn agtimer_intr(frame: &ClockFrame) -> i32 {
    i32::from(clockintr_dispatch(frame))
}

/// `agtimer_set_clockrate`: the counter's rate changed to `new_frequency`; nothing more
/// before the attach.
pub fn agtimer_set_clockrate(sc: &Agtimer, new_frequency: i32) {
    sc.frequency.store(new_frequency, Ordering::Relaxed);

    if !sc.attached() {
        return;
    }

    sc.set_ticks();
    if let Some(tc) = sc.tc.get() {
        tc.set_frequency(u64::from(sc.ticks_per_second()));
    }

    printf(format_args!(
        "agtimer0: adjusting clock: new tick rate {} kHz\n",
        sc.ticks_per_second() / 1000
    ));
}

/// The PPI `agtimer_cpu_initclocks` establishes for the running exception level `el`
/// (`CurrentEL`'s bits): `hyp-virt` at EL2, `virt` below, by `interrupt-names` or else the
/// specification's index (4 and 2).
pub fn agtimer_ppi_index(node: i32, el: u64) -> usize {
    let hyp = el & CURRENTEL_EL_MASK == CURRENTEL_EL_EL2;
    let name: &[u8] = if hyp { b"hyp-virt" } else { b"virt" };
    match OF_getindex(node, Some(name), b"interrupt-names") {
        -1 => {
            if hyp {
                4
            } else {
                2
            }
        }
        idx => idx as usize,
    }
}

/// `agtimer_cpu_initclocks`: `stathz`, `profhz`, the softc's rate and the virtual timer's
/// interrupt.
pub fn agtimer_cpu_initclocks(sc: &Agtimer) {
    let hz = hz();
    STATHZ.store(hz, Ordering::Relaxed);
    PROFHZ.store(hz * 10, Ordering::Relaxed);
    STATCLOCK_IS_RANDOMIZED.store(true, Ordering::Relaxed);

    if sc.ticks_per_second() != sc.frequency() as u32 {
        agtimer_set_clockrate(sc, sc.frequency());
    }

    // Pick the correct PPI depending on the running EL.
    let node = sc.node.load(Ordering::Relaxed);
    let idx = agtimer_ppi_index(node, timer::current_el());

    // configure virtual timer interrupt
    if let Some(ih) = arm_intr_establish_fdt_idx(
        node,
        idx,
        IPL_CLOCK | IPL_MPSAFE,
        IntrHandler::Frame(agtimer_intr),
        "tick",
    ) {
        let _ = sc.ih.set(ih);
    }
}

/// `agtimer_delay`: busy-waits `usecs` on the counter.
pub fn agtimer_delay(sc: &Agtimer, usecs: u32) {
    let start = agtimer_readcnt64();
    let cycles = u64::from(usecs) * sc.frequency() as u64 / 1_000_000;
    while agtimer_readcnt64().wrapping_sub(start) < cycles {
        core::hint::spin_loop(); // CPU_BUSY_CYCLE()
    }
}

/// `agtimer_setstatclockrate`: the statclock shares the timer; nothing to change.
pub fn agtimer_setstatclockrate(_sc: &Agtimer, _newhz: i32) {}

/// `agtimer_startclock`'s routing: on a secondary CPU (`primary` false), the PPI is routed to
/// CPU `cpu`.
fn agtimer_route(sc: &Agtimer, primary: bool, cpu: usize) {
    if !primary && let Some(ih) = sc.ih.get() {
        arm_intr_route(*ih, true, cpu);
    }
}

/// `agtimer_startclock`: the running CPU's clock interrupts start.
pub fn agtimer_startclock(sc: &'static Agtimer) {
    agtimer_route(sc, cpu_is_primary(), hal::cpu::cpu_number());

    clockintr_cpu_init(Some(sc));

    let mut reg = agtimer_get_ctrl();
    reg &= !CNTV_CTL_IMASK;
    reg |= CNTV_CTL_ENABLE;
    agtimer_set_tval(i32::MAX as u32);
    agtimer_set_ctrl(reg);

    clockintr_trigger();

    // enable userland access to virtual counter
    timer::set_cntkctl(CNTKCTL_EL0VCTEN);
    // Not in the C: the write above clears the event stream; keep it on (the deviations).
    agtimer_evtstrm_enable();
}

/// `agtimer_init`: what `mainbus` starts before its children attach: with a frequency in
/// `CNTFRQ_EL0`, the counter's rate and `agtimer_delay` as `delay(9)`.
pub fn agtimer_init() {
    // XXX: Check for Generic Timer support.
    let cntfrq = agtimer_get_freq();

    if cntfrq != 0 {
        AGTIMER_SC.frequency.store(cntfrq as i32, Ordering::Relaxed);
        // arm_clock_register(NULL, agtimer_delay, NULL, NULL): until the attach, the three
        // clock entries of the same softc panic as the C's NULL ones.
        let _ = install_clock_hardware(&AGTIMER_SC);
    }
}

/// Not in the C: turns on this CPU's event stream, a `wfe` wake-up about every 100 µs, so
/// that a `wfe` loop never depends on one `sev` alone (the module's deviations). Registers
/// only: it reads the counter frequency itself, since an application processor calls it
/// from its first kernel instructions, before the attach may have run.
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

impl Timecounter for Agtimer {
    fn get_timecount(&self) -> u32 {
        if self.sun50i.load(Ordering::Relaxed) {
            agtimer_get_timecount_sun50i()
        } else {
            agtimer_get_timecount_default()
        }
    }

    fn counter_mask(&self) -> u32 {
        0xffff_ffff
    }

    fn frequency(&self) -> u64 {
        u64::from(self.ticks_per_second())
    }

    fn name(&self) -> &'static str {
        "agtimer"
    }

    fn quality(&self) -> i32 {
        0
    }

    fn user(&self) -> i32 {
        if self.sun50i.load(Ordering::Relaxed) {
            timer::TC_AGTIMER_SUN50I
        } else {
            timer::TC_AGTIMER
        }
    }
}

impl Intrclock for Agtimer {
    fn rearm(&self, nsecs: u64) {
        agtimer_rearm(self, nsecs);
    }

    fn trigger(&self) {
        agtimer_trigger();
    }
}

impl ClockHardware for Agtimer {
    fn initclocks(&self) {
        if !self.attached() {
            kpanic(format_args!("initclocks function not initialized yet"));
        }
        agtimer_cpu_initclocks(self);
    }

    fn startclock(&self) {
        if !self.attached() {
            kpanic(format_args!("startclock function not initialized yet"));
        }
        // The clock interrupt keeps the softc: the kernel's is the static one.
        agtimer_startclock(&AGTIMER_SC);
    }

    fn setstatclockrate(&self, newhz: i32) {
        if !self.attached() {
            kpanic(format_args!(
                "arm_clock_func.setstatclockrate not initialized"
            ));
        }
        agtimer_setstatclockrate(self, newhz);
    }

    fn delay(&self, usecs: u32) {
        agtimer_delay(self, usecs);
    }
}

// QEMU's 62.5 MHz counter: bit 12, an event every 131 µs; 24 MHz: bit 11, every 171 µs.
const _: () = {
    assert!(agtimer_evtstrm_evnti(62_500_000) == 12);
    assert!(agtimer_evtstrm_evnti(24_000_000) == 11);
    assert!(agtimer_evtstrm_evnti(0) == 0);
    assert!(agtimer_evtstrm_evnti(u64::MAX) == 15);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Each test acts on a softc of its own (`Agtimer::new()`) over the calling thread's timer
    // registers (`hal::cpu::timer`'s host double); nothing here installs the process's
    // `ClockHardware` or registers the timecounter (the attach and `agtimer_init` do, and the
    // kernel's clock tests own that slot). Trees are `testfdt`'s, after the test memory.

    use std::boxed::Box;
    use std::sync::Mutex;
    use std::vec::Vec;

    use hal::cpu::timer::{TimerRegs, regs, set_regs};

    use super::*;
    use crate::dev::arm64::intr::{BarrierWait, IcCookie, IcDesc, InterruptController};
    use crate::dev::ofw::fdt::OF_finddevice;
    use crate::dev::ofw::fdt::testfdt::{FdtBuilder, TreeGuard, install};
    use crate::testutil;

    /// A softc at `freq` Hz, as the attach leaves it.
    fn at(freq: i32) -> Agtimer {
        let sc = Agtimer::new();
        sc.frequency.store(freq, Ordering::Relaxed);
        sc.set_ticks();
        sc
    }

    /// Timers with and without `clock-frequency` and `interrupt-names`, and an interrupt
    /// controller (phandle 1, three cells) for the route test.
    fn tree() -> TreeGuard {
        testutil::install();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.begin(b"intc@0");
        b.prop(b"interrupt-controller", b"");
        b.prop_cells(b"#interrupt-cells", &[3]);
        b.prop_cells(b"phandle", &[1]);
        b.end();
        b.begin(b"timer");
        b.prop(b"compatible", b"arm,armv8-timer\0");
        b.end();
        b.begin(b"timer-freq");
        b.prop(b"compatible", b"arm,armv7-timer\0");
        b.prop_cells(b"clock-frequency", &[19_200_000]);
        b.prop(b"interrupt-names", b"hyp-virt\0virt\0");
        b.end();
        b.end();
        install(b.finish())
    }

    #[test]
    fn rearm_scales_and_clamps_as_the_c() {
        set_regs(TimerRegs::INIT);
        // 62.5 MHz: the ratio is 2^28 exactly, so a millisecond is 62500 cycles.
        let sc = at(62_500_000);
        assert_eq!(sc.nsec_cycle_ratio.load(Ordering::Relaxed), 1 << 28);
        assert_eq!(sc.nsec_max.load(Ordering::Relaxed), u64::MAX >> 28);
        for (nsecs, want) in [
            (0u64, 0u32),
            (16, 1),
            (1_000_000, 62_500),
            (1_000_000_000, 62_500_000),
            // 2^35 ns is 2^31 cycles: INT32_MAX; anything longer clamps to sc_nsec_max first.
            (1 << 35, i32::MAX as u32),
            (u64::MAX, i32::MAX as u32),
        ] {
            agtimer_rearm(&sc, nsecs);
            assert_eq!(regs().cntv_tval, want, "{nsecs} ns");
        }
        Intrclock::trigger(&sc);
        assert_eq!(regs().cntv_tval, 0, "trigger: now");
        Intrclock::rearm(&sc, 1_000_000);
        assert_eq!(regs().cntv_tval, 62_500);
    }

    #[test]
    fn set_clockrate_waits_for_the_attach() {
        testutil::install();
        // The softc with an entry of its own, never registered (the attach sets the
        // kernel's `AGTIMER_TIMECOUNTER` here).
        let sc: &'static Agtimer = Box::leak(Box::new(at(24_000_000)));
        let tc: &'static TcEntry = Box::leak(Box::new(TcEntry::new(sc)));
        assert!(sc.tc.set(tc).is_ok());
        agtimer_set_clockrate(sc, 62_500_000);
        assert_eq!(sc.frequency(), 62_500_000);
        assert_eq!(
            sc.ticks_per_second(),
            24_000_000,
            "not attached: the rate stays"
        );
        assert_ne!(tc.frequency(), 62_500_000, "nor the timecounter's");
        sc.attached.store(true, Ordering::Relaxed);
        agtimer_set_clockrate(sc, 62_500_000);
        assert_eq!(sc.ticks_per_second(), 62_500_000);
        assert_eq!(sc.nsec_cycle_ratio.load(Ordering::Relaxed), 1 << 28);
        assert_eq!(Timecounter::frequency(sc), 62_500_000);
        assert_eq!(tc.frequency(), 62_500_000, "tc_frequency follows");
        assert!(testutil::printf_count("agtimer0: adjusting clock: new tick rate 62500 kHz") >= 1);
    }

    #[test]
    fn the_attach_rate_comes_from_clock_frequency_then_cntfrq() {
        let _t = tree();
        let plain = OF_finddevice(b"/timer");
        let freq = OF_finddevice(b"/timer-freq");
        // The C's default, CNTFRQ_EL0, and the property over either.
        assert_eq!(Agtimer::new().configure(plain, 0), 24_000_000);
        assert_eq!(Agtimer::new().configure(plain, 62_500_000) / 1000, 62_500);
        let sc = Agtimer::new();
        assert_eq!(sc.configure(freq, 62_500_000), 19_200_000);
        assert_eq!(sc.frequency(), 19_200_000);
        assert_eq!(sc.node.load(Ordering::Relaxed), freq);
    }

    #[test]
    fn the_ppi_follows_the_exception_level_and_the_names() {
        let _t = tree();
        let plain = OF_finddevice(b"/timer");
        let named = OF_finddevice(b"/timer-freq");
        let (el1, el2) = (1u64 << 2, 2u64 << 2);
        assert_eq!(
            agtimer_ppi_index(plain, el1),
            2,
            "virt by the specification"
        );
        assert_eq!(
            agtimer_ppi_index(plain, el2),
            4,
            "hyp-virt by the specification"
        );
        assert_eq!(agtimer_ppi_index(named, el1), 1);
        assert_eq!(agtimer_ppi_index(named, el2), 0);
    }

    #[test]
    fn the_sun50i_erratum_switches_the_reader_and_the_user_value() {
        set_regs(TimerRegs {
            cntvct: 0x1_0000_01fe,
            ..TimerRegs::INIT
        });
        let sc = at(24_000_000);
        assert_eq!(sc.user(), timer::TC_AGTIMER);
        assert_eq!(sc.get_timecount(), 0x1fe, "the low 32 bits");
        sc.sun50i.store(true, Ordering::Relaxed);
        timer::set_readcnt64_sun50i();
        assert_eq!(sc.user(), timer::TC_AGTIMER_SUN50I);
        // 0x1_0000_01ff and 0x1_0000_0200 are at a roll-over; 0x1_0000_0201 is read.
        assert_eq!(sc.get_timecount(), 0x201);
        assert_eq!(
            (sc.name(), sc.quality(), sc.counter_mask()),
            ("agtimer", 0, 0xffff_ffff)
        );
    }

    #[test]
    fn delay_waits_for_its_cycles() {
        set_regs(TimerRegs {
            cntvct: 1000,
            step: 3,
            ..TimerRegs::INIT
        });
        let sc = at(1_000_000);
        // 10 us at 1 MHz: 10 cycles, the counter read until it moved that far.
        ClockHardware::delay(&sc, 10);
        assert!(regs().cntvct >= 1000 + 10, "{:?}", regs());
        let before = regs().cntvct;
        agtimer_delay(&sc, 0);
        assert_eq!(regs().cntvct, before + 2 * 3, "the start and one check");
    }

    #[test]
    fn the_event_stream_keeps_the_other_bits() {
        set_regs(TimerRegs {
            cntfrq: 62_500_000,
            cntkctl: CNTKCTL_EL0VCTEN | CNTKCTL_EVNTDIR | (3 << CNTKCTL_EVNTI_SHIFT),
            ..TimerRegs::INIT
        });
        agtimer_evtstrm_enable();
        assert_eq!(
            regs().cntkctl,
            CNTKCTL_EL0VCTEN | CNTKCTL_EVNTEN | (12 << CNTKCTL_EVNTI_SHIFT)
        );
    }

    #[test]
    fn agtimer_init_without_a_frequency_keeps_the_rate() {
        // CNTFRQ_EL0 is 0 on a fresh test thread: agtimer_frequency stays (the C registers
        // nothing either; whether the slot stays empty is not checked here, the clock tests
        // own it).
        set_regs(TimerRegs::INIT);
        let before = AGTIMER_SC.frequency();
        agtimer_init();
        assert_eq!(AGTIMER_SC.frequency(), before);
    }

    #[test]
    #[should_panic(expected = "initclocks function not initialized yet")]
    fn initclocks_before_the_attach_panics() {
        ClockHardware::initclocks(&Agtimer::new());
    }

    #[test]
    #[should_panic(expected = "startclock function not initialized yet")]
    fn startclock_before_the_attach_panics() {
        ClockHardware::startclock(&Agtimer::new());
    }

    #[test]
    #[should_panic(expected = "arm_clock_func.setstatclockrate not initialized")]
    fn setstatclockrate_before_the_attach_panics() {
        ClockHardware::setstatclockrate(&Agtimer::new(), 100);
    }

    /// A controller that records the routes it is asked for.
    struct Routes(Mutex<Vec<(bool, usize)>>);

    impl InterruptController for Routes {
        fn establish(
            &self,
            _cell: &[u32],
            _level: i32,
            _cpu: Option<usize>,
            _h: IntrHandler,
            _name: &'static str,
        ) -> Option<IcCookie> {
            Some(IcCookie::Word(1))
        }

        fn disestablish(&self, _ih: IcCookie) {}

        fn route(&self, _ih: IcCookie, enable: bool, cpu: usize) {
            self.0.lock().unwrap().push((enable, cpu));
        }

        fn barrier(&self, _ih: IcCookie) -> BarrierWait {
            BarrierWait::Nothing
        }
    }

    #[test]
    fn startclock_routes_the_ppi_on_a_secondary_only() {
        let _t = tree();
        let routes: &'static Routes = Box::leak(Box::new(Routes(Mutex::new(Vec::new()))));
        let ic = crate::dev::arm64::intr::test_register(IcDesc {
            node: OF_finddevice(b"/intc@0"),
            gic_its_id: 0,
            ops: routes,
        });
        let sc = at(62_500_000);
        agtimer_route(&sc, false, 3);
        assert!(routes.0.lock().unwrap().is_empty(), "no PPI yet: nothing");
        let ih = ic
            .establish(
                &[1, 11, 4],
                IPL_CLOCK | IPL_MPSAFE,
                None,
                IntrHandler::Frame(agtimer_intr),
                "tick",
            )
            .unwrap();
        assert!(sc.ih.set(ih).is_ok());
        agtimer_route(&sc, true, 0);
        assert!(routes.0.lock().unwrap().is_empty(), "the primary: no route");
        agtimer_route(&sc, false, 3);
        assert_eq!(*routes.0.lock().unwrap(), [(true, 3)]);
    }
}
/* </TESTS> */
