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
//! zlib as the kernel uses it: OpenBSD `sys/lib/libz`.
//!
//! Upstream: sys/lib/libz @ 3ce1f3f79392
//!
//! OpenBSD builds `libz` as a library of its own (`sys/lib/libz/Makefile`, with `-DSLOW -DSMALL
//! -DNO_GZIP`) and the kernel links what `sys/conf/files` lists: `crc32.c` always (`subr_disk.c`
//! checksums GPT headers with it), the rest for `ipsec`, `crypto`, `ppp_deflate` and `ddb`
//! (IPComp's `deflate_global`, `cryptosoft.c`, `ppp-deflate.c`, CTF sections). This crate ports
//! the same files, one module per C file (`deflate.c` and `deflate.h` → `deflate.rs`;
//! `zlib.h` → `zlib.rs`), grouped by purpose (`base`, `deflate`, `inflate`, `check`): the
//! public functions are `libz::inflate::inflate::inflate`, `libz::check::crc32::crc32`, ...
//!
//! The files here are under the zlib licence (`zopenbsd.c` is ISC), and are altered source
//! versions (rewrites in Rust); each file says so and carries the notice in full. This crate
//! depends on nothing but `alloc` and must stay that way.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
#[cfg(test)]
extern crate std;

#[forbid(unsafe_code)]
pub mod base;
#[forbid(unsafe_code)]
pub mod check;
#[forbid(unsafe_code)]
pub mod deflate;
#[forbid(unsafe_code)]
pub mod inflate;

// Two aliases for inherited files that stay byte-identical to LZ's: `inflate/inffixed.rs` names
// `crate::inftrees`, and the boot loaders' `efiboot/efidev.rs` (both archs) call
// `libz::crc32`. They are the only reason of these lines; no file of ours uses them.
pub use check::crc32::crc32;
pub(crate) use inflate::inftrees;
/* </CODE> */
