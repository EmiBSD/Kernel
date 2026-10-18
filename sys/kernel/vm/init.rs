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
 * from: Id: uvm_init.c,v 1.1.2.3 1998/02/06 05:15:27 chs Exp
 */
/* </LICENSES> */

/* <CODE> */
//! Init the vm system: `uvm/uvm_init.c`, the kernel's steps of `uvm_init`.
//!
//! Upstream: sys/uvm/uvm_init.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_init.rs@3c62ede68802
//!
//! ## Redesign
//! - The machine's numbers the virtual memory system needs that `hal` does not carry
//!   (`<machine/param.h>`'s `USPACE`,
//!   `USPACE_ALIGN` and `HAVE_USPACE_GUARD`, `<machine/exec.h>`'s `__LDPGSZ`,
//!   `<machine/pmap.h>`'s `dma_constraint`) are the machine's traits in `bsd`'s arch: `bsd`'s
//!   `uvm_init` hands them down once as a [`VmMachine`] value ([`uvm_machine_init`]), which
//!   the kernel's modules read through [`machine`]. `<machine/vmparam.h>`'s values
//!   (`VM_MAXUSER_ADDRESS`, `DFLDSIZ`, `MAXSSIZ`, ...) are `hal`'s
//!   (`hal::arch::current::vmparam`).

use hal::once::OnceStatic;

use super::types::UvmConstraintRange;
use crate::assert::kpanic;

/// The machine's constants of the virtual memory system, handed down once by `bsd`'s
/// `uvm_init` ([`uvm_machine_init`]).
#[derive(Clone, Copy, Debug)]
pub struct VmMachine {
    /// `__LDPGSZ`: the page size the linker assumes (`mmap(2)`'s alignment of a large
    /// anonymous mapping).
    pub ldpgsz: usize,
    /// `USPACE`: a thread's u-area (PCB and kernel stack).
    pub uspace: usize,
    /// `USPACE_ALIGN`: the u-area's alignment.
    pub uspace_align: usize,
    /// `__HAVE_USPACE_GUARD`: a guard page between the PCB and the stack.
    pub have_uspace_guard: bool,
    /// `dma_constraint`: the physical range every DMA-capable device reaches.
    pub dma_constraint: &'static UvmConstraintRange,
}

/// The machine's constants, once `uvm_init` handed them down.
static MACHINE: OnceStatic<&'static VmMachine> = OnceStatic::new();

/// Records the machine's constants, once (`bsd`'s `uvm_init`, first; the tests' set-up); a
/// second call keeps the first.
pub fn uvm_machine_init(m: &'static VmMachine) {
    let _ = MACHINE.set(m);
}

/// The machine's constants; panics before [`uvm_machine_init`] (nothing that reads them runs
/// before `uvm_init`).
pub fn machine() -> &'static VmMachine {
    match MACHINE.get() {
        Some(m) => m,
        None => kpanic(format_args!(
            "uvm: the machine's constants used before uvm_init"
        )),
    }
}
/* </CODE> */
