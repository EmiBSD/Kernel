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
//! The signal frame of the running thread: `sendsig` builds a handler's frame on its user
//! stack and points its user context at the handler, `sigreturn` checks the context a
//! handler gives back and restores it. Both are the machine's (`sig_machdep.c`, the layouts
//! of `<machine/signal.h>` and `<machine/frame.h>`: arm64's in
//! `arch/arm64/sig.rs`); what decides them is the kernel's (`kernel::sig`: which signal,
//! the mask, the alternate stack, `sigexit` for a frame that cannot be written or a context
//! whose cookie is not the one written).
//!
//! On the host there is no user context: [`sendsig`] records what it was asked and
//! [`sigreturn`] answers what the test set (`arch/host/sig.rs`).
//!
//! ## Redesign
//! - Not a port: the C's `kern_sig.c` calls the machine's `sendsig(catcher, sig, mask,
//!   ksip, info, onstack)`, which reads `curproc`'s `p_sigstk`, `ps_sigcode` and
//!   `ps_sigcookie` itself. Here the kernel owns the process and `hal` the frame, so the
//!   kernel hands those members down in a [`SigDelivery`] (N4.8b; LZ: `ProcGlue::sendsig`
//!   over `bsd`'s `sig_machdep.rs`), and `sigreturn` hands back the mask it read for the
//!   kernel to install (`sys_sigreturn`'s machine-independent half, `kernel::sig`).

use crate::sys::errno::Errno;
use crate::sys::siginfo::Siginfo;
use crate::sys::signal::{Sig, Sigaltstack, Sigset};

/// The machine's signal frame (arm64's `sig_machdep.c`).
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::arch::arm64::sig as machine;
/// The host's double.
#[cfg(not(target_os = "none"))]
use crate::arch::host::sig as machine;

/// What `sendsig(catcher, sig, mask, ksip, info, onstack)` delivers to the running thread,
/// with the members of its process and thread the C reads from `curproc`.
#[derive(Clone, Copy, Debug)]
pub struct SigDelivery<'a> {
    /// `catcher`: the handler's user address.
    pub catcher: Sig,
    /// `sig`.
    pub sig: i32,
    /// `mask`: the signal mask the context restores when the handler returns.
    pub mask: Sigset,
    /// `ksip` when `info` (`SA_SIGINFO`): copied out beside the context, its address the
    /// handler's second argument.
    pub info: Option<&'a Siginfo>,
    /// `onstack`: the action asks for the alternate signal stack (`SA_ONSTACK`).
    pub onstack: bool,
    /// `p->p_sigstk`: the thread's alternate signal stack.
    pub sigstk: Sigaltstack,
    /// `pr->ps_sigcode`: the signal trampoline, where the handler returns.
    pub sigcode: usize,
    /// `pr->ps_sigcookie`: the context's cookie is its address XOR this.
    pub cookie: u64,
}

/// Why `sigreturn` refused the context it was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SigreturnError {
    /// The context could not be copied in: `EFAULT`.
    Fault,
    /// Its cookie is not the one `sendsig` wrote for that address (a forged or reused
    /// context): the caller kills the process (`sigexit(p, SIGILL)`).
    Cookie,
    /// It would return to a privileged mode or with interrupts masked: `EINVAL`.
    Mode,
}

/// `sendsig`: the handler's frame (the context, and the `siginfo` when asked) on the
/// running thread's user stack, or the top of its alternate stack when asked and not
/// already on it, and its user context pointed at `catcher` with the trampoline as the
/// return address. `Err` (the C's nonzero return) when the frame cannot be written, for which
/// the kernel kills the process.
pub fn sendsig(d: &SigDelivery<'_>) -> Result<(), Errno> {
    machine::sendsig(d)
}

/// `sys_sigreturn`'s machine half: the context at user address `scp` copied in, its cookie
/// checked against `scp ^ cookie` and then cleared in user memory (it is not reused), its
/// mode checked, and the running thread's user context restored from it. The signal mask it
/// carries is the answer, for the kernel to install.
pub fn sigreturn(scp: usize, cookie: u64) -> Result<Sigset, SigreturnError> {
    machine::sigreturn(scp, cookie)
}

/// The host double's record of a `sendsig` and its knobs.
#[cfg(not(target_os = "none"))]
pub use crate::arch::host::sig::{Sent, set_sendsig_fails, set_sigreturn, take_sent};
/* </CODE> */
