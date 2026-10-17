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
//! `abi_openbsd.ko`, the ABI module's binary (decision 22): the `abi` crate linked alone,
//! with no kernel code and no `hal` state in it, and the header `hal::module` checks at the
//! ELF entry address before it calls [`abi::abi_init`].
//!
//! Everything the module calls in the kernel is a field of the `KernelApi` table it is
//! handed; it imports no symbol. Its one piece of runtime, the panic handler, is `hal`'s
//! (feature `module-rt`), since this crate, like every crate of `sys/abi`, is
//! `forbid(unsafe_code)` and holds no loop and no state (`cargo xtask abi-check`).
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

// The module's runtime: the panic handler (hal's feature `module-rt`).
extern crate hal;

use hal::api::AbiModuleHeader;

/// The header `hal::module` reads at the ELF entry address (`abi.ld` puts this static
/// there): the magic, the `KernelApi` version and size the module was built against, and
/// its entry point.
#[used]
static ABI_OPENBSD_MODULE_HEADER: AbiModuleHeader = AbiModuleHeader::new(abi::abi_init);
/* </CODE> */
