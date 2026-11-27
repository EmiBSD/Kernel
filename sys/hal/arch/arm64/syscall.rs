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
//! arm64's `syscall.c`, its machine half: the `svc` trap's mechanics around the ABI's
//! handler; [`UserRegs`], the view of the trapping thread's user registers that handler gets;
//! and the write of a return register with the carry, which a forked child's frame also
//! takes (`switch.rs`'s `build_frames`).
//!
//! Upstream: sys/arch/arm64/arm64/syscall.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/syscall.rs@3c62ede68802
//!
//! The entry keeps what is the machine's: the running thread checked, the counter, the
//! interrupts the user had, the speculation barrier after the `svc` skipped. Everything that
//! is OpenBSD's system call convention (which register holds the number, the six argument
//! registers, `x0` and the carry for the result, when to rewind to the `svc`, `EJUSTRETURN`,
//! the forked child's `x0 = 0`) is the ABI's (`abi::arch::arm64`, decision 53), which then
//! calls `mi_syscall_return` through `kernel::api`.
//!
//! ## Redesign
//! - N4.8b: the system call number is `x8` truncated to an `int`, as the C's `int code`
//!   (`syscall.c:46`): LZ took the whole 64-bit register, so a number that differed only above
//!   the low 32 bits got `ENOSYS` (LZ's deviation, gap 2 of the N4.8 brief, closed by the
//!   user's decision of 2026-10-22). Since decision 53 that read is the ABI's.
//! - N4.8b: from `bsd` into `hal`. `svc_handler` read `ci_curproc` through a raw pointer and
//!   called `bsd`'s `abi_module::syscall` and `mi_syscall_return` by name; here the running
//!   thread is checked in `hal::cpu`'s core.
//! - Decision 53 (N7.abi2, the user, 2026-11-26): `svc_handler` read the number and the
//!   arguments, called `hal::syscall::dispatch`, wrote `x0` and the carry or rewound the pc,
//!   and called `mi_syscall_return` through the trap handler; now it hands the ABI's handler
//!   (`hal::syscall::enter`) a [`UserRegs`] and the convention is the ABI's. `child_return`'s
//!   two frame writes went the same way (`hal::syscall::child`, until E1). What the ABI can do to the
//!   frame is [`UserRegs`]' three methods: read general registers, write one with the error
//!   flag (the carry, no other bit of `tf_spsr`), and move the pc back over the `svc` and its
//!   barrier; it never writes a pc of its own choice, the stack pointer or the mode.
//! - Decision 56 (E1, U5): `child_return` is gone from here. Its two frame writes (`x0 = 0`,
//!   the carry cleared: the ABI's `hal::syscall::ChildReturn`) are made by [`set_return`]
//!   while `cpu_fork` builds the child's frame (`switch.rs`'s `build_frames`), before the
//!   child runs, and its `KERNEL_UNLOCK` and `mi_child_return` are the kernel's start's
//!   (`kernel::proc::fork`'s `child_return`): a child's first run calls nothing upward.
//! - The frame is the running thread's `UserFrame` (`trap.rs`'s frame premise), borrowed for
//!   one closure at a time and never across the ABI's call: the call reaches the same frame
//!   through `pcb_tf` (`sigreturn`, `execve`'s `setregs`, `pin_check`'s `PROC_PC`). A system
//!   call borrows it three times (the entry, the ABI's read, its write or rewind) where it
//!   was twice before decision 53, accepted until the N7 close measures it.

use core::sync::atomic::Ordering;

use super::cpu::intr_enable;
use super::gpr::{Gpr, NGPR};
use super::include::armreg::{PSR_C, PSR_I};
use super::include::frame::Trapframe;
use super::trap::UserFrame;
use crate::assert::hal_panic;
use crate::syscall::Register;
use crate::vm::uvmexp::UVMEXP;

/// The `svc` instruction's length.
const SVC_LEN: Register = 4;
/// The speculation barrier every `svc` of OpenBSD's libc (and `sigcode`) is followed by
/// (`dsb nsh; isb`), which the entry skips: the user resumes after it.
const BARRIER_LEN: Register = 8;

/// The user registers of the thread whose system call (or first return from `fork`) is
/// being answered: its trap frame, valid for the handler's call alone.
///
/// Not `Clone`, not `Copy`, no constructor outside `hal`, `!Send` and `!Sync` (the frame's
/// `NonNull`): the handler gets a `&UserRegs` that cannot outlive its call or reach another
/// thread. Each method is one borrow of the frame for a closure of `hal`'s own that calls
/// nothing (`UserFrame::with`'s premise).
pub struct UserRegs(UserFrame);

impl UserRegs {
    /// The registers `which`, read in one borrow of the frame.
    #[inline]
    pub fn gprs<const N: usize>(&self, which: [Gpr; N]) -> [Register; N] {
        self.0.with(|tf| which.map(|r| tf.tf_x[r.index()]))
    }

    /// Writes register `reg` with `value` and the error flag, the carry (`PSR_C` of the saved
    /// `tf_spsr`, set for `error`, else cleared; no other bit), in one borrow of the frame.
    #[inline]
    pub fn set_return(&self, reg: Gpr, value: Register, error: bool) {
        self.0.with(|tf| set_return(tf, reg, value, error));
    }

    /// Moves the saved pc back to the `svc`, over the barrier the entry skipped: the system
    /// call runs again when the thread returns to user mode (`ERESTART`).
    #[inline]
    pub fn restart(&self) {
        // Reconstruct the pc to point at the svc.
        self.0.with(|tf| tf.tf_elr -= SVC_LEN + BARRIER_LEN);
    }
}

/// Register `reg` of `tf` set to `value` and the error flag, the carry (`PSR_C` of the saved
/// `tf_spsr`, set for `error`, else cleared; no other bit): a system call's return
/// ([`UserRegs::set_return`]) and a forked child's (`switch.rs`'s `build_frames`, from
/// `hal::syscall::ChildReturn`).
#[inline]
pub(super) fn set_return(tf: &mut Trapframe, reg: Gpr, value: Register, error: bool) {
    tf.tf_x[reg.index()] = value;
    if error {
        tf.tf_spsr |= PSR_C as Register; // carry bit
    } else {
        tf.tf_spsr &= !(PSR_C as Register); // carry bit
    }
}

/// `svc_handler`: a system call from EL0 (`do_el0_sync`, `EXCP_SVC`), on the trap frame of
/// the running thread.
///
/// The machine's part of the C's handler; the convention (the number in `x8` truncated to
/// an `int`, the arguments in `x0`..`x5`, the result in `x0` with the carry, the restart,
/// `EJUSTRETURN`, `mi_syscall_return`) is the ABI's handler's, called with the frame as a
/// [`UserRegs`] after the barrier is skipped, so `pin_check` reads the pc past it
/// (`PROC_PC`, `syscall_mi.h`) as on OpenBSD.
pub(crate) fn svc_handler(frame: UserFrame) {
    if crate::cpu::curcpu().curproc().is_null() {
        hal_panic(format_args!("svc_handler: no curproc"));
    }

    UVMEXP.syscalls.fetch_add(1, Ordering::Relaxed);

    // One borrow of the frame before the call: whether the user had interrupts enabled, and
    // the barrier skipped.
    let user_irqs = frame.with(|tf| {
        // Skip over speculation-blocking barrier.
        tf.tf_elr += BARRIER_LEN;
        tf.tf_spsr as u64 & PSR_I == 0
    });

    // Re-enable interrupts if they were enabled previously
    if user_irqs {
        intr_enable();
    }

    // The frame is not borrowed across the call: the handler borrows it for each of
    // `UserRegs`' methods, and what it calls may read or rewrite it through `pcb_tf`
    // (`sigreturn`, `execve`'s `setregs`, `pin_check`'s `PROC_PC`).
    crate::syscall::enter(&UserRegs(frame));
}

// `Gpr` indexes `tf_x` whole: `tf_x` is exactly `[Register; NGPR]`.
const _: fn(&Trapframe) -> &[Register; NGPR] = |tf| &tf.tf_x;
/* </CODE> */
