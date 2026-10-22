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

/*
 * Copyright (c) 1996 Theo de Raadt
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)sys_generic.c	8.5 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! `sys_generic.c`'s `sysent[]` rows: `read`, `readv`, `write`, `writev`, `ioctl` and `utrace`
//! (`kernel::descrip`'s since N4.3), `select`, `pselect`, `poll` and `ppoll`
//! (`kernel::select`'s since N4.4), and `selwakeup` under LZ's path.
//!
//! Upstream: sys/kern/sys_generic.c @ 3ce1f3f79392
//! LZ: sys/kern/sys_generic.rs@3c62ede68802
//!
//! ## Deviations
//! - `KTRACE` is not configured.
//!
//! ## Redesign
//! - N4.3: the generic read/write/ioctl/utrace bodies moved into `kernel::descrip` (and
//!   `kernel::uio`); N4.4: `select(2)`/`poll(2)` (`dopselect`, `doppoll`, the register and
//!   collect helpers, `pollout`, `selwakeup`) into `kernel::select`, over `kernel::event`'s
//!   thread poll kqueue. The rows here turn the system call arguments into the kernel's
//!   typed ones, with 0 `unsafe` (LZ: 3, the scan's setup and `POLL_LASTERR`). The signal
//!   mask of `pselect`/`ppoll` is copied in here and installed by `kern_sig`'s
//!   `dosigsuspend` at the kernel's call, the C's point.

use hal::user::UserPtr;
use kernel::api::SysResult;

use crate::kern::kern_descrip::native;
use crate::kern::kern_sig::dosigsuspend;
use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::errno::Errno;
use crate::sys::proc::Proc;
use crate::sys::selinfo::Selinfo;
use crate::sys::signal::Sigset;
use crate::sys::signalvar::sigcantmask;
use crate::sys::syscall::{SYS_poll, SYS_ppoll, SYS_pselect, SYS_select};
use crate::sys::syscallargs::{
    SysIoctlArgs, SysPollArgs, SysPpollArgs, SysPselectArgs, SysReadArgs, SysReadvArgs,
    SysSelectArgs, SysWriteArgs, SysWritevArgs,
};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::Register;

/// Read system call (`sysent[]`'s row: `kernel::descrip::sys_read`).
pub fn sys_read(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysReadArgs = sysargs(v);
    let n = kernel::descrip::sys_read(p, uap.fd.get(), uap.buf.get().addr(), uap.nbyte.get())?;
    retval[0] = n as Register;
    Ok(())
}

/// Scatter read system call (`sysent[]`'s row: `kernel::descrip::sys_readv`).
pub fn sys_readv(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysReadvArgs = sysargs(v);
    let n = kernel::descrip::sys_readv(
        p,
        uap.fd.get(),
        UserPtr::new(uap.iovp.get().addr()),
        uap.iovcnt.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Write system call (`sysent[]`'s row: `kernel::descrip::sys_write`).
pub fn sys_write(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysWriteArgs = sysargs(v);
    let n = kernel::descrip::sys_write(p, uap.fd.get(), uap.buf.get().addr(), uap.nbyte.get())?;
    retval[0] = n as Register;
    Ok(())
}

/// Gather write system call (`sysent[]`'s row: `kernel::descrip::sys_writev`).
pub fn sys_writev(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysWritevArgs = sysargs(v);
    let n = kernel::descrip::sys_writev(
        p,
        uap.fd.get(),
        UserPtr::new(uap.iovp.get().addr()),
        uap.iovcnt.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Ioctl system call (`sysent[]`'s row: `kernel::descrip::sys_ioctl`).
pub fn sys_ioctl(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysIoctlArgs = sysargs(v);
    kernel::descrip::sys_ioctl(p, uap.fd.get(), uap.com.get(), uap.data.get().addr())
}

/// Select system call (`sysent[]`'s row: `kernel::select::sys_select`).
pub fn sys_select(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSelectArgs = sysargs(v);
    let sets = [
        UserPtr::new(uap.r#in.get().addr()),
        UserPtr::new(uap.ou.get().addr()),
        UserPtr::new(uap.ex.get().addr()),
    ];
    let n = kernel::select::sys_select(p, uap.nd.get(), sets, UserPtr::new(uap.tv.get().addr()))?;
    retval[0] = n as Register;
    Ok(())
}

/// The signal mask at `umask` (`NULL`: none), copied in.
fn copyin_mask(umask: usize) -> Result<Option<Sigset>, Errno> {
    if umask == 0 {
        return Ok(None);
    }
    UserPtr::<Sigset>::new(umask).copyin().map(Some)
}

/// `pselect(2)` (`sysent[]`'s row: `kernel::select::dopselect`).
pub fn sys_pselect(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPselectArgs = sysargs(v);
    let sets = [
        UserPtr::new(uap.r#in.get().addr()),
        UserPtr::new(uap.ou.get().addr()),
        UserPtr::new(uap.ex.get().addr()),
    ];
    // The timeout is copied in and checked before the mask, as the C.
    let tsp = kernel::select::copyin_timespec(UserPtr::new(uap.ts.get().addr()))?;
    let mask = copyin_mask(uap.mask.get().addr())?;
    let sigsuspend = mask.map(|m| move || dosigsuspend(p, m & !sigcantmask()));
    let n = kernel::select::dopselect(
        p,
        uap.nd.get(),
        sets,
        tsp,
        sigsuspend.as_ref().map(|f| f as &dyn Fn()),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// `selwakeup(sip)`: do a wakeup when a selectable event occurs.
pub fn selwakeup(sip: &Selinfo) {
    kernel::select::selwakeup::<BsdProcGlue>(sip);
}

/// `poll(2)` (`sysent[]`'s row: `kernel::select::sys_poll`).
pub fn sys_poll(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPollArgs = sysargs(v);
    let n = kernel::select::sys_poll(
        p,
        UserPtr::new(uap.fds.get().addr()),
        uap.nfds.get(),
        uap.timeout.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// `ppoll(2)` (`sysent[]`'s row: `kernel::select::doppoll`).
pub fn sys_ppoll(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPpollArgs = sysargs(v);
    let tsp = kernel::select::copyin_timespec(UserPtr::new(uap.ts.get().addr()))?;
    let mask = copyin_mask(uap.mask.get().addr())?;
    let sigsuspend = mask.map(|m| move || dosigsuspend(p, m & !sigcantmask()));
    let n = kernel::select::doppoll(
        p,
        UserPtr::new(uap.fds.get().addr()),
        uap.nfds.get(),
        tsp,
        sigsuspend.as_ref().map(|f| f as &dyn Fn()),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// `KernelApi::event.select`.
pub fn native_select(
    nr: Register,
    nd: i32,
    sets: [usize; 3],
    tv: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        nd as Register,
        sets[0] as Register,
        sets[1] as Register,
        sets[2] as Register,
        tv as Register,
        0,
    ];
    native(nr, SYS_select, args, rval)
}

/// `KernelApi::event.pselect`.
pub fn native_pselect(
    nr: Register,
    nd: i32,
    sets: [usize; 3],
    ts: usize,
    mask: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        nd as Register,
        sets[0] as Register,
        sets[1] as Register,
        sets[2] as Register,
        ts as Register,
        mask as Register,
    ];
    native(nr, SYS_pselect, args, rval)
}

/// `KernelApi::event.poll`.
pub fn native_poll(
    nr: Register,
    fds: usize,
    nfds: u32,
    timeout: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fds as Register,
        nfds as Register,
        timeout as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_poll, args, rval)
}

/// `KernelApi::event.ppoll`.
pub fn native_ppoll(
    nr: Register,
    fds: usize,
    nfds: u32,
    ts: usize,
    mask: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fds as Register,
        nfds as Register,
        ts as Register,
        mask as Register,
        0,
        0,
    ];
    native(nr, SYS_ppoll, args, rval)
}

/// `utrace(2)`: a user record for the process's `ktrace(1)` trace. `KTRACE` is not
/// configured, so there is no trace to add to and the call succeeds, as the C's `#else`.
pub fn sys_utrace(curp: &Proc, _v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    // KTRACE: ktruser(curp, label, addr, len) (`kernel::descrip::sys_utrace`).
    kernel::descrip::sys_utrace(curp)
}
/* </CODE> */
