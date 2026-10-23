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
//! `TPIDR_EL0`; the kernel half.
//!
//! Upstream: sys/arch/arm64/include/tcb.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/tcb.rs@3c62ede68802
//!
//! The TCB is a user address the kernel never follows, saved in the thread's pcb
//! (`pcb_tcb`); `cpu_switchto` (`cpuswitch.S`) loads it into `TPIDR_EL0` for the thread it
//! switches to, and `TCB_SET` loads it at once for the running thread.
//!
//! ## Deviations
//! - The userland half (`TLS_VARIANT`, `__aarch64_read_tcb`, the userland `TCB_GET()`) is not
//!   kernel code and is not here.
//!
//! ## Redesign
//! - N4.8b: into `hal` with the pcb it reads. LZ's `TCB_GET(p)` and `TCB_SET(p, addr)`
//!   (`bsd`'s `tcb_get`/`tcb_set` over `p.pcb()`) are `hal::thread::tcb_get` and `tcb_set`
//!   (the pcb through the switch's `with_pcb_of`, another thread's read under the
//!   scheduler's lock, which the setter takes too); here is the register's half,
//!   [`__aarch64_set_tcb`], which `TCB_SET` calls. arm64's header has no `TCB_INVALID`:
//!   [`tcb_invalid`] accepts every address.

use core::arch::asm;

/// `__aarch64_set_tcb(tcb)`: loads the user thread pointer register.
#[inline]
pub fn __aarch64_set_tcb(tcb: usize) {
    // SAFETY: `TPIDR_EL0` is the user's thread pointer: the kernel never reads it (it keeps
    // `curcpu` in `TPIDR_EL1`), so writing any value changes only what user space sees, and the
    // switch rewrites it from the next thread's pcb. The `msr` touches no memory, no stack and
    // no flags.
    unsafe { asm!("msr tpidr_el0, {}", in(reg) tcb, options(nomem, nostack, preserves_flags)) };
}

/// `TCB_INVALID(addr)`: arm64's header defines none, so no address is refused.
pub const fn tcb_invalid(_addr: usize) -> bool {
    false
}
/* </CODE> */
