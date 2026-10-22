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
use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

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
    /// The core the thread runs as: one installed on it, else its own, kept here at its
    /// first read so a read is one access to a `const` thread-local.
    static CURRENT: Cell<Option<&'static CpuCore>> = const { Cell::new(None) };
}

/// The running thread's core: the one installed on it, else its own.
#[inline]
pub(crate) fn current() -> Option<&'static CpuCore> {
    CURRENT.with(|cur| match cur.get() {
        Some(c) => Some(c),
        None => {
            let own = OWN.with(|c| *c);
            cur.set(Some(own));
            Some(own)
        }
    })
}

/// The installed cores and the host thread each is installed on (the address of that
/// thread's [`CLAIM`]): one runner per core, as one CPU runs one context at a time.
static CLAIMS: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());

/// The calling thread's claim on its installed core (the core's address, 0 for none), given
/// up when the thread ends (its destructor) or hands the core on ([`release_current`]).
struct Claim(Cell<usize>);

impl Drop for Claim {
    fn drop(&mut self) {
        let core = self.0.get();
        if core != 0 {
            let me = core::ptr::from_ref(self).addr();
            let mut claims = CLAIMS.lock().unwrap_or_else(PoisonError::into_inner);
            if claims.get(&core) == Some(&me) {
                claims.remove(&core);
            }
        }
    }
}

std::thread_local! {
    /// The calling thread's [`Claim`].
    static CLAIM: Claim = const { Claim(Cell::new(0)) };
}

/// Installs `c` on the calling thread. Panics when another live host thread runs `c` (a core
/// is one CPU: two runners would share its owner word and its per-CPU state); the thread's
/// previous claim, if any, is given up.
pub(crate) fn set_current(c: &'static CpuCore) {
    let core = core::ptr::from_ref(c).addr();
    CLAIM.with(|claim| {
        let me = core::ptr::from_ref(claim).addr();
        let mut claims = CLAIMS.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(&owner) = claims.get(&core)
            && owner != me
        {
            drop(claims);
            crate::assert::hal_panic(format_args!(
                "hal::cpu::set_current: the core runs on another host thread"
            ));
        }
        let old = claim.0.replace(core);
        if old != 0 && old != core && claims.get(&old) == Some(&me) {
            claims.remove(&old);
        }
        claims.insert(core, me);
    });
    CURRENT.with(|cur| cur.set(Some(c)));
}

/// The calling thread gives its installed core up (the switch double hands it to the next
/// thread's host thread): the thread runs as no CPU until a core is installed again.
pub(crate) fn release_current() {
    CLAIM.with(|claim| {
        let me = core::ptr::from_ref(claim).addr();
        let core = claim.0.replace(0);
        if core != 0 {
            let mut claims = CLAIMS.lock().unwrap_or_else(PoisonError::into_inner);
            if claims.get(&core) == Some(&me) {
                claims.remove(&core);
            }
        }
    });
    CURRENT.with(|cur| cur.set(None));
}

/// `intr_disable()` of the host double: a host thread takes no interrupts; nothing to mask.
#[inline]
pub(crate) fn intr_disable() -> u64 {
    0
}

/// `intr_restore(s)` of the host double: nothing was masked.
#[inline]
pub(crate) fn intr_restore(_s: u64) {}

/// `ncpusfound` as the per-CPU readers walk it: every slot, as any thread may be a CPU.
#[inline]
pub(crate) fn ncpus(_attached: usize) -> usize {
    MAXCPUS
}
/* </CODE> */
