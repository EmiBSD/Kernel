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
//! The host's running CPU for `hal::cpu`: every host thread is a CPU of its own (its core
//! leaked at its first use, numbered from a counter), unless a core was installed on it
//! ([`set_current`], what a host thread running as another CPU does), so two test threads
//! contend for a lock and update per-CPU copies as two CPUs do.

// The host double alone links std outside the tests (rust-kernel.md: the host doubles).
extern crate std;

use core::cell::Cell;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::boxed::Box;

use crate::cpu::{CpuCore, MAXCPUS};

/// The number the next thread's own core gets.
static NEXT: AtomicUsize = AtomicUsize::new(0);

std::thread_local! {
    /// The thread's own core, numbered at its first use.
    static OWN: &'static CpuCore = {
        let c: &'static CpuCore = Box::leak(Box::new(CpuCore::new()));
        c.set_number(NEXT.fetch_add(1, Ordering::Relaxed) % MAXCPUS);
        c
    };
    /// A core installed on the thread, which answers instead of its own.
    static CURRENT: Cell<Option<&'static CpuCore>> = const { Cell::new(None) };
}

/// The running thread's core: the one installed on it, else its own.
#[inline]
pub(crate) fn current() -> Option<&'static CpuCore> {
    Some(CURRENT.with(Cell::get).unwrap_or_else(|| OWN.with(|c| *c)))
}

/// Installs `c` on the calling thread.
pub(crate) fn set_current(c: &'static CpuCore) {
    CURRENT.with(|cur| cur.set(Some(c)));
}

/// `ncpusfound` as the per-CPU readers walk it: every slot, as any thread may be a CPU.
#[inline]
pub(crate) fn ncpus(_attached: usize) -> usize {
    MAXCPUS
}
/* </CODE> */
