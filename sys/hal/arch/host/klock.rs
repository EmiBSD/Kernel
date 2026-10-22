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
//! The host double of the kernel lock (`crate::klock`): a per-thread depth. Every test thread
//! is a CPU of its own here, and tests that take the kernel lock wait for each other's
//! threads, so the double counts each thread's holds and excludes nothing (the ticket lock is
//! tested on its own, `crate::klock`'s tests).

extern crate std;

use core::cell::Cell;

std::thread_local! {
    /// How many times this thread holds the kernel lock.
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// `KERNEL_LOCK_INIT()`: this thread holds nothing.
pub(crate) fn init() {
    DEPTH.with(|d| d.set(0));
}

/// `__mp_lock(&kernel_lock)`: one level more.
pub(crate) fn lock() {
    DEPTH.with(|d| d.set(d.get() + 1));
}

/// `__mp_unlock(&kernel_lock)`: one level less; `panic(9)` when none is held.
pub(crate) fn unlock() {
    DEPTH.with(|d| match d.get().checked_sub(1) {
        Some(n) => d.set(n),
        None => crate::assert::hal_panic(format_args!("kernel_unlock: not locked")),
    });
}

/// `__mp_lock_held(&kernel_lock, curcpu())`.
pub(crate) fn held() -> bool {
    DEPTH.with(Cell::get) > 0
}

/// This thread's depth.
pub(crate) fn depth() -> u32 {
    DEPTH.with(Cell::get)
}

/// `__mp_release_all(&kernel_lock)`: every level given up; how many there were.
pub(crate) fn release_all() -> u32 {
    DEPTH.with(|d| d.replace(0))
}

/// `__mp_acquire_count(&kernel_lock, count)`.
pub(crate) fn acquire_count(count: u32) {
    DEPTH.with(|d| d.set(d.get() + count));
}
/* </CODE> */
