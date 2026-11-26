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
//! `sysdefs`: OpenBSD's basic types, constants and `#[repr(C)]` structures, the ABI edge's
//! data shapes (`errno.h`, `types.h`, `syscall.h`, the network headers, ...), in one leaf
//! crate under every layer (decision 54, the user, 2026-11-26): `hal`, `kernel`, `abi` and
//! `bsd` depend on it and it depends on nothing. `#![forbid(unsafe_code)]`, no state and no
//! logic; a byte reinterpretation of one of its types is `hal`'s (`hal::user::AbiBytes`,
//! `hal::sysbytes`), and `AbiPod`/`abi_pod!` (the marker those trust) are [`pod`].
//!
//! Moved from `hal::sys` (N2.2 to N7.1d), and grouped by purpose since 2026-11-26 (`base`,
//! `proc`, `sig`, `time`, `vm`, `file`, `fs`, `dev`, `net`, `netinet`, `netinet6`, as `hal` and
//! `kernel` are): `sysdefs::base::errno` became `sysdefs::base::errno`, `sysdefs::net::if_` became
//! `sysdefs::net::if_`. `hal-check` L1 keeps the root to `lib.rs` and the groups.
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

#[forbid(unsafe_code)]
pub mod base;
#[forbid(unsafe_code)]
pub mod dev;
#[forbid(unsafe_code)]
pub mod file;
#[forbid(unsafe_code)]
pub mod fs;
#[forbid(unsafe_code)]
pub mod net;
#[forbid(unsafe_code)]
pub mod netinet;
#[forbid(unsafe_code)]
pub mod netinet6;
#[forbid(unsafe_code)]
pub mod proc;
#[forbid(unsafe_code)]
pub mod sig;
#[forbid(unsafe_code)]
pub mod time;
#[forbid(unsafe_code)]
pub mod vm;

// The headers still `inherited` from LZ (`dirent`, `fcntl`, `filio`, `ioccom`, `syslimits`) name
// their siblings as `crate::sys::x`, and their tests `crate::base::reftest`, as LZ's tree does; these
// two crate-private aliases keep them byte-identical and are their only reason (never `pub`,
// never used by a file of ours).
#[forbid(unsafe_code)]
pub(crate) mod sys {
    pub(crate) use crate::base::{ioccom, param, types};
}
#[cfg(test)]
pub(crate) use base::reftest;

#[cfg(test)]
extern crate std;
/* </CODE> */
