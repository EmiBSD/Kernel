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
//! `ReadMostly<T>`: a value read on a hot path by any CPU and replaced rarely, by a path the
//! C already makes single-threaded: a process's system call pin tables (`ps_pin`,
//! `ps_libcpin`), read by `pin_check` at every system call, replaced by `exec` (under
//! `single_thread_set`), by `pinsyscalls(2)` once, and by the last thread's `exit1`.
//!
//! Not an OpenBSD file. The C reads the table's words with no lock and trusts its callers
//! never to replace a table another thread is reading. A Rust reference handed out of a cell
//! must not outlive a replacement, so the trust becomes a count: a reader holds a
//! [`ReadMostlyRef`] (one atomic add, one subtract), and a writer ([`ReadMostly::update`],
//! [`ReadMostly::replace`]) claims the cell only when no reader is inside, spinning until
//! then; readers that arrive while a writer holds it spin until it is done. The C's premise
//! (no replacement races a reader) makes the spins empty; a broken premise costs a wait,
//! never a dangling reference.
//!
//! What the user of a cell must keep (each is a wait, not a hazard): a reader does not sleep
//! or take the cell again for writing while it holds a [`ReadMostlyRef`] (a writer on the
//! same CPU would spin for ever), and the cell is not read or written from an interrupt
//! handler that may interrupt a holder.
//!
//! ## Redesign
//! - N4.8's decision 12, pulled forward with its first user (N4.6a): LZ's `struct
//!   pinsyscall` of atomic words with the table's address in a `PtrWord`, read through an
//!   `unsafe` deref at every system call (`sys/sys/syscall_mi.rs`) and freed by `exit` and
//!   `exec` behind the same premise -> an owned table in a [`ReadMostly`] cell: readers
//!   borrow it, the replacement waits for them, the old table goes with its owner.

use core::cell::UnsafeCell;
use core::ops::Deref;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::assert::hal_panic;

/// The writer's bit of [`ReadMostly`]'s state; the low bits count the readers inside.
const WRITER: u32 = 1 << 31;

/// A value many readers borrow and a rare writer replaces (see the module's docs).
pub struct ReadMostly<T> {
    /// The readers inside (bits 0-30) and the writer (bit 31).
    state: AtomicU32,
    /// The value: shared by the readers counted in `state`, mutated by the writer alone.
    value: UnsafeCell<T>,
}

// SAFETY: the value is reached only through a `ReadMostlyRef` (a shared reference, while the
// reader count it took keeps every writer out) or inside `update` (the only `&mut`, while the
// writer bit keeps every reader and other writer out): so references to it never alias a
// `&mut`, and any CPU may hold one. A reader on another CPU gets `&T` (hence `T: Sync`) and a
// writer may install a value made on another CPU and hand the old one back there (hence
// `T: Send`).
unsafe impl<T: Send + Sync> Sync for ReadMostly<T> {}

impl<T> ReadMostly<T> {
    /// A cell holding `value`, no reader inside.
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            value: UnsafeCell::new(value),
        }
    }

    /// The value, borrowed while the returned reference lives: one more reader counted,
    /// once no writer holds the cell (spinning until then). `panic(9)` past 2^31 - 1
    /// readers.
    #[inline]
    pub fn read(&self) -> ReadMostlyRef<'_, T> {
        let mut s = self.state.load(Ordering::Relaxed);
        loop {
            if s & WRITER != 0 {
                core::hint::spin_loop();
                s = self.state.load(Ordering::Relaxed);
                continue;
            }
            if s == WRITER - 1 {
                hal_panic(format_args!("ReadMostly: too many readers"));
            }
            match self
                .state
                .compare_exchange_weak(s, s + 1, Ordering::Acquire, Ordering::Relaxed)
            {
                Ok(_) => return ReadMostlyRef { cell: self },
                Err(now) => s = now,
            }
        }
    }

    /// `f` on the value, alone: the cell is claimed once no reader is inside and no other
    /// writer holds it (spinning until then), and released after `f`, also when it unwinds.
    /// `f` must not read the cell (it would wait for itself).
    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self
            .state
            .compare_exchange_weak(0, WRITER, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let claim = Claim(&self.state);
        let v = self.value.get();
        // SAFETY: the compare-and-swap above moved the state from 0 (no reader, no writer)
        // to `WRITER`, which `read` and this function refuse to enter while it is set, until
        // `claim`'s drop clears it after `f` (on return or unwind). So no `ReadMostlyRef` of
        // this cell is alive (each holds a count the state did not have) and none is made, and
        // no other `update` runs: this `&mut` is the only reference to the value while it
        // lives. `v` is the cell's own, valid, initialised value.
        let r = f(unsafe { &mut *v });
        drop(claim);
        r
    }

    /// The value replaced by `value` (as [`ReadMostly::update`]); the old one, which the
    /// caller drops outside the cell.
    pub fn replace(&self, value: T) -> T {
        self.update(|v| core::mem::replace(v, value))
    }

    /// The value, by the cell's sole owner (no reader can exist: `&mut self`).
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

impl<T: Default> Default for ReadMostly<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// The writer's claim of a [`ReadMostly`], released when dropped (also on unwind).
struct Claim<'a>(&'a AtomicU32);

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        self.0.store(0, Ordering::Release);
    }
}

/// A reader's borrow of a [`ReadMostly`]'s value; the reader leaves when it drops.
pub struct ReadMostlyRef<'a, T> {
    /// The cell, with this reader counted in its state.
    cell: &'a ReadMostly<T>,
}

impl<T> Deref for ReadMostlyRef<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        let v = self.cell.value.get();
        // SAFETY: this reader is counted in the cell's state from `read` until its drop, and
        // no writer claims the cell while the count is not 0 (`update` moves it only from 0):
        // so no `&mut` to the value exists while this shared reference lives (it borrows
        // `self`, which the count outlives). `v` is the cell's valid, initialised value.
        unsafe { &*v }
    }
}

impl<T> Drop for ReadMostlyRef<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.cell.state.fetch_sub(1, Ordering::Release);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    extern crate std;

    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::vec::Vec;

    use super::*;

    #[test]
    fn readers_borrow_and_a_replacement_hands_the_old_value_back() {
        let c = ReadMostly::new(Vec::from([1u32, 2, 3]));
        {
            let a = c.read();
            let b = c.read();
            assert_eq!(a[1], 2);
            assert_eq!(b.len(), 3);
            assert_eq!(c.state.load(Ordering::Relaxed), 2);
        }
        assert_eq!(c.state.load(Ordering::Relaxed), 0);
        let old = c.replace(Vec::from([9]));
        assert_eq!(old, [1, 2, 3]);
        assert_eq!(*c.read(), [9]);
        c.update(|v| v.push(10));
        assert_eq!(*c.read(), [9, 10]);
    }

    #[test]
    fn a_writer_waits_for_the_readers_inside() {
        let c = Arc::new(ReadMostly::new(0u64));
        let r = c.read();
        let done = Arc::new(AtomicBool::new(false));
        let t = {
            let c = Arc::clone(&c);
            let done = Arc::clone(&done);
            std::thread::spawn(move || {
                c.replace(7);
                done.store(true, Ordering::Release);
            })
        };
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(
            !done.load(Ordering::Acquire),
            "the writer went in past a reader"
        );
        assert_eq!(*r, 0);
        drop(r);
        t.join().unwrap();
        assert_eq!(*c.read(), 7);
    }

    #[test]
    fn a_panicking_writer_releases_the_cell() {
        let c = ReadMostly::new(1i32);
        let r = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
            c.update(|_| std::panic!("writer"));
        }));
        assert!(r.is_err());
        assert_eq!(c.state.load(Ordering::Relaxed), 0);
        assert_eq!(*c.read(), 1);
    }
}
/* </TESTS> */
