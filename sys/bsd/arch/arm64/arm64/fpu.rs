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
 * Copyright (c) 2022 Mark Kettenis <kettenis@openbsd.org>
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
//! arm64 `fpu.c`: the floating point and SVE state of a thread.
//!
//! Upstream: sys/arch/arm64/arm64/fpu.c @ 3ce1f3f79392
//!
//! Status: an adapter (N4.5a). The register moves, the `CPACR_EL1` writes and the SVE
//! reports are `hal`'s (`hal::arch::arm64::fpu`), which act on the running thread's pcb
//! alone (the FP registers are the running CPU's); these keep LZ's names and signatures for
//! the trap, signal and efi(4) code, whose thread is always `curproc`.

use crate::kassert;
use crate::machine::cpu::curproc;
use crate::sys::proc::Proc;

/// Whether `p` is the running thread (the only one whose FP state is in the CPU).
fn is_curproc(p: &Proc) -> bool {
    curproc().is_some_and(|c| core::ptr::eq(c, p))
}

/// `fpu_save`: saves `p`'s FP (or SVE) registers into its pcb, if the FPU is enabled; `p` is
/// the running thread.
pub fn fpu_save(p: &Proc) {
    kassert!(is_curproc(p));
    hal::arch::arm64::fpu::save_current();
}

/// `fpu_load`: loads `p`'s FP registers from its pcb and enables the FPU (the
/// `EXCP_FP_SIMD`/`EXCP_TRAP_FP` trap of a thread's first FP instruction since its last
/// switch); `p` is the running thread.
pub fn fpu_load(p: &Proc) {
    kassert!(is_curproc(p));
    hal::arch::arm64::fpu::load_current();
}

/// `fpu_drop`: disable FPU and SVE.
pub fn fpu_drop() {
    hal::arch::arm64::fpu::drop_state();
}

/// `fpu_kernel_enter`: lets EL1 use the FPU (for the UEFI runtime services, which may),
/// saving the current thread's FP state first.
pub fn fpu_kernel_enter() {
    hal::arch::arm64::fpu::kernel_enter();
}

/// `fpu_kernel_exit`: traps FP use from EL0 and EL1 again.
pub fn fpu_kernel_exit() {
    hal::arch::arm64::fpu::kernel_exit();
}

/// `sve_load`: the `EXCP_SVE` trap's handler (reported, `hal`'s).
pub fn sve_load(p: &Proc) {
    kassert!(is_curproc(p));
    hal::arch::arm64::fpu::sve_load_current();
}
/* </CODE> */
