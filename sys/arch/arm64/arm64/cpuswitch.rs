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
 * Copyright (c) 2008-2009,2014-2016 Dale Rahn <drahn@dalerahn.com>
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
//! The `pmap_setttb` entry the context switch's assembly calls: the switch itself, its
//! assembly (`cpuswitch.S`) and `proc_trampoline` are `hal`'s since N4.5a
//! (`hal::arch::arm64::switch`).
//!
//! Upstream: sys/arch/arm64/arm64/cpuswitch.S @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/cpuswitch.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//!
//! ## Redesign
//! - `pmap_setttb` (LZ `arm64/pmap.rs`, whose pmap moved into `hal` at N3.1) is only the
//!   C-named entry the switch's assembly branches to: it turns the `struct proc *` into a
//!   `&Proc` and calls `machine::pmap::pmap_setttb`, where the ASID check, the empty table
//!   first and `cpu_setttb` are `hal`'s (`hal::pmap`, the arm64 backend's `load`) and the
//!   `ci_curpm` record and the branch-predictor flush are `pmap_switched_md`'s. It is an
//!   `unsafe extern "C" fn`: its one caller is the assembly, and a safe signature would let
//!   any Rust caller pass a dangling pointer.
//! - N4.5a: `cpu_switchto_asm`, `proc_trampoline` and `proc_trampoline_run` moved into `hal`
//!   (`hal_cpu_switchto`, `hal_proc_trampoline`, `trampoline_run`), with `cpuswitch.S`; the
//!   switch still calls this entry by its name, with the thread it just made `ci_curproc`.

use core::ffi::c_void;

use crate::sys::proc::Proc;

/// `pmap_setttb(p)`: loads `p`'s address space into `TTBR0_EL1` (the ASID check, the empty
/// table first, `cpu_setttb`, then `ci_curpm` and the branch-predictor flush:
/// `machine::pmap::pmap_setttb` over `hal::pmap`), for every switch `hal_cpu_switchto` makes
/// (hence `extern "C"` by its C name, N3.1: moved here from LZ's `arm64/pmap.rs`).
///
/// # Safety
///
/// `p` is a live `struct proc *` (a `Proc`, opaque to the ABI) that stays alive for the
/// call: the thread being switched to, `hal_cpu_switchto`'s `newproc`, which the switch's
/// caller keeps alive (`hal::thread`'s E3). Only `hal_cpu_switchto` calls this.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmap_setttb(p: *const c_void) {
    // SAFETY: the caller's guarantee: `p` points to a live `Proc` for the whole call; a
    // `Proc` is only ever shared (`&Proc`, its mutable state in cells), so a shared reference
    // for the call's duration aliases nothing exclusive.
    let p = unsafe { &*p.cast::<Proc>() };
    crate::machine::pmap::pmap_setttb(p);
}
/* </CODE> */
