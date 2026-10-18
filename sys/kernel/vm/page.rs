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
 * from: Id: uvm.h,v 1.1.2.14 1998/02/02 20:07:19 chuck Exp
 */
/* </LICENSES> */

/* <CODE> */
//! Resident memory: `struct vm_page`, the physical segments and the page operations of
//! `<uvm/uvm_page.h>` and `uvm/uvm_page.c`, with the page fields of `struct uvm` (`<uvm/uvm.h>`).
//!
//! Upstream: sys/uvm/uvm_page.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_page.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm.h @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_page.rs@3c62ede68802
//! LZ: sys/uvm/uvm.rs@3c62ede68802
//!
//! Management of resident (logical) pages. A [`VmPage`] is kept for each resident page, all of
//! them in one array stolen at boot ([`uvm_page_init`]) and never freed, so a page is a `&'static VmPage`
//! for real. Each page carries a list entry (`pageq`: the active and inactive queues, the free
//! lists of `pmemrange`, any caller's [`Pglist`]) and a tree entry (`objt`: its object's
//! pages by offset, or a free range's place in its pmemrange's trees), its owner, its offset
//! in the owner, status bits and the pmap's data (`hal::pmap::PageMd`, embedded).
//!
//! Locks, as in the C: `I` immutable after creation, `a` atomic operations, `Q` the page
//! queues ([`PageQueues`], the data of the queue mutex, `uvm.pageqlock`), `F` the free page
//! state (`pmemrange`'s, `uvm.fpageqlock`), `o` the owner's lock (`uobject->vmobjlock` or
//! `uanon->an_lock`). The lock order is the C's: `Q` before `F`; the owner's lock before `Q`;
//! `Q` before the pmap's `pv_mtx`.
//!
//! The physical segments (`vm_physmem[]`) are written while the system boots
//! ([`uvm_page_physload`], [`uvm_page_physsteal`] behind `hal::pmap::steal_memory`) and frozen by [`uvm_page_init`] into
//! a table published once, which [`page_of`] (`PHYS_TO_VM_PAGE`) reads without a lock.
//!
//! The owners (`struct uvm_object`, `struct vm_anon`) have not moved (N3.4): a page records
//! its owner's identity ([`OwnerTag`]) and never follows it; every operation whose C body
//! reaches `pg->uobject` or `pg->uanon` takes the owner from its caller ([`Owner`]), who holds
//! the owner's lock by the C's `[o]` rule, through the two traits bsd's `UvmObject` and
//! `VmAnon` implement ([`PageObject`], [`PageAnon`]; bridges.toml `PageOwner`). An owner that
//! is not the page's is refused (a panic): the C would act on the page's own.
//!
//! ## Deviations
//! - The `!PMAP_STEAL_MEMORY` path of `uvm_pageboot_alloc` (`pmap_virtual_space`, a frame at a
//!   time from `uvm_page_physget`, `pmap_kenter_pa` per page) is not written: every backend
//!   of `hal::pmap` (amd64, arm64, the host double) defines `PMAP_STEAL_MEMORY`, and the boot
//!   allocations are `hal::bootmem`'s over `steal_memory`. `uvm_page_physget` had no other
//!   caller.
//! - `uvm_page_physload` after `uvm_page_init` needs `km_alloc` for the new segment's pages; as
//!   in LZ the segment is ignored, which is what the C does when that allocation fails:
//!   [`uvm_page_physload`] answers [`PhysloadError::AfterInit`] and the caller (`bsd`'s
//!   `uvm_page_physload`) prints the C's two lines.
//! - `uvm_pagezero_thread` (`uvm_pmemrange.c`, started by `main()`) is not here: it needs a
//!   kernel thread (N4). Zeroed ranges still exist and `UVM_PLA_ZERO` zeroes on demand.
//! - There is no page daemon (N3.5): a `UVM_PLA_WAITOK` request that cannot be met reports
//!   `unported: uvm_wait (uvm_pdaemon.c, M5)` once (`pmemrange::uvm_wait`) and fails with
//!   `ENOMEM` (LZ's deviation, kept).
//! - `uvm_pagewait` (a sleep on a legacy `rw_obj` lock), `uvm_pagealloc_multi` (the buffer
//!   cache's, under the kernel lock), `uvm_shutdown` (`smr_flush`) and the DMA reachability of
//!   `machine::Pmap` stay in `bsd`'s `sys/uvm/uvm_page.rs`, over [`VmPage::set_bits`] and
//!   [`uvm_pagealloc_pg`].
//!
//! ## Redesign
//! - `struct vm_page`'s `uobject`/`uanon` back pointers (`Cell<*const T>`, followed by every
//!   caller) -> an [`OwnerTag`] (the owner's address as a number, its kind in the low bit),
//!   compared and never dereferenced, and an [`Owner`] argument to [`uvm_pagealloc`],
//!   [`uvm_pagealloc_pg`], [`uvm_pagerealloc`], [`uvm_pageclean`], [`uvm_pagefree`], [`uvm_page_unbusy`],
//!   [`uvm_pagewire`], [`uvm_pageunwire`], [`uvm_pageactivate`], [`uvm_pagedeactivate`], [`uvm_pagedequeue`] and
//!   [`uvm_page_owner_locked_p`]; `pg->uobject == obj` is [`VmPage::owned_by_object`].
//! - The `[o]`/`[F]` scalars (`offset`, `wire_count`, `pg_version`, `fpgsz`) are relaxed
//!   atomics written under their lock (decision 12's shape); `pg_flags` stays atomic.
//! - `pg + n` in the array is `pages()[pg.index + n]` (a bounds-checked slice index: each page
//!   knows its position, [`VmPage::index`]); the pmemrange's "the array broke" test is
//!   `prev.index + prev.fpgsz == pg.index`. `VmPage::{add, sub, ptr_add, ptr_sub, forever}`
//!   are gone, and a `&VmPage` becomes the `&'static` the lists need by its index.
//! - `vm_physmem[]` (a `StaticCell` written by `unsafe` boot code) -> the boot table inside a
//!   mutex ([`uvm_page_physload`], [`uvm_page_physsteal`]) and the frozen table [`uvm_page_init`] publishes once
//!   (`PHYS_TO_VM_PAGE`, [`vm_physmem`]); the machine's `VM_PHYSSEG_MAX`/`VM_PHYSSEG_STRAT` arrive as a
//!   [`SegPolicy`] from the caller, the machine's data passed down (N3.2's D5), so an unknown
//!   strategy (the C's panic) cannot be expressed.
//! - `uvm.page_active`, `uvm.page_inactive` and `uvm.pageqlock` -> [`PageQueues`] inside a
//!   `Mutexed` ([`uvm_lock_pageq`]); `uvm_pagedequeue`'s `MUTEX_ASSERT_LOCKED(&uvm.pageqlock)` is
//!   its `&mut PageQueues` parameter, a check made a type. `uvm.page_init_done` is an atomic
//!   ([`page_init_done`]). The other fields of `struct uvm` stay in `bsd` (N3.5).
//! - `uvm_pagelookup` searches the object's tree with a comparison ([`hal::tree::Tree::find_by`])
//!   instead of an on-stack key page.
//! - `hal::pmap::PhysPages` is implemented here ([`PHYS_PAGES`]), passed by the boot glue to
//!   `hal::pmap::bootstrap`.

use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicUsize, Ordering as Mem};

use hal::arch::current::param::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use hal::bootmem;
use hal::intr::IPL_VM;
use hal::link::Link;
use hal::list::{List, ListEntry};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::pmap::{PageMd, PhysPages};
use hal::sleep::wakeup;
use hal::sys::errno::Errno;
use hal::sys::mman::PROT_NONE;
use hal::sys::types::Paddr;
use hal::tree::{Tree, TreeEntry};
use hal::vm::param::{DEFAULT_PAGE_SIZE, atop, ptoa, round_page, trunc_page};
use hal::vm::uvmexp::UVMEXP;

use super::pmemrange;
use super::types::{
    UVM_PGA_USERESERVE, UVM_PGA_ZERO, UVM_PLA_NOWAIT, UVM_PLA_USERESERVE, UVM_PLA_WAITOK,
    UVM_PLA_ZERO, UvmConstraintRange, Voff,
};
use crate::assert::{kassert, kpanic};
use crate::prf::printf;

// pg_flags: object flags

/// VmPage is locked.
pub const PG_BUSY: u32 = 0x0000_0001;
/// Someone is waiting for page.
pub const PG_WANTED: u32 = 0x0000_0002;
/// VmPage is in VP table.
pub const PG_TABLED: u32 = 0x0000_0004;
/// VmPage has not been modified.
pub const PG_CLEAN: u32 = 0x0000_0008;
/// Clean bit has been checked.
pub const PG_CLEANCHK: u32 = 0x0000_0010;
/// VmPage released while paging.
pub const PG_RELEASED: u32 = 0x0000_0020;
/// VmPage is not yet initialized.
pub const PG_FAKE: u32 = 0x0000_0040;
/// VmPage must be mapped read-only.
pub const PG_RDONLY: u32 = 0x0000_0080;
/// VmPage is pre-zero'd.
pub const PG_ZERO: u32 = 0x0000_0100;
/// VmPage is in device space, lay off.
pub const PG_DEV: u32 = 0x0000_0200;
/// Mask of the object flags.
pub const PG_MASK: u32 = 0x0000_ffff;

/// VmPage is on free list.
pub const PQ_FREE: u32 = 0x0001_0000;
/// VmPage is in inactive list.
pub const PQ_INACTIVE: u32 = 0x0002_0000;
/// VmPage is in active list.
pub const PQ_ACTIVE: u32 = 0x0004_0000;
/// VmPage is an iterator marker.
pub const PQ_ITER: u32 = 0x0008_0000;
/// VmPage is part of an anon, rather than an uvm_object.
pub const PQ_ANON: u32 = 0x0010_0000;
/// VmPage is part of an anonymous uvm_object.
pub const PQ_AOBJ: u32 = 0x0020_0000;
/// Swap-backed pages.
pub const PQ_SWAPBACKED: u32 = PQ_ANON | PQ_AOBJ;
/// VmPage needs {en,de}cryption.
pub const PQ_ENCRYPT: u32 = 0x0040_0000;
/// Mask of the queue flags.
pub const PQ_MASK: u32 = 0x00ff_0000;

/// The most physical segments any machine asks for (`VM_PHYSSEG_MAX`: amd64 16, arm64 32): the
/// capacity of the segment table; a machine's own limit is its [`SegPolicy::max`].
pub const MAX_SEGS: usize = 32;

/// `struct vm_page`.
pub struct VmPage {
    /// \[Q\] LRU or free page queue (\[F\] on a free list).
    pub(super) pageq: ListEntry<VmPage>,
    /// \[o\] object tree (\[F\] a free range's trees, `pmemrange`).
    pub(super) objt: TreeEntry<VmPage>,
    /// \[o\] the owner's [`OwnerTag`] (0: none).
    owner: AtomicUsize,
    /// \[o\] offset into object (a relaxed atomic written under the owner's lock).
    offset: AtomicI64,
    /// \[a\] object flags.
    pg_flags: AtomicU32,
    /// \[o\] version count (relaxed, under the owner's lock or `F`).
    pub(super) pg_version: AtomicU32,
    /// \[o\] wired down map refs (relaxed, under the owner's lock).
    wire_count: AtomicU32,
    /// \[I\] physical address.
    phys_addr: Paddr,
    /// \[I\] the page's position in the page array (`pg - pagearray`).
    index: u32,
    /// \[F\] free page range size, in pages (relaxed, under `F`).
    pub(super) fpgsz: AtomicUsize,
    /// Pmap-specific data (`struct vm_page_md`).
    pub mdpage: PageMd,
}

impl VmPage {
    /// The page structure for the frame at `phys_addr`, slot `index` of the array: what
    /// `uvm_page_init` leaves after zeroing the array and setting `phys_addr` and
    /// `VM_MDPAGE_INIT`.
    const fn new(phys_addr: Paddr, index: u32) -> Self {
        Self {
            pageq: ListEntry::new(),
            objt: TreeEntry::new(),
            owner: AtomicUsize::new(0),
            offset: AtomicI64::new(0),
            pg_flags: AtomicU32::new(0),
            pg_version: AtomicU32::new(0),
            wire_count: AtomicU32::new(0),
            phys_addr,
            index,
            fpgsz: AtomicUsize::new(0),
            mdpage: PageMd::new(),
        }
    }

    /// `VM_PAGE_TO_PHYS(pg)`.
    pub fn phys_addr(&self) -> Paddr {
        self.phys_addr
    }

    /// The page's position in the page array (the C's `pg - pagearray`).
    pub fn index(&self) -> usize {
        self.index as usize
    }

    /// `pg_flags`.
    pub fn flags(&self) -> u32 {
        self.pg_flags.load(Mem::Relaxed)
    }

    /// `atomic_setbits_int(&pg->pg_flags, bits)`.
    pub fn set_bits(&self, bits: u32) {
        self.pg_flags.fetch_or(bits, Mem::Relaxed);
    }

    /// `atomic_clearbits_int(&pg->pg_flags, bits)`.
    pub fn clear_bits(&self, bits: u32) {
        self.pg_flags.fetch_and(!bits, Mem::Relaxed);
    }

    /// `pg->offset` (\[o\]).
    pub fn offset(&self) -> Voff {
        self.offset.load(Mem::Relaxed)
    }

    /// `pg->wire_count` (\[o\]).
    pub fn wire_count(&self) -> u32 {
        self.wire_count.load(Mem::Relaxed)
    }

    /// `pg->wire_count = n`, under the owner's lock (the buffer cache's pages, a pmap's PTPs).
    pub fn set_wire_count(&self, n: u32) {
        self.wire_count.store(n, Mem::Relaxed);
    }

    /// `pg->pg_version`.
    pub fn version(&self) -> u32 {
        self.pg_version.load(Mem::Relaxed)
    }

    /// The owner's identity (`OwnerTag::NONE` without one).
    pub fn owner_tag(&self) -> OwnerTag {
        OwnerTag(self.owner.load(Mem::Relaxed))
    }

    /// Whether the page has an owner (`pg->uobject != NULL || pg->uanon != NULL`).
    pub fn has_owner(&self) -> bool {
        self.owner_tag() != OwnerTag::NONE
    }

    /// `pg->uobject == obj`.
    pub fn owned_by_object(&self, obj: &dyn PageObject) -> bool {
        self.owner_tag() == obj.tag()
    }

    /// `pg->uanon == anon`.
    pub fn owned_by_anon(&self, anon: &dyn PageAnon) -> bool {
        self.owner_tag() == anon.tag()
    }

    /// Whether an object owns the page (`pg->uobject != NULL`).
    pub fn has_object(&self) -> bool {
        let t = self.owner_tag();
        t != OwnerTag::NONE && !t.is_anon()
    }

    /// Whether an anon owns the page (`pg->uanon != NULL`).
    pub fn has_anon(&self) -> bool {
        self.owner_tag().is_anon()
    }

    /// `VM_PAGE_IS_FREE(pg)`.
    pub fn is_free(&self) -> bool {
        self.flags() & PQ_FREE != 0
    }

    /// The page as a wait channel (`tsleep(pg, ..)`, `wakeup(pg)`): its address.
    pub fn chan(&self) -> *const () {
        hal::sleep::chan(ptr::from_ref(self))
    }

    /// The owner identity of the page, set under the owner's lock.
    fn set_owner(&self, t: OwnerTag) {
        self.owner.store(t.0, Mem::Relaxed);
    }

    fn set_offset(&self, off: Voff) {
        self.offset.store(off, Mem::Relaxed);
    }

    fn bump_version(&self) {
        self.pg_version.fetch_add(1, Mem::Relaxed);
    }
}

hal::list_adapter!(
    /// The `pageq` list adapter: the page queues, the free lists, any caller's page list.
    pub Pageq: VmPage, pageq
);

/// `struct pglist`: a `TAILQ` of pages through `pageq`.
pub type Pglist = List<Pageq>;

/// `uvm_pagecmp`: orders an object's pages by offset.
pub fn uvm_pagecmp(a: &VmPage, b: &VmPage) -> core::cmp::Ordering {
    a.offset().cmp(&b.offset())
}

hal::tree_adapter!(
    /// `uvm_objtree`: an object's pages by offset.
    pub UvmObjtree: VmPage, objt, uvm_pagecmp
);

/// The identity of a page's owner: the owner's address as a number, an anon's with the low bit
/// set (0: no owner). Compared, never dereferenced (see the module's notes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OwnerTag(usize);

impl OwnerTag {
    /// No owner.
    pub const NONE: OwnerTag = OwnerTag(0);

    /// The identity of the object at `obj` (`&uobj->...`; an object is at least 2-aligned).
    pub fn object<T: ?Sized>(obj: &T) -> Self {
        Self(ptr::from_ref(obj).cast::<u8>().addr() & !1)
    }

    /// The identity of the anon at `anon`.
    pub fn anon<T: ?Sized>(anon: &T) -> Self {
        Self(ptr::from_ref(anon).cast::<u8>().addr() | 1)
    }

    /// Whether the owner is an anon.
    pub fn is_anon(self) -> bool {
        self.0 & 1 != 0
    }
}

/// An object that owns pages (`struct uvm_object`, still `bsd`'s: bridges.toml `PageOwner`,
/// N3.4). Implemented by `bsd`'s `UvmObject`, passed as a value, never stored.
pub trait PageObject {
    /// Its identity (`OwnerTag::object(self)`).
    fn tag(&self) -> OwnerTag;
    /// `UVM_OBJ_IS_DUMMY(uobj)`: a pager without a lock (the pmap's, the buffer cache's).
    fn is_dummy(&self) -> bool;
    /// `UVM_OBJ_IS_KERN_OBJECT(uobj)`: the kernel object, whose pages may use the reserve.
    fn is_kern_object(&self) -> bool;
    /// `rw_write_held(uobj->vmobjlock)` (`exclusive`) or `rw_lock_held`.
    fn locked(&self, exclusive: bool) -> bool;
    /// `uobj->memt`: the object's pages by offset.
    fn pages(&self) -> &Tree<UvmObjtree>;
    /// `uobj->uo_npages`.
    fn npages(&self) -> i32;
    /// `uobj->uo_npages = n` (under the object's lock).
    fn set_npages(&self, n: i32);
}

/// An anon that owns a page (`struct vm_anon`, still `bsd`'s: bridges.toml `PageOwner`, N3.4).
/// Implemented by `bsd`'s `VmAnon`, passed as a value, never stored.
pub trait PageAnon {
    /// Its identity (`OwnerTag::anon(self)`).
    fn tag(&self) -> OwnerTag;
    /// `rw_write_held(anon->an_lock)` (`exclusive`) or `rw_lock_held`; `None` when the anon
    /// has no lock (`an_lock == NULL`).
    fn locked(&self, exclusive: bool) -> Option<bool>;
    /// `anon->an_ref`.
    fn refs(&self) -> i32;
    /// `anon->an_page = pg` (under the anon's lock).
    fn set_page(&self, pg: Option<&'static VmPage>);
}

/// The owner a caller names for a page: the C's `(obj, anon)` pair, at most one of them.
#[derive(Clone, Copy)]
pub enum Owner<'a> {
    /// No owner (`obj == NULL && anon == NULL`).
    None,
    /// An object (`obj`).
    Object(&'a dyn PageObject),
    /// An anon (`anon`).
    Anon(&'a dyn PageAnon),
}

impl Owner<'_> {
    /// The owner's identity.
    pub fn tag(self) -> OwnerTag {
        match self {
            Owner::None => OwnerTag::NONE,
            Owner::Object(o) => o.tag(),
            Owner::Anon(a) => a.tag(),
        }
    }
}

/// `VM_PSTRAT_*`: how the segment table is kept (`VM_PHYSSEG_STRAT`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SegStrat {
    /// `VM_PSTRAT_RANDOM`: in load order.
    Random,
    /// `VM_PSTRAT_BSEARCH`: sorted by address, searched by bisection.
    Bsearch,
    /// `VM_PSTRAT_BIGFIRST`: largest segment first, searched linearly.
    Bigfirst,
}

/// The machine's segment policy: `VM_PHYSSEG_MAX` (at most [`MAX_SEGS`]) and
/// `VM_PHYSSEG_STRAT`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SegPolicy {
    /// `VM_PHYSSEG_MAX`.
    pub max: usize,
    /// `VM_PHYSSEG_STRAT`.
    pub strat: SegStrat,
}

/// `struct vm_physseg`: one segment of physical memory, in page frame numbers.
#[derive(Clone, Copy, Debug)]
pub struct VmPhysseg {
    /// PF# of first page in segment.
    pub start: usize,
    /// (PF# of last page in segment) + 1.
    pub end: usize,
    /// PF# of first free page in segment.
    pub avail_start: usize,
    /// (PF# of last free page in segment) + 1.
    pub avail_end: usize,
    /// The array index of the page of `start` (the C's `pgs`), set by [`uvm_page_init`].
    first: usize,
}

impl VmPhysseg {
    /// An unused slot of the table.
    pub const EMPTY: VmPhysseg = VmPhysseg {
        start: 0,
        end: 0,
        avail_start: 0,
        avail_end: 0,
        first: 0,
    };
}

/// Why [`uvm_page_physload`] ignored a segment (the caller prints the C's lines).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PhysloadError {
    /// `vm_nphysseg == VM_PHYSSEG_MAX`: "unable to load physical memory segment".
    Full {
        /// The machine's `VM_PHYSSEG_MAX`.
        max: usize,
    },
    /// After `uvm_page_init` (the `km_alloc` path, module deviations): "can not malloc vm_page
    /// structs for segment".
    AfterInit,
}

/// `vm_physmem[]` while the system boots: written by [`uvm_page_physload`] and [`uvm_page_physsteal`].
struct BootSegs {
    segs: [VmPhysseg; MAX_SEGS],
    n: usize,
    policy: SegPolicy,
}

impl BootSegs {
    const fn new() -> Self {
        Self {
            segs: [VmPhysseg::EMPTY; MAX_SEGS],
            n: 0,
            policy: SegPolicy {
                max: MAX_SEGS,
                strat: SegStrat::Random,
            },
        }
    }

    /// Removes segment `lcv`, shifting the later ones down (the C's `seg[0] = seg[1]` loop).
    fn remove(&mut self, lcv: usize) {
        if self.n == 1 {
            kpanic(format_args!("uvm_page_physget: out of memory!"));
        }
        self.segs.copy_within(lcv + 1..self.n, lcv);
        self.n -= 1;
    }
}

/// The frozen segment table and the page array, as [`uvm_page_init`] publishes them.
struct TableInner {
    segs: [VmPhysseg; MAX_SEGS],
    n: usize,
    strat: SegStrat,
    pages: &'static [VmPage],
}

impl TableInner {
    /// `vm_physseg_find`: the segment holding page frame `pframe` and the frame's offset in it.
    fn find(&self, pframe: usize) -> Option<(&VmPhysseg, usize)> {
        let segs = &self.segs[..self.n];
        if self.strat == SegStrat::Bsearch {
            // Bisection: a try too large halves the length; a try too small moves the start
            // past it ("adjust": round(len/2) - 1 == trunc((len - 1) / 2)).
            let mut start = 0;
            let mut len = segs.len();
            while len != 0 {
                let t = start + len / 2;
                let seg = &segs[t];
                if pframe >= seg.start {
                    if pframe < seg.end {
                        return Some((seg, pframe - seg.start));
                    }
                    start = t + 1;
                    len -= 1;
                }
                len /= 2;
            }
            None
        } else {
            segs.iter()
                .find(|seg| pframe >= seg.start && pframe < seg.end)
                .map(|seg| (seg, pframe - seg.start))
        }
    }
}

/// A published table: stolen (empty) before the page array, filled once after it, so no steal
/// comes between the array's layout and the pages it frees.
struct PageTable {
    inner: OnceStatic<TableInner>,
}

/// `uvm.page_active` and `uvm.page_inactive`, the data of `uvm.pageqlock`.
pub struct PageQueues {
    /// \[Q\] allocated pages, in use.
    active: Pglist,
    /// \[Q\] pages inactive (reclaim/free).
    inactive: Pglist,
}

impl PageQueues {
    const fn new() -> Self {
        Self {
            active: Pglist::new(),
            inactive: Pglist::new(),
        }
    }

    /// The first page of the active queue.
    pub fn first_active(&self) -> Option<&'static VmPage> {
        self.active.first()
    }

    /// The first page of the inactive queue.
    pub fn first_inactive(&self) -> Option<&'static VmPage> {
        self.inactive.first()
    }
}

/// The page provider of `hal::pmap` (`hal::pmap::PhysPages`), over this module's functions.
pub struct PhysPagesImpl;

/// The segment table while the system boots.
static BOOT: Mutexed<BootSegs> = Mutexed::new(IPL_VM, BootSegs::new());
/// The frozen table, set by [`uvm_page_init`] (`None` before; a host test's reset empties it).
static TABLE: Link<PageTable> = Link::new();
/// `uvm.pageqlock` and the queues it guards.
static PAGEQ: Mutexed<PageQueues> = Mutexed::new(IPL_VM, PageQueues::new());
/// `uvm.page_init_done`.
static PAGE_INIT_DONE: AtomicBool = AtomicBool::new(false);
/// `&uvm.pagedaemon`, the page daemon's wait channel (its address; the daemon is N3.5's).
static PAGEDAEMON: u8 = 0;
/// `pmap_steal_memory`'s provider, handed to `hal::pmap::bootstrap` by the boot glue.
pub static PHYS_PAGES: PhysPagesImpl = PhysPagesImpl;

/// The published table, once [`uvm_page_init`] has run.
fn table() -> Option<&'static TableInner> {
    TABLE.get()?.inner.get()
}

/// `VM_PAGE_TO_PHYS(pg)`.
pub fn vm_page_to_phys(pg: &VmPage) -> Paddr {
    pg.phys_addr
}

/// `VM_PAGE_IS_FREE(pg)`.
pub fn vm_page_is_free(pg: &VmPage) -> bool {
    pg.is_free()
}

/// The page array (empty before [`uvm_page_init`]): `pages()[i]` is the C's `pagearray[i]`.
pub fn pages() -> &'static [VmPage] {
    table().map_or(&[], |t| t.pages)
}

/// The segments frozen by [`uvm_page_init`] (`vm_physmem[0..vm_nphysseg]`; empty before).
pub fn vm_physmem() -> &'static [VmPhysseg] {
    table().map_or(&[], |t| &t.segs[..t.n])
}

/// `uvm.page_init_done`.
pub fn page_init_done() -> bool {
    PAGE_INIT_DONE.load(Mem::Acquire)
}

/// The page daemon's wait channel (`&uvm.pagedaemon`).
pub fn pagedaemon_chan() -> *const () {
    hal::sleep::chan(ptr::from_ref(&PAGEDAEMON))
}

/// `uvm_lock_pageq()`: the page queues, locked until the guard drops (`uvm_unlock_pageq()`).
pub fn uvm_lock_pageq() -> MutexedGuard<'static, PageQueues> {
    PAGEQ.lock()
}

/// `pg` with the lifetime of the array, found by its index; panics when `pg` is not a page of
/// the array (a marker or a copy).
pub fn static_of(pg: &VmPage) -> &'static VmPage {
    match pages().get(pg.index()) {
        Some(p) if ptr::eq(p, pg) => p,
        _ => kpanic(format_args!(
            "page {:#x} is not in the page array",
            pg.phys_addr.as_usize()
        )),
    }
}

/// `pg + n`: the page `n` slots further into the array; panics past its end.
pub fn nth(pg: &VmPage, n: usize) -> &'static VmPage {
    match pages().get(pg.index() + n) {
        Some(p) => p,
        None => kpanic(format_args!("page {} + {n}: past the page array", pg.index)),
    }
}

/// `pg - n`: the page `n` slots earlier in the array; panics before its start.
pub fn nth_back(pg: &VmPage, n: usize) -> &'static VmPage {
    match pg.index().checked_sub(n).and_then(|i| pages().get(i)) {
        Some(p) => p,
        None => kpanic(format_args!(
            "page {} - {n}: before the page array",
            pg.index
        )),
    }
}

/// `UVM_PAGEZERO_TARGET`: how many zeroed pages the zeroing thread keeps ready.
pub fn uvm_pagezero_target() -> i32 {
    UVMEXP.free.load(Mem::Relaxed) / 8
}

/// Panics unless `owner` is `pg`'s (`what` names the operation): the C acts on the page's own
/// owner, which the caller here names.
fn check_owner(pg: &VmPage, owner: Owner<'_>, what: &str) {
    if pg.owner_tag() != owner.tag() {
        kpanic(format_args!(
            "{what}: page {:#x} is not owned by the owner given",
            pg.phys_addr.as_usize()
        ));
    }
}

/// `uvm_pageinsert`: inserts `pg` in `obj`'s tree. The caller locks the object and has set
/// the page's owner and offset and bumped its version.
fn uvm_pageinsert(pg: &'static VmPage, obj: &dyn PageObject) {
    kassert!(obj.is_dummy() || obj.locked(true));
    kassert!(pg.flags() & PG_TABLED == 0);

    let dupe = obj.pages().insert(pg);
    // not allowed to insert over another page
    kassert!(dupe.is_none());
    let _ = dupe;
    pg.set_bits(PG_TABLED);
    obj.set_npages(obj.npages() + 1);
}

/// `uvm_pageremove`: removes `pg` from `obj`, its owner. The caller locks the object.
fn uvm_pageremove(pg: &VmPage, obj: &dyn PageObject) {
    kassert!(obj.is_dummy() || obj.locked(true));
    kassert!(pg.flags() & PG_TABLED != 0);

    obj.pages().remove(pg);

    pg.clear_bits(PG_TABLED);
    obj.set_npages(obj.npages() - 1);
    pg.set_owner(OwnerTag::NONE);
    pg.bump_version();
}

/// `uvm_page_init`: inits the page system (`uvm_init`'s first step): the pmemranges, the page
/// array for every page of the loaded segments, their free pages handed to the allocator, the
/// reserves. `constraints` are the machine's DMA ranges (`uvm_md_constraints[]`). The kernel
/// virtual range left for the upper layers is the pmap's after the last steal (the caller's
/// `pmap_steal_bounds`). Once per boot (per host test, after [`uvm_page_test_reset`]).
pub fn uvm_page_init(constraints: &[&UvmConstraintRange]) {
    // The page queues and their lock are statics, empty until now.
    pmemrange::uvm_pmr_init(constraints);

    // sanity check: the machine has registered some free RAM with physload().
    let (nseg, freepages) = {
        let b = BOOT.lock();
        let free: usize = b.segs[..b.n].iter().map(|s| s.end - s.start).sum();
        (b.n, free)
    };
    if nseg == 0 {
        kpanic(format_args!("uvm_page_bootstrap: no memory pre-allocated"));
    }

    // The table first (see `PageTable`), then the array. We count start..end rather than
    // avail_start..avail_end, so there are page structures for memory given back later; each
    // page used costs PAGE_SIZE plus its structure, and one more page is a fudge factor
    // against truncation.
    let shell = bootmem::steal_one(|| PageTable {
        inner: OnceStatic::new(),
    });
    let pagecount = ((freepages + 1) << PAGE_SHIFT) / (PAGE_SIZE + size_of::<VmPage>());
    if u32::try_from(pagecount).is_err() {
        kpanic(format_args!("uvm_page_init: {pagecount} pages"));
    }
    // The layout is the segment table after the array's own steal, read at the first page.
    let mut layout: Option<Layout> = None;
    let pages = bootmem::steal_slice(pagecount, |i| {
        let l = layout.get_or_insert_with(|| Layout::of(&BOOT.lock()));
        VmPage::new(l.pa_of(i), i as u32)
    });
    let l = match layout {
        Some(l) => l,
        None => Layout::of(&BOOT.lock()),
    };
    let (mut segs, n) = (l.segs, l.n);
    let strat = BOOT.lock().policy.strat;

    // The page array pointers of each segment, and the pages counted.
    let mut left = pagecount;
    let mut first = 0;
    let mut npages = 0;
    for seg in &mut segs[..n] {
        let n = seg.end - seg.start;
        if n > left {
            // XXXCDC: shouldn't happen?
            kpanic(format_args!(
                "uvm_page_init: lost {} page(s) in init",
                n - left
            ));
        }
        seg.first = first;
        first += n;
        left -= n;
        npages += seg
            .end
            .min(seg.avail_end)
            .saturating_sub(seg.start.max(seg.avail_start));
    }
    UVMEXP.npages.fetch_add(npages as i32, Mem::Relaxed);

    if shell
        .inner
        .set(TableInner {
            segs,
            n,
            strat,
            pages,
        })
        .is_err()
    {
        kpanic(format_args!("uvm_page_init: the table is set"));
    }
    TABLE.set(Some(shell));

    // Add the available pages to the free pool.
    for seg in &segs[..n] {
        if seg.avail_end > seg.avail_start {
            let pg = &pages[seg.first + (seg.avail_start - seg.start)];
            pmemrange::uvm_pmr_freepages(pg, seg.avail_end - seg.avail_start);
        }
    }

    // init reserve thresholds.
    //
    // XXX As long as some disk drivers cannot write any physical page, we need DMA reachable
    // reserves for the pagedaemon. We cannot enforce such requirement but it should be ok in
    // most of the cases because the pmemrange tries hard to allocate them last.
    UVMEXP.reserve_pagedaemon.store(32, Mem::Relaxed);
    UVMEXP.reserve_kernel.store(32 + 32, Mem::Relaxed);

    PAGE_INIT_DONE.store(true, Mem::Release);
}

/// The page array's layout: the boot table as it is once the array has been stolen, and a
/// cursor that maps array slots to frames in order.
struct Layout {
    segs: [VmPhysseg; MAX_SEGS],
    n: usize,
    seg: usize,
    base: usize,
}

impl Layout {
    fn of(b: &BootSegs) -> Self {
        let mut segs = [VmPhysseg::EMPTY; MAX_SEGS];
        segs[..b.n].copy_from_slice(&b.segs[..b.n]);
        Self {
            segs,
            n: b.n,
            seg: 0,
            base: 0,
        }
    }

    /// The frame of array slot `i` (slots asked in increasing order): segment by segment from
    /// `start`, as `uvm_page_init` lays them; 0 for the slots past the last segment, which no
    /// segment uses.
    fn pa_of(&mut self, i: usize) -> Paddr {
        while self.seg < self.n {
            let s = &self.segs[self.seg];
            let size = s.end - s.start;
            if i < self.base + size {
                return Paddr::ptoa(s.start + (i - self.base));
            }
            self.base += size;
            self.seg += 1;
        }
        Paddr::new(0)
    }
}

/// `uvm_setpagesize`: sets `uvmexp.pageshift` and `uvmexp.pagemask` from `uvmexp.pagesize`
/// (`DEFAULT_PAGE_SIZE` when unset).
pub fn uvm_setpagesize() {
    if UVMEXP.pagesize.load(Mem::Relaxed) == 0 {
        UVMEXP
            .pagesize
            .store(DEFAULT_PAGE_SIZE as i32, Mem::Relaxed);
    }
    let pagesize = UVMEXP.pagesize.load(Mem::Relaxed);
    UVMEXP.pagemask.store(pagesize - 1, Mem::Relaxed);
    if (pagesize - 1) & pagesize != 0 {
        kpanic(format_args!(
            "uvm_setpagesize: page size not a power of two"
        ));
    }
    let mut shift = 0;
    while (1 << shift) != pagesize {
        shift += 1;
    }
    UVMEXP.pageshift.store(shift, Mem::Relaxed);
}

/// Takes `npg` contiguous page frames out of the boot segment table, at an unused boundary (the
/// start or the end) of the first segment with room; a segment used up on the way is removed.
///
/// Not a function of the C: the `vm_physmem[]` half of amd64's `pmap_steal_memory`, which every
/// `pmap_steal_memory` working through a direct map shares (`hal::pmap::PhysPages::steal`).
/// `None` when no segment has `npg` free frames at a boundary. Panics once stealing is closed:
/// when `uvm_page_init` has published the page array, right before it hands the segments'
/// free frames to the allocator (a frame stolen after that could be both stolen and free; the
/// C only refuses after `page_init_done`, and nothing steals in between), and after it.
pub fn uvm_page_physsteal(npg: usize) -> Option<Paddr> {
    if page_init_done() || TABLE.get().is_some() {
        kpanic(format_args!("uvm_page_physsteal: called _after_ bootstrap"));
    }

    let mut b = BOOT.lock();
    let n = b.n;
    let segno = b.segs[..n].iter().position(|seg| {
        seg.avail_end - seg.avail_start >= npg
            // We can only steal at an ``unused'' segment boundary, i.e. either at the start or
            // at the end.
            && (seg.avail_start == seg.start || seg.avail_end == seg.end)
    })?;
    let seg = &mut b.segs[segno];
    let pa = if seg.avail_start == seg.start {
        let pa = ptoa(seg.avail_start);
        seg.avail_start += npg;
        seg.start += npg;
        pa
    } else {
        let pa = ptoa(seg.avail_end - npg);
        seg.avail_end -= npg;
        seg.end -= npg;
        pa
    };
    // If all the segment has been consumed now, remove it. Note that the crash dump code still
    // knows about it and will dump it correctly.
    if seg.start == seg.end {
        b.remove(segno);
    }
    Some(Paddr::new(pa))
}

/// `uvm_page_physload`: loads physical memory into the VM system. All arguments are page frame
/// numbers; every page in `start..end` gets a page structure at [`uvm_page_init`], and
/// `avail_start..avail_end` goes to the free pool; at most `policy.max` segments, kept by
/// `policy.strat`. A segment that cannot be taken is the error (the caller prints the C's
/// lines).
pub fn uvm_page_physload(
    start: usize,
    end: usize,
    avail_start: usize,
    avail_end: usize,
    flags: i32,
    policy: SegPolicy,
) -> Result<(), PhysloadError> {
    if cfg!(feature = "diagnostic") {
        if UVMEXP.pagesize.load(Mem::Relaxed) == 0 {
            kpanic(format_args!("uvm_page_physload: page size not set!"));
        }
        if start >= end {
            kpanic(format_args!("uvm_page_physload: start >= end"));
        }
    }
    let _ = flags; // PHYSLOAD_DEVICE matters only to the km_alloc path (module deviations).
    let max = policy.max.min(MAX_SEGS);

    let mut b = BOOT.lock();
    let n = b.n;

    // do we have room?
    if n == max {
        return Err(PhysloadError::Full { max });
    }

    // a "preload" (uvm_page_init has not run, so the page structures are made then)?
    if page_init_done() {
        return Err(PhysloadError::AfterInit);
    }

    // now insert us in the proper place in vm_physmem[]
    let lcv = match policy.strat {
        // random: put it at the end (easy!)
        SegStrat::Random => n,
        // sort by address for binary search
        SegStrat::Bsearch => b.segs[..n]
            .iter()
            .position(|seg| start < seg.start)
            .unwrap_or(n),
        // sort by largest segment first
        SegStrat::Bigfirst => b.segs[..n]
            .iter()
            .position(|seg| (end - start) > (seg.end - seg.start))
            .unwrap_or(n),
    };
    // move back other entries, if necessary ...
    b.segs.copy_within(lcv..n, lcv + 1);
    b.segs[lcv] = VmPhysseg {
        start,
        end,
        avail_start,
        avail_end,
        first: 0,
    };
    b.n = n + 1;
    b.policy = policy;
    Ok(())
}

/// The boot segment table as it is now (`vm_physmem[]` before [`uvm_page_init`] freezes it), for the
/// callers that read it while the system boots (`uvm_page_physdump`).
pub fn boot_segments(out: &mut [VmPhysseg; MAX_SEGS]) -> usize {
    let b = BOOT.lock();
    out[..b.n].copy_from_slice(&b.segs[..b.n]);
    b.n
}

/// `uvm_pagealloc_pg`: gives `pg` to `owner` at `off`: basically [`uvm_pagealloc`] with the page
/// already in hand. The caller holds the owner's lock.
pub fn uvm_pagealloc_pg(pg: &VmPage, owner: Owner<'_>, off: Voff) {
    let pg = static_of(pg);
    kassert!(!matches!(owner, Owner::Anon(_)) || off == 0);
    kassert!(off == trunc_page(off as usize) as Voff);
    kassert!(match owner {
        Owner::Object(o) => o.is_dummy() || o.locked(true),
        Owner::Anon(a) => a.locked(true).unwrap_or(true),
        Owner::None => true,
    });

    let mut flags = PG_BUSY | PG_FAKE;
    pg.set_offset(off);
    pg.set_owner(owner.tag());
    kassert!(uvm_page_owner_locked_p(pg, owner, true));
    match owner {
        Owner::Anon(anon) => {
            anon.set_page(Some(pg));
            flags |= PQ_ANON;
        }
        Owner::Object(obj) => uvm_pageinsert(pg, obj),
        Owner::None => {}
    }
    pg.set_bits(flags);
    // UVM_PAGE_OWN(pg, "new alloc"): UVM_PAGE_TRKOWN is not configured.
}

/// `uvm_pglistalloc`: allocates the pages of `size` bytes (rounded to pages) within `low` to
/// `high`, aligned to `alignment`, no segment crossing `boundary` (powers of two; 0: none), in
/// at most `nsegs` segments, onto the tail of `rlist`. `flags`: `UVM_PLA_NOWAIT` or
/// `UVM_PLA_WAITOK` (which fails with `ENOMEM` here, module deviations), `UVM_PLA_ZERO`.
#[allow(clippy::too_many_arguments, reason = "the C signature")]
pub fn uvm_pglistalloc(
    size: usize,
    low: Paddr,
    high: Paddr,
    alignment: Paddr,
    boundary: Paddr,
    rlist: &Pglist,
    nsegs: i32,
    flags: i32,
) -> Result<(), Errno> {
    let mut alignment = alignment.as_usize();
    let mut boundary = boundary.as_usize();
    kassert!(alignment & alignment.wrapping_sub(1) == 0);
    kassert!(boundary & boundary.wrapping_sub(1) == 0);
    kassert!((flags & UVM_PLA_WAITOK == 0) ^ (flags & UVM_PLA_NOWAIT == 0));

    if size == 0 {
        return Err(Errno::EINVAL);
    }
    let size = atop(round_page(size));

    // XXX uvm_pglistalloc is currently only used for kernel objects. Unlike the checks in
    // uvm_pagealloc, below, here we are always allowed to use the kernel reserve.
    let flags = flags | UVM_PLA_USERESERVE;

    if high.as_usize() & PAGE_MASK != PAGE_MASK {
        printf(format_args!(
            "uvm_pglistalloc: Upper boundary {:#x} not on pagemask.\n",
            high.as_usize()
        ));
    }

    // Our allocations are always page granularity, so our alignment must be, too.
    if alignment < PAGE_SIZE {
        alignment = PAGE_SIZE;
    }

    let low = atop(low.as_usize().div_ceil(alignment) * alignment);
    // high + 1 may result in overflow, in which case high becomes 0x0, which is the 'don't
    // care' value. The only requirement in that case is that low is also 0x0, or the low<high
    // assert will fail.
    let high = atop(high.as_usize().wrapping_add(1));
    let alignment = atop(alignment);
    if boundary < PAGE_SIZE && boundary != 0 {
        boundary = PAGE_SIZE;
    }
    let boundary = atop(boundary);

    pmemrange::uvm_pmr_getpages(size, low, high, alignment, boundary, nsegs, flags, rlist)
}

/// `uvm_pglistfree`: frees a list of pages (already unmapped), leaving it empty.
pub fn uvm_pglistfree(list: &Pglist) {
    pmemrange::uvm_pmr_freepageq(list);
}

/// `uvm_pagealloc`: allocates a page for `owner` at `off` (an anon's at 0); `None` when no page
/// is free. The caller holds the owner's lock, and activates or deactivates the page unless it
/// wires it.
pub fn uvm_pagealloc(owner: Owner<'_>, off: Voff, flags: i32) -> Option<&'static VmPage> {
    kassert!(!matches!(owner, Owner::Anon(_)) || off == 0);
    kassert!(off == trunc_page(off as usize) as Voff);
    kassert!(match owner {
        Owner::Object(o) => o.is_dummy() || o.locked(true),
        Owner::Anon(a) => a.locked(true).unwrap_or(true),
        Owner::None => true,
    });

    let mut pmr_flags = UVM_PLA_NOWAIT;

    // We're allowed to use the kernel reserve if the page is being allocated to a kernel
    // object.
    if flags & UVM_PGA_USERESERVE != 0 || matches!(owner, Owner::Object(o) if o.is_kern_object()) {
        pmr_flags |= UVM_PLA_USERESERVE;
    }

    if flags & UVM_PGA_ZERO != 0 {
        pmr_flags |= UVM_PLA_ZERO;
    }

    let pg = pmemrange::uvm_pmr_cache_get(pmr_flags)?;
    uvm_pagealloc_pg(pg, owner, off);
    kassert!(pg.flags() & PG_DEV == 0);
    if flags & UVM_PGA_ZERO != 0 {
        pg.clear_bits(PG_CLEAN);
    } else {
        pg.set_bits(PG_CLEAN);
    }

    Some(pg)
}

/// `uvm_pagerealloc`: moves `pg` from its object (`old`, when it has one) to the object `new`
/// at `newoff` (`Owner::None`: to no object). An anon is not an object: it panics.
pub fn uvm_pagerealloc(pg: &VmPage, old: Owner<'_>, new: Owner<'_>, newoff: Voff) {
    let pg = static_of(pg);
    // remove it from the old object
    if pg.has_object() {
        check_owner(pg, old, "uvm_pagerealloc");
        if let Owner::Object(o) = old {
            uvm_pageremove(pg, o);
        }
    }

    // put it in the new object
    let new = match new {
        Owner::Object(o) => Some(o),
        Owner::None => None,
        Owner::Anon(_) => kpanic(format_args!("uvm_pagerealloc: to an anon")),
    };
    if let Some(newobj) = new {
        pg.set_owner(newobj.tag());
        pg.set_offset(newoff);
        pg.bump_version();
        uvm_pageinsert(pg, newobj);
    }
}

/// `uvm_pageclean`: erases the page's identity (takes it out of its object or anon, off the
/// page queues, unwired); all its valid mappings are gone. `owner` is the page's.
pub fn uvm_pageclean(pg: &VmPage, owner: Owner<'_>) {
    check_owner(pg, owner, "uvm_pageclean");
    // hal's byte view of the page (the pager's window) must be gone before the page is.
    if pg.mdpage.in_view() {
        kpanic(format_args!("uvm_pageclean: a page in a byte view"));
    }
    kassert!(pg.flags() & PG_DEV == 0);
    kassert!(match owner {
        Owner::Object(o) => o.is_dummy() || o.locked(true),
        Owner::Anon(a) => a.locked(true).unwrap_or(false),
        Owner::None => true,
    });

    // if the page was an object page (and thus "TABLED"), remove it from the object.
    if pg.flags() & PG_TABLED != 0
        && let Owner::Object(o) = owner
    {
        uvm_pageremove(pg, o);
    }

    // now remove the page from the queues
    if pg.flags() & (PQ_ACTIVE | PQ_INACTIVE) != 0 {
        // An object page has no owner left (`pageremove`); an anon's keeps it until below.
        let qowner = if pg.has_owner() { owner } else { Owner::None };
        let mut q = PAGEQ.lock();
        uvm_pagedequeue(&mut q, pg, qowner);
    }

    // if the page was wired, unwire it now.
    if pg.wire_count() != 0 {
        pg.set_wire_count(0);
        UVMEXP.wired.fetch_sub(1, Mem::Relaxed);
    }
    if let Owner::Anon(anon) = owner {
        anon.set_page(None);
        pg.set_owner(OwnerTag::NONE);
    }

    // Clean page state bits.
    pg.clear_bits(
        PQ_ANON
            | PQ_AOBJ
            | PQ_ENCRYPT
            | PG_ZERO
            | PG_FAKE
            | PG_BUSY
            | PG_RELEASED
            | PG_CLEAN
            | PG_CLEANCHK,
    );

    // DEBUG: the 0xdeadbeef poisoning of uobject/offset/uanon is not compiled in.
}

/// `uvm_pagefree`: frees `pg` (erases its identity, then gives it to this CPU's page cache or
/// the free lists); all its valid mappings are gone. `owner` is the page's.
pub fn uvm_pagefree(pg: &VmPage, owner: Owner<'_>) {
    uvm_pageclean(pg, owner);
    pmemrange::uvm_pmr_cache_put(static_of(pg));
}

/// `uvm_page_unbusy`: unbusies the pages of `pgs` (`None`s skipped), all of `owner`, whose lock
/// the caller holds; an anon's page must have a reference left and not be `PG_RELEASED`.
/// A waiter is woken; a released page is freed.
pub fn uvm_page_unbusy(pgs: &[Option<&VmPage>], owner: Owner<'_>) {
    for pg in pgs.iter().flatten() {
        kassert!(uvm_page_owner_locked_p(pg, owner, true));
        kassert!(pg.flags() & PG_BUSY != 0);

        if pg.flags() & PG_WANTED != 0 {
            wakeup(pg.chan());
        }
        if pg.flags() & PG_RELEASED != 0 {
            kassert!(match owner {
                Owner::Object(_) => true,
                Owner::Anon(a) => a.refs() > 0,
                Owner::None => false,
            });
            pg.clear_bits(PG_WANTED);
            hal::pmap::page_protect(&pg.mdpage, PROT_NONE);
            uvm_pagefree(pg, owner);
        } else {
            kassert!(pg.flags() & PG_FAKE == 0);
            pg.clear_bits(PG_WANTED | PG_BUSY);
            // UVM_PAGE_OWN(pg, NULL): not configured.
        }
    }
}

/// `vm_physseg_find`: the segment (its index in [`vm_physmem`]) holding page frame `pframe`
/// and the frame's offset in it.
pub fn vm_physseg_find(pframe: usize) -> Option<(usize, usize)> {
    let t = table()?;
    let (seg, off) = t.find(pframe)?;
    let i = t.segs[..t.n].iter().position(|s| ptr::eq(s, seg))?;
    Some((i, off))
}

/// `PHYS_TO_VM_PAGE(pa)`: the page of a physical address; `None` outside the managed pages or
/// before [`uvm_page_init`].
#[allow(non_snake_case, reason = "the C name, used in MD code too")]
pub fn PHYS_TO_VM_PAGE(pa: Paddr) -> Option<&'static VmPage> {
    let t = table()?;
    let (seg, off) = t.find(pa.atop())?;
    t.pages.get(seg.first + off)
}

/// `uvm_pagelookup`: the page of `obj` at `off`. The caller holds the object's lock.
pub fn uvm_pagelookup(obj: &dyn PageObject, off: Voff) -> Option<&'static VmPage> {
    let pg = obj.pages().find_by(|p| off.cmp(&p.offset()));

    kassert!(pg.is_none() || obj.npages() != 0);
    kassert!(pg.is_none_or(|pg| pg.flags() & PG_RELEASED == 0 || pg.flags() & PG_BUSY != 0));
    pg
}

/// `uvm_pagewire`: wires the page, taking it out of the page daemon's reach.
pub fn uvm_pagewire(pg: &VmPage, owner: Owner<'_>) {
    kassert!(uvm_page_owner_locked_p(pg, owner, true));

    if pg.wire_count() == 0 {
        let mut q = PAGEQ.lock();
        uvm_pagedequeue(&mut q, pg, owner);
        drop(q);
        UVMEXP.wired.fetch_add(1, Mem::Relaxed);
    }
    kassert!(pg.flags() & (PQ_INACTIVE | PQ_ACTIVE) == 0);
    pg.set_wire_count(pg.wire_count() + 1);
    kassert!(pg.wire_count() > 0); // detect wraparound
}

/// `uvm_pageunwire`: unwires the page; activates it when the wire count goes to zero.
pub fn uvm_pageunwire(pg: &VmPage, owner: Owner<'_>) {
    kassert!(uvm_page_owner_locked_p(pg, owner, true));
    kassert!(pg.wire_count() != 0);

    pg.set_wire_count(pg.wire_count() - 1);
    if pg.wire_count() == 0 {
        uvm_pageactivate(pg, owner);
        UVMEXP.wired.fetch_sub(1, Mem::Relaxed);
    }
}

/// `uvm_pagedeactivate`: deactivates the page (unless wired). The caller holds the owner's
/// lock.
pub fn uvm_pagedeactivate(pg: &VmPage, owner: Owner<'_>) {
    kassert!(uvm_page_owner_locked_p(pg, owner, false));

    if pg.wire_count() > 0 {
        kassert!(pg.flags() & (PQ_INACTIVE | PQ_ACTIVE) == 0);
        return;
    }

    let mut q = PAGEQ.lock();
    if pg.flags() & PQ_INACTIVE != 0 {
        return;
    }

    // Make sure next access to this page will fault.
    hal::pmap::page_protect(&pg.mdpage, PROT_NONE);

    uvm_pagedequeue(&mut q, pg, owner);
    q.inactive.insert_tail(static_of(pg));
    pg.set_bits(PQ_INACTIVE);
    UVMEXP.inactive.fetch_add(1, Mem::Relaxed);
    drop(q);

    let _ = hal::pmap::clear_reference(&pg.mdpage);
    // update the "clean" bit. this isn't 100% accurate, and doesn't have to be. we'll re-sync
    // it after we zap all mappings when scanning the inactive list.
    if pg.flags() & PG_CLEAN != 0 && hal::pmap::is_modified(&pg.mdpage) {
        pg.clear_bits(PG_CLEAN);
    }
}

/// `uvm_pageactivate`: activates the page (unless wired).
pub fn uvm_pageactivate(pg: &VmPage, owner: Owner<'_>) {
    kassert!(uvm_page_owner_locked_p(pg, owner, false));

    if pg.wire_count() > 0 {
        kassert!(pg.flags() & (PQ_INACTIVE | PQ_ACTIVE) == 0);
        return;
    }

    let mut q = PAGEQ.lock();
    uvm_pagedequeue(&mut q, pg, owner);
    q.active.insert_tail(static_of(pg));
    pg.set_bits(PQ_ACTIVE);
    UVMEXP.active.fetch_add(1, Mem::Relaxed);
}

/// `uvm_pagedequeue`: takes the page off any paging queue; `q` is the proof the caller holds
/// the page queue lock (the C's `MUTEX_ASSERT_LOCKED(&uvm.pageqlock)`).
pub fn uvm_pagedequeue(q: &mut PageQueues, pg: &VmPage, owner: Owner<'_>) {
    kassert!(uvm_page_owner_locked_p(pg, owner, false));
    kassert!(pg.wire_count() == 0);

    if pg.flags() & PQ_ACTIVE != 0 {
        q.active.remove(pg);
        pg.clear_bits(PQ_ACTIVE);
        UVMEXP.active.fetch_sub(1, Mem::Relaxed);
    }
    if pg.flags() & PQ_INACTIVE != 0 {
        q.inactive.remove(pg);
        pg.clear_bits(PQ_INACTIVE);
        UVMEXP.inactive.fetch_sub(1, Mem::Relaxed);
    }
}

/// The page half of `uvm_obj_free` (the buffer cache's, `bsd`'s `uvm_object.rs`): every page
/// of `obj`, which is being thrown away, leaves its tree in offset order (the physical
/// addresses usually increase in that order, which is better for `uvm_pglistfree`), loses
/// `PG_TABLED` and its owner without the object's bookkeeping (the C's shortcut: the object is
/// initialised again before it is reused), is cleaned and goes onto one list that is freed.
/// The C walks the tree without taking the pages out of it; here each leaves it, so its
/// entry is free for the allocator's trees.
pub fn uvm_obj_free_pages(obj: &dyn PageObject) {
    let pgl = Pglist::new();
    while let Some(pg) = obj.pages().min() {
        check_owner(pg, Owner::Object(obj), "uvm_obj_free");
        obj.pages().remove(pg);
        // clear PG_TABLED and `uobject' so we don't do work to remove this pg from the uobj
        // we are throwing away.
        pg.clear_bits(PG_TABLED);
        pg.set_owner(OwnerTag::NONE);
        uvm_pageclean(pg, Owner::None);
        pgl.insert_tail(pg);
    }
    uvm_pglistfree(&pgl);
}

/// `uvm_pagezero`: zero-fills the page.
pub fn uvm_pagezero(pg: &VmPage) {
    pg.clear_bits(PG_CLEAN);
    hal::pmap::zero_page(pg.phys_addr);
}

/// `uvm_pagecopy`: copies `src` into `dst`.
pub fn uvm_pagecopy(src: &VmPage, dst: &VmPage) {
    dst.clear_bits(PG_CLEAN);
    hal::pmap::copy_page(src.phys_addr, dst.phys_addr);
}

/// `uvm_page_owner_locked_p`: whether `owner`, which must be the page's, is locked (for
/// assertions only). An anon without a lock panics, as in the C.
pub fn uvm_page_owner_locked_p(pg: &VmPage, owner: Owner<'_>, exclusive: bool) -> bool {
    if pg.owner_tag() != owner.tag() {
        return false;
    }
    match owner {
        Owner::Object(obj) => obj.is_dummy() || obj.locked(exclusive),
        Owner::Anon(anon) => match anon.locked(exclusive) {
            Some(held) => held,
            None => kpanic(format_args!("uvm_page_owner_locked_p: anon without a lock")),
        },
        Owner::None => true,
    }
}

/// `uvm_pagecount`: the physical pages available in `constraint`'s range.
pub fn uvm_pagecount(constraint: &UvmConstraintRange) -> usize {
    // Algorithm uses page numbers.
    let low = constraint.ucr_low.atop();
    let high = constraint.ucr_high.atop();

    vm_physmem()
        .iter()
        .map(|seg| {
            let ps_low = low.max(seg.avail_start);
            let ps_high = high.min(seg.avail_end);
            ps_high.saturating_sub(ps_low)
        })
        .sum()
}

// The page provider's contract (`hal::pmap::PhysPages`, a documented trust boundary): a frame
// `alloc_ptp` or `steal` hands out is RAM of the loaded segments (inside the direct map) that
// nothing else uses until `free_ptp` takes it back (stolen frames never come back); `md_of`
// answers the `PageMd` embedded in the frame's page, the same for ever (the array is never
// freed).
impl PhysPages for PhysPagesImpl {
    fn alloc_ptp(&self) -> Option<Paddr> {
        let pg = uvm_pagealloc(Owner::None, 0, UVM_PGA_USERESERVE | UVM_PGA_ZERO)?;
        pg.clear_bits(PG_BUSY | PG_FAKE);
        pg.set_wire_count(1);
        Some(pg.phys_addr)
    }

    fn free_ptp(&self, pa: Paddr) {
        match PHYS_TO_VM_PAGE(pa) {
            Some(pg) => {
                pg.set_wire_count(0);
                uvm_pagefree(pg, Owner::None);
            }
            None => kpanic(format_args!("pmap: PTP {pa:?} is not a managed page")),
        }
    }

    fn md_of(&self, pa: Paddr) -> Option<&'static PageMd> {
        PHYS_TO_VM_PAGE(pa).map(|pg| &pg.mdpage)
    }

    fn is_busy(&self, pa: Paddr) -> bool {
        PHYS_TO_VM_PAGE(pa).is_some_and(|pg| pg.flags() & PG_BUSY != 0)
    }

    fn steal(&self, npg: usize) -> Option<Paddr> {
        uvm_page_physsteal(npg)
    }

    fn page_init_done(&self) -> bool {
        page_init_done()
    }
}

const _: () = {
    // The page and the tables are shared between CPUs by reference.
    const fn shared<T: Sync>() {}
    shared::<VmPage>();
    shared::<PageTable>();
    // A frame number fits the array index type wherever the array is built (`init` checks it).
    assert!(MAX_SEGS >= 16);
};

/// Forgets the page system (the segments, the table, the queues, the free state and the page
/// counters), so a host test can load memory of its own; the old array is leaked, as the
/// memory a test loaded is. Host tests only (the `doubles` feature exposes it to `bsd`'s).
#[cfg(any(test, feature = "doubles"))]
pub fn uvm_page_test_reset() {
    TABLE.set(None);
    *BOOT.lock() = BootSegs::new();
    *PAGEQ.lock() = PageQueues::new();
    pmemrange::uvm_pmr_cache_test_reset();
    PAGE_INIT_DONE.store(false, Mem::Release);
    for c in [
        &UVMEXP.npages,
        &UVMEXP.free,
        &UVMEXP.zeropages,
        &UVMEXP.active,
        &UVMEXP.inactive,
        &UVMEXP.wired,
        &UVMEXP.percpucaches,
    ] {
        c.store(0, Mem::Relaxed);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Over hal's host pmap (bootstrapped once per process with this module's provider,
    // `testutil::pmap_boot`) and synthetic segments: the host's `steal_memory` takes its frames
    // from the segment table and hands out heap memory, and `pmap_zero_page` models no
    // contents, so the frames need not be memory. The page system is one set of statics: every
    // test holds `testutil::MEMORY` through its set-up.

    use std::sync::MutexGuard;
    use std::sync::atomic::AtomicI32;

    use super::*;
    use crate::testutil;

    /// A segment in the ISA range: pages 0x100..0x200 (1 MiB..2 MiB).
    const SEG_ISA: (usize, usize) = (0x100, 0x200);
    /// A segment above 4 GiB: pages 0x10_0000..0x10_1000 (16 MiB).
    const SEG_HIGH: (usize, usize) = (0x10_0000, 0x10_1000);

    fn setup() -> MutexGuard<'static, ()> {
        testutil::pages_setup(&[SEG_HIGH, SEG_ISA], &testutil::HOST_CONSTRAINTS)
    }

    /// The free pages, those in this CPU's page cache (`MULTIPROCESSOR`) included: what an
    /// allocation takes and a free gives back, wherever it goes.
    fn free() -> i32 {
        UVMEXP.free.load(Mem::Relaxed) + UVMEXP.percpucaches.load(Mem::Relaxed)
    }

    /// An object double: its tree, a page count, always locked.
    struct Obj {
        memt: Tree<UvmObjtree>,
        n: AtomicI32,
    }

    impl Obj {
        const fn new() -> Self {
            Self {
                memt: Tree::new(),
                n: AtomicI32::new(0),
            }
        }
    }

    impl PageObject for Obj {
        fn tag(&self) -> OwnerTag {
            OwnerTag::object(self)
        }
        fn is_dummy(&self) -> bool {
            false
        }
        fn is_kern_object(&self) -> bool {
            false
        }
        fn locked(&self, _exclusive: bool) -> bool {
            true
        }
        fn pages(&self) -> &Tree<UvmObjtree> {
            &self.memt
        }
        fn npages(&self) -> i32 {
            self.n.load(Mem::Relaxed)
        }
        fn set_npages(&self, n: i32) {
            self.n.store(n, Mem::Relaxed);
        }
    }

    /// An anon double: its page link and a reference.
    struct Anon {
        page: Link<VmPage>,
    }

    impl PageAnon for Anon {
        fn tag(&self) -> OwnerTag {
            OwnerTag::anon(self)
        }
        fn locked(&self, _exclusive: bool) -> Option<bool> {
            Some(true)
        }
        fn refs(&self) -> i32 {
            1
        }
        fn set_page(&self, pg: Option<&'static VmPage>) {
            self.page.set(pg);
        }
    }

    #[test]
    fn physsteal_takes_from_either_end_and_removes_a_used_up_segment() {
        let _g = testutil::pages_reset();
        let p = testutil::HOST_SEGS;
        // Bigfirst: the larger segment first.
        assert_eq!(uvm_page_physload(0x100, 0x104, 0x100, 0x104, 0, p), Ok(()));
        assert_eq!(uvm_page_physload(0x200, 0x210, 0x202, 0x210, 0, p), Ok(()));
        let mut segs = [VmPhysseg::EMPTY; MAX_SEGS];
        assert_eq!(boot_segments(&mut segs), 2);
        assert_eq!((segs[0].start, segs[1].start), (0x200, 0x100));

        // The first segment with room at a boundary: 0x200's avail_end == end.
        assert_eq!(uvm_page_physsteal(2), Some(Paddr::new(ptoa(0x20e))));
        assert_eq!(boot_segments(&mut segs), 2);
        assert_eq!((segs[0].end, segs[0].avail_end), (0x20e, 0x20e));
        // Too big for any segment.
        assert_eq!(uvm_page_physsteal(64), None);
        // The rest of 0x200's free frames, from its end again (its start is not free).
        assert_eq!(uvm_page_physsteal(12), Some(Paddr::new(ptoa(0x202))));
        assert_eq!(boot_segments(&mut segs), 2);
        assert_eq!((segs[0].start, segs[0].end), (0x200, 0x202));
        // All of 0x100's, from its start: the segment goes.
        assert_eq!(uvm_page_physsteal(4), Some(Paddr::new(ptoa(0x100))));
        assert_eq!(boot_segments(&mut segs), 1);
        assert_eq!((segs[0].start, segs[0].end), (0x200, 0x202));
    }

    #[test]
    #[should_panic(expected = "uvm_page_physsteal: called _after_ bootstrap")]
    fn physsteal_after_init_panics() {
        let _g = testutil::pages_setup(&[(0x100, 0x200)], &[]);
        let _ = uvm_page_physsteal(1);
    }

    #[test]
    #[should_panic(expected = "pmap_steal_memory: after uvm_page_init")]
    fn boot_memory_after_init_panics() {
        // `hal::bootmem` over the kernel's provider: once the page system is up, the pmap
        // refuses to steal before any frame is taken.
        let _g = testutil::pages_setup(&[(0x100, 0x200)], &[]);
        let _ = hal::bootmem::steal_one(|| 0u64);
    }

    #[test]
    #[should_panic(expected = "uvm_page_physsteal: called _after_ bootstrap")]
    fn stealing_closes_when_the_page_array_is_published() {
        // The window between the page array's publication and `page_init_done` (where the
        // segments' free frames go to the allocator): no frame may be stolen in it.
        let _g = testutil::pages_setup(&[(0x100, 0x200)], &[]);
        PAGE_INIT_DONE.store(false, Mem::Release);
        assert!(TABLE.get().is_some(), "the table is published");
        let _ = uvm_page_physsteal(1);
    }

    #[test]
    fn bsearch_sorts_the_segments_and_bisection_finds_every_frame() {
        // arm64's VM_PSTRAT_BSEARCH: physload keeps the table sorted by address whatever the
        // load order, and vm_physseg_find bisects it. For tables of 1 to 6 segments (odd and
        // even lengths), every frame from below the first to past the last is found exactly
        // where a linear search finds it, gaps included.
        let p = SegPolicy {
            max: 16,
            strat: SegStrat::Bsearch,
        };
        let order = [3usize, 1, 5, 0, 4, 2];
        for nseg in 1..=order.len() {
            let _g = testutil::pages_reset();
            for &k in order.iter().filter(|&&k| k < nseg) {
                let start = 0x100 + k * 0x100;
                assert_eq!(
                    uvm_page_physload(start, start + 0x40, start, start + 0x40, 0, p),
                    Ok(())
                );
            }
            let mut segs = [VmPhysseg::EMPTY; MAX_SEGS];
            let n = boot_segments(&mut segs);
            assert_eq!(n, nseg);
            assert!(
                segs[..n].windows(2).all(|w| w[0].start < w[1].start),
                "{nseg} segments sorted by address"
            );

            uvm_page_init(&[]);
            let table = vm_physmem();
            assert_eq!(table.len(), nseg);
            for pframe in 0xf0..0x100 + nseg * 0x100 + 0x10 {
                let linear = table
                    .iter()
                    .position(|s| pframe >= s.start && pframe < s.end)
                    .map(|i| (i, pframe - table[i].start));
                assert_eq!(
                    vm_physseg_find(pframe),
                    linear,
                    "frame {pframe:#x} of {nseg}"
                );
                if let Some((i, off)) = linear {
                    let pg = PHYS_TO_VM_PAGE(Paddr::ptoa(pframe)).expect("a page");
                    assert_eq!(pg.phys_addr, Paddr::ptoa(pframe));
                    assert_eq!(pg.index(), table[i].first + off);
                }
            }
        }
    }

    #[test]
    fn a_full_table_and_a_late_segment_are_refused() {
        let _g = testutil::pages_reset();
        let p = SegPolicy {
            max: 2,
            strat: SegStrat::Random,
        };
        assert_eq!(uvm_page_physload(0x100, 0x200, 0x100, 0x200, 0, p), Ok(()));
        assert_eq!(uvm_page_physload(0x300, 0x400, 0x300, 0x400, 0, p), Ok(()));
        assert_eq!(
            uvm_page_physload(0x500, 0x600, 0x500, 0x600, 0, p),
            Err(PhysloadError::Full { max: 2 })
        );
        drop(_g);
        let _g = setup();
        assert_eq!(
            uvm_page_physload(0x500, 0x600, 0x500, 0x600, 0, testutil::HOST_SEGS),
            Err(PhysloadError::AfterInit)
        );
    }

    #[test]
    fn every_frame_of_both_segments_has_its_page_and_index() {
        let _g = setup();
        assert!(page_init_done());
        let segs = vm_physmem();
        assert_eq!(segs.len(), 2);
        for seg in segs {
            for pf in [seg.start, (seg.start + seg.end) / 2, seg.end - 1] {
                let pg = PHYS_TO_VM_PAGE(Paddr::new(ptoa(pf))).expect("a managed frame");
                assert_eq!(pg.phys_addr().atop(), pf);
                assert!(ptr::eq(&pages()[pg.index()], pg));
                assert!(ptr::eq(static_of(pg), pg));
            }
            // pg + 1 is the next frame inside a segment.
            let first = PHYS_TO_VM_PAGE(Paddr::new(ptoa(seg.start))).expect("first");
            assert_eq!(nth(first, 1).phys_addr().atop(), seg.start + 1);
            assert!(ptr::eq(nth_back(nth(first, 1), 1), first));
            assert!(
                PHYS_TO_VM_PAGE(Paddr::new(ptoa(seg.end)))
                    .is_none_or(|p| { p.phys_addr().atop() == seg.end })
            );
        }
        assert!(PHYS_TO_VM_PAGE(Paddr::new(ptoa(0x50))).is_none());
        assert!(vm_physseg_find(SEG_ISA.0 + 0x80).is_some_and(|(_, off)| off > 0));
        assert_eq!(
            UVMEXP.npages.load(Mem::Relaxed) as usize,
            segs.iter()
                .map(|s| s.avail_end - s.avail_start)
                .sum::<usize>()
        );
    }

    #[test]
    fn alloc_and_free_put_the_free_count_back() {
        let _g = setup();
        let before = free();
        let pg = uvm_pagealloc(Owner::None, 0, 0).expect("a page");
        assert_ne!(pg.flags() & PG_BUSY, 0);
        assert_eq!(pg.flags() & PQ_FREE, 0);
        assert!(!pg.has_owner());
        assert_eq!(free(), before - 1);
        uvm_pagefree(pg, Owner::None);
        assert_eq!(free(), before);
        // A page of the unused high range goes to one of this CPU's magazines, on no free
        // list.
        assert!(
            !pg.is_free() && pmemrange::uvm_pmr_cache_holds(pg),
            "the page is in this CPU's magazine"
        );
        assert!(!pg.has_owner() && pg.flags() & PG_BUSY == 0);
    }

    #[test]
    fn an_object_finds_its_pages_by_offset_and_loses_them_when_freed() {
        let _g = setup();
        let obj = Obj::new();
        let o = Owner::Object(&obj);
        let a = uvm_pagealloc(o, 0x3000, 0).expect("a page");
        let b = uvm_pagealloc(o, 0x1000, 0).expect("a page");
        assert!(a.owned_by_object(&obj) && a.has_object() && !a.has_anon());
        assert_eq!(obj.npages(), 2);
        assert_ne!(a.flags() & PG_TABLED, 0);
        assert!(uvm_pagelookup(&obj, 0x3000).is_some_and(|p| ptr::eq(p, a)));
        assert!(uvm_pagelookup(&obj, 0x1000).is_some_and(|p| ptr::eq(p, b)));
        assert!(uvm_pagelookup(&obj, 0x2000).is_none());

        // Moved to another object at another offset.
        let other = Obj::new();
        uvm_pagerealloc(b, o, Owner::Object(&other), 0x8000);
        assert!(uvm_pagelookup(&obj, 0x1000).is_none());
        assert!(uvm_pagelookup(&other, 0x8000).is_some_and(|p| ptr::eq(p, b)));

        uvm_pagefree(a, o);
        uvm_pagefree(b, Owner::Object(&other));
        assert_eq!((obj.npages(), other.npages()), (0, 0));
        assert!(obj.memt.is_empty() && other.memt.is_empty());
    }

    #[test]
    fn an_anon_gets_its_page_and_gives_it_back() {
        let _g = setup();
        let anon = Anon { page: Link::new() };
        let pg = uvm_pagealloc(Owner::Anon(&anon), 0, UVM_PGA_ZERO).expect("a page");
        assert!(anon.page.points_to(pg));
        assert!(pg.owned_by_anon(&anon) && pg.has_anon());
        assert_ne!(pg.flags() & PQ_ANON, 0);
        assert_eq!(pg.flags() & PG_CLEAN, 0, "zeroed pages are dirty");
        uvm_pagefree(pg, Owner::Anon(&anon));
        assert!(anon.page.is_none());
        assert!(!pg.has_owner() && pg.flags() & PQ_ANON == 0);
    }

    #[test]
    #[should_panic(expected = "is not owned by the owner given")]
    fn freeing_with_another_owner_panics() {
        let _g = setup();
        let obj = Obj::new();
        let pg = uvm_pagealloc(Owner::Object(&obj), 0, 0).expect("a page");
        uvm_pagefree(pg, Owner::None);
    }

    #[test]
    fn wiring_and_the_queues_keep_their_counts() {
        let _g = setup();
        let obj = Obj::new();
        let o = Owner::Object(&obj);
        let pg = uvm_pagealloc(o, 0, 0).expect("a page");
        let (act, inact, wired) = (
            UVMEXP.active.load(Mem::Relaxed),
            UVMEXP.inactive.load(Mem::Relaxed),
            UVMEXP.wired.load(Mem::Relaxed),
        );
        uvm_pageactivate(pg, o);
        assert_ne!(pg.flags() & PQ_ACTIVE, 0);
        assert_eq!(UVMEXP.active.load(Mem::Relaxed), act + 1);
        assert!(
            uvm_lock_pageq()
                .first_active()
                .is_some_and(|p| ptr::eq(p, pg))
        );

        uvm_pagedeactivate(pg, o);
        assert_eq!(pg.flags() & (PQ_ACTIVE | PQ_INACTIVE), PQ_INACTIVE);
        assert_eq!(UVMEXP.active.load(Mem::Relaxed), act);
        assert_eq!(UVMEXP.inactive.load(Mem::Relaxed), inact + 1);

        uvm_pagewire(pg, o);
        assert_eq!(pg.wire_count(), 1);
        assert_eq!(
            pg.flags() & (PQ_ACTIVE | PQ_INACTIVE),
            0,
            "a wired page is on no queue"
        );
        assert_eq!(UVMEXP.wired.load(Mem::Relaxed), wired + 1);
        assert_eq!(UVMEXP.inactive.load(Mem::Relaxed), inact);
        uvm_pagewire(pg, o);
        uvm_pageunwire(pg, o);
        assert_eq!(UVMEXP.wired.load(Mem::Relaxed), wired + 1);
        uvm_pageunwire(pg, o);
        assert_eq!(UVMEXP.wired.load(Mem::Relaxed), wired);
        assert_ne!(pg.flags() & PQ_ACTIVE, 0, "unwired to zero is activated");

        uvm_pagefree(pg, o);
        assert_eq!(UVMEXP.active.load(Mem::Relaxed), act);
        assert!(uvm_lock_pageq().first_active().is_none());
    }

    #[test]
    fn unbusy_wakes_and_frees_a_released_page() {
        let _g = setup();
        let obj = Obj::new();
        let o = Owner::Object(&obj);
        let a = uvm_pagealloc(o, 0, 0).expect("a page");
        let b = uvm_pagealloc(o, 0x1000, 0).expect("a page");
        a.clear_bits(PG_FAKE);
        b.set_bits(PG_RELEASED | PG_WANTED);
        let before = free();
        uvm_page_unbusy(&[Some(a), None, Some(b)], o);
        assert_eq!(a.flags() & (PG_BUSY | PG_WANTED), 0);
        assert!(
            !b.has_owner() && b.flags() & (PG_RELEASED | PG_BUSY) == 0,
            "the released page was freed"
        );
        assert_eq!(free(), before + 1);
        assert_eq!(obj.npages(), 1);
        uvm_pagefree(a, o);
    }

    #[test]
    fn a_page_table_page_is_wired_and_known_to_the_pmap() {
        let _g = setup();
        let pa = PHYS_PAGES.alloc_ptp().expect("a PTP");
        let pg = PHYS_TO_VM_PAGE(pa).expect("managed");
        assert_eq!(pg.wire_count(), 1);
        assert_eq!(pg.flags() & (PG_BUSY | PG_FAKE | PG_CLEAN), 0);
        let md = PHYS_PAGES.md_of(pa).expect("its PageMd");
        assert!(ptr::eq(md, &pg.mdpage));
        assert!(PHYS_PAGES.page_init_done());
        let before = free();
        PHYS_PAGES.free_ptp(pa);
        assert_eq!(free(), before + 1);
        assert_eq!(pg.wire_count(), 0);
        assert!(PHYS_PAGES.md_of(Paddr::new(ptoa(0x50))).is_none());
    }

    #[test]
    fn a_reset_twice_forgets_everything() {
        let _g = setup();
        drop(_g);
        let g = testutil::pages_reset();
        uvm_page_test_reset();
        assert!(!page_init_done());
        assert!(pages().is_empty() && vm_physmem().is_empty());
        assert!(PHYS_TO_VM_PAGE(Paddr::new(ptoa(SEG_HIGH.0))).is_none());
        assert_eq!(UVMEXP.free.load(Mem::Relaxed), 0);
        drop(g);
        let _g = setup();
        assert!(page_init_done());
    }

    #[test]
    fn the_page_size_gives_the_shift_and_mask() {
        let _g = testutil::pages_reset();
        UVMEXP.pagesize.store(0, Mem::Relaxed);
        uvm_setpagesize();
        assert_eq!(UVMEXP.pagesize.load(Mem::Relaxed), 4096);
        assert_eq!(UVMEXP.pagemask.load(Mem::Relaxed), 4095);
        assert_eq!(UVMEXP.pageshift.load(Mem::Relaxed), 12);
    }

    #[test]
    fn the_count_of_a_constrained_range() {
        let _g = setup();
        let isa = testutil::HOST_CONSTRAINTS[0];
        let n = uvm_pagecount(isa);
        let seg = vm_physmem()
            .iter()
            .find(|s| s.start < 0x1000)
            .expect("the ISA segment");
        assert_eq!(n, seg.avail_end - seg.avail_start);
    }
}
/* </TESTS> */
