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
//! The context switch of `arch/arm64/arm64/cpuswitch.S`, pulled in from the `.S` file next
//! to this module (the file keeps OpenBSD's licence block and layout; `{NAME}` placeholders
//! are what `assym.h` provides in C), and the `pmap_setttb` entry the assembly calls.
//!
//! Upstream: sys/arch/arm64/arm64/cpuswitch.S @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/cpuswitch.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//!
//! Status: `ported` (M5): `cpu_switchto_asm` and `proc_trampoline`. `machdep.rs` wraps the
//! former as `cpu_switchto` (the FPU state goes with the old thread there).
//!
//! ## Deviations
//! - No retguard: the cookie load/calc/check around the stack pivot has no counterpart in
//!   this kernel yet.
//! - `proc_trampoline` calls `proc_trampoline_run` (Rust) with the function and argument
//!   instead of `blr`ing the function itself: Rust `fn` pointers have no C calling
//!   convention. After the function returns both go to `syscall_return`.
//!
//! ## Redesign
//! - `pmap_setttb` (LZ `arm64/pmap.rs`, whose pmap moved into `hal` at N3.1) is only the
//!   C-named entry `cpu_switchto_asm` branches to: it turns the `struct proc *` into a
//!   `&Proc` and calls `machine::pmap::pmap_setttb`, where the ASID check, the empty table
//!   first and `cpu_setttb` are `hal`'s (`hal::pmap`, the arm64 backend's `load`) and the
//!   `ci_curpm` record and the branch-predictor flush are `pmap_switched_md`'s. It is an
//!   `unsafe extern "C" fn`: its one caller is the assembly, and a safe signature would let
//!   any Rust caller pass a dangling pointer.

use core::arch::global_asm;
use core::ffi::c_void;
use core::mem::offset_of;

use crate::arch::arm64::include::cpu::CpuInfo;
use crate::arch::arm64::include::frame::{SWITCHFRAME_SZ, Switchframe};
use crate::arch::arm64::include::pcb::Pcb;
use crate::kern::kern_fork::proc_trampoline_mi;
use crate::sys::proc::{Proc, SONPROC};

global_asm!(
    include_str!("cpuswitch.S"),
    SWITCHFRAME_SZ = const SWITCHFRAME_SZ,
    SF_X19 = const offset_of!(Switchframe, sf_x19),
    SF_X21 = const offset_of!(Switchframe, sf_x21),
    SF_X23 = const offset_of!(Switchframe, sf_x23),
    SF_X25 = const offset_of!(Switchframe, sf_x25),
    SF_X27 = const offset_of!(Switchframe, sf_x27),
    SF_X29 = const offset_of!(Switchframe, sf_x29),
    CI_CURPCB = const offset_of!(CpuInfo, ci_curpcb),
    CI_CURPROC = const offset_of!(CpuInfo, ci_curproc),
    P_STAT = const offset_of!(Proc, p_stat),
    P_CPU = const offset_of!(Proc, p_cpu),
    P_ADDR = const offset_of!(Proc, p_addr),
    PCB_SP = const offset_of!(Pcb, pcb_sp),
    PCB_TCB = const offset_of!(Pcb, pcb_tcb),
    SONPROC = const SONPROC,
);

unsafe extern "C" {
    /// `cpu_switchto_asm(old, new)`: the switch itself (`machine::cpu::Cpu::cpu_switchto`
    /// states the contract; `machdep::cpu_switchto` is the entry point). Both are `struct
    /// proc *` (opaque to the ABI: `Proc` has Rust layout, the assembly reads it by
    /// `offset_of!`).
    pub fn cpu_switchto_asm(old: *const c_void, new: *const c_void);
    /// `proc_trampoline`: the first instructions of a thread built by `cpu_fork`.
    pub fn proc_trampoline();
}

/// What `proc_trampoline` calls with the switch frame's `sf_x19`/`sf_x20`: the
/// machine-independent start of a thread, then its function. The function never returns for
/// a kernel thread; a user thread's (`child_return`, or `start_init` after its exec) does,
/// and the assembly then takes `syscall_return` to user mode.
///
/// # Safety
///
/// Only `proc_trampoline` calls this, on a thread `cpu_fork` built: `func` is the
/// `fn(*mut c_void)` it stored in the switch frame, as a pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_trampoline_run(func: *const (), arg: *mut c_void) {
    proc_trampoline_mi();
    // SAFETY: the caller's guarantee: `cpu_fork` stored a `fn(*mut c_void)` in `sf_x19` as a
    // pointer; this is the inverse cast.
    let func: fn(*mut c_void) = unsafe { core::mem::transmute::<*const (), fn(*mut c_void)>(func) };
    func(arg);
}

/// `pmap_setttb(p)`: loads `p`'s address space into `TTBR0_EL1` (the ASID check, the empty
/// table first, `cpu_setttb`, then `ci_curpm` and the branch-predictor flush:
/// `machine::pmap::pmap_setttb` over `hal::pmap`), for every switch `cpu_switchto_asm` makes
/// (hence `extern "C"` by its C name, N3.1: moved here from LZ's `arm64/pmap.rs`).
///
/// # Safety
///
/// `p` is a live `struct proc *` (a `Proc`, opaque to the ABI) that stays alive for the
/// call: the thread being switched to, `cpu_switchto_asm`'s `new`, which the caller of
/// `cpu_switchto` keeps alive. Only `cpu_switchto_asm` calls this.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmap_setttb(p: *const c_void) {
    // SAFETY: the caller's guarantee: `p` points to a live `Proc` for the whole call; a
    // `Proc` is only ever shared (`&Proc`, its mutable state in cells), so a shared reference
    // for the call's duration aliases nothing exclusive.
    let p = unsafe { &*p.cast::<Proc>() };
    crate::machine::pmap::pmap_setttb(p);
}

/* </CODE> */
