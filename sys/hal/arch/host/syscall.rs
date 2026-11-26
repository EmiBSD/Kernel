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
//! The host's double of the user registers a system call trap hands the ABI
//! (`hal::syscall::UserRegs`, decision 53). There is no trap frame on the host: a test makes
//! one with [`UserRegs::fake`] (arm64's thirty general registers, `hal::arch::host::gpr`, and
//! the carry), hands it to the ABI's handler and reads back what the handler did: the
//! registers and the carry, how many return-register writes and how many restarts. Each test
//! owns its double; nothing is shared between tests. The frame's own logic is arm64's
//! (`arch/arm64/syscall.rs`, the smokes).

use core::cell::Cell;

use super::gpr::{Gpr, NGPR};
use crate::syscall::Register;

/// The trapping thread's user registers, as a test set them.
pub struct UserRegs {
    /// `x0` to `x29`.
    x: [Cell<Register>; NGPR],
    /// The saved carry flag (arm64's `PSR_C`: a system call's error).
    carry: Cell<bool>,
    /// How many times [`Self::set_return`] wrote.
    writes: Cell<u32>,
    /// How many times [`Self::restart`] moved the pc back.
    restarts: Cell<u32>,
}

impl UserRegs {
    /// The registers `which`, read in one go.
    pub fn gprs<const N: usize>(&self, which: [Gpr; N]) -> [Register; N] {
        which.map(|r| self.x[r.index()].get())
    }

    /// Writes register `reg` and the error flag (the carry), as arm64's frame does.
    pub fn set_return(&self, reg: Gpr, value: Register, error: bool) {
        self.x[reg.index()].set(value);
        self.carry.set(error);
        self.writes.set(self.writes.get() + 1);
    }

    /// The pc moved back to the `svc`: counted.
    pub fn restart(&self) {
        self.restarts.set(self.restarts.get() + 1);
    }

    /// A trap with these registers and the carry clear.
    pub fn fake(x: [Register; NGPR]) -> Self {
        Self {
            x: x.map(Cell::new),
            carry: Cell::new(false),
            writes: Cell::new(0),
            restarts: Cell::new(0),
        }
    }

    /// A trap with these registers and this carry.
    pub fn fake_with_carry(x: [Register; NGPR], carry: bool) -> Self {
        let regs = Self::fake(x);
        regs.carry.set(carry);
        regs
    }

    /// Every register as the handler left it.
    pub fn fake_regs(&self) -> [Register; NGPR] {
        core::array::from_fn(|i| self.x[i].get())
    }

    /// The carry as the handler left it.
    pub fn fake_carry(&self) -> bool {
        self.carry.get()
    }

    /// How many return-register writes the handler made.
    pub fn fake_writes(&self) -> u32 {
        self.writes.get()
    }

    /// How many restarts the handler asked for.
    pub fn fake_restarts(&self) -> u32 {
        self.restarts.get()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_double_reads_writes_and_counts() {
        let regs = UserRegs::fake(core::array::from_fn(|i| i as Register * 10));
        assert_eq!(regs.gprs([Gpr::X8, Gpr::X0, Gpr::X29]), [80, 0, 290]);
        assert!(!regs.fake_carry());

        regs.set_return(Gpr::X0, -5, true);
        assert_eq!(regs.fake_regs()[0], -5);
        assert_eq!(regs.fake_regs()[1], 10, "x1 untouched");
        assert!(regs.fake_carry());
        regs.set_return(Gpr::X0, 7, false);
        assert!(!regs.fake_carry());
        assert_eq!(regs.fake_writes(), 2);

        regs.restart();
        assert_eq!(regs.fake_restarts(), 1);
        assert_eq!(regs.fake_writes(), 2, "a restart writes no register");

        assert!(UserRegs::fake_with_carry([0; NGPR], true).fake_carry());
    }
}
/* </TESTS> */
