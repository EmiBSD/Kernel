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
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993, The Regents of the University of California.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * The Mach Operating System project at Carnegie-Mellon University.
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
 *	@(#)vm_pageout.c        8.5 (Berkeley) 2/14/94
 * from: Id: uvm_pdaemon.c,v 1.1.2.32 1998/02/06 05:26:30 chs Exp
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
 *
 * Permission to use, copy, modify and distribute this software and
 * its documentation is hereby granted, provided that both the copyright
 * notice and this permission notice appear in all copies of the
 * software, derivative works or modified versions, and any portions
 * thereof, and that both notices appear in supporting documentation.
 *
 * CARNEGIE MELLON ALLOWS FREE USE OF THIS SOFTWARE IN ITS "AS IS"
 * CONDITION.  CARNEGIE MELLON DISCLAIMS ANY LIABILITY OF ANY KIND
 * FOR ANY DAMAGES WHATSOEVER RESULTING FROM THE USE OF THIS SOFTWARE.
 *
 * Carnegie Mellon requests users of this software to return to
 *
 *  Software Distribution Coordinator  or  Software.Distribution@CS.CMU.EDU
 *  School of Computer Science
 *  Carnegie Mellon University
 *  Pittsburgh PA 15213-3890
 *
 * any improvements or extensions that they make and grant Carnegie the
 * rights to redistribute these changes.
 */
/* </LICENSES> */

/* <CODE> */
//! The page daemon: `uvm/uvm_pdaemon.c`.
//!
//! Upstream: sys/uvm/uvm_pdaemon.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_pdaemon.rs@3c62ede68802
//!
//! Status: `wip` (LZ's). Only what the allocator and `main` name: [`uvm_wait`], which
//! reports the gap, because there is no daemon to wait for; the daemon itself
//! (`uvm_pageout`, `uvmpd_scan`, the aiodone daemon) arrives with swapping, which LZ has not
//! ported. [`uvmpd_tune`] is ported: `uvm_pageout` calls it first thing, and since the daemon
//! thread is not created, `main` calls it where the C forks the daemon, so `wiredmax`,
//! `freemin` and `freetarg` have their values (`sysctl(2)` and `mlock(2)` compare against
//! `wiredmax`).
//!
//! ## Deviations
//! - `uvmpd_tune` runs from `main` instead of from the page daemon's first loop (LZ's).
//! - No page daemon (`uvm_pageout`, `uvmpd_scan`, `uvm_aiodone_daemon`): `uvm_wait` reports
//!   `unported` once (`vm::pmemrange::uvm_wait`), kept visible.
//!
//! ## Redesign
//! - LZ's `bsd` module moved into the kernel crate whole (N3.5 S1, the N3 exit with the
//!   daemon a visible stub, the main session's decision of 2026-10-18); nothing else changed.

use core::sync::atomic::Ordering;

use hal::arch::current::param::PAGE_SHIFT;
use hal::vm::uvmexp::UVMEXP;

/// `uvm_wait`: wait for the page daemon to free memory; nothing can be waited for yet. The
/// page allocator's (`vm::pmemrange::uvm_wait`), which reports the gap once for every
/// waiter.
pub fn uvm_wait(wmsg: &str) {
    super::pmemrange::uvm_wait(wmsg);
}

/// `uvmpd_tune`: tune paging parameters.
pub fn uvmpd_tune() {
    let npages = UVMEXP.npages.load(Ordering::Relaxed);
    let reserve_kernel = UVMEXP.reserve_kernel.load(Ordering::Relaxed);
    let (freemin, freetarg, wiredmax) = tune(npages, reserve_kernel);
    UVMEXP.freemin.store(freemin, Ordering::Relaxed);
    UVMEXP.freetarg.store(freetarg, Ordering::Relaxed);
    UVMEXP.wiredmax.store(wiredmax, Ordering::Relaxed);
}

/// `uvmpd_tune`'s arithmetic: `freemin`, `freetarg` and `wiredmax` for `npages` pages and
/// `reserve_kernel` reserved ones.
fn tune(npages: i32, reserve_kernel: i32) -> (i32, i32, i32) {
    let mut val = npages / 30;

    // XXX: what are these values good for?
    val = val.max((16 * 1024) >> PAGE_SHIFT);

    // Make sure there's always a user page free.
    if val < reserve_kernel + 1 {
        val = reserve_kernel + 1;
    }
    let freemin = val;

    // Calculate free target.
    let mut val = (freemin * 4) / 3;
    if val <= freemin {
        val = freemin + 1;
    }
    let freetarg = val;

    (freemin, freetarg, npages / 3)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tuning_is_the_cs() {
        // 1 GiB of 4 KiB pages.
        assert_eq!(tune(262_144, 100), (8738, 11650, 87381));
        // A small machine: the 16 KiB floor, then the reserve.
        assert_eq!(tune(30, 0), (4, 5, 10));
        assert_eq!(tune(30, 10), (11, 14, 10));
        // freetarg is always above freemin.
        assert_eq!(tune(0, 1), (4, 5, 0));
    }
}
/* </TESTS> */
