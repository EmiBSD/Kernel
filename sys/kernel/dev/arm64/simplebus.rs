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
//! The device tree's `simple-bus` in the kernel: `arch/arm64/dev/simplebus.c`. Simplebus is a
//! generic bus with no special casings: it offers each child node to the drivers that attach at
//! `fdt`, in the three `early` passes mainbus makes, on a translation of its own
//! ([`FdtBus::child`]: the node's `ranges` and `dma-ranges` over its parent's). The interrupt
//! controllers with child nodes (the GICs' MSI frames) attach their children with
//! [`simplebus_attach`] too, on their own node.
//!
//! Upstream: sys/arch/arm64/dev/simplebus.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/include/simplebusvar.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/simplebus.rs@e8249449d905
//! LZ: sys/arch/arm64/include/simplebusvar.rs@e8249449d905
//!
//! ## Redesign
//! - `simplebus_ca`/`simplebus_cd` (a `struct cfattach` of `sizeof(struct simplebus_softc)`
//!   and the match and attach functions) -> [`SIMPLEBUS`], a [`DriverOps`] (decision 46): its
//!   probe reads the bus's [`FdtArgs`] back from [`Aux::Typed`] and answers 0 for any other
//!   argument (a legacy bus's, which `cfdata[]`'s `fdt?` parents also offer it).
//! - `struct simplebus_softc` (`machine/simplebusvar.h`: the node, the parent's tags, the
//!   cells, the tag copies, the range arrays, the pass under way and the early nodes, all
//!   members of a zeroed softc LZ wrote through `Cell`s and two `MaybeUninit` tables) -> the
//!   bus's [`FdtBus`] (the translation every child keeps, immutable once made), held by the
//!   children (the softc, [`Simplebus`], holds nothing); the pass and the early nodes are
//!   locals of [`simplebus_attach`], which nothing reads after it.
//! - `simplebus_bs_map`, `simplebus_bs_mmap`, `simplebus_dmamap_load_buffer` and
//!   `simplebus_dmamap_load_raw` (the tag entries that read `bus_private`/`_cookie` back as the
//!   softc) -> [`FdtBus::step`], [`FdtBus::mmap_step`] and [`FdtBus::dma_step`]: no cookie, no
//!   cast. A legacy child below this bus gets tags that call them from `bsd`'s presenter
//!   (`sys/bsd/arch/arm64/include/fdt.rs`; scaffolding).
//! - `simplebus_submatch` and `simplebus_print` -> closures of [`simplebus_attach_node`] over
//!   the node and the pass under way.
//! - N7.2b1b: `bsd`'s `simplebus.rs` and `simplebusvar.rs` stay whole beside this module for
//!   the two legacy GICs, which call LZ's `simplebus_attach` on themselves (P24), until
//!   N7.2a3; their `[[module.fn]]` rows go with them.
//!
//! ## Deviations
//! - A row without locators reads `early` 0, the locator's default, as LZ.
//! - The native row is never matched below a legacy GIC: the legacy simplebus's submatch
//!   calls `cf_attach->ca_match` by hand, which a native row's `CFATTACH_NATIVE` answers 0 for,
//!   so a `simple-bus` node below an interrupt controller would not attach (none does on any
//!   machine this kernel boots), until N7.2a3 makes the GICs native.
//! - A `simple-bus` node's own `iommus` is not applied to the tags its legacy children get
//!   (`bsd`'s legacy edge, `include/fdt.rs`): the C maps the bus node like any other and copies
//!   the result (`simplebus.c:82,110`), but an SMMU domain's tag keeps the outer bus's
//!   `_dmamap_load_buffer` with the domain as its cookie (`smmu.rs:669-679`), so an inner bus
//!   forwarding to it would hand that entry a cookie of another type (the C has the same
//!   latent confusion). No device tree this kernel boots puts `iommus` on a `simple-bus`.

use hal::mem::malloc::{M_NOWAIT, MArc};
use hal::mem::pool::AnyArc;

use crate::assert::kpanic;
use crate::base::prf::{Str, printf};
use crate::dev::arm64::fdt::{EarlyNodes, FdtArgs, FdtBus, FdtBusRef};
use crate::dev::autoconf::config_found_sm;
use crate::dev::device::{
    Aux, DV_DULL, DevRef, Devclass, DriverOps, Ident, M_DEVBUF, Match, QUIET, UNCONF, UnitTable,
};
use crate::dev::ofw::fdt::{
    OF_child, OF_getprop, OF_getproplen, OF_is_compatible, OF_is_enabled, OF_peer,
};

/// The softc of a `simplebus` node: nothing outlives the attach but the bus, which every
/// child holds (the C's `sc_bus` and `sc_dma` live in the softc because the children keep
/// their addresses).
pub struct Simplebus;

/// `simplebus_ca` with `simplebus_cd`'s constants.
pub struct SimplebusDriver;

/// The driver `cfdata[]`'s `simplebus* at fdt?` row names (`Cfdata::native`).
pub static SIMPLEBUS: SimplebusDriver = SimplebusDriver;

/// `simplebus_cd`'s `cd_devs`.
static SIMPLEBUS_UNITS: UnitTable = UnitTable::new();

/// A C string in a buffer, up to its first NUL.
fn cstr(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}

/// `simplebus_match`: simplebus is a generic bus with no special casings.
pub fn simplebus_match(fa: &FdtArgs) -> i32 {
    if fa.node == 0 {
        return 0;
    }

    // Qualcomm GENI can mostly be treated as simple-bus.
    if OF_is_compatible(fa.node, b"qcom,geni-se-qup") {
        return 1;
    }

    if !OF_is_compatible(fa.node, b"simple-bus") {
        return 0;
    }

    1
}

impl DriverOps for SimplebusDriver {
    fn name(&self) -> &'static [u8] {
        b"simplebus"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &SIMPLEBUS_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        match MArc::new(M_DEVBUF, M_NOWAIT, Simplebus) {
            Some(sc) => MArc::erase(sc),
            None => kpanic(format_args!(
                "config_make_softc: allocation for device softc failed"
            )),
        }
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        FdtArgs::of_aux(aux).map_or(0, simplebus_match)
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let Some(fa) = FdtArgs::of_aux(aux) else {
            kpanic(format_args!("{}: not a device-tree node", dev.xname()));
        };
        // The children hold the bus; nothing after the attach reads it (no detach).
        let _ = simplebus_attach(dev, fa);
    }
}

/// `simplebus_attach`: prints the node's name, makes the children's translation over the
/// parent's (`fa.bus`) and offers every child node, in three `early` passes; the bus is
/// returned for the caller to keep (the children hold their own handles). What a native
/// interrupt controller calls on its own node (N7.2a2, N7.2a3).
pub fn simplebus_attach(dev: &DevRef, fa: &FdtArgs) -> FdtBusRef {
    let mut name = [0u8; 32];

    if OF_getprop(fa.node, b"name", &mut name) > 0 {
        let last = name.len() - 1;
        name[last] = 0;
        printf(format_args!(": \"{}\"", Str(cstr(&name))));
    }

    printf(format_args!("\n"));

    let bus = FdtBus::child(&fa.bus, fa.node, fa.acells, fa.scells);

    // Scan the whole tree.
    let mut early_nodes = EarlyNodes::new();
    for early in (0..=2).rev() {
        let mut node = OF_child(fa.node);
        while node != 0 {
            simplebus_attach_node(dev, &bus, node, early, &mut early_nodes);
            node = OF_peer(node);
        }
    }
    bus
}

/// `simplebus_print`: names a child no driver took.
fn simplebus_print(node: i32, pnp: Option<&[u8]>) -> i32 {
    let mut name = [0u8; 32];

    let Some(pnp) = pnp else {
        return QUIET;
    };

    if OF_getprop(node, b"name", &mut name) > 0 {
        let last = name.len() - 1;
        name[last] = 0;
        printf(format_args!("\"{}\"", Str(cstr(&name))));
    } else {
        printf(format_args!("node {}", node as u32));
    }

    printf(format_args!(" at {}", Str(pnp)));

    UNCONF
}

/// `simplebus_attach_node`: look for a driver that wants to be attached to this node, among
/// the rows whose `early` locator is the pass under way (`simplebus_submatch`); a node
/// attached in an early pass is recorded and skipped by the later ones.
pub fn simplebus_attach_node(
    dev: &DevRef,
    bus: &FdtBusRef,
    node: i32,
    early: i32,
    early_nodes: &mut EarlyNodes,
) {
    if OF_getproplen(node, b"compatible") <= 0 {
        return;
    }

    if !OF_is_enabled(node) {
        return;
    }

    // Skip if already attached early.
    if early_nodes.contains(node) {
        return;
    }

    let fa = FdtArgs::of_node(node, bus, bus.acells(), bus.scells());
    let print = |_: &Aux<'_>, pnp: Option<&[u8]>| simplebus_print(node, pnp);
    let submatch = |parent: Option<&DevRef>, m: &Match, aux: &Aux<'_>| {
        let row = m.row();
        if row.loc(0).unwrap_or(0) == i64::from(early) {
            row.driver().probe(parent, m, aux)
        } else {
            0
        }
    };
    let child = config_found_sm(
        dev,
        &Aux::Typed(&fa),
        if early != 0 { None } else { Some(&print) },
        Some(&submatch),
    );

    // Record nodes that we attach early.
    if child.is_some() && early != 0 {
        early_nodes.record(node);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // simplebus's match over a tree built here; its passes, its print and the nodes it skips
    // run below mainbus in `kernel::dev::arm64::mainbus`'s tests (one fake machine per
    // process).

    use std::assert_eq;

    use super::*;
    use crate::dev::autoconf::tests::row_loc;
    use crate::dev::ofw::fdt::OF_finddevice;
    use crate::dev::ofw::fdt::testfdt::{FdtBuilder, install};
    use crate::testutil;

    #[test]
    fn simplebus_matches_simple_bus_and_qualcomm_geni() {
        testutil::install();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        for (name, compat) in [
            (&b"soc"[..], &b"vendor,soc\0simple-bus\0"[..]),
            (b"geni", b"qcom,geni-se-qup\0"),
            (b"uart", b"arm,pl011\0"),
        ] {
            b.begin(name);
            b.prop(b"compatible", compat);
            b.end();
        }
        b.end();
        let _g = install(b.finish());
        let root = FdtBus::root();
        let at = |path: &[u8]| simplebus_match(&FdtArgs::of_node(OF_finddevice(path), &root, 2, 2));
        assert_eq!(at(b"/soc"), 1);
        assert_eq!(at(b"/geni"), 1);
        assert_eq!(at(b"/uart"), 0);
        assert_eq!(simplebus_match(&FdtArgs::named(b"efi", &root)), 0);
        let m = Match::Row(row_loc(&SIMPLEBUS, &[], &[], 0, 0, 0));
        // Another bus's argument, or a legacy one, is not a device-tree node.
        assert_eq!(SIMPLEBUS.probe(None, &m, &Aux::Typed(&3u8)), 0);
        assert_eq!(SIMPLEBUS.probe(None, &m, &Aux::Legacy(0)), 0);
    }
}
/* </TESTS> */
