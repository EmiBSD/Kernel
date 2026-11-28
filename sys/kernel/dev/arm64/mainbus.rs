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
 * Copyright (c) 2016 Patrick Wildt <patrick@blueri.se>
 * Copyright (c) 2017 Mark Kettenis <kettenis@openbsd.org>
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
//! The arm64 root bus in the kernel: `arch/arm64/dev/mainbus.c`. Mainbus takes care of FDT
//! and non-FDT machines: it starts the interrupt controllers and the generic timer's delay,
//! then offers every node of the device tree to the drivers that attach at `fdt` (PSCI first,
//! the CPUs, `/firmware`, `/reserved-memory`, then the root's children in three passes,
//! `early 2`, `early 1` and the rest), and leaves the framebuffers in `/chosen` for after the
//! root file system is mounted.
//!
//! Upstream: sys/arch/arm64/dev/mainbus.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/mainbus.rs@e8249449d905
//!
//! ## Redesign
//! - `mainbus_ca`/`mainbus_cd` -> [`MAINBUS`], a [`DriverOps`] (decision 46): its probe is
//!   `mainbus_match`'s 1, its attach [`mainbus_attach`]; `cfdata[]`'s row 0 names it
//!   (`Cfdata::native`) with a stub `Cfdriver` of the name (`bsd`'s `conf/ioconf.rs`).
//! - `struct mainbus_softc` (a zeroed softc LZ wrote through `Cell`s: the root node, the two
//!   tags, the cells of the pass, the root's `ranges`, the pass under way and the early nodes)
//!   -> attach locals ([`Attach`], lent by `&mut` to the passes) and a softc, [`Mainbus`],
//!   that keeps only what outlives the attach: the root's bus and, set once when the passes
//!   end, the root's cells and the early nodes the deferred framebuffer pass reads.
//! - `fa_iot = &arm64_bs_tag`, `fa_dmat = &mainbus_dma_tag` (or a `dma-coherent` copy, then
//!   `iommu_device_map`) -> every child's [`FdtArgs`] names the root's [`FdtBus`]; the
//!   coherent copy is [`FdtArgs::dma_coherent`]. The legacy tables and `iommu_device_map`
//!   are `bsd`'s, made for every offered node through the `FdtOffer` bridge that
//!   [`FdtArgs::of_node`] calls, once per offer as the C's (`mainbus.c:273`), and lent to the
//!   legacy children by its presenter (`sys/bsd/arch/arm64/include/fdt.rs`, scaffolding);
//!   `mainbus_dma_tag` moved there.
//! - `mainbus_match_status`, `mainbus_match_primary` and `mainbus_match_secondary` (submatch
//!   functions that cast `aux` and the parent back) -> closures of [`mainbus_attach_node`]
//!   and [`mainbus_attach_cpus`] over the node's arguments and the pass; the boot CPU's id is
//!   `hal::cpu::hwid()` (`READ_SPECIALREG(mpidr_el1)`).
//! - `agtimer_init()` -> `kernel::time::clock::clock_early_init()` (the `ClockHardware`
//!   bridge's `early_init`, `bsd`'s agtimer until N7.2b2); `ncpusfound` is
//!   `kernel::base::sysctl::NCPUSFOUND`; `hw_prod` and `hw_serial` the kernel's sysctl
//!   strings, each a leaked `malloc(len, M_DEVBUF, M_NOWAIT)` copy as the C's.
//!
//! ## Deviations
//! - `thermal_init` (`ofw_thermal.c`) is reported.
//! - `cf_loc[0]` (the `early` locator) of an entry without locators reads as 0, the
//!   locator's default, as LZ.
//! - A node whose `reg` lines would be zero cells long gets no `reg`, where the C would divide
//!   by zero (`mainbus.c:229-230`), as LZ (`kernel::dev::arm64::fdt`).
//! - The root's `ranges` is not copied (`sc_ranges`, `M_TEMP`, which nothing in the C reads
//!   and never frees); `efi` and `apm` get the root's bus where the C gives `apm` none, as LZ.
//! - `hw_prod` and `hw_serial` are a `malloc`ed block of the property's length inside a box
//!   of its own (`MBox<MArray<u8>>`): one `M_DEVBUF` block more each than the C.
//! - `hw.vendor` and `hw.uuid` stay unset: OpenBSD sets them from `smbios0 at efi?`, which LZ
//!   has not ported.

use core::sync::atomic::Ordering;

use hal::mem::malloc::{M_NOWAIT, MArc, MArray, MBox};
use hal::mem::pool::AnyArc;
use hal::sync::once::OnceStatic;
use libkern::string::strlcpy::strlcpy;

use crate::assert::kpanic;
use crate::base::prf::{Str, printf, unported};
use crate::base::sysctl::{NCPUSFOUND, hw_prod, hw_serial};
use crate::dev::arm64::fdt::{EarlyNodes, FdtArgs, FdtBus, FdtBusRef};
use crate::dev::arm64::intr::arm_intr_init_fdt;
use crate::dev::autoconf::{Print, config_found, config_found_sm, config_mountroot};
use crate::dev::device::{
    Aux, DV_DULL, Deferred, DevRef, Devclass, DriverOps, Ident, M_DEVBUF, Match, QUIET, UNCONF,
    UnitTable,
};
use crate::dev::ofw::fdt::{
    OF_child, OF_finddevice, OF_getprop, OF_getpropint, OF_getproplen, OF_is_compatible,
    OF_is_enabled, OF_peer,
};
use crate::time::clock::clock_early_init;

/// `MPIDR_AFF` (`<machine/armreg.h>`): the affinity fields of `MPIDR_EL1` (Aff3, Aff2, Aff1,
/// Aff0), what a `/cpus` node's `reg` holds.
pub const MPIDR_AFF: u64 = 0xff_00ff_ffff;

/// The softc of mainbus: what outlives its attach.
pub struct Mainbus {
    /// The root's bus, made with the softc (`&arm64_bs_tag`, `&mainbus_dma_tag`).
    bus: FdtBusRef,
    /// The root's cells and the early nodes once the passes are done, for the deferred
    /// framebuffer pass.
    done: OnceStatic<Attached>,
}

/// What [`mainbus_attach`] leaves for [`mainbus_attach_framebuffer`].
struct Attached {
    /// The root's `#address-cells` and `#size-cells`.
    acells: i32,
    scells: i32,
    /// `sc_early_nodes`.
    nodes: EarlyNodes,
}

impl Mainbus {
    /// The root's bus: every child of the root sits on it.
    pub fn bus(&self) -> &FdtBusRef {
        &self.bus
    }
}

/// The state of one attach, the C's softc members that only the passes read (`sc_acells`,
/// `sc_scells`, `sc_early`, `sc_early_nodes`), lent to them by `&mut`.
pub struct Attach {
    /// The root's bus.
    bus: FdtBusRef,
    /// `sc_acells`: the `#address-cells` of the nodes being offered.
    acells: i32,
    /// `sc_scells`: their `#size-cells`.
    scells: i32,
    /// `sc_early`: the `early` locator of the pass under way.
    early: i32,
    /// `sc_early_nodes`.
    nodes: EarlyNodes,
}

/// `mainbus_ca` with `mainbus_cd`'s constants.
pub struct MainbusDriver;

/// The driver `cfdata[]`'s `mainbus0 at root` row names (`Cfdata::native`).
pub static MAINBUS: MainbusDriver = MainbusDriver;

/// `mainbus_cd`'s `cd_devs`.
static MAINBUS_UNITS: UnitTable = UnitTable::new();

/// A C string in a buffer, up to its first NUL.
fn cstr(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}

/// `mainbus_match`: mainbus takes care of FDT and non-FDT machines, so we always attach.
pub fn mainbus_match() -> i32 {
    1
}

impl DriverOps for MainbusDriver {
    fn name(&self) -> &'static [u8] {
        b"mainbus"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &MAINBUS_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        let sc = Mainbus {
            bus: FdtBus::root(),
            done: OnceStatic::new(),
        };
        match MArc::new(M_DEVBUF, M_NOWAIT, sc) {
            Some(sc) => MArc::erase(sc),
            None => kpanic(format_args!(
                "config_make_softc: allocation for device softc failed"
            )),
        }
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, _aux: &Aux<'_>) -> i32 {
        mainbus_match()
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, _aux: &Aux<'_>) {
        mainbus_attach(dev);
    }
}

/// `(struct mainbus_softc *)self`.
fn softc(dev: &DevRef) -> &Mainbus {
    match dev.softc().downcast_ref::<Mainbus>() {
        Some(sc) => sc,
        None => kpanic(format_args!("{}: not mainbus", dev.xname())),
    }
}

/// `s = malloc(len, M_DEVBUF, M_NOWAIT); if (s) strlcpy(s, prop, len);`: a copy of a
/// device-tree string that lives as long as the kernel, or `None` when memory is short.
fn kept_string(prop: &[u8], len: usize) -> Option<&'static [u8]> {
    let mut s = MArray::new(M_DEVBUF, M_NOWAIT, len, |_| 0u8)?;
    strlcpy(&mut s, prop);
    // Never freed, as the C never frees hw_prod and hw_serial.
    let s: &'static MArray<u8> = MBox::leak(MBox::new(M_DEVBUF, M_NOWAIT, s)?);
    Some(s)
}

/// `mainbus_attach`: the interrupt controllers and the timer's delay first, the model line,
/// then the device tree in the C's order.
pub fn mainbus_attach(dev: &DevRef) {
    let sc = softc(dev);
    let mut prop = [0u8; 128];

    arm_intr_init_fdt();
    clock_early_init();

    let root = OF_peer(0);
    let mut st = Attach {
        bus: sc.bus.clone(),
        acells: OF_getpropint(root, b"#address-cells", 1) as i32,
        scells: OF_getpropint(root, b"#size-cells", 1) as i32,
        early: 0,
        nodes: EarlyNodes::new(),
    };

    let len = OF_getprop(root, b"model", &mut prop);
    if len > 0 {
        printf(format_args!(": {}\n", Str(cstr(&prop))));
        // mainbus attaches once, on the boot CPU: hw_prod is set once.
        hw_prod.write(kept_string(&prop, len as usize));
    } else {
        printf(format_args!(": unknown model\n"));
    }

    let len = OF_getprop(root, b"serial-number", &mut prop);
    if len > 0 {
        hw_serial.write(kept_string(&prop, len as usize));
    }

    mainbus_attach_psci(dev, &mut st);
    mainbus_attach_efi(dev, &st);

    // Attach primary CPU first.
    mainbus_attach_cpus(dev, &mut st, true);

    // Attach secondary CPUs.
    mainbus_attach_cpus(dev, &mut st, false);

    mainbus_attach_firmware(dev, &mut st);
    mainbus_attach_resvmem(dev, &mut st);

    mainbus_attach_apm(dev, &st);

    // Scan the whole tree.
    for early in (0..=2).rev() {
        st.early = early;
        let mut node = OF_child(root);
        while node != 0 {
            mainbus_attach_node(dev, &mut st, node, None);
            node = OF_peer(node);
        }
        if early == 1 {
            // The early devices are attached (psci(4) in its own pass, or here): the SMCCC
            // call's record of a PSCI 0.1 firmware's ids closes, whether or not one attached.
            hal::cpu::smccc::seal_psci_0_1();
        }
    }
    st.early = 0;

    let _ = sc.done.set(Attached {
        acells: st.acells,
        scells: st.scells,
        nodes: st.nodes,
    });

    // Delay attaching the framebuffer to give other drivers a chance to claim it.
    config_mountroot(
        dev,
        Deferred {
            func: mainbus_attach_framebuffer,
            arg: None,
        },
    );

    let _ = unported!("thermal_init (ofw_thermal.c)");
}

/// `mainbus_print`: names a node no driver took, unless it is one nobody expects a driver
/// for.
pub fn mainbus_print(node: i32, pnp: Option<&[u8]>) -> i32 {
    let mut buf = [0u8; 32];

    let Some(pnp) = pnp else {
        return QUIET;
    };

    if !OF_is_enabled(node) {
        return QUIET;
    }

    if OF_getprop(node, b"name", &mut buf) > 0 {
        let last = buf.len() - 1;
        buf[last] = 0;
        let name = cstr(&buf);
        if name == b"aliases"
            || name == b"chosen"
            || name == b"cpus"
            || name == b"memory"
            || name == b"reserved-memory"
            || name == b"thermal-zones"
            || name.starts_with(b"__")
        {
            return QUIET;
        }
        printf(format_args!("\"{}\"", Str(name)));
    } else {
        printf(format_args!("node {}", node as u32));
    }

    printf(format_args!(" at {}", Str(pnp)));

    UNCONF
}

/// Which `/cpus` node a CPU pass offers (`mainbus_match_primary`, `_secondary`).
#[derive(Clone, Copy)]
pub enum CpuPass {
    /// The CPU node of the CPU we are running on.
    Primary,
    /// The other CPU nodes.
    Secondary,
}

/// `mainbus_attach_node`: look for a driver that wants to be attached to this node. Without
/// a CPU pass the candidates are the rows whose `early` locator is the pass under way, for an
/// enabled node (`mainbus_match_status`); in a CPU pass, the rows the node's first `reg`
/// selects against the running CPU's affinity (`mainbus_match_primary`, `_secondary`). A node
/// attached in an early pass is recorded and skipped by the later ones.
pub fn mainbus_attach_node(dev: &DevRef, st: &mut Attach, node: i32, cpus: Option<CpuPass>) {
    // Skip if already attached early.
    if st.nodes.contains(node) {
        return;
    }

    let fa = FdtArgs::of_node(node, &st.bus, st.acells, st.scells);
    let early = st.early;
    let print = |_: &Aux<'_>, pnp: Option<&[u8]>| mainbus_print(node, pnp);
    let submatch = |parent: Option<&DevRef>, m: &Match, aux: &Aux<'_>| {
        let row = m.row();
        let take = match cpus {
            None => OF_is_enabled(node) && row.loc(0).unwrap_or(0) == i64::from(early),
            Some(pass) => {
                let mpidr = hal::cpu::hwid();
                let ours = fa.reg().first().map(|r| r.addr == mpidr & MPIDR_AFF);
                match pass {
                    CpuPass::Primary => ours == Some(true),
                    CpuPass::Secondary => ours == Some(false),
                }
            }
        };
        if take {
            row.driver().probe(parent, m, aux)
        } else {
            0
        }
    };
    let print: Option<Print<'_>> = if cpus.is_none() && early == 0 {
        Some(&print)
    } else {
        None
    };

    let child = config_found_sm(dev, &Aux::Typed(&fa), print, Some(&submatch));

    // Record nodes that we attach early.
    if child.is_some() && early != 0 {
        st.nodes.record(node);
    }
}

/// `mainbus_attach_cpus`: offers the children of `/cpus` with its cells, the primary CPU's
/// node or the others', counting `ncpusfound`.
pub fn mainbus_attach_cpus(dev: &DevRef, st: &mut Attach, primary: bool) {
    let mut node = OF_finddevice(b"/cpus");
    let mut buf = [0u8; 32];

    if node == -1 {
        return;
    }

    let acells = st.acells;
    let scells = st.scells;
    st.acells = OF_getpropint(node, b"#address-cells", 2) as i32;
    st.scells = OF_getpropint(node, b"#size-cells", 0) as i32;

    let pass = if primary {
        CpuPass::Primary
    } else {
        CpuPass::Secondary
    };
    NCPUSFOUND.store(0, Ordering::Relaxed);
    node = OF_child(node);
    while node != 0 {
        if OF_getprop(node, b"device_type", &mut buf) > 0 && cstr(&buf) == b"cpu" {
            NCPUSFOUND.fetch_add(1, Ordering::Relaxed);
        }

        mainbus_attach_node(dev, st, node, Some(pass));
        node = OF_peer(node);
    }

    st.acells = acells;
    st.scells = scells;
}

/// `mainbus_attach_psci`: PSCI attaches first, in an early pass of its own.
pub fn mainbus_attach_psci(dev: &DevRef, st: &mut Attach) {
    let node = OF_finddevice(b"/psci");

    if node == -1 {
        return;
    }

    st.early = 1;
    mainbus_attach_node(dev, st, node, None);
    st.early = 0;
}

/// `mainbus_attach_efi`: the EFI runtime services, when the bootloader put their table in
/// `/chosen`.
pub fn mainbus_attach_efi(dev: &DevRef, st: &Attach) {
    let node = OF_finddevice(b"/chosen");

    if node == -1 || OF_getproplen(node, b"openbsd,uefi-system-table") <= 0 {
        return;
    }

    let fa = FdtArgs::named(b"efi", &st.bus);
    let _ = config_found(dev, &Aux::Typed(&fa), None);
}

/// `mainbus_attach_apm`.
pub fn mainbus_attach_apm(dev: &DevRef, st: &Attach) {
    let fa = FdtArgs::named(b"apm", &st.bus);

    let _ = config_found(dev, &Aux::Typed(&fa), None);
}

/// `mainbus_attach_framebuffer`: the `simple-framebuffer` nodes of `/chosen`, once the root
/// file system is mounted (`config_mountroot`).
pub fn mainbus_attach_framebuffer(dev: &DevRef, _arg: Option<&AnyArc>) {
    let sc = softc(dev);
    let mut node = OF_finddevice(b"/chosen");

    if node == -1 {
        return;
    }

    let Some(done) = sc.done.get() else {
        return;
    };
    // On some systems, such as the Raspberry Pi 5B, /chosen has its own #address-cells and
    // #size-cells that differ from the root node.
    let mut st = Attach {
        bus: sc.bus.clone(),
        acells: OF_getpropint(node, b"#address-cells", done.acells as u32) as i32,
        scells: OF_getpropint(node, b"#size-cells", done.scells as u32) as i32,
        early: 0,
        nodes: done.nodes,
    };

    node = OF_child(node);
    while node != 0 {
        if OF_is_compatible(node, b"simple-framebuffer") {
            mainbus_attach_node(dev, &mut st, node, None);
        }
        node = OF_peer(node);
    }
}

/// `mainbus_attach_firmware`: the children of `/firmware`.
pub fn mainbus_attach_firmware(dev: &DevRef, st: &mut Attach) {
    let mut node = OF_finddevice(b"/firmware");

    if node == -1 {
        return;
    }

    node = OF_child(node);
    while node != 0 {
        mainbus_attach_node(dev, st, node, None);
        node = OF_peer(node);
    }
}

/// `mainbus_attach_resvmem`: the children of `/reserved-memory`.
pub fn mainbus_attach_resvmem(dev: &DevRef, st: &mut Attach) {
    let mut node = OF_finddevice(b"/reserved-memory");

    if node == -1 {
        return;
    }

    node = OF_child(node);
    while node != 0 {
        mainbus_attach_node(dev, st, node, None);
        node = OF_peer(node);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // mainbus and simplebus over a tree built here (`testfdt`, no interrupt controller, so
    // `arm_intr_init_fdt` registers nothing) and the autoconfiguration tests' fake machine
    // (`setup_root`: this module's MAINBUS as `cfdata[0]`, leaf drivers below it that match
    // a `compatible` or a pseudo-child's name and log their attach): the attach order of
    // mainbus.c, the early passes and the early-node list, the CPU passes by the running CPU's
    // id, the framebuffer after the root is mounted, the print function, the identity strings
    // and `ncpusfound`; simplebus's passes below it. Under the page system's lock and the
    // tree's.

    use std::boxed::Box;
    use std::string::String;
    use std::sync::{Mutex, PoisonError};
    use std::vec::Vec;
    use std::{assert, assert_eq, format};

    use super::*;
    use crate::dev::arm64::simplebus::SIMPLEBUS;
    use crate::dev::autoconf::config_process_deferred_mountroot;
    use crate::dev::autoconf::config_rootfound;
    use crate::dev::autoconf::tests::{add_row, row_loc, setup_root};
    use crate::dev::device::FSTATE_STAR;
    use crate::dev::ofw::fdt::testfdt::{FdtBuilder, TreeGuard, install};
    use crate::testutil;

    /// What the leaves attached, in order: `xname node`.
    static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

    /// The `FdtOffer` bridge of the tests: the nodes offered, in order (every test that
    /// offers holds the tree's lock, so one test's offers are its own between two
    /// `take_offers`).
    struct Offers(Mutex<Vec<i32>>);

    impl crate::dev::arm64::fdt::FdtOffer for Offers {
        fn offered(&self, fa: &FdtArgs) {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(fa.node);
        }
    }

    static OFFERS: Offers = Offers(Mutex::new(Vec::new()));

    fn take_offers() -> Vec<i32> {
        core::mem::take(&mut *OFFERS.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn take_log() -> Vec<String> {
        core::mem::take(&mut *LOG.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// How a leaf recognises its node.
    enum Want {
        /// A node `compatible` with this.
        Compatible(&'static [u8]),
        /// A `/cpus` child of `device_type` "cpu".
        Cpu,
        /// A pseudo-child of this `fa_name` (`efi`, `apm`).
        Named(&'static [u8]),
    }

    /// A leaf driver: attaches where its `Want` matches, logs it.
    struct Leaf {
        name: &'static [u8],
        units: &'static UnitTable,
        want: Want,
    }

    impl DriverOps for Leaf {
        fn name(&self) -> &'static [u8] {
            self.name
        }
        fn class(&self) -> Devclass {
            DV_DULL
        }
        fn mode(&self) -> i32 {
            0
        }
        fn units(&self) -> &'static UnitTable {
            self.units
        }
        fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
            MArc::erase(MArc::new(M_DEVBUF, M_NOWAIT, ()).expect("softc"))
        }
        fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
            let Some(fa) = FdtArgs::of_aux(aux) else {
                return 0;
            };
            let mut buf = [0u8; 16];
            let yes = match self.want {
                Want::Compatible(c) => fa.node != 0 && OF_is_compatible(fa.node, c),
                Want::Cpu => {
                    fa.node != 0
                        && OF_getprop(fa.node, b"device_type", &mut buf) > 0
                        && cstr(&buf) == b"cpu"
                }
                Want::Named(n) => fa.name == n,
            };
            i32::from(yes)
        }
        fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
            let fa = FdtArgs::of_aux(aux).expect("an FDT child");
            assert!(MArc::ptr_eq(&fa.bus, softc_bus(dev)) || fa.bus.parent().is_some());
            printf(format_args!("\n"));
            LOG.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(format!("{} {}", dev.xname(), fa.node));
        }
    }

    /// The root bus below which `dev` sits (its parent's, for a child of mainbus).
    fn softc_bus(dev: &DevRef) -> &FdtBusRef {
        let mut d = dev;
        while let Some(p) = d.parent() {
            d = p;
        }
        softc(d).bus()
    }

    /// A starred leaf row below the rows `parents`, at pass `early`.
    fn leaf(name: &'static [u8], want: Want, parents: &'static [i16], early: &'static [i64]) {
        let drv: &'static Leaf = Box::leak(Box::new(Leaf {
            name,
            units: Box::leak(Box::new(UnitTable::new())),
            want,
        }));
        add_row(row_loc(drv, parents, early, 0, FSTATE_STAR, 0));
    }

    /// The machine: `cfdata[0]` mainbus, the leaves below it, a simplebus (row 10) with a
    /// leaf of its own (row 11).
    fn machine() -> std::sync::MutexGuard<'static, ()> {
        let mem = setup_root(&MAINBUS);
        crate::time::clock::tests::install_test_hw();
        let _ = crate::dev::arm64::fdt::install_fdt_offer(&OFFERS);
        let _ = take_log();
        leaf(b"cpu", Want::Cpu, &[0], &[]); // 1
        leaf(b"psci", Want::Compatible(b"arm,psci-1.0"), &[0], &[1]); // 2
        leaf(b"efi", Want::Named(b"efi"), &[0], &[]); // 3
        leaf(b"apm", Want::Named(b"apm"), &[0], &[]); // 4
        leaf(b"earlyd", Want::Compatible(b"test,early"), &[0, 10], &[1]); // 5
        leaf(b"lated", Want::Compatible(b"test,late"), &[0, 10], &[0]); // 6
        leaf(b"fw", Want::Compatible(b"test,fw"), &[0], &[]); // 7
        leaf(b"rsv", Want::Compatible(b"test,rsv"), &[0], &[]); // 8
        leaf(b"fb", Want::Compatible(b"simple-framebuffer"), &[0], &[]); // 9
        add_row(row_loc(&SIMPLEBUS, &[0, 10], &[0], 0, FSTATE_STAR, 0)); // 10
        leaf(b"sbleaf", Want::Compatible(b"test,sbleaf"), &[10], &[0]); // 11
        mem
    }

    /// The tree: a root with a model (when `model`) and a serial number, PSCI, `/chosen` with
    /// the UEFI table's property (when `efi`) and a framebuffer, three CPUs, `/firmware`,
    /// `/reserved-memory`, an early and a late node, a disabled one, one no driver takes, the
    /// memory node, and a simple-bus with a child, a node without `compatible` and a
    /// disabled one.
    fn tree(model: bool, efi: bool) -> TreeGuard {
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop_cells(b"#address-cells", &[2]);
        b.prop_cells(b"#size-cells", &[2]);
        if model {
            b.prop(b"model", b"b1b,test-board\0");
        }
        b.prop(b"serial-number", b"B1B-SN-42\0");
        b.begin(b"psci");
        b.prop(b"compatible", b"arm,psci-1.0\0");
        b.end();
        b.begin(b"chosen");
        if efi {
            b.prop_cells(b"openbsd,uefi-system-table", &[0, 0x4000_0000]);
        }
        b.begin(b"framebuffer@0");
        b.prop(b"compatible", b"simple-framebuffer\0");
        b.prop_cells(b"reg", &[0, 0x1000_0000, 0, 0x1000]);
        b.end();
        b.end();
        b.begin(b"cpus");
        b.prop_cells(b"#address-cells", &[1]);
        b.prop_cells(b"#size-cells", &[0]);
        for (name, id) in [(&b"cpu@0"[..], 0u32), (b"cpu@1", 1), (b"cpu@100", 0x100)] {
            b.begin(name);
            b.prop(b"device_type", b"cpu\0");
            b.prop_cells(b"reg", &[id]);
            b.end();
        }
        b.end();
        b.begin(b"firmware");
        b.begin(b"fw");
        b.prop(b"compatible", b"test,fw\0");
        b.end();
        b.end();
        b.begin(b"reserved-memory");
        b.begin(b"rsv@0");
        b.prop(b"compatible", b"test,rsv\0");
        b.end();
        b.end();
        b.begin(b"late@0");
        b.prop(b"compatible", b"test,late\0");
        b.end();
        b.begin(b"early@0");
        b.prop(b"compatible", b"test,early\0");
        b.end();
        b.begin(b"off@0");
        b.prop(b"compatible", b"test,late\0");
        b.prop(b"status", b"disabled\0");
        b.end();
        b.begin(b"b1bnobody@0");
        b.prop(b"compatible", b"test,nobody\0");
        b.end();
        b.begin(b"memory@40000000");
        b.prop(b"device_type", b"memory\0");
        b.end();
        b.begin(b"b1bsoc");
        b.prop(b"compatible", b"simple-bus\0");
        b.prop_cells(b"#address-cells", &[1]);
        b.prop_cells(b"#size-cells", &[1]);
        b.prop_cells(b"ranges", &[0, 0, 0x0a00_0000, 0x1000]);
        b.begin(b"b1bnocompat@0");
        b.end();
        b.begin(b"leaf@100");
        b.prop(b"compatible", b"test,sbleaf\0");
        b.prop_cells(b"reg", &[0x100, 0x10]);
        b.end();
        b.begin(b"leaf@200");
        b.prop(b"compatible", b"test,sbleaf\0");
        b.prop(b"status", b"disabled\0");
        b.end();
        b.begin(b"b1bsbnobody@300");
        b.prop(b"compatible", b"test,nobody\0");
        b.end();
        b.end();
        b.end();
        install(b.finish())
    }

    fn node(path: &[u8]) -> i32 {
        OF_finddevice(path)
    }

    #[test]
    fn mainbus_attaches_in_the_c_order() {
        let _m = machine();
        let _t = tree(true, true);
        hal::cpu::set_hwid(0x8000_0001); // MPIDR's RES1 bit 31 is masked off: cpu@1 boots
        let count = |s: &str| testutil::printf_count(s);
        let printed = count(": b1b,test-board\n");
        let lines = [
            "\"b1bnobody\" at mainbus0 not configured\n",
            "\"b1bsbnobody\" at simplebus0 not configured\n",
            "simplebus0 at mainbus0: \"b1bsoc\"\n",
            "\"b1bnocompat\"",
            "\"off\" at",
        ];
        let before: Vec<usize> = lines.iter().map(|l| count(l)).collect();
        let root = config_rootfound(b"mainbus", &Aux::Legacy(0)).expect("mainbus0");
        assert_eq!(root.xname(), "mainbus0");
        assert_eq!(testutil::printf_count(": b1b,test-board\n"), printed + 1);
        let want = [
            format!("psci0 {}", node(b"/psci")),
            String::from("efi0 0"),
            format!("cpu0 {}", node(b"/cpus/cpu@1")),
            format!("cpu1 {}", node(b"/cpus/cpu@0")),
            format!("cpu2 {}", node(b"/cpus/cpu@100")),
            format!("fw0 {}", node(b"/firmware/fw")),
            format!("rsv0 {}", node(b"/reserved-memory/rsv@0")),
            String::from("apm0 0"),
            // The early 1 pass: earlyd before lated, psci not offered again.
            format!("earlyd0 {}", node(b"/early@0")),
            format!("lated0 {}", node(b"/late@0")),
            format!("sbleaf0 {}", node(b"/b1bsoc/leaf@100")),
        ];
        assert_eq!(take_log(), want);
        assert_eq!(NCPUSFOUND.load(Ordering::Relaxed), 3);
        assert_eq!(hw_prod.read(), Some(&b"b1b,test-board\0"[..]));
        assert_eq!(hw_serial.read(), Some(&b"B1B-SN-42\0"[..]));
        // A node no driver takes is printed at its bus once (the last pass); a node without
        // `compatible` (simplebus) and a disabled one are not.
        let after: Vec<usize> = lines.iter().map(|l| count(l)).collect();
        let printed: Vec<usize> = after.iter().zip(&before).map(|(a, b)| a - b).collect();
        assert_eq!(printed, [1, 1, 1, 0, 0]);
        // The framebuffer waits for the root file system.
        config_process_deferred_mountroot();
        assert_eq!(
            take_log(),
            [format!("fb0 {}", node(b"/chosen/framebuffer@0"))]
        );
    }

    #[test]
    fn every_offer_is_looked_at_once_as_the_c_maps_it() {
        // mainbus.c:273 and simplebus.c:258: `iommu_device_map` once per offered node, every
        // pass, a disabled node included (its status is the submatch's test); simplebus skips
        // a node without `compatible` and a disabled one before it; an early-attached node is
        // not offered again; efi and apm are not nodes.
        let _m = machine();
        let _t = tree(true, true);
        hal::cpu::set_hwid(1);
        let _ = take_offers();
        let _root = config_rootfound(b"mainbus", &Aux::Legacy(0)).expect("mainbus0");
        config_process_deferred_mountroot();
        let offers = take_offers();
        let count = |path: &[u8]| offers.iter().filter(|&&n| n == node(path)).count();
        let want: [(&[u8], usize); 20] = [
            (b"/psci", 1),
            (b"/chosen", 3),
            (b"/chosen/framebuffer@0", 1),
            (b"/cpus", 3),
            (b"/cpus/cpu@0", 2),
            (b"/cpus/cpu@1", 2),
            (b"/cpus/cpu@100", 2),
            (b"/firmware", 3),
            (b"/firmware/fw", 1),
            (b"/reserved-memory", 3),
            (b"/reserved-memory/rsv@0", 1),
            (b"/late@0", 3),
            (b"/early@0", 2),
            (b"/off@0", 3),
            (b"/b1bnobody@0", 3),
            (b"/memory@40000000", 3),
            (b"/b1bsoc", 3),
            (b"/b1bsoc/leaf@100", 3),
            (b"/b1bsoc/b1bnocompat@0", 0),
            (b"/b1bsoc/leaf@200", 0),
        ];
        for (path, n) in want {
            assert_eq!(count(path), n, "{}", String::from_utf8_lossy(path));
        }
        assert_eq!(count(b"/b1bsoc/b1bsbnobody@300"), 3);
        assert!(!offers.contains(&0), "efi and apm are not offered nodes");
    }

    #[test]
    fn no_model_and_no_uefi_table() {
        let _m = machine();
        let _t = tree(false, false);
        hal::cpu::set_hwid(0x100);
        let unknown = testutil::printf_count(": unknown model\n");
        let _root = config_rootfound(b"mainbus", &Aux::Legacy(0)).expect("mainbus0");
        assert_eq!(testutil::printf_count(": unknown model\n"), unknown + 1);
        let log = take_log();
        assert!(!log.iter().any(|l| l.starts_with("efi")), "{log:?}");
        // The primary CPU is the one the running CPU's id names.
        assert_eq!(log[1], format!("cpu0 {}", node(b"/cpus/cpu@100")));
        assert_eq!(log[2], format!("cpu1 {}", node(b"/cpus/cpu@0")));
    }

    #[test]
    fn mainbus_print_is_quiet_for_the_expected_nodes() {
        testutil::install();
        let _t = tree(true, true);
        for path in [
            &b"/chosen"[..],
            b"/cpus",
            b"/memory@40000000",
            b"/reserved-memory",
        ] {
            assert_eq!(mainbus_print(node(path), Some(b"mainbus0")), QUIET);
        }
        assert_eq!(mainbus_print(node(b"/off@0"), Some(b"mainbus0")), QUIET);
        assert_eq!(mainbus_print(node(b"/late@0"), None), QUIET);
        let named = testutil::printf_count("\"late\" at b1bbus0");
        assert_eq!(mainbus_print(node(b"/late@0"), Some(b"b1bbus0")), UNCONF);
        assert_eq!(testutil::printf_count("\"late\" at b1bbus0"), named + 1);
        // A handle that names no node has no name: its number.
        let numbered = testutil::printf_count("node 777777 at b1bbus0");
        assert_eq!(mainbus_print(777_777, Some(b"b1bbus0")), UNCONF);
        assert_eq!(
            testutil::printf_count("node 777777 at b1bbus0"),
            numbered + 1
        );
    }
}
/* </TESTS> */
