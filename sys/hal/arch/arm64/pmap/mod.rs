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
/* </LICENSES> */

/* <CODE> */
//! arm64's pmap behind `hal::pmap` (N3.1 commit 5): `backend` (the `PmapBackend`
//! implementation: pmaps, pte descriptors, pv lists, the mapping functions, the bootstrap),
//! `vp` (the vp tables through the direct map: the frame view, the walks, the descriptor links),
//! `asid` (the ASID pairs, generations and rollover), `cpufunc` (the TLB, cache and
//! translation-register instructions), `window` (the ABI module's executable window beside
//! the image, N3.1 commit 7) and `layout` (the header's constants and the arithmetic that
//! needs no machine, pure, also compiled for the host tests).
//!
//! What arm64's own code (`bsd`'s `sys/arch/arm64/`, until it moves with its owners) names by
//! path: [`VMMAP`] and [`VMMAP_SIZE`] (`bus_space`'s device window); the cache modes and the
//! vp geometry of [`layout`] (`bus_space`, `bus_dma`, `smmu`); [`init_lower_half`] (the boot
//! glue's `pmap_init`, after `pluartcn_remap`); [`stolen_bounds`] (`pmap_steal_memory`'s
//! out-parameters); [`kernel_pt0pa`] (`cpu_init` and the secondary processors' start);
//! [`switch_mmu_kernel_table`] (the secondary processors' start, onto the kernel's tables);
//! [`asid_width`] (the probe's answer, never printed at boot); `Pmap::set_guarded` (`setregs`).

mod asid;
pub(crate) mod backend;
mod cpufunc;
pub mod layout;
mod vp;
mod window;

pub use asid::asid_width;
pub(crate) use backend::{Backend, PmapMd};
pub use backend::{
    VMMAP, VMMAP_SIZE, init_lower_half, kernel_pt0pa, stolen_bounds, switch_mmu_kernel_table,
};
pub(crate) use vp::PvHead;
/* </CODE> */
