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
//! amd64's pmap behind `hal::pmap` (N3.1): `backend` (the `PmapBackend` implementation: pmaps,
//! pv lists, PTPs, the mapping functions), `tables` (the frame view of the direct map, the
//! walks, the PTE bits the boot path decides), `tlb` (the shootdown protocol and the MMU's
//! instructions), `window` (the ABI module's executable window) and `layout` (the slot layout
//! and the page-table geometry, pure constants, also compiled for the host tests).
//!
//! What amd64's own code (`bsd`'s `sys/arch/amd64/`, until it moves with its owners) names by
//! path: `PG_NX_BIT`, `PG_CRYPT`, `PG_G_KERN` (`locore0.S` by symbol, the trap code);
//! `TlbShootWord` and the five `TLB_SHOOT_*` (`vector.S` by symbol); the shootdown functions
//! (`ipi.c`'s self-test); `find_pte` (`bus_space`'s ISA hole check);
//! `KernelMapper::prealloc_lowmem_ptps` (machdep after boot(8)); `MODULE_WINDOW_SIZE`.

pub(crate) mod backend;
pub mod layout;
pub mod tables;
pub mod tlb;
mod window;

pub use backend::find_pte;
pub(crate) use backend::{Backend, PmapMd, PvHead};
pub use tables::{PG_CRYPT, PG_G_KERN, PG_NX_BIT};
#[cfg(feature = "multiprocessor")]
pub use tlb::{
    TLB_SHOOT_ADDR1, TLB_SHOOT_ADDR2, TLB_SHOOT_COUNTS, TLB_SHOOT_CPU, TLB_SHOOT_LOCK, TlbShootWord,
};
pub use tlb::{pmap_tlb_shootpage, pmap_tlb_shootrange, pmap_tlb_shoottlb, pmap_tlb_shootwait};
pub use window::MODULE_WINDOW_SIZE;
/* </CODE> */
