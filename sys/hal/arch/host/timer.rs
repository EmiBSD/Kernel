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
//! The host double of the machine's clock: the clock frame (what a test says the interrupted
//! context was) and the ARM generic timer's registers under the names of
//! `arch/arm64/timer.rs` (`hal::cpu::timer`), one register file per test thread (a thread is a
//! CPU of its own on the host). Each register holds what was last written or what the test
//! set; the virtual counter advances by the file's step at every read, so a busy-wait on it
//! ends. With the file as a thread starts (`CNTFRQ_EL0` zero) the kernel's `agtimer_init`
//! installs nothing, as on a machine without a generic timer.

// The host double alone links std outside the tests (rust-kernel.md: the host doubles).
extern crate std;

use core::cell::Cell;

use crate::assert::kassert;

/// The host's `struct clockframe`: the two facts a clock callback reads, set by the test.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClockFrame {
    /// The interrupt came from user mode (`CLKF_USERMODE`).
    pub user: bool,
    /// The interrupted program counter (`CLKF_PC`).
    pub pc: usize,
}

/// The generic timer's registers of the calling thread's CPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerRegs {
    /// `CNTFRQ_EL0`, as the firmware set it (0: not set).
    pub cntfrq: u64,
    /// `CNTVCT_EL0`: what the next read answers.
    pub cntvct: u64,
    /// How far the counter advances at each read.
    pub step: u64,
    /// `CNTV_CTL_EL0`, as last written.
    pub cntv_ctl: u32,
    /// `CNTV_TVAL_EL0`, as last written.
    pub cntv_tval: u32,
    /// `CNTKCTL_EL1`, as last written.
    pub cntkctl: u64,
    /// `CurrentEL`: the running exception level in its bits 3:2.
    pub current_el: u64,
    /// `agtimer_readcnt64 == agtimer_readcnt64_sun50i` (arm64's `SUN50I`, per thread here).
    pub sun50i: bool,
}

impl TimerRegs {
    /// A CPU as a test thread starts: no frequency, the counter at 0 stepping by 1, at EL1.
    pub const INIT: Self = Self {
        cntfrq: 0,
        cntvct: 0,
        step: 1,
        cntv_ctl: 0,
        cntv_tval: 0,
        cntkctl: 0,
        current_el: 1 << 2,
        sun50i: false,
    };
}

impl Default for TimerRegs {
    fn default() -> Self {
        Self::INIT
    }
}

std::thread_local! {
    /// The calling thread's register file.
    static REGS: Cell<TimerRegs> = const { Cell::new(TimerRegs::INIT) };
}

/// The calling thread's register file, for a test to read what the driver wrote.
pub fn regs() -> TimerRegs {
    REGS.with(Cell::get)
}

/// Sets the calling thread's register file, for a test to say what the hardware answers.
pub fn set_regs(r: TimerRegs) {
    REGS.with(|c| c.set(r));
}

/// One read of `CNTVCT_EL0`: its value, the counter advanced by the step.
fn read_cntvct() -> u64 {
    let mut r = regs();
    let v = r.cntvct;
    r.cntvct = v.wrapping_add(r.step);
    set_regs(r);
    v
}

/// `agtimer_readcnt64_default`: one counter read (the Cortex-A73 erratum has no host).
pub fn readcnt64_default() -> u64 {
    read_cntvct()
}

/// `agtimer_readcnt64_sun50i`: the counter read until its low nine bits are not at a
/// roll-over, at most 150 times, as arm64's.
pub fn readcnt64_sun50i() -> u64 {
    let mut val = 0;
    let mut retry = 0;
    while retry < 150 {
        val = read_cntvct();
        if (val.wrapping_add(1) & 0x1ff) > 1 {
            break;
        }
        retry += 1;
    }
    kassert!(retry < 150);
    val
}

/// `agtimer_readcnt64()`: the counter through the reader the attach chose.
pub fn readcnt64() -> u64 {
    if regs().sun50i {
        readcnt64_sun50i()
    } else {
        readcnt64_default()
    }
}

/// `agtimer_readcnt64 = agtimer_readcnt64_sun50i`, for the calling thread.
pub fn set_readcnt64_sun50i() {
    let mut r = regs();
    r.sun50i = true;
    set_regs(r);
}

/// The timecounter's reading: the counter's low 32 bits, through the chosen reader.
pub fn timecount() -> u32 {
    readcnt64() as u32
}

/// `CNTFRQ_EL0`.
pub fn cntfrq() -> u64 {
    regs().cntfrq
}

/// `CNTV_CTL_EL0`.
pub fn cntv_ctl() -> u32 {
    regs().cntv_ctl
}

/// `CNTV_CTL_EL0 = val`.
pub fn set_cntv_ctl(val: u32) {
    let mut r = regs();
    r.cntv_ctl = val;
    set_regs(r);
}

/// `CNTV_TVAL_EL0 = val`.
pub fn set_cntv_tval(val: u32) {
    let mut r = regs();
    r.cntv_tval = val;
    set_regs(r);
}

/// `CNTKCTL_EL1`.
pub fn cntkctl() -> u64 {
    regs().cntkctl
}

/// `CNTKCTL_EL1 = v`.
pub fn set_cntkctl(v: u64) {
    let mut r = regs();
    r.cntkctl = v;
    set_regs(r);
}

/// `READ_SPECIALREG(CurrentEL)`.
pub fn current_el() -> u64 {
    regs().current_el
}

/// `CLKF_USERMODE(frame)`.
#[inline]
pub fn clkf_usermode(frame: &ClockFrame) -> bool {
    frame.user
}

/// `CLKF_PC(frame)`.
#[inline]
pub fn clkf_pc(frame: &ClockFrame) -> usize {
    frame.pc
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counter_steps_at_each_read_and_the_registers_keep_what_was_written() {
        set_regs(TimerRegs {
            cntvct: 10,
            step: 5,
            ..TimerRegs::INIT
        });
        assert_eq!(readcnt64(), 10);
        assert_eq!(readcnt64_default(), 15);
        assert_eq!(timecount(), 20);
        set_cntv_tval(7);
        set_cntv_ctl(1);
        set_cntkctl(2);
        let r = regs();
        assert_eq!(
            (r.cntv_tval, r.cntv_ctl, r.cntkctl, r.cntvct),
            (7, 1, 2, 25)
        );
        assert_eq!((cntfrq(), current_el()), (0, 1 << 2));
    }

    #[test]
    fn the_sun50i_reader_skips_the_roll_over_values() {
        // 0x1ff and 0x200 are refused ((v + 1) & 0x1ff <= 1), 0x201 is the reading.
        set_regs(TimerRegs {
            cntvct: 0x1ff,
            ..TimerRegs::INIT
        });
        set_readcnt64_sun50i();
        assert_eq!(readcnt64(), 0x201);
        assert!(regs().sun50i);
    }
}
/* </TESTS> */
