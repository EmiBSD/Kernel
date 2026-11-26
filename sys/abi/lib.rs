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
//! Translation only: userland bytes to the kernel's types and back, one `kernel::api` call
//! per syscall, an errno from its `Result`; no decision on what a value means (decision 21,
//! `cargo xtask abi-check`). `forbid(unsafe_code)`: the copies from and to user memory are
//! the kernel's, behind `kernel::api`.
//!
//! The chain is userland -> `abi` -> `kernel` -> `hal` (decision 52, the user, 2026-11-26):
//! the crate depends on `kernel` and `hal`, never on `bsd`, and `abi-check` R6 confines what
//! it names: of `kernel`, only `kernel::api` (one function per system call); of `hal`, only
//! the passive `sysdefs::*`, `hal::syscall`'s types and the registration of its handler
//! ([`abi_init`]), and (decision 53) `hal::syscall::UserRegs`, the trapping thread's user
//! registers, since `hal`'s public API otherwise acts on the kernel's own state. Neither
//! `kernel` nor `hal` names it: the image's boot glue calls [`abi_init`]
//! (`sys/kern/abi_module.rs` until N9), and `hal`'s system call trap calls the handler it
//! registered, the one upward call, with the trap's registers uninterpreted: the per-arch
//! system call convention is this crate's ([`arch`], decision 53). The constant headers of the ABI edge
//! (`<sys/syscall.h>`, `<sys/errno.h>`, ...) are `hal`'s, inherited from LZ (`sys/lib/sysdefs/`,
//! the user's decision of 2026-10-16): the numbers of the table are `sysdefs::proc::syscall`'s,
//! not a copy of them.
//!
//! The translations sit in one file per group of the kernel ([`proc`], [`vm`], [`file`],
//! [`event`], [`time`], [`base`], [`fs`], [`net`]), the table and the dispatcher in
//! [`sysent`], the argument reads in [`scarg`], the per-arch registers of a system call in
//! [`arch`].
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

pub mod arch;
pub mod base;
pub mod event;
pub mod file;
pub mod fs;
pub mod net;
pub mod proc;
pub mod scarg;
pub mod sysent;
pub mod time;
pub mod vm;

use hal::syscall::SyscallFn;

/// The ABI's entry point, which the image's boot glue calls once, before `main()` and so
/// before any system call: registers the machine's system call handler, its forked child's
/// first return ([`arch`]) and what the table holds. `hal` keeps the first handler
/// registered: when one was already there, the `Err` hands it back and the boot glue must
/// not go on with it.
pub fn abi_init() -> Result<(), SyscallFn> {
    hal::syscall::set_handler(arch::current::svc, arch::current::child, sysent::INFO)
}
/* </CODE> */
