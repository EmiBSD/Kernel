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
 * Copyright (c) 2001-2004 Opsycon AB  (www.opsycon.se / www.opsycon.com)
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS
 * OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
 * DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 */
/* </LICENSES> */

/* <CODE> */
//! arm64 `<machine/intr.h>`: interrupt priority levels and the interrupt framework.
//!
//! Upstream: sys/arch/arm64/include/intr.h @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M3 ports the levels; M4 adds the `IST_*` trigger types,
//! `SOFTINTR_XCALL`, `struct machine_intr_handle`, `struct arm_intr_func` and
//! `struct interrupt_controller`; M11a the `ARM_IPI_*` numbers (`MULTIPROCESSOR`); M12
//! `ic_establish_msi` (the GICv2m frame, PCI MSI).
//! The functions it declares are `arm64/intr.rs`; the `spl*()` helpers are the
//! `machine::intr` contract.
//!
//! N7.2a1: the levels are `hal::cpu::intr`'s (the owner of `spl(9)`'s numbers), the trigger
//! types, `SOFTINTR_XCALL`, the IPI numbers and the handler type the kernel's
//! (`kernel::dev::arm64::intr`, `kernel::dev::intr`), re-exported here under their names;
//! `struct machine_intr_handle`, `struct arm_intr_func` and `struct interrupt_controller`
//! stay for the drivers still in `bsd`, which the kernel reaches through `arm64/intr.rs`'s
//! adapter.
//!
//! ## Deviations
//! - `ic_establish`'s `char *name` is a `&'static str`; the controller's `ic_cookie` is the
//!   controller itself in every driver, so it is a `*const ()`. `ic_establish_msi`'s
//!   `uint64_t *addr, *data` are `&mut u64`.
//! - The C's `void *` cell arrays (`int *cell`) are `&[u32]`.

pub use hal::cpu::intr::{
    IPL_AUDIO, IPL_BIO, IPL_CLOCK, IPL_FLAGMASK, IPL_HIGH, IPL_IPI, IPL_IRQMASK, IPL_MPFLOOR,
    IPL_MPSAFE, IPL_NET, IPL_NONE, IPL_SCHED, IPL_SOFTCLOCK, IPL_SOFTNET, IPL_SOFTTTY,
    IPL_STATCLOCK, IPL_TTY, IPL_VM, IPL_WAKEUP, NIPL,
};
pub use kernel::dev::arm64::intr::{
    ARM_IPI_DDB, ARM_IPI_HALT, ARM_IPI_NOP, ARM_IPI_XCALL, IST_EDGE, IST_EDGE_BOTH,
    IST_EDGE_FALLING, IST_EDGE_RISING, IST_LEVEL, IST_LEVEL_HIGH, IST_LEVEL_LOW, IST_NONE,
    IST_PULSE, SOFTINTR_XCALL,
};
pub use kernel::dev::intr::IntrFn;

/// `struct machine_intr_handle`: what `arm_intr_establish_fdt` returns.
pub struct MachineIntrHandle {
    /// `ih_ic`: the controller.
    pub ih_ic: *const InterruptController,
    /// `ih_ih`: the controller's own handle.
    pub ih_ih: *mut core::ffi::c_void,
}

/// `struct arm_intr_func`: the interrupt controller's `spl` implementation.
pub struct ArmIntrFunc {
    /// `raise`.
    pub raise: fn(i32) -> i32,
    /// `lower`.
    pub lower: fn(i32) -> i32,
    /// `x`.
    pub x: fn(i32),
    /// `setipl`.
    pub setipl: fn(i32),
    /// `enable_wakeup`.
    pub enable_wakeup: Option<fn()>,
    /// `disable_wakeup`.
    pub disable_wakeup: Option<fn()>,
}

/// `ic_establish(cookie, cell, level, ci, func, arg, name)`.
pub type IcEstablishFn = fn(
    *const (),
    &[u32],
    i32,
    Option<&'static crate::arch::arm64::include::cpu::CpuInfo>,
    IntrFn,
    *mut core::ffi::c_void,
    &'static str,
) -> *mut core::ffi::c_void;

/// `ic_establish_msi(cookie, addr, data, level, ci, func, arg, name)`: establishes an MSI
/// and hands back, through `addr` and `data`, the doorbell address and the payload the
/// device must write; `data` comes in as the device's requester ID or `msi-map` output.
pub type IcEstablishMsiFn = fn(
    *const (),
    &mut u64,
    &mut u64,
    i32,
    Option<&'static crate::arch::arm64::include::cpu::CpuInfo>,
    IntrFn,
    *mut core::ffi::c_void,
    &'static str,
) -> *mut core::ffi::c_void;

/// `struct interrupt_controller`: a registered interrupt controller.
pub struct InterruptController {
    /// `ic_node`: the device tree node.
    pub ic_node: core::cell::Cell<i32>,
    /// `ic_cookie`.
    pub ic_cookie: core::cell::Cell<*const ()>,
    /// `ic_establish`.
    pub ic_establish: Option<IcEstablishFn>,
    /// `ic_establish_msi`.
    pub ic_establish_msi: Option<IcEstablishMsiFn>,
    /// `ic_disestablish`.
    pub ic_disestablish: Option<fn(*mut core::ffi::c_void)>,
    /// `ic_enable`.
    pub ic_enable: Option<fn(*mut core::ffi::c_void)>,
    /// `ic_disable`.
    pub ic_disable: Option<fn(*mut core::ffi::c_void)>,
    /// `ic_route`.
    pub ic_route:
        Option<fn(*mut core::ffi::c_void, bool, &crate::arch::arm64::include::cpu::CpuInfo)>,
    /// `ic_cpu_enable`.
    pub ic_cpu_enable: Option<fn()>,
    /// `ic_barrier`.
    pub ic_barrier: Option<fn(*mut core::ffi::c_void)>,
    /// `ic_set_wakeup`.
    pub ic_set_wakeup: Option<fn(*mut core::ffi::c_void)>,
    /// `ic_list`: the `interrupt_controllers` link.
    pub ic_list: crate::sys::queue::ListEntry<InterruptController>,
    /// `ic_phandle`.
    pub ic_phandle: core::cell::Cell<u32>,
    /// `ic_cells`: `#interrupt-cells`.
    pub ic_cells: core::cell::Cell<u32>,
    /// `ic_gic_its_id`.
    pub ic_gic_its_id: core::cell::Cell<u32>,
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/arch/arm64/include/intr.h");
        let ours: &[(&str, i64)] = &[
            ("IPL_NONE", i64::from(IPL_NONE)),
            ("IPL_SOFTCLOCK", i64::from(IPL_SOFTCLOCK)),
            ("IPL_SOFTNET", i64::from(IPL_SOFTNET)),
            ("IPL_SOFTTTY", i64::from(IPL_SOFTTTY)),
            ("IPL_BIO", i64::from(IPL_BIO)),
            ("IPL_NET", i64::from(IPL_NET)),
            ("IPL_TTY", i64::from(IPL_TTY)),
            ("IPL_VM", i64::from(IPL_VM)),
            ("IPL_AUDIO", i64::from(IPL_AUDIO)),
            ("IPL_CLOCK", i64::from(IPL_CLOCK)),
            ("IPL_HIGH", i64::from(IPL_HIGH)),
            ("IPL_IPI", i64::from(IPL_IPI)),
            ("IPL_IRQMASK", i64::from(IPL_IRQMASK)),
            ("IPL_FLAGMASK", i64::from(IPL_FLAGMASK)),
            ("IPL_MPSAFE", i64::from(IPL_MPSAFE)),
            ("IPL_WAKEUP", i64::from(IPL_WAKEUP)),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
