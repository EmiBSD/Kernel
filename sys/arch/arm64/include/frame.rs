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
 * Copyright (c) 2016 Dale Rahn <drahn@dalerahn.com>
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
//! arm64 `<machine/frame.h>`: the stack frames the kernel walks and builds.
//!
//! Upstream: sys/arch/arm64/include/frame.h @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M2 ports `struct callframe`, M4 `struct trapframe` (`clockframe` is the
//! same struct), M5 `struct switchframe`, `kern_sig.c` `struct sigframe` (with the hole after
//! `sf_signum` spelled out as `_pad`). M2's note: `struct callframe`, what the frame-pointer chain is made
//! of (`stp x29, x30, [sp, #-16]!` leaves the caller's frame and the link register at `x29`).
//! `struct trapframe` and `struct switchframe` arrive with the trap and context-switch code
//! (M4, M5). N4.5a: `callframe`, `trapframe` and `switchframe` are `hal`'s
//! (`hal::arch::arm64::include::frame`, the switch builds them), re-exported here; `sigframe`
//! stays. N4.8b: `sigframe` is `hal`'s too (`hal::sig`'s `sendsig` builds it), re-exported.

pub use hal::arch::arm64::include::frame::{
    Callframe, SWITCHFRAME_SZ, Sigframe, Switchframe, TF_SIZE, Trapframe,
};
/* </CODE> */
