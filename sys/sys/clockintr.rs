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
 * Copyright (c) 2020-2024 Scott Cheloha <cheloha@openbsd.org>
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
//! `<sys/clockintr.h>`: schedulable clock interrupts and the per-CPU queue that dispatches
//! them, at its LZ path.
//!
//! Upstream: sys/sys/clockintr.h @ 3ce1f3f79392
//! LZ: sys/sys/clockintr.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.8a: the structures and constants are `kernel::clockintr`'s (a `struct clockintr` a
//!   `Copy` handle of a CPU's slot, `struct clockqueue` a `PerCpu<ClockCpu>` out of
//!   `cpu_info`, `struct intrclock` the `Intrclock` trait), re-exported here for `bsd`'s
//!   binders and clock drivers. 0 `unsafe` (LZ 2).

pub use kernel::clockintr::{
    CL_BARRIER, CL_FLAG_MASK, CQ_IGNORE_REQUEST, CQ_INIT, CQ_INTRCLOCK, CQ_NEED_WAKEUP,
    CQ_STATE_MASK, CR_RESCHEDULE, Clockintr, ClockintrFn, ClockintrStat, Clockrequest, Intrclock,
    Slot,
};
/* </CODE> */
