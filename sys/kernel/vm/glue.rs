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
 * Copyright (c) 1991, 1993, The Regents of the University of California.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * The Mach Operating System project at Carnegie-Mellon University.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)vm_glue.c	8.6 (Berkeley) 1/5/94
 * from: Id: uvm_glue.c,v 1.1.2.8 1998/02/07 01:16:54 chs Exp
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
 *
 * Permission to use, copy, modify and distribute this software and
 * its documentation is hereby granted, provided that both the copyright
 * notice and this permission notice appear in all copies of the
 * software, derivative works or modified versions, and any portions
 * thereof, and that both notices appear in supporting documentation.
 *
 * CARNEGIE MELLON ALLOWS FREE USE OF THIS SOFTWARE IN ITS "AS IS"
 * CONDITION.  CARNEGIE MELLON DISCLAIMS ANY LIABILITY OF ANY KIND
 * FOR ANY DAMAGES WHATSOEVER RESULTING FROM THE USE OF THIS SOFTWARE.
 *
 * Carnegie Mellon requests users of this software to return to
 *
 *  Software Distribution Coordinator  or  Software.Distribution@CS.CMU.EDU
 *  School of Computer Science
 *  Carnegie Mellon University
 *  Pittsburgh PA 15213-3890
 *
 * any improvements or extensions that they make and grant Carnegie the
 * rights to redistribute these changes.
 */
/* </LICENSES> */

/* <CODE> */
//! `uvm_glue.c`: glue functions between UVM and the rest of the kernel: wiring user memory
//! for I/O (`uvm_vslock`, `uvm_vslock_device`), the u-areas (`uvm_uarea_alloc`,
//! `uvm_uarea_free`) and the initial limits' numbers (`uvm_init_limits`).
//!
//! Upstream: sys/uvm/uvm_glue.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_glue.rs@3c62ede68802
//!
//! ## Deviations
//! - `__HAVE_USPACE_GUARD`'s guard page is not carved out: the carve-out is reported once and
//!   the u-area has no guard (LZ's, kept visible).
//!
//! ## Redesign
//! - The functions take the address space (`&Vmspace`, `&VmMap`) instead of a `struct proc
//!   *`: `bsd`'s callers pass `p->p_vmspace`. `uvm_purge`, `uvm_exit` and `uvm_init_limits`
//!   are operations on `struct proc` and `struct plimit`, which stay `bsd`'s
//!   (`sys/kern/vm_bsd.rs`); [`default_limits`] gives `uvm_init_limits` its numbers.
//! - `uvm_vslock_device`'s result is a [`DeviceLock`]: the map's read guard (held from the
//!   wiring to [`uvm_vsunlock_device`], as the C keeps the map read-locked across the
//!   transfer) and the bounce buffer, a [`KmBuf`] of DMA-reachable wired pages
//!   (`vm::km::dma_memory`) instead of `km_alloc(kp_none)`, `uvm_pglistalloc(dma_constraint)`
//!   and `pmap_kenter_pa` per page: the same pages, mapped, owned, freed by its drop. LZ's five
//!   `unsafe` blocks (the two `pmap_kenter_pa`/`pmap_kremove` and the two slices over the
//!   bounce, the `ps_vmspace` deref) are gone. The guard order is the C's: unwire, unlock,
//!   then the bounce is freed.
//! - A u-area is a [`Vaddr`] (`bsd` keeps `p_addr`).

use hal::arch::current::vmparam::{DFLDSIZ, DFLSSIZ, MAXDSIZ, MAXSSIZ};
use hal::kmbuf::KmBuf;
use hal::rwlock::RwMode;
use hal::sys::errno::Errno;
use hal::sys::types::{Paddr, Vaddr};
use hal::user::UserPtr;
use hal::vm::param::{atop, ptoa, round_page, trunc_page};
use hal::vm::uvmexp::UVMEXP;

use super::fault::{FaultMap, FaultMapGuard, uvm_fault_unwire_locked, uvm_fault_wire};
use super::init::machine;
use super::km::{KD_WAITOK, KP_ZERO, KmemVaMode, KvMap, dma_memory, km_alloc, km_free};
use super::map::{VmMap, uvm_map_pageable};
use super::types::VmProt;
use crate::prf::unported;

/// The u-area mode (`kv_uarea`): `kernel_map`, `USPACE_ALIGN`ed.
pub fn kv_uarea() -> KmemVaMode {
    KmemVaMode {
        kv_map: KvMap::Kernel,
        kv_align: machine().uspace_align,
        kv_wait: false,
        kv_singlepage: false,
    }
}

/// `uvm_vslock`'s and `uvm_vslock_device`'s range: `[trunc_page(addr), round_page(addr +
/// len))`, `None` (the C's `EINVAL`) when it is empty or wraps.
fn page_range(addr: usize, len: usize) -> Option<(usize, usize)> {
    let start = trunc_page(addr);
    let end = round_page(addr.wrapping_add(len));
    (end > start).then_some((start, end))
}

/// `uvm_vslock`: wires the user memory `[addr, addr + len)` of `map` for I/O, so that it
/// cannot fault while a lock is held (`sys_sysctl`). `access_type` is the C's argument,
/// which the wiring does not use either.
pub fn uvm_vslock(map: &VmMap, addr: usize, len: usize, access_type: VmProt) -> Result<(), Errno> {
    let _ = access_type;
    let Some((start, end)) = page_range(addr, len) else {
        return Err(Errno::EINVAL);
    };
    uvm_map_pageable(map, start, end, false)
}

/// `uvm_vsunlock`: unwires the user memory wired by [`uvm_vslock`] (`sys_sysctl`).
pub fn uvm_vsunlock(map: &VmMap, addr: usize, len: usize) {
    let start = trunc_page(addr);
    let end = round_page(addr.wrapping_add(len));
    crate::assert::kassert!(end > start);
    let _ = uvm_map_pageable(map, start, end, true);
}

/// What [`uvm_vslock_device`] holds until [`uvm_vsunlock_device`]: the map read-locked with
/// `[start, end)` wired, and the bounce buffer the device uses instead of the user pages
/// when one of them is out of its reach.
pub struct DeviceLock<'a> {
    /// The wired pages' range.
    start: usize,
    end: usize,
    /// The bounce buffer and the user bytes' offset into it (`addr - start`).
    bounce: Option<(KmBuf, usize)>,
    /// The map's read lock, held across the transfer. Dropped after the bounce's copy-out
    /// and the unwiring ([`uvm_vsunlock_device`]).
    guard: <VmMap as FaultMap>::Guard<'a>,
}

impl DeviceLock<'_> {
    /// The C's `*retp`: the kernel address the transfer uses in the user buffer's place
    /// (`None` when the device reaches the user pages themselves).
    pub fn bounce_va(&self) -> Option<Vaddr> {
        self.bounce
            .as_ref()
            .map(|(buf, off)| Vaddr::new(buf.as_ptr().addr() + off))
    }
}

/// `paddr_is_dma_reachable(pa)`: `pa` lies within the machine's `dma_constraint`.
pub fn paddr_is_dma_reachable(pa: Paddr) -> bool {
    let r = machine().dma_constraint;
    pa >= r.ucr_low && pa <= r.ucr_high
}

/// `uvm_vslock_device`: wire user memory, make sure it's device reachable and bounce if
/// necessary (physio).
///
/// On success the map stays read-locked until [`uvm_vsunlock_device`] (the [`DeviceLock`]
/// holds the guard) and the bounce buffer, if any, holds a copy of the `len` user bytes.
pub fn uvm_vslock_device(
    map: &VmMap,
    addr: usize,
    len: usize,
    access_type: VmProt,
) -> Result<DeviceLock<'_>, Errno> {
    let Some((start, end)) = page_range(addr, len) else {
        return Err(Errno::EINVAL);
    };
    let sz = end - start;
    let off = addr - start;

    // Wire, then take the read lock and keep it if no writer came in between (retry: the
    // C's `mapv != map->timestamp` loop).
    let mut mapv = FaultMapGuard::version(&FaultMap::lock(map, RwMode::Read));
    let guard = loop {
        uvm_fault_wire(map, start, end, access_type)?;
        match FaultMap::relock(map, mapv) {
            Some(g) => break g,
            None => mapv = FaultMapGuard::version(&FaultMap::lock(map, RwMode::Read)),
        }
    };

    let npages = atop(sz);
    let error = 'out_unwire: {
        let mut reachable = true;
        for i in 0..npages {
            let Some(pa) = FaultMap::pmap(map).extract(Vaddr::new(start + ptoa(i))) else {
                break 'out_unwire Errno::EFAULT;
            };
            if !paddr_is_dma_reachable(pa) {
                reachable = false;
                break;
            }
        }
        if reachable {
            return Ok(DeviceLock {
                start,
                end,
                bounce: None,
                guard,
            });
        }

        let Some(mut buf) = KmBuf::alloc_wired(dma_memory(), sz, true) else {
            break 'out_unwire Errno::ENOMEM;
        };
        match UserPtr::<u8>::new(addr).copyin_slice(&mut buf[off..off + len]) {
            Ok(()) => {
                return Ok(DeviceLock {
                    start,
                    end,
                    bounce: Some((buf, off)),
                    guard,
                });
            }
            // out_unmap: the bounce's pages and space go back with `buf`.
            Err(e) => e,
        }
    };
    // out_unwire:
    uvm_fault_unwire_locked(&guard, start, end);
    drop(guard);
    Err(error)
}

/// `uvm_vsunlock_device`: unwire user memory wired by [`uvm_vslock_device`], copying the
/// bounce buffer back out to the user first, and drop the map lock it kept.
pub fn uvm_vsunlock_device(lock: DeviceLock<'_>, addr: usize, len: usize) {
    let DeviceLock {
        start,
        end,
        bounce,
        guard,
    } = lock;
    crate::assert::kassert!(start == trunc_page(addr) && end == round_page(addr + len));

    if let Some((buf, off)) = &bounce {
        // The C ignores the copyout's result too: the transfer's error is already known.
        let _ = UserPtr::<u8>::new(addr).copyout_slice(&buf[*off..*off + len]);
    }

    uvm_fault_unwire_locked(&guard, start, end);
    drop(guard);

    // the bounce's pages (uvm_km_pgremove_intrsafe, pmap_kremove) and space (km_free).
    drop(bounce);
}

/// `uvm_uarea_alloc`: allocates a u-area (`USPACE` bytes of zeroed, wired kernel memory for
/// a thread's `struct user` and kernel stack). `None` (the C's `0`) when memory is short.
pub fn uvm_uarea_alloc() -> Option<Vaddr> {
    let va = km_alloc(machine().uspace, &kv_uarea(), &KP_ZERO, &KD_WAITOK)?;

    if machine().have_uspace_guard {
        // Carve out a guard page between the PCB and the stack: pmap_extract(pmap_kernel(),
        // va + PAGE_SIZE), pmap_kremove(va + PAGE_SIZE, PAGE_SIZE), pmap_update,
        // uvm_pagefree(pg): see the module's deviations.
        let _ = unported!("uvm_uarea_alloc: the guard page (km_alloc from kernel_map, M6)");
    }

    Some(va)
}

/// `uvm_uarea_free`: frees the u-area at `va` (the caller clears `p_addr`).
pub fn uvm_uarea_free(va: Vaddr) {
    km_free(va, machine().uspace, &kv_uarea(), &KP_ZERO);
}

/// The initial limits `uvm_init_limits` sets on `limit0`: `(cur, max)` of `RLIMIT_STACK` and
/// `RLIMIT_DATA`, and `RLIMIT_RSS`'s current value (its maximum is left as it was).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// `RLIMIT_STACK`: `DFLSSIZ`, `MAXSSIZ`.
    pub stack: (u64, u64),
    /// `RLIMIT_DATA`: `DFLDSIZ`, `MAXDSIZ`.
    pub data: (u64, u64),
    /// `RLIMIT_RSS`'s `rlim_cur`: all of the free memory.
    pub rss_cur: u64,
}

/// `uvm_init_limits`'s numbers: set up the initial limits on process VM. Set the maximum
/// resident set size to be all of (reasonably) available memory. This causes any single,
/// large process to start random page replacement once it fills memory.
pub fn default_limits() -> Limits {
    let free = UVMEXP.free.load(core::sync::atomic::Ordering::Relaxed);
    Limits {
        stack: (DFLSSIZ as u64, MAXSSIZ as u64),
        data: (DFLDSIZ as u64, MAXDSIZ as u64),
        rss_cur: ptoa(free.max(0) as usize) as u64,
    }
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn the_range_is_the_pages_of_the_bytes() {
        assert_eq!(page_range(0x1234, 0x10), Some((0x1000, 0x2000)));
        assert_eq!(page_range(0x1000, 0x1000), Some((0x1000, 0x2000)));
        assert_eq!(page_range(0x1fff, 2), Some((0x1000, 0x3000)));
        assert_eq!(page_range(0x1000, 0), None, "EINVAL: empty");
        assert_eq!(page_range(usize::MAX - 10, 100), None, "EINVAL: wraps");
    }

    #[test]
    fn dma_reachability_is_the_machines_constraint() {
        testutil::install();
        // The host's constant: no constraint.
        assert!(paddr_is_dma_reachable(Paddr::new(0)));
        assert!(paddr_is_dma_reachable(Paddr::new(usize::MAX)));
    }

    #[test]
    fn the_limits_are_the_machines() {
        testutil::install();
        let m = machine();
        let l = default_limits();
        assert_eq!(l.stack, (DFLSSIZ as u64, MAXSSIZ as u64));
        assert_eq!(l.data, (DFLDSIZ as u64, MAXDSIZ as u64));
        assert_eq!(kv_uarea().kv_map, KvMap::Kernel);
        assert_eq!(kv_uarea().kv_align, m.uspace_align);
    }
}
/* </TESTS> */
