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
 * Copyright (c) 1998, 1999 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Jason R. Thorpe of the Numerical Aerospace Simulation Facility,
 * NASA Ames Research Center.
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
//! Kernel thread handling: `kern/kern_kthread.c`, `bsd`'s half.
//!
//! Upstream: sys/kern/kern_kthread.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_kthread.rs@3c62ede68802
//!
//! `kthread_create` (over `fork1`, which is `bsd`'s until N4.6) and `kthread_exit` (over
//! `exit1`). The deferred creations (`kthread_create_deferred`, `kthread_run_deferred_queue`,
//! `kthread_create_now`) are `kernel::thread`'s since N4.5a, re-exported here under their
//! names.
//!
//! ## Deviations
//! - `kthread_create` returns the new thread (`Result<&Proc, Errno>`) instead of an `int`
//!   plus an out-pointer.
//!
//! ## Redesign
//! - LZ kept the deferred queue here, a `SIMPLEQ` of `malloc`'d entries behind an
//!   `unsafe impl Sync`; it is `kernel::thread`'s `VecDeque` inside a mutex, where the
//!   kernel's own deferred creations wait in the same order.

use core::ffi::c_void;

use crate::kern::init_main::PROC0;
use crate::kern::kern_exit::exit1;
use crate::kern::kern_fork::fork1;
use crate::kern::subr_prf::{Str, panic, printf};
use crate::machine::cpu::curproc;
use crate::sys::errno::Errno;
use crate::sys::proc::{
    EXIT_NORMAL, FORK_NOZOMBIE, FORK_SHAREFILES, FORK_SHAREVM, FORK_SYSTEM, Proc,
};
use crate::sys::systm::{kernel_lock, kernel_unlock};

pub use kernel::thread::{KTHREAD_CREATE_NOW, kthread_create_deferred, kthread_run_deferred_queue};

/// `kthread_create`: fork a kernel thread. Any process can request this to be done. The VM
/// space and limits, etc. will be shared with proc0. Returns the new thread (the C's
/// `*newpp`).
pub fn kthread_create(
    func: fn(*mut c_void),
    arg: *mut c_void,
    name: &[u8],
) -> Result<&'static Proc, Errno> {
    kernel_lock(); // KERNEL_LOCK()

    // First, create the new process. Share the memory, file descriptors and don't leave the
    // exit status around for the parent to wait for.
    let p = match fork1(
        &PROC0,
        FORK_SHAREVM | FORK_SHAREFILES | FORK_NOZOMBIE | FORK_SYSTEM,
        func,
        arg,
    ) {
        Ok(p) => p,
        Err(error) => {
            kernel_unlock(); // KERNEL_UNLOCK()
            return Err(error);
        }
    };

    // Name it as specified.
    p.process().set_comm(name);

    kernel_unlock(); // KERNEL_UNLOCK()

    // All done!
    Ok(p)
}

/// `kthread_exit`: cause a kernel thread to exit. Assumes the exiting thread is the current
/// context.
pub fn kthread_exit(ecode: i32) -> ! {
    // XXX What do we do with the exit code? Should we even bother with it? The parent
    // (proc0) isn't going to do much with it.
    if ecode != 0
        && let Some(p) = curproc()
    {
        printf(format_args!(
            "WARNING: thread `{}' ({}) exits with status {}\n",
            Str(&p.process().comm()),
            p.p_tid.get(),
            ecode
        ));
    }

    let Some(p) = curproc() else {
        panic(format_args!("kthread_exit: no curproc"));
    };
    exit1(p, ecode, 0, EXIT_NORMAL)
}
/* </CODE> */
