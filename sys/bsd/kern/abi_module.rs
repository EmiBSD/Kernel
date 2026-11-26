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
//! `bsd`'s side of the ABI (`abi`, linked into the image, decision 47) until the subsystems
//! it stands for move: the `sysent[]` rows (LZ's `init_sysent` table and `mi_syscall`, what
//! both arch traps called themselves), which it registers as the kernel's `LegacySyscalls`
//! bridge, and the boot step that registers that bridge and calls `abi::abi_init` before
//! `main()`, so the ABI's handler answers every system call from the first one on.
//!
//! arm64's trap in `hal` hands the ABI's handler the trap's registers (decision 53: the
//! per-arch system call convention is `abi::arch`'s); amd64's frozen trap still names the
//! `syscall` function this file had over `hal`'s `dispatch`, both gone with decision 53,
//! until N10 rewrites that entry (decision 44). Every entry of the ABI's table, `Native` (through
//! its `kernel::api` function) or `Legacy` (through `kernel::api::syscall_legacy`), comes
//! back here, to [`BsdLegacySyscalls`], for its row. Either way a system call reaches
//! `mi_syscall` with the same thread, number, arguments and return registers as LZ's, and
//! comes back with the same `Result`; only the route changed (decision 52: `abi` calls
//! `kernel::api` directly, the `KernelApi` table `bsd` filled is gone).
//!
//! The image crate (`bsd` until N9) is the one that names `abi`: `kernel` and `hal` never
//! do, and `abi` names nothing of `kernel` but `kernel::api` (docs/SCAFFOLDING.md, the boot
//! glue's call). No `unsafe`: every call here is a safe function of `abi`, `bsd`, `hal` or
//! `kernel`. Deleted at N9 with `bsd`.

use kernel::syscall::{LegacySyscalls, install_legacy_syscalls};

use crate::kern::init_sysent::SYSENT;
use crate::kern::subr_prf::panic;
use crate::machine::cpu::curproc;
use crate::sys::errno::Errno;
use crate::sys::syscall::SYS_MAXSYSCALL;
use crate::sys::syscall_mi::mi_syscall;
use crate::sys::systm::SysArgs;
use crate::sys::types::Register;

/// The `sysent[]` rows, the kernel's `LegacySyscalls` bridge (`bridges.toml`, paid by N9).
struct BsdLegacySyscalls;

impl LegacySyscalls for BsdLegacySyscalls {
    fn syscall(&self, code: Register, args: &SysArgs, rval: &mut [Register; 2]) -> Result<(), i32> {
        legacy_syscall(code, args, rval)
    }
}

/// The one implementation the boot step registers.
static LEGACY_SYSCALLS: BsdLegacySyscalls = BsdLegacySyscalls;

/// Row `code` of `sysent[]` ([`BsdLegacySyscalls`]): what the traps did before (`ENOSYS`
/// for a number outside `sysent[]`, else `mi_syscall` on the entry), for the thread on this
/// CPU.
pub fn legacy_syscall(code: Register, args: &SysArgs, rval: &mut [Register; 2]) -> Result<(), i32> {
    let Some(p) = curproc() else {
        panic(format_args!("syscall: no curproc"));
    };
    let error = if code <= 0 || code as usize >= SYS_MAXSYSCALL {
        Err(Errno::ENOSYS)
    } else {
        mi_syscall(p, code, &SYSENT[code as usize], args, rval)
    };
    error.map_err(Errno::as_i32)
}

/// The boot step, from the boot glue after `hal_bridges_register` and before `main()`:
/// registers the `sysent[]` rows as the kernel's `LegacySyscalls`, has the ABI register its
/// system call handler, and prints the one line that says what the handler's table holds.
/// Panics when either registration is missing after it, or when a handler other than the
/// ABI's was registered before it, which no system call may find out later.
pub fn abi_boot() {
    if install_legacy_syscalls(&LEGACY_SYSCALLS).is_err() {
        panic(format_args!("abi: the sysent[] rows were registered twice"));
    }
    if abi::abi_init().is_err() {
        panic(format_args!(
            "abi: a system call handler was registered before abi_init"
        ));
    }
    let Some(info) = hal::syscall::handler_info() else {
        panic(format_args!(
            "abi: abi_init registered no system call handler"
        ));
    };
    crate::kprintf!("abi: {} syscalls, {} native\n", info.syscalls, info.native);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the test rows fold the number and the arguments into one i32"
)]
mod tests {
    use super::*;
    use abi::sysent::Sysent as AbiSysent;

    /// The rows of `bsd`'s test process (the image registers [`BsdLegacySyscalls`]): the
    /// number times a million plus the six arguments as decimal digits (9 outside 0..=9).
    struct Rows;

    impl LegacySyscalls for Rows {
        fn syscall(
            &self,
            code: Register,
            args: &SysArgs,
            rval: &mut [Register; 2],
        ) -> Result<(), i32> {
            rval[0] = code;
            let digits = args.iter().fold(0, |acc, &a| {
                acc * 10 + if (0..=9).contains(&a) { a as i32 } else { 9 }
            });
            Err(code as i32 * 1_000_000 + digits)
        }
    }

    static ROWS: Rows = Rows;

    /// Decision 52's path end to end: every `Native` entry of the ABI's table, through its
    /// `kernel::api` function, hands its row exactly the arguments the row reads, in order,
    /// as the `Legacy` path hands them: the first `sy_narg` of `sysent[]` (LZ's
    /// `init_sysent.rs`, generated from `syscalls.master`) as the trap gave them, zeros
    /// after. A translation that dropped, swapped or added an argument fails here.
    #[test]
    fn every_native_call_passes_the_arguments_its_row_reads() {
        let _ = install_legacy_syscalls(&ROWS);
        let args: SysArgs = [1, 2, 3, 4, 5, 6];
        let mut native = 0;
        for nr in 1..SYS_MAXSYSCALL {
            if !matches!(abi::sysent::SYSENT[nr], AbiSysent::Native(_)) {
                continue;
            }
            native += 1;
            let k = SYSENT[nr].sy_narg as u32;
            let digits = 123_456 / 10i32.pow(6 - k) * 10i32.pow(6 - k);
            let mut rval = [-7; 2];
            assert_eq!(
                abi::sysent::dispatch(&abi::sysent::SYSENT, nr as Register, &args, &mut rval),
                Err(nr as i32 * 1_000_000 + digits),
                "SYS {nr}: sy_narg {k}"
            );
        }
        assert_eq!(native, abi::sysent::NATIVE);
    }
}
/* </TESTS> */
