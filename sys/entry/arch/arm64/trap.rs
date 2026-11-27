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
 * Copyright (c) 2014 Andrew Turner
 * All rights reserved.
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

/*
 * Copyright (c) 2015 Dale Rahn <drahn@dalerahn.com>
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
 * Copyright (c) 2011 Dale Rahn <drahn@openbsd.org>
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
//! A trap from user mode, classified (decision 56 under decision 58): the C's `do_el0_sync`
//! `switch`, `do_el0_error`, and the IRQ and FIQ from user mode, behind the one function
//! `hal`'s EL0 handlers call, `entry_user_trap`, with the frame they saved as a
//! `hal::arch::arm64::usertrap::Arrival` and the vector the CPU took.
//!
//! Upstream: sys/arch/arm64/arm64/trap.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/syscall.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/intr.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/trap.rs@e8249449d905
//! LZ: sys/arch/arm64/arm64/syscall.rs@e8249449d905
//! LZ: sys/arch/arm64/arm64/intr.rs@e8249449d905
//!
//! **The lock** (the user, 2026-11-27; docs/ENTRY.md, section 5, R6): both matches are closed
//! and fixed when the kernel is built. The vector match has four arms and no default (the
//! vector is `hal`'s enum). In the class match every class the C names has one destination:
//! a system call goes to the ABI's one entry function (`abi::arch::current::svc`, with the
//! registers `kernel::trap::svc` hands out); a fault, an undefined instruction, a breakpoint or
//! single step, an alignment fault, an identification register's read, the FPU and SVE traps,
//! a BTI or pointer authentication failure go to `kernel::trap`'s function of that class; any
//! other class is `kernel::trap::unknown`, the C's `default:` (the registers printed, the
//! process killed with `SIGILL`). An IRQ, an FIQ and an SError go to `hal` (the `Arrival`'s
//! `irq`, `fiq`, `serror`). No table, no function pointer, no registration: a new system call
//! or identification register changes the ABI or the kernel, never this file.
//!
//! **The export.** `entry_user_trap` is the one symbol of this crate `hal`'s assembly names
//! (`bl entry_user_trap` in its four EL0 handlers, resolved when the image is linked; nothing
//! stores its address). It is not `pub`: rustc gives a `#[no_mangle]` function external
//! linkage whatever its visibility, so the linker finds it and no Rust code can call it. Its
//! attribute is the crate's only `unsafe` (decision 58), and its signature is pinned to the
//! one the assembly assumes, `hal`'s `UserTrapFn`. It returns the token of `hal`'s exit gate
//! (`hal::cpu::usertrap::UserExit`), which only the kernel's AST loop hands out
//! (`kernel::trap::leave` after a synchronous trap, `kernel::trap::exit` after an interrupt);
//! an SError does not return.
//!
//! ## Redesign
//! - Decision 56 (E2) under decision 58: LZ's `do_el0_sync` (and `hal`'s from N4.8b) read the
//!   syndrome, enabled the interrupts, recorded the frame, classified the exception and
//!   handled each class in place, calling machine-independent code by name (`hal`: through a
//!   trap handler `bsd` registered in `hal`, eight methods); `svc_handler` called the ABI's
//!   handler through `hal::syscall`'s registration; `hal`'s assembly called four handlers of
//!   its own. Here the classification is all that is left, two `match`es whose every arm is
//!   one call down: the prologue and each class's handling are `kernel::trap`'s (which drives
//!   `hal`'s mechanics on the trap's token), the system call a direct call of the ABI, the
//!   return the kernel's loop's token. Nothing is registered and nothing calls back into this
//!   crate but `hal`'s one `bl`.
//! - `udata_abort`, `emulate_msr` and the signals are `kernel::trap`'s (`udata_abort`, `msr`,
//!   ...), on the token this crate hands over; `ast()` and `do_ast` are the kernel's loop
//!   (`kernel::trap::leave`, `exit`), over `hal`'s exit gate.

use hal::arch::arm64::include::armreg::{
    EXCP_BRANCH_TGT, EXCP_BRK, EXCP_DATA_ABORT_L, EXCP_FP_SIMD, EXCP_FPAC, EXCP_INSN_ABORT_L,
    EXCP_MSR, EXCP_PC_ALIGN, EXCP_SOFTSTP_EL0, EXCP_SP_ALIGN, EXCP_SVC, EXCP_SVE, EXCP_TRAP_FP,
    EXCP_UNKNOWN,
};
use hal::arch::arm64::usertrap::{Arrival, UserTrapFn, UserVector};
use hal::cpu::usertrap::UserExit;

/// The trap `hal`'s EL0 handlers took (`handle_el0_sync`, `_irq`, `_fiq`, `_error`), after
/// the save: the vector's destination, then the exit token for the restore. The crate's one
/// exported symbol (the module's export).
// The one item the root's `deny(unsafe_code)` allows (decision 58): its attribute exports the
// symbol `hal`'s assembly calls.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
extern "C" fn entry_user_trap(a: Arrival, v: UserVector) -> UserExit {
    match v {
        UserVector::Sync => do_el0_sync(a),
        UserVector::Irq => {
            a.irq();
            kernel::trap::exit()
        }
        UserVector::Fiq => {
            a.fiq();
            kernel::trap::exit()
        }
        UserVector::SError => a.serror(),
    }
}

// The signature `hal`'s assembly assumes (the frame first, the vector second, the exit token
// back): the compiler refuses another (decision 58). A coercion, not a stored pointer.
const _: UserTrapFn = entry_user_trap;

/// `do_el0_sync`: a synchronous exception from user mode: the prologue, the class's
/// destination (the lock), then `userret` and the AST loop, whose token goes back to `hal`.
fn do_el0_sync(a: Arrival) -> UserExit {
    let (t, s) = kernel::trap::begin(a);

    match s.class {
        EXCP_UNKNOWN => kernel::trap::undefined(t, s),
        EXCP_SVE => kernel::trap::sve(t),
        EXCP_FP_SIMD | EXCP_TRAP_FP => kernel::trap::fpu(t),
        EXCP_BRANCH_TGT => kernel::trap::bti(t, s),
        EXCP_MSR => kernel::trap::msr(t, s),
        EXCP_FPAC => kernel::trap::fpac(t, s),
        EXCP_SVC => abi::arch::current::svc(&kernel::trap::svc(t)),
        EXCP_INSN_ABORT_L => kernel::trap::udata_abort(t, s, true),
        EXCP_PC_ALIGN => kernel::trap::pc_align(t, s),
        EXCP_SP_ALIGN => kernel::trap::sp_align(t, s),
        EXCP_DATA_ABORT_L => kernel::trap::udata_abort(t, s, false),
        EXCP_BRK => kernel::trap::breakpoint(t, s),
        EXCP_SOFTSTP_EL0 => kernel::trap::single_step(t, s),
        _ => kernel::trap::unknown(t, s),
    }

    kernel::trap::leave()
}
/* </CODE> */
