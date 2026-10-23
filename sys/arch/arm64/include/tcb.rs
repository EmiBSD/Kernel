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

/*
 * Copyright (c) 2011 Philip Guenther <guenther@openbsd.org>
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
//! arm64 `<machine/tcb.h>`: the thread control block, which user space reaches through
//! `TPIDR_EL0`.
//!
//! Upstream: sys/arch/arm64/include/tcb.h @ 3ce1f3f79392
//!
//! Status: `ported` (the kernel half). N4.8b: `hal`'s (`hal::arch::arm64::include::tcb`
//! and `hal::thread::tcb_get`/`tcb_set`, with the pcb they read); the names stay here for
//! `bsd`'s callers (exec's `TCB_SET(p, 0)`, the `Tcb` trait).
//!
//! ## Deviations
//! - The macros `TCB_GET(p)`/`TCB_SET(p, addr)` are the functions `tcb_get`/`tcb_set`; the
//!   TCB is a `usize` (a user address the kernel never dereferences) instead of a `void *`.
//! - The userland half (`TLS_VARIANT`, `__aarch64_read_tcb`, the userland `TCB_GET()`) is not
//!   kernel code and is not here.

use crate::sys::proc::Proc;

pub use hal::arch::arm64::include::tcb::__aarch64_set_tcb;

/// `TCB_GET(p)`: `p`'s saved TCB address (`hal::thread::tcb_get`).
#[inline]
pub fn tcb_get(p: &Proc) -> usize {
    hal::thread::tcb_get(p)
}

/// `TCB_SET(p, addr)`: records `addr` in `p`'s pcb and loads it into `TPIDR_EL0` (`p` is the
/// running thread; `hal::thread::tcb_set`).
#[inline]
pub fn tcb_set(p: &Proc, addr: usize) {
    hal::thread::tcb_set(p, addr);
}
/* </CODE> */
