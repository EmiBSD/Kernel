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
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * All rights reserved.
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

/*
 * Copyright 2001 (c) Wasabi Systems, Inc.
 * All rights reserved.
 *
 * Written by Frank van der Linden for Wasabi Systems, Inc.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *      This product includes software developed for the NetBSD Project by
 *      Wasabi Systems, Inc.
 * 4. The name of Wasabi Systems, Inc. may not be used to endorse
 *    or promote products derived from this software without specific prior
 *    written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY WASABI SYSTEMS, INC. ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL WASABI SYSTEMS, INC
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

/*-
 * Copyright (c) 1998 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Charles M. Hannum.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! amd64's TLB: the `MULTIPROCESSOR` shootdown protocol of `pmap.c` and the MMU's own
//! instructions the pmap uses (`%cr3`, `invlpg`, the cache flushes).
//!
//! Upstream: sys/arch/amd64/amd64/pmap.c @ 3ce1f3f79392
//! Upstream: sys/arch/amd64/include/cpufunc.h @ 3ce1f3f79392
//! LZ: sys/arch/amd64/amd64/pmap.rs@3c62ede68802
//! LZ: sys/arch/amd64/include/cpufunc.rs@3c62ede68802
//!
//! The protocol, as the C's comment says: a shooting CPU takes `tlb_shoot_lock`, sets its
//! `tlb_shoot_counts[]` to the number of CPUs it will interrupt and sends them the IPI at
//! `splvm`; each target's stub (`vector.S`, `Xipi_invlpg`/`invlrange`/`invltlb`) flushes,
//! decrements the shooter's count and releases the lock with the last decrement. The call
//! that takes the lock is not the one that releases it, and nothing can block the IPI that
//! releases it. The shooter flushes its own TLB last, in parallel with the others;
//! `pmap_tlb_shootwait` spins until its count is 0. Page-table pages and pv entries are freed
//! only after it (`backend.rs`). A failure to send an IPI (only possible at boot) counts the
//! CPU as done (`pmap_tlb_shootfail`).
//!
//! The five words are `pub static`s of the C's shape, each on its own 64-byte line
//! ([`TlbShootWord`]): `vector.S` reaches them by symbol (`sym` in `bsd`'s
//! `sys/arch/amd64/amd64/vector.rs`), so their types and layout never change.
//!
//! ## Deviations
//! - PCID is off (`pmap_use_pcid` is never set: the C requires INVPCID, and nothing probes
//!   it): the `invpcid` branches and `tlb_shoot_first_pcid` are not there, where LZ kept them
//!   as `unported!` sites.
//! - `MP_LOCKDEBUG`'s spin-out into ddb is not configured.
//!
//! ## Redesign
//! - LZ walks `CPU_INFO_FOREACH` over `bsd`'s `struct cpu_info` (`ci_proc_pmap`, `ci_cpuid`,
//!   `cpu_is_running`) and calls `x86_fast_ipi`: `hal` reaches them through the
//!   [`crate::pmap::TlbCpus`] bridge (bridges.toml; paid by N4). Before it is registered one
//!   CPU runs and there is nobody to shoot at. `splvm()` is `hal::intr::splraise(IPL_VM)`,
//!   `curcpu()->ci_cpuid` is `hal::cpu::cpu_number()`, `CPU_BUSY_CYCLE()` is `spin_loop`.
//! - `rcr3`, `lcr3`, `invlpg`, `tlbflush`, `mfence`, `clflush` and `wbinvd` are private
//!   `asm!` here (`cpufunc.h`'s, split from `bsd`'s `include/cpufunc.rs`, which keeps them for
//!   its other users until N4): `lcr3` takes a pmap, whose root is a PML4 with the kernel
//!   half, so no caller can load anything else; `wbinvd_on_all_cpus` is the bridge's, or
//!   this CPU's `wbinvd` while one CPU runs.

use core::arch::asm;
#[cfg(feature = "multiprocessor")]
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use super::super::include::param::PAGE_SIZE;
#[cfg(feature = "multiprocessor")]
use super::super::include::vmparam::VM_MIN_KERNEL_ADDRESS;
#[cfg(feature = "multiprocessor")]
use crate::bridges;
use crate::pmap::Pmap;
#[cfg(feature = "multiprocessor")]
use crate::pmap::TlbIpi;

/// The CPUs `tlb_shoot_counts[]` has a word for: amd64's `MAXCPUS` (`<machine/cpu.h>`).
#[cfg(feature = "multiprocessor")]
pub const TLB_MAXCPUS: usize = 255;

/// `struct { volatile int x __attribute__((aligned(64))); }`: a shootdown word on a cache
/// line of its own, which the IPI stubs of `vector.S` reach by symbol.
#[cfg(feature = "multiprocessor")]
#[repr(C, align(64))]
pub struct TlbShootWord<T>(pub T);

/// `tlb_shoot_lock`: held from `pmap_start_tlb_shoot` until the last target acknowledges
/// (the IPI stubs release it).
#[cfg(feature = "multiprocessor")]
pub static TLB_SHOOT_LOCK: TlbShootWord<AtomicU32> = TlbShootWord(AtomicU32::new(0));
/// `tlb_shoot_cpu`: the `ci_cpuid` of the CPU holding `tlb_shoot_lock`.
#[cfg(feature = "multiprocessor")]
pub static TLB_SHOOT_CPU: TlbShootWord<AtomicU32> = TlbShootWord(AtomicU32::new(0));
/// `tlb_shoot_counts[]`: per shooting CPU, the targets that have not flushed yet.
#[cfg(feature = "multiprocessor")]
pub static TLB_SHOOT_COUNTS: [AtomicU32; TLB_MAXCPUS] = [const { AtomicU32::new(0) }; TLB_MAXCPUS];
/// `tlb_shoot_addr1`: the page, or the start of the range, to flush.
#[cfg(feature = "multiprocessor")]
pub static TLB_SHOOT_ADDR1: TlbShootWord<AtomicUsize> = TlbShootWord(AtomicUsize::new(0));
/// `tlb_shoot_addr2`: the end of the range to flush.
#[cfg(feature = "multiprocessor")]
pub static TLB_SHOOT_ADDR2: TlbShootWord<AtomicUsize> = TlbShootWord(AtomicUsize::new(0));

/// The targets of one shootdown: a bit per `ci_cpuid` (`u_int8_t mask[howmany(MAXCPUS, 8)]`).
#[cfg(feature = "multiprocessor")]
type ShootMask = [u8; TLB_MAXCPUS.div_ceil(8)];

/// `rcr3`: reads `CR3`.
pub(super) fn rcr3() -> u64 {
    let val: u64;
    // SAFETY: reading CR3 has no side effect and touches no memory; the kernel runs at CPL 0,
    // where the instruction is allowed.
    unsafe { asm!("mov {}, cr3", out(reg) val, options(nomem, nostack, preserves_flags)) };
    val
}

/// `lcr3(pmap's root)`: switches this CPU to `pm`'s address space (PCID off: no PCID bits).
pub(super) fn lcr3(pm: &Pmap) {
    let root = pm.md.root();
    // SAFETY: `root` is the root of a pmap (PmapMd's invariant: the PML4 `pmap_bootstrap`
    // adopted from the boot loader, which maps the running kernel, or one `create` built and
    // filled with the kernel pmap's whole upper half, the slots of `kernel_map`, the direct
    // map and the image; the root is set before the pmap is shared and freed only with the
    // pmap, which is not loaded anywhere then). The kernel's code, stack and data stay mapped
    // at the same addresses after the switch, so execution continues; only the user half
    // changes, which no Rust reference points into. Memory is a clobber (not `nomem`): the
    // switch must not be moved across the accesses it orders.
    unsafe { asm!("mov cr3, {}", in(reg) root, options(nostack, preserves_flags)) };
}

/// `tlbflush`: reloads `CR3`, dropping the non-global TLB entries.
pub(super) fn tlbflush() {
    // SAFETY: loading the value `CR3` already holds changes no mapping; it only drops cached
    // translations, which the MMU refetches from the same tables.
    unsafe {
        asm!("mov {tmp}, cr3", "mov cr3, {tmp}", tmp = out(reg) _, options(nostack, preserves_flags))
    };
}

/// `invlpg(va)`: drops this CPU's TLB entry of `va` (and its paging-structure caches).
pub(super) fn invlpg(va: usize) {
    // SAFETY: dropping a TLB entry changes no mapping and touches no data: the next access
    // refetches the translation from the tables. Any address is allowed (a non-canonical one
    // is a no-op).
    unsafe { asm!("invlpg [{}]", in(reg) va, options(nostack, preserves_flags)) };
}

/// `mfence`: orders every load and store before it against those after it.
pub(super) fn mfence() {
    // SAFETY: a fence changes no state; memory is a clobber (not `nomem`), as the C's.
    unsafe { asm!("mfence", options(nostack, preserves_flags)) };
}

/// `clflush(va)`: writes the cache line holding `va` back and drops it from every cache.
pub(super) fn clflush(va: usize) {
    // SAFETY: `clflush` changes no data, only where it is cached; an address that is not
    // mapped faults like any kernel access (a trap, then a panic), it corrupts nothing. Not
    // `nomem`: it must stay between the accesses to that line.
    unsafe { asm!("clflush [{}]", in(reg) va, options(nostack, preserves_flags)) };
}

/// `wbinvd`: writes back and invalidates this CPU's caches.
fn wbinvd() {
    // SAFETY: the caches are written back before they are invalidated, so no data is lost and
    // none changes; memory is a clobber.
    unsafe { asm!("wbinvd", options(nostack, preserves_flags)) };
}

/// `wbinvd_on_all_cpus()`: every CPU's caches written back and invalidated (the bridge's IPI;
/// this CPU's alone while the bridge is not registered: one CPU runs).
pub(super) fn wbinvd_on_all_cpus() {
    #[cfg(feature = "multiprocessor")]
    if let Some(b) = bridges::tlb_cpus() {
        b.wbinvd_all();
        return;
    }
    wbinvd();
}

/// `curcpu()->ci_cflushsz`: the `clflush` line size (0 without `clflush`, or while the bridge
/// is not registered: then the flush is a `wbinvd`, which is always correct).
pub(super) fn cflushsz() -> usize {
    #[cfg(feature = "multiprocessor")]
    if let Some(b) = bridges::tlb_cpus() {
        return b.cflushsz();
    }
    0
}

/// `pmap_update_pg(va)`: drops the TLB entry of one page.
fn pmap_update_pg(va: usize) {
    invlpg(va);
}

/// `pmap_is_active`: is this pmap loaded into the specified processor's `%cr3`? (`active` is
/// that CPU's `ci_proc_pmap`.)
#[cfg(feature = "multiprocessor")]
fn pmap_is_active(pm: &Pmap, active: *const Pmap) -> bool {
    pm.is_kernel() || core::ptr::eq(pm.as_ptr(), active)
}

/// `pmap_start_tlb_shoot`: obtain the "lock" for TLB shooting, and expect `targets` CPUs to
/// acknowledge.
#[cfg(feature = "multiprocessor")]
fn pmap_start_tlb_shoot(targets: u32) {
    let cpuid = crate::cpu::cpu_number();
    while TLB_SHOOT_LOCK
        .0
        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        while TLB_SHOOT_LOCK.0.load(Ordering::Relaxed) != 0 {
            core::hint::spin_loop();
        }
    }
    TLB_SHOOT_CPU.0.store(cpuid as u32, Ordering::Relaxed);
    shoot_count(cpuid).swap(targets, Ordering::SeqCst);
}

/// `tlb_shoot_counts[cpuid]`; a CPU id past the table is the machine's broken answer.
#[cfg(feature = "multiprocessor")]
fn shoot_count(cpuid: usize) -> &'static AtomicU32 {
    match TLB_SHOOT_COUNTS.get(cpuid) {
        Some(c) => c,
        None => crate::assert::hal_panic(format_args!(
            "pmap_tlb_shoot: cpu {cpuid} past tlb_shoot_counts"
        )),
    }
}

/// `pmap_tlb_shootwait`: waits until every CPU this CPU shot at has flushed.
pub fn pmap_tlb_shootwait() {
    #[cfg(feature = "multiprocessor")]
    {
        let count = shoot_count(crate::cpu::cpu_number());
        while count.load(Ordering::Acquire) > 0 {
            core::hint::spin_loop();
        }
    }
}

/// `pmap_tlb_shootfail`: a target could not take the IPI: count it as done.
#[cfg(feature = "multiprocessor")]
fn pmap_tlb_shootfail() {
    if shoot_count(crate::cpu::cpu_number()).fetch_sub(1, Ordering::AcqRel) == 1 {
        TLB_SHOOT_LOCK.0.store(0, Ordering::Release);
    }
}

/// The CPUs a shootdown of `pm` goes to: running, not this one, and (but for a kernel
/// address, `is_kva`) with `pm` active. The mask and the count.
#[cfg(feature = "multiprocessor")]
fn shoot_targets(pm: &Pmap, is_kva: bool) -> (ShootMask, u32) {
    let mut mask: ShootMask = [0; TLB_MAXCPUS.div_ceil(8)];
    let mut targets = 0;
    if let Some(b) = bridges::tlb_cpus() {
        b.for_each_other_running(&mut |id, active| {
            if id >= TLB_MAXCPUS {
                // Left out, it would keep a stale TLB: the machine broke its own bound.
                crate::assert::hal_panic(format_args!(
                    "pmap_tlb_shoot: cpu {id} past tlb_shoot_counts"
                ));
            }
            if !is_kva && !pmap_is_active(pm, active) {
                return;
            }

            mask[id / 8] |= 1 << (id % 8);
            targets += 1;
        });
    }
    (mask, targets)
}

/// Sends `ipi` to the CPUs of `mask`, counting each one that cannot take it as done.
#[cfg(feature = "multiprocessor")]
fn shoot_send(mask: &ShootMask, ipi: TlbIpi) {
    let Some(b) = bridges::tlb_cpus() else {
        return;
    };
    for id in (0..TLB_MAXCPUS).filter(|id| mask[id / 8] & (1 << (id % 8)) != 0) {
        if !b.send(id, ipi) {
            pmap_tlb_shootfail();
        }
    }
}

/// `pmap_tlb_shootpage`: drops one page's TLB entry here (when `shootself`) and, with
/// `MULTIPROCESSOR`, on every other CPU that may cache it.
pub fn pmap_tlb_shootpage(pm: &Pmap, va: usize, shootself: bool) {
    #[cfg(feature = "multiprocessor")]
    {
        let (mask, targets) = shoot_targets(pm, va >= VM_MIN_KERNEL_ADDRESS);
        if targets != 0 {
            let _s = crate::intr::splraise(crate::intr::IPL_VM);
            pmap_start_tlb_shoot(targets);
            TLB_SHOOT_ADDR1.0.store(va, Ordering::Relaxed);
            shoot_send(&mask, TlbIpi::Page);
        }
    }
    #[cfg(not(feature = "multiprocessor"))]
    let _ = pm;
    if shootself {
        pmap_update_pg(va);
    }
}

/// `pmap_tlb_shootrange`: drops a range's TLB entries (here when `shootself`, and on the
/// other CPUs with `MULTIPROCESSOR`).
pub fn pmap_tlb_shootrange(pm: &Pmap, sva: usize, eva: usize, shootself: bool) {
    #[cfg(feature = "multiprocessor")]
    {
        let (mask, targets) = shoot_targets(pm, sva >= VM_MIN_KERNEL_ADDRESS);
        if targets != 0 {
            let _s = crate::intr::splraise(crate::intr::IPL_VM);
            pmap_start_tlb_shoot(targets);
            TLB_SHOOT_ADDR1.0.store(sva, Ordering::Relaxed);
            TLB_SHOOT_ADDR2.0.store(eva, Ordering::Relaxed);
            shoot_send(&mask, TlbIpi::Range);
        }
    }
    #[cfg(not(feature = "multiprocessor"))]
    let _ = pm;
    if shootself {
        for va in (sva..eva).step_by(PAGE_SIZE) {
            pmap_update_pg(va);
        }
    }
}

/// `pmap_tlb_shoottlb`: drops the whole (non-global) TLB of a user pmap, here when
/// `shootself` and, with `MULTIPROCESSOR`, on the other CPUs that run it.
pub fn pmap_tlb_shoottlb(pm: &Pmap, shootself: bool) {
    #[cfg(feature = "multiprocessor")]
    {
        crate::assert::kassert!(!pm.is_kernel());
        let (mask, targets) = shoot_targets(pm, false);
        if targets != 0 {
            let _s = crate::intr::splraise(crate::intr::IPL_VM);
            pmap_start_tlb_shoot(targets);
            shoot_send(&mask, TlbIpi::Tlb);
        }
    }
    #[cfg(not(feature = "multiprocessor"))]
    let _ = pm;
    if shootself {
        tlbflush();
    }
}

#[cfg(feature = "multiprocessor")]
const _: () = {
    assert!(TLB_MAXCPUS <= crate::cpu::MAXCPUS);
    assert!(size_of::<TlbShootWord<AtomicU32>>() == 64);
    assert!(align_of::<TlbShootWord<AtomicUsize>>() == 64);
};
/* </CODE> */
