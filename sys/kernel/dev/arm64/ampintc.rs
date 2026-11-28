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
 * Copyright (c) 2007,2009,2011 Dale Rahn <drahn@openbsd.org>
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
//! The ARM generic interrupt controller, version 2, in the kernel: `ampintc(4)` and its GICv2m
//! MSI frames (`ampintcmsi`), `arch/arm64/dev/ampintc.c`. This driver implements the
//! interrupt controller as specified in DDI0407E_cortex_a9_mpcore_r2p0_trm with the
//! IHI0048A_gic_architecture_spec_v1_0 underlying specification.
//!
//! Upstream: sys/arch/arm64/dev/ampintc.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/ampintc.rs@e8249449d905
//!
//! The controller attaches at `fdt` (`ampintc* at fdt? early 1`, mainbus's or a simplebus's
//! node): it maps the distributor and the CPU interface through its bus, resets the
//! distributor, becomes the machine's interrupt controller (`hal::cpu::intr::install`, the
//! C's `arm_set_intr_handler`: `spl(9)` and the IRQ dispatch are its methods), finds the
//! IPI's SGI and establishes it, enables the interfaces, registers with the interrupt core
//! (`arm_intr_register_fdt`) and attaches its children, the GICv2m frames, as a simplebus.
//! A frame (`ampintcmsi* at fdt? early 1`) registers as an MSI controller whose interrupts
//! are SPIs of its parent GIC.
//!
//! ## Deviations
//! - `ampintc_activate` (`DVACT_RESUME`: the priorities, enables and routes written again)
//!   is not here, as in LZ: no suspend or resume runs on arm64 yet.
//! - A handler record is never freed (P12): the C frees `struct intrhand` at its
//!   disestablish; here it is unlinked, its counter detached, marked dead and kept (an
//!   `M_DEVBUF` block per disestablish of a GIC interrupt: `vmstat -m`'s `devbuf` counts
//!   move, which no smoke compares), because the dispatch walks the chain with no lock and a
//!   record must stay readable under a walker on another CPU. The C's race stays, made safe:
//!   such a walker that already read the record calls it once after its disestablish, one
//!   standing on it ends its walk early (the C reads freed memory). A second disestablish of
//!   the same record is `panic(9)`.
//! - The machine's interrupt controller is the first that installs itself: a second GIC's
//!   attach is `panic(9)` where the C's `arm_set_intr_handler` would replace the first's
//!   table (no machine has two).
//! - The primary CPU (`cpu_info_primary`, an establish without a CPU) is the CPU that ran the
//!   attach: mainbus attaches the controller during `cpu_configure` on the boot CPU, which is
//!   the primary one.
//! - `vmstat -m`'s `devbuf` also moves by the handles: an interrupt established through
//!   `bsd`'s adapter on this controller gets a 40-byte native handle (bucket 64) where LZ's
//!   was 16 bytes, and a GICv2m frame keeps the parent's handle by value in its slot (no
//!   `malloc` per MSI).
//! - The dispatch's closing `ampintc_splx(s)` runs the pending soft interrupts with the
//!   machine's interrupts on (`ampintc_splx_in_irq`, `hal::cpu::intr::do_pending_intr_in_irq`);
//!   the C runs them masked, where a non-`MPSAFE` one waiting for the kernel lock that a CPU
//!   in ddb holds never takes `ARM_IPI_DDB` (`machine ddbcpu` then hangs: `hal::cpu::intr`'s
//!   deviations). The level is still restored masked, so the dispatch returns as the C's.
//!
//! ## Redesign
//! - `struct ampintc_softc` (the simplebus softc first, the bus tag and two handles, the
//!   handler queues `mallocarray`ed, the masks, the spurious counter, the embedded `struct
//!   interrupt_controller`) and the global `ampintc` -> [`AmpintcSoftc`], built whole at
//!   attach (the two `Mmio` blocks, the queues as an `MArray`, the masks and IPI reasons
//!   atomics) and kept in [`AMPINTC`], a set-once `MArc`; it implements
//!   `hal::cpu::intr::IntrHardware` (the C's `arm_set_intr_handler` table: `spl(9)` and the
//!   dispatch, called directly on the softc) and `InterruptController` (the C's `sc_ic`
//!   entries). LZ's `Cell`s of a static softc and its `unsafe impl Sync` are gone; so are the
//!   simplebus softc (the children's bus is [`simplebus_attach`]'s) and `ATTACHED`
//!   (`AMPINTC.get()`).
//! - `struct intrhand` -> [`Intrhand`]: immutable once linked (the handler an
//!   [`IntrHandler`], the CPU a number), `MBox::leak`ed at establish and linked with
//!   `hal::ds::list::List::insert_tail` under the softc's mutex and the C's `intr_disable()`;
//!   its cookie is `IcCookie::Rec` (downcast back, never a cast). The dispatch walks
//!   `List::iter` with no lock (the C's `TAILQ_FOREACH`, O(1) a step). See the Deviations.
//! - `struct intrq`'s `iq_ci`, `iq_irq_max`, `iq_irq_min` -> atomics (the dispatch reads
//!   `iq_irq_max` with no lock; establish writes it under the mutex); `iq_ist`, which
//!   nothing in the C reads or writes, is not kept.
//! - The registers: `bus_space_read/write_N(iot, ioh, off)` through the tag -> `Mmio<Gicd>`,
//!   `Mmio<Gicc>` and `Mmio<Gicv2m>` mapped through the bus ([`FdtBus::map`]); a fixed
//!   register (`ICPIAR`, `ICPEOIR`, `ICPIPMR`, `ICD_SGIR`, ...) is a `Reg`, a load or store
//!   with no check; a per-IRQ one (`ICD_ISERn`, `ICD_IPRn`, ...) a computed offset with one
//!   bounds compare (establish, route, the resets: not the dispatch).
//! - `ampintc_run_handler`'s `ih_arg ? ih_arg : frame` -> `IntrHandler::call`; the IPI
//!   handler's softc argument is a counted handle (`IntrHandler::Shared`).
//! - `intr_send_ipi_func = ampintc_send_ipi` -> the kernel's IPI slot
//!   (`kernel::dev::arm64::intr::intr_set_send_ipi`); `db_enter()` and `cpu_halt()` of the
//!   IPI reasons -> `hal::cpu::trap::db_enter` and `cpu_halt` (the `Debugger` and `CpuHooks`
//!   bridges, `bsd`'s ddb and `cpu.c` until they move).
//! - The softc's `splassert` is `IntrHardware`'s default, `hal::cpu::intr::splassert_check`
//!   (the C's generic `arm_splassert_check`, the report through `hal::prf::splassert_fail`
//!   and `bsd`'s `splassert_ctl`): the GICv2 checks what the legacy GICv3's table does.
//! - The dispatch's unmask pair is `hal::cpu::intr::unmasked`, given the dispatch's token
//!   (`InIrq`), which only `hal`'s IRQ entry makes.
//! - `ampintc_intr_barrier`'s `sched_barrier(ih->ih_ci)` -> the CPU to wait for
//!   (`BarrierWait::Cpu`), which the core's `intr_barrier` waits for.
//! - The GICv2m frame: `sc_spi`, a `mallocarray` of `void *` handles (NULL: free) whose
//!   slot's address was the MSI's cookie -> a mutexed `MArray` of slots (free, being taken,
//!   taken with the parent's `IntrHandle` by value), the cookie the slot's index
//!   (`IcCookie::Word`). Its parent is found in the core's registry (`controllers()`, the C's
//!   `LIST_FOREACH` on `interrupt_controllers`), and the doorbell's physical address is the
//!   bus's translation of the frame's `reg` (what the C's `pmap_extract` of the mapping
//!   answers, read without a page table walk).

use core::any::Any;
use core::sync::atomic::{
    AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicUsize, Ordering, fence,
};

use hal::cpu::clock::ClockFrame;
use hal::cpu::intr::{
    IPL_FLAGMASK, IPL_HIGH, IPL_IPI, IPL_IRQMASK, IPL_MPSAFE, IPL_NONE, IPL_SCHED, InIrq,
    IntrHardware,
};
use hal::dev::mmio::{DevCache, Mmio, Reg, RegBlock};
use hal::ds::list::{List, ListEntry};
use hal::mem::malloc::{M_NOWAIT, M_WAITOK, MArc, MArray, MBox};
use hal::mem::pool::AnyArc;
use hal::sync::mutex::Mutexed;
use hal::sync::once::OnceStatic;

use crate::assert::{kassert, kpanic};
use crate::base::prf::{Str, printf};
use crate::dev::arm64::fdt::FdtArgs;
use crate::dev::arm64::intr::{
    ARM_IPI_DDB, ARM_IPI_HALT, ARM_IPI_NOP, BarrierWait, IST_EDGE_RISING, IST_LEVEL_HIGH, IcCookie,
    IcDesc, IcRef, InterruptController, IntrHandle, arm_intr_disestablish_fdt,
    arm_intr_register_fdt, controllers, intr_set_send_ipi,
};
use crate::dev::arm64::simplebus::simplebus_attach;
use crate::dev::device::{
    Aux, DV_DULL, DevRef, Devclass, DriverOps, Ident, M_DEVBUF, Match, UnitTable,
};
use crate::dev::evcount::{Evcount, evcount_attach, evcount_detach};
use crate::dev::intr::IntrHandler;
use crate::dev::ofw::fdt::{OF_getpropint, OF_is_compatible, OF_parent};

// registers
/// `ICD_DCR`: the distributor control register.
pub const ICD_DCR: usize = 0x000;
/// `ICD_DCR_ES`: enable secure.
pub const ICD_DCR_ES: u32 = 0x0000_0001;
/// `ICD_DCR_ENS`: enable non-secure.
pub const ICD_DCR_ENS: u32 = 0x0000_0002;

/// `ICD_ICTR`: the interrupt controller type register.
pub const ICD_ICTR: usize = 0x004;
/// `ICD_ICTR_LSPI_SH`.
pub const ICD_ICTR_LSPI_SH: u32 = 11;
/// `ICD_ICTR_LSPI_M`.
pub const ICD_ICTR_LSPI_M: u32 = 0x1f;
/// `ICD_ICTR_CPU_SH`.
pub const ICD_ICTR_CPU_SH: u32 = 5;
/// `ICD_ICTR_CPU_M`: the CPU count's mask (minus one): at most eight CPUs.
pub const ICD_ICTR_CPU_M: u32 = 0x07;
/// `ICD_ICTR_ITL_SH`.
pub const ICD_ICTR_ITL_SH: u32 = 0;
/// `ICD_ICTR_ITL_M`: the interrupt lines' count (in 32s, minus one).
pub const ICD_ICTR_ITL_M: u32 = 0x1f;
/// `ICD_IDIR`: the distributor implementer identification register.
pub const ICD_IDIR: usize = 0x008;

/// `IRQ_TO_REG32(i)`: the word of a one-bit-per-interrupt bank.
pub const fn irq_to_reg32(i: usize) -> usize {
    (i >> 5) & 0x1f
}
/// `IRQ_TO_REG32BIT(i)`: the bit in it.
pub const fn irq_to_reg32bit(i: usize) -> u32 {
    (i & 0x1f) as u32
}
/// `IRQ_TO_REG16(i)`: the word of a two-bits-per-interrupt bank.
pub const fn irq_to_reg16(i: usize) -> usize {
    (i >> 4) & 0x3f
}
/// `IRQ_TO_REG16BIT(i)`: the field in it.
pub const fn irq_to_reg16bit(i: usize) -> u32 {
    (i & 0xf) as u32
}

/// `ICD_ISRn(i)`: the security bank.
pub const fn icd_isrn(i: usize) -> usize {
    0x080 + irq_to_reg32(i) * 4
}
/// `ICD_ISERn(i)`: set enable.
pub const fn icd_isern(i: usize) -> usize {
    0x100 + irq_to_reg32(i) * 4
}
/// `ICD_ICERn(i)`: clear enable.
pub const fn icd_icern(i: usize) -> usize {
    0x180 + irq_to_reg32(i) * 4
}
/// `ICD_ISPRn(i)`: set pending.
pub const fn icd_isprn(i: usize) -> usize {
    0x200 + irq_to_reg32(i) * 4
}
/// `ICD_ICPRn(i)`: clear pending.
pub const fn icd_icprn(i: usize) -> usize {
    0x280 + irq_to_reg32(i) * 4
}
/// `ICD_ABRn(i)`: active.
pub const fn icd_abrn(i: usize) -> usize {
    0x300 + irq_to_reg32(i) * 4
}
/// `ICD_IPRn(i)`: the interrupt's priority byte.
pub const fn icd_iprn(i: usize) -> usize {
    0x400 + i
}
/// `ICD_IPTRn(i)`: the interrupt's CPU targets byte.
pub const fn icd_iptrn(i: usize) -> usize {
    0x800 + i
}
/// `ICD_ICRn(i)`: the configuration (trigger) bank.
pub const fn icd_icrn(i: usize) -> usize {
    0xc00 + irq_to_reg16(i) * 4
}
/// `ICD_ICR_TRIG_LEVEL(i)`.
pub const fn icd_icr_trig_level(i: usize) -> u32 {
    0x0 << (irq_to_reg16bit(i) * 2)
}
/// `ICD_ICR_TRIG_EDGE(i)`.
pub const fn icd_icr_trig_edge(i: usize) -> u32 {
    0x2 << (irq_to_reg16bit(i) * 2)
}
/// `ICD_ICR_TRIG_MASK(i)`.
pub const fn icd_icr_trig_mask(i: usize) -> u32 {
    0x2 << (irq_to_reg16bit(i) * 2)
}

/// `ICD_PPI`: the PPI status (what about (ppi|spi)_status).
pub const ICD_PPI: usize = 0xd00;
/// `ICD_SPI_BASE`.
pub const ICD_SPI_BASE: usize = 0xd04;
/// `ICD_SGIR`: the software generated interrupt register.
pub const ICD_SGIR: usize = 0xf00;

/// `ICPICR`: the CPU interface control register.
pub const ICPICR: usize = 0x00;
/// `ICPIPMR`: the priority mask.
pub const ICPIPMR: usize = 0x04;
/// `ICMIPMR_SH`: XXX - must left justify bits to 0 - 7.
pub const ICMIPMR_SH: u32 = 4;
/// `ICPBPR`: the binary point.
pub const ICPBPR: usize = 0x08;
/// `ICPIAR`: the interrupt acknowledge register.
pub const ICPIAR: usize = 0x0c;
/// `ICPIAR_IRQ_SH`.
pub const ICPIAR_IRQ_SH: u32 = 0;
/// `ICPIAR_IRQ_M`: the interrupt number.
pub const ICPIAR_IRQ_M: u32 = 0x3ff;
/// `ICPIAR_CPUID_SH`.
pub const ICPIAR_CPUID_SH: u32 = 10;
/// `ICPIAR_CPUID_M`.
pub const ICPIAR_CPUID_M: u32 = 0x7;
/// `ICPIAR_NO_PENDING_IRQ`: the spurious interrupt's number.
pub const ICPIAR_NO_PENDING_IRQ: u32 = ICPIAR_IRQ_M;
/// `ICPEOIR`: the end of interrupt register.
pub const ICPEOIR: usize = 0x10;
/// `ICPPRP`: the running priority.
pub const ICPPRP: usize = 0x14;
/// `ICPHPIR`: the highest pending interrupt.
pub const ICPHPIR: usize = 0x18;
/// `ICPIIR`: the CPU interface identification register.
pub const ICPIIR: usize = 0xfc;

/// `IRQ_ENABLE`.
pub const IRQ_ENABLE: bool = true;
/// `IRQ_DISABLE`.
pub const IRQ_DISABLE: bool = false;

/// GICv2m frame controller for MSI interrupts: `GICV2M_TYPER`.
pub const GICV2M_TYPER: usize = 0x008;
/// `GICV2M_TYPER_SPI_BASE(x)`.
pub const fn gicv2m_typer_spi_base(x: u32) -> u32 {
    (x >> 16) & 0x3ff
}
/// `GICV2M_TYPER_SPI_COUNT(x)`.
pub const fn gicv2m_typer_spi_count(x: u32) -> u32 {
    x & 0x3ff
}
/// `GICV2M_SETSPI_NS`: the doorbell a device writes the SPI number to.
pub const GICV2M_SETSPI_NS: usize = 0x040;

/// The distributor's registers (`sc_d_ioh`): `ICD_PERIPH_ID_*` end the 4 KiB block.
pub struct Gicd;

impl RegBlock for Gicd {
    const SIZE: usize = 0x1000;
}

/// The CPU interface's registers (`sc_p_ioh`), up to `ICPIIR`.
pub struct Gicc;

impl RegBlock for Gicc {
    const SIZE: usize = 0x100;
}

/// A GICv2m frame's registers (`sc_ioh`): one 4 KiB frame.
pub struct Gicv2m;

impl RegBlock for Gicv2m {
    const SIZE: usize = 0x1000;
}

const DCR: Reg<Gicd, u32> = Reg::new(ICD_DCR);
const ICTR: Reg<Gicd, u32> = Reg::new(ICD_ICTR);
const SGIR: Reg<Gicd, u32> = Reg::new(ICD_SGIR);
const CICR: Reg<Gicc, u32> = Reg::new(ICPICR);
const PMR: Reg<Gicc, u32> = Reg::new(ICPIPMR);
const IAR: Reg<Gicc, u32> = Reg::new(ICPIAR);
const EOIR: Reg<Gicc, u32> = Reg::new(ICPEOIR);
const TYPER: Reg<Gicv2m, u32> = Reg::new(GICV2M_TYPER);

/// The number of CPUs a GICv2 targets (`ICD_ICTR_CPU_M + 1`).
const NCPU: usize = ICD_ICTR_CPU_M as usize + 1;

/// `iq_ci == NULL`: a queue that never had a handler.
const NO_CPU: usize = usize::MAX;

/// `struct intrhand`: one established handler. Immutable once linked, never freed (see the
/// Deviations): the dispatch reads it with no lock.
pub struct Intrhand {
    /// `ih_list`: link on intrq list.
    link: ListEntry<Intrhand>,
    /// `ih_func`, `ih_arg`: the handler.
    h: IntrHandler,
    /// `ih_ipl`: `IPL_*`.
    ipl: i32,
    /// `ih_flags`: `IPL_MPSAFE`, `IPL_WAKEUP`.
    flags: i32,
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

/// `struct ampintc_softc`, built whole at attach.
pub struct AmpintcSoftc {
    /// `sc_d_ioh`: the distributor.
    d: Mmio<Gicd>,
    /// `sc_p_ioh`: the CPU interface.
    p: Mmio<Gicc>,
    /// `sc_handler`: one `intrq` per interrupt, `sc_nintr` of them.
    handler: MArray<Intrq>,
    /// `sc_cpu_mask`: each CPU's bit in the target registers.
    cpu_mask: [AtomicU8; NCPU],
    /// `sc_spur`: the spurious interrupt counter.
    spur: Evcount,
    /// `sc_ipi_reason`: per CPU, the `1 << ARM_IPI_*` bits waiting for its IPI handler.
    ipi_reason: [AtomicU32; NCPU],
    /// `sc_ipi_num`: the SGI the IPIs use (set once at attach).
    ipi_num: AtomicI32,
    /// `cpu_info_primary`'s number: the CPU that ran the attach.
    primary: usize,
    /// The device's name, for the panics (`sc_sbus.sc_dev.dv_xname`).
    xname: [u8; 16],
    /// Serialises establish and disestablish (the C's autoconf and kernel lock, beside its
    /// `intr_disable()`); never held across a call out of the driver but `evcount_attach`
    /// and `evcount_detach`.
    lock: Mutexed<()>,
}

impl AmpintcSoftc {
    /// The softc over the two register blocks: `sc_nintr` from `ICD_ICTR` (32 per line
    /// count, plus the 32 SGIs and PPIs), the queues allocated empty
    /// (`mallocarray(nintr, .., M_ZERO | M_NOWAIT)`; `None` when it fails), named `xname`.
    pub fn new(d: Mmio<Gicd>, p: Mmio<Gicc>, xname: &[u8], primary: usize) -> Option<Self> {
        let ictr = d.read(ICTR);
        let nintr = 32 * ((ictr >> ICD_ICTR_ITL_SH) & ICD_ICTR_ITL_M) as usize + 32;
        let handler = MArray::new(M_DEVBUF, M_NOWAIT, nintr, |_| Intrq::new())?;
        let mut name = [0u8; 16];
        let n = xname.len().min(name.len() - 1);
        name[..n].copy_from_slice(&xname[..n]);
        Some(AmpintcSoftc {
            d,
            p,
            handler,
            cpu_mask: [const { AtomicU8::new(0) }; NCPU],
            spur: Evcount::new(),
            ipi_reason: [const { AtomicU32::new(0) }; NCPU],
            ipi_num: AtomicI32::new(0),
            primary,
            xname: name,
            lock: Mutexed::new(IPL_HIGH, ()),
        })
    }

    /// `sc_nintr`.
    pub fn nintr(&self) -> usize {
        self.handler.len()
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
            None => kpanic(format_args!("ampintc: irq {irq} out of range")),
        }
    }

    /// `sc_cpu_mask[cpu]`, the C's `KASSERT(ci->ci_cpuid <= ICD_ICTR_CPU_M)` made a bound.
    fn mask_of(&self, cpu: usize) -> &AtomicU8 {
        match self.cpu_mask.get(cpu) {
            Some(m) => m,
            None => kpanic(format_args!("ampintc: cpu {cpu} beyond the GIC's eight")),
        }
    }
}

/// The handler record behind a cookie this controller answered.
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
        None => kpanic(format_args!("ampintc: not a handler of this controller")),
    }
}

/// A GICv2m frame's slot (`sc_spi[i]`, NULL for free).
#[derive(Clone, Copy)]
enum Spi {
    /// Free.
    Free,
    /// Taken by an establish under way (the parent's establish may sleep, so the slot is
    /// claimed under the mutex and filled after).
    Claimed,
    /// Established on the parent GIC.
    Taken(IntrHandle),
}

/// `struct ampintc_msi_softc`: a GICv2m frame, built whole at attach and kept for ever (its
/// registration names it).
pub struct AmpintcMsiSoftc {
    /// `sc_node`.
    node: i32,
    /// `sc_addr`: the frame's physical address, as the CPU sees it.
    addr: u64,
    /// `sc_bspi`: the first SPI.
    bspi: i32,
    /// `sc_spi`: one slot per SPI, `sc_nspi` of them.
    spi: Mutexed<MArray<Spi>>,
}

/// `ampintc`: the attached controller.
pub static AMPINTC: OnceStatic<MArc<AmpintcSoftc>> = OnceStatic::new();

/// `ampintc_ca` with `ampintc_cd`'s constants (`cfdata[]`'s `ampintc* at fdt? early 1` row
/// names it, `Cfdata::native`).
pub struct AmpintcDriver;

/// The driver of `ampintc*`.
pub static AMPINTC_CA: AmpintcDriver = AmpintcDriver;

/// `ampintc_cd`'s `cd_devs`.
static AMPINTC_UNITS: UnitTable = UnitTable::new();

/// `ampintcmsi_ca` with `ampintcmsi_cd`'s constants.
pub struct AmpintcMsiDriver;

/// The driver of `ampintcmsi*`.
pub static AMPINTCMSI_CA: AmpintcMsiDriver = AmpintcMsiDriver;

/// `ampintcmsi_cd`'s `cd_devs`.
static AMPINTCMSI_UNITS: UnitTable = UnitTable::new();

/// `ampintc_compatibles[]`.
static AMPINTC_COMPATIBLES: [&[u8]; 4] = [
    b"arm,cortex-a7-gic",
    b"arm,cortex-a9-gic",
    b"arm,cortex-a15-gic",
    b"arm,gic-400",
];

/// The device softc both drivers make: the state is [`AMPINTC`]'s and the frame's own,
/// built at attach.
struct GicDev;

/// The softc of a device of either driver.
fn make_softc() -> AnyArc {
    match MArc::new(M_DEVBUF, M_NOWAIT, GicDev) {
        Some(sc) => MArc::erase(sc),
        None => kpanic(format_args!(
            "config_make_softc: allocation for device softc failed"
        )),
    }
}

impl DriverOps for AmpintcDriver {
    fn name(&self) -> &'static [u8] {
        b"ampintc"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &AMPINTC_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        make_softc()
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        FdtArgs::of_aux(aux).map_or(0, ampintc_match)
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let Some(fa) = FdtArgs::of_aux(aux) else {
            kpanic(format_args!("{}: not a device-tree node", dev.xname()));
        };
        ampintc_attach(dev, fa);
    }
}

impl DriverOps for AmpintcMsiDriver {
    fn name(&self) -> &'static [u8] {
        b"ampintcmsi"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &AMPINTCMSI_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        make_softc()
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        FdtArgs::of_aux(aux).map_or(0, ampintc_msi_match)
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let Some(fa) = FdtArgs::of_aux(aux) else {
            kpanic(format_args!("{}: not a device-tree node", dev.xname()));
        };
        ampintc_msi_attach(fa);
    }
}

/// `ampintc_match`: whether the node is a GICv2 this driver drives.
pub fn ampintc_match(fa: &FdtArgs) -> i32 {
    i32::from(
        AMPINTC_COMPATIBLES
            .iter()
            .any(|c| OF_is_compatible(fa.node, c)),
    )
}

/// `bus_space_map(faa->fa_iot, faa->fa_reg[i].addr, faa->fa_reg[i].size, 0, &ioh)`, the
/// C's panic when it fails.
fn map_reg<R: RegBlock>(fa: &FdtArgs, i: usize, what: &str, func: &str) -> Mmio<R> {
    match fa.reg().get(i) {
        Some(r) => match fa.bus.map::<R>(r.addr, r.size, DevCache::Device) {
            Ok(m) => m,
            Err(_) => kpanic(format_args!("{func}: {what}bus_space_map failed!")),
        },
        None => kpanic(format_args!("{func}: {what}bus_space_map failed!")),
    }
}

/// `ampintc_attach`: maps the distributor and CPU interface, resets the controller, takes
/// over `spl` and the IRQ dispatch, sets up the IPI, and registers with the device tree.
pub fn ampintc_attach(dev: &DevRef, fa: &FdtArgs) {
    hal::cpu::intr::init_smask();

    // First row: ICD
    let d = map_reg::<Gicd>(fa, 0, "ICD ", "ampintc_attach");
    // Second row: ICP
    let p = map_reg::<Gicc>(fa, 1, "ICP ", "ampintc_attach");

    let Some(sc) = AmpintcSoftc::new(d, p, dev.xname().as_bytes(), hal::cpu::cpu_number()) else {
        kpanic(format_args!(
            "ampintc_attach: no memory for the handler queues"
        ));
    };
    let Some(sc) = MArc::new(M_DEVBUF, M_WAITOK, sc) else {
        kpanic(format_args!("ampintc_attach: no malloc(9)"));
    };
    if AMPINTC.set(sc).is_err() {
        kpanic(format_args!("ampintc_attach: a second GIC"));
    }
    let Some(arc) = AMPINTC.get() else {
        kpanic(format_args!("ampintc_attach: the softc went"));
    };
    let sc: &'static AmpintcSoftc = arc;

    evcount_attach(&sc.spur, "irq1023/spur", None);

    let ictr = sc.d.read(ICTR);
    let ncpu = ((ictr >> ICD_ICTR_CPU_SH) & ICD_ICTR_CPU_M) + 1;
    printf(format_args!(" nirq {}, ncpu {}", sc.nintr(), ncpu));

    let me = hal::cpu::cpu_number();
    kassert!(me <= ICD_ICTR_CPU_M as usize);
    sc.mask_of(me)
        .store(sc.d.read_at::<u8>(icd_iptrn(0)), Ordering::Relaxed);

    ampintc_init(sc);

    // software reset of the part?
    // set protection bit (kernel only)?
    // XXX - check power saving bit

    sc.setipl(IPL_HIGH); // XXX ???
    ampintc_calc_mask(sc);

    // insert self as interrupt handler
    if hal::cpu::intr::install(sc).is_err() {
        kpanic(format_args!(
            "ampintc_attach: another interrupt controller attached"
        ));
    }

    // setup IPI interrupts
    let Some(ipiirq) = ampintc_find_ipi(sc) else {
        kpanic(format_args!("no irq available for IPI"));
    };
    printf(format_args!(" ipi {ipiirq}"));
    let _ = ampintc_intr_establish(
        sc,
        ipiirq,
        IST_EDGE_RISING,
        IPL_IPI | IPL_MPSAFE,
        None,
        IntrHandler::Shared(ampintc_ipi_handler, MArc::erase(arc.clone())),
        Some("ipi"),
    );
    sc.ipi_num.store(ipiirq as i32, Ordering::Relaxed);
    intr_set_send_ipi(ampintc_send_ipi);

    // enable interrupts
    sc.d.write(DCR, 3);
    sc.p.write(CICR, 1);
    hal::cpu::intr_enable();

    arm_intr_register_fdt(IcDesc {
        node: fa.node,
        gic_its_id: 0,
        ops: sc,
    });

    // attach GICv2M frame controller
    let _ = simplebus_attach(dev, fa);
}

/// The SGI the IPIs use: the first whose priority byte holds a written value (an unusable
/// one reads back unchanged), restored as found (it is set when used).
fn ampintc_find_ipi(sc: &AmpintcSoftc) -> Option<usize> {
    for i in 0..16 {
        let oldreg = sc.d.read_at::<u8>(icd_iprn(i));
        sc.d.write_at::<u8>(icd_iprn(i), oldreg ^ 0x20);

        // if this interrupt is not usable, route will be zero
        let reg = sc.d.read_at::<u8>(icd_iprn(i));
        if reg == oldreg {
            continue;
        }

        // return to original value, will be set when used
        sc.d.write_at::<u8>(icd_iprn(i), oldreg);
        return Some(i);
    }
    None
}

/// `ampintc_init`: disable all interrupts, clear all pending.
fn ampintc_init(sc: &AmpintcSoftc) {
    let nintr = sc.nintr();

    for i in 0..nintr / 32 {
        sc.d.write_at::<u32>(icd_icern(i * 32), !0);
        sc.d.write_at::<u32>(icd_icprn(i * 32), !0);
    }
    for i in 0..nintr {
        // lowest priority ??
        sc.d.write_at::<u8>(icd_iprn(i), 0xff);
        // target no cpus
        sc.d.write_at::<u8>(icd_iptrn(i), 0);
    }
    for i in 2..nintr / 16 {
        // irq 32 - N
        sc.d.write_at::<u32>(icd_icrn(i * 16), 0);
    }
}

/// `ampintc_set_priority`: we only use 16 (13 really) interrupt priorities, and a CPU is
/// only required to implement bit 4-7 of each field so shift into the top bits. Also low
/// values are higher priority thus `IPL_HIGH - pri`.
fn ampintc_set_priority(sc: &AmpintcSoftc, irq: usize, pri: i32) {
    let prival = ((IPL_HIGH - pri) as u32) << ICMIPMR_SH;
    sc.d.write_at::<u8>(icd_iprn(irq), prival as u8);
}

/// `ampintc_intr_enable`.
fn ampintc_intr_enable(sc: &AmpintcSoftc, irq: usize) {
    sc.d.write_at::<u32>(icd_isern(irq), 1 << irq_to_reg32bit(irq));
}

/// `ampintc_intr_disable`.
fn ampintc_intr_disable(sc: &AmpintcSoftc, irq: usize) {
    sc.d.write_at::<u32>(icd_icern(irq), 1 << irq_to_reg32bit(irq));
}

/// `ampintc_intr_config`: level or rising-edge trigger.
fn ampintc_intr_config(sc: &AmpintcSoftc, irqno: usize, type_: i32) {
    let mut ctrl = sc.d.read_at::<u32>(icd_icrn(irqno));
    ctrl &= !icd_icr_trig_mask(irqno);
    if type_ == IST_EDGE_RISING {
        ctrl |= icd_icr_trig_edge(irqno);
    } else {
        ctrl |= icd_icr_trig_level(irqno);
    }
    sc.d.write_at::<u32>(icd_icrn(irqno), ctrl);
}

/// `ampintc_calc_mask`: recomputes every interrupt's priority and enable.
fn ampintc_calc_mask(sc: &AmpintcSoftc) {
    for irq in 0..sc.nintr() {
        ampintc_calc_irq(sc, irq);
    }
}

/// `ampintc_calc_irq`: one interrupt's priority and enable from its handlers.
fn ampintc_calc_irq(sc: &AmpintcSoftc, irq: usize) {
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

    // Enable interrupts at lower levels, clear -> enable
    // Set interrupt priority/enable
    if min == IPL_NONE {
        ampintc_intr_disable(sc, irq);
        if let Some(cpu) = cpu {
            ampintc_route(sc, irq, IRQ_DISABLE, cpu);
        }
    } else {
        ampintc_set_priority(sc, irq, min);
        ampintc_intr_enable(sc, irq);
        if let Some(cpu) = cpu {
            ampintc_route(sc, irq, IRQ_ENABLE, cpu);
        }
    }
}

/// `ampintc_route`: targets (or untargets) `irq` at CPU `cpu`.
fn ampintc_route(sc: &AmpintcSoftc, irq: usize, enable: bool, cpu: usize) {
    let mask = sc.mask_of(cpu).load(Ordering::Relaxed);

    let mut val = sc.d.read_at::<u8>(icd_iptrn(irq));
    if enable == IRQ_ENABLE {
        val |= mask;
    } else {
        val &= !mask;
    }
    sc.d.write_at::<u8>(icd_iptrn(irq), val);
}

/// `ampintc_cpuinit`: a CPU learns its target mask and routes its interrupts.
fn ampintc_cpuinit(sc: &AmpintcSoftc) {
    let me = hal::cpu::cpu_number();
    let mask = sc.mask_of(me);

    // XXX - this is the only cpu specific call to set this
    if mask.load(Ordering::Relaxed) == 0 {
        for i in 0..32 {
            let cpumask = sc.d.read_at::<u8>(icd_iptrn(i));
            if cpumask != 0 {
                mask.store(cpumask, Ordering::Relaxed);
                break;
            }
        }
    }

    if mask.load(Ordering::Relaxed) == 0 {
        kpanic(format_args!("could not determine cpu target mask"));
    }

    for irq in 0..sc.nintr() {
        let iq = sc.queue(irq);
        if iq.cpu() != Some(me) {
            continue;
        }
        // IRQ_ENABLE when it has handlers, IRQ_DISABLE otherwise.
        let enable = iq.irq_min.load(Ordering::Relaxed) != IPL_NONE;
        ampintc_route(sc, irq, enable, me);
    }

    // If a secondary CPU is turned off from an IPI handler and the GIC did not go through a
    // full reset (for example when we fail to suspend) the IPI might still be active. So
    // signal EOI here to make sure new interrupts will be serviced.
    sc.p.write(EOIR, sc.ipi_num.load(Ordering::Relaxed) as u32);
}

/// `ampintc_route_irq`: the `ic_route` hook.
fn ampintc_route_irq(sc: &AmpintcSoftc, ih: &Intrhand, enable: bool, cpu: usize) {
    sc.p.write(CICR, 1);
    sc.d.write_at::<u32>(icd_icrn(ih.irq), 0);
    if enable {
        ampintc_set_priority(sc, ih.irq, sc.queue(ih.irq).irq_min.load(Ordering::Relaxed));
        ampintc_intr_enable(sc, ih.irq);
    }

    ampintc_route(sc, ih.irq, enable, cpu);
}

/// `ampintc_run_handler`: one handler, with its argument or the frame, under the kernel
/// lock unless it is `IPL_MPSAFE` or the interrupted level is at least `IPL_SCHED`.
#[inline]
fn ampintc_run_handler(ih: &Intrhand, frame: &ClockFrame, s: i32) {
    let need_lock = ih.flags & IPL_MPSAFE == 0 && s < IPL_SCHED;
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

/// `ampintc_irq_handler`: the IRQ dispatcher `arm_cpu_irq` calls: acknowledge, raise to the
/// interrupt's level, run its handlers with the machine's interrupts on, end it, restore.
pub fn ampintc_irq_handler(sc: &AmpintcSoftc, frame: &ClockFrame, irq_: &InIrq) {
    let iack_val = sc.p.read(IAR);
    // DEBUG_INTC: not configured.
    let irq = (iack_val & ICPIAR_IRQ_M) as usize;

    if irq == 1023 {
        sc.spur.ec_count.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let Some(iq) = sc.handler.get(irq) else {
        return;
    };

    let pri = iq.irq_max.load(Ordering::Relaxed);
    let s = sc.splraise(pri);
    hal::cpu::intr::unmasked(irq_, || {
        for ih in &iq.list {
            ampintc_run_handler(ih, frame, s);
        }
    });
    sc.p.write(EOIR, iack_val);

    ampintc_splx_in_irq(sc, irq_, s);
}

/// The dispatch's `ampintc_splx(s)`: the soft interrupts pending below the interrupt's level
/// run with the machine's interrupts on (`hal::cpu::intr::do_pending_intr_in_irq`, the
/// module's deviations), then the level is restored, masked.
fn ampintc_splx_in_irq(sc: &AmpintcSoftc, irq: &InIrq, s: i32) {
    if hal::cpu::curcpu().ipending() & hal::cpu::intr::smask(s) != 0 {
        hal::cpu::intr::do_pending_intr_in_irq(irq, s);
    }
    sc.setipl(s);
}

/// `ampintc_intr_establish_fdt`'s reading of the specifier: 1st cell contains type: 0 SPI
/// (32-X), 1 PPI (16-31); 2nd cell contains the interrupt number; 3rd the trigger. The
/// interrupt and its type (`IST_*`).
fn ampintc_fdt_cells(sc: &AmpintcSoftc, cell: &[u32]) -> (usize, i32) {
    let [kind, num, flags, ..] = *cell else {
        kpanic(format_args!("{}: bogus interrupt type", sc.xname()));
    };
    // 2nd cell contains the interrupt number
    let mut irq = num as usize;

    // 1st cell contains type: 0 SPI (32-X), 1 PPI (16-31)
    match kind {
        0 => irq += 32,
        1 => irq += 16,
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

/// `ampintc_intr_establish_fdt`: the `ic_establish` hook.
fn ampintc_intr_establish_fdt(
    sc: &AmpintcSoftc,
    cell: &[u32],
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: &'static str,
) -> Option<IcCookie> {
    let (irq, type_) = ampintc_fdt_cells(sc, cell);
    let ih = ampintc_intr_establish(sc, irq, type_, level, cpu, h, Some(name))?;
    Some(IcCookie::Rec(ih))
}

/// `ampintc_intr_establish`: registers `h` for `irqno` at `level` on CPU `cpu` (`None`: the
/// primary one); `None` when the interrupt already runs on another CPU.
pub fn ampintc_intr_establish(
    sc: &AmpintcSoftc,
    irqno: usize,
    type_: i32,
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: Option<&'static str>,
) -> Option<&'static Intrhand> {
    if irqno >= sc.nintr() {
        kpanic(format_args!(
            "ampintc_intr_establish: bogus irqnumber {irqno}: {}",
            name.unwrap_or("")
        ));
    }

    let cpu = cpu.unwrap_or(sc.primary);

    let type_ = if irqno < 16 {
        // SGI are only EDGE
        IST_EDGE_RISING
    } else if irqno < 32 {
        // PPI are only LEVEL
        IST_LEVEL_HIGH
    } else {
        type_
    };

    let Some(ih) = MBox::new(
        M_DEVBUF,
        M_WAITOK,
        Intrhand {
            link: ListEntry::new(),
            h,
            ipl: level & IPL_IRQMASK,
            flags: level & IPL_FLAGMASK,
            irq: irqno,
            count: Evcount::new(),
            name,
            cpu,
            dead: AtomicBool::new(false),
        },
    ) else {
        kpanic(format_args!("ampintc_intr_establish: no malloc(9)"));
    };

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

    // DEBUG_INTC: not configured.

    ampintc_intr_config(sc, irqno, type_);
    ampintc_calc_mask(sc);

    drop(psw);
    drop(g);
    Some(ih)
}

/// `ampintc_intr_disestablish`: the `ic_disestablish` hook; the record is kept (see the
/// Deviations), a second disestablish of it `panic(9)`.
fn ampintc_intr_disestablish(sc: &AmpintcSoftc, ih: &'static Intrhand) {
    // DEBUG_INTC: not configured.
    if ih.dead.swap(true, Ordering::AcqRel) {
        kpanic(format_args!(
            "ampintc_intr_disestablish: irq {} disestablished twice",
            ih.irq
        ));
    }

    let g = sc.lock.lock();
    let psw = hal::cpu::intr_disable();

    sc.queue(ih.irq).list.remove(ih);
    if ih.name.is_some() {
        evcount_detach(&ih.count);
    }

    ampintc_calc_mask(sc);

    drop(psw);
    drop(g);
}

impl IntrHardware for AmpintcSoftc {
    /// `ampintc_splraise`: `setipl` must always be called because there is a race window
    /// where the variable is updated before the mask is set an interrupt occurs in that
    /// window without the mask always being set, the hardware might not get updated on the
    /// next `splraise` completely messing up spl protection.
    #[inline]
    fn splraise(&self, new: i32) -> i32 {
        let old = hal::cpu::curcpu().cpl();
        self.setipl(new.max(old));
        old
    }

    /// `ampintc_spllower`.
    #[inline]
    fn spllower(&self, new: i32) -> i32 {
        let old = hal::cpu::curcpu().cpl();
        self.splx(new);
        old
    }

    /// `ampintc_splx`.
    #[inline]
    fn splx(&self, new: i32) {
        if hal::cpu::curcpu().ipending() & hal::cpu::intr::smask(new) != 0 {
            hal::cpu::intr::do_pending_intr(new);
        }
        self.setipl(new);
    }

    /// `ampintc_setipl`: the level in `ci_cpl` and in the CPU interface's priority mask (low
    /// values are higher priority, thus `IPL_HIGH - new`); the disable is only to keep the
    /// hardware in sync with `ci_cpl`.
    #[inline]
    fn setipl(&self, new: i32) {
        let _psw = hal::cpu::intr_disable();
        hal::cpu::curcpu().set_cpl(new);
        self.p.write(PMR, ((IPL_HIGH - new) as u32) << ICMIPMR_SH);
    }

    /// `ampintc_irq_handler`, as `arm_irq_dispatch`.
    fn dispatch_irq(&self, frame: &mut ClockFrame, irq: &InIrq) {
        ampintc_irq_handler(self, frame, irq);
    }
}

impl InterruptController for AmpintcSoftc {
    fn establish(
        &self,
        cell: &[u32],
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        ampintc_intr_establish_fdt(self, cell, level, cpu, h, name)
    }

    fn disestablish(&self, ih: IcCookie) {
        ampintc_intr_disestablish(self, hand(ih));
    }

    fn route(&self, ih: IcCookie, enable: bool, cpu: usize) {
        ampintc_route_irq(self, hand(ih), enable, cpu);
    }

    fn cpu_enable(&self) {
        ampintc_cpuinit(self);
    }

    /// `ampintc_intr_barrier`: `sched_barrier(ih->ih_ci)`.
    fn barrier(&self, ih: IcCookie) -> BarrierWait {
        BarrierWait::Cpu(hand(ih).cpu)
    }
}

/// `ampintc_msi_match`.
pub fn ampintc_msi_match(fa: &FdtArgs) -> i32 {
    i32::from(OF_is_compatible(fa.node, b"arm,gic-v2m-frame"))
}

/// `ampintc_msi_attach`: maps the frame, finds its SPIs and registers it as an MSI
/// controller.
pub fn ampintc_msi_attach(fa: &FdtArgs) {
    let m = map_reg::<Gicv2m>(fa, 0, "", "ampintc_msi_attach");

    // XXX: Hack to retrieve the physical address (from a CPU PoV): the bus's translation of
    // the frame's address, which the mapping just made is of.
    let Some(addr) = fa
        .reg()
        .first()
        .and_then(|r| fa.bus.translate(r.addr, r.size).ok())
    else {
        printf(format_args!(": cannot retrieve msi addr\n"));
        return;
    };

    let typer = m.read(TYPER);
    let bspi = OF_getpropint(fa.node, b"arm,msi-base-spi", gicv2m_typer_spi_base(typer)) as i32;
    let nspi = OF_getpropint(fa.node, b"arm,msi-num-spis", gicv2m_typer_spi_count(typer)) as i32;

    printf(format_args!(": nspi {nspi}\n"));

    let Some(spi) = MArray::new(
        M_DEVBUF,
        M_WAITOK,
        usize::try_from(nspi).unwrap_or(0),
        |_| Spi::Free,
    ) else {
        kpanic(format_args!("ampintc_msi_attach: out of memory"));
    };
    let Some(sc) = MBox::new(
        M_DEVBUF,
        M_WAITOK,
        AmpintcMsiSoftc {
            node: fa.node,
            addr: addr.as_usize() as u64,
            bspi,
            spi: Mutexed::new(IPL_HIGH, spi),
        },
    ) else {
        kpanic(format_args!("ampintc_msi_attach: no malloc(9)"));
    };

    arm_intr_register_fdt(IcDesc {
        node: fa.node,
        gic_its_id: 0,
        ops: MBox::leak(sc),
    });
}

impl AmpintcMsiSoftc {
    /// The first free slot, claimed (`sc_spi[i] == NULL`).
    fn claim(&self) -> Option<usize> {
        let mut g = self.spi.lock();
        let i = g.iter().position(|s| matches!(s, Spi::Free))?;
        if let Some(s) = g.get_mut(i) {
            *s = Spi::Claimed;
        }
        Some(i)
    }

    /// `sc_spi[i] = v`.
    fn set(&self, i: usize, v: Spi) {
        let mut g = self.spi.lock();
        if let Some(s) = g.get_mut(i) {
            *s = v;
        }
    }

    /// The parent's handle in slot `i`, when one was established there.
    fn taken(&self, i: usize) -> Option<IntrHandle> {
        match self.spi.lock().get(i) {
            Some(Spi::Taken(h)) => Some(*h),
            _ => None,
        }
    }
}

/// The slot index a frame's cookie is.
fn slot(ih: IcCookie) -> usize {
    match ih {
        IcCookie::Word(i) => i,
        IcCookie::Rec(_) => kpanic(format_args!("ampintcmsi: not a slot of this frame")),
    }
}

impl InterruptController for AmpintcMsiSoftc {
    /// `ampintc_intr_establish_msi`: takes a free SPI, establishes it on the parent GIC
    /// (edge-triggered) and hands back the doorbell and the SPI number as the MSI's address
    /// and data. The cookie is the SPI's slot.
    fn establish_msi(
        &self,
        addr: &mut u64,
        data: &mut u64,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        let parent = OF_parent(self.node);
        let ic = controllers().find(|ic| ic.node() == parent)?;
        self.establish_msi_on(ic, addr, data, level, cpu, h, name)
    }

    fn has_msi(&self) -> bool {
        true
    }

    /// `ampintc_intr_disestablish_msi`: the parent's handle disestablished, then the slot
    /// freed.
    fn disestablish(&self, ih: IcCookie) {
        let i = slot(ih);
        if let Some(h) = self.taken(i) {
            arm_intr_disestablish_fdt(h);
        }
        self.set(i, Spi::Free);
    }

    /// `ampintc_intr_barrier_msi`: the parent's barrier for the handle in the slot.
    fn barrier(&self, ih: IcCookie) -> BarrierWait {
        match self.taken(slot(ih)) {
            Some(h) => h.controller().ops().barrier(h.cookie()),
            None => BarrierWait::Nothing,
        }
    }
}

impl AmpintcMsiSoftc {
    /// [`AmpintcMsiSoftc::establish_msi`] on the parent GIC `ic`: a free SPI claimed and
    /// established there, the doorbell and the SPI number handed back.
    #[allow(clippy::too_many_arguments)] // the C's signature, the parent beside it
    fn establish_msi_on(
        &self,
        ic: IcRef,
        addr: &mut u64,
        data: &mut u64,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        let i = self.claim()?;
        let cells = [
            0,                                  // SPI
            (self.bspi + i as i32 - 32) as u32, // the SPI's number
            1,                                  // Edge-Rising
        ];
        let Some(ih) = ic.establish(&cells, level, cpu, h, name) else {
            self.set(i, Spi::Free);
            return None;
        };

        *addr = self.addr + GICV2M_SETSPI_NS as u64;
        *data = (self.bspi + i as i32) as u64;
        self.set(i, Spi::Taken(ih));
        Some(IcCookie::Word(i))
    }
}

/// `ampintc_ipi_ddb`: another CPU entered ddb.
fn ampintc_ipi_ddb() -> i32 {
    // XXX
    hal::cpu::trap::db_enter();
    1
}

/// `ampintc_ipi_halt`: stop this CPU (`cpu_halt`).
fn ampintc_ipi_halt() -> i32 {
    hal::cpu::trap::cpu_halt();
    1
}

/// `ampintc_ipi_handler`: the IPI's interrupt handler, at `IPL_IPI`, without the kernel lock:
/// runs the reasons posted for this CPU (`ARM_IPI_NOP` posts none; the interrupt itself was
/// the point). Its argument is the softc's counted handle.
fn ampintc_ipi_handler(v: &AnyArc) -> i32 {
    let Some(sc) = v.downcast_ref::<AmpintcSoftc>() else {
        kpanic(format_args!("ampintc_ipi_handler: not the softc"));
    };
    ampintc_ipi_run(sc, hal::cpu::cpu_number())
}

/// [`ampintc_ipi_handler`]'s work for CPU `cpu`: its reasons taken, each run.
fn ampintc_ipi_run(sc: &AmpintcSoftc, cpu: usize) -> i32 {
    if let Some(reason) = sc.ipi_reason.get(cpu)
        && reason.load(Ordering::Relaxed) != 0
    {
        let reasons = reason.swap(0, Ordering::AcqRel);
        if reasons & (1 << ARM_IPI_DDB) != 0 {
            ampintc_ipi_ddb();
        }
        if reasons & (1 << ARM_IPI_HALT) != 0 {
            ampintc_ipi_halt();
        }
        // ARM_IPI_XCALL: NXCALL is 0 on arm64 (arm_cpu_xcall_dispatch).
    }

    1
}

/// `ampintc_send_ipi`: `intr_send_ipi_func`: posts `reason` for CPU `cpu` and raises the
/// IPI's SGI at it alone.
fn ampintc_send_ipi(cpu: usize, reason: i32) {
    if let Some(sc) = AMPINTC.get() {
        ampintc_send_ipi_sc(sc, cpu, reason);
    }
}

/// [`ampintc_send_ipi`] on `sc`.
fn ampintc_send_ipi_sc(sc: &AmpintcSoftc, cpu: usize, reason: i32) {
    if reason == ARM_IPI_NOP {
        if cpu == hal::cpu::cpu_number() {
            return;
        }
    } else if let Some(r) = sc.ipi_reason.get(cpu) {
        r.fetch_or(1 << reason, Ordering::Release);
    }

    // currently will only send to one cpu
    let mut sendmask = u32::from(sc.mask_of(cpu).load(Ordering::Relaxed)) << 16;
    sendmask |= sc.ipi_num.load(Ordering::Relaxed) as u32;

    // The reason (and whatever the sender prepared) before the interrupt.
    fence(Ordering::SeqCst);
    sc.d.write(SGIR, sendmask);
}

/// The number of IPIs the IPI handler has run, on every CPU (its event counter): what the
/// `qemu` self-check of `cpu_boot_secondary_processors` watches; 0 before the controller
/// attached.
pub fn ampintc_ipi_count() -> u64 {
    let Some(sc) = AMPINTC.get() else {
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
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The distributor and the CPU interface are pages of host memory mapped with
    // `km::map_device`: a register reads back what was last written, which is what the C
    // writes and reads; a test writes the "hardware's" answers (ICTR, IAR) itself. The
    // handlers are established without a name (no `evcount_attach`: the counters' list is
    // the process's, which the evcount tests count).

    use core::panic::AssertUnwindSafe;
    use std::boxed::Box;
    use std::sync::{Mutex, MutexGuard};
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use hal::cpu::intr::{IPL_BIO, IPL_CLOCK, IPL_NET};
    use sysdefs::base::types::Paddr;

    use super::*;
    use crate::dev::arm64::intr::ARM_IPI_XCALL;
    use crate::testutil;

    const PAGE: usize = 4096;

    fn device_frame() -> Paddr {
        #[repr(C, align(4096))]
        struct DevPage([u8; PAGE]);
        let p = Box::into_raw(Box::new(DevPage([0; PAGE])));
        Paddr::new(p.expose_provenance())
    }

    fn setup() -> MutexGuard<'static, ()> {
        let g = testutil::pages_install(4 << 20);
        testutil::kernel_maps_install();
        g
    }

    /// A softc over fresh register pages whose `ICD_ICTR` says 288 interrupts and 2 CPUs,
    /// every CPU's target mask 1.
    fn new_softc(name: &[u8]) -> AmpintcSoftc {
        let d: Mmio<Gicd> =
            crate::vm::km::map_device(device_frame(), PAGE, DevCache::Device).unwrap();
        let p: Mmio<Gicc> =
            crate::vm::km::map_device(device_frame(), PAGE, DevCache::Device).unwrap();
        d.write(ICTR, 8 | (1 << ICD_ICTR_CPU_SH));
        // The primary CPU is 0 (a host thread's own number may be past the GIC's eight).
        let sc = AmpintcSoftc::new(d, p, name, 0).unwrap();
        for c in &sc.cpu_mask {
            c.store(1, Ordering::Relaxed);
        }
        sc
    }

    fn softc() -> &'static AmpintcSoftc {
        Box::leak(Box::new(new_softc(b"ampintc0")))
    }

    /// The softc the dispatch test's handler reads its registers from.
    static DISPATCHED: Mutex<Option<&'static AmpintcSoftc>> = Mutex::new(None);
    /// What the handler saw: its argument, the level, the mask, the EOI register.
    static SEEN: Mutex<Vec<(usize, i32, u32, u32)>> = Mutex::new(Vec::new());

    fn record(arg: *mut core::ffi::c_void) -> i32 {
        if let Some(sc) = *DISPATCHED.lock().unwrap() {
            SEEN.lock().unwrap().push((
                arg.addr(),
                hal::cpu::curcpu().cpl(),
                sc.p.read(PMR),
                sc.p.read(EOIR),
            ));
        }
        1
    }

    fn establish(
        sc: &'static AmpintcSoftc,
        irq: usize,
        type_: i32,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
    ) -> Option<&'static Intrhand> {
        ampintc_intr_establish(sc, irq, type_, level, cpu, h, None)
    }

    fn pmr_of(ipl: i32) -> u32 {
        ((IPL_HIGH - ipl) as u32) << ICMIPMR_SH
    }

    #[test]
    fn the_type_register_sizes_the_queues_and_init_resets_every_interrupt() {
        let _g = setup();
        let sc = softc();
        assert_eq!(sc.nintr(), 288);
        sc.d.write_at::<u8>(icd_iprn(40), 0x12);
        sc.d.write_at::<u8>(icd_iptrn(40), 0x3);
        sc.d.write_at::<u32>(icd_icrn(40), 0x5);
        ampintc_init(sc);
        assert_eq!(sc.d.read_at::<u32>(icd_icern(64)), !0);
        assert_eq!(sc.d.read_at::<u32>(icd_icprn(256)), !0);
        assert_eq!(sc.d.read_at::<u8>(icd_iprn(40)), 0xff);
        assert_eq!(sc.d.read_at::<u8>(icd_iptrn(40)), 0);
        assert_eq!(sc.d.read_at::<u32>(icd_icrn(40)), 0);
        // The IPI's SGI: memory keeps what is written, so SGI 0 is usable, and restored.
        sc.d.write_at::<u8>(icd_iprn(0), 0x40);
        assert_eq!(ampintc_find_ipi(sc), Some(0));
        assert_eq!(sc.d.read_at::<u8>(icd_iprn(0)), 0x40);
        // The specifier: SPI 13 is 45, PPI 11 is 27, the trigger from the third cell.
        assert_eq!(ampintc_fdt_cells(sc, &[0, 13, 1]), (45, IST_EDGE_RISING));
        assert_eq!(ampintc_fdt_cells(sc, &[1, 11, 4]), (27, IST_LEVEL_HIGH));
        let bogus =
            std::panic::catch_unwind(AssertUnwindSafe(|| ampintc_fdt_cells(sc, &[2, 1, 4])));
        assert!(bogus.is_err(), "a third interrupt type is the C's panic");
        let short = std::panic::catch_unwind(AssertUnwindSafe(|| ampintc_fdt_cells(sc, &[0, 1])));
        assert!(short.is_err(), "and so is a short specifier");
    }

    #[test]
    fn establish_configures_prioritises_enables_and_routes_and_a_shared_irq_stays_on_its_cpu() {
        let _g = setup();
        let sc = softc();
        let ih = establish(
            sc,
            45,
            IST_EDGE_RISING,
            IPL_BIO,
            None,
            IntrHandler::Raw(record, 0),
        )
        .unwrap();
        assert_eq!(ih.cpu, 0, "no CPU: the primary one, the attach's");
        assert_eq!(
            sc.d.read_at::<u32>(icd_icrn(45)) & icd_icr_trig_mask(45),
            icd_icr_trig_edge(45)
        );
        let iq = sc.queue(45);
        assert_eq!(
            (
                iq.irq_max.load(Ordering::Relaxed),
                iq.irq_min.load(Ordering::Relaxed)
            ),
            (IPL_BIO, IPL_BIO)
        );
        assert_eq!(sc.d.read_at::<u8>(icd_iprn(45)), pmr_of(IPL_BIO) as u8);
        assert_eq!(sc.d.read_at::<u32>(icd_isern(45)), 1 << irq_to_reg32bit(45));
        assert_eq!(sc.d.read_at::<u8>(icd_iptrn(45)), 1);
        // A second handler at another level: max and min span both.
        let ih2 = establish(
            sc,
            45,
            IST_LEVEL_HIGH,
            IPL_NET,
            Some(0),
            IntrHandler::Raw(record, 8),
        )
        .unwrap();
        assert_eq!(iq.irq_max.load(Ordering::Relaxed), IPL_NET.max(IPL_BIO));
        assert_eq!(iq.irq_min.load(Ordering::Relaxed), IPL_NET.min(IPL_BIO));
        // On another CPU: refused, nothing linked.
        assert!(
            establish(
                sc,
                45,
                IST_LEVEL_HIGH,
                IPL_NET,
                Some(1),
                IntrHandler::Raw(record, 0)
            )
            .is_none()
        );
        assert_eq!(iq.list.iter().count(), 2);
        // A PPI is level whatever is asked.
        assert!(
            establish(
                sc,
                20,
                IST_EDGE_RISING,
                IPL_CLOCK,
                None,
                IntrHandler::Raw(record, 0)
            )
            .is_some()
        );
        assert_eq!(sc.d.read_at::<u32>(icd_icrn(20)) & icd_icr_trig_mask(20), 0);
        // Disestablish both: disabled and unrouted, the records kept and refused twice.
        ampintc_intr_disestablish(sc, ih);
        ampintc_intr_disestablish(sc, ih2);
        assert_eq!(iq.irq_min.load(Ordering::Relaxed), IPL_NONE);
        assert_eq!(sc.d.read_at::<u32>(icd_icern(45)), 1 << irq_to_reg32bit(45));
        assert_eq!(sc.d.read_at::<u8>(icd_iptrn(45)), 0);
        assert!(ih.dead.load(Ordering::Relaxed));
        assert!(!List::<IhList>::is_linked(ih));
        let again =
            std::panic::catch_unwind(AssertUnwindSafe(|| ampintc_intr_disestablish(sc, ih)));
        assert!(again.is_err(), "a dead record is refused");
    }

    #[test]
    fn the_dispatch_raises_runs_the_handlers_unmasked_then_ends_and_restores() {
        let _g = setup();
        let sc = softc();
        *DISPATCHED.lock().unwrap() = Some(sc);
        SEEN.lock().unwrap().clear();
        let softc_word = 0x5000usize;
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
            IntrHandler::Raw(record, softc_word),
        )
        .unwrap();
        let frame = ClockFrame::default();
        let before = hal::cpu::curcpu().cpl();
        sc.p.write(EOIR, 0xdead);
        sc.p.write(IAR, 40);
        ampintc_irq_handler(sc, &frame, &InIrq::host());
        let seen = core::mem::take(&mut *SEEN.lock().unwrap());
        *DISPATCHED.lock().unwrap() = None;
        assert_eq!(
            seen,
            vec![
                (
                    core::ptr::from_ref(&frame).addr(),
                    IPL_BIO,
                    pmr_of(IPL_BIO),
                    0xdead
                ),
                (softc_word, IPL_BIO, pmr_of(IPL_BIO), 0xdead),
            ],
            "a NULL argument gets the frame; both run raised, in order, before the EOI"
        );
        assert_eq!(sc.p.read(EOIR), 40, "then the end of interrupt");
        assert_eq!(hal::cpu::curcpu().cpl(), before, "and the level restored");
        assert_eq!(sc.p.read(PMR), pmr_of(before));
        assert_eq!(a.count.ec_count.load(Ordering::Relaxed), 1);
        // The spurious interrupt is counted and not ended; one past the queues ignored.
        sc.p.write(EOIR, 0);
        sc.p.write(IAR, 1023);
        ampintc_irq_handler(sc, &frame, &InIrq::host());
        assert_eq!(sc.spur.ec_count.load(Ordering::Relaxed), 1);
        sc.p.write(IAR, 300);
        ampintc_irq_handler(sc, &frame, &InIrq::host());
        assert_eq!(sc.p.read(EOIR), 0);
    }

    static LOCKED: Mutex<Vec<bool>> = Mutex::new(Vec::new());

    fn lock_seen(_arg: *mut core::ffi::c_void) -> i32 {
        LOCKED
            .lock()
            .unwrap()
            .push(hal::sync::klock::kernel_lock_held());
        0
    }

    #[test]
    fn a_handler_runs_under_the_kernel_lock_unless_mpsafe_or_above_sched() {
        let _g = setup();
        let sc = softc();
        LOCKED.lock().unwrap().clear();
        let ih = establish(
            sc,
            50,
            IST_LEVEL_HIGH,
            IPL_BIO,
            None,
            IntrHandler::Raw(lock_seen, 0),
        )
        .unwrap();
        let safe = establish(
            sc,
            51,
            IST_LEVEL_HIGH,
            IPL_BIO | IPL_MPSAFE,
            None,
            IntrHandler::Raw(lock_seen, 0),
        )
        .unwrap();
        let frame = ClockFrame::default();
        ampintc_run_handler(ih, &frame, IPL_NONE);
        ampintc_run_handler(safe, &frame, IPL_NONE);
        ampintc_run_handler(ih, &frame, IPL_SCHED);
        assert_eq!(*LOCKED.lock().unwrap(), vec![true, false, false]);
        assert_eq!(
            ih.count.ec_count.load(Ordering::Relaxed),
            0,
            "0: not its interrupt"
        );
    }

    #[test]
    fn the_ipi_posts_its_reason_and_sends_the_sgi_to_the_target_alone() {
        let _g = setup();
        let sc = MArc::new(M_DEVBUF, M_WAITOK, new_softc(b"ampintc0")).unwrap();
        sc.ipi_num.store(3, Ordering::Relaxed);
        let me = hal::cpu::cpu_number();
        let (a, b) = if me < 2 { (2, 3) } else { (0, 1) };
        sc.cpu_mask[b].store(0x2, Ordering::Relaxed);
        sc.d.write(SGIR, 0);
        ampintc_send_ipi_sc(&sc, me, ARM_IPI_NOP);
        assert_eq!(sc.d.read(SGIR), 0, "a NOP to oneself is not sent");
        ampintc_send_ipi_sc(&sc, b, ARM_IPI_NOP);
        assert_eq!(
            sc.d.read(SGIR),
            (0x2 << 16) | 3,
            "to the target's mask alone"
        );
        assert_eq!(
            sc.ipi_reason[b].load(Ordering::Relaxed),
            0,
            "a NOP posts nothing"
        );
        ampintc_send_ipi_sc(&sc, a, ARM_IPI_XCALL);
        assert_eq!(sc.ipi_reason[a].load(Ordering::Relaxed), 1 << ARM_IPI_XCALL);
        assert_eq!(sc.d.read(SGIR), (1 << 16) | 3);
        // The handler takes the CPU's reasons (no ddb, no halt among them) and answers 1.
        assert_eq!(ampintc_ipi_run(&sc, a), 1);
        assert_eq!(sc.ipi_reason[a].load(Ordering::Relaxed), 0);
        // Its argument is the softc's counted handle.
        let h = IntrHandler::Shared(ampintc_ipi_handler, MArc::erase(sc.clone()));
        assert_eq!(h.call(&ClockFrame::default()), 1);
    }

    #[test]
    fn the_controller_hooks_route_and_barrier_on_the_handlers_cpu() {
        let _g = setup();
        let sc = softc();
        let ih = establish(
            sc,
            45,
            IST_LEVEL_HIGH,
            IPL_BIO,
            Some(1),
            IntrHandler::Raw(record, 0),
        )
        .unwrap();
        let c = IcCookie::Rec(ih);
        assert_eq!(InterruptController::barrier(sc, c), BarrierWait::Cpu(1));
        sc.p.write(CICR, 0);
        InterruptController::route(sc, c, false, 1);
        assert_eq!(sc.d.read_at::<u8>(icd_iptrn(45)), 0);
        assert_eq!(sc.p.read(CICR), 1);
        InterruptController::route(sc, c, true, 1);
        assert_eq!(sc.d.read_at::<u8>(icd_iptrn(45)), 1);
        assert_eq!(sc.d.read_at::<u8>(icd_iprn(45)), pmr_of(IPL_BIO) as u8);
        InterruptController::disestablish(sc, c);
        assert!(ih.dead.load(Ordering::Relaxed));
        let word = std::panic::catch_unwind(AssertUnwindSafe(|| {
            InterruptController::barrier(sc, IcCookie::Word(1))
        }));
        assert!(word.is_err(), "a cookie this controller did not answer");
    }

    // The GICv2m frame over a fake parent GIC registered on a registry of its own
    // (`intr::test_register`), so the frame's work is tested without the kernel's registry.

    /// A parent GIC that records what it is asked and answers words `1, 2, ...` (or refuses).
    struct FakeGic {
        calls: Mutex<Vec<(&'static str, Vec<u32>, usize)>>,
        refuse: core::sync::atomic::AtomicBool,
        next: core::sync::atomic::AtomicUsize,
    }

    fn word_of(ih: IcCookie) -> usize {
        match ih {
            IcCookie::Word(w) => w,
            IcCookie::Rec(_) => usize::MAX,
        }
    }

    impl InterruptController for FakeGic {
        fn establish(
            &self,
            cell: &[u32],
            level: i32,
            _cpu: Option<usize>,
            _h: IntrHandler,
            _name: &'static str,
        ) -> Option<IcCookie> {
            self.calls
                .lock()
                .unwrap()
                .push(("establish", cell.to_vec(), level as usize));
            if self.refuse.load(Ordering::Relaxed) {
                return None;
            }
            Some(IcCookie::Word(self.next.fetch_add(1, Ordering::Relaxed)))
        }

        fn disestablish(&self, ih: IcCookie) {
            self.calls
                .lock()
                .unwrap()
                .push(("disestablish", vec![], word_of(ih)));
        }

        fn barrier(&self, ih: IcCookie) -> BarrierWait {
            self.calls
                .lock()
                .unwrap()
                .push(("barrier", vec![], word_of(ih)));
            BarrierWait::Cpu(3)
        }
    }

    /// The parent registered (over a one-node tree: its `#interrupt-cells` read at
    /// registration) and a frame of two SPIs from 80 at 0x0802_0000.
    fn frame_and_parent() -> (
        crate::dev::ofw::fdt::testfdt::TreeGuard,
        &'static FakeGic,
        IcRef,
        AmpintcMsiSoftc,
    ) {
        use crate::dev::ofw::fdt::testfdt::{FdtBuilder, install};

        testutil::install();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.begin(b"intc@0");
        b.prop(b"interrupt-controller", b"");
        b.prop_cells(b"#interrupt-cells", &[3]);
        b.prop_cells(b"phandle", &[1]);
        b.end();
        b.end();
        let t = install(b.finish());
        let gic: &'static FakeGic = Box::leak(Box::new(FakeGic {
            calls: Mutex::new(Vec::new()),
            refuse: core::sync::atomic::AtomicBool::new(false),
            next: core::sync::atomic::AtomicUsize::new(1),
        }));
        let ic = crate::dev::arm64::intr::test_register(IcDesc {
            node: crate::dev::ofw::fdt::OF_finddevice(b"/intc@0"),
            gic_its_id: 0,
            ops: gic,
        });
        let spi = MArray::new(M_DEVBUF, M_WAITOK, 2, |_| Spi::Free).unwrap();
        let frame = AmpintcMsiSoftc {
            node: 0,
            addr: 0x0802_0000,
            bspi: 80,
            spi: Mutexed::new(IPL_HIGH, spi),
        };
        (t, gic, ic, frame)
    }

    fn msi(frame: &AmpintcMsiSoftc, ic: IcRef) -> Option<(IcCookie, u64, u64)> {
        let (mut addr, mut data) = (0, 7);
        let c = frame.establish_msi_on(
            ic,
            &mut addr,
            &mut data,
            IPL_BIO,
            None,
            IntrHandler::Raw(record, 0),
            "msi",
        )?;
        Some((c, addr, data))
    }

    #[test]
    fn a_frame_hands_out_its_spis_as_the_doorbell_and_the_spi_number() {
        let (_t, gic, ic, frame) = frame_and_parent();
        let (c0, addr, data) = msi(&frame, ic).unwrap();
        assert_eq!((word_of(c0), addr, data), (0, 0x0802_0040, 80));
        let (c1, addr, data) = msi(&frame, ic).unwrap();
        assert_eq!((word_of(c1), addr, data), (1, 0x0802_0040, 81));
        assert!(msi(&frame, ic).is_none(), "every SPI taken");
        assert_eq!(
            *gic.calls.lock().unwrap(),
            vec![
                ("establish", vec![0, 48, 1], IPL_BIO as usize),
                ("establish", vec![0, 49, 1], IPL_BIO as usize),
            ],
            "an edge-rising SPI each, numbered from the frame's base"
        );
    }

    #[test]
    fn a_refused_spi_stays_free() {
        let (_t, gic, ic, frame) = frame_and_parent();
        gic.refuse.store(true, Ordering::Relaxed);
        assert!(msi(&frame, ic).is_none());
        gic.refuse.store(false, Ordering::Relaxed);
        let (c, _, data) = msi(&frame, ic).unwrap();
        assert_eq!(
            (word_of(c), data),
            (0, 80),
            "the slot the refusal left is taken again"
        );
    }

    #[test]
    fn a_disestablished_spi_is_freed_after_its_parent_and_reclaimed() {
        let (_t, gic, ic, frame) = frame_and_parent();
        let (c0, _, _) = msi(&frame, ic).unwrap();
        let (c1, _, _) = msi(&frame, ic).unwrap();
        // The barrier forwards to the parent with its own cookie (words 1 and 2).
        assert_eq!(
            InterruptController::barrier(&frame, c1),
            BarrierWait::Cpu(3)
        );
        InterruptController::disestablish(&frame, c0);
        assert_eq!(
            InterruptController::barrier(&frame, c0),
            BarrierWait::Nothing,
            "a free slot has nothing to wait for"
        );
        let (again, _, data) = msi(&frame, ic).unwrap();
        assert_eq!((word_of(again), data), (0, 80));
        let calls = gic.calls.lock().unwrap();
        assert_eq!(calls[2], ("barrier", vec![], 2));
        assert_eq!(calls[3], ("disestablish", vec![], 1));
        assert_eq!(calls[4].0, "establish");
        assert!(InterruptController::has_msi(&frame));
    }
}
/* </TESTS> */
