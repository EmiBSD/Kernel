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
//! The host's half of `hal::thread`'s switch: a double over host threads, so the
//! scheduler's tests switch for real (N4.5a).
//!
//! Every kernel thread that runs is a host thread: the test's own for the thread it installs
//! as the current one, a host thread spawned by [`thread_setup`] (or [`thread_restart`]) for
//! a thread made to start. A "CPU" is a `hal::cpu::CpuCore` a host thread installs on itself
//! (`hal::cpu::set_current`). A switch does what the machine's assembly does to the slot and
//! the core (`SONPROC`, `p_cpu`, `ci_curproc`), hands the running core to the new thread's
//! host thread through that thread's gate, and parks the old thread's host thread on its own
//! gate until a later switch hands it a core, which it installs before it returns: so the
//! scheduler's lock, owned by the core, passes from one host thread to the other as it passes
//! from one stack to the other on a machine. A dead thread's host thread parks for ever.
//!
//! It may use `std` (rust-kernel.md: the host doubles); it holds no logic of the
//! scheduler's: that is the code under test.

// The host double alone links std outside the tests (rust-kernel.md: the host doubles).
extern crate std;

use core::ptr::NonNull;
use core::sync::atomic::Ordering;
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use crate::cpu::CpuCore;
use crate::thread::{MachineSlot, SONPROC};

/// `proc0`'s u-area on the host: bytes enough for `bsd`'s host `struct user` (whose pcb is a
/// word), page aligned; the host builds no frames in it.
#[repr(C, align(4096))]
struct BootUarea([core::sync::atomic::AtomicU8; 4096]);

/// `proc0`'s u-area (`hal::thread::boot_uarea`).
static PROC0_UAREA: BootUarea = BootUarea([const { core::sync::atomic::AtomicU8::new(0) }; 4096]);

/// The address of `proc0`'s u-area.
pub(crate) fn boot_uarea_word() -> usize {
    core::ptr::from_ref(&PROC0_UAREA).expose_provenance()
}

/// `curpcb = &proc0paddr->u_pcb` on this host thread's core.
pub(crate) fn set_boot_pcb() {
    crate::cpu::curcpu().set_curpcb(core::ptr::with_exposed_provenance::<()>(boot_uarea_word()));
}

/// A thread's gate: the core a switch handed it, until its host thread takes it.
#[derive(Default)]
struct Gate {
    core: Mutex<Option<&'static CpuCore>>,
    cv: Condvar,
}

/// The gates, by slot address.
static GATES: Mutex<BTreeMap<usize, Arc<Gate>>> = Mutex::new(BTreeMap::new());

/// A mutex's guard, a panicking test's poison ignored.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The gate of the slot at `addr`, made at its first use.
fn gate(addr: usize) -> Arc<Gate> {
    Arc::clone(lock(&GATES).entry(addr).or_default())
}

/// Hands `core` to the thread of the gate.
fn open(g: &Gate, core: &'static CpuCore) {
    *lock(&g.core) = Some(core);
    g.cv.notify_all();
}

/// Waits until a switch hands this gate a core, then installs it on the calling host thread.
fn wait(g: &Gate) {
    let mut c = lock(&g.core);
    loop {
        if let Some(core) = c.take() {
            drop(c);
            crate::cpu::set_current(core);
            return;
        }
        c = g.cv.wait(c).unwrap_or_else(PoisonError::into_inner);
    }
}

/// The switch: the assembly's stores, then the running core to the new thread, then (with an
/// old thread) the wait for a core to resume on, or (without) a park for ever.
pub(crate) fn switch(
    old: Option<&MachineSlot>,
    new_slot: NonNull<MachineSlot>,
    new_thread: NonNull<()>,
) {
    let core = crate::cpu::curcpu();
    let new_addr = new_slot.addr().get();
    {
        // SAFETY: `new_slot` comes from a `Next` the caller checked against the scheduler's
        // hold it holds (`hal::thread`'s E3): the slot of a thread chosen under that hold,
        // which is not freed while the hold is held, and whose members are atomics (the
        // machine's assembly stores into them through the same address). The reference ends
        // in this block, before the hold can be released.
        let slot = unsafe { new_slot.as_ref() };
        slot.stat.store(SONPROC, Ordering::Relaxed);
        slot.cpu.store(core.word(), Ordering::Relaxed);
    }
    core.set_curproc(new_thread.as_ptr());
    let next = gate(new_addr);
    // The core leaves this host thread before the next one installs it (one runner a core).
    crate::arch::host::cpu::release_current();
    match old {
        Some(old) => {
            let mine = gate(core::ptr::from_ref(old).addr());
            open(&next, core);
            wait(&mine);
        }
        None => {
            open(&next, core);
            loop {
                std::thread::park();
            }
        }
    }
}

/// `cpu_fork`'s host half: a host thread for the child, which waits for the first switch to
/// it, then runs the trampoline with `start` and `arg`.
pub(crate) fn thread_setup(
    _parent: &MachineSlot,
    _parent_runs: bool,
    child: &MachineSlot,
    _stack: usize,
    _tcb: usize,
    start: usize,
    arg: usize,
) {
    spawn(core::ptr::from_ref(child).addr(), start, arg);
}

/// A thread that runs nowhere starts afresh with `start` and `arg`.
pub(crate) fn thread_restart(t: &MachineSlot, start: usize, arg: usize) {
    spawn(core::ptr::from_ref(t).addr(), start, arg);
}

/// The host thread of the thread whose slot is at `addr`.
fn spawn(addr: usize, start: usize, arg: usize) {
    let g = gate(addr);
    std::thread::spawn(move || {
        wait(&g);
        crate::thread::trampoline(start, arg);
        // A thread whose start returns leaves for user mode on a machine; the host has none.
        loop {
            std::thread::park();
        }
    });
}
/* </CODE> */
