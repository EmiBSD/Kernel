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
//! The clock frame as the kernel's clock callbacks read it: `struct clockframe` and the
//! `CLKF_USERMODE`, `CLKF_PC` and `CLKF_INTR` macros of `<machine/cpu.h>`.
//!
//! The frame is the machine's own type (arm64's `struct trapframe`; amd64's `struct
//! intrframe` at N10, since `lapic_clockintr` dispatches with the interrupt frame), and the
//! kernel never names its fields: it asks [`ClockFrame::usermode`], [`ClockFrame::pc`] and
//! [`ClockFrame::intr`]. `CLKF_INTR` is the same on both machines (`ci_idepth > 1`: the
//! clock interrupted another interrupt handler), so it is written here once, over the core's
//! depth (`hal::cpu`).

pub use crate::arch::current::timer::ClockFrame;

impl ClockFrame {
    /// `CLKF_USERMODE(frame)`: the interrupt came from user mode.
    #[inline]
    pub fn usermode(&self) -> bool {
        crate::arch::current::timer::clkf_usermode(self)
    }

    /// `CLKF_PC(frame)`: the interrupted program counter.
    #[inline]
    pub fn pc(&self) -> usize {
        crate::arch::current::timer::clkf_pc(self)
    }

    /// `CLKF_INTR(frame)`: the clock interrupted another interrupt handler (the running
    /// CPU's interrupt depth is above the clock's own).
    #[inline]
    pub fn intr(&self) -> bool {
        crate::cpu::curcpu().idepth() > 1
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_frame_answers_what_the_test_set() {
        let f = ClockFrame {
            user: true,
            pc: 0x1234,
        };
        assert!(f.usermode());
        assert_eq!(f.pc(), 0x1234);
        assert!(!f.intr(), "no interrupt is being handled on a test thread");
        assert!(!ClockFrame::default().usermode());
    }
}
/* </TESTS> */
