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
 * Copyright (c) 2003 Dale Rahn <drahn@openbsd.org>
 * Copyright (c) 2020 Mark Kettenis <kettenis@openbsd.org>
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
//! Schedulable clock interrupts: `kern/kern_clockintr.c`, at its LZ path for `bsd`'s
//! callers, over the kernel crate's `kernel::clockintr`.
//!
//! Upstream: sys/kern/kern_clockintr.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_clockintr.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.8a: the queues, the dispatch and the binders' API moved into `kernel::clockintr`
//!   (one `ClockCpu` per CPU in a `PerCpu`, a clock interrupt a `(CPU, Slot)` handle, the
//!   `[m]` state inside the queue's `Mutexed`, `CL_BARRIER` sleeping). This path re-exports
//!   the names `bsd`'s binders and clock drivers call. 0 `unsafe` (LZ 7).

pub use kernel::clockintr::{
    clockintr_advance, clockintr_bind, clockintr_cancel, clockintr_cpu_init, clockintr_dispatch,
    clockintr_hardclock, clockintr_schedule, clockintr_stagger, clockintr_trigger,
    clockintr_unbind, clockqueue_init, clockqueue_intrclock_install, clockrequest_advance,
    clockrequest_advance_random, nsec_advance, sysctl_clockintr, uptime_check_counts,
};
/* </CODE> */
