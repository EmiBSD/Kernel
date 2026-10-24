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
//! arm64's `syscall.c`: the `svc` system call entry and a forked child's first return.
//!
//! Upstream: sys/arch/arm64/arm64/syscall.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/syscall.rs@3c62ede68802
//!
//! The entry keeps what the C does around the call: the counter, the interrupts the user had,
//! the `svc` skipped with the speculation barrier after it, the number and the six argument
//! registers read from the frame, the return registers, the carry bit, the restart and
//! `EJUSTRETURN`; the call itself goes to `hal::syscall::dispatch` (the module's handler or
//! the `Legacy` fallback), and `mi_syscall_return` to the registered trap handler.
//!
//! ## Redesign
//! - N4.8b: the system call number is `x8` truncated to an `int`, as the C's `int code`
//!   (`syscall.c:46`; [`svc_handler`] explains why): LZ took the whole 64-bit register, so a
//!   number that differed only above the low 32 bits got `ENOSYS` (LZ's deviation, gap 2 of
//!   the brief, closed by the user's decision of 2026-10-22).
//! - N4.8b: from `bsd` into `hal`. `svc_handler` read `ci_curproc` through a raw pointer and
//!   called `bsd`'s `abi_module::syscall` and `mi_syscall_return` by name; here the running
//!   thread is checked in `hal::cpu`'s core, the call is `hal::syscall::dispatch`, and the
//!   machine-independent return is `hal::trap::TrapHandler::syscall_return`.
//! - `child_return(void *arg)` cast its argument back to the thread; here it takes nothing:
//!   the child is the running thread, whose trap frame is its pcb's `pcb_tf`
//!   (`hal::thread`'s `thread_setup` built it), and its machine-independent half is the trap
//!   handler's `child_return` (`mi_child_return`).
//! - The frame is the running thread's `UserFrame` (`trap.rs`'s frame premise), borrowed
//!   before and after the call, never across it: the call reaches the same frame through
//!   `pcb_tf` (`sigreturn`, `execve`'s `setregs`, `pin_check`'s `PROC_PC`). `child_return`'s
//!   frame write is a borrow of it too (LZ: a raw pointer write).

use core::sync::atomic::Ordering;

use super::cpu::intr_enable;
use super::include::armreg::{PSR_C, PSR_I};
use super::trap::UserFrame;
use crate::assert::hal_panic;
use crate::sys::errno::Errno;
use crate::syscall::{Register, Rval, SysArgs};
use crate::vm::uvmexp::UVMEXP;

/// `ERESTART` as the dispatch point returns it.
const ERESTART: i32 = Errno::ERESTART.as_i32();
/// `EJUSTRETURN` as the dispatch point returns it.
const EJUSTRETURN: i32 = Errno::EJUSTRETURN.as_i32();

/// `svc_handler`: a system call from EL0 (`do_el0_sync`, `EXCP_SVC`), on the trap frame of
/// the running thread.
///
/// The system call number is an identifier, not a count: the index into `sysent[]`,
/// assigned by OpenBSD in `sys/kern/syscalls.master` (from which libc's `syscall.h` is
/// generated) and part of the ABI; the low numbers date from the first Unix (exit 1,
/// fork 2, read 3). libc loads it into `x8` and executes `svc`; the CPU only changes
/// mode, and the entry saves `x8` in `tf_x[8]`. Every arm64 register is 64 bits wide,
/// but the number fits in 32 (there are about 331), and OpenBSD's arm64 entry reads it
/// into an `int` (`syscall.c:46`), dropping the high half; so does this one, so that a
/// malformed number behaves as on OpenBSD (the user's decision of 2026-10-22): its low 32
/// bits are the number (0 or negative, the C's `code <= 0`: `ENOSYS`), and `pin_check`
/// kills the process with `SIGABRT` unless the `svc` is at the site pinned for it (LZ took
/// the whole register and returned `ENOSYS`). amd64's entry keeps the whole register
/// (`trap.c:781`, a `register_t`, not truncated). diff-openbsd: `syscalls/entry#1`
/// (`difftest syscalls entry`), its line `int-syscall`.
pub(crate) fn svc_handler(frame: UserFrame) {
    if crate::cpu::curcpu().curproc().is_null() {
        hal_panic(format_args!("svc_handler: no curproc"));
    }

    UVMEXP.syscalls.fetch_add(1, Ordering::Relaxed);

    // One borrow of the frame before the call: whether the user had interrupts enabled,
    // the barrier skipped, the number and the arguments.
    let (user_irqs, code, args) = frame.with(|tf| {
        let user_irqs = tf.tf_spsr as u64 & PSR_I == 0;

        // Skip over speculation-blocking barrier.
        tf.tf_elr += 8;

        // The C's `int code = frame->tf_x[8]`: the low 32 bits, sign-extended (see above).
        let code: Register = tf.tf_x[8] as i32 as Register;
        let args: SysArgs = match tf.tf_x.first_chunk::<6>() {
            Some(a) => *a,
            None => [0; 6],
        };
        (user_irqs, code, args)
    });

    // Re-enable interrupts if they were enabled previously
    if user_irqs {
        intr_enable();
    }

    let mut rval: Rval = [0, 0];

    // The frame is not borrowed across the call: the call may read or rewrite it through
    // `pcb_tf` (`sigreturn`, `execve`'s `setregs`, `pin_check`'s `PROC_PC`).
    let error = crate::syscall::dispatch(code, &args, &mut rval);

    frame.with(|tf| match error {
        Ok(()) => {
            tf.tf_x[0] = rval[0];
            tf.tf_spsr &= !(PSR_C as Register); // carry bit
        }
        Err(ERESTART) => {
            // Reconstruct the pc to point at the svc.
            tf.tf_elr -= 12;
        }
        Err(EJUSTRETURN) => {
            // nothing to do
        }
        Err(e) => {
            tf.tf_x[0] = e as Register;
            tf.tf_spsr |= PSR_C as Register; // carry bit
        }
    });

    match crate::trap::handler() {
        Some(h) => h.syscall_return(code, error, &rval),
        None => hal_panic(format_args!("svc_handler: no trap handler")),
    }
}

/// `child_return`: the first thing a forked user thread runs, on itself: a `fork` return of 0
/// with the carry clear in its trap frame, the kernel lock its trampoline took released, then
/// `mi_child_return`.
pub fn child_return() {
    // The running thread's `pcb_tf`, which `thread_setup` set to the trap frame it wrote at
    // the top of this thread's u-area before the thread ever ran (`cpu_fork`'s).
    let Some(frame) = UserFrame::current() else {
        hal_panic(format_args!("child_return: no trap frame"));
    };
    frame.with(|tf| {
        tf.tf_x[0] = 0;
        tf.tf_spsr &= !(PSR_C as Register); // carry bit
    });

    // The kernel lock `proc_trampoline_mi` took for the new thread.
    crate::klock::kernel_unlock();

    match crate::trap::handler() {
        Some(h) => h.child_return(),
        None => hal_panic(format_args!("child_return: no trap handler")),
    }
}
/* </CODE> */
