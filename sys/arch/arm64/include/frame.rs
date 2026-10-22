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
//! stays.

use crate::arch::arm64::include::signal::Sigcontext;
use crate::machine::copy::AbiPod;
use crate::sys::siginfo::Siginfo;
pub use hal::arch::arm64::include::frame::{
    Callframe, SWITCHFRAME_SZ, Switchframe, TF_SIZE, Trapframe,
};

/// `struct sigframe`: what `sendsig` builds on the user stack for `sigcode`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sigframe {
    /// `sf_signum`.
    pub sf_signum: i32,
    /// The C compiler's alignment hole before `sf_sc`.
    pub _pad: i32,
    /// `sf_sc`.
    pub sf_sc: Sigcontext,
    /// `sf_si`.
    pub sf_si: Siginfo,
}

// SAFETY: `repr(C)` of two `i32`s and two `AbiPod` structures whose sizes are multiples of 8:
// no implicit padding, every bit pattern valid.
unsafe impl AbiPod for Sigframe {}

const _: () = {
    assert!(core::mem::offset_of!(Sigframe, sf_sc) == 8);
    assert!(size_of::<Sigframe>() == 8 + size_of::<Sigcontext>() + size_of::<Siginfo>());
};
/* </CODE> */
