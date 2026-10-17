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
pub mod bridges;
pub mod cons;
pub mod cpu;
pub mod intr;
pub mod kmbuf;
pub mod malloc;
pub mod mutex;
pub mod once;
pub mod percpu;
// pmap(9) (N3.1). A machine's build has the module once its backend is in
// `arch/<machine>/pmap/` (amd64: N3.1 commit 4, arm64: commit 5, each adding its `target_arch`
// here and on the two pmap bridges of `bridges.rs`); the host double has one now.
#[cfg(not(target_os = "none"))]
#[forbid(unsafe_code)]
pub mod pmap;

pub mod pool;
pub mod refcnt;
pub mod rwlock;
pub mod sleep;
#[cfg(test)]
pub(crate) mod testutil;
pub(crate) mod trust;
pub mod user;
pub mod vm;
/* </CODE> */
