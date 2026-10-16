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
//! An init-once cell for a `static`: written once, read by anyone after.
//!
//! TODO(N2 integrate): replace with `hal::OnceStatic` (the runtime's init-once static,
//! docs/ZERO_UNSAFE.md 0.8 step 3) once that branch is in; this is the minimum the system
//! call registry ([`crate::syscall`]), `kernel::api` and the module loader need before it.
//!
//! The state goes `EMPTY` -> `BUSY` (one writer won the exchange) -> `READY` (the value is
//! written and published); it never goes back, and the value is never written again or
//! dropped (a `static` is never dropped; a local one leaks its value).

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

/// Nothing written yet.
const EMPTY: u8 = 0;
/// A writer won the exchange and is writing.
const BUSY: u8 = 1;
/// The value is written and visible.
const READY: u8 = 2;

/// A value written at most once, then shared.
pub struct Once<T> {
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> Once<T> {
    /// An empty cell.
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(EMPTY),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Writes the value if the cell is empty; otherwise hands it back.
    pub fn set(&self, value: T) -> Result<(), T> {
        if self
            .state
            .compare_exchange(EMPTY, BUSY, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(value);
        }
        // SAFETY: the EMPTY -> BUSY exchange succeeds for one caller only, ever (the state
        // never returns to EMPTY), so this is the only write to `value`; no reader touches it
        // until the READY store below, which happens after the write.
        unsafe { (*self.value.get()).write(value) };
        self.state.store(READY, Ordering::Release);
        Ok(())
    }

    /// The value, once written.
    pub fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) != READY {
            return None;
        }
        // SAFETY: READY is stored (Release) only after the single write in `set` completed,
        // and this Acquire load saw it, so the value is initialised and visible here; it is
        // never written again, so a shared reference for `&self`'s lifetime cannot alias a
        // write.
        Some(unsafe { (*self.value.get()).assume_init_ref() })
    }
}

impl<T> Default for Once<T> {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: the value is written once, by the one thread that won the exchange, and only read
// (shared) afterwards, behind the Release/Acquire pair; so sharing the cell shares `&T`
// across threads (`T: Sync`) and the write moves a `T` in from the setting thread
// (`T: Send`).
unsafe impl<T: Send + Sync> Sync for Once<T> {}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_until_set() {
        let cell: Once<u32> = Once::new();
        assert_eq!(cell.get(), None);
        assert_eq!(cell.set(7), Ok(()));
        assert_eq!(cell.get(), Some(&7));
    }

    #[test]
    fn a_second_set_is_refused() {
        let cell: Once<u32> = Once::new();
        assert_eq!(cell.set(1), Ok(()));
        assert_eq!(cell.set(2), Err(2));
        assert_eq!(cell.get(), Some(&1));
    }

    #[test]
    fn one_writer_among_threads() {
        static CELL: Once<usize> = Once::new();
        let winners: usize = std::thread::scope(|s| {
            let handles: std::vec::Vec<_> = (0..8)
                .map(|i| s.spawn(move || usize::from(CELL.set(i).is_ok())))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap_or(0)).sum()
        });
        assert_eq!(winners, 1);
        assert!(CELL.get().is_some());
    }
}
/* </TESTS> */
