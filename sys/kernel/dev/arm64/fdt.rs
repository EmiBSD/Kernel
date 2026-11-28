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
//! arm64's `<machine/fdt.h>` in the kernel: what a device-tree node's driver is attached with
//! (`struct fdt_attach_args`, [`FdtArgs`]), and the half of `<machine/simplebusvar.h>` a
//! child sees, the address translation of the bus it sits on ([`FdtBus`]: the C's
//! `fa_iot`/`fa_dmat`, a `simplebus` copy of its parent's tags with the `ranges` and
//! `dma-ranges` entries replaced). `mainbus(4)` and `simplebus(4)`
//! (`kernel::dev::arm64::{mainbus, simplebus}`) make both; a native child reads them back
//! from [`Aux::Typed`](crate::dev::device::Aux) with `downcast_ref::<FdtArgs>()`.
//!
//! Upstream: sys/arch/arm64/include/fdt.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/fdt.rs@e8249449d905
//!
//! The `fdt_intr_*` names of the header are `kernel::dev::arm64::intr`'s functions, as the C
//! defines them to the `arm_intr_*_fdt` ones; `stdout_node`, `stdout_speed` and
//! `fdt_find_cons` stay `bsd`'s `arm64/machdep.rs` with the console (N7.3c).
//!
//! ## Redesign
//! - LZ's `FdtAttachArgs<'a>` borrowed `malloc`ed arrays and two raw tag tables for the
//!   length of one `config_found_sm`: here [`FdtArgs`] is an owned, `'static` value under
//!   [`Aux::Typed`](crate::dev::device::Aux) (a bus's argument must be `Any`), its `reg` and
//!   `interrupts` [`MArray`]s (the C's `M_DEVBUF` blocks) freed when the bus drops it after
//!   the call, as the C's `free`s. A child keeps what it needs (a clone of the bus handle).
//! - `fa_iot` and `fa_dmat` (`bus_space_tag_t`, `bus_dma_tag_t`: tables of function pointers
//!   a `simplebus` copied from its parent with two entries replaced, `bus_private` and
//!   `_cookie` pointing back at its softc) -> one immutable chain, [`FdtBus`], a counted
//!   handle ([`FdtBusRef`]) the children clone: a level holds its node's cells and
//!   `ranges`/`dma-ranges`, and the replaced entries are its methods, [`FdtBus::step`]
//!   (`simplebus_bs_map`'s walk), [`FdtBus::mmap_step`] (`simplebus_bs_mmap`'s) and
//!   [`FdtBus::dma_step`] (one segment of `simplebus_dmamap_load_buffer`/`_raw`).
//!   [`FdtBus::map`] is `bus_space_map` below the chain: every level's walk up to the root,
//!   then `km::map_device` of the physical address. Not a trait (decision 46's census):
//!   `simplebus` is the only tag overrider on the FDT side, so the chain is one concrete type.
//! - The `dma-coherent` copy of the DMA tag (`BUS_DMA_COHERENT` in a `malloc`ed tag the child
//!   keeps) -> [`FdtArgs::dma_coherent`], read by whoever makes the child's DMA memory
//!   (`DmaReq::coherent`).
//! - Scaffolding (docs/SCAFFOLDING.md, "The FDT root's legacy edge"): [`FdtArgs::legacy`] and
//!   [`FdtBus::legacy`] are set-once slots `bsd`'s presenter fills with the legacy tags it
//!   lends a legacy driver (`sys/bsd/arch/arm64/include/fdt.rs`); nothing here reads them.
//!
//! ## Deviations
//! - A node whose `reg` lines would be zero cells long (`#address-cells` and `#size-cells`
//!   both 0) gets no `reg`, where the C's mainbus divides by zero (`mainbus.c:229-230`;
//!   simplebus tests `line > 0`), as LZ; negative cell counts (a malformed tree) read as no
//!   `reg` either.
//! - A `ranges` or `dma-ranges` whose length is not a whole number of cells reads as none, so
//!   [`FdtBus::step`] answers `ESRCH` where the C walks the NULL `sc_ranges`
//!   (`simplebus.c:102-108`, `:299-303`), LZ's unlisted behaviour (P28); a cell read past the
//!   property's end reads 0 (the C reads past the array), and an entry of zero cells is walked
//!   one cell at a time (the C loops for ever), as LZ.
//! - `simplebus_bs_mmap`'s `-1` and its `EINVAL` (an errno returned as a `paddr_t`) are both
//!   `None` ([`FdtBus::mmap_step`]), as LZ.
//! - `rlen`, the count of `dma-ranges` cells left, is not reset between the segments of one
//!   load, as the C's (`simplebus.c:403-407`, `:479-481`): [`FdtBus::dma_step`] takes it by
//!   `&mut` from its caller.
//! - The root's bus has no node and no `ranges` (mainbus's `sc_ranges` copy of the root's,
//!   which nothing in the C reads, is not made); `efi` and `apm` get the root's bus where the
//!   C gives `apm` no tags at all, since the typed form has no null, as LZ.

use core::any::Any;

use hal::dev::mmio::{DevCache, Mmio, RegBlock};
use hal::mem::malloc::{M_WAITOK, MArc, MArray};
use hal::mem::pool::AnyArc;
use hal::sync::once::OnceStatic;
use sysdefs::base::errno::Errno;
use sysdefs::base::types::Paddr;

use crate::assert::kpanic;
use crate::dev::device::{Aux, M_DEVBUF};
use crate::dev::ofw::fdt::{
    OF_getpropint, OF_getpropintarray, OF_getproplen, OF_is_compatible, OF_parent,
};

pub use crate::dev::ofw::fdt::FdtReg;

/// `M_TEMP` (`<sys/malloc.h>`): the cells read before they are parsed.
const M_TEMP: i32 = 127;

/// `nitems(sc_early_nodes)`: how many early-attached nodes a bus remembers.
pub const EARLY_NODES: usize = 64;

/// A counted handle of a bus's translation: what the C's children keep as `fa_iot` and
/// `fa_dmat` for their life.
pub type FdtBusRef = MArc<FdtBus>;

/// `struct fdt_attach_args`: what a device-tree node's driver is attached with. Owned and
/// `'static` (a bus's argument is [`Any`]); the bus drops it once `config_found_sm` returns,
/// so a child keeps what it needs (a clone of [`FdtArgs::bus`]).
pub struct FdtArgs {
    /// `fa_name`: `""` for a node, `"efi"` and `"apm"` for mainbus's two pseudo-children.
    pub name: &'static [u8],
    /// `fa_node`: the node's handle, 0 for `efi` and `apm`.
    pub node: i32,
    /// `fa_iot` and `fa_dmat`: the translation of the bus the node sits on.
    pub bus: FdtBusRef,
    /// The `BUS_DMA_COHERENT` of `fa_dmat`: the node has `dma-coherent`.
    pub dma_coherent: bool,
    /// `fa_acells`: the `#address-cells` of the node's `reg`.
    pub acells: i32,
    /// `fa_scells`: its `#size-cells`.
    pub scells: i32,
    /// `fa_reg`/`fa_nreg` (`M_DEVBUF`), `None` for none (the C's NULL, no block).
    reg: Option<MArray<FdtReg>>,
    /// `fa_intr`/`fa_nintr` (`M_DEVBUF`): the `interrupts` cells, `None` for none.
    intr: Option<MArray<u32>>,
    /// Scaffolding (docs/SCAFFOLDING.md): `bsd`'s legacy tags for this node, made at the
    /// first legacy driver's look (its presenter's) and kept for the call's other looks.
    legacy: OnceStatic<AnyArc>,
}

impl FdtArgs {
    /// The arguments of `node` on `bus`, its `reg` read with `acells` and `scells` (the
    /// parent's), as `mainbus_attach_node` and `simplebus_attach_node` read them: `reg`
    /// parsed from its cells, `interrupts` copied, `dma-coherent` noted.
    pub fn of_node(node: i32, bus: &FdtBusRef, acells: i32, scells: i32) -> FdtArgs {
        FdtArgs {
            name: b"",
            node,
            bus: bus.clone(),
            dma_coherent: OF_getproplen(node, b"dma-coherent") >= 0,
            acells,
            scells,
            reg: read_reg(node, acells, scells),
            intr: read_cells(node, b"interrupts", M_DEVBUF),
            legacy: OnceStatic::new(),
        }
    }

    /// A pseudo-child's arguments (`memset(&fa, 0, sizeof(fa)); fa.fa_name = name`): node 0,
    /// no registers, the root's `bus` (`mainbus_attach_efi`'s `sc_iot` and `sc_dmat`).
    pub fn named(name: &'static [u8], bus: &FdtBusRef) -> FdtArgs {
        FdtArgs {
            name,
            node: 0,
            bus: bus.clone(),
            dma_coherent: false,
            acells: 0,
            scells: 0,
            reg: None,
            intr: None,
            legacy: OnceStatic::new(),
        }
    }

    /// The `FdtArgs` a bus handed as `aux`, `None` for another bus's (or a legacy) argument:
    /// how an FDT child's probe tells its bus.
    pub fn of_aux<'a>(aux: &'a Aux<'_>) -> Option<&'a FdtArgs> {
        match aux {
            Aux::Typed(a) => {
                let a: &'a (dyn Any + Send + Sync) = *a;
                a.downcast_ref::<FdtArgs>()
            }
            Aux::Legacy(_) => None,
        }
    }

    /// `fa_reg[0 .. fa_nreg]`.
    pub fn reg(&self) -> &[FdtReg] {
        self.reg.as_deref().unwrap_or(&[])
    }

    /// `fa_intr[0 .. fa_nintr]`.
    pub fn intr(&self) -> &[u32] {
        self.intr.as_deref().unwrap_or(&[])
    }

    /// Scaffolding: the slot of `bsd`'s legacy tags for this node (its presenter's alone).
    pub fn legacy(&self) -> &OnceStatic<AnyArc> {
        &self.legacy
    }
}

/// `malloc(len, type, M_WAITOK)` of `n` cells, every one 0.
fn cells(type_: i32, n: usize) -> MArray<u32> {
    match MArray::new(type_, M_WAITOK, n, |_| 0) {
        Some(a) => a,
        None => kpanic(format_args!("fdt: out of memory for {n} cells")),
    }
}

/// A property of whole cells, copied (`malloc(len, type, M_WAITOK)` and
/// `OF_getpropintarray`), `None` when the node has none or its length is not a whole number
/// of cells.
fn read_cells(node: i32, prop: &[u8], type_: i32) -> Option<MArray<u32>> {
    let len = OF_getproplen(node, prop);
    if len <= 0 || len % 4 != 0 {
        return None;
    }
    let mut a = cells(type_, len as usize / 4);
    OF_getpropintarray(node, prop, &mut a);
    Some(a)
}

/// The node's `reg` with `acells` address and `scells` size cells per entry: each entry's
/// address and size from its first one or two cells (the C reads one cell for a count of 1
/// and two for 2; the first for any other positive count), `None` when the node has none,
/// when an entry would be zero cells long or the length is not a whole number of entries.
fn read_reg(node: i32, acells: i32, scells: i32) -> Option<MArray<FdtReg>> {
    let len = OF_getproplen(node, b"reg");
    let (Ok(a), Ok(s)) = (usize::try_from(acells), usize::try_from(scells)) else {
        return None;
    };
    let line = (a + s) * 4;
    let len = usize::try_from(len).ok().filter(|&l| l > 0)?;
    if line == 0 || len % line != 0 {
        return None;
    }
    let mut raw = cells(M_TEMP, len / 4);
    OF_getpropintarray(node, b"reg", &mut raw);
    let word = |i: usize| u64::from(raw.get(i).copied().unwrap_or(0));
    let regs = MArray::new(M_DEVBUF, M_WAITOK, len / line, |i| {
        let at = i * (a + s);
        let mut r = FdtReg::default();
        if a >= 1 {
            r.addr = word(at);
        }
        if a == 2 {
            r.addr = (r.addr << 32) | word(at + 1);
        }
        if s >= 1 {
            r.size = word(at + a);
        }
        if s == 2 {
            r.size = (r.size << 32) | word(at + a + 1);
        }
        r
    });
    match regs {
        Some(r) => Some(r),
        None => kpanic(format_args!(
            "fdt: out of memory for {} reg entries",
            len / line
        )),
    }
}

/// `sc_early_nodes[64]`: the nodes a bus attached in an early pass, which its later passes
/// skip; a 65th is not recorded, as the C's loop just ends.
#[derive(Clone, Copy)]
pub struct EarlyNodes {
    /// 0-terminated, as the C's array.
    nodes: [i32; EARLY_NODES],
}

impl EarlyNodes {
    /// None yet.
    pub const fn new() -> Self {
        Self {
            nodes: [0; EARLY_NODES],
        }
    }

    /// "Skip if already attached early": `node` is among them (the walk stops at the first
    /// empty slot).
    pub fn contains(&self, node: i32) -> bool {
        self.nodes
            .iter()
            .take_while(|&&n| n != 0)
            .any(|&n| n == node)
    }

    /// "Record nodes that we attach early": the first empty slot takes `node`; nothing when
    /// all 64 are taken.
    pub fn record(&mut self, node: i32) {
        if let Some(slot) = self.nodes.iter_mut().find(|n| **n == 0) {
            *slot = node;
        }
    }
}

impl Default for EarlyNodes {
    fn default() -> Self {
        Self::new()
    }
}

/// The address translation a bus gives its children: the root's (`&arm64_bs_tag`,
/// `&mainbus_dma_tag`: physical addresses, no translation), or a `simplebus`'s over its
/// parent's (`sc_bus`/`sc_dma` with `sc_ranges`/`sc_dmaranges`). Immutable once made, so any
/// CPU reads it as the C reads `sc_bus`.
pub struct FdtBus {
    /// The parent bus's translation; `None` for the root.
    parent: Option<FdtBusRef>,
    /// `sc_node`; 0 for the root.
    node: i32,
    /// `OF_parent(sc_node) == 0`: the bus's node is the tree's root, whose children's
    /// addresses `simplebus_bs_map` hands to the parent untranslated (read once here, where
    /// the C asks at each map).
    top: bool,
    /// `sc_acells`: the children's `#address-cells`.
    acells: i32,
    /// `sc_scells`: the children's `#size-cells`.
    scells: i32,
    /// `sc_pacells`: the parent's `#address-cells`.
    pacells: i32,
    /// `sc_rangeslen`: `ranges`' length in bytes, negative when the node has none.
    rangeslen: i32,
    /// `sc_ranges`: `ranges`' cells; `None` when it has none or not whole cells.
    ranges: Option<MArray<u32>>,
    /// `sc_dmaranges`: `dma-ranges`' cells (or the Raspberry Pi's); `None` for none.
    dmaranges: Option<MArray<u32>>,
    /// Scaffolding (docs/SCAFFOLDING.md): `bsd`'s legacy tags of this bus, made at the first
    /// legacy child's look and kept for ever.
    legacy: OnceStatic<AnyArc>,
}

/// One cell of a property as a `u64`; 0 past its end (see the module's deviations).
fn cell(r: &[u32], i: usize) -> u64 {
    r.get(i).copied().map_or(0, u64::from)
}

impl FdtBus {
    /// A new handle of `bus`, as a `malloc(9)`'d block that lives while a child holds it.
    fn counted(bus: FdtBus) -> FdtBusRef {
        match MArc::new(M_DEVBUF, M_WAITOK, bus) {
            Some(b) => b,
            None => kpanic(format_args!("fdt: out of memory for a bus")),
        }
    }

    /// The root's: mainbus's `&arm64_bs_tag` and `&mainbus_dma_tag`, which map and load
    /// physical addresses as they are.
    pub fn root() -> FdtBusRef {
        Self::counted(FdtBus {
            parent: None,
            node: 0,
            top: true,
            acells: 0,
            scells: 0,
            pacells: 0,
            rangeslen: 0,
            ranges: None,
            dmaranges: None,
            legacy: OnceStatic::new(),
        })
    }

    /// `simplebus_attach`'s tags for the bus at `node` below `parent`, whose own cells are
    /// `pacells` and `pscells`: the children's cells (the node's, defaulting to the parent's),
    /// `ranges`, `dma-ranges`, and the Raspberry Pi's missing `dma-ranges` supplied
    /// (`brcm,bcm2709`).
    pub fn child(parent: &FdtBusRef, node: i32, pacells: i32, pscells: i32) -> FdtBusRef {
        let rangeslen = OF_getproplen(node, b"ranges");
        let mut dmaranges = read_cells(node, b"dma-ranges", M_TEMP);
        // The device tree provided by the Raspberry Pi firmware lacks a "dma-ranges" option.
        // So provide the information until that gets fixed.
        if dmaranges.is_none() && OF_is_compatible(OF_parent(node), b"brcm,bcm2709") {
            let rpi = [0xc000_0000, 0x0000_0000, 0x3f00_0000];
            dmaranges = Some(match MArray::new(M_TEMP, M_WAITOK, 3, |i| rpi[i]) {
                Some(a) => a,
                None => kpanic(format_args!("fdt: out of memory for dma-ranges")),
            });
        }
        Self::counted(FdtBus {
            parent: Some(parent.clone()),
            node,
            top: OF_parent(node) == 0,
            acells: OF_getpropint(node, b"#address-cells", pacells as u32) as i32,
            scells: OF_getpropint(node, b"#size-cells", pscells as u32) as i32,
            pacells,
            rangeslen,
            ranges: read_cells(node, b"ranges", M_TEMP),
            dmaranges,
            legacy: OnceStatic::new(),
        })
    }

    /// The parent bus's translation; `None` for the root.
    pub fn parent(&self) -> Option<&FdtBusRef> {
        self.parent.as_ref()
    }

    /// `sc_node`: the bus's node (0 for the root).
    pub fn node(&self) -> i32 {
        self.node
    }

    /// `sc_acells`: the `#address-cells` of the children's `reg`.
    pub fn acells(&self) -> i32 {
        self.acells
    }

    /// `sc_scells`: their `#size-cells`.
    pub fn scells(&self) -> i32 {
        self.scells
    }

    /// The cells of one `ranges`/`dma-ranges` entry: child address, parent address, size.
    fn rone(&self) -> usize {
        let n = self.pacells.max(0) + self.acells.max(0) + self.scells.max(0);
        n as usize
    }

    /// The `ranges` entry covering `addr` (`covers(rfrom, rsize)`), translated: the loop of
    /// `simplebus_bs_map` and `simplebus_bs_mmap`.
    fn ranges_walk(&self, addr: u64, covers: impl Fn(u64, u64) -> bool) -> Option<u64> {
        let r = self.ranges.as_deref()?;
        let (a, p, s) = (
            self.acells.max(0) as usize,
            self.pacells.max(0) as usize,
            self.scells.max(0) as usize,
        );
        // An entry of zero cells is walked a cell at a time (the module's deviations).
        let step = self.rone().max(1);
        let mut rlen = r.len();
        let mut off = 0;
        // For each range.
        while rlen >= step {
            // Extract from and size, so we can see if we fit.
            let mut rfrom = cell(r, off);
            if a == 2 {
                rfrom = (rfrom << 32).wrapping_add(cell(r, off + 1));
            }
            let mut rsize = cell(r, off + a + p);
            if s == 2 {
                rsize = (rsize << 32).wrapping_add(cell(r, off + a + p + 1));
            }
            // Try next, if we're not in the range.
            if covers(rfrom, rsize) {
                // All good, extract to address and translate.
                let mut rto = cell(r, off + a);
                if p > 1 {
                    rto = (rto << 32).wrapping_add(cell(r, off + a + 1));
                }
                // Quietly drop the "flags" part of PCI addresses.
                if p > 2 {
                    rto = (rto << 32).wrapping_add(cell(r, off + a + 2));
                }
                return Some(addr.wrapping_sub(rfrom).wrapping_add(rto));
            }
            rlen -= step;
            off += step;
        }
        None
    }

    /// One level of `simplebus_bs_map`: `[addr, addr + size)` on this bus as its parent's
    /// address. The root, a bus whose node is the tree's root and a bus with an empty
    /// `ranges` pass it through; no `ranges` at all is `EINVAL`, no entry covering it
    /// `ESRCH`.
    pub fn step(&self, addr: u64, size: u64) -> Result<u64, Errno> {
        if self.parent.is_none() || self.top {
            return Ok(addr);
        }
        if self.rangeslen < 0 {
            return Err(Errno::EINVAL);
        }
        if self.rangeslen == 0 {
            return Ok(addr);
        }
        self.ranges_walk(addr, |rfrom, rsize| {
            addr >= rfrom && addr.wrapping_add(size) <= rfrom.wrapping_add(rsize)
        })
        .ok_or(Errno::ESRCH)
    }

    /// `bus_space_map`'s address on this bus as the physical one: [`FdtBus::step`] at every
    /// level up to the root, the first refusal answered.
    pub fn translate(&self, addr: u64, size: u64) -> Result<Paddr, Errno> {
        let mut a = addr;
        let mut bus = Some(self);
        while let Some(b) = bus {
            a = b.step(a, size)?;
            bus = b.parent.as_deref();
        }
        usize::try_from(a)
            .map(Paddr::new)
            .map_err(|_| Errno::EINVAL)
    }

    /// One level of `simplebus_bs_mmap`: `addr` on this bus as its parent's, `None` for the
    /// C's `-1` (no entry covers it) and its `EINVAL` (no `ranges`).
    pub fn mmap_step(&self, addr: u64) -> Option<u64> {
        if self.parent.is_none() || self.top {
            return Some(addr);
        }
        if self.rangeslen < 0 {
            return None;
        }
        if self.rangeslen == 0 {
            return Some(addr);
        }
        self.ranges_walk(addr, |rfrom, rsize| {
            addr >= rfrom && addr < rfrom.wrapping_add(rsize)
        })
    }

    /// `sc_dmarangeslen / sizeof(uint32_t)`: the `rlen` a load starts [`FdtBus::dma_step`]
    /// with, `None` when the bus has no `dma-ranges` (the C's `sc_dmaranges == NULL`: the
    /// parent's segments are the children's).
    pub fn dma_cells(&self) -> Option<usize> {
        self.dmaranges.as_ref().map(|r| r.len())
    }

    /// One segment of `simplebus_dmamap_load_buffer`/`_raw`, after the parent's load:
    /// `[addr, addr + size)` in the parent's address space as the children's, through the
    /// first `dma-ranges` entry that covers it, or `addr` when none does. `rlen` is the count
    /// of cells left, which the C carries from one segment to the next.
    pub fn dma_step(&self, addr: u64, size: u64, rlen: &mut usize) -> u64 {
        let Some(r) = self.dmaranges.as_deref() else {
            return addr;
        };
        let (a, p, s) = (
            self.acells.max(0) as usize,
            self.pacells.max(0) as usize,
            self.scells.max(0) as usize,
        );
        let rone = self.rone();
        let mut off = 0;
        // For each range.
        while *rlen >= rone && rone > 0 {
            // Extract from and size, so we can see if we fit.
            let mut rfrom = cell(r, off + a);
            if p > 1 {
                rfrom = (rfrom << 32).wrapping_add(cell(r, off + a + 1));
            }
            // Quietly drop the "flags" part of PCI addresses.
            if p > 2 {
                rfrom = (rfrom << 32).wrapping_add(cell(r, off + a + 2));
            }
            let mut rsize = cell(r, off + a + p);
            if s == 2 {
                rsize = (rsize << 32).wrapping_add(cell(r, off + a + p + 1));
            }
            // Try next, if we're not in the range.
            if addr < rfrom || addr.wrapping_add(size) > rfrom.wrapping_add(rsize) {
                *rlen -= rone;
                off += rone;
                continue;
            }
            // All good, extract to address and translate.
            let mut rto = cell(r, off);
            if a == 2 {
                rto = (rto << 32).wrapping_add(cell(r, off + 1));
            }
            return addr.wrapping_sub(rfrom).wrapping_add(rto);
        }
        addr
    }

    /// `bus_space_map(fa_iot, addr, size, flags, &ioh)`: the block `[addr, addr + size)` of
    /// this bus mapped as `R` with `cache`: [`FdtBus::translate`], then `km::map_device` of
    /// the physical address. Made once, at attach.
    pub fn map<R: RegBlock>(
        &self,
        addr: u64,
        size: u64,
        cache: DevCache,
    ) -> Result<Mmio<R>, Errno> {
        let pa = self.translate(addr, size)?;
        let size = usize::try_from(size).map_err(|_| Errno::EINVAL)?;
        crate::vm::km::map_device(pa, size, cache)
    }

    /// Scaffolding: the slot of `bsd`'s legacy tags of this bus (its presenter's alone).
    pub fn legacy(&self) -> &OnceStatic<AnyArc> {
        &self.legacy
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The attach arguments and the translation chain over trees built here (`testfdt`): `reg`
    // for every cell count the C reads, the early-node list's bound, and the `ranges` and
    // `dma-ranges` walks of simplebus.c's tags (the pass-through cases, `EINVAL`, `ESRCH`, one,
    // two and three parent cells, two levels, the mmap bound, the `rlen` carry and the Raspberry
    // Pi's missing `dma-ranges`). The tree is the process's: `install` serialises the tests.

    use std::vec::Vec;
    use std::{assert, assert_eq};

    use super::*;
    use crate::dev::ofw::fdt::OF_finddevice;
    use crate::dev::ofw::fdt::testfdt::{FdtBuilder, TreeGuard, install};
    use crate::testutil;

    /// A tree whose root holds `node` with `props`.
    fn one_node(props: &[(&[u8], &[u32])]) -> TreeGuard {
        testutil::install();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.begin(b"dev@0");
        for (name, cells) in props {
            b.prop_cells(name, cells);
        }
        b.end();
        b.end();
        install(b.finish())
    }

    fn regs(acells: i32, scells: i32) -> Vec<(u64, u64)> {
        let root = FdtBus::root();
        let fa = FdtArgs::of_node(OF_finddevice(b"/dev@0"), &root, acells, scells);
        fa.reg().iter().map(|r| (r.addr, r.size)).collect()
    }

    #[test]
    fn reg_is_read_with_the_parents_cells() {
        let _g = one_node(&[(b"reg", &[0x1, 0x2, 0x3, 0x4])]);
        // (1,1): two entries of one cell each.
        assert_eq!(regs(1, 1), [(1, 2), (3, 4)]);
        // (2,2): one entry of 64-bit address and size.
        assert_eq!(regs(2, 2), [(0x1_0000_0002, 0x3_0000_0004)]);
        // (2,0): two addresses, no size.
        assert_eq!(regs(2, 0), [(0x1_0000_0002, 0), (0x3_0000_0004, 0)]);
        // (0,0): an entry of no cells is no reg (the C's mainbus divides by zero).
        assert!(regs(0, 0).is_empty());
        // (2,1): 12 bytes do not divide 16: no reg.
        assert!(regs(2, 1).is_empty());
        // (3,1): the first cell of an address of three, as the C.
        assert_eq!(regs(3, 1), [(1, 4)]);
        // A negative count (a malformed tree) is no reg.
        assert!(regs(-1, 2).is_empty());
    }

    #[test]
    fn reg_two_one_and_the_other_properties() {
        let _g = one_node(&[
            (b"reg", &[0, 0x0900_0000, 0x1000]),
            (b"interrupts", &[0, 1, 4]),
            (b"dma-coherent", &[]),
        ]);
        let root = FdtBus::root();
        let fa = FdtArgs::of_node(OF_finddevice(b"/dev@0"), &root, 2, 1);
        assert_eq!(
            fa.reg(),
            [FdtReg {
                addr: 0x0900_0000,
                size: 0x1000
            }]
        );
        assert_eq!(fa.intr(), [0, 1, 4]);
        assert!(fa.dma_coherent);
        assert_eq!((fa.acells, fa.scells, fa.name), (2, 1, &b""[..]));
        assert!(MArc::ptr_eq(&fa.bus, &root));
        let efi = FdtArgs::named(b"efi", &root);
        assert_eq!((efi.node, efi.reg().len(), efi.intr().len()), (0, 0, 0));
        assert!(!efi.dma_coherent);
        // Typed arguments read back, a legacy one is no FDT bus's.
        assert!(FdtArgs::of_aux(&Aux::Typed(&efi)).is_some_and(|a| a.name == b"efi"));
        assert!(FdtArgs::of_aux(&Aux::Typed(&7u32)).is_none());
        assert!(FdtArgs::of_aux(&Aux::Legacy(0)).is_none());
    }

    #[test]
    fn the_sixty_fifth_early_node_is_not_recorded() {
        let mut e = EarlyNodes::new();
        assert!(!e.contains(5));
        for n in 1..=65 {
            e.record(n);
        }
        assert!(e.contains(1) && e.contains(64));
        assert!(!e.contains(65));
    }

    /// A bus `bus@0` below the root with `props`, and a grandchild bus `sub@0` below it with
    /// `sub` (none when empty), both made as simplebus makes them.
    fn buses(props: &[(&[u8], &[u32])], sub: &[(&[u8], &[u32])]) -> (TreeGuard, FdtBusRef) {
        testutil::install();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop_cells(b"#address-cells", &[2]);
        b.prop_cells(b"#size-cells", &[2]);
        b.begin(b"bus@0");
        for (name, cells) in props {
            b.prop_cells(name, cells);
        }
        if !sub.is_empty() {
            b.begin(b"sub@0");
            for (name, cells) in sub {
                b.prop_cells(name, cells);
            }
            b.end();
        }
        b.end();
        b.end();
        let g = install(b.finish());
        let root = FdtBus::root();
        let bus = FdtBus::child(&root, OF_finddevice(b"/bus@0"), 2, 2);
        (g, bus)
    }

    #[test]
    fn the_root_and_an_empty_ranges_pass_addresses_through() {
        testutil::install();
        let root = FdtBus::root();
        assert_eq!(root.step(0x1234, 8), Ok(0x1234));
        assert_eq!(root.mmap_step(0x1234), Some(0x1234));
        assert!(root.parent().is_none());
        let (_g, bus) = buses(&[(b"ranges", &[])], &[]);
        assert_eq!(bus.step(0x1234, 8), Ok(0x1234));
        assert_eq!(bus.mmap_step(0x1234), Some(0x1234));
        // The children's cells default to the parent's.
        assert_eq!((bus.acells(), bus.scells()), (2, 2));
    }

    #[test]
    fn no_ranges_is_einval_and_an_uncovered_address_esrch() {
        let (g, bus) = buses(&[], &[]);
        assert_eq!(bus.step(0x1000, 4), Err(Errno::EINVAL));
        assert_eq!(bus.mmap_step(0x1000), None);
        drop((bus, g));
        let (_g, bus) = buses(
            &[
                (b"#address-cells", &[1]),
                (b"#size-cells", &[1]),
                (b"ranges", &[0x1000_0000, 0, 0x0900_0000, 0x0100_0000]),
            ],
            &[],
        );
        assert_eq!(bus.step(0x2000_0000, 4), Err(Errno::ESRCH));
        // The last bytes of the window are in it, one past is not.
        assert_eq!(bus.step(0x10ff_fffc, 4), Ok(0x09ff_fffc));
        assert_eq!(bus.step(0x10ff_fffc, 8), Err(Errno::ESRCH));
        assert_eq!(bus.mmap_step(0x10ff_ffff), Some(0x09ff_ffff));
        assert_eq!(bus.mmap_step(0x1100_0000), None);
    }

    #[test]
    fn ranges_translate_with_one_two_and_three_parent_cells() {
        // Two parent cells (the root's), one child cell each.
        let (_g, bus) = buses(
            &[
                (b"#address-cells", &[1]),
                (b"#size-cells", &[1]),
                (b"ranges", &[0x1000_0000, 0, 0x0900_0000, 0x0100_0000]),
            ],
            &[],
        );
        assert_eq!(bus.step(0x1000_0010, 4), Ok(0x0900_0010));
        drop(bus);
        drop(_g);
        // One parent cell: a bus whose parent is the bus above, of one address cell.
        let (_g, bus) = buses(
            &[
                (b"#address-cells", &[1]),
                (b"#size-cells", &[1]),
                (b"ranges", &[0, 0, 0x4000_0000, 0x1000_0000]),
            ],
            &[
                (b"#address-cells", &[1]),
                (b"#size-cells", &[1]),
                (b"ranges", &[0x100, 0x0020_0000, 0x100]),
            ],
        );
        let sub = FdtBus::child(&bus, OF_finddevice(b"/bus@0/sub@0"), 1, 1);
        assert_eq!(sub.step(0x180, 4), Ok(0x0020_0080));
        // Two levels: the grandchild's address, then the child's window.
        assert_eq!(sub.translate(0x180, 4), Ok(Paddr::new(0x4020_0080)));
        assert_eq!(sub.translate(0x300, 4), Err(Errno::ESRCH));
        assert!(sub.parent().is_some_and(|p| MArc::ptr_eq(p, &bus)));
        drop((sub, bus));
        drop(_g);
        // Three parent cells (PCI's phys.hi, the flags, is dropped): child 2 cells, size 2.
        let (_g, bus) = buses(
            &[(b"#address-cells", &[2]), (b"#size-cells", &[2])],
            &[
                (b"#address-cells", &[2]),
                (b"#size-cells", &[2]),
                (
                    b"ranges",
                    &[0, 0x1000, 0x0200_0000, 0x1, 0x2000_0000, 0, 0x1000],
                ),
            ],
        );
        let sub = FdtBus::child(&bus, OF_finddevice(b"/bus@0/sub@0"), 3, 2);
        assert_eq!(sub.step(0x1800, 4), Ok(0x1_2000_0800));
    }

    #[test]
    fn a_ranges_of_a_bad_length_reads_as_none() {
        testutil::install();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.begin(b"bus@0");
        b.prop(b"ranges", &[0, 0, 0, 1, 0, 0]);
        b.end();
        b.end();
        let _g = install(b.finish());
        let root = FdtBus::root();
        let bus = FdtBus::child(&root, OF_finddevice(b"/bus@0"), 2, 2);
        assert_eq!(bus.step(0, 4), Err(Errno::ESRCH));
    }

    #[test]
    fn dma_ranges_translate_each_segment_and_carry_rlen() {
        // Two entries: child 0x8000_0000 <- parent 0x4000_0000 (1 MiB), child 0 <- parent
        // 0x6000_0000 (1 MiB); cells 1, 1, parent 2.
        let (_g, bus) = buses(
            &[
                (b"#address-cells", &[1]),
                (b"#size-cells", &[1]),
                (
                    b"dma-ranges",
                    &[
                        0x8000_0000,
                        0,
                        0x4000_0000,
                        0x10_0000,
                        0,
                        0,
                        0x6000_0000,
                        0x10_0000,
                    ],
                ),
            ],
            &[],
        );
        let mut rlen = bus.dma_cells().expect("dma-ranges");
        assert_eq!(rlen, 8);
        // A segment covered by the second entry walks past the first: rlen falls by one entry.
        assert_eq!(bus.dma_step(0x6000_1000, 0x1000, &mut rlen), 0x1000);
        assert_eq!(rlen, 4);
        // The next segment starts at the first entry again but with one entry's cells left
        // (the C's): the second window is out of its reach.
        assert_eq!(bus.dma_step(0x6000_2000, 0x1000, &mut rlen), 0x6000_2000);
        assert_eq!(rlen, 0);
        assert_eq!(bus.dma_step(0x4000_1000, 0x1000, &mut rlen), 0x4000_1000);
        // A fresh load sees both again.
        let mut rlen = bus.dma_cells().expect("dma-ranges");
        assert_eq!(bus.dma_step(0x4000_1000, 0x1000, &mut rlen), 0x8000_1000);
        assert_eq!(rlen, 8);
        // No dma-ranges: the parent's segments as they are.
        let root = FdtBus::root();
        assert_eq!(root.dma_cells(), None);
        assert_eq!(root.dma_step(0x1234, 4, &mut 0), 0x1234);
    }

    #[test]
    fn a_raspberry_pi_bus_gets_its_missing_dma_ranges() {
        testutil::install();
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop(b"compatible", b"raspberrypi,3-model-b\0brcm,bcm2709\0");
        b.begin(b"soc");
        b.prop_cells(b"#address-cells", &[1]);
        b.prop_cells(b"#size-cells", &[1]);
        b.end();
        b.end();
        let _g = install(b.finish());
        let root = FdtBus::root();
        let bus = FdtBus::child(&root, OF_finddevice(b"/soc"), 1, 1);
        let mut rlen = bus.dma_cells().expect("the fallback");
        assert_eq!(rlen, 3);
        // pacells 1 + acells 1 + scells 1: child 0xc000_0000 <- parent 0, 0x3f00_0000 long.
        assert_eq!(bus.dma_step(0x1000, 0x1000, &mut rlen), 0xc000_1000);
    }
}
/* </TESTS> */
