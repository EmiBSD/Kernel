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
 * Copyright (c) 2007, 2009, 2011, 2017 Dale Rahn <drahn@dalerahn.com>
 * Copyright (c) 2018 Mark Kettenis <kettenis@openbsd.org>
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
//! The GICv3 CPU interface of the executing CPU: its system registers (`ICC_*_EL1`), as
//! `agintc.c` reads and writes them with `mrs`/`msr`, each with the barrier the C puts beside
//! it, and the `dsb sy` that makes a table the ITS or a redistributor reads visible to it
//! (`hal::cpu::icc`, the kernel's `agintc(4)`).
//!
//! Upstream: sys/arch/arm64/dev/agintc.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/agintc.rs@e8249449d905
//!
//! Every access is one instruction on a register of the CPU that runs it: a thread moved
//! between two calls reaches the other CPU's interface, which is the C's too (its callers run
//! with interrupts masked, at the IRQ dispatch, or on a CPU fixed by its own start).
//!
//! ## Redesign
//! - LZ's `icc_reg!`, `icc_read!` and `icc_write!` macros expanded an `asm!` block, and an
//!   `unsafe` block, at each use inside `bsd`'s driver, beside the free `isb()` and `dsb_sy()`
//!   (four `unsafe` sites in `bsd`, one per use in effect): here the two macros define one
//!   function per register access, the C's barrier in the same instruction sequence (`isb`
//!   after the priority mask, `SRE` and an end of interrupt, `dsb sy` after an acknowledge,
//!   `dsb ish` before and `isb` after an SGI, LZ's deviation for the posted reason), so the
//!   driver in the kernel calls safe functions and the sequence cannot be split. The `unsafe`
//!   is one site per macro and one for [`dsb_sy`].

use core::arch::asm;

use crate::cpu::intr::InIrq;

/// The CPU interface's system registers, by the C's names, as `mrs`/`msr` operands.
macro_rules! icc_reg {
    (ICC_PMR) => {
        "s3_0_c4_c6_0"
    };
    (ICC_IAR1) => {
        "s3_0_c12_c12_0"
    };
    (ICC_EOIR1) => {
        "s3_0_c12_c12_1"
    };
    (ICC_BPR1) => {
        "s3_0_c12_c12_3"
    };
    (ICC_CTLR_EL1) => {
        "s3_0_c12_c12_4"
    };
    (ICC_SRE_EL1) => {
        "s3_0_c12_c12_5"
    };
    (ICC_IGRPEN1) => {
        "s3_0_c12_c12_7"
    };
    (ICC_SGI1R) => {
        "s3_0_c12_c11_5"
    };
}

/// One function reading a CPU interface register (`mrs`), then the instructions `$post`; its
/// parameters (`$arg: $ty`, none or the dispatch's token) are proofs the read asks of its
/// caller and does not use.
macro_rules! icc_read {
    ($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*), $reg:ident $(, $post:literal)*) => {
        $(#[$doc])*
        #[inline(always)]
        pub fn $name($($arg: $ty),*) -> u64 {
            let val: u64;
            // SAFETY: `mrs` of a GICv3 CPU interface register (`icc_reg!`'s list: the
            // priority mask, the control register or the group 1 acknowledge) of the CPU
            // that executes it, followed only by barriers (`$post`). No memory operand, no
            // stack, no flags; the result is a plain integer, so no Rust value or invariant
            // depends on the bits. Before `ICC_SRE_EL1.SRE` is set the access is an
            // undefined-instruction trap, which the kernel reports: not undefined behaviour.
            // Reading `ICC_IAR1_EL1` acknowledges the highest pending interrupt: a change of
            // the GIC's own state, which no Rust object holds or reads, so neither the read
            // nor an interrupt left unended can break a Rust invariant (that the dispatch
            // ends it is the driver's protocol, not this block's premise; `iar1` asks for the
            // dispatch's token, `InIrq`, so it is called inside one). No `nomem`: the access
            // stays ordered with the memory accesses around it (the level in `ci_cpl`, the
            // handler records), as the C's `volatile` asm.
            unsafe {
                asm!(
                    concat!("mrs {}, ", icc_reg!($reg)),
                    $($post,)*
                    out(reg) val,
                    options(nostack, preserves_flags)
                )
            };
            val
        }
    };
}

/// One function writing a CPU interface register (`msr`), between the instructions `$pre`
/// and `$post`; its parameters before the value (`$arg: $ty`, none or the dispatch's token)
/// are proofs the write asks of its caller and does not use.
macro_rules! icc_write {
    ($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*), $reg:ident, [$($pre:literal),*], [$($post:literal),*]) => {
        $(#[$doc])*
        #[inline(always)]
        pub fn $name($($arg: $ty,)* v: u64) {
            // SAFETY: `msr` of a GICv3 CPU interface register (`icc_reg!`'s list) of the CPU
            // that executes it, between barriers only (`$pre`, `$post`). No memory operand,
            // no stack, no flags. What a write changes is how interrupts reach this CPU (its
            // priority mask, binary point, group 1 enable, the system register enable), the
            // end of an interrupt it acknowledged, or an SGI raised at other CPUs, which take
            // it as an interrupt the kernel's dispatch handles: no memory and no Rust-visible
            // state, whatever the value (reserved bits are ignored by the architecture).
            // Before `ICC_SRE_EL1.SRE` is set the access is an undefined-instruction trap,
            // which the kernel reports: not undefined behaviour. No soundness argument rests
            // on the level or the mask this leaves: the caller's `spl` bookkeeping is the
            // kernel's, as the C's `ci_cpl` is. No `nomem`, as `icc_read!`.
            unsafe {
                asm!(
                    $($pre,)*
                    concat!("msr ", icc_reg!($reg), ", {}"),
                    $($post,)*
                    in(reg) v,
                    options(nostack, preserves_flags)
                )
            };
        }
    };
}

icc_read!(
    /// `mrs ICC_CTLR_EL1`: the control register (its `PRIbits`, the priority bits implemented).
    ctlr(),
    ICC_CTLR_EL1
);

icc_read!(
    /// `mrs ICC_PMR_EL1`: the priority mask.
    pmr(),
    ICC_PMR
);

icc_write!(
    /// `msr ICC_PMR_EL1; isb`: the priority mask (`agintc_setipl`), in effect before the next
    /// instruction (the attach's probe and its reset to `0xff`, which the C writes without
    /// the `isb`, take it too: a barrier more, outside any hot path).
    set_pmr(),
    ICC_PMR,
    [],
    ["isb"]
);

icc_write!(
    /// `msr ICC_SRE_EL1; isb`: the system register interface enable (`ICC_SRE_EL1_EN`).
    set_sre(),
    ICC_SRE_EL1,
    [],
    ["isb"]
);

icc_write!(
    /// `msr ICC_BPR1_EL1`: the group 1 binary point.
    set_bpr1(),
    ICC_BPR1,
    [],
    []
);

icc_write!(
    /// `msr ICC_IGRPEN1_EL1`: the group 1 interrupt enable.
    set_igrpen1(),
    ICC_IGRPEN1,
    [],
    []
);

icc_read!(
    /// `mrs ICC_IAR1_EL1; dsb sy` (`agintc_iack`): acknowledges the highest pending group 1
    /// interrupt and answers its number (1023: none); inside an IRQ dispatch alone (its
    /// token).
    iar1(_irq: &InIrq),
    ICC_IAR1,
    "dsb sy"
);

icc_write!(
    /// `msr ICC_EOIR1_EL1; isb` (`agintc_eoi`): ends the interrupt `iar1` answered, inside the
    /// same dispatch (its token).
    eoir1(_irq: &InIrq),
    ICC_EOIR1,
    [],
    ["isb"]
);

icc_write!(
    /// `dsb ish; msr ICC_SGI1R_EL1; isb` (`agintc_send_ipi`): raises an SGI at the CPUs the
    /// value names, after every store before it (the posted IPI reason) is visible to them.
    sgi1r(),
    ICC_SGI1R,
    ["dsb ish"],
    ["isb"]
);

/// `__asm volatile("dsb sy")`: completes every memory access before it, for every observer of
/// the system (the C's "make globally visible" after a write to a table the ITS or a
/// redistributor reads, before the register write that makes it read the table).
#[inline(always)]
pub fn dsb_sy() {
    // SAFETY: a barrier: it accesses no memory and no register, and only waits for the CPU's
    // earlier accesses to complete. No stack, no flags. No `nomem`: it must stay ordered with
    // the stores to the table before it and the register store after it.
    unsafe { asm!("dsb sy", options(nostack, preserves_flags)) };
}
/* </CODE> */
