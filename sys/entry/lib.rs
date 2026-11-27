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
//! `entry`: the crate that decides where a trap from userland goes (decision 56 as amended by
//! decision 58, the user, 2026-11-27: every trap from userland is classified and dispatched by
//! `entry` alone, with direct calls downward, nothing registered at runtime; `kernel` never
//! calls `entry`, and `hal` reaches it through one symbol resolved at link time, from its
//! assembly, storing no pointer).
//!
//! Safe Rust with no assembly: `hal`'s EL0 handlers save the registers and call this crate's
//! one exported function (`arch::<machine>::trap`) with the frame, as `hal`'s `Arrival`, and
//! the vector the CPU took; it classifies the event in a closed `match` and hands it down:
//! `abi` for a system call, `kernel` (`kernel::trap`) for everything else a program can cause,
//! `hal` for an interrupt and an SError; then it returns the exit token the kernel's AST loop
//! got from `hal`'s gate, and `hal`'s assembly restores the registers.
//!
//! The rules (docs/ENTRY.md, section 5; `cargo xtask entry-check`): no `pub` item, no state,
//! no registration, no loop, no table; only the image crate names it (`bsd`'s kernel binary
//! until N9); the root denies `unsafe_code` and one item, the attribute that exports the
//! symbol, allows it. Off the machine (the host's tests and checks) the crate is empty: there
//! is no trap there.
#![no_std]
#![deny(unsafe_code)]
// `clippy::pedantic` is on, as in the kernel's other crates (the user's decision of
// 2026-10-29).
#![warn(clippy::pedantic)]

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
mod arch;
/* </CODE> */
