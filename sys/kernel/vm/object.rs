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
 * from: Id: uvm_object.h,v 1.1.2.2 1998/01/04 22:44:51 chuck Exp
 */
/* </LICENSES> */

/* <CODE> */
//! The memory object: `struct uvm_object` of `<uvm/uvm_object.h>` and the object operations of
//! `uvm/uvm_object.c`.
//!
//! Upstream: sys/uvm/uvm_object.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_object.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_object.rs@3c62ede68802
//!
//! A UVM memory object represents a list of pages, which are managed by the object's pager
//! operations. All pages belonging to an object are owned by it and thus protected by the
//! object lock.
//!
//! The lock may be shared amongst the UVM objects; by default it is a lock object of its own
//! (`"uobjlk"`). At the pin only the kernel object's bootstrap lock is a different one
//! (`kernel::vm::aobj`); tmpfs's sharing of an aobj's lock with its vnode is `#if 0` in the C.
//!
//! The reference count is managed atomically for the anonymous UVM objects; a vnode object's
//! changes under its lock.
//!
//! ## Deviations
//! - `UVM_OBJ_IS_DEVICE` answers false: the device pager (`uvm_device.c`) is not ported.
//! - `UVM_OBJ_IS_VTEXT` ([`ObjRef::is_vtext`]) tests `VTEXT` on the object's vnode: the C casts
//!   the object itself to `struct vnode *`, which reads a member of the `uvm_vnode` instead.
//! - `pgo_fault` is not offered: no pager at the pin implements it (`uvm_device.c` is not
//!   ported), and the fault calls `pgo_get` for every object here.
//!
//! ## Redesign
//! - The pager operations (`struct uvm_pagerops`, a table of nine `Option<fn>` reached through
//!   `uobj->pgops`, each casting its `&UvmObject` back to the `uvm_aobj` or `uvm_vnode` it is
//!   the first member of, `#[repr(C)]`) -> [`ObjRef`], an enum of the handles to the objects a
//!   map entry, the buffer cache or exec holds (`Aobj`: a counted `hal::pool::PoolArc` of
//!   `"aobjpl"`; `Kernel`: the kernel object, a static; `Vnode`: a vnode's object, which is never
//!   freed), whose methods are the operations ([`ObjRef::get`], [`ObjRef::get_sync`],
//!   [`ObjRef::put`], [`ObjRef::flush`], [`ObjRef::reference`], [`ObjRef::detach`],
//!   [`ObjRef::cluster`], [`ObjRef::mk_pcluster`]) dispatching on the arm: no table, no cast.
//!   `pgops == &aobj_pager` and its kin are [`ObjKind`], fixed when the object is made;
//!   `pmap_pager` and `bufcache_pager` are the kinds of the dummy objects, which have no handle
//!   arm (the buffer cache embeds its [`Object`] in `struct buf`).
//! - `pgo_get`'s two protocols are two methods: with `PGO_LOCKED` the object lock is held in
//!   either mode and stays held ([`ObjRef::get`] borrows a witness of it, any mode: `hal`'s
//!   sealed `Witness`); without it the lock is held for writing and the pager returns with it
//!   released ([`ObjRef::get_sync`] consumes the write witness). `pgo_put` and `pgo_flush` keep
//!   the lock held across, releasing it around their I/O and sleeps: they borrow the write
//!   witness mutably. Each checks that the witness is its object's lock's.
//! - `vmobjlock: Cell<*const Rwlock>` (`rw_obj_alloc`, `rw_obj_free` in `uvm_obj_destroy`) ->
//!   `Option<hal::rwlock::SharedRwlock>`, `None` for the dummies; `uvm_obj_setlock` is
//!   [`Object::setlock`] on an object still its maker's (`&mut`: the C's "caller is responsible
//!   to ensure that UVM objects is not use" as a type). [`uvm_obj_destroy`] keeps the C's
//!   assertion; the lock goes with the object's memory (the last handle).
//! - `uo_refs` stays the C's count (map entries and other holders of a reference), a relaxed
//!   atomic: [`Object::refs_inc`]/[`Object::refs_dec_nv`] for the aobjs' atomic updates,
//!   [`Object::set_refs`] (which asserts the lock) for a vnode object's; the memory of an aobj
//!   belongs to the handle count, as an anon's and an amap's do (N3.4b).
//! - The page array of `pgo_get`/`pgo_put` is `&mut [PageSlot]` (`kernel::vm::pager`):
//!   `PGO_DONTCARE` is a variant, not a pointer value.
//! - An object is the page system's owner by implementing `kernel::vm::page::PageObject` here,
//!   in the kernel (the `PageOwner` bridge's object half, which the flip retires).

use core::ops::Deref;
use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};
use hal::pool::PoolArc;
use hal::rwlock::{LockId, RW_READ, RW_WRITE, RawRwlock, SharedRwlock, Witness, WriteWitness};
use hal::sleep::PVM;
use hal::sys::mman::{MADV_SEQUENTIAL, PROT_READ, PROT_WRITE};
use hal::tree::Tree;

use super::aobj::{
    Aobj, uao_detach, uao_dropswap, uao_flush, uao_get_locked, uao_get_sync, uao_reference,
};
use super::page::{
    Owner, OwnerTag, PG_BUSY, PG_CLEAN, PG_RELEASED, PG_WANTED, PQ_AOBJ, PageObject, Pglist,
    UvmObjtree, VmPage, uvm_obj_free_pages, uvm_page_owner_locked_p, uvm_page_unbusy,
    uvm_pagelookup, uvm_pageunwire, uvm_pagewire,
};
use super::pager::{PGO_ALLPAGES, PGO_LOCKED, PGO_SYNCIO, PageSlot, uvm_mk_pcluster};
use super::types::{VmProt, Voff};
use super::vnode::{
    Uvn, uvn_cluster, uvn_detach, uvn_flush, uvn_get_locked, uvn_get_sync, uvn_put, uvn_reference,
};
use crate::assert::{kassert, kpanic};

/// `UVM_OBJ_KERN` is a 'special' `uo_refs` value which indicates that the object is a kernel
/// memory object rather than a normal one (kernel memory objects don't have reference counts:
/// they never die).
///
/// This value is used to detected kernel object mappings at `uvm_unmap()` time. Normally when
/// an object is unmapped its pages eventually become deactivated and then paged out and/or
/// freed. This is not useful for kernel objects... when a kernel object is unmapped we always
/// want to free the resources associated with the mapping. `UVM_OBJ_KERN` allows us to decide
/// which type of unmapping we want to do.
///
/// In addition, we have kernel objects which may be used in an interrupt context. These
/// objects get their mappings entered with `pmap_kenter*()` and removed with `pmap_kremove()`,
/// which are safe to call in interrupt context, and must be used ONLY for wired kernel
/// mappings in these objects and their associated maps.
pub const UVM_OBJ_KERN: i32 = -2;

/// `FETCH_PAGECOUNT`: page count to fetch per single step.
const FETCH_PAGECOUNT: usize = 16;

/// The pager of an object (`uobj->pgops`), fixed when the object is made.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjKind {
    /// `aobj_pager`: anonymous memory (`kernel::vm::aobj`).
    Aobj,
    /// `uvm_vnodeops`: a vnode's pages (`kernel::vm::vnode`).
    Vnode,
    /// `pmap_pager`: a dummy object some pmaps use for sanity checks; no lock.
    Pmap,
    /// `bufcache_pager`: a dummy object of the buffer cache; no lock (the kernel lock guards
    /// it).
    Bufcache,
}

impl ObjKind {
    /// `UVM_OBJ_IS_DUMMY`: a pmap or bufcache object, which has no lock of its own.
    pub const fn is_dummy(self) -> bool {
        matches!(self, ObjKind::Pmap | ObjKind::Bufcache)
    }
}

/// `struct uvm_object`.
///
/// Locks: `lock` (`vmobjlock`) guards the page tree and `npages` (the page system writes
/// them under it); `refs` is atomic for an aobj and changed under the lock for a vnode object;
/// `kind` never changes. A dummy object has no lock.
pub struct Object {
    /// `vmobjlock`: lock on object.
    lock: Option<SharedRwlock>,
    /// `pgops`: the pager.
    kind: ObjKind,
    /// `memt`: pages in object.
    pages: Tree<UvmObjtree>,
    /// `uo_npages`: # of pages in memt.
    npages: AtomicI32,
    /// `uo_refs`: reference count.
    refs: AtomicI32,
}

/// `rw_obj_alloc(&lock, name)`: a lock object of its own, which the C cannot fail to make (its
/// pool sleeps for an item, and panics without one).
fn shared_lock(name: &'static str) -> SharedRwlock {
    match SharedRwlock::new(name) {
        Some(l) => l,
        None => kpanic(format_args!("rw_obj_alloc: rwlock_obj_pool is empty")),
    }
}

impl Object {
    /// `uvm_obj_init(uobj, pgops, refs)` of a new object of `kind` with `refs` references: a
    /// lock object of its own (`"uobjlk"`), unless the object is a dummy or the kernel object
    /// (`refs == UVM_OBJ_KERN`), whose lock is set with [`Object::setlock`].
    pub fn new(kind: ObjKind, refs: i32) -> Self {
        let alock = !kind.is_dummy() && refs != UVM_OBJ_KERN;
        Self {
            // Allocate and assign a lock; or the lock will need to be set via setlock.
            lock: alock.then(|| shared_lock("uobjlk")),
            kind,
            pages: Tree::new(),
            npages: AtomicI32::new(0),
            refs: AtomicI32::new(refs),
        }
    }

    /// `uvm_obj_init(uobj, &pmap_pager or &bufcache_pager, refs)` as a `const`: an empty,
    /// lockless dummy object (`kind` must be [`ObjKind::Pmap`] or [`ObjKind::Bufcache`]), for
    /// the objects embedded in other structures (`struct buf`'s `b_uobj`).
    pub const fn dummy(kind: ObjKind, refs: i32) -> Self {
        Self {
            lock: None,
            kind,
            pages: Tree::new(),
            npages: AtomicI32::new(0),
            refs: AtomicI32::new(refs),
        }
    }

    /// An empty object of `kind` with `refs` references under the lock that lives for ever at
    /// `lock` (the kernel object and `"kobjlk"`: `uvm_obj_init` then `uvm_obj_setlock`).
    pub(super) const fn with_static_lock(
        kind: ObjKind,
        refs: i32,
        lock: &'static RawRwlock,
    ) -> Self {
        Self {
            lock: Some(SharedRwlock::from_static(lock)),
            kind,
            pages: Tree::new(),
            npages: AtomicI32::new(0),
            refs: AtomicI32::new(refs),
        }
    }

    /// `uvm_obj_init(uobj, pgops, refs)` again on a dummy object that is reused (the buffer
    /// cache's, when a buffer takes new pages): no pages, `refs` references.
    pub fn reinit_dummy(&self, refs: i32) {
        kassert!(self.kind.is_dummy());
        // RBT_INIT(uvm_objtree, &uobj->memt): an emptied object's tree has nothing in it.
        kassert!(self.pages.is_empty());
        self.npages.store(0, Ordering::Relaxed);
        self.refs.store(refs, Ordering::Relaxed);
    }

    /// `uvm_obj_setlock`: assign a vmobjlock to the UVM object, `None` for a lock object of its
    /// own (`"uobjlk"`); the old lock, if any, loses this object's reference. The object is its
    /// maker's alone (`&mut`): nothing uses it yet.
    pub fn setlock(&mut self, lock: Option<SharedRwlock>) {
        // Drop the reference on the old lock; if new lock is not passed - allocate default one.
        self.lock = Some(lock.unwrap_or_else(|| shared_lock("uobjlk")));
    }

    /// `uobj->vmobjlock`, which every object but the dummies has.
    pub fn lock(&self) -> &SharedRwlock {
        match &self.lock {
            Some(l) => l,
            None => kpanic(format_args!("uvm_object without a lock")),
        }
    }

    /// The pager of the object.
    pub fn kind(&self) -> ObjKind {
        self.kind
    }

    /// `UVM_OBJ_IS_DUMMY(uobj)`.
    pub fn is_dummy(&self) -> bool {
        self.kind.is_dummy()
    }

    /// `UVM_OBJ_IS_KERN_OBJECT(uobj)`.
    pub fn is_kern_object(&self) -> bool {
        self.refs() == UVM_OBJ_KERN
    }

    /// `UVM_OBJ_IS_AOBJ(uobj)`.
    pub fn is_aobj(&self) -> bool {
        self.kind == ObjKind::Aobj
    }

    /// `UVM_OBJ_IS_VNODE(uobj)`.
    pub fn is_vnode(&self) -> bool {
        self.kind == ObjKind::Vnode
    }

    /// `UVM_OBJ_IS_DEVICE(uobj)`: false until the device pager exists (see the module's
    /// deviations).
    pub fn is_device(&self) -> bool {
        false
    }

    /// `UVM_OBJ_IS_PMAP(uobj)`.
    pub fn is_pmap(&self) -> bool {
        self.kind == ObjKind::Pmap
    }

    /// `UVM_OBJ_IS_BUFCACHE(uobj)`.
    pub fn is_bufcache(&self) -> bool {
        self.kind == ObjKind::Bufcache
    }

    /// `uo_refs`.
    pub fn refs(&self) -> i32 {
        self.refs.load(Ordering::Relaxed)
    }

    /// `uo_refs = n`, under the object's lock held for writing (a vnode object's count).
    pub fn set_refs(&self, n: i32) {
        kassert!(self.write_held());
        self.refs.store(n, Ordering::Relaxed);
    }

    /// `atomic_inc_int(&uo_refs)` (an aobj's count).
    pub fn refs_inc(&self) {
        self.refs.fetch_add(1, Ordering::Relaxed);
    }

    /// `atomic_dec_int_nv(&uo_refs)`: the count after the decrement. The release/acquire pair
    /// orders every use of the object before its teardown by the last holder.
    pub fn refs_dec_nv(&self) -> i32 {
        self.refs.fetch_sub(1, Ordering::AcqRel) - 1
    }

    /// `uo_npages`.
    pub fn npages(&self) -> i32 {
        self.npages.load(Ordering::Relaxed)
    }

    /// `uobj->memt`: the object's pages by offset.
    pub fn pages(&self) -> &Tree<UvmObjtree> {
        &self.pages
    }

    /// `rw_write_held(uobj->vmobjlock)`: this process holds the lock for writing.
    pub fn write_held(&self) -> bool {
        self.lock.as_ref().is_some_and(|l| l.status() == RW_WRITE)
    }

    /// `rw_lock_held(uobj->vmobjlock)`.
    pub fn lock_held(&self) -> bool {
        self.lock
            .as_ref()
            .is_some_and(|l| matches!(l.status(), RW_WRITE | RW_READ))
    }

    /// `panic(9)` naming `who` unless `id` is the identity of the object's lock: the witness a
    /// pager operation is given must be of its object's lock (the C's `KASSERT(rw_lock_held(
    /// uobj->vmobjlock))`, made the check that lets the operation trust the witness).
    pub(super) fn check_witness(&self, id: LockId, who: &str) {
        if self.lock().id() != id {
            kpanic(format_args!("{who}: a witness of another lock"));
        }
    }
}

// The page system's view of an object (`kernel::vm::page`): the owner of its pages.
impl PageObject for Object {
    fn tag(&self) -> OwnerTag {
        OwnerTag::object(self)
    }

    fn is_dummy(&self) -> bool {
        Object::is_dummy(self)
    }

    fn is_kern_object(&self) -> bool {
        Object::is_kern_object(self)
    }

    fn locked(&self, exclusive: bool) -> bool {
        if exclusive {
            self.write_held()
        } else {
            self.lock_held()
        }
    }

    fn pages(&self) -> &Tree<UvmObjtree> {
        &self.pages
    }

    fn npages(&self) -> i32 {
        Object::npages(self)
    }

    fn set_npages(&self, n: i32) {
        self.npages.store(n, Ordering::Relaxed);
    }
}

/// A handle to an object that has a pager: what a map entry, exec or tmpfs holds (`struct
/// uvm_object *`). `Clone` copies the handle (for an aobj, one more holder of its memory); the
/// C's reference (`uo_refs`) is [`ObjRef::reference`] and [`ObjRef::detach`], which the
/// holders call as the C does.
#[derive(Clone)]
pub enum ObjRef {
    /// An anonymous object (`uao_create`).
    Aobj(PoolArc<Aobj>),
    /// The kernel object (`uvm.kernel_object`), which never dies.
    Kernel(&'static Aobj),
    /// A vnode's object (`uvn_attach`), which is never freed: the vnode keeps it.
    Vnode(&'static Uvn),
}

impl Deref for ObjRef {
    type Target = Object;

    fn deref(&self) -> &Object {
        match self {
            ObjRef::Aobj(a) => a.object(),
            ObjRef::Kernel(a) => a.object(),
            ObjRef::Vnode(u) => u.object(),
        }
    }
}

impl ObjRef {
    /// The aobj behind an anonymous object or the kernel object.
    pub fn aobj(&self) -> Option<&Aobj> {
        match self {
            ObjRef::Aobj(a) => Some(a),
            ObjRef::Kernel(a) => Some(a),
            ObjRef::Vnode(_) => None,
        }
    }

    /// The `uvm_vnode` behind a vnode object (`(struct uvm_vnode *)uobj`, `uvn_of`).
    pub fn uvn(&self) -> Option<&'static Uvn> {
        match self {
            ObjRef::Vnode(u) => Some(u),
            ObjRef::Aobj(_) | ObjRef::Kernel(_) => None,
        }
    }

    /// The object's address, as an identity compared and never dereferenced (the futex key,
    /// `ft_obj`).
    pub fn as_ptr(&self) -> *const Object {
        ptr::from_ref::<Object>(self)
    }

    /// Whether two handles name the same object.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        ptr::eq(a.as_ptr(), b.as_ptr())
    }

    /// `UVM_OBJ_IS_VTEXT(uobj)`: a vnode object whose vnode is a running text (see the
    /// module's deviations).
    pub fn is_vtext(&self) -> bool {
        match self {
            ObjRef::Vnode(u) => u.vnode().vtext(),
            ObjRef::Aobj(_) | ObjRef::Kernel(_) => false,
        }
    }

    /// `pgo_reference`: add a reference to the object (the caller keeps a clone of the handle
    /// with it).
    pub fn reference(&self) {
        match self {
            ObjRef::Aobj(a) => uao_reference(a),
            ObjRef::Kernel(a) => uao_reference(a),
            ObjRef::Vnode(u) => uvn_reference(u),
        }
    }

    /// `pgo_detach`: drop a reference to the object; the last one destroys it (an aobj: its
    /// pages, once unbusy, and its swap slots; its memory goes with the last handle). May sleep.
    pub fn detach(&self) {
        match self {
            ObjRef::Aobj(a) => uao_detach(a),
            ObjRef::Kernel(a) => uao_detach(a),
            ObjRef::Vnode(u) => uvn_detach(u),
        }
    }

    /// `pgo_flush(uobj, start, stop, flags)`: flush the object's pages in `[start, stop)`
    /// (`PGO_ALLPAGES`: all of them) as `flags` says (`PGO_CLEANIT`, `PGO_SYNCIO`,
    /// `PGO_DEACTIVATE`, `PGO_FREE`). `w` is the write witness of the object's lock, held
    /// across (released and taken again while the flush sleeps or does I/O). False on an I/O
    /// error.
    pub fn flush(&self, w: &mut WriteWitness<'_>, start: Voff, stop: Voff, flags: i32) -> bool {
        self.check_witness(w.lock_id(), "pgo_flush");
        match self {
            ObjRef::Aobj(a) => uao_flush(a, w, start, stop, flags),
            ObjRef::Kernel(a) => uao_flush(a, w, start, stop, flags),
            ObjRef::Vnode(u) => uvn_flush(u, w, start, stop, flags),
        }
    }

    /// `pgo_get(uobj, off, pps, npages, centeridx, access, advice, PGO_LOCKED | ..)`: the
    /// resident, unbusy pages of `[off, off + npages pages)` that the caller cares about (a
    /// [`PageSlot::DontCare`] slot is skipped) into `pps`, `*npages` set to how many; the lock
    /// stays held (`w`, a witness of the object's lock in either mode: a read witness only
    /// with `access` without `PROT_WRITE`). `VM_PAGER_OK` when the centre page (or, with
    /// `PGO_ALLPAGES`, every page) was found, `VM_PAGER_UNLOCK` when the caller must unlock and
    /// call [`ObjRef::get_sync`], `VM_PAGER_BAD` past an aobj's end. `off` is the offset of
    /// `pps[0]`, not of `pps[centeridx]`; the caller checks for released pages.
    #[allow(clippy::too_many_arguments, reason = "the C's signature")]
    pub fn get<W: Witness>(
        &self,
        w: &W,
        off: Voff,
        pps: &mut [PageSlot],
        npages: &mut i32,
        centeridx: i32,
        access: VmProt,
        advice: i32,
        flags: i32,
    ) -> i32 {
        self.check_witness(w.lock_id(), "pgo_get");
        kassert!(flags & PGO_LOCKED != 0);
        kassert!(self.write_held() || access & PROT_WRITE == 0);
        match self {
            ObjRef::Aobj(a) => uao_get_locked(a, w, off, pps, npages, centeridx, advice, flags),
            ObjRef::Kernel(a) => uao_get_locked(a, w, off, pps, npages, centeridx, advice, flags),
            ObjRef::Vnode(u) => uvn_get_locked(u, w, off, pps, npages, centeridx, advice, flags),
        }
    }

    /// `pgo_get(uobj, off, pps, npages, centeridx, access, advice, flags)` without
    /// `PGO_LOCKED`: the pages the caller must have (the centre one, or with `PGO_ALLPAGES`
    /// every empty slot), read in or made as the pager does, busy for the caller, into `pps`;
    /// the slots already filled are left as they are. Entered with the object's lock held for
    /// writing (`w`), returns with it released (the witness is consumed), whatever the result:
    /// `VM_PAGER_OK`, `VM_PAGER_BAD` (outside the object), `VM_PAGER_ERROR` (an I/O error).
    #[allow(clippy::too_many_arguments, reason = "the C's signature")]
    pub fn get_sync(
        &self,
        w: WriteWitness<'_>,
        off: Voff,
        pps: &mut [PageSlot],
        npages: &mut i32,
        centeridx: i32,
        access: VmProt,
        advice: i32,
        flags: i32,
    ) -> i32 {
        self.check_witness(w.lock_id(), "pgo_get");
        kassert!(flags & PGO_LOCKED == 0);
        match self {
            ObjRef::Aobj(a) => {
                uao_get_sync(a, w, off, pps, npages, centeridx, access, advice, flags)
            }
            ObjRef::Kernel(a) => {
                uao_get_sync(a, w, off, pps, npages, centeridx, access, advice, flags)
            }
            ObjRef::Vnode(u) => {
                uvn_get_sync(u, w, off, pps, npages, centeridx, access, advice, flags)
            }
        }
    }

    /// `pgo_put(uobj, pps, npages, flags)`: write the busy pages `pps[..npages]` to backing
    /// store (`PGO_SYNCIO`: synchronously). `w`, the write witness of the object's lock, is held
    /// across (released and taken again around the I/O). The caller sets `PG_CLEAN` and clears
    /// the modified bits. An aobj has no `pgo_put` (its pages go to swap, `uvm_swap`):
    /// `panic(9)` as the C's call through `NULL` would fault.
    pub fn put(
        &self,
        w: &mut WriteWitness<'_>,
        pps: &mut [PageSlot],
        npages: i32,
        flags: i32,
    ) -> i32 {
        self.check_witness(w.lock_id(), "pgo_put");
        match self {
            ObjRef::Vnode(u) => uvn_put(u, w, pps, npages, flags),
            ObjRef::Aobj(_) | ObjRef::Kernel(_) => {
                kpanic(format_args!("uvm_pager_put: pager without pgo_put"))
            }
        }
    }

    /// `pgo_cluster(uobj, off, &lo, &hi)`: the range of offsets around `off` in which I/O may be
    /// clustered. The object's lock is held for writing (`w`). An aobj has none: `panic(9)`.
    pub fn cluster(&self, w: &WriteWitness<'_>, off: Voff) -> (Voff, Voff) {
        self.check_witness(w.lock_id(), "pgo_cluster");
        match self {
            ObjRef::Vnode(u) => uvn_cluster(u, off),
            ObjRef::Aobj(_) | ObjRef::Kernel(_) => {
                kpanic(format_args!("uvm_mk_pcluster: pager without pgo_cluster"))
            }
        }
    }

    /// `pgo_mk_pcluster(uobj, pps, npages, center, flags, mlo, mhi)`: the "put" cluster around
    /// the busy page `center` in `pps`, its start index returned and its length in `*npages`;
    /// `None` for a pager that does not cluster (an aobj: `pgo_mk_pcluster` is `NULL`). The
    /// object's lock (`w`) and the page queues are held.
    #[allow(clippy::too_many_arguments, reason = "the C's signature")]
    pub fn mk_pcluster(
        &self,
        w: &WriteWitness<'_>,
        pps: &mut [PageSlot],
        npages: &mut i32,
        center: &'static VmPage,
        flags: i32,
        mlo: Voff,
        mhi: Voff,
    ) -> Option<usize> {
        self.check_witness(w.lock_id(), "pgo_mk_pcluster");
        match self {
            // use generic version of this: see uvm_pager.c
            ObjRef::Vnode(_) => Some(uvm_mk_pcluster(
                self, w, pps, npages, center, flags, mlo, mhi,
            )),
            ObjRef::Aobj(_) | ObjRef::Kernel(_) => None,
        }
    }
}

/// `uvm_obj_destroy`: an object that goes away has no pages left. Its lock (`rw_obj_free`)
/// goes with its memory, the last handle.
pub fn uvm_obj_destroy(uo: &Object) {
    kassert!(uo.pages.is_empty());
}

/// `PG_WANTED` on the busy page `pg` of `obj`, then asleep on it with the object's lock
/// released and taken again for writing (the C's `uvm_pagewait(pg, lock, wmesg)` followed by
/// `rw_enter(lock, RW_WRITE)`, for a caller that holds the lock across: `w` is borrowed, not
/// consumed). The caller looks the page up again.
pub(super) fn pagewait_relock(
    pg: &VmPage,
    obj: &Object,
    w: &mut WriteWitness<'_>,
    wmesg: &'static str,
) {
    kassert!(pg.flags() & PG_BUSY != 0);
    kassert!(uvm_page_owner_locked_p(pg, Owner::Object(obj), false));

    pg.set_bits(PG_WANTED);
    // Without PCATCH and a timeout the sleep ends only by a wakeup.
    let _ = w.sleep(pg.chan(), PVM, wmesg);
}

/// The page in a slot the pager filled, which the C dereferences: neither empty nor
/// `PGO_DONTCARE`.
pub(super) fn slot_page(slot: PageSlot, who: &str) -> &'static VmPage {
    match slot.page() {
        Some(pg) => pg,
        None => kpanic(format_args!("{who}: no page in the slot")),
    }
}

/// `uvm_obj_wire`: wire the pages of entire UVM object.
///
/// NOTE: this function should only be used for types of objects where `PG_RELEASED` flag is
/// never set (aobj objects). Caller must pass page-aligned start and end values. If the
/// caller passes in a pageq pointer, we'll return a list of wired pages.
pub fn uvm_obj_wire(uobj: &ObjRef, start: Voff, end: Voff, pageq: Option<&Pglist>) -> i32 {
    let mut pgs = [PageSlot::Empty; FETCH_PAGECOUNT];
    let mut offset = start;
    let mut left = ((end - start) >> PAGE_SHIFT) as usize;
    let lock = uobj.lock();
    let owner = Owner::Object(&**uobj);

    // RW_DUPOK: a witness(4) flag, nothing else.
    let mut w = lock.write();
    while left != 0 {
        let mut npages = FETCH_PAGECOUNT.min(left) as i32;

        // Get the pages
        pgs.fill(PageSlot::Empty);
        let error = uobj.get_sync(
            w,
            offset,
            &mut pgs,
            &mut npages,
            0,
            PROT_READ | PROT_WRITE,
            MADV_SEQUENTIAL,
            PGO_ALLPAGES | PGO_SYNCIO,
        );

        if error != 0 {
            // error: Unwire the pages which have been wired
            uvm_obj_unwire(uobj, start, offset);
            return error;
        }

        w = lock.write();
        let npages = npages as usize;
        let mut unbusy: [Option<&VmPage>; FETCH_PAGECOUNT] = [None; FETCH_PAGECOUNT];
        for (slot, out) in pgs[..npages].iter().zip(unbusy.iter_mut()) {
            let pg = slot_page(*slot, "uvm_obj_wire");
            kassert!(pg.flags() & PG_RELEASED == 0);

            if pg.flags() & PQ_AOBJ != 0 {
                pg.clear_bits(PG_CLEAN);
                if let Some(aobj) = uobj.aobj() {
                    let _ = uao_dropswap(aobj, &mut w, (pg.offset() >> PAGE_SHIFT) as i32);
                }
            }
            *out = Some(pg);
        }

        // Wire the pages
        for pg in unbusy[..npages].iter().flatten() {
            uvm_pagewire(pg, owner);
            if let Some(pageq) = pageq {
                // A page of this object, on no page queue once wired: the caller's list.
                pageq.insert_tail(super::page::static_of(pg));
            }
        }

        // Unbusy the pages
        uvm_page_unbusy(&unbusy[..npages], owner);

        left -= npages;
        offset += (npages as Voff) << PAGE_SHIFT;
    }
    drop(w);

    0
}

/// `uvm_obj_unwire`: unwire the pages of entire UVM object.
///
/// Caller must pass page-aligned start and end values.
pub fn uvm_obj_unwire(uobj: &Object, start: Voff, end: Voff) {
    let _w = uobj.lock().write();
    let mut offset = start;
    while offset < end {
        let Some(pg) = uvm_pagelookup(uobj, offset) else {
            kpanic(format_args!("uvm_obj_unwire: page {offset:#x} is gone"));
        };
        kassert!(pg.flags() & PG_RELEASED == 0);

        uvm_pageunwire(pg, Owner::Object(uobj));
        offset += PAGE_SIZE as Voff;
    }
}

/// `uvm_obj_free`: free all pages in a uvm object, used by the buffer cache to free all pages
/// attached to a buffer. The kernel lock is held.
pub fn uvm_obj_free(uobj: &Object) {
    kassert!(uobj.is_bufcache());
    hal::klock::kernel_assert_locked();

    // Extract from rb tree in offset order, clean each page and free them as one list.
    uvm_obj_free_pages(uobj);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;

    use super::*;
    use crate::testutil;
    use crate::vm::page::{PG_FAKE, uvm_pagealloc};

    /// Real page memory and the lock objects.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(256 * 4096);
        let _ = hal::rwlock::init_shared();
        guard
    }

    #[test]
    fn uo_refs_count_like_the_c_atomics() {
        let _g = setup();
        let obj = Object::new(ObjKind::Aobj, 1);
        obj.refs_inc();
        assert_eq!(obj.refs(), 2);
        assert_eq!(obj.refs_dec_nv(), 1);
        assert_eq!(obj.refs_dec_nv(), 0);
        let kern = Object::new(ObjKind::Aobj, UVM_OBJ_KERN);
        assert!(kern.is_kern_object());
    }

    #[test]
    fn a_new_object_has_a_lock_of_its_own_and_a_dummy_none() {
        let _g = setup();
        let a = Object::new(ObjKind::Vnode, 0);
        let b = Object::new(ObjKind::Vnode, 0);
        assert!(!SharedRwlock::same(a.lock(), b.lock()));
        assert!(a.is_vnode() && !a.is_dummy());
        let d = Object::dummy(ObjKind::Bufcache, 1);
        assert!(d.is_dummy() && d.is_bufcache() && !d.lock_held());
        // The kernel object's refs make no lock: it is set afterwards.
        let mut k = Object::new(ObjKind::Aobj, UVM_OBJ_KERN);
        k.setlock(Some(a.lock().clone()));
        assert!(SharedRwlock::same(k.lock(), a.lock()));
        k.setlock(None);
        assert!(!SharedRwlock::same(k.lock(), a.lock()));
    }

    #[test]
    fn the_lock_is_what_the_page_system_sees() {
        let _g = setup();
        let obj = Object::new(ObjKind::Aobj, 1);
        assert!(!PageObject::locked(&obj, false));
        {
            let _r = obj.lock().read();
            assert!(PageObject::locked(&obj, false) && !PageObject::locked(&obj, true));
        }
        let w = obj.lock().write();
        assert!(obj.write_held());
        let pg = uvm_pagealloc(Owner::Object(&obj), 0, 0).expect("a page");
        assert_eq!(obj.npages(), 1);
        assert!(pg.owned_by_object(&obj));
        assert!(uvm_pagelookup(&obj, 0).is_some_and(|p| core::ptr::eq(p, pg)));
        pg.clear_bits(PG_BUSY | PG_FAKE);
        super::super::page::uvm_pagefree(pg, Owner::Object(&obj));
        assert_eq!(obj.npages(), 0);
        uvm_obj_destroy(&obj);
        drop(w);
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_pager_refuses_a_witness_of_another_lock() {
        let _g = setup();
        let obj = Object::new(ObjKind::Aobj, 1);
        let other = SharedRwlock::new("other").expect("a lock");
        let w = other.write();
        obj.check_witness(w.lock_id(), "pgo_get");
    }

    #[test]
    fn a_dummy_object_is_reused_empty() {
        let _g = setup();
        let d = Object::dummy(ObjKind::Bufcache, 0);
        d.reinit_dummy(1);
        assert_eq!((d.refs(), d.npages()), (1, 0));
    }
}
/* </TESTS> */
