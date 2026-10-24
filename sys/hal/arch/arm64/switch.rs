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

/*
 * Copyright (c) 2014 Patrick Wildt <patrick@blueri.se>
 * Copyright (c) 2021 Mark Kettenis <kettenis@openbsd.org>
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
//! arm64's half of `hal::thread`: the context switch of `cpuswitch.S` (pulled in from the
//! `.S` file next to this module, whose `{NAME}` placeholders are what `assym.h` provides in
//! C), `cpu_switchto`'s FPU work (`machdep.c`), `cpu_fork`'s frames (`vm_machdep.c`), and the
//! running thread's pcb.
//!
//! Upstream: sys/arch/arm64/arm64/cpuswitch.S @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/vm_machdep.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/machdep.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/cpuswitch.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/vm_machdep.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/machdep.rs@3c62ede68802
//!
//! **What the assembly reaches.** `ci_curproc` and `ci_curpcb` at the offsets of the
//! `CpuCore` that `TPIDR_EL1` names (`hal::cpu`); a thread's `p_stat`, `p_cpu` and `p_addr`
//! in its [`MachineSlot`], through the slot's own address (never the thread's plus an
//! offset); its pcb at `p_addr` (`struct user`'s first member, `u_pcb`, as `bsd`'s
//! `sys/sys/user.rs` lays it out); the pcb's `pcb_sp` and `pcb_tcb`.
//!
//! **Two symbols of `bsd`'s** the assembly calls, as the C's `cpuswitch.S` and `locore.S`
//! do, until their owners move (N4.8, the trap entry; the pmap's switch record): `pmap_setttb`
//! (`sys/arch/arm64/arm64/cpuswitch.rs`, the new thread's address space, called with the
//! thread `ci_curproc` now names) and `syscall_return` (`exception.S`, a new user thread's way
//! out). The ABI module's copy of `hal` (`module-rt`) does not compile this module.
//!
//! ## Deviations
//! - No retguard: the cookie load/calc/check around the stack pivot has no counterpart in
//!   this kernel yet (LZ's).
//! - The switch frame's `sf_x19` holds the thread's start word, which the trampoline hands
//!   to the start registered with `hal::thread` (`register_start`), instead of a function
//!   pointer the trampoline `blr`s: no address crosses as code.
//!
//! ## Redesign
//! - LZ's `cpu_switchto_asm(oldproc, newproc)` read the thread's slot members at
//!   `offset_of!(Proc, machine.*)` and wrote `ci_curproc` from the same register; here the
//!   assembly takes the two slots and the new thread apart (`hal_cpu_switchto(oldslot,
//!   newslot, newproc)`), so `hal` names no thread type and assumes no offset of the slot in
//!   it (the kernel keeps it first for its own readers of `ci_curproc`: `kernel::proc`'s tests
//!   check the offset 0 of its generic thread, `bsd` asserts it of its own at compile time).
//! - LZ's `machdep.rs` `cpu_switchto` (the FPU state saved and dropped for the old thread,
//!   then the assembly) is [`switch`]; `vm_machdep.rs` `cpu_fork`'s frames are
//!   [`thread_setup`], and `cpu_init_secondary`'s rebuild of the idle thread's switch frame
//!   is [`thread_restart`]; `proc_trampoline_run` is [`trampoline_run`], reached from the
//!   assembly by a `sym` operand (no unmangled symbol).

use core::arch::global_asm;
use core::mem::offset_of;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicU8, Ordering};

use crate::arch::arm64::fpu;
use crate::arch::arm64::include::frame::{SWITCHFRAME_SZ, Switchframe, Trapframe};
use crate::arch::arm64::include::param::PAGE_SIZE;
use crate::arch::arm64::include::pcb::{PCB_FPU, Pcb};
use crate::arch::arm64::include::tcb::__aarch64_set_tcb;
use crate::assert::hal_panic;
use crate::cpu::{CORE_CURPCB, CORE_CURPROC};
use crate::sys::types::Register;
use crate::thread::{MachineSlot, SONPROC};

/// `UPAGES`: the u-area's pages (arm64 `<machine/param.h>`; `bsd`'s `param.rs` asserts it).
pub const UPAGES: usize = 6;
/// `USPACE`: the u-area's size, the `struct user` at its bottom and the kernel stack above.
pub const USPACE: usize = UPAGES * PAGE_SIZE;

/// `STACKALIGN(p)`: a stack pointer rounded down to 16 bytes.
const fn stackalign(p: usize) -> usize {
    p & !(16 - 1)
}

/// `proc0paddr`: `proc0`'s u-area, `USPACE` zero bytes, page aligned (`locore`'s reservation
/// in the C), `hal`'s own so that its address is never a caller's (`hal::thread`'s E4). Bytes
/// as atomics: the pcb at its bottom and the stack above it are written through the address
/// (by `proc0` itself and the switch), which a shared reference to cells permits.
#[repr(C, align(4096))]
struct BootUarea([AtomicU8; USPACE]);

/// `proc0`'s u-area.
static PROC0_UAREA: BootUarea = BootUarea([const { AtomicU8::new(0) }; USPACE]);

/// The address of `proc0`'s u-area (`hal::thread::boot_uarea`).
pub(crate) fn boot_uarea_word() -> usize {
    ptr::from_ref(&PROC0_UAREA).expose_provenance()
}

/// `curpcb = &proc0paddr->u_pcb` on this CPU (`hal::thread::boot_uarea`): the pcb at the
/// bottom of `proc0`'s u-area, zero bytes as `Pcb::new()` makes it.
pub(crate) fn set_boot_pcb() {
    crate::cpu::curcpu().set_curpcb(ptr::with_exposed_provenance::<()>(boot_uarea_word()));
}

global_asm!(
    include_str!("cpuswitch.S"),
    SWITCHFRAME_SZ = const SWITCHFRAME_SZ,
    SF_X19 = const offset_of!(Switchframe, sf_x19),
    SF_X21 = const offset_of!(Switchframe, sf_x21),
    SF_X23 = const offset_of!(Switchframe, sf_x23),
    SF_X25 = const offset_of!(Switchframe, sf_x25),
    SF_X27 = const offset_of!(Switchframe, sf_x27),
    SF_X29 = const offset_of!(Switchframe, sf_x29),
    CI_CURPCB = const CORE_CURPCB,
    CI_CURPROC = const CORE_CURPROC,
    P_STAT = const offset_of!(MachineSlot, stat),
    P_CPU = const offset_of!(MachineSlot, cpu),
    P_ADDR = const offset_of!(MachineSlot, addr),
    PCB_SP = const offset_of!(Pcb, pcb_sp),
    PCB_TCB = const offset_of!(Pcb, pcb_tcb),
    SONPROC = const SONPROC,
    TRAMPOLINE_RUN = sym trampoline_run,
);

unsafe extern "C" {
    /// The switch (`cpuswitch.S`): saves the old context (when `oldslot` is not null) on its
    /// stack and its pcb, makes `newproc` `ci_curproc` with its slot `SONPROC` on this CPU,
    /// loads its pcb, TCB, address space and stack, and returns on its stack. See [`switch`].
    fn hal_cpu_switchto(oldslot: *const (), newslot: *const (), newproc: *const ());
    /// Where a thread [`thread_setup`] built starts (`cpuswitch.S`); its address only.
    fn hal_proc_trampoline();
}

/// The u-area of the thread whose slot is `slot` and the pcb at its bottom
/// (`p->p_addr->u_pcb`), for `f`; `None` when the slot has no u-area (`p_addr` 0).
fn with_pcb_of<R>(slot: &MachineSlot, f: impl FnOnce(Option<&Pcb>) -> R) -> R {
    let addr = slot.addr.load(Ordering::Relaxed);
    let pcb = ptr::with_exposed_provenance::<Pcb>(addr);
    // SAFETY: `p_addr` is 0 or the exposed address of the u-area the slot owns (E4: a
    // `UArea` from the registered provider, whose contract gives `USPACE` bytes of zeroed
    // kernel memory nothing else references, or `hal`'s own `PROC0_UAREA`; no safe function
    // stores another address there), whose bottom is a `struct user` with the pcb first (zero
    // bytes are `Pcb::new()`). It is given back only by `take_uarea`, under the scheduler's
    // lock and for a thread with no live context (E6: none running, none suspended in a
    // switch); the callers here hold the lock or run the thread (`thread_setup`,
    // `thread_restart`, the switch's old thread), so the u-area is alive for `f`. A pcb is
    // only ever shared (`&Pcb`, its members cells written by the thread, its switch and
    // `cpu_fork` before it runs), so this reference aliases no `&mut`; it ends with `f`.
    let pcb = unsafe { pcb.as_ref() };
    f(pcb)
}

/// The running thread's pcb (`ci_curpcb`), for `f`; `None` before the first thread.
pub(crate) fn with_curpcb<R>(f: impl FnOnce(Option<&Pcb>) -> R) -> R {
    let pcb = crate::cpu::curcpu().curpcb().cast::<Pcb>();
    // SAFETY: `ci_curpcb` is null or the running thread's pcb (E5): the switch stores the new
    // thread's `p_addr` there (its u-area's `u_pcb`, E4), `hal::thread::boot_uarea` `proc0`'s
    // (`hal`'s static), `clear_current_pcb` null; it is written by this CPU alone. The running
    // thread is not freed while it runs (E2), nor is its u-area given back (`take_uarea`
    // refuses a running thread, E4), so the pcb is alive for `f`; pcbs are only shared, as
    // above.
    let pcb = unsafe { pcb.as_ref() };
    f(pcb)
}

/// `TCB_GET(p)` of the thread whose slot is `slot`: its pcb's `pcb_tcb` (0 without a u-area),
/// for a caller that runs the thread or holds the scheduler's lock (`with_pcb_of`'s premise;
/// `hal::thread::tcb_get`).
pub(crate) fn tcb_of(slot: &MachineSlot) -> usize {
    with_pcb_of(slot, |pcb| pcb.map_or(0, |pcb| pcb.pcb_tcb.get().addr()))
}

/// `TCB_SET(p, addr)` for the running thread, whose slot is `slot`, under the scheduler's lock
/// (`hal::thread::tcb_set`): its pcb's `pcb_tcb`, which the switch loads into `TPIDR_EL0`
/// whenever it runs again, and `TPIDR_EL0` now.
pub(crate) fn set_tcb(slot: &MachineSlot, tcb: usize) {
    with_pcb_of(slot, |pcb| {
        if let Some(pcb) = pcb {
            pcb.pcb_tcb.set(ptr::with_exposed_provenance_mut(tcb));
        }
    });
    __aarch64_set_tcb(tcb);
}

/// `TCB_INVALID(addr)`: none on arm64.
pub(crate) const fn tcb_invalid(addr: usize) -> bool {
    crate::arch::arm64::include::tcb::tcb_invalid(addr)
}

/// `ci->ci_curpcb = NULL` on this CPU: a secondary CPU's start, before its first thread (E5:
/// the other writers are the switch and `hal::thread::boot_uarea`). Refused (`panic(9)`)
/// while the CPU runs a thread (`curproc` not null): a running thread's pcb is the one its
/// copy faults and the switch save into.
pub fn clear_current_pcb() {
    let me = crate::cpu::curcpu();
    if !me.curproc().is_null() {
        hal_panic(format_args!("curpcb: this CPU runs a thread"));
    }
    me.set_curpcb(ptr::null());
}

/// `cpu_switchto(old, new)`: the old thread's FP state saved and the FPU dropped (when there
/// is an old thread), then the assembly. Returns on the old thread's stack once a switch
/// back to it is made. The caller (`hal::thread::switch_to`, `exit_to_idle`) holds the
/// scheduler's lock and checked the token `new_slot`/`new_thread` come from (E1, E3).
pub(crate) fn switch(
    old: Option<&MachineSlot>,
    new_slot: NonNull<MachineSlot>,
    new_thread: NonNull<()>,
) {
    if old.is_some() {
        with_curpcb(|pcb| {
            if let Some(pcb) = pcb
                && pcb.pcb_flags.get() & PCB_FPU != 0
            {
                fpu::save(pcb);
            }
        });
        fpu::drop_state();
    }
    let old = old.map_or(ptr::null(), |s| ptr::from_ref(s).cast::<()>());
    // SAFETY: the assembly (`cpuswitch.S`) saves the callee-saved registers and the link
    // register on the running stack and that stack in the running thread's pcb (when `old` is
    // given: `ci_curpcb` is then the old thread's, the running one), stores into the new
    // slot (`SONPROC`, this CPU) and into this CPU's core (`ci_curpcb`, `ci_curproc`), loads
    // the new pcb's TCB into `TPIDR_EL0`, calls `pmap_setttb` with the new thread, and
    // resumes on the stack the new pcb saved, restoring its registers. For the compiler it is
    // an opaque `extern "C"` call that preserves the callee-saved registers: when a later
    // switch back to the old thread resumes here, the registers and the stack are the ones
    // saved here. What it reaches is alive and touched by nothing else meanwhile:
    // - the new thread and its slot come from a `Next` made under the scheduler's lock, which
    //   this CPU holds (E3, the token checked by the caller); a thread on a queue, or chosen
    //   off one, is not freed (E3), nor does another CPU switch to it while this one holds the
    //   lock; its `p_addr` names its u-area, whose pcb's `pcb_sp` is the switch frame its last
    //   switch away saved, or the one `thread_setup`/`thread_restart` built (a new thread's);
    // - the old thread is the running one (its pcb is `ci_curpcb`): no other CPU runs it or
    //   switches to it before this CPU releases the lock, which happens on the new thread
    //   after this switch has saved the old frame (E1);
    // - `ci_curproc`/`ci_curpcb` are this CPU's core's atomics, and the slot's members
    //   atomics, so the plain stores alias no Rust reference's assumptions.
    unsafe { hal_cpu_switchto(old, new_slot.as_ptr().cast(), new_thread.as_ptr()) };
}

/// `cpu_fork(p1, p2, stack, tcb, func, arg)`'s machine work: the parent's FP state saved
/// (when the parent runs here and used the FPU), its pcb copied to the child's, the child's
/// trap frame a copy of the parent's at the top of its u-area (its user stack pointer
/// `stack` and TCB `tcb` when not 0), and below it a switch frame that returns into the
/// trampoline with `start` and `arg`.
pub(crate) fn thread_setup(
    parent: &MachineSlot,
    parent_runs: bool,
    child: &MachineSlot,
    stack: usize,
    tcb: usize,
    start: usize,
    arg: usize,
) {
    with_pcb_of(parent, |pcb1| {
        let Some(pcb1) = pcb1 else {
            hal_panic(format_args!("cpu_fork: the parent has no u-area"));
        };
        // Save FPU state to PCB if necessary (only the running thread's is in the CPU).
        if parent_runs && pcb1.pcb_flags.get() & PCB_FPU != 0 {
            fpu::save(pcb1);
        }
        with_pcb_of(child, |pcb| {
            let Some(pcb) = pcb else {
                hal_panic(format_args!("cpu_fork: the child has no u-area"));
            };
            // Copy the pcb.
            pcb.copy_from(pcb1);
            let parent_tf = pcb1.pcb_tf.get();
            build_frames(child, pcb, Some((parent_tf, stack)), tcb, start, arg);
        });
    });
}

/// `cpu_init_secondary`'s rebuild of the idle thread's switch frame: a fresh trap frame slot
/// at the top of `t`'s u-area (nothing copied into it) and below it a switch frame into the
/// trampoline with `start` and `arg`, for a thread that is not running anywhere.
pub(crate) fn thread_restart(t: &MachineSlot, start: usize, arg: usize) {
    with_pcb_of(t, |pcb| {
        let Some(pcb) = pcb else {
            hal_panic(format_args!(
                "cpu_init_secondary: the idle thread has no u-area"
            ));
        };
        build_frames(t, pcb, None, 0, start, arg);
    });
}

/// The frames of a thread about to start: its trap frame's place at the top of the u-area
/// (`pcb_tf`), filled from `copy`'s trap frame with the user stack pointer when given, the
/// TCB when not 0, and the switch frame below it (`pcb_sp`).
fn build_frames(
    slot: &MachineSlot,
    pcb: &Pcb,
    copy: Option<(*mut Trapframe, usize)>,
    tcb: usize,
    start: usize,
    arg: usize,
) {
    let uarea = slot.addr.load(Ordering::Relaxed);
    let tf_addr = stackalign(uarea + USPACE - size_of::<Trapframe>() - 0x10);
    let tf = ptr::with_exposed_provenance_mut::<Trapframe>(tf_addr);
    let sf = tf.cast::<Switchframe>().wrapping_sub(1);
    pcb.pcb_tf.set(tf);
    let trampoline = hal_proc_trampoline as *const () as usize;
    // SAFETY: `tf` and `sf` lie inside the thread's u-area (`p_addr`, `USPACE` bytes, the
    // `struct user` at its bottom far below: the trap frame is at its top, the switch frame
    // right under it), which is alive (as in `with_pcb_of`) and which nothing runs on: the
    // thread is not running anywhere (a child before it is runnable, an idle thread before
    // its CPU starts it), so no reference into those bytes exists; both are aligned (16 bytes,
    // `stackalign`, and `Switchframe` is twelve words). The copied trap frame is the parent's
    // `pcb_tf`, which names its trap frame (the one of the system call that forks, on its own
    // kernel stack while it runs `fork1`, or `proc0`'s static one) and is only read here.
    unsafe {
        match copy {
            Some((from, stack)) => {
                let mut frame = from.read();
                if stack != 0 {
                    frame.tf_sp = stackalign(stack) as Register;
                }
                tf.write(frame);
            }
            None => tf.write(Trapframe::new()),
        }
        sf.write(Switchframe {
            sf_x19: start as Register,
            sf_x20: arg as Register,
            sf_lr: trampoline as Register,
            ..Switchframe::default()
        });
    }
    if tcb != 0 {
        pcb.pcb_tcb.set(ptr::with_exposed_provenance_mut(tcb));
    }
    pcb.pcb_sp.set(sf.addr() as u64);
}

/// What `hal_proc_trampoline` calls with the switch frame's `sf_x19` and `sf_x20`: the
/// machine-independent start (`hal::thread`'s trampoline, then the registered start). When
/// it returns the assembly takes `syscall_return` to user mode.
extern "C" fn trampoline_run(start: usize, arg: usize) {
    crate::thread::trampoline(start, arg);
}

/* </CODE> */
