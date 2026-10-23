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
/* </LICENSES> */

/* <CODE> */
//! What the machine's trap code asks the rest of the kernel: the registrations behind the
//! exception entry (`arch/<machine>/trap.rs`, `syscall.rs`).
//!
//! The machine keeps the decode (the syndrome and fault address, which map a fault belongs
//! to, the frame's fix-ups, `pcb_onfault`, the FPU traps, the instruction emulation) and asks
//! for every decision the C makes in machine-independent code through one registered
//! [`TrapHandler`]: the page fault (`uvm_fault` on the right map, `uvm_grow`, the
//! `MAP_STACK` check of a fault from user mode), the signal (`trapsignal`, `sigexit`),
//! `refreshcreds`, `userret`, the system call's return (`mi_syscall_return`), the AST
//! (`mi_ast`) and a forked child's first return (`mi_child_return`). The machine's hooks the
//! C keeps in its `struct cpu_info` (`ci_flush_bp`, `ci_serror`, which the CPU and PSCI
//! drivers still in `bsd` choose and rewrite) are [`CpuHooks`]; the kernel debugger
//! (`db_ktrap`) is [`Debugger`]; `fault()`'s claim of the CPU's panic buffer is the hooks'
//! too.
//!
//! **Before registration** (early boot): a kernel page fault is a failed fault (the
//! `pcb_onfault` recovery, else `panic(9)`, as an unserved `uvm_fault`); the hooks do nothing
//! and no debugger answers (`db_ktrap` returns 0, as the C without `DDB`). No user trap can
//! come before `bsd` registers, since no user process runs yet.
//!
//! ## Redesign
//! - Not a port: the C's `trap.c` calls `uvm_fault`, `trapsignal`, `userret`, ... by name.
//!   Here the machine code is `hal`'s and those are the kernel's, so they are reached through
//!   one trait registered once, as the system call dispatch point is (`crate::syscall`): the
//!   arrows go down, and what goes up is one audited registration.
//! - `ci_flush_bp`, `ci_serror` (`struct cpu_info`'s function pointers) are [`CpuHooks`],
//!   whose methods act on the running CPU's choice: the PSCI driver rewrites the choice on
//!   its first call (`psci_flush_bp`), which a set-once slot per CPU could not hold. So is
//!   `fault()`'s `ci_panicbuf`, a member of the machine's `cpu_info` that only its owner
//!   names.

use crate::bridges::AlreadyRegistered;
use crate::once::OnceStatic;
use crate::pmap::VmProt;
use crate::sys::errno::Errno;
use crate::syscall::{Register, Rval, SysResult};

/// The machine's trap frame (`struct trapframe`), as the exception entry saved it.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub use crate::arch::arm64::include::frame::Trapframe;

/// The host's trap frame: no exception entry runs there, so nothing is saved.
#[cfg(not(target_os = "none"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct Trapframe;

/// Which map serves a page fault, as the machine decided from the fault (the C's choice of
/// `map` in `udata_abort` and `kdata_abort`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultSource {
    /// A fault from user mode (`udata_abort`): the running process's map, after the
    /// `MAP_STACK` check of its stack pointer (`uvm_map_inentry`), and `uvm_grow` when served.
    User,
    /// A kernel access to a user address by an unprivileged load or store
    /// (`kdata_abort`'s `&p->p_vmspace->vm_map`: the copy routines): the running process's
    /// map, `uvm_grow` when served.
    KernelUser,
    /// A kernel address, or a user one under `pcb_onfault` (`kdata_abort`'s `kernel_map`).
    Kernel,
}

/// What the machine's trap code asks the kernel (the machine-independent calls of `trap.c`,
/// `syscall.c` and `ast.c`), registered once by the kernel's boot path ([`install`]).
/// Every method acts on the running thread (`curproc`).
pub trait TrapHandler: Sync {
    /// `pmap_fault_fixup(map->pmap, va, access)`, then `uvm_fault(map, va, 0, access)` and,
    /// for a process's map, `uvm_grow(p, va)` when it was served, on the map `source` names;
    /// for [`FaultSource::User`] the `MAP_STACK` check comes first, and when it fails (the
    /// check signalled the thread) the fault is `Ok` with nothing more to do. `va` is the
    /// faulting page (`trunc_page(far)`); the error is `uvm_fault`'s.
    fn page_fault(&self, source: FaultSource, va: usize, access: VmProt) -> Result<(), Errno>;
    /// `trapsignal(p, sig, trapno, code, sv)` with `sv.sival_ptr = addr`.
    fn trapsignal(&self, sig: i32, trapno: u64, code: i32, addr: usize);
    /// `sigexit(p, sig)`, the kernel lock held by the caller; never returns.
    fn sigexit(&self, sig: i32) -> !;
    /// `refreshcreds(p)`.
    fn refreshcreds(&self);
    /// `userret(p)`: the last thing before the thread returns to user mode.
    fn userret(&self);
    /// `mi_syscall_return(p, code, error, rval)`, after the machine set the registers.
    fn syscall_return(&self, code: Register, error: SysResult, rval: &Rval);
    /// `mi_ast(p, resched)` (`ast()`'s machine-independent part, before its `userret`).
    fn ast(&self, resched: bool);
    /// `mi_child_return(p)`: a forked child's first return, after the machine set its
    /// return registers and released the kernel lock its trampoline held.
    fn child_return(&self);
}

/// The machine's per-CPU hooks a trap calls (`struct cpu_info`'s `ci_flush_bp` and
/// `ci_serror`), as the CPU code that chooses them registers them (`bsd`'s arm64 `cpu.c`
/// and its PSCI driver, N7). The defaults are a CPU that needs neither.
pub trait CpuHooks: Sync {
    /// `curcpu()->ci_flush_bp()`: the branch predictor flushed (Spectre-V2), as the running
    /// CPU's choice says.
    fn flush_bp(&self) {}
    /// `ci->ci_serror()`, when the running CPU has one: the implementation's own SError
    /// report.
    fn serror(&self) {}
    /// `fault(fmt, ...)` of the trap code: `panicstr` claimed for the running CPU's
    /// `ci_panicbuf`, `args` formatted into it and printed (`db_printf("%s\n")` under
    /// `DDB`), before the `panic(9)` or the debugger that follows. The default prints the
    /// line.
    fn fault(&self, args: core::fmt::Arguments<'_>) {
        crate::prf::printf(format_args!("{args}\n"));
    }
}

/// The kernel debugger's entry from a trap (`db_ktrap`), registered by `ddb` (`bsd`'s, N9).
pub trait Debugger: Sync {
    /// `db_ktrap(type, frame)`: whether the debugger handled the trap.
    fn ktrap(&self, kind: i32, frame: &mut Trapframe) -> bool;
}

/// The registered trap handler.
static HANDLER: OnceStatic<&'static dyn TrapHandler> = OnceStatic::new();
/// The registered CPU hooks.
static HOOKS: OnceStatic<&'static dyn CpuHooks> = OnceStatic::new();
/// The registered debugger.
static DEBUGGER: OnceStatic<&'static dyn Debugger> = OnceStatic::new();

/// Registers the trap handler, once (a second registration keeps the first).
pub fn install(h: &'static dyn TrapHandler) -> Result<(), AlreadyRegistered> {
    HANDLER.set(h).map_err(|_| AlreadyRegistered("TrapHandler"))
}

/// Registers the CPU hooks, once.
pub fn install_cpu_hooks(h: &'static dyn CpuHooks) -> Result<(), AlreadyRegistered> {
    HOOKS.set(h).map_err(|_| AlreadyRegistered("CpuHooks"))
}

/// Registers the kernel debugger, once.
pub fn install_debugger(d: &'static dyn Debugger) -> Result<(), AlreadyRegistered> {
    DEBUGGER.set(d).map_err(|_| AlreadyRegistered("Debugger"))
}

/// The registered trap handler, `None` before registration.
#[cfg_attr(
    any(
        not(all(target_os = "none", target_arch = "aarch64")),
        feature = "module-rt"
    ),
    allow(dead_code, reason = "the machine's trap code alone calls it")
)]
#[inline]
pub(crate) fn handler() -> Option<&'static dyn TrapHandler> {
    HANDLER.get().copied()
}

/// `curcpu()->ci_flush_bp()`: nothing before the hooks are registered.
#[cfg_attr(
    any(
        not(all(target_os = "none", target_arch = "aarch64")),
        feature = "module-rt"
    ),
    allow(dead_code, reason = "the machine's trap code alone calls it")
)]
#[inline]
pub(crate) fn flush_bp() {
    if let Some(h) = HOOKS.get() {
        h.flush_bp();
    }
}

/// `fault(fmt, ...)`: the registered hooks' (the running CPU's panic buffer), a printed line
/// before.
#[cfg_attr(
    any(
        not(all(target_os = "none", target_arch = "aarch64")),
        feature = "module-rt"
    ),
    allow(dead_code, reason = "the machine's trap code alone calls it")
)]
pub(crate) fn fault(args: core::fmt::Arguments<'_>) {
    match HOOKS.get() {
        Some(h) => h.fault(args),
        None => crate::prf::printf(format_args!("{args}\n")),
    }
}

/// `if (ci->ci_serror) ci->ci_serror()`.
#[cfg_attr(
    any(
        not(all(target_os = "none", target_arch = "aarch64")),
        feature = "module-rt"
    ),
    allow(dead_code, reason = "the machine's SError entry alone calls it")
)]
pub(crate) fn serror() {
    if let Some(h) = HOOKS.get() {
        h.serror();
    }
}

/// `db_ktrap(kind, frame)` when a debugger is registered; `None` without one (the C built
/// without `DDB`).
#[cfg_attr(
    any(
        not(all(target_os = "none", target_arch = "aarch64")),
        feature = "module-rt"
    ),
    allow(dead_code, reason = "the machine's trap code alone calls it")
)]
pub(crate) fn debugger_ktrap(kind: i32, frame: &mut Trapframe) -> Option<bool> {
    DEBUGGER.get().map(|d| d.ktrap(kind, frame))
}

/// Whether a debugger is registered (the C's `#ifdef DDB`).
#[cfg_attr(
    any(
        not(all(target_os = "none", target_arch = "aarch64")),
        feature = "module-rt"
    ),
    allow(dead_code, reason = "the machine's trap code alone calls it")
)]
pub(crate) fn have_debugger() -> bool {
    DEBUGGER.is_set()
}

/// `PROC_PC(p)` of the running thread: the user program counter of its trap frame
/// (`pcb_tf`; the instruction after the `svc` during a system call), 0 for a thread with
/// none. The running thread's alone: no other thread's frame is read from this CPU.
#[cfg(any(
    not(target_os = "none"),
    all(target_arch = "aarch64", not(feature = "module-rt"))
))]
pub fn proc_pc() -> usize {
    user_regs::proc_pc()
}

/// `PROC_STACK(p)` of the running thread: the user stack pointer of its trap frame, 0 for a
/// thread with none.
#[cfg(any(
    not(target_os = "none"),
    all(target_arch = "aarch64", not(feature = "module-rt"))
))]
pub fn proc_stack() -> usize {
    user_regs::proc_stack()
}

/// The machine's user frame (arm64's `UserFrame`).
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    not(feature = "module-rt")
))]
use crate::arch::arm64::trap as user_regs;
/// The host's double: the registers a test set.
#[cfg(not(target_os = "none"))]
use crate::arch::host::sig as user_regs;

/// The host double's knob for [`proc_pc`] and [`proc_stack`].
#[cfg(not(target_os = "none"))]
pub use crate::arch::host::sig::set_user_regs;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    struct Hooks;
    impl CpuHooks for Hooks {}

    /// The hooks' defaults do nothing, and a second registration keeps the first.
    #[test]
    fn hooks_register_once() {
        flush_bp();
        serror();
        static H: Hooks = Hooks;
        let first = install_cpu_hooks(&H);
        assert!(first.is_ok() || first == Err(AlreadyRegistered("CpuHooks")));
        assert_eq!(install_cpu_hooks(&H), Err(AlreadyRegistered("CpuHooks")));
        flush_bp();
        serror();
    }

    /// Without a debugger, a trap is not the debugger's.
    #[test]
    fn no_debugger_answers_none() {
        if !have_debugger() {
            assert_eq!(debugger_ktrap(0, &mut Trapframe), None);
        }
    }
}
/* </TESTS> */
