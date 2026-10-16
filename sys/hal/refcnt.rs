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

/*
 * Copyright (c) 2015 David Gwynne <dlg@openbsd.org>
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

/*
 * Copyright (c) 1982, 1986, 1990, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)kern_synch.c	8.6 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! `Refcnt`: the reference count of `refcnt_init(9)`, with its sleeping finalize.
//!
//! Upstream: sys/sys/refcnt.h @ 3ce1f3f79392
//! Upstream: sys/kern/kern_synch.c @ 3ce1f3f79392
//! LZ: sys/sys/refcnt.rs@3c62ede68802
//! LZ: sys/kern/kern_synch.rs@3c62ede68802
//!
//! The behaviour of `refcnt(9)`: a count that starts at 1 (`refcnt_init`); `refcnt_take`
//! adds one to an object already referenced; `refcnt_rele` drops one and answers whether it
//! was the last (with release ordering before the drop and acquire after the last, so the
//! last owner sees every write of the others); `refcnt_rele_wake` drops one and wakes the
//! finalizer when it was the last; `refcnt_finalize` drops the caller's own and sleeps
//! (`PWAIT`, the given message) until every other reference is gone; `refcnt_read` and
//! `refcnt_shared` are snapshots.
//!
//! The finalizer's sleep goes through the `Sleep` bridge (`sleep.rs`); before it is
//! registered nothing else runs that could drop a reference, so a finalizer that finds other
//! references then would wait for ever: it panics naming its wait message instead.
//!
//! ## Deviations
//! - `r_traceidx` and its `dt(4)` tracepoints are not configured (`refcnt_init_trace` is
//!   [`Refcnt::new`]).
//!
//! ## Redesign
//! - LZ's `struct refcnt` and the free functions over `&Refcnt` in `kern_synch.rs` are methods
//!   of one type; the count is an atomic, so the type needs no `Cell` and is `Sync` by its
//!   parts.
//! - Taking a reference on an object whose count reached 0 (the C's `KASSERT(refs > 1)`
//!   after the increment) and dropping one from 0 (`KASSERT(refs != ~0)`) panic in every
//!   configuration: the type cannot tell its users' objects freed, so a wrapped count is
//!   reported where it happens. `Refcnt` frees nothing itself; the owning handles that free
//!   at zero ([`crate::pool::PoolArc`]) keep a count of their own.

use core::sync::atomic::{AtomicU32, Ordering, fence};

use crate::assert::hal_panic;
use crate::bridges;
use crate::sleep::{self, INFSLP, PWAIT, chan};

/// `struct refcnt`: a reference count.
pub struct Refcnt {
    /// `r_refs`.
    refs: AtomicU32,
}

impl Refcnt {
    /// `REFCNT_INITIALIZER()` / `refcnt_init`: one reference, the creator's.
    pub const fn new() -> Self {
        Self {
            refs: AtomicU32::new(1),
        }
    }

    /// `refcnt_take`: one more reference to an object already referenced.
    pub fn take(&self) {
        let refs = self.refs.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        if refs <= 1 {
            hal_panic(format_args!(
                "refcnt_take: count was {}",
                refs.wrapping_sub(1)
            ));
        }
    }

    /// `refcnt_rele`: one reference fewer; `true` when it was the last, after which the
    /// caller owns the object alone and sees every write the others made.
    pub fn rele(&self) -> bool {
        // membar_exit_before_atomic()
        let refs = self.refs.fetch_sub(1, Ordering::Release).wrapping_sub(1);
        if refs == u32::MAX {
            hal_panic(format_args!("refcnt_rele: count was 0"));
        }
        if refs == 0 {
            fence(Ordering::Acquire); // membar_enter_after_atomic()
            return true;
        }
        false
    }

    /// `refcnt_rele_wake`: one reference fewer, waking the finalizer when it was the last.
    pub fn rele_wake(&self) {
        if self.rele() {
            sleep::wakeup_one(core::ptr::from_ref(self));
        }
    }

    /// `refcnt_finalize`: drops the caller's reference, then sleeps (`PWAIT`, `wmesg`) until
    /// every other one is gone.
    pub fn finalize(&self, wmesg: &'static str) {
        // membar_exit_before_atomic()
        let mut refs = self.refs.fetch_sub(1, Ordering::Release).wrapping_sub(1);
        if refs == u32::MAX {
            hal_panic(format_args!("refcnt_finalize: count was 0"));
        }
        if refs != 0 {
            let Some(b) = bridges::sleep() else {
                hal_panic(format_args!(
                    "refcnt_finalize: {wmesg}: {refs} references left before the sleep queues exist"
                ));
            };
            // Go on the queue first, then look: a last `rele_wake` between the two finds us
            // queued, so its wakeup is not lost.
            while refs != 0 {
                b.sleep_setup(chan(core::ptr::from_ref(self)), PWAIT, wmesg);
                refs = self.refs.load(Ordering::Relaxed);
                let _ = b.sleep_finish(INFSLP, refs != 0);
            }
        }
        // What the caller does with the object from here must not move before the load that
        // saw the count at 0.
        fence(Ordering::SeqCst); // membar_sync()
    }

    /// `refcnt_read`: the count now (a snapshot).
    pub fn read(&self) -> u32 {
        self.refs.load(Ordering::Relaxed)
    }

    /// `refcnt_shared`: whether more than one reference is held (a snapshot).
    pub fn shared(&self) -> bool {
        self.read() > 1
    }
}

impl Default for Refcnt {
    fn default() -> Self {
        Self::new()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::testutil;

    #[test]
    fn take_and_rele_count_and_the_last_rele_says_so() {
        let r = Refcnt::new();
        assert_eq!(r.read(), 1);
        assert!(!r.shared());
        r.take();
        assert!(r.shared());
        assert!(!r.rele());
        assert!(r.rele());
        assert_eq!(r.read(), 0);
    }

    #[test]
    #[should_panic(expected = "refcnt_take: count was 0")]
    fn taking_a_dead_count_panics() {
        let r = Refcnt::new();
        assert!(r.rele());
        r.take();
    }

    #[test]
    #[should_panic(expected = "refcnt_rele: count was 0")]
    fn releasing_a_dead_count_panics() {
        let r = Refcnt::new();
        assert!(r.rele());
        r.rele();
    }

    #[test]
    fn finalize_waits_for_the_other_references() {
        testutil::install();
        let r = Arc::new(Refcnt::new());
        r.take();
        let r2 = Arc::clone(&r);
        let holder = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            r2.rele_wake();
        });
        r.finalize("testfin");
        assert_eq!(r.read(), 0);
        holder.join().unwrap();
    }
}
/* </TESTS> */
