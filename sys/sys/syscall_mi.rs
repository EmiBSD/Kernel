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
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
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
 *	@(#)kern_xxx.c	8.2 (Berkeley) 11/14/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/syscall_mi.h>`: `bsd`'s names over the kernel's (`kernel::syscall`, N4.8b), for the
//! callers still here: the `Legacy` fallback and the `Native` wrappers that run a `sysent[]`
//! row through the prologue (`abi_module.rs`, `vm_bsd.rs`, `kern_descrip.rs`,
//! `pipe_glue.rs`), and the machine trap code that still calls the return paths by name
//! (amd64's, N10).
//!
//! Upstream: sys/sys/syscall_mi.h @ 3ce1f3f79392
//! LZ: sys/sys/syscall_mi.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.8b: `pin_check`, `mi_syscall`, `mi_syscall_return`, `mi_child_return`, `mi_ast` and
//!   `SY_NOLOCK_DEFERRED` are `kernel::syscall`'s; each function here is one call to its
//!   kernel namesake on the thread pinned (`bsd`'s threads are placed: `ProcExt::as_static`),
//!   `mi_syscall` passing the row's lock (`syscall_lock` of its `sy_flags`) and its function as
//!   the body. The `MAP_STACK` check LZ reported is made there (the module's redesign).

use core::pin::Pin;

use crate::sys::errno::Errno;
use crate::sys::proc::{Proc, ProcExt};
use crate::sys::systm::{SysArgs, Sysent};
use crate::sys::types::Register;

pub use kernel::syscall::SY_NOLOCK_DEFERRED;

/// The thread `p` of a system call, pinned.
fn pinned(p: &Proc) -> Pin<&'static Proc> {
    hal::pin::pin_static(p.as_static())
}

/// `pin_check(p, code)` (`kernel::syscall::pin_check`).
#[inline]
pub fn pin_check(p: &Proc, code: Register) -> Result<(), Errno> {
    kernel::syscall::pin_check(pinned(p), code)
}

/// `mi_syscall(p, code, callp, argp, retval)`: the prologue around row `callp`'s function
/// (`kernel::syscall::mi_syscall`, under the kernel lock unless the row is `SY_NOLOCK`).
#[inline]
pub fn mi_syscall(
    p: &Proc,
    code: Register,
    callp: &Sysent,
    argp: &SysArgs,
    retval: &mut [Register; 2],
) -> Result<(), Errno> {
    let lock = kernel::syscall::syscall_lock(callp.sy_flags, code);
    kernel::syscall::mi_syscall(pinned(p), code, lock, argp, retval, |args, rval| {
        (callp.sy_call)(p, args, rval)
    })
}

/// `mi_syscall_return(p, code, error, retval)`.
#[inline]
pub fn mi_syscall_return(
    p: &Proc,
    code: Register,
    error: Result<(), Errno>,
    retval: &[Register; 2],
) {
    kernel::syscall::mi_syscall_return(pinned(p), code, error, retval);
}

/// `mi_child_return(p)`.
#[inline]
pub fn mi_child_return(p: &Proc) {
    kernel::syscall::mi_child_return(pinned(p));
}

/// `mi_ast(p, resched)`.
#[inline]
pub fn mi_ast(p: &Proc, resched: bool) {
    kernel::syscall::mi_ast(pinned(p), resched);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `mi_syscall`'s lock choice: the `SY_NOLOCK_DEFERRED` table against
    // `sysent`.

    use super::*;
    use crate::kern::init_sysent::SYSENT;
    use crate::sys::syscall::*;
    use crate::sys::systm::SY_NOLOCK;

    /// The kernel's `SY_NOLOCK` is `<sys/systm.h>`'s.
    #[test]
    fn nolock_is_the_headers() {
        assert_eq!(kernel::syscall::SY_NOLOCK, SY_NOLOCK);
    }

    /// Every deferred entry is a `NOLOCK` system call, listed once.
    #[test]
    fn deferred_entries_are_nolock() {
        for (i, &code) in SY_NOLOCK_DEFERRED.iter().enumerate() {
            assert!(
                SYSENT[code as usize].sy_flags & SY_NOLOCK != 0,
                "{code} is not NOLOCK"
            );
            assert!(
                !SY_NOLOCK_DEFERRED[..i].contains(&code),
                "{code} listed twice"
            );
        }
    }

    /// The `NOLOCK` system calls that run without the kernel lock are the audited ones.
    #[test]
    fn audited_nolock_set() {
        let lock = |code: i32| {
            kernel::syscall::syscall_lock(SYSENT[code as usize].sy_flags, code as Register)
        };
        let unlocked: std::vec::Vec<i32> =
            (0..SYS_MAXSYSCALL as i32).filter(|&c| !lock(c)).collect();
        let mut audited = std::vec![
            SYS_getentropy,
            SYS_getpid,
            SYS_getuid,
            SYS_geteuid,
            SYS_getppid,
            SYS_getegid,
            SYS_getgid,
            SYS_sigprocmask,
            SYS_gettimeofday,
            SYS_settimeofday,
            SYS_setitimer,
            SYS_getitimer,
            SYS_getgroups,
            SYS_futex,
            SYS_clock_gettime,
            SYS_clock_settime,
            SYS_clock_getres,
            SYS_nanosleep,
            SYS_sigsuspend,
            SYS_adjtime,
            SYS_getrlimit,
            SYS_utrace,
            SYS_issetugid,
            SYS_getresuid,
            SYS_getresgid,
            SYS_sched_yield,
            SYS_getthrid,
            SYS___thrsigdivert,
            SYS_adjfreq,
            SYS___set_tcb,
            SYS___get_tcb,
            // the `files` audit
            SYS_open,
            SYS_openat,
            SYS___pledge_open,
            SYS_stat,
            SYS_lstat,
            SYS_fstatat,
            SYS___realpath,
            SYS_dup,
            SYS_flock,
            SYS_lseek,
            SYS_kqueue,
            SYS_kqueue1,
            SYS_pipe,
            SYS_pipe2,
            SYS_getrtable,
            SYS_getdtablecount,
            // the `uvm` audit (setrlimit(2)'s RLIMIT_STACK change is mprotect's uvm_map_protect)
            SYS_munmap,
            SYS_mprotect,
            SYS_minherit,
            SYS_kbind,
            SYS_setrlimit,
            // pledge(2): ps_pledge is an atomic, the rest runs under ps_mtx (kern_pledge.c)
            SYS_pledge,
            // file I/O (vnodes, pipes, kqueues, sockets), umask(2) and fcntl(2): the `files`,
            // `net` and `netinet` audits
            SYS_read,
            SYS_write,
            SYS_close,
            SYS_fstat,
            SYS_ioctl,
            SYS_umask,
            SYS_select,
            SYS_dup2,
            SYS_fcntl,
            SYS_dup3,
            SYS_ppoll,
            SYS_pselect,
            SYS_readv,
            SYS_writev,
            SYS_pread,
            SYS_pwrite,
            SYS_preadv,
            SYS_pwritev,
            SYS_poll,
            SYS_closefrom,
            // sockets (the kern side, `net`, `netinet`), sendsyslog(2) over them
            SYS_recvmsg,
            SYS_sendmsg,
            SYS_recvfrom,
            SYS_accept,
            SYS_getpeername,
            SYS_getsockname,
            SYS_accept4,
            SYS_socket,
            SYS_connect,
            SYS_bind,
            SYS_setsockopt,
            SYS_listen,
            SYS_sendsyslog,
            SYS_recvmmsg,
            SYS_sendmmsg,
            SYS_getsockopt,
            SYS_sendto,
            SYS_shutdown,
            SYS_socketpair,
            SYS_ypconnect,
            SYS_setrtable,
            // kevent(2), mmap(2) (its file path is not ported) and sysctl(2)
            SYS_kevent,
            SYS_mmap,
            SYS_sysctl,
        ];
        audited.sort_unstable();
        assert_eq!(unlocked, audited);
        // A number past the table is locked (`sys_nosys`'s SIGSYS).
        assert!(kernel::syscall::syscall_lock(
            SYSENT[0].sy_flags,
            SYS_MAXSYSCALL as Register
        ));
    }
}
/* </TESTS> */
