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
//! arm64's exception entry: the vector table and the handlers' assembly of `exception.S`,
//! and the trampoline vectors of `trampoline.S` through which a return to user mode goes
//! (both pulled in from the `.S` files next to this module, which keep OpenBSD's licence
//! blocks and layout; `{NAME}` placeholders are what `assym.h` provides in C, and the Rust
//! handlers are `sym` operands).
//!
//! Upstream: sys/arch/arm64/arm64/exception.S @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/trampoline.S @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/exception.rs@3c62ede68802
//!
//! **What the assembly reaches.** `ci_curproc`, `ci_curpcb` and `ci_trampoline_vectors` at
//! the offsets of the [`CpuCore`] that `TPIDR_EL1` names (`hal::cpu`); the running thread's
//! `md_astpending` in its `MachineSlot`, first in the thread (`hal::thread`), through
//! `ci_curproc`; its pcb's `pcb_flags` (`PCB_SINGLESTEP`); the trap frame it builds on the
//! kernel stack. It calls [`do_el1h_sync`](super::trap), `do_el0_sync`, the two error
//! entries, `ast` and the IRQ/FIQ entries of `trap.rs` with the frame in `x0`.
//!
//! **The vector premise.** `VBAR_EL1` holds [`exception_vectors_addr`] while the CPU runs
//! kernel code and one of the trampoline tables while it runs user code: the boot paths
//! install the kernel's ([`install_vectors`], before any exception can be taken, on every
//! CPU), the `return` macro installs this CPU's trampoline table just before the `eret` to
//! user mode, and a trampoline table's EL0 entries install the kernel's again before they
//! branch to the handlers. The trampoline table a CPU uses is one of this file's, chosen by
//! [`set_bhb_mitigation`] (the C's `cpu_mitigate_spectre_bhb`), `none` until then.
//!
//! ## Deviations
//! - `x18` is a general register here (the C builds with `-ffixed-x18` and keeps `curcpu()`
//!   in it): the EL1 paths of the macros save the pre-exception `sp` with an `add` instead of
//!   parking it in `x18` first, restore `x18` from the frame, and recover `sp` from the
//!   frame's size instead of `mov sp, x18`. The EL0 paths are the C's.
//! - The trampoline vectors do not switch `TTBR1_EL1` to the trampoline's tables
//!   (`pmap_tramp`, the page-table isolation of the kernel while user code runs): the pmap's
//!   half is not configured (`sys/hal/arch/arm64/pmap/backend.rs`, deviations), so the
//!   kernel stays mapped at EL0 as in LZ. Against OpenBSD that is the isolation missing
//!   (Meltdown, CVE-2017-5754, on the cores it affects; the kernel's layout open to timing
//!   probes); it is N4.8d's, the slice right after N4.8b, before the N4 close (the user's
//!   decision of 2026-10-23). The vector switch and the Spectre-BHB mitigations
//!   of each table (the branch loops, the firmware's `SMCCC_ARCH_WORKAROUND_3`, `clrbhb`) are
//!   the C's. `tramp_enter` loads `exception_vectors` by `adrp`/`add` where the C uses a
//!   literal (`ldr =`); `return` takes `trampoline_vectors_none` for a CPU whose table is
//!   still 0.
//!
//! ## Redesign
//! - N4.8b: the entry moves from `bsd` (`sys/arch/arm64/arm64/exception.rs`, an `extern`
//!   static and the vectors' address) into `hal` with the handlers it calls, which it reaches
//!   by `sym` operands instead of unmangled symbols; LZ's `return` was a bare `eret` with the
//!   kernel vectors left in `VBAR_EL1` (the trampoline was not ported, LZ's deviation, gap 3
//!   of the brief, closed by the user's decision 7 of N4.8 except for the `TTBR1_EL1` half,
//!   N4.8d's).
//! - `ci_trampoline_vectors` (`bsd`'s `cpu_info` held a table number) is the core's word
//!   ([`CpuCore::trampoline_vectors`]), an address only this module writes: a CPU's code
//!   chooses a [`BhbMitigation`], never an address.

use core::arch::{asm, global_asm};
use core::mem::offset_of;
use core::ptr;

use super::include::armreg::DBG_MDSCR_SS;
use super::include::frame::{TF_SIZE, Trapframe};
use super::include::pcb::Pcb;
use super::include::proc::Mdproc;
use crate::cpu::{CORE_CURPCB, CORE_CURPROC, CORE_TRAMPOLINE_VECTORS, CpuCore};
use crate::thread::MachineSlot;

/// The bytes of one trampoline table (`.align 11`).
const TRAMPOLINE_TABLE: usize = 2048;

/// The trampoline tables of `trampoline.S`, in its order: one per Spectre-BHB mitigation
/// `cpu_mitigate_spectre_bhb` chooses (`trampoline_vectors_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BhbMitigation {
    /// `trampoline_vectors_none`: nothing (not vulnerable, or `ECBHB`, `CSV2_HCXT`).
    None = 0,
    /// `trampoline_vectors_loop_8`: Cortex-A57, A72.
    Loop8 = 1,
    /// `trampoline_vectors_loop_11`: Ampere-1 AC03.
    Loop11 = 2,
    /// `trampoline_vectors_loop_24`: Cortex-A76, A76AE, A77, Neoverse N1.
    Loop24 = 3,
    /// `trampoline_vectors_loop_32`: Cortex-A78 and its siblings, X1, X2, A710, Neoverse
    /// N2, V1.
    Loop32 = 4,
    /// `trampoline_vectors_loop_132`: Cortex-X3, X4, X925, Neoverse V2, V3, V3AE.
    Loop132 = 5,
    /// `trampoline_vectors_psci_hvc`: the firmware's `SMCCC_ARCH_WORKAROUND_3` through `hvc`.
    PsciHvc = 6,
    /// `trampoline_vectors_psci_smc`: the same through `smc`.
    PsciSmc = 7,
    /// `trampoline_vectors_clrbhb`: the `clrbhb` instruction.
    Clrbhb = 8,
}

impl BhbMitigation {
    /// The table's offset from `trampoline_vectors`.
    const fn offset(self) -> usize {
        self as usize * TRAMPOLINE_TABLE
    }
}

/// The trampoline tables' count.
const TRAMPOLINE_TABLES: usize = 9;

global_asm!(
    include_str!("exception.S"),
    include_str!("trampoline.S"),
    TF_SIZE = const TF_SIZE,
    TF_X = const offset_of!(Trapframe, tf_x),
    TF_ELR = const offset_of!(Trapframe, tf_elr),
    TF_SP = const offset_of!(Trapframe, tf_sp),
    CI_CURPROC = const CORE_CURPROC,
    CI_CURPCB = const CORE_CURPCB,
    CI_TRAMPOLINE_VECTORS = const CORE_TRAMPOLINE_VECTORS,
    P_ASTPENDING = const offset_of!(MachineSlot, md) + offset_of!(Mdproc, md_astpending),
    PCB_FLAGS = const offset_of!(Pcb, pcb_flags),
    DBG_MDSCR_SS = const DBG_MDSCR_SS,
    DO_EL1H_SYNC = sym super::trap::do_el1h_sync,
    DO_EL1H_ERROR = sym super::trap::do_el1h_error,
    DO_EL0_SYNC = sym super::trap::do_el0_sync,
    DO_EL0_ERROR = sym super::trap::do_el0_error,
    AST = sym super::trap::ast,
    ARM_CPU_IRQ = sym super::trap::arm_cpu_irq,
    ARM_CPU_FIQ = sym super::trap::arm_cpu_fiq,
    ARM_CPU_IRQ_EL0 = sym super::trap::arm_cpu_irq_el0,
    ARM_CPU_FIQ_EL0 = sym super::trap::arm_cpu_fiq_el0,
);

// SAFETY: the two items name tables defined above in this crate by their symbols, as their
// instruction words: `exception_vectors` is sixteen entries of 128 bytes (`.align 7` each, the
// table `.align 11`), 512 words; `trampoline_vectors` is the nine `tramp_vector` tables that
// follow its label back to back, 2 KiB each (`.align 11`), 4608 words, before
// `tramp_return`. They lie in the kernel text, which both boot paths and the kernel's own
// tables map readable and nothing ever writes, so a shared view of them aliases no write;
// the kernel only takes their addresses.
unsafe extern "C" {
    /// `exception_vectors`: the kernel's vector table (`VBAR_EL1` while kernel code runs).
    #[link_name = "exception_vectors"]
    safe static EXCEPTION_VECTORS: [u32; 512];
    /// `trampoline_vectors`: the trampoline tables, one per [`BhbMitigation`].
    #[link_name = "trampoline_vectors"]
    safe static TRAMPOLINE_VECTORS: [u32; TRAMPOLINE_TABLES * TRAMPOLINE_TABLE / 4];
}

/// The address of the kernel's vector table, for `VBAR_EL1`.
pub fn exception_vectors_addr() -> u64 {
    ptr::addr_of!(EXCEPTION_VECTORS).addr() as u64
}

/// The address of the trampoline table of mitigation `m`.
fn trampoline_vectors_addr(m: BhbMitigation) -> usize {
    ptr::addr_of!(TRAMPOLINE_VECTORS).addr() + m.offset()
}

/// `WRITE_SPECIALREG(vbar_el1, exception_vectors); isb`: the kernel's vector table on this
/// CPU (the boot CPU's `initarm`, a secondary CPU's start), so the exceptions it takes from
/// now on enter the handlers of `trap.rs`.
pub fn install_vectors() {
    let va = exception_vectors_addr();
    // SAFETY: `msr vbar_el1` writes this CPU's vector base, no memory and no stack. The value
    // is the address of `exception_vectors`, this module's table, whose every entry is a
    // branch to a handler of `exception.S` (the kernel's EL1 entries) or a `brk` stop (the
    // EL0 and EL1t entries, never taken while it is installed: user code runs under a
    // trampoline table); it lies in the kernel text, mapped executable at EL1 for the
    // kernel's whole life, and the `.align 11` the architecture requires. The `isb` makes the
    // next exception use it. No Rust state depends on which table is installed.
    unsafe { asm!("msr vbar_el1, {}", "isb", in(reg) va, options(nostack, preserves_flags)) };
}

/// `ci->ci_trampoline_vectors = (vaddr_t)trampoline_vectors_<m>` for the CPU `core`
/// (`cpu_mitigate_spectre_bhb`, which the CPU's identification runs): the table its next
/// returns to user mode install.
pub fn set_bhb_mitigation(core: &CpuCore, m: BhbMitigation) {
    core.set_trampoline_vectors(trampoline_vectors_addr(m));
}

/// The mitigation whose table `core` uses, `None` (the Rust option) before one was set.
pub fn bhb_mitigation(core: &CpuCore) -> Option<BhbMitigation> {
    let va = core.trampoline_vectors();
    let base = trampoline_vectors_addr(BhbMitigation::None);
    let idx = va.checked_sub(base)? / TRAMPOLINE_TABLE;
    [
        BhbMitigation::None,
        BhbMitigation::Loop8,
        BhbMitigation::Loop11,
        BhbMitigation::Loop24,
        BhbMitigation::Loop32,
        BhbMitigation::Loop132,
        BhbMitigation::PsciHvc,
        BhbMitigation::PsciSmc,
        BhbMitigation::Clrbhb,
    ]
    .get(idx)
    .copied()
}

/// `cpu_attach`'s `ci->ci_trampoline_vectors = (vaddr_t)trampoline_vectors_none`: a CPU
/// whose identification has not chosen a table yet takes the one without a mitigation.
pub(crate) fn attached(core: &CpuCore) {
    if core.trampoline_vectors() == 0 {
        set_bhb_mitigation(core, BhbMitigation::None);
    }
}
/* </CODE> */
