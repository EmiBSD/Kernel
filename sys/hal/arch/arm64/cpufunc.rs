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

/*-
 * Copyright (c) 2014 Robin Randhawa
 * Copyright (c) 2015 The FreeBSD Foundation
 * All rights reserved.
 *
 * Portions of this software were developed by Andrew Turner
 * under sponsorship from the FreeBSD Foundation
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
/* </LICENSES> */
/* <CODE> */
//! arm64's cache, TLB and idle instructions for the code outside the pmap (`bus_dma`, the
//! interrupt controllers, the SMMU, the CPU and its idle loop): `cpufunc_asm.S`.
//!
//! Upstream: sys/arch/arm64/arm64/cpufunc_asm.S @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/cpufunc.rs@3c62ede68802
//!
//! Every routine here is safe for any argument: a TLB invalidation only forces refetches,
//! a cache operation by address writes back or drops clean copies (none discards a dirty
//! line), an address that is not mapped faults as any kernel access does, and `wfi` waits.
//! The operations that are not (`cpu_dcache_inv_range`'s `dc ivac`, which discards dirty
//! lines; `cpu_setttb`; the firmware calls `smc`/`hvc`) stay `bsd`'s until their callers
//! have typed ones.
//!
//! ## Deviations
//! - The `CPTAG_REPEAT_TLBI` code patch (an erratum workaround that repeats the `tlbi` on
//!   affected cores) is not applied, as in LZ.
//!
//! ## Redesign
//! - N4.8b: from `bsd`'s `cpufunc.rs`, which kept its own copies of the pmap's routines "until
//!   N4" (N3.1): the TLB flushes and the cache operations are the pmap's (`pmap/cpufunc.rs`,
//!   one `asm!` block per `cache_handle_range` as the C's macro, the line sizes from
//!   `CTR_EL0`), reached from here; `cpu_wfi` is this module's. `bsd`'s module keeps the
//!   names for its callers (re-exported) and the operations that stay its own. A zero length
//!   now handles one line, as the C's `cache_handle_range` (LZ's loop handled none).

use core::arch::asm;

use super::pmap::cpufunc as pmap;

/// `cpu_wfi`: `dsb sy; wfi`: waits for an interrupt, the default `cpu_idle_cycle_fcn`.
pub fn cpu_wfi() {
    // SAFETY: waiting for an interrupt changes no memory and no register; the barrier only
    // completes the CPU's pending accesses first. No stack, no flags.
    unsafe { asm!("dsb sy", "wfi", options(nomem, nostack, preserves_flags)) };
}

/// `cpu_tlb_flush`: invalidates every TLB entry of the inner-shareable domain.
#[inline]
pub fn cpu_tlb_flush() {
    pmap::cpu_tlb_flush();
}

/// `cpu_tlb_flush_asid(va)`: invalidates one page of one ASID (`va` carries the ASID in bits
/// 63:48 and the page number in 43:0).
#[inline]
pub fn cpu_tlb_flush_asid(va: u64) {
    pmap::cpu_tlb_flush_asid(va);
}

/// `cpu_tlb_flush_all_asid(va)`: invalidates one page for every ASID (kernel mappings).
#[inline]
pub fn cpu_tlb_flush_all_asid(va: u64) {
    pmap::cpu_tlb_flush_all_asid(va);
}

/// `cpu_tlb_flush_asid_all(asid)`: invalidates every entry of one ASID (bits 63:48).
#[inline]
pub fn cpu_tlb_flush_asid_all(asid: u64) {
    pmap::cpu_tlb_flush_asid_all(asid);
}

/// `cpu_dcache_wb_range(va, len)`: writes the dirty data cache lines of `[va, va + len)` back
/// to memory (`dc cvac`), for a device about to read it.
#[inline]
pub fn cpu_dcache_wb_range(va: usize, len: usize) {
    pmap::cpu_dcache_wb_range(va, len);
}

/// `cpu_dcache_wbinv_range(va, len)`: writes back and invalidates the data cache lines of
/// `[va, va + len)` (`dc civac`).
#[inline]
pub fn cpu_dcache_wbinv_range(va: usize, len: usize) {
    pmap::cpu_dcache_wbinv_range(va, len);
}

/// `cpu_icache_sync_range(va, len)`: makes instructions written to `[va, va + len)` visible to
/// instruction fetch (data cache cleaned to the point of unification, instruction cache
/// invalidated).
#[inline]
pub fn cpu_icache_sync_range(va: usize, len: usize) {
    pmap::cpu_icache_sync_range(va, len);
}

/// `cpu_idcache_wbinv_range(va, len)`: writes back and invalidates the data cache lines of
/// `[va, va + len)`, then invalidates its instruction cache lines; for a managed page about
/// to be mapped non-cacheable.
#[inline]
pub fn cpu_idcache_wbinv_range(va: usize, len: usize) {
    pmap::cpu_idcache_wbinv_range(va, len);
}
/* </CODE> */
