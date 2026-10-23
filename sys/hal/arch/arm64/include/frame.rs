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
//! arm64 `<machine/frame.h>`: the stack frames the kernel walks and builds, as the context
//! switch (`hal::thread`) and the exception entry need them.
//!
//! Upstream: sys/arch/arm64/include/frame.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/frame.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.5a: `struct callframe`, `struct trapframe` and `struct switchframe` with their sizes
//!   move from `bsd`'s header, which keeps `struct sigframe` (it names the signal context and
//!   `siginfo`, `bsd`'s until N4.5b) and re-exports these: the switch and `cpu_fork` are
//!   `hal`'s and build the frames.
//! - N4.8b: `struct sigframe` follows (`hal::sig`'s `sendsig` builds it), with the signal
//!   context (`signal.rs`); LZ's `unsafe impl AbiPod` is `hal::abi_pod!`.

use super::signal::Sigcontext;
use crate::sys::siginfo::Siginfo;
use crate::sys::types::Register;

/// `struct callframe`: one link of the frame-pointer chain.
#[repr(C)]
pub struct Callframe {
    /// The caller's frame (its saved `x29`).
    pub f_frame: *const Callframe,
    /// The saved link register: the return address into the caller.
    pub f_lr: Register,
}

const _: () = {
    assert!(core::mem::size_of::<Callframe>() == 16);
    assert!(core::mem::offset_of!(Callframe, f_lr) == 8);
};

/// `struct trapframe` (`trapframe_t`): what `exception.S` saves on entry.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Trapframe {
    /// `tf_sp`.
    pub tf_sp: Register,
    /// `tf_lr`.
    pub tf_lr: Register,
    /// `tf_elr`: the exception link register (the PC).
    pub tf_elr: Register,
    /// `tf_spsr`: the saved program status.
    pub tf_spsr: Register,
    /// `tf_x`: x0 to x29.
    pub tf_x: [Register; 30],
}

impl Trapframe {
    /// An empty frame.
    pub const fn new() -> Self {
        Self {
            tf_sp: 0,
            tf_lr: 0,
            tf_elr: 0,
            tf_spsr: 0,
            tf_x: [0; 30],
        }
    }
}

impl Default for Trapframe {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct switchframe`: stack frame inside `cpu_switch()`: the callee-saved registers
/// `cpu_switchto_asm` stores, `x19` to `x29` and `lr`. `cpu_fork` builds one so a new
/// thread's first switch returns into `proc_trampoline`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Switchframe {
    /// `sf_x19`.
    pub sf_x19: Register,
    /// `sf_x20`.
    pub sf_x20: Register,
    /// `sf_x21`.
    pub sf_x21: Register,
    /// `sf_x22`.
    pub sf_x22: Register,
    /// `sf_x23`.
    pub sf_x23: Register,
    /// `sf_x24`.
    pub sf_x24: Register,
    /// `sf_x25`.
    pub sf_x25: Register,
    /// `sf_x26`.
    pub sf_x26: Register,
    /// `sf_x27`.
    pub sf_x27: Register,
    /// `sf_x28`.
    pub sf_x28: Register,
    /// `sf_x29`.
    pub sf_x29: Register,
    /// `sf_lr`.
    pub sf_lr: Register,
}

/// `struct sigframe`: what `sendsig` builds on the user stack for `sigcode`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl Sigframe {
    /// An all-zero frame (`bzero(&frame, sizeof(frame))`).
    pub const fn zeroed() -> Self {
        Self {
            sf_signum: 0,
            _pad: 0,
            sf_sc: Sigcontext {
                __sc_unused: 0,
                sc_mask: 0,
                sc_sp: 0,
                sc_lr: 0,
                sc_elr: 0,
                sc_spsr: 0,
                sc_x: [0; 30],
                sc_cookie: 0,
            },
            sf_si: Siginfo::zeroed(),
        }
    }
}

crate::abi_pod!(Sigframe {
    sf_signum: i32,
    _pad: i32,
    sf_sc: Sigcontext,
    sf_si: Siginfo,
});

/// `TF_SIZE`: the size of a trap frame.
pub const TF_SIZE: usize = size_of::<Trapframe>();

/// `SWITCHFRAME_SZ`: the size of a switch frame.
pub const SWITCHFRAME_SZ: usize = size_of::<Switchframe>();

const _: () = {
    assert!(TF_SIZE == 34 * 8);
    assert!(core::mem::offset_of!(Trapframe, tf_x) == 32);
    assert!(SWITCHFRAME_SZ == 12 * 8);
    assert!(core::mem::offset_of!(Sigframe, sf_sc) == 8);
    assert!(size_of::<Sigframe>() == 8 + size_of::<Sigcontext>() + size_of::<Siginfo>());
};
/* </CODE> */
