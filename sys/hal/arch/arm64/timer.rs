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
 * Copyright (c) 2011 Dale Rahn <drahn@openbsd.org>
 * Copyright (c) 2013 Patrick Wildt <patrick@blueri.se>
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
//! The ARM generic timer's registers, the half of `agtimer.c` that touches the hardware: the
//! virtual counter (`CNTVCT_EL0`, read around two errata), its frequency (`CNTFRQ_EL0`), the
//! virtual timer's control and countdown (`CNTV_CTL_EL0`, `CNTV_TVAL_EL0`), the kernel's
//! counter control (`CNTKCTL_EL1`) and the running exception level (`CurrentEL`).
//!
//! Upstream: sys/arch/arm64/dev/agtimer.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/agtimer.rs@3c62ede68802
//!
//! Every function reads or writes one system register of the calling CPU and touches no
//! memory; the driver half (attach, the timecounter, the interrupt clock, the clock
//! registration) stays `bsd`'s `agtimer.rs` until N7, over these.
//!
//! ## Redesign
//! - LZ's `agtimer.rs` wrote its `asm!` and `write_specialreg!` sites in the driver (13
//!   `unsafe` with the autoconf casts): the register accessors are here, each one `unsafe`
//!   block with its argument, and the driver calls safe functions.
//! - `agtimer_readcnt64` was a `StaticCell<fn() -> u64>` the attach rewrote for the Allwinner
//!   A64 erratum (two `unsafe` to read and write it): it is [`SUN50I`], a flag the attach sets
//!   once ([`set_readcnt64_sun50i`]), and [`readcnt64`] branches on it; the timecounter's
//!   `tc_get_timecount` swap is the same branch in [`timecount`].
//! - `agtimer_get_freq`, `agtimer_get_ctrl`, `agtimer_set_ctrl`, `agtimer_set_tval` keep
//!   their names in the driver as wrappers of [`cntfrq`], [`cntv_ctl`], [`set_cntv_ctl`] and
//!   [`set_cntv_tval`].

use core::arch::asm;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::arm64::include::frame::Trapframe;
use crate::assert::kassert;

/// `struct clockframe`: on arm64 the trap frame the exception entry saved (`hal::clock`
/// reads it through [`clkf_usermode`] and [`clkf_pc`]).
pub type ClockFrame = Trapframe;

/// `agtimer_readcnt64 == agtimer_readcnt64_sun50i`: the counter's low nine bits cannot be
/// trusted while the higher bits roll over (the Allwinner A64's erratum); set once by the
/// driver's attach, on the boot CPU, before the clocks start.
static SUN50I: AtomicBool = AtomicBool::new(false);

/// `agtimer_readcnt64_default`: `CNTVCT_EL0`, read twice around the Cortex-A73 erratum
/// 858921 (a one-cycle window where the low 32 bits are old and the high 32 new when the low
/// half rolls over).
pub fn readcnt64_default() -> u64 {
    let (val0, val1): (u64, u64);
    // SAFETY: `isb` and two reads of the virtual counter, which EL1 may always read; no memory
    // and no flags change. The `memory` clobber is kept (no `nomem`), as the C's, so that the
    // compiler does not move loads across the barrier.
    unsafe {
        asm!(
            "isb",
            "mrs {0}, cntvct_el0",
            "mrs {1}, cntvct_el0",
            out(reg) val0,
            out(reg) val1,
            options(nostack, preserves_flags)
        )
    };
    if (val0 ^ val1) & 0x1_0000_0000 != 0 {
        val0
    } else {
        val1
    }
}

/// `agtimer_readcnt64_sun50i`: the counter read until its low nine bits are not at a roll-over
/// (the Allwinner A64's erratum), at most 150 times.
pub fn readcnt64_sun50i() -> u64 {
    let mut val: u64 = 0;
    let mut retry = 0;
    // SAFETY: an instruction barrier; no memory, stack or flags touched beyond ordering.
    unsafe { asm!("isb", options(nostack, preserves_flags)) };
    while retry < 150 {
        // SAFETY: a read of the virtual counter, which EL1 may always read; nothing else.
        unsafe {
            asm!("mrs {}, cntvct_el0", out(reg) val, options(nomem, nostack, preserves_flags))
        };
        if (val.wrapping_add(1) & 0x1ff) > 1 {
            break;
        }
        retry += 1;
    }
    kassert!(retry < 150);

    val
}

/// `agtimer_readcnt64()`: the counter through the reader the attach chose.
#[inline]
pub fn readcnt64() -> u64 {
    if SUN50I.load(Ordering::Relaxed) {
        readcnt64_sun50i()
    } else {
        readcnt64_default()
    }
}

/// `agtimer_readcnt64 = agtimer_readcnt64_sun50i`: from now on the counter is read around the
/// Allwinner A64's erratum ([`readcnt64`], [`timecount`]).
pub fn set_readcnt64_sun50i() {
    SUN50I.store(true, Ordering::Relaxed);
}

/// `agtimer_get_timecount_default` (or `_sun50i` once the attach chose it): the counter's low
/// 32 bits, the timecounter's reading. No need to work around the Cortex-A73 erratum 858921
/// for the low half alone.
#[inline]
pub fn timecount() -> u32 {
    if SUN50I.load(Ordering::Relaxed) {
        return readcnt64_sun50i() as u32;
    }
    let val: u64;
    // SAFETY: as in `readcnt64_default`: a barrier and one counter read, no memory written.
    unsafe {
        asm!(
            "isb",
            "mrs {}, cntvct_el0",
            out(reg) val,
            options(nostack, preserves_flags)
        )
    };
    val as u32
}

/// `agtimer_get_freq`: `CNTFRQ_EL0`, the counter's frequency as the firmware set it.
#[inline]
pub fn cntfrq() -> u64 {
    let v: u64;
    // SAFETY: a read of a read-only (at EL1) system register; no memory, stack or flags.
    unsafe { asm!("mrs {}, cntfrq_el0", out(reg) v, options(nomem, nostack, preserves_flags)) };
    v
}

/// `agtimer_get_ctrl`: `CNTV_CTL_EL0`.
#[inline]
pub fn cntv_ctl() -> u32 {
    let v: u64;
    // SAFETY: a read of this CPU's virtual timer control; no memory, stack or flags.
    unsafe { asm!("mrs {}, cntv_ctl_el0", out(reg) v, options(nomem, nostack, preserves_flags)) };
    v as u32
}

/// `agtimer_set_ctrl`: writes `CNTV_CTL_EL0`, then `isb` so that it takes effect before the
/// next instruction.
#[inline]
pub fn set_cntv_ctl(val: u32) {
    // SAFETY: the virtual timer of this CPU is the kernel's to program (its interrupt is
    // established by the clock driver); the write changes no memory, and a fired timer only
    // raises the PPI the interrupt code handles.
    unsafe {
        asm!(
            "msr cntv_ctl_el0, {}",
            "isb",
            in(reg) u64::from(val),
            options(nostack, preserves_flags)
        )
    };
}

/// `agtimer_set_tval`: writes `CNTV_TVAL_EL0` (the countdown to the next interrupt), then
/// `isb`.
#[inline]
pub fn set_cntv_tval(val: u32) {
    // SAFETY: as `set_cntv_ctl`: this CPU's timer, no memory touched.
    unsafe {
        asm!(
            "msr cntv_tval_el0, {}",
            "isb",
            in(reg) u64::from(val),
            options(nostack, preserves_flags)
        )
    };
}

/// `CNTKCTL_EL1`: what EL0 may read of the counter and the event stream's settings.
#[inline]
pub fn cntkctl() -> u64 {
    let v: u64;
    // SAFETY: a read of this CPU's counter control; no memory, stack or flags.
    unsafe { asm!("mrs {}, cntkctl_el1", out(reg) v, options(nomem, nostack, preserves_flags)) };
    v
}

/// Writes `CNTKCTL_EL1`: `CNTKCTL_EL0VCTEN` lets userland read the virtual counter (the
/// timekeep page's `tk_user`), the event stream fields end `wfe` waits periodically.
#[inline]
pub fn set_cntkctl(v: u64) {
    // SAFETY: the counter control of this CPU is the kernel's; it grants EL0 a read-only view
    // of the counter or makes `wfe` return early, neither of which touches memory (every `wfe`
    // in the kernel sits in a loop that checks its condition again).
    unsafe { asm!("msr cntkctl_el1, {}", in(reg) v, options(nomem, nostack, preserves_flags)) };
}

/// `READ_SPECIALREG(CurrentEL)`: the running exception level, `CURRENTEL_EL_MASK` bits.
#[inline]
pub fn current_el() -> u64 {
    let v: u64;
    // SAFETY: a read of a read-only system register; no memory, stack or flags.
    unsafe { asm!("mrs {}, CurrentEL", out(reg) v, options(nomem, nostack, preserves_flags)) };
    v
}

/// `CLKF_USERMODE(frame)`: the exception link register is a user address (bit 63 clear).
#[inline]
pub fn clkf_usermode(frame: &ClockFrame) -> bool {
    (frame.tf_elr as u64) & (1u64 << 63) == 0
}

/// `CLKF_PC(frame)`: the exception link register.
#[inline]
pub fn clkf_pc(frame: &ClockFrame) -> usize {
    frame.tf_elr as usize
}
/* </CODE> */
