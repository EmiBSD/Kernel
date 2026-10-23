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
//! frame it just saved on this CPU's kernel stack: a valid, aligned `struct trapframe` that
//! nothing else references until the handler returns and the assembly restores from it. So
//! the handlers take it as `&mut Trapframe`.
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

use core::fmt;
use core::ptr;
use core::sync::atomic::{AtomicU64, Ordering};

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
use crate::cpu::curcpu;
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
        )
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

/// `p->p_addr->u_pcb.pcb_tf = frame`: the running thread's trap frame, for `sendsig`,
/// `sys_sigreturn`, `ptrace` and the `PROC_PC`/`PROC_STACK` readers.
#[inline]
fn set_pcb_tf(frame: &mut Trapframe) {
    let tf = ptr::from_mut(frame);
    with_curpcb(|pcb| {
        if let Some(pcb) = pcb {
            pcb.pcb_tf.set(tf);
        }
    });
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
/// process's map (`pmap_fault_fixup`, `uvm_fault`, the `MAP_STACK` check first); a fault it
/// cannot serve is a signal.
fn udata_abort(esr: u64, far: u64, exe: bool) {
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

    let error = match h.page_fault(FaultSource::User, va, access_type) {
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
        core::arch::asm!("mrs {}, midr_el1", out(reg) v, options(nomem, nostack, preserves_flags))
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

/// `serror`: reports a system error interrupt and calls the CPU's handler, if any.
fn serror(frame: &Trapframe) {
    let (esr, far) = esr_far();

    crate::prf::printf(format_args!(
        "SError: {:x} esr {:x} far {:0x}\n",
        frame.tf_elr, esr, far
    ));

    crate::trap::serror();
}

/// `do_el1h_error`: an SError taken at EL1, called from `handle_el1h_error` (`exception.S`).
pub(crate) extern "C" fn do_el1h_error(frame: &mut Trapframe) {
    serror(frame);
    hal_panic(format_args!("do_el1h_error"));
}

/// `do_el0_sync`: the synchronous exception handler for EL0, called from `handle_el0_sync`
/// (`exception.S`) with the saved registers (the module's frame premise).
pub(crate) extern "C" fn do_el0_sync(frame: &mut Trapframe) {
    if !have_curproc() {
        hal_panic(format_args!("do_el0_sync: no curproc"));
    }
    let h = handler("do_el0_sync");

    let (esr, far) = esr_far();
    let exception = esr_elx_exception(esr);

    intr_enable();
    UVMEXP.traps.fetch_add(1, Ordering::Relaxed);

    set_pcb_tf(frame);
    h.refreshcreds();

    let elr = frame.tf_elr as usize;

    match exception {
        EXCP_UNKNOWN => {
            crate::trap::flush_bp();
            h.trapsignal(SIGILL, esr, ILL_ILLOPC, elr);
        }
        EXCP_SVE => fpu::sve_load_current(),
        EXCP_FP_SIMD | EXCP_TRAP_FP => fpu::load_current(),
        EXCP_BRANCH_TGT => {
            crate::trap::flush_bp();
            h.trapsignal(SIGILL, esr, ILL_BTCFI, elr);
        }
        EXCP_MSR if emulate_msr(frame, esr) => {}
        EXCP_MSR | EXCP_FPAC => {
            crate::trap::flush_bp();
            h.trapsignal(SIGILL, esr, ILL_ILLOPC, elr);
        }
        EXCP_SVC => super::syscall::svc_handler(frame),
        EXCP_INSN_ABORT_L => udata_abort(esr, far, true),
        EXCP_PC_ALIGN => {
            crate::trap::flush_bp();
            h.trapsignal(SIGBUS, esr, BUS_ADRALN, elr);
        }
        EXCP_SP_ALIGN => {
            crate::trap::flush_bp();
            h.trapsignal(SIGBUS, esr, BUS_ADRALN, frame.tf_sp as usize);
        }
        EXCP_DATA_ABORT_L => udata_abort(esr, far, false),
        EXCP_BRK => h.trapsignal(SIGTRAP, esr, TRAP_BRKPT, elr),
        EXCP_SOFTSTP_EL0 => h.trapsignal(SIGTRAP, esr, TRAP_TRACE, elr),
        _ => {
            // panic("Unknown userland exception %x esr_el1 %lx", exception, esr);
            // USERLAND MUST NOT PANIC MACHINE
            // only here to debug !?!?
            crate::prf::printf(format_args!("exception {exception:x} esr_el1 {esr:x}\n"));
            dumpregs(frame);
            crate::trap::flush_bp();
            crate::klock::kernel_lock();
            h.sigexit(SIGILL);
            // KERNEL_UNLOCK(): sigexit does not return.
        }
    }

    h.userret();
}

/// `do_el0_error`: an SError taken at EL0, called from `handle_el0_error` (`exception.S`).
pub(crate) extern "C" fn do_el0_error(frame: &mut Trapframe) {
    serror(frame);
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
/// `exception.S`), with the frame of the thread returning there.
pub(crate) extern "C" fn ast(tf: &mut Trapframe) {
    if !have_curproc() {
        hal_panic(format_args!("ast: no curproc"));
    }
    let h = handler("ast");

    set_pcb_tf(tf);

    h.refreshcreds();
    UVMEXP.softs.fetch_add(1, Ordering::Relaxed);
    h.ast(curcpu().want_resched() != 0);
    h.userret();
}

/// `arm_cpu_irq`: the IRQ entry, called from `handle_el1h_irq` and `handle_el0_irq`
/// (`exception.S`): `ci_idepth` around the controller's dispatch (`arm_irq_dispatch`).
pub(crate) extern "C" fn arm_cpu_irq(frame: &mut Trapframe) {
    let ci = curcpu();

    UVMEXP.intrs.fetch_add(1, Ordering::Relaxed);
    ci.idepth_add(1);
    crate::intr::dispatch_irq(frame);
    ci.idepth_add(-1);
}

/// `arm_cpu_fiq`: the FIQ entry, as [`arm_cpu_irq`] (`arm_fiq_dispatch`).
pub(crate) extern "C" fn arm_cpu_fiq(frame: &mut Trapframe) {
    let ci = curcpu();

    UVMEXP.intrs.fetch_add(1, Ordering::Relaxed);
    ci.idepth_add(1);
    crate::intr::dispatch_fiq(frame);
    ci.idepth_add(-1);
}
/* </CODE> */
