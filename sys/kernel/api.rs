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
//! `kernel::api`: the kernel's one door for the ABI (decision 52, the user, 2026-11-26): one
//! plain function per system call, which `abi`'s translations call directly, and nothing of
//! the kernel else is named by `abi` (`cargo xtask abi-check`, R6). The chain is userland ->
//! `abi` -> `kernel` -> `hal`; this crate never names `abi`.
//!
//! The functions are grouped as the kernel's modules are, and as `abi`'s files are:
//! [`proc`] (the processes, futex(2), sigreturn(2)), [`vm`], [`file`] (pipe(2) and the
//! descriptors), [`event`], [`time`], [`base`] (sysctl(2), sendsyslog(2), reboot(2)),
//! [`fs`] (the VFS and nfssvc(2)) and [`net`] (the sockets), plus [`syscall_legacy`], the
//! `Legacy` path of the system calls nothing translates yet.
//!
//! Every one of them still runs its call's `sysent[]` row, which `bsd` holds, through
//! `mi_syscall` (the credentials, the `MAP_STACK` check, the pinned syscalls, pledge, the
//! kernel lock): the same prologue and the same row as the `Legacy` path, so the two paths
//! cannot diverge. The rows are reached through one bridge, [`crate::syscall::LegacySyscalls`]
//! (`bridges.toml`, paid by N9, when the rows and `bsd` go); a function moves to call the
//! kernel's own function directly when its row leaves `bsd`.
//!
//! ## Redesign
//! - Decision 22 (N2.4) made this module the kernel's side of `hal::api::KernelApi`: a table
//!   of function pointers the image's boot glue filled with `bsd`'s glue and handed to the
//!   loadable ABI module, so that `abi` named no `kernel`. Decision 47 linked `abi` into the
//!   image; decision 52 drops the table: `abi` depends on `kernel` in Cargo and calls these
//!   functions, so a system call is a direct call where it was an indirect one through the
//!   table, and `hal`'s dispatch point calls `abi`'s handler directly (the trampoline that
//!   looked the table up, `abi_syscall`, is gone).
//! - The 189 typed entries of `KernelApi` (`VmEntries`, `FdEntries`, ..., pipe2 serving two
//!   numbers) were `bsd`'s glue (`native_*` and the `*_ENTRIES` tables of
//!   `sys/kern/pipe_glue.rs`, `vm_bsd.rs`, `kern_descrip.rs`, `kern_event.rs`,
//!   `sys_generic.rs`, `subr_log.rs`, `sys_futex.rs`, `proc_glue.rs`, `kern_sysctl.rs`,
//!   `kern_time.rs`, `kern_sig.rs`, `vfs_syscalls.rs`, `socket_glue.rs`, `kern_xxx.rs`);
//!   they move here as they were, named after their `KernelApi` member (`native_read` is
//!   [`file::read`]), with `bsd`'s `kern_descrip::native` and `vm_bsd`'s `native` as
//!   [`native`] and [`native_rval`] over the bridge. Same arguments, same number check, same
//!   return registers (the vm calls and pipe(2) still run the row over registers of their
//!   own), same errno.

pub mod base;
pub mod event;
pub mod file;
pub mod fs;
pub mod net;
pub mod proc;
pub mod time;
pub mod vm;

use hal::syscall::{ENOSYS, Register, Rval, SysArgs, SysResult};

pub use crate::syscall::legacy_syscall as syscall_legacy;

/// A `Native` call: `ENOSYS` unless `nr` is `want`'s number, else row `nr` of `sysent[]`
/// through `mi_syscall` with `args` and the trap's return registers, as on the `Legacy` path.
fn native(nr: Register, want: i32, args: SysArgs, rval: &mut Rval) -> SysResult {
    if nr != want as Register {
        return Err(ENOSYS);
    }
    syscall_legacy(nr, &args, rval)
}

/// A `Native` call whose row writes return registers of its own (the vm calls): as
/// [`native`], with `rval[0]` back in the `Ok`.
fn native_rval(nr: Register, want: i32, args: SysArgs) -> Result<Register, i32> {
    let mut rval: Rval = [0, 0];
    native(nr, want, args, &mut rval)?;
    Ok(rval[0])
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the test rows fold the number and the arguments into one i32"
)]
mod tests {
    use super::*;
    use crate::syscall::{LegacySyscalls, install_legacy_syscalls};
    use hal::sys::syscall::{
        SYS_clock_gettime, SYS_close, SYS_dup, SYS_fork, SYS_getdtablecount, SYS_kevent, SYS_mmap,
        SYS_munlockall, SYS_open, SYS_pipe, SYS_pipe2, SYS_pwritev, SYS_read, SYS_reboot,
        SYS_recvfrom, SYS_socket,
    };

    /// The `sysent[]` rows of the kernel's test process: the number into `rval[0]`, the
    /// first argument into `rval[1]`; `Ok` when the first argument is 8, else the number
    /// times a million plus the six arguments as decimal digits (9 outside 0..=9).
    struct Rows;

    impl LegacySyscalls for Rows {
        fn syscall(&self, code: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
            rval[0] = code;
            rval[1] = args[0];
            if args[0] == 8 {
                return Ok(());
            }
            let digits = args.iter().fold(0, |acc, &a| {
                acc * 10 + if (0..=9).contains(&a) { a as i32 } else { 9 }
            });
            Err(code as i32 * 1_000_000 + digits)
        }
    }

    static ROWS: Rows = Rows;

    fn rows() {
        let _ = install_legacy_syscalls(&ROWS);
    }

    /// Each group's functions run their own number's row with the arguments in
    /// `<sys/syscallargs.h>`'s order and zeros after, over the trap's return registers.
    #[test]
    fn each_call_runs_its_own_row_with_its_arguments_in_order() {
        rows();
        let mut rval = [-7; 2];
        let n = |nr: i32| nr as Register;
        let e = |nr: i32, digits: i32| Err(nr * 1_000_000 + digits);
        assert_eq!(
            file::read(n(SYS_read), 1, 2, 3, &mut rval),
            e(SYS_read, 123_000)
        );
        assert_eq!(rval, [n(SYS_read), 1]);
        assert_eq!(
            file::close(n(SYS_close), 1, &mut rval),
            e(SYS_close, 100_000)
        );
        assert_eq!(
            file::pwritev(n(SYS_pwritev), 1, 2, 3, 4, &mut rval),
            e(SYS_pwritev, 123_400)
        );
        assert_eq!(
            event::kevent(n(SYS_kevent), 1, (2, 3), (4, 5), 6, &mut rval),
            e(SYS_kevent, 123_456)
        );
        assert_eq!(
            fs::open(n(SYS_open), 1, 2, 3, &mut rval),
            e(SYS_open, 123_000)
        );
        assert_eq!(
            net::recvfrom(n(SYS_recvfrom), 1, 2, 3, 4, 5, 6, &mut rval),
            e(SYS_recvfrom, 123_456)
        );
        assert_eq!(
            net::socket(n(SYS_socket), 1, 2, 3, &mut rval),
            e(SYS_socket, 123_000)
        );
        assert_eq!(proc::fork(n(SYS_fork), &mut rval), e(SYS_fork, 0));
        assert_eq!(
            time::clock_gettime(n(SYS_clock_gettime), 1, 2, &mut rval),
            e(SYS_clock_gettime, 120_000)
        );
        assert_eq!(
            base::reboot(n(SYS_reboot), 1, &mut rval),
            e(SYS_reboot, 100_000)
        );
        assert_eq!(rval, [n(SYS_reboot), 1]);
        let mut rval = [-7; 2];
        assert_eq!(
            syscall_legacy(9, &[1, 2, 3, 4, 5, 6], &mut rval),
            e(9, 123_456)
        );
        assert_eq!(rval, [9, 1]);
    }

    /// The vm calls and pipe(2) run their row over registers of their own: the trap's are
    /// not the row's, and `mmap(2)`'s address is the row's `rval[0]`, in the `Ok`.
    #[test]
    fn the_vm_calls_and_pipe_keep_their_own_return_registers() {
        rows();
        assert_eq!(
            vm::mmap(SYS_mmap as Register, 8, 2, 3, 4, 5, 6),
            Ok(SYS_mmap as usize)
        );
        assert_eq!(
            vm::mmap(SYS_mmap as Register, 1, 2, 3, 4, 5, 6),
            Err(SYS_mmap * 1_000_000 + 123_456)
        );
        assert_eq!(
            vm::munlockall(SYS_munlockall as Register),
            Err(SYS_munlockall * 1_000_000)
        );
        assert_eq!(
            file::pipe2(SYS_pipe as Register, 1, 0),
            Err(SYS_pipe * 1_000_000 + 100_000)
        );
        assert_eq!(
            file::pipe2(SYS_pipe2 as Register, 1, 2),
            Err(SYS_pipe2 * 1_000_000 + 120_000)
        );
    }

    /// A function reached with another call's number answers `ENOSYS` before it reaches a
    /// row, and leaves the return registers alone (`bsd`'s glue's check, kept).
    #[test]
    fn a_call_refuses_another_number() {
        rows();
        let enosys = Err(ENOSYS);
        let mut rval: Rval = [-7, -7];
        let nr = SYS_dup as Register;
        assert_eq!(file::close(nr, 3, &mut rval), enosys);
        assert_eq!(file::read(nr, 3, 0, 0, &mut rval), enosys);
        assert_eq!(file::pwritev(nr, 3, 0, 0, 0, &mut rval), enosys);
        assert_eq!(file::getdtablecount(nr, &mut rval), enosys);
        assert_eq!(file::dup(SYS_close as Register, 3, &mut rval), enosys);
        assert_eq!(
            net::socket(SYS_getdtablecount as Register, 1, 1, 0, &mut rval),
            enosys
        );
        assert_eq!(
            net::recvfrom(SYS_socket as Register, 3, 0, 0, 0, 0, 0, &mut rval),
            enosys
        );
        assert_eq!(net::ypconnect(SYS_socket as Register, 1, &mut rval), enosys);
        assert_eq!(rval, [-7, -7]);
        assert_eq!(file::pipe2(SYS_read as Register, 0, 0), enosys);
        assert_eq!(vm::munlockall(SYS_read as Register), enosys);
    }
}
/* </TESTS> */
