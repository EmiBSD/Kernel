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

/*-
 * Copyright (c) 1991, 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)vm_extern.h	8.5 (Berkeley) 5/3/95
 */
/* </LICENSES> */

/* <CODE> */
//! The types and flags of `<uvm/uvm_extern.h>` that name nothing but the machine's own types:
//! what the kernel's memory code and the legacy `uvm` share.
//!
//! Upstream: sys/uvm/uvm_extern.h @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_extern.rs@3c62ede68802
//!
//! `voff_t`, `vm_prot_t`, `vm_inherit_t`, `vm_fault_t`, the `UVM_MAPFLAG` encoding and its
//! extractors, the `UVM_FLAG_*` mapping flags, the `UVM_PGA_*`/`UVM_PLA_*` page allocation
//! flags, the `UVM_LK_*` lock flags, `struct uvm_constraint_range` and the core dump callback
//! types. The kernel memory allocation modes (`struct kmem_va_mode` and its siblings) are
//! `km`'s ([`super::km`]), which uses them; `struct vmspace` and the `exec_map`/`phys_map`
//! submaps embed or name the map, which is still `bsd`'s (`sys/uvm/uvm_extern.rs` keeps them
//! until N3.5 and re-exports this module).
//!
//! ## Deviations
//! - `UVM_MAPFLAG` and its extractors are `const fn`s.
//! - The core dump callbacks return `Result<(), Errno>` instead of an `int`.
//!
//! ## Redesign
//! - LZ's `uvm_extern.rs` is one module for the whole header; its items are split by owner
//!   (N3.2, DECISIONS G1): those that name only `hal`'s types are here, in the kernel crate,
//!   so the kernel's `km` can use them; the map's and the address space's stay in `bsd` with
//!   the map. `bsd`'s module re-exports these by name, so its 49 users compile unchanged.
//! - `vm_prot_t` is one type, `hal::pmap::VmProt` (the pmap's, N3.1); this alias names it.
//! - N3.5 S1: the kernel's `uvm_coredump_walkmap` (`vm::unix`) takes its callbacks as
//!   `&mut dyn FnMut` closures, the walk's over a [`CoredumpSeg`]: the `void *cookie` the C
//!   threads through is whatever the closure captures. The two `fn` types stay for `bsd`'s
//!   copy of the walk until the flip (S2) deletes it with them.

use hal::sys::errno::Errno;
use hal::sys::mman::{PROT_EXEC, PROT_READ, PROT_WRITE};
use hal::sys::types::{Off, Paddr};

/// `vm_fault_t`.
pub type VmFault = i32;
/// `vm_inherit_t`: XXX inheritance codes.
pub type VmInherit = i32;
/// `voff_t`: XXX offset within a uvm_object.
pub type Voff = Off;
/// `vm_prot_t` (the pmap's type, `hal::pmap::VmProt`).
pub type VmProt = hal::pmap::VmProt;

// Bit assignments assigned by UVM_MAPFLAG() and extracted by
// UVM_{PROTECTION,INHERIT,MAXPROTECTION,ADVICE}(): bits 0-2 protection, bits 4-5 inheritance,
// bits 8-10 max protection, bits 12-14 advice, bits 16-N flags.

/// Protections bits.
pub const PROT_MASK: i32 = PROT_READ | PROT_WRITE | PROT_EXEC;
/// Inherit mask.
pub const MAP_INHERIT_MASK: i32 = 0x3;
/// Advice mask.
pub const MADV_MASK: i32 = 0x7;

// mapping flags

/// Find space.
pub const UVM_FLAG_FIXED: u32 = 0x0010000;
/// Establish overlay.
pub const UVM_FLAG_OVERLAY: u32 = 0x0020000;
/// Don't merge map entries.
pub const UVM_FLAG_NOMERGE: u32 = 0x0040000;
/// Set copy_on_write flag.
pub const UVM_FLAG_COPYONW: u32 = 0x0080000;
/// Fail if we can not lock map.
pub const UVM_FLAG_TRYLOCK: u32 = 0x0100000;
/// No backend.
pub const UVM_FLAG_HOLE: u32 = 0x0200000;
/// Do everything, except actual execution.
pub const UVM_FLAG_QUERY: u32 = 0x0400000;
/// Don't fault.
pub const UVM_FLAG_NOFAULT: u32 = 0x0800000;
/// Unmap to make space.
pub const UVM_FLAG_UNMAP: u32 = 0x1000000;
/// Page may contain a stack.
pub const UVM_FLAG_STACK: u32 = 0x2000000;
/// Write combining.
pub const UVM_FLAG_WC: u32 = 0x4000000;
/// Omit from dumps.
pub const UVM_FLAG_CONCEAL: u32 = 0x8000000;
/// Sigaltstack validation required.
pub const UVM_FLAG_SIGALTSTACK: u32 = 0x20000000;

/// `UVM_PROTECTION(X)`.
pub const fn uvm_protection(x: u32) -> VmProt {
    (x as i32) & PROT_MASK
}

/// `UVM_INHERIT(X)`.
pub const fn uvm_inherit(x: u32) -> VmInherit {
    ((x >> 4) as i32) & MAP_INHERIT_MASK
}

/// `UVM_MAXPROTECTION(X)`.
pub const fn uvm_maxprotection(x: u32) -> VmProt {
    ((x >> 8) as i32) & PROT_MASK
}

/// `UVM_ADVICE(X)`.
pub const fn uvm_advice(x: u32) -> i32 {
    ((x >> 12) as i32) & MADV_MASK
}

/// `UVM_MAPFLAG(prot, maxprot, inh, advice, flags)`.
pub const fn uvm_mapflag(
    prot: VmProt,
    maxprot: VmProt,
    inh: VmInherit,
    advice: i32,
    flags: u32,
) -> u32 {
    (prot as u32) | ((maxprot as u32) << 8) | ((inh as u32) << 4) | ((advice as u32) << 12) | flags
}

/// Magic offset value: offset not known (obj) or don't care (!obj).
pub const UVM_UNKNOWN_OFFSET: Voff = -1;

// flags for uvm_pagealloc()

/// Ok to use reserve pages.
pub const UVM_PGA_USERESERVE: i32 = 0x0001;
/// Returned page must be zeroed.
pub const UVM_PGA_ZERO: i32 = 0x0002;

// flags for uvm_pglistalloc() also used by uvm_pmr_getpages()

/// May sleep.
pub const UVM_PLA_WAITOK: i32 = 0x0001;
/// Can't sleep (need one of the two).
pub const UVM_PLA_NOWAIT: i32 = 0x0002;
/// Zero all pages before returning.
pub const UVM_PLA_ZERO: i32 = 0x0004;
/// Try to allocate contig physmem.
pub const UVM_PLA_TRYCONTIG: i32 = 0x0008;
/// Don't wake page daemon on failure.
pub const UVM_PLA_NOWAKE: i32 = 0x0020;
/// Can allocate from kernel reserve.
pub const UVM_PLA_USERESERVE: i32 = 0x0040;

// lockflags that control the locking behavior of various functions.

/// Map locked on entry.
pub const UVM_LK_ENTER: i32 = 0x0000_0001;
/// Leave map locked on exit.
pub const UVM_LK_EXIT: i32 = 0x0000_0002;

/// `uvm_io` flag: extract the mappings with their maximum protection (`UVM_EXTRACT_FIXPROT`).
pub const UVM_IO_FIXPROT: i32 = 0x01;

/// Flag to `uvm_page_physload`: don't add to the page queue.
pub const PHYSLOAD_DEVICE: i32 = 0x01;

/// `struct uvm_constraint_range`: a physical address range an allocation must come from. The
/// machine sets such ranges up for the memory allocators, chiefly to keep the memory of some
/// consumers (the mbuf pools) where every device that does DMA can reach it, and to keep
/// allocations that could be served elsewhere out of ranges such as the ISA memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UvmConstraintRange {
    /// Lowest allowed address.
    pub ucr_low: Paddr,
    /// Highest allowed address.
    pub ucr_high: Paddr,
}

/// One segment of a core dump, as `uvm_coredump_walkmap` hands it to its walk callback
/// (`uvm_coredump_walk_cb`'s arguments, the `cookie` gone into the closure): `[start,
/// realend)` holds data, `[realend, end)` is absent (zero) memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoredumpSeg {
    /// `start`.
    pub start: usize,
    /// `realend`.
    pub realend: usize,
    /// `end`.
    pub end: usize,
    /// `prot`.
    pub prot: VmProt,
    /// `isvnode`: the segment maps a vnode.
    pub isvnode: bool,
    /// `nsegment`: the segment's index.
    pub nsegment: i32,
}

/// `uvm_coredump_setup_cb`: called once with the number of segments a core dump will hold.
pub type UvmCoredumpSetupCb =
    fn(nsegment: i32, cookie: *mut core::ffi::c_void) -> Result<(), Errno>;

/// `uvm_coredump_walk_cb`: called for each segment of a core dump: `[start, realend)` holds
/// data, `[realend, end)` is absent (zero) memory.
pub type UvmCoredumpWalkCb = fn(
    start: usize,
    realend: usize,
    end: usize,
    prot: VmProt,
    isvnode: bool,
    nsegment: i32,
    cookie: *mut core::ffi::c_void,
) -> Result<(), Errno>;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use hal::sys::mman::{MADV_RANDOM, MAP_INHERIT_NONE};

    #[test]
    fn map_flags_round_trip() {
        let f = uvm_mapflag(
            PROT_READ | PROT_WRITE,
            PROT_MASK,
            MAP_INHERIT_NONE,
            MADV_RANDOM,
            UVM_FLAG_FIXED,
        );
        assert_eq!(uvm_protection(f), PROT_READ | PROT_WRITE);
        assert_eq!(uvm_maxprotection(f), PROT_MASK);
        assert_eq!(uvm_inherit(f), MAP_INHERIT_NONE);
        assert_eq!(uvm_advice(f), MADV_RANDOM);
        assert_ne!(f & UVM_FLAG_FIXED, 0);
    }

    /// The flag values of `<uvm/uvm_extern.h>`, as the C writes them.
    #[test]
    fn the_flags_are_the_headers() {
        let table: &[(&str, u32, u32)] = &[
            ("UVM_FLAG_FIXED", UVM_FLAG_FIXED, 0x0010000),
            ("UVM_FLAG_OVERLAY", UVM_FLAG_OVERLAY, 0x0020000),
            ("UVM_FLAG_NOMERGE", UVM_FLAG_NOMERGE, 0x0040000),
            ("UVM_FLAG_COPYONW", UVM_FLAG_COPYONW, 0x0080000),
            ("UVM_FLAG_TRYLOCK", UVM_FLAG_TRYLOCK, 0x0100000),
            ("UVM_FLAG_HOLE", UVM_FLAG_HOLE, 0x0200000),
            ("UVM_FLAG_QUERY", UVM_FLAG_QUERY, 0x0400000),
            ("UVM_FLAG_NOFAULT", UVM_FLAG_NOFAULT, 0x0800000),
            ("UVM_FLAG_UNMAP", UVM_FLAG_UNMAP, 0x1000000),
            ("UVM_FLAG_STACK", UVM_FLAG_STACK, 0x2000000),
            ("UVM_FLAG_WC", UVM_FLAG_WC, 0x4000000),
            ("UVM_FLAG_CONCEAL", UVM_FLAG_CONCEAL, 0x8000000),
            ("UVM_FLAG_SIGALTSTACK", UVM_FLAG_SIGALTSTACK, 0x20000000),
            ("UVM_PLA_WAITOK", UVM_PLA_WAITOK as u32, 0x0001),
            ("UVM_PLA_NOWAIT", UVM_PLA_NOWAIT as u32, 0x0002),
            ("UVM_PLA_ZERO", UVM_PLA_ZERO as u32, 0x0004),
            ("UVM_PLA_TRYCONTIG", UVM_PLA_TRYCONTIG as u32, 0x0008),
            ("UVM_PLA_NOWAKE", UVM_PLA_NOWAKE as u32, 0x0020),
            ("UVM_PLA_USERESERVE", UVM_PLA_USERESERVE as u32, 0x0040),
        ];
        for &(name, got, want) in table {
            assert_eq!(got, want, "{name}");
        }
        assert_eq!(UVM_UNKNOWN_OFFSET, -1);
    }
}
/* </TESTS> */
