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
//! The kernel's door for a trap from user mode (decision 56), as `kernel::api` is the ABI's:
//! what the code that classifies the trap (`entry`, which no crate names) asks the kernel
//! (the credentials refreshed, a fault from user mode served or signalled, a signal sent,
//! the process killed, the work before the return), and the kernel's side of the way back
//! to user mode: `ast.c`'s `ast()` and the loop that runs it until `hal`'s exit gate finds
//! nothing pending.
//!
//! Upstream: sys/arch/arm64/arm64/trap.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/ast.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/trap.rs@e8249449d905
//! LZ: sys/arch/arm64/arm64/ast.rs@3c62ede68802
//!
//! The door is plain functions ([`enter`], [`fault`], [`signal`], [`kill`], [`leave`],
//! [`exit`], [`syscall_return`]): the code that calls them cannot name the glue the kernel's
//! process code is generic over (`bsd`'s, until N9), so they reach it through one table the
//! image's boot glue installs once, [`UserTraps`], whose one implementation is the kernel's
//! own [`KernelTrap`] over that glue (docs/SCAFFOLDING.md: the `dyn` goes when the glue's
//! generic does, N9). [`exit`], an interrupt's way back, calls through it only for a pending
//! AST.
//!
//! The C's `do_ast` (`exception.S`) is the loop, in assembly, around `ast()`: the interrupts
//! masked, `md_astpending` tested and cleared, the interrupts restored and `ast()` called
//! while it was set, the return to user mode taken only once it was clear and with the
//! interrupts still masked. Here `hal` keeps the test with the interrupts masked
//! (`hal::cpu::usertrap::ExitGate`) and the kernel runs the loop ([`user_exit`], [`exit`]):
//! `hal` calls nobody on the way out, and a return to user mode needs the gate's token
//! (`hal::cpu::usertrap::UserExit`), which only a pass that found nothing pending hands out.
//!
//! ## Redesign
//! - E1 (decision 56, the way out): a new thread's first return to user mode runs [`user_exit`]
//!   from its start (`kernel::proc::fork`'s `thread_start`), generic over the glue: no
//!   indirect call, where `hal`'s `syscall_return` ran `do_ast` and `hal`'s `ast()` called the
//!   kernel three times through `hal::cpu::trap::TrapHandler` (`refreshcreds`, `mi_ast`,
//!   `userret`).
//! - `ast()`'s `p->p_addr->u_pcb.pcb_tf = tf` is not here: the frame of a thread's user
//!   context is always at the top of its kernel stack, where `cpu_fork` built it and every
//!   entry from user mode saves it, and `pcb_tf` names that place already (decision 56's P5).
//! - E2 (decision 56, the door): `trap.c`'s machine-independent calls (`refreshcreds`,
//!   `udata_abort`'s fault on the process's map and its choice of signal, `trapsignal`,
//!   `KERNEL_LOCK(); sigexit()`, `userret`), `ast.c`'s `ast()` and `syscall.c`'s
//!   `mi_syscall_return` were asked by `hal`'s trap code through the `TrapHandler` `bsd`
//!   registered in `hal`; they are the kernel's door, called downward by `entry`, and the
//!   table behind it is the kernel's own ([`install`]). `udata_abort` is split: the decode
//!   (the alignment fault, the branch predictor flush for a kernel address) is the trap's,
//!   the fault and the errno's signal ([`udata_abort_signal`]) are here. The system call's
//!   return (`kernel::api::syscall_return`) reaches the kernel through this table, where it
//!   went through `hal`'s registry and back.

use core::pin::Pin;
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use hal::arch::current::include::armreg::{ISS_DATA_DFSC_ALIGN, ISS_DATA_DFSC_MASK, accesstype};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use hal::arch::current::usertrap::{Arrival, SyncTrap, Syndrome, midr};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use hal::arch::current::vmparam::VM_MAXUSER_ADDRESS;
use hal::bridges::AlreadyRegistered;
use hal::cpu::usertrap::{AstPending, ExitGate, UserExit};
use hal::sync::once::OnceStatic;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use hal::syscall::UserRegs;
use hal::syscall::{Register, Rval, SysResult};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use hal::vm::param::trunc_page;
use hal::vm::pmap::VmProt;
use hal::vm::uvmexp::UVMEXP;
use sysdefs::base::errno::Errno;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use sysdefs::sig::siginfo::{BUS_ADRALN, ILL_BTCFI, ILL_ILLOPC, TRAP_BRKPT, TRAP_TRACE};
use sysdefs::sig::siginfo::{BUS_OBJERR, SEGV_ACCERR, SEGV_MAPERR, Sigval};
use sysdefs::sig::signal::{SIGBUS, SIGKILL, SIGSEGV};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use sysdefs::sig::signal::{SIGILL, SIGTRAP};

use crate::assert::kpanic;
use crate::proc::{ProcGlue, Thread};

/// A fault from user mode as the trap decoded it (`udata_abort`'s arguments): the faulting
/// page, the access, and what the signal of a fault the kernel cannot serve carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserFault {
    /// The user stack pointer at the fault (`PROC_STACK(p)`), for the `MAP_STACK` check.
    pub sp: usize,
    /// The faulting page (`trunc_page(far)`).
    pub va: usize,
    /// The access (`accesstype(esr, exe)`).
    pub access: VmProt,
    /// The trap number a signal carries (`esr`).
    pub trapno: u64,
    /// The faulting address a signal carries (`far`, its `sival_ptr`).
    pub addr: usize,
}

/// The identification registers a user program may read with `mrs` (arm64's `emulate_msr`
/// cases, `trap.c`): each is answered with the value [`id_value`] gives, never the CPU's own
/// but for `MIDR_EL1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdReg {
    /// `MIDR_EL1`: the CPU's model, read live.
    Midr,
    /// `MPIDR_EL1`: a fixed value, the topology not revealed.
    Mpidr,
    /// `REVIDR_EL1`.
    Revidr,
    /// `ID_AA64PFR0_EL1`.
    Pfr0,
    /// `ID_AA64PFR1_EL1`.
    Pfr1,
    /// `ID_AA64PFR2_EL1`.
    Pfr2,
    /// `ID_AA64ZFR0_EL1`.
    Zfr0,
    /// `ID_AA64SMFR0_EL1`.
    Smfr0,
    /// `ID_AA64ISAR0_EL1`.
    Isar0,
    /// `ID_AA64ISAR1_EL1`.
    Isar1,
    /// `ID_AA64ISAR2_EL1`.
    Isar2,
    /// `ID_AA64MMFR0_EL1`.
    Mmfr0,
    /// `ID_AA64MMFR1_EL1`.
    Mmfr1,
    /// `ID_AA64MMFR2_EL1`.
    Mmfr2,
    /// `ID_AA64MMFR3_EL1`.
    Mmfr3,
    /// `ID_AA64MMFR4_EL1`.
    Mmfr4,
}

/// The reads answered, by CRm and op2 of `S3_0_C0_<CRm>_<op2>` (op0 3, op1 0, CRn 0, which
/// the trap checked): `emulate_msr`'s `switch`, as data. Any other is `SIGILL`.
pub const ID_READS: [[Option<IdReg>; 8]; 16] = {
    let mut t = [[None; 8]; 16];
    t[0][0] = Some(IdReg::Midr);
    t[0][5] = Some(IdReg::Mpidr);
    t[0][6] = Some(IdReg::Revidr);
    t[4][0] = Some(IdReg::Pfr0);
    t[4][1] = Some(IdReg::Pfr1);
    t[4][2] = Some(IdReg::Pfr2);
    t[4][4] = Some(IdReg::Zfr0);
    t[4][5] = Some(IdReg::Smfr0);
    t[6][0] = Some(IdReg::Isar0);
    t[6][1] = Some(IdReg::Isar1);
    t[6][2] = Some(IdReg::Isar2);
    t[7][0] = Some(IdReg::Mmfr0);
    t[7][1] = Some(IdReg::Mmfr1);
    t[7][2] = Some(IdReg::Mmfr2);
    t[7][3] = Some(IdReg::Mmfr3);
    t[7][4] = Some(IdReg::Mmfr4);
    t
};

/// What the kernel answers a trap from user mode with, on the running thread, for the glue its
/// process code is generic over. The kernel's [`KernelTrap`] is the one implementation; the
/// image's boot glue installs it ([`install`]) before the first thread runs in user mode.
pub trait UserTraps: Sync {
    /// `refreshcreds(p)`, the first thing a synchronous trap does once the frame is recorded.
    fn enter(&self);
    /// `udata_abort`'s kernel half: the `MAP_STACK` check of `f.sp`, then
    /// `pmap_fault_fixup`, `uvm_fault` and `uvm_grow` on the process's map; a fault it cannot
    /// serve is the signal its errno names ([`udata_abort_signal`]).
    fn fault(&self, f: UserFault);
    /// `trapsignal(p, sig, trapno, code, sv)` with `sv.sival_ptr = addr`.
    fn signal(&self, sig: i32, trapno: u64, code: i32, addr: usize);
    /// `KERNEL_LOCK(); sigexit(p, sig)`: the process killed; never returns.
    fn kill(&self, sig: i32) -> !;
    /// `userret(p)`, then the AST loop ([`user_exit`]): the end of a synchronous trap.
    fn leave(&self) -> UserExit;
    /// `ast()`'s work on the running thread ([`ast`]).
    fn ast(&self);
    /// `mi_syscall_return(p, code, error, rval)`, after the ABI wrote the return registers.
    fn syscall_return(&self, code: Register, error: SysResult, rval: &Rval);
}

/// The kernel's answers to a trap from user mode ([`UserTraps`]), over the threads of glue `G`:
/// installed once by the image's boot glue (`bsd`'s `hal_bridges_register`, until N9).
pub struct KernelTrap<G: ProcGlue>(core::marker::PhantomData<fn() -> G>);

impl<G: ProcGlue> KernelTrap<G> {
    /// The table (a `static` of the glue's).
    pub const fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<G: ProcGlue> Default for KernelTrap<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The installed table.
static TRAPS: OnceStatic<&'static dyn UserTraps> = OnceStatic::new();

/// Installs the table, once; a second call keeps the first and names it.
pub fn install(t: &'static dyn UserTraps) -> Result<(), AlreadyRegistered> {
    TRAPS.set(t).map_err(|_| AlreadyRegistered("UserTraps"))
}

/// The installed table; `panic(9)` for `what` before it is installed, which no trap from user
/// mode can see (the boot glue installs it before the first thread leaves for user mode).
#[inline]
fn traps(what: &str) -> &'static dyn UserTraps {
    match TRAPS.get() {
        Some(t) => *t,
        None => kpanic(format_args!("{what}: no user trap table")),
    }
}

/// `refreshcreds(p)` for the thread that trapped ([`UserTraps::enter`]).
#[inline]
pub fn enter() {
    traps("enter").enter();
}

/// A fault from user mode ([`UserTraps::fault`]).
#[inline]
pub fn fault(f: UserFault) {
    traps("fault").fault(f);
}

/// `trapsignal` for the thread that trapped ([`UserTraps::signal`]).
#[inline]
pub fn signal(sig: i32, trapno: u64, code: i32, addr: usize) {
    traps("signal").signal(sig, trapno, code, addr);
}

/// The thread that trapped killed by `sig` ([`UserTraps::kill`]).
pub fn kill(sig: i32) -> ! {
    traps("kill").kill(sig)
}

/// The end of a synchronous trap: `userret` and the AST loop ([`UserTraps::leave`]), the
/// gate's token for the return to user mode.
#[inline]
pub fn leave() -> UserExit {
    traps("leave").leave()
}

/// The end of an interrupt taken in user mode (the C's `do_ast` after `arm_cpu_irq`): the AST
/// loop alone, which calls through the table only for a pending AST.
#[inline]
pub fn exit() -> UserExit {
    let mut gate = ExitGate::new();
    loop {
        match gate.pass() {
            Ok(exit) => return exit,
            // handle the ast
            Err(AstPending) => traps("ast").ast(),
        }
    }
}

/// `mi_syscall_return(p, code, error, rval)` for the running thread, which the ABI calls
/// (`kernel::api::syscall_return`) once it wrote the return registers or rewound the pc
/// (decision 53): the end of the C's `svc_handler`, through the table, since this function is
/// not generic over the glue.
#[inline]
pub fn syscall_return(code: Register, error: SysResult, rval: &Rval) {
    traps("syscall_return").syscall_return(code, error, rval);
}

/// `udata_abort`'s choice of signal for a fault `uvm_fault` could not serve: no memory kills
/// (`SIGKILL`), an I/O error is `SIGBUS` (`BUS_OBJERR`), a protection error `SIGSEGV`
/// (`SEGV_ACCERR`), anything else `SIGSEGV` (`SEGV_MAPERR`).
pub fn udata_abort_signal(error: Errno) -> (i32, i32) {
    if error == Errno::ENOMEM {
        (SIGKILL, 0)
    } else if error == Errno::EIO {
        (SIGBUS, BUS_OBJERR)
    } else if error == Errno::EACCES {
        (SIGSEGV, SEGV_ACCERR)
    } else {
        (SIGSEGV, SEGV_MAPERR)
    }
}

/// The register a read of `S3_0_C0_<crm>_<op2>` names when the kernel answers it
/// ([`ID_READS`]), `None` when it does not.
pub fn id_reg(crm: u8, op2: u8) -> Option<IdReg> {
    ID_READS
        .get(usize::from(crm))
        .and_then(|row| row.get(usize::from(op2)))
        .copied()
        .flatten()
}

/// `cpu_id_aa64pfr0`: the userland view of `ID_AA64PFR0_EL1`, which `bsd`'s CPU code fills
/// from the boot CPU and masks (`cpu_identify_cleanup`), [`id_value`]'s answer and
/// `sysctl machdep.id_aa64pfr0`'s.
pub static CPU_ID_AA64PFR0: AtomicU64 = AtomicU64::new(0);
/// `cpu_id_aa64pfr1`.
pub static CPU_ID_AA64PFR1: AtomicU64 = AtomicU64::new(0);
/// `cpu_id_aa64isar0`.
pub static CPU_ID_AA64ISAR0: AtomicU64 = AtomicU64::new(0);
/// `cpu_id_aa64isar1`.
pub static CPU_ID_AA64ISAR1: AtomicU64 = AtomicU64::new(0);
/// `cpu_id_aa64isar2`.
pub static CPU_ID_AA64ISAR2: AtomicU64 = AtomicU64::new(0);

/// What a program reading `r` is told: the kernel's computed views of the feature registers
/// (`cpu_id_aa64*`), `MPIDR_EL1`'s fixed value (bit 31 is RES1; the topology is not revealed),
/// 0 for the others; `None` for `MIDR_EL1`, the CPU's own, which the trap reads live.
pub fn id_value(r: IdReg) -> Option<u64> {
    let id = |a: &AtomicU64| a.load(Ordering::Relaxed);
    #[allow(
        clippy::match_same_arms,
        reason = "the C keeps the cases apart: one switch case per register"
    )]
    Some(match r {
        IdReg::Midr => return None,
        // MPIDR_EL1: don't reveal the topology to userland. But return a valid value; Bit
        // 31 is RES1.
        IdReg::Mpidr => 0x8000_0000,
        IdReg::Revidr => 0,
        IdReg::Pfr0 => id(&CPU_ID_AA64PFR0),
        IdReg::Pfr1 => id(&CPU_ID_AA64PFR1),
        IdReg::Pfr2 | IdReg::Zfr0 | IdReg::Smfr0 => 0,
        IdReg::Isar0 => id(&CPU_ID_AA64ISAR0),
        IdReg::Isar1 => id(&CPU_ID_AA64ISAR1),
        IdReg::Isar2 => id(&CPU_ID_AA64ISAR2),
        IdReg::Mmfr0 | IdReg::Mmfr1 | IdReg::Mmfr2 | IdReg::Mmfr3 | IdReg::Mmfr4 => 0,
    })
}

/// `do_el0_sync`'s prologue for the trap `entry` classifies: `hal`'s half (the syndrome, the
/// interrupts, the counter, the frame recorded) on the token the assembly passed, then
/// `refreshcreds` ([`enter`]). The trap and its syndrome go back to `entry`, which matches on
/// the class and hands both to that class's function here.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn begin(a: Arrival) -> (SyncTrap, Syndrome) {
    let r = a.begin();
    enter();
    r
}

/// `EXCP_UNKNOWN` (an undefined instruction, `udf`): the branch predictor flushed, `SIGILL`
/// (`ILL_ILLOPC`) at the pc.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn undefined(t: SyncTrap, s: Syndrome) {
    t.flush_bp();
    signal(SIGILL, s.esr, ILL_ILLOPC, t.pc());
}

/// `EXCP_SVE`: `sve_load(p)` (`hal`).
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn sve(t: SyncTrap) {
    t.sve_load();
}

/// `EXCP_FP_SIMD`, `EXCP_TRAP_FP`: `fpu_load(p)` (`hal`), the thread's FP state loaded at its
/// first FP instruction since its last switch.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn fpu(t: SyncTrap) {
    t.fp_load();
}

/// `EXCP_BRANCH_TGT` (a BTI fault): the branch predictor flushed, `SIGILL` (`ILL_BTCFI`) at
/// the pc.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn bti(t: SyncTrap, s: Syndrome) {
    t.flush_bp();
    signal(SIGILL, s.esr, ILL_BTCFI, t.pc());
}

/// `EXCP_MSR`: `emulate_msr`, a read of an identification register answered with the value
/// the kernel announces ([`id_reg`], [`id_value`]) and the instruction skipped, the decode and
/// the register's write `hal`'s on the trap's token (decision 56's U7, amended by the user's
/// lock: the list is the kernel's); any other access falls through to `EXCP_FPAC`'s `SIGILL`
/// (`ILL_ILLOPC`) at the pc, the branch predictor flushed.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn msr(t: SyncTrap, s: Syndrome) {
    let answered = t
        .id_read(s.esr)
        .and_then(|r| id_reg(r.crm, r.op2).map(|id| (r, id)));
    match answered {
        Some((r, id)) => {
            let value = id_value(id).unwrap_or_else(midr);
            t.id_answer(r, value);
        }
        None => fpac(t, s),
    }
}

/// `EXCP_FPAC` (a pointer authentication failure): the branch predictor flushed, `SIGILL`
/// (`ILL_ILLOPC`) at the pc.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn fpac(t: SyncTrap, s: Syndrome) {
    t.flush_bp();
    signal(SIGILL, s.esr, ILL_ILLOPC, t.pc());
}

/// `EXCP_SVC`: `svc_handler`'s machine half (`hal`: the counter, the barrier skipped, the
/// user's interrupts), handing out the trapping thread's registers, which `entry` gives the
/// ABI's handler (`abi::arch::current::svc`): the convention is the ABI's, decision 53.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn svc(t: SyncTrap) -> UserRegs {
    t.svc()
}

/// `EXCP_INSN_ABORT_L`, `EXCP_DATA_ABORT_L` (`exe`: an instruction abort): `udata_abort`, the
/// decode of the syndrome (the branch predictor flushed for a kernel address, an alignment
/// fault's `SIGBUS` at the address), then the fault on the process's map ([`fault`]: the
/// `MAP_STACK` check of the frame's stack pointer first, a fault it cannot serve signalled).
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
#[allow(
    clippy::cast_possible_truncation,
    reason = "far is an address: 64 bits on the only machine this function is built for"
)]
pub fn udata_abort(t: SyncTrap, s: Syndrome, exe: bool) {
    let access = accesstype(s.esr, exe);
    let addr = s.far as usize;

    let va = trunc_page(addr);
    if va >= VM_MAXUSER_ADDRESS {
        t.flush_bp();
    }

    if s.esr & ISS_DATA_DFSC_MASK == ISS_DATA_DFSC_ALIGN {
        signal(SIGBUS, s.esr, BUS_ADRALN, addr);
        return;
    }

    fault(UserFault {
        sp: t.sp(),
        va,
        access,
        trapno: s.esr,
        addr,
    });
}

/// `EXCP_PC_ALIGN`: the branch predictor flushed, `SIGBUS` (`BUS_ADRALN`) at the pc.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn pc_align(t: SyncTrap, s: Syndrome) {
    t.flush_bp();
    signal(SIGBUS, s.esr, BUS_ADRALN, t.pc());
}

/// `EXCP_SP_ALIGN`: the branch predictor flushed, `SIGBUS` (`BUS_ADRALN`) at the stack
/// pointer.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn sp_align(t: SyncTrap, s: Syndrome) {
    t.flush_bp();
    signal(SIGBUS, s.esr, BUS_ADRALN, t.sp());
}

/// `EXCP_BRK` (`brk`): `SIGTRAP` (`TRAP_BRKPT`) at the pc.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn breakpoint(t: SyncTrap, s: Syndrome) {
    signal(SIGTRAP, s.esr, TRAP_BRKPT, t.pc());
}

/// `EXCP_SOFTSTP_EL0` (a single step): `SIGTRAP` (`TRAP_TRACE`) at the pc.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn single_step(t: SyncTrap, s: Syndrome) {
    signal(SIGTRAP, s.esr, TRAP_TRACE, t.pc());
}

/// Any other class (`do_el0_sync`'s `default:`, "USERLAND MUST NOT PANIC MACHINE"): the class,
/// the syndrome and the registers printed, the branch predictor flushed, then the process
/// killed with `SIGILL` under the kernel lock ([`kill`]). Never returns.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn unknown(t: SyncTrap, s: Syndrome) -> ! {
    // panic("Unknown userland exception %x esr_el1 %lx", exception, esr);
    // only here to debug !?!?
    t.dump(s.class, s.esr);
    t.flush_bp();
    kill(SIGILL)
}

/// `ast(tf)`: the asynchronous system trap's work for the thread `p` returning to user mode:
/// its credentials refreshed, the counter, `mi_ast` (profiling, the preemption
/// `ci_want_resched` asks for), then `userret` (the signals).
pub fn ast<G: ProcGlue>(p: Pin<&Thread<G>>) {
    crate::syscall::refreshcreds(&p);
    UVMEXP.softs.fetch_add(1, Ordering::Relaxed);
    crate::syscall::mi_ast(p, hal::cpu::curcpu().want_resched() != 0);
    crate::sig::userret(p);
}

/// `do_ast`'s loop: [`ast`] for the running thread while the exit gate finds one pending,
/// then the gate's token (the interrupts masked, nothing pending) for the return to user
/// mode. A thread that sleeps or moves to another CPU in `ast` passes the gate again where it
/// resumes, as the C's loop does.
pub fn user_exit<G: ProcGlue>() -> UserExit {
    let mut gate = ExitGate::new();
    loop {
        match gate.pass() {
            Ok(exit) => return exit,
            // handle the ast
            Err(AstPending) => hal::cpu::thread::with_current::<Thread<G>, _>(ast::<G>),
        }
    }
}

/// The running thread, for `f`; a trap without one is `panic(9)` (the trap checked it).
fn with_trapped<G: ProcGlue, R>(f: impl FnOnce(Pin<&Thread<G>>) -> R) -> R {
    hal::cpu::thread::with_current::<Thread<G>, R>(f)
}

impl<G: ProcGlue> UserTraps for KernelTrap<G> {
    fn enter(&self) {
        // refresh the thread's cache of the process's creds
        with_trapped::<G, _>(|p| crate::syscall::refreshcreds(&p));
    }

    fn fault(&self, f: UserFault) {
        with_trapped::<G, _>(|p| {
            let served = crate::syscall::user_page_fault(p, f.sp, |vm| {
                crate::syscall::uvm_user_fault(&p, vm, f.va, f.access)
            });
            if let Err(error) = served {
                let (sig, code) = udata_abort_signal(error);
                crate::sig::trapsignal(p, sig, f.trapno, code, Sigval::from_ptr(f.addr));
            }
        });
    }

    fn signal(&self, sig: i32, trapno: u64, code: i32, addr: usize) {
        with_trapped::<G, _>(|p| {
            crate::sig::trapsignal(p, sig, trapno, code, Sigval::from_ptr(addr));
        });
    }

    fn kill(&self, sig: i32) -> ! {
        hal::sync::klock::kernel_lock(); // KERNEL_LOCK()
        with_trapped::<G, _>(|p| crate::sig::sigexit(&p, sig))
        // KERNEL_UNLOCK(): sigexit does not return.
    }

    fn leave(&self) -> UserExit {
        with_trapped::<G, _>(crate::sig::userret);
        user_exit::<G>()
    }

    fn ast(&self) {
        with_trapped::<G, _>(ast::<G>);
    }

    fn syscall_return(&self, code: Register, error: SysResult, rval: &Rval) {
        let error = error.map_err(|e| Errno::from_raw(e).unwrap_or(Errno::EINVAL));
        with_trapped::<G, _>(|p| crate::syscall::mi_syscall_return(p, code, error, rval));
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sched::tests::{SchedGlue, VCPUS, run_as, setup, thread};

    /// The tests here count `uvmexp.softs`, which only [`ast`] increments.
    fn softs() -> i64 {
        i64::from(UVMEXP.softs.load(Ordering::Relaxed))
    }

    /// The table the tests install: the kernel's own, over the scheduler tests' glue.
    static KT: KernelTrap<SchedGlue> = KernelTrap::new();

    /// Nothing pending: the gate opens at once and no AST runs.
    #[test]
    fn no_ast_pending_leaves_at_once() {
        let _g = setup();
        let me = thread(40, 50);
        run_as(VCPUS[0], me);
        let before = softs();
        let _exit = user_exit::<SchedGlue>();
        assert_eq!(softs(), before);
        hal::cpu::thread::set_current_static::<Thread<SchedGlue>>(None);
    }

    /// A pending AST runs once (`aston`, as `signotify` posts it), is cleared, and the loop
    /// leaves at the next pass.
    #[test]
    fn a_pending_ast_runs_once_then_the_gate_opens() {
        let _g = setup();
        let me = thread(41, 50);
        run_as(VCPUS[0], me);
        me.machine.aston();
        let before = softs();
        let _exit = user_exit::<SchedGlue>();
        assert_eq!(softs(), before + 1);
        let _exit = user_exit::<SchedGlue>();
        assert_eq!(softs(), before + 1, "the AST was cleared by the first pass");
        hal::cpu::thread::set_current_static::<Thread<SchedGlue>>(None);
    }

    /// An interrupt's way back ([`exit`]): through the installed table only for a pending AST,
    /// once, then the gate opens.
    #[test]
    fn an_interrupts_exit_runs_a_pending_ast_through_the_table() {
        let _g = setup();
        let first = install(&KT);
        assert!(first.is_ok() || first == Err(AlreadyRegistered("UserTraps")));
        assert_eq!(install(&KT), Err(AlreadyRegistered("UserTraps")));
        let me = thread(42, 50);
        run_as(VCPUS[0], me);
        let before = softs();
        let _exit = exit();
        assert_eq!(
            softs(),
            before,
            "nothing pending: no call through the table"
        );
        me.machine.aston();
        let _exit = exit();
        assert_eq!(softs(), before + 1);
        hal::cpu::thread::set_current_static::<Thread<SchedGlue>>(None);
    }

    /// `emulate_msr`'s sixteen reads (`trap.c:233-299`), by CRm and op2: exactly these are
    /// answered, every other read of the identification space is `SIGILL`.
    #[test]
    fn the_id_reads_answered_are_the_cs_sixteen() {
        let want = [
            (0, 0, IdReg::Midr),
            (0, 5, IdReg::Mpidr),
            (0, 6, IdReg::Revidr),
            (4, 0, IdReg::Pfr0),
            (4, 1, IdReg::Pfr1),
            (4, 2, IdReg::Pfr2),
            (4, 4, IdReg::Zfr0),
            (4, 5, IdReg::Smfr0),
            (6, 0, IdReg::Isar0),
            (6, 1, IdReg::Isar1),
            (6, 2, IdReg::Isar2),
            (7, 0, IdReg::Mmfr0),
            (7, 1, IdReg::Mmfr1),
            (7, 2, IdReg::Mmfr2),
            (7, 3, IdReg::Mmfr3),
            (7, 4, IdReg::Mmfr4),
        ];
        let mut answered = 0;
        for crm in 0..=15u8 {
            for op2 in 0..=7u8 {
                let w = want
                    .iter()
                    .find(|(c, o, _)| (*c, *o) == (crm, op2))
                    .map(|(_, _, r)| *r);
                assert_eq!(id_reg(crm, op2), w, "CRm {crm} op2 {op2}");
                answered += usize::from(w.is_some());
            }
        }
        assert_eq!(answered, 16);
        // Out of the table (the decode never gives these): not answered.
        assert_eq!(id_reg(16, 0), None);
        assert_eq!(id_reg(0, 8), None);
    }

    /// What a program is told: the kernel's views, MPIDR's fixed value, zeros; MIDR live.
    #[test]
    fn the_id_values_are_the_kernels() {
        CPU_ID_AA64PFR0.store(0x1100_0000_0011_1112, Ordering::Relaxed);
        CPU_ID_AA64PFR1.store(0x20, Ordering::Relaxed);
        CPU_ID_AA64ISAR0.store(0x0221_1000_1021_1120, Ordering::Relaxed);
        CPU_ID_AA64ISAR1.store(0x10_0001, Ordering::Relaxed);
        CPU_ID_AA64ISAR2.store(0x3, Ordering::Relaxed);
        assert_eq!(id_value(IdReg::Midr), None);
        assert_eq!(id_value(IdReg::Mpidr), Some(0x8000_0000));
        assert_eq!(id_value(IdReg::Pfr0), Some(0x1100_0000_0011_1112));
        assert_eq!(id_value(IdReg::Pfr1), Some(0x20));
        assert_eq!(id_value(IdReg::Isar0), Some(0x0221_1000_1021_1120));
        assert_eq!(id_value(IdReg::Isar1), Some(0x10_0001));
        assert_eq!(id_value(IdReg::Isar2), Some(0x3));
        for r in [
            IdReg::Revidr,
            IdReg::Pfr2,
            IdReg::Zfr0,
            IdReg::Smfr0,
            IdReg::Mmfr0,
            IdReg::Mmfr1,
            IdReg::Mmfr2,
            IdReg::Mmfr3,
            IdReg::Mmfr4,
        ] {
            assert_eq!(id_value(r), Some(0), "{r:?}");
        }
    }

    /// `udata_abort`'s table of errno to signal, the C's (`trap.c:143-155`).
    #[test]
    fn a_fault_not_served_is_the_cs_signal() {
        for (error, want) in [
            (Errno::ENOMEM, (SIGKILL, 0)),
            (Errno::EIO, (SIGBUS, BUS_OBJERR)),
            (Errno::EACCES, (SIGSEGV, SEGV_ACCERR)),
            (Errno::EFAULT, (SIGSEGV, SEGV_MAPERR)),
            (Errno::EINVAL, (SIGSEGV, SEGV_MAPERR)),
        ] {
            assert_eq!(udata_abort_signal(error), want, "{error:?}");
        }
    }
}
/* </TESTS> */
