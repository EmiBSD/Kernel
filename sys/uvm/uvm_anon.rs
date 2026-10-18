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
/* </LICENSES> */

/* <CODE> */
//! Anonymous memory management: `<uvm/uvm_anon.h>` and `uvm_anon.c` (uvm anon ops): `bsd`'s
//! adapter to the kernel's anons, `kernel::vm::anon` (N3.4e).
//!
//! Upstream: sys/uvm/uvm_anon.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_anon.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_anon.rs@3c62ede68802
//!
//! The anons, their pool and every function of `uvm_anon.c` are the kernel's
//! (`sys/kernel/vm/anon.rs`), re-exported here by their names, so the legacy callers keep
//! their paths. What stays is `struct vm_aref`: the map's reference to an amap ("Locked by
//! vm_map lock", `uvm_anon.h`), which lives in `bsd`'s map entries until the map moves into
//! the kernel (N3.5).
//!
//! ## Deviations
//! - Swap is not here (N3.5): the kernel's `uvm_anon_dropswap` reports `uvm_swap_free` when
//!   an anon has a slot, which nothing assigns yet (LZ's).
//! - `uvm_anon_pagein` reports `uvmfault_anonget` without a fault (swapoff, N3.5; LZ's).
//!
//! ## Redesign
//! - LZ's `VmAnon` (a `bsd` pool item handed out as `&'static`, `an_lock` a raw pointer to a
//!   legacy `rw_obj`) is the kernel's `Anon`, a counted handle of `"anonpl"` whose lock is a
//!   `hal::rwlock::SharedRwlock` (see `sys/kernel/vm/anon.rs`, `## Redesign`).
//! - `VmAref::ar_amap` was a `Cell<*const VmAmap>` read through an `amap()` that made a
//!   `&'static` from it: it holds the entry's counted handle now ([`AmapRef`], the kernel's),
//!   in a `Cell` of the same size, read by cloning ([`VmAref::amap`]). The C copies the
//!   pointer between entries without counting (`uvm_mapent_copy`, the clips); here each copy
//!   is a handle clone ([`VmAref::copy_from`]), which keeps the amap's memory alive and
//!   leaves `am_ref`, the C's count of map entries, to the functions that change it
//!   (`amap_ref`, `amap_unref`, `amap_splitref`, `amap_copy`). An entry's handle goes when
//!   the entry drops its amap ([`VmAref::clear`]) or is freed (`uvm_mapent_free` clears it).
//!   `ar_amap` is private and no reference into the cell is ever handed out. The map lock
//!   guards it as the C says, but the read lock lets several readers in at once (two faults
//!   on one entry; `uvm_map_clean` on a busy map beside a fault), and a read takes the handle
//!   out of the `Cell` and puts it back: each read and write of the cell holds the reference's
//!   own `busy` flag for that window (a clone or a swap, nothing else), which fits in the
//!   padding beside `ar_pageoff`, so the entry keeps its size.

use core::cell::Cell;
use core::sync::atomic::{AtomicBool, Ordering};

pub use kernel::vm::anon::*;

use kernel::vm::amap::{Amap, AmapRef};

/// `struct vm_aref`: processes reference anonymous virtual memory maps with an anonymous
/// reference structure. Note that the offset field indicates which part of the amap we are
/// referencing. Locked by vm_map lock.
pub struct VmAref {
    /// `ar_pageoff`: page offset into amap we start.
    pub ar_pageoff: Cell<i32>,
    /// Held while `ar_amap`'s handle is taken out and put back (see the module's notes).
    busy: AtomicBool,
    /// `ar_amap`: the amap, the entry's counted handle (see the module's notes).
    ar_amap: Cell<Option<AmapRef>>,
}

impl VmAref {
    /// No amap.
    pub const fn new() -> Self {
        Self {
            ar_pageoff: Cell::new(0),
            busy: AtomicBool::new(false),
            ar_amap: Cell::new(None),
        }
    }

    /// `f` on the cell, with `busy` held (a short window: a clone or a swap, no sleep, no
    /// other lock; no interrupt handler reads an entry of a map it does not hold).
    fn with<R>(&self, f: impl FnOnce(&Cell<Option<AmapRef>>) -> R) -> R {
        while self
            .busy
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let r = f(&self.ar_amap);
        self.busy.store(false, Ordering::Release);
        r
    }

    /// `f` on the handle, in the cell's window.
    fn peek<R>(&self, f: impl FnOnce(Option<&AmapRef>) -> R) -> R {
        self.with(|c| {
            let a = c.take();
            let r = f(a.as_ref());
            c.set(a);
            r
        })
    }

    /// `ar_amap`, if the entry has one: a clone of the entry's handle.
    pub fn amap(&self) -> Option<AmapRef> {
        self.peek(|a| a.cloned())
    }

    /// `ar_amap != NULL`.
    pub fn has_amap(&self) -> bool {
        self.peek(|a| a.is_some())
    }

    /// `ar_amap == amap`: the reference is to `amap` (by identity).
    pub fn is_amap(&self, amap: &Amap) -> bool {
        self.peek(|a| a.is_some_and(|a| core::ptr::eq(&**a, amap)))
    }

    /// `ar_amap = amap`: the entry's handle replaced (the old one dropped once the cell is let
    /// go).
    pub fn set_amap(&self, amap: Option<AmapRef>) {
        drop(self.with(|c| c.replace(amap)));
    }

    /// `ar_pageoff = 0; ar_amap = NULL`.
    pub fn clear(&self) {
        self.ar_pageoff.set(0);
        self.set_amap(None);
    }

    /// `dst->aref = src->aref`: the C's copy of the pointer and the offset, a handle clone
    /// here (no `am_ref` is taken: the caller's `amap_ref`/`amap_splitref` does that where the
    /// C does).
    pub fn copy_from(&self, src: &VmAref) {
        self.ar_pageoff.set(src.ar_pageoff.get());
        self.set_amap(src.amap());
    }
}

impl Default for VmAref {
    fn default() -> Self {
        Self::new()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::machine::intr::IPL_MPFLOOR;
    use crate::sys::malloc::M_WAITOK;
    use crate::sys::param::PAGE_SIZE;
    use crate::uvm::uvm_map::VmMapEntry;
    use kernel::vm::amap::{amap_alloc, amap_init};

    #[test]
    fn an_aref_holds_a_handle_and_copies_share_it() {
        let _g = setup_real_memory();
        let _ = hal::rwlock::init_shared();
        let _ = amap_init(IPL_MPFLOOR);
        let amap = amap_alloc(4 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        let raw: *const Amap = &*amap;

        let a = VmAref::new();
        assert!(!a.has_amap() && a.amap().is_none());
        a.ar_pageoff.set(2);
        a.set_amap(Some(amap));
        assert!(a.has_amap());
        assert!(a.amap().is_some_and(|m| core::ptr::eq(&*m, raw)));

        // A copy (uvm_mapent_copy) shares the amap and the offset; am_ref is untouched.
        let b = VmAref::new();
        b.copy_from(&a);
        assert_eq!(b.ar_pageoff.get(), 2);
        assert!(b.amap().is_some_and(|m| a.is_amap(&m)));
        assert_eq!(a.amap().map(|m| m.refs()), Some(1));

        // Clearing one leaves the other's handle.
        a.clear();
        assert!(!a.has_amap() && a.ar_pageoff.get() == 0);
        assert!(b.has_amap());
        b.clear();
        assert!(!b.has_amap());
    }

    #[test]
    fn concurrent_readers_always_see_the_amap() {
        // Two faults on one entry read its aref under the map's read lock at once: neither
        // may see the cell empty while the other clones the handle.
        let _g = setup_real_memory();
        let _ = hal::rwlock::init_shared();
        let _ = amap_init(IPL_MPFLOOR);
        // The entry (Sync under its map's lock) holds the reference, as in a map.
        let e: &'static VmMapEntry = std::boxed::Box::leak(std::boxed::Box::new(VmMapEntry::new()));
        e.aref.set_amap(amap_alloc(PAGE_SIZE, M_WAITOK, false));
        let readers: std::vec::Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(move || {
                    (0..20_000).all(|_| e.aref.amap().is_some() && e.aref.has_amap())
                })
            })
            .collect();
        for r in readers {
            assert!(r.join().expect("a reader"), "a reader saw no amap");
        }
        e.aref.clear();
    }
}
/* </TESTS> */
