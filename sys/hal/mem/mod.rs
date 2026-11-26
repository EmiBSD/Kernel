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
//! Kernel memory: allocators, typed pools, owned and mapped byte views (hal's groups, 2026-11-26).

pub mod kmbuf;
pub mod malloc;
pub mod pool;
// The byte view of frames mapped for I/O over the pmap (N3.4a): wherever the pmap is.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
pub mod mapped;
// Typed memory stolen at boot over the pmap's `steal_memory` (N3.3): wherever the pmap is.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
pub mod bootmem;
pub mod explicit_bzero;
// The bootloader's device tree as bytes (N7.2b0): machine-independent, unused on amd64.
pub mod fdtblob;
pub mod overlay;
pub mod pod;
pub(crate) mod trust;

pub use explicit_bzero::explicit_bzero;
/* </CODE> */
