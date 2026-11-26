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

/*-
 * Copyright (c) 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)libkern.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! Freestanding kernel C library: OpenBSD `sys/lib/libkern` and its header `libkern.h`.
//!
//! Upstream: sys/lib/libkern/libkern.h @ 3ce1f3f79392
//!
//! One module per C file (`strlcpy.c` → `strlcpy.rs`), grouped by purpose (`string`, `mem`,
//! `hash`, `rand`): `libkern::string::strlcpy::strlcpy(..)`. Functions whose semantics `core` already
//! provides exactly (`memcpy`, `strlen`, `qsort`) are not ported; see `ports.toml`
//! (`skipped: provided-by-core`). This crate has no dependencies and must stay that way.
//!
//! String arguments are byte slices: a C string ends at its first NUL or at the end of the
//! slice, whichever comes first, and a destination's size is its slice length.
//!
//! `libkern.h`'s prototypes are the re-exports below; its `imax`/`min`/`abs` family is
//! `Ord::max`, `Ord::min` and `i32::abs`; `KASSERT`/`KDASSERT` are `kassert!`/`kdassert!` in
//! `sys/kern/subr_prf.rs`, next to the `__assert` they call (this crate cannot call into `bsd`).
//!
//! Decision 55 (the user, 2026-11-26): this crate holds no `unsafe` and says so (`forbid` below).
//! `explicit_bzero` is `hal::mem::explicit_bzero`, and `StaticCell`, the one project helper
//! that was here, is `bsd`'s (`sys/sys/staticcell.rs`, until N9) and `libsa`'s (the boot
//! loaders').

#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
extern crate std;

#[forbid(unsafe_code)]
pub mod hash;
#[forbid(unsafe_code)]
pub mod mem;
#[forbid(unsafe_code)]
pub mod rand;
#[forbid(unsafe_code)]
pub mod string;

// The one function at the root: the inherited `sys/net/pf_osfp.rs` (byte-identical to LZ's,
// `lineage.toml` `inherited`) calls `libkern::strnlen`; no file of ours does. Remove it with
// that file.
pub use string::strnlen::strnlen;
/* </CODE> */
