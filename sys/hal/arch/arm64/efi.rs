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
 * Copyright (c) 2017 Mark Kettenis <kettenis@openbsd.org>
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
//! arm64's half of the UEFI runtime call: `efi_enter`, `efi_leave`, `efi_enter_check` and
//! `efi_fault` (`arch/arm64/dev/efi_machdep.c`, `machine/efivar.h`), as one transaction that
//! `hal::cpu::efi::EfiRuntime`'s methods run under their lock.
//!
//! Upstream: sys/arch/arm64/dev/efi_machdep.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/efi_machdep.rs@e8249449d905
//!
//! [`transaction`] masks the interrupts, switches the lower half to the EFI pmap, lets EL1 use
//! the FPU, runs its body with an [`Inside`] (the one value that can read the firmware's memory
//! or call a service), then undoes the three in the C's order. A service is called through a
//! trampoline of this file (`hal_efi_rs_call`): it saves `x19`..`x30` and `sp` in its own
//! checkpoint, then arms `pcb_onfault` with its fault target, so a fault during the call (the
//! trap's `pcb_onfault` recovery, `trap.rs`) comes back here with the registers the checkpoint
//! holds and `faulted` set, which is `EFAULT`.
//!
//! ## Deviations
//! - The firmware's tables are read with `kcopy` (its own `pcb_onfault`), so a fault while
//!   reading the vendor string or the configuration table is `EFAULT` for that read; the C
//!   reads them with plain loads under `efi_fault`, which would `longjmp` to whatever
//!   `efi_jmpbuf` held.
//! - Every address read and every service called lies below the EFI pmap's limit
//!   (`hal::cpu::efi`'s bound): an address of the kernel's half is `EFAULT`.
//!
//! ## Redesign
//! - LZ's `efi_enter(sc)`/`efi_leave(sc)` (`bsd`'s, with `sc_psw` in the softc, `pcb_onfault`
//!   reached through `curcpu()` with an `unsafe` each, `ci_curpm` dereferenced raw, the pmap
//!   switch behind `bsd`'s `Switcher` token) and `efi_call(sc, slot, a0, a1)` (any slot, any two
//!   integers) -> [`transaction`] and [`Inside`]'s two methods: the interrupt state is the
//!   transaction's `IntrGuard`, `pcb_onfault` the running thread's pcb (`with_curpcb`), the
//!   pmap the CPU ran on `asid.rs`'s record (`efi_leave_current`), and the only pointer the
//!   firmware is handed is an `EfiTime` the caller lends.
//! - LZ's `efi_jmpbuf` (an `UnsafeCell<Label>` static behind an `unsafe impl Sync`, written by
//!   the trampoline and read by `efi_fault`) -> 104 bytes of the trampoline's own `.bss`, which
//!   no Rust code names; `hal::cpu::efi`'s lock makes one transaction at a time.
//! - The trampoline takes the service's address, not its slot: the slot is read first, in the
//!   same transaction, through `kcopy` (`hal::cpu::efi`), so a fault there is `EFAULT` too.
//! - LZ's order corrected: LZ armed `pcb_onfault` in `efi_enter`, before its trampoline saved
//!   the registers, so a fault between the two would have resumed with a stale `efi_jmpbuf`.
//!   The C checkpoints first (`efivar.h`'s `efi_enter_check`: the `setjmp`, then `efi_enter`,
//!   which arms the fault target, then the call), and here the trampoline writes the
//!   checkpoint before it arms its fault target, both in one routine.
//! - What the EFI pmap maps is `hal::cpu::efi::EfiRuntime::install`'s alone (the C's policy
//!   over the runtime descriptors of the boot record of the firmware's memory map, refusing
//!   RAM the page system was loaded with, managed pages and the kernel image, once): the
//!   SAFETY arguments below rest on it, and on the firmware's memory map, trusted as the C
//!   trusts it.

use core::arch::global_asm;
use core::marker::PhantomData;

use sysdefs::base::errno::Errno;
use sysdefs::dev::efi::{EfiStatus, EfiTime};

use super::copy::kcopy_in;
use super::fpu;
use super::pmap::backend::{efi_enter as efi_enter_pmap, efi_leave_current};
use super::switch::with_curpcb;
use crate::cpu::efi::{EfiLocked, below};
use crate::cpu::{IntrGuard, intr_disable};
use crate::vm::pmap::Pmap;

/// `PMAP_DEVICE` (`machine/pmap.h`) as the physical address's flag bit: device memory.
pub(crate) const PMAP_DEVICE: usize = super::pmap::layout::PMAP_DEVICE as usize;

/// What the trampoline returns: the service's status, and whether its fault target cut it
/// short.
#[repr(C)]
struct EfiCallResult {
    /// The service's `EFI_STATUS` (0 when it faulted).
    status: u64,
    /// 1 when the call faulted.
    faulted: u64,
}

/// What a transaction may do: read the firmware's memory and call a service, under the EFI
/// pmap with interrupts masked. Made only by [`transaction`], lent to its body.
pub(crate) struct Inside {
    /// The EFI pmap's limit, `1 << EFI_SPACE_BITS`.
    limit: u64,
    /// Bound to the CPU the transaction runs on.
    _here: PhantomData<*const ()>,
}

// hal_efi_rs_call(fn, time, a1, onfault): saves x19-x30 and sp in hal_efi_jmpbuf, then (when
// `onfault`, the running pcb's `pcb_onfault`, is not null) stores the fault target there, and
// calls `fn(time, a1)`; returns { status, 0 }. The fault target, reached through the trap's
// `pcb_onfault` recovery, restores the saved registers and returns { 0, 1 } from
// hal_efi_rs_call.
global_asm!(
    ".pushsection .bss.hal_efi_jmpbuf, \"aw\", %nobits",
    ".balign 16",
    ".Lhal_efi_jmpbuf:",
    ".zero 104",
    ".popsection",
    ".text",
    ".balign 4",
    ".global hal_efi_rs_call",
    ".type hal_efi_rs_call, %function",
    "hal_efi_rs_call:",
    "    adrp x9, .Lhal_efi_jmpbuf",
    "    add x9, x9, :lo12:.Lhal_efi_jmpbuf",
    "    mov x10, sp",
    "    stp x19, x20, [x9, #0]",
    "    stp x21, x22, [x9, #16]",
    "    stp x23, x24, [x9, #32]",
    "    stp x25, x26, [x9, #48]",
    "    stp x27, x28, [x9, #64]",
    "    stp x29, x30, [x9, #80]",
    "    str x10, [x9, #96]",
    "    stp x29, x30, [sp, #-16]!",
    "    mov x29, sp",
    "    cbz x3, 1f",
    "    adr x10, .Lhal_efi_fault",
    "    str x10, [x3]",
    "1:",
    "    mov x16, x0",
    "    mov x0, x1",
    "    mov x1, x2",
    "    blr x16",
    "    mov x1, #0",
    "    ldp x29, x30, [sp], #16",
    "    ret",
    ".Lhal_efi_fault:",
    "    adrp x9, .Lhal_efi_jmpbuf",
    "    add x9, x9, :lo12:.Lhal_efi_jmpbuf",
    "    ldp x19, x20, [x9, #0]",
    "    ldp x21, x22, [x9, #16]",
    "    ldp x23, x24, [x9, #32]",
    "    ldp x25, x26, [x9, #48]",
    "    ldp x27, x28, [x9, #64]",
    "    ldp x29, x30, [x9, #80]",
    "    ldr x10, [x9, #96]",
    "    mov sp, x10",
    "    mov x0, #0",
    "    mov x1, #1",
    "    ret",
    ".size hal_efi_rs_call, . - hal_efi_rs_call",
);

// SAFETY: the item names this file's routine above, `hal_efi_rs_call`, by its symbol, with the
// AAPCS64 prototype it keeps: the four arguments in x0-x3 (`fn_va`, `time`, `a1`, `onfault`),
// the two-word result in x0/x1 (`EfiCallResult`, 16 bytes of two integers). On both of its
// paths out (the service's return, and the fault target reached through the trap's
// `pcb_onfault` recovery) it hands back x19-x30 and sp as it found them: the return path does
// not touch them but the frame it pops, the fault path reloads them from the checkpoint it
// wrote on entry. d8-d15, callee-saved in the C convention, are not saved: the kernel is
// softfloat and keeps nothing in the FP/SIMD registers across a call, and
// `fpu::kernel_enter` saved the running thread's FP state before the transaction lets the
// firmware use them. The caller-saved registers are dead across the call, as the declaration
// says (an `extern "C"` call). The call site states what its arguments must be.
unsafe extern "C" {
    /// The trampoline above: `fn_va(time, a1)` with the checkpoint taken and `onfault` armed.
    fn hal_efi_rs_call(
        fn_va: u64,
        time: *mut EfiTime,
        a1: u64,
        onfault: *mut usize,
    ) -> EfiCallResult;
}

impl Inside {
    /// `kcopy` of `buf.len()` bytes from the firmware's address `va`: `EFAULT` when a page is
    /// not mapped by the EFI pmap, or the range does not lie wholly below its limit.
    pub(crate) fn read(&self, va: u64, buf: &mut [u8]) -> Result<(), Errno> {
        if !below(va, buf.len(), self.limit) {
            return Err(Errno::EFAULT);
        }
        // SAFETY: `[va, va + len)` lies below the EFI pmap's limit (just checked), so in the
        // lower half, which `TTBR0_EL1` translates through the EFI pmap for as long as `self`
        // lives (`transaction` loaded it, with interrupts masked, before making `self`, and
        // puts the CPU's pmap back only after its body returns; `self` cannot outlive the
        // body). What the EFI pmap maps there is the firmware's runtime memory, which no Rust
        // object of the kernel lives in: `EfiRuntime::install` is the only code that enters
        // pages into it, once (its first call closes the door), from the runtime descriptors of
        // the boot record of the firmware's memory map (trusted as the C trusts that map), and
        // it refuses every frame of the RAM the page system was loaded with (stolen frames
        // included), every managed page and the kernel image: memory foreign to the kernel as a user pmap's pages
        // are to `copyin`, read here as bytes. A page it does not map faults inside `kcopy`,
        // which recovers through its own `pcb_onfault` into `EFAULT`; `buf` is the caller's,
        // borrowed exclusively for the copy.
        unsafe { kcopy_in(va as usize, buf) }
    }

    /// The service at `fn_va` called as `fn_va(time, NULL)` (`GetTime(&time, NULL)`,
    /// `SetTime(&time)`, whose second argument the firmware ignores): its status, or `EFAULT`
    /// when the address is not below the EFI pmap's limit or the call faulted.
    pub(crate) fn time_call(&self, fn_va: u64, time: &mut EfiTime) -> Result<EfiStatus, Errno> {
        if !below(fn_va, 4, self.limit) {
            return Err(Errno::EFAULT);
        }
        let r = with_curpcb(|pcb| {
            let onfault = pcb.map_or(core::ptr::null_mut(), |p| p.pcb_onfault.as_ptr());
            // SAFETY: interrupts are masked, the EFI pmap is in the lower half and the FPU is
            // usable for as long as `self` lives (`transaction`), and `hal::cpu::efi`'s lock is
            // held across it (`EfiLocked`), so this is the only call the trampoline runs on any
            // CPU: its checkpoint is this call's alone. `fn_va` lies below the EFI pmap's
            // limit (just checked), so the code it names is what the EFI pmap maps executable
            // there: only `EfiRuntime::install` enters pages into that pmap, once (its first call
            // closes the door), executable only for `EfiRuntimeServicesCode` descriptors of the
            // boot record of the firmware's memory map and never a frame of the RAM the page
            // system was loaded with, a managed page or the kernel image, so it is the
            // firmware's runtime code (the map trusted as the C trusts it), never the kernel's. The
            // trampoline saves the callee-saved registers and `sp` before it arms `onfault`
            // (the running thread's own `pcb_onfault`, alive while it runs, or null for none),
            // so a fault anywhere in the call resumes at its fault target with this frame's
            // registers and returns `faulted`. Such a fault's `do_el1h_sync` unmasks the CPU's
            // interrupts (`PSTATE.I`, `trap.rs`) while it handles the abort: the runtime's lock,
            // held at `IPL_HIGH`, keeps every device interrupt out meanwhile, so nothing else
            // runs on this CPU before the fault target restores the frame (the `eret` puts the
            // masked state back). The caller-saved registers the C convention lets a call
            // clobber are what the declaration says. The one pointer handed over is `time`,
            // an `EfiTime` the caller borrows exclusively for the call, of the size the
            // service reads or writes (16 bytes, `EFI_TIME`); the second argument is NULL.
            let r = unsafe { hal_efi_rs_call(fn_va, core::ptr::from_mut(time), 0, onfault) };
            if let Some(p) = pcb {
                p.pcb_onfault.set(0);
            }
            r
        });
        if r.faulted != 0 {
            return Err(Errno::EFAULT);
        }
        Ok(r.status)
    }
}

/// `efi_enter`: interrupts masked, the EFI pmap `pm` in the lower half, the FPU usable.
fn efi_enter(off: &IntrGuard, pm: &Pmap) {
    efi_enter_pmap(off, pm);
    fpu::kernel_enter();
}

/// `efi_leave`: the FPU trapped again and the CPU's own pmap back; the interrupt state is
/// restored when the guard drops.
fn efi_leave(off: &IntrGuard) {
    fpu::kernel_exit();
    efi_leave_current(off);
}

/// `efi_enter`, `body` with what a transaction may do, then `efi_leave`, on every path out of
/// `body` (it returns a value; nothing in it unwinds, the kernel aborts on a panic).
pub(crate) fn transaction<R>(
    _held: &EfiLocked<'_>,
    pm: &Pmap,
    body: impl FnOnce(&Inside) -> R,
) -> R {
    let off = intr_disable();
    efi_enter(&off, pm);
    let r = body(&Inside {
        limit: pm.va_limit().as_usize() as u64,
        _here: PhantomData,
    });
    efi_leave(&off);
    drop(off);
    r
}
/* </CODE> */
