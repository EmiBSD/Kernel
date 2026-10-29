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
//! The host's double of the running thread's user context: the signal frame (`hal::sig`)
//! and the registers `hal::trap::proc_pc`/`proc_stack` read. There is no user context, so
//! `sendsig` records what it was asked, per test thread, and fails when the test says so;
//! `sigreturn` answers what the test set (`EFAULT` by default: no context to read); the
//! program counter and stack pointer are what the test set (0 before). The frame's own logic
//! is arm64's, tested on the host with its headers (`arch/arm64/sig.rs`).

extern crate std;

use core::cell::{Cell, RefCell};
use std::vec::Vec;

use crate::sig::{SigDelivery, SigreturnError};
use crate::sys::errno::Errno;
use crate::sys::siginfo::Siginfo;
use crate::sys::signal::{Sig, Sigset};

/// A `sendsig` the double was asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sent {
    /// The handler.
    pub catcher: Sig,
    /// The signal.
    pub sig: i32,
    /// The mask the context would restore.
    pub mask: Sigset,
    /// The `siginfo` copied out, when asked.
    pub info: Option<Siginfo>,
    /// Whether the alternate stack was asked for.
    pub onstack: bool,
    /// The trampoline.
    pub sigcode: usize,
    /// The process's cookie.
    pub cookie: u64,
}

std::thread_local! {
    static SENT: RefCell<Vec<Sent>> = const { RefCell::new(Vec::new()) };
    static SENDSIG_FAILS: Cell<bool> = const { Cell::new(false) };
    static SIGRETURN: Cell<Result<Sigset, SigreturnError>> =
        const { Cell::new(Err(SigreturnError::Fault)) };
    static USER_REGS: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

/// `sendsig`: recorded; `EFAULT` when [`set_sendsig_fails`] said so (the frame could not be
/// written).
pub(crate) fn sendsig(d: &SigDelivery<'_>) -> Result<(), Errno> {
    SENT.with(|s| {
        s.borrow_mut().push(Sent {
            catcher: d.catcher,
            sig: d.sig,
            mask: d.mask,
            info: d.info.copied(),
            onstack: d.onstack,
            sigcode: d.sigcode,
            cookie: d.cookie,
        });
    });
    if SENDSIG_FAILS.with(Cell::get) {
        return Err(Errno::EFAULT);
    }
    Ok(())
}

/// `sigreturn`: what [`set_sigreturn`] set for this thread.
pub(crate) fn sigreturn(_scp: usize, _cookie: u64) -> Result<Sigset, SigreturnError> {
    SIGRETURN.with(Cell::get)
}

/// The `sendsig`s this test thread made since the last call, oldest first.
pub fn take_sent() -> Vec<Sent> {
    SENT.with(|s| core::mem::take(&mut *s.borrow_mut()))
}

/// Makes this test thread's `sendsig` fail (`true`) or succeed.
pub fn set_sendsig_fails(fails: bool) {
    SENDSIG_FAILS.with(|c| c.set(fails));
}

/// What this test thread's `sigreturn` answers.
pub fn set_sigreturn(r: Result<Sigset, SigreturnError>) {
    SIGRETURN.with(|c| c.set(r));
}

/// `PROC_PC(p)`: what [`set_user_regs`] set for this test thread (0 before).
pub(crate) fn proc_pc() -> usize {
    USER_REGS.with(Cell::get).0
}

/// `PROC_STACK(p)`: what [`set_user_regs`] set for this test thread (0 before).
pub(crate) fn proc_stack() -> usize {
    USER_REGS.with(Cell::get).1
}

/// `PROC_PC(p)` and `PROC_STACK(p)` at once.
pub(crate) fn proc_regs() -> (usize, usize) {
    USER_REGS.with(Cell::get)
}

/// The user program counter and stack pointer this test thread's "trap frame" holds
/// (`hal::trap::proc_pc`, `proc_stack`).
pub fn set_user_regs(pc: usize, sp: usize) {
    USER_REGS.with(|c| c.set((pc, sp)));
}
/* </CODE> */
