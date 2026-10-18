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
/* </LICENSES> */

/* <CODE> */
//! `struct vm_map` as a placeholder (N3.5 S1): the type `km`'s map statics name, with the
//! bounds `uvm_km_pgremove` reads, until R1's `kernel::vm::map` (`uvm_map.c`, the brief's
//! "The shape") replaces this file whole. Nothing here is a map yet: no entries, no lock,
//! no caller in any kernel image (`bsd`'s `uvm_map.rs` is the live map until the flip, S2).

use core::sync::atomic::{AtomicUsize, Ordering};

/// `struct vm_map` (placeholder): its bounds only.
pub struct VmMap {
    /// `min_offset` \[I\].
    min_offset: AtomicUsize,
    /// `max_offset` \[I\].
    max_offset: AtomicUsize,
}

impl VmMap {
    /// A map of `[min, max)`.
    pub const fn new(min: usize, max: usize) -> Self {
        Self {
            min_offset: AtomicUsize::new(min),
            max_offset: AtomicUsize::new(max),
        }
    }

    /// `vm_map_min(map)`.
    pub fn min_offset(&self) -> usize {
        self.min_offset.load(Ordering::Relaxed)
    }

    /// `vm_map_max(map)`.
    pub fn max_offset(&self) -> usize {
        self.max_offset.load(Ordering::Relaxed)
    }
}
/* </CODE> */
