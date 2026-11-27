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
 * Copyright (c) 2015 Dale Rahn <drahn@dalerahn.com>
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
//! The kernel's side of the way back to user mode (decision 56): `ast.c`'s `ast()`, the work
//! done for a pending AST, and the loop that runs it until `hal`'s exit gate finds nothing
//! pending.
//!
//! Upstream: sys/arch/arm64/arm64/ast.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/ast.rs@3c62ede68802
//!
//! The C's `do_ast` (`exception.S`) is the loop, in assembly, around `ast()`: the interrupts
//! masked, `md_astpending` tested and cleared, the interrupts restored and `ast()` called
//! while it was set, the return to user mode taken only once it was clear and with the
//! interrupts still masked. Here `hal` keeps the test with the interrupts masked
//! (`hal::cpu::usertrap::ExitGate`) and the kernel runs the loop ([`user_exit`]): `hal` calls
//! nobody on the way out, and a return to user mode needs the gate's token
//! (`hal::cpu::usertrap::UserExit`), which only a pass that found nothing pending hands out.
//!
//! ## Redesign
//! - E1 (decision 56, the way out): a new thread's first return to user mode runs [`user_exit`]
//!   from its start (`kernel::proc::fork`'s `thread_start`), generic over the glue: no
//!   indirect call, where `hal`'s `syscall_return` ran `do_ast` and `hal`'s `ast()` called the
//!   kernel three times through `hal::cpu::trap::TrapHandler` (`refreshcreds`, `mi_ast`,
//!   `userret`). A return after a trap from user mode still runs `exception.S`'s `do_ast`
//!   until the trap moves to `entry` (E2), and its `ast()` asks the kernel once
//!   (`TrapHandler::ast`, which is [`ast`]).
//! - `ast()`'s `p->p_addr->u_pcb.pcb_tf = tf` is not here: the frame of a thread's user
//!   context is always at the top of its kernel stack, where `cpu_fork` built it and every
//!   entry from user mode saves it, and `pcb_tf` names that place already (`hal`'s trap code
//!   records it until E2; decision 56's P5).

use core::pin::Pin;
use core::sync::atomic::Ordering;

use hal::cpu::usertrap::{AstPending, ExitGate, UserExit};
use hal::vm::uvmexp::UVMEXP;

use crate::proc::{ProcGlue, Thread};

/// `ast(tf)`: the asynchronous system trap's work for the thread `p` returning to user mode:
/// its credentials refreshed, the counter, `mi_ast` (profiling, the preemption
/// `ci_want_resched` asks for), then `userret` (the signals).
pub fn ast<G: ProcGlue>(p: Pin<&Thread<G>>) {
    crate::syscall::refreshcreds(&p);
    UVMEXP.softs.fetch_add(1, Ordering::Relaxed);
    crate::syscall::mi_ast(p, hal::cpu::curcpu().want_resched() != 0);
    crate::sig::userret(p);
}

/// `do_ast`'s loop: [`ast`] for the running thread while the exit gate finds one pending,
/// then the gate's token (the interrupts masked, nothing pending) for the return to user
/// mode. A thread that sleeps or moves to another CPU in `ast` passes the gate again where it
/// resumes, as the C's loop does.
pub fn user_exit<G: ProcGlue>() -> UserExit {
    let mut gate = ExitGate::new();
    loop {
        match gate.pass() {
            Ok(exit) => return exit,
            // handle the ast
            Err(AstPending) => hal::cpu::thread::with_current::<Thread<G>, _>(ast::<G>),
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sched::tests::{SchedGlue, VCPUS, run_as, setup, thread};

    /// The tests here count `uvmexp.softs`, which only [`ast`] increments.
    fn softs() -> i64 {
        i64::from(UVMEXP.softs.load(Ordering::Relaxed))
    }

    /// Nothing pending: the gate opens at once and no AST runs.
    #[test]
    fn no_ast_pending_leaves_at_once() {
        let _g = setup();
        let me = thread(40, 50);
        run_as(VCPUS[0], me);
        let before = softs();
        let _exit = user_exit::<SchedGlue>();
        assert_eq!(softs(), before);
        hal::cpu::thread::set_current_static::<Thread<SchedGlue>>(None);
    }

    /// A pending AST runs once (`aston`, as `signotify` posts it), is cleared, and the loop
    /// leaves at the next pass.
    #[test]
    fn a_pending_ast_runs_once_then_the_gate_opens() {
        let _g = setup();
        let me = thread(41, 50);
        run_as(VCPUS[0], me);
        me.machine.aston();
        let before = softs();
        let _exit = user_exit::<SchedGlue>();
        assert_eq!(softs(), before + 1);
        let _exit = user_exit::<SchedGlue>();
        assert_eq!(softs(), before + 1, "the AST was cleared by the first pass");
        hal::cpu::thread::set_current_static::<Thread<SchedGlue>>(None);
    }
}
/* </TESTS> */
