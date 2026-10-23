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
 * Copyright (c) 2000 Poul-Henning Kamp <phk@FreeBSD.org>
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
 * If we meet some day, and you think this stuff is worth it, you
 * can buy me a beer in return. Poul-Henning Kamp
 */
/* </LICENSES> */

/* <CODE> */
//! Timecounters: `kern/kern_tc.c`, at its LZ path for `bsd`'s callers, over the kernel
//! crate's `kernel::tc`.
//!
//! Upstream: sys/kern/kern_tc.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_tc.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.8a: the timecounters moved into `kernel::tc` (the `Timecounter` trait and `TcEntry`,
//!   the timehands as atomics behind the generation, `windup_mtx` a `Mutexed`, `tc_lock` a
//!   `hal` rwlock, the timekeep page through `TimekeepAtomic`). This path keeps the names
//!   `bsd`'s callers use, re-exported, and `sysctl_tc` with `kern_sysctl.c`'s arguments over
//!   the kernel's node. 0 `unsafe` (LZ 7).
//! - The drivers' `struct timecounter` (`sys/sys/timetc.rs`, the C's cells) stays theirs
//!   until each moves (agtimer(4) implements the kernel's trait itself): [`tc_init`] gives
//!   such a counter its `kernel::tc::TcEntry` from a fixed table ([`ENTRIES`], one per
//!   counter registered; a machine has at most five) and registers it, and
//!   [`tc_reset_quality`] and [`tc_set_frequency`] reach the entry through the counter's
//!   `tc_entry` link.

use hal::once::OnceStatic;
use kernel::tc::TcEntry;

use crate::sys::errno::Errno;
use crate::sys::timetc::Timecounter;

pub use kernel::tc::{
    NAPTIME, TC_LOCK, TC_TICK, TIME_SECOND, TIME_UPTIME, TIMESTEPWARNINGS, binboottime, binruntime,
    bintime, binuptime, getbinruntime, getbinuptime, getmicrotime, getmicrouptime, getnanotime,
    getnanouptime, getnsecruntime, getnsecuptime, gettime, getuptime, inittimecounter,
    microboottime, microtime, microuptime, nanoboottime, nanoruntime, nanotime, nanouptime,
    nsecuptime, tc_adjfreq, tc_adjtime, tc_getfrequency, tc_getprecision, tc_setclock,
    tc_setrealtimeclock, tc_ticktock, tc_windup, timecounter,
};

/// The kernel's records of the drivers' counters, one per `tc_init`.
static ENTRIES: [OnceStatic<TcEntry>; 8] = [const { OnceStatic::new() }; 8];

/// `tc_init`: registers a driver's counter with the kernel's timecounters (its quality,
/// precision and the switch to it are `kernel::tc::tc_init`'s).
pub fn tc_init(tc: &'static Timecounter) {
    for slot in &ENTRIES {
        if slot.set(TcEntry::new(tc)).is_ok() {
            if let Some(e) = slot.get() {
                tc.tc_entry.set(Some(e));
                kernel::tc::tc_init(e);
            }
            return;
        }
    }
    crate::kern::subr_prf::panic(format_args!("tc_init: more than 8 timecounters"));
}

/// `tc_reset_quality`: the kernel's, on the counter's record (a counter never registered
/// has none: nothing more to change).
pub fn tc_reset_quality(tc: &'static Timecounter, quality: i32) {
    tc.tc_quality.set(quality);
    if let Some(e) = tc.tc_entry.get() {
        kernel::tc::tc_reset_quality(e, quality);
    }
}

/// `tc->tc_frequency = freq` after `tc_init` (a driver's clock-rate change): the counter's
/// cell and the kernel's record, which the next windup scales with.
pub fn tc_set_frequency(tc: &'static Timecounter, freq: u64) {
    tc.tc_frequency.set(freq);
    if let Some(e) = tc.tc_entry.get() {
        e.set_frequency(freq);
    }
}

/// `sysctl_tc`: `kern.timecounter`, with `kern_sysctl.c`'s arguments (`kernel::tc::sysctl_tc`).
pub fn sysctl_tc(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let mut req = kernel::sysctl::SysctlReq {
        oldp,
        oldlenp,
        newp,
        newlen,
    };
    kernel::tc::sysctl_tc(name, &mut req)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Serialises `bsd`'s tests that step the clock or change the active counter (the kernel
    /// crate's own tests serialise in their own process).
    pub(crate) static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn kern_timecounter_answers_through_the_lz_arguments() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut len = 0;
        // A size query of kern.timecounter.tick: an int.
        assert_eq!(sysctl_tc(&[1], 0, &mut len, 0, 0), Ok(()));
        assert_eq!(len, 4);
        assert_eq!(sysctl_tc(&[1, 2], 0, &mut len, 0, 0), Err(Errno::ENOTDIR));
        let a = nsecuptime();
        assert!(nsecuptime() > a, "the readers are the kernel's");
    }
}
/* </TESTS> */
