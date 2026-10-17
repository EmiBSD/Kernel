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
 *	@(#)vm_page.h   7.3 (Berkeley) 4/21/91
 *	@(#)vm_page.c   8.3 (Berkeley) 3/21/94
 * from: Id: uvm_page.h,v 1.1.2.6 1998/02/04 02:31:42 chuck Exp
 * from: Id: uvm_page.c,v 1.1.2.18 1998/02/06 05:24:42 chs Exp
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
//! Resident memory system definitions and page ops (`<uvm/uvm_page.h>`, `uvm/uvm_page.c`):
//! `bsd`'s adapter to the kernel's page system, `kernel::vm::page` (N3.3).
//!
//! Upstream: sys/uvm/uvm_page.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_page.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_page.rs@3c62ede68802
//!
//! The pages, the segments, the queues and every page operation are the kernel's
//! (`sys/kernel/vm/page.rs`, with the free page allocator in `sys/kernel/vm/pmemrange.rs`),
//! re-exported here by their C names, so the legacy callers keep their paths. What stays here
//! is what is still `bsd`'s or the machine's:
//!
//! - the machine's segment policy (`VM_PHYSSEG_MAX`, `VM_PHYSSEG_STRAT`, the `VM_PSTRAT_*`
//!   values) and the C's lines when a segment is refused ([`uvm_page_physload`]);
//! - `uvm_page_init`'s machine inputs: the DMA constraints (`uvm_md_constraints[]`), the kernel
//!   virtual range the pmap leaves after the last steal ([`uvm_page_init`]), and the buffer
//!   cache's statistics, registered for the allocator (the `BufCacheStats` bridge);
//! - [`uvm_pagealloc`] with the C's `(obj, anon)` pair, [`uvm_pagewait`] (a sleep on the
//!   owner's legacy `rw_obj`), [`uvm_pagealloc_multi`] (the buffer cache's, under the kernel
//!   lock), [`uvm_shutdown`] (`smr_flush`), [`paddr_is_dma_reachable`],
//!   [`uvm_page_physdump`] (ddb's `printf`);
//! - [`owner`]: the C's `(obj, anon)` as the kernel's [`Owner`].
//!
//! ## Deviations
//! - `uvm_page_physload` after `uvm_init` needs `km_alloc` for the segment's pages: the
//!   segment is ignored, with the C's lines, as the C does when that allocation fails (LZ's
//!   deviation, kept).
//! - On a machine without an MMU (`PMAP_NOMMU`, the host double) `uvm_pagealloc_multi` asks
//!   for one physical segment: the buffer cache reaches a buffer's pages through the direct
//!   map there (`vfs_biomem.rs`).
//!
//! ## Redesign
//! - N3.3: LZ's module is the whole page system. It moved into the kernel crate (see
//!   `sys/kernel/vm/page.rs`'s `## Redesign`); this file holds no `unsafe` and is `forbid`.
//! - The functions whose C body follows `pg->uobject` or `pg->uanon` take the owner
//!   ([`Owner`], [`owner`]): `uvm_pagefree`, `uvm_pageclean`, `uvm_page_unbusy`,
//!   `uvm_pagewire`, `uvm_pageunwire`, `uvm_pageactivate`, `uvm_pagedeactivate`,
//!   `uvm_pagedequeue`, `uvm_pagerealloc` and [`uvm_pagewait`]. `uvm_pagealloc` keeps its
//!   `(obj, off, anon, flags)` here.
//! - `uvm_lock_pageq()` returns the queues' guard and dropping it is `uvm_unlock_pageq()`;
//!   `uvm_pagedequeue` takes the guarded queues.

use core::fmt::Write;

pub use kernel::vm::page::*;
pub use kernel::vm::pmemrange::{BufCacheStats, uvm_pmr_cache_drain};

use crate::kern::kern_synch::rwsleep_nsec;
use crate::machine::pmap::Pmap;
use crate::machine::{Machine, VmParam};
use crate::sys::errno::Errno;
use crate::sys::param::{PNORELOCK, PVM};
use crate::sys::rwlock::{Rwlock, rw_lock_held};
use crate::sys::smr::smr_flush;
use crate::sys::systm::{INFSLP, kernel_assert_locked};
use crate::sys::types::{Paddr, Vaddr};
use crate::uvm::uvm_anon::VmAnon;
use crate::uvm::uvm_extern::Voff;
use crate::uvm::uvm_object::UvmObject;
use crate::uvm::uvm_param::{atop, ptoa, round_page, trunc_page};
use crate::{kassert, kprintf};

// Physical memory segment strategies: how vm_physmem[] is kept.

/// Random: put it at the end (if this is "1" then we revert to a "contig" case).
pub const VM_PSTRAT_RANDOM: i32 = 1;
/// Binary search (sorted by address).
pub const VM_PSTRAT_BSEARCH: i32 = 2;
/// Linear search (sorted by largest segment first).
pub const VM_PSTRAT_BIGFIRST: i32 = 3;

/// `VM_PHYSSEG_MAX` of the selected machine.
pub const VM_PHYSSEG_MAX: usize = <Machine as VmParam>::VM_PHYSSEG_MAX;
/// `VM_PHYSSEG_STRAT` of the selected machine.
pub const VM_PHYSSEG_STRAT: i32 = <Machine as VmParam>::VM_PHYSSEG_STRAT;

/// The machine's segment policy, as the kernel's page system takes it (an unknown strategy,
/// the C's "unknown physseg strategy selected!" panic, fails the build: see the end).
const SEG_POLICY: SegPolicy = SegPolicy {
    max: VM_PHYSSEG_MAX,
    strat: match VM_PHYSSEG_STRAT {
        VM_PSTRAT_BSEARCH => SegStrat::Bsearch,
        VM_PSTRAT_BIGFIRST => SegStrat::Bigfirst,
        _ => SegStrat::Random,
    },
};

/// The buffer cache's statistics of `<sys/mount.h>`, for the kernel's allocator (the
/// `BufCacheStats` bridge): one call each into the legacy function of the same name.
struct BsdBufCacheStats;

impl BufCacheStats for BsdBufCacheStats {
    fn bufpages_deficit(&self) -> i64 {
        crate::sys::mount::bufpages_deficit()
    }

    fn bufpages_inact(&self) -> i64 {
        crate::sys::mount::bufpages_inact()
    }
}

static BSD_BUFCACHE_STATS: BsdBufCacheStats = BsdBufCacheStats;

/// The C's `(obj, anon)` pair (at most one of them) as the kernel's [`Owner`].
pub fn owner<'a>(obj: Option<&'a UvmObject>, anon: Option<&'a VmAnon>) -> Owner<'a> {
    kassert!(obj.is_none() || anon.is_none());
    match (obj, anon) {
        (Some(o), _) => Owner::Object(o),
        (None, Some(a)) => Owner::Anon(a),
        (None, None) => Owner::None,
    }
}

/// `PADDR_IS_DMA_REACHABLE(paddr)`.
pub fn paddr_is_dma_reachable(paddr: Paddr) -> bool {
    let c = <Machine as Pmap>::DMA_CONSTRAINT;
    c.ucr_low <= paddr && c.ucr_high > paddr
}

/// `uvm_page_init`: init the page system (the kernel's, with the machine's DMA constraints
/// and the buffer cache's statistics). Called from `uvm_init()`. Returns the range of kernel
/// virtual memory left after the boot allocations in `kvm_startp`/`kvm_endp`.
pub fn uvm_page_init(kvm_startp: &mut Vaddr, kvm_endp: &mut Vaddr) {
    // A second registration (a host test's next set-up) keeps the first: the same statics.
    let _ = kernel::vm::pmemrange::uvm_pmr_install_stats(&BSD_BUFCACHE_STATS);
    // The page system steals its boot memory through the pmap (the host double bootstraps it
    // here, as no boot path does).
    <Machine as Pmap>::pmap_bootstrap_lazy();
    kernel::vm::page::uvm_page_init(<Machine as Pmap>::UVM_MD_CONSTRAINTS);

    // pass up the values of virtual_space_start and virtual_space_end (what the last
    // pmap_steal_memory left) to the upper layers of the VM.
    let (start, end) = <Machine as Pmap>::pmap_steal_bounds();
    *kvm_startp = Vaddr::new(round_page(start.as_usize()));
    *kvm_endp = Vaddr::new(trunc_page(end.as_usize()));
}

/// `uvm_page_physload`: load physical memory into VM system. All args are PFs; all pages in
/// start/end get vm_page structures; areas marked by avail_start/avail_end get added to the
/// free page pool; we are limited to `VM_PHYSSEG_MAX` physical memory segments.
pub fn uvm_page_physload(
    start: usize,
    end: usize,
    avail_start: usize,
    avail_end: usize,
    flags: i32,
) {
    match kernel::vm::page::uvm_page_physload(start, end, avail_start, avail_end, flags, SEG_POLICY)
    {
        Ok(()) => {}
        Err(PhysloadError::Full { max }) => {
            kprintf!("uvm_page_physload: unable to load physical memory segment\n");
            kprintf!(
                "\t{} segments allocated, ignoring {:#x} -> {:#x}\n",
                max,
                start,
                end
            );
            kprintf!("\tincrease VM_PHYSSEG_MAX\n");
        }
        Err(PhysloadError::AfterInit) => {
            kprintf!("uvm_page_physload: can not malloc vm_page structs for segment\n");
            kprintf!("\tignoring {:#x} -> {:#x}\n", start, end);
        }
    }
}

/// `uvm_page_physdump`: prints the physical memory config (XXXCDC: TMP TMP TMP DEBUG DEBUG
/// DEBUG). Call from DDB.
pub fn uvm_page_physdump() {
    let mut boot = [VmPhysseg::EMPTY; MAX_SEGS];
    let segs: &[VmPhysseg] = if page_init_done() {
        vm_physmem()
    } else {
        let n = boot_segments(&mut boot);
        &boot[..n]
    };
    kprintf!(
        "uvm_page_physdump: physical memory config [segs={} of {}]:\n",
        segs.len(),
        VM_PHYSSEG_MAX
    );
    for seg in segs {
        kprintf!(
            "{:#x}->{:#x} [{:#x}->{:#x}]\n",
            seg.start,
            seg.end,
            seg.avail_start,
            seg.avail_end
        );
    }
    kprintf!("STRATEGY = ");
    match VM_PHYSSEG_STRAT {
        VM_PSTRAT_RANDOM => kprintf!("RANDOM\n"),
        VM_PSTRAT_BSEARCH => kprintf!("BSEARCH\n"),
        VM_PSTRAT_BIGFIRST => kprintf!("BIGFIRST\n"),
        _ => kprintf!("<<UNKNOWN>>!!!!\n"),
    };
}

/// `uvm_pmr_print` for ddb: the kernel allocator's ranges through `printf`.
pub fn uvm_pmr_print() {
    struct Printf;
    impl Write for Printf {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            kprintf!("{}", s);
            Ok(())
        }
    }
    kernel::vm::pmemrange::uvm_pmr_print(&mut Printf);
}

/// `uvm_shutdown`: what the VM does on the way down.
pub fn uvm_shutdown() {
    // UVM_SWAP_ENCRYPT: uvm_swap_finicrypt_all(): not configured.
    smr_flush();
}

/// `uvm_pagealloc`: allocate vm_page from a particular free list. Returns `None` if no pages
/// free; only one of obj or anon can be non-null; caller must activate/deactivate page if it
/// is not wired.
pub fn uvm_pagealloc(
    obj: Option<&UvmObject>,
    off: Voff,
    anon: Option<&VmAnon>,
    flags: i32,
) -> Option<&'static VmPage> {
    kernel::vm::page::uvm_pagealloc(owner(obj, anon), off, flags)
}

/// `uvm_pagealloc_multi`: interface used by the buffer cache to allocate a buffer at a time.
pub fn uvm_pagealloc_multi(
    obj: &UvmObject,
    off: Voff,
    size: usize,
    flags: i32,
) -> Result<(), Errno> {
    kassert!(crate::uvm::uvm_object::uvm_obj_is_bufcache(obj));
    kernel_assert_locked();
    let plist = Pglist::new();
    let r = uvm_pglistalloc(
        size,
        crate::uvm::uvm_km::NO_CONSTRAINT.ucr_low,
        crate::uvm::uvm_km::NO_CONSTRAINT.ucr_high,
        Paddr::new(0),
        Paddr::new(0),
        &plist,
        // No MMU (the host double): the buffer cache reaches the pages through the direct
        // map, so they must be one segment (see the module's deviations).
        if <Machine as Pmap>::PMAP_NOMMU {
            1
        } else {
            atop(round_page(size)) as i32
        },
        flags,
    );
    if r.is_ok() {
        let mut i = 0;
        while let Some(pg) = plist.pop_head() {
            pg.set_wire_count(1);
            pg.set_bits(PG_CLEAN | PG_FAKE);
            kassert!(pg.flags() & PG_DEV == 0);
            uvm_pagealloc_pg(pg, Owner::Object(obj), off + ptoa(i) as Voff);
            i += 1;
        }
    }
    r
}

/// `uvm_pagewait`: wait for a busy page of `owner`. Page must be known `PG_BUSY`; the owner's
/// `lock` must be held; it is released on return.
pub fn uvm_pagewait(pg: &VmPage, owner: Owner<'_>, lock: &Rwlock, wmesg: &'static str) {
    kassert!(rw_lock_held(lock));
    kassert!(pg.flags() & PG_BUSY != 0);
    kassert!(uvm_page_owner_locked_p(pg, owner, false));

    pg.set_bits(PG_WANTED);
    let _ = rwsleep_nsec(pg.chan(), lock, PVM | PNORELOCK, wmesg, INFSLP);
}

const _: () = {
    // The kernel's segment table holds the machine's.
    assert!(VM_PHYSSEG_MAX <= MAX_SEGS);
    // uvm_page_physload: unknown physseg strategy selected!
    assert!(matches!(
        VM_PHYSSEG_STRAT,
        VM_PSTRAT_RANDOM | VM_PSTRAT_BSEARCH | VM_PSTRAT_BIGFIRST
    ));
};
/* </CODE> */
