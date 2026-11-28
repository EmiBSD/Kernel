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
//! The ARM generic timer of the executing CPU, for the kernel's `agtimer(4)`: the virtual
//! counter and its readers (around the Cortex-A73 and Allwinner A64 errata), its frequency,
//! the virtual timer's control and countdown, the kernel's counter control and the running
//! exception level, each access one function of arm64's `arch/arm64/timer.rs`; and the
//! `tc_user` values of `<machine/timetc.h>` the counter's timecounter hands to userland. On
//! the host, one register file per test thread (`arch/host/timer.rs`), which a test reads
//! and sets ([`regs`], [`set_regs`]). arm64 and the host only: amd64's clocks are N10's.

#[cfg(target_os = "none")]
pub use crate::arch::current::include::timetc::{TC_AGTIMER, TC_AGTIMER_SUN50I};
#[cfg(not(target_os = "none"))]
pub use crate::arch::current::timer::{TimerRegs, regs, set_regs};
pub use crate::arch::current::timer::{
    cntfrq, cntkctl, cntv_ctl, current_el, readcnt64, readcnt64_default, readcnt64_sun50i,
    set_cntkctl, set_cntv_ctl, set_cntv_tval, set_readcnt64_sun50i, timecount,
};
#[cfg(not(target_os = "none"))]
pub use crate::arch::current::timetc::{TC_AGTIMER, TC_AGTIMER_SUN50I};
/* </CODE> */
