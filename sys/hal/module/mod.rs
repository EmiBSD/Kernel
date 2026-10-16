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
//! The loadable module `abi-openbsd.ko` (decision 22, docs/ZERO_UNSAFE.md 0.1): what `hal`
//! needs to load it and to let it register itself.
//!
//! - `elf`: the ELF64 file as the loader reads it, parsed in safe code.
//! - `load`: the loader: placement, relocation, mapping through the platform, the header's
//!   check and the call of the entry point; the boot line.
//! - `once`: the init-once cell the registrations are kept in.
//! - `rt`: the module's own runtime, its panic handler (feature `module-rt`, set by
//!   `sys/abi/ko` alone).
//!
//! The module's header, which its linker script puts at the ELF entry address, is read
//! here as bytes before its type is known: `kernel::api::AbiModuleHeader` is laid out as
//! the offsets below say, and a compile-time check there keeps the two in step.

#[forbid(unsafe_code)]
pub mod elf;
pub mod load;
pub mod once;
#[forbid(unsafe_code)]
mod rt;

pub use load::{LoadError, Loaded, ModulePlatform, Prot, load};

/// The page size the module is laid out and mapped in (4 KiB on both machines).
pub const PAGE_SIZE: usize = 4096;

/// The module's file name on the boot medium and in the boot line.
pub const MODULE_NAME: &str = "abi-openbsd.ko";

/// The first eight bytes of the module's header.
pub const MODULE_MAGIC: [u8; 8] = *b"EMIABIKO";

/// Offset of the header's magic.
pub const HEADER_MAGIC: usize = 0;
/// Offset of the `KernelApi` version the module was built against (`u32`).
pub const HEADER_API_VERSION: usize = 8;
/// Offset of the `KernelApi` size the module was built against (`u32`).
pub const HEADER_API_SIZE: usize = 12;
/// Offset of the entry point (a pointer, relocated).
pub const HEADER_INIT: usize = 16;
/// The header's size.
pub const HEADER_SIZE: usize = 24;
/* </CODE> */
