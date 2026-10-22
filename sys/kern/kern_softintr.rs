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
 * Copyright (c) 2021, 2025 Visa Hankala
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
//! Machine-independent soft interrupts: `kern/kern_softintr.c` (`__USE_MI_SOFTINTR`), the
//! names `bsd`'s drivers and machines call, over `hal::softintr` (N4.5c).
//!
//! Upstream: sys/kern/kern_softintr.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_softintr.rs@3c62ede68802
//!
//! ## Deviations
//! - The handle is a `hal::softintr::SoftHandle` (`Copy`) where the C passes `void *`.
//!
//! ## Redesign
//! - The queues, the handlers and `softintr_lock` are `hal::softintr`'s (the module's
//!   `## Redesign` says how they are owned); this module keeps the C's function names for
//!   `bsd`, and `softintr_disestablish`'s wait for a running handler (`sched_barrier`, the
//!   kernel scheduler's, between `hal`'s two steps).

use hal::softintr::SoftintrFn;

use crate::kern::kern_sched::{cpu_info_by_number, sched_barrier};
use crate::kern::subr_xxx::assertwaitok;

pub use hal::softintr::{
    SIF_MPSAFE, SIS_DYING, SIS_PENDING, SIS_RESTART, SoftHandle, SoftintrHand,
};

/// `softintr_init`: the queues (`hal::softintr::init`).
pub fn softintr_init() {
    hal::softintr::init();
}

/// `softintr_dispatch`: runs the handlers pending at `level`, from the soft interrupt stub.
pub fn softintr_dispatch(level: i32) {
    if let Ok(level) = usize::try_from(level) {
        hal::softintr::dispatch(level);
    }
}

/// `softintr_establish`: registers `func(arg)` to run at the soft level that serves `ipl`;
/// `None` when no memory can be had.
pub fn softintr_establish(
    ipl: i32,
    func: SoftintrFn,
    arg: *mut core::ffi::c_void,
) -> Option<SoftHandle> {
    hal::softintr::establish(ipl, func, arg)
}

/// `softintr_disestablish`: removes a handler, waiting for it if it is running elsewhere;
/// the handle is dead afterwards (a later use is `panic(9)`).
pub fn softintr_disestablish(sih: SoftHandle) {
    assertwaitok();
    if let Some(runner) = sih.disestablish() {
        sched_barrier(cpu_info_by_number(runner));
    }
    sih.release();
}

/// `softintr_schedule`: queues the handler (or marks it for a restart if it is running) and
/// raises the soft interrupt.
pub fn softintr_schedule(sih: SoftHandle) {
    sih.schedule();
}
/* </CODE> */
