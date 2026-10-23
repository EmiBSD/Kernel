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

/*-
 * Copyright (c) 2013, 2014 Andrew Turner
 * Copyright (c) 2015 The FreeBSD Foundation
 * All rights reserved.
 *
 * This software was developed by Andrew Turner under
 * sponsorship from the FreeBSD Foundation.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * $FreeBSD: head/sys/arm64/include/armreg.h 309248 2016-11-28 14:24:07Z andrew $
 */
/* </LICENSES> */

/* <CODE> */
//! arm64 `<machine/armreg.h>`: the bits of the system registers that `hal`'s machine code
//! reads (the exception entry, the trap, the system call and the signal frame).
//!
//! Upstream: sys/arch/arm64/include/armreg.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/armreg.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.8b: the part of the header the trap code needs moves with it into `hal` (the
//!   `ESR_ELx` exception classes and syndrome bits, `INSN_SIZE`, the `PSR` bits, the
//!   `MDSCR_EL1` single-step bit); `bsd`'s `sys/arch/arm64/include/armreg.rs` keeps the whole
//!   header, with its `READ_SPECIALREG`/`WRITE_SPECIALREG` macros, for the CPU, timer and
//!   interrupt-controller code still there (N7). Values are the C's; `hal` reads the
//!   registers with its own `asm!` where it needs them.
//! - `trap.c`'s `accesstype`, a pure decode of the data-abort syndrome, sits beside the bits
//!   it reads, so the host tests check it (`sys/hal/arch/host/mod.rs` compiles this header).

use crate::sys::mman::{PROT_EXEC, PROT_READ, PROT_WRITE};

/// `INSN_SIZE`: every A64 instruction is four bytes.
pub const INSN_SIZE: usize = 4;

/// `ESR_ELx_ISS_MASK`: the instruction specific syndrome.
pub const ESR_ELX_ISS_MASK: u64 = 0x00ff_ffff;
/// `ISS_DATA_CM`: a cache maintenance instruction.
pub const ISS_DATA_CM: u64 = 0x01 << 8;
/// `ISS_DATA_WnR`: the access was a write.
pub const ISS_DATA_WNR: u64 = 0x01 << 6;
/// `ISS_DATA_DFSC_MASK`: the data fault status code.
pub const ISS_DATA_DFSC_MASK: u64 = 0x3f;
/// `ISS_DATA_DFSC_ALIGN`: alignment fault.
pub const ISS_DATA_DFSC_ALIGN: u64 = 0x21;
/// `ISS_BRK_COMMENT_MASK`: the immediate of a `brk` instruction.
pub const ISS_BRK_COMMENT_MASK: u64 = 0xffff;
/// `ISS_MSR_DIR`: a read (`mrs`).
pub const ISS_MSR_DIR: u64 = 0x01;
/// `ISS_MSR_Rt_SHIFT`.
pub const ISS_MSR_RT_SHIFT: u32 = 5;
/// `ISS_MSR_Rt_MASK`.
pub const ISS_MSR_RT_MASK: u64 = 0x1f << ISS_MSR_RT_SHIFT;
/// `ISS_MSR_CRm_SHIFT`.
pub const ISS_MSR_CRM_SHIFT: u32 = 1;
/// `ISS_MSR_CRm_MASK`.
pub const ISS_MSR_CRM_MASK: u64 = 0xf << ISS_MSR_CRM_SHIFT;
/// `ISS_MSR_CRn_SHIFT`.
pub const ISS_MSR_CRN_SHIFT: u32 = 10;
/// `ISS_MSR_CRn_MASK`.
pub const ISS_MSR_CRN_MASK: u64 = 0xf << ISS_MSR_CRN_SHIFT;
/// `ISS_MSR_OP1_SHIFT`.
pub const ISS_MSR_OP1_SHIFT: u32 = 14;
/// `ISS_MSR_OP1_MASK`.
pub const ISS_MSR_OP1_MASK: u64 = 0x7 << ISS_MSR_OP1_SHIFT;
/// `ISS_MSR_OP2_SHIFT`.
pub const ISS_MSR_OP2_SHIFT: u32 = 17;
/// `ISS_MSR_OP2_MASK`.
pub const ISS_MSR_OP2_MASK: u64 = 0x7 << ISS_MSR_OP2_SHIFT;
/// `ISS_MSR_OP0_SHIFT`.
pub const ISS_MSR_OP0_SHIFT: u32 = 20;
/// `ISS_MSR_OP0_MASK`.
pub const ISS_MSR_OP0_MASK: u64 = 0x3 << ISS_MSR_OP0_SHIFT;

/// `ISS_MSR_OP0(esr)` and its siblings: one field of a trapped `msr`/`mrs` syndrome.
pub const fn iss_msr_field(esr: u64, mask: u64, shift: u32) -> u64 {
    (esr & mask) >> shift
}

/// `ESR_ELx_EC_SHIFT`: the exception class field.
pub const ESR_ELX_EC_SHIFT: u32 = 26;
/// `ESR_ELx_EC_MASK`.
pub const ESR_ELX_EC_MASK: u64 = 0x3f << 26;

/// `ESR_ELx_EXCEPTION(esr)`: the exception class of a syndrome.
pub const fn esr_elx_exception(esr: u64) -> u32 {
    ((esr & ESR_ELX_EC_MASK) >> ESR_ELX_EC_SHIFT) as u32
}

/// `accesstype` (`trap.c`): the access an abort's syndrome describes, `PROT_EXEC` for an
/// instruction abort; a cache maintenance instruction reports `WnR` but reads.
pub const fn accesstype(esr: u64, exe: bool) -> i32 {
    if exe {
        return PROT_EXEC;
    }
    if (esr & ISS_DATA_CM) == 0 && (esr & ISS_DATA_WNR) != 0 {
        PROT_WRITE
    } else {
        PROT_READ
    }
}

/// `EXCP_UNKNOWN`: Unkwn exception.
pub const EXCP_UNKNOWN: u32 = 0x00;
/// `EXCP_FP_SIMD`: FP/SIMD trap.
pub const EXCP_FP_SIMD: u32 = 0x07;
/// `EXCP_BRANCH_TGT`: Branch target exception.
pub const EXCP_BRANCH_TGT: u32 = 0x0d;
/// `EXCP_SVC`: SVC trap.
pub const EXCP_SVC: u32 = 0x15;
/// `EXCP_MSR`: MSR/MRS trap.
pub const EXCP_MSR: u32 = 0x18;
/// `EXCP_SVE`: SVE trap.
pub const EXCP_SVE: u32 = 0x19;
/// `EXCP_FPAC`: Faulting PAC trap.
pub const EXCP_FPAC: u32 = 0x1c;
/// `EXCP_INSN_ABORT_L`: Instruction abort, from lower EL.
pub const EXCP_INSN_ABORT_L: u32 = 0x20;
/// `EXCP_INSN_ABORT`: Instruction abort, from same EL.
pub const EXCP_INSN_ABORT: u32 = 0x21;
/// `EXCP_PC_ALIGN`: PC alignment fault.
pub const EXCP_PC_ALIGN: u32 = 0x22;
/// `EXCP_DATA_ABORT_L`: Data abort, from lower EL.
pub const EXCP_DATA_ABORT_L: u32 = 0x24;
/// `EXCP_DATA_ABORT`: Data abort, from same EL.
pub const EXCP_DATA_ABORT: u32 = 0x25;
/// `EXCP_SP_ALIGN`: SP alignment fault.
pub const EXCP_SP_ALIGN: u32 = 0x26;
/// `EXCP_TRAP_FP`: Trapped FP exception.
pub const EXCP_TRAP_FP: u32 = 0x2c;
/// `EXCP_SOFTSTP_EL0`: Software Step, from lower EL.
pub const EXCP_SOFTSTP_EL0: u32 = 0x32;
/// `EXCP_SOFTSTP_EL1`: Software Step, from same EL.
pub const EXCP_SOFTSTP_EL1: u32 = 0x33;
/// `EXCP_WATCHPT_EL1`: Watchpoint, from same EL.
pub const EXCP_WATCHPT_EL1: u32 = 0x35;
/// `EXCP_BRK`: Breakpoint.
pub const EXCP_BRK: u32 = 0x3c;

/// `DBG_MDSCR_SS`: software step enable.
pub const DBG_MDSCR_SS: u64 = 0x1;

/// `PSR_M_MASK`.
pub const PSR_M_MASK: u64 = 0x0000_001f;
/// `PSR_M_EL0t`: the EL0 mode (`SPSR_EL1.M`).
#[allow(non_upper_case_globals)]
pub const PSR_M_EL0t: u64 = 0x0000_0000;
/// `PSR_F`: FIQ masked.
pub const PSR_F: u64 = 0x0000_0040;
/// `PSR_I`: IRQ masked.
pub const PSR_I: u64 = 0x0000_0080;
/// `PSR_BTYPE`: the branch type of the last indirect branch (BTI).
pub const PSR_BTYPE: u64 = 0x0000_0c00;
/// `PSR_C`: the carry flag, the system call's error flag.
pub const PSR_C: u64 = 0x2000_0000;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// The exception class sits in bits 31:26; the syndrome fields are the C's.
    #[test]
    fn syndrome_fields() {
        assert_eq!(esr_elx_exception(0x5600_0000), EXCP_SVC);
        assert_eq!(esr_elx_exception(0x9200_0021), EXCP_DATA_ABORT_L);
        assert_eq!(esr_elx_exception(0xf200_f000), EXCP_BRK);
        // mrs x3, midr_el1: op0 3, op1 0, CRn 0, CRm 0, op2 0, Rt 3, a read.
        let esr = (3 << ISS_MSR_OP0_SHIFT) | (3 << ISS_MSR_RT_SHIFT) | ISS_MSR_DIR;
        assert_eq!(iss_msr_field(esr, ISS_MSR_OP0_MASK, ISS_MSR_OP0_SHIFT), 3);
        assert_eq!(iss_msr_field(esr, ISS_MSR_RT_MASK, ISS_MSR_RT_SHIFT), 3);
        assert_eq!(iss_msr_field(esr, ISS_MSR_CRN_MASK, ISS_MSR_CRN_SHIFT), 0);
    }

    /// `accesstype`: an instruction abort executes; a data abort writes only with `WnR` set
    /// and no cache maintenance (LZ's `syndrome_to_access`).
    #[test]
    fn syndrome_to_access() {
        assert_eq!(accesstype(0, true), PROT_EXEC);
        assert_eq!(accesstype(0, false), PROT_READ);
        assert_eq!(accesstype(ISS_DATA_WNR, false), PROT_WRITE);
        assert_eq!(accesstype(ISS_DATA_WNR | ISS_DATA_CM, false), PROT_READ);
    }
}
/* </TESTS> */
