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
//! The GICv3 CPU interface of the executing CPU (`ICC_*_EL1`), for the kernel's `agintc(4)`:
//! the system registers it reads and writes, each access one function with the C's barrier
//! (arm64's `arch/arm64/icc.rs`), and the `dsb sy` after a table the GIC reads. On the host,
//! one register file per test thread (`arch/host/icc.rs`), which a test reads and sets
//! ([`regs`], [`set_regs`]). arm64 and the host only: amd64 has no GICv3.

#[cfg(not(target_os = "none"))]
pub use crate::arch::current::icc::{IccRegs, regs, set_regs};
pub use crate::arch::current::icc::{
    ctlr, dsb_sy, eoir1, iar1, pmr, set_bpr1, set_igrpen1, set_pmr, set_sre, sgi1r,
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_double_keeps_the_implemented_priority_bits_and_acknowledges_once() {
        set_regs(IccRegs {
            pmr_bits: 0xf0,
            iar1: 27,
            ..IccRegs::default()
        });
        set_pmr(0xbf);
        assert_eq!(pmr(), 0xb0, "only the implemented bits stick");
        assert_eq!(iar1(), 27);
        assert_eq!(iar1(), 1023, "then nothing is pending");
        eoir1(27);
        sgi1r(0x0100_0001);
        set_sre(0x7);
        set_bpr1(0);
        set_igrpen1(1);
        dsb_sy();
        let r = regs();
        assert_eq!(
            (r.eoir1, r.sgi1r, r.sre, r.bpr1, r.igrpen1),
            (27, 0x0100_0001, 7, 0, 1)
        );
        assert_eq!(ctlr(), 0);
    }
}
/* </TESTS> */
