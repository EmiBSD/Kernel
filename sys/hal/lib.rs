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
//! The HAL: the hardware (`arch/amd64`, `arch/arm64`, `machine`: the 24 machine headers that
//! name nothing of `bsd` and the 18 constant headers of `sys/`, moved in at N2.2; the rest of
//! the hardware arrives with its owner, the pmap at N3, cpu and traps at N4, the buses at N7)
//! and the runtime written new (locks that own their data, typed pools, owned memory, the
//! typed user copy, the bridge registry, and the system call dispatch point with the
//! registration of the ABI's handler; since decision 52 the ABI calls the kernel through
//! `kernel::api`, no table of `hal`'s). The only crate of the system where `unsafe` may appear;
//! every site carries a soundness proof and `cargo xtask hal-check` refuses a
//! `pub unsafe fn`: the API is safe (docs/ZERO_UNSAFE.md, section 0).
#![no_std]
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

pub mod arch;
pub mod machine;
#[forbid(unsafe_code)]
pub mod syscall;

// The host tests link std; a kernel image never does (the host double of the pmap declares
// its own, `arch/host/pmap.rs`).
#[cfg(test)]
extern crate std;
#[cfg(test)]
#[forbid(unsafe_code)]
// A verbatim copy of bsd's helper: the headers moved so far use only part of it.
#[allow(dead_code, unused_macros, unused_imports)]
pub(crate) mod reftest;

pub(crate) mod assert;
pub mod bridges;
pub mod cons;
pub mod cpu;
pub mod dev;
pub mod ds;
pub mod mem;
pub mod sync;
#[forbid(unsafe_code)]
pub mod sysbytes;

#[forbid(unsafe_code)]
pub mod prf;
// arc4random(9) over the pmap's `Random` bridge (N3.5): wherever that bridge is.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
#[forbid(unsafe_code)]
pub mod random;
#[cfg(test)]
pub(crate) mod testutil;
pub mod user;
pub mod vm;
/* </CODE> */
