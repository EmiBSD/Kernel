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
//! arm64's system call convention: the ABI's half of `syscall.c`, `svc_handler` from the
//! number on and `child_return`'s two frame writes.
//!
//! Upstream: sys/arch/arm64/arm64/syscall.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/syscall.rs@e8249449d905
//!
//! libc loads the number into `x8` and the arguments into `x0`..`x5` and executes `svc`
//! followed by a speculation barrier; `hal`'s entry counts the call, re-enables the
//! interrupts the user had, skips the barrier and hands the registers here as a
//! `hal::syscall::UserRegs`. [`svc`] reads them, runs the table's entry
//! ([`crate::sysent::dispatch`]), writes the result as the C's switch does (`x0` and the
//! carry clear; an errno in `x0` and the carry set; the pc back to the `svc` for `ERESTART`;
//! nothing for `EJUSTRETURN`, whose call rewrote the frame itself) and only then calls
//! `mi_syscall_return` (`kernel::api::syscall_return`), whose `userret` may save the frame
//! for a signal. [`child`] is a forked child's `fork` return of 0, carry clear.
//!
//! The system call number is an identifier, not a count: the index into `sysent[]`,
//! assigned by OpenBSD in `sys/kern/syscalls.master` (from which libc's `syscall.h` is
//! generated) and part of the ABI; the low numbers date from the first Unix (exit 1, fork 2,
//! read 3). Every arm64 register is 64 bits wide, but the number fits in 32 (there are about
//! 331), and OpenBSD's arm64 entry reads it into an `int` (`syscall.c:46`), dropping the high
//! half; so does this one (`scarg::int`), so that a malformed number behaves as on OpenBSD
//! (the user's decision of 2026-10-22): its low 32 bits are the number (0 or negative, the
//! C's `code <= 0`: `ENOSYS`), and `pin_check` kills the process with `SIGABRT` unless the
//! `svc` is at the site pinned for it. amd64's entry keeps the whole register (`trap.c:781`,
//! a `register_t`), the one difference of width between the two conventions. diff-openbsd:
//! `syscalls/entry#1` (`difftest syscalls entry`), its line `int-syscall`.
//!
//! ## Redesign
//! - Decision 53 (N7.abi2, the user, 2026-11-26): LZ's `svc_handler` and `child_return`
//!   (`bsd`, then `hal` from N4.8b) read and wrote the trap frame themselves; the convention
//!   moved here, out of `hal`, which keeps the machine's half (`hal::arch::arm64::syscall`).
//!   The frame is reached only through `UserRegs`: one batch read, one write of a named
//!   register with the error flag, a restart whose pc arithmetic is `hal`'s, so this crate
//!   never writes a pc, the stack pointer or a bit of `tf_spsr` but the carry.
//! - The C's `goto bad` for a number out of range is [`crate::sysent::dispatch`]'s: a number
//!   without an entry goes to `kernel::api::syscall_legacy`, which answers `ENOSYS` for
//!   `code <= 0` or past `SYS_MAXSYSCALL`, and the error is written as the switch's
//!   `default:` writes it.
//! - `mi_syscall_return` is a call to `kernel::api::syscall_return` after the writes, where
//!   LZ's handler called it by name (and `hal` through its trap handler, N4.8b..N7.abi2).

use hal::syscall::{Gpr, Register, Rval, SysResult, UserRegs};
use sysdefs::errno::Errno;

use crate::scarg::int;
use crate::sysent::{SYSENT, Sysent, dispatch};

/// `ERESTART` as a system call's result carries it.
const ERESTART: i32 = Errno::ERESTART.as_i32();
/// `EJUSTRETURN` as a system call's result carries it.
const EJUSTRETURN: i32 = Errno::EJUSTRETURN.as_i32();

/// `svc_handler` from the number on: the system call the registers name, over `sysent[]`,
/// its result written, then `mi_syscall_return`. Registered in `hal` by
/// [`crate::abi_init`] (`hal::syscall::SyscallFn`).
pub fn svc(regs: &UserRegs) {
    let (nr, error, rval) = call(&SYSENT, regs);
    kernel::api::syscall_return(nr, error, &rval);
}

/// The call and its writes over `table`: the number (`x8` as an `int`) and the six
/// arguments read, the entry run, the result written as the C's switch writes it. Returns
/// what `mi_syscall_return` takes: the number, the result and the return registers.
pub fn call(table: &[Sysent], regs: &UserRegs) -> (Register, SysResult, Rval) {
    let [x0, x1, x2, x3, x4, x5, x8] = regs.gprs([
        Gpr::X0,
        Gpr::X1,
        Gpr::X2,
        Gpr::X3,
        Gpr::X4,
        Gpr::X5,
        Gpr::X8,
    ]);
    // `int code = frame->tf_x[8]`: the low 32 bits, sign-extended.
    let nr = int(x8) as Register;
    let mut rval: Rval = [0, 0];
    let error = dispatch(table, nr, &[x0, x1, x2, x3, x4, x5], &mut rval);
    match error {
        Ok(()) => regs.set_return(Gpr::X0, rval[0], false),
        // Reconstruct the pc to point at the svc.
        Err(ERESTART) => regs.restart(),
        // nothing to do
        Err(EJUSTRETURN) => {}
        Err(e) => regs.set_return(Gpr::X0, e as Register, true),
    }
    (nr, error, rval)
}

/// `child_return`'s frame writes: a forked child's `fork` returns 0, carry clear.
/// Registered in `hal` by [`crate::abi_init`] (`hal::syscall::ChildFn`).
pub fn child(regs: &UserRegs) {
    regs.set_return(Gpr::X0, 0, false);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "the test rows fold registers into an errno and indexes into registers"
)]
mod tests {
    use super::*;
    use hal::syscall::SysArgs;

    /// `ENOSYS`, the answer of a number without an entry.
    const ENOSYS: i32 = Errno::ENOSYS.as_i32();

    /// A test row: `rval[0]` the six arguments as decimal digits, `rval[1]` -3; the result
    /// chosen by the sixth argument: 0 `Ok`, 1 `ERESTART`, 2 `EJUSTRETURN`, else that errno.
    fn row(args: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = args.iter().fold(0, |acc, &a| acc * 10 + a);
        rval[1] = -3;
        match args[5] {
            0 => Ok(()),
            1 => Err(ERESTART),
            2 => Err(EJUSTRETURN),
            e => Err(e as i32),
        }
    }

    /// A table of 21 rows, all [`row`].
    const TABLE: [Sysent; 21] = [Sysent::Native(row); 21];

    /// A trap with number `x8` and arguments `args` in `x0`..`x5`; every other register
    /// `100 + n`, so a stray write shows.
    fn trap(x8: Register, args: [Register; 6], carry: bool) -> (UserRegs, [Register; 30]) {
        let mut x: [Register; 30] = core::array::from_fn(|n| 100 + n as Register);
        x[..6].copy_from_slice(&args);
        x[8] = x8;
        (UserRegs::fake_with_carry(x, carry), x)
    }

    #[test]
    fn ok_writes_x0_and_clears_the_carry_and_nothing_else() {
        let (regs, mut before) = trap(1, [1, 2, 3, 4, 5, 0], true);
        let (nr, error, rval) = call(&TABLE, &regs);
        // The arguments in order, x0 to x5.
        assert_eq!((nr, error, rval), (1, Ok(()), [123_450, -3]));
        // The write is made before `call` returns, so before `mi_syscall_return`.
        assert_eq!((regs.fake_writes(), regs.fake_restarts()), (1, 0));
        assert!(!regs.fake_carry());
        // x0 is rval[0]; x1 is never written (the C writes only tf_x[0]).
        before[0] = 123_450;
        assert_eq!(regs.fake_regs(), before);
    }

    #[test]
    fn an_error_is_x0_with_the_carry_set() {
        let (regs, mut before) = trap(3, [0, 0, 0, 0, 0, 9], false);
        let (nr, error, _) = call(&TABLE, &regs);
        assert_eq!((nr, error), (3, Err(9)));
        assert!(regs.fake_carry());
        before[0] = 9;
        assert_eq!(regs.fake_regs(), before);
        assert_eq!((regs.fake_writes(), regs.fake_restarts()), (1, 0));
    }

    #[test]
    fn erestart_rewinds_and_writes_nothing() {
        for carry in [false, true] {
            let (regs, before) = trap(4, [0, 0, 0, 0, 0, 1], carry);
            assert_eq!(call(&TABLE, &regs).1, Err(ERESTART));
            assert_eq!((regs.fake_writes(), regs.fake_restarts()), (0, 1));
            assert_eq!(regs.fake_regs(), before);
            assert_eq!(regs.fake_carry(), carry);
        }
    }

    #[test]
    fn ejustreturn_leaves_the_frame_as_the_call_left_it() {
        for carry in [false, true] {
            let (regs, before) = trap(5, [0, 0, 0, 0, 0, 2], carry);
            assert_eq!(call(&TABLE, &regs).1, Err(EJUSTRETURN));
            assert_eq!((regs.fake_writes(), regs.fake_restarts()), (0, 0));
            assert_eq!(regs.fake_regs(), before);
            assert_eq!(regs.fake_carry(), carry);
        }
    }

    #[test]
    fn the_number_is_x8_as_an_int() {
        // The high half is dropped: 0x1_0000_0014 is entry 20.
        let (regs, _) = trap(0x1_0000_0014, [7, 0, 0, 0, 0, 0], false);
        let (nr, error, rval) = call(&TABLE, &regs);
        assert_eq!((nr, error, rval[0]), (20, Ok(()), 700_000));
        // A number whose low 32 bits are negative has no entry: `ENOSYS`, carry set.
        for x8 in [0xffff_ffff, 0x8000_0000, 0x7_ffff_ffff] {
            let (regs, _) = trap(x8, [0; 6], false);
            let (nr, error, _) = call(&TABLE, &regs);
            assert!(nr < 0, "{x8:#x} is negative as an int");
            assert_eq!(error, Err(ENOSYS), "{x8:#x}");
            assert_eq!(regs.fake_regs()[0], ENOSYS as Register);
            assert!(regs.fake_carry());
        }
    }

    #[test]
    fn a_forked_child_returns_zero_with_the_carry_clear() {
        let (regs, mut before) = trap(2, [55, 66, 0, 0, 0, 0], true);
        child(&regs);
        before[0] = 0;
        assert_eq!(regs.fake_regs(), before);
        assert!(!regs.fake_carry());
        assert_eq!((regs.fake_writes(), regs.fake_restarts()), (1, 0));
    }
}
/* </TESTS> */
