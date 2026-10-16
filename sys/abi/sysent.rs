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
//! The module's system call table: one entry per system call number of
//! `<sys/syscall.h>` (`crate::syscall`, the ABI edge), each `Native` (translated into a
//! `kernel` entry point) or `Legacy` (handed to the LZ path `bsd` registered, through
//! `KernelApi::syscall_legacy`), and the dispatcher `hal` reaches through the handler the
//! module registered.
//!
//! N2.4: every entry is `Legacy`, so every system call behaves exactly as LZ's: the number
//! is not even read here but to pick the entry, and an out-of-range number goes to the
//! fallback too, which answers `ENOSYS` as the traps did. A subsystem that moves into
//! `kernel` flips its entries to `Native` (pipe(2) first, N2.6) and raises [`NATIVE`].
//!
//! Translation only (decision 21; `cargo xtask abi-check`, decision 25): no loop, a
//! `match` only on the number's conversion and on `Some`/`None`, one kernel call per
//! function, no state.

use kernel::api::{HandlerInfo, KernelApi, Register, Rval, SysArgs, SysResult};

use crate::syscall::SYS_MAXSYSCALL;

/// A `Native` entry: the translation of one system call into its `kernel` entry point.
pub type NativeFn = fn(&'static KernelApi, &SysArgs, &mut Rval) -> SysResult;

/// One entry of the table.
#[derive(Clone, Copy)]
pub enum Sysent {
    /// Translated here into the kernel crate.
    Native(NativeFn),
    /// Answered by the LZ path `bsd` registered.
    Legacy,
}

/// How many entries of [`SYSENT`] are `Native` (a test counts them).
pub const NATIVE: u32 = 0;

/// What the module says about itself when it registers ([`crate::abi_init`]).
pub const INFO: HandlerInfo = HandlerInfo {
    syscalls: SYS_MAXSYSCALL as u32,
    native: NATIVE,
};

/// `sysent[]`: the entry of every system call number below `SYS_MAXSYSCALL`.
pub static SYSENT: [Sysent; SYS_MAXSYSCALL] = [Sysent::Legacy; SYS_MAXSYSCALL];

/// The module's system call handler (`kernel::api::SyscallHandler`).
pub fn syscall(
    api: &'static KernelApi,
    nr: Register,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    dispatch(&SYSENT, api, nr, args, rval)
}

/// The dispatcher over a table: a `Native` entry is called, a `Legacy` one or a number with
/// no entry goes to the fallback with the number unchanged.
pub fn dispatch(
    table: &[Sysent],
    api: &'static KernelApi,
    nr: Register,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    match entry(table, nr) {
        Some(Sysent::Native(f)) => f(api, args, rval),
        Some(Sysent::Legacy) | None => (api.syscall_legacy)(nr, args, rval),
    }
}

/// The entry of number `nr`, if the table has one (a negative number has none).
fn entry(table: &[Sysent], nr: Register) -> Option<&Sysent> {
    match usize::try_from(nr) {
        Ok(i) => table.get(i),
        Err(_) => None,
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// The fallback of the test table: says which number reached it.
    fn legacy(nr: Register, _: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(78) // ENOSYS, as the LZ path answers an out-of-range number
    }

    fn copyin(_: usize, _: &mut [u8]) -> Result<(), i32> {
        Ok(())
    }

    fn copyout(_: &[u8], _: usize) -> Result<(), i32> {
        Ok(())
    }

    static API: KernelApi = KernelApi::new(legacy, copyin, copyout);

    /// A `Native` entry of the test table.
    fn native(_: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = args[0] + args[1];
        rval[1] = -1;
        Ok(())
    }

    #[test]
    fn every_number_has_an_entry() {
        assert_eq!(SYSENT.len(), SYS_MAXSYSCALL);
        assert_eq!(INFO.syscalls as usize, SYS_MAXSYSCALL);
    }

    #[test]
    fn native_counts_the_native_entries() {
        let n = SYSENT
            .iter()
            .filter(|e| matches!(e, Sysent::Native(_)))
            .count();
        assert_eq!(n, NATIVE as usize);
        assert_eq!(INFO.native, NATIVE);
    }

    /// Every `SYS_*` number of `<sys/syscall.h>` (the generated file, read as text) is an
    /// entry of the table, and no two names share a number.
    #[test]
    fn the_numbers_are_sys_syscall_h() {
        let src = include_str!("../sys/syscall.rs");
        let mut seen = std::collections::BTreeMap::new();
        for line in src.lines() {
            let Some(rest) = line.strip_prefix("pub const SYS_") else {
                continue;
            };
            let Some((name, value)) = rest.split_once(": i32 = ") else {
                continue;
            };
            let n: usize = value.trim_end_matches(';').parse().unwrap();
            assert!(n < SYSENT.len(), "SYS_{name} = {n} has no entry");
            assert!(n > 0, "SYS_{name} = {n}: the traps refuse 0");
            assert_eq!(seen.insert(n, name), None, "SYS_{name} reuses {n}");
        }
        assert!(seen.len() > 200, "parsed {} names", seen.len());
        assert_eq!(seen.get(&4), Some(&"write"));
        assert_eq!(seen.get(&1), Some(&"exit"));
    }

    #[test]
    fn every_entry_is_legacy_at_n2_4() {
        for nr in 0..SYS_MAXSYSCALL as Register {
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr, &[0; 6], &mut rval), Err(78));
            assert_eq!(rval[0], nr, "number {nr} reached the fallback unchanged");
        }
    }

    #[test]
    fn a_number_without_an_entry_goes_to_the_fallback() {
        for nr in [-1, Register::MIN, SYS_MAXSYSCALL as Register, 100_000] {
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr, &[0; 6], &mut rval), Err(78));
            assert_eq!(rval[0], nr);
        }
    }

    #[test]
    fn a_native_entry_reaches_its_function() {
        let table = [Sysent::Legacy, Sysent::Native(native), Sysent::Legacy];
        let mut rval = [0; 2];
        assert_eq!(
            dispatch(&table, &API, 1, &[2, 3, 0, 0, 0, 0], &mut rval),
            Ok(())
        );
        assert_eq!(rval, [5, -1]);
        assert_eq!(
            dispatch(&table, &API, 2, &[2, 3, 0, 0, 0, 0], &mut rval),
            Err(78)
        );
        assert_eq!(rval[0], 2);
    }
}
/* </TESTS> */
