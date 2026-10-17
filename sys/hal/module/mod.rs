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
//! The loadable module `abi_openbsd.ko` (decision 22, docs/ZERO_UNSAFE.md 0.1): what `hal`
//! needs to load it and to let it register itself.
//!
//! - `elf`: the ELF64 file as the loader reads it, parsed in safe code.
//! - `load`: the loader: placement, relocation, mapping through the platform, the header's
//!   check and the call of the entry point; the boot line.
//! - `rt`: the module's own runtime, its panic handler (feature `module-rt`, set by
//!   `sys/abi/ko` alone).
//!
//! The module's header, which its linker script puts at the ELF entry address, is read
//! here as bytes before anything of the module runs, at the offsets of
//! [`crate::api::AbiModuleHeader`] (re-exported below).

#[forbid(unsafe_code)]
pub mod elf;
pub mod load;
#[forbid(unsafe_code)]
mod rt;

pub use crate::api::{
    HEADER_API_SIZE, HEADER_API_VERSION, HEADER_INIT, HEADER_MAGIC, HEADER_SIZE, MODULE_MAGIC,
};
pub use load::{LoadError, Loaded, ModulePlatform, Prot, load};

/// The page size the module is laid out and mapped in (4 KiB on both machines).
pub const PAGE_SIZE: usize = 4096;

/// The module's file name on the boot medium and in the boot line.
pub const MODULE_NAME: &str = "abi_openbsd.ko";
/* </CODE> */
