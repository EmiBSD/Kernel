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

/*
 * Copyright (c) 2011 Dale Rahn <drahn@openbsd.org>
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
//! arm64's exception handlers: `trap.c` (the synchronous exceptions and the SErrors of EL1
//! and EL0), `ast.c` (the AST on the way back to user mode) and `intr.c`'s IRQ and FIQ
//! entries, called by the assembly of `exception.rs` with the trap frame it built.
//!
//! Upstream: sys/arch/arm64/arm64/trap.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/ast.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/intr.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/trap.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/ast.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/intr.rs@3c62ede68802
//!
//! The machine's decode is here: the syndrome and the fault address, which map a fault
//! belongs to (the top bit of the address, an unprivileged load or store, `pcb_onfault`), the
//! `pcb_onfault` recovery and its `ci_idepth == 0` rule, the instruction emulation of the ID
//! registers, the FPU and SVE traps, the frame's fix-ups. Every decision the C makes in
//! machine-independent code (the fault, the signal, `refreshcreds`, `userret`, `mi_ast`,
//! `sigexit`) is asked of the registered `hal::trap::TrapHandler`; the branch predictor
//! flush and the SError report of the CPU code still in `bsd` are `hal::trap::CpuHooks`;
//! the kernel debugger is `hal::trap::Debugger`; the interrupt controller's dispatch is
//! `hal::intr::IntrHardware`'s.
//!
//! **The frame premise.** Every `extern "C"` handler here is reached only from the assembly
//! of `exception.S` (`sym` operands, no symbol another object could call), with `x0` the trap
//! frame it just saved on this CPU's kernel stack: a valid, aligned `struct trapframe` the
//! assembly reads again only after the handler returns. The frame of an entry from user mode
//! (`do_el0_sync`, `do_el0_error`, `ast`, and the IRQ and FIQ entries from EL0,
//! `arm_cpu_irq_el0` and `arm_cpu_fiq_el0`) is the thread's user context, which `pcb_tf`
//! names (an entry from user mode starts on an empty kernel stack, so its frame is always at
//! the same place) and which the code those handlers call out to reads and writes
//! (`sendsig`, `sigreturn`, `PROC_PC`, `bsd`'s `setregs`): they hold it as a [`UserFrame`],
//! borrowed one closure at a time, never across such a call; an interrupt's dispatch runs
//! inside its borrow, so a handler that reached the frame again would be `panic(9)`, never a
//! second `&mut`. The kernel-mode handlers take `&mut Trapframe`: their frame lies below the
//! user frame on the kernel stack, and nothing names it but the handler.
//!
//! ## Deviations
//! - `do_el0_sync`'s `KERNEL_UNLOCK` after `sigexit` is not written: `sigexit` never returns.
//! - The `we_re_toast` path prints the syndrome and enters the debugger as the `DDB` build
//!   does, then panics with the same message as the non-`DDB` build: ddb-lite has no command
//!   loop to stay in, and returning would re-execute the faulting instruction (LZ's).
//! - `kdata_abort`'s unprivileged access with no thread running uses the kernel map where the
//!   C dereferences `p` (the trap handler's choice, `hal::trap::FaultSource::KernelUser`).
//!
//! ## Redesign
//! - N4.8b: from `bsd` into `hal`. LZ called `uvm_fault`, `trapsignal`, `userret`, ... by
//!   name and read `ci_curproc`/`ci_curpcb` through raw pointers; here the machine-independent
//!   calls go through `hal::trap::TrapHandler` (one registration), the running thread's pcb
//!   through `hal::thread`'s `with_curpcb` (its premise E5), and the handlers are entered by
//!   `sym` (no `#[no_mangle]`). `fault()`'s `panicstr` claim and `ci_panicbuf` are the
//!   CPU hooks' (`hal::trap::CpuHooks::fault`), `db_ktrap` the `Debugger` registration (absent:
//!   the C without `DDB`, `panic("No debugger in kernel")` for a kernel breakpoint).
//! - `ast.c`'s `ast()` and `intr.c`'s `arm_cpu_irq`/`arm_cpu_fiq` join the trap: the IRQ
//!   entry dispatches through `hal::intr::IntrHardware::dispatch_irq` (the controller's
//!   `arm_irq_dispatch`, which `bsd`'s GIC drivers register, N7).
//! - `cpu_id_aa64pfr0`, `pfr1`, `isar0`, `isar1`, `isar2` (`cpu.c`'s userland view of the ID
//!   registers, `emulate_msr`'s answers) are this module's statics, which `bsd`'s CPU code
//!   writes (`cpu_identify_cleanup`).
//! - N4.8d: `do_el0_sync` reads the frame's `elr` or `sp` in the arm that uses it, not once
//!   for every exception, and `udata_abort` hands `PROC_STACK(p)`, read from the frame it
//!   holds, to the kernel's `MAP_STACK` check in `hal::trap::FaultSource::User { sp }`: a user
//!   fault borrows the frame once instead of twice, a system call not at all here.

use core::fmt;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering, compiler_fence};

use super::cpu::intr_enable;
use super::fpu;
use super::include::armreg::{
    EXCP_BRANCH_TGT, EXCP_BRK, EXCP_DATA_ABORT, EXCP_DATA_ABORT_L, EXCP_FP_SIMD, EXCP_FPAC,
    EXCP_INSN_ABORT, EXCP_INSN_ABORT_L, EXCP_MSR, EXCP_PC_ALIGN, EXCP_SOFTSTP_EL0,
    EXCP_SOFTSTP_EL1, EXCP_SP_ALIGN, EXCP_SVC, EXCP_SVE, EXCP_TRAP_FP, EXCP_UNKNOWN,
    EXCP_WATCHPT_EL1, INSN_SIZE, ISS_BRK_COMMENT_MASK, ISS_DATA_DFSC_ALIGN, ISS_DATA_DFSC_MASK,
    ISS_MSR_CRM_MASK, ISS_MSR_CRM_SHIFT, ISS_MSR_CRN_MASK, ISS_MSR_CRN_SHIFT, ISS_MSR_DIR,
    ISS_MSR_OP0_MASK, ISS_MSR_OP0_SHIFT, ISS_MSR_OP1_MASK, ISS_MSR_OP1_SHIFT, ISS_MSR_OP2_MASK,
    ISS_MSR_OP2_SHIFT, ISS_MSR_RT_MASK, ISS_MSR_RT_SHIFT, accesstype, esr_elx_exception,
    iss_msr_field,
};
use super::include::frame::Trapframe;
use super::include::vmparam::VM_MAXUSER_ADDRESS;
use super::switch::with_curpcb;
use crate::assert::hal_panic;
use crate::cpu::{MAXCPUS, cpu_number, curcpu};
use crate::sys::errno::Errno;
use crate::sys::siginfo::{
    BUS_ADRALN, BUS_OBJERR, ILL_BTCFI, ILL_ILLOPC, SEGV_ACCERR, SEGV_MAPERR, TRAP_BRKPT, TRAP_TRACE,
};
use crate::sys::signal::{SIGBUS, SIGILL, SIGKILL, SIGSEGV, SIGTRAP};
use crate::sys::types::Register;
use crate::trap::{FaultSource, TrapHandler};
use crate::vm::param::trunc_page;
use crate::vm::uvmexp::UVMEXP;

/// `cpu_id_aa64pfr0`: the userland view of `ID_AA64PFR0_EL1` (`cpu.c`'s
/// `cpu_identify_cleanup`, which `bsd`'s CPU code runs), `emulate_msr`'s answer.
pub static CPU_ID_AA64PFR0: AtomicU64 = AtomicU64::new(0);
/// `cpu_id_aa64pfr1`.
pub static CPU_ID_AA64PFR1: AtomicU64 = AtomicU64::new(0);
/// `cpu_id_aa64isar0`.
pub static CPU_ID_AA64ISAR0: AtomicU64 = AtomicU64::new(0);
/// `cpu_id_aa64isar1`.
pub static CPU_ID_AA64ISAR1: AtomicU64 = AtomicU64::new(0);
/// `cpu_id_aa64isar2`.
pub static CPU_ID_AA64ISAR2: AtomicU64 = AtomicU64::new(0);

/// `READ_SPECIALREG(esr_el1)` and `READ_SPECIALREG(far_el1)`: the syndrome and the fault
/// address of the exception this CPU is handling.
#[inline]
fn esr_far() -> (u64, u64) {
    let (esr, far): (u64, u64);
    // SAFETY: `mrs` of `ESR_EL1` and `FAR_EL1` reads two system registers of this CPU: no
    // memory, no stack, no flags; reading them changes nothing.
    unsafe {
        core::arch::asm!(
            "mrs {}, esr_el1",
            "mrs {}, far_el1",
            out(reg) esr,
            out(reg) far,
            options(nomem, nostack, preserves_flags)
        );
    };
    (esr, far)
}

/// The registered trap handler; a trap that needs one before it is registered is
/// `panic(9)` (no user process runs before `bsd` registers it).
fn handler(what: &str) -> &'static dyn TrapHandler {
    match crate::trap::handler() {
        Some(h) => h,
        None => hal_panic(format_args!("{what}: no trap handler")),
    }
}

/// Whether a thread runs on this CPU (`curproc != NULL`).
#[inline]
fn have_curproc() -> bool {
    !curcpu().curproc().is_null()
}

/// Per CPU, whether a [`UserFrame`] is borrowed on it ([`UserFrame::with`]).
static USER_FRAME_BORROWED: [AtomicBool; MAXCPUS] = [const { AtomicBool::new(false) }; MAXCPUS];

/// The trap frame of the running thread's user context: the one `handle_el0_sync` or the
/// return to user mode (`do_ast`) saved at the top of its kernel stack, which `pcb_tf` names
/// from then on (`cpu_fork` points a new thread's at the frame it built in the same place).
///
/// The handler that got it and the code its calls out reach through `pcb_tf` (the signal
/// frame's `sendsig` and `sigreturn`, `child_return`, `PROC_PC`/`PROC_STACK`, `bsd`'s
/// `setregs`) all touch it, so no reference to it is kept: each access borrows it for one
/// closure ([`Self::with`]) that calls nothing which reaches it again, and a second borrow on
/// the CPU while one is in progress is `panic(9)` rather than a second `&mut`.
#[derive(Clone, Copy)]
pub(crate) struct UserFrame(NonNull<Trapframe>);

impl UserFrame {
    /// The frame an EL0 entry of `exception.S` passes in `x0`; `panic(9)` for null.
    fn from_entry(frame: *mut Trapframe) -> Self {
        match NonNull::new(frame) {
            Some(f) => Self(f),
            None => hal_panic(format_args!("trap: no frame")),
        }
    }

    /// `p->p_addr->u_pcb.pcb_tf` of the running thread: `None` before the first thread or
    /// for a thread whose pcb names no frame.
    pub(crate) fn current() -> Option<Self> {
        with_curpcb(|pcb| pcb.and_then(|pcb| NonNull::new(pcb.pcb_tf.get()))).map(Self)
    }

    /// `p->p_addr->u_pcb.pcb_tf = frame`: this frame is the running thread's user context
    /// from here on.
    fn make_current(self) {
        with_curpcb(|pcb| {
            if let Some(pcb) = pcb {
                pcb.pcb_tf.set(self.0.as_ptr());
            }
        });
    }

    /// `f` on the frame, borrowed for the call alone; `panic(9)` when this CPU borrows it
    /// already (a closure that reached it again).
    pub(crate) fn with<R>(self, f: impl FnOnce(&mut Trapframe) -> R) -> R {
        // The CPU's own flag, read and written by this CPU alone (the closure neither sleeps
        // nor switches, and an interrupt taken meanwhile runs on this CPU, which sees its own
        // stores in order): relaxed. The two compiler fences keep the closure's accesses to the
        // frame between the swap and the store: relaxed orders the flag for the hardware, not
        // the compiler's placement of plain accesses around it, and an interrupt on this CPU
        // must never see the flag clear while the frame is borrowed. They emit no instruction.
        let borrowed = &USER_FRAME_BORROWED[cpu_number()];
        if borrowed.swap(true, Ordering::Relaxed) {
            hal_panic(format_args!("trap: the user frame is borrowed twice"));
        }
        compiler_fence(Ordering::Acquire);
        // SAFETY: valid: the pointer is the frame `exception.S` saved for this EL0 entry (`x0`
        // of `handle_el0_sync`, `handle_el0_irq`, `handle_el0_fiq`, `handle_el0_error` or
        // `do_ast`: a whole, aligned `struct trapframe` it wrote at the top of the running
        // thread's kernel stack, alive until the handler returns and the assembly restores
        // from it), or the running thread's `pcb_tf`, which only ever names that same place:
        // `make_current` stores an EL0 entry's frame (an entry from user mode starts on an
        // empty kernel stack, so its frame is always at the top), `cpu_fork`
        // (`hal::thread::thread_setup`) the frame it built at the top of the child's u-area,
        // `bsd`'s `cpu_startup` `proc0`'s static one; the u-area is alive while its thread runs
        // (`hal::thread` E2, E4) and the kernel stack below never grows into the frame.
        // Unique: no reference to that storage is kept anywhere else: every EL0 entry (the
        // synchronous one, the IRQ and FIQ ones, the error and the AST) holds it as a
        // `UserFrame` and every access in `hal` goes through this function; the kernel-mode
        // handlers' `&mut Trapframe` are their own frames, below it on the stack; `bsd`'s
        // `setregs` writes through the raw pointer between its own statements, outside any
        // borrow (it runs inside a system call's dispatch, which `svc_handler` makes outside
        // its borrows); the assembly reads it only after the handler returned. The frame
        // belongs to the thread running on this CPU, which no other CPU reads through
        // `pcb_tf` (`current` names this CPU's thread only), and the flag above refused a
        // second borrow on this CPU. The reference ends with `f`.
        let r = f(unsafe { &mut *self.0.as_ptr() });
        compiler_fence(Ordering::Release);
        borrowed.store(false, Ordering::Relaxed);
        r
    }
}

/// `PROC_PC(p)` of the running thread (`hal::trap::proc_pc`): its user frame's `tf_elr`, 0
/// without one.
pub(crate) fn proc_pc() -> usize {
    UserFrame::current().map_or(0, |f| f.with(|tf| tf.tf_elr as usize))
}

/// `PROC_STACK(p)` of the running thread (`hal::trap::proc_stack`): its user frame's
/// `tf_sp`, 0 without one.
pub(crate) fn proc_stack() -> usize {
    UserFrame::current().map_or(0, |f| f.with(|tf| tf.tf_sp as usize))
}

/// `PROC_PC(p)` and `PROC_STACK(p)` of the running thread in one borrow
/// (`hal::trap::proc_regs`, the system call prologue's), `(0, 0)` without a frame.
pub(crate) fn proc_regs() -> (usize, usize) {
    UserFrame::current().map_or((0, 0), |f| {
        f.with(|tf| (tf.tf_elr as usize, tf.tf_sp as usize))
    })
}

/// `curcpu()->ci_curpcb->pcb_onfault`: the running thread's fault recovery address, 0 for
/// none (or before the first thread).
#[inline]
fn pcb_onfault() -> usize {
    with_curpcb(|pcb| pcb.map_or(0, |pcb| pcb.pcb_onfault.get()))
}

/// `is_unpriv_ldst`: whether the instruction at `elr` (a kernel address) is an unprivileged
/// load or store (`ldtr`/`sttr` family), the only way the kernel may touch user addresses.
fn is_unpriv_ldst(elr: u64) -> bool {
    if (elr >> 63) == 1 {
        let insn_at = ptr::with_exposed_provenance::<u32>(elr as usize);
        // SAFETY: `elr` is the address of the instruction this CPU was executing at EL1 when
        // the abort was taken (the frame's `tf_elr`, an instruction abort's own address being
        // a user one, which the top-bit test above excludes): the kernel fetched it from there
        // an instant ago, so it lies in mapped kernel text (the image or a loaded module,
        // neither ever unmapped), which is readable at EL1 and never written; an instruction
        // address is 4-byte aligned. The read is volatile: the word is not a Rust object.
        let insn = unsafe { ptr::read_volatile(insn_at) };
        return (insn & 0x3f20_0c00) == 0x3800_0800;
    }

    false
}

/// `fault`: `panicstr` claimed for this CPU's `ci_panicbuf`, the fatal fault's message
/// formatted into it and printed (`hal::trap::CpuHooks::fault`, the CPU code's); the `panic()`
/// that follows is then the second one on this CPU.
fn fault(args: fmt::Arguments<'_>) {
    crate::trap::fault(args);
}

/// `udata_abort`: a data or instruction abort from EL0: the branch predictor flushed for a
/// kernel address, an alignment fault signalled, then the fault served by the kernel on the
/// process's map (`pmap_fault_fixup`, `uvm_fault`, the `MAP_STACK` check of `sp`, the
/// frame's stack pointer, first); a fault it cannot serve is a signal.
fn udata_abort(esr: u64, far: u64, exe: bool, sp: usize) {
    let access_type = accesstype(esr, exe);
    let h = handler("udata_abort");

    let va = trunc_page(far as usize);
    if va >= VM_MAXUSER_ADDRESS {
        crate::trap::flush_bp();
    }

    if esr & ISS_DATA_DFSC_MASK == ISS_DATA_DFSC_ALIGN {
        h.trapsignal(SIGBUS, esr, BUS_ADRALN, far as usize);
        return;
    }

    let error = match h.page_fault(FaultSource::User { sp }, va, access_type) {
        Ok(()) => return,
        Err(e) => e,
    };

    let (sig, code) = if error == Errno::ENOMEM {
        (SIGKILL, 0)
    } else if error == Errno::EIO {
        (SIGBUS, BUS_OBJERR)
    } else if error == Errno::EACCES {
        (SIGSEGV, SEGV_ACCERR)
    } else {
        (SIGSEGV, SEGV_MAPERR)
    };
    h.trapsignal(sig, esr, code, far as usize);
}

/// `kdata_abort`: a data or instruction abort taken at EL1.
fn kdata_abort(frame: &mut Trapframe, esr: u64, far: u64, exe: bool) {
    let access_type = accesstype(esr, exe);
    let va = trunc_page(far as usize);

    // The top bit tells us which range to use
    let source = if (far >> 63) == 1 {
        FaultSource::Kernel
    } else if is_unpriv_ldst(frame.tf_elr as u64) {
        // Only allow user-space access using unprivileged load/store instructions.
        FaultSource::KernelUser
    } else if pcb_onfault() != 0 {
        FaultSource::Kernel
    } else if crate::trap::have_debugger() {
        fault(format_args!(
            "attempt to {} user address 0x{:x} from EL1",
            if exe { "execute" } else { "access" },
            far
        ));
        let _ = crate::trap::debugger_ktrap(esr_elx_exception(esr) as i32, frame);
        FaultSource::Kernel
    } else {
        hal_panic(format_args!(
            "attempt to {} user address 0x{:x} from EL1",
            if exe { "execute" } else { "access" },
            far
        ));
    };

    // Handle referenced/modified emulation, then uvm_fault (and uvm_grow on a process's
    // map): the kernel's, on the map `source` names. Before the handler is registered no
    // fault is served.
    let error = match crate::trap::handler() {
        Some(h) => h.page_fault(source, va, access_type),
        None => Err(Errno::EFAULT),
    };

    if error.is_err() {
        let onfault = pcb_onfault();
        if curcpu().idepth() == 0 && onfault != 0 {
            frame.tf_elr = onfault as Register;
            return;
        }
        hal_panic(format_args!(
            "uvm_fault failed: {:x} esr {:x} far {:x}",
            frame.tf_elr, esr, far
        ));
    }
}

/// `emulate_msr`: a read of an ID register from EL0 answered with the values userland may
/// see (`cpu_identify_cleanup`'s): `true` when the access was emulated and the instruction
/// skipped.
fn emulate_msr(frame: &mut Trapframe, esr: u64) -> bool {
    let field = |mask, shift| iss_msr_field(esr, mask, shift);
    let rt = field(ISS_MSR_RT_MASK, ISS_MSR_RT_SHIFT) as usize;
    let id = |a: &AtomicU64| a.load(Ordering::Relaxed);

    // Only emulate reads.
    if esr & ISS_MSR_DIR == 0 {
        return false;
    }

    // Only emulate non-debug System register access.
    if field(ISS_MSR_OP0_MASK, ISS_MSR_OP0_SHIFT) != 3
        || field(ISS_MSR_OP1_MASK, ISS_MSR_OP1_SHIFT) != 0
        || field(ISS_MSR_CRN_MASK, ISS_MSR_CRN_SHIFT) != 0
    {
        return false;
    }

    let op2 = field(ISS_MSR_OP2_MASK, ISS_MSR_OP2_SHIFT);
    #[allow(
        clippy::match_same_arms,
        reason = "the C keeps the cases apart: one switch case per register"
    )]
    let val = match (field(ISS_MSR_CRM_MASK, ISS_MSR_CRM_SHIFT), op2) {
        // MIDR_EL1
        (0, 0) => midr(),
        // MPIDR_EL1: don't reveal the topology to userland. But return a valid value; Bit
        // 31 is RES1.
        (0, 5) => 0x8000_0000,
        // REVIDR_EL1
        (0, 6) => 0,
        // ID_AA64PFR0_EL1
        (4, 0) => id(&CPU_ID_AA64PFR0),
        // ID_AA64PFR1_EL1
        (4, 1) => id(&CPU_ID_AA64PFR1),
        // ID_AA64PFR2_EL1, ID_AA64ZFR0_EL1, ID_AA64SMFR0_EL1
        (4, 2 | 4 | 5) => 0,
        // ID_AA64ISAR0_EL1
        (6, 0) => id(&CPU_ID_AA64ISAR0),
        // ID_AA64ISAR1_EL1
        (6, 1) => id(&CPU_ID_AA64ISAR1),
        // ID_AA64ISAR2_EL2
        (6, 2) => id(&CPU_ID_AA64ISAR2),
        // ID_AA64MMFR0_EL1 .. ID_AA64MMFR4_EL1
        (7, 0..=4) => 0,
        _ => return false,
    };

    if rt < 30 {
        frame.tf_x[rt] = val as Register;
    } else if rt == 30 {
        frame.tf_lr = val as Register;
    }
    frame.tf_elr += 4;

    true
}

/// `READ_SPECIALREG(midr_el1)`.
fn midr() -> u64 {
    let v: u64;
    // SAFETY: `mrs` of `MIDR_EL1`, a read-only identification register: no memory, no
    // stack, no flags.
    unsafe {
        core::arch::asm!("mrs {}, midr_el1", out(reg) v, options(nomem, nostack, preserves_flags));
    };
    v
}

/// `do_el1h_sync`: the synchronous exception handler for EL1, called from
/// `handle_el1h_sync` (`exception.S`) with the saved registers (the module's frame premise).
pub(crate) extern "C" fn do_el1h_sync(frame: &mut Trapframe) {
    // Read the ESR and FAR registers to get the exception details
    let (esr, far) = esr_far();

    intr_enable();
    UVMEXP.traps.fetch_add(1, Ordering::Relaxed);

    let exception = esr_elx_exception(esr);
    let toast = match exception {
        EXCP_FP_SIMD | EXCP_TRAP_FP => {
            fault(format_args!("FP exception in kernel"));
            true
        }
        EXCP_BRANCH_TGT => {
            fault(format_args!("Branch target exception in kernel"));
            true
        }
        EXCP_FPAC => {
            fault(format_args!("Pointher authentication failure in kernel"));
            true
        }
        EXCP_INSN_ABORT => {
            kdata_abort(frame, esr, far, true);
            false
        }
        EXCP_DATA_ABORT => {
            kdata_abort(frame, esr, far, false);
            false
        }
        EXCP_BRK | EXCP_WATCHPT_EL1 | EXCP_SOFTSTP_EL1 => {
            if crate::trap::debugger_ktrap(exception as i32, frame).is_none() {
                hal_panic(format_args!("No debugger in kernel"));
            }
            // Step over permanent breakpoints.
            if exception == EXCP_BRK && (esr & ISS_BRK_COMMENT_MASK) == 0xf000 {
                frame.tf_elr += INSN_SIZE as Register;
            }
            false
        }
        _ => {
            fault(format_args!("Unknown kernel exception 0x{exception:02x}"));
            true
        }
    };
    if toast {
        // we_re_toast:
        crate::prf::printf(format_args!(
            "esr 0x{:08x} far 0x{:016x} elr 0x{:016x}",
            esr, far, frame.tf_elr
        ));
        let _ = crate::trap::debugger_ktrap(exception as i32, frame);
        hal_panic(format_args!(
            "esr 0x{:08x} far 0x{:016x} elr 0x{:016x}",
            esr, far, frame.tf_elr
        ));
    }
}

/// `serror`: reports a system error interrupt taken at `elr` (the frame's) and calls the
/// CPU's handler, if any.
fn serror(elr: Register) {
    let (esr, far) = esr_far();

    crate::prf::printf(format_args!("SError: {elr:x} esr {esr:x} far {far:0x}\n"));

    crate::trap::serror();
}

/// `do_el1h_error`: an SError taken at EL1, called from `handle_el1h_error` (`exception.S`).
pub(crate) extern "C" fn do_el1h_error(frame: &mut Trapframe) {
    serror(frame.tf_elr);
    hal_panic(format_args!("do_el1h_error"));
}

/// `do_el0_sync`: the synchronous exception handler for EL0, called from `handle_el0_sync`
/// (`exception.S`) with the saved registers, the running thread's [`UserFrame`] (the
/// module's frame premise).
pub(crate) extern "C" fn do_el0_sync(frame: *mut Trapframe) {
    let frame = UserFrame::from_entry(frame);
    if !have_curproc() {
        hal_panic(format_args!("do_el0_sync: no curproc"));
    }
    let h = handler("do_el0_sync");

    let (esr, far) = esr_far();
    let exception = esr_elx_exception(esr);

    intr_enable();
    UVMEXP.traps.fetch_add(1, Ordering::Relaxed);

    frame.make_current();
    h.refreshcreds();

    // The frame's `elr` and `sp`, read in the arms that use them (one borrow each, none on
    // the system call's and the FP traps' paths).
    let elr = || frame.with(|tf| tf.tf_elr as usize);
    let sp = || frame.with(|tf| tf.tf_sp as usize);

    #[allow(
        clippy::match_same_arms,
        reason = "the C keeps the cases apart: one switch case per exception class"
    )]
    match exception {
        EXCP_UNKNOWN => {
            crate::trap::flush_bp();
            h.trapsignal(SIGILL, esr, ILL_ILLOPC, elr());
        }
        EXCP_SVE => fpu::sve_load_current(),
        EXCP_FP_SIMD | EXCP_TRAP_FP => fpu::load_current(),
        EXCP_BRANCH_TGT => {
            crate::trap::flush_bp();
            h.trapsignal(SIGILL, esr, ILL_BTCFI, elr());
        }
        EXCP_MSR if frame.with(|tf| emulate_msr(tf, esr)) => {}
        EXCP_MSR | EXCP_FPAC => {
            crate::trap::flush_bp();
            h.trapsignal(SIGILL, esr, ILL_ILLOPC, elr());
        }
        EXCP_SVC => super::syscall::svc_handler(frame),
        EXCP_INSN_ABORT_L => udata_abort(esr, far, true, sp()),
        EXCP_PC_ALIGN => {
            crate::trap::flush_bp();
            h.trapsignal(SIGBUS, esr, BUS_ADRALN, elr());
        }
        EXCP_SP_ALIGN => {
            crate::trap::flush_bp();
            h.trapsignal(SIGBUS, esr, BUS_ADRALN, sp());
        }
        EXCP_DATA_ABORT_L => udata_abort(esr, far, false, sp()),
        EXCP_BRK => h.trapsignal(SIGTRAP, esr, TRAP_BRKPT, elr()),
        EXCP_SOFTSTP_EL0 => h.trapsignal(SIGTRAP, esr, TRAP_TRACE, elr()),
        _ => {
            // panic("Unknown userland exception %x esr_el1 %lx", exception, esr);
            // USERLAND MUST NOT PANIC MACHINE
            // only here to debug !?!?
            crate::prf::printf(format_args!("exception {exception:x} esr_el1 {esr:x}\n"));
            frame.with(|tf| dumpregs(tf));
            crate::trap::flush_bp();
            crate::klock::kernel_lock();
            h.sigexit(SIGILL);
            // KERNEL_UNLOCK(): sigexit does not return.
        }
    }

    h.userret();
}

/// `do_el0_error`: an SError taken at EL0, called from `handle_el0_error` (`exception.S`).
pub(crate) extern "C" fn do_el0_error(frame: *mut Trapframe) {
    let elr = UserFrame::from_entry(frame).with(|tf| tf.tf_elr);
    serror(elr);
    hal_panic(format_args!("do_el0_error"));
}

/// `dumpregs`: prints a trap frame.
pub fn dumpregs(frame: &Trapframe) {
    for i in (0..30).step_by(2) {
        crate::prf::printf(format_args!(
            "x{:02}: 0x{:016x} 0x{:016x}\n",
            i,
            frame.tf_x[i],
            frame.tf_x[i + 1]
        ));
    }
    crate::prf::printf(format_args!("sp: 0x{:016x}\n", frame.tf_sp));
    crate::prf::printf(format_args!("lr: 0x{:016x}\n", frame.tf_lr));
    crate::prf::printf(format_args!("pc: 0x{:016x}\n", frame.tf_elr));
    crate::prf::printf(format_args!("spsr: 0x{:016x}\n", frame.tf_spsr));
}

/// `ast(tf)`: the asynchronous system trap on the way back to user mode (`do_ast` in
/// `exception.S`), with the frame of the thread returning there, its [`UserFrame`].
pub(crate) extern "C" fn ast(tf: *mut Trapframe) {
    let tf = UserFrame::from_entry(tf);
    if !have_curproc() {
        hal_panic(format_args!("ast: no curproc"));
    }
    let h = handler("ast");

    tf.make_current();

    h.refreshcreds();
    UVMEXP.softs.fetch_add(1, Ordering::Relaxed);
    h.ast(curcpu().want_resched() != 0);
    h.userret();
}

/// `arm_cpu_irq`: `ci_idepth` around the controller's dispatch (`arm_irq_dispatch`).
fn irq(frame: &mut Trapframe) {
    let ci = curcpu();

    UVMEXP.intrs.fetch_add(1, Ordering::Relaxed);
    ci.idepth_add(1);
    crate::intr::dispatch_irq(frame);
    ci.idepth_add(-1);
}

/// `arm_cpu_fiq`: as [`irq`] (`arm_fiq_dispatch`).
fn fiq(frame: &mut Trapframe) {
    let ci = curcpu();

    UVMEXP.intrs.fetch_add(1, Ordering::Relaxed);
    ci.idepth_add(1);
    crate::intr::dispatch_fiq(frame);
    ci.idepth_add(-1);
}

/// `arm_cpu_irq` from `handle_el1h_irq` (`exception.S`): an interrupt taken in kernel mode,
/// on its own frame.
pub(crate) extern "C" fn arm_cpu_irq(frame: &mut Trapframe) {
    irq(frame);
}

/// `arm_cpu_fiq` from `handle_el1h_fiq`.
pub(crate) extern "C" fn arm_cpu_fiq(frame: &mut Trapframe) {
    fiq(frame);
}

/// `arm_cpu_irq` from `handle_el0_irq`: an interrupt taken in user mode, whose frame is the
/// running thread's [`UserFrame`]; the dispatch runs inside its borrow (the frame premise).
pub(crate) extern "C" fn arm_cpu_irq_el0(frame: *mut Trapframe) {
    UserFrame::from_entry(frame).with(irq);
}

/// `arm_cpu_fiq` from `handle_el0_fiq`, as [`arm_cpu_irq_el0`].
pub(crate) extern "C" fn arm_cpu_fiq_el0(frame: *mut Trapframe) {
    UserFrame::from_entry(frame).with(fiq);
}
/* </CODE> */
