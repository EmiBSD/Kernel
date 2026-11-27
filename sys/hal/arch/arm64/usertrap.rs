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
 * Copyright (c) 2014 Andrew Turner
 * All rights reserved.
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
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
//! A trap from user mode, the machine's half (decision 56): the token the EL0 entry's assembly
//! (`exception.S`'s `handle_el0_*`) passes with the trap frame it saved ([`Arrival`]) to the
//! one function it calls (`entry`'s, decision 58), and what may be done with the frame of a
//! synchronous trap once its prologue ran ([`SyncTrap`]): the C's `do_el0_sync` up to its
//! `switch`, `svc_handler`'s mechanics, the FPU traps, the reads of the frame a signal needs,
//! the branch predictor flush; the IRQ and FIQ entries from user mode and the SError.
//!
//! Upstream: sys/arch/arm64/arm64/trap.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/syscall.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/trap.rs@e8249449d905
//! LZ: sys/arch/arm64/arm64/syscall.rs@e8249449d905
//!
//! **The arrival premise.** An [`Arrival`] has no constructor: only the assembly of an EL0
//! entry makes one, by passing in `x0` the trap frame it just saved at the top of the running
//! thread's kernel stack to the handler it calls (an `extern "C"` function whose parameter is
//! the `Arrival`, `#[repr(transparent)]` over the frame's pointer). So the mechanics that rest
//! on "this is the frame of a trap from user mode, taken on this CPU, being handled now" (the
//! frame recorded as the thread's `pcb_tf`, the interrupts enabled, the barrier after an `svc`
//! skipped, the interrupt controller's dispatch inside the frame's borrow) can be reached only
//! by a handler the assembly entered, once per trap: each method consumes the token
//! (decision 56's P13), and [`SyncTrap`], made only by [`Arrival::begin`], is the
//! synchronous trap's frame for the rest of its handler. Neither is `Clone`, `Send` or `Sync`.
//!
//! Every access to the frame is one borrow of the running thread's `UserFrame` for a closure
//! of this crate's own (`trap.rs`'s frame premise); nothing here holds the frame across a call
//! out of `hal`.
//!
//! ## Redesign
//! - Decision 56 (E2): `entry` (`sys/entry/arch/arm64/trap.rs`) is the one crate that names
//!   these types (`cargo xtask hal-check` H7): its handlers get the `Arrival`, classify the
//!   trap and call the methods; `hal` calls nothing back.
//! - Decision 56 (E2): the prologue of LZ's `do_el0_sync` (the syndrome and the fault
//!   address read, the interrupts enabled, `uvmexp.traps`, `pcb_tf = frame`), `svc_handler`'s
//!   machine half and the EL0 IRQ, FIQ and SError entries were `hal`'s handlers' bodies,
//!   entered by the assembly with a raw frame pointer; they are this token's methods, so the
//!   classification of the trap (the C's `switch`) can live above `hal` (in `entry`) and
//!   still reach them only with the frame the assembly saved. `svc_handler` is
//!   [`SyncTrap::svc`], which hands the ABI's handler its [`UserRegs`] instead of calling it.

use core::sync::atomic::Ordering;

use super::cpu::intr_enable;
use super::fpu;
use super::include::armreg::{
    ISS_MSR_CRM_MASK, ISS_MSR_CRM_SHIFT, ISS_MSR_CRN_MASK, ISS_MSR_CRN_SHIFT, ISS_MSR_DIR,
    ISS_MSR_OP0_MASK, ISS_MSR_OP0_SHIFT, ISS_MSR_OP1_MASK, ISS_MSR_OP1_SHIFT, ISS_MSR_OP2_MASK,
    ISS_MSR_OP2_SHIFT, ISS_MSR_RT_MASK, ISS_MSR_RT_SHIFT, PSR_I, esr_elx_exception, iss_msr_field,
};
use super::syscall::{BARRIER_LEN, UserRegs};
use super::trap::{UserFrame, dumpregs, esr_far, fiq, irq, serror};
use crate::assert::hal_panic;
use crate::cpu::curcpu;
use crate::cpu::usertrap::UserExit;
use crate::syscall::Register;
use crate::vm::uvmexp::UVMEXP;

/// A trap from user mode, as the EL0 entry's assembly hands it to its handler: the trap frame
/// it saved (the module's arrival premise). Consumed by the one method that handles the event.
#[repr(transparent)]
pub struct Arrival(UserFrame);

/// Which of the four EL0 vectors the CPU took (`x1` of the call `hal`'s EL0 handlers make,
/// decision 58): the values are `const` operands of `hal`'s own assembly, which passes one of
/// the four and nothing else (part of its contract, as "`x0` is the frame" is).
#[repr(u64)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserVector {
    /// A synchronous exception (`handle_el0_sync`): [`Arrival::begin`], then its class.
    Sync = 0,
    /// An IRQ (`handle_el0_irq`): [`Arrival::irq`].
    Irq = 1,
    /// An FIQ (`handle_el0_fiq`): [`Arrival::fiq`].
    Fiq = 2,
    /// An SError (`handle_el0_error`): [`Arrival::serror`], which does not return.
    SError = 3,
}

/// The signature of the one function `hal`'s EL0 handlers call with the trap (decision 58):
/// the frame's token first, the vector second, the exit gate's token back. Its definer pins
/// its function to this type (`const _: UserTrapFn = ..;`), so the compiler holds the
/// signature the assembly assumes.
pub type UserTrapFn = extern "C" fn(Arrival, UserVector) -> UserExit;

impl Arrival {
    /// `do_el0_sync`'s prologue: the running thread checked, the syndrome and the fault
    /// address read, the interrupts enabled, `uvmexp.traps` counted, the frame recorded as the
    /// thread's user context (`p->p_addr->u_pcb.pcb_tf = frame`). `panic(9)` with no thread
    /// on this CPU.
    pub fn begin(self) -> (SyncTrap, Syndrome) {
        if curcpu().curproc().is_null() {
            hal_panic(format_args!("do_el0_sync: no curproc"));
        }
        // Read the ESR and FAR registers to get the exception details.
        let (esr, far) = esr_far();

        intr_enable();
        UVMEXP.traps.fetch_add(1, Ordering::Relaxed);

        self.0.make_current();
        let class = esr_elx_exception(esr);
        (SyncTrap(self.0), Syndrome { class, esr, far })
    }

    /// `arm_cpu_irq` from user mode: the interrupt controller's dispatch, inside the frame's
    /// borrow (`trap.rs`'s frame premise).
    pub fn irq(self) {
        self.0.with(irq);
    }

    /// `arm_cpu_fiq` from user mode, as [`Self::irq`].
    pub fn fiq(self) {
        self.0.with(fiq);
    }

    /// `do_el0_error`: an SError taken in user mode, reported (the frame's pc, the syndrome,
    /// the CPU's own report), then `panic(9)`.
    pub fn serror(self) -> ! {
        let elr = self.0.with(|tf| tf.tf_elr);
        serror(elr);
        hal_panic(format_args!("do_el0_error"));
    }
}

/// `ESR_EL1` and `FAR_EL1` of a synchronous trap, as its prologue read them, with the
/// exception class.
#[derive(Clone, Copy, Debug)]
pub struct Syndrome {
    /// The exception class (`ESR_ELx_EXCEPTION(esr)`): what the trap's classification
    /// matches on.
    pub class: u32,
    /// The exception syndrome (`esr`): the class (`ESR_ELx_EXCEPTION`) and its details.
    pub esr: u64,
    /// The fault address (`far`), meaningful for an abort.
    pub far: u64,
}

/// The frame of a synchronous trap from user mode once [`Arrival::begin`] ran: what its
/// handler may do with it.
pub struct SyncTrap(UserFrame);

impl SyncTrap {
    /// `frame->tf_elr`: the user pc of the trapping instruction (the `sv.sival_ptr` of the
    /// signals the C sends at the pc).
    pub fn pc(&self) -> usize {
        self.0.with(|tf| tf.tf_elr as usize)
    }

    /// `frame->tf_sp`: the user stack pointer (`PROC_STACK(p)`, an alignment fault's address).
    pub fn sp(&self) -> usize {
        self.0.with(|tf| tf.tf_sp as usize)
    }

    /// The unknown exception's report: `exception <class> esr_el1 <esr>` and the registers
    /// (`dumpregs`).
    pub fn dump(&self, exception: u32, esr: u64) {
        crate::prf::printf(format_args!("exception {exception:x} esr_el1 {esr:x}\n"));
        self.0.with(|tf| dumpregs(tf));
    }

    /// `curcpu()->ci_flush_bp()`: the branch predictor flushed as the running CPU's choice
    /// says (`hal::cpu::trap::CpuHooks`).
    #[allow(
        clippy::unused_self,
        reason = "asked through the trap so that only the handler a trap from user mode entered can ask"
    )]
    pub fn flush_bp(&self) {
        crate::cpu::trap::flush_bp();
    }

    /// `fpu_load(p)`: the FP trap of the thread's first FP instruction since its last switch.
    #[allow(
        clippy::unused_self,
        reason = "asked through the trap so that only the handler a trap from user mode entered can ask"
    )]
    pub fn fp_load(&self) {
        fpu::load_current();
    }

    /// `sve_load(p)`: the SVE trap.
    #[allow(
        clippy::unused_self,
        reason = "asked through the trap so that only the handler a trap from user mode entered can ask"
    )]
    pub fn sve_load(&self) {
        fpu::sve_load_current();
    }

    /// `emulate_msr`'s decode of a trapped `msr`/`mrs` (`EXCP_MSR`) with syndrome `esr`: a
    /// read (`ISS_MSR_DIR`) of the non-debug identification space (op0 3, op1 0, CRn 0), as the
    /// register's CRm and op2 and the destination; `None` for any other access (a write, a
    /// debug or another system register), which the kernel answers with `SIGILL`.
    #[allow(
        clippy::unused_self,
        reason = "asked through the trap so that only the handler a trap from user mode entered can ask"
    )]
    pub fn id_read(&self, esr: u64) -> Option<IdRead> {
        let field = |mask, shift| iss_msr_field(esr, mask, shift);

        // Only emulate reads.
        if esr & ISS_MSR_DIR == 0 {
            return None;
        }

        // Only emulate non-debug System register access.
        if field(ISS_MSR_OP0_MASK, ISS_MSR_OP0_SHIFT) != 3
            || field(ISS_MSR_OP1_MASK, ISS_MSR_OP1_SHIFT) != 0
            || field(ISS_MSR_CRN_MASK, ISS_MSR_CRN_SHIFT) != 0
        {
            return None;
        }

        // The fields are 4, 3 and 5 bits wide: they fit.
        Some(IdRead {
            crm: field(ISS_MSR_CRM_MASK, ISS_MSR_CRM_SHIFT) as u8,
            op2: field(ISS_MSR_OP2_MASK, ISS_MSR_OP2_SHIFT) as u8,
            rt: field(ISS_MSR_RT_MASK, ISS_MSR_RT_SHIFT) as u8,
        })
    }

    /// `emulate_msr`'s answer to the read `r`: `value` in its destination register (`x0` to
    /// `x29`; `lr` for 30; nothing for 31, the zero register) and the pc past the `mrs`; no
    /// flag of `tf_spsr` changes. Consumes the trap: a read is answered once.
    pub fn id_answer(self, r: IdRead, value: u64) {
        let rt = usize::from(r.rt);
        self.0.with(|tf| {
            if rt < 30 {
                tf.tf_x[rt] = value as Register;
            } else if rt == 30 {
                tf.tf_lr = value as Register;
            }
            tf.tf_elr += 4;
        });
    }

    /// `svc_handler`'s machine half: `uvmexp.syscalls` counted, the speculation barrier after
    /// the `svc` skipped and the interrupts the user had re-enabled, in one borrow of the frame
    /// before the call; then the trapping thread's registers for the ABI's handler, which
    /// reads the number and the arguments, answers the call and writes the result (the
    /// convention is the ABI's, decision 53). Consumes the trap: a system call is answered
    /// once.
    pub fn svc(self) -> UserRegs {
        UVMEXP.syscalls.fetch_add(1, Ordering::Relaxed);

        let user_irqs = self.0.with(|tf| {
            // Skip over speculation-blocking barrier.
            tf.tf_elr += BARRIER_LEN;
            tf.tf_spsr as u64 & PSR_I == 0
        });

        // Re-enable interrupts if they were enabled previously
        if user_irqs {
            intr_enable();
        }

        // The frame is not borrowed across the handler's call: it borrows it for each of
        // `UserRegs`' methods, and what it calls may read or rewrite it through `pcb_tf`
        // (`sigreturn`, `execve`'s `setregs`, `pin_check`'s `PROC_PC`).
        UserRegs(self.0)
    }
}

/// A trapped read of an identification register (`mrs xN, S3_0_C0_<CRm>_<op2>`), decoded by
/// [`SyncTrap::id_read`]: which register (`crm`, `op2`), for the kernel's list of the reads it
/// answers, and the destination, which only [`SyncTrap::id_answer`] writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdRead {
    /// CRm (0..=15).
    pub crm: u8,
    /// op2 (0..=7).
    pub op2: u8,
    /// The destination register (Rt, 0..=31).
    rt: u8,
}

/// `READ_SPECIALREG(midr_el1)`: this CPU's model, which a program reading `MIDR_EL1` is told.
pub fn midr() -> u64 {
    let v: u64;
    // SAFETY: `mrs` of `MIDR_EL1`, a read-only identification register: no memory, no
    // stack, no flags.
    unsafe {
        core::arch::asm!("mrs {}, midr_el1", out(reg) v, options(nomem, nostack, preserves_flags));
    };
    v
}
/* </CODE> */
