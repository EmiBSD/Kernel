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
 * Copyright (c) 2016 Dale Rahn <drahn@dalerahn.com>
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
//! arm64's running CPU for `hal::cpu`: `curcpu()` reads `TPIDR_EL1`, as `<machine/cpu.h>`'s
//! does.
//!
//! Upstream: sys/arch/arm64/include/cpu.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/cpu.rs@3c62ede68802
//!
//! **The register's premise.** `TPIDR_EL1` holds the address of the running CPU's `struct
//! cpu_info`, whose first member is its [`CpuCore`] (`#[repr(C)]`; `bsd`'s
//! `sys/arch/arm64/include/cpu.rs` asserts the offset 0 at compile time), so the register
//! names the core. On the boot CPU [`set_current`] writes it (`initarm`'s first statement);
//! on a secondary CPU the entry code writes it before the CPU runs any code that reaches
//! `hal::cpu` (`bsd`'s `locore.S` `cpu_hatch_secondary` with the `cpu_info` that
//! `cpu_start_secondary` passed, `cpu.rs`'s `cpu_hatch_entry` likewise under Limine, right
//! after it switched to the kernel's tables, where that `cpu_info` is mapped). Every
//! `cpu_info` is a `static` (`cpu_info_primary`) or a leaked allocation (`cpu_info_alloc`):
//! it lives for ever. That is the boot contract this module's read rests on, as the bridges'
//! registration is (`sys/hal/bridges.rs`); before [`set_current`] ran on the boot CPU the
//! register holds what the boot loader left, and [`current`] does not read it.
//!
//! ## Redesign
//! - LZ's `curcpu()` (`bsd`) turned the register into a `&'static CpuInfo`; this one yields
//!   the core at its head, the part `hal` knows, and only after the boot CPU installed it.

use core::arch::asm;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::cpu::CpuCore;

/// The boot CPU's core is in `TPIDR_EL1`: set once by [`set_current`], before the secondary
/// CPUs start (their start publishes it with a release store, `BootMp::start`).
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// The running CPU's core, `None` before the boot CPU's was installed.
#[inline]
pub(crate) fn current() -> Option<&'static CpuCore> {
    if !INSTALLED.load(Ordering::Relaxed) {
        return None;
    }
    let core: *const CpuCore;
    // SAFETY: `mrs` of `TPIDR_EL1` reads a system register: no memory, no stack, no flags.
    // `INSTALLED` is set only by `set_current`, after it wrote the boot CPU's core there, and
    // every other CPU's register is written before that CPU runs code that reaches here (the
    // module's premise): so the value is the address of a `CpuCore` at the head of a
    // `cpu_info` that lives for ever. Every member of a `CpuCore` is an atomic, so the shared
    // reference aliases no exclusive one, and `'static` is the `cpu_info`'s lifetime.
    unsafe {
        asm!("mrs {}, tpidr_el1", out(reg) core, options(nomem, nostack, preserves_flags));
        Some(&*core)
    }
}

/// Installs `core` in `TPIDR_EL1` on the running CPU (the boot CPU's `initarm`). Once a core
/// is installed, installing another one on this CPU is refused (`hal::thread`'s E5: a core
/// names one CPU, whose owner word every lock it holds carries); installing the same one
/// again changes nothing.
pub(crate) fn set_current(c: &'static CpuCore) {
    if let Some(cur) = current() {
        if core::ptr::eq(cur, c) {
            return;
        }
        crate::assert::hal_panic(format_args!(
            "hal::cpu::set_current: this CPU's core is installed already"
        ));
    }
    // SAFETY: `msr` of `TPIDR_EL1` writes a system register that only `curcpu()` reads (here
    // and in `bsd`'s copy and assembly, which read the same `cpu_info` through it): no memory
    // operand, no stack, no flags. The value is a `CpuCore` that lives for ever (`'static`),
    // which is what every reader assumes the register holds. Without `nomem`, so the store
    // of `INSTALLED` below stays after it.
    unsafe {
        asm!("msr tpidr_el1, {}", in(reg) core::ptr::from_ref(c), options(nostack, preserves_flags))
    };
    INSTALLED.store(true, Ordering::Release);
}

/// `intr_disable()`: masks IRQ and FIQ on this CPU (`msr daifset, #3`) and returns the
/// `DAIF` it found (`disable_irq_daif_ret`).
#[inline]
pub(crate) fn intr_disable() -> u64 {
    let daif: u64;
    // SAFETY: `mrs` of `DAIF` and `msr daifset` read and set this CPU's interrupt mask: no
    // memory operand, no stack, no flags. Masking interrupts cannot make any access unsound;
    // it only keeps this CPU's interrupt handlers from running until the mask is restored.
    // Without `nomem`, so the compiler keeps the memory accesses after it inside the masked
    // section.
    unsafe {
        asm!(
            "mrs {}, daif",
            "msr daifset, #3",
            out(reg) daif,
            options(nostack, preserves_flags)
        )
    };
    daif
}

/// `intr_restore(s)`: `DAIF` set back to what [`intr_disable`] returned
/// (`restore_daif`); only `hal::cpu::IntrGuard`'s drop calls it, with its own saved state.
#[inline]
pub(crate) fn intr_restore(daif: u64) {
    // SAFETY: `msr daif` writes this CPU's interrupt mask: no memory operand, no stack, no
    // flags. The value is the one `intr_disable` read on this CPU (the guard is not `Send`
    // and holds it), so the mask returns to what it was before the section, as the C's
    // `intr_restore` does; unmasking cannot make an access unsound. Without `nomem`, so the
    // section's memory accesses stay before it.
    unsafe { asm!("msr daif, {}", in(reg) daif, options(nostack, preserves_flags)) };
}

/// `ncpusfound` as the per-CPU readers walk it: one past the highest attached number.
#[inline]
pub(crate) fn ncpus(attached: usize) -> usize {
    attached
}
/* </CODE> */
