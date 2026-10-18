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
 * Copyright (c) 1991, 1993 The Regents of the University of California.
 * Copyright (c) 1988 University of Utah.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
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
 * from: Utah $Hdr: vm_unix.c 1.1 89/11/07$
 *      @(#)vm_unix.c   8.1 (Berkeley) 6/11/93
 * from: Id: uvm_unix.c,v 1.1.2.2 1997/08/25 18:52:30 chuck Exp
 */
/* </LICENSES> */

/* <CODE> */
//! `uvm_unix.c`: traditional sbrk/grow interface to vm, and the core dump's walk of the
//! address space.
//!
//! Upstream: sys/uvm/uvm_unix.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_unix.rs@3c62ede68802
//!
//! ## Deviations
//! - `sys_obreak` prints its failure with `printf`: there is no controlling terminal for
//!   `uprintf` yet (LZ's).
//!
//! ## Redesign
//! - The functions take the address space (`&Vmspace`) instead of a `struct proc *`: `bsd`'s
//!   system call adapter passes `p->p_vmspace`; the current process's limits, `ps_sigcode`
//!   and `ps_timekeep` come through the `VmProc` bridge (`vm::proc`).
//! - The core dump callbacks are `&mut dyn FnMut` closures (the C's `void *cookie` is what
//!   they capture), the walk's over a `CoredumpSeg` (`vm::types`).
//! - The C walks the map unlocked (the dumping process is single-threaded by then, and
//!   nothing else changes its map). Entries are reached here only under the map's guard, and
//!   the walk callback writes the core file, which faults on this very map: so each step
//!   takes the read lock, copies out of the entry what the walk needs ([`Seg`]: the bounds,
//!   the protection, the amap handle, what the object is) and drops the lock before any
//!   callback runs. The next step looks up the entry after the last one's start. The same
//!   entries in the same order, the same segments.

use core::sync::atomic::Ordering;

use hal::arch::current::param::PAGE_SHIFT;
use hal::arch::current::vmparam::VM_MAXUSER_ADDRESS;
use hal::sys::errno::Errno;
use hal::sys::mman::{MADV_NORMAL, MAP_INHERIT_COPY, PROT_EXEC, PROT_READ, PROT_WRITE};
use hal::vm::param::{atop, ptoa, round_page};

use super::amap::{Amap, AmapRef, amap_lookups};
use super::fault::FaultEntry;
use super::map::{UVM_ET_CONCEAL, VmMap, Vmspace, uvm_map, uvm_unmap};
use super::proc::{RLIMIT_DATA, RLIMIT_STACK, vmproc};
use super::types::{
    CoredumpSeg, UVM_FLAG_COPYONW, UVM_FLAG_FIXED, UVM_UNKNOWN_OFFSET, VmProt, uvm_mapflag,
};
use crate::assert::kpanic;
use crate::prf::printf;

/// `WALK_CHUNK`.
const WALK_CHUNK: usize = 32;

/// What the core dump's walk reads of one map entry, copied out under the map's read lock.
struct Seg {
    /// `entry->start`.
    start: usize,
    /// `entry->end`.
    end: usize,
    /// `entry->protection`.
    prot: VmProt,
    /// `entry->aref`: the amap and the page offset into it.
    aref: Option<(AmapRef, i32)>,
    /// `UVM_ET_ISSUBMAP(entry)`.
    submap: bool,
    /// `UVM_ET_ISCONCEAL(entry)`.
    conceal: bool,
    /// `UVM_OBJ_IS_DEVICE(entry->object.uvm_obj)`.
    device: bool,
    /// `UVM_OBJ_IS_VNODE(entry->object.uvm_obj)`.
    vnode: bool,
}

impl Seg {
    /// The amap of the entry, if it has one.
    fn amap(&self) -> Option<&AmapRef> {
        self.aref.as_ref().map(|(a, _)| a)
    }
}

/// `sys_obreak`: set break, to `nsize`.
pub fn sys_obreak(vm: &Vmspace, nsize: usize) -> Result<(), Errno> {
    let base = vm.vm_daddr.load(Ordering::Relaxed);
    let new = round_page(nsize);
    if new < base || (new - base) as u64 > vmproc().lim_cur(RLIMIT_DATA) {
        return Err(Errno::ENOMEM);
    }

    let mut old = round_page(base + ptoa(vm.vm_dsize.load(Ordering::Relaxed) as usize));

    if new == old {
        return Ok(());
    }

    // grow or shrink?
    if new > old {
        let grow = new - old;
        if let Err(error) = uvm_map(
            &vm.vm_map,
            &mut old,
            grow,
            None,
            UVM_UNKNOWN_OFFSET,
            0,
            uvm_mapflag(
                PROT_READ | PROT_WRITE,
                PROT_READ | PROT_WRITE | PROT_EXEC,
                MAP_INHERIT_COPY,
                MADV_NORMAL,
                UVM_FLAG_FIXED | UVM_FLAG_COPYONW,
            ),
        ) {
            // uprintf: see the module's deviations.
            printf(format_args!(
                "sbrk: grow {} failed, error = {}\n",
                grow,
                error.as_i32()
            ));
            return Err(Errno::ENOMEM);
        }
        vm.vm_dsize.fetch_add(atop(grow) as i32, Ordering::Relaxed);
    } else {
        uvm_unmap(&vm.vm_map, new, old);
        vm.vm_dsize
            .fetch_sub(atop(old - new) as i32, Ordering::Relaxed);
    }

    Ok(())
}

/// `uvm_grow`: enlarge the "stack segment" to include sp.
pub fn uvm_grow(vm: &Vmspace, sp: usize) {
    // For user defined stacks (from sendsig).
    if sp < vm.vm_maxsaddr.load(Ordering::Relaxed) {
        return;
    }
    // MACHINE_STACK_GROWS_UP: neither amd64 nor arm64.

    let guard = vm.vm_map.lock();

    let minsaddr = vm.vm_minsaddr.load(Ordering::Relaxed);
    let ssize = vm.vm_ssize.load(Ordering::Relaxed);
    // For common case of already allocated (from trap).
    if sp < minsaddr - ptoa(ssize as usize) {
        // Really need to check vs limit and increment stack size if ok.
        let si = atop(minsaddr - sp) as i64 - i64::from(ssize);
        if i64::from(ssize) + si <= atop(vmproc().lim_cur(RLIMIT_STACK) as usize) as i64 {
            vm.vm_ssize
                .store((i64::from(ssize) + si) as i32, Ordering::Relaxed);
        }
    }
    // out:
    drop(guard);
}

/// `uvm_coredump_walk_amap`.
///
/// Not all the pages in an amap may be present. When dumping core, we don't want to force
/// all the pages to be present: it's a waste of time and memory when we already know what
/// they contain (zeros) and the ELF format at least can adequately represent them as a
/// segment with memory size larger than its file size.
///
/// So, we walk the amap with calls to `amap_lookups()` and scan the resulting pointers to
/// find ranges of zero or more present pages followed by at least one absent page or the end
/// of the amap. When then pass that range to the walk callback with 'start' pointing to the
/// start of the present range, 'realend' pointing to the first absent page (or the end of
/// the entry), and 'end' pointing to the page past the last absent page (or the end of the
/// entry).
///
/// Note that if the first page of the amap is empty then the callback must be invoked with
/// 'start' == 'realend' so it can present that first range of absent pages.
fn uvm_coredump_walk_amap(
    seg: &Seg,
    nsegmentp: &mut i32,
    walk: &mut dyn FnMut(CoredumpSeg) -> Result<(), Errno>,
) -> Result<(), Errno> {
    let Some((amap, pageoff)) = &seg.aref else {
        kpanic(format_args!(
            "uvm_coredump_walk_amap: entry without an amap"
        ));
    };
    let prot = seg.prot;
    let mut nsegment = *nsegmentp;
    let mut start = seg.start;
    let entry_end = seg.end.min(VM_MAXUSER_ADDRESS);
    let mut present = [false; WALK_CHUNK];

    let mut absent = false;
    let mut realend = start;
    let mut pos = start;
    while pos < entry_end {
        let npages = ((entry_end - pos) >> PAGE_SHIFT).min(WALK_CHUNK);
        amap_present(amap, *pageoff, pos - seg.start, &mut present[..npages]);
        for (i, &here) in present[..npages].iter().enumerate() {
            if !here == absent {
                continue;
            }
            if !absent {
                // going from present to absent: set realend
                realend = pos + (i << PAGE_SHIFT);
                absent = true;
                continue;
            }

            // going from absent to present: invoke callback
            let end = pos + (i << PAGE_SHIFT);
            if start != end {
                walk(CoredumpSeg {
                    start,
                    realend,
                    end,
                    prot,
                    isvnode: false,
                    nsegment,
                })?;
                nsegment += 1;
            }
            start = end;
            realend = end;
            absent = false;
        }
        pos += npages << PAGE_SHIFT;
    }

    if !absent {
        realend = entry_end;
    }
    let error = walk(CoredumpSeg {
        start,
        realend,
        end: entry_end,
        prot,
        isvnode: false,
        nsegment,
    });
    *nsegmentp = nsegment + 1;
    error
}

/// `amap_lookups(&entry->aref, offset, anons, npages)` reduced to whether each page has an
/// anon: the amap's read lock is held for the lookups only (the C reads the slots unlocked),
/// since the walk callback writes the core file and may fault on this very amap.
fn amap_present(amap: &Amap, pageoff: i32, offset: usize, present: &mut [bool]) {
    let w = amap.lock().read();
    let mut anons = [None; WALK_CHUNK];
    let anons = &mut anons[..present.len()];
    amap_lookups(amap, &w, pageoff, offset, anons);
    for (p, a) in present.iter_mut().zip(anons.iter()) {
        *p = a.is_some();
    }
}

/// `uvm_should_coredump`: common logic for whether a map entry should be included in a
/// coredump.
fn uvm_should_coredump(seg: &Seg, sigcode: usize, timekeep: usize) -> bool {
    if seg.prot & PROT_WRITE == 0
        && seg.aref.is_none()
        && seg.start != sigcode
        && seg.start != timekeep
    {
        return false;
    }

    // Skip ranges marked as unreadable, as uiomove(UIO_USERSPACE) will fail on them. Maybe
    // this really should be a test of entry->max_protection, but doing
    // uvm_map_extract(UVM_EXTRACT_FIXPROT) on each such page would suck.
    if seg.prot & PROT_READ == 0 && seg.start != sigcode {
        return false;
    }

    // Skip ranges excluded from coredumps.
    if seg.conceal {
        return false;
    }

    // Don't dump mmaped devices.
    if seg.device {
        return false;
    }

    if seg.start >= VM_MAXUSER_ADDRESS {
        return false;
    }

    true
}

/// `amap->am_ref += delta`: the coredump's extra reference to a shared amap, taken and
/// dropped under the amap's write lock (the C changes the count with no lock at all; the
/// dumping process's other threads are stopped, so the amap's lock is the only one held
/// here).
fn amap_refs_adjust(amap: &Amap, delta: i32) {
    let w = amap.lock().write();
    amap.set_refs_locked(&w, amap.refs() + delta);
}

/// The entry of `map` that follows the one starting at `after` (the first entry for
/// `None`), copied out under the map's read lock, which is released before the return.
fn seg_after(map: &VmMap, after: Option<usize>) -> Option<Seg> {
    let guard = map.lock_read();
    let addr = &guard.state().addr;
    let e = match after {
        None => addr.min(),
        Some(s) => addr.nfind_by(|n| (s + 1).cmp(&n.start())),
    }?;
    let object = FaultEntry::object(e);
    Some(Seg {
        start: e.start(),
        end: e.end(),
        prot: e.protection(),
        aref: FaultEntry::aref(e),
        submap: FaultEntry::is_submap(e),
        conceal: e.etype() & UVM_ET_CONCEAL != 0,
        device: object.as_ref().is_some_and(|o| o.is_device()),
        vnode: object.as_ref().is_some_and(|o| o.is_vnode()),
    })
}

/// `uvm_coredump_walkmap`: walk the VA space for a process to identify what to write to a
/// coredump. First the number of contiguous ranges is counted, then the 'setup' callback is
/// invoked to prepare for actually recording the ranges, then the VA is walked again,
/// invoking the 'walk' callback for each range. The number of ranges walked is guaranteed to
/// match the count seen by the 'setup' callback.
pub fn uvm_coredump_walkmap(
    vm: &Vmspace,
    setup: &mut dyn FnMut(i32) -> Result<(), Errno>,
    walk: &mut dyn FnMut(CoredumpSeg) -> Result<(), Errno>,
) -> Result<(), Errno> {
    let map = &vm.vm_map;
    let p = vmproc();
    let (sigcode, timekeep) = (p.ps_sigcode(), p.ps_timekeep());
    let should = |s: &Seg| uvm_should_coredump(s, sigcode, timekeep);
    let mut refed_amaps = 0;

    // Walk the map once to count the segments. If an amap is referenced more than once than
    // take *another* reference and treat the amap as exactly one segment instead of checking
    // page presence inside it. On the second pass we'll recognize which amaps we did that for
    // by the ref count being >1...and decrement it then.
    let mut nsegment = 0;
    let mut it = seg_after(map, None);
    while let Some(seg) = it {
        it = seg_after(map, Some(seg.start));
        // should never happen for a user process
        if seg.submap {
            kpanic(format_args!(
                "uvm_coredump_walkmap: user process with submap?"
            ));
        }

        if !should(&seg) {
            continue;
        }

        if let Some(amap) = seg.amap() {
            if amap.refs() == 1 {
                let _ = uvm_coredump_walk_amap(&seg, &mut nsegment, &mut |_| Ok(()));
                continue;
            }

            // Multiple refs currently, so take another and treat it as a single segment
            amap_refs_adjust(amap, 1);
            refed_amaps += 1;
        }

        nsegment += 1;
    }

    // Okay, we have a count in nsegment. Prepare to walk it again, then invoke the setup
    // callback.
    let mut entry = seg_after(map, None);
    let mut error = setup(nsegment);
    if error.is_ok() {
        // Setup went okay, so do the second walk, invoking the walk callback on the counted
        // segments and cleaning up references as we go.
        nsegment = 0;
        while let Some(seg) = entry.take() {
            if !should(&seg) {
                entry = seg_after(map, Some(seg.start));
                continue;
            }

            if seg.amap().is_some_and(|a| a.refs() == 1) {
                error = uvm_coredump_walk_amap(&seg, &mut nsegment, walk);
                if error.is_err() {
                    entry = Some(seg);
                    break;
                }
                entry = seg_after(map, Some(seg.start));
                continue;
            }

            let end = seg.end.min(VM_MAXUSER_ADDRESS);
            error = walk(CoredumpSeg {
                start: seg.start,
                realend: end,
                end,
                prot: seg.prot,
                isvnode: seg.vnode,
                nsegment,
            });
            if error.is_err() {
                entry = Some(seg);
                break;
            }
            nsegment += 1;

            if let Some(amap) = seg.amap()
                && amap.refs() > 1
            {
                // multiple refs, so we need to drop one
                amap_refs_adjust(amap, -1);
                refed_amaps -= 1;
            }
            entry = seg_after(map, Some(seg.start));
        }
    }

    if error.is_err() {
        // cleanup: clean up the extra references from where we left off
        if refed_amaps > 0 {
            while let Some(seg) = entry {
                entry = seg_after(map, Some(seg.start));
                let Some(amap) = seg.amap() else {
                    continue;
                };
                if amap.refs() == 1 {
                    continue;
                }
                if !should(&seg) {
                    continue;
                }
                amap_refs_adjust(amap, -1);
                let was = refed_amaps;
                refed_amaps -= 1;
                if was == 0 {
                    break;
                }
            }
        }
    }

    error
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: usize, prot: VmProt) -> Seg {
        Seg {
            start,
            end: start + 0x1000,
            prot,
            aref: None,
            submap: false,
            conceal: false,
            device: false,
            vnode: false,
        }
    }

    /// `uvm_should_coredump`'s rules, in the C's order.
    #[test]
    fn what_a_core_dump_holds() {
        let (sigcode, timekeep) = (0x7000, 0x8000);
        let should = |s: &Seg| uvm_should_coredump(s, sigcode, timekeep);
        assert!(should(&seg(0x1000, PROT_READ | PROT_WRITE)));
        assert!(!should(&seg(0x1000, PROT_READ)), "read-only, no amap");
        assert!(should(&seg(sigcode, PROT_READ)), "the signal trampoline");
        assert!(should(&seg(timekeep, PROT_READ)), "the timekeep page");
        assert!(!should(&seg(0x1000, PROT_WRITE)), "unreadable");
        assert!(
            should(&seg(sigcode, PROT_EXEC)),
            "the trampoline, even unreadable"
        );
        assert!(
            !should(&seg(timekeep, PROT_EXEC)),
            "the timekeep page, unreadable"
        );
        let mut s = seg(0x1000, PROT_READ | PROT_WRITE);
        s.conceal = true;
        assert!(!should(&s), "MAP_CONCEAL");
        let mut s = seg(0x1000, PROT_READ | PROT_WRITE);
        s.device = true;
        assert!(!should(&s), "a device mapping");
        assert!(!should(&seg(VM_MAXUSER_ADDRESS, PROT_READ | PROT_WRITE)));
    }
}
/* </TESTS> */
