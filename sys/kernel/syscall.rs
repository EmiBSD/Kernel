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
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)kern_xxx.c	8.2 (Berkeley) 11/14/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/syscall_mi.h>`: the machine-independent part of a system call, around the
//! machine's entry (`hal::arch::arm64::syscall::svc_handler`), and the machine-independent
//! calls of the machine's trap code (`hal::trap::TrapHandler`).
//!
//! Upstream: sys/sys/syscall_mi.h @ 3ce1f3f79392
//! LZ: sys/sys/syscall_mi.rs@3c62ede68802
//!
//! [`mi_syscall`] is the C's prologue around a system call's body: the thread's credentials
//! refreshed, the stack pointer checked to lie in a `MAP_STACK` mapping (`uvm_map_inentry`),
//! the call site checked against the pinned system calls ([`pin_check`]), `pledge(2)`, and
//! the kernel lock around the body unless the call is `SY_NOLOCK`. Every path into a system
//! call's body comes through it: `bsd`'s `Legacy` fallback (`sysent[]`) and each `Native`
//! entry of the ABI module's table (`bsd`'s wrappers over the kernel's functions), the body
//! a closure over the row. [`KernelTrap`] is what `hal`'s trap code asks: the page fault on
//! the map the trap chose, the signals, `refreshcreds`, `userret`, the system call's return,
//! the AST and a forked child's first return.
//!
//! ## Deviations
//! - `KTRACE` and dt(4) are not configured; `SYSCALL_DEBUG` is `bsd`'s feature
//!   `syscall_debug`, reached through the glue (`ProcGlue::scdebug_call`, `scdebug_ret`).
//! - `SY_NOLOCK` is honoured as in C for the system calls whose bodies are audited for
//!   `MULTIPROCESSOR`; those whose bodies are not ported ([`SY_NOLOCK_DEFERRED`]) still run
//!   under the kernel lock (LZ's).
//! - `mi_ast`'s `ADDUPROF` (the profiling clock, `subr_prof.c`) is reported (`unported!`).
//!
//! ## Redesign
//! - N4.8b: from `bsd`'s `sys/sys/syscall_mi.rs` (which stays as the adapter of its callers,
//!   amd64's trap code among them) into the kernel, generic over the glue. `mi_syscall`
//!   takes the row's body as a closure and its lock as a value (`syscall_lock` over the
//!   row's `sy_flags`), so the table stays the caller's (`bsd`'s `sysent[]`, the ABI
//!   module's).
//! - The `MAP_STACK` check is made (LZ reported it `unported!` and went on: gap 1 of the
//!   brief, the user's decision 5 of N4.8): a system call whose stack pointer is not in a
//!   `MAP_STACK` mapping gets `SIGSEGV` (from `uvm_map_inentry`) and fails with `EPERM`, as
//!   in the C; a thread whose glue names no address space ([`ProcGlue::user_vmspace`],
//!   `None`) fails the check, so a glue cannot skip it by forgetting it.
//! - `pin_check` reads the pin tables through `hal::slot::ReadMostly` (N4.6a) and the
//!   machine's facts (`PROC_PC`, the `sigcode` offsets) through the glue; the tables' guards
//!   end before the kernel lock and the terminal are taken (a `ReadMostly` reader never
//!   waits).
//! - `pin_check`'s message goes to the process's controlling terminal with
//!   `kernel::tty::uprintf`, as the C's `uprintf` (LZ printed it on the console: it had no
//!   `uprintf`); a process without one, as diff-openbsd's `syscalls/entry` children, prints
//!   nothing.
//! - `KernelTrap<G>` implements `hal::trap::TrapHandler` (`bsd`'s `BsdTrap` before, N4.8b
//!   b1): the fault on the map `hal` chose (`pmap_fault_fixup`, `uvm_fault`, `uvm_grow`, the
//!   `MAP_STACK` check of a user fault), `trapsignal`, `sigexit`, `refreshcreds`, `userret`,
//!   [`mi_syscall_return`], [`mi_ast`] and [`mi_child_return`], on the running thread.

use core::marker::PhantomData;
use core::pin::Pin;
use core::sync::atomic::Ordering;

use hal::sys::errno::Errno;
use hal::sys::syscall::{SYS___thrsleep, SYS___thrwakeup, SYS_MAXSYSCALL, SYS_sigreturn};
use hal::syscall::{Register, Rval, SysArgs, SysResult};
use hal::trap::{FaultSource, TrapHandler};

use crate::acct::APINSYS;
use crate::assert::kpanic;
use crate::proc::{P_OWEUPC, PS_PLEDGE, PinTable, ProcGlue, SINGLE_DEEP, SINGLE_UNWIND, Thread};
use crate::vm::map::{Vmspace, uvm_map_inentry, uvm_map_inentry_sp};

/// `SY_NOLOCK` (`<sys/systm.h>`): a `sysent[]` row whose body runs without the kernel lock.
pub const SY_NOLOCK: i32 = 0x01;

/// `NOLOCK` system calls (`syscalls.master`) whose bodies are not audited for
/// `MULTIPROCESSOR`: [`mi_syscall`]'s callers keep the kernel lock for them although
/// `sysent` marks them `SY_NOLOCK` (see the module's deviations). Since the end of LZ's M11e
/// audit only the unported calls are left.
pub const SY_NOLOCK_DEFERRED: &[i32] = &[
    // __thrsleep(2) and __thrwakeup(2) are not ported (`sys_nosys`, which posts SIGSYS).
    SYS___thrsleep,
    SYS___thrwakeup,
];

/// [`SY_NOLOCK_DEFERRED`] as a table indexed by system call number.
const SY_NOLOCK_DEFERRED_MAP: [bool; SYS_MAXSYSCALL] = {
    let mut map = [false; SYS_MAXSYSCALL];
    let mut i = 0;
    while i < SY_NOLOCK_DEFERRED.len() {
        map[SY_NOLOCK_DEFERRED[i] as usize] = true;
        i += 1;
    }
    map
};

/// A command name as `%s` prints it: its bytes up to the NUL.
struct Comm<'a>(&'a [u8]);

impl core::fmt::Display for Comm<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for &b in self.0 {
            core::fmt::Write::write_char(f, char::from(b))?;
        }
        Ok(())
    }
}

/// What `hal`'s trap code asks the kernel (`hal::trap::TrapHandler`), for the threads of
/// glue `G`: registered once by the kernel's boot path (`bsd`'s `hal_bridges_register`).
pub struct KernelTrap<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> KernelTrap<G> {
    /// The handler (a `static` of the glue's).
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<G: ProcGlue> Default for KernelTrap<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether system call `code` with `sysent[]` flags `sy_flags` runs under the kernel lock:
/// the C's `!(callp->sy_flags & SY_NOLOCK)`, plus the not yet audited
/// [`SY_NOLOCK_DEFERRED`] (and a number past the table, `sys_nosys`'s).
pub fn syscall_lock(sy_flags: i32, code: Register) -> bool {
    sy_flags & SY_NOLOCK == 0
        || usize::try_from(code)
            .ok()
            .and_then(|code| SY_NOLOCK_DEFERRED_MAP.get(code))
            .is_none_or(|&deferred| deferred)
}

/// The system call pin table entry of `code` in `pin` (`pn_pins[code]`), `None` past the table
/// (or for a negative number).
fn pin_entry(pin: &PinTable, code: Register) -> Option<u32> {
    usize::try_from(code)
        .ok()
        .and_then(|c| pin.pins.as_ref()?.get(c).copied())
}

/// `pin_check`'s decision for system call `code` whose call instruction is at `addr`, the
/// thread's `PROC_PC` being `pc` and its process's `ps_sigcoderet` `sigcoderet`, against
/// `libc` (`ps_libcpin`) and `prog` (`ps_pin`): `Ok` when the call may proceed, else the error
/// and the table that answered (`None`: neither).
fn pin_verdict<'t>(
    addr: usize,
    pc: usize,
    sigcoderet: usize,
    code: Register,
    libc: &'t PinTable,
    prog: &'t PinTable,
) -> Result<(), (Errno, Option<&'t PinTable>)> {
    let within = |pin: &PinTable| pin.pins.is_some() && addr >= pin.start && addr < pin.end;

    // System calls come from the following places, checks are ordered by most common case:
    // 1) dynamic binary: syscalls in libc.so (in the ps_libcpin region)
    // 2a) static binary: syscalls in main program (in the ps_pin region)
    // 2b) dynamic binary: syscalls in ld.so (in the ps_pin region)
    // 3) sigtramp, containing only sigreturn(2)
    let pin = if within(libc) {
        libc
    } else if within(prog) {
        prog
    } else if pc == sigcoderet {
        if code == SYS_sigreturn as Register {
            return Ok(());
        }
        return Err((Errno::EPERM, None)); // goto die
    } else {
        return Err((Errno::ENOSYS, None));
    };
    match pin_entry(pin, code) {
        None | Some(0) => Err((Errno::ENOSYS, Some(pin))),
        // correct location
        Some(off) if (off as usize).wrapping_add(pin.start) == addr => Ok(()),
        // multiple locations, hopefully a boring operation
        Some(u32::MAX) => Ok(()),
        Some(_) => Err((Errno::ENOSYS, Some(pin))),
    }
}

/// `pin_check`: check if a system call is entered from precisely the correct location.
#[inline]
pub fn pin_check<G: ProcGlue>(p: Pin<&Thread<G>>, code: Register) -> Result<(), Errno> {
    pin_check_at(p, code, hal::trap::proc_pc())
}

/// [`pin_check`] for a thread whose `PROC_PC(p)` is `pc` (`mi_syscall` read it with the stack
/// pointer, in one access to the frame).
#[inline]
fn pin_check_at<G: ProcGlue>(p: Pin<&Thread<G>>, code: Register, pc: usize) -> Result<(), Errno> {
    let pr = p.process();

    // point at start of syscall instruction
    let addr = pc.wrapping_sub(G::sigcode_svc_offset());
    // The two tables, borrowed for the check (`hal::slot::ReadMostly`: exec and exit
    // replace them only when no thread reads them).
    let ppin = pr.ps_pin.read();
    let plibcpin = pr.ps_libcpin.read();

    let (error, pin) = match pin_verdict(addr, pc, pr.ps_sigcoderet.get(), code, &plibcpin, &ppin) {
        Ok(()) => return Ok(()),
        Err(v) => v,
    };

    // die:
    // What the message prints, copied out of the tables so their read guards end here: a
    // `ReadMostly` reader must not sleep or wait (exec's and exit's replacement spins until
    // the readers leave), and `KERNEL_LOCK()` and the terminal may do both.
    let pinoff = pin
        .and_then(|pin| pin_entry(pin, code))
        .map_or(-1i64, i64::from);
    let is = |which: &PinTable| {
        if pin.is_some_and(|pin| core::ptr::eq(pin, which)) {
            "(Y)"
        } else {
            ""
        }
    };
    let shown = |t: &PinTable| (is(t), t.npins(), t.start, t.end);
    let (pin_y, pin_n, pin_start, pin_end) = shown(&ppin);
    let (libc_y, libc_n, libc_start, libc_end) = shown(&plibcpin);
    drop(ppin);
    drop(plibcpin);

    // KTRACE (KTR_PINSYSCALL): not configured.
    hal::klock::kernel_lock(); // KERNEL_LOCK()
    // XXX remove or simplify this uprintf() call after OpenBSD 7.5 release
    let comm = pr.comm();
    crate::tty::uprintf(
        pr,
        format_args!(
            "{}[{}]: pinsyscalls addr {:x} code {}, pinoff {:#x} (pin{} {} {:x}-{:x} {:x}) \
         (libcpin{} {} {:x}-{:x} {:x}) error {}\n",
            Comm(&comm),
            pr.ps_pid,
            addr,
            code,
            pinoff as u32,
            pin_y,
            pin_n,
            pin_start,
            pin_end,
            pin_end.wrapping_sub(pin_start),
            libc_y,
            libc_n,
            libc_start,
            libc_end,
            libc_end.wrapping_sub(libc_start),
            error as i32
        ),
    );
    pr.ps_acflag.set(pr.ps_acflag.get() | APINSYS);

    // Try to stop threads immediately, because this process is suspect
    if p.has_sibling() {
        let _ = crate::sig::single_thread_set(&p, SINGLE_UNWIND | SINGLE_DEEP);
    }
    // Send uncatchable SIGABRT for coredump
    crate::sig::sigabort(p);
    hal::klock::kernel_unlock(); // KERNEL_UNLOCK()
    Err(error)
}

/// `refreshcreds(p)`: refresh the thread's cache of the process's creds. This is an unlocked
/// access to `ps_ucred`, but the result is benign.
#[inline]
fn refreshcreds<G: ProcGlue>(p: &Thread<G>) {
    if p.cred_stale() {
        crate::prot::dorefreshcreds(p);
    }
}

/// `uvm_map_inentry(p, &p->p_spinentry, PROC_STACK(p), ..., uvm_map_inentry_sp, ...)`: the
/// thread's stack pointer `sp` lies in a `MAP_STACK` mapping of `vm`, its address space; when
/// it does not, `uvm_map_inentry` has signalled the thread (`SIGSEGV`).
fn sp_in_map_stack<G: ProcGlue>(p: &Thread<G>, vm: &Vmspace, sp: usize) -> bool {
    let map = &vm.vm_map;
    let mut ie = p.p_spinentry.get();
    let ok = uvm_map_inentry(
        map,
        &mut ie,
        sp,
        "sp",
        "not MAP_STACK",
        uvm_map_inentry_sp,
        map.sserial(),
    );
    p.p_spinentry.set(ie);
    ok
}

/// The system call prologue's `MAP_STACK` check of `p`, whose `PROC_STACK(p)` is `sp`: a
/// thread without an address space fails it (the glue cannot skip the check by naming none).
fn stack_in_map_stack<G: ProcGlue>(p: &Thread<G>, sp: usize) -> bool {
    match G::user_vmspace(p) {
        Some(vm) => sp_in_map_stack(p, vm, sp),
        None => false,
    }
}

/// `udata_abort`'s fault for `p`, whose `PROC_STACK(p)` is `sp`: the `MAP_STACK` check first
/// (a stack pointer outside one has had its `SIGSEGV`, and the fault is done), then `fault` on
/// `p`'s address space. A fault from user mode on a thread with no address space cannot be:
/// `panic(9)`, as the C's `p->p_vmspace` dereference would, rather than a fault answered
/// `Ok` that the thread takes again for ever.
fn user_page_fault<G: ProcGlue>(
    p: &Thread<G>,
    sp: usize,
    fault: impl FnOnce(&Vmspace) -> Result<(), Errno>,
) -> Result<(), Errno> {
    let Some(vm) = G::user_vmspace(p) else {
        kpanic(format_args!("trap: a user fault with no address space"));
    };
    // SP must be within MAP_STACK space
    if !sp_in_map_stack(p, vm, sp) {
        return Ok(());
    }
    fault(vm)
}

/// `mi_syscall`: the MD setup for a system call has been done; here's the MI part. `lock`
/// is [`syscall_lock`] of the call's row, `body` its function over the arguments and the
/// return registers.
#[inline]
pub fn mi_syscall<G: ProcGlue>(
    p: Pin<&Thread<G>>,
    code: Register,
    lock: bool,
    argp: &SysArgs,
    retval: &mut Rval,
    body: impl FnOnce(&SysArgs, &mut Rval) -> Result<(), Errno>,
) -> Result<(), Errno> {
    // refresh the thread's cache of the process's creds
    refreshcreds(&p);

    // SYSCALL_DEBUG: bsd's feature, through the glue.
    G::scdebug_call(&p, code, argp);
    // dt(4), KTRACE: not configured.

    // PROC_PC and PROC_STACK, in one access to the frame.
    let (pc, sp) = hal::trap::proc_regs();

    // SP must be within MAP_STACK space
    if !stack_in_map_stack(&p, sp) {
        return Err(Errno::EPERM);
    }

    pin_check_at(p, code, pc)?;

    let pledged = p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0;
    if pledged && let Err((error, tval)) = G::pledge_syscall(&p, code as i32) {
        hal::klock::kernel_lock(); // KERNEL_LOCK()
        let error = G::pledge_fail(&p, error, tval);
        hal::klock::kernel_unlock(); // KERNEL_UNLOCK()
        return Err(error);
    }
    if lock {
        hal::klock::kernel_lock(); // KERNEL_LOCK()
    }
    let error = body(argp, retval);
    if lock {
        hal::klock::kernel_unlock(); // KERNEL_UNLOCK()
    }

    error
}

/// `mi_syscall_return`: finish MI stuff on return, after the registers have been set.
#[inline]
pub fn mi_syscall_return<G: ProcGlue>(
    p: Pin<&Thread<G>>,
    code: Register,
    error: Result<(), Errno>,
    retval: &Rval,
) {
    G::scdebug_ret(&p, code, error, retval);
    // dt(4), KTRACE: not configured.
    crate::sig::userret(p);
}

/// `mi_child_return`: finish MI stuff for a new process/thread to return.
#[inline]
pub fn mi_child_return<G: ProcGlue>(p: Pin<&Thread<G>>) {
    // TRACEPOINT(sched, on__cpu, NULL), SYSCALL_DEBUG, dt(4), KTRACE: not configured.
    crate::sig::userret(p);
}

/// `mi_ast`: do the specific processing necessary for an AST.
#[inline]
pub fn mi_ast<G: ProcGlue>(p: Pin<&Thread<G>>, resched: bool) {
    if p.p_flag.load(Ordering::Relaxed) & P_OWEUPC != 0 {
        // KERNEL_LOCK(); ADDUPROF(p); KERNEL_UNLOCK(): the profiling clock (subr_prof.c).
        let _ = crate::prf::unported!("mi_ast: ADDUPROF (subr_prof.c)");
    }
    if resched {
        crate::sched::preempt::<G>();
    }

    // XXX could move call to userret() here, but hppa calls ast() in syscall return and sh
    // calls it after userret()
}

/// The running thread, for `f`; a trap without one is `panic(9)` (the machine checked it).
fn with_trapped<G: ProcGlue, R>(f: impl FnOnce(Pin<&Thread<G>>) -> R) -> R {
    hal::thread::with_current::<Thread<G>, R>(f)
}

impl<G: ProcGlue> TrapHandler for KernelTrap<G> {
    /// `udata_abort`'s and `kdata_abort`'s `pmap_fault_fixup`, `uvm_fault` and `uvm_grow` on
    /// the map the trap chose (`udata_abort`'s `MAP_STACK` check first).
    fn page_fault(
        &self,
        source: FaultSource,
        va: usize,
        access: hal::pmap::VmProt,
    ) -> Result<(), Errno> {
        let kernel_fault = |va| {
            let map = crate::vm::km::kernel_map();
            #[cfg(target_arch = "aarch64")]
            if map
                .pmap()
                .fault_fixup(hal::sys::types::Vaddr::new(va), access)
            {
                return Ok(());
            }
            crate::vm::fault::uvm_fault(map, va, 0, access)
        };
        let user_fault = |vm: &crate::vm::map::Vmspace| {
            let map = &vm.vm_map;
            // Handle referenced/modified emulation
            #[cfg(target_arch = "aarch64")]
            if map
                .pmap()
                .fault_fixup(hal::sys::types::Vaddr::new(va), access)
            {
                return Ok(());
            }
            crate::vm::fault::uvm_fault(map, va, 0, access)?;
            crate::vm::unix::uvm_grow(vm, va);
            Ok(())
        };
        match source {
            FaultSource::Kernel => kernel_fault(va),
            // map = &p->p_vmspace->vm_map; with no thread (or no address space) the kernel
            // map, where the C dereferences p (trap.rs's deviations).
            FaultSource::KernelUser => G::with_curproc(|p| match p.and_then(G::user_vmspace) {
                Some(vm) => user_fault(vm),
                None => kernel_fault(va),
            }),
            FaultSource::User => {
                with_trapped::<G, _>(|p| user_page_fault(&p, hal::trap::proc_stack(), user_fault))
            }
        }
    }

    fn trapsignal(&self, sig: i32, trapno: u64, code: i32, addr: usize) {
        with_trapped::<G, _>(|p| {
            crate::sig::trapsignal(
                p,
                sig,
                trapno,
                code,
                hal::sys::siginfo::Sigval::from_ptr(addr),
            );
        });
    }

    fn sigexit(&self, sig: i32) -> ! {
        with_trapped::<G, _>(|p| crate::sig::sigexit(&p, sig))
    }

    fn refreshcreds(&self) {
        with_trapped::<G, _>(|p| refreshcreds(&p));
    }

    fn userret(&self) {
        with_trapped::<G, _>(crate::sig::userret);
    }

    fn syscall_return(&self, code: Register, error: SysResult, rval: &Rval) {
        let error = error.map_err(|e| Errno::from_raw(e).unwrap_or(Errno::EINVAL));
        with_trapped::<G, _>(|p| mi_syscall_return(p, code, error, rval));
    }

    fn ast(&self, resched: bool) {
        with_trapped::<G, _>(|p| mi_ast(p, resched));
    }

    fn child_return(&self) {
        with_trapped::<G, _>(mi_child_return);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cred::{CredRef, Ucred};
    use crate::proc::{process_alloc, procinit, thread_alloc};
    use crate::testutil;
    use hal::malloc::{M_WAITOK, MArray};
    use hal::pin::pin_static;
    use hal::pool::PoolBox;
    use hal::sys::syscall::{SYS_getpid, SYS_read};

    crate::test_file_glue!(Yf, Yg);
    crate::test_proc_glue!(Yg, Yf);

    /// A pin table of `[start, start + 0x1000)` with `pins` by system call number.
    fn table(start: usize, pins: &[u32]) -> PinTable {
        testutil::install();
        PinTable {
            start,
            end: start + 0x1000,
            pins: MArray::new(crate::vm::mmap::M_PINSYSCALL, M_WAITOK, pins.len(), |i| {
                pins[i]
            }),
        }
    }

    /// `pin_check`'s table, case by case (syscall_mi.h): libc's region first, the program's,
    /// then the signal trampoline's `sigreturn`; the entry 0 or past the table `ENOSYS`, the
    /// exact offset or `(u_int)-1` allowed, any other site `ENOSYS`, a non-`sigreturn` call
    /// from the trampoline `EPERM`.
    #[test]
    fn pin_check_is_the_cs_table() {
        // getpid (20) at libc + 0x40, read (3) anywhere in libc; nothing for 1.
        let mut libc_pins = [0u32; 32];
        libc_pins[SYS_getpid as usize] = 0x40;
        libc_pins[SYS_read as usize] = u32::MAX;
        let libc = table(0x1000_0000, &libc_pins);
        let mut prog_pins = [0u32; 32];
        prog_pins[SYS_getpid as usize] = 0x80;
        let prog = table(0x2000_0000, &prog_pins);
        let none = PinTable::NONE;
        let ret = 0x3000_0000;
        let getpid = SYS_getpid as Register;
        let read = SYS_read as Register;
        let sigreturn = SYS_sigreturn as Register;

        let which = |r: Result<(), (Errno, Option<&PinTable>)>| {
            r.map_err(|(e, t)| {
                (
                    e,
                    t.map(|t| {
                        if core::ptr::eq(t, &libc) {
                            "libc"
                        } else {
                            "prog"
                        }
                    }),
                )
            })
        };
        // libc, the exact site, any site for (u_int)-1, a wrong site, no entry, past it.
        assert_eq!(
            which(pin_verdict(0x1000_0040, 0, ret, getpid, &libc, &prog)),
            Ok(())
        );
        assert_eq!(
            which(pin_verdict(0x1000_0100, 0, ret, read, &libc, &prog)),
            Ok(())
        );
        assert_eq!(
            which(pin_verdict(0x1000_0044, 0, ret, getpid, &libc, &prog)),
            Err((Errno::ENOSYS, Some("libc")))
        );
        assert_eq!(
            which(pin_verdict(0x1000_0040, 0, ret, 1, &libc, &prog)),
            Err((Errno::ENOSYS, Some("libc")))
        );
        assert_eq!(
            which(pin_verdict(0x1000_0040, 0, ret, 32, &libc, &prog)),
            Err((Errno::ENOSYS, Some("libc")))
        );
        assert_eq!(
            which(pin_verdict(0x1000_0040, 0, ret, -1, &libc, &prog)),
            Err((Errno::ENOSYS, Some("libc")))
        );
        // The program's region (a static binary, ld.so).
        assert_eq!(
            which(pin_verdict(0x2000_0080, 0, ret, getpid, &libc, &prog)),
            Ok(())
        );
        assert_eq!(
            which(pin_verdict(0x2000_0040, 0, ret, getpid, &libc, &prog)),
            Err((Errno::ENOSYS, Some("prog")))
        );
        // The signal trampoline: sigreturn alone.
        assert_eq!(
            which(pin_verdict(0x5000_0000, ret, ret, sigreturn, &libc, &prog)),
            Ok(())
        );
        assert_eq!(
            which(pin_verdict(0x5000_0000, ret, ret, getpid, &libc, &prog)),
            Err((Errno::EPERM, None))
        );
        // Anywhere else, and a process with no tables at all.
        assert_eq!(
            which(pin_verdict(0x5000_0000, 0, ret, getpid, &libc, &prog)),
            Err((Errno::ENOSYS, None))
        );
        assert_eq!(
            which(pin_verdict(0x1000_0040, 0, ret, getpid, &none, &none)),
            Err((Errno::ENOSYS, None))
        );
    }

    /// `SY_NOLOCK` runs unlocked unless the call is one of the deferred ones; a number past
    /// the table is locked (`sys_nosys`).
    #[test]
    fn the_lock_follows_the_row_and_the_deferred_table() {
        assert!(syscall_lock(0, SYS_getpid as Register));
        assert!(!syscall_lock(SY_NOLOCK, SYS_getpid as Register));
        for &code in SY_NOLOCK_DEFERRED {
            assert!(syscall_lock(SY_NOLOCK, code as Register), "{code}");
        }
        assert!(syscall_lock(SY_NOLOCK, SYS_MAXSYSCALL as Register));
        assert!(syscall_lock(SY_NOLOCK, -1));
    }

    /// A thread whose glue names no address space fails the `MAP_STACK` check: `EPERM`, the
    /// body never runs (the check comes before `pin_check` and pledge, as in the C).
    #[test]
    fn no_map_stack_is_eperm_and_runs_nothing() {
        testutil::install();
        hal::klock::kernel_lock();
        procinit::<Yg>(64, 64);
        let pr = process_alloc::<Yg>(4242, ());
        drop(pr.set_ucred(CredRef::new(Ucred::new())));
        let t = thread_alloc::<Yg>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        hal::klock::kernel_unlock();
        let t: &'static PoolBox<Thread<Yg>> = std::boxed::Box::leak(std::boxed::Box::new(t));
        let p = pin_static::<Thread<Yg>>(t);
        let mut ran = false;
        let r = mi_syscall(
            p,
            SYS_getpid as Register,
            true,
            &[0; 6],
            &mut [0; 2],
            |_, _| {
                ran = true;
                Ok(())
            },
        );
        assert_eq!(r, Err(Errno::EPERM));
        assert!(!ran, "the body ran past a failed MAP_STACK check");
        core::mem::forget(pr);
    }

    /// The glue of the tests that run the `MAP_STACK` check on a real address space: its
    /// threads' address space is the one the test thread set in [`VG_VM`].
    mod vmglue {
        std::thread_local! {
            pub(super) static VG_VM: core::cell::Cell<Option<&'static crate::vm::map::Vmspace>> =
                const { core::cell::Cell::new(None) };
        }

        crate::test_file_glue!(Vf, Vg);
        crate::test_proc_glue!(Vg, Vf, {
            fn user_vmspace(_p: &crate::proc::Thread<Self>) -> Option<&crate::vm::map::Vmspace> {
                VG_VM.with(core::cell::Cell::get)
            }
        });
    }
    use vmglue::{VG_VM, Vg};

    /// The prologue's `MAP_STACK` check on the thread's own map: a stack pointer in a
    /// `MAP_STACK` mapping passes to the body; one in a plain anonymous mapping is `EPERM`
    /// before the body, with `uvm_map_inentry`'s `SIGSEGV` (syscall_mi.h:158-162).
    #[test]
    fn the_map_stack_check_reads_the_threads_map() {
        use crate::vm::map::{uvm_map_init, uvm_mapanon, uvmspace_alloc};
        use crate::vm::types::{UVM_FLAG_COPYONW, UVM_FLAG_FIXED, UVM_FLAG_STACK, uvm_mapflag};
        use hal::arch::current::param::PAGE_SIZE;
        use hal::arch::current::vmparam::{VM_MAXUSER_ADDRESS, VM_MIN_ADDRESS};
        use hal::sys::mman::{MADV_NORMAL, MAP_INHERIT_COPY, PROT_EXEC, PROT_READ, PROT_WRITE};

        let _pages = testutil::pages_install(1024 * 4096);
        testutil::init_shared_once();
        let _ = crate::vm::anon::uvm_anon_init(0);
        let _ = crate::vm::amap::amap_init(0);
        let _ = crate::vm::aobj::uao_init();
        hal::pmap::init();
        uvm_map_init();
        crate::vm::fault::uvmfault_init();

        let vm = uvmspace_alloc(VM_MIN_ADDRESS, VM_MAXUSER_ADDRESS, true, false);
        let rw = PROT_READ | PROT_WRITE;
        let flags = |extra| {
            uvm_mapflag(
                rw,
                rw | PROT_EXEC,
                MAP_INHERIT_COPY,
                MADV_NORMAL,
                UVM_FLAG_COPYONW | extra,
            )
        };
        let len = 4 * PAGE_SIZE;
        let mut stk = 0x4800_0000;
        uvm_mapanon(
            &vm.vm_map,
            &mut stk,
            len,
            0,
            flags(UVM_FLAG_FIXED | UVM_FLAG_STACK),
        )
        .expect("the stack");
        let mut plain = 0x4900_0000;
        uvm_mapanon(&vm.vm_map, &mut plain, len, 0, flags(UVM_FLAG_FIXED)).expect("plain");
        let vm: &'static crate::vm::map::VmspaceRef =
            std::boxed::Box::leak(std::boxed::Box::new(vm));
        VG_VM.with(|c| c.set(Some(&**vm)));

        hal::klock::kernel_lock();
        procinit::<Vg>(64, 64);
        let pr = process_alloc::<Vg>(4343, ());
        drop(pr.set_ucred(CredRef::new(Ucred::new())));
        let t = thread_alloc::<Vg>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        hal::klock::kernel_unlock();
        // libc's pins: getpid at its + 0x40, where the thread's pc is.
        let mut pins = [0u32; 32];
        pins[SYS_getpid as usize] = 0x40;
        drop(pr.ps_libcpin.replace(table(0x1000_0000, &pins)));
        let pc = 0x1000_0040;
        let t: &'static PoolBox<Thread<Vg>> = std::boxed::Box::leak(std::boxed::Box::new(t));
        let p = pin_static::<Thread<Vg>>(t);
        testutil::vmproc_with(|s| {
            s.segv.clear();
            s.uprintfs.clear();
        });
        let getpid = SYS_getpid as Register;

        hal::trap::set_user_regs(pc, stk + 2 * PAGE_SIZE);
        let mut ran = false;
        let r = mi_syscall(p, getpid, false, &[0; 6], &mut [0; 2], |_, _| {
            ran = true;
            Ok(())
        });
        assert_eq!(r, Ok(()));
        assert!(ran, "a MAP_STACK stack reaches the body");
        assert!(testutil::vmproc_with(|s| s.segv.is_empty()));

        hal::trap::set_user_regs(pc, plain + PAGE_SIZE);
        let mut ran = false;
        let r = mi_syscall(p, getpid, false, &[0; 6], &mut [0; 2], |_, _| {
            ran = true;
            Ok(())
        });
        assert_eq!(r, Err(Errno::EPERM));
        assert!(!ran, "the body ran past a failed MAP_STACK check");
        assert_eq!(testutil::vmproc_with(|s| s.segv.len()), 1, "SIGSEGV");

        hal::trap::set_user_regs(0, 0);
        VG_VM.with(|c| c.set(None));
        core::mem::forget(pr);
    }

    /// A fault from user mode on a thread with no address space cannot be: `panic(9)`, not a
    /// fault answered `Ok` that the thread would take again for ever.
    #[test]
    #[should_panic(expected = "trap: a user fault with no address space")]
    fn a_user_fault_without_an_address_space_panics() {
        testutil::install();
        hal::klock::kernel_lock();
        procinit::<Yg>(64, 64);
        let pr = process_alloc::<Yg>(4244, ());
        let t = thread_alloc::<Yg>(&pr, ());
        hal::klock::kernel_unlock();
        core::mem::forget(pr);
        let _ = user_page_fault(&t, 0x7f00_0000, |_| Ok(()));
    }
}
/* </TESTS> */
