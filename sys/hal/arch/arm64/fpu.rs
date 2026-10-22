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
//! arm64 `fpu.c`: the floating point and SVE state of a thread, as the context switch, the
//! FP trap, signal delivery, `cpu_fork` and efi(4) need it.
//!
//! Upstream: sys/arch/arm64/arm64/fpu.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/fpu.rs@3c62ede68802
//!
//! ## Deviations
//! - The kernel is built for `aarch64-unknown-none-softfloat`: the register moves are
//!   inline assembly bracketed by `.arch_extension fp`/`nofp`, as the C's `.arch
//!   armv8-a+fp`, and `fpsr`/`fpcr` are reached by their encodings (`s3_3_c4_c4_1`,
//!   `s3_3_c4_c4_0`), as the C's `#define`s do.
//! - SVE is never enabled (`CPACR_ZEN` stays trapped), so `sve_save` is reached only through
//!   a state this kernel cannot create; it is reported (`unported: sve_save: ...`, once), and
//!   so is `sve_load` (the `EXCP_SVE` trap).
//!
//! ## Redesign
//! - LZ's `fpu_save(p)`/`fpu_load(p)` took any thread and reached its pcb through `p_addr`;
//!   the FP registers they move are the running CPU's, so only the running thread's pcb is
//!   ever right. Here they act on the running thread's pcb alone ([`save_current`],
//!   [`load_current`], `ci_curpcb`, which the switch keeps), and the switch saves the old
//!   thread's state itself before it leaves it (`hal::thread::switch_to`, N4.5a): no caller
//!   can name another thread's pcb, whose state another CPU may be moving.

use core::arch::asm;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::arm64::include::pcb::{PCB_FPU, Pcb};
use crate::arch::arm64::include::reg::Fpreg;
use crate::arch::arm64::switch::with_curpcb;
use crate::assert::kassert;

/// `CPACR_EL1.FPEN`: the FP/SIMD trap field.
const CPACR_FPEN_MASK: u64 = 0x3 << 20;
/// `CPACR_FPEN_TRAP_ALL1`: EL0 and EL1 trap.
const CPACR_FPEN_TRAP_ALL1: u64 = 0x0 << 20;
/// `CPACR_FPEN_TRAP_EL0`: EL0 traps, EL1 does not.
const CPACR_FPEN_TRAP_EL0: u64 = 0x1 << 20;
/// `CPACR_FPEN_TRAP_NONE`: neither traps.
const CPACR_FPEN_TRAP_NONE: u64 = 0x3 << 20;
/// `CPACR_EL1.ZEN`: the SVE trap field.
const CPACR_ZEN_MASK: u64 = 0x3 << 16;
/// `CPACR_ZEN_TRAP_ALL1`: EL0 and EL1 trap.
const CPACR_ZEN_TRAP_ALL1: u64 = 0x0 << 16;
/// `CPACR_ZEN_TRAP_NONE`: neither traps.
const CPACR_ZEN_TRAP_NONE: u64 = 0x3 << 16;

/// Reads `CPACR_EL1`.
#[inline]
fn cpacr() -> u64 {
    let v: u64;
    // SAFETY: `mrs` of `CPACR_EL1` reads a system register: no memory, no stack, no flags.
    unsafe { asm!("mrs {}, cpacr_el1", out(reg) v, options(nomem, nostack, preserves_flags)) };
    v
}

/// Writes `CPACR_EL1` (which of EL0 and EL1 trap FP/SIMD and SVE), with an `isb` after it
/// when `sync` (an FP move of this CPU follows), without one otherwise (the C's: returning to
/// EL0 is a context synchronization event).
#[inline]
fn set_cpacr(v: u64, sync: bool) {
    // SAFETY: `CPACR_EL1` only decides which FP/SIMD and SVE instructions trap; the kernel is
    // built without floating point (softfloat target), so its own code never executes one, and
    // the moves of this module run with the FPU enabled or not as they need (`sync` orders the
    // change before them). No memory, no stack, no flags.
    unsafe {
        if sync {
            asm!(
                "msr cpacr_el1, {}",
                "isb",
                in(reg) v,
                options(nomem, nostack, preserves_flags)
            );
        } else {
            asm!("msr cpacr_el1, {}", in(reg) v, options(nomem, nostack, preserves_flags));
        }
    }
}

/// `sve_save`/`sve_load`'s report (see the module's deviations): `unported: <what>`, once.
fn report_sve(what: &str, once: &AtomicBool) {
    if !once.swap(true, Ordering::Relaxed) {
        crate::prf::printf(format_args!("unported: {what}\n"));
    }
}

/// `fpu_save(curproc)`: the running thread's FP (or SVE) registers into its pcb, if the FPU
/// is enabled on this CPU (`cpu_fork` of the parent, `sendsig`, the switch away).
pub fn save_current() {
    with_curpcb(|pcb| {
        if let Some(pcb) = pcb {
            save(pcb);
        }
    });
}

/// `fpu_save` of the running thread whose pcb is `pcb` (the switch's old thread, before it
/// leaves this CPU).
pub(crate) fn save(pcb: &Pcb) {
    let cpacr = cpacr();
    if cpacr & CPACR_FPEN_MASK == CPACR_FPEN_TRAP_ALL1 {
        return;
    }

    kassert!(cpacr & CPACR_FPEN_MASK == CPACR_FPEN_TRAP_NONE);

    if cpacr & CPACR_ZEN_MASK == CPACR_ZEN_TRAP_NONE {
        static ONCE: AtomicBool = AtomicBool::new(false);
        report_sve("sve_save: the SVE register block", &ONCE);
        return;
    }

    let fp = pcb.pcb_fpstate.get();
    let fpsr: u64;
    let fpcr: u64;
    // SAFETY: the FPU is enabled on this CPU (checked above), so the vector registers and
    // `fpsr`/`fpcr` hold the running thread's state; `pcb` is that thread's pcb (the callers:
    // `ci_curpcb`, or the switch's old thread, still running here), and its FP block is
    // written only by that thread or the switch away from it, both on this CPU, never at once
    // with anything else: the stores write the 32 16-byte slots of `fp_reg` alone, and
    // `fp_sr`/`fp_cr` after. No stack, no flags.
    unsafe {
        asm!(
            ".arch_extension fp",
            "str q0, [{fp}, #0]",
            "str q1, [{fp}, #16]",
            "str q2, [{fp}, #32]",
            "str q3, [{fp}, #48]",
            "str q4, [{fp}, #64]",
            "str q5, [{fp}, #80]",
            "str q6, [{fp}, #96]",
            "str q7, [{fp}, #112]",
            "str q8, [{fp}, #128]",
            "str q9, [{fp}, #144]",
            "str q10, [{fp}, #160]",
            "str q11, [{fp}, #176]",
            "str q12, [{fp}, #192]",
            "str q13, [{fp}, #208]",
            "str q14, [{fp}, #224]",
            "str q15, [{fp}, #240]",
            "str q16, [{fp}, #256]",
            "str q17, [{fp}, #272]",
            "str q18, [{fp}, #288]",
            "str q19, [{fp}, #304]",
            "str q20, [{fp}, #320]",
            "str q21, [{fp}, #336]",
            "str q22, [{fp}, #352]",
            "str q23, [{fp}, #368]",
            "str q24, [{fp}, #384]",
            "str q25, [{fp}, #400]",
            "str q26, [{fp}, #416]",
            "str q27, [{fp}, #432]",
            "str q28, [{fp}, #448]",
            "str q29, [{fp}, #464]",
            "str q30, [{fp}, #480]",
            "str q31, [{fp}, #496]",
            "mrs {sr}, s3_3_c4_c4_1",
            "mrs {cr}, s3_3_c4_c4_0",
            ".arch_extension nofp",
            fp = in(reg) ptr::addr_of_mut!((*fp).fp_reg),
            sr = out(reg) fpsr,
            cr = out(reg) fpcr,
            options(nostack, preserves_flags)
        );
        (*fp).fp_sr = fpsr as u32;
        (*fp).fp_cr = fpcr as u32;
    }
}

/// `fpu_load(curproc)`: the `EXCP_FP_SIMD`/`EXCP_TRAP_FP` trap of the running thread's first
/// FP instruction since its last switch: its state loaded from its pcb (zeroed the first
/// time, `PCB_FPU` set) and the FPU enabled.
pub fn load_current() {
    with_curpcb(|pcb| {
        let Some(pcb) = pcb else {
            crate::assert::hal_panic(format_args!("fpu_load: no running thread"));
        };
        load(pcb);
    });
}

/// `fpu_load` of the running thread whose pcb is `pcb`.
fn load(pcb: &Pcb) {
    let fp = pcb.pcb_fpstate.get();

    let mut cpacr = cpacr();
    kassert!(cpacr & CPACR_FPEN_MASK == CPACR_FPEN_TRAP_ALL1);
    kassert!(cpacr & CPACR_ZEN_MASK == CPACR_ZEN_TRAP_ALL1);

    if pcb.pcb_flags.get() & PCB_FPU == 0 {
        // SAFETY: the running thread's own FP block (`ci_curpcb`), which only it or its
        // switch away write, both on this CPU: nothing else touches it now.
        unsafe { fp.write(Fpreg::zeroed()) };
        pcb.pcb_flags.set(pcb.pcb_flags.get() | PCB_FPU);
    }

    // Enable FPU.
    cpacr &= !CPACR_FPEN_MASK;
    cpacr |= CPACR_FPEN_TRAP_NONE;
    set_cpacr(cpacr, true);

    // SAFETY: the FPU is now enabled on this CPU; `fp` is the running thread's own FP block
    // (as above), read here alone. The loads overwrite the vector registers and `fpsr`/
    // `fpcr`, which no kernel code uses (softfloat target), so nothing the compiler keeps is
    // clobbered. No stack, no flags.
    unsafe {
        asm!(
            ".arch_extension fp",
            "ldr q0, [{fp}, #0]",
            "ldr q1, [{fp}, #16]",
            "ldr q2, [{fp}, #32]",
            "ldr q3, [{fp}, #48]",
            "ldr q4, [{fp}, #64]",
            "ldr q5, [{fp}, #80]",
            "ldr q6, [{fp}, #96]",
            "ldr q7, [{fp}, #112]",
            "ldr q8, [{fp}, #128]",
            "ldr q9, [{fp}, #144]",
            "ldr q10, [{fp}, #160]",
            "ldr q11, [{fp}, #176]",
            "ldr q12, [{fp}, #192]",
            "ldr q13, [{fp}, #208]",
            "ldr q14, [{fp}, #224]",
            "ldr q15, [{fp}, #240]",
            "ldr q16, [{fp}, #256]",
            "ldr q17, [{fp}, #272]",
            "ldr q18, [{fp}, #288]",
            "ldr q19, [{fp}, #304]",
            "ldr q20, [{fp}, #320]",
            "ldr q21, [{fp}, #336]",
            "ldr q22, [{fp}, #352]",
            "ldr q23, [{fp}, #368]",
            "ldr q24, [{fp}, #384]",
            "ldr q25, [{fp}, #400]",
            "ldr q26, [{fp}, #416]",
            "ldr q27, [{fp}, #432]",
            "ldr q28, [{fp}, #448]",
            "ldr q29, [{fp}, #464]",
            "ldr q30, [{fp}, #480]",
            "ldr q31, [{fp}, #496]",
            "msr s3_3_c4_c4_1, {sr}",
            "msr s3_3_c4_c4_0, {cr}",
            ".arch_extension nofp",
            fp = in(reg) ptr::addr_of!((*fp).fp_reg),
            sr = in(reg) u64::from((*fp).fp_sr),
            cr = in(reg) u64::from((*fp).fp_cr),
            options(nostack, readonly, preserves_flags)
        );
    }
}

/// `fpu_drop`: disable FPU and SVE (traps from EL0 and EL1).
pub fn drop_state() {
    let mut cpacr = cpacr();
    cpacr &= !(CPACR_FPEN_MASK | CPACR_ZEN_MASK);
    cpacr |= CPACR_FPEN_TRAP_ALL1 | CPACR_ZEN_TRAP_ALL1;
    // No ISB instruction needed here, as returning to EL0 is a context synchronization
    // event.
    set_cpacr(cpacr, false);
}

/// `fpu_kernel_enter`: lets EL1 use the FPU (for the UEFI runtime services, which may),
/// saving the running thread's FP state first.
pub fn kernel_enter() {
    with_curpcb(|pcb| {
        if let Some(pcb) = pcb
            && pcb.pcb_flags.get() & PCB_FPU != 0
        {
            save(pcb);
        }
    });

    // Enable FPU (kernel only).
    let mut cpacr = cpacr();
    cpacr &= !(CPACR_FPEN_MASK | CPACR_ZEN_MASK);
    cpacr |= CPACR_FPEN_TRAP_EL0 | CPACR_ZEN_TRAP_ALL1;
    set_cpacr(cpacr, true);
}

/// `fpu_kernel_exit`: traps FP use from EL0 and EL1 again.
pub fn kernel_exit() {
    drop_state();
}

/// `sve_load`: the `EXCP_SVE` trap's handler (see the module's deviations).
pub fn sve_load_current() {
    static ONCE: AtomicBool = AtomicBool::new(false);
    report_sve("sve_load: the SVE register block", &ONCE);
}
/* </CODE> */
