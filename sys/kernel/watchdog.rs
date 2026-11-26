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
 * Copyright (c) 2003 Markus Friedl.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! The watchdog framework: `kern/kern_watchdog.c`, with the sysctl node `kern.watchdog`. One
//! hardware watchdog driver registers its control callback ([`wdog_register`]);
//! `sysctl kern.watchdog.period` arms it (the callback returns the period it really set) and,
//! while `kern.watchdog.auto` is on, a timeout re-arms it every half period, so that the
//! machine resets only when the kernel stops running timeouts. [`wdog_shutdown`] disarms it
//! when its driver powers down.
//!
//! Upstream: sys/kern/kern_watchdog.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_watchdog.rs@e8249449d905
//!
//! ## Deviations
//! - `name` is the slice after `KERN_WATCHDOG`, as `kern_sysctl_dirs_locked` hands it; an
//!   empty one is `EINVAL` (the C reads `name[0]` unchecked) (LZ's).
//! - The timeout's period in milliseconds is computed in 64 bits, so a period above
//!   `INT_MAX / 1000` seconds does not overflow as the C's `int` product would (LZ's).
//!
//! ## Redesign
//! - LZ's callback in a `StaticCell` (three `unsafe` blocks whose comment names the kernel
//!   lock) beside an `AtomicPtr` argument -> the pair in one [`Mutexed`] at `IPL_SOFTCLOCK`
//!   (the tickle runs from softclock), copied out before the call, so the callback never
//!   runs under the lock and a registration is seen whole.
//! - The callback is the legacy driver's `fn(*mut c_void, i32) -> i32` and its argument a
//!   word (the pointer's exposed address): the kernel keeps the word, hands it back as a
//!   pointer with `core::ptr::with_exposed_provenance_mut` and never reads through it
//!   (`docs/IDIOMS.md`, a legacy callback's `void *`). [`wdog_shutdown`] compares addresses
//!   only. The typed form comes with the first native caller (ipmi, N7.12).
//! - `wdog_timeout` is a kernel [`Timeout`] static, pinned; `wdog_period` and `wdog_auto`
//!   are private atomics, `kern.watchdog` the kernel's [`WatchdogNode`] (LZ's
//!   `sysctl_wdog` behind `kern_sysctl.rs`'s wrapper).

use core::ffi::c_void;
use core::pin::Pin;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::intr::IPL_SOFTCLOCK;
use hal::mutex::Mutexed;
use hal::pin::pin_static;
use hal::sys::errno::Errno;
use hal::sys::sysctl::{KERN_WATCHDOG_AUTO, KERN_WATCHDOG_PERIOD};

use crate::sysctl::{SysctlNode, SysctlReq, int_bounded};
use crate::timeout::Timeout;

/// `int (*)(void *, int)`: a watchdog driver's control callback. It sets the period (in
/// seconds, 0 disarms) and returns the period it really set.
pub type WdogCtlFn = fn(*mut c_void, i32) -> i32;

/// `wdog_ctl_cb` and `wdog_ctl_cb_arg` (the argument's exposed address): the registered
/// driver's callback, `None` for none.
static WDOG_CTL: Mutexed<Option<(WdogCtlFn, usize)>> = Mutexed::new(IPL_SOFTCLOCK, None);
/// `wdog_period`: the period in seconds the driver accepted; 0 when disarmed.
static WDOG_PERIOD: AtomicI32 = AtomicI32::new(0);
/// `wdog_auto`: re-arm the watchdog from a timeout.
static WDOG_AUTO: AtomicI32 = AtomicI32::new(1);
/// `wdog_timeout`: [`wdog_tickle`], set up at the first registration.
static WDOG_TIMEOUT: Timeout = Timeout::new(wdog_tickle, core::ptr::null_mut());

/// `wdog_timeout`, pinned.
fn wdog_timeout() -> Pin<&'static Timeout> {
    pin_static(&WDOG_TIMEOUT)
}

/// The registered callback and its argument.
fn wdog_ctl_cb() -> Option<(WdogCtlFn, usize)> {
    *WDOG_CTL.lock()
}

/// `(*wdog_ctl_cb)(wdog_ctl_cb_arg, period)`; `None` when no driver is registered.
fn wdog_ctl(period: i32) -> Option<i32> {
    let (cb, arg) = wdog_ctl_cb()?;
    Some(cb(core::ptr::with_exposed_provenance_mut(arg), period))
}

/// Half of `period` seconds, in milliseconds (`period * 1000 / 2`).
fn wdog_half_period_msec(period: i32) -> u64 {
    (i64::from(period) * 1000 / 2).max(0) as u64
}

/// `wdog_register(cb, cb_arg)`: makes `cb` the watchdog, with `cb_arg` the exposed address
/// (`expose_provenance`) of what it is called with. The first driver to register keeps it.
pub fn wdog_register(cb: WdogCtlFn, cb_arg: usize) {
    let mut g = WDOG_CTL.lock();
    if g.is_none() {
        *g = Some((cb, cb_arg));
    }
}

/// `wdog_tickle(arg)`: re-arms the watchdog for the current period and comes back after half
/// of it.
pub fn wdog_tickle(_arg: *mut c_void) {
    if wdog_ctl(WDOG_PERIOD.load(Ordering::Relaxed)).is_none() {
        return;
    }
    let _ = wdog_timeout().add_msec(wdog_half_period_msec(WDOG_PERIOD.load(Ordering::Relaxed)));
}

/// `wdog_shutdown(arg)`: disarms and unregisters the watchdog if `arg` is the address it was
/// registered with (a driver passes its device, which begins its softc).
pub fn wdog_shutdown(arg: usize) {
    match wdog_ctl_cb() {
        Some((_, a)) if a == arg => {}
        _ => return,
    }
    let _ = WDOG_TIMEOUT.del();
    let _ = wdog_ctl(0);
    *WDOG_CTL.lock() = None;
    WDOG_PERIOD.store(0, Ordering::Relaxed);
    WDOG_AUTO.store(1, Ordering::Relaxed);
}

/// `sysctl_wdog`: `kern.watchdog.period` and `kern.watchdog.auto`. `EOPNOTSUPP` while no
/// watchdog is registered.
fn sysctl_wdog(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    if wdog_ctl_cb().is_none() {
        return Err(Errno::EOPNOTSUPP);
    }

    match name.first().copied() {
        Some(KERN_WATCHDOG_PERIOD) => {
            let period = AtomicI32::new(WDOG_PERIOD.load(Ordering::Relaxed));
            int_bounded(req, &period, 0, i32::MAX)?;
            if req.newp != 0 {
                let _ = WDOG_TIMEOUT.del();
                if let Some(set) = wdog_ctl(period.into_inner()) {
                    WDOG_PERIOD.store(set, Ordering::Relaxed);
                }
            }
        }
        Some(KERN_WATCHDOG_AUTO) => int_bounded(req, &WDOG_AUTO, 0, 1)?,
        _ => return Err(Errno::EINVAL),
    }

    let period = WDOG_PERIOD.load(Ordering::Relaxed);
    if WDOG_AUTO.load(Ordering::Relaxed) != 0 && period > 0 {
        let _ = wdog_ctl(period);
        let _ = wdog_timeout().add_msec(wdog_half_period_msec(period));
    } else {
        let _ = WDOG_TIMEOUT.del();
    }

    Ok(())
}

/// `kern.watchdog` (`KERN_WATCHDOG`): `sysctl_wdog`.
pub struct WatchdogNode;

impl SysctlNode for WatchdogNode {
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        sysctl_wdog(name, req)
    }
}

/// Forgets the registered driver and the settings, the timeout stopped, for the host tests
/// (`testing.md`).
#[cfg(any(test, feature = "doubles"))]
pub fn wdog_test_reset() {
    let _ = WDOG_TIMEOUT.del();
    *WDOG_CTL.lock() = None;
    WDOG_PERIOD.store(0, Ordering::Relaxed);
    WDOG_AUTO.store(1, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The registration, kern.watchdog's two leaves and their refusals as the C's, the tickle,
    // the shutdown by argument.

    use std::sync::Mutex;
    use std::vec::Vec;

    use hal::user::UserPtr;

    use super::*;
    use crate::testutil;

    /// The periods the fake driver was asked for, with its argument, in order.
    static CALLS: Mutex<Vec<(usize, i32)>> = Mutex::new(Vec::new());

    /// A driver that rounds periods below 10 s up to 10, as ipmi(4) does.
    fn fake_cb(arg: *mut c_void, period: i32) -> i32 {
        CALLS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((arg.addr(), period));
        if period > 0 && period < 10 {
            10
        } else {
            period
        }
    }

    fn calls() -> Vec<(usize, i32)> {
        core::mem::take(&mut *CALLS.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// The calls were `want`, then any number of `tick`s: the armed timeout may run in
    /// another test's `tick_once` (the wheel is the process's).
    fn assert_calls(want: &[(usize, i32)], tick: (usize, i32)) {
        let got = calls();
        assert!(got.len() >= want.len(), "{got:?}");
        assert_eq!(&got[..want.len()], want);
        assert!(got[want.len()..].iter().all(|c| *c == tick), "{got:?}");
    }

    /// `kern.watchdog.<leaf>`: the old value read at `u`, `new` written first when given.
    fn sysctl(leaf: i32, u: usize, new: Option<i32>) -> Result<i32, Errno> {
        let newp = u + 0x10;
        if let Some(v) = new {
            UserPtr::<i32>::new(newp).copyout(&v).unwrap();
        }
        let mut len = 4;
        let mut q = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: if new.is_some() { newp } else { 0 },
            newlen: if new.is_some() { 4 } else { 0 },
        };
        WatchdogNode.sysctl(&[leaf], &mut q)?;
        UserPtr::<i32>::new(u).copyin()
    }

    /// One test, because the framework is a set of globals.
    #[test]
    fn register_arm_tickle_and_shutdown() {
        testutil::install();
        wdog_test_reset();
        calls();
        let u = testutil::USER_BASE + 0xD000;

        // Nothing registered.
        assert_eq!(
            sysctl(KERN_WATCHDOG_PERIOD, u, None),
            Err(Errno::EOPNOTSUPP)
        );

        let me = 0x1000usize;
        wdog_register(fake_cb, me);
        // A second driver does not replace the first.
        wdog_register(fake_cb, 0x2000);

        assert_eq!(sysctl(KERN_WATCHDOG_PERIOD, u, None), Ok(0));
        assert_eq!(sysctl(KERN_WATCHDOG_AUTO, u, None), Ok(1));
        assert!(calls().is_empty());
        assert!(!WDOG_TIMEOUT.pending());

        // Arm with 3 s: the driver rounds it to 10, then auto re-arms with that.
        assert_eq!(sysctl(KERN_WATCHDOG_PERIOD, u, Some(3)), Ok(0));
        assert_eq!(WDOG_PERIOD.load(Ordering::Relaxed), 10);
        assert_calls(&[(me, 3), (me, 10)], (me, 10));
        assert!(WDOG_TIMEOUT.pending());
        assert_eq!(sysctl(KERN_WATCHDOG_PERIOD, u, None), Ok(10));

        // The timeout's function re-arms with the current period.
        wdog_tickle(core::ptr::null_mut());
        assert_calls(&[(me, 10)], (me, 10));

        // Bad names and bounds (the period's are 0..INT_MAX, auto's 0..1).
        assert_eq!(sysctl(99, u, None), Err(Errno::EINVAL));
        assert_eq!(sysctl(KERN_WATCHDOG_AUTO, u, Some(2)), Err(Errno::EINVAL));
        assert_eq!(
            sysctl(KERN_WATCHDOG_PERIOD, u, Some(-1)),
            Err(Errno::EINVAL)
        );
        let mut len = 4;
        let mut q = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(WatchdogNode.sysctl(&[], &mut q), Err(Errno::EINVAL));
        assert_calls(&[], (me, 10));

        // auto off: the timeout stops, the period stays; the tickle is not rescheduled.
        assert_eq!(sysctl(KERN_WATCHDOG_AUTO, u, Some(0)), Ok(1));
        assert!(!WDOG_TIMEOUT.pending());
        assert_calls(&[], (me, 10));
        // A new period with auto off: the driver is set, no timeout.
        assert_eq!(sysctl(KERN_WATCHDOG_PERIOD, u, Some(20)), Ok(10));
        assert_eq!(calls(), [(me, 20)]);
        assert!(!WDOG_TIMEOUT.pending());

        // Another argument does not shut it down; the right one does.
        wdog_shutdown(0x2000);
        assert!(wdog_ctl_cb().is_some());
        wdog_shutdown(me);
        assert_eq!(calls(), [(me, 0)]);
        assert!(wdog_ctl_cb().is_none());
        assert_eq!(WDOG_PERIOD.load(Ordering::Relaxed), 0);
        assert_eq!(WDOG_AUTO.load(Ordering::Relaxed), 1);
        assert_eq!(
            sysctl(KERN_WATCHDOG_PERIOD, u, Some(5)),
            Err(Errno::EOPNOTSUPP)
        );
    }

    #[test]
    fn half_period_in_msec() {
        assert_eq!(wdog_half_period_msec(0), 0);
        assert_eq!(wdog_half_period_msec(30), 15_000);
        assert_eq!(wdog_half_period_msec(i32::MAX), 1_073_741_823_500);
    }
}
/* </TESTS> */
