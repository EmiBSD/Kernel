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

/*-
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * William Jolitz.
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
 *	@(#)vmparam.h	5.9 (Berkeley) 5/12/91
 */
/* </LICENSES> */

/* <CODE> */
//! amd64 `<machine/vmparam.h>`: the address-space bounds, the part `hal` needs.
//!
//! Upstream: sys/arch/amd64/include/vmparam.h @ 3ce1f3f79392
//! LZ: sys/arch/amd64/include/vmparam.rs@3c62ede68802
//!
//! ## Redesign
//! - LZ's `include/vmparam.rs` holds the whole header. The bounds of the user and kernel
//!   address spaces moved here at N3.1 (decision 27's pattern): `hal`'s pmap refuses a user
//!   mapping at or above `VM_MAXUSER_ADDRESS` (amd64's recursive slot 255 starts just above
//!   it) and places the kernel's managed space by `VM_MIN/MAX_KERNEL_ADDRESS`. `bsd`'s file
//!   keeps the user limits and the physical segment policy (`VM_PSTRAT_*` is `uvm_page`'s) and
//!   re-exports these; `sys/machine/mod.rs` asserts that the two agree.
//! - N3.5 S1: the user layout and the default limits of `vmparam.h` (`USRSTACK`, `MAXTSIZ`,
//!   `DFLDSIZ`, `MAXDSIZ`, `BRKSIZ`, `DFLSSIZ`, `MAXSSIZ`, `STACKGAP_RANDOM`,
//!   `VM_MIN_STACK_ADDRESS`) are here too, for the kernel's map (`uvm_map_hint`, the brk
//!   area) and its limits; `bsd`'s file keeps its own copies, and `sys/machine/mod.rs`
//!   asserts at compile time that the two agree.

use super::param::PAGE_SIZE;

/// `VM_MIN_ADDRESS`: the lowest user address.
pub const VM_MIN_ADDRESS: usize = PAGE_SIZE;
/// `VM_MAXUSER_ADDRESS`: the highest address user mappings may reach.
pub const VM_MAXUSER_ADDRESS: usize = 0x0000_7f7f_ffff_c000;
/// `VM_MAX_ADDRESS`: the end of the user address space.
pub const VM_MAX_ADDRESS: usize = 0x0000_7fbf_dfef_f000;
/// `VM_MIN_KERNEL_ADDRESS`: the start of the kernel address space (the direct map).
pub const VM_MIN_KERNEL_ADDRESS: usize = 0xffff_8000_0000_0000;
/// `VM_MAX_KERNEL_ADDRESS`: the end of the kernel's own virtual space.
pub const VM_MAX_KERNEL_ADDRESS: usize = 0xffff_8080_0000_0000;

/// `USRSTACK`: the top (end) of the user stack.
pub const USRSTACK: usize = VM_MAXUSER_ADDRESS;
/// `MAXTSIZ`: max text size.
pub const MAXTSIZ: usize = 256 * 1024 * 1024;
/// `DFLDSIZ`: initial data size limit.
pub const DFLDSIZ: usize = 128 * 1024 * 1024;
/// `MAXDSIZ`: max data size.
pub const MAXDSIZ: usize = 128 * 1024 * 1024 * 1024;
/// `BRKSIZ`: heap gap size.
pub const BRKSIZ: usize = 8 * 1024 * 1024 * 1024;
/// `DFLSSIZ`: initial stack size limit.
pub const DFLSSIZ: usize = 2 * 1024 * 1024;
/// `MAXSSIZ`: max stack size.
pub const MAXSSIZ: usize = 32 * 1024 * 1024;
/// `STACKGAP_RANDOM`: the range of the random gap below the stack.
pub const STACKGAP_RANDOM: usize = 256 * 1024;
/// `VM_MIN_STACK_ADDRESS`: the lowest address the stack may be placed at.
pub const VM_MIN_STACK_ADDRESS: usize = 0x0000_6000_0000_0000;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/arch/amd64/include/vmparam.h");
        let ours: &[(&str, i64)] = &[
            ("VM_MAXUSER_ADDRESS", VM_MAXUSER_ADDRESS as i64),
            ("VM_MAX_ADDRESS", VM_MAX_ADDRESS as i64),
            ("VM_MIN_KERNEL_ADDRESS", VM_MIN_KERNEL_ADDRESS as i64),
            ("VM_MAX_KERNEL_ADDRESS", VM_MAX_KERNEL_ADDRESS as i64),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }

    #[test]
    fn the_user_space_ends_below_the_kernel() {
        assert!(VM_MIN_ADDRESS < VM_MAXUSER_ADDRESS);
        assert!(VM_MAXUSER_ADDRESS <= VM_MAX_ADDRESS);
        assert!(VM_MAX_ADDRESS < VM_MIN_KERNEL_ADDRESS);
        assert!(VM_MIN_KERNEL_ADDRESS < VM_MAX_KERNEL_ADDRESS);
    }
}
/* </TESTS> */
