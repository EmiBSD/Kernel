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
//! The ABI: OpenBSD's userland interface, byte-identical, linked into the kernel image
//! (decision 47; the loadable module of decision 22 until the N5 close).
//! Translation only: userland bytes to the kernel's types and back, one `KernelApi` entry per
//! syscall, an errno from its `Result`; no decision on what a value means (decision 21,
//! `cargo xtask abi-check`). `forbid(unsafe_code)`: the copies from and to user memory are
//! the kernel's, behind `KernelApi`.
//!
//! Cargo keeps the crate away from `kernel` and `bsd` (it depends on `hal` alone); `hal`'s
//! public API acts on the kernel's own state, so `abi-check` R6 confines it to `hal::api`,
//! `hal::sys::*` and `hal::syscall`'s types (decision 47). It reaches the kernel only
//! through the `KernelApi` table the image's boot glue fills and hands to [`abi_init`]
//! (`sys/kern/abi_module.rs` until N9), and nothing of `kernel` names it. The constant
//! headers of the ABI edge (`<sys/syscall.h>`, `<sys/errno.h>`, ...) are `hal`'s, inherited from LZ
//! (`sys/hal/sys/`, the user's decision of 2026-10-16): the numbers of the table are
//! `hal::sys::syscall`'s, not a copy of them.
#![no_std]
#![forbid(unsafe_code)]
// `clippy::pedantic` is on in this crate (the user's decision of 2026-10-29, measured first:
// docs/JOURNAL.md). What it asks and this crate declines, each with its reason. The three cast
// lints are not among them: every system call argument is read through `scarg`, the C's
// `SCARG` per type family, whose two narrowings carry the crate's `#[expect]`s (the abi casts,
// the N5 close; kernel's and hal's casts are typed at N11, docs/ROADMAP.md).
#![warn(clippy::pedantic)]
#![allow(
    clippy::unreadable_literal,
    reason = "the hex tables of the ciphers and the constants of the C headers read as the C writes them"
)]
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    reason = "the C's names in prose and the `Result`'s type say it; backticks and an `# Errors` section per function are noise"
)]
#![allow(
    clippy::must_use_candidate,
    reason = "a `#[must_use]` on every pure function is noise; the ones that matter carry it"
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::many_single_char_names,
    reason = "OpenBSD's names are kept so the C can be grepped (`uvm::uvm_fault::uvm_fault`, `p`, `pr`, `ps`)"
)]
#![allow(
    clippy::too_many_lines,
    clippy::inline_always,
    reason = "a function the C keeps whole stays whole; `#[inline(always)]` sits on the hot paths decision 43 measures"
)]
#![allow(
    clippy::used_underscore_items,
    clippy::pub_underscore_fields,
    reason = "the C's `_IOR`/`_IOW` names and the `_pad` fields of `#[repr(C)]` layouts, kept as the C spells them"
)]

#[cfg(test)]
extern crate std;

pub mod scarg;
pub mod sysent;

use hal::api::KernelApi;

/// The ABI's entry point, which the image's boot glue calls once, before `main()` and so
/// before any system call: registers the system call handler and what the table holds.
pub fn abi_init(api: &'static KernelApi) {
    (api.register_syscall_handler)(api, sysent::syscall, sysent::INFO);
}
/* </CODE> */
