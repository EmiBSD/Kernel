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
 * Copyright (c) 1980, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)netisr.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! Network software interrupt bits: `<net/netisr.h>`, and `if.c`'s `netisr` word and its
//! task (`if_netisr`).
//!
//! Upstream: sys/net/netisr.h @ 3ce1f3f79392
//! LZ: sys/net/netisr.rs@3c62ede68802
//!
//! Each "pup-level-1" input queue has a bit in a "netisr" status word which is used to
//! de-multiplex a single software interrupt used for scheduling the network code to calls on
//! the lowest level routine of each protocol. The constants are bit numbers.
//!
//! ## Deviations
//! - `schednetisr(anisr)` before `softnet_init` (no softnet task queue yet) sets the bit
//!   only; the C would dereference the NULL queue (LZ's).
//! - `NPPP`, `NBRIDGE` and `NPPPOE` are not configured: their bits are never served (LZ's).
//!
//! ## Redesign
//! - `netisr` and `if_input_task_locked` were `net/if.c`'s statics: the word is
//!   [`NETISR`] here, the task the glue's (`NetTables::if_input_task_locked`), since
//!   `if_netisr`'s yield is the scheduler's, generic over the process glue. `if_netisr` holds
//!   the net lock's write witness and lets it go only around a yield the scheduler asks for
//!   (`WriteWitness::unlocked`), as the C's `NET_UNLOCK(); yield(); NET_LOCK()`.

use core::ffi::c_void;
use core::pin::Pin;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::netlock::NETLOCK;

use crate::net::domain::net;
use crate::net::if_::{inet, net_tq};
use crate::proc::ProcGlue;
use crate::task::task_add;

/// Same as `AF_INET`.
pub const NETISR_IP: i32 = 2;
/// Same as `AF_LINK`.
pub const NETISR_ARP: i32 = 18;
/// Same as `AF_INET6`.
pub const NETISR_IPV6: i32 = 24;
/// For PPP processing.
pub const NETISR_PPP: i32 = 28;
/// For bridge processing.
pub const NETISR_BRIDGE: i32 = 29;
/// For pppoe processing.
pub const NETISR_PPPOE: i32 = 30;

/// `netisr`: the protocols' pending bits.
pub static NETISR: AtomicI32 = AtomicI32::new(0);

/// `schednetisr(anisr)`: marks protocol queue `anisr` (`NETISR_*`) for `if_netisr` and
/// queues that task on the first softnet task queue.
pub fn schednetisr<G: ProcGlue>(anisr: i32) {
    NETISR.fetch_or(1 << anisr, Ordering::Relaxed);
    if let Some(tq) = net_tq(0) {
        let _ = task_add(tq, Pin::static_ref(&net::<G>().if_input_task_locked));
    }
}

/// `if_netisr`: the protocols' queues whose bits are set, served under the exclusive net
/// lock, yielding between rounds when the scheduler asks.
pub(crate) fn if_netisr<G: ProcGlue>(_unused: *mut c_void) {
    let mut w = NETLOCK.write();

    loop {
        let n = NETISR.load(Ordering::Relaxed);
        if n == 0 {
            break;
        }

        // Like sched_pause() but with a rwlock dance.
        if crate::sched::should_yield::<G>() {
            w.unlocked(crate::sched::r#yield::<G>);
        }

        NETISR.fetch_and(!n, Ordering::Relaxed);

        if let Some(inet) = inet() {
            // NETHER > 0
            if n & (1 << NETISR_ARP) != 0 {
                inet.arpintr();
            }
            if n & (1 << NETISR_IP) != 0 {
                inet.ipintr();
            }
            if n & (1 << NETISR_IPV6) != 0 {
                inet.ip6intr();
            }
        }
        // NPPP > 0: NETISR_PPP runs pppintr() under the kernel lock; ppp(4) is not configured.
        // NBRIDGE > 0: NETISR_BRIDGE runs bridgeintr(); bridge(4) is not configured.
        // NPPPOE > 0: NETISR_PPPOE runs pppoeintr(); pppoe(4) is not configured.
        // (t |= n: the C accumulates the bits it ran and never reads them.)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/net/netisr.h");
        let ours = crate::reftest::assert_defines!(defs;
            NETISR_IP, NETISR_ARP, NETISR_IPV6, NETISR_PPP, NETISR_BRIDGE, NETISR_PPPOE);
        crate::reftest::assert_complete(&defs, "NETISR_", &ours);
    }
}
/* </TESTS> */
