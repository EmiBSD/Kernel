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
//! arm64's interrupt core: the registry of interrupt controllers, the device tree's interrupt
//! specifiers resolved to one of them, and the pre-registration that holds a handler until its
//! controller attaches (`arch/arm64/arm64/intr.c`, `struct interrupt_controller` of
//! `<machine/intr.h>`).
//!
//! Upstream: sys/arch/arm64/arm64/intr.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/include/intr.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/intr.rs@e8249449d905
//! LZ: sys/arch/arm64/include/intr.rs@e8249449d905
//!
//! A controller registers once ([`arm_intr_register_fdt`]) and stays registered for ever (the
//! C has no unregister). A driver establishes a handler on the interrupt its node's
//! `interrupts` (with the `interrupt-parent` walk, [`arm_intr_get_parent`]) or
//! `interrupts-extended` property names ([`arm_intr_establish_fdt_idx_cpu`]), on the one a
//! bus's `interrupt-map` routes a child's unit address and pin to
//! ([`arm_intr_establish_fdt_imap_cpu`]), or on an MSI through `msi-map`/`msi-parent`
//! ([`arm_intr_establish_fdt_msi_cpu`]); the controller the property names takes it and
//! answers its own cookie, kept with the controller in the [`IntrHandle`] the driver gets
//! back. Every node with an `interrupt-controller` property is pre-registered at boot
//! ([`arm_intr_init_fdt`], from mainbus's attach): a handler established before its
//! controller attaches is recorded, and established for real when the controller registers.
//!
//! `spl(9)` and the controller's `spl` table are `hal::cpu::intr`'s (N4.5c, the
//! `IntrHardware` bridge until N7.2a3), the clock table and `delay(9)` are still `bsd`'s
//! (`arch/arm64/arm64/intr.rs`, N7.2b2), and the IPIs follow with the first native controller
//! (N7.2a2).
//!
//! ## Deviations
//! - A pre-registration's record is never freed: the C frees it when its handler is
//!   disestablished (`arm_intr_prereg_disestablish_fdt`); here it is leaked (about 100 bytes
//!   of `M_DEVBUF` per disestablish of a handler established before its controller attached),
//!   because a record is reached through the cookie the driver holds and through the
//!   registration walk, and a `&'static` read in safe code cannot be freed under either. The
//!   same choice as the controllers' handler records (P12; `docs/SCAFFOLDING.md`).
//!
//! ## Redesign
//! - `struct interrupt_controller`, a table of nine nullable function pointers beside the
//!   controller's data (`ic_node`, `ic_cookie`, `ic_phandle`, `ic_cells`, `ic_gic_its_id`, the
//!   list entry), -> the trait [`InterruptController`] (a default method is the C's `NULL`
//!   entry) and the registration [`IcReg`] the registry builds whole from an [`IcDesc`]: the
//!   node, the `phandle` and the `#interrupt-cells` read once at registration, immutable
//!   afterwards, so the lookups walk the list with no lock as the C does. `ic_cookie` is gone:
//!   the controller is the trait object itself.
//! - `ic_establish`'s `void *` result, which only its controller reads, -> [`IcCookie`]: a
//!   record of the controller's that lives for ever (`Rec`, downcast by the controller alone)
//!   or a word (`Word`, a slot's index, or a legacy controller's pointer).
//! - `struct machine_intr_handle`, `malloc`ed at every establish, -> [`IntrHandle`], the
//!   registration and the cookie by value (`Copy`): no allocation per handle, and a handle
//!   cannot name a controller that is not registered.
//! - `struct cpu_info *ci` -> the CPU's number (`Option<usize>`, `None` for the C's `NULL`,
//!   the primary CPU; IDIOMS "a CPU by number").
//! - `ic_barrier`, which calls `sched_barrier(ih->ih_ci)` inside the controller, -> a method
//!   that answers the CPU to wait for ([`BarrierCpu`]), and [`intr_barrier`] makes the call:
//!   the scheduler is generic over the process glue until N8, which a trait object's method
//!   cannot be (scaffolding).
//! - `LIST_HEAD interrupt_controllers` and `prereg_interrupts`, raw lists touched "at attach
//!   time", -> `hal::ds::list::List`s of `&'static` records in one [`IntrRegistry`] (the
//!   process's is a static; a test builds its own), their inserts and removes under one
//!   `Mutexed<()>` at `IPL_NONE` (the C relies on autoconf and the kernel lock), never held
//!   across a controller's method (an establish may sleep in `malloc`). The registration's
//!   walk of the pre-registrations (`LIST_FOREACH_SAFE`) takes the matching records off under
//!   the lock onto a list of its own and establishes them after, in the C's order.
//! - `struct intr_prereg` -> [`Prereg`] (renamed), `MBox::leak`ed; its controller and cookie,
//!   set later, are set-once cells, its level an atomic (`intr_set_wakeup` ORs `IPL_WAKEUP`).
//!   The dummy controller is [`PreregIc`], the first native implementer of the trait.
//! - The cell buffers `malloc(len, M_TEMP)` + `free` -> `MArray<u32>` dropped at the end of
//!   the call (`M_TEMP`, `interrupt-map`'s `M_DEVBUF` as the C).
//! - LZ's empty-phandle step of the `interrupts-extended` walk did not count the empty entry
//!   as an interrupt (a `while` whose `continue` skipped `i += 1`); the C's `for` counts it:
//!   the C's is kept here.

use core::any::Any;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::cpu::intr::{IPL_NONE, IPL_WAKEUP};
use hal::ds::list::{List, ListEntry};
use hal::mem::malloc::{M_WAITOK, MArray, MBox};
use hal::sync::mutex::Mutexed;
use hal::sync::once::OnceStatic;

use crate::assert::{kassert, kpanic};
use crate::base::prf::printf;
use crate::dev::device::M_DEVBUF;
use crate::dev::intr::IntrHandler;
use crate::dev::ofw::fdt::{
    OF_child, OF_getnodebyphandle, OF_getpropbool, OF_getpropint, OF_getpropintarray,
    OF_getproplen, OF_parent, OF_peer,
};
use crate::proc::ProcGlue;
use crate::sched::sched_barrier;

/// `M_TEMP` (`<sys/malloc.h>`): the cell buffers.
const M_TEMP: i32 = 127;

/// `MAX_INTERRUPT_CELLS`: the most `#interrupt-cells` a controller may have.
pub const MAX_INTERRUPT_CELLS: usize = 4;

/// `IST_NONE`: none.
pub const IST_NONE: i32 = 0;
/// `IST_PULSE`: pulsed.
pub const IST_PULSE: i32 = 1;
/// `IST_EDGE`: edge-triggered.
pub const IST_EDGE: i32 = 2;
/// `IST_LEVEL`: level-triggered.
pub const IST_LEVEL: i32 = 3;
/// `IST_LEVEL_LOW`.
pub const IST_LEVEL_LOW: i32 = IST_LEVEL;
/// `IST_LEVEL_HIGH`.
pub const IST_LEVEL_HIGH: i32 = 4;
/// `IST_EDGE_FALLING`.
pub const IST_EDGE_FALLING: i32 = IST_EDGE;
/// `IST_EDGE_RISING`.
pub const IST_EDGE_RISING: i32 = 5;
/// `IST_EDGE_BOTH`.
pub const IST_EDGE_BOTH: i32 = 6;

/// `SOFTINTR_XCALL`: the cross-call soft interrupt, after the machine-independent ones.
pub const SOFTINTR_XCALL: i32 = sysdefs::dev::softintr::NSOFTINTR as i32;

/// `ARM_IPI_NOP` (`MULTIPROCESSOR`): only wakes the CPU up.
pub const ARM_IPI_NOP: i32 = 0;
/// `ARM_IPI_DDB`: enter ddb.
pub const ARM_IPI_DDB: i32 = 1;
/// `ARM_IPI_HALT`: halt the CPU (`cpu_halt`).
pub const ARM_IPI_HALT: i32 = 2;
/// `ARM_IPI_XCALL`: run the cross calls.
pub const ARM_IPI_XCALL: i32 = 3;

/// What a controller's establish answers (`ic_establish`'s `void *`): read back by that
/// controller alone, at disestablish, barrier, route and the rest.
#[derive(Clone, Copy)]
pub enum IcCookie {
    /// A record of the controller's, kept for ever (a handler, a pre-registration); the
    /// controller downcasts it to its own type.
    Rec(&'static (dyn Any + Send + Sync)),
    /// A word: a slot's index (an MSI frame's), or a legacy controller's pointer as an exposed
    /// address (scaffolding).
    Word(usize),
}

/// The CPU `intr_barrier` waits for (`sched_barrier(ih->ih_ci)`): a CPU's number, or `None`
/// for the primary CPU (the C's `NULL`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarrierCpu(pub Option<usize>);

/// A controller's registration request: the members of `struct interrupt_controller` the
/// driver fills before `arm_intr_register_fdt`.
#[derive(Clone, Copy)]
pub struct IcDesc {
    /// `ic_node`: the controller's device tree node.
    pub node: i32,
    /// `ic_gic_its_id`: the ITS's identifier (agintc's MSI frames), 0 for others.
    pub gic_its_id: u32,
    /// The controller's entry points (`ic_establish` .. `ic_set_wakeup`).
    pub ops: &'static dyn InterruptController,
}

/// A registered controller (`struct interrupt_controller` on `interrupt_controllers`): built
/// whole by the registration, immutable, never freed.
pub struct IcReg {
    node: i32,
    phandle: u32,
    cells: u32,
    gic_its_id: u32,
    ops: &'static dyn InterruptController,
    link: ListEntry<IcReg>,
}

impl IcReg {
    /// `ic_node`.
    pub fn node(&self) -> i32 {
        self.node
    }

    /// `ic_phandle`: the node's `phandle`, read at registration.
    pub fn phandle(&self) -> u32 {
        self.phandle
    }

    /// `ic_cells`: the node's `#interrupt-cells`, read at registration.
    pub fn cells(&self) -> u32 {
        self.cells
    }

    /// `ic_gic_its_id`.
    pub fn gic_its_id(&self) -> u32 {
        self.gic_its_id
    }

    /// The controller's entry points.
    pub fn ops(&self) -> &'static dyn InterruptController {
        self.ops
    }

    /// `ic->ic_cells` as a count of cells, bounded by [`MAX_INTERRUPT_CELLS`] where the C
    /// only asserts it (`KASSERT` at registration).
    fn ncells(&self) -> usize {
        (self.cells as usize).min(MAX_INTERRUPT_CELLS)
    }
}

hal::list_adapter!(
    /// `LIST_ENTRY(interrupt_controller) ic_list`.
    IcList: IcReg, link
);

/// A registered controller: `struct interrupt_controller *`, which the C never frees.
pub type IcRef = &'static IcReg;

/// `struct machine_intr_handle`: the controller that took a handler and its cookie, by value.
#[derive(Clone, Copy)]
pub struct IntrHandle {
    ic: IcRef,
    ih: IcCookie,
}

impl IntrHandle {
    /// `ih_ic`: the controller.
    pub fn controller(&self) -> IcRef {
        self.ic
    }

    /// `ih_ih`: the controller's cookie.
    pub fn cookie(&self) -> IcCookie {
        self.ih
    }
}

/// `struct intr_prereg`: a handler established before its controller attached, recorded by
/// the node's dummy controller ([`PreregIc`]) and established for real when the controller
/// registers.
pub struct Prereg {
    link: ListEntry<Prereg>,
    /// `ip_phandle`: the controller's.
    phandle: u32,
    /// `ip_cell`: the interrupt specifier.
    cell: [u32; MAX_INTERRUPT_CELLS],
    /// `ip_level` (`intr_set_wakeup` ORs `IPL_WAKEUP` in).
    level: AtomicI32,
    /// `ip_ci`.
    cpu: Option<usize>,
    /// `ip_func`, `ip_arg`.
    handler: IntrHandler,
    /// `ip_name`.
    name: &'static str,
    /// `ip_ic`: the real controller, once it registered (set under the registry's lock).
    ic: OnceStatic<IcRef>,
    /// `ip_ih`: its cookie, once it established the handler.
    ih: OnceStatic<IcCookie>,
}

hal::list_adapter!(
    /// `LIST_ENTRY(intr_prereg) ip_list`.
    PreregList: Prereg, link
);

/// The dummy controller of one `interrupt-controller` node (`arm_intr_init_fdt_recurse`'s):
/// its establish records the handler ([`Prereg`]).
pub struct PreregIc {
    /// The registry whose pre-registration list the records join.
    registry: &'static IntrRegistry,
    /// Its own registration (`ic->ic_phandle`, `ic->ic_cells`), set right after it is linked.
    reg: OnceStatic<IcRef>,
}

impl PreregIc {
    /// The record behind a cookie this controller answered.
    fn record(ih: IcCookie) -> &'static Prereg {
        match ih {
            IcCookie::Rec(r) => match r.downcast_ref::<Prereg>() {
                Some(ip) => ip,
                None => kpanic(format_args!("arm_intr_prereg: not a pre-registration")),
            },
            IcCookie::Word(_) => kpanic(format_args!("arm_intr_prereg: not a pre-registration")),
        }
    }

    /// The real controller and its cookie, once both exist (`ip->ip_ic`, `ip->ip_ih`).
    fn taken(ip: &Prereg) -> Option<(IcRef, IcCookie)> {
        Some((*ip.ic.get()?, *ip.ih.get()?))
    }
}

/// The two lists of `intr.c` (`interrupt_controllers`, `prereg_interrupts`) and the lock
/// their writers take.
pub struct IntrRegistry {
    /// `interrupt_controllers`: the latest registered first.
    controllers: List<IcList>,
    /// `prereg_interrupts`: the handlers waiting for their controller, the latest first.
    prereg: List<PreregList>,
    /// Serialises the two lists' inserts and removes; lookups walk with no lock.
    lock: Mutexed<()>,
}

impl IntrRegistry {
    /// Both lists empty.
    pub const fn new() -> Self {
        Self {
            controllers: List::new(),
            prereg: List::new(),
            lock: Mutexed::new(IPL_NONE, ()),
        }
    }

    /// The registered controller of `node` (`LIST_FOREACH` on `ic_node`).
    fn find_node(&self, node: i32) -> Option<IcRef> {
        self.controllers.iter().find(|ic| ic.node == node)
    }

    /// The registered controller of `phandle` (`LIST_FOREACH` on `ic_phandle`).
    fn find_phandle(&self, phandle: u32) -> Option<IcRef> {
        self.controllers.iter().find(|ic| ic.phandle == phandle)
    }

    /// `arm_intr_init_fdt_recurse`: a dummy controller for every `interrupt-controller` node
    /// at or under `node`.
    fn init_fdt_recurse(&'static self, node: i32) {
        if OF_getproplen(node, b"interrupt-controller") >= 0 {
            let ic = leak(PreregIc {
                registry: self,
                reg: OnceStatic::new(),
            });
            self.register_fdt(
                IcDesc {
                    node,
                    gic_its_id: 0,
                    ops: ic,
                },
                |reg| {
                    let _ = ic.reg.set(reg);
                },
            );
        }

        let mut child = OF_child(node);
        while child != 0 {
            self.init_fdt_recurse(child);
            child = OF_peer(child);
        }
    }

    /// `arm_intr_register_fdt`: `desc`'s controller registers, `linked` runs once it is on
    /// the list, and the handlers pre-registered for its `phandle` are established on it, in
    /// the list's order; one it refuses prints `can't establish interrupt <name>`.
    fn register_fdt(&'static self, desc: IcDesc, linked: impl FnOnce(IcRef)) -> IcRef {
        let cells = OF_getpropint(desc.node, b"#interrupt-cells", 0);
        let phandle = OF_getpropint(desc.node, b"phandle", 0);
        kassert!(cells as usize <= MAX_INTERRUPT_CELLS);
        let ic = leak(IcReg {
            node: desc.node,
            phandle,
            cells,
            gic_its_id: desc.gic_its_id,
            ops: desc.ops,
            link: ListEntry::new(),
        });
        {
            let _g = self.lock.lock();
            self.controllers.insert_head(ic);
        }
        linked(ic);

        // Establish pre-registered interrupt handlers: off the list under the lock, then on
        // the controller without it.
        let taken: List<PreregList> = List::new();
        {
            let _g = self.lock.lock();
            for ip in &self.prereg {
                if ip.phandle != ic.phandle {
                    continue;
                }
                self.prereg.remove(ip);
                let _ = ip.ic.set(ic);
                taken.insert_tail(ip);
            }
        }
        while let Some(ip) = taken.pop_head() {
            let ih = ic.ops.establish(
                &ip.cell[..ic.ncells()],
                ip.level.load(Ordering::Relaxed),
                ip.cpu,
                ip.handler.clone(),
                ip.name,
            );
            match ih {
                Some(ih) => {
                    let _ = ip.ih.set(ih);
                }
                None => printf(format_args!("can't establish interrupt {}\n", ip.name)),
            }
        }
        ic
    }

    /// `arm_intr_establish_fdt_idx_cpu`'s walk.
    fn establish_fdt_idx_cpu(
        &self,
        node: i32,
        idx: usize,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IntrHandle> {
        let mut extended = true;
        let mut prop: &[u8] = b"interrupts-extended";
        let mut len = OF_getproplen(node, prop);
        if len <= 0 {
            prop = b"interrupts";
            len = OF_getproplen(node, prop);
            extended = false;
        }
        if len <= 0 || len % 4 != 0 {
            return None;
        }

        // Old style.
        let mut ic = None;
        if !extended {
            let parent = arm_intr_get_parent(node);
            ic = Some(self.find_node(parent)?);
        }

        let mut cells = MArray::new(M_TEMP, M_WAITOK, len as usize / 4, |_| 0u32)?;
        OF_getpropintarray(node, prop, &mut cells);
        let mut cell: &[u32] = &cells;
        let mut i = 0;
        while i <= idx && !cell.is_empty() {
            if extended {
                let phandle = cell[0];
                cell = &cell[1..];
                // Handle "empty" phandle reference: it counts as an interrupt (the C's `for`).
                if phandle == 0 {
                    i += 1;
                    continue;
                }
                ic = Some(self.find_phandle(phandle)?);
            }
            let c = ic?;
            let n = c.ncells();
            if i == idx && cell.len() >= n {
                return c
                    .ops
                    .establish(&cell[..n], level, cpu, h, name)
                    .map(|ih| IntrHandle { ic: c, ih });
            }
            cell = cell.get(n..)?;
            i += 1;
        }
        None
    }

    /// `arm_intr_establish_fdt_imap_cpu`'s walk of `interrupt-map`.
    fn establish_fdt_imap_cpu(
        &self,
        node: i32,
        reg: &[u32],
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IntrHandle> {
        let mut map_mask = [0u32; 4];
        if reg.len() != map_mask.len() {
            return None;
        }
        if OF_getpropintarray(node, b"interrupt-map-mask", &mut map_mask) != 16 {
            return None;
        }
        let len = OF_getproplen(node, b"interrupt-map");
        if len <= 0 {
            return None;
        }

        let mut map = MArray::new(M_DEVBUF, M_WAITOK, len as usize / 4, |_| 0u32)?;
        OF_getpropintarray(node, b"interrupt-map", &mut map);
        let mut cell: &[u32] = &map;
        while cell.len() > 5 {
            let ic = self.find_phandle(cell[4])?;
            let acells = OF_getpropint(ic.node, b"#address-cells", 0) as usize;
            let n = ic.ncells();
            let entry = 5 + acells + n;
            if cell.len() >= entry && (0..4).all(|k| reg[k] & map_mask[k] == cell[k]) {
                return ic
                    .ops
                    .establish(&cell[5 + acells..entry], level, cpu, h, name)
                    .map(|ih| IntrHandle { ic, ih });
            }
            cell = cell.get(entry..)?;
        }
        None
    }

    /// `arm_intr_establish_fdt_msi_cpu`'s lookup.
    #[allow(clippy::too_many_arguments)] // the C's signature, the registry beside it
    fn establish_fdt_msi_cpu(
        &self,
        node: i32,
        addr: &mut u64,
        data: &mut u64,
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IntrHandle> {
        let phandle = arm_intr_map_msi(node, data);
        let ic = self.find_phandle(phandle)?;
        ic.ops
            .establish_msi(addr, data, level, cpu, h, name)
            .map(|ih| IntrHandle { ic, ih })
    }

    /// `arm_intr_parent_establish_fdt`: on the parent of `ic`'s node.
    fn parent_establish_fdt(
        &self,
        ic: IcRef,
        cell: &[u32],
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IntrHandle> {
        let parent = arm_intr_get_parent(ic.node);
        let pic = self.find_node(parent)?;
        pic.ops
            .establish(cell, level, cpu, h, name)
            .map(|ih| IntrHandle { ic: pic, ih })
    }
}

impl Default for IntrRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The kernel's registry.
static INTR: IntrRegistry = IntrRegistry::new();

/// `struct interrupt_controller`'s entry points. A default method is the C's `NULL` entry:
/// what the C's caller does when it finds one (nothing, or the `KASSERT` it makes).
pub trait InterruptController: Sync + 'static {
    /// `ic_establish(cookie, cell, level, ci, func, arg, name)`: takes the handler for the
    /// interrupt `cell` (the controller's `#interrupt-cells` of them) at `level` on CPU `cpu`
    /// (`None`: the controller's choice); `None` when it cannot.
    fn establish(
        &self,
        _cell: &[u32],
        _level: i32,
        _cpu: Option<usize>,
        _h: IntrHandler,
        _name: &'static str,
    ) -> Option<IcCookie> {
        None
    }

    /// `ic_establish_msi(cookie, addr, data, level, ci, func, arg, name)`: an MSI; `data`
    /// comes in as the device's identifier and both come back as what the device must write
    /// where.
    fn establish_msi(
        &self,
        _addr: &mut u64,
        _data: &mut u64,
        _level: i32,
        _cpu: Option<usize>,
        _h: IntrHandler,
        _name: &'static str,
    ) -> Option<IcCookie> {
        None
    }

    /// `ic_disestablish(ih)`.
    fn disestablish(&self, ih: IcCookie);

    /// `ic_enable(ih)`: `arm_intr_enable` asserts the entry exists (`KASSERT`); without it
    /// the C calls through `NULL`.
    fn enable(&self, _ih: IcCookie) {
        kpanic(format_args!(
            "arm_intr_enable: the controller has no ic_enable"
        ))
    }

    /// `ic_disable(ih)`: as [`InterruptController::enable`].
    fn disable(&self, _ih: IcCookie) {
        kpanic(format_args!(
            "arm_intr_disable: the controller has no ic_disable"
        ))
    }

    /// `ic_route(ih, enable, ci)`: (un)routes the interrupt to CPU `cpu`.
    fn route(&self, _ih: IcCookie, _enable: bool, _cpu: usize) {}

    /// `ic_cpu_enable()`: the per-CPU setup on a CPU that comes up.
    fn cpu_enable(&self) {}

    /// `ic_barrier(ih)`: the CPU whose pass through the scheduler proves no handler of `ih`
    /// still runs; `None` when there is nothing to wait for (or the controller waited
    /// itself).
    fn barrier(&self, ih: IcCookie) -> Option<BarrierCpu>;

    /// `ic_set_wakeup(ih)`: the interrupt may wake the machine.
    fn set_wakeup(&self, _ih: IcCookie) {}

    /// The address of `bsd`'s `struct interrupt_controller` behind an adapter (scaffolding,
    /// `docs/SCAFFOLDING.md`); `None` for a native controller.
    fn legacy(&self) -> Option<usize> {
        None
    }
}

/// A value of the kernel's for ever: `malloc(M_DEVBUF, M_WAITOK)`, never freed.
fn leak<T>(value: T) -> &'static T {
    match MBox::new(M_DEVBUF, M_WAITOK, value) {
        Some(b) => MBox::leak(b),
        None => kpanic(format_args!("arm_intr: no malloc(9)")),
    }
}

/// `arm_intr_get_parent`: the interrupt parent of `node`: its `interrupt-parent`, or the
/// first `interrupt-controller` up the tree; 0 when there is none.
pub fn arm_intr_get_parent(node: i32) -> i32 {
    let mut node = node;
    while node != 0 {
        let phandle = OF_getpropint(node, b"interrupt-parent", 0);
        if phandle != 0 {
            return OF_getnodebyphandle(phandle);
        }
        node = OF_parent(node);
        if OF_getpropbool(node, b"interrupt-controller") {
            return node;
        }
    }
    0
}

/// `arm_intr_map_msi`: the phandle of `node`'s MSI controller, through its `msi-map` (which
/// also turns the requester ID in `data` into the controller's device ID) or the first
/// `msi-parent` up the tree; 0 when there is none.
pub fn arm_intr_map_msi(node: i32, data: &mut u64) -> u32 {
    let len = OF_getproplen(node, b"msi-map");
    if len <= 0 {
        let mut node = node;
        let mut phandle = 0;
        while node != 0 && phandle == 0 {
            phandle = OF_getpropint(node, b"msi-parent", 0);
            node = OF_parent(node);
        }
        return phandle;
    }

    let Some(mut map) = MArray::new(M_TEMP, M_WAITOK, len as usize / 4, |_| 0u32) else {
        return 0;
    };
    OF_getpropintarray(node, b"msi-map", &mut map);

    let mask = OF_getpropint(node, b"msi-map-mask", 0xffff);
    let rid = (*data as u32) & mask;

    let mut cell: &[u32] = &map;
    while cell.len() > 1 {
        let ctrl = OF_getnodebyphandle(cell[1]);
        if ctrl == 0 {
            break;
        }

        // Some device trees (e.g. those for the Rockchip RK3399 boards) are missing a
        // #msi-cells property. Assume the msi-specifier uses a single cell in that case.
        let mcells = OF_getpropint(ctrl, b"#msi-cells", 1) as usize;
        if cell.len() < mcells + 3 {
            break;
        }

        let rid_base = cell[0];
        let length = cell[2 + mcells];
        let msi_base = cell[2..2 + mcells]
            .iter()
            .fold(0u64, |base, &c| base << 32 | u64::from(c));
        if rid >= rid_base && rid < rid_base.wrapping_add(length) {
            *data = msi_base.wrapping_add(u64::from(rid - rid_base));
            return cell[1];
        }

        cell = &cell[3 + mcells..];
    }
    0
}

impl InterruptController for PreregIc {
    /// `arm_intr_prereg_establish_fdt`: records the handler until the controller registers.
    fn establish(
        &self,
        cell: &[u32],
        level: i32,
        cpu: Option<usize>,
        h: IntrHandler,
        name: &'static str,
    ) -> Option<IcCookie> {
        let reg = *self.reg.get()?;
        let mut ip_cell = [0u32; MAX_INTERRUPT_CELLS];
        for (to, from) in ip_cell.iter_mut().zip(cell).take(reg.ncells()) {
            *to = *from;
        }
        let ip = leak(Prereg {
            link: ListEntry::new(),
            phandle: reg.phandle,
            cell: ip_cell,
            level: AtomicI32::new(level),
            cpu,
            handler: h,
            name,
            ic: OnceStatic::new(),
            ih: OnceStatic::new(),
        });
        let _g = self.registry.lock.lock();
        self.registry.prereg.insert_head(ip);
        Some(IcCookie::Rec(ip))
    }

    /// `arm_intr_prereg_disestablish_fdt`: off the list while no controller took it; through
    /// the controller once one did. The record is not freed (see the Deviations).
    fn disestablish(&self, ih: IcCookie) {
        let ip = Self::record(ih);
        {
            let _g = self.registry.lock.lock();
            if ip.ic.get().is_none() {
                self.registry.prereg.remove(ip);
                return;
            }
        }
        if let Some((ic, ih)) = Self::taken(ip) {
            ic.ops.disestablish(ih);
        }
    }

    /// `arm_intr_prereg_barrier_fdt`: the real controller's answer, once it took the handler.
    fn barrier(&self, ih: IcCookie) -> Option<BarrierCpu> {
        let (ic, ih) = Self::taken(Self::record(ih))?;
        ic.ops.barrier(ih)
    }

    /// `arm_intr_prereg_set_wakeup_fdt`: through the real controller once it took the
    /// handler; the level keeps `IPL_WAKEUP` either way.
    fn set_wakeup(&self, ih: IcCookie) {
        let ip = Self::record(ih);
        if let Some((ic, ih)) = Self::taken(ip) {
            ic.ops.set_wakeup(ih);
        }
        ip.level.fetch_or(IPL_WAKEUP, Ordering::Relaxed);
    }
}

/// `arm_intr_init_fdt`: pre-registers every interrupt controller of the tree (mainbus's
/// attach).
pub fn arm_intr_init_fdt() {
    let node = OF_peer(0);
    if node != 0 {
        INTR.init_fdt_recurse(node);
    }
}

/// `arm_intr_register_fdt`: a real controller registers; the handlers pre-registered for its
/// node are established on it now.
pub fn arm_intr_register_fdt(desc: IcDesc) -> IcRef {
    INTR.register_fdt(desc, |_| {})
}

/// [`arm_intr_register_fdt`] with `linked` run once the controller is on the list and before
/// the pre-registered handlers are established: `bsd`'s adapter writes `ic_cells` and
/// `ic_phandle` back into its legacy structure there and links its own list (scaffolding).
pub fn arm_intr_register_fdt_linked(desc: IcDesc, linked: impl FnOnce(IcRef)) -> IcRef {
    INTR.register_fdt(desc, linked)
}

/// `arm_intr_establish_fdt`: the first interrupt of `node`.
pub fn arm_intr_establish_fdt(
    node: i32,
    level: i32,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    arm_intr_establish_fdt_idx_cpu(node, 0, level, None, h, name)
}

/// `arm_intr_establish_fdt_cpu`.
pub fn arm_intr_establish_fdt_cpu(
    node: i32,
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    arm_intr_establish_fdt_idx_cpu(node, 0, level, cpu, h, name)
}

/// `arm_intr_establish_fdt_idx`: interrupt `idx` of `node`.
pub fn arm_intr_establish_fdt_idx(
    node: i32,
    idx: usize,
    level: i32,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    arm_intr_establish_fdt_idx_cpu(node, idx, level, None, h, name)
}

/// `arm_intr_establish_fdt_idx_cpu`: interrupt `idx` of `node` on CPU `cpu`, through the
/// controller its `interrupts-extended` or `interrupts` property names.
pub fn arm_intr_establish_fdt_idx_cpu(
    node: i32,
    idx: usize,
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    INTR.establish_fdt_idx_cpu(node, idx, level, cpu, h, name)
}

/// `arm_intr_establish_fdt_imap`.
pub fn arm_intr_establish_fdt_imap(
    node: i32,
    reg: &[u32],
    level: i32,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    arm_intr_establish_fdt_imap_cpu(node, reg, level, None, h, name)
}

/// `arm_intr_establish_fdt_imap_cpu`: the interrupt `node`'s `interrupt-map` routes the child
/// unit address and pin `reg` (four cells: `phys.hi`, `phys.mid`, `phys.lo`, the pin) to,
/// after `interrupt-map-mask` (the C's `int *reg, int nreg`, `nreg` in bytes, is the slice).
pub fn arm_intr_establish_fdt_imap_cpu(
    node: i32,
    reg: &[u32],
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    INTR.establish_fdt_imap_cpu(node, reg, level, cpu, h, name)
}

/// `arm_intr_establish_fdt_msi`.
pub fn arm_intr_establish_fdt_msi(
    node: i32,
    addr: &mut u64,
    data: &mut u64,
    level: i32,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    arm_intr_establish_fdt_msi_cpu(node, addr, data, level, None, h, name)
}

/// `arm_intr_establish_fdt_msi_cpu`: an MSI through the controller `node`'s `msi-map` or
/// `msi-parent` names; `addr` and `data` come back as what the device must write where.
pub fn arm_intr_establish_fdt_msi_cpu(
    node: i32,
    addr: &mut u64,
    data: &mut u64,
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    INTR.establish_fdt_msi_cpu(node, addr, data, level, cpu, h, name)
}

/// `arm_intr_disestablish_fdt`: through the controller that took the handler.
pub fn arm_intr_disestablish_fdt(ih: IntrHandle) {
    ih.ic.ops.disestablish(ih.ih);
}

/// `arm_intr_enable`.
pub fn arm_intr_enable(ih: IntrHandle) {
    ih.ic.ops.enable(ih.ih);
}

/// `arm_intr_disable`.
pub fn arm_intr_disable(ih: IntrHandle) {
    ih.ic.ops.disable(ih.ih);
}

/// `arm_intr_parent_establish_fdt`: some interrupt controllers transparently forward
/// interrupts to their parent. Such interrupt controllers can use this function to delegate
/// the interrupt handler to their parent.
pub fn arm_intr_parent_establish_fdt(
    ic: IcRef,
    cell: &[u32],
    level: i32,
    cpu: Option<usize>,
    h: IntrHandler,
    name: &'static str,
) -> Option<IntrHandle> {
    INTR.parent_establish_fdt(ic, cell, level, cpu, h, name)
}

/// `arm_intr_parent_disestablish_fdt`.
pub fn arm_intr_parent_disestablish_fdt(ih: IntrHandle) {
    arm_intr_disestablish_fdt(ih);
}

/// `arm_intr_route`: (un)routes the interrupt to CPU `cpu`, when the controller can.
pub fn arm_intr_route(ih: IntrHandle, enable: bool, cpu: usize) {
    ih.ic.ops.route(ih.ih, enable, cpu);
}

/// `arm_intr_cpu_enable`: every controller's per-CPU setup, on a CPU that comes up.
pub fn arm_intr_cpu_enable() {
    for ic in &INTR.controllers {
        ic.ops.cpu_enable();
    }
}

/// `intr_barrier`: returns once no CPU runs a handler of `ih` (`ic_barrier`, whose
/// `sched_barrier` is made here, `G` being the scheduler's glue).
pub fn intr_barrier<G: ProcGlue>(ih: IntrHandle) {
    if let Some(BarrierCpu(cpu)) = ih.ic.ops.barrier(ih.ih) {
        sched_barrier::<G>(cpu);
    }
}

/// `intr_set_wakeup`.
pub fn intr_set_wakeup(ih: IntrHandle) {
    ih.ic.ops.set_wakeup(ih.ih);
}

/// `LIST_FOREACH(ic, &interrupt_controllers, ic_list)`: the registered controllers, the
/// latest first.
pub fn controllers() -> impl Iterator<Item = IcRef> {
    INTR.controllers.iter()
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The registry over a tree built here (`testfdt`) and fake controllers that record their
    // calls: the parent walk, the specifier walks (plain, extended with its empty phandle, a
    // short property, `interrupt-map` and its mask, `msi-map` with and without `#msi-cells`,
    // `msi-parent`), the pre-registration before and after its controller, head insertion.
    // Each test builds its own registry; the tree is the process's, so `install` serialises
    // them.

    use core::sync::atomic::AtomicUsize;
    use std::boxed::Box;
    use std::sync::Mutex;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use super::*;
    use crate::dev::ofw::fdt::OF_finddevice;
    use crate::dev::ofw::fdt::testfdt::{FdtBuilder, TreeGuard, install};
    use crate::testutil;

    #[derive(Clone, Debug, PartialEq)]
    enum Call {
        Establish(Vec<u32>, i32, Option<usize>, usize, &'static str),
        Msi(u64, u64, &'static str),
        Disestablish(usize),
        Barrier(usize),
        Wakeup(usize),
        Route(usize, bool, usize),
    }

    /// A controller that records its calls and answers words `1, 2, ...` (or refuses).
    struct Fake {
        calls: Mutex<Vec<Call>>,
        refuse: bool,
        next: AtomicUsize,
    }

    impl Fake {
        fn new(refuse: bool) -> &'static Fake {
            Box::leak(Box::new(Fake {
                calls: Mutex::new(Vec::new()),
                refuse,
                next: AtomicUsize::new(1),
            }))
        }

        fn push(&self, c: Call) {
            self.calls.lock().unwrap().push(c);
        }

        fn calls(&self) -> Vec<Call> {
            self.calls.lock().unwrap().clone()
        }

        fn answer(&self) -> Option<IcCookie> {
            if self.refuse {
                None
            } else {
                Some(IcCookie::Word(self.next.fetch_add(1, Ordering::Relaxed)))
            }
        }
    }

    fn word(ih: IcCookie) -> usize {
        match ih {
            IcCookie::Word(w) => w,
            IcCookie::Rec(_) => panic!("a record"),
        }
    }

    impl InterruptController for Fake {
        fn establish(
            &self,
            cell: &[u32],
            level: i32,
            cpu: Option<usize>,
            h: IntrHandler,
            name: &'static str,
        ) -> Option<IcCookie> {
            self.push(Call::Establish(
                cell.to_vec(),
                level,
                cpu,
                h.raw_arg().addr(),
                name,
            ));
            self.answer()
        }

        fn establish_msi(
            &self,
            addr: &mut u64,
            data: &mut u64,
            _level: i32,
            _cpu: Option<usize>,
            _h: IntrHandler,
            name: &'static str,
        ) -> Option<IcCookie> {
            self.push(Call::Msi(*addr, *data, name));
            *addr = 0x0802_0040;
            *data += 0x1000;
            self.answer()
        }

        fn disestablish(&self, ih: IcCookie) {
            self.push(Call::Disestablish(word(ih)));
        }

        fn route(&self, ih: IcCookie, enable: bool, cpu: usize) {
            self.push(Call::Route(word(ih), enable, cpu));
        }

        fn barrier(&self, ih: IcCookie) -> Option<BarrierCpu> {
            self.push(Call::Barrier(word(ih)));
            Some(BarrierCpu(Some(1)))
        }

        fn set_wakeup(&self, ih: IcCookie) {
            self.push(Call::Wakeup(word(ih)));
        }
    }

    fn nop(_arg: *mut core::ffi::c_void) -> i32 {
        0
    }

    /// A handler whose argument is `arg`.
    fn h(arg: usize) -> IntrHandler {
        IntrHandler::Raw(nop, arg)
    }

    /// The test tree: a GIC (phandle 1, three cells), two MSI frames (phandle 2 with
    /// `#msi-cells`, 3 without), a bus whose `interrupt-parent` is the GIC with devices of
    /// each kind, a one-cell controller under it, a PCI host with an `interrupt-map` and an
    /// `msi-map`, and a device with an `msi-parent`.
    fn tree() -> TreeGuard {
        testutil::install();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop_cells(b"#address-cells", &[2]);
        b.begin(b"intc@0");
        b.prop(b"interrupt-controller", b"");
        b.prop_cells(b"#interrupt-cells", &[3]);
        b.prop_cells(b"phandle", &[1]);
        b.end();
        b.begin(b"msi@1");
        b.prop(b"msi-controller", b"");
        b.prop_cells(b"#msi-cells", &[1]);
        b.prop_cells(b"phandle", &[2]);
        b.end();
        b.begin(b"msi@2");
        b.prop(b"msi-controller", b"");
        b.prop_cells(b"phandle", &[3]);
        b.end();
        b.begin(b"soc");
        b.prop_cells(b"interrupt-parent", &[1]);
        b.begin(b"dev@0");
        b.prop_cells(b"interrupts", &[0, 5, 4, 0, 6, 4]);
        b.end();
        b.begin(b"ext@1");
        b.prop_cells(b"interrupts-extended", &[0, 1, 0, 7, 4, 1, 0, 8, 4]);
        b.end();
        b.begin(b"short@2");
        b.prop_cells(b"interrupts-extended", &[1, 0, 9]);
        b.end();
        b.begin(b"odd@3");
        b.prop(b"interrupts", &[0, 0, 0, 1, 0, 0]);
        b.end();
        b.begin(b"sub@4");
        b.prop(b"interrupt-controller", b"");
        b.prop_cells(b"#interrupt-cells", &[1]);
        b.prop_cells(b"phandle", &[4]);
        b.begin(b"leaf@0");
        b.prop_cells(b"interrupts", &[3]);
        b.end();
        b.end();
        b.begin(b"child@5");
        b.prop_cells(b"interrupt-parent", &[4]);
        b.prop_cells(b"interrupts", &[2]);
        b.end();
        b.end();
        b.begin(b"pci");
        b.prop_cells(b"interrupt-map-mask", &[0x1800, 0, 0, 7]);
        b.prop_cells(
            b"interrupt-map",
            &[
                0x0000, 0, 0, 1, 1, 0, 0x20, 4, //
                0x0800, 0, 0, 1, 1, 0, 0x21, 4,
            ],
        );
        b.prop_cells(b"msi-map", &[0x0, 2, 0x100, 0x10, 0x10, 3, 0x200, 0x10]);
        b.end();
        b.begin(b"msidev");
        b.prop_cells(b"msi-parent", &[2]);
        b.begin(b"fn@0");
        b.end();
        b.end();
        b.end();
        install(b.finish())
    }

    fn node(path: &[u8]) -> i32 {
        let n = OF_finddevice(path);
        assert!(n > 0, "{:?}", core::str::from_utf8(path));
        n
    }

    fn registry() -> &'static IntrRegistry {
        Box::leak(Box::new(IntrRegistry::new()))
    }

    fn register(r: &'static IntrRegistry, path: &[u8], ops: &'static Fake) -> IcRef {
        r.register_fdt(
            IcDesc {
                node: node(path),
                gic_its_id: 0,
                ops,
            },
            |_| {},
        )
    }

    #[test]
    fn the_parent_is_the_interrupt_parent_or_the_first_controller_up() {
        let _t = tree();
        let gic = node(b"/intc@0");
        let sub = node(b"/soc/sub@4");
        assert_eq!(arm_intr_get_parent(node(b"/soc/dev@0")), gic);
        assert_eq!(arm_intr_get_parent(node(b"/soc/child@5")), sub);
        assert_eq!(arm_intr_get_parent(node(b"/soc/sub@4/leaf@0")), sub);
        assert_eq!(arm_intr_get_parent(node(b"/pci")), 0);
    }

    #[test]
    fn registration_reads_the_cells_and_the_phandle_and_inserts_at_the_head() {
        let _t = tree();
        let r = registry();
        let (a, b) = (Fake::new(false), Fake::new(false));
        let seen = core::cell::Cell::new(None);
        let ra = r.register_fdt(
            IcDesc {
                node: node(b"/intc@0"),
                gic_its_id: 7,
                ops: a,
            },
            |ic| seen.set(Some(ic.phandle())),
        );
        assert_eq!(seen.get(), Some(1));
        assert_eq!((ra.cells(), ra.phandle(), ra.gic_its_id()), (3, 1, 7));
        let rb = register(r, b"/soc/sub@4", b);
        assert_eq!((rb.cells(), rb.phandle()), (1, 4));
        let order: Vec<i32> = r.controllers.iter().map(IcReg::node).collect();
        assert_eq!(order, vec![rb.node(), ra.node()]);
        assert!(ra.ops().legacy().is_none());
    }

    #[test]
    fn idx_picks_the_cells_of_the_plain_and_the_extended_properties() {
        let _t = tree();
        let r = registry();
        let gic = Fake::new(false);
        let sub = Fake::new(false);
        // No controller yet: the old style finds no parent.
        assert!(
            r.establish_fdt_idx_cpu(node(b"/soc/dev@0"), 0, 5, None, h(0), "a")
                .is_none()
        );
        register(r, b"/intc@0", gic);
        register(r, b"/soc/sub@4", sub);

        let dev = node(b"/soc/dev@0");
        let ih = r.establish_fdt_idx_cpu(dev, 1, 5, Some(1), h(0x40), "dev1");
        assert_eq!(ih.map(|ih| word(ih.cookie())), Some(1));
        assert!(
            r.establish_fdt_idx_cpu(dev, 2, 5, None, h(0), "dev2")
                .is_none()
        );
        // The empty phandle is interrupt 0 (the C's `for`): 1 and 2 are the GIC's.
        let ext = node(b"/soc/ext@1");
        assert!(
            r.establish_fdt_idx_cpu(ext, 0, 6, None, h(0), "ext0")
                .is_none()
        );
        assert!(
            r.establish_fdt_idx_cpu(ext, 2, 6, None, h(0), "ext2")
                .is_some()
        );
        assert!(
            r.establish_fdt_idx_cpu(ext, 1, 6, None, h(0), "ext1")
                .is_some()
        );
        // Too short for the controller's three cells; a length not of whole cells.
        let short = node(b"/soc/short@2");
        assert!(
            r.establish_fdt_idx_cpu(short, 0, 6, None, h(0), "s")
                .is_none()
        );
        let odd = node(b"/soc/odd@3");
        assert!(
            r.establish_fdt_idx_cpu(odd, 0, 6, None, h(0), "o")
                .is_none()
        );
        // A one-cell controller through `interrupt-parent`.
        let child = node(b"/soc/child@5");
        let ih = r
            .establish_fdt_idx_cpu(child, 0, 7, None, h(0), "child")
            .unwrap();
        assert_eq!(ih.controller().node(), node(b"/soc/sub@4"));

        assert_eq!(
            gic.calls(),
            vec![
                Call::Establish(vec![0, 6, 4], 5, Some(1), 0x40, "dev1"),
                Call::Establish(vec![0, 8, 4], 6, None, 0, "ext2"),
                Call::Establish(vec![0, 7, 4], 6, None, 0, "ext1"),
            ]
        );
        assert_eq!(
            sub.calls(),
            vec![Call::Establish(vec![2], 7, None, 0, "child")]
        );
    }

    #[test]
    fn the_handle_reaches_its_controller() {
        let _t = tree();
        let r = registry();
        let gic = Fake::new(false);
        register(r, b"/intc@0", gic);
        let ih = r
            .establish_fdt_idx_cpu(node(b"/soc/dev@0"), 0, 5, None, h(0), "d")
            .unwrap();
        arm_intr_route(ih, true, 3);
        arm_intr_disestablish_fdt(ih);
        intr_set_wakeup(ih);
        assert_eq!(
            ih.controller().ops().barrier(ih.cookie()),
            Some(BarrierCpu(Some(1)))
        );
        assert_eq!(
            gic.calls()[1..],
            [
                Call::Route(1, true, 3),
                Call::Disestablish(1),
                Call::Wakeup(1),
                Call::Barrier(1),
            ]
        );
    }

    #[test]
    fn a_preregistered_handler_is_established_when_its_controller_registers() {
        let _t = tree();
        let r = registry();
        r.init_fdt_recurse(OF_peer(0));
        // The dummies of the two `interrupt-controller` nodes, the MSI frames have none.
        assert_eq!(r.controllers.iter().count(), 2);

        let dev = node(b"/soc/dev@0");
        let first = r
            .establish_fdt_idx_cpu(dev, 0, 5, None, h(0x10), "pre0")
            .unwrap();
        let second = r
            .establish_fdt_idx_cpu(dev, 1, 5, Some(1), h(0x20), "pre1")
            .unwrap();
        let gone = r
            .establish_fdt_idx_cpu(dev, 1, 6, None, h(0x30), "gone")
            .unwrap();
        assert!(matches!(first.cookie(), IcCookie::Rec(_)));
        // Before the controller: nothing to wait for, the level keeps the flag, a
        // disestablish takes the record off the list.
        assert_eq!(first.controller().ops().barrier(first.cookie()), None);
        intr_set_wakeup(second);
        arm_intr_disestablish_fdt(gone);

        let gic = Fake::new(false);
        let real = register(r, b"/intc@0", gic);
        // The list's order: the latest first, as the C's LIST_INSERT_HEAD.
        assert_eq!(
            gic.calls(),
            vec![
                Call::Establish(vec![0, 6, 4], 5 | IPL_WAKEUP, Some(1), 0x20, "pre1"),
                Call::Establish(vec![0, 5, 4], 5, None, 0x10, "pre0"),
            ]
        );
        assert!(r.prereg.is_empty());

        // After it: the pre-registration forwards to the controller's cookie.
        assert_eq!(
            first.controller().ops().barrier(first.cookie()),
            Some(BarrierCpu(Some(1)))
        );
        intr_set_wakeup(first);
        arm_intr_disestablish_fdt(first);
        assert_eq!(
            gic.calls()[2..],
            [Call::Barrier(2), Call::Wakeup(2), Call::Disestablish(2)]
        );

        // The real controller shadows its dummy: a new handler goes to it directly.
        let direct = r
            .establish_fdt_idx_cpu(dev, 0, 5, None, h(0), "direct")
            .unwrap();
        assert!(core::ptr::eq(direct.controller(), real));
        assert!(matches!(direct.cookie(), IcCookie::Word(3)));
    }

    #[test]
    fn a_refused_preregistration_prints_the_c_line() {
        let _t = tree();
        let r = registry();
        r.init_fdt_recurse(OF_peer(0));
        let child = node(b"/soc/child@5");
        let pre = r
            .establish_fdt_idx_cpu(child, 0, 5, None, h(0), "refusedirq")
            .unwrap();
        let before = testutil::printf_count("can't establish interrupt refusedirq\n");
        let sub = Fake::new(true);
        register(r, b"/soc/sub@4", sub);
        assert_eq!(
            testutil::printf_count("can't establish interrupt refusedirq\n"),
            before + 1
        );
        assert_eq!(sub.calls().len(), 1);
        // Taken, with no cookie: nothing to forward to.
        assert_eq!(pre.controller().ops().barrier(pre.cookie()), None);
    }

    #[test]
    fn the_interrupt_map_matches_under_its_mask() {
        let _t = tree();
        let r = registry();
        let gic = Fake::new(false);
        register(r, b"/intc@0", gic);
        let pci = node(b"/pci");
        assert!(
            r.establish_fdt_imap_cpu(pci, &[0x0900, 0, 0, 1], 5, None, h(0), "m1")
                .is_some()
        );
        assert!(
            r.establish_fdt_imap_cpu(pci, &[0x0400, 0, 0, 1], 5, None, h(0), "m2")
                .is_some()
        );
        assert!(
            r.establish_fdt_imap_cpu(pci, &[0x0800, 0, 0, 2], 5, None, h(0), "m3")
                .is_none()
        );
        assert!(
            r.establish_fdt_imap_cpu(pci, &[0x0800, 0, 1], 5, None, h(0), "m4")
                .is_none()
        );
        assert_eq!(
            gic.calls(),
            vec![
                Call::Establish(vec![0, 0x21, 4], 5, None, 0, "m1"),
                Call::Establish(vec![0, 0x20, 4], 5, None, 0, "m2"),
            ]
        );
    }

    #[test]
    fn the_msi_map_and_the_msi_parent_name_the_frame() {
        let _t = tree();
        let pci = node(b"/pci");
        let mut data = 0x5;
        assert_eq!(arm_intr_map_msi(pci, &mut data), 2);
        assert_eq!(data, 0x105);
        // No `#msi-cells` on phandle 3: one cell.
        let mut data = 0x1_0012;
        assert_eq!(arm_intr_map_msi(pci, &mut data), 3);
        assert_eq!(data, 0x202);
        let mut data = 0x30;
        assert_eq!(arm_intr_map_msi(pci, &mut data), 0);
        assert_eq!(data, 0x30);
        let mut data = 9;
        assert_eq!(arm_intr_map_msi(node(b"/msidev/fn@0"), &mut data), 2);
        assert_eq!(data, 9);

        let r = registry();
        let frame = Fake::new(false);
        register(r, b"/msi@1", frame);
        let (mut addr, mut data) = (0, 0x7);
        let ih = r.establish_fdt_msi_cpu(pci, &mut addr, &mut data, 5, None, h(0), "msi");
        assert!(ih.is_some());
        assert_eq!((addr, data), (0x0802_0040, 0x1107));
        assert_eq!(frame.calls(), vec![Call::Msi(0, 0x107, "msi")]);
        // A frame that is not registered.
        let (mut addr, mut data) = (0, 0x12);
        assert!(
            r.establish_fdt_msi_cpu(pci, &mut addr, &mut data, 5, None, h(0), "none")
                .is_none()
        );
    }

    #[test]
    fn the_parent_of_a_forwarding_controller_takes_the_handler() {
        let _t = tree();
        let r = registry();
        let gic = Fake::new(false);
        let sub = Fake::new(false);
        register(r, b"/intc@0", gic);
        let rs = register(r, b"/soc/sub@4", sub);
        let ih = r
            .parent_establish_fdt(rs, &[0, 9, 4], 5, None, h(0), "fwd")
            .unwrap();
        assert_eq!(ih.controller().node(), node(b"/intc@0"));
        assert_eq!(
            gic.calls(),
            vec![Call::Establish(vec![0, 9, 4], 5, None, 0, "fwd")]
        );
    }
}
/* </TESTS> */
