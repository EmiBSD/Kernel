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
 * Copyright (c) 1995 Charles M. Hannum.  All rights reserved.
 * Copyright (c) 1982, 1986 The Regents of the University of California.
 * Copyright (c) 1989, 1990 William Jolitz
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department, and William Jolitz.
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
 *	@(#)vm_machdep.c	7.3 (Berkeley) 5/13/91
 */

/*
 *	Utah $Hdr: vm_machdep.c 1.16.1.1 89/06/23$
 */
/* </LICENSES> */

/* <CODE> */
//! arm64 `vm_machdep.c`: the machine-dependent part of creating and tearing down threads.
//!
//! Upstream: sys/arch/arm64/arm64/vm_machdep.c @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M5 (part b2) ports `cpu_fork` and `cpu_exit`; M10a added physio's
//! `kv_physwait`, `vmapbuf` and `vunmapbuf`, which N5.1 moved into the kernel crate
//! (`kernel::vm::km::KV_PHYSWAIT` and `PhysWindow`, one machine-independent window for every
//! machine, made by `kernel::vm::glue::DeviceLock::vmapbuf`).
//!
//! ## Deviations
//! - The pointer authentication keys (`pm_ap*key`) the C copies from the parent's pmap wait
//!   for user pmaps (M6): kernel threads share the kernel pmap.
//! - The switch frame's `sf_x19` holds the start word of `kernel::fork::Start` (for a `bsd`
//!   function, its slot in the kernel's table), which the trampoline hands back to
//!   `kernel::fork`'s registered start: no code address crosses into `hal`, whose
//!   `thread_setup` builds the frames (N4.5a). Since N4.6a the kernel's `fork1` calls
//!   `thread_setup` itself; `cpu_fork` stays for the machine contract (amd64's, N10).

use core::ffi::c_void;
use core::ptr;

use crate::machine::pmap::pmap_activate;
use crate::sys::proc::Proc;
use crate::sys::proc::ProcExt;

/// `cpu_fork`: finish a fork operation, with process `p2` nearly set up. Copy and update the
/// kernel stack and pcb, making the child ready to run, and marking it so that it can return
/// differently than the parent: `hal::thread::thread_setup` (the parent's FP state saved, the
/// pcb copied, the trap frame copied with the user stack and TCB, the switch frame into the
/// trampoline with `func`'s number and `arg`).
pub fn cpu_fork(
    p1: &Proc,
    p2: &Proc,
    stack: *mut u8,
    tcb: *mut u8,
    func: fn(*mut c_void),
    arg: *mut c_void,
) {
    // pm->pm_ap{ia,da,ib,db,ga}key = pm1's: user pmaps (M6).

    pmap_activate(p2);

    // Both words of the start: a NULL `arg` is the new thread itself (`words` puts `p2`'s
    // address in the second), as `kernel::fork`'s own `cpu_fork` passes them.
    let (start, arg) =
        kernel::fork::Start::Legacy(func, arg.expose_provenance()).words(ptr::from_ref(p2).addr());
    let h = hal::thread::sched_lock();
    hal::thread::thread_setup(
        &h,
        hal::pin::pin_static(p1.as_static()),
        hal::pin::pin_static(p2.as_static()),
        stack.addr(),
        tcb.expose_provenance(),
        start,
        arg,
    );
    drop(h);
}

/// `cpu_exit`: nothing to do on arm64.
pub fn cpu_exit(_p: &Proc) {}
/* </CODE> */
