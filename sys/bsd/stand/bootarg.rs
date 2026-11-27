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
//! The kernel's entry from OpenBSD's own boot loader (boot(8)/efiboot, M14), its only one
//! since the removal of Limine (2026-11-26).
//!
//! The machine's entry code runs first: on amd64 `locore0.S`'s `start`, which boot(8) enters
//! in 32-bit protected mode with its arguments (`howto`, `bootdev`, `bootapiver`, `esym`, the
//! `bootarg` list), copies the list into the kernel, builds the bootstrap page tables, enters
//! long mode and calls [`bootarg_main`] on the kernel's boot stack; on arm64 `locore0.S`'s
//! `_start`, which efiboot enters with the device tree. The machine turns what boot(8)
//! passed into a `BootInfo` (`Machine::getbootinfo`) and [`super::start_kernel`] does the
//! rest.

use bsd::machine::{Cpu, Exit, ExitStatus, Machine};

/// Called by the machine's entry code (amd64: `locore0.S`, after `longmode_hi`) with the
/// argument it hands `Machine::getbootinfo`. Never returns.
///
/// # Safety
///
/// Called once, on the boot CPU, by the machine's own entry code, on the kernel's boot stack
/// with the bootstrap page tables loaded and the frame pointer zeroed (the end of `ddb`'s
/// frame chain).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bootarg_main(arg: usize) -> ! {
    // SAFETY: the caller's guarantee: once, from the entry code, before anything else.
    match unsafe { Machine::getbootinfo(arg) } {
        // SAFETY: once, on the boot CPU; `boot` describes the image boot(8) loaded.
        Ok(boot) => unsafe { super::start_kernel(boot) },
        // No console yet: the failure exit status is the only trace.
        Err(_unprintable) => Machine::exit(ExitStatus::Failure),
    }
}
/* </CODE> */
