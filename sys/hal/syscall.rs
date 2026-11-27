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
//! The system call entry point: where both architectures' traps hand a system call, and a
//! forked child's first return, to the ABI's handler, and the registration of that handler.
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
//! forked child's first return is the handler's [`ChildFn`] on the child's registers.
//! Nothing userland sends is interpreted here.
//!
//! One answer is registered, once, at boot (docs/ZERO_UNSAFE.md 0.1, the sequence diagram):
//! the ABI's, which `abi::abi_init` registers with [`set_handler`] when the image's boot glue
//! calls it, before `main()` and so before the first system call: the one upward call of
//! `hal`, like the panic handler (decision 52). With nothing registered a system call is
//! `panic(9)`: answering `ENOSYS` would be the convention's, and only a kernel that skipped
//! its own registration can get there (the boot glue panics first).
//!
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
//!   carry themselves; now it is `fn(&UserRegs)` called by [`enter`] (and [`child`] for a
//!   forked child), and the per-arch convention is the ABI's (`abi::arch::<arch>`).
//!   `dispatch` and its `ENOSYS` without a handler are gone.

use crate::assert::hal_panic;
use crate::sync::once::OnceStatic;

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

/// The ABI's system call handler: the trapping thread's user registers, for the call.
pub type SyscallFn = fn(&UserRegs);

/// The ABI's first return of a forked child: the child's user registers, before
/// `mi_child_return`.
pub type ChildFn = fn(&UserRegs);

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

/// What the registered handler says about itself, for the boot line the image's boot glue
/// prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandlerInfo {
    /// Entries in its system call table (`SYS_MAXSYSCALL`).
    pub syscalls: u32,
    /// How many of them answer in the kernel crate (`Native`); the others go to `Legacy`.
    pub native: u32,
}

/// The registered handler and what it said about itself.
#[derive(Clone, Copy)]
struct Handler {
    call: SyscallFn,
    child: ChildFn,
    info: HandlerInfo,
}

/// The ABI's handler, once `abi_init` registered it.
static HANDLER: OnceStatic<Handler> = OnceStatic::new();

/// Registers the ABI's system call handler, its forked child's first return and what it
/// says about itself. Once; a second registration is refused and its `call` returned.
pub fn set_handler(call: SyscallFn, child: ChildFn, info: HandlerInfo) -> Result<(), SyscallFn> {
    HANDLER
        .set(Handler { call, child, info })
        .map_err(|h| h.call)
}

/// What the registered handler said about itself; `None` before it registered.
pub fn handler_info() -> Option<HandlerInfo> {
    HANDLER.get().map(|h| h.info)
}

/// The registered handler, `panic(9)` for `what` without one.
fn handler(what: &str) -> &'static Handler {
    match HANDLER.get() {
        Some(h) => h,
        None => hal_panic(format_args!("{what}: no system call handler")),
    }
}

/// A system call: the trap's registers handed to the ABI's handler, where the C chose the
/// `sysent[]` entry and called `mi_syscall`.
#[cfg_attr(
    not(all(target_os = "none", target_arch = "aarch64")),
    allow(dead_code, reason = "the machine's trap code alone calls it")
)]
#[inline]
pub(crate) fn enter(regs: &UserRegs) {
    (handler("svc").call)(regs);
}

/// A forked child's first return: its registers handed to the ABI's [`ChildFn`].
#[cfg_attr(
    not(all(target_os = "none", target_arch = "aarch64")),
    allow(dead_code, reason = "the machine's trap code alone calls it")
)]
pub(crate) fn child(regs: &UserRegs) {
    (handler("child_return").child)(regs);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// A handler that writes `x8 * 10 + x0` into `x0`, the carry clear.
    fn call(regs: &UserRegs) {
        let [x0, x8] = regs.gprs([Gpr::X0, Gpr::X8]);
        regs.set_return(Gpr::X0, x8 * 10 + x0, false);
    }

    /// A child return that asks for a restart (to tell it from `call`).
    fn child_restart(regs: &UserRegs) {
        regs.restart();
    }

    fn other(_regs: &UserRegs) {}

    const INFO: HandlerInfo = HandlerInfo {
        syscalls: 3,
        native: 1,
    };

    /// One test for the registration: `HANDLER` is the process's, set once.
    #[test]
    fn the_registered_handler_answers_and_cannot_be_replaced() {
        assert_eq!(handler_info(), None);
        assert!(set_handler(call, child_restart, INFO).is_ok());
        assert_eq!(handler_info(), Some(INFO));
        // A second registration is refused and handed back, the first one kept.
        assert!(set_handler(other, other, INFO).is_err());
        assert_eq!(handler_info(), Some(INFO));

        let mut x = [0; 30];
        x[0] = 2;
        x[8] = 4;
        let regs = UserRegs::fake(x);
        enter(&regs);
        assert_eq!(regs.fake_regs()[0], 42);
        assert_eq!((regs.fake_writes(), regs.fake_restarts()), (1, 0));

        let regs = UserRegs::fake(x);
        child(&regs);
        assert_eq!((regs.fake_writes(), regs.fake_restarts()), (0, 1));
        assert_eq!(regs.fake_regs(), x);
    }

    #[test]
    fn enosys_is_the_c_value() {
        assert_eq!(ENOSYS, 78);
    }
}
/* </TESTS> */
