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
//! The host double's `<machine/param.h>` and `<machine/vmparam.h>`: amd64's page geometry and
//! address-space bounds, so `hal`'s host tests and `bsd`'s host build see the numbers of a real
//! architecture (the same ones `bsd`'s `sys/arch/host/mod.rs` gives its `MachineParam` and
//! `VmParam`; `sys/machine/mod.rs` asserts that they agree). Not an OpenBSD file.

/// log2 of the page size.
pub const PAGE_SHIFT: usize = 12;
/// Bytes per page.
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
/// Byte offset mask within a page.
pub const PAGE_MASK: usize = PAGE_SIZE - 1;
/// Start of kernel virtual space (amd64's).
pub const KERNBASE: usize = 0xffff_ffff_8000_0000;

/// `VM_MIN_ADDRESS`: the lowest user address.
pub const VM_MIN_ADDRESS: usize = PAGE_SIZE;
/// `VM_MAXUSER_ADDRESS`: the highest address user mappings may reach (amd64's).
pub const VM_MAXUSER_ADDRESS: usize = 0x0000_7f7f_ffff_c000;
/// `VM_MAX_ADDRESS`: the end of the user address space (amd64's).
pub const VM_MAX_ADDRESS: usize = 0x0000_7fbf_dfef_f000;
/// `VM_MIN_KERNEL_ADDRESS`: where the host pretends kernel virtual space starts (amd64's).
pub const VM_MIN_KERNEL_ADDRESS: usize = 0xffff_8000_0000_0000;
/// `VM_MAX_KERNEL_ADDRESS`: where it ends (amd64's).
pub const VM_MAX_KERNEL_ADDRESS: usize = 0xffff_8080_0000_0000;

// The user layout and the default limits (`vmparam.h`; the values `bsd`'s host `VmParam`
// has, amd64's but for none; N3.5 S1, for the kernel's map and its limits).

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
