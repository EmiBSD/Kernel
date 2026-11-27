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
//! arm64's exception handlers for kernel mode: `trap.c` (the synchronous exceptions and the
//! SErrors of EL1) and `intr.c`'s IRQ and FIQ entries, called by the assembly of
//! `exception.rs` with the trap frame it built; and what a trap from user mode (`entry`'s,
//! decision 56) shares with them: the user frame ([`UserFrame`], behind `usertrap.rs`'s
//! `Arrival`), the syndrome's read, the interrupt dispatch, the SError report, the
//! identification registers' emulation and the register dump.
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
//! registers, the frame's fix-ups. A kernel-mode fault is asked of the registered
//! `hal::cpu::trap::TrapHandler` (the kernel's); a trap from user mode is classified by
//! `entry`, which calls the kernel's `kernel::trap` itself, downward; the branch predictor
//! flush and the SError report of the CPU code still in `bsd` are `hal::cpu::trap::CpuHooks`;
//! the kernel debugger is `hal::cpu::trap::Debugger`; the interrupt controller's dispatch is
//! `hal::cpu::intr::IntrHardware`'s.
//!
//! **The frame premise.** Every `extern "C"` handler here is reached only from the assembly
//! of `exception.S` (`sym` operands, no symbol another object could call), with `x0` the trap
//! frame it just saved on this CPU's kernel stack: a valid, aligned `struct trapframe` the
//! assembly reads again only after the handler returns. The frame of an entry from user mode
//! is saved by the EL0 handlers of `exception.S` and passed, as an `Arrival` (`usertrap.rs`),
//! a token only that assembly makes, to the one function it calls (`entry`'s
//! `entry_user_trap`, decision 58):
//! it is the thread's user context, which `pcb_tf` names (an entry from user mode starts on
//! an empty kernel stack, so its frame is always at the same place) and which the code a
//! trap calls out to reads and writes (`sendsig`, `sigreturn`, `PROC_PC`, `exec`'s
//! `setregs`): `hal` holds it as a [`UserFrame`], borrowed one closure at a time, never
//! across such a call; an interrupt's dispatch runs inside its borrow, so a handler that
//! reached the frame again would be `panic(9)`, never a second `&mut`. What the proof takes
//! from `entry` is the function's signature, which `entry` pins to `usertrap::UserTrapFn`
//! (decision 58). The kernel-mode handlers take `&mut Trapframe`: their frame
//! lies below the user frame on the kernel stack, and nothing names it but the handler.
//!
//! ## Deviations
//! - The `we_re_toast` path prints the syndrome and enters the debugger as the `DDB` build
//!   does, then panics with the same message as the non-`DDB` build: ddb-lite has no command
//!   loop to stay in, and returning would re-execute the faulting instruction (LZ's).
//! - `kdata_abort`'s unprivileged access with no thread running uses the kernel map where the
//!   C dereferences `p` (the trap handler's choice, `hal::cpu::trap::FaultSource::KernelUser`).
//!
//! ## Redesign
//! - N4.8b: from `bsd` into `hal`. LZ called `uvm_fault`, `trapsignal`, `userret`, ... by
//!   name and read `ci_curproc`/`ci_curpcb` through raw pointers; here the machine-independent
//!   calls go through `hal::cpu::trap::TrapHandler` (one registration), the running thread's pcb
//!   through `hal::cpu::thread`'s `with_curpcb` (its premise E5), and the handlers are entered by
//!   `sym` (no `#[no_mangle]`). `fault()`'s `panicstr` claim and `ci_panicbuf` are the
//!   CPU hooks' (`hal::cpu::trap::CpuHooks::fault`), `db_ktrap` the `Debugger` registration (absent:
//!   the C without `DDB`, `panic("No debugger in kernel")` for a kernel breakpoint).
//! - `ast.c`'s `ast()` and `intr.c`'s `arm_cpu_irq`/`arm_cpu_fiq` join the trap: the IRQ
//!   entry dispatches through `hal::cpu::intr::IntrHardware::dispatch_irq` (the controller's
//!   `arm_irq_dispatch`, which `bsd`'s GIC drivers register, N7).
//! - `cpu_id_aa64pfr0`, `pfr1`, `isar0`, `isar1`, `isar2` (`cpu.c`'s userland view of the ID
//!   registers, `emulate_msr`'s answers) were this module's statics until decision 56's E2:
//!   the values a program is told are the kernel's (`kernel::trap`), and `emulate_msr` is
//!   split: the decode of the trapped `mrs` and the answer's write are `usertrap.rs`'s
//!   (`SyncTrap::id_read`, `id_answer`, and `midr`), the list of the reads answered and the
//!   values the kernel's (decision 56's U7, amended by the user's lock of 2026-11-27: the
//!   list is not the ABI's).
//! - Decision 56 (E1): `ast()`'s machine-independent work (`refreshcreds`, `uvmexp.softs`,
//!   `mi_ast` with `ci_want_resched`, `userret`) is the kernel's `kernel::trap::ast`, asked
//!   in one call (`TrapHandler::ast`) where it took three; the frame stays recorded here
//!   until the trap moves to `entry` (E2).
//! - Decision 56's U8 (E1): `exec`'s `setregs` wrote the running thread's frame through the
//!   raw `pcb_tf` (`bsd`'s arm64 `machdep.rs`, five writes outside any borrow, which the
//!   frame's uniqueness rested on as a fact of `bsd`'s call graph); its frame writes are
//!   [`set_exec_regs`], one borrow of the [`UserFrame`] (`hal::cpu::trap::set_exec_regs`), so
//!   every write of a user frame goes through [`UserFrame::with`].
//! - Decision 56 (E2): the EL0 entries' handlers take an `Arrival` (`usertrap.rs`), the
//!   token of the frame the assembly saved, in place of the raw pointer, and reach the
//!   frame's mechanics (the prologue, the FPU traps, `svc_handler`'s half, the dispatch of an
//!   interrupt, the SError) only through it. Then the crate's step: the classification
//!   (`do_el0_sync`'s `switch`, `udata_abort`'s decode, `do_el0_error` and the EL0 IRQ and FIQ
//!   entries) is `entry`'s (`sys/entry/arch/arm64/trap.rs`), the kernel's half of
//!   `udata_abort` and the signals `kernel::trap`'s, and `ast()` (its `pcb_tf` store dropped,
//!   P5: the frame is always at one place) the kernel's loop's; `hal` keeps the frame, the
//!   mechanics and the kernel-mode handlers. `do_el0_sync`'s `KERNEL_UNLOCK` after `sigexit`,
//!   a deviation here, went with it (`kernel::trap::kill` never returns either).
//! - N4.8d: `do_el0_sync` read the frame's `elr` or `sp` in the arm that used it, not once
//!   for every exception, and `udata_abort` handed `PROC_STACK(p)`, read from the frame it
//!   held, to the kernel's `MAP_STACK` check: a user fault borrows the frame once instead of
//!   twice, a system call not at all here (kept by `entry`'s `SyncTrap` calls).

use core::fmt;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, Ordering, compiler_fence};

use super::cpu::intr_enable;
use super::include::armreg::{
    EXCP_BRANCH_TGT, EXCP_BRK, EXCP_DATA_ABORT, EXCP_FP_SIMD, EXCP_FPAC, EXCP_INSN_ABORT,
    EXCP_SOFTSTP_EL1, EXCP_TRAP_FP, EXCP_WATCHPT_EL1, INSN_SIZE, ISS_BRK_COMMENT_MASK, PSR_DIT,
    PSR_M_EL0t, accesstype, esr_elx_exception,
};
use super::include::frame::Trapframe;
use super::switch::with_curpcb;
use crate::assert::hal_panic;
use crate::cpu::trap::FaultSource;
use crate::cpu::{MAXCPUS, cpu_number, curcpu};
use crate::vm::param::trunc_page;
use crate::vm::uvmexp::UVMEXP;
use sysdefs::base::errno::Errno;
use sysdefs::base::types::Register;

/// `READ_SPECIALREG(esr_el1)` and `READ_SPECIALREG(far_el1)`: the syndrome and the fault
/// address of the exception this CPU is handling.
#[inline]
pub(super) fn esr_far() -> (u64, u64) {
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

/// Per CPU, whether a [`UserFrame`] is borrowed on it ([`UserFrame::with`]).
static USER_FRAME_BORROWED: [AtomicBool; MAXCPUS] = [const { AtomicBool::new(false) }; MAXCPUS];

/// The trap frame of the running thread's user context: the one `handle_el0_sync` or the
/// return to user mode (`do_ast`) saved at the top of its kernel stack, which `pcb_tf` names
/// from then on (`cpu_fork` points a new thread's at the frame it built in the same place).
///
/// The handler that got it and the code its calls out reach through `pcb_tf` (the signal
/// frame's `sendsig` and `sigreturn`, `PROC_PC`/`PROC_STACK`, `bsd`'s `setregs`) all touch
/// it, so no reference to it is kept: each access borrows it for one
/// closure ([`Self::with`]) that calls nothing which reaches it again, and a second borrow on
/// the CPU while one is in progress is `panic(9)` rather than a second `&mut`.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub(crate) struct UserFrame(NonNull<Trapframe>);

impl UserFrame {
    /// `p->p_addr->u_pcb.pcb_tf` of the running thread: `None` before the first thread or
    /// for a thread whose pcb names no frame.
    pub(crate) fn current() -> Option<Self> {
        with_curpcb(|pcb| pcb.and_then(|pcb| NonNull::new(pcb.pcb_tf.get()))).map(Self)
    }

    /// `p->p_addr->u_pcb.pcb_tf = frame`: this frame is the running thread's user context
    /// from here on.
    pub(super) fn make_current(self) {
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
        // of `handle_el0_sync`, `handle_el0_irq`, `handle_el0_fiq` and `handle_el0_error`, the
        // `Arrival` of the one call they make, `entry`'s `entry_user_trap` with the signature
        // `usertrap::UserTrapFn` its definer pins, a token only that assembly makes,
        // `usertrap.rs`: a whole, aligned `struct trapframe` it wrote at the top of the running
        // thread's kernel stack, alive until the handler returns and the assembly restores
        // from it), or the running thread's `pcb_tf`, which only ever names that same place:
        // `make_current` stores an EL0 entry's frame (an entry from user mode starts on an
        // empty kernel stack, so its frame is always at the top), `cpu_fork`
        // (`hal::cpu::thread::thread_setup`) the frame it built at the top of the child's u-area,
        // `bsd`'s `cpu_startup` `proc0`'s static one; the u-area is alive while its thread runs
        // (`hal::cpu::thread` E2, E4) and the kernel stack below never grows into the frame.
        // Unique: no reference to that storage is kept anywhere else: every EL0 entry (the
        // synchronous one, the IRQ and FIQ ones and the error) gives it to `hal` as an
        // `Arrival`, which `entry` cannot open, and every access in `hal` goes through this
        // function; the kernel's AST loop reaches it only through `hal`'s functions; the kernel-mode
        // handlers' `&mut Trapframe` are their own frames, below it on the stack; `exec`'s
        // `setregs` writes it through `set_exec_regs`, here (decision 56's U8: `bsd` wrote
        // through the raw pointer before); `bsd`'s arm64 `Machine::proc_pc` and `proc_stack`
        // still read another thread's `pcb_tf` raw (`sys/bsd/arch/arm64/mod.rs`), but only
        // amd64's files call them (N10), so on arm64 no such read happens; the ABI's handler (another crate's code) never gets this
        // function or a closure over the frame, only `syscall::UserRegs`, whose three
        // methods each borrow the frame here for a closure of `hal`'s own that indexes,
        // reads and writes the frame and calls nothing; the assembly reads it only after the
        // handler returned. The frame
        // belongs to the thread running on this CPU, which no other CPU reads through
        // `pcb_tf` (`current` names this CPU's thread only), and the flag above refused a
        // second borrow on this CPU. The reference ends with `f`.
        let r = f(unsafe { &mut *self.0.as_ptr() });
        compiler_fence(Ordering::Release);
        borrowed.store(false, Ordering::Relaxed);
        r
    }
}

/// `PROC_PC(p)` of the running thread (`hal::cpu::trap::proc_pc`): its user frame's `tf_elr`, 0
/// without one.
pub(crate) fn proc_pc() -> usize {
    UserFrame::current().map_or(0, |f| f.with(|tf| tf.tf_elr as usize))
}

/// `PROC_STACK(p)` of the running thread (`hal::cpu::trap::proc_stack`): its user frame's
/// `tf_sp`, 0 without one.
pub(crate) fn proc_stack() -> usize {
    UserFrame::current().map_or(0, |f| f.with(|tf| tf.tf_sp as usize))
}

/// `setregs`' frame writes for the running thread, which is exec'ing (`hal::cpu::trap::
/// set_exec_regs`): its user frame cleared whole, the stack pointer `sp`, the link register
/// and the pc the entry point `entry`, EL0 with data independent timing; in one borrow.
/// `panic(9)` for a thread with no frame (no exec runs before the first thread).
pub(crate) fn set_exec_regs(sp: usize, entry: usize) {
    let Some(f) = UserFrame::current() else {
        hal_panic(format_args!("setregs: no trap frame"));
    };
    f.with(|tf| {
        *tf = Trapframe::default();
        tf.tf_sp = sp as Register;
        tf.tf_lr = entry as Register;
        // The pc a program starts at; the link register gets the same value, which no
        // program returns through (OpenBSD marks this choice as unsettled).
        tf.tf_elr = entry as Register;
        tf.tf_spsr = (PSR_M_EL0t | PSR_DIT) as Register;
    });
}

/// `PROC_PC(p)` and `PROC_STACK(p)` of the running thread in one borrow
/// (`hal::cpu::trap::proc_regs`, the system call prologue's), `(0, 0)` without a frame.
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
        // an instant ago, so it lies in mapped kernel text (the image's text: nothing else
        // in the upper half executes at EL1, and it is never unmapped), which is readable at
        // EL1 and never written; an instruction address is 4-byte aligned. The read is
        // volatile: the word is not a Rust object.
        let insn = unsafe { ptr::read_volatile(insn_at) };
        return (insn & 0x3f20_0c00) == 0x3800_0800;
    }

    false
}

/// `fault`: `panicstr` claimed for this CPU's `ci_panicbuf`, the fatal fault's message
/// formatted into it and printed (`hal::cpu::trap::CpuHooks::fault`, the CPU code's); the `panic()`
/// that follows is then the second one on this CPU.
fn fault(args: fmt::Arguments<'_>) {
    crate::cpu::trap::fault(args);
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
    } else if crate::cpu::trap::have_debugger() {
        fault(format_args!(
            "attempt to {} user address 0x{:x} from EL1",
            if exe { "execute" } else { "access" },
            far
        ));
        let _ = crate::cpu::trap::debugger_ktrap(esr_elx_exception(esr) as i32, frame);
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
    let error = match crate::cpu::trap::handler() {
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
            if crate::cpu::trap::debugger_ktrap(exception as i32, frame).is_none() {
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
        let _ = crate::cpu::trap::debugger_ktrap(exception as i32, frame);
        hal_panic(format_args!(
            "esr 0x{:08x} far 0x{:016x} elr 0x{:016x}",
            esr, far, frame.tf_elr
        ));
    }
}

/// `serror`: reports a system error interrupt taken at `elr` (the frame's) and calls the
/// CPU's handler, if any.
pub(super) fn serror(elr: Register) {
    let (esr, far) = esr_far();

    crate::prf::printf(format_args!("SError: {elr:x} esr {esr:x} far {far:0x}\n"));

    crate::cpu::trap::serror();
}

/// `do_el1h_error`: an SError taken at EL1, called from `handle_el1h_error` (`exception.S`).
pub(crate) extern "C" fn do_el1h_error(frame: &mut Trapframe) {
    serror(frame.tf_elr);
    hal_panic(format_args!("do_el1h_error"));
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

/// `arm_cpu_irq`: `ci_idepth` around the controller's dispatch (`arm_irq_dispatch`).
pub(super) fn irq(frame: &mut Trapframe) {
    let ci = curcpu();

    UVMEXP.intrs.fetch_add(1, Ordering::Relaxed);
    ci.idepth_add(1);
    crate::cpu::intr::dispatch_irq(frame);
    ci.idepth_add(-1);
}

/// `arm_cpu_fiq`: as [`irq`] (`arm_fiq_dispatch`).
pub(super) fn fiq(frame: &mut Trapframe) {
    let ci = curcpu();

    UVMEXP.intrs.fetch_add(1, Ordering::Relaxed);
    ci.idepth_add(1);
    crate::cpu::intr::dispatch_fiq(frame);
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

/* </CODE> */
