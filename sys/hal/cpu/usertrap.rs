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
//! The way back to user mode, its machine-neutral face (decision 56): the exit gate, the
//! check made with the interrupts masked that no AST is pending before the return, and the
//! token it hands out when the way is clear.
//!
//! The C's `do_ast` (arm64 `exception.S`) is a loop in assembly: the interrupts masked, the
//! running thread's `md_astpending` tested and cleared, the interrupts restored to what they
//! were and `ast()` called while it was set, the return to user mode only once it was clear,
//! with the interrupts still masked to the `eret`. Here the loop is the kernel's
//! (`kernel::trap`), over [`ExitGate::pass`]: `hal` calls nobody, and the kernel gets a
//! [`UserExit`] only from a pass that found nothing pending, so a return to user mode is
//! taken only with the check made and the interrupts masked since (the hardware's return,
//! `hal`'s assembly, trusts both).
//!
//! The host has no user mode: the gate reads and clears the running test thread's
//! `md_astpending` as the machine does (`aston` sets it on both), so the kernel's loop runs
//! in its tests; there are no interrupts to mask.

use core::sync::atomic::Ordering;

use crate::arch::current::cpu as machine;

/// The leave token: the exit gate found no AST pending and left this CPU's interrupts
/// masked; what a return to user mode needs from its caller. Made by [`ExitGate::pass`]
/// alone. It carries the interrupt state the gate found first, which nothing reads: the
/// return restores the user's own from the trap frame.
#[repr(transparent)]
#[must_use = "a return to user mode is taken only with the token the gate handed out"]
#[derive(Debug)]
pub struct UserExit(u64);

/// An AST was pending: the gate cleared it and restored the interrupts; the caller handles
/// it (`ast()`) and passes the gate again.
#[derive(Debug, PartialEq, Eq)]
pub struct AstPending;

/// The exit gate of one return to user mode (`do_ast`'s `x19`: the interrupt state found at
/// its first pass, which every pending AST is handled under). One per return, kept across the
/// loop's passes; the thread may move to another CPU while it handles an AST, as the C's
/// `x19` does.
#[derive(Debug, Default)]
pub struct ExitGate(Option<u64>);

impl ExitGate {
    /// A gate not passed yet.
    pub const fn new() -> Self {
        Self(None)
    }

    /// `do_ast`'s test: this CPU's interrupts masked (the state found the first time kept),
    /// then the running thread's `md_astpending` read; clear (or no thread runs here), the
    /// way is open: [`UserExit`], the interrupts left masked. Set: cleared, the interrupts
    /// restored to the state the first pass found, and [`AstPending`].
    pub fn pass(&mut self) -> Result<UserExit, AstPending> {
        // Disable interrupts (`mrs x19, daif` the first time; `msr daifset, #3` every time).
        let found = machine::intr_disable();
        let saved = *self.0.get_or_insert(found);
        // Check for astpending, and clear it before enabling interrupts: a load, and a store
        // only when it was set, as the C (another CPU's `aston` meanwhile sets the same 1).
        let pending = crate::cpu::thread::with_current_slot(|slot| {
            slot.is_some_and(|slot| {
                let ast = slot.astpending();
                let set = ast.load(Ordering::Relaxed) != 0;
                if set {
                    ast.store(0, Ordering::Relaxed);
                }
                set
            })
        });
        if pending {
            // Restore interrupts.
            machine::intr_restore(saved);
            Err(AstPending)
        } else {
            Ok(UserExit(saved))
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::thread::tests::TestThread;
    use crate::cpu::thread::{MachineSlot, set_current_static};

    /// With no thread running, the gate opens: nothing can be pending for nobody (`do_ast`'s
    /// `cbz` on `ci_curproc`).
    #[test]
    fn no_thread_no_ast() {
        let mut g = ExitGate::new();
        assert!(g.pass().is_ok());
    }

    /// A posted AST is reported once and cleared; the next pass opens. The gate keeps the
    /// interrupt state of its first pass.
    #[test]
    fn a_pending_ast_is_reported_once_and_cleared() {
        let t: &'static TestThread = std::boxed::Box::leak(std::boxed::Box::new(TestThread {
            machine: MachineSlot::new(),
            n: 0,
        }));
        set_current_static(Some(crate::ds::pin::pin_static(t)));
        let mut g = ExitGate::new();
        assert!(g.pass().is_ok());
        t.machine.aston();
        assert_eq!(g.pass().map(|_| ()), Err(AstPending));
        assert_eq!(t.machine.astpending().load(Ordering::Relaxed), 0);
        let exit = g.pass();
        assert!(exit.is_ok());
        assert_eq!(
            g.0,
            Some(0),
            "the host's interrupt state, found at the first pass"
        );
        set_current_static::<TestThread>(None);
    }
}
/* </TESTS> */
