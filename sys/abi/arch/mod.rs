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
//! OpenBSD's system call convention, one file per machine (decision 53, the user,
//! 2026-11-26): which register holds the number and how wide it is read, the argument
//! registers, the return register and the error flag, when the call restarts, and a forked
//! child's first return. `hal`'s trap hands the trapping thread's registers over as a
//! `hal::syscall::UserRegs` and interprets none of them; the file of the machine the kernel
//! is built for is `current`, whose handler the crate `entry` calls (decision 56).
//!
//! The host build (the tests) has arm64's: `hal`'s host double has arm64's registers.
//! amd64's file (`rax` whole, the arguments in `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`, the
//! carry in `rflags`) is N10's (decision 44).

#[cfg(any(target_arch = "aarch64", not(target_os = "none")))]
pub mod arm64;

#[cfg(any(target_arch = "aarch64", not(target_os = "none")))]
pub use self::arm64 as current;
/* </CODE> */
