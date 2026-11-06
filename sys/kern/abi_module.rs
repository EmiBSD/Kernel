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
//! it stands for move: the `Legacy` system call path (LZ's `sysent[]` and `mi_syscall`, what
//! both arch traps called themselves), the `KernelApi` table `bsd` fills, and the boot step
//! that hands that table to `abi::abi_init` before `main()`, so the ABI's handler answers
//! every system call from the first one on.
//!
//! The arch traps reach `hal`'s dispatch point (arm64's directly, amd64's through
//! [`syscall`]), which calls the ABI's handler: a `Native` entry goes to its `KernelApi`
//! group, a `Legacy` one (and a number outside the table) back here, to [`legacy_syscall`].
//! Either way a system call reaches `mi_syscall` with the same thread, number, arguments and
//! return registers as LZ's, and comes back with the same `Result`; only the route changed.
//!
//! The image crate (`bsd` until N9) is the one that names `abi`: `kernel` never does, and
//! `abi` names nothing but `hal` (docs/SCAFFOLDING.md, the boot glue's call). No `unsafe`:
//! every call here is a safe function of `abi`, `bsd`, `hal` or `kernel`. Deleted at N9 with
//! `bsd`.

use kernel::api::KernelApi;

use crate::kern::init_sysent::SYSENT;
use crate::kern::subr_prf::panic;
use crate::machine::cpu::curproc;
use crate::sys::errno::Errno;
use crate::sys::syscall::SYS_MAXSYSCALL;
use crate::sys::syscall_mi::mi_syscall;
use crate::sys::systm::SysArgs;
use crate::sys::types::Register;

/// The `KernelApi` `bsd` hands the ABI: the `Legacy` path is `bsd`'s; pipe(2), the vm system
/// calls (`KernelApi::vm`, `sys/kern/vm_bsd.rs`), the descriptor system calls
/// (`KernelApi::fd`, `sys/kern/kern_descrip.rs`), the event system calls (`KernelApi::event`,
/// `sys/kern/kern_event.rs`), sendsyslog(2) (`KernelApi::sendsyslog`, `sys/kern/subr_log.rs`),
/// futex(2) (`KernelApi::futex`, `sys/kern/sys_futex.rs`), the process system calls
/// (`KernelApi::proc`, `sys/kern/proc_glue.rs`), sysctl(2) (`KernelApi::sysctl`,
/// `sys/kern/kern_sysctl.rs`), the time system calls with reboot(2) (`KernelApi::time`,
/// `sys/kern/kern_time.rs`), sigreturn(2) (`KernelApi::sigreturn`, `sys/kern/kern_sig.rs`) and
/// the VFS system calls (`KernelApi::vfs`, `sys/kern/vfs_syscalls.rs`) are the kernel crate's,
/// entered through `bsd`'s system call prologue.
static KERNEL_API: KernelApi = kernel::api::table(
    legacy_syscall,
    crate::kern::pipe_glue::native_pipe2,
    crate::kern::vm_bsd::VM_ENTRIES,
    crate::kern::kern_descrip::FD_ENTRIES,
    crate::kern::kern_event::EVENT_ENTRIES,
    crate::kern::subr_log::native_sendsyslog,
    crate::kern::sys_futex::native_futex,
    crate::kern::proc_glue::PROC_ENTRIES,
    crate::kern::kern_sysctl::native_sysctl,
    crate::kern::kern_time::TIME_ENTRIES,
    crate::kern::kern_sig::native_sigreturn,
    crate::kern::vfs_syscalls::VFS_ENTRIES,
);

/// The system call path amd64's trap calls where it chose the `sysent[]` entry: `hal`'s
/// dispatch point, its errno value back as an [`Errno`].
pub fn syscall(code: Register, args: &SysArgs, rval: &mut [Register; 2]) -> Result<(), Errno> {
    hal::syscall::dispatch(code, args, rval).map_err(errno_of)
}

/// The `Legacy` path, `KernelApi::syscall_legacy`: what the traps did before (`ENOSYS` for a
/// number outside `sysent[]`, else `mi_syscall` on the entry), for the thread on this CPU.
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

/// The boot step, from the boot glue after `hal_bridges_register` and before `main()`: hands
/// the table to the ABI, which registers its system call handler, and prints the one line
/// that says what the handler's table holds. Panics when no handler is registered after it,
/// which no system call may find out later.
pub fn abi_boot() {
    abi::abi_init(&KERNEL_API);
    let Some(info) = hal::syscall::handler_info() else {
        panic(format_args!(
            "abi: abi_init registered no system call handler"
        ));
    };
    crate::kprintf!("abi: {} syscalls, {} native\n", info.syscalls, info.native);
}

/// An errno value from `hal` back as the [`Errno`] it was made from.
fn errno_of(raw: i32) -> Errno {
    match Errno::from_raw(raw) {
        Some(e) => e,
        None => panic(format_args!("syscall: errno {raw} out of range")),
    }
}
/* </CODE> */
