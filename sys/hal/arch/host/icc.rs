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
//! The host double of the GICv3 CPU interface (`hal::cpu::icc`): one register file per test
//! thread (a thread is a CPU of its own on the host), each register holding what was last
//! written, and what a test says the hardware answers: the control register, the priority
//! mask's implemented bits and the interrupt the next acknowledge returns.

// The host double alone links std outside the tests (rust-kernel.md: the host doubles).
extern crate std;

use core::cell::Cell;

use crate::cpu::intr::InIrq;

/// The registers of the calling thread's CPU interface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IccRegs {
    /// `ICC_CTLR_EL1`, as the test set it.
    pub ctlr: u64,
    /// `ICC_PMR_EL1`, as last written (the implemented bits only).
    pub pmr: u64,
    /// The bits of `ICC_PMR_EL1` that stick (0 for all eight).
    pub pmr_bits: u64,
    /// `ICC_SRE_EL1`, as last written.
    pub sre: u64,
    /// `ICC_BPR1_EL1`, as last written.
    pub bpr1: u64,
    /// `ICC_IGRPEN1_EL1`, as last written.
    pub igrpen1: u64,
    /// What the next `ICC_IAR1_EL1` read answers (1023: nothing pending).
    pub iar1: u64,
    /// `ICC_EOIR1_EL1`, as last written.
    pub eoir1: u64,
    /// `ICC_SGI1R_EL1`, as last written.
    pub sgi1r: u64,
}

std::thread_local! {
    /// The calling thread's register file.
    static REGS: Cell<IccRegs> = const {
        Cell::new(IccRegs {
            ctlr: 0,
            pmr: 0,
            pmr_bits: 0,
            sre: 0,
            bpr1: 0,
            igrpen1: 0,
            iar1: 1023,
            eoir1: 0,
            sgi1r: 0,
        })
    };
}

/// The calling thread's register file, for a test to read what the driver wrote.
pub fn regs() -> IccRegs {
    REGS.with(Cell::get)
}

/// Sets the calling thread's register file, for a test to say what the hardware answers.
pub fn set_regs(r: IccRegs) {
    REGS.with(|c| c.set(r));
}

/// `ICC_CTLR_EL1`.
pub fn ctlr() -> u64 {
    regs().ctlr
}

/// `ICC_PMR_EL1`.
pub fn pmr() -> u64 {
    regs().pmr
}

/// `ICC_PMR_EL1 = v`: the implemented bits stick.
pub fn set_pmr(v: u64) {
    let mut r = regs();
    let bits = if r.pmr_bits == 0 { 0xff } else { r.pmr_bits };
    r.pmr = v & bits;
    set_regs(r);
}

/// `ICC_SRE_EL1 = v`.
pub fn set_sre(v: u64) {
    let mut r = regs();
    r.sre = v;
    set_regs(r);
}

/// `ICC_BPR1_EL1 = v`.
pub fn set_bpr1(v: u64) {
    let mut r = regs();
    r.bpr1 = v;
    set_regs(r);
}

/// `ICC_IGRPEN1_EL1 = v`.
pub fn set_igrpen1(v: u64) {
    let mut r = regs();
    r.igrpen1 = v;
    set_regs(r);
}

/// `ICC_IAR1_EL1`: what the test said, once (then nothing pending).
pub fn iar1(_irq: &InIrq) -> u64 {
    let mut r = regs();
    let v = r.iar1;
    r.iar1 = 1023;
    set_regs(r);
    v
}

/// `ICC_EOIR1_EL1 = v`.
pub fn eoir1(_irq: &InIrq, v: u64) {
    let mut r = regs();
    r.eoir1 = v;
    set_regs(r);
}

/// `ICC_SGI1R_EL1 = v`.
pub fn sgi1r(v: u64) {
    let mut r = regs();
    r.sgi1r = v;
    set_regs(r);
}

/// `dsb sy`: the host's memory is the tests' own, which no device reads.
pub fn dsb_sy() {}
/* </CODE> */
