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
// Typed memory stolen at boot over the pmap's `steal_memory` (N3.3): wherever the pmap is.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
pub mod bootmem;
pub mod bridges;
pub mod cons;
#[forbid(unsafe_code)]
pub mod cpu;
pub mod intr;
#[forbid(unsafe_code)]
pub mod klock;
pub mod kmbuf;
pub mod link;
#[forbid(unsafe_code)]
pub mod list;
pub mod malloc;
// The byte view of frames mapped for I/O over the pmap (N3.4a): wherever the pmap is.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
pub mod mapped;
pub mod mutex;
#[forbid(unsafe_code)]
pub mod netlock;
pub mod once;
pub mod owned;
pub mod percpu;
pub mod pin;
pub mod pinlist;
// pmap(9) (N3.1). A machine's build has the module once its backend is in
// `arch/<machine>/pmap/`: the host double's, amd64's (N3.1 commit 4) and arm64's (commit 5),
// as the two pmap bridges of `bridges.rs`.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
#[forbid(unsafe_code)]
pub mod pmap;

pub mod pool;
#[forbid(unsafe_code)]
pub mod prf;
// arc4random(9) over the pmap's `Random` bridge (N3.5): wherever that bridge is.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
#[forbid(unsafe_code)]
pub mod random;
pub mod refcnt;
pub mod rwlock;
pub mod seqlock;
pub mod sleep;
pub mod slot;
#[forbid(unsafe_code)]
pub mod softintr;
#[cfg(test)]
pub(crate) mod testutil;
pub mod thread;
#[forbid(unsafe_code)]
pub mod tree;
pub(crate) mod trust;
pub mod user;
pub mod vm;
/* </CODE> */
