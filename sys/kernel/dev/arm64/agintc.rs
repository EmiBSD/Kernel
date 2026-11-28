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
 * Copyright (c) 2007, 2009, 2011, 2017 Dale Rahn <drahn@dalerahn.com>
 * Copyright (c) 2018 Mark Kettenis <kettenis@openbsd.org>
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
//! The ARM generic interrupt controller, versions 3 and 4, in the kernel: `agintc(4)` and its
//! ITS (`agintcmsi`), `arch/arm64/dev/agintc.c`. This is a device driver for the GICv3/GICv4
//! IP from ARM as specified in IHI0069C, an example of this hardware is the GIC 500.
//!
//! Upstream: sys/arch/arm64/dev/agintc.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/agintc.rs@e8249449d905
//!
//! The controller attaches at `fdt` (`agintc* at fdt? early 1`): it maps the distributor
//! (`GICD_*`) and the redistributor regions, finds one redistributor per CPU by walking their
//! `GICR_TYPER` frames, allocates the LPI configuration and pending tables when the
//! distributor has LPIs, reads the message based SPIs (`mbi-ranges`), learns the priority
//! shifts (the non-secure view and the RK3399's secure-only registers), resets the
//! distributor, wakes this CPU's redistributor (`agintc_cpuinit`), becomes the machine's
//! interrupt controller (`hal::cpu::intr::install`, the C's `arm_set_intr_handler`: `spl(9)`,
//! the IRQ dispatch and the wakeup hooks are its methods), enables group 1, finds the IPI's
//! SGI and establishes it, registers with the interrupt core (`arm_intr_register_fdt`) and
//! attaches its children, the ITS, as a simplebus. The CPU interface is reached through its
//! system registers (`hal::cpu::icc`: the priority mask is the `spl` level, `ICC_IAR1` and
//! `ICC_EOIR1` acknowledge and end an interrupt, `ICC_SGI1R` raises the IPIs).
//!
//! The ITS (`agintcmsi* at fdt? early 1`, `arm,gic-v3-its`) sets up its command queue and its
//! device and collection tables (flat or indirect), maps one collection per CPU and registers
//! as an MSI controller: an MSI is an LPI of the GIC that the ITS translates from the
//! device's write (`MAPD`, `MAPTI`, `INV`, `DISCARD`, `SYNC`).
//!
//! ## Deviations
//! - A handler record is never freed (P12, as `ampintc`'s): the C frees `struct intrhand` at
//!   its disestablish, and `struct agintc_lpi_info` at an MSI's; here both are unlinked
//!   (the LPI's slot emptied), the handler's counter detached, the handler marked dead and
//!   kept (an `M_DEVBUF` block each per disestablish: `vmstat -m`'s `devbuf` counts move,
//!   which no smoke compares), because the dispatch walks the chain and reads the LPI's slot
//!   with no lock. The C's race stays, made safe: a dispatch on another CPU that already read
//!   the record calls it once after its disestablish (the C may read freed memory). A second
//!   disestablish of the same record is `panic(9)`, where the C frees it twice.
//! - The LPI slots (`sc_lpi`, a `mallocarray` of `sc_nlpi` pointers, 64 KiB for 8192 LPIs)
//!   are one `hal::ds::list::List` head each (two words: 128 KiB), so the dispatch reads a
//!   slot another CPU fills or empties with no lock (the list holds at most one record).
//! - The machine's interrupt controller is the first that installs itself: a second GIC's
//!   attach is `panic(9)` where the C's `arm_set_intr_handler` would replace the first's
//!   table (no machine has two).
//! - The primary CPU (`cpu_info_primary`, an establish without a CPU) is the CPU that ran the
//!   attach: mainbus attaches the controller during `cpu_configure` on the boot CPU, which is
//!   the primary one. Every CPU's `ci_mpidr` is read once at the attach (the CPUs attach
//!   before the GIC, on device tree and ACPI boots alike), where the C reads it at each
//!   route and SGI.
//! - Each redistributor's frames are mapped on their own, a second mapping of frames the
//!   region's mapping (kept, as the C's `sc_rbase_ioh`) already covers: the C reaches a
//!   redistributor through `bus_space_subregion` of its region, which `hal` has no form of.
//! - `agintc_send_ipi` puts a `dsb ish` before the `ICC_SGI1R` write and an `isb` after it,
//!   as LZ: the posted reason is visible to the target before the SGI is (a system register
//!   write is not ordered by the atomic's `dmb`).
//! - `agintc_attach`'s failure path (`unmap`) restores the interrupt state it disabled, as LZ;
//!   the C returns with interrupts still disabled.
//! - `agintc_msi_create_device` returns no device when its ITT cannot be allocated (the C
//!   would hand the ITS a NULL table), as LZ; `agintc_intr_establish_msi`'s event ID check
//!   takes a shift past 31 as a free bit (the C's `1U << eventid` is undefined there), as LZ.
//! - An MSI establish or disestablish is serialised by the GIC's sleeping lock (`agintcmsi`,
//!   the C's kernel lock and autoconf); the command queue, the LPI configuration table and
//!   the device table's writes by a mutex each (the C relies on the same kernel lock).
//! - `ffs`/`fls` (libkern) are local helpers until libkern ports them, as LZ.
//! - `agintc_ipi_count` (the `qemu` self-check of `cpu_boot_secondary_processors`) is not in
//!   the C.
//!
//! ## Redesign
//! - `struct agintc_softc` (the simplebus softc first, the tag and handles, `mallocarray`ed
//!   redistributor handles, processor numbers, handler queues and LPI slots, the shifts, the
//!   DMA tables, the embedded `struct interrupt_controller`) and the global `agintc_sc` ->
//!   [`AgintcSoftc`], built whole at attach (`Mmio<Gicd>`, one `Mmio<Gicr>` per
//!   redistributor mapped through the bus, the queues an `MArray` of atomics, the tables
//!   `hal::dev::dma::DmaBuf`s with the bus's DMA address) and kept in [`AGINTC`], a set-once
//!   `MArc`; it implements `hal::cpu::intr::IntrHardware` (the C's `arm_set_intr_handler`
//!   table, its wakeup hooks included) and `InterruptController` (the C's `sc_ic` entries:
//!   `ic_establish_msi` is the MBI establish, [`InterruptController::has_msi`] when the
//!   distributor has MBI ranges). LZ's static softc of `Cell`s with its `UnsafeCell` `sc_ic`
//!   and `unsafe impl Sync` is gone; so are the simplebus softc (the ITS's bus is
//!   [`simplebus_attach`]'s) and `ATTACHED` (`AGINTC.get()`).
//! - The CPU interface's `mrs`/`msr` (LZ: `icc_read!`/`icc_write!` and `isb()`/`dsb_sy()`,
//!   four `unsafe` sites in `bsd`) -> `hal::cpu::icc`'s safe functions, each with its barrier.
//! - `struct intrhand` -> [`Intrhand`]: immutable once linked but for `ih_flags`, an atomic
//!   (`agintc_intr_set_wakeup` ORs `IPL_WAKEUP` in), `MBox::leak`ed at establish and linked
//!   with `hal::ds::list::List::insert_tail` under the softc's mutex and the C's
//!   `intr_disable()`; its cookie is `IcCookie::Rec`. The dispatch walks `List::iter` with no
//!   lock (the C's `TAILQ_FOREACH`, O(1) a step). See the Deviations.
//! - `struct agintc_lpi_info` and `sc_lpi[]` -> [`AgintcLpiInfo`], leaked, linked alone on its
//!   LPI's slot list ([`AgintcSoftc::lpi`]), handed out as the MSI's cookie
//!   (`IcCookie::Rec`, where the C's cookie is the slot's address). The dispatch's LPI arm
//!   reads the slot's first record and its handler: two loads, as the C's
//!   `sc_lpi[irq - LPI_BASE]->li_ih`.
//! - An MBI range's slots (`void *`s whose only use is NULL or not) -> `AtomicBool`s claimed
//!   by a compare-and-swap before the establish (which may sleep) and freed if it fails.
//! - The registers: `bus_space_read/write_N(iot, ioh, off)` -> `Mmio<Gicd>`, `Mmio<Gicr>` and
//!   `Mmio<Gits>`; a fixed register a `Reg` (no check), a per-IRQ one a computed offset (one
//!   bounds compare; establish, route and the resets, not the dispatch).
//! - `struct agintc_dmamem` (a map, a segment, a `caddr_t` written through casts) ->
//!   [`AgintcDmamem`]: a `DmaBuf` (zeroed, uncached, one segment, freed by its drop) and the
//!   address the device is told (`AGINTC_DMA_DVA`: the segment after every bus's
//!   `dma-ranges`, `FdtBus::dma_addr`). A write is a checked volatile store, then
//!   `DmaBuf::sync` (`cpu_dcache_wb_range`) and `hal::cpu::icc::dsb_sy`, the C's "make
//!   globally visible". The buffer is always made by the non-coherent provider
//!   (`bus_dma_memory(false)`), whatever the bus's tag says: the C writes the lines back
//!   unconditionally (`agintc_dmamem_alloc`, the table writes), because the GIC and the ITS
//!   read the tables non-cacheable while the page system zeroed them through the cached
//!   direct map; on a coherent tag `DmaBuf::sync` would be a barrier alone and leave those
//!   dirty lines to be evicted over the ITS's commands later. The mapping stays uncached
//!   (`BUS_DMA_NOCACHE`). The indirect device table's second level, which the C leaks
//!   (`FIXME: leaks`), is a leaked `MBox` of its buffer: the ITS reads it for ever.
//! - `struct gits_cmd`, `memcpy`ed into the queue -> [`GitsCmd::words`], four little-endian
//!   double words, the same bytes; the queue and its index in one mutex with the ITS's
//!   `GITS_CWRITER` write and the wait for `GITS_CREADR` ([`ItsCmdq`]).
//! - `struct agintc_msi_device` on `sc_msi_devices` -> [`AgintcMsiDevice`], leaked, on a
//!   `hal::ds::list::List` (`LIST_INSERT_HEAD`, walked as the C's `LIST_FOREACH`), its event
//!   bits an atomic.
//! - `ampintc_run_handler`'s `ih_arg ? ih_arg : frame` -> `IntrHandler::call`; the IPI
//!   handler's softc argument is a counted handle (`IntrHandler::Shared`).
//! - `intr_send_ipi_func = agintc_send_ipi` -> the kernel's IPI slot (`intr_set_send_ipi`);
//!   `db_enter()` and `cpu_halt()` of the IPI reasons -> `hal::cpu::trap::db_enter` and
//!   `cpu_halt`.
//! - `agintc_intr_barrier`'s `sched_barrier(ih->ih_ci)` -> the CPU to wait for
//!   (`BarrierWait::Cpu`), which the core's `intr_barrier` waits for.
//! - `ci_mpidr` of every CPU -> the softc's copy, read from `hal::cpu::CpuCore::hwid` at the
//!   attach; `curcpu()->ci_midr` (the ITS's Oryon test) -> `CpuCore::midr`.

use core::any::Any;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

use hal::cpu::clock::ClockFrame;
use hal::cpu::icc;
use hal::cpu::intr::{
    IPL_FLAGMASK, IPL_HIGH, IPL_IPI, IPL_IRQMASK, IPL_MPSAFE, IPL_NONE, IPL_SCHED, IPL_WAKEUP,
    InIrq, IntrHardware,
};
use hal::dev::dma::{DmaBuf, DmaReq, DmaSync};
use hal::dev::mmio::{DevCache, Mmio, Reg, RegBlock};
use hal::ds::list::{List, ListEntry};
use hal::mem::malloc::{M_WAITOK, MArc, MArray, MBox};
use hal::mem::pool::AnyArc;
use hal::sync::mutex::Mutexed;
use hal::sync::once::OnceStatic;
use hal::sync::rwlock::Rwlocked;
use sysdefs::base::errno::Errno;
use sysdefs::base::param::PAGE_SIZE;

use crate::assert::{kassert, kpanic};
use crate::base::prf::{Str, printf};
use crate::dev::arm64::fdt::{FdtArgs, FdtBusRef};
use crate::dev::arm64::intr::{
    ARM_IPI_DDB, ARM_IPI_HALT, ARM_IPI_NOP, BarrierWait, IST_EDGE_RISING, IST_LEVEL_HIGH, IcCookie,
    IcDesc, InterruptController, arm_intr_register_fdt, intr_set_send_ipi,
};
use crate::dev::arm64::mainbus::MPIDR_AFF;
use crate::dev::arm64::simplebus::simplebus_attach;
use crate::dev::autoconf::config_activate_children;
use crate::dev::device::{
    Aux, DV_DULL, DVACT_RESUME, DevRef, Devclass, DriverOps, Ident, M_DEVBUF, Match, UnitTable,
};
use crate::dev::evcount::{Evcount, evcount_attach, evcount_detach};
use crate::dev::intr::IntrHandler;
use crate::dev::ofw::fdt::{
    OF_getpropint, OF_getpropint64, OF_getpropintarray, OF_getproplen, OF_is_compatible, OF_peer,
};

/// `M_TEMP` (`<sys/malloc.h>`): the `mbi-ranges` cells.
const M_TEMP: i32 = 127;

/// `ICC_SRE_EL1_EN`: `SRE`, `DFB` and `DIB`.
pub const ICC_SRE_EL1_EN: u64 = 0x7;

/// `ICC_CTLR_EL1_PRIBITS(reg)` (`<machine/armreg.h>`): the priority bits implemented, minus
/// one.
pub const fn icc_ctlr_el1_pribits(reg: u64) -> u64 {
    (reg >> 8) & 0x7
}

/* distributor registers */
/// `GICD_CTLR`.
pub const GICD_CTLR: usize = 0x0000;
/// `GICD_CTLR_RWP` (non-secure).
pub const GICD_CTLR_RWP: u32 = 1 << 31;
/// `GICD_CTLR_EnableGrp1`.
pub const GICD_CTLR_ENABLE_GRP1: u32 = 1 << 0;
/// `GICD_CTLR_EnableGrp1A`.
pub const GICD_CTLR_ENABLE_GRP1A: u32 = 1 << 1;
/// `GICD_CTLR_ARE_NS`.
pub const GICD_CTLR_ARE_NS: u32 = 1 << 4;
/// `GICD_CTLR_DS`: one security state.
pub const GICD_CTLR_DS: u32 = 1 << 6;
/// `GICD_TYPER`.
pub const GICD_TYPER: usize = 0x0004;
/// `GICD_TYPER_MBIS`: message based SPIs.
pub const GICD_TYPER_MBIS: u32 = 1 << 16;
/// `GICD_TYPER_LPIS`.
pub const GICD_TYPER_LPIS: u32 = 1 << 17;
/// `GICD_TYPER_ITLINE_M`.
pub const GICD_TYPER_ITLINE_M: u32 = 0x1f;
/// `GICD_IIDR`.
pub const GICD_IIDR: usize = 0x0008;
/// `GICD_SETSPI_NSR`.
pub const GICD_SETSPI_NSR: u64 = 0x0040;
/// `GICD_CLRSPI_NSR`.
pub const GICD_CLRSPI_NSR: u64 = 0x0048;

/// `IRQ_TO_REG32(i)`.
pub const fn irq_to_reg32(i: usize) -> usize {
    (i >> 5) & 0x1f
}

/// `IRQ_TO_REG32BIT(i)`.
pub const fn irq_to_reg32bit(i: usize) -> u32 {
    (i & 0x1f) as u32
}

/// `IRQ_TO_REG16(i)`.
pub const fn irq_to_reg16(i: usize) -> usize {
    (i >> 4) & 0x3f
}

/// `IRQ_TO_REG16BIT(i)`.
pub const fn irq_to_reg16bit(i: usize) -> u32 {
    (i & 0xf) as u32
}

/// `GICD_IGROUPR(i)`.
pub const fn gicd_igroupr(i: usize) -> usize {
    0x0080 + irq_to_reg32(i) * 4
}

/// `GICD_ISENABLER(i)`.
pub const fn gicd_isenabler(i: usize) -> usize {
    0x0100 + irq_to_reg32(i) * 4
}

/// `GICD_ICENABLER(i)`.
pub const fn gicd_icenabler(i: usize) -> usize {
    0x0180 + irq_to_reg32(i) * 4
}

/// `GICD_ISPENDR(i)`.
pub const fn gicd_ispendr(i: usize) -> usize {
    0x0200 + irq_to_reg32(i) * 4
}

/// `GICD_ICPENDR(i)`.
pub const fn gicd_icpendr(i: usize) -> usize {
    0x0280 + irq_to_reg32(i) * 4
}

/// `GICD_ISACTIVER(i)`.
pub const fn gicd_isactiver(i: usize) -> usize {
    0x0300 + irq_to_reg32(i) * 4
}

/// `GICD_ICACTIVER(i)`.
pub const fn gicd_icactiver(i: usize) -> usize {
    0x0380 + irq_to_reg32(i) * 4
}

/// `GICD_IPRIORITYR(i)`: one byte per interrupt.
pub const fn gicd_ipriorityr(i: usize) -> usize {
    0x0400 + i
}

/// `GICD_ICFGR(i)`.
pub const fn gicd_icfgr(i: usize) -> usize {
    0x0c00 + irq_to_reg16(i) * 4
}

/// `GICD_ICFGR_TRIG_LEVEL(i)`.
pub const fn gicd_icfgr_trig_level(i: usize) -> u32 {
    0x0 << (irq_to_reg16bit(i) * 2)
}

/// `GICD_ICFGR_TRIG_EDGE(i)`.
pub const fn gicd_icfgr_trig_edge(i: usize) -> u32 {
    0x2 << (irq_to_reg16bit(i) * 2)
}

/// `GICD_ICFGR_TRIG_MASK(i)`.
pub const fn gicd_icfgr_trig_mask(i: usize) -> u32 {
    0x2 << (irq_to_reg16bit(i) * 2)
}

/// `GICD_IGRPMODR(i)`.
pub const fn gicd_igrpmodr(i: usize) -> usize {
    0x0d00 + irq_to_reg32(i) * 4
}

/// `GICD_NSACR(i)`.
pub const fn gicd_nsacr(i: usize) -> usize {
    0x0e00 + irq_to_reg16(i) * 4
}

/// `GICD_IROUTER(i)`: 64 bits per SPI, the affinity it is routed to.
pub const fn gicd_irouter(i: usize) -> usize {
    0x6000 + i * 8
}

/* redistributor registers */
/// `GICR_CTLR`.
pub const GICR_CTLR: usize = 0x00000;
/// `GICR_CTLR_RWP`.
pub const GICR_CTLR_RWP: u32 = (1 << 31) | (1 << 3);
/// `GICR_CTLR_ENABLE_LPIS`.
pub const GICR_CTLR_ENABLE_LPIS: u32 = 1 << 0;
/// `GICR_IIDR`.
pub const GICR_IIDR: usize = 0x00004;
/// `GICR_TYPER`: 64 bits; the affinity in 63:32, the processor number in 23:8.
pub const GICR_TYPER: usize = 0x00008;
/// `GICR_TYPER_LAST`: the last redistributor of its region.
pub const GICR_TYPER_LAST: u64 = 1 << 4;
/// `GICR_TYPER_VLPIS`: two more 64 KB frames for virtual LPIs.
pub const GICR_TYPER_VLPIS: u64 = 1 << 1;
/// `GICR_WAKER`.
pub const GICR_WAKER: usize = 0x00014;
/// `GICR_WAKER_X31`.
pub const GICR_WAKER_X31: u32 = 1 << 31;
/// `GICR_WAKER_CHILDRENASLEEP`.
pub const GICR_WAKER_CHILDRENASLEEP: u32 = 1 << 2;
/// `GICR_WAKER_PROCESSORSLEEP`.
pub const GICR_WAKER_PROCESSORSLEEP: u32 = 1 << 1;
/// `GICR_WAKER_X0`.
pub const GICR_WAKER_X0: u32 = 1 << 0;
/// `GICR_PROPBASER`.
pub const GICR_PROPBASER: usize = 0x00070;
/// `GICR_PROPBASER_ISH`.
pub const GICR_PROPBASER_ISH: u64 = 1 << 10;
/// `GICR_PROPBASER_IC_NORM_NC`.
pub const GICR_PROPBASER_IC_NORM_NC: u64 = 1 << 7;
/// `GICR_PENDBASER`.
pub const GICR_PENDBASER: usize = 0x00078;
/// `GICR_PENDBASER_PTZ`: the pending table is zeroed.
pub const GICR_PENDBASER_PTZ: u64 = 1 << 62;
/// `GICR_PENDBASER_ISH`.
pub const GICR_PENDBASER_ISH: u64 = 1 << 10;
/// `GICR_PENDBASER_IC_NORM_NC`.
pub const GICR_PENDBASER_IC_NORM_NC: u64 = 1 << 7;
/// `GICR_IGROUPR0`.
pub const GICR_IGROUPR0: usize = 0x10080;
/// `GICR_ISENABLE0`.
pub const GICR_ISENABLE0: usize = 0x10100;
/// `GICR_ICENABLE0`.
pub const GICR_ICENABLE0: usize = 0x10180;
/// `GICR_ISPENDR0`.
pub const GICR_ISPENDR0: usize = 0x10200;
/// `GICR_ICPENDR0`.
pub const GICR_ICPENDR0: usize = 0x10280;
/// `GICR_ISACTIVE0`.
pub const GICR_ISACTIVE0: usize = 0x10300;
/// `GICR_ICACTIVE0`.
pub const GICR_ICACTIVE0: usize = 0x10380;

/// `GICR_IPRIORITYR(i)`: one byte per SGI or PPI, in the SGI frame.
pub const fn gicr_ipriorityr(i: usize) -> usize {
    0x10400 + i
}

/// `GICR_ICFGR0`.
pub const GICR_ICFGR0: usize = 0x10c00;
/// `GICR_ICFGR1`.
pub const GICR_ICFGR1: usize = 0x10c04;
/// `GICR_IGRPMODR0`.
pub const GICR_IGRPMODR0: usize = 0x10d00;

/// `GICR_PROP_SIZE`: the LPI configuration table.
pub const GICR_PROP_SIZE: usize = 64 * 1024;
/// `GICR_PROP_GROUP1`.
pub const GICR_PROP_GROUP1: u8 = 1 << 1;
/// `GICR_PROP_ENABLE`.
pub const GICR_PROP_ENABLE: u8 = 1 << 0;
/// `GICR_PEND_SIZE`: the LPI pending table.
pub const GICR_PEND_SIZE: usize = 64 * 1024;

/// `PPI_BASE`.
pub const PPI_BASE: usize = 16;
/// `SPI_BASE`.
pub const SPI_BASE: usize = 32;
/// `LPI_BASE`.
pub const LPI_BASE: usize = 8192;

/// `IRQ_ENABLE`.
pub const IRQ_ENABLE: bool = true;
/// `IRQ_DISABLE`.
pub const IRQ_DISABLE: bool = false;

/*
 * GICv3 ITS controller for MSI interrupts.
 */
/// `GITS_CTLR`.
pub const GITS_CTLR: usize = 0x0000;
/// `GITS_CTLR_ENABLED`.
pub const GITS_CTLR_ENABLED: u32 = 1 << 0;
/// `GITS_TYPER`.
pub const GITS_TYPER: usize = 0x0008;
/// `GITS_TYPER_CIL`: the collection ID limit is `GITS_TYPER_CIDBITS`.
pub const GITS_TYPER_CIL: u64 = 1 << 36;

/// `GITS_TYPER_CIDBITS(x)`.
pub const fn gits_typer_cidbits(x: u64) -> u64 {
    (x >> 32) & 0xf
}

/// `GITS_TYPER_HCC(x)`.
pub const fn gits_typer_hcc(x: u64) -> u64 {
    (x >> 24) & 0xff
}

/// `GITS_TYPER_PTA`: the target addresses are physical ones.
pub const GITS_TYPER_PTA: u64 = 1 << 19;

/// `GITS_TYPER_DEVBITS(x)`.
pub const fn gits_typer_devbits(x: u64) -> u64 {
    (x >> 13) & 0x1f
}

/// `GITS_TYPER_ITE_SZ(x)`.
pub const fn gits_typer_ite_sz(x: u64) -> u64 {
    (x >> 4) & 0xf
}

/// `GITS_TYPER_PHYS`: physical LPIs.
pub const GITS_TYPER_PHYS: u64 = 1 << 0;
/// `GITS_CBASER`.
pub const GITS_CBASER: usize = 0x0080;
/// `GITS_CBASER_VALID`.
pub const GITS_CBASER_VALID: u64 = 1 << 63;
/// `GITS_CBASER_IC_NORM_NC`.
pub const GITS_CBASER_IC_NORM_NC: u64 = 1 << 59;
/// `GITS_CBASER_MASK`.
pub const GITS_CBASER_MASK: u64 = 0x1f_ffff_ffff_f000;
/// `GITS_CWRITER`.
pub const GITS_CWRITER: usize = 0x0088;
/// `GITS_CREADR`.
pub const GITS_CREADR: usize = 0x0090;

/// `GITS_BASER(i)`.
pub const fn gits_baser(i: usize) -> usize {
    0x0100 + i * 8
}

/// `GITS_BASER_VALID`.
pub const GITS_BASER_VALID: u64 = 1 << 63;
/// `GITS_BASER_INDIRECT`.
pub const GITS_BASER_INDIRECT: u64 = 1 << 62;
/// `GITS_BASER_IC_NORM_NC`.
pub const GITS_BASER_IC_NORM_NC: u64 = 1 << 59;
/// `GITS_BASER_TYPE_MASK`.
pub const GITS_BASER_TYPE_MASK: u64 = 7 << 56;
/// `GITS_BASER_TYPE_DEVICE`.
pub const GITS_BASER_TYPE_DEVICE: u64 = 1 << 56;
/// `GITS_BASER_TYPE_COLL`.
pub const GITS_BASER_TYPE_COLL: u64 = 4 << 56;

/// `GITS_BASER_TTE_SZ(x)`.
pub const fn gits_baser_tte_sz(x: u64) -> u64 {
    (x >> 48) & 0x1f
}

/// `GITS_BASER_PGSZ_MASK`.
pub const GITS_BASER_PGSZ_MASK: u64 = 3 << 8;
/// `GITS_BASER_PGSZ_4K`.
pub const GITS_BASER_PGSZ_4K: u64 = 0 << 8;
/// `GITS_BASER_PGSZ_16K`.
pub const GITS_BASER_PGSZ_16K: u64 = 1 << 8;
/// `GITS_BASER_PGSZ_64K`.
pub const GITS_BASER_PGSZ_64K: u64 = 2 << 8;
/// `GITS_BASER_SZ_MASK`.
pub const GITS_BASER_SZ_MASK: u64 = 0xff;
/// `GITS_BASER_PA_MASK`.
pub const GITS_BASER_PA_MASK: u64 = 0x7fff_ffff_f000;
/// `GITS_TRANSLATER`: the doorbell a device writes an MSI's event ID to.
pub const GITS_TRANSLATER: u64 = 0x10040;

/// `GITS_NUM_BASER`.
pub const GITS_NUM_BASER: usize = 8;

/// `GITS_CMD_VALID`.
pub const GITS_CMD_VALID: u64 = 1 << 63;

/* ITS commands */
/// `SYNC`.
pub const SYNC: u8 = 0x05;
/// `MAPD`.
pub const MAPD: u8 = 0x08;
/// `MAPC`.
pub const MAPC: u8 = 0x09;
/// `MAPTI`.
pub const MAPTI: u8 = 0x0a;
/// `INV`.
pub const INV: u8 = 0x0c;
/// `INVALL`.
pub const INVALL: u8 = 0x0d;
/// `DISCARD`.
pub const DISCARD: u8 = 0x0f;

/// `GITS_CMDQ_SIZE`.
pub const GITS_CMDQ_SIZE: usize = 64 * 1024;
/// `sizeof(struct gits_cmd)`.
pub const GITS_CMD_SIZE: usize = 32;
/// `GITS_CMDQ_NENTRIES`.
pub const GITS_CMDQ_NENTRIES: usize = GITS_CMDQ_SIZE / GITS_CMD_SIZE;

/// `CPU_IMPL(midr)`.
pub const fn cpu_impl(midr: u64) -> u64 {
    (midr >> 24) & 0xff
}

/// `CPU_PART(midr)`.
pub const fn cpu_part(midr: u64) -> u64 {
    (midr >> 4) & 0xfff
}

/// `CPU_IMPL_QCOM`.
pub const CPU_IMPL_QCOM: u64 = 0x51;
/// `CPU_PART_ORYON`.
pub const CPU_PART_ORYON: u64 = 0x001;

/// The distributor's registers (`sc_d_ioh`): 64 KiB, `GICD_IROUTER` the last used.
pub struct Gicd;

impl RegBlock for Gicd {
    const SIZE: usize = 0x10000;
}

/// A redistributor region (`sc_rbase_ioh[idx]`): its frames walked by `GICR_TYPER`; one
/// redistributor's two frames at least.
pub struct GicrRegion;

impl RegBlock for GicrRegion {
    const SIZE: usize = 0x20000;
}

/// One redistributor's frames (`sc_r_ioh[n]`): `RD_base` and `SGI_base`, 64 KiB each.
pub struct Gicr;

impl RegBlock for Gicr {
    const SIZE: usize = 0x20000;
}

/// The ITS's control registers (`sc_ioh`), up to `GITS_BASER(7)`; `GITS_TRANSLATER`, in the
/// next frame, is the devices'.
pub struct Gits;

impl RegBlock for Gits {
    const SIZE: usize = 0x200;
}

const DCTLR: Reg<Gicd, u32> = Reg::new(GICD_CTLR);
const DTYPER: Reg<Gicd, u32> = Reg::new(GICD_TYPER);
const RCTLR: Reg<Gicr, u32> = Reg::new(GICR_CTLR);
const RWAKER: Reg<Gicr, u32> = Reg::new(GICR_WAKER);
const RPROPBASER: Reg<Gicr, u64> = Reg::new(GICR_PROPBASER);
const RPENDBASER: Reg<Gicr, u64> = Reg::new(GICR_PENDBASER);
const RIGROUPR0: Reg<Gicr, u32> = Reg::new(GICR_IGROUPR0);
const RISENABLE0: Reg<Gicr, u32> = Reg::new(GICR_ISENABLE0);
const RICENABLE0: Reg<Gicr, u32> = Reg::new(GICR_ICENABLE0);
const RICPENDR0: Reg<Gicr, u32> = Reg::new(GICR_ICPENDR0);
const RICACTIVE0: Reg<Gicr, u32> = Reg::new(GICR_ICACTIVE0);
const RIGRPMODR0: Reg<Gicr, u32> = Reg::new(GICR_IGRPMODR0);
const SCTLR: Reg<Gits, u32> = Reg::new(GITS_CTLR);
const STYPER: Reg<Gits, u64> = Reg::new(GITS_TYPER);
const SCBASER: Reg<Gits, u64> = Reg::new(GITS_CBASER);
const SCWRITER: Reg<Gits, u64> = Reg::new(GITS_CWRITER);
const SCREADR: Reg<Gits, u64> = Reg::new(GITS_CREADR);

/// `iq_ci == NULL`: a queue that never had a handler.
const NO_CPU: usize = usize::MAX;

/// `struct agintc_mbi_range`: a range of message based SPIs.
pub struct AgintcMbiRange {
    /// `mr_base`: the first SPI.
    base: usize,
    /// `mr_mbi`: one slot per SPI (`mr_span` of them), taken while an MSI holds it.
    mbi: MArray<AtomicBool>,
}

impl AgintcMbiRange {
    /// `mr_span`.
    fn span(&self) -> usize {
        self.mbi.len()
    }
}

/// `struct agintc_lpi_info`: an LPI the ITS translates an MSI to. Immutable, never freed (see
/// the Deviations): on its LPI's slot while the MSI is established.
pub struct AgintcLpiInfo {
    /// The slot's link (`sc_lpi[i]`).
    link: ListEntry<AgintcLpiInfo>,
    /// `li_msic`: the ITS.
    msic: &'static AgintcMsiSoftc,
    /// `li_ci`: the CPU the LPI runs on, by number.
    cpu: usize,
    /// `li_deviceid`.
    deviceid: u32,
    /// `li_eventid`.
    eventid: u32,
    /// `li_ih`: the LPI's handler.
    ih: &'static Intrhand,
}

hal::list_adapter!(
    /// An LPI's slot: its one record, or none.
    LpiList: AgintcLpiInfo, link
);

/// `struct intrhand`: one established handler. Immutable once linked but for its flags,
/// never freed (see the Deviations): the dispatch reads it with no lock.
pub struct Intrhand {
    /// `ih_list`: link on intrq list.
    link: ListEntry<Intrhand>,
    /// `ih_func`, `ih_arg`: the handler.
    h: IntrHandler,
    /// `ih_ipl`: `IPL_*`.
    ipl: i32,
    /// `ih_flags`: `IPL_MPSAFE`, `IPL_WAKEUP` (`agintc_intr_set_wakeup` sets it).
    flags: AtomicI32,
    /// `ih_type`: trigger type.
    type_: i32,
    /// `ih_irq`: IRQ number.
    irq: usize,
    /// `ih_count`.
    count: Evcount,
    /// `ih_name`.
    name: Option<&'static str>,
    /// `ih_ci`: CPU the IRQ runs on, by number.
    cpu: usize,
    /// Disestablished: unlinked, its counter detached, kept.
    dead: AtomicBool,
}

hal::list_adapter!(
    /// `TAILQ_HEAD(, intrhand) iq_list`.
    IhList: Intrhand, link
);

/// `struct intrq`: the handlers of one interrupt.
pub struct Intrq {
    /// `iq_list`: handler list.
    list: List<IhList>,
    /// `iq_ci`: CPU the IRQ runs on, [`NO_CPU`] before a handler.
    cpu: AtomicUsize,
    /// `iq_irq_max`: IRQ to mask while handling.
    irq_max: AtomicI32,
    /// `iq_irq_min`: lowest IRQ when shared.
    irq_min: AtomicI32,
}

impl Intrq {
    /// An empty queue (`M_ZERO` and `TAILQ_INIT`).
    const fn new() -> Self {
        Intrq {
            list: List::new(),
            cpu: AtomicUsize::new(NO_CPU),
            irq_max: AtomicI32::new(IPL_NONE),
            irq_min: AtomicI32::new(IPL_NONE),
        }
    }

    /// `iq_ci`, `None` for the C's `NULL`.
    fn cpu(&self) -> Option<usize> {
        match self.cpu.load(Ordering::Relaxed) {
            NO_CPU => None,
            n => Some(n),
        }
    }
}

/// `struct agintc_dmamem`: zeroed memory the GIC or the ITS reads, mapped uncached, with the
/// address it is told (`AGINTC_DMA_DVA`).
pub struct AgintcDmamem {
    /// The memory (`adm_kva`, `adm_seg`, `adm_size`).
    buf: DmaBuf,
    /// `AGINTC_DMA_DVA`: the segment's address as the device sees it.
    dva: u64,
}

impl AgintcDmamem {
    /// The table's byte `i` set to `v`, made globally visible (`cpu_dcache_wb_range` and
    /// `dsb sy`).
    fn set_u8(&mut self, i: usize, v: u8) {
        self.buf.write::<u8>(i, v);
        self.publish(i, 1);
    }

    /// The table's byte `i`.
    fn get_u8(&self, i: usize) -> u8 {
        self.buf.read::<u8>(i)
    }

    /// The table's double word `i` (8-byte entries).
    fn get_u64(&self, i: usize) -> u64 {
        self.buf.read::<u64>(i * 8)
    }

    /// The table's double word `i` set to `v`, made globally visible.
    fn set_u64(&mut self, i: usize, v: u64) {
        self.buf.write::<u64>(i * 8, v);
        self.publish(i * 8, 8);
    }

    /// Make globally visible: `cpu_dcache_wb_range(kva + off, len); dsb sy`.
    fn publish(&self, off: usize, len: usize) {
        self.buf.sync(off, len, DmaSync::PREWRITE);
        icc::dsb_sy();
    }
}

/// `struct agintc_softc`, built whole at attach.
pub struct AgintcSoftc {
    /// `sc_d_ioh`: the distributor.
    d: Mmio<Gicd>,
    /// `sc_rbase_ioh`: the redistributor regions, mapped for the attach's walk and kept.
    _rbase: MArray<Option<Mmio<GicrRegion>>>,
    /// `sc_r_ioh`: one redistributor's frames each, `sc_num_redist` of them.
    r: MArray<Mmio<Gicr>>,
    /// `sc_processor`: each redistributor's processor number (`GICR_TYPER` 23:8).
    processor: MArray<u16>,
    /// `sc_cpuremap`: each CPU's redistributor, `MAXCPUS` of them.
    cpuremap: MArray<usize>,
    /// Each CPU's `ci_mpidr`, as the attach read it, `MAXCPUS` of them.
    mpidr: MArray<u64>,
    /// `sc_handler`: one `intrq` per interrupt, `sc_nintr` of them.
    handler: MArray<Intrq>,
    /// `sc_lpi`: one slot per LPI, `sc_nlpi` of them.
    lpi: MArray<List<LpiList>>,
    /// `sc_prop`: the LPI configuration table (`None` without LPIs).
    prop: Option<Mutexed<AgintcDmamem>>,
    /// `sc_pend`: the LPI pending table, which only the redistributors read.
    _pend: Option<AgintcDmamem>,
    /// `sc_mbi_addr`: the doorbell of the message based SPIs.
    mbi_addr: u64,
    /// `sc_mbi_ranges`: `sc_mbi_nranges` of them.
    mbi: MArray<AgintcMbiRange>,
    /// `sc_prio_shift`: the shift of an interrupt's priority byte.
    prio_shift: u32,
    /// `sc_pmr_shift`: the shift of the priority mask.
    pmr_shift: u32,
    /// `sc_spur`: the spurious interrupt counter.
    spur: Evcount,
    /// `sc_ipi_num`: the SGI the IPIs use (set once at attach).
    ipi_num: AtomicU32,
    /// `sc_ipi_reason`: per CPU, the `1 << ARM_IPI_*` bits waiting for its IPI handler.
    ipi_reason: MArray<AtomicU32>,
    /// `sc_ipi_irq`: the IPI's handler.
    ipi_irq: OnceStatic<&'static Intrhand>,
    /// `cpu_info_primary`'s number: the CPU that ran the attach.
    primary: usize,
    /// The device's name, for the panics (`sc_sbus.sc_dev.dv_xname`).
    xname: [u8; 16],
    /// Serialises establish and disestablish of the SGIs, PPIs and SPIs (the C's autoconf
    /// and kernel lock, beside its `intr_disable()`); never held across a call out of the
    /// driver but `evcount_attach` and `evcount_detach`.
    lock: Mutexed<()>,
    /// Serialises an MSI's establish and disestablish, which may sleep (the C's kernel lock):
    /// the LPI slots, the ITS's device lists.
    msi_lock: Rwlocked<()>,
}

impl AgintcSoftc {
    /// `sc_nintr`.
    pub fn nintr(&self) -> usize {
        self.handler.len()
    }

    /// `sc_nlpi`.
    pub fn nlpi(&self) -> usize {
        self.lpi.len()
    }

    /// `sc_num_redist`.
    pub fn num_redist(&self) -> usize {
        self.r.len()
    }

    /// The device's name.
    fn xname(&self) -> Str<'_> {
        let n = self.xname.iter().position(|&c| c == 0).unwrap_or(16);
        Str(&self.xname[..n])
    }

    /// `sc->sc_handler[irq]`.
    fn queue(&self, irq: usize) -> &Intrq {
        match self.handler.get(irq) {
            Some(iq) => iq,
            None => kpanic(format_args!("agintc: irq {irq} out of range")),
        }
    }

    /// `sc->sc_cpuremap[cpu]`: the CPU's redistributor.
    fn hwcpu(&self, cpu: usize) -> usize {
        self.cpuremap.get(cpu).copied().unwrap_or(0)
    }

    /// `sc->sc_r_ioh[sc->sc_cpuremap[cpu]]`.
    fn redist(&self, cpu: usize) -> &Mmio<Gicr> {
        let hwcpu = self.hwcpu(cpu);
        match self.r.get(hwcpu) {
            Some(r) => r,
            None => kpanic(format_args!("agintc: no redistributor {hwcpu}")),
        }
    }

    /// `sc->sc_processor[sc->sc_cpuremap[cpu]]`.
    fn processor_of(&self, cpu: usize) -> u64 {
        u64::from(self.processor.get(self.hwcpu(cpu)).copied().unwrap_or(0))
    }

    /// `ci->ci_mpidr` of CPU `cpu`.
    fn mpidr_of(&self, cpu: usize) -> u64 {
        self.mpidr.get(cpu).copied().unwrap_or(0)
    }
}

/// The handler record behind a cookie the GIC answered.
fn hand(ih: IcCookie) -> &'static Intrhand {
    let r: Option<&'static Intrhand> = match ih {
        IcCookie::Rec(r) => {
            let r: &'static (dyn Any + Send + Sync) = r;
            r.downcast_ref::<Intrhand>()
        }
        IcCookie::Word(_) => None,
    };
    match r {
        Some(ih) => ih,
        None => kpanic(format_args!("agintc: not a handler of this controller")),
    }
}

/// The LPI record behind a cookie the ITS answered.
fn lpi_info(ih: IcCookie) -> &'static AgintcLpiInfo {
    let r: Option<&'static AgintcLpiInfo> = match ih {
        IcCookie::Rec(r) => {
            let r: &'static (dyn Any + Send + Sync) = r;
            r.downcast_ref::<AgintcLpiInfo>()
        }
        IcCookie::Word(_) => None,
    };
    match r {
        Some(li) => li,
        None => kpanic(format_args!("agintcmsi: not an MSI of this controller")),
    }
}

/// `struct gits_cmd`: one ITS command.
#[derive(Clone, Copy, Default)]
pub struct GitsCmd {
    /// `cmd`.
    pub cmd: u8,
    /// `deviceid`.
    pub deviceid: u32,
    /// `eventid`.
    pub eventid: u32,
    /// `intid`.
    pub intid: u32,
    /// `dw2`.
    pub dw2: u64,
    /// `dw3`.
    pub dw3: u64,
}

impl GitsCmd {
    /// The command's four double words, as the C's `struct gits_cmd` lays them out in memory
    /// (`memset` zeroed, then `memcpy`ed): `cmd` in byte 0, `deviceid` in bytes 4-7, `eventid`
    /// and `intid` in the second double word.
    pub const fn words(&self) -> [u64; 4] {
        [
            self.cmd as u64 | ((self.deviceid as u64) << 32),
            self.eventid as u64 | ((self.intid as u64) << 32),
            self.dw2,
            self.dw3,
        ]
    }
}

/// `struct agintc_msi_device`: a device ID the ITS has a translation table for. Never freed,
/// as in the C.
pub struct AgintcMsiDevice {
    /// `md_list`.
    link: ListEntry<AgintcMsiDevice>,
    /// `md_deviceid`.
    deviceid: u32,
    /// `md_events`: the event IDs taken.
    events: AtomicU32,
    /// `md_itt`: the device's interrupt translation table, which the ITS reads.
    _itt: AgintcDmamem,
}

hal::list_adapter!(
    /// `LIST_HEAD(, agintc_msi_device) sc_msi_devices`.
    MsiDevList: AgintcMsiDevice, link
);

/// The ITS's command queue (`sc_cmdq`) and its write index (`sc_cmdidx`), with the registers
/// that drive it.
pub struct ItsCmdq {
    /// `sc_cmdq`.
    q: AgintcDmamem,
    /// `sc_cmdidx`.
    idx: usize,
}

/// `struct agintc_msi_softc`: an ITS, built whole at attach and kept for ever (its
/// registration and its LPI records name it).
pub struct AgintcMsiSoftc {
    /// `sc_ioh`.
    m: Mmio<Gits>,
    /// `sc_dmat`'s bus: the DMA addresses of the tables.
    bus: FdtBusRef,
    /// `sc_msi_addr`.
    msi_addr: u64,
    /// `sc_msi_delta`.
    msi_delta: u64,
    /// `sc_cmdq` and `sc_cmdidx`.
    cmdq: Mutexed<ItsCmdq>,
    /// `sc_deviceid_max`.
    deviceid_max: u32,
    /// `sc_dtt`: the device table (`None` when the ITS has none).
    dtt: Option<Mutexed<AgintcDmamem>>,
    /// `sc_dtt_pgsz`.
    dtt_pgsz: usize,
    /// `sc_dte_sz`.
    dte_sz: usize,
    /// `sc_dtt_indirect`.
    dtt_indirect: bool,
    /// `sc_ctt`: the collection table, which only the ITS reads.
    _ctt: Option<AgintcDmamem>,
    /// `sc_ite_sz`.
    ite_sz: usize,
    /// `sc_msi_devices`.
    devices: List<MsiDevList>,
    /// `agintc_sc`: the GIC whose LPIs the ITS raises.
    gic: &'static AgintcSoftc,
    /// The device's name (`sc_dev.dv_xname`).
    xname: [u8; 16],
    /// The softc's own handle, set once it is leaked: what an LPI record names (`li_msic`).
    me: OnceStatic<&'static AgintcMsiSoftc>,
}

impl AgintcMsiSoftc {
    /// The device's name.
    fn xname(&self) -> Str<'_> {
        let n = self.xname.iter().position(|&c| c == 0).unwrap_or(16);
        Str(&self.xname[..n])
    }
}

/// `agintc_sc`: the attached controller.
pub static AGINTC: OnceStatic<MArc<AgintcSoftc>> = OnceStatic::new();

/// `agintc_ca` with `agintc_cd`'s constants (`cfdata[]`'s `agintc* at fdt? early 1` row
/// names it, `Cfdata::native`).
pub struct AgintcDriver;

/// The driver of `agintc*`.
pub static AGINTC_CA: AgintcDriver = AgintcDriver;

/// `agintc_cd`'s `cd_devs`.
static AGINTC_UNITS: UnitTable = UnitTable::new();

/// `agintcmsi_ca` with `agintcmsi_cd`'s constants.
pub struct AgintcMsiDriver;

/// The driver of `agintcmsi*`.
pub static AGINTCMSI_CA: AgintcMsiDriver = AgintcMsiDriver;

/// `agintcmsi_cd`'s `cd_devs`.
static AGINTCMSI_UNITS: UnitTable = UnitTable::new();

/// `agintc_compatibles[]`.
static AGINTC_COMPATIBLES: [&[u8]; 2] = [b"arm,gic-v3", b"arm,gic-v4"];

/// The device softc both drivers make: the state is [`AGINTC`]'s and the ITS's own, built at
/// attach.
struct GicDev;

/// The softc of a device of either driver.
fn make_softc() -> AnyArc {
    match MArc::new(M_DEVBUF, M_WAITOK, GicDev) {
        Some(sc) => MArc::erase(sc),
        None => kpanic(format_args!(
            "config_make_softc: allocation for device softc failed"
        )),
    }
}

impl DriverOps for AgintcDriver {
    fn name(&self) -> &'static [u8] {
        b"agintc"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &AGINTC_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        make_softc()
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        FdtArgs::of_aux(aux).map_or(0, agintc_match)
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let Some(fa) = FdtArgs::of_aux(aux) else {
            kpanic(format_args!("{}: not a device-tree node", dev.xname()));
        };
        agintc_attach(dev, fa);
    }
    fn activate(&self, dev: &DevRef, act: i32) -> Option<Result<(), Errno>> {
        Some(agintc_activate(dev, act))
    }
}

impl DriverOps for AgintcMsiDriver {
    fn name(&self) -> &'static [u8] {
        b"agintcmsi"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &AGINTCMSI_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        make_softc()
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        FdtArgs::of_aux(aux).map_or(0, agintc_msi_match)
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let Some(fa) = FdtArgs::of_aux(aux) else {
            kpanic(format_args!("{}: not a device-tree node", dev.xname()));
        };
        agintc_msi_attach(dev, fa);
    }
}

/// `ffs(3)`: the 1-based index of the lowest set bit, 0 for none.
const fn ffs(x: u64) -> u32 {
    if x == 0 { 0 } else { x.trailing_zeros() + 1 }
}

/// `fls(3)`: the 1-based index of the highest set bit, 0 for none.
const fn fls(x: u32) -> u32 {
    32 - x.leading_zeros()
}

/// The priority byte an interrupt or the mask gets for `ipl`: `((0xff - ipl) << shift) &
/// 0xff`.
const fn agintc_prival(ipl: i32, shift: u32) -> u8 {
    (((0xff - ipl) << shift) & 0xff) as u8
}

/// `GICR_PROPBASER`'s value: the table, inner shareable, non-cacheable, and the ID bits for
/// `nlpi` LPIs above `LPI_BASE`.
const fn agintc_propbaser(pa: u64, nlpi: usize) -> u64 {
    pa | GICR_PROPBASER_ISH
        | GICR_PROPBASER_IC_NORM_NC
        | (fls((LPI_BASE + nlpi - 1) as u32) - 1) as u64
}

/// `ICC_SGI1R`'s value that raises SGI `sgi` at the CPU whose affinity is `mpidr` alone
/// (`agintc_send_ipi`'s `sendmask`).
const fn agintc_sgi1r(mpidr: u64, sgi: u32) -> u64 {
    const MPIDR_AFF3: u64 = 0xff << 32;
    const MPIDR_AFF2: u64 = 0xff << 16;
    const MPIDR_AFF1: u64 = 0xff << 8;
    let mut sendmask = (mpidr & MPIDR_AFF3) << 16;
    sendmask |= (mpidr & MPIDR_AFF2) << 16;
    sendmask |= (mpidr & MPIDR_AFF1) << 8;
    sendmask |= 1 << (mpidr & 0x0f);
    sendmask |= (sgi as u64) << 24;
    sendmask
}

/// The redistributor frames' size for a `GICR_TYPER`: two 64 KB frames, two more with virtual
/// LPIs, unless the device tree gives a stride.
const fn agintc_redist_size(typer: u64, redist_stride: u64) -> usize {
    if redist_stride == 0 {
        let mut sz = 64 * 1024 * 2;
        if typer & GICR_TYPER_VLPIS != 0 {
            sz += 64 * 1024 * 2;
        }
        sz
    } else {
        redist_stride as usize
    }
}

/// The affinity `GICR_TYPER` reports for the CPU whose `MPIDR_EL1` is `mpidr`: Aff3 in the
/// top byte, then Aff2..Aff0.
const fn agintc_mpidr_affinity(mpidr: u64) -> u32 {
    (((mpidr >> 8) & 0xff00_0000) | (mpidr & 0x00ff_ffff)) as u32
}

/// `agintc_match`: whether the node is a GICv3 or GICv4.
pub fn agintc_match(fa: &FdtArgs) -> i32 {
    i32::from(
        AGINTC_COMPATIBLES
            .iter()
            .any(|c| OF_is_compatible(fa.node, c)),
    )
}

/// `bus_space_map(faa->fa_iot, faa->fa_reg[i].addr, faa->fa_reg[i].size, 0, &ioh)`, the
/// C's panic when it fails.
fn map_reg<R: RegBlock>(fa: &FdtArgs, i: usize, what: &str) -> Mmio<R> {
    match fa.reg().get(i) {
        Some(r) => match fa.bus.map::<R>(r.addr, r.size, DevCache::Device) {
            Ok(m) => m,
            Err(_) => kpanic(format_args!("agintc_attach: {what} bus_space_map failed")),
        },
        None => kpanic(format_args!("agintc_attach: {what} bus_space_map failed")),
    }
}

/// The walk of the redistributor regions: where the next redistributor's frames are, by
/// `GICR_TYPER`'s `Last` bit and the region's size (both of the attach's loops).
struct RedistWalk {
    idx: usize,
    offset: usize,
}

impl RedistWalk {
    /// The next redistributor: its region, its offset in it, its frames' size and its
    /// `GICR_TYPER`; `None` past the last region.
    fn next(
        &mut self,
        rbase: &[Option<Mmio<GicrRegion>>],
        sizes: &[u64],
        stride: u64,
    ) -> Option<(usize, usize, usize, u64)> {
        let region = rbase.get(self.idx)?.as_ref()?;
        let size = sizes.get(self.idx).copied().unwrap_or(0) as usize;
        let at = (self.idx, self.offset);
        let typer = region.read_at::<u64>(self.offset + GICR_TYPER);
        let sz = agintc_redist_size(typer, stride);

        // DEBUG_AGINTC: not configured.

        self.offset += sz;
        if self.offset >= size || typer & GICR_TYPER_LAST != 0 {
            self.offset = 0;
            self.idx += 1;
        }
        Some((at.0, at.1, sz, typer))
    }
}

/// `agintc_attach`: maps the distributor and the redistributors, sets up the LPI tables, the
/// priority shifts, the distributor and this CPU's redistributor, takes over `spl` and the
/// IRQ dispatch, enables group 1, sets up the IPI, registers with the device tree and
/// attaches the ITS.
pub fn agintc_attach(dev: &DevRef, fa: &FdtArgs) {
    let psw = hal::cpu::intr_disable();
    hal::cpu::intr::init_smask();

    let nregions =
        usize::try_from(OF_getpropint(fa.node, b"#redistributor-regions", 1)).unwrap_or(0);

    if fa.reg().len() < nregions + 1 {
        kpanic(format_args!("agintc_attach: missing registers"));
    }

    let d = map_reg::<Gicd>(fa, 0, "GICD");

    let Some(rbase) = MArray::new(M_DEVBUF, M_WAITOK, nregions, |idx| {
        Some(map_reg::<GicrRegion>(fa, 1 + idx, "GICR"))
    }) else {
        kpanic(format_args!("agintc_attach: no malloc(9)"));
    };
    let Some(sizes) = MArray::new(M_TEMP, M_WAITOK, nregions, |idx| {
        fa.reg().get(1 + idx).map_or(0, |r| r.size)
    }) else {
        kpanic(format_args!("agintc_attach: no malloc(9)"));
    };

    let typer = d.read(DTYPER);

    let mut prop = None;
    let mut pend = None;
    let mut nlpi = 0;
    if typer & GICD_TYPER_LPIS != 0 {
        // Allocate redistributor tables
        let Some(p) = agintc_dmamem_alloc(&fa.bus, GICR_PROP_SIZE, GICR_PROP_SIZE) else {
            printf(format_args!(": can't alloc LPI config table\n"));
            agintc_attach_unmap(d, rbase, psw);
            return;
        };
        prop = Some(p);
        let Some(p) = agintc_dmamem_alloc(&fa.bus, GICR_PEND_SIZE, GICR_PEND_SIZE) else {
            printf(format_args!(": can't alloc LPI pending table\n"));
            agintc_attach_unmap(d, rbase, psw);
            return;
        };
        pend = Some(p);

        // Minimum number of LPIs supported by any implementation.
        nlpi = 8192;
    }

    let reg0 = fa.reg().first().map_or(0, |r| r.addr);
    let (mbi, mbi_addr) = if typer & GICD_TYPER_MBIS != 0 {
        agintc_mbiinit(fa.node, reg0)
    } else {
        (no_ranges(), 0)
    };

    // We are guaranteed to have at least 16 priority levels, so in principle we just want to
    // use the top 4 bits of the (non-secure) priority field.
    let mut prio_shift = 4;
    let mut pmr_shift = 4;

    // If the system supports two security states and SCR_EL3.FIQ is zero, the non-secure
    // shifted view applies. We detect this by checking whether the number of writable bits
    // matches the number of implemented priority bits. If that is the case we will need to
    // adjust the priorities that we write into ICC_PMR_EL1 accordingly.
    //
    // On Ampere eMAG it appears as if there are five writable bits when we write 0xff. But
    // for higher priorities (smaller values) only the top 4 bits stick. So we use 0xbf
    // instead to determine the number of writable bits.
    let ctrl = d.read(DCTLR);
    if ctrl & GICD_CTLR_DS == 0 && agintc_nonsecure_view() {
        pmr_shift -= 1;
    }

    // The Rockchip RK3399 is busted. Its GIC-500 treats all access to its memory mapped
    // registers as "secure". As a result, several registers don't behave as expected. For
    // example, the GICD_IPRIORITYRn and GICR_IPRIORITYRn registers expose the full priority
    // range available to secure interrupts. We need to be aware of this and write an
    // adjusted priority value into these registers. We also need to be careful not to touch
    // any bits that shouldn't be writable in non-secure mode.
    //
    // We check whether we have secure mode access to these registers by attempting to write
    // to the GICD_NSACR register and check whether its contents actually change. In that
    // case we need to adjust the priorities we write into GICD_IPRIORITYRn and
    // GICRIPRIORITYRn accordingly.
    let mut rk3399_quirk = false;
    let oldnsacr = d.read_at::<u32>(gicd_nsacr(32));
    d.write_at::<u32>(gicd_nsacr(32), oldnsacr ^ 0xffff_ffff);
    let nsacr = d.read_at::<u32>(gicd_nsacr(32));
    if nsacr != oldnsacr {
        d.write_at::<u32>(gicd_nsacr(32), oldnsacr);
        rk3399_quirk = true;
        prio_shift -= 1;
        printf(format_args!(" sec"));
    }

    printf(format_args!(" shift {prio_shift}:{pmr_shift}"));

    icc::set_sre(ICC_SRE_EL1_EN);

    let mut nintr = 32 * (typer & GICD_TYPER_ITLINE_M) as usize;
    nintr += 32; // ICD_ICTR + 1, irq 0-31 is SGI, 32+ is PPI

    // find the redistributors.
    let stride = OF_getpropint64(fa.node, b"redistributor-stride", 0);
    let mut walk = RedistWalk { idx: 0, offset: 0 };
    let mut nredist = 0;
    while walk.next(&rbase, &sizes, stride).is_some() {
        nredist += 1;
    }

    printf(format_args!(" nirq {nintr} nredist {nredist}"));

    // submap and configure the redistributors.
    let Some(mut cpuremap) = MArray::new(M_DEVBUF, M_WAITOK, hal::cpu::MAXCPUS, |_| 0usize) else {
        kpanic(format_args!("agintc_attach: no malloc(9)"));
    };
    let Some(mut mpidr) = MArray::new(M_DEVBUF, M_WAITOK, hal::cpu::MAXCPUS, |_| 0u64) else {
        kpanic(format_args!("agintc_attach: no malloc(9)"));
    };
    for ci in hal::cpu::cores() {
        if let Some(m) = mpidr.get_mut(ci.number()) {
            *m = ci.hwid();
        }
    }
    let Some(mut processor) = MArray::new(M_DEVBUF, M_WAITOK, nredist, |_| 0u16) else {
        kpanic(format_args!("agintc_attach: no malloc(9)"));
    };
    let mut walk = RedistWalk { idx: 0, offset: 0 };
    let Some(r) = MArray::new(M_DEVBUF, M_WAITOK, nredist, |n| {
        let Some((idx, offset, sz, typer)) = walk.next(&rbase, &sizes, stride) else {
            kpanic(format_args!("agintc_attach: the redistributors changed"));
        };
        let affinity = (typer >> 32) as u32;
        for ci in hal::cpu::cores() {
            if affinity == agintc_mpidr_affinity(ci.hwid()) {
                if let Some(slot) = cpuremap.get_mut(ci.number()) {
                    *slot = n;
                }
                break;
            }
        }
        if let Some(p) = processor.get_mut(n) {
            *p = (typer >> 8) as u16;
        }

        let base = fa.reg().get(1 + idx).map_or(0, |r| r.addr);
        let Ok(frame) = fa
            .bus
            .map::<Gicr>(base + offset as u64, sz as u64, DevCache::Device)
        else {
            kpanic(format_args!("agintc_attach: GICR bus_space_map failed"));
        };

        if let (Some(prop), Some(pend)) = (&prop, &pend) {
            frame.write(RPROPBASER, agintc_propbaser(prop.dva, nlpi));
            frame.write(
                RPENDBASER,
                pend.dva | GICR_PENDBASER_ISH | GICR_PENDBASER_IC_NORM_NC | GICR_PENDBASER_PTZ,
            );
            frame.write(RCTLR, GICR_CTLR_ENABLE_LPIS);
        }
        frame
    }) else {
        kpanic(format_args!("agintc_attach: no malloc(9)"));
    };
    drop(sizes);

    // Disable all interrupts, clear all pending
    for i in 1..nintr / 32 {
        d.write_at::<u32>(gicd_icactiver(i * 32), !0);
        d.write_at::<u32>(gicd_icenabler(i * 32), !0);
    }

    for i in (4..nintr).step_by(4) {
        // lowest priority ??
        d.write_at::<u32>(gicd_ipriorityr(i), 0xffff_ffff);
    }

    // Set all interrupts to G1NS
    for i in 1..nintr / 32 {
        d.write_at::<u32>(gicd_igroupr(i * 32), !0);
        d.write_at::<u32>(gicd_igrpmodr(i * 32), 0);
    }

    for i in 2..nintr / 16 {
        // irq 32 - N
        d.write_at::<u32>(gicd_icfgr(i * 16), 0);
    }

    let Some(handler) = MArray::new(M_DEVBUF, M_WAITOK, nintr, |_| Intrq::new()) else {
        kpanic(format_args!(
            "agintc_attach: no memory for the handler queues"
        ));
    };
    let Some(lpi) = MArray::new(M_DEVBUF, M_WAITOK, nlpi, |_| List::new()) else {
        kpanic(format_args!("agintc_attach: no memory for the LPI slots"));
    };
    let Some(ipi_reason) =
        MArray::new(M_DEVBUF, M_WAITOK, hal::cpu::MAXCPUS, |_| AtomicU32::new(0))
    else {
        kpanic(format_args!("agintc_attach: no malloc(9)"));
    };

    let mut xname = [0u8; 16];
    let name = dev.xname().as_bytes();
    let n = name.len().min(xname.len() - 1);
    xname[..n].copy_from_slice(&name[..n]);

    let sc = AgintcSoftc {
        d,
        _rbase: rbase,
        r,
        processor,
        cpuremap,
        mpidr,
        handler,
        lpi,
        prop: prop.map(|p| Mutexed::new(IPL_NONE, p)),
        _pend: pend,
        mbi_addr,
        mbi,
        prio_shift,
        pmr_shift,
        spur: Evcount::new(),
        ipi_num: AtomicU32::new(0),
        ipi_reason,
        ipi_irq: OnceStatic::new(),
        primary: hal::cpu::cpu_number(),
        xname,
        lock: Mutexed::new(IPL_HIGH, ()),
        msi_lock: Rwlocked::new("agintcmsi", ()),
    };
    let Some(sc) = MArc::new(M_DEVBUF, M_WAITOK, sc) else {
        kpanic(format_args!("agintc_attach: no malloc(9)"));
    };
    if AGINTC.set(sc).is_err() {
        kpanic(format_args!("agintc_attach: a second GIC"));
    }
    let Some(arc) = AGINTC.get() else {
        kpanic(format_args!("agintc_attach: the softc went"));
    };
    let sc: &'static AgintcSoftc = arc;

    evcount_attach(&sc.spur, "irq1023/spur", None);

    agintc_cpuinit(sc);

    // set priority to IPL_HIGH until configure lowers to desired IPL
    sc.setipl(IPL_HIGH);

    // initialize all interrupts as disabled
    agintc_calc_mask(sc);

    // insert self as interrupt handler
    if hal::cpu::intr::install(sc).is_err() {
        kpanic(format_args!(
            "agintc_attach: another interrupt controller attached"
        ));
    }

    // enable interrupts
    let ctrl = sc.d.read(DCTLR);
    let mut bits = GICD_CTLR_ARE_NS | GICD_CTLR_ENABLE_GRP1A | GICD_CTLR_ENABLE_GRP1;
    if rk3399_quirk {
        bits &= !GICD_CTLR_ENABLE_GRP1A;
        bits <<= 1;
    }
    sc.d.write(DCTLR, ctrl | bits);

    icc::set_pmr(0xff);
    icc::set_bpr1(0);
    icc::set_igrpen1(1);

    // setup IPI interrupts
    let Some(ipiirq) = agintc_find_ipi(sc, hal::cpu::cpu_number()) else {
        kpanic(format_args!("no irq available for IPI"));
    };

    printf(format_args!(" ipi {ipiirq}"));

    if let Some(ih) = agintc_intr_establish(
        sc,
        ipiirq,
        IST_EDGE_RISING,
        IPL_IPI | IPL_MPSAFE,
        None,
        IntrHandler::Shared(agintc_ipi_handler, MArc::erase(arc.clone())),
        Some("ipi"),
    ) {
        let _ = sc.ipi_irq.set(ih);
    }
    sc.ipi_num.store(ipiirq as u32, Ordering::Relaxed);

    intr_set_send_ipi(agintc_send_ipi);

    arm_intr_register_fdt(IcDesc {
        node: fa.node,
        gic_its_id: 0,
        ops: sc,
    });

    drop(psw);

    // Attach ITS.
    let _ = simplebus_attach(dev, fa);
}

/// No MBI range (`sc_mbi_nranges` 0).
fn no_ranges() -> MArray<AgintcMbiRange> {
    match MArray::new(M_DEVBUF, M_WAITOK, 0, |_| {
        kpanic(format_args!("agintc: an MBI range of none"))
    }) {
        Some(m) => m,
        None => kpanic(format_args!("agintc: no malloc(9)")),
    }
}

/// Whether the priority mask has as many writable bits as the CPU interface implements
/// priority bits (`0xbf` written: the non-secure shifted view of two security states), the
/// mask put back as found.
fn agintc_nonsecure_view() -> bool {
    let nbits = icc_ctlr_el1_pribits(icc::ctlr()) + 1;
    let oldpmr = icc::pmr();
    icc::set_pmr(0xbf);
    let pmr = icc::pmr();
    icc::set_pmr(oldpmr);
    // The C's `int` arithmetic (`ffs(0)` is 0, so a mask of no bit compares with 9).
    nbits as i64 == 8 - (i64::from(ffs(pmr)) - 1)
}

/// The SGI the IPIs use, on CPU `cpu`'s redistributor: the first whose priority byte holds a
/// written value (if this interrupt is not usable, pri will be unmodified), restored as found
/// (it is set when used).
fn agintc_find_ipi(sc: &AgintcSoftc, cpu: usize) -> Option<usize> {
    let r = sc.redist(cpu);
    for i in 0..16 {
        let oldreg = r.read_at::<u8>(gicr_ipriorityr(i));
        r.write_at::<u8>(gicr_ipriorityr(i), oldreg ^ 0x20);

        // if this interrupt is not usable, pri will be unmodified
        let reg = r.read_at::<u8>(gicr_ipriorityr(i));
        if reg == oldreg {
            continue;
        }

        // return to original value, will be set when used
        r.write_at::<u8>(gicr_ipriorityr(i), oldreg);
        return Some(i);
    }
    None
}

/// `agintc_attach`'s `unmap:`: what was mapped and allocated given back, the interrupt state
/// restored (see the Deviations).
fn agintc_attach_unmap(
    d: Mmio<Gicd>,
    mut rbase: MArray<Option<Mmio<GicrRegion>>>,
    psw: hal::cpu::IntrGuard,
) {
    for region in rbase.iter_mut().filter_map(Option::take) {
        crate::vm::km::unmap_device(region);
    }
    crate::vm::km::unmap_device(d);
    drop(psw);
}

/// `agintc_activate`: `DVACT_RESUME` writes the controller's state again, then the children
/// are told.
pub fn agintc_activate(dev: &DevRef, act: i32) -> Result<(), Errno> {
    if act == DVACT_RESUME
        && let Some(sc) = AGINTC.get()
    {
        agintc_restore(sc);
    }
    config_activate_children(dev, act)
}

/// `agintc_restore`: every interrupt with a handler configured, prioritised, routed and
/// enabled again; every LPI enabled again in the configuration table and in the ITS's cache.
pub fn agintc_restore(sc: &AgintcSoftc) {
    for irq in 0..sc.nintr() {
        let iq = sc.queue(irq);
        let Some(ih) = iq.list.first() else {
            continue;
        };
        agintc_intr_config(sc, irq, ih.type_);
        agintc_set_priority(sc, irq, iq.irq_min.load(Ordering::Relaxed));
        agintc_route(sc, irq, IRQ_ENABLE, Some(ih.cpu));
        agintc_intr_enable(sc, irq);
    }

    let Some(prop) = &sc.prop else {
        return;
    };
    for (irq, slot) in sc.lpi.iter().enumerate() {
        let Some(li) = slot.first() else {
            continue;
        };
        {
            let mut p = prop.lock();
            let v = p.get_u8(irq) | GICR_PROP_ENABLE;
            // Make globally visible.
            p.set_u8(irq, v);
        }
        // Invalidate cache
        agintc_msi_inv(li);
    }
}

/// `agintc_mbiinit`: the message based SPIs of `mbi-ranges` (a controller that is no
/// `msi-controller` has none) and their doorbell (`mbi-alias`, else the distributor's
/// `GICD_SETSPI_NSR`).
fn agintc_mbiinit(node: i32, addr: u64) -> (MArray<AgintcMbiRange>, u64) {
    if OF_getproplen(node, b"msi-controller") != 0 {
        return (no_ranges(), 0);
    }

    let len = OF_getproplen(node, b"mbi-ranges");
    if len <= 0 || (len % 2) * 4 != 0 {
        return (no_ranges(), 0);
    }
    let len = len as usize;

    let Some(mut ranges) = MArray::new(M_TEMP, M_WAITOK, len / 4, |_| 0u32) else {
        kpanic(format_args!("agintc_mbiinit: no malloc(9)"));
    };
    OF_getpropintarray(node, b"mbi-ranges", &mut ranges);

    let nranges = len / (2 * 4);
    let Some(mbi) = MArray::new(M_DEVBUF, M_WAITOK, nranges, |i| {
        let base = ranges.get(2 * i).copied().unwrap_or(0) as usize;
        let span = ranges.get(2 * i + 1).copied().unwrap_or(0) as usize;
        let Some(slots) = MArray::new(M_DEVBUF, M_WAITOK, span, |_| AtomicBool::new(false)) else {
            kpanic(format_args!("agintc_mbiinit: no malloc(9)"));
        };
        AgintcMbiRange { base, mbi: slots }
    }) else {
        kpanic(format_args!("agintc_mbiinit: no malloc(9)"));
    };

    drop(ranges);

    let addr = OF_getpropint64(node, b"mbi-alias", addr);

    printf(format_args!(" mbi"));
    (mbi, addr + GICD_SETSPI_NSR)
}

/// `agintc_cpuinit`: initialize the redistributor of the calling CPU.
fn agintc_cpuinit(sc: &AgintcSoftc) {
    let r = sc.redist(hal::cpu::cpu_number());
    let mut timeout = 100_000;

    let mut waker = r.read(RWAKER);
    waker &= !GICR_WAKER_PROCESSORSLEEP;
    r.write(RWAKER, waker);

    loop {
        waker = r.read(RWAKER);
        timeout -= 1;
        if timeout == 0 || waker & GICR_WAKER_CHILDRENASLEEP == 0 {
            break;
        }
    }
    if timeout == 0 {
        printf(format_args!("agintc_cpuinit: waker timed out\n"));
    }

    r.write(RICENABLE0, !0);
    r.write(RICPENDR0, !0);
    r.write(RICACTIVE0, !0);
    for i in (0..32).step_by(4) {
        r.write_at::<u32>(gicr_ipriorityr(i), !0);
    }
    r.write(RIGROUPR0, !0);
    r.write(RIGRPMODR0, 0);

    if let Some(ih) = sc.ipi_irq.get() {
        agintc_route_irq(sc, ih, IRQ_ENABLE, hal::cpu::cpu_number());
    }

    icc::set_pmr(0xff);
    icc::set_bpr1(0);
    icc::set_igrpen1(1);
    hal::cpu::intr_enable();
}

/// `agintc_set_priority`: SPIs at the distributor, the others at the calling CPU's
/// redistributor (only sets local redistributor).
fn agintc_set_priority(sc: &AgintcSoftc, irq: usize, ipl: i32) {
    let prival = agintc_prival(ipl, sc.prio_shift);

    if irq >= SPI_BASE {
        sc.d.write_at::<u8>(gicd_ipriorityr(irq), prival);
    } else {
        // only sets local redistributor
        sc.redist(hal::cpu::cpu_number())
            .write_at::<u8>(gicr_ipriorityr(irq), prival);
    }
}

/// `agintc_enable_wakeup`: every interrupt and LPI that no handler established with
/// `IPL_WAKEUP` is disabled before the machine sleeps.
fn agintc_enable_wakeup(sc: &AgintcSoftc) {
    for irq in 0..sc.nintr() {
        let iq = sc.queue(irq);
        // No handler? Disabled already.
        if iq.list.is_empty() {
            continue;
        }
        // Unless we're WAKEUP, disable.
        let wakeup = iq
            .list
            .iter()
            .any(|ih| ih.flags.load(Ordering::Relaxed) & IPL_WAKEUP != 0);
        if !wakeup {
            agintc_intr_disable(sc, irq);
        }
    }

    let Some(prop) = &sc.prop else {
        return;
    };
    for (irq, slot) in sc.lpi.iter().enumerate() {
        let Some(li) = slot.first() else {
            continue;
        };
        if li.ih.flags.load(Ordering::Relaxed) & IPL_WAKEUP != 0 {
            continue;
        }
        {
            let mut p = prop.lock();
            let v = p.get_u8(irq) & !GICR_PROP_ENABLE;
            // Make globally visible.
            p.set_u8(irq, v);
        }
        // Invalidate cache
        agintc_msi_inv(li);
    }
}

/// `agintc_intr_enable`: SPIs at the distributor, the others at the calling CPU's
/// redistributor.
fn agintc_intr_enable(sc: &AgintcSoftc, irq: usize) {
    let bit = 1u32 << irq_to_reg32bit(irq);

    if irq >= 32 {
        sc.d.write_at::<u32>(gicd_isenabler(irq), bit);
    } else {
        sc.redist(hal::cpu::cpu_number()).write(RISENABLE0, bit);
    }
}

/// `agintc_intr_disable`.
fn agintc_intr_disable(sc: &AgintcSoftc, irq: usize) {
    let bit = 1u32 << irq_to_reg32bit(irq);

    if irq >= 32 {
        sc.d.write_at::<u32>(gicd_icenabler(irq), bit);
    } else {
        sc.redist(hal::cpu::cpu_number()).write(RICENABLE0, bit);
    }
}

/// `agintc_intr_config`: level or rising-edge trigger. Don't dare to change SGIs or PPIs
/// (yet).
fn agintc_intr_config(sc: &AgintcSoftc, irq: usize, type_: i32) {
    if irq < 32 {
        return;
    }

    let mut reg = sc.d.read_at::<u32>(gicd_icfgr(irq));
    reg &= !gicd_icfgr_trig_mask(irq);
    if type_ == IST_EDGE_RISING {
        reg |= gicd_icfgr_trig_edge(irq);
    } else {
        reg |= gicd_icfgr_trig_level(irq);
    }
    sc.d.write_at::<u32>(gicd_icfgr(irq), reg);
}

/// `agintc_calc_mask`: recomputes every interrupt's priority and enable.
fn agintc_calc_mask(sc: &AgintcSoftc) {
    for irq in 0..sc.nintr() {
        agintc_calc_irq(sc, irq);
    }
}

/// `agintc_calc_irq`: one interrupt's priority, route and enable from its handlers.
fn agintc_calc_irq(sc: &AgintcSoftc, irq: usize) {
    let iq = sc.queue(irq);
    let cpu = iq.cpu();
    let mut max = IPL_NONE;
    let mut min = IPL_HIGH;

    for ih in &iq.list {
        max = max.max(ih.ipl);
        min = min.min(ih.ipl);
    }

    if max == IPL_NONE {
        min = IPL_NONE;
    }

    if iq.irq_max.load(Ordering::Relaxed) == max && iq.irq_min.load(Ordering::Relaxed) == min {
        return;
    }

    iq.irq_max.store(max, Ordering::Relaxed);
    iq.irq_min.store(min, Ordering::Relaxed);

    // DEBUG_AGINTC: not configured.
    // Enable interrupts at lower levels, clear -> enable
    // Set interrupt priority/enable
    if min == IPL_NONE {
        agintc_intr_disable(sc, irq);
        agintc_route(sc, irq, IRQ_DISABLE, cpu);
    } else {
        agintc_set_priority(sc, irq, min);
        agintc_route(sc, irq, IRQ_ENABLE, cpu);
        agintc_intr_enable(sc, irq);
    }
}

/// `agintc_route_irq`: the `ic_route` hook (an enable only: the C does nothing for a
/// disable).
fn agintc_route_irq(sc: &AgintcSoftc, ih: &Intrhand, enable: bool, cpu: usize) {
    if enable {
        agintc_set_priority(sc, ih.irq, sc.queue(ih.irq).irq_min.load(Ordering::Relaxed));
        agintc_route(sc, ih.irq, IRQ_ENABLE, Some(cpu));
        agintc_intr_enable(sc, ih.irq);
    }
}

/// `agintc_route`: an SPI routed to CPU `cpu`'s affinity (`XXX does not yet support
/// 'participating node'`); SGIs and PPIs are each CPU's own.
fn agintc_route(sc: &AgintcSoftc, irq: usize, _enable: bool, cpu: Option<usize>) {
    if irq >= 32
        && let Some(cpu) = cpu
    {
        // DEBUG_AGINTC: not configured.
        sc.d.write_at::<u64>(gicd_irouter(irq), sc.mpidr_of(cpu) & MPIDR_AFF);
    }
}

/// `agintc_run_handler`: one handler, with its argument or the frame, under the kernel lock
/// unless it is `IPL_MPSAFE` or the interrupted level is at least `IPL_SCHED`.
#[inline]
fn agintc_run_handler(ih: &Intrhand, frame: &ClockFrame, s: i32) {
    let need_lock = ih.flags.load(Ordering::Relaxed) & IPL_MPSAFE == 0 && s < IPL_SCHED;
    if need_lock {
        hal::sync::klock::kernel_lock();
    }

    if ih.h.call(frame) != 0 {
        ih.count.ec_count.fetch_add(1, Ordering::Relaxed);
    }

    if need_lock {
        hal::sync::klock::kernel_unlock();
    }
}

/// `agintc_irq_handler`: the IRQ dispatcher `arm_cpu_irq` calls: acknowledge, raise to the
/// interrupt's level, run its handlers with the machine's interrupts on, end it, restore. An
/// LPI runs the handler its slot names.
pub fn agintc_irq_handler(sc: &AgintcSoftc, frame: &ClockFrame, irq_: &InIrq) {
    let irq = icc::iar1() as usize;

    // DEBUG_AGINTC: not configured.

    if irq == 1023 {
        sc.spur.ec_count.fetch_add(1, Ordering::Relaxed);
        return;
    }

    if (irq >= sc.nintr() && irq < LPI_BASE) || irq >= LPI_BASE + sc.nlpi() {
        return;
    }

    if irq >= LPI_BASE {
        let Some(li) = sc.lpi.get(irq - LPI_BASE).and_then(List::first) else {
            return;
        };
        let ih = li.ih;

        let s = sc.splraise(ih.ipl);
        hal::cpu::intr::unmasked(irq_, || agintc_run_handler(ih, frame, s));
        icc::eoir1(irq as u64);

        sc.splx(s);
        return;
    }

    let iq = sc.queue(irq);
    let pri = iq.irq_max.load(Ordering::Relaxed);
    let s = sc.splraise(pri);
    hal::cpu::intr::unmasked(irq_, || {
        for ih in &iq.list {
            agintc_run_handler(ih, frame, s);
        }
    });
    icc::eoir1(irq as u64);

    sc.splx(s);
}

/// `agintc_intr_establish_fdt`'s reading of the specifier: 1st cell contains type: 0 SPI
/// (32-X), 1 PPI (16-31); 2nd cell contains the interrupt number; 3rd the trigger. The
/// interrupt and its type (`IST_*`).
fn agintc_fdt_cells(sc: &AgintcSoftc, cell: &[u32]) -> (usize, i32) {
    let [kind, num, flags, ..] = *cell else {
        kpanic(format_args!("{}: bogus interrupt type", sc.xname()));
    };
    // 2nd cell contains the interrupt number
    let mut irq = num as usize;

    // 1st cell contains type: 0 SPI (32-X), 1 PPI (16-31)
    match kind {
        0 => irq += SPI_BASE,
        1 => irq += PPI_BASE,
        _ => kpanic(format_args!("{}: bogus interrupt type", sc.xname())),
    }

    // SPIs are only active-high level or low-to-high edge
    let type_ = if flags & 0x3 != 0 {
        IST_EDGE_RISING
    } else {
        IST_LEVEL_HIGH
    };
    (irq, type_)
}

/// `agintc_intr_establish`: registers `h` for `irqno` at `level` on CPU `cpu` (`None`: the
/// primary one); `None` when the interrupt already runs on another CPU. An LPI is enabled in
/// the configuration table at its priority; the ITS's caller puts it in its slot.
pub fn agintc_intr_establish(
    sc: &AgintcSoftc,
    irqno: usize,
    type_: i32,
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: Option<&'static str>,
) -> Option<&'static Intrhand> {
    if (irqno >= sc.nintr() && irqno < LPI_BASE) || irqno >= LPI_BASE + sc.nlpi() {
        kpanic(format_args!(
            "agintc_intr_establish: bogus irqnumber {irqno}: {}",
            name.unwrap_or("")
        ));
    }

    let cpu = cpu.unwrap_or(sc.primary);

    let Some(ih) = MBox::new(
        M_DEVBUF,
        M_WAITOK,
        Intrhand {
            link: ListEntry::new(),
            h,
            ipl: level & IPL_IRQMASK,
            flags: AtomicI32::new(level & IPL_FLAGMASK),
            type_,
            irq: irqno,
            count: Evcount::new(),
            name,
            cpu,
            dead: AtomicBool::new(false),
        },
    ) else {
        kpanic(format_args!("agintc_intr_establish: no malloc(9)"));
    };

    if irqno < LPI_BASE {
        let g = sc.lock.lock();
        let psw = hal::cpu::intr_disable();

        let iq = sc.queue(irqno);
        if !iq.list.is_empty() && iq.cpu() != Some(cpu) {
            drop(psw);
            drop(g);
            drop(ih);
            return None;
        }
        let ih: &'static Intrhand = MBox::leak(ih);
        iq.list.insert_tail(ih);
        iq.cpu.store(cpu, Ordering::Relaxed);

        if let Some(name) = name {
            evcount_attach(&ih.count, name, Some(irqno as i32));
        }

        // DEBUG_AGINTC: not configured.

        agintc_intr_config(sc, irqno, type_);
        agintc_calc_irq(sc, irqno);

        drop(psw);
        drop(g);
        return Some(ih);
    }

    let ih: &'static Intrhand = MBox::leak(ih);
    if let Some(name) = name {
        evcount_attach(&ih.count, name, Some(irqno as i32));
    }
    if let Some(prop) = &sc.prop {
        let v = agintc_prival(ih.ipl, 4) | GICR_PROP_GROUP1 | GICR_PROP_ENABLE;
        // Make globally visible.
        prop.lock().set_u8(irqno - LPI_BASE, v);
    }
    Some(ih)
}

/// `agintc_intr_disestablish`: the `ic_disestablish` hook; the record is kept (see the
/// Deviations), a second disestablish of it `panic(9)`. An MBI's slot is freed; an LPI is
/// disabled in the configuration table.
fn agintc_intr_disestablish(sc: &AgintcSoftc, ih: &'static Intrhand) {
    let irqno = ih.irq;
    if ih.dead.swap(true, Ordering::AcqRel) {
        kpanic(format_args!(
            "agintc_intr_disestablish: irq {irqno} disestablished twice"
        ));
    }

    if irqno < LPI_BASE {
        let g = sc.lock.lock();
        let psw = hal::cpu::intr_disable();

        sc.queue(irqno).list.remove(ih);
        agintc_calc_irq(sc, irqno);

        // In case this is an MBI, free it
        for mr in sc.mbi.iter() {
            if irqno < mr.base {
                continue;
            }
            if irqno >= mr.base + mr.span() {
                break;
            }
            if let Some(slot) = mr.mbi.get(irqno - mr.base) {
                slot.store(false, Ordering::Release);
            }
        }

        if ih.name.is_some() {
            evcount_detach(&ih.count);
        }
        drop(psw);
        drop(g);
        return;
    }

    if let Some(prop) = &sc.prop {
        // Make globally visible.
        prop.lock().set_u8(irqno - LPI_BASE, 0);
    }

    if ih.name.is_some() {
        evcount_detach(&ih.count);
    }
}

/// `agintc_intr_establish_mbi`: the GIC's `ic_establish_msi`: a free message based SPI
/// claimed and established (edge-triggered), its doorbell and number handed back as the
/// MSI's address and data. The cookie is the handler.
fn agintc_intr_establish_mbi(
    sc: &AgintcSoftc,
    addr: &mut u64,
    data: &mut u64,
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: &'static str,
) -> Option<IcCookie> {
    let cpu = cpu.unwrap_or(sc.primary);

    for mr in sc.mbi.iter() {
        for (j, slot) in mr.mbi.iter().enumerate() {
            if slot
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
                .is_err()
            {
                continue;
            }

            let Some(ih) = agintc_intr_establish(
                sc,
                mr.base + j,
                IST_EDGE_RISING,
                level,
                Some(cpu),
                h,
                Some(name),
            ) else {
                slot.store(false, Ordering::Release);
                return None;
            };

            *addr = sc.mbi_addr;
            *data = (mr.base + j) as u64;

            return Some(IcCookie::Rec(ih));
        }
    }

    None
}

/// `agintc_d_wait_rwp`: waits for the distributor's register writes to take effect.
pub fn agintc_d_wait_rwp(sc: &AgintcSoftc) {
    let mut count = 100_000;
    let mut v;

    loop {
        v = sc.d.read(DCTLR);
        count -= 1;
        if count == 0 || v & GICD_CTLR_RWP == 0 {
            break;
        }
    }

    if count == 0 {
        kpanic(format_args!("agintc_d_wait_rwp: RWP timed out 0x08{v:x}"));
    }
}

/// `agintc_r_wait_rwp`: waits for the calling CPU's redistributor's register writes to take
/// effect.
pub fn agintc_r_wait_rwp(sc: &AgintcSoftc) {
    let r = sc.redist(hal::cpu::cpu_number());
    let mut count = 100_000;
    let mut v;

    loop {
        v = r.read(RCTLR);
        count -= 1;
        if count == 0 || v & GICR_CTLR_RWP == 0 {
            break;
        }
    }

    if count == 0 {
        kpanic(format_args!("agintc_r_wait_rwp: RWP timed out 0x08{v:x}"));
    }
}

/// `agintc_ipi_ddb`: another CPU entered ddb.
fn agintc_ipi_ddb() -> i32 {
    // XXX
    hal::cpu::trap::db_enter();
    1
}

/// `agintc_ipi_halt`: stop this CPU: the IPI ended and the level dropped first, so the CPU
/// halts with nothing masked, then both put back if it comes back.
fn agintc_ipi_halt(sc: &AgintcSoftc) -> i32 {
    let old = hal::cpu::curcpu().cpl();

    let psw = hal::cpu::intr_disable();
    icc::eoir1(u64::from(sc.ipi_num.load(Ordering::Relaxed)));
    sc.setipl(IPL_NONE);

    hal::cpu::trap::cpu_halt();

    sc.setipl(old);
    drop(psw);
    1
}

/// `agintc_ipi_handler`: the IPI's interrupt handler, at `IPL_IPI`, without the kernel lock:
/// runs the reasons posted for this CPU (`ARM_IPI_NOP` posts none). Its argument is the
/// softc's counted handle.
fn agintc_ipi_handler(v: &AnyArc) -> i32 {
    let Some(sc) = v.downcast_ref::<AgintcSoftc>() else {
        kpanic(format_args!("agintc_ipi_handler: not the softc"));
    };
    agintc_ipi_run(sc, hal::cpu::cpu_number())
}

/// [`agintc_ipi_handler`]'s work for CPU `cpu`: its reasons taken, each run.
fn agintc_ipi_run(sc: &AgintcSoftc, cpu: usize) -> i32 {
    if let Some(reason) = sc.ipi_reason.get(cpu)
        && reason.load(Ordering::Relaxed) != 0
    {
        let reasons = reason.swap(0, Ordering::AcqRel);
        if reasons & (1 << ARM_IPI_DDB) != 0 {
            agintc_ipi_ddb();
        }
        if reasons & (1 << ARM_IPI_HALT) != 0 {
            agintc_ipi_halt(sc);
        }
        // ARM_IPI_XCALL: NXCALL is 0 on arm64 (arm_cpu_xcall_dispatch).
    }

    1
}

/// `agintc_send_ipi`: `intr_send_ipi_func`: posts `reason` for CPU `cpu` and raises the IPI's
/// SGI at it alone.
fn agintc_send_ipi(cpu: usize, reason: i32) {
    if let Some(sc) = AGINTC.get() {
        agintc_send_ipi_sc(sc, cpu, reason);
    }
}

/// [`agintc_send_ipi`] on `sc`.
fn agintc_send_ipi_sc(sc: &AgintcSoftc, cpu: usize, reason: i32) {
    if reason == ARM_IPI_NOP {
        if cpu == hal::cpu::cpu_number() {
            return;
        }
    } else if let Some(r) = sc.ipi_reason.get(cpu) {
        r.fetch_or(1 << reason, Ordering::Release);
    }

    // will only send 1 cpu
    let sendmask = agintc_sgi1r(sc.mpidr_of(cpu), sc.ipi_num.load(Ordering::Relaxed));

    // The reason before the SGI (`hal::cpu::icc::sgi1r`'s `dsb ish`).
    icc::sgi1r(sendmask);
}

impl IntrHardware for AgintcSoftc {
    /// `agintc_splraise`: `setipl` must always be called because there is a race window where
    /// the variable is updated before the mask is set an interrupt occurs in that window
    /// without the mask always being set, the hardware might not get updated on the next
    /// `splraise` completely messing up spl protection.
    #[inline]
    fn splraise(&self, new: i32) -> i32 {
        let old = hal::cpu::curcpu().cpl();
        self.setipl(new.max(old));
        old
    }

    /// `agintc_spllower`.
    #[inline]
    fn spllower(&self, new: i32) -> i32 {
        let old = hal::cpu::curcpu().cpl();
        self.splx(new);
        old
    }

    /// `agintc_splx`.
    #[inline]
    fn splx(&self, new: i32) {
        if hal::cpu::curcpu().ipending() & hal::cpu::intr::smask(new) != 0 {
            hal::cpu::intr::do_pending_intr(new);
        }
        self.setipl(new);
    }

    /// `agintc_setipl`: the level in `ci_cpl` and in the priority mask (`ICC_PMR_EL1`); the
    /// disable here is only to keep hardware in sync with `ci_cpl`.
    #[inline]
    fn setipl(&self, new: i32) {
        let _psw = hal::cpu::intr_disable();
        hal::cpu::curcpu().set_cpl(new);
        icc::set_pmr(u64::from(agintc_prival(new, self.pmr_shift)));
    }

    /// `agintc_irq_handler`, as `arm_irq_dispatch`.
    fn dispatch_irq(&self, frame: &mut ClockFrame, irq: &InIrq) {
        agintc_irq_handler(self, frame, irq);
    }

    /// `agintc_enable_wakeup`.
    fn enable_wakeup(&self) {
        agintc_enable_wakeup(self);
    }

    /// `agintc_disable_wakeup`: all interrupts have already been enabled.
    fn disable_wakeup(&self) {}
}

impl InterruptController for AgintcSoftc {
    /// `agintc_intr_establish_fdt`.
    fn establish(
        &self,
        cell: &[u32],
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        let (irq, type_) = agintc_fdt_cells(self, cell);
        let ih = agintc_intr_establish(self, irq, type_, level, cpu, h, Some(name))?;
        Some(IcCookie::Rec(ih))
    }

    /// `agintc_intr_establish_mbi`, set when the distributor has MBI ranges.
    fn establish_msi(
        &self,
        addr: &mut u64,
        data: &mut u64,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        agintc_intr_establish_mbi(self, addr, data, level, cpu, h, name)
    }

    /// `sc_mbi_nranges > 0`: the C sets `ic_establish_msi` only then.
    fn has_msi(&self) -> bool {
        !self.mbi.is_empty()
    }

    fn disestablish(&self, ih: IcCookie) {
        agintc_intr_disestablish(self, hand(ih));
    }

    /// `agintc_route_irq`.
    fn route(&self, ih: IcCookie, enable: bool, cpu: usize) {
        agintc_route_irq(self, hand(ih), enable, cpu);
    }

    /// `agintc_cpuinit`.
    fn cpu_enable(&self) {
        agintc_cpuinit(self);
    }

    /// `agintc_intr_barrier`: `sched_barrier(ih->ih_ci)`.
    fn barrier(&self, ih: IcCookie) -> BarrierWait {
        BarrierWait::Cpu(hand(ih).cpu)
    }

    /// `agintc_intr_set_wakeup`.
    fn set_wakeup(&self, ih: IcCookie) {
        hand(ih).flags.fetch_or(IPL_WAKEUP, Ordering::Relaxed);
    }
}

/// `agintc_msi_match`: an ITS, except on a Qualcomm Oryon booted with ACPI (XXX for some
/// reason MSIs don't work on Qualcomm X1E SoCs in ACPI mode, so skip attaching the ITS in
/// that case; MSIs work fine when booting with a DTB).
pub fn agintc_msi_match(fa: &FdtArgs) -> i32 {
    let midr = hal::cpu::curcpu().midr();
    if OF_is_compatible(OF_peer(0), b"openbsd,acpi")
        && cpu_impl(midr) == CPU_IMPL_QCOM
        && cpu_part(midr) == CPU_PART_ORYON
    {
        return 0;
    }

    i32::from(OF_is_compatible(fa.node, b"arm,gic-v3-its"))
}

/// The page size a `GITS_BASER`'s `Page_Size` field names, in bytes.
const fn gits_baser_pgsz(baser: u64) -> usize {
    match baser & GITS_BASER_PGSZ_MASK {
        GITS_BASER_PGSZ_16K => 4 * PAGE_SIZE,
        GITS_BASER_PGSZ_64K => 16 * PAGE_SIZE,
        _ => PAGE_SIZE,
    }
}

/// `roundup(x, y)`.
const fn roundup(x: usize, y: usize) -> usize {
    x.div_ceil(y) * y
}

/// The device table's layout: its size in bytes and the highest device ID it can map, for
/// `devbits` device ID bits, `dte_sz`-byte entries and `pgsz`-byte pages, flat or `indirect`
/// (a level of 8-byte pointers to pages of entries); clamped to the 256 pages a `GITS_BASER`
/// can describe.
const fn agintc_msi_dtt_layout(
    devbits: u32,
    dte_sz: usize,
    pgsz: usize,
    indirect: bool,
) -> (usize, u32) {
    let mut size;
    if indirect {
        size = 1usize << devbits;
        size /= pgsz / dte_sz;
        size *= size_of::<u64>();
        size = roundup(size, pgsz);
    } else {
        size = roundup((1usize << devbits) * dte_sz, pgsz);
    }

    // Clamp down to maximum configurable num pages
    if size / pgsz > GITS_BASER_SZ_MASK as usize + 1 {
        size = (GITS_BASER_SZ_MASK as usize + 1) * pgsz;
    }

    // Calculate max deviceid based off configured size
    let max = if indirect {
        (size / size_of::<u64>()) * (pgsz / dte_sz) - 1
    } else {
        size / dte_sz - 1
    };
    (size, max as u32)
}

/// Determine the maximum supported page size: `GITS_BASER(i)` tried with 64 KB, 16 KB, then
/// 4 KB pages, the register as it reads back.
fn agintc_msi_baser_pgsz(m: &Mmio<Gits>, i: usize) -> u64 {
    for pgsz in [GITS_BASER_PGSZ_64K, GITS_BASER_PGSZ_16K] {
        let cur = m.read_at::<u64>(gits_baser(i));
        m.write_at::<u64>(gits_baser(i), (cur & !GITS_BASER_PGSZ_MASK) | pgsz);
        let got = m.read_at::<u64>(gits_baser(i));
        if got & GITS_BASER_PGSZ_MASK == pgsz {
            return got;
        }
    }

    let cur = m.read_at::<u64>(gits_baser(i));
    m.write_at::<u64>(
        gits_baser(i),
        (cur & !GITS_BASER_PGSZ_MASK) | GITS_BASER_PGSZ_4K,
    );
    m.read_at::<u64>(gits_baser(i))
}

/// The ITS's registers and arguments, before its tables are set up.
struct ItsInit<'a> {
    m: Mmio<Gits>,
    bus: &'a FdtBusRef,
    msi_addr: u64,
    msi_delta: u64,
    xname: [u8; 16],
}

/// `agintc_msi_attach`: maps the ITS, sets up its command queue and its device and collection
/// tables, enables it, maps one collection per CPU and registers it as an MSI controller.
pub fn agintc_msi_attach(dev: &DevRef, fa: &FdtArgs) {
    let Some(reg) = fa.reg().first() else {
        printf(format_args!(": no registers\n"));
        return;
    };

    let Ok(m) = fa.bus.map::<Gits>(reg.addr, reg.size, DevCache::Device) else {
        printf(format_args!(": can't map registers\n"));
        return;
    };

    let mut msi_addr = reg.addr + GITS_TRANSLATER;
    let mut msi_delta = 0;
    let mut pre_its = [0u32; 2];
    if OF_getpropintarray(fa.node, b"socionext,synquacer-pre-its", &mut pre_its) == 8 {
        msi_addr = u64::from(pre_its[0]);
        msi_delta = 4;
    }

    let Some(gic) = AGINTC.get() else {
        printf(format_args!(": no GIC\n"));
        crate::vm::km::unmap_device(m);
        return;
    };

    let mut xname = [0u8; 16];
    let name = dev.xname().as_bytes();
    let n = name.len().min(xname.len() - 1);
    xname[..n].copy_from_slice(&name[..n]);

    let init = ItsInit {
        m,
        bus: &fa.bus,
        msi_addr,
        msi_delta,
        xname,
    };
    let Some(sc) = agintc_msi_init(init, gic, hal::cpu::ncpus()) else {
        return;
    };

    printf(format_args!("\n"));

    arm_intr_register_fdt(IcDesc {
        node: fa.node,
        gic_its_id: OF_getpropint(fa.node, b"openbsd,gic-its-id", 0),
        ops: sc,
    });
}

/// The heart of `agintc_msi_attach`: the ITS's type checked, its command queue, device and
/// collection tables set up, the ITS enabled and one collection per CPU mapped (`ncpus` of
/// them); the softc, kept for ever, or `None` after the C's message (its `unmap:` path).
fn agintc_msi_init(
    init: ItsInit<'_>,
    gic: &'static AgintcSoftc,
    ncpus: usize,
) -> Option<&'static AgintcMsiSoftc> {
    let ItsInit {
        m,
        bus,
        msi_addr,
        msi_delta,
        xname,
    } = init;

    let typer = m.read(STYPER);
    if typer & GITS_TYPER_PHYS == 0 || typer & GITS_TYPER_PTA != 0 {
        printf(format_args!(": unsupported type 0x{typer:016x}\n"));
        crate::vm::km::unmap_device(m);
        return None;
    }
    let ite_sz = gits_typer_ite_sz(typer) as usize + 1;
    let devbits = gits_typer_devbits(typer) as u32 + 1;
    let cidbits = if typer & GITS_TYPER_CIL != 0 {
        gits_typer_cidbits(typer) as u32 + 1
    } else {
        16
    };

    // Set up command queue.
    let Some(cmdq) = agintc_dmamem_alloc(bus, GITS_CMDQ_SIZE, GITS_CMDQ_SIZE) else {
        printf(format_args!(": can't alloc command queue\n"));
        crate::vm::km::unmap_device(m);
        return None;
    };
    m.write(
        SCBASER,
        cmdq.dva
            | GITS_CBASER_IC_NORM_NC
            | ((GITS_CMDQ_SIZE / PAGE_SIZE) as u64 - 1)
            | GITS_CBASER_VALID,
    );

    // Set up device translation table.
    let mut dtt: Option<AgintcDmamem> = None;
    let mut dtt_pgsz = PAGE_SIZE;
    let mut dte_sz = 8;
    let mut dtt_indirect = false;
    let mut deviceid_max = 0;
    for i in 0..GITS_NUM_BASER {
        let baser = m.read_at::<u64>(gits_baser(i));
        if baser & GITS_BASER_TYPE_MASK != GITS_BASER_TYPE_DEVICE {
            continue;
        }

        // Determine the maximum supported page size.
        let mut baser = agintc_msi_baser_pgsz(&m, i);
        dtt_pgsz = gits_baser_pgsz(baser);

        // Calculate table size.
        dte_sz = gits_baser_tte_sz(baser) as usize + 1;
        let (flat, _) = agintc_msi_dtt_layout(devbits, dte_sz, dtt_pgsz, false);

        // Might make sense to go indirect
        if flat > 2 * dtt_pgsz {
            m.write_at::<u64>(gits_baser(i), baser | GITS_BASER_INDIRECT);
            if m.read_at::<u64>(gits_baser(i)) & GITS_BASER_INDIRECT != 0 {
                dtt_indirect = true;
            }
        }
        let (size, max) = agintc_msi_dtt_layout(devbits, dte_sz, dtt_pgsz, dtt_indirect);
        deviceid_max = max;

        // Allocate table.
        let Some(table) = agintc_dmamem_alloc(bus, size, dtt_pgsz) else {
            printf(format_args!(": can't alloc translation table\n"));
            crate::vm::km::unmap_device(m);
            return None;
        };

        // Configure table.
        let dtt_pa = table.dva;
        kassert!((dtt_pa & GITS_BASER_PA_MASK) == dtt_pa);
        baser = GITS_BASER_IC_NORM_NC
            | (baser & GITS_BASER_PGSZ_MASK)
            | dtt_pa
            | ((size / dtt_pgsz) as u64 - 1)
            | if dtt_indirect { GITS_BASER_INDIRECT } else { 0 }
            | GITS_BASER_VALID;
        m.write_at::<u64>(gits_baser(i), baser);
        // A second device table keeps the first, which a BASER names (the C leaks it).
        if let Some(old) = dtt.replace(table) {
            let _ = MBox::new(M_DEVBUF, M_WAITOK, old).map(MBox::leak);
        }
    }

    // Set up collection translation table.
    let mut ctt: Option<AgintcDmamem> = None;
    for i in 0..GITS_NUM_BASER {
        let baser = m.read_at::<u64>(gits_baser(i));
        if baser & GITS_BASER_TYPE_MASK != GITS_BASER_TYPE_COLL {
            continue;
        }

        // Determine the maximum supported page size.
        let baser = agintc_msi_baser_pgsz(&m, i);
        let ctt_pgsz = gits_baser_pgsz(baser);

        // Calculate table size.
        let cte_sz = gits_baser_tte_sz(baser) as usize + 1;
        let size = roundup((1usize << cidbits) * cte_sz, ctt_pgsz);

        // Allocate table.
        let Some(table) = agintc_dmamem_alloc(bus, size, ctt_pgsz) else {
            printf(format_args!(": can't alloc translation table\n"));
            crate::vm::km::unmap_device(m);
            return None;
        };

        // Configure table.
        let ctt_pa = table.dva;
        kassert!((ctt_pa & GITS_BASER_PA_MASK) == ctt_pa);
        m.write_at::<u64>(
            gits_baser(i),
            GITS_BASER_IC_NORM_NC
                | (baser & GITS_BASER_PGSZ_MASK)
                | ctt_pa
                | ((size / ctt_pgsz) as u64 - 1)
                | GITS_BASER_VALID,
        );
        if let Some(old) = ctt.replace(table) {
            let _ = MBox::new(M_DEVBUF, M_WAITOK, old).map(MBox::leak);
        }
    }

    // Enable ITS.
    m.write(SCTLR, GITS_CTLR_ENABLED);

    let Some(sc) = MBox::new(
        M_DEVBUF,
        M_WAITOK,
        AgintcMsiSoftc {
            m,
            bus: bus.clone(),
            msi_addr,
            msi_delta,
            cmdq: Mutexed::new(IPL_NONE, ItsCmdq { q: cmdq, idx: 0 }),
            deviceid_max,
            dtt: dtt.map(|t| Mutexed::new(IPL_NONE, t)),
            dtt_pgsz,
            dte_sz,
            dtt_indirect,
            _ctt: ctt,
            ite_sz,
            devices: List::new(),
            gic,
            xname,
            me: OnceStatic::new(),
        },
    ) else {
        kpanic(format_args!("agintc_msi_attach: no malloc(9)"));
    };
    let sc: &'static AgintcMsiSoftc = MBox::leak(sc);
    let _ = sc.me.set(sc);

    // Create one collection per core.
    kassert!(ncpus <= gic.num_redist());
    for i in 0..ncpus {
        let cmd = GitsCmd {
            cmd: MAPC,
            dw2: GITS_CMD_VALID | (gic.processor_of(i) << 16) | i as u64,
            ..GitsCmd::default()
        };
        agintc_msi_cmds(sc, &[cmd]);
    }

    Some(sc)
}

/// `agintc_msi_send_cmd` for each of `cmds`, then `agintc_msi_wait_cmd`, under the queue's
/// mutex.
fn agintc_msi_cmds(sc: &AgintcMsiSoftc, cmds: &[GitsCmd]) {
    let mut q = sc.cmdq.lock();
    for cmd in cmds {
        agintc_msi_send_cmd(sc, &mut q, cmd);
    }
    agintc_msi_wait_cmd(sc, &q);
}

/// `agintc_msi_send_cmd`: the command at the queue's write index, made globally visible,
/// then the index moved on and given to the ITS.
fn agintc_msi_send_cmd(sc: &AgintcMsiSoftc, q: &mut ItsCmdq, cmd: &GitsCmd) {
    let off = q.idx * GITS_CMD_SIZE;
    for (k, w) in cmd.words().into_iter().enumerate() {
        q.q.buf.write::<u64>(off + 8 * k, w);
    }

    // Make globally visible.
    q.q.publish(off, GITS_CMD_SIZE);

    q.idx = (q.idx + 1) % GITS_CMDQ_NENTRIES;
    sc.m.write(SCWRITER, (q.idx * GITS_CMD_SIZE) as u64);
}

/// `agintc_msi_wait_cmd`: waits (up to a thousand microseconds) for the ITS to read up to the
/// write index.
fn agintc_msi_wait_cmd(sc: &AgintcMsiSoftc, q: &ItsCmdq) {
    let mut timo = 1000;
    while timo > 0 {
        let creadr = sc.m.read(SCREADR);
        if creadr == (q.idx * GITS_CMD_SIZE) as u64 {
            break;
        }
        crate::time::clock::delay(1);
        timo -= 1;
    }
    if timo == 0 {
        printf(format_args!("{}: command queue timeout\n", sc.xname()));
    }
}

/// `agintc_msi_create_device_table`: an indirect device table's second level for
/// `deviceid`, allocated the first time one of its devices is mapped (`ENXIO` out of bounds,
/// `ENOMEM` when there is no memory).
pub fn agintc_msi_create_device_table(sc: &AgintcMsiSoftc, deviceid: u32) -> Result<(), Errno> {
    let idx = deviceid as usize / (sc.dtt_pgsz / sc.dte_sz);

    // Out of bounds
    if deviceid > sc.deviceid_max {
        return Err(Errno::ENXIO);
    }

    // No need to adjust
    if !sc.dtt_indirect {
        return Ok(());
    }
    let Some(dtt) = &sc.dtt else {
        return Ok(());
    };

    // Table already allocated
    if dtt.lock().get_u64(idx) != 0 {
        return Ok(());
    }

    // FIXME: leaks
    let Some(l2) = agintc_dmamem_alloc(&sc.bus, sc.dtt_pgsz, sc.dtt_pgsz) else {
        return Err(Errno::ENOMEM);
    };

    let dtt_pa = l2.dva;
    kassert!((dtt_pa & GITS_BASER_PA_MASK) == dtt_pa);
    dtt.lock().set_u64(idx, dtt_pa | GITS_BASER_VALID);
    // The ITS reads the table for ever.
    let _ = MBox::new(M_DEVBUF, M_WAITOK, l2).map(MBox::leak);
    Ok(())
}

/// `agintc_msi_create_device`: a device ID mapped to a new interrupt translation table
/// (`MAPD`); `None` past the table, or when there is no memory (see the Deviations).
pub fn agintc_msi_create_device(
    sc: &AgintcMsiSoftc,
    deviceid: u32,
) -> Option<&'static AgintcMsiDevice> {
    if deviceid > sc.deviceid_max {
        return None;
    }

    if agintc_msi_create_device_table(sc, deviceid).is_err() {
        return None;
    }

    let itt = agintc_dmamem_alloc(&sc.bus, 32 * sc.ite_sz, PAGE_SIZE)?;
    let itt_dva = itt.dva;
    let Some(md) = MBox::new(
        M_DEVBUF,
        M_WAITOK,
        AgintcMsiDevice {
            link: ListEntry::new(),
            deviceid,
            events: AtomicU32::new(0),
            _itt: itt,
        },
    ) else {
        kpanic(format_args!("agintc_msi_create_device: no malloc(9)"));
    };
    let md: &'static AgintcMsiDevice = MBox::leak(md);
    sc.devices.insert_head(md);

    let cmd = GitsCmd {
        cmd: MAPD,
        deviceid,
        eventid: 4, // size
        dw2: itt_dva | GITS_CMD_VALID,
        ..GitsCmd::default()
    };
    agintc_msi_cmds(sc, &[cmd]);

    Some(md)
}

/// `agintc_msi_find_device`: the device ID's record, made when there is none.
pub fn agintc_msi_find_device(
    sc: &AgintcMsiSoftc,
    deviceid: u32,
) -> Option<&'static AgintcMsiDevice> {
    if let Some(md) = sc.devices.iter().find(|md| md.deviceid == deviceid) {
        return Some(md);
    }

    agintc_msi_create_device(sc, deviceid)
}

/// `agintc_msi_discard`: the ITS forgets the LPI's event (`DISCARD`, then `SYNC` at its
/// redistributor).
pub fn agintc_msi_discard(li: &AgintcLpiInfo) {
    let sc = li.msic;
    let cmds = [
        GitsCmd {
            cmd: DISCARD,
            deviceid: li.deviceid,
            eventid: li.eventid,
            ..GitsCmd::default()
        },
        GitsCmd {
            cmd: SYNC,
            dw2: sc.gic.processor_of(li.cpu) << 16,
            ..GitsCmd::default()
        },
    ];
    agintc_msi_cmds(sc, &cmds);
}

/// `agintc_msi_inv`: the ITS reads the LPI's configuration again (`INV`, then `SYNC` at its
/// redistributor).
pub fn agintc_msi_inv(li: &AgintcLpiInfo) {
    let sc = li.msic;
    let cmds = [
        GitsCmd {
            cmd: INV,
            deviceid: li.deviceid,
            eventid: li.eventid,
            ..GitsCmd::default()
        },
        GitsCmd {
            cmd: SYNC,
            dw2: sc.gic.processor_of(li.cpu) << 16,
            ..GitsCmd::default()
        },
    ];
    agintc_msi_cmds(sc, &cmds);
}

impl AgintcMsiSoftc {
    /// [`AgintcMsiSoftc::establish_msi`] on `self` with the GIC's MSI lock held: an event ID
    /// of the device taken, a free LPI established and mapped (`MAPTI`, `SYNC`).
    fn establish_msi_locked(
        &'static self,
        addr: &mut u64,
        data: &mut u64,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        let gic = self.gic;
        let deviceid = *data as u32;

        let cpu = cpu.unwrap_or(gic.primary);

        let md = agintc_msi_find_device(self, deviceid)?;

        let bit = |e: u32| 1u32.checked_shl(e).unwrap_or(0);
        let events = md.events.load(Ordering::Relaxed);
        let mut eventid = *addr as u32;
        if eventid > 0 && events & bit(eventid) != 0 {
            return None;
        }
        while eventid < 32 {
            if md.events.load(Ordering::Relaxed) & bit(eventid) == 0 {
                md.events.fetch_or(bit(eventid), Ordering::Relaxed);
                break;
            }
            eventid += 1;
        }
        if eventid >= 32 {
            return None;
        }

        for (i, slot) in gic.lpi.iter().enumerate() {
            if slot.first().is_some() {
                continue;
            }

            let ih = agintc_intr_establish(
                gic,
                LPI_BASE + i,
                IST_EDGE_RISING,
                level,
                Some(cpu),
                h,
                Some(name),
            )?;
            let Some(li) = MBox::new(
                M_DEVBUF,
                M_WAITOK,
                AgintcLpiInfo {
                    link: ListEntry::new(),
                    msic: self,
                    cpu,
                    deviceid,
                    eventid,
                    ih,
                },
            ) else {
                kpanic(format_args!("agintc_intr_establish_msi: no malloc(9)"));
            };
            let li: &'static AgintcLpiInfo = MBox::leak(li);
            slot.insert_head(li);

            let cmds = [
                GitsCmd {
                    cmd: MAPTI,
                    deviceid,
                    eventid,
                    intid: (LPI_BASE + i) as u32,
                    dw2: cpu as u64,
                    ..GitsCmd::default()
                },
                GitsCmd {
                    cmd: SYNC,
                    dw2: gic.processor_of(cpu) << 16,
                    ..GitsCmd::default()
                },
            ];
            agintc_msi_cmds(self, &cmds);

            *addr = self.msi_addr + u64::from(deviceid) * self.msi_delta;
            *data = u64::from(eventid);
            return Some(IcCookie::Rec(li));
        }

        None
    }
}

impl InterruptController for AgintcMsiSoftc {
    /// `agintc_intr_establish_msi`: a device's MSI (its ID in `*data`, the event ID wanted
    /// in `*addr`, 0 for any) as an LPI of the GIC; the doorbell and the event ID handed
    /// back as the MSI's address and data. The cookie is the LPI's record.
    fn establish_msi(
        &self,
        addr: &mut u64,
        data: &mut u64,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        let Some(&this) = self.me.get() else {
            kpanic(format_args!("agintcmsi: an ITS that was never attached"));
        };
        let _w = this.gic.msi_lock.write();
        this.establish_msi_locked(addr, data, level, cpu, h, name)
    }

    fn has_msi(&self) -> bool {
        true
    }

    /// `agintc_intr_disestablish_msi`: the LPI's handler disestablished, its event discarded
    /// and the ITS's cache invalidated, its slot emptied (the record kept, see the
    /// Deviations).
    fn disestablish(&self, ih: IcCookie) {
        let li = lpi_info(ih);
        let gic = li.msic.gic;
        let _w = gic.msi_lock.write();

        agintc_intr_disestablish(gic, li.ih);
        agintc_msi_discard(li);
        agintc_msi_inv(li);

        if let Some(slot) = gic.lpi.get(li.ih.irq - LPI_BASE) {
            slot.remove(li);
        }
    }

    /// `agintc_intr_barrier_msi`: the GIC's barrier for the LPI's handler.
    fn barrier(&self, ih: IcCookie) -> BarrierWait {
        BarrierWait::Cpu(lpi_info(ih).ih.cpu)
    }
}

/// `agintc_dmamem_alloc`: `size` bytes of zeroed, physically contiguous memory aligned to
/// `align`, mapped uncached, at the address the bus's `dma-ranges` give it, made globally
/// visible; `None` when there is none. The provider is the non-coherent one whatever the
/// bus's tag: every write-back of the C (`cpu_dcache_wb_range`, here and at each table write)
/// is unconditional, and on a coherent tag `DmaBuf::sync` would not write back (see the
/// module's Redesign).
fn agintc_dmamem_alloc(bus: &FdtBusRef, size: usize, align: usize) -> Option<AgintcDmamem> {
    let req = DmaReq {
        size,
        align,
        zero: true,
        nocache: true,
        ..DmaReq::default()
    };
    let buf = DmaBuf::alloc(crate::vm::km::bus_dma_memory(false), &req).ok()?;
    let dva = bus.dma_addr(buf.seg().addr, size as u64);
    let adm = AgintcDmamem { buf, dva };

    // Make globally visible.
    adm.publish(0, size);
    Some(adm)
}

/// The number of IPIs the IPI handler has run, on every CPU (its event counter): what the
/// `qemu` self-check of `cpu_boot_secondary_processors` watches; 0 before the controller
/// attached.
pub fn agintc_ipi_count() -> u64 {
    let Some(sc) = AGINTC.get() else {
        return 0;
    };
    let irq = sc.ipi_num.load(Ordering::Relaxed) as usize;
    sc.handler.get(irq).map_or(0, |iq| {
        iq.list
            .iter()
            .map(|ih| ih.count.ec_count.load(Ordering::Relaxed))
            .sum()
    })
}

const _: () = {
    assert!(GITS_CMDQ_NENTRIES == 2048);
    assert!(size_of::<[u64; 4]>() == GITS_CMD_SIZE);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The distributor, the redistributors and the ITS are blocks of host memory mapped with
    // `km::map_device`: a register reads back what was last written, which is what the C
    // writes and reads; a test writes the "hardware's" answers (GITS_TYPER, GITS_CREADR, the
    // BASERs' types) itself. The CPU interface is the host's double (`hal::cpu::icc`, one
    // register file per test thread), the tables `DmaBuf`s of the test's pages. Handlers are
    // established without a name (no `evcount_attach`: the counters' list is the process's,
    // which the evcount tests count).

    use core::panic::AssertUnwindSafe;
    use std::sync::{Mutex, MutexGuard};
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use hal::cpu::icc::IccRegs;
    use hal::cpu::intr::{IPL_BIO, IPL_CLOCK, IPL_NET};
    use sysdefs::base::types::Paddr;

    use super::*;
    use crate::dev::arm64::fdt::FdtBus;
    use crate::dev::arm64::intr::ARM_IPI_XCALL;
    use crate::testutil;

    const PAGE: usize = 4096;

    /// A zeroed, page-aligned block of `len` bytes of host memory, leaked: the device's
    /// registers.
    fn device_block(len: usize) -> Paddr {
        let block: &'static mut [u8] = vec![0u8; len + PAGE].leak();
        Paddr::new(block.as_ptr().expose_provenance().next_multiple_of(PAGE))
    }

    fn map<R: RegBlock>(len: usize) -> Mmio<R> {
        crate::vm::km::map_device(device_block(len), len, DevCache::Device).unwrap()
    }

    fn setup() -> MutexGuard<'static, ()> {
        let g = testutil::pages_install(16 << 20);
        testutil::kernel_maps_install();
        icc::set_regs(IccRegs::default());
        g
    }

    /// A softc of `nintr` interrupts and `nlpi` LPIs over two fresh redistributors, CPU 1 on
    /// the second, every CPU's MPIDR its number, the attach's CPU 0, and the MBI ranges
    /// `mbi` (`(base, span)`).
    fn new_softc(nintr: usize, nlpi: usize, mbi: &[(usize, usize)]) -> AgintcSoftc {
        let d = map::<Gicd>(0x10000);
        let r = MArray::new(M_DEVBUF, M_WAITOK, 2, |_| map::<Gicr>(0x20000)).unwrap();
        let mut cpuremap = MArray::new(M_DEVBUF, M_WAITOK, hal::cpu::MAXCPUS, |_| 0).unwrap();
        cpuremap[1] = 1;
        let prop = (nlpi > 0).then(|| {
            let root = FdtBus::root();
            Mutexed::new(
                IPL_NONE,
                agintc_dmamem_alloc(&root, GICR_PROP_SIZE, GICR_PROP_SIZE).unwrap(),
            )
        });
        let mbi = MArray::new(M_DEVBUF, M_WAITOK, mbi.len(), |i| AgintcMbiRange {
            base: mbi[i].0,
            mbi: MArray::new(M_DEVBUF, M_WAITOK, mbi[i].1, |_| AtomicBool::new(false)).unwrap(),
        })
        .unwrap();
        let mut xname = [0u8; 16];
        xname[..7].copy_from_slice(b"agintc0");
        AgintcSoftc {
            d,
            _rbase: MArray::new(M_DEVBUF, M_WAITOK, 0, |_| None).unwrap(),
            r,
            processor: MArray::new(M_DEVBUF, M_WAITOK, 2, |n| 0x10 + n as u16).unwrap(),
            cpuremap,
            mpidr: MArray::new(M_DEVBUF, M_WAITOK, hal::cpu::MAXCPUS, |n| n as u64).unwrap(),
            handler: MArray::new(M_DEVBUF, M_WAITOK, nintr, |_| Intrq::new()).unwrap(),
            lpi: MArray::new(M_DEVBUF, M_WAITOK, nlpi, |_| List::new()).unwrap(),
            prop,
            _pend: None,
            mbi_addr: 0x0800_0040,
            mbi,
            prio_shift: 4,
            pmr_shift: 4,
            spur: Evcount::new(),
            ipi_num: AtomicU32::new(0),
            ipi_reason: MArray::new(M_DEVBUF, M_WAITOK, hal::cpu::MAXCPUS, |_| AtomicU32::new(0))
                .unwrap(),
            ipi_irq: OnceStatic::new(),
            primary: 0,
            xname,
            lock: Mutexed::new(IPL_HIGH, ()),
            msi_lock: Rwlocked::new("agintcmsi", ()),
        }
    }

    fn softc(nlpi: usize, mbi: &[(usize, usize)]) -> &'static AgintcSoftc {
        MBox::leak(MBox::new(M_DEVBUF, M_WAITOK, new_softc(288, nlpi, mbi)).unwrap())
    }

    /// What the handlers saw: their argument's address, the level, the priority mask.
    static SEEN: Mutex<Vec<(usize, i32, u64)>> = Mutex::new(Vec::new());

    fn record(arg: *mut core::ffi::c_void) -> i32 {
        SEEN.lock()
            .unwrap()
            .push((arg.addr(), hal::cpu::curcpu().cpl(), icc::pmr()));
        1
    }

    fn on_frame(frame: &ClockFrame) -> i32 {
        SEEN.lock().unwrap().push((
            core::ptr::from_ref(frame).addr(),
            hal::cpu::curcpu().cpl(),
            icc::pmr(),
        ));
        1
    }

    fn establish(
        sc: &'static AgintcSoftc,
        irq: usize,
        type_: i32,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
    ) -> Option<&'static Intrhand> {
        agintc_intr_establish(sc, irq, type_, level, cpu, h, None)
    }

    /// The calling thread's CPU interface with `f` applied to its registers.
    fn icc_set(f: impl FnOnce(&mut IccRegs)) {
        let mut r = icc::regs();
        f(&mut r);
        icc::set_regs(r);
    }

    #[test]
    fn the_register_offsets_priorities_and_encodings_are_the_cs() {
        assert_eq!(gicd_isenabler(32), 0x104);
        assert_eq!(gicd_icenabler(64), 0x188);
        assert_eq!(gicd_ipriorityr(33), 0x421);
        assert_eq!(gicd_icfgr(32), 0xc08);
        assert_eq!(gicd_icfgr_trig_edge(33), 0x2 << 2);
        assert_eq!(gicd_nsacr(32), 0xe08);
        assert_eq!(gicd_irouter(32), 0x6100);
        assert_eq!(gicr_ipriorityr(3), 0x10403);
        // arm64's levels (the host's are amd64's): IPL_HIGH 11, IPL_IPI 12, whose priority
        // stays above IPL_HIGH's mask.
        assert_eq!(agintc_prival(11, 4), 0x40);
        assert_eq!(agintc_prival(IPL_NONE, 4), 0xf0);
        assert_eq!(agintc_prival(12, 4), 0x30);
        assert_eq!(agintc_prival(IPL_NONE, 3), 0xf8);
        assert_eq!(ffs(0xb0), 5);
        assert_eq!(ffs(0), 0);
        assert_eq!(fls(16383), 14);
        // 8192 LPIs above 8192: 14 ID bits, encoded as 13.
        assert_eq!(
            agintc_propbaser(0x4000_0000, 8192),
            0x4000_0000 | 0x400 | 0x80 | 13
        );
        // Aff0 3, Aff1 1, Aff2 2, Aff3 4.
        let mpidr = (4u64 << 32) | (2 << 16) | (1 << 8) | 3;
        let v = agintc_sgi1r(mpidr, 5);
        assert_eq!(v & 0xffff, 1 << 3);
        assert_eq!((v >> 16) & 0xff, 1);
        assert_eq!((v >> 24) & 0xf, 5);
        assert_eq!((v >> 32) & 0xff, 2);
        assert_eq!((v >> 48) & 0xff, 4);
        assert_eq!(agintc_mpidr_affinity(mpidr | (1 << 31)), 0x0402_0103);
        assert_eq!(agintc_redist_size(0, 0), 0x20000);
        assert_eq!(agintc_redist_size(GICR_TYPER_VLPIS, 0), 0x40000);
        assert_eq!(agintc_redist_size(GICR_TYPER_VLPIS, 0x30000), 0x30000);
        let midr = (0x51u64 << 24) | (0x001 << 4);
        assert_eq!(
            (cpu_impl(midr), cpu_part(midr)),
            (CPU_IMPL_QCOM, CPU_PART_ORYON)
        );
    }

    #[test]
    fn the_non_secure_view_is_found_by_the_priority_bits_that_stick() {
        // Four priority bits implemented (PRIbits 3) and four that stick: the shifted view.
        icc::set_regs(IccRegs {
            ctlr: 3 << 8,
            pmr: 0xf0,
            pmr_bits: 0xf0,
            ..IccRegs::default()
        });
        assert!(agintc_nonsecure_view());
        assert_eq!(icc::pmr(), 0xf0, "the mask put back");
        // Five bits stick (0xbf keeps 0xb8): not the shifted view.
        icc_set(|r| r.pmr_bits = 0xf8);
        assert!(!agintc_nonsecure_view());
        // Eight bits implemented, all sticking: 0xbf's lowest bit is bit 0.
        icc_set(|r| {
            r.ctlr = 7 << 8;
            r.pmr_bits = 0;
        });
        assert!(agintc_nonsecure_view());
    }

    #[test]
    fn the_its_commands_and_tables_are_laid_out_as_the_cs() {
        let cmd = GitsCmd {
            cmd: MAPTI,
            deviceid: 0x10,
            eventid: 2,
            intid: 8193,
            dw2: 1,
            ..GitsCmd::default()
        };
        assert_eq!(cmd.words(), [0x10_0000_000a, (8193 << 32) | 2, 1, 0]);
        let mapd = GitsCmd {
            cmd: MAPD,
            deviceid: 8,
            eventid: 4,
            dw2: 0x8000_0000 | GITS_CMD_VALID,
            ..GitsCmd::default()
        };
        assert_eq!(mapd.words()[0], (8 << 32) | 0x08);
        assert_eq!(mapd.words()[1], 4);
        assert_eq!(gits_baser_pgsz(GITS_BASER_PGSZ_64K), 65536);
        assert_eq!(gits_baser_pgsz(GITS_BASER_PGSZ_16K), 16384);
        assert_eq!(gits_baser_pgsz(GITS_BASER_PGSZ_4K), PAGE_SIZE);
        // 16 device ID bits, 8-byte entries, 64 KB pages: 512 KB flat; indirect, one page of
        // pointers covers every ID.
        assert_eq!(
            agintc_msi_dtt_layout(16, 8, 65536, false),
            (512 * 1024, 65535)
        );
        assert_eq!(
            agintc_msi_dtt_layout(16, 8, 65536, true),
            (65536, 8192 * 8192 - 1)
        );
        // 32 bits flat clamps at 256 pages.
        assert_eq!(agintc_msi_dtt_layout(32, 8, 4096, false).0, 256 * 4096);
        assert_eq!(gits_typer_devbits(0xf << 13), 15);
        assert_eq!(gits_typer_ite_sz(0x7 << 4), 7);
        assert_eq!(gits_baser_tte_sz(7 << 48), 7);
    }

    #[test]
    fn the_redistributor_walk_follows_the_last_bit_and_the_region_size() {
        let _g = setup();
        // Two regions: the first holds two redistributors (the second's LAST bit ends it),
        // the second one (its size ends it), the middle one with virtual LPIs.
        let a: Mmio<GicrRegion> = map(0x80000);
        let b: Mmio<GicrRegion> = map(0x20000);
        a.write_at::<u64>(GICR_TYPER, 0);
        a.write_at::<u64>(0x20000 + GICR_TYPER, GICR_TYPER_VLPIS | GICR_TYPER_LAST);
        b.write_at::<u64>(GICR_TYPER, 1 << 32);
        let rbase = [Some(a), Some(b)];
        let sizes = [0x80000, 0x20000];
        let mut w = RedistWalk { idx: 0, offset: 0 };
        let seen: Vec<_> = core::iter::from_fn(|| w.next(&rbase, &sizes, 0))
            .map(|(idx, off, sz, _)| (idx, off, sz))
            .collect();
        assert_eq!(
            seen,
            vec![(0, 0, 0x20000), (0, 0x20000, 0x40000), (1, 0, 0x20000)]
        );
        // A stride replaces the frames' size.
        let mut w = RedistWalk { idx: 0, offset: 0 };
        assert_eq!(w.next(&rbase, &sizes, 0x40000).map(|t| t.2), Some(0x40000));
    }

    #[test]
    fn establish_configures_prioritises_routes_and_enables_and_a_shared_irq_stays_on_its_cpu() {
        let _g = setup();
        let sc = softc(0, &[]);
        let ih = establish(
            sc,
            45,
            IST_EDGE_RISING,
            IPL_BIO,
            Some(1),
            IntrHandler::Raw(record, 0),
        )
        .unwrap();
        assert_eq!(ih.cpu, 1);
        assert_eq!(
            sc.d.read_at::<u32>(gicd_icfgr(45)) & gicd_icfgr_trig_mask(45),
            gicd_icfgr_trig_edge(45)
        );
        assert_eq!(
            sc.d.read_at::<u8>(gicd_ipriorityr(45)),
            agintc_prival(IPL_BIO, 4)
        );
        assert_eq!(
            sc.d.read_at::<u64>(gicd_irouter(45)),
            1,
            "routed to CPU 1's affinity"
        );
        assert_eq!(
            sc.d.read_at::<u32>(gicd_isenabler(45)),
            1 << irq_to_reg32bit(45)
        );
        // On another CPU: refused, nothing linked.
        assert!(
            establish(
                sc,
                45,
                IST_LEVEL_HIGH,
                IPL_NET,
                Some(0),
                IntrHandler::Raw(record, 0)
            )
            .is_none()
        );
        assert_eq!(sc.queue(45).list.iter().count(), 1);
        // No CPU: the primary one; a PPI's trigger is left alone and it is enabled at this
        // CPU's redistributor, its priority there too.
        let me = hal::cpu::cpu_number();
        let ppi = establish(
            sc,
            27,
            IST_EDGE_RISING,
            IPL_CLOCK,
            None,
            IntrHandler::Raw(record, 0),
        )
        .unwrap();
        assert_eq!(ppi.cpu, 0);
        assert_eq!(sc.d.read_at::<u32>(gicd_icfgr(27)), 0);
        let r = sc.redist(me);
        assert_eq!(r.read(RISENABLE0), 1 << 27);
        assert_eq!(
            r.read_at::<u8>(gicr_ipriorityr(27)),
            agintc_prival(IPL_CLOCK, 4)
        );
        // The specifier: SPI 13 is 45, PPI 11 is 27, the trigger from the third cell.
        assert_eq!(agintc_fdt_cells(sc, &[0, 13, 1]), (45, IST_EDGE_RISING));
        assert_eq!(agintc_fdt_cells(sc, &[1, 11, 4]), (27, IST_LEVEL_HIGH));
        let bogus = std::panic::catch_unwind(AssertUnwindSafe(|| agintc_fdt_cells(sc, &[2, 1, 4])));
        assert!(bogus.is_err(), "a third interrupt type is the C's panic");
        let range = std::panic::catch_unwind(AssertUnwindSafe(|| {
            establish(
                sc,
                300,
                IST_LEVEL_HIGH,
                IPL_BIO,
                None,
                IntrHandler::Raw(record, 0),
            )
        }));
        assert!(
            range.is_err(),
            "past the SPIs, below the LPIs: bogus irqnumber"
        );
        // Disestablish: disabled, the record kept and refused twice.
        agintc_intr_disestablish(sc, ih);
        assert_eq!(sc.queue(45).irq_min.load(Ordering::Relaxed), IPL_NONE);
        assert_eq!(
            sc.d.read_at::<u32>(gicd_icenabler(45)),
            1 << irq_to_reg32bit(45)
        );
        assert!(ih.dead.load(Ordering::Relaxed));
        assert!(!List::<IhList>::is_linked(ih));
        let again = std::panic::catch_unwind(AssertUnwindSafe(|| agintc_intr_disestablish(sc, ih)));
        assert!(again.is_err(), "a dead record is refused");
    }

    #[test]
    fn the_dispatch_raises_runs_the_handlers_unmasked_then_ends_and_restores() {
        let _g = setup();
        let sc = softc(0, &[]);
        SEEN.lock().unwrap().clear();
        let word = 0x5000usize;
        let a = establish(
            sc,
            40,
            IST_LEVEL_HIGH,
            IPL_BIO,
            None,
            IntrHandler::Raw(record, 0),
        )
        .unwrap();
        let _b = establish(
            sc,
            40,
            IST_LEVEL_HIGH,
            IPL_BIO,
            None,
            IntrHandler::Raw(record, word),
        )
        .unwrap();
        let _c = establish(
            sc,
            40,
            IST_LEVEL_HIGH,
            IPL_BIO,
            None,
            IntrHandler::Frame(on_frame),
        )
        .unwrap();
        let frame = ClockFrame::default();
        let before = hal::cpu::curcpu().cpl();
        icc_set(|r| {
            r.iar1 = 40;
            r.eoir1 = 0xdead;
        });
        agintc_irq_handler(sc, &frame, &InIrq::host());
        let seen = core::mem::take(&mut *SEEN.lock().unwrap());
        let at = core::ptr::from_ref(&frame).addr();
        let pmr = u64::from(agintc_prival(IPL_BIO, 4));
        assert_eq!(
            seen,
            vec![(at, IPL_BIO, pmr), (word, IPL_BIO, pmr), (at, IPL_BIO, pmr)],
            "a NULL argument and a native frame handler get the frame; all run raised, in order"
        );
        assert_eq!(icc::regs().eoir1, 40, "then the end of interrupt");
        assert_eq!(hal::cpu::curcpu().cpl(), before, "and the level restored");
        assert_eq!(icc::pmr(), u64::from(agintc_prival(before, 4)));
        assert_eq!(a.count.ec_count.load(Ordering::Relaxed), 1);
        // The spurious interrupt is counted and not ended; one past the queues ignored.
        icc_set(|r| {
            r.iar1 = 1023;
            r.eoir1 = 0;
        });
        agintc_irq_handler(sc, &frame, &InIrq::host());
        assert_eq!(sc.spur.ec_count.load(Ordering::Relaxed), 1);
        icc_set(|r| r.iar1 = 300);
        agintc_irq_handler(sc, &frame, &InIrq::host());
        assert_eq!(icc::regs().eoir1, 0);
    }

    #[test]
    fn the_cpu_interface_wakes_the_redistributor_and_routes_the_ipi() {
        let _g = setup();
        let sc = softc(0, &[]);
        let me = hal::cpu::cpu_number();
        let r = sc.redist(me);
        r.write(RWAKER, GICR_WAKER_PROCESSORSLEEP);
        agintc_cpuinit(sc);
        assert_eq!(r.read(RWAKER), 0, "awake");
        assert_eq!(r.read(RICENABLE0), !0);
        assert_eq!(r.read_at::<u32>(gicr_ipriorityr(0)), !0);
        assert_eq!((r.read(RIGROUPR0), r.read(RIGRPMODR0)), (!0, 0));
        let regs = icc::regs();
        assert_eq!((regs.pmr, regs.bpr1, regs.igrpen1), (0xff, 0, 1));
        // The IPI's SGI: memory keeps what is written, so SGI 0 is usable, and restored.
        assert_eq!(agintc_find_ipi(sc, me), Some(0));
        assert_eq!(r.read_at::<u8>(gicr_ipriorityr(0)), 0xff);
        let ipi = establish(
            sc,
            0,
            IST_EDGE_RISING,
            IPL_IPI | IPL_MPSAFE,
            None,
            IntrHandler::Raw(record, 0),
        )
        .unwrap();
        assert!(sc.ipi_irq.set(ipi).is_ok());
        r.write(RISENABLE0, 0);
        agintc_cpuinit(sc);
        assert_eq!(
            r.read(RISENABLE0),
            1,
            "each CPU's start enables the IPI there"
        );
        // `agintc_calc_irq`'s minimum starts at IPL_HIGH, so the IPI (IPL_IPI, above it) is
        // prioritised as IPL_HIGH, as in the C.
        assert_eq!(
            r.read_at::<u8>(gicr_ipriorityr(0)),
            agintc_prival(IPL_HIGH, 4)
        );
    }

    #[test]
    fn the_ipi_posts_its_reason_and_sends_the_sgi_to_the_target_alone() {
        let _g = setup();
        let sc = MArc::new(M_DEVBUF, M_WAITOK, new_softc(288, 0, &[])).unwrap();
        sc.ipi_num.store(3, Ordering::Relaxed);
        let me = hal::cpu::cpu_number();
        let (a, b) = if me < 2 { (2, 3) } else { (0, 1) };
        icc_set(|r| r.sgi1r = 0);
        agintc_send_ipi_sc(&sc, me, ARM_IPI_NOP);
        assert_eq!(icc::regs().sgi1r, 0, "a NOP to oneself is not sent");
        agintc_send_ipi_sc(&sc, b, ARM_IPI_NOP);
        assert_eq!(
            icc::regs().sgi1r,
            agintc_sgi1r(b as u64, 3),
            "to the target alone"
        );
        assert_eq!(
            sc.ipi_reason[b].load(Ordering::Relaxed),
            0,
            "a NOP posts nothing"
        );
        agintc_send_ipi_sc(&sc, a, ARM_IPI_XCALL);
        assert_eq!(sc.ipi_reason[a].load(Ordering::Relaxed), 1 << ARM_IPI_XCALL);
        assert_eq!(icc::regs().sgi1r, agintc_sgi1r(a as u64, 3));
        // The handler takes the CPU's reasons (no ddb, no halt among them) and answers 1.
        assert_eq!(agintc_ipi_run(&sc, a), 1);
        assert_eq!(sc.ipi_reason[a].load(Ordering::Relaxed), 0);
        let h = IntrHandler::Shared(agintc_ipi_handler, MArc::erase(sc.clone()));
        assert_eq!(h.call(&ClockFrame::default()), 1);
    }

    #[test]
    fn the_controller_hooks_route_barrier_and_wakeup_on_the_handlers_cpu() {
        let _g = setup();
        let sc = softc(0, &[]);
        let ih = establish(
            sc,
            50,
            IST_LEVEL_HIGH,
            IPL_BIO,
            Some(1),
            IntrHandler::Raw(record, 0),
        )
        .unwrap();
        let other = establish(
            sc,
            51,
            IST_LEVEL_HIGH,
            IPL_BIO,
            Some(1),
            IntrHandler::Raw(record, 0),
        )
        .unwrap();
        let c = IcCookie::Rec(ih);
        assert_eq!(InterruptController::barrier(sc, c), BarrierWait::Cpu(1));
        sc.d.write_at::<u64>(gicd_irouter(50), 7);
        InterruptController::route(sc, c, false, 0);
        assert_eq!(
            sc.d.read_at::<u64>(gicd_irouter(50)),
            7,
            "a disable does nothing"
        );
        InterruptController::route(sc, c, true, 0);
        assert_eq!(sc.d.read_at::<u64>(gicd_irouter(50)), 0, "routed to CPU 0");
        InterruptController::set_wakeup(sc, c);
        assert_ne!(ih.flags.load(Ordering::Relaxed) & IPL_WAKEUP, 0);
        // Before a sleep, what may not wake the machine is disabled.
        sc.d.write_at::<u32>(gicd_icenabler(50), 0);
        agintc_enable_wakeup(sc);
        assert_eq!(
            sc.d.read_at::<u32>(gicd_icenabler(50)),
            1 << irq_to_reg32bit(51),
            "51 alone"
        );
        assert_eq!(other.flags.load(Ordering::Relaxed) & IPL_WAKEUP, 0);
        // A resume enables both again, with their route.
        sc.d.write_at::<u32>(gicd_isenabler(50), 0);
        sc.d.write_at::<u64>(gicd_irouter(51), 0);
        agintc_restore(sc);
        assert_eq!(
            sc.d.read_at::<u32>(gicd_isenabler(50)),
            1 << irq_to_reg32bit(51),
            "the word's last write: 51's bit"
        );
        assert_eq!(sc.d.read_at::<u64>(gicd_irouter(51)), 1);
        InterruptController::disestablish(sc, c);
        assert!(ih.dead.load(Ordering::Relaxed));
        let word = std::panic::catch_unwind(AssertUnwindSafe(|| {
            InterruptController::barrier(sc, IcCookie::Word(1))
        }));
        assert!(word.is_err(), "a cookie this controller did not answer");
    }

    #[test]
    fn a_message_based_spi_is_claimed_established_and_freed_at_its_disestablish() {
        let _g = setup();
        let sc = softc(0, &[(64, 2)]);
        assert!(InterruptController::has_msi(sc));
        let msi = |addr: &mut u64, data: &mut u64| {
            InterruptController::establish_msi(
                sc,
                addr,
                data,
                IPL_NET,
                None,
                IntrHandler::Raw(record, 0),
                "msi",
            )
        };
        let (mut addr, mut data) = (0, 0);
        let c = msi(&mut addr, &mut data).unwrap();
        assert_eq!((addr, data), (0x0800_0040, 64));
        assert_eq!(
            sc.d.read_at::<u32>(gicd_icfgr(64)) & gicd_icfgr_trig_mask(64),
            gicd_icfgr_trig_edge(64)
        );
        assert!(msi(&mut addr, &mut data).is_some());
        assert_eq!(data, 65);
        assert!(msi(&mut addr, &mut data).is_none(), "the range is full");
        InterruptController::disestablish(sc, c);
        assert!(
            !sc.mbi[0].mbi[0].load(Ordering::Relaxed),
            "the slot is free again"
        );
        // Without ranges the GIC is no MSI controller.
        assert!(!InterruptController::has_msi(softc(0, &[])));
    }

    /// An ITS over fresh registers on `gic`, as `agintc_msi_init` sets it up: physical LPIs,
    /// 16 device ID bits, 8-byte entries, a device table (BASER 0) and a collection table
    /// (BASER 1), one CPU; its queue read up to the first command (the `MAPC`).
    fn new_its(gic: &'static AgintcSoftc) -> &'static AgintcMsiSoftc {
        let m = map::<Gits>(PAGE);
        m.write(STYPER, GITS_TYPER_PHYS | (15 << 13) | (7 << 4));
        m.write_at::<u64>(gits_baser(0), GITS_BASER_TYPE_DEVICE | (7 << 48));
        m.write_at::<u64>(gits_baser(1), GITS_BASER_TYPE_COLL | (7 << 48));
        m.write(SCREADR, GITS_CMD_SIZE as u64);
        let mut xname = [0u8; 16];
        xname[..10].copy_from_slice(b"agintcmsi0");
        let root = FdtBus::root();
        let init = ItsInit {
            m,
            bus: &root,
            msi_addr: 0x0808_0000 + GITS_TRANSLATER,
            msi_delta: 0,
            xname,
        };
        agintc_msi_init(init, gic, 1).unwrap()
    }

    /// The queue's command at `idx`, as four double words.
    fn queued(its: &AgintcMsiSoftc, idx: usize) -> [u64; 4] {
        let q = its.cmdq.lock();
        core::array::from_fn(|k| q.q.buf.read::<u64>(idx * GITS_CMD_SIZE + 8 * k))
    }

    #[test]
    fn the_its_sets_up_its_queue_and_tables_and_maps_a_collection_per_cpu() {
        let _g = setup();
        let gic = softc(8, &[]);
        let its = new_its(gic);
        let cbaser = its.m.read(SCBASER);
        assert_eq!(
            cbaser & !GITS_CBASER_MASK,
            GITS_CBASER_VALID | GITS_CBASER_IC_NORM_NC | 15
        );
        // 64 KB pages stuck, the flat table (512 KB) went indirect: one page of pointers.
        let baser = its.m.read_at::<u64>(gits_baser(0));
        assert_eq!(baser & GITS_BASER_PGSZ_MASK, GITS_BASER_PGSZ_64K);
        assert_ne!(baser & GITS_BASER_INDIRECT, 0);
        assert_ne!(baser & GITS_BASER_VALID, 0);
        assert_eq!(baser & GITS_BASER_SZ_MASK, 0, "one page");
        assert!(its.dtt_indirect);
        assert_eq!(its.deviceid_max, 8192 * 8192 - 1);
        // 16 collection ID bits of 8 bytes: 512 KB, eight 64 KB pages.
        let baser = its.m.read_at::<u64>(gits_baser(1));
        assert_eq!(baser & GITS_BASER_SZ_MASK, 7);
        assert_eq!(its.m.read(SCTLR), GITS_CTLR_ENABLED);
        // CPU 0's collection, at its redistributor's processor.
        assert_eq!(
            queued(its, 0),
            [MAPC as u64, 0, GITS_CMD_VALID | (0x10 << 16), 0]
        );
        assert_eq!(its.m.read(SCWRITER), GITS_CMD_SIZE as u64);
        // An ITS without physical LPIs is refused.
        let m = map::<Gits>(PAGE);
        m.write(STYPER, 0);
        let root = FdtBus::root();
        let init = ItsInit {
            m,
            bus: &root,
            msi_addr: 0,
            msi_delta: 0,
            xname: [0; 16],
        };
        assert!(agintc_msi_init(init, gic, 1).is_none());
    }

    #[test]
    fn an_msi_maps_a_device_an_event_and_an_lpi_and_its_disestablish_discards_them() {
        let _g = setup();
        let gic = softc(8, &[]);
        let its = new_its(gic);
        SEEN.lock().unwrap().clear();
        let msi = |addr: &mut u64, data: &mut u64, h: IntrHandler| {
            InterruptController::establish_msi(its, addr, data, IPL_BIO, None, h, "msi")
        };
        // The device's table and its MAPD (the queue read up to it).
        its.m.write(SCREADR, 2 * GITS_CMD_SIZE as u64);
        let md = {
            let _w = gic.msi_lock.write();
            agintc_msi_find_device(its, 0x10).unwrap()
        };
        let mapd = queued(its, 1);
        assert_eq!(mapd[0], (0x10 << 32) | MAPD as u64);
        assert_eq!(mapd[1], 4, "32 events");
        assert_ne!(mapd[2] & GITS_CMD_VALID, 0);
        assert_ne!(
            its.dtt.as_ref().unwrap().lock().get_u64(0),
            0,
            "the second level"
        );
        // The MSI: event 0 of device 0x10 on LPI 8192, then MAPTI and SYNC.
        its.m.write(SCREADR, 4 * GITS_CMD_SIZE as u64);
        let (mut addr, mut data) = (0, 0x10);
        let c = msi(&mut addr, &mut data, IntrHandler::Raw(record, 0)).unwrap();
        assert_eq!((addr, data), (0x0808_0000 + GITS_TRANSLATER, 0));
        assert_eq!(
            queued(its, 2),
            [(0x10 << 32) | MAPTI as u64, 8192 << 32, 0, 0]
        );
        assert_eq!(queued(its, 3), [SYNC as u64, 0, 0x10 << 16, 0]);
        let li = lpi_info(c);
        assert!(core::ptr::eq(gic.lpi[0].first().unwrap(), li));
        assert_eq!(
            gic.prop.as_ref().unwrap().lock().get_u8(0),
            agintc_prival(IPL_BIO, 4) | GICR_PROP_GROUP1 | GICR_PROP_ENABLE
        );
        assert_eq!(md.events.load(Ordering::Relaxed), 1);
        assert_eq!(InterruptController::barrier(its, c), BarrierWait::Cpu(0));
        // The next MSI of the device takes event 1 and LPI 8193; event 1 asked again: refused.
        its.m.write(SCREADR, 6 * GITS_CMD_SIZE as u64);
        let (mut addr, mut data) = (0, 0x10);
        let c2 = msi(&mut addr, &mut data, IntrHandler::Frame(on_frame)).unwrap();
        assert_eq!(data, 1);
        assert_eq!(lpi_info(c2).ih.irq, LPI_BASE + 1);
        let (mut addr, mut data) = (1, 0x10);
        assert!(msi(&mut addr, &mut data, IntrHandler::Raw(record, 0)).is_none());
        // The LPIs dispatch through their slots, the frame to the native handler.
        let frame = ClockFrame::default();
        icc_set(|r| r.iar1 = (LPI_BASE + 1) as u64);
        agintc_irq_handler(gic, &frame, &InIrq::host());
        assert_eq!(icc::regs().eoir1, (LPI_BASE + 1) as u64);
        let seen = core::mem::take(&mut *SEEN.lock().unwrap());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, core::ptr::from_ref(&frame).addr());
        assert_eq!(seen[0].1, IPL_BIO);
        // An LPI with an empty slot is not ended.
        icc_set(|r| {
            r.iar1 = (LPI_BASE + 5) as u64;
            r.eoir1 = 0;
        });
        agintc_irq_handler(gic, &frame, &InIrq::host());
        assert_eq!(icc::regs().eoir1, 0);
        // Disestablish: the LPI disabled, DISCARD and INV (with their SYNCs), the slot empty,
        // the records kept. The queue is read up to the INV's SYNC (the DISCARD's wait times
        // out on the fake ITS, which reads nothing by itself: one line of output).
        its.m.write(SCREADR, 10 * GITS_CMD_SIZE as u64);
        InterruptController::disestablish(its, c);
        assert_eq!(gic.prop.as_ref().unwrap().lock().get_u8(0), 0);
        assert_eq!(queued(its, 6), [(0x10 << 32) | DISCARD as u64, 0, 0, 0]);
        assert_eq!(queued(its, 8), [(0x10 << 32) | INV as u64, 0, 0, 0]);
        assert!(gic.lpi[0].first().is_none());
        assert!(li.ih.dead.load(Ordering::Relaxed));
        let again = std::panic::catch_unwind(AssertUnwindSafe(|| {
            InterruptController::disestablish(its, c);
        }));
        assert!(again.is_err(), "a dead MSI is refused");
    }

    #[test]
    fn a_table_is_written_back_even_on_a_coherent_bus() {
        // A `dma-coherent` bus (its tag keeps DMA coherent): the table's buffer is still the
        // non-coherent provider's, so each `publish` is the C's write-back (`DmaBuf::sync`
        // of a non-coherent buffer: hal's `a_coherent_sync_touches_no_cache_line_and_the_
        // others_do_the_cs`), and the address is the bus's.
        use crate::dev::ofw::fdt::OF_finddevice;
        use crate::dev::ofw::fdt::testfdt::{FdtBuilder, install};

        let _g = setup();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.begin(b"soc");
        b.prop(b"dma-coherent", b"");
        b.end();
        b.end();
        let _t = install(b.finish());
        let root = FdtBus::root();
        let soc = FdtBus::child(&root, OF_finddevice(b"/soc"), 2, 2);
        assert!(soc.dma_coherent(), "the double: a coherent tag");
        let mut t = agintc_dmamem_alloc(&soc, GITS_CMDQ_SIZE, GITS_CMDQ_SIZE).unwrap();
        assert!(!t.buf.is_coherent(), "written back whatever the tag");
        assert_eq!(t.dva, t.buf.seg().addr, "no dma-ranges: the address itself");
        t.set_u64(3, 0x1234);
        assert_eq!(t.get_u64(3), 0x1234);
    }

    #[test]
    fn the_its_refuses_a_device_past_its_table() {
        let _g = setup();
        let gic = softc(8, &[]);
        let its = new_its(gic);
        assert_eq!(
            agintc_msi_create_device_table(its, its.deviceid_max + 1),
            Err(Errno::ENXIO)
        );
        assert!(agintc_msi_create_device(its, its.deviceid_max + 1).is_none());
    }
}
/* </TESTS> */
