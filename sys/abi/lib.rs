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
//! The ABI: OpenBSD's userland interface, byte-identical, as the loadable module
//! `abi_openbsd.ko`. Translation only: userland bytes to the kernel's types and back, one
//! `KernelApi` entry per syscall, an errno from its `Result`; no decision on what a value means
//! (decision 21, `cargo xtask abi-check`). `forbid(unsafe_code)`: `hal::UserPtr` is safe.
//!
//! The module's binary is `sys/abi/ko` (package `abi_openbsd`), which carries the header
//! `hal::module` checks and calls [`abi_init`] through. The constant headers of the ABI edge
//! (`<sys/syscall.h>`, `<sys/errno.h>`, ...) are `hal`'s, inherited from LZ
//! (`sys/hal/sys/`, the user's decision of 2026-10-16): the numbers of the table are
//! `hal::sys::syscall`'s, not a copy of them.
#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

pub mod sysent;

use hal::api::KernelApi;

/// The module's entry point, which `hal::module` calls once, before `init(8)` starts:
/// registers the system call handler and what the table holds.
pub fn abi_init(api: &'static KernelApi) {
    (api.register_syscall_handler)(api, sysent::syscall, sysent::INFO);
}
/* </CODE> */
