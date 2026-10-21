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
//! Soft interrupts, as the kernel reaches them: `softintr_establish(9)` and
//! `softintr_schedule(9)`.
//!
//! The soft interrupt queues and their dispatch stay in `bsd` until N4.8
//! (`sys/kern/kern_softintr.rs`, the machine's `softintr()`), so they arrive through the
//! [`SoftIntr`] bridge, registered by `bsd` at boot (bridges.toml; paid by N4). A handler is a
//! plain `fn()`: the kernel's handlers keep their state in their own statics, so no `void *`
//! argument crosses (the C's `arg` is `NULL` for `softclock`), and the handle the bridge
//! returns is an opaque word, compared and passed back, never dereferenced.
//!
//! A handler established without `IPL_MPSAFE` in `ipl` runs under the kernel lock (the C's
//! `SIF_MPSAFE` unset), as `softclock` does.
//!
//! **Before registration** (the host tests that register none) nothing can be established:
//! [`establish`] answers `ENXIO`, the C's `softintr_establish` returning `NULL`, which its
//! callers turn into `panic(9)`.
//!
//! ## Redesign
//! - Not a port: the C calls `softintr_establish(ipl, fn, arg)` and keeps the `void *`
//!   handle; the kernel calls one trait object and keeps a [`SoftHandle`], until the soft
//!   interrupts enter `hal` with the interrupt level (N4.5, N4.8).

use crate::bridges;
use crate::sys::errno::Errno;

/// The soft interrupts of the machine, as `bsd` registers them (`sys/kern/kern_softintr.rs`).
pub trait SoftIntr: Sync {
    /// `softintr_establish(ipl, run, NULL)`: `run` runs at the soft level that serves `ipl`
    /// each time the handle is scheduled; the handle as a word (never 0), `None` when the C's
    /// `softintr_establish` returns `NULL` (no memory for the handle, no free slot).
    fn establish(&self, ipl: i32, run: fn()) -> Option<usize>;
    /// `softintr_schedule(handle)`: queues the handler (or marks it to run again if it is
    /// running) and raises the soft interrupt.
    fn schedule(&self, handle: usize);
}

/// An established soft interrupt handler: the bridge's word, opaque.
#[derive(Debug)]
pub struct SoftHandle(usize);

impl SoftHandle {
    /// `softintr_schedule(9)`: the handler runs soon at its soft level.
    pub fn schedule(&self) {
        if let Some(b) = bridges::softintr() {
            b.schedule(self.0);
        }
    }
}

/// `softintr_establish(9)`: `run` will run at the soft level that serves `ipl` whenever the
/// handle is scheduled. `ENXIO` before the bridge is registered, `ENOMEM` when the C would
/// return `NULL`.
pub fn establish(ipl: i32, run: fn()) -> Result<SoftHandle, Errno> {
    let b = bridges::softintr().ok_or(Errno::ENXIO)?;
    b.establish(ipl, run).map(SoftHandle).ok_or(Errno::ENOMEM)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::testutil;

    static RAN: AtomicU32 = AtomicU32::new(0);

    fn bump() {
        RAN.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn a_scheduled_handler_runs_through_the_bridge() {
        testutil::install();
        let h = establish(crate::intr::IPL_SOFTCLOCK, bump).unwrap();
        let before = RAN.load(Ordering::Relaxed);
        h.schedule();
        h.schedule();
        assert_eq!(RAN.load(Ordering::Relaxed), before + 2);
    }
}
/* </TESTS> */
