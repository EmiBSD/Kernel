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
//! through the controller installed in `hal::cpu::intr`), the `spl` machinery (`arm_smask`,
//! the `arm_dflt_*` functions, `arm_do_pending_intr`, `arm_init_smask`, `splraise`,
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
//! adapter `bsd`'s drivers keep calling under LZ's names and signatures, and the handles stay
//! `NonNull<MachineIntrHandle>`. The clock table is still here (N7.2b2). N7.2a3: both GICs are
//! the kernel's (`kernel::dev::arm64::{ampintc, agintc}`), each installed as the machine's
//! controller in `hal::cpu::intr` with the kernel's IPI slot set, so `arm_intr_func`,
//! `arm_set_intr_handler`, the IRQ and FIQ dispatch slots and `bsd`'s own
//! `intr_send_ipi_func` are gone; and with bsd's agintc, the last driver that registered a
//! `struct interrupt_controller`, the registration of a legacy controller went too
//! (`LegacyIc`, the adapter that made such a structure the kernel's `InterruptController`,
//! `arm_intr_register_fdt` and `arm_intr_parent_establish_fdt`/`_disestablish_fdt`, which
//! found a legacy controller's registration: none has a caller, the kernel's own are the
//! drivers'). The legacy table type stays for the handles `pciecam` makes by hand.
//!
//! ## Deviations
//! - `arm_clock_func` is a [`StaticCell`], written by `arm_clock_register` during
//!   autoconfiguration on the boot CPU.
//! - `arm_dflt_delay`'s inner loop spins on `yield` so the compiler keeps it; the C's empty
//!   loop body relies on the compiler not optimising it away.
//! - `arm_intr_func` is `hal::cpu::intr::IntrHardware`, implemented by the controller in the
//!   kernel and installed by its attach (N7.2a3); `intr_send_ipi_func` is the kernel's slot
//!   (`kernel::dev::arm64::intr::intr_send_ipi_func`), which [`arm_send_ipi`] reads; `arm_smask`,
//!   `ci_cpl`, `ci_ipending` and the `arm_dflt_*` bodies are `hal::cpu::intr`'s.
//! - `interrupt_controllers` is the kernel's registry (`controllers()`, every registered
//!   controller and the pre-registrations, in the C's order) since N7.2a2:
//!   `acpipci` and `acpi_machdep` walk it there, and `ampintc`'s GICv2m frames, its last
//!   walker here, are the kernel's. `bsd`'s mirror list of the legacy structures is gone.
//! - The `spl` functions (`splraise`, `spllower`, `splx`) and `arm_splassert_check`'s
//!   `setipl` go through the installed controller (`hal::cpu::intr`): the controller's method,
//!   one indirect call, as LZ's `arm_intr_func`.
//! - A handle a driver makes by hand around its own `struct interrupt_controller` is LZ's
//!   `MachineIntrHandle` (`ih_ic`, `ih_ih`, the size `free` checks: `pciecam`'s, whose table
//!   has only `ic_barrier`, the only kind left since N7.2a3); a handle on a registered
//!   controller (the kernel's, all of them) has a null `ih_ic` and carries the kernel's
//!   `IntrHandle` after it (`NativeIntrHandle`); every handle function takes the path its
//!   `ih_ic` says.
//!   `acpi_intr_establish` and `acpipci`'s MSIs establish on the controller they found with
//!   the kernel's `IcReg::establish`/`establish_msi` and take their handle from the adapter
//!   ([`new_handle`]; `acpipci` embeds a [`NativeIntrHandle`] first in its own and tells it
//!   back with [`native_intr_handle`]). No `bsd` reader of `ih_ic` meets a native handle: the
//!   readers read only the handles they made.
//! - `arm_intr_establish_fdt_imap*` take the child's four `reg` cells as a slice (the C's
//!   `int *reg, int nreg`, `nreg` in bytes); `arm_intr_map_msi`'s `data` is a `&mut u64`.
//! - `splassert` before a controller attaches: LZ's table was registered by `initarm`, so
//!   the kernel's callers reached `arm_splassert_check` from the first `spl` on; since N7.2a2
//!   `hal::cpu::intr::splassert` runs the same check itself until a controller installs
//!   (`hal::cpu::intr::splassert_check`, the report through `bsd`'s `splassert_fail` by the
//!   `Printf` bridge): main's behaviour kept, not a gap.

use core::ffi::c_void;
use core::ptr::{self, NonNull};

use crate::sys::staticcell::StaticCell;

use crate::arch::arm64::include::cpu::{CpuInfo, curcpu};
use crate::arch::arm64::include::intr::{IPL_NONE, InterruptController, IntrFn, MachineIntrHandle};
use crate::kassert;
use crate::kern::kern_malloc::{free, malloc};
use crate::kern::kern_sched::cpu_number_of;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::{panic, splassert_fail};
use crate::sys::malloc::{M_DEVBUF, M_WAITOK};
use crate::unported;
use kernel::dev::arm64::intr::{self as core_intr, IntrHandle};
use kernel::dev::intr::IntrHandler;

pub use kernel::dev::arm64::intr::{
    MAX_INTERRUPT_CELLS, arm_intr_cpu_enable, arm_intr_get_parent, arm_intr_init_fdt,
    arm_intr_map_msi, intr_disable_wakeup, intr_enable_wakeup,
};

/// `arm_smask[level]` (`hal::cpu::intr::smask`, the table's owner since N4.5c).
#[inline]
pub fn arm_smask(level: i32) -> u64 {
    hal::cpu::intr::smask(level)
}

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

/// `arm_init_smask`: which soft interrupts each level leaves unmasked, once
/// (`hal::cpu::intr::init_smask`, which owns the table since N4.5c).
pub fn arm_init_smask() {
    hal::cpu::intr::init_smask();
}

/// `splraise`: through the installed controller (`hal::cpu::intr::splraise_raw`: the
/// kernel's `ampintc` or `agintc` itself; the defaults before either attaches).
pub fn splraise(ipl: i32) -> i32 {
    hal::cpu::intr::splraise_raw(ipl)
}

/// `spllower`: through the installed controller.
pub fn spllower(ipl: i32) -> i32 {
    hal::cpu::intr::spllower(ipl)
}

/// `splx`: through the installed controller.
pub fn splx(ipl: i32) {
    hal::cpu::intr::splx(ipl)
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
        hal::cpu::intr::setipl(wantipl);
    }

    if wantipl == IPL_NONE && curcpu().core.idepth() != 0 {
        splassert_fail(-1, curcpu().core.idepth() as i32, func);
    }
}

/// `ci` as the kernel's CPU number.
fn cpu_of(ci: Option<&CpuInfo>) -> Option<usize> {
    ci.map(cpu_number_of)
}

/// A handler of `bsd`'s as the kernel's: the function and its argument's address.
fn raw(func: IntrFn, arg: *mut c_void) -> IntrHandler {
    IntrHandler::Raw(func, arg.expose_provenance())
}

/// A handle on a native controller: `ih_ic` null, the kernel's handle after it. A caller
/// that makes its own handle around one embeds it first, by value (`acpipci`'s MSI handle,
/// `#[repr(C)]` with this first), so every handle function reads it as this one.
#[repr(C)]
pub(crate) struct NativeIntrHandle {
    /// `ih_ic` and `ih_ih` both null: what tells this kind from LZ's.
    mih: MachineIntrHandle,
    /// The kernel's handle.
    ih: IntrHandle,
}

impl NativeIntrHandle {
    /// The handle for the kernel's `ih` (scaffolding, docs/SCAFFOLDING.md: the constructor
    /// `acpipci` embeds in its own MSI handle; N7.11).
    pub(crate) fn new(ih: IntrHandle) -> Self {
        NativeIntrHandle {
            mih: MachineIntrHandle {
                ih_ic: ptr::null(),
                ih_ih: ptr::null_mut(),
            },
            ih,
        }
    }
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
        // SAFETY: only `NativeIntrHandle::new` makes a null `ih_ic`, as the first member of a
        // `#[repr(C)]` `NativeIntrHandle`; one reaches here as the block `new_handle`
        // allocated whole, or as the first member of a `#[repr(C)]` block that embeds it first
        // (`acpipci`'s MSI handle, through `native_intr_handle`, whose caller guarantees it),
        // and `cookie` is that block's own pointer, so the bytes at it are a
        // `NativeIntrHandle`; it lives as long as the `MachineIntrHandle` above.
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

/// The handle a driver gets for the kernel's `ih`: a [`NativeIntrHandle`] (every registered
/// controller is the kernel's since N7.2a3). What `acpi_intr_establish` hands back for the
/// handle it made by hand (`malloc`, then `ih_ic` and `ih_ih` filled; scaffolding, N7.11).
pub(crate) fn new_handle(ih: IntrHandle) -> Option<NonNull<MachineIntrHandle>> {
    alloc(NativeIntrHandle::new(ih)).map(NonNull::cast)
}

/// The kernel's handle behind `cookie` when it is a native one (`ih_ic` null), `None` for
/// LZ's pair: how `acpipci_intr_disestablish` tells its own MSI handle (which embeds a
/// [`NativeIntrHandle`] first) from an INTx one, by the controller's `has_msi` where the C
/// reads `ih_ic->ic_establish_msi` (scaffolding, N7.11).
///
/// # Safety
///
/// `cookie` must be a live handle: one `arm_intr_establish_fdt*` or [`new_handle`] made, a
/// `MachineIntrHandle` a driver made by hand, or a
/// `#[repr(C)]` block whose first member is a [`NativeIntrHandle`].
pub(crate) unsafe fn native_intr_handle(cookie: NonNull<MachineIntrHandle>) -> Option<IntrHandle> {
    match handle(cookie) {
        Handle::Native(ih) => Some(ih),
        Handle::Legacy(..) => None,
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
            kassert!(ic.ic_barrier.is_some());
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

/// `arm_send_ipi`: sends IPI `id` (`ARM_IPI_*`) to `ci` through `intr_send_ipi_func`, the
/// kernel's slot that the attaching controller sets (`arm_no_send_ipi` before it).
#[cfg(feature = "multiprocessor")]
pub fn arm_send_ipi(ci: &CpuInfo, id: i32) {
    match core_intr::intr_send_ipi_func() {
        Some(f) => f(cpu_number_of(ci), id),
        None => arm_no_send_ipi(ci, id),
    }
}

/// `arm_no_send_ipi`: the controller sends no IPIs.
#[cfg(feature = "multiprocessor")]
pub fn arm_no_send_ipi(_ci: &CpuInfo, _id: i32) {
    panic(format_args!("arm_send_ipi() called: no ipi function"));
}
/* </CODE> */
