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
 * Copyright (c) 2008-2009,2014-2016 Dale Rahn <drahn@dalerahn.com>
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
//! arm64's address space identifiers: `pmap.c`'s ASID pairs, generations and rollover.
//!
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//!
//! We allocate ASIDs in pairs. The first ASID is used to run the kernel and has both userland
//! and the full kernel mapped. The second ASID is used for running userland and has only the
//! trampoline page mapped in addition to userland. A pmap's `pm_asid` is its even ASID with
//! the generation above `PMAP_ASID_MASK`; a pmap whose generation is old gets a new ASID when
//! it is next loaded (`pmap_setttb`). When none is free, a rollover starts a new generation:
//! the ASIDs active on some CPU carry over, every TLB is flushed, the others are free again.
//! `pmap_asid_mtx` (`IPL_HIGH`, taken on the context switch's path) guards the bitmap and the
//! rollover; the generation is read without it (`READ_ONCE(pmap_asid_gen)`).
//!
//! ## Deviations
//! - `pmap_rollover_asid` walks `CPU_INFO_FOREACH` for each CPU's `ci_curpm`; the CPUs'
//!   `struct cpu_info` is still `bsd`'s (N4), so the backend keeps its own record of each
//!   CPU's loaded pmap ([`set_curpm`], written by `Backend::load` where `pmap_setttb` writes
//!   `ci_curpm`, which the caller still does), indexed by `cpu_number()`.
//! - The trampoline pmap (`pmap_tramp`, the user ASID's kernel view) is not configured, as in
//!   LZ: the user ASID of a pair runs with the same tables (`TCR_EL1.E0PD` and the
//!   trampoline's `TTBR1_EL1` switch are N4.8d's, the user's decision of 2026-10-23).
//!
//! ## Redesign
//! - LZ caps the ASIDs at 256 (`PMAP_NASID`, an 8-word bitmap): the C's probe of
//!   `ID_AA64MMFR0_EL1` for 16 bits is done here ([`probe`]), with the 2,048-word bitmap of
//!   the C, and it also reads `TCR_EL1.AS`, since under Limine the boot loader's `TCR_EL1` is
//!   the kernel's ([`AsidWidth`] says which way it went).
//! - `pmap_asid[]` lives inside `pmap_asid_mtx` (`Mutexed<AsidMap>`), so the bitmap cannot be
//!   touched without the lock; `PMAP_ASID_GEN` stays an atomic beside it.
//! - `arc4random()` comes through the `Random` bridge (`sys/dev/rnd.rs`, N7); before it is
//!   registered the search goes linearly from the start.

use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicU8, AtomicU64, Ordering};

use super::cpufunc::{cpu_tlb_flush, cpu_tlb_flush_asid_all};
use super::layout::{
    ASID_USER, AsidWidth, PMAP_ASID_MASK, PMAP_MAX_NASID, asid_slot, nasid_probe, tlbi_asid,
};
use crate::assert::kassert;
use crate::bridges;
use crate::cpu::{MAXCPUS, cpu_number};
use crate::intr::IPL_HIGH;
use crate::mutex::{Mutexed, MutexedGuard};
use crate::pmap::Pmap;

/// `pmap_asid[]`: two bits per pmap (its kernel and user ASIDs), `PMAP_MAX_NASID` bits.
struct AsidMap {
    bits: [u32; PMAP_MAX_NASID / 32],
}

impl AsidMap {
    /// Whether the pair of `asid` is free.
    fn free(&self, asid: usize) -> bool {
        let (w, m) = asid_slot(asid);
        self.bits[w] & m == 0
    }

    /// Marks the pair of `asid` in use.
    fn take(&mut self, asid: usize) {
        let (w, m) = asid_slot(asid);
        self.bits[w] |= m;
    }
}

/// `pmap_asid_mtx` and the bitmap it guards.
static PMAP_ASID: Mutexed<AsidMap> = Mutexed::new(
    IPL_HIGH,
    AsidMap {
        bits: [0; PMAP_MAX_NASID / 32],
    },
);
/// `pmap_asid_gen`: the current generation (a multiple of `PMAP_MAX_NASID`); a fresh pmap's
/// zero never matches it.
static PMAP_ASID_GEN: AtomicU64 = AtomicU64::new(PMAP_MAX_NASID as u64);
/// `pmap_nasid`, as [`AsidWidth`]: 8 bits until the bootstrap's probe.
static PMAP_NASID: AtomicU8 = AtomicU8::new(WIDTH_8);
/// Each CPU's loaded pmap (its `ci_curpm`, see the deviations), null before its first load.
static CURPM: [AtomicPtr<Pmap>; MAXCPUS] = [const { AtomicPtr::new(ptr::null_mut()) }; MAXCPUS];

/// [`PMAP_NASID`]'s encodings.
const WIDTH_8: u8 = 0;
/// 8 bits for want of `TCR_EL1.AS`.
const WIDTH_8_TCR: u8 = 1;
/// 16 bits.
const WIDTH_16: u8 = 2;

/// `pmap_bootstrap`'s `id_aa64mmfr0_el1` probe (with the boot loader's `TCR_EL1`, see the
/// redesign), and ASID 0 marked in use: the kernel's.
pub(super) fn probe(id_aa64mmfr0: u64, tcr: u64) {
    let w = match nasid_probe(id_aa64mmfr0, tcr) {
        AsidWidth::Bits16 => WIDTH_16,
        AsidWidth::Bits8 => WIDTH_8,
        AsidWidth::Bits8TcrAs => WIDTH_8_TCR,
    };
    PMAP_NASID.store(w, Ordering::Relaxed);
    // Mark ASID 0 as in-use.
    PMAP_ASID.lock().take(0);
}

/// The ASIDs this machine runs with and why (the bootstrap's probe): no boot line prints it,
/// as OpenBSD prints none.
pub fn asid_width() -> AsidWidth {
    match PMAP_NASID.load(Ordering::Relaxed) {
        WIDTH_16 => AsidWidth::Bits16,
        WIDTH_8_TCR => AsidWidth::Bits8TcrAs,
        _ => AsidWidth::Bits8,
    }
}

/// `pmap_nasid`.
fn nasid() -> usize {
    asid_width().nasid()
}

/// `READ_ONCE(pmap_asid_gen)`.
pub(super) fn generation() -> u64 {
    PMAP_ASID_GEN.load(Ordering::Acquire)
}

/// Records `pm` as the pmap this CPU runs on (`ci->ci_curpm = pm`).
pub(super) fn set_curpm(pm: &Pmap) {
    CURPM[cpu_number()].store(ptr::from_ref(pm).cast_mut(), Ordering::Release);
}

/// Whether some CPU has `pm` loaded.
pub(super) fn loaded_anywhere(pm: &Pmap) -> bool {
    CURPM.iter().any(|c| ptr::eq(c.load(Ordering::Acquire), pm))
}

/// `pmap_find_asid`: a free pair: `pm`'s old one if it is free, else a random one, else the
/// first free; `None` when all are taken.
fn pmap_find_asid(map: &MutexedGuard<'_, AsidMap>, pm: &Pmap) -> Option<usize> {
    // Attempt to re-use the old ASID.
    let asid = (pm.md.asid() & PMAP_ASID_MASK) as usize;
    if map.free(asid) {
        return Some(asid);
    }

    let nasid = nasid();
    // Attempt to obtain a random ASID.
    if let Some(r) = bridges::random() {
        for _ in 0..5 {
            let asid = r.random_u32() as usize & (nasid - 2);
            if map.free(asid) {
                return Some(asid);
            }
        }
    }

    // Do a linear search if that fails.
    for asid in (0..nasid).step_by(32) {
        let bits = map.bits[asid / 32];
        if bits == !0 {
            continue;
        }
        for bit in (0..32).step_by(2) {
            if bits & (3u32 << bit) == 0 {
                return Some(asid + bit);
            }
        }
    }
    None
}

/// `pmap_rollover_asid`: a new generation; `pm`'s ASID in it.
fn pmap_rollover_asid(map: &mut MutexedGuard<'_, AsidMap>, pm: &Pmap) -> Option<usize> {
    // Start a new generation. Mark ASID 0 as in-use again.
    let generation =
        PMAP_ASID_GEN.fetch_add(PMAP_MAX_NASID as u64, Ordering::AcqRel) + PMAP_MAX_NASID as u64;
    let words = nasid() / 32;
    map.bits[..words].fill(0);
    map.take(0);

    // Carry over all the ASIDs that are currently active into the new generation and reserve
    // them. CPUs in cpu_switchto() will spin in pmap_setttb() waiting for the mutex. In that
    // case an old ASID will be carried over but that is not problematic.
    for c in &CURPM {
        let p = c.load(Ordering::Acquire);
        if p.is_null() {
            continue;
        }
        // SAFETY: `p` was recorded by `set_curpm` from a live `&Pmap` (with its provenance) when
        // a CPU loaded it; a record can change at any time without this lock (a load whose
        // generation matches takes none), but the pmap behind it cannot be freed while this
        // lock is held: a pmap is freed only by its last handle's drop, whose backend `destroy`
        // first refuses (a panic) while any record names it (Acquire loads pairing with
        // `set_curpm`'s Release stores, so a record that named it before this read is seen or
        // was replaced) and then takes `pmap_asid_mtx` in `pmap_free_asid`, before any table or
        // the pool item is freed. So a pmap this loop read from a record is either still named
        // by it, which `destroy` refuses, or its `destroy` waits for this lock: it is alive
        // until the rollover ends. No new record can name a pmap whose last handle is gone
        // (`load` needs a reference). The reference made is shared, its fields atomic.
        let curpm = unsafe { &*p };
        let asid = curpm.md.asid() & PMAP_ASID_MASK;
        curpm.md.set_asid(asid | generation);
        map.take(asid as usize);
    }

    // Flush the TLBs on all CPUs.
    cpu_tlb_flush();

    if pm.md.asid() & !PMAP_ASID_MASK == generation {
        return Some((pm.md.asid() & PMAP_ASID_MASK) as usize);
    }
    pmap_find_asid(map, pm)
}

/// `pmap_allocate_asid`: gives `pm` an ASID pair of the current generation.
pub(super) fn pmap_allocate_asid(pm: &Pmap) {
    let mut map = PMAP_ASID.lock();
    // We have no free ASIDs. Do a rollover to clear all inactive ASIDs and pick a fresh one.
    let asid = match pmap_find_asid(&map, pm) {
        Some(a) => a,
        None => pmap_rollover_asid(&mut map, pm).unwrap_or(0),
    };
    kassert!(asid > 0 && asid < nasid());
    map.take(asid);
    pm.md
        .set_asid(asid as u64 | PMAP_ASID_GEN.load(Ordering::Acquire));
}

/// `pmap_free_asid`: drops `pm`'s TLB entries on every CPU and frees its pair if it is of the
/// current generation.
pub(super) fn pmap_free_asid(pm: &Pmap) {
    kassert!(!ptr::eq(CURPM[cpu_number()].load(Ordering::Relaxed), pm));
    let asid = pm.md.asid();
    cpu_tlb_flush_asid_all(tlbi_asid(asid));
    cpu_tlb_flush_asid_all(tlbi_asid(asid | ASID_USER));

    let mut map = PMAP_ASID.lock();
    if asid & !PMAP_ASID_MASK == PMAP_ASID_GEN.load(Ordering::Acquire) {
        let (w, m) = asid_slot((asid & PMAP_ASID_MASK) as usize);
        map.bits[w] &= !m;
    }
}
/* </CODE> */
