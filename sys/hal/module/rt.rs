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
//! The module's runtime: what `abi_openbsd.ko`'s binary needs that its own crates may not
//! hold, its panic handler. Compiled in only with feature `module-rt`, which `sys/abi/ko`
//! alone sets, and only for the bare targets; the kernel's binary has its own (`bsd`'s
//! panic(9)).
//!
//! A panic in the module has no `KernelApi` in hand (the module keeps no state, so it has
//! no way to reach the kernel's console or panic(9)): the handler parks the processor. The
//! module's code has no panicking path by construction (entries through `get`, no
//! `unwrap`), so this is the path of a bug in it; under QEMU the hang is what the smokes'
//! watchdog reports.

/// The module's `#[panic_handler]`.
#[cfg(all(feature = "module-rt", target_os = "none"))]
#[panic_handler]
fn module_panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
/* </CODE> */
