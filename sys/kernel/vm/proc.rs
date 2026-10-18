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
//! The current process as the virtual memory system asks about it (`curproc`'s `struct
//! proc` and `struct process`, `p_ru`, its limits, its descriptors), still `bsd`'s until
//! `kern_proc`, `kern_resource` and the descriptors move (N4): one registered bridge,
//! [`VmProc`] (bridges.toml), whose every method is one call into `bsd` acting on the
//! thread that runs it. The checks and the arithmetic are the kernel's (`vm::mmap`,
//! `vm::unix`, `vm::map`, `vm::glue`): the bridge answers, it decides nothing.

use core::fmt;

use hal::bridges::AlreadyRegistered;
use hal::malloc::MArray;
use hal::once::OnceStatic;
use hal::sys::errno::Errno;

use super::types::VmProt;
use super::vnode::VnodeOps;
use crate::assert::kpanic;

/// `RLIMIT_DATA`: data size (`<sys/resource.h>`).
pub const RLIMIT_DATA: i32 = 2;
/// `RLIMIT_STACK`: stack size.
pub const RLIMIT_STACK: i32 = 3;
/// `RLIMIT_RSS`: resident set size.
pub const RLIMIT_RSS: i32 = 5;
/// `RLIMIT_MEMLOCK`: locked-in-memory address space.
pub const RLIMIT_MEMLOCK: i32 = 6;

/// `LOG_NOTICE`: normal but significant condition (`<sys/syslog.h>`).
pub const LOG_NOTICE: i32 = 5;

/// `BOGO_PC`: a program counter no thread can have (`<sys/proc.h>`, `(u_long)-1`).
pub const BOGO_PC: usize = usize::MAX;

/// `ps_kbind_addr` and `ps_kbind_cookie` of the current process, as `sys_kbind` reads and
/// sets them under `ps_mtx` ([`VmProc::with_kbind`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KbindSlot {
    /// `ps_kbind_addr`: the call site of the first `kbind(2)`, 0 before it, [`BOGO_PC`] when
    /// disabled.
    pub addr: usize,
    /// `ps_kbind_cookie`.
    pub cookie: u64,
}

/// The registered process bridge ([`install`]).
static VM_PROC: OnceStatic<&'static dyn VmProc> = OnceStatic::new();

/// `curproc`, as `uvm` sees it: a bridge (bridges.toml), registered once by `bsd`'s
/// `uvm_init`; every method acts on the thread that calls it and is one call of `bsd`'s.
pub trait VmProc: Sync {
    /// `pledge_protexec(p, prot)`: `EPERM` when `prot` asks for `PROT_EXEC` and the process
    /// pledged without `prot_exec`.
    fn pledge_protexec(&self, prot: VmProt) -> Result<(), Errno>;
    /// `uvm_wxcheck`'s exemption: `ps_textvp`'s mount is `MNT_WXALLOWED` and the process is
    /// `PSI_WXNEEDED`.
    fn wx_allowed(&self) -> bool;
    /// `pr->ps_wxcounter++`: the old value.
    fn wxcounter_inc(&self) -> u32;
    /// `sigexit(p, sig)`, which does not return (the caller holds the kernel lock).
    fn sigexit(&self, sig: i32) -> !;
    /// `trapsignal(p, SIGSEGV, 0, SEGV_ACCERR, sv)` with `sv.sival_ptr = pc`
    /// (`uvm_map_inentry`'s failure; the caller holds the kernel lock).
    fn trapsignal_segv(&self, pc: usize);
    /// `pr->ps_pid`.
    fn pid(&self) -> i32;
    /// `p->p_tid`.
    fn tid(&self) -> i32;
    /// `pr->ps_comm` into `out` (no NUL): the bytes written.
    fn comm(&self, out: &mut [u8]) -> usize;
    /// `PROC_PC(p)`: the user program counter of the trap that entered the kernel.
    fn pc(&self) -> usize;
    /// `atomic_setbits_int(&pr->ps_acflag, AMAP)` (`uvm_map_inentry`'s report).
    fn set_acflag_amap(&self);
    /// `lim_cur(which)`: the soft limit `which` (`RLIMIT_*`).
    fn lim_cur(&self, which: i32) -> u64;
    /// `P_HASSIBLING(p)`: the process has another thread.
    fn has_sibling(&self) -> bool;
    /// `pr->ps_strings`.
    fn ps_strings(&self) -> usize;
    /// `pr->ps_sigcode`.
    fn ps_sigcode(&self) -> usize;
    /// `pr->ps_timekeep`.
    fn ps_timekeep(&self) -> usize;
    /// `fd_getfile(p->p_fd, fd)` (`EBADF` when there is none), `f(vp, fp->f_flag)` with the
    /// file's vnode when it is `DTYPE_VNODE` (`None` otherwise), then `FRELE(fp, p)`; `f`'s
    /// result.
    fn with_fd_vnode(
        &self,
        fd: i32,
        f: &mut dyn FnMut(Option<&'static dyn VnodeOps>, u32) -> Result<(), Errno>,
    ) -> Result<(), Errno>;
    /// `mtx_enter(&pr->ps_mtx)`, `f` on the process's kbind slot, `mtx_leave`: the read, the
    /// compare and the update are one critical section, as the C's.
    fn with_kbind(&self, f: &mut dyn FnMut(&mut KbindSlot));
    /// `pr->ps_libcpin = { base, end, pins, npins }`: installs `pinsyscalls(2)`'s table.
    fn pin_table_set(&self, base: usize, end: usize, pins: MArray<u32>);
    /// `curproc->p_ru.ru_minflt++`.
    fn ru_minflt_inc(&self);
    /// `curproc->p_ru.ru_majflt++`.
    fn ru_majflt_inc(&self);
    /// `if (p->p_ru.ru_maxrss < kb) p->p_ru.ru_maxrss = kb`.
    fn ru_maxrss(&self, kb: usize);
    /// `log(level, ...)`: a line for `syslogd` (and the console), as `uvm_wxcheck`'s notice.
    fn log(&self, level: i32, args: fmt::Arguments<'_>);
}

/// Registers the process bridge, once (`bsd`'s `uvm_init`; the tests' doubles); a second call
/// keeps the first and names the bridge.
pub fn install(imp: &'static dyn VmProc) -> Result<(), AlreadyRegistered> {
    VM_PROC.set(imp).map_err(|_| AlreadyRegistered("VmProc"))
}

/// Whether the bridge is registered.
pub fn installed() -> bool {
    VM_PROC.is_set()
}

/// The current process; panics before [`install`]: only a system call, a fault or a core
/// dump asks, and none runs before `uvm_init`.
pub fn vmproc() -> &'static dyn VmProc {
    match VM_PROC.get() {
        Some(p) => *p,
        None => kpanic(format_args!("VmProc: curproc used before uvm_init")),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    /// The double registered by the tests' set-up answers per thread: two test threads are
    /// two processes.
    #[test]
    fn the_registered_process_answers_for_its_thread() {
        testutil::vmproc_with(|s| {
            s.pid = 42;
            s.limits.insert(RLIMIT_DATA, 1 << 20);
        });
        assert!(installed());
        assert_eq!(vmproc().pid(), 42);
        assert_eq!(vmproc().lim_cur(RLIMIT_DATA), 1 << 20);
        assert_eq!(vmproc().lim_cur(RLIMIT_STACK), testutil::RLIM_INFINITY);
        assert_eq!(vmproc().wxcounter_inc(), 0);
        assert_eq!(vmproc().wxcounter_inc(), 1);
        let other = std::thread::spawn(|| (vmproc().pid(), vmproc().wxcounter_inc()));
        assert_eq!(other.join().unwrap(), (0, 0));
        let mut seen = None;
        vmproc().with_kbind(&mut |k| {
            seen = Some(*k);
            k.addr = BOGO_PC;
            k.cookie = 7;
        });
        assert_eq!(seen, Some(KbindSlot::default()));
        vmproc().with_kbind(&mut |k| seen = Some(*k));
        assert_eq!(
            seen,
            Some(KbindSlot {
                addr: BOGO_PC,
                cookie: 7
            })
        );
        assert_eq!(
            vmproc().with_fd_vnode(3, &mut |_, _| Ok(())),
            Err(Errno::EBADF)
        );
    }
}
/* </TESTS> */
