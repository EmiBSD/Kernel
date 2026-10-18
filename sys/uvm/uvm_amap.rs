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
//! `uvm_amap.h`: general amap interface and amap implementation-specific info, and
//! `uvm_amap.c`: amap operations: `bsd`'s adapter to the kernel's amaps, `kernel::vm::amap`
//! (N3.4e).
//!
//! Upstream: sys/uvm/uvm_amap.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_amap.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_amap.rs@3c62ede68802
//!
//! The amaps, their chunks, their pools and every function of `uvm_amap.c` but the map's
//! half of `amap_copy` are the kernel's (`sys/kernel/vm/amap.rs`), re-exported here by their
//! names. What stays is what writes a map entry, which is `bsd`'s until N3.5:
//! [`amap_copy`] (the chunking clips and the install of the new reference), and the
//! `amap_flags`/`amap_refs` macros over a map entry's amap.
//!
//! ## Deviations
//! - Swap off (`amap_swap_off`) is the kernel's visible stub until `uvm_swap.c` (N3.5, G7 of
//!   N3.4); `UVM_PAGE_OWN` is not configured (LZ's).
//!
//! ## Redesign
//! - N3.4e: LZ's module is the whole amap (`VmAmap` as a `&'static` pool item, its anons as
//!   raw pointers, `am_lock` a legacy `rw_obj`, the global amap list). It moved into the
//!   kernel crate (`sys/kernel/vm/amap.rs`, `## Redesign`); this file holds no `unsafe` and
//!   is `forbid`.
//! - `amap_copy(map, entry, ..)` is split (DECISIONS G9 of N3.4, N3.2's `uvm_km_init`
//!   precedent): the kernel's `amap_copy_from` copies the anons and shares the lock
//!   ([`CopyOutcome`]); this half keeps the chunking decision, the clips
//!   (`UVM_MAP_CLIP_START`/`END`), the install of the entry's new reference and
//!   `UVM_ET_NEEDSCOPY`.
//! - The forms of the C that take a `struct vm_aref *` (`amap_lookup`, `amap_lookups`,
//!   `amap_add`, `amap_unadd`, `amap_splitref`, `amap_cow_now`) are the kernel's over the
//!   amap and the reference's page offset, under a witness of the amap's lock where the C
//!   requires it held; the callers pass `entry.aref`'s parts.

pub use kernel::vm::amap::*;

use crate::sys::param::PAGE_SHIFT;
use crate::uvm::uvm::UVM_ET_NEEDSCOPY;
use crate::uvm::uvm_map::{VmMap, VmMapEntry, uvm_map_clip_end_at, uvm_map_clip_start_at};
use crate::uvm::uvm_param::atop;

/// `amap_flags(AMAP)`.
pub fn amap_flags(amap: &Amap) -> i32 {
    amap.flags()
}

/// `amap_refs(AMAP)`.
pub fn amap_refs(amap: &Amap) -> i32 {
    amap.refs()
}

/// `amap_copy`: ensure that a map entry's "needs_copy" flag is false by copying the amap if
/// necessary.
///
/// An entry with a null amap pointer will get a new (blank) one. The map that the map entry
/// belongs to must be locked by caller. The amap currently attached to "entry" (if any) must
/// be unlocked. If canchunk is true, then we may clip the entry into a chunk. "startva" and
/// "endva" are used only if canchunk is true. They are used to limit chunking (e.g. if you
/// have a large space that you know you are going to need to allocate amaps for, there is
/// no point in allowing that to be chunked).
pub fn amap_copy(
    map: &VmMap,
    entry: &VmMapEntry,
    waitf: i32,
    canchunk: bool,
    startva: usize,
    endva: usize,
) {
    let mut lazyalloc = false;

    // KASSERT(map != kernel_map): we use sleeping locks (the kernel map, M7a part 2).

    // Is there an amap to copy? If not, create one.
    let Some(srcamap) = entry.aref.amap() else {
        // Check to see if we have a large amap that we can chunk. We align startva/endva
        // to chunk-sized boundaries and then clip to them.
        //
        // If we cannot chunk the amap, allocate it in a way that makes it grow or shrink
        // dynamically with the number of slots.
        if atop(entry.end.get() - entry.start.get()) >= UVM_AMAP_LARGE {
            if canchunk {
                // convert slots to bytes
                let chunksize = UVM_AMAP_CHUNK << PAGE_SHIFT;
                let startva = (startva / chunksize) * chunksize;
                // roundup(endva, chunksize), wrapping as the C's unsigned arithmetic does.
                let endva = (endva.wrapping_add(chunksize - 1) / chunksize) * chunksize;
                uvm_map_clip_start_at(map, entry, startva);
                // watch out for endva wrap-around!
                if endva >= startva {
                    uvm_map_clip_end_at(map, entry, endva);
                }
            } else {
                lazyalloc = true;
            }
        }

        entry.aref.ar_pageoff.set(0);
        let amap = amap_alloc(entry.end.get() - entry.start.get(), waitf, lazyalloc);
        let made = amap.is_some();
        entry.aref.set_amap(amap);
        if made {
            entry.etype.set(entry.etype.get() & !UVM_ET_NEEDSCOPY);
        }
        return;
    };

    // The amap's half: take the source over when the entry holds its only reference, else
    // copy its slots (sharing the anons and the lock) and drop the entry's reference to it.
    let slots = (entry.end.get() - entry.start.get()) >> PAGE_SHIFT;
    match amap_copy_from(&srcamap, entry.aref.ar_pageoff.get(), slots, waitf) {
        // Just take over the existing amap.
        CopyOutcome::TakeOver => entry.etype.set(entry.etype.get() & !UVM_ET_NEEDSCOPY),
        // Install new amap (the entry's handle to the source goes with the old value).
        CopyOutcome::New(amap) => {
            entry.aref.ar_pageoff.set(0);
            entry.aref.set_amap(Some(amap));
            entry.etype.set(entry.etype.get() & !UVM_ET_NEEDSCOPY);
        }
        // No memory: the entry still needs a copy, as the C leaves it.
        CopyOutcome::NoMemory => {}
    }
}
/* </CODE> */
