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
//! arm64's devices in the kernel: the interrupt core (`arch/arm64/arm64/intr.c`, N7.2a1); the
//! device tree's attach arguments and its buses, `mainbus(4)` and `simplebus(4)` (N7.2b1b);
//! the GICv2, `ampintc(4)` and its GICv2m frames (N7.2a2); efi(4)'s runtime services and
//! time-of-day chip (`efi_machdep`, N7.2b2a); the GICv3 follows (N7.2a3).

pub mod ampintc;
pub mod efi_machdep;
pub mod fdt;
pub mod intr;
pub mod mainbus;
pub mod simplebus;
/* </CODE> */
