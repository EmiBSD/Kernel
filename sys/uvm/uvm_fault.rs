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
 *
 * from: Id: uvm_fault.h,v 1.1.2.2 1997/12/08 16:07:12 chuck Exp
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
 *
 * from: Id: uvm_fault.c,v 1.1.2.23 1998/02/06 05:29:05 chs Exp
 */
/* </LICENSES> */

/* <CODE> */
//! `uvm_fault.h` / `uvm_fault.c`: fault handler: `bsd`'s adapter to the kernel's fault
//! handler, `kernel::vm::fault` (N3.4e).
//!
//! Upstream: sys/uvm/uvm_fault.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_fault.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_fault.rs@3c62ede68802
//!
//! The handler is the kernel's (`sys/kernel/vm/fault.rs`): `uvm_fault`, the wiring and
//! unwiring, over the kernel's anons, amaps and objects, generic over the map it resolves
//! addresses in. The map is `bsd`'s until N3.5, so this file is what the kernel's handler sees
//! of it: [`VmMap`] implements `FaultMap`, [`VmMapEntry`] `FaultEntry`, and [`MapGuard`] is
//! the map locked (bridges.toml `FaultMap`). The legacy callers (the two `trap.rs`,
//! `uvm_map.rs`, `uvm_glue.rs`, `kern_exec.rs`, the selftests) keep their calls: the functions
//! below forward with `bsd`'s map.
//!
//! ## Deviations
//! - Kept from LZ, in the kernel's handler: `uvmadvice[]` a constant table; the swap paths
//!   (`uvm_swap_get`, `uvm_swap_markbad`) reported until `uvm_swap.c` (N3.5); shared start
//!   locks; `pmap_nested` 0, `UVM_PAGE_OWN` and `TRACEPOINT` not configured
//!   (`sys/kernel/vm/fault.rs`, `## Deviations`).
//!
//! ## Redesign
//! - N3.4e: LZ's module is the whole handler, its locks taken C-style in one function and
//!   released in another, its fault info and entry raw pointers made `&'static` (`ufi.entry()`,
//!   `anon_at`, `page_at`). It moved into the kernel crate, where the locks are witnesses
//!   that travel and the entries borrow the map's guard (`sys/kernel/vm/fault.rs`,
//!   `## Redesign`); this file holds no `unsafe` and is `forbid`.
//! - The map's lock as a value ([`MapGuard`]): `vm_map_lock`/`vm_map_lock_read` give one,
//!   its drop is the unlock, `vm_map_upgrade`/`vm_map_downgrade` change it in place, and
//!   `uvmfault_relock`'s map half compares the timestamp under it. The callers of
//!   `uvm_fault_unwire_locked` hold the map lock themselves (the C's shape): [`MapGuard::held`]
//!   is their lock as a guard that does not unlock on drop, made only after
//!   `vm_map_assert_anylock`.
//! - `curproc->p_ru` (`ru_minflt`, `ru_majflt`, `ru_maxrss`): the map's notes; `note_maxrss`
//!   keeps the C's `KASSERT(p != NULL && &p->p_vmspace->vm_map == map)` (LZ returned when
//!   there was no process).

use core::ptr;

pub use kernel::vm::fault::{
    EntryLocks, FaultEntry, FaultMap, FaultMapGuard, UVM_MAXRANGE, VM_FAULT_INVALID,
    VM_FAULT_PROTECT, VM_FAULT_WIRE,
};

use crate::kassert;
use crate::kern::kern_rwlock::rw_status;
use crate::machine::cpu::curproc;
use crate::machine::pmap::PmapRef;
use crate::sys::errno::Errno;
use crate::sys::rwlock::RW_WRITE;
use crate::sys::tree::RbtHead;
use crate::uvm::uvm::{
    uvm_et_iscopyonwrite, uvm_et_isneedscopy, uvm_et_isnofault, uvm_et_isstack, uvm_et_issubmap,
    uvm_et_iswc,
};
use crate::uvm::uvm_amap::{AmapRef, amap_copy};
use crate::uvm::uvm_extern::{VmFault, VmProt, Voff};
use crate::uvm::uvm_map::{
    UvmMapAddr, VM_MAP_INTRSAFE, VM_MAP_ISVMSPACE, VM_MAP_PAGEABLE, VmMap, VmMapEntry,
    uvm_map_lookup_entry, vm_map_assert_anylock, vm_map_downgrade, vm_map_lock, vm_map_lock_read,
    vm_map_unlock, vm_map_unlock_read, vm_map_upgrade, vm_mapent_iswired,
};
use crate::uvm::uvm_object::ObjRef;
use hal::rwlock::RwMode;

/// `bsd`'s map locked ([`FaultMap::lock`]): read or write, as `mode` says. Dropped is
/// `vm_map_unlock` or `vm_map_unlock_read`, unless the guard stands for a lock its caller
/// holds ([`MapGuard::held`]).
pub struct MapGuard<'a> {
    /// The map.
    map: &'a VmMap,
    /// How it is locked.
    mode: RwMode,
    /// Whether the drop unlocks (the guard took the lock).
    owned: bool,
}

impl<'a> MapGuard<'a> {
    /// The lock the caller already holds on `map` (`vm_map_assert_anylock(map)`), as a guard
    /// that leaves it held: for `uvm_fault_unwire_locked`'s callers, which lock and unlock
    /// the map themselves.
    pub fn held(map: &'a VmMap) -> Self {
        vm_map_assert_anylock(map);
        let mode = if map.flags.get() & VM_MAP_INTRSAFE != 0 || rw_status(&map.lock) == RW_WRITE {
            RwMode::Write
        } else {
            RwMode::Read
        };
        Self {
            map,
            mode,
            owned: false,
        }
    }
}

impl Drop for MapGuard<'_> {
    fn drop(&mut self) {
        if self.owned {
            match self.mode {
                RwMode::Write => vm_map_unlock(self.map),
                RwMode::Read => vm_map_unlock_read(self.map),
            }
        }
    }
}

impl FaultMapGuard for MapGuard<'_> {
    type Entry = VmMapEntry;
    type Map = VmMap;

    fn map(&self) -> &VmMap {
        self.map
    }

    fn mode(&self) -> RwMode {
        self.mode
    }

    fn version(&self) -> u32 {
        self.map.timestamp.get()
    }

    fn lookup(&self, va: usize) -> Option<&VmMapEntry> {
        uvm_map_lookup_entry(self.map, va)
    }

    fn next<'a>(&'a self, entry: &'a VmMapEntry) -> Option<&'a VmMapEntry> {
        RbtHead::<UvmMapAddr>::next(entry)
    }

    fn upgrade(mut self) -> Result<Self, Self> {
        kassert!(self.owned && self.mode == RwMode::Read);
        if vm_map_upgrade(self.map) {
            self.mode = RwMode::Write;
            Ok(self)
        } else {
            Err(self)
        }
    }

    fn downgrade(mut self) -> Self {
        kassert!(self.owned && self.mode == RwMode::Write);
        vm_map_downgrade(self.map);
        self.mode = RwMode::Read;
        self
    }

    fn copy_amap(&self, entry: &VmMapEntry, waitf: i32, canchunk: bool, start: usize, end: usize) {
        kassert!(self.mode == RwMode::Write);
        amap_copy(self.map, entry, waitf, canchunk, start, end);
    }
}

impl FaultMap for VmMap {
    type Object = ObjRef;
    type Entry = VmMapEntry;
    type Guard<'a> = MapGuard<'a>;

    fn min_offset(&self) -> usize {
        self.min_offset.get()
    }

    fn max_offset(&self) -> usize {
        self.max_offset.get()
    }

    fn lock(&self, mode: RwMode) -> MapGuard<'_> {
        match mode {
            RwMode::Write => vm_map_lock(self),
            RwMode::Read => vm_map_lock_read(self),
        }
        MapGuard {
            map: self,
            mode,
            owned: true,
        }
    }

    fn relock(&self, version: u32) -> Option<MapGuard<'_>> {
        // relock map. fail if version mismatch (in which case nothing gets locked).
        let guard = self.lock(RwMode::Read);
        (guard.version() == version).then_some(guard)
    }

    fn pmap(&self) -> &PmapRef {
        self.pmap_ref()
    }

    fn is_vmspace(&self) -> bool {
        self.flags.get() & VM_MAP_ISVMSPACE != 0
    }

    fn is_intrsafe(&self) -> bool {
        self.flags.get() & VM_MAP_INTRSAFE != 0
    }

    fn is_pageable(&self) -> bool {
        self.flags.get() & VM_MAP_PAGEABLE != 0
    }

    fn note_minflt(&self) {
        if let Some(p) = curproc() {
            p.p_ru.ru_minflt.set(p.p_ru.ru_minflt.get() + 1);
        }
    }

    fn note_majflt(&self) {
        if let Some(p) = curproc() {
            p.p_ru.ru_majflt.set(p.p_ru.ru_majflt.get() + 1);
        }
    }

    fn note_maxrss(&self, kb: usize) {
        let p = curproc();
        kassert!(p.is_some_and(|p| ptr::eq(&p.vmspace().vm_map, self)));
        let Some(p) = p else {
            return;
        };
        let kb = i64::try_from(kb).unwrap_or(i64::MAX);
        if p.p_ru.ru_maxrss.get() < kb {
            p.p_ru.ru_maxrss.set(kb);
        }
    }
}

impl FaultEntry for VmMapEntry {
    type Map = VmMap;
    type Object = ObjRef;

    fn start(&self) -> usize {
        self.start.get()
    }

    fn end(&self) -> usize {
        self.end.get()
    }

    fn offset(&self) -> Voff {
        self.offset.get()
    }

    fn protection(&self) -> VmProt {
        self.protection.get()
    }

    fn advice(&self) -> i32 {
        self.advice.get()
    }

    fn is_wired(&self) -> bool {
        vm_mapent_iswired(self)
    }

    fn is_submap(&self) -> bool {
        uvm_et_issubmap(self)
    }

    fn is_copyonwrite(&self) -> bool {
        uvm_et_iscopyonwrite(self)
    }

    fn is_needscopy(&self) -> bool {
        uvm_et_isneedscopy(self)
    }

    fn is_nofault(&self) -> bool {
        uvm_et_isnofault(self)
    }

    fn is_stack(&self) -> bool {
        uvm_et_isstack(self)
    }

    fn is_wc(&self) -> bool {
        uvm_et_iswc(self)
    }

    fn submap(&self) -> Option<&'static VmMap> {
        self.sub_map()
    }

    fn aref(&self) -> Option<(AmapRef, i32)> {
        self.aref.amap().map(|a| (a, self.aref.ar_pageoff.get()))
    }

    fn object(&self) -> Option<ObjRef> {
        self.uvm_obj()
    }
}

/// `uvmfault_init`: compute proper values for the `uvmadvice[]` array (the kernel's checks
/// of its constant table).
pub fn uvmfault_init() {
    kernel::vm::fault::uvmfault_init();
}

/// `uvm_fault`: page fault handler, in `bsd`'s map (the kernel's handler).
///
/// - called from MD code to resolve a page fault
/// - MUST NEVER BE CALLED IN INTERRUPT CONTEXT
pub fn uvm_fault(
    orig_map: &VmMap,
    vaddr: usize,
    fault_type: VmFault,
    access_type: VmProt,
) -> Result<(), Errno> {
    kernel::vm::fault::uvm_fault(orig_map, vaddr, fault_type, access_type)
}

/// `uvm_fault_wire`: wire down a range of virtual addresses in a map.
///
/// - map may be read-locked by caller, but MUST NOT be write-locked.
pub fn uvm_fault_wire(
    map: &VmMap,
    start: usize,
    end: usize,
    access_type: VmProt,
) -> Result<(), Errno> {
    kernel::vm::fault::uvm_fault_wire(map, start, end, access_type)
}

/// `uvm_fault_unwire`: unwire range of virtual space.
pub fn uvm_fault_unwire(map: &VmMap, start: usize, end: usize) {
    kernel::vm::fault::uvm_fault_unwire(map, start, end);
}

/// `uvm_fault_unwire_locked`: the guts of `uvm_fault_unwire()`.
///
/// - map must be at least read-locked (by the caller: [`MapGuard::held`]).
pub fn uvm_fault_unwire_locked(map: &VmMap, start: usize, end: usize) {
    kernel::vm::fault::uvm_fault_unwire_locked(&MapGuard::held(map), start, end);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the fault handler over bsd's map: a page fault on an anonymous mapping of
    // a user vmspace, served by the kernel's handler against the host pmap double (see
    // `uvm_map.rs` for the map side, `sys/kernel/vm/fault.rs` for the handler's own tests).

    use std::sync::MutexGuard;
    use std::{assert, assert_eq, assert_ne};

    use super::*;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::machine::Machine;
    use crate::machine::intr::IPL_MPFLOOR;
    use crate::machine::pmap::pmap_extract;
    use crate::machine::vmparam::VmParam;
    use crate::sys::mman::{
        MADV_NORMAL, MAP_INHERIT_COPY, MAP_INHERIT_SHARE, PROT_EXEC, PROT_READ, PROT_WRITE,
    };
    use crate::sys::param::PAGE_SIZE;
    use crate::sys::proc::Process;
    use crate::sys::types::{Vaddr, Vsize};
    use crate::uvm::uvm_amap::{amap_init, amap_lookup, amap_refs};
    use crate::uvm::uvm_anon::{AnonRef, uvm_anon_init};
    use crate::uvm::uvm_aobj::{uao_create, uao_init};
    use crate::uvm::uvm_extern::{UVM_FLAG_COPYONW, UVM_FLAG_FIXED, Vmspace, uvm_mapflag};
    use crate::uvm::uvm_map::{
        uvm_map, uvm_map_init, uvm_mapanon, uvmspace_alloc, uvmspace_fork, uvmspace_free,
    };
    use crate::uvm::uvm_page::{
        PG_BUSY, PG_CLEAN, PG_FAKE, PHYS_TO_VM_PAGE, PQ_ANON, vm_page_to_phys,
    };
    use crate::uvm::uvmexp::{UvmExpCounters, counters_read};

    const RW: VmProt = PROT_READ | PROT_WRITE;

    /// Real memory, the shared locks, the anon, amap and map pools and the console.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = setup_real_memory();
        crate::machine::cons::consinit();
        let _ = hal::rwlock::init_shared();
        let _ = uvm_anon_init(IPL_MPFLOOR);
        let _ = amap_init(IPL_MPFLOOR);
        uvm_map_init();
        uvmfault_init();
        guard
    }

    /// A vmspace with `len` bytes of private anonymous read-write memory at `addr`.
    fn vmspace_with_anon(addr: usize, len: usize) -> &'static Vmspace {
        let vm = uvmspace_alloc(
            <Machine as VmParam>::VM_MIN_ADDRESS,
            <Machine as VmParam>::VM_MAXUSER_ADDRESS,
            true,
            false,
        );
        vm.vm_daddr.set(256 << 20);
        vm.vm_minsaddr.set(<Machine as VmParam>::USRSTACK);
        vm.vm_maxsaddr
            .set(<Machine as VmParam>::USRSTACK - <Machine as VmParam>::MAXSSIZ);
        let mut a = addr;
        let flags = uvm_mapflag(
            RW,
            RW | PROT_EXEC,
            MAP_INHERIT_COPY,
            MADV_NORMAL,
            UVM_FLAG_FIXED | UVM_FLAG_COPYONW,
        );
        uvm_mapanon(&vm.vm_map, &mut a, len, 0, flags).expect("mapanon");
        vm
    }

    /// The anon the amap of `vm` holds at `va` (a handle), under the map's and the amap's
    /// locks.
    fn anon_of(vm: &Vmspace, va: usize) -> Option<AnonRef> {
        let guard = vm.vm_map.lock(RwMode::Read);
        let entry = guard.lookup(va)?;
        let (amap, pageoff) = entry.aref()?;
        let w = amap.lock().read();
        amap_lookup(&amap, &w, pageoff, va - entry.start.get()).cloned()
    }

    fn same(a: &AnonRef, b: &AnonRef) -> bool {
        core::ptr::eq(&**a, &**b)
    }

    #[test]
    fn zero_fill_fault_maps_a_fresh_anon_page() {
        let _g = setup();
        let va = 0x2000_0000;
        let vm = vmspace_with_anon(va, 4 * PAGE_SIZE);
        let pmap = vm.vm_map.pmap();
        assert!(pmap_extract(pmap, Vaddr::new(va)).is_none());
        let faults = counters_read(UvmExpCounters::Faults);

        // A read fault: the lazy amap is made, an anon with a zeroed page is promoted and
        // mapped (case 2B), the neighbours are not (no anons yet).
        uvm_fault(&vm.vm_map, va + 8, VM_FAULT_INVALID, PROT_READ).expect("read fault");
        let pa = pmap_extract(pmap, Vaddr::new(va)).expect("mapped");
        let anon = anon_of(vm, va).expect("an anon");
        let pg = anon.page().expect("its page");
        assert_eq!(vm_page_to_phys(pg), pa.trunc_page());
        assert_ne!(pg.flags() & PQ_ANON, 0);
        assert_eq!(pg.flags() & (PG_BUSY | PG_FAKE), 0);
        assert_eq!(anon.refs(), 1);
        assert_eq!(counters_read(UvmExpCounters::Faults), faults + 1);

        // A write fault on the same page is case 1A: the same anon and page stay.
        uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, PROT_WRITE).expect("write fault");
        assert!(anon_of(vm, va).is_some_and(|a| same(&a, &anon)));
        assert_eq!(pmap_extract(pmap, Vaddr::new(va)), Some(pa));

        // The next page faults in with its own anon; the first stays.
        uvm_fault(&vm.vm_map, va + PAGE_SIZE, VM_FAULT_INVALID, PROT_WRITE).expect("second page");
        let anon2 = anon_of(vm, va + PAGE_SIZE).expect("second anon");
        assert!(!same(&anon, &anon2));
        drop((anon, anon2));
        uvmspace_free(vm);
    }

    #[test]
    fn faults_outside_or_beyond_protection_fail() {
        let _g = setup();
        let va = 0x3000_0000;
        let vm = vmspace_with_anon(va, PAGE_SIZE);

        // Free space: a case 0 fault.
        assert_eq!(
            uvm_fault(&vm.vm_map, va + 0x10_0000, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EFAULT)
        );
        // Outside the map.
        assert_eq!(
            uvm_fault(&vm.vm_map, 0, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EFAULT)
        );
        // More access than the entry allows.
        assert_eq!(
            uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, PROT_EXEC),
            Err(Errno::EACCES)
        );
        uvmspace_free(vm);
    }

    #[test]
    fn copy_on_write_after_fork_promotes_a_new_anon() {
        let _g = setup();
        let va = 0x4000_0000;
        let vm = vmspace_with_anon(va, PAGE_SIZE);
        uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, PROT_WRITE).expect("parent fault");
        let parent_anon = anon_of(vm, va).expect("parent anon");
        let parent_pa = pmap_extract(vm.vm_map.pmap(), Vaddr::new(va)).expect("parent pa");

        let pr = Process::new();
        pr.ps_vmspace.set(vm);
        let child = uvmspace_fork(&pr);
        // The amap is shared and the anon has two references until the child writes.
        let parent_amap = {
            let guard = vm.vm_map.lock(RwMode::Read);
            guard.lookup(va).and_then(|e| e.aref.amap())
        }
        .expect("parent amap");
        assert_eq!(amap_refs(&parent_amap), 2);

        // The child's read fault sees the parent's page, read-only (case 1A, an_ref > 1).
        uvm_fault(&child.vm_map, va, VM_FAULT_INVALID, PROT_READ).expect("child read");
        assert_eq!(
            pmap_extract(child.vm_map.pmap(), Vaddr::new(va)),
            Some(parent_pa)
        );

        // The child's write fault copies the amap (needs_copy), then promotes a new anon with
        // a copy of the page (case 1B): the two maps now differ.
        uvm_fault(&child.vm_map, va, VM_FAULT_INVALID, PROT_WRITE).expect("child write");
        let child_anon = anon_of(child, va).expect("child anon");
        assert!(!same(&child_anon, &parent_anon));
        assert_eq!(parent_anon.refs(), 1);
        assert_eq!(child_anon.refs(), 1);
        let child_pa = pmap_extract(child.vm_map.pmap(), Vaddr::new(va)).expect("child pa");
        assert_ne!(child_pa, parent_pa);
        assert_eq!(
            pmap_extract(vm.vm_map.pmap(), Vaddr::new(va)),
            Some(parent_pa)
        );
        assert_eq!(amap_refs(&parent_amap), 1);
        assert!(counters_read(UvmExpCounters::FltAcow) >= 1);

        drop((parent_anon, child_anon, parent_amap));
        uvmspace_free(child);
        uvmspace_free(vm);
    }

    #[test]
    fn wire_and_unwire_a_range() {
        let _g = setup();
        let va = 0x5000_0000;
        let len = 3 * PAGE_SIZE;
        let vm = vmspace_with_anon(va, len);
        let pmap = vm.vm_map.pmap();

        uvm_fault_wire(&vm.vm_map, va, va + len, RW).expect("wire");
        for i in 0..3 {
            let pa = pmap_extract(pmap, Vaddr::new(va + i * PAGE_SIZE)).expect("mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 1, "page {i} wired");
            assert_eq!(pg.flags() & PG_CLEAN, 0);
        }

        uvm_fault_unwire(&vm.vm_map, va, va + len);
        for i in 0..3 {
            let pa = pmap_extract(pmap, Vaddr::new(va + i * PAGE_SIZE)).expect("still mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 0, "page {i} unwired");
        }
        uvmspace_free(vm);
    }

    #[test]
    fn unwire_locked_under_the_callers_lock() {
        // uvm_map's and uvm_glue's callers hold the map lock C-style: the held guard leaves
        // it held, and the pages are unwired all the same.
        let _g = setup();
        let va = 0x5800_0000;
        let len = 2 * PAGE_SIZE;
        let vm = vmspace_with_anon(va, len);
        let map = &vm.vm_map;
        let pmap = map.pmap();
        uvm_fault_wire(map, va, va + len, RW).expect("wire");

        vm_map_lock_read(map);
        uvm_fault_unwire_locked(map, va, va + len);
        assert!(rw_status(&map.lock) != 0, "the caller's lock is still held");
        vm_map_unlock_read(map);
        for i in 0..2 {
            let pa = pmap_extract(pmap, Vaddr::new(va + i * PAGE_SIZE)).expect("still mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 0, "page {i} unwired");
        }
        uvmspace_free(vm);
    }

    #[test]
    fn unwire_an_object_entry_without_an_amap() {
        // A shared mapping of an object (a MAP_SHARED file, here an aobj) has no amap: wiring
        // it faults the object's pages in, and unwiring must find their owner without looking
        // up an amap.
        let _g = setup();
        let _ = uao_init();
        let vm = vmspace_with_anon(0x7000_0000, PAGE_SIZE);
        let map = &vm.vm_map;
        let pmap = map.pmap();
        let len = 2 * PAGE_SIZE;
        let uobj = uao_create(Vsize::new(len), 0).expect("an aobj");
        let mut va = 0x7100_0000;
        let flags = uvm_mapflag(RW, RW, MAP_INHERIT_SHARE, MADV_NORMAL, UVM_FLAG_FIXED);
        uvm_map(map, &mut va, len, Some(uobj.clone()), 0, 0, flags).expect("map the aobj");
        {
            let guard = map.lock(RwMode::Read);
            let entry = guard.lookup(va).expect("the entry");
            assert!(!entry.aref.has_amap(), "a shared object entry has no amap");
        }

        uvm_fault_wire(map, va, va + len, PROT_READ).expect("wire");
        for i in 0..2 {
            let pa = pmap_extract(pmap, Vaddr::new(va + i * PAGE_SIZE)).expect("mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert!(pg.owned_by_object(&*uobj), "page {i} is the object's");
            assert_eq!(pg.wire_count(), 1, "page {i} wired");
        }

        uvm_fault_unwire(map, va, va + len);
        for i in 0..2 {
            let pa = pmap_extract(pmap, Vaddr::new(va + i * PAGE_SIZE)).expect("still mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 0, "page {i} unwired");
        }
        drop(uobj);
        uvmspace_free(vm);
    }

    #[test]
    fn neighbour_anons_are_mapped_ahead() {
        let _g = setup();
        let va = 0x6000_0000;
        let vm = vmspace_with_anon(va, 8 * PAGE_SIZE);
        let pmap = vm.vm_map.pmap();

        // Fault the pages in one by one, then drop their mappings: the anons stay.
        for i in 0..8 {
            uvm_fault(&vm.vm_map, va + i * PAGE_SIZE, VM_FAULT_INVALID, PROT_WRITE).expect("fault");
        }
        crate::machine::pmap::pmap_remove(pmap, Vaddr::new(va), Vaddr::new(va + 8 * PAGE_SIZE));
        assert!(pmap_extract(pmap, Vaddr::new(va + 4 * PAGE_SIZE)).is_none());

        // One read fault in the middle maps the MADV_NORMAL window (3 back, 4 ahead) too.
        uvm_fault(&vm.vm_map, va + 4 * PAGE_SIZE, VM_FAULT_INVALID, PROT_READ).expect("refault");
        for i in 1..8 {
            assert!(
                pmap_extract(pmap, Vaddr::new(va + i * PAGE_SIZE)).is_some(),
                "page {i} mapped by the fault-ahead"
            );
        }
        assert!(
            pmap_extract(pmap, Vaddr::new(va)).is_none(),
            "page 0 is behind the window"
        );
        assert!(counters_read(UvmExpCounters::FltNamap) >= 6);
        uvmspace_free(vm);
    }
}
/* </TESTS> */
