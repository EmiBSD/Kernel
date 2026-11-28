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
 * Copyright (c) 2014 Robin Randhawa
 * Copyright (c) 2015 The FreeBSD Foundation
 * All rights reserved.
 *
 * Portions of this software were developed by Andrew Turner
 * under sponsorship from the FreeBSD Foundation
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
 */
/* </LICENSES> */

/* <CODE> */
//! arm64's SMCCC conduit: `smc_call` and `hvc_call` (`support.S`), the firmware call of the
//! Arm SMC Calling Convention, `x0`..`x3` in and `x0` out, for `hal::cpu::smccc::call`'s value
//! class only.
//!
//! Upstream: sys/arch/arm64/arm64/cpufunc_asm.S @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/cpufunc.rs@e8249449d905
//!
//! LZ's port of the two routines sits in its `cpufunc.rs` (`support.S`'s `smc_call` and
//! `hvc_call` are four instructions each); `bsd`'s copy keeps them for `CPU_ON` alone until
//! N7.2c, whose arguments are an entry point and a context (docs/SCAFFOLDING.md).
//!
//! ## Redesign
//! - LZ's `smc_call(a0, a1, a2, a3)`/`hvc_call` (two functions any caller could reach with any
//!   id, psci(4)'s `sc_callfn` choosing between them) -> [`trap`] over a [`Conduit`] held by
//!   value, behind `hal::cpu::smccc::call`, which refuses the entry-point functions before
//!   the trap.
//! - LZ's `asm!` declared `x0`..`x3` alone; the C's routine is a function call, across which
//!   `x4`..`x17` are dead. Both blocks here say so (`clobber_abi("C")`), so no value the
//!   compiler keeps in a caller-saved register is assumed to survive the firmware.

use core::arch::asm;

/// psci(4)'s `sc_callfn`: which instruction reaches the firmware.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Conduit {
    /// `hvc #0`: the hypervisor, or the firmware at EL2 (`method = "hvc"`).
    Hvc,
    /// `smc #0`: the secure monitor at EL3 (`method = "smc"`).
    Smc,
}

/// `hvc_call(fid, a1, a2, a3)` or `smc_call(..)`: the firmware's `x0`. Only
/// `hal::cpu::smccc::call` reaches it, after its refusal of the entry-point functions.
pub(crate) fn trap(conduit: Conduit, fid: u32, a1: u64, a2: u64, a3: u64) -> u64 {
    let mut x0 = u64::from(fid);
    match conduit {
        Conduit::Hvc => {
            // SAFETY: `hvc #0` enters the hypervisor (or the firmware at EL2) and comes back to
            // the next instruction (SMCCC: a call returns, or never does for SYSTEM_OFF and
            // SYSTEM_RESET). The firmware may change `x0`..`x17` (`x0`..`x3` are the operands
            // here, the rest `clobber_abi("C")` declares dead, as across the C's call) and
            // preserves `x18`..`x30` and `sp`; it touches no memory the kernel owns because
            // `fid` is not one of the functions that take an address (`call` refused those),
            // so every argument is a value the firmware reads as a number, never a pointer it
            // follows or an entry it runs. No stack is used here.
            unsafe {
                asm!(
                    "hvc #0",
                    inout("x0") x0,
                    inout("x1") a1 => _,
                    inout("x2") a2 => _,
                    inout("x3") a3 => _,
                    clobber_abi("C"),
                    options(nostack),
                );
            }
        }
        Conduit::Smc => {
            // SAFETY: as for `hvc` above, with `smc #0` entering the secure monitor at EL3,
            // which keeps the same convention.
            unsafe {
                asm!(
                    "smc #0",
                    inout("x0") x0,
                    inout("x1") a1 => _,
                    inout("x2") a2 => _,
                    inout("x3") a3 => _,
                    clobber_abi("C"),
                    options(nostack),
                );
            }
        }
    }
    x0
}
/* </CODE> */
