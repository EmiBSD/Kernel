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
// `clippy::pedantic` is on in this crate (the user's decision of 2026-10-29, measured first:
// docs/JOURNAL.md). What it asks and this crate declines, each with its reason; the three cast
// lints are allowed until N11 types their sites one by one (docs/ROADMAP.md, N11).
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
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "typed one by one, 1,348 sites: abi's 235 (sysent.rs, the syscall arguments) in a slice of their own at the N5 close or the N6 start, the user's decision of 2026-10-29; kernel's 1,061 and hal's 52 at N11"
)]

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
