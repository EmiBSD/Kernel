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

#[cfg(test)]
extern crate std;
// A verbatim copy of bsd's helper: the modules moved so far use only part of it.
#[cfg(test)]
#[allow(dead_code, unused_macros, unused_imports)]
pub(crate) mod reftest;

extern crate alloc;

pub use hal::sys;
pub mod crypto;
/* </CODE> */
