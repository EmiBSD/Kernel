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
//! Kernel thread handling: `kern/kern_kthread.c`, `bsd`'s names over `kernel::thread`.
//!
//! Upstream: sys/kern/kern_kthread.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_kthread.rs@3c62ede68802
//!
//! An adapter (N4.6a): `kthread_create` is `kernel::thread::kthread_create_with` of a
//! `bsd` thread function and its argument (`kernel::fork::Start::Legacy`: the argument's
//! address, 0 standing for the new thread, the C's `arg ? arg : p`), and `kthread_exit` the
//! kernel's. The deferred creations (`kthread_create_deferred`,
//! `kthread_run_deferred_queue`, `kthread_create_now`) are `kernel::thread`'s since N4.5a,
//! re-exported here under their names.
//!
//! ## Deviations
//! - `kthread_create` returns the new thread (`Result<&'static Proc, Errno>`) instead of an
//!   `int` plus an out-pointer: the kernel lends it before it can run, `bsd` keeps it as its
//!   placed `&'static` (`ProcExt::as_static`, N4.8).
//!
//! ## Redesign
//! - LZ kept the deferred queue here, a `SIMPLEQ` of `malloc`'d entries behind an
//!   `unsafe impl Sync`; it is `kernel::thread`'s `VecDeque` inside a mutex, where the
//!   kernel's own deferred creations wait in the same order.
//! - N4.6a: the creation and the exit are the kernel's (`kernel::thread::kthread_create`,
//!   over `kernel::fork::fork1`); LZ's `fork1(&proc0, ..., func, arg)` with the function
//!   pointer filed by `cpu_fork` became a `Start::Legacy` in two words.

use core::ffi::c_void;

use kernel::fork::Start;

use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::sys::errno::Errno;
use crate::sys::proc::{Proc, ProcExt};

pub use kernel::thread::{KTHREAD_CREATE_NOW, kthread_create_deferred, kthread_run_deferred_queue};

/// `kthread_create`: fork a kernel thread running `func(arg)` (a NULL `arg` is the new
/// thread itself), named `name`, sharing proc0's address space and descriptors and leaving
/// no zombie. Returns the new thread (the C's `*newpp`).
pub fn kthread_create(
    func: fn(*mut c_void),
    arg: *mut c_void,
    name: &[u8],
) -> Result<&'static Proc, Errno> {
    let mut made: Option<&'static Proc> = None;
    kernel::thread::kthread_create_with::<BsdProcGlue>(
        name,
        Start::Legacy(func, arg.expose_provenance()),
        &mut |p| made = Some(p.as_static()),
    )?;
    match made {
        Some(p) => Ok(p),
        None => panic(format_args!("kthread_create: no thread lent")),
    }
}

/// `kthread_exit`: cause a kernel thread to exit. Assumes the exiting thread is the current
/// context.
pub fn kthread_exit(ecode: i32) -> ! {
    kernel::thread::kthread_exit::<BsdProcGlue>(ecode)
}
/* </CODE> */
