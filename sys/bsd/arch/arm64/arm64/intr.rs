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
//! arm64 interrupt dispatch and the clock hooks: `arch/arm64/arm64/intr.c`.
//!
//! Upstream: sys/arch/arm64/arm64/intr.c @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M2 ports the clock function table and `delay(9)`
//! (`arm_clock_func`, `arm_clock_register`, `arm_dflt_delay`, `delay`, `cpu_initclocks`,
//! `cpu_startclock`, `setstatclockrate`); M4 adds the IRQ/FIQ entry from `exception.S`
//! (`arm_dflt_irq`, `arm_dflt_fiq`, `arm_irq_dispatch`, `arm_fiq_dispatch`; `arm_cpu_irq`
//! and `arm_cpu_fiq` are `hal`'s since N4.8b, `sys/hal/arch/arm64/trap.rs`, which dispatch
//! through [`ArmIntrHardware`]), the `spl` machinery (`arm_smask`, `arm_intr_func` with the `arm_dflt_*`
//! functions, `arm_do_pending_intr`, `arm_set_intr_handler`, `arm_init_smask`, `splraise`,
//! `spllower`, `splx`, `softintr`, `arm_splassert_check`), the wakeup hooks and the device
//! tree registration (`arm_intr_get_parent`, the pre-registration `arm_intr_prereg_*` and
//! `arm_intr_init_fdt`, `arm_intr_register_fdt`, `arm_intr_establish_fdt`/`_cpu`/`_idx`/
//! `_idx_cpu`, `arm_intr_disestablish_fdt`, `arm_intr_enable`/`disable`,
//! `arm_intr_parent_establish_fdt`/`_disestablish_fdt`, `arm_intr_route`,
//! `arm_intr_cpu_enable`, `intr_barrier`, `intr_set_wakeup`). M12 adds, with PCI, the
//! `imap` and `msi` variants (`arm_intr_establish_fdt_imap`/`_imap_cpu`/`_msi`/`_msi_cpu`)
//! and `arm_intr_map_msi`; M11a adds the `MULTIPROCESSOR` IPIs
//! (`intr_send_ipi_func`, `arm_send_ipi`, `arm_no_send_ipi`); the generic
//! timer (`agtimer.c`) that replaces `arm_dflt_delay` attaches from `mainbus` (M5).
//!
//! N7.2a1: the registry of interrupt controllers, the pre-registration and the device tree's
//! interrupt specifiers are the kernel's (`kernel::dev::arm64::intr`); this file is the
//! adapter `bsd`'s drivers keep calling under LZ's names and signatures: [`LegacyIc`] makes a
//! driver's `struct interrupt_controller` the kernel's `InterruptController`,
//! [`arm_intr_register_fdt`] registers it and links it on the list
//! [`interrupt_controllers`] serves, and the handles stay `NonNull<MachineIntrHandle>`. The
//! clock table, the `spl` table and the IPI slot are still here (N7.2b2, N7.2a3, N7.2a2).
//!
//! ## Deviations
//! - `arm_clock_func` is a [`StaticCell`], written by `arm_clock_register` during
//!   autoconfiguration on the boot CPU.
//! - `arm_dflt_delay`'s inner loop spins on `yield` so the compiler keeps it; the C's empty
//!   loop body relies on the compiler not optimising it away.
//! - `arm_intr_func` and `intr_send_ipi_func` are `StaticCell`s written by the controller's
//!   attach (`arm_set_intr_handler`) on the boot CPU before interrupts are enabled; `hal`
//!   reaches `arm_intr_func` as `hal::cpu::intr::IntrHardware` ([`ArmIntrHardware`], N4.5c), and
//!   `arm_smask`, `ci_cpl`, `ci_ipending` and the `arm_dflt_*` bodies are `hal::cpu::intr`'s.
//! - `interrupt_controllers` (which `ampintc.c`, `acpipci.c` and `acpi_machdep.c` walk) is the
//!   list of the legacy structures registered through [`arm_intr_register_fdt`], linked as
//!   the kernel links their registrations, the latest first; the pre-registration's dummy
//!   controllers are the kernel's and not on it. Every walk of it looks for a real controller
//!   (by node, by phandle, with `ic_establish_msi`, by ITS), which a real controller's
//!   registration puts ahead of its node's dummy in the C, so no walk finds another one.
//!   That holds once the real controller has registered: before it, a walk by phandle
//!   (`acpi_intr_establish`'s) would find the node's dummy in the C and pre-register, where
//!   this list has nothing and the establish fails. Unreachable on the boots there are: an
//!   ACPI boot attaches the GIC at mainbus before `acpi0` and its devices.
//! - A handle on a legacy controller is LZ's `MachineIntrHandle` (`ih_ic`, `ih_ih`, the size
//!   `free` checks, as the handles `acpi_machdep`, `ampintc`'s MSI frames and `pciecam` make
//!   by hand); a handle on a native controller (a pre-registration until N7.2a2) has a null
//!   `ih_ic` and carries the kernel's `IntrHandle` after it (`NativeIntrHandle`); every
//!   handle function takes the path its `ih_ic` says. No `bsd` reader of `ih_ic` meets a
//!   native handle: the readers read only the handles they made.
//! - `arm_intr_establish_fdt_imap*` take the child's four `reg` cells as a slice (the C's
//!   `int *reg, int nreg`, `nreg` in bytes); `arm_intr_map_msi`'s `data` is a `&mut u64`.

use core::ffi::c_void;
use core::ptr::{self, NonNull};

use crate::sys::staticcell::StaticCell;

use crate::arch::arm64::include::cpu::{CpuInfo, curcpu};
use crate::arch::arm64::include::frame::Trapframe;
use crate::arch::arm64::include::intr::{
    ArmIntrFunc, IPL_NONE, InterruptController, IntrFn, MachineIntrHandle,
};
use crate::kassert;
use crate::kern::kern_malloc::{free, malloc};
use crate::kern::kern_sched::{cpu_info_by_number, cpu_number_of};
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::{panic, splassert_fail};
use crate::queue_adapter;
use crate::sys::malloc::{M_DEVBUF, M_WAITOK};
use crate::sys::queue::{ListEntry, ListHead};
use crate::unported;
use kernel::dev::arm64::intr::{self as core_intr, BarrierWait, IcCookie, IcDesc, IntrHandle};
use kernel::dev::intr::IntrHandler;

pub use kernel::dev::arm64::intr::{
    MAX_INTERRUPT_CELLS, arm_intr_cpu_enable, arm_intr_get_parent, arm_intr_init_fdt,
    arm_intr_map_msi,
};

/// `arm_intr_func`: the controller's `spl` functions, the defaults until one attaches; `hal`
/// reaches them through [`ArmIntrHardware`].
static ARM_INTR_FUNC: StaticCell<ArmIntrFunc> = StaticCell::new(ArmIntrFunc {
    raise: arm_dflt_splraise,
    lower: arm_dflt_spllower,
    x: arm_dflt_splx,
    setipl: arm_dflt_setipl,
    enable_wakeup: None,
    disable_wakeup: None,
});

/// The registered `spl` functions.
fn arm_intr_func() -> &'static ArmIntrFunc {
    // SAFETY: written only by `arm_set_intr_handler` during autoconfiguration on the boot
    // CPU, before interrupts are enabled; read afterwards.
    unsafe { ARM_INTR_FUNC.get() }
}

/// `arm_smask[level]` (`hal::cpu::intr::smask`, the table's owner since N4.5c).
#[inline]
pub fn arm_smask(level: i32) -> u64 {
    hal::cpu::intr::smask(level)
}

/// `struct arm_intr_func` as `hal` calls it (decision 46: the C's function table is the
/// trait `hal::cpu::intr::IntrHardware`): the table [`arm_set_intr_handler`] filled, the C's
/// defaults before.
pub struct ArmIntrHardware;

impl hal::cpu::intr::IntrHardware for ArmIntrHardware {
    fn splraise(&self, ipl: i32) -> i32 {
        (arm_intr_func().raise)(ipl)
    }

    fn spllower(&self, ipl: i32) -> i32 {
        (arm_intr_func().lower)(ipl)
    }

    fn splx(&self, ipl: i32) {
        (arm_intr_func().x)(ipl);
    }

    fn setipl(&self, ipl: i32) {
        (arm_intr_func().setipl)(ipl);
    }

    fn splassert(&self, wantipl: i32, func: &str) {
        arm_splassert_check(wantipl, func);
    }

    /// `(*arm_irq_dispatch)(frame)`: what the controller gave `arm_set_intr_handler`.
    fn dispatch_irq(&self, frame: &mut Trapframe) {
        // SAFETY: written once during autoconfiguration, read on every interrupt afterwards.
        let dispatch = unsafe { ARM_IRQ_DISPATCH.read() };
        dispatch(frame);
    }

    /// `(*arm_fiq_dispatch)(frame)`.
    fn dispatch_fiq(&self, frame: &mut Trapframe) {
        // SAFETY: as for `dispatch_irq`.
        let dispatch = unsafe { ARM_FIQ_DISPATCH.read() };
        dispatch(frame);
    }
}

/// The controller `hal` reaches (`sys/kern/hal_bridges.rs` registers it).
pub static ARM_INTR_HARDWARE: ArmIntrHardware = ArmIntrHardware;

/// `arm_clock_func`: the clock driver's entry points, registered by the driver that attaches.
pub struct ArmClockFunc {
    /// `delay`: busy-wait for a number of microseconds.
    pub delay: fn(u32),
    /// `initclocks`: start the clock interrupts.
    pub initclocks: Option<fn()>,
    /// `setstatclockrate`: change the statistics clock rate.
    pub setstatclockrate: Option<fn(i32)>,
    /// `mpstartclock`: start the clock on a secondary CPU.
    pub mpstartclock: Option<fn()>,
}

/// `arm_clock_func`: starts with the calibration-free delay and no clock.
static ARM_CLOCK_FUNC: StaticCell<ArmClockFunc> = StaticCell::new(ArmClockFunc {
    delay: arm_dflt_delay,
    initclocks: None,
    setstatclockrate: None,
    mpstartclock: None,
});

/// The registered clock functions.
fn arm_clock_func() -> &'static ArmClockFunc {
    // SAFETY: written only by `arm_clock_register` during autoconfiguration on the boot CPU,
    // before interrupts exist; read afterwards.
    unsafe { ARM_CLOCK_FUNC.get() }
}

/// `arm_clock_register`: installs the clock driver's functions; the first registration wins.
pub fn arm_clock_register(
    initclock: Option<fn()>,
    delay: fn(u32),
    statclock: Option<fn(i32)>,
    mpstartclock: Option<fn()>,
) {
    if arm_clock_func().initclocks.is_some() {
        return;
    }
    // SAFETY: the single writer, on the boot CPU during autoconfiguration; no reference from
    // `arm_clock_func` is held across this call.
    unsafe {
        ARM_CLOCK_FUNC.write(ArmClockFunc {
            delay,
            initclocks: initclock,
            setstatclockrate: statclock,
            mpstartclock,
        });
    }
}

/// `delay(9)`: busy-waits `usec` microseconds through the registered clock.
pub fn delay(usec: u32) {
    (arm_clock_func().delay)(usec)
}

/// `cpu_initclocks`: starts the clock interrupts.
pub fn cpu_initclocks() {
    match arm_clock_func().initclocks {
        Some(initclocks) => initclocks(),
        None => {
            let _ = unported!("initclocks function not initialized yet");
        }
    }
}

/// `cpu_startclock`: starts the clock on this (secondary) CPU.
pub fn cpu_startclock() {
    match arm_clock_func().mpstartclock {
        Some(mpstartclock) => mpstartclock(),
        None => {
            let _ = unported!("startclock function not initialized yet");
        }
    }
}

/// `setstatclockrate`: changes the statistics clock rate.
pub fn setstatclockrate(new: i32) {
    match arm_clock_func().setstatclockrate {
        Some(setstatclockrate) => setstatclockrate(new),
        None => {
            let _ = unported!("arm_clock_func.setstatclockrate not initialized");
        }
    }
}

/// `arm_dflt_delay`: BAH - there is no good way to make this close, but this isn't supposed to
/// be used after the real clock attaches.
pub fn arm_dflt_delay(usecs: u32) {
    for _ in 0..usecs {
        for _ in 0..100 {
            core::hint::spin_loop();
        }
    }
}

/// `arm_dflt_irq`: the IRQ dispatcher before an interrupt controller registers one.
pub fn arm_dflt_irq(_frame: &mut Trapframe) {
    panic(format_args!("arm_dflt_irq"));
}

/// `arm_dflt_fiq`: the FIQ dispatcher before an interrupt controller registers one.
pub fn arm_dflt_fiq(_frame: &mut Trapframe) {
    panic(format_args!("arm_dflt_fiq"));
}

/// `arm_irq_dispatch`: where `arm_cpu_irq` sends an IRQ; set by `arm_intr_register_fdt`
/// (M4-b) during autoconfiguration on the boot CPU.
pub static ARM_IRQ_DISPATCH: StaticCell<fn(&mut Trapframe)> = StaticCell::new(arm_dflt_irq);

/// `arm_fiq_dispatch`: as `arm_irq_dispatch`, for FIQs.
pub static ARM_FIQ_DISPATCH: StaticCell<fn(&mut Trapframe)> = StaticCell::new(arm_dflt_fiq);

/// `arm_dflt_splraise`: raise the level in `ci_cpl` (`hal::cpu::intr`'s).
pub fn arm_dflt_splraise(newcpl: i32) -> i32 {
    hal::cpu::intr::dflt_splraise(newcpl)
}

/// `arm_dflt_spllower`: lower the level, running what becomes unmasked (`hal::cpu::intr`'s).
pub fn arm_dflt_spllower(newcpl: i32) -> i32 {
    hal::cpu::intr::dflt_spllower(newcpl)
}

/// `arm_dflt_splx`: restore the level, running the pending soft interrupts it unmasks
/// (`hal::cpu::intr`'s).
pub fn arm_dflt_splx(newcpl: i32) {
    hal::cpu::intr::dflt_splx(newcpl);
}

/// `arm_dflt_setipl`: set the level (`hal::cpu::intr`'s).
pub fn arm_dflt_setipl(newcpl: i32) {
    hal::cpu::intr::dflt_setipl(newcpl);
}

/// `arm_do_pending_intr`: runs the soft interrupts pending above `pcpl`, highest first
/// (`hal::cpu::intr::do_pending_intr`).
pub fn arm_do_pending_intr(pcpl: i32) {
    hal::cpu::intr::do_pending_intr(pcpl);
}

/// `arm_set_intr_handler`: the interrupt controller registers its `spl` functions and
/// dispatchers.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn arm_set_intr_handler(
    raise: fn(i32) -> i32,
    lower: fn(i32) -> i32,
    x: fn(i32),
    setipl: fn(i32),
    irq_dispatch: Option<fn(&mut Trapframe)>,
    fiq_dispatch: Option<fn(&mut Trapframe)>,
    enable_wakeup: Option<fn()>,
    disable_wakeup: Option<fn()>,
) {
    // SAFETY: the controller attaches once, on the boot CPU, before interrupts are enabled.
    unsafe {
        ARM_INTR_FUNC.write(ArmIntrFunc {
            raise,
            lower,
            x,
            setipl,
            enable_wakeup,
            disable_wakeup,
        });
        if let Some(irq) = irq_dispatch {
            ARM_IRQ_DISPATCH.write(irq);
        }
        if let Some(fiq) = fiq_dispatch {
            ARM_FIQ_DISPATCH.write(fiq);
        }
    }
}

/// `arm_init_smask`: which soft interrupts each level leaves unmasked, once
/// (`hal::cpu::intr::init_smask`, which owns the table since N4.5c).
pub fn arm_init_smask() {
    hal::cpu::intr::init_smask();
}

/// `splraise`: through `arm_intr_func` (what `hal::cpu::intr::splraise_raw` reaches through
/// [`ArmIntrHardware`], called here without that hop).
pub fn splraise(ipl: i32) -> i32 {
    (arm_intr_func().raise)(ipl)
}

/// `spllower`: through `arm_intr_func`.
pub fn spllower(ipl: i32) -> i32 {
    (arm_intr_func().lower)(ipl)
}

/// `splx`: through `arm_intr_func`.
pub fn splx(ipl: i32) {
    (arm_intr_func().x)(ipl)
}

/// `softintr`: marks a soft interrupt pending on this CPU.
pub fn softintr(si: i32) {
    hal::cpu::intr::softintr(si as usize);
}

/// `arm_splassert_check`: the `DIAGNOSTIC` level check behind `splassert`.
pub fn arm_splassert_check(wantipl: i32, func: &str) {
    let oldipl = curcpu().core.cpl();

    if oldipl < wantipl {
        splassert_fail(wantipl, oldipl, func);
        // If the splassert_ctl is set to not panic, raise the ipl in a feeble attempt to
        // reduce damage.
        (arm_intr_func().setipl)(wantipl);
    }

    if wantipl == IPL_NONE && curcpu().core.idepth() != 0 {
        splassert_fail(-1, curcpu().core.idepth() as i32, func);
    }
}

/// `intr_enable_wakeup`.
pub fn intr_enable_wakeup() {
    if let Some(f) = arm_intr_func().enable_wakeup {
        f();
    }
}

/// `intr_disable_wakeup`.
pub fn intr_disable_wakeup() {
    if let Some(f) = arm_intr_func().disable_wakeup {
        f();
    }
}

queue_adapter!(
    /// `LIST_HEAD(, interrupt_controller) interrupt_controllers`.
    pub IcList: InterruptController, ic_list => ListEntry<InterruptController>
);

/// A list head that can be a static: linked only by [`arm_intr_register_fdt`], during
/// autoconfiguration under the kernel lock (the C's protocol, which the kernel's registry
/// keeps for its own list with a mutex), and never unlinked.
struct IcHead(ListHead<IcList>);
// SAFETY: the only writer is `arm_intr_register_fdt`, which autoconf runs on one CPU at a time
// under the kernel lock (no two inserts race), and an insert publishes a structure its driver
// filled before registering; the readers (`interrupt_controllers`) only walk it, and nothing is
// ever removed, so a walker never meets a freed or half-linked element.
unsafe impl Sync for IcHead {}

/// `interrupt_controllers`.
static INTERRUPT_CONTROLLERS: IcHead = IcHead(ListHead::new());

/// `interrupt_controllers`, which `ampintc.c` names `extern` for the GICv2m frame (and
/// `acpipci`, `acpi_machdep` walk): the registered legacy controllers, the latest first.
pub fn interrupt_controllers() -> &'static ListHead<IcList> {
    &INTERRUPT_CONTROLLERS.0
}

/// A driver's `struct interrupt_controller` as the kernel's
/// [`InterruptController`](kernel::dev::arm64::intr::InterruptController): its entry points
/// called as LZ called them, a `NULL` entry doing what LZ did (nothing), its cookies words.
pub struct LegacyIc(&'static InterruptController);

// SAFETY: `LegacyIc` reads, through `&InterruptController`, the entry points (plain
// `Option<fn>` fields, never written after the driver built the structure) and the `ic_cookie`
// `Cell`, which the driver writes before it calls `arm_intr_register_fdt`, so before the kernel
// publishes the registration holding this `LegacyIc` (`insert_head`, a release store that the
// walkers' acquire loads pair with). Of the structure's other `Cell`s, `ic_node` and
// `ic_gic_its_id` are only read, once, by `arm_intr_register_fdt` before that publication;
// `ic_cells` and `ic_phandle` are written after it, by the registration's `linked` hook on the
// registering CPU, but `LegacyIc` never reads them, and their readers (the walkers of
// `interrupt_controllers`) reach the structure only through that list, which links it after
// the writes, under `IcHead`'s autoconf protocol. No `Cell` that `LegacyIc` reads is written
// once another CPU can reach it, so sharing `&LegacyIc` cannot race.
unsafe impl Sync for LegacyIc {}

/// `ci` as the kernel's CPU number.
fn cpu_of(ci: Option<&CpuInfo>) -> Option<usize> {
    ci.map(cpu_number_of)
}

/// The kernel's CPU number as `bsd`'s `cpu_info`.
fn ci_of(cpu: Option<usize>) -> Option<&'static CpuInfo> {
    cpu.and_then(cpu_info_by_number)
}

/// A handler of `bsd`'s as the kernel's: the function and its argument's address.
fn raw(func: IntrFn, arg: *mut c_void) -> IntrHandler {
    IntrHandler::Raw(func, arg.expose_provenance())
}

/// A legacy controller's `void *` handle as the kernel's cookie (`NULL`: refused).
fn word(ih: *mut c_void) -> Option<IcCookie> {
    (!ih.is_null()).then(|| IcCookie::Word(ih.expose_provenance()))
}

/// The `void *` handle a legacy controller answered.
fn legacy_cookie(ih: IcCookie) -> *mut c_void {
    match ih {
        IcCookie::Word(w) => ptr::with_exposed_provenance_mut(w),
        IcCookie::Rec(_) => panic(format_args!("a legacy controller's cookie is a record")),
    }
}

impl core_intr::InterruptController for LegacyIc {
    fn establish(
        &self,
        cell: &[u32],
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        let IntrHandler::Raw(func, _) = h;
        let establish = self.0.ic_establish?;
        word(establish(
            self.0.ic_cookie.get(),
            cell,
            level,
            ci_of(cpu),
            func,
            h.raw_arg(),
            name,
        ))
    }

    fn establish_msi(
        &self,
        addr: &mut u64,
        data: &mut u64,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        let IntrHandler::Raw(func, _) = h;
        let establish_msi = self.0.ic_establish_msi?;
        word(establish_msi(
            self.0.ic_cookie.get(),
            addr,
            data,
            level,
            ci_of(cpu),
            func,
            h.raw_arg(),
            name,
        ))
    }

    fn disestablish(&self, ih: IcCookie) {
        if let Some(dis) = self.0.ic_disestablish {
            dis(legacy_cookie(ih));
        }
    }

    fn enable(&self, ih: IcCookie) {
        kassert!(self.0.ic_enable.is_some());
        if let Some(enable) = self.0.ic_enable {
            enable(legacy_cookie(ih));
        }
    }

    fn disable(&self, ih: IcCookie) {
        kassert!(self.0.ic_disable.is_some());
        if let Some(disable) = self.0.ic_disable {
            disable(legacy_cookie(ih));
        }
    }

    fn route(&self, ih: IcCookie, enable: bool, cpu: usize) {
        if let (Some(route), Some(ci)) = (self.0.ic_route, cpu_info_by_number(cpu)) {
            route(legacy_cookie(ih), enable, ci);
        }
    }

    fn cpu_enable(&self) {
        if let Some(cpu_enable) = self.0.ic_cpu_enable {
            cpu_enable();
        }
    }

    /// The driver's `ic_barrier` makes its own `sched_barrier`: nothing left to wait for.
    fn barrier(&self, ih: IcCookie) -> BarrierWait {
        if let Some(barrier) = self.0.ic_barrier {
            barrier(legacy_cookie(ih));
        }
        BarrierWait::Nothing
    }

    fn set_wakeup(&self, ih: IcCookie) {
        if let Some(set_wakeup) = self.0.ic_set_wakeup {
            set_wakeup(legacy_cookie(ih));
        }
    }

    fn legacy(&self) -> Option<usize> {
        Some(ptr::from_ref(self.0).expose_provenance())
    }
}

/// `arm_intr_register_fdt`: a real controller registers with the kernel (its `ic_cells` and
/// `ic_phandle` written back, the structure linked on [`interrupt_controllers`]); the
/// handlers pre-registered for its node are established on it now.
pub fn arm_intr_register_fdt(ic: &'static InterruptController) {
    let Some(ops) = hal::mem::malloc::MBox::new(M_DEVBUF, M_WAITOK, LegacyIc(ic)) else {
        panic(format_args!("arm_intr_register_fdt: no malloc(9)"));
    };
    let desc = IcDesc {
        node: ic.ic_node.get(),
        gic_its_id: ic.ic_gic_its_id.get(),
        ops: hal::mem::malloc::MBox::leak(ops),
    };
    core_intr::arm_intr_register_fdt_linked(desc, |reg| {
        ic.ic_cells.set(reg.cells());
        ic.ic_phandle.set(reg.phandle());
        // SAFETY: a controller registers once (its driver's attach), so `ic` is on no list;
        // autoconf, under the kernel lock (see `IcHead`).
        unsafe { INTERRUPT_CONTROLLERS.0.insert_head(ic) };
    });
}

/// A handle on a native controller: `ih_ic` null, the kernel's handle after it.
#[repr(C)]
struct NativeIntrHandle {
    /// `ih_ic` and `ih_ih` both null: what tells this kind from LZ's.
    mih: MachineIntrHandle,
    /// The kernel's handle.
    ih: IntrHandle,
}

/// What a handle names: a legacy controller and its `void *`, or the kernel's handle.
enum Handle {
    /// `ih_ic`, `ih_ih`.
    Legacy(&'static InterruptController, *mut c_void),
    /// A native controller's.
    Native(IntrHandle),
}

/// What `cookie` names.
fn handle(cookie: NonNull<MachineIntrHandle>) -> Handle {
    // SAFETY: an established handle (`arm_intr_establish_fdt*`, `arm_intr_parent_establish_fdt`
    // or one a driver made by hand, LZ's contract for every caller) is a live
    // `MachineIntrHandle` until its disestablish frees it.
    let mih = unsafe { cookie.as_ref() };
    if mih.ih_ic.is_null() {
        // SAFETY: only `new_handle` writes a null `ih_ic`, and only as the first member of a
        // `#[repr(C)]` `NativeIntrHandle` it allocated whole, so the block is one; it lives
        // as long as the `MachineIntrHandle` above.
        let native = unsafe { cookie.cast::<NativeIntrHandle>().as_ref() };
        Handle::Native(native.ih)
    } else {
        // SAFETY: a non-null `ih_ic` names a `struct interrupt_controller` that is never
        // freed: a registered one (a driver's static or softc member) or a static that is not
        // registered (`pciecam`'s `PCIECAM_IC`, whose handles it makes by hand).
        Handle::Legacy(unsafe { &*mih.ih_ic }, mih.ih_ih)
    }
}

/// `malloc(sizeof(v), M_DEVBUF, M_WAITOK)` holding `v`.
fn alloc<T>(v: T) -> Option<NonNull<T>> {
    let p = malloc(size_of::<T>(), M_DEVBUF, M_WAITOK)?.cast::<T>();
    // SAFETY: a fresh block of `size_of::<T>()` bytes, aligned for every type of a handle
    // (`malloc(9)`'s blocks are aligned to their power-of-two size, at least 16), written once
    // before anything reads it.
    unsafe { p.write(v) };
    Some(p)
}

/// The handle a driver gets for the kernel's `ih`: LZ's pair for a legacy controller, a
/// [`NativeIntrHandle`] for a native one.
fn new_handle(ih: IntrHandle) -> Option<NonNull<MachineIntrHandle>> {
    match ih.controller().ops().legacy() {
        Some(ic) => alloc(MachineIntrHandle {
            ih_ic: ptr::with_exposed_provenance(ic),
            ih_ih: legacy_cookie(ih.cookie()),
        }),
        None => alloc(NativeIntrHandle {
            mih: MachineIntrHandle {
                ih_ic: ptr::null(),
                ih_ih: ptr::null_mut(),
            },
            ih,
        })
        .map(NonNull::cast),
    }
}

/// `arm_intr_establish_fdt`: the first interrupt of `node`.
pub fn arm_intr_establish_fdt(
    node: i32,
    level: i32,
    func: IntrFn,
    cookie: *mut c_void,
    name: &'static str,
) -> Option<NonNull<MachineIntrHandle>> {
    arm_intr_establish_fdt_idx(node, 0, level, func, cookie, name)
}

/// `arm_intr_establish_fdt_cpu`.
pub fn arm_intr_establish_fdt_cpu(
    node: i32,
    level: i32,
    ci: Option<&'static CpuInfo>,
    func: IntrFn,
    cookie: *mut c_void,
    name: &'static str,
) -> Option<NonNull<MachineIntrHandle>> {
    arm_intr_establish_fdt_idx_cpu(node, 0, level, ci, func, cookie, name)
}

/// `arm_intr_establish_fdt_idx`: interrupt `idx` of `node`.
pub fn arm_intr_establish_fdt_idx(
    node: i32,
    idx: usize,
    level: i32,
    func: IntrFn,
    cookie: *mut c_void,
    name: &'static str,
) -> Option<NonNull<MachineIntrHandle>> {
    arm_intr_establish_fdt_idx_cpu(node, idx, level, None, func, cookie, name)
}

/// `arm_intr_establish_fdt_idx_cpu`: interrupt `idx` of `node` on `ci`, through the
/// controller its `interrupts-extended` or `interrupts` property names (the kernel's walk).
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn arm_intr_establish_fdt_idx_cpu(
    node: i32,
    idx: usize,
    level: i32,
    ci: Option<&'static CpuInfo>,
    func: IntrFn,
    cookie: *mut c_void,
    name: &'static str,
) -> Option<NonNull<MachineIntrHandle>> {
    let h = raw(func, cookie);
    new_handle(core_intr::arm_intr_establish_fdt_idx_cpu(
        node,
        idx,
        level,
        cpu_of(ci),
        h,
        name,
    )?)
}

/// `arm_intr_establish_fdt_imap`.
pub fn arm_intr_establish_fdt_imap(
    node: i32,
    reg: &[u32],
    level: i32,
    func: IntrFn,
    cookie: *mut c_void,
    name: &'static str,
) -> Option<NonNull<MachineIntrHandle>> {
    arm_intr_establish_fdt_imap_cpu(node, reg, level, None, func, cookie, name)
}

/// `arm_intr_establish_fdt_imap_cpu`: the interrupt that `node`'s `interrupt-map` routes the
/// child unit address and pin `reg` (four cells: `phys.hi`, `phys.mid`, `phys.lo`, the pin)
/// to, after `interrupt-map-mask` (the kernel's walk).
pub fn arm_intr_establish_fdt_imap_cpu(
    node: i32,
    reg: &[u32],
    level: i32,
    ci: Option<&'static CpuInfo>,
    func: IntrFn,
    cookie: *mut c_void,
    name: &'static str,
) -> Option<NonNull<MachineIntrHandle>> {
    let h = raw(func, cookie);
    new_handle(core_intr::arm_intr_establish_fdt_imap_cpu(
        node,
        reg,
        level,
        cpu_of(ci),
        h,
        name,
    )?)
}

/// `arm_intr_establish_fdt_msi`.
pub fn arm_intr_establish_fdt_msi(
    node: i32,
    addr: &mut u64,
    data: &mut u64,
    level: i32,
    func: IntrFn,
    cookie: *mut c_void,
    name: &'static str,
) -> Option<NonNull<MachineIntrHandle>> {
    arm_intr_establish_fdt_msi_cpu(node, addr, data, level, None, func, cookie, name)
}

/// `arm_intr_establish_fdt_msi_cpu`: an MSI through the controller `node`'s `msi-map` or
/// `msi-parent` names; `addr` and `data` come back as what the device must write where.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn arm_intr_establish_fdt_msi_cpu(
    node: i32,
    addr: &mut u64,
    data: &mut u64,
    level: i32,
    ci: Option<&'static CpuInfo>,
    func: IntrFn,
    cookie: *mut c_void,
    name: &'static str,
) -> Option<NonNull<MachineIntrHandle>> {
    let h = raw(func, cookie);
    new_handle(core_intr::arm_intr_establish_fdt_msi_cpu(
        node,
        addr,
        data,
        level,
        cpu_of(ci),
        h,
        name,
    )?)
}

/// `arm_intr_disestablish_fdt`.
///
/// # Safety
///
/// `cookie` must come from `arm_intr_establish_fdt*` (or be a `MachineIntrHandle` a driver
/// made by hand, with `M_DEVBUF`) and not be used afterwards.
pub unsafe fn arm_intr_disestablish_fdt(cookie: NonNull<MachineIntrHandle>) {
    match handle(cookie) {
        Handle::Legacy(ic, ih) => {
            if let Some(dis) = ic.ic_disestablish {
                dis(ih);
            }
            free(cookie.cast(), M_DEVBUF, size_of::<MachineIntrHandle>());
        }
        Handle::Native(ih) => {
            core_intr::arm_intr_disestablish_fdt(ih);
            free(cookie.cast(), M_DEVBUF, size_of::<NativeIntrHandle>());
        }
    }
}

/// `arm_intr_enable`.
pub fn arm_intr_enable(cookie: NonNull<MachineIntrHandle>) {
    match handle(cookie) {
        Handle::Legacy(ic, ih) => {
            kassert!(ic.ic_enable.is_some());
            if let Some(enable) = ic.ic_enable {
                enable(ih);
            }
        }
        Handle::Native(ih) => core_intr::arm_intr_enable(ih),
    }
}

/// `arm_intr_disable`.
pub fn arm_intr_disable(cookie: NonNull<MachineIntrHandle>) {
    match handle(cookie) {
        Handle::Legacy(ic, ih) => {
            kassert!(ic.ic_disable.is_some());
            if let Some(disable) = ic.ic_disable {
                disable(ih);
            }
        }
        Handle::Native(ih) => core_intr::arm_intr_disable(ih),
    }
}

/// `arm_intr_parent_establish_fdt`: some interrupt controllers transparently forward
/// interrupts to their parent. Such interrupt controllers can use this function to delegate
/// the interrupt handler to their parent.
pub fn arm_intr_parent_establish_fdt(
    cookie: *const (),
    cell: &[u32],
    level: i32,
    ci: Option<&'static CpuInfo>,
    func: IntrFn,
    arg: *mut c_void,
    name: &'static str,
) -> *mut c_void {
    let me = cookie.addr();
    let Some(ic) = core_intr::controllers().find(|ic| ic.ops().legacy() == Some(me)) else {
        return ptr::null_mut();
    };
    core_intr::arm_intr_parent_establish_fdt(ic, cell, level, cpu_of(ci), raw(func, arg), name)
        .and_then(new_handle)
        .map_or(ptr::null_mut(), |ih| ih.as_ptr().cast::<c_void>())
}

/// `arm_intr_parent_disestablish_fdt`.
pub fn arm_intr_parent_disestablish_fdt(cookie: *mut c_void) {
    let Some(ih) = NonNull::new(cookie.cast::<MachineIntrHandle>()) else {
        return;
    };
    // SAFETY: a handle from `arm_intr_parent_establish_fdt`, not used after this.
    unsafe { arm_intr_disestablish_fdt(ih) };
}

/// `arm_intr_route`.
pub fn arm_intr_route(cookie: NonNull<MachineIntrHandle>, enable: bool, ci: &CpuInfo) {
    match handle(cookie) {
        Handle::Legacy(ic, ih) => {
            if let Some(route) = ic.ic_route {
                route(ih, enable, ci);
            }
        }
        Handle::Native(ih) => core_intr::arm_intr_route(ih, enable, cpu_number_of(ci)),
    }
}

/// `intr_barrier`: waits until no CPU runs the handler.
pub fn intr_barrier(cookie: NonNull<MachineIntrHandle>) {
    match handle(cookie) {
        Handle::Legacy(ic, ih) => {
            if let Some(barrier) = ic.ic_barrier {
                barrier(ih);
            }
        }
        Handle::Native(ih) => core_intr::intr_barrier::<BsdProcGlue>(ih),
    }
}

/// `intr_set_wakeup`.
pub fn intr_set_wakeup(cookie: NonNull<MachineIntrHandle>) {
    match handle(cookie) {
        Handle::Legacy(ic, ih) => {
            if let Some(set_wakeup) = ic.ic_set_wakeup {
                set_wakeup(ih);
            }
        }
        Handle::Native(ih) => core_intr::intr_set_wakeup(ih),
    }
}

// IPI implementation (MULTIPROCESSOR)

/// `intr_send_ipi_func`: how the interrupt controller sends an IPI; `ampintc_attach`
/// installs its own, on the boot CPU before any application processor starts.
#[cfg(feature = "multiprocessor")]
pub static INTR_SEND_IPI_FUNC: StaticCell<fn(&CpuInfo, i32)> = StaticCell::new(arm_no_send_ipi);

/// `arm_send_ipi`: sends IPI `id` (`ARM_IPI_*`) to `ci`.
#[cfg(feature = "multiprocessor")]
pub fn arm_send_ipi(ci: &CpuInfo, id: i32) {
    // SAFETY: written once by the controller's attach, before any CPU but the boot one runs
    // (see `INTR_SEND_IPI_FUNC`); read-only afterwards.
    let f = unsafe { INTR_SEND_IPI_FUNC.read() };
    f(ci, id)
}

/// `arm_no_send_ipi`: the controller sends no IPIs.
#[cfg(feature = "multiprocessor")]
pub fn arm_no_send_ipi(_ci: &CpuInfo, _id: i32) {
    panic(format_args!("arm_send_ipi() called: no ipi function"));
}
/* </CODE> */
