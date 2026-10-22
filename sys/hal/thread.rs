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
//! The part of a thread the machine reaches by offset: `struct proc`'s `p_stat`, `p_cpu`,
//! `p_addr` and `p_md`, which both machines' context switch, syscall entry and exception
//! return read and write from assembly (`genassym.cf`'s `P_STAT`, `P_CPU`, `P_ADDR`,
//! `P_MD_ASTPENDING`, and amd64's `P_MD_REGS`, `P_MD_FLAGS`).
//!
//! Not an OpenBSD file. The kernel's thread (`kernel::proc::Thread`) is `#[repr(C)]` with a
//! [`MachineSlot`] as its first field, so a thread's address is its slot's: `ci_curproc`
//! keeps holding the thread, as `CPUVAR(CURPROC)` and `CI_CURPROC` do, and the assembly's
//! offsets are `offset_of!(MachineSlot, field)`, the same whether the code naming them is
//! `bsd`'s today or `hal`'s at N4.5, which cannot name the kernel's type.
//!
//! ## Redesign
//! - The four members are atomics: the switch writes `p_stat` (`SONPROC`) and `p_cpu` while
//!   `mi_switch` holds `sched_lock` but cannot present a witness from assembly, `signotify`
//!   reads `p_cpu` from another CPU without the lock (`cpu_unidle(p->p_cpu)`), and `sysctl`
//!   and ddb read `p_stat` unlocked. `cpu` holds the `cpu_info` word the switch stores
//!   (`hal::cpu::CpuId::curcpu_word`); `addr` the u-area's address, 0 before `cpu_fork` (an
//!   `Option` of a word would put a discriminant where the assembly reads the address).
//! - N4.2 writes the struct and its constructor only; N4.5 moves the switch, the trampoline
//!   and `cpu_fork` here (`hal::thread`). amd64's `Mdproc` is N10's (decision 44): this file
//!   names arm64's and the host's.

use core::sync::atomic::{AtomicU8, AtomicUsize};

/// `struct mdproc`: the machine's per-thread state (arm64: `md_astpending`).
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub use crate::arch::arm64::include::proc::Mdproc;

/// `struct mdproc` of the host tests: nothing of a machine's.
#[cfg(not(target_os = "none"))]
#[derive(Debug, Default)]
pub struct Mdproc;

#[cfg(not(target_os = "none"))]
impl Mdproc {
    /// All zero.
    pub const fn new() -> Self {
        Self
    }
}

/// The members of a thread the machine reaches by offset, first in the thread
/// (`#[repr(C)]`), so their offsets are the same from the thread and from the slot.
#[repr(C)]
#[derive(Default)]
pub struct MachineSlot {
    /// `p_stat`: the thread's `S*` state (`SONPROC` stored by the switch).
    pub stat: AtomicU8,
    /// `p_cpu`: the `cpu_info` word of the CPU the thread runs or last ran on, 0 before it
    /// first runs.
    pub cpu: AtomicUsize,
    /// `p_addr`: the address of the thread's u-area (`struct user`, whose `u_pcb` is first),
    /// 0 before `cpu_fork`.
    pub addr: AtomicUsize,
    /// `p_md`.
    pub md: Mdproc,
}

impl MachineSlot {
    /// A slot of a thread being made (`SIDL` is set by `fork1`): all zero, as a `static
    /// struct proc` holds.
    pub const fn new() -> Self {
        Self {
            stat: AtomicU8::new(0),
            cpu: AtomicUsize::new(0),
            addr: AtomicUsize::new(0),
            md: Mdproc::new(),
        }
    }
}

const _: () = {
    assert!(core::mem::offset_of!(MachineSlot, stat) == 0);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::mem::offset_of;
    use core::sync::atomic::Ordering;

    use super::*;

    #[repr(C)]
    struct Outer {
        machine: MachineSlot,
        tail: u64,
    }

    #[test]
    fn the_slot_is_at_the_start_of_what_embeds_it_first() {
        assert_eq!(offset_of!(Outer, machine), 0);
        assert_eq!(
            offset_of!(Outer, machine) + offset_of!(MachineSlot, cpu),
            offset_of!(MachineSlot, cpu)
        );
        let o = Outer {
            machine: MachineSlot::new(),
            tail: 1,
        };
        assert_eq!(o.machine.stat.load(Ordering::Relaxed), 0);
        assert_eq!(o.machine.addr.load(Ordering::Relaxed), 0);
        assert_eq!(o.tail, 1);
    }
}
/* </TESTS> */
