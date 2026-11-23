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
 * Copyright (c) 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Ralph Campbell.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 * 4. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)sys_machdep.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! `sysarch(2)`: the machine-dependent operations of `<machine/sysarch.h>`.
//!
//! Upstream: sys/arch/arm64/arm64/sys_machdep.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/sys_machdep.rs@0925b8a94390
//!
//! arm64 has no operation: the C's `switch` on `op` has only its `default`, so every `op` is
//! `EINVAL` and `parms` is never read. A pledged process is refused first, before `op` is
//! looked at, through `pledge_fail(p, EINVAL, 0)`: no promise allows `sysarch` past that
//! point, and the "prot_exec" promise that lets the call reach this function at all is
//! `mi_syscall`'s test, before the row. `bsd`'s `sysent[]` row 165 (`kern_xxx.rs`) is a
//! forward here, `Legacy` in the ABI.
//!
//! ## Deviations
//! - None. `KTRACE` (not configured) is `pledge_fail`'s.
//!
//! ## Redesign
//! - Into the kernel crate (an `lz-sync` of LZ's port): generic over the process glue instead
//!   of `bsd`'s `Proc`, the arguments typed (`op`, `parms` as an address) instead of
//!   `SysArgs`, and `pledge_fail` through `ProcGlue` (`kern_pledge` is `bsd`'s until N8).
//! - No `match` on `op`: LZ kept the C's one-arm `switch` under a clippy allow; the one
//!   answer is written as such.
//! - LZ's `MachineSysarch` trait, its `machine::sys_sysarch` entry and its host double are
//!   not taken: the C links one `sys_sysarch` by name, not through a table, and the pledge
//!   test is the same text in both machines' C, so it is written once here, naming no
//!   architecture. amd64's `AMD64_IOPL` arm (`amd64_iopl`: `suser`, the securelevel test,
//!   the copyin of `struct amd64_iopl_args`, `PSL_IOPL` in the trap frame) is N10's
//!   (decision 44): LZ's amd64 file is kept frozen, not compiled, in
//!   `sys/arch/amd64/amd64/sys_machdep.rs`.

use core::sync::atomic::Ordering;

use hal::sys::errno::Errno;

use crate::proc::{PS_PLEDGE, ProcGlue, Thread};

/// `sys_sysarch` (`sys_machdep.c`): the machine-dependent operations of `sysarch(2)`. A
/// pledged process is refused through `pledge_fail` before `op` is looked at; arm64 has no
/// operation, so every `op` is `EINVAL` and `parms` is never read.
pub fn sys_sysarch<G: ProcGlue>(p: &Thread<G>, _op: i32, _parms: usize) -> Result<(), Errno> {
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0 {
        return Err(G::pledge_fail(p, Errno::EINVAL, 0));
    }
    Err(Errno::EINVAL)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // `sysarch(2)` on arm64: `EINVAL` for every op, whatever `parms` holds; a pledged process
    // goes through `pledge_fail` (the test glue records `pledge_fail <code>`) with code 0 and
    // gets the error it returns, `EINVAL`.

    use std::string::String;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use hal::pool::PoolBox;

    use super::*;
    use crate::proc::{ProcessRef, process_alloc, procinit, thread_alloc};
    use crate::testutil;

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// The ops tried: none of arm64's, amd64's `AMD64_IOPL` (2) among them, and the ends of
    /// `int`.
    const OPS: [i32; 7] = [0, 1, 2, 3, -1, i32::MAX, i32::MIN];

    /// The `parms` tried: NULL and an address nothing maps (never read).
    const PARMS: [usize; 2] = [0, 0xdead_b000];

    /// A thread of a new process `pid`, its process's flags clear.
    fn thread(pid: i32) -> (ProcessRef<Procs>, PoolBox<Thread<Procs>>) {
        std::thread_local! {
            static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        testutil::install();
        if !MADE.with(|m| m.replace(true)) {
            procinit::<Procs>(64, 64);
        }
        let pr = process_alloc::<Procs>(pid, ());
        let t = thread_alloc::<Procs>(&pr, ());
        (pr, t)
    }

    #[test]
    fn every_op_is_einval_without_pledge() {
        let (_pr, p) = thread(2);
        let _ = testutil::file_log();
        for op in OPS {
            for parms in PARMS {
                assert_eq!(
                    sys_sysarch(&p, op, parms),
                    Err(Errno::EINVAL),
                    "op {op} parms {parms:#x}"
                );
            }
        }
        assert!(
            testutil::file_log().is_empty(),
            "no pledge_fail without pledge"
        );
    }

    #[test]
    fn a_pledged_process_goes_through_pledge_fail() {
        let (_pr, p) = thread(3);
        let _ = testutil::file_log();
        p.process().ps_flags.fetch_or(PS_PLEDGE, Ordering::Relaxed);
        for op in OPS {
            for parms in PARMS {
                assert_eq!(
                    sys_sysarch(&p, op, parms),
                    Err(Errno::EINVAL),
                    "op {op} parms {parms:#x}"
                );
                let log: Vec<String> = testutil::file_log();
                assert_eq!(log, vec![String::from("pledge_fail 0x0")], "op {op}");
            }
        }
        p.process()
            .ps_flags
            .fetch_and(!PS_PLEDGE, Ordering::Relaxed);
    }
}
/* </TESTS> */
