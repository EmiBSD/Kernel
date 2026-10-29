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
//! The kernel: every decision the operating system makes, in safe Rust. `forbid(unsafe_code)`
//! at this root cannot be overridden below it: one `unsafe` anywhere in this crate fails the
//! build. The hardware and the runtime come from `hal`; what has not moved yet from the LZ
//! tree is reached only through a registered bridge (docs/ZERO_UNSAFE.md, section 0).
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
    reason = "N11: 1,348 sites (kernel 1,061, abi 235, hal 52) typed one by one, abi's syscall arguments first"
)]

// The host tests (and the doubles other crates' tests use) link std; a kernel image never
// does.
#[cfg(any(test, feature = "doubles"))]
extern crate std;
// A verbatim copy of bsd's helper: the modules moved so far use only part of it.
#[cfg(test)]
#[allow(dead_code, unused_macros, unused_imports)]
pub(crate) mod reftest;

extern crate alloc;

pub use hal::sys;
pub mod acct;
pub mod api;
pub(crate) mod assert;
pub mod clist;
pub mod clock;
pub mod clockintr;
pub mod cred;
pub mod crypto;
pub mod descrip;
pub mod event;
pub mod exit;
pub mod extent;
pub mod file;
pub mod fork;
pub mod fs;
pub mod futex;
pub mod log;
pub mod pipe;
pub mod prf;
pub mod proc;
pub mod prot;
pub mod pty;
pub mod reboot;
pub mod resource;
pub mod sched;
pub mod select;
pub mod sig;
pub mod sigio;
pub mod syscall;
pub mod sysctl;
pub mod task;
pub mod tc;
pub mod termios;
#[cfg(any(test, feature = "doubles"))]
pub mod testutil;
pub mod thread;
pub mod time;
pub mod timeout;
pub mod tty;
pub mod uio;
pub mod vm;
/* </CODE> */
