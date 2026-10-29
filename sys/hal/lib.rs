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
//! typed user copy, the bridge registry, the system call dispatch point and the ABI module's
//! loader). The only crate of the system where `unsafe` may appear; every site carries a
//! soundness proof and `cargo xtask hal-check` refuses a `pub unsafe fn`: the API is safe
//! (docs/ZERO_UNSAFE.md, section 0).
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
    reason = "N11: 1,348 sites (kernel 1,061, abi 235, hal 52) typed one by one, abi's syscall arguments first"
)]

#[forbid(unsafe_code)]
pub mod api;
pub mod arch;
pub mod machine;
pub mod module;
pub mod sys;
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
// Typed memory stolen at boot over the pmap's `steal_memory` (N3.3): wherever the pmap is.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
pub mod bootmem;
pub mod bridges;
// The clock frame (N4.8a): wherever a machine's frame is (arm64 and the host; amd64's
// `struct intrframe` at N10).
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[forbid(unsafe_code)]
pub mod clock;
pub mod cons;
#[forbid(unsafe_code)]
pub mod cpu;
pub mod intr;
#[forbid(unsafe_code)]
pub mod klock;
pub mod kmbuf;
pub mod link;
#[forbid(unsafe_code)]
pub mod list;
pub mod malloc;
// The byte view of frames mapped for I/O over the pmap (N3.4a): wherever the pmap is.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
pub mod mapped;
pub mod mutex;
#[forbid(unsafe_code)]
pub mod netlock;
pub mod once;
pub mod owned;
pub mod percpu;
pub mod pin;
pub mod pinlist;
// pmap(9) (N3.1). A machine's build has the module once its backend is in
// `arch/<machine>/pmap/`: the host double's, amd64's (N3.1 commit 4) and arm64's (commit 5),
// as the two pmap bridges of `bridges.rs`.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
#[forbid(unsafe_code)]
pub mod pmap;

pub mod pool;
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
pub mod refcnt;
pub mod rwlock;
pub mod seqlock;
// The signal frame of the running thread (N4.8b): wherever a machine's trap code is (arm64;
// amd64's is N10's) and on the host, where it is a double.
#[cfg(any(
    not(target_os = "none"),
    all(target_arch = "aarch64", not(feature = "module-rt"))
))]
#[forbid(unsafe_code)]
pub mod sig;
pub mod sleep;
pub mod slot;
#[forbid(unsafe_code)]
pub mod softintr;
#[cfg(test)]
pub(crate) mod testutil;
pub mod thread;
// What the machine's trap code asks the kernel (N4.8b): wherever a machine's trap code is
// (arm64; amd64's is N10's) and on the host, where it has no entry.
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[forbid(unsafe_code)]
pub mod trap;
#[forbid(unsafe_code)]
pub mod tree;
pub(crate) mod trust;
pub mod user;
pub mod vm;
/* </CODE> */
