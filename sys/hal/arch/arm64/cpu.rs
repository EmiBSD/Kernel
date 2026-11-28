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
//! `cpu_start_secondary` passed, right after `cpu_hatch_entry` switched to the kernel's
//! tables, where that `cpu_info` is mapped). Every
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
/// is installed, installing another one on this CPU is refused (`hal::cpu::thread`'s E5: a core
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
        asm!("msr tpidr_el1, {}", in(reg) core::ptr::from_ref(c), options(nostack, preserves_flags));
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
        );
    };
    daif
}

/// `intr_restore(s)`: `DAIF` set back to what [`intr_disable`] returned
/// (`restore_daif`); its callers are `hal::cpu::IntrGuard`'s drop, with its own saved state,
/// `hal::sync::mutex::DbMutex::leave`, with the state its owner's `enter` saved on this CPU,
/// and the exit gate (`hal::cpu::usertrap::ExitGate::pass`), with the state its first pass
/// found, perhaps on another CPU (the C's `do_ast` keeps it in `x19` across `ast()`).
#[inline]
pub(crate) fn intr_restore(daif: u64) {
    // SAFETY: `msr daif` writes this CPU's interrupt mask: no memory operand, no stack, no
    // flags. The value is one `intr_disable` read: on this CPU for the guard (not `Send`,
    // it holds the word) and for `DbMutex::leave` (which panics, in every build, unless the
    // running CPU owns the mutex, whose owner's `enter` stored the word on this CPU before
    // returning), so the mask returns to what it was before the section, as the C's
    // `intr_restore` does; for the exit gate, the state the running thread had when its
    // first pass began, possibly on another CPU before a pending `ast()` moved the thread
    // (the C's `x19` across `bl ast`): the thread's own kernel-mode mask, interrupts
    // enabled on that path, which the C restores the same way. Whatever the mask, no memory
    // access becomes unsound: it only decides when this CPU's handlers run. Without
    // `nomem`, so the section's memory accesses stay before it.
    unsafe { asm!("msr daif, {}", in(reg) daif, options(nostack, preserves_flags)) };
}

/// `intr_enable()`: unmasks IRQ and FIQ on this CPU (`msr daifclr, #3`,
/// `enable_irq_daif`): the exception entry's, once the trap frame is saved (`trap.rs`,
/// `syscall.rs`), and `hal::cpu::intr_enable`'s (the interrupt controllers in the kernel).
#[inline]
pub(crate) fn intr_enable() {
    // SAFETY: `msr daifclr` clears this CPU's IRQ and FIQ mask bits: no memory operand, no
    // stack, no flags. As for `intr_restore`, no memory access becomes unsound whatever the
    // mask: it only decides when this CPU's interrupt handlers run, and they save and restore
    // what they use. Without `nomem`, so the compiler keeps the frame's reads before it.
    unsafe { asm!("msr daifclr, #3", options(nostack, preserves_flags)) };
}

/// `ncpusfound` as the per-CPU readers walk it: one past the highest attached number.
#[inline]
pub(crate) fn ncpus(attached: usize) -> usize {
    attached
}

/// `READ_SPECIALREG(mpidr_el1)`: the executing CPU's Multiprocessor Affinity Register, the
/// identifier the device tree's `/cpus` nodes and ACPI's MADT give it (`hal::cpu::hwid`).
#[inline]
pub(crate) fn hwid() -> u64 {
    let mpidr: u64;
    // SAFETY: `mrs` of `MPIDR_EL1` reads a read-only identification register of the CPU that
    // executes it, which the architecture makes readable at EL1 whatever the state of the MMU,
    // the caches or `cpu_info` (the kernel runs at EL1 from its first instruction, `locore`):
    // no memory operand, no stack, no flags, and no effect on any state, so it can violate no
    // invariant any other code holds. A thread moved between the read and the use reads its
    // old CPU's value, which is the C's `READ_SPECIALREG` too (its callers run on the boot CPU
    // during autoconfiguration, or with the CPU fixed).
    unsafe {
        asm!("mrs {}, mpidr_el1", out(reg) mpidr, options(nomem, nostack, preserves_flags));
    }
    mpidr
}
/* </CODE> */
