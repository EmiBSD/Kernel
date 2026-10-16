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
//! `OnceStatic<T>`: a `static` set once, before its first use, and read for ever after.
//!
//! Not an OpenBSD file. The C spells a table written at boot and read afterwards (a function
//! table, a console, the attachments that `main()` makes) as a plain global assigned once;
//! LZ spells it `StaticCell<T>` (`sys/lib/libkern/staticcell.rs`), whose every access is
//! `unsafe` and names the boot phase that excludes a writer. This type checks the phase
//! instead: [`OnceStatic::set`] succeeds once and refuses every later call (handing the value
//! back), and [`OnceStatic::get`] answers `None` until the value is in place. No caller writes
//! `unsafe`.
//!
//! What "before set" means is the reader's decision, made a type: `get` is an `Option`, so
//! each user states the behaviour of the early boot (the runtime's bridges, `sys/hal/
//! bridges.rs`, say it per bridge: an absent interrupt-level bridge means no interrupts yet,
//! an absent console drops the output, ...). A read never blocks and never panics.
//!
//! ## Redesign
//! - Replaces `StaticCell<T>` for new code whose value is written once (the registry of the
//!   bridges first, docs/ZERO_UNSAFE.md section 0.1). `StaticCell` stays in `libkern` for the
//!   legacy tree (`hal` depends on `libkern`, so `libkern` cannot re-export this type); its
//!   uses move subsystem by subsystem.
//! - The state is one atomic byte: `EMPTY`, `BUSY` while the winner of [`OnceStatic::set`]
//!   writes, `READY` once it is visible. A concurrent second `set` fails instead of waiting;
//!   a reader that races with the writer sees `None`, which is the documented "before set".

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

/// No value, and no `set` under way.
const EMPTY: u8 = 0;
/// A `set` is writing the value.
const BUSY: u8 = 1;
/// The value is written and visible.
const READY: u8 = 2;

/// A value set once, before its first use, then read through `&self` for ever.
///
/// Invariant: `value` is initialised exactly when `state` is `READY`; from then on it is
/// never written again (only dropped with the cell), so shared references to it never alias
/// a write.
pub struct OnceStatic<T> {
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> OnceStatic<T> {
    /// An empty cell; `get` answers `None` until `set`.
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(EMPTY),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Stores `value`, once. A second call (or one racing with the first) gets its value back
    /// in the `Err`, untouched.
    pub fn set(&self, value: T) -> Result<(), T> {
        if self
            .state
            .compare_exchange(EMPTY, BUSY, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(value);
        }
        // SAFETY: the compare-exchange above moved `state` from EMPTY to BUSY, which only one
        // caller can do, ever (no path moves it back), so this is the only write and no
        // reader has a reference yet: `get` hands one out only after the READY store below.
        unsafe { (*self.value.get()).write(value) };
        self.state.store(READY, Ordering::Release);
        Ok(())
    }

    /// The value, once `set` has stored it; `None` before.
    pub fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) != READY {
            return None;
        }
        // SAFETY: READY is stored (with Release, matched by the Acquire above) only after the
        // one write of `set`, so the value is initialised and visible here; it is never
        // written again while `self` lives, so a shared reference for `&self`'s lifetime
        // aliases no write.
        Some(unsafe { (*self.value.get()).assume_init_ref() })
    }

    /// Whether `set` has stored the value.
    pub fn is_set(&self) -> bool {
        self.state.load(Ordering::Acquire) == READY
    }
}

impl<T> Default for OnceStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for OnceStatic<T> {
    fn drop(&mut self) {
        if *self.state.get_mut() == READY {
            // SAFETY: READY means `set` initialised the value (the invariant), and `&mut self`
            // proves no reference from `get` is alive; it is dropped once, here.
            unsafe { self.value.get_mut().assume_init_drop() };
        }
    }
}

// SAFETY: sharing `&OnceStatic<T>` across CPUs lets one of them move a `T` in (`set`: `T:
// Send`) and all of them read `&T` (`get`: `T: Sync`); the state machine above makes the one
// write happen before every read.
unsafe impl<T: Send + Sync> Sync for OnceStatic<T> {}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::thread;
    use std::vec::Vec;

    use super::*;

    #[test]
    fn empty_until_set_then_the_value() {
        let c = OnceStatic::<u32>::new();
        assert!(c.get().is_none());
        assert!(!c.is_set());
        assert_eq!(c.set(7), Ok(()));
        assert_eq!(c.get(), Some(&7));
        assert!(c.is_set());
    }

    #[test]
    fn a_second_set_is_refused_and_returns_its_value() {
        let c = OnceStatic::new();
        assert_eq!(c.set(1u8), Ok(()));
        assert_eq!(c.set(2u8), Err(2));
        assert_eq!(c.get(), Some(&1));
    }

    #[test]
    fn works_as_a_static() {
        static S: OnceStatic<&str> = OnceStatic::new();
        assert!(S.get().is_none() || S.get() == Some(&"x"));
        let _ = S.set("x");
        assert_eq!(S.get(), Some(&"x"));
    }

    #[test]
    fn racing_setters_one_wins() {
        let c = Arc::new(OnceStatic::<usize>::new());
        let wins = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let c = Arc::clone(&c);
                let wins = Arc::clone(&wins);
                thread::spawn(move || {
                    if c.set(i).is_ok() {
                        wins.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(wins.load(Ordering::Relaxed), 1);
        assert!(c.get().is_some_and(|v| *v < 8));
    }

    #[test]
    fn drops_the_value_once() {
        struct D(Arc<AtomicUsize>);
        impl Drop for D {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }
        let n = Arc::new(AtomicUsize::new(0));
        {
            let c = OnceStatic::new();
            assert!(c.set(D(Arc::clone(&n))).is_ok());
            // The refused value is handed back and dropped by the caller.
            drop(c.set(D(Arc::clone(&n))));
            assert_eq!(n.load(Ordering::Relaxed), 1);
        }
        assert_eq!(n.load(Ordering::Relaxed), 2);
        let empty = OnceStatic::<D>::new();
        drop(empty);
        assert_eq!(n.load(Ordering::Relaxed), 2);
    }
}
/* </TESTS> */
