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
//! arm64 machine code moved into the HAL.

pub(crate) mod cpu;
// The context switch and the FP state (N4.5a): the kernel's alone, not the ABI module's
// runtime (`module-rt`), whose image links no thread code (the switch's assembly calls two
// symbols of `bsd`'s, `switch.rs`).
#[cfg(not(feature = "module-rt"))]
pub mod fpu;
pub mod include;
pub mod pmap;
#[cfg(not(feature = "module-rt"))]
pub mod switch;

/// `<machine/param.h>` and `<machine/vmparam.h>` as `hal::arch::current` names them.
pub use self::include::{param, vmparam};
/* </CODE> */
