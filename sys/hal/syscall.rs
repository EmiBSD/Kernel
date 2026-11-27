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
 * Copyright (c) 1998, 2000 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Charles M. Hannum.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

/*-
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the University of Utah, and William Jolitz.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)trap.c	7.4 (Berkeley) 5/13/91
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
/* </LICENSES> */

/* <CODE> */
//! The types of the system call's edge between the trap and the ABI's handler: the trapping
//! thread's registers ([`UserRegs`], named by [`Gpr`]), the argument and return registers,
//! the result, and a forked child's first return as the ABI's data ([`ChildReturn`]).
//!
//! Upstream: sys/arch/amd64/amd64/trap.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/syscall.c @ 3ce1f3f79392
//! LZ: sys/arch/amd64/amd64/trap.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/syscall.rs@3c62ede68802
//!
//! `hal` is the cable (decision 53, the user, 2026-11-26): the trap keeps the machine's
//! mechanics (the vector, the frame, the counter, the interrupts, the barrier skip) and
//! hands the ABI's handler the trapping thread's user registers as a [`UserRegs`], whose
//! registers are named by the machine's [`Gpr`]; the handler reads the number and the
//! arguments, answers the call through the kernel, writes the result (or rewinds the pc
//! through [`UserRegs::restart`]) and calls `mi_syscall_return` through `kernel::api`. A
//! forked child's first return is a [`ChildReturn`], the ABI's constant, which the kernel
//! hands to `hal::cpu::thread::thread_setup` with each fork of a returning child (decision
//! 56, E1). Nothing userland sends is interpreted here.
//!
//! Nothing is registered here (decision 56): the trap's classification (`entry`) calls the
//! ABI's handler (`abi::arch::current::svc`) itself, with the [`UserRegs`]
//! `kernel::trap::svc` hands out, so no call goes upward from `hal` for a system call.

//! ## Redesign
//! - LZ's two traps each chose the `sysent[]` entry and called `mi_syscall` (amd64
//!   `syscall`, arm64 `svc_handler`); that choice is the registered handler's, so the system
//!   call table lives in the ABI (decision 22; linked into the image since decision 47) and
//!   `bsd`'s table is the `Legacy` path until every entry is `Native`.
//! - The registration is an init-once cell written by registration, never by a store a
//!   caller can repeat: a second registration is refused, so a system call never sees the
//!   handler change under it. Decision 29's second slot, the `Legacy` fallback the traps took
//!   when the module was not loaded, went with the module at the N5 close (decision 47).
//! - Decision 53 (N7.abi2): the handler was `fn(number, &args, &mut rval) -> error`, called
//!   by `dispatch`, and the traps read the number and arguments and wrote the result and the
//!   carry themselves; now it is `fn(&UserRegs)` called by [`enter`], and the per-arch
//!   convention is the ABI's (`abi::arch::<arch>`). `dispatch` and its `ENOSYS` without a
//!   handler are gone.
//! - Decision 56 (E2, U2): the registration goes (the handler's type, the setter, what the
//!   handler said about itself, and `enter`, which called it): the trap's classification
//!   (`entry`) calls the ABI's handler itself, and what the ABI says about its table is its
//!   own constant (`abi::sysent::INFO`), which the image's boot glue prints. Decision 47's,
//!   52's and 53's registration of the ABI's handler is amended: the ABI registers nothing.
//! - Decision 56 (E1, U5): the forked child's first return was a second registered function,
//!   which `hal` called on the child's registers at its first run; now it is
//!   data, [`ChildReturn`], written while `cpu_fork` builds the child's frame: no call goes
//!   upward for it.

/// A general register of the machine, by its name (arm64: `Gpr::X0` .. `Gpr::X29`; the host
/// double has arm64's; amd64's at N10).
pub use crate::arch::current::gpr::Gpr;
/// The trapping thread's user registers, for the handler's call: read a set of registers,
/// write the return register with the error flag, restart the system call.
pub use crate::arch::current::syscall::UserRegs;

/// `register_t`: a machine register as a system call sees it (64 bits on both machines).
pub use sysdefs::base::types::Register;

/// The six argument registers of a system call, in the C ABI's order.
pub type SysArgs = [Register; 6];

/// `retval[2]`: the two return registers.
pub type Rval = [Register; 2];

/// A system call's result as the trap sees it: `Ok(())` or an errno value.
pub type SysResult = Result<(), i32>;

/// A forked child's first return from the system call that made it, as data: which register
/// holds the value, the value, and the error flag (arm64: the carry). The convention's (the
/// ABI's constant); `hal` writes it into the child's trap frame while `cpu_fork` builds it
/// (`hal::cpu::thread::thread_setup`), the C's `child_return` writes done before the child
/// runs (nothing another thread can read sees the frame in between: the child has not run,
/// and its registers are read only once it stopped itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildReturn {
    /// The return register (arm64 `x0`).
    pub reg: Gpr,
    /// What `fork` returns in the child.
    pub value: Register,
    /// The error flag (arm64's carry, `PSR_C`): set for an error.
    pub error: bool,
}

/// `ENOSYS` (`<sys/errno.h>`): what an unanswered system call returns.
pub const ENOSYS: i32 = sysdefs::base::errno::Errno::ENOSYS.as_i32();

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enosys_is_the_c_value() {
        assert_eq!(ENOSYS, 78);
    }
}
/* </TESTS> */
