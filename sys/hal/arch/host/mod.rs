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
//! The host double of `hal`'s machine code. Not an OpenBSD architecture.
//!
//! Compiled whenever `target_os != "none"`, so that `hal`'s host tests and `bsd`'s host build
//! (`cargo test`, on macOS or Linux) have a machine under `hal::arch::current`. It may use
//! `std` (rust-kernel.md: the host doubles); it must not grow logic: behaviour belongs in the
//! generic code it stands under. Its page geometry and address-space bounds are amd64's
//! (`param`, which also serves as `vmparam`); its pmap (`pmap`, behind `hal::pmap`) is a
//! `BTreeMap` of mappings per pmap, as `bsd`'s host double's was.
//!
//! The constant headers of the two machines are compiled here for their host tests only
//! (docs/ARCHITECTURE.md, "Host tests of arch code"): `sys/hal/arch/<arch>/` is built for
//! `target_os = "none"` alone, and the tests of a header are plain constant checks, five of
//! them against the C under `just test-ref`. arm64's `pte.rs` is inherited from LZ and names
//! `crate::arch::arm64::include::param`: on the host that path is an alias of these headers
//! (`sys/hal/arch/mod.rs`), so `pte.rs` compiles unchanged for the arm64 layout's leaf tests
//! (N3.1 commit 8b). The two pmap layouts (`pmap/layout.rs`, pure constants) are compiled
//! with them.

pub(crate) mod cpu;
pub(crate) mod klock;
pub mod param;
pub(crate) mod pmap;
#[cfg(not(feature = "module-rt"))]
pub(crate) mod switch;
pub mod timer;

pub use self::param as vmparam;
/// The host pmap's test knob (its only one): a failing `pmap_enter`.
pub use self::pmap::fail_enters;

/// amd64's constant headers in their own layout (`include/`), compiled for their tests only.
#[cfg(test)]
#[path = "../amd64"]
#[allow(dead_code)] // compiled for its tests: what reads the constants is built for amd64 only
mod amd64_headers {
    pub mod include {
        pub mod param;
        pub mod pte;
        pub mod vmparam;
    }
    pub mod pmap {
        pub mod layout;
    }
}

/// arm64's constant headers in their own layout (`include/`), compiled for their tests only.
#[cfg(test)]
#[path = "../arm64"]
#[allow(dead_code)] // compiled for its tests: what reads the constants is built for arm64 only
pub(crate) mod arm64_headers {
    pub mod include {
        pub mod param;
        pub mod pte;
        pub mod vmparam;
    }
    pub mod pmap {
        pub mod layout;
    }
}
/* </CODE> */
