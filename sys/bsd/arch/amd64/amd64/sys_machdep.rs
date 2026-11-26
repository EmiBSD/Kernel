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
 * Copyright (c) 1998 The NetBSD Foundation, Inc.
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
/* </LICENSES> */

/* <CODE> */
//! `sysarch(2)` for amd64 (TD3).
//!
//! Upstream: sys/arch/amd64/amd64/sys_machdep.c @ 3ce1f3f79392
//!
//! One operation, `AMD64_IOPL`: the superuser, at a security level of 0 or less, sets or
//! clears the I/O privilege level of its own trap frame (`PSL_IOPL`), which lets X servers
//! reach the VGA ports. Every other `op` is EINVAL; a pledged process is refused through
//! `pledge_fail`. Reached from the system call table through `machine::sys_sysarch`.
//!
//! ## Deviations
//! - `option APERTURE` (`allowaperture`) is not configured, as in a kernel without it: the
//!   `#else` branch, EPERM above security level 0.

use core::sync::atomic::Ordering;

use crate::arch::amd64::include::psl::PSL_IOPL;
use crate::kern::kern_pledge::pledge_fail;
use crate::kern::kern_prot::suser;
use crate::kern::kern_sysctl::SECURELEVEL;
use crate::machine::copy::copyin_obj;
use crate::sys::errno::Errno;
use crate::sys::proc::{PS_PLEDGE, Proc};
use crate::sys::syscallargs::SysSysarchArgs;
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::Register;

/// `AMD64_IOPL` (`<machine/sysarch.h>`): set or clear the I/O privilege level.
pub const AMD64_IOPL: i32 = 2;

/// `amd64_iopl`: `struct amd64_iopl_args { int iopl; }` is read from `args`; nonzero sets
/// `PSL_IOPL` in the caller's saved flags, zero clears it.
fn amd64_iopl(p: &Proc, args: usize) -> Result<(), Errno> {
    suser(p)?;

    if SECURELEVEL.load(Ordering::Relaxed) > 0 {
        return Err(Errno::EPERM);
    }

    let iopl: i32 = copyin_obj(args)?;

    // SAFETY: the current thread's trap frame, which the system call entry recorded in
    // `md_regs` (as `sys_sigreturn` uses it); this thread is the only one running on it.
    let tf = unsafe { &mut *p.p_md.md_regs.get() };
    if iopl != 0 {
        tf.tf_rflags |= PSL_IOPL as i64;
    } else {
        tf.tf_rflags &= !(PSL_IOPL as i64);
    }

    Ok(())
}

/// `sys_sysarch`: the machine-dependent operations of `sysarch(2)`.
pub fn sys_sysarch(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSysarchArgs = sysargs(v);

    if p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0 {
        return Err(pledge_fail(p, Errno::EINVAL, 0));
    }

    match uap.op.get() {
        AMD64_IOPL => amd64_iopl(p, uap.parms.get() as usize),
        _ => Err(Errno::EINVAL),
    }
}
/* </CODE> */
