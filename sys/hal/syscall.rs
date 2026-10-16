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
//! The system call dispatch point: the one place both architectures' traps hand a system
//! call to, and the registry of who answers it.
//!
//! Upstream: sys/arch/amd64/amd64/trap.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/syscall.c @ 3ce1f3f79392
//! LZ: sys/arch/amd64/amd64/trap.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/syscall.rs@3c62ede68802
//!
//! The traps keep everything the C does around the call (the counters, the frame, the carry
//! bit, the restart, `mi_syscall_return`); what they hand here is the number, the six
//! argument registers and the two return registers, and what comes back is the C's `error`:
//! `Ok(())`, or the errno value (`ERESTART` is -1, `EJUSTRETURN` -2), as an `i32` because
//! this crate sits below every crate that names `Errno`.
//!
//! Two answers can be registered, each once, at boot (docs/ZERO_UNSAFE.md 0.1, the
//! sequence diagram): the module's handler (`abi-openbsd.ko` registers it through
//! `KernelApi` when `hal::module` loads it) and the `Legacy` fallback (`bsd`'s `main`
//! registers the LZ path: `sysent[]` and `mi_syscall`). [`dispatch`] calls the handler when
//! there is one, the fallback otherwise; nothing registered at all answers `ENOSYS`, which
//! only a kernel that skipped its own registration can see.
//!
//! ## Redesign
//! - LZ's two traps each chose the `sysent[]` entry and called `mi_syscall` (amd64
//!   `syscall`, arm64 `svc_handler`); that choice is now this module's, through two
//!   registered functions, so the system call table can live in the ABI module (decision 22)
//!   and `bsd`'s table becomes the `Legacy` fallback until every entry is `Native`.
//! - The registry is a pair of init-once cells written by registration, never by a store a
//!   caller can repeat: a second registration is refused, so a system call never sees a
//!   handler change under it.

use crate::module::once::Once;

/// `register_t`: a machine register as a system call sees it (64 bits on both machines).
pub type Register = isize;

/// The six argument registers of a system call, in the C ABI's order.
pub type SysArgs = [Register; 6];

/// `retval[2]`: the two return registers.
pub type Rval = [Register; 2];

/// A system call's result as the trap sees it: `Ok(())` or an errno value.
pub type SysResult = Result<(), i32>;

/// A function that answers system calls: the number, the arguments, the return registers.
pub type SyscallFn = fn(Register, &SysArgs, &mut Rval) -> SysResult;

/// `ENOSYS` (`<sys/errno.h>`): what an unanswered system call returns.
pub const ENOSYS: i32 = crate::sys::errno::Errno::ENOSYS.as_i32();

/// What the registered handler says about itself, for the boot line `hal::module` prints.
#[repr(C)]
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
    info: HandlerInfo,
}

/// The module's handler, once `abi_init` registered it.
static HANDLER: Once<Handler> = Once::new();

/// The `Legacy` fallback, once `bsd`'s `main` registered it.
static LEGACY: Once<SyscallFn> = Once::new();

/// Registers the `Legacy` fallback: the LZ system call path, which answers every entry the
/// module's table marks `Legacy`. Once; a second registration is refused and returned.
pub fn set_legacy(f: SyscallFn) -> Result<(), SyscallFn> {
    LEGACY.set(f)
}

/// Registers the module's handler and what it says about itself. Once; a second
/// registration is refused and returned.
pub fn set_handler(call: SyscallFn, info: HandlerInfo) -> Result<(), SyscallFn> {
    HANDLER.set(Handler { call, info }).map_err(|h| h.call)
}

/// What the registered handler said about itself; `None` before it registered.
pub fn handler_info() -> Option<HandlerInfo> {
    HANDLER.get().map(|h| h.info)
}

/// Calls the `Legacy` fallback (what a `Legacy` entry of the module's table reaches).
pub fn legacy(code: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
    route(None, LEGACY.get().copied(), code, args, rval)
}

/// The dispatch point: what both traps call where the C chose the `sysent[]` entry and
/// called `mi_syscall`.
pub fn dispatch(code: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
    route(
        HANDLER.get().map(|h| h.call),
        LEGACY.get().copied(),
        code,
        args,
        rval,
    )
}

/// The choice itself, over what is registered: the handler, else the fallback, else
/// `ENOSYS`.
fn route(
    handler: Option<SyscallFn>,
    legacy: Option<SyscallFn>,
    code: Register,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    match (handler, legacy) {
        (Some(f), _) | (None, Some(f)) => f(code, args, rval),
        (None, None) => Err(ENOSYS),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn module(code: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = code * 10 + args[0];
        Ok(())
    }

    fn fallback(code: Register, _args: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[1] = code;
        Err(-1) // ERESTART travels as its raw value
    }

    #[test]
    fn the_handler_answers_when_registered() {
        let mut rval = [0; 2];
        let r = route(
            Some(module as SyscallFn),
            Some(fallback as SyscallFn),
            4,
            &[2, 0, 0, 0, 0, 0],
            &mut rval,
        );
        assert_eq!(r, Ok(()));
        assert_eq!(rval, [42, 0]);
    }

    #[test]
    fn the_fallback_answers_without_a_handler() {
        let mut rval = [0; 2];
        let r = route(None, Some(fallback as SyscallFn), 5, &[0; 6], &mut rval);
        assert_eq!(r, Err(-1));
        assert_eq!(rval, [0, 5]);
    }

    #[test]
    fn nothing_registered_is_enosys() {
        let mut rval = [7, 7];
        assert_eq!(route(None, None, 1, &[0; 6], &mut rval), Err(ENOSYS));
        assert_eq!(rval, [7, 7], "the return registers are left alone");
    }

    #[test]
    fn enosys_is_the_c_value() {
        assert_eq!(ENOSYS, 78);
    }
}
/* </TESTS> */
