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
 * Copyright (c) 2016-2017 Martin Pieuchot
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
//! `futex(2)`'s `sysent[]` row and `futex_init`, over `kernel::futex`.
//!
//! Upstream: sys/kern/sys_futex.c @ 3ce1f3f79392
//! LZ: sys/kern/sys_futex.rs@3c62ede68802
//!
//! ## Deviations
//! - `KTRACE` is not configured (`kernel::futex`'s).
//!
//! ## Redesign
//! - N4.6c: the futex itself (the buckets, the waiters on their stacks, `futex_wait`,
//!   `futex_wake`, `futex_requeue`, the private and shared keys) is `kernel::futex`'s, with 0
//!   `unsafe` (LZ: 9, the raw list links, the `ft_proc` and `ft_fsq` pointers, the buckets'
//!   `Sync`). This row turns the system call's arguments into the kernel's (the word and
//!   `g` as addresses, `timeout` as the address of a `struct timespec` or, for
//!   `FUTEX_REQUEUE`, the count to move) and writes the count woken in `retval[0]`;
//!   [`native_futex`], `KernelApi::futex`, runs `mi_syscall` on it (the `Native` entry).

use hal::syscall::SysResult;

use crate::kern::kern_descrip::native;
use crate::sys::errno::Errno;
use crate::sys::proc::Proc;
use crate::sys::syscall::SYS_futex;
use crate::sys::syscallargs::SysFutexArgs;
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::Register;

/// `futex_init`: the buckets' lock-ordering ids (`kernel::futex::futex_init`).
pub fn futex_init() {
    kernel::futex::futex_init();
}

/// `futex(2)`.
pub fn sys_futex(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFutexArgs = sysargs(v);
    retval[0] = kernel::futex::sys_futex(
        p,
        uap.f.get().addr(),
        uap.op.get(),
        uap.val.get(),
        uap.timeout.get().addr(),
        uap.g.get().addr(),
    )? as Register;
    Ok(())
}

/// `KernelApi::futex`: [`sys_futex`]'s row through the system call prologue.
pub fn native_futex(
    nr: Register,
    uaddr: usize,
    op: i32,
    val: i32,
    timeout: usize,
    g: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        uaddr as Register,
        op as Register,
        val as Register,
        timeout as Register,
        g as Register,
        0,
    ];
    native(nr, SYS_futex, args, rval)
}
/* </CODE> */
