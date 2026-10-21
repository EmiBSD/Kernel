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
//! Kernel threads, as the kernel crate makes them: `kthread_create(9)`,
//! `kthread_create_deferred`, `kthread_exit`, and pegging a thread to the primary CPU.
//!
//! The process and scheduler code stays in `bsd` until N4.5 (`kern_kthread.rs`, `kern_fork.rs`,
//! `kern_sched.rs`), so it arrives through the [`Kthread`] bridge (bridges.toml; paid by N4),
//! registered by `bsd` at boot.
//!
//! **The start ticket.** The C hands a new thread a function and a `void *` (`softclock_thread`
//! and `NULL`, `taskq_thread` and the queue), which the thread casts back. Here no pointer
//! crosses: the kernel files what the new thread is to run as a [`Start`] in [`STARTS`] and
//! hands the bridge the slot's number, a ticket; `bsd`'s thread function calls [`run`] (or,
//! for a deferred creation, [`run_deferred`]) with it, which takes the `Start` back out, in
//! safe code: what a ticket names is a value of this crate's own types (a queue's counted
//! handle), never a re-typed address. A ticket that cannot be filed (no memory to grow the
//! table) is the C's failure of the allocation behind it: `kthread_create`'s `ENOMEM`, or
//! `kthread_create_deferred`'s "unable to allocate kthread_q" panic.
//!
//! ## Redesign
//! - `kthread_create(func, arg, NULL, name)` with `arg` cast back by `func` -> [`create`] with
//!   a [`Start`]: the argument is typed, owned by the ticket until the thread takes it.

use alloc::vec::Vec;

use hal::bridges::AlreadyRegistered;
use hal::intr::IPL_NONE;
use hal::mutex::Mutexed;
use hal::once::OnceStatic;
use hal::sys::errno::Errno;

use crate::assert::kpanic;

/// What a kernel thread made through this module runs, or what a deferred creation calls:
/// filed under a ticket until the thread (or the deferred call) takes it. The variants are
/// the kernel's subsystems that make threads (timeout(9), task(9)).
pub(crate) enum Start {}

/// The starts filed and not yet taken, by ticket; a taken slot is `None` and reused.
static STARTS: Mutexed<Vec<Option<Start>>> = Mutexed::new(IPL_NONE, Vec::new());

/// The [`Kthread`] bridge, registered once by `bsd` (the host tests' doubles).
static KTHREAD: OnceStatic<&'static dyn Kthread> = OnceStatic::new();

/// The kernel threads of `bsd`'s process code (`sys/kern/kern_kthread.rs`,
/// `sys/kern/kern_sched.rs`), until they move (N4.5): bridges.toml `Kthread`.
pub trait Kthread: Sync {
    /// `kthread_create(func, ticket, NULL, name)`: a kernel thread named `name` whose function
    /// calls [`run`] with `ticket`; the error of `fork1` (the C's non-zero return).
    fn create(&self, name: &'static [u8], ticket: usize) -> Result<(), Errno>;
    /// `kthread_create_deferred(func, ticket)`: a function that calls [`run_deferred`] with
    /// `ticket` once threads can be made (`kthread_run_deferred_queue`), or at once after.
    fn create_deferred(&self, ticket: usize);
    /// `kthread_exit(ecode)`: the running kernel thread ends.
    fn exit(&self, ecode: i32) -> !;
    /// `sched_peg_curproc(ci)` of the primary CPU (`CPU_INFO_FOREACH` until
    /// `CPU_IS_PRIMARY(ci)`): the running thread runs only there from now on.
    fn peg_to_primary_cpu(&self);
}

/// Registers the [`Kthread`] bridge, once; a second call keeps the first and names it.
pub fn install_kthread(imp: &'static dyn Kthread) -> Result<(), AlreadyRegistered> {
    KTHREAD.set(imp).map_err(|_| AlreadyRegistered("Kthread"))
}

/// The registered bridge: making a thread before `bsd` registered it is a boot-order bug.
fn bridge() -> &'static dyn Kthread {
    match KTHREAD.get() {
        Some(k) => *k,
        None => kpanic(format_args!("kthread: no Kthread bridge registered")),
    }
}

/// Files `s` under a ticket: a free slot, or a new one when the table can grow; `s` back
/// when it cannot.
fn ticket(s: Start) -> Result<usize, Start> {
    let mut v = STARTS.lock();
    if let Some(i) = v.iter().position(Option::is_none) {
        v[i] = Some(s);
        return Ok(i);
    }
    if v.try_reserve(1).is_err() {
        return Err(s);
    }
    v.push(Some(s));
    Ok(v.len() - 1)
}

/// The start filed under `t`, taken out (`None` for a ticket not filed or taken already).
fn take(t: usize) -> Option<Start> {
    STARTS.lock().get_mut(t).and_then(Option::take)
}

/// `kthread_create(9)`: a kernel thread named `name` that runs `s`. `ENOMEM` when the ticket
/// cannot be filed; `fork1`'s error otherwise (the start is dropped then).
pub(crate) fn create(name: &'static [u8], s: Start) -> Result<(), Errno> {
    let t = ticket(s).map_err(|_| Errno::ENOMEM)?;
    bridge().create(name, t).inspect_err(|_| drop(take(t)))
}

/// `kthread_create_deferred(9)`: `s` runs (by [`run_deferred`]) once threads can be made, or
/// at once after that. Panics where the C's allocation of its queue entry fails.
pub(crate) fn create_deferred(s: Start) {
    let Ok(t) = ticket(s) else {
        kpanic(format_args!("unable to allocate kthread_q"));
    };
    bridge().create_deferred(t);
}

/// What a kernel thread made by [`create`] runs: called by `bsd`'s thread function with the
/// ticket it was given. Never returns (each start ends in [`exit`] or runs for ever).
pub fn run(ticket: usize) -> ! {
    let Some(s) = take(ticket) else {
        kpanic(format_args!("kthread: ticket {ticket} names no start"));
    };
    match s {}
}

/// What a deferred creation calls: called by `bsd`'s deferred function with its ticket.
pub fn run_deferred(ticket: usize) {
    let Some(s) = take(ticket) else {
        kpanic(format_args!("kthread: ticket {ticket} names no start"));
    };
    match s {}
}

/// `kthread_exit(9)`.
pub(crate) fn exit(ecode: i32) -> ! {
    bridge().exit(ecode)
}

/// `sched_peg_curproc` of the primary CPU.
pub(crate) fn peg_to_primary_cpu() {
    bridge().peg_to_primary_cpu();
}
/* </CODE> */
