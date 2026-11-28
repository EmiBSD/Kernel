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
//! arm64 `<machine/fdt.h>`: the device-tree attach arguments and the `fdt_intr_*` names.
//!
//! Upstream: sys/arch/arm64/include/fdt.h @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M4 ports `struct fdt_attach_args` and the `fdt_intr_*` aliases
//! of the `arm_intr_*_fdt` functions that exist; `stdout_node`, `stdout_speed`,
//! `fdt_cons_bs_tag` and `fdt_find_cons` are `arm64/machdep.rs` (and the `machine::fdt`
//! contract). M12 adds the `imap`/`msi` aliases, with PCI.
//!
//! Since N7.2b1b the root bus and the `simplebus*` row are the kernel's
//! (`kernel::dev::arm64::{mainbus, simplebus}`), which hand their children a typed
//! `kernel::dev::arm64::fdt::FdtArgs`; this file is also the legacy edge of those buses, for
//! the legacy children below them: the machine's presenter ([`present_aux`]: the typed
//! argument lent as a `struct fdt_attach_args` on the stack), the legacy tags of a native bus
//! (the root's `&arm64_bs_tag` and [`MAINBUS_DMA_TAG`], moved here from LZ's `mainbus.rs`; a
//! native simplebus's, its parent's copied with the four entries `simplebus_attach` replaces,
//! calling the kernel's translation), and a node's `dma-coherent` copy and `iommu_device_map`
//! (once per offered node, at the first legacy driver's look). Scaffolding
//! (docs/SCAFFOLDING.md, "The FDT root's legacy edge"), gone with the last legacy FDT child
//! (N7.14).
//!
//! ## Deviations
//! - `fa_reg`/`fa_nreg` and `fa_intr`/`fa_nintr` are slices; `fa_name` is a byte string.

pub use crate::arch::arm64::arm64::intr::{
    arm_intr_disable as fdt_intr_disable, arm_intr_disestablish_fdt as fdt_intr_disestablish,
    arm_intr_enable as fdt_intr_enable, arm_intr_establish_fdt as fdt_intr_establish,
    arm_intr_establish_fdt_cpu as fdt_intr_establish_cpu,
    arm_intr_establish_fdt_idx as fdt_intr_establish_idx,
    arm_intr_establish_fdt_idx_cpu as fdt_intr_establish_idx_cpu,
    arm_intr_establish_fdt_imap as fdt_intr_establish_imap,
    arm_intr_establish_fdt_imap_cpu as fdt_intr_establish_imap_cpu,
    arm_intr_establish_fdt_msi as fdt_intr_establish_msi,
    arm_intr_establish_fdt_msi_cpu as fdt_intr_establish_msi_cpu,
    arm_intr_get_parent as fdt_intr_get_parent,
    arm_intr_parent_disestablish_fdt as fdt_intr_parent_disestablish,
    arm_intr_parent_establish_fdt as fdt_intr_parent_establish,
    arm_intr_register_fdt as fdt_intr_register,
};
use core::any::Any;
use core::ffi::c_void;
use core::ops::Range;
use core::ptr;

use alloc::boxed::Box;
use hal::mem::malloc::{M_WAITOK, MArc};
use hal::mem::pool::AnyArc;
use kernel::dev::arm64::fdt::{FdtArgs, FdtBus, FdtBusRef};

use crate::arch::arm64::arm64::bus_dma::{
    _dmamap_create, _dmamap_destroy, _dmamap_load, _dmamap_load_buffer, _dmamap_load_mbuf,
    _dmamap_load_raw, _dmamap_load_uio, _dmamap_sync, _dmamap_unload, _dmamem_alloc,
    _dmamem_alloc_range, _dmamem_free, _dmamem_map, _dmamem_mmap, _dmamem_unmap,
};
use crate::arch::arm64::arm64::bus_space::ARM64_BS_TAG;
use crate::arch::arm64::include::bus::{self, BUS_DMA_COHERENT, BusDmamap, BusSpace};
use crate::dev::ofw::fdt::FdtReg;
use crate::dev::ofw::ofw_misc::iommu_device_map;
use crate::kern::subr_prf::panic;
use crate::machine::bus::{BusAddr, BusDmaTag, BusSpaceTag};
use crate::sys::malloc::M_DEVBUF;

/// `struct fdt_attach_args`: what a device-tree node's driver is attached with.
pub struct FdtAttachArgs<'a> {
    /// `fa_name`.
    pub fa_name: &'a [u8],
    /// `fa_node`: the node handle.
    pub fa_node: i32,
    /// `fa_iot`.
    pub fa_iot: BusSpaceTag,
    /// `fa_dmat`: the node's DMA tag (mainbus's, or a `dma-coherent` copy of it).
    pub fa_dmat: BusDmaTag,
    /// `fa_reg`: the node's `reg` entries (`fa_nreg` of them).
    pub fa_reg: &'a [FdtReg],
    /// `fa_intr`: the node's `interrupts` cells (`fa_nintr` of them).
    pub fa_intr: &'a [u32],
    /// `fa_acells`.
    pub fa_acells: i32,
    /// `fa_scells`.
    pub fa_scells: i32,
}
/// `mainbus_dma_tag`: the generic `bus_dma` functions, not cache-coherent; the root's DMA tag
/// a legacy child of the kernel's mainbus gets (`fa_dmat`), and the self-test's. Moved from
/// LZ's `mainbus.rs` with the root bus (N7.2b1b); a Rust `static` without interior mutability
/// is read-only, as the C puts it in `.rodata` by hand. Scaffolding until the last legacy FDT
/// child moves (docs/SCAFFOLDING.md, N7.14).
pub static MAINBUS_DMA_TAG: bus::BusDmaTag = bus::BusDmaTag {
    _cookie: ptr::null_mut(),
    _flags: 0,
    _dmamap_create,
    _dmamap_destroy,
    _dmamap_load,
    _dmamap_load_mbuf,
    _dmamap_load_uio,
    _dmamap_load_raw,
    _dmamap_load_buffer,
    _dmamap_unload,
    _dmamap_sync,
    _dmamem_alloc,
    _dmamem_alloc_range,
    _dmamem_free,
    _dmamem_map,
    _dmamem_unmap,
    _dmamem_mmap,
    _dma_mask: 0,
};

/// The legacy tags of one bus: what its legacy children get as `fa_iot` and `fa_dmat`.
struct BusTags {
    /// `fa_iot`.
    iot: &'static BusSpace,
    /// `fa_dmat`.
    dmat: &'static bus::BusDmaTag,
}

/// What a native simplebus's legacy tags reach through `bus_private` and `_cookie`: the
/// kernel's translation of the bus (one handle, leaked with the tags, so the bus lives for
/// ever as the C's softc) and the parent bus's tags the entries call after it (the C's
/// `sc_iot`, `sc_dmat`).
struct TagCookie {
    /// The bus's translation.
    bus: FdtBusRef,
    /// The parent's tags.
    parent: BusTags,
}

/// The cookie the tags of [`legacy_bus_tags`] point at: the `bus_private` of their bus space
/// and the `_cookie` of their DMA tag.
fn tag_cookie(p: *mut c_void) -> &'static TagCookie {
    // SAFETY: only `legacy_bus_tags` writes the entries that call this (`_space_map`,
    // `_space_mmap`, `_dmamap_load_buffer`, `_dmamap_load_raw` of its two leaked tables),
    // and it writes their `bus_private` and `_cookie` in the same initialiser, pointing at a
    // `TagCookie` it leaked: so the address is a live `TagCookie` for the rest of the kernel's
    // life, and only shared references to it are made (it holds a counted handle and two
    // `&'static` tables, all read-only). A copy of such a table that changes the cookie keeps
    // none of these entries callable with it: a child simplebus's copy (LZ's or the kernel's)
    // replaces all four with its own, the `dma-coherent` copy keeps the cookie, and smmu's
    // domain tag (`smmu.rs`), which replaces `_cookie`, replaces `_dmamap_load_raw` and every
    // entry that calls `_dmamap_load_buffer` through its own tag (`_dmamap_load`, `_mbuf`,
    // `_uio`) with ones that call the parent's (this) tag, as the C's; the same premise LZ's
    // `simplebus.rs` rests on (its `private_softc`).
    unsafe { &*p.cast_const().cast::<TagCookie>() }
}

/// The legacy tags of `bus`: the root's are `&arm64_bs_tag` and `&mainbus_dma_tag`; a native
/// simplebus's are its parent's copied with the four entries `simplebus_attach` replaces,
/// each calling the kernel's translation of one level ([`FdtBus::step`], `mmap_step`,
/// `dma_step`) and then the parent's entry, made at the first legacy child's look and kept
/// in [`FdtBus::legacy`] (leaked: the children keep them for ever, as the C's `sc_bus`).
fn legacy_bus_tags(bus: &FdtBusRef) -> BusTags {
    let Some(parent) = bus.parent() else {
        return BusTags {
            iot: &ARM64_BS_TAG,
            dmat: &MAINBUS_DMA_TAG,
        };
    };
    if let Some(t) = bus
        .legacy()
        .get()
        .and_then(|t| t.downcast_ref::<LeakedTags>())
    {
        return BusTags {
            iot: t.0,
            dmat: t.1,
        };
    }
    let up = legacy_bus_tags(parent);
    let cookie: &'static TagCookie = Box::leak(Box::new(TagCookie {
        bus: bus.clone(),
        parent: BusTags {
            iot: up.iot,
            dmat: up.dmat,
        },
    }));
    let private = ptr::from_ref(cookie).cast_mut().cast::<c_void>();
    let iot: &'static BusSpace = Box::leak(Box::new(BusSpace {
        bus_private: private,
        _space_map: |t, bpa, size, flag| {
            let c = tag_cookie(t.bus_private);
            let addr = c.bus.step(bpa as u64, size as u64)?;
            // SAFETY: this entry's caller made `bus_space_map`'s promise for the region (the
            // field's `unsafe fn` type); the parent's entry is handed the same region at the
            // address its bus gives it, the C's `bus_space_map(sc->sc_iot, ..)`.
            unsafe { (c.parent.iot._space_map)(c.parent.iot, addr as BusAddr, size, flag) }
        },
        _space_mmap: |t, bpa, off, prot, flags| {
            let c = tag_cookie(t.bus_private);
            let addr = c.bus.mmap_step(bpa as u64)?;
            (c.parent.iot._space_mmap)(c.parent.iot, addr as BusAddr, off, prot, flags)
        },
        ..*up.iot
    }));
    let dmat: &'static bus::BusDmaTag = Box::leak(Box::new(bus::BusDmaTag {
        _cookie: private,
        _dmamap_load_buffer: |t,
                              map,
                              buf,
                              buflen,
                              p,
                              flags,
                              lastaddrp,
                              segp,
                              usedp,
                              lastbouncep,
                              first| {
            let c = tag_cookie(t._cookie);
            let lastaddr = *lastaddrp;
            let mut firstseg = *segp as usize;
            let lastlen = map.seg(firstseg).get().ds_len;
            let up = c.parent.dmat;
            // SAFETY: this entry's caller made the parent's `_dmamap_load_buffer` promise (the
            // field's `unsafe fn` type), forwarded as it is, the C's
            // `sc->sc_dmat->_dmamap_load_buffer(sc->sc_dmat, ..)`.
            unsafe {
                (up._dmamap_load_buffer)(
                    up,
                    map,
                    buf,
                    buflen,
                    p,
                    flags,
                    lastaddrp,
                    segp,
                    usedp,
                    lastbouncep,
                    first,
                )
            }?;
            let Some(mut rlen) = c.bus.dma_cells() else {
                return Ok(());
            };
            // If we already translated the first segment, don't do it again!
            if !first && lastaddr == map.seg(firstseg).get()._ds_paddr + lastlen {
                firstseg += 1;
            }
            dma_translate(&c.bus, map, firstseg..*segp as usize + 1, &mut rlen);
            Ok(())
        },
        _dmamap_load_raw: |t, map, segs, size, flags| {
            let c = tag_cookie(t._cookie);
            let up = c.parent.dmat;
            // SAFETY: as for `_dmamap_load_buffer`: the caller's promise, forwarded.
            unsafe { (up._dmamap_load_raw)(up, map, segs, size, flags) }?;
            let Some(mut rlen) = c.bus.dma_cells() else {
                return Ok(());
            };
            dma_translate(&c.bus, map, 0..map.dm_nsegs.get() as usize, &mut rlen);
            Ok(())
        },
        ..*up.dmat
    }));
    let _ = bus.legacy().set(counted(LeakedTags(iot, dmat)));
    BusTags { iot, dmat }
}

/// [`FdtBus::legacy`]'s value: a bus's leaked legacy tags.
struct LeakedTags(&'static BusSpace, &'static bus::BusDmaTag);

/// The segments `segs` of `map` translated through the bus's `dma-ranges`, one `rlen` for
/// them all (the C's, not reset between segments).
fn dma_translate(bus: &FdtBus, map: &BusDmamap, segs: Range<usize>, rlen: &mut usize) {
    // For each segment.
    for seg in segs {
        let mut s = map.seg(seg).get();
        s.ds_addr = bus.dma_step(s.ds_addr as u64, s.ds_len as u64, rlen) as BusAddr;
        map.seg(seg).set(s);
    }
}

/// A value for a kernel set-once slot of `AnyArc`.
fn counted<T: Send + Sync + 'static>(v: T) -> AnyArc {
    match MArc::new(M_DEVBUF, M_WAITOK, v) {
        Some(a) => MArc::erase(a),
        None => panic(format_args!("fdt: out of memory for legacy tags")),
    }
}

/// The legacy tags of one offered node (`FdtArgs::legacy`'s value): its bus's, or the
/// `dma-coherent` copy of the DMA tag, after `iommu_device_map`.
struct NodeTags(BusTags);

/// The node's legacy tags, made at the first legacy look and kept for the call's others, so
/// the `dma-coherent` copy (leaked, as the C's: the child keeps it) and `iommu_device_map` run
/// once per offered node, as in `mainbus_attach_node` and `simplebus_attach_node`. The
/// pseudo-children (`efi`, `apm`: node 0) get their bus's as they are, as LZ's.
fn node_tags(fa: &FdtArgs) -> BusTags {
    if let Some(t) = fa.legacy().get().and_then(|t| t.downcast_ref::<NodeTags>()) {
        return BusTags {
            iot: t.0.iot,
            dmat: t.0.dmat,
        };
    }
    let bus = legacy_bus_tags(&fa.bus);
    if fa.node == 0 {
        return bus;
    }
    let mut dmat = bus.dmat;
    if fa.dma_coherent {
        dmat = Box::leak(Box::new(bus::BusDmaTag {
            _flags: dmat._flags | BUS_DMA_COHERENT,
            ..*dmat
        }));
    }
    let dmat = iommu_device_map(fa.node, dmat);
    let _ = fa
        .legacy()
        .set(counted(NodeTags(BusTags { iot: bus.iot, dmat })));
    BusTags { iot: bus.iot, dmat }
}

/// The machine's presenter (`machine::autoconf::present_aux`, arm64's): a native FDT bus's
/// [`FdtArgs`] lent to a legacy driver as LZ's `struct fdt_attach_args`, on the stack for the
/// call, with the node's legacy tags; nothing for an argument of another type. Scaffolding
/// (docs/SCAFFOLDING.md) until the last legacy FDT child moves (N7.14).
pub fn present_aux(aux: &(dyn Any + Send + Sync), f: &mut dyn FnMut(*mut c_void)) {
    let Some(fa) = aux.downcast_ref::<FdtArgs>() else {
        return;
    };
    let tags = node_tags(fa);
    let mut faa = FdtAttachArgs {
        fa_name: fa.name,
        fa_node: fa.node,
        fa_iot: tags.iot,
        fa_dmat: tags.dmat,
        fa_reg: fa.reg(),
        fa_intr: fa.intr(),
        fa_acells: fa.acells,
        fa_scells: fa.scells,
    };
    f(ptr::from_mut(&mut faa).cast());
}
/* </CODE> */
