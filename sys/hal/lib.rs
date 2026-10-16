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
//! The HAL: the hardware (`arch/amd64`, `arch/arm64`, `machine`, moved in at N2.2) and the
//! runtime written new (locks that own their data, typed pools, owned memory, lists and
//! trees, the typed user copy, the bridge registry, the module loader). The only crate of the
//! system where `unsafe` may appear; every site carries a soundness proof and `cargo xtask
//! hal-check` refuses a `pub unsafe fn`: the API is safe (docs/ZERO_UNSAFE.md, section 0).
#![no_std]

// The host tests link std; a kernel image never does.
#[cfg(test)]
extern crate std;

pub(crate) mod assert;
pub mod bridges;
pub mod cons;
pub mod cpu;
pub mod errno;
pub mod intr;
pub mod kmbuf;
pub mod list;
pub mod malloc;
pub mod mutex;
pub mod once;
pub mod percpu;
pub mod pool;
pub mod rbtree;
pub mod refcnt;
pub mod rwlock;
pub mod sleep;
#[cfg(test)]
pub(crate) mod testutil;
pub mod tree;
pub(crate) mod trust;
pub mod user;
/* </CODE> */
