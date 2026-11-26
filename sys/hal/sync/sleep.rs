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
//! The sleep queues, as the runtime reaches them: `tsleep(9)`, `wakeup(9)` and the two halves
//! `sleep_setup`/`sleep_finish` that the sleeping locks use.
//!
//! The sleep queues and the scheduler stay in `bsd` until N4 (`sys/kern/kern_synch.rs`), so
//! they arrive through the [`Sleep`] bridge, registered by `bsd` at boot (bridges.toml; paid by
//! N4). A wait channel is an address and nothing more: it is compared, never dereferenced, so
//! it travels as `*const ()` and any object's address serves, as in the C.
//!
//! **Before registration** (the boot CPU before `main()` registers the bridges, the host tests
//! that register none) nothing else runs, so nothing could wake a sleeper: [`tsleep_nsec`]
//! returns at once, as the C's `tsleep` does while `cold`, and the sleeping locks spin
//! instead of sleeping (`rwlock.rs`, `refcnt.rs`), which on one CPU with nothing else to run
//! means the lock is free whenever it is asked for. A wakeup is then a no-op.
//!
//! ## Redesign
//! - Not a port: the C calls the sleep queue functions directly; the runtime calls them
//!   through one trait object until the scheduler moves into `kernel` (N4).
//! - N3.5 S0: [`curproc_word`] and [`cold`] are public, for the kernel's map (the `busy`
//!   owner of `vm_map_busy`, `uvm_mapent_alloc`'s `cold` test).
//! - N3.5 S1: [`sched_pause`], for `uvm_map_teardown`'s yield between entries.
//! - N4.1: [`sleep_setup`] and [`sleep_finish`], the two halves the C calls apart to wait
//!   without holding its own mutex and the scheduler's lock at once (the softclock threads),
//!   and [`PSWP`], [`PRIBIO`].

use crate::bridges;
use crate::sys::errno::Errno;

/// `PSWP`: the highest sleep priority (the softclock threads' `"tmoslp"`).
pub const PSWP: i32 = 0;
/// `PVM`: the priority of a wait for virtual memory (`km_alloc`'s `"km_allocva"`).
pub const PVM: i32 = 4;
/// `PRIBIO`: the priority of a wait for a block device or a resource map (extent(9)'s
/// `"extnt"`).
pub const PRIBIO: i32 = 16;
/// `PWAIT`: the priority of a plain wait (`refcnt_finalize`).
pub const PWAIT: i32 = 32;
/// `PLOCK`: the priority of a lock's sleep (`rw_enter`'s readers; writers sleep at
/// `PLOCK - 4`).
pub const PLOCK: i32 = 36;
/// `PRIMASK`: the priority bits of a sleep's `prio`.
pub const PRIMASK: i32 = 0x0ff;
/// `PCATCH`: OR'd with the priority, a signal ends the sleep (`EINTR` or `ERESTART`).
pub const PCATCH: i32 = 0x100;
/// `INFSLP`: no timeout.
pub const INFSLP: u64 = u64::MAX;

/// The sleep queues of the scheduler, as `bsd` registers them (`sys/kern/kern_synch.rs`).
pub trait Sleep: Sync {
    /// `curproc` as a word, 0 when no process runs: an rwlock's writer (`rw_self`).
    fn curproc_word(&self) -> usize;
    /// `sleep_setup(ident, prio, wmesg)`: the running thread goes on `ident`'s sleep queue.
    fn sleep_setup(&self, ident: *const (), prio: i32, wmesg: &'static str);
    /// `sleep_finish(nsecs, do_sleep)`: sleeps (when `do_sleep`) until woken, the timeout
    /// (`EWOULDBLOCK`) or, with `PCATCH`, a signal (`EINTR`, `ERESTART`); leaves the queue.
    fn sleep_finish(&self, nsecs: u64, do_sleep: bool) -> Result<(), Errno>;
    /// `tsleep_nsec(ident, prio, wmesg, nsecs)`, its `cold || panicstr` path included.
    fn tsleep_nsec(
        &self,
        ident: *const (),
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno>;
    /// `wakeup(ident)`: every thread sleeping on `ident`.
    fn wakeup(&self, ident: *const ());
    /// `wakeup_one(ident)`: one thread sleeping on `ident`.
    fn wakeup_one(&self, ident: *const ());
    /// `_kernel_lock_held()`: the running CPU holds the kernel lock (an rwlock writer does not
    /// spin then, so the CPUs waiting for the kernel lock progress).
    fn kernel_lock_held(&self) -> bool;
    /// `cold || panicstr`: autoconfiguration is not over, or the kernel panicked, so a sleep
    /// must not reach the sleep queues (`msleep_nsec`'s and `tsleep_nsec`'s early return).
    fn cold(&self) -> bool;
    /// That early return's work: `splhigh`, `splx(safepri)` (interrupts get their chance),
    /// the kernel lock released and taken back when this CPU holds it (`MULTIPROCESSOR`), the
    /// level put back. No process switch.
    fn cold_yield(&self);
    /// `sched_pause(yield)`: `yield()` when the running CPU's scheduler asks its thread to
    /// give the CPU up (`spc_schedflags & SPCF_SHOULDYIELD`), nothing otherwise.
    fn sched_pause(&self);
}

/// A wait channel: the address of `ident`, compared and never dereferenced.
pub fn chan<T: ?Sized>(ident: *const T) -> *const () {
    ident.cast::<()>()
}

/// `tsleep_nsec(9)`: sleeps on `ident` until a wakeup, the timeout (`EWOULDBLOCK`) or, with
/// `PCATCH` in `prio`, a signal. Returns at once before the bridge is registered.
pub fn tsleep_nsec<T: ?Sized>(
    ident: *const T,
    prio: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    crate::assert::kassert!(prio & !(PRIMASK | PCATCH) == 0);
    match bridges::sleep() {
        Some(b) => b.tsleep_nsec(chan(ident), prio, wmesg, nsecs),
        None => Ok(()),
    }
}

/// `sleep_setup(ident, prio, wmesg)`: the running thread goes on `ident`'s sleep queue, to
/// sleep in [`sleep_finish`] unless a wakeup on `ident` comes first. Nothing before the bridge
/// is registered.
pub fn sleep_setup<T: ?Sized>(ident: *const T, prio: i32, wmesg: &'static str) {
    crate::assert::kassert!(prio & !(PRIMASK | PCATCH) == 0);
    if let Some(b) = bridges::sleep() {
        b.sleep_setup(chan(ident), prio, wmesg);
    }
}

/// `sleep_finish(nsecs, do_sleep)`: sleeps (when `do_sleep`) until a wakeup on the channel
/// [`sleep_setup`] named, the timeout (`EWOULDBLOCK`) or, with `PCATCH`, a signal; leaves the
/// queue. `Ok` at once before the bridge is registered.
pub fn sleep_finish(nsecs: u64, do_sleep: bool) -> Result<(), Errno> {
    match bridges::sleep() {
        Some(b) => b.sleep_finish(nsecs, do_sleep),
        None => Ok(()),
    }
}

/// `wakeup(9)`: wakes every thread sleeping on `ident`.
pub fn wakeup<T: ?Sized>(ident: *const T) {
    if let Some(b) = bridges::sleep() {
        b.wakeup(chan(ident));
    }
}

/// `wakeup_one(9)`: wakes one thread sleeping on `ident`.
pub fn wakeup_one<T: ?Sized>(ident: *const T) {
    if let Some(b) = bridges::sleep() {
        b.wakeup_one(chan(ident));
    }
}

/// `curproc` as a word (0 before the bridge is registered): an identity, compared and never
/// dereferenced (an rwlock's writer, a map's `busy` owner, `vm_map_busy_ln`).
pub fn curproc_word() -> usize {
    bridges::sleep().map_or(0, Sleep::curproc_word)
}

/// `cold`: autoconfiguration is not over, so nothing may sleep or rely on the kernel map
/// growing (`uvm_mapent_alloc` takes its entries from the static ones then). The bridge
/// answers `cold || panicstr` (after a panic the static entries serve too, harmlessly);
/// before it is registered the kernel is booting: `true`.
pub fn cold() -> bool {
    bridges::sleep().is_none_or(Sleep::cold)
}

/// `sched_pause(yield)` (`<sys/sched.h>`): gives the CPU up when its scheduler asks
/// (`SPCF_SHOULDYIELD`), as a long loop of the kernel's does once per step
/// (`uvm_map_teardown`'s entries). Nothing before the bridge is registered: nothing else runs.
pub fn sched_pause() {
    if let Some(b) = bridges::sleep() {
        b.sched_pause();
    }
}

/// `_kernel_lock_held()` (`false` before the bridge is registered).
pub(crate) fn kernel_lock_held() -> bool {
    bridges::sleep().is_some_and(Sleep::kernel_lock_held)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::testutil;

    #[test]
    fn a_wakeup_ends_the_sleep() {
        testutil::install();
        let flag = Arc::new(AtomicBool::new(false));
        let f2 = Arc::clone(&flag);
        let sleeper = thread::spawn(move || {
            while !f2.load(Ordering::Acquire) {
                // Timed: the flag is read outside any lock, so the wakeup may come first.
                let _ = tsleep_nsec(Arc::as_ptr(&f2), PWAIT, "test", 50_000_000);
            }
        });
        thread::sleep(Duration::from_millis(10));
        flag.store(true, Ordering::Release);
        wakeup(Arc::as_ptr(&flag));
        sleeper.join().unwrap();
    }

    #[test]
    fn a_timed_sleep_expires_with_ewouldblock() {
        testutil::install();
        let x = 0u8;
        assert_eq!(
            tsleep_nsec(&raw const x, PWAIT, "test", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
    }

    #[test]
    fn cold_and_curproc_are_the_bridges() {
        testutil::install();
        assert!(!cold());
        testutil::set_cold(true);
        assert!(cold());
        testutil::set_cold(false);
        let me = curproc_word();
        assert_ne!(me, 0);
        let other = thread::spawn(curproc_word).join().unwrap();
        assert_ne!(me, other);
    }

    #[test]
    fn sched_pause_asks_the_bridge() {
        testutil::install();
        let before = testutil::sched_pauses();
        sched_pause();
        sched_pause();
        assert_eq!(testutil::sched_pauses(), before + 2);
    }
}
/* </TESTS> */
