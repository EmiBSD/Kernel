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
 * Copyright (c) 1997 Theo de Raadt. All rights reserved.
 * Copyright (c) 1982, 1986, 1989, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)kern_sig.c	8.7 (Berkeley) 4/18/94
 */
/* </LICENSES> */

/* <CODE> */
//! Signals: `kern/kern_sig.c`, `bsd`'s half over `kernel::sig`.
//!
//! Upstream: sys/kern/kern_sig.c @ 3ce1f3f79392
//!
//! N4.5b: the signals' logic is `kernel::sig` (`sys/kernel/sig.rs`): the actions, the
//! generation and delivery of signals, job control and the debugger's stops,
//! single-threading and the system calls' bodies. What stays here is what `bsd` still owns:
//! - the `sysent[]` rows (`sys_sigaction` ... `sys___thrsigdivert`, `sys_nosys`,
//!   `sys_sigreturn`), which decode their arguments and call the kernel's bodies (`sys_kill`
//!   checks `pledge_kill` first, as the C);
//! - the names the rest of `bsd` calls, with their LZ signatures over `bsd`'s `Proc`
//!   (`psignal`, `ptsignal`, `trapsignal`, `userret`, `sigabort`, `sigonstack`, ...),
//!   which pin a thread where the kernel makes it runnable;
//! - `coredump` (the file systems), reached by the kernel through `BsdProcGlue`;
//! - the `sigio` registrations (`sigio_*`, `pgsigio`) until they move too.
//!
//! ## Deviations
//! - Calls into subsystems that are not ported are reported with `unported!`: in
//!   `coredump`, the filesystem half: `vn_open` of the core file and everything after it
//!   (`VOP_GETATTR`/`VOP_SETATTR`, `coredump_elf`, `vn_close`) is reported and the dump fails
//!   with `ENOSYS`, so `sigexit` never sets `WCOREFLAG`; `coredump_write`'s `vn_rdwr` is
//!   reported likewise. `coredump` builds the core file name in a stack buffer instead of a
//!   `namei_pool` item, and `KERNELPATH` (`<sys/namei.h>`) is the boolean `incrash`; with no
//!   file descriptor table there is no `fd_rdir` to release.
//! - `KTRACE` is not configured.
//! - `sys_sigreturn` lives in `machdep.c` (amd64) and `sig_machdep.c` (arm64); the system
//!   call table finds `pub fn sys_*` only under `sys/kern` and `sys/uvm`, so the entry here
//!   forwards to `machine::MachineSignal::sys_sigreturn`.
//! - `sigio_setown`/`sigio_getown` take the `int` that `data` points to as `&i32`/`&mut i32`.
//!   A sigio's credentials are its own counted reference (`CredSlot`), given back by
//!   `sigio_del`.

use core::pin::Pin;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicI32, Ordering};

use crate::kassert;
use crate::kern::kern_lock::{mtx_enter, mtx_leave};
use crate::kern::kern_malloc::{free, malloc};
use crate::kern::kern_pledge::pledge_kill;
use crate::kern::kern_prot::suser;
use crate::kern::proc_glue::{BsdProcGlue, pgfind, prfind};
use crate::kern::sched_bsd::r#yield;
use crate::kern::subr_prf::{Str, log, panic, snprintf};
use crate::machine::Machine;
use crate::machine::cpu::curproc;
use crate::machine::intr::IPL_HIGH;
use crate::machine::signal::MachineSignal;
use crate::sys::errno::Errno;
use crate::sys::malloc::{M_SIGIO, M_WAITOK};
use crate::sys::mutex::{Mutex, mutex_assert_locked};
use crate::sys::param::{MAXPATHLEN, MAXPHYS, USPACE};
use crate::sys::proc::ProcExt;
use crate::sys::proc::{PS_CONTROLT, PS_COREDUMP, PS_EXITING, PS_SUGID, Pgrp, Proc, Process};
use crate::sys::resource::RLIMIT_CORE;
use crate::sys::resourcevar::lim_cur;
use crate::sys::siginfo::{Siginfo, Sigval};
use crate::sys::sigio::{Sigio, SigioRef, Sigiolst};
use crate::sys::signal::{SIGKILL, Sigaction, Sigaltstack, Sigset, sigmask};
use crate::sys::signalvar::SignalType;
use crate::sys::syscallargs::{
    SysKillArgs, SysSigactionArgs, SysSigaltstackArgs, SysSigprocmaskArgs, SysSigsuspendArgs,
    SysThrkillArgs, SysThrsigdivertArgs,
};
use crate::sys::syslog::LOG_ERR;
use crate::sys::systm::{SysArgs, kernel_lock, kernel_unlock, sysargs};
use crate::sys::ttycom::{TIOCGPGRP, TIOCSPGRP};
use crate::sys::types::{Pid, Register};
use crate::sys::ucred::{CredRef, Ucred};
use crate::unported;
use hal::klock;
use hal::pin::pin_static;
use hal::user::UserPtr;
use hal::vm::param::ptoa;
use kernel::sig::{UserTimespec, cansigio};
use kernel::vm::map::uvm_unmap;

pub use kernel::sig::{
    SIGACTS_POOL, SIGPROP, cansignal, cursig, dosigsuspend, execsigs, initsiginfo, killpg1,
    postsig, postsig_done, proc_stop_finish, proc_stop_setup, proc_suspend_check,
    proc_suspend_check_locked, proc_trap, process_continue, process_stop, process_suspend_signal,
    setsigctx, setsigvec, siginit, sigismasked, signal_init, sigstkinit, single_thread_clear,
    single_thread_set,
};

/// `struct coredump_iostate`: what `coredump_write` and `coredump_unmap` work on.
pub struct CoredumpIostate<'a> {
    /// `io_proc`.
    pub io_proc: &'a Proc,
    /// `io_vp`: the core file (`struct vnode`, not ported).
    pub io_vp: *const (),
    /// `io_cred`.
    pub io_cred: &'static Ucred,
    /// `io_offset`.
    pub io_offset: i64,
}

/// `nosuidcoredump` \[a\].
pub static NOSUIDCOREDUMP: AtomicI32 = AtomicI32::new(1);

/// `sigio_lock`.
static SIGIO_LOCK: Mutex = Mutex::new(IPL_HIGH);

/// The address of `pr`'s session (`pr->ps_session`), 0 without one: what the C compares.
fn session_addr(r: &klock::KernelReader, pr: &Process) -> usize {
    pr.session(r).map_or(0, |s| ptr::from_ref(s).addr())
}

/// The thread `p`, pinned, for the kernel's calls that make it runnable: a thread of `bsd`
/// is a `procpl` box its process owns (or `proc0`), at its address until `proc_free`
/// (`ProcExt::as_static`).
fn pinned(p: &Proc) -> Pin<&'static Proc> {
    pin_static(p.as_static())
}

/// `sigactsinit` of the new process `pr`: an initial sigacts structure with the same signal
/// state as `parent` (`kernel::sig::sigactsinit`).
pub fn sigactsinit(pr: &Process, parent: &Process) {
    kernel::sig::sigactsinit(pr, parent);
}

/// `sys_sigaction`: examine and change a signal's action.
pub fn sys_sigaction(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSigactionArgs = sysargs(v);
    kernel::sig::sigaction(
        p,
        uap.signum.get(),
        UserPtr::<Sigaction>::new(uap.nsa.get() as usize),
        UserPtr::<Sigaction>::new(uap.osa.get() as usize),
    )
}

/// `sys_sigprocmask`: manipulate signal mask. Note that we receive new mask, not pointer,
/// and return old mask as return value; the library stub does the rest.
pub fn sys_sigprocmask(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSigprocmaskArgs = sysargs(v);
    // The old mask is the return value even when `how` is refused, as the C stores it
    // first.
    retval[0] = p.p_sigmask.get() as Register;
    kernel::sig::sigprocmask(p, uap.how.get(), uap.mask.get()).map(|_| ())
}

/// `sys_sigpending`: the signals pending for the thread and its process.
pub fn sys_sigpending(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::sig::sigpending_all(p) as Register;
    Ok(())
}

/// `sys_sigsuspend`: suspend thread until signal, providing mask to be set in the meantime.
/// Note nonstandard calling convention: libc stub passes mask, not pointer, to save a copyin.
pub fn sys_sigsuspend(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSigsuspendArgs = sysargs(v);
    Err(kernel::sig::sigsuspend(p, uap.mask.get() as Sigset))
}

/// `sigonstack`: whether the user stack pointer `stack` is on curproc's alternate signal
/// stack.
pub fn sigonstack(stack: usize) -> bool {
    kernel::sig::sigonstack::<BsdProcGlue>(stack)
}

/// `sys_sigaltstack`: set and/or get the alternate signal stack.
pub fn sys_sigaltstack(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSigaltstackArgs = sysargs(v);
    kernel::sig::sigaltstack(
        p,
        UserPtr::<Sigaltstack>::new(uap.nss.get() as usize),
        UserPtr::<Sigaltstack>::new(uap.oss.get() as usize),
    )
}

/// `sys_kill`: send `signum` to a process, a process group or every process.
pub fn sys_kill(cp: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysKillArgs = sysargs(v);
    let pid = uap.pid.get();

    pledge_kill(cp, pid)?;
    kernel::sig::kill(cp, pid, uap.signum.get())
}

/// `sys_thrkill`: send `signum` to a thread of the calling process.
pub fn sys_thrkill(cp: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysThrkillArgs = sysargs(v);
    kernel::sig::thrkill(
        pinned(cp),
        uap.tid.get(),
        uap.signum.get(),
        uap.tcb.get() as usize,
    )
}

/// `pgsignal`: send a signal to a process group. If `checkctty` is set, limit to members
/// which have a controlling terminal.
pub fn pgsignal(pgrp: Option<&Pgrp>, signum: i32, checkctty: bool) {
    kernel::sig::pgsignal(pgrp, signum, checkctty);
}

/// `trapsignal`: send a signal caused by a trap to the current thread.
pub fn trapsignal(p: &Proc, signum: i32, trapno: u64, code: i32, sigval: Sigval) {
    kernel::sig::trapsignal(pinned(p), signum, trapno, code, sigval);
}

/// `psignal`: send the signal to the process of `p`.
pub fn psignal(p: &Proc, signum: i32) {
    kernel::sig::psignal(pinned(p), signum);
}

/// `prsignal`: send the signal to the process `pr`.
pub fn prsignal(pr: &Process, signum: i32) {
    kernel::sig::prsignal(pr, signum);
}

/// `ptsignal`: `type_` = `SPROCESS`: process signal, can be diverted (sigwait());
/// `STHREAD`: thread signal, but should be propagated if unhandled.
pub fn ptsignal(p: &Proc, signum: i32, type_: SignalType) {
    kernel::sig::ptsignal(pinned(p), signum, type_);
}

/// `sigexit`: force the current process to exit with the specified signal, dumping core if
/// appropriate; never returns.
pub fn sigexit(p: &Proc, signum: i32) -> ! {
    kernel::sig::sigexit(p, signum)
}

/// `sigabort`: send uncatchable SIGABRT for coredump.
pub fn sigabort(p: &Proc) {
    kernel::sig::sigabort(pinned(p));
}

/// `userret`: the last thing before a thread returns to user mode.
pub fn userret(p: &Proc) {
    kernel::sig::userret(pinned(p));
}

/// `sys_nosys`: nonexistent system call-- signal process (may want to handle it). Flag error
/// in case process won't see signal immediately (blocked or ignored).
pub fn sys_nosys(p: &Proc, _v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    Err(kernel::sig::nosys(pinned(p)))
}

/// `sys___thrsigdivert`: `sigwait(3)`'s system call: wait for one of the signals in
/// `sigmask`, which are taken out of the normal delivery.
#[allow(non_snake_case)] // the generator finds the C's name, `sys___thrsigdivert`
pub fn sys___thrsigdivert(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysThrsigdivertArgs = sysargs(v);
    let signo = kernel::sig::thrsigdivert(
        p,
        uap.sigmask.get(),
        UserPtr::<Siginfo>::new(uap.info.get() as usize),
        UserPtr::<UserTimespec>::new(uap.timeout.get() as usize),
    )?;
    retval[0] = signo as Register;
    Ok(())
}

/// `sys_sigreturn`: the machine-dependent `sigreturn(2)` (see the module's deviations).
pub fn sys_sigreturn(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    <Machine as MachineSignal>::sys_sigreturn(p, v, retval)
}

/// `pgsigio`: send a SIGIO or SIGURG signal to a process or process group using stored
/// credentials rather than those of the current process.
pub fn pgsigio(sir: &SigioRef, sig: i32, checkctty: bool) {
    if sir.sir_sigio.get().is_null() {
        return;
    }

    kernel_lock(); // KERNEL_LOCK()
    mtx_enter(&SIGIO_LOCK);
    // SAFETY: a registered sigio stays allocated until `sigio_del`, which runs after it is
    // unlinked under `sigio_lock`.
    if let Some(sigio) = unsafe { sir.sir_sigio.get().as_ref() }
        && let Some(cred) = sigio.sio_ucred.cred()
    {
        let cred: &Ucred = &cred;
        if sigio.sio_pgid.get() > 0 {
            // SAFETY: the process unlinks its sigios (`sigio_freelist`) before it goes.
            if let Some(pr) = unsafe { sigio.sio_proc.get().as_ref() }
                && cansigio(cred, pr)
            {
                prsignal(pr, sig);
            }
        } else if sigio.sio_pgid.get() < 0 {
            // SAFETY: as above, for the process group.
            if let Some(pg) = unsafe { sigio.sio_pgrp.get().as_ref() } {
                let r = klock::reader();
                for pr in pg.pg_members.iter(&r) {
                    if cansigio(cred, pr)
                        && (!checkctty || pr.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT != 0)
                    {
                        prsignal(pr, sig);
                    }
                }
            }
        }
    }
    // out:
    mtx_leave(&SIGIO_LOCK);
    kernel_unlock(); // KERNEL_UNLOCK()
}

/// `coredump`: dump core, into a file named "progname.core", unless the process was
/// setuid/setgid. The file is written by the filesystems, which are not ported: see the
/// module's deviations.
pub fn coredump(p: &Proc) -> Result<(), Errno> {
    // SMALL_KERNEL: not configured.
    let pr = p.process();
    let vm = p.vmspace();
    let dir = "/var/crash";
    let nosuidcoredump_local = NOSUIDCOREDUMP.load(Ordering::Relaxed);

    pr.ps_flags.fetch_or(PS_COREDUMP, Ordering::Relaxed);

    // PMAP_CHECK_COPYIN: not configured.

    // Don't dump if will exceed file size limit.
    let size = USPACE
        + ptoa(
            vm.vm_dsize.load(Ordering::Relaxed) as usize
                + vm.vm_ssize.load(Ordering::Relaxed) as usize,
        );
    if size as u64 >= lim_cur(RLIMIT_CORE) {
        return Err(Errno::EFBIG);
    }

    // name = pool_get(&namei_pool, PR_WAITOK): a stack buffer (see the module's deviations).
    let mut name = [0u8; MAXPATHLEN];
    let incrash: bool;
    let len;

    // If the process has inconsistent uids, nosuidcoredump determines coredump placement
    // policy.
    let sugid = pr.ps_flags.load(Ordering::Relaxed) & PS_SUGID != 0;
    if (sugid && suser(p).is_err()) || (sugid && nosuidcoredump_local != 0) {
        if nosuidcoredump_local == 3 {
            // If the program directory does not exist, dumps of that core will silently
            // fail.
            len = snprintf(
                &mut name,
                format_args!("{}/{}/{}.core", dir, Str(&pr.comm()), pr.ps_pid as u32),
            );
            incrash = true; // KERNELPATH
        } else if nosuidcoredump_local == 2 {
            len = snprintf(&mut name, format_args!("{}/{}.core", dir, Str(&pr.comm())));
            incrash = true; // KERNELPATH
        } else {
            return Err(Errno::EPERM);
        }
    } else {
        len = snprintf(&mut name, format_args!("{}.core", Str(&pr.comm())));
        incrash = false;
    }

    if len >= MAXPATHLEN {
        return Err(Errno::EACCES);
    }

    // Control the UID used to write out. The normal case uses the real UID. If the sugid
    // case is going to write into the controlled directory, we do so as root.
    let cred: CredRef = if !incrash {
        let mut c = (*p.ucred()).clone();
        let (ruid, rgid) = (c.cr_ruid.get(), c.cr_rgid.get());
        c.cr_uid.set(ruid);
        c.cr_gid.set(rgid);
        CredRef::new(c)
    } else {
        // p->p_fd->fd_rdir: no file descriptor table, so no root directory to release.
        // The thread's own credentials become a copy with root's user and group
        // (`p->p_ucred = crdup(p->p_ucred)` and the stores through it).
        let mut c = (*p.ucred()).clone();
        c.cr_uid.set(0);
        c.cr_gid.set(0);
        let c = CredRef::new(c);
        drop(p.set_cred(c.clone()));
        c
    };

    // incrash should be 0 or KERNELPATH only
    // NDINIT(&nd, 0, BYPASSUNVEIL | incrash, UIO_SYSSPACE, name, p) and vn_open(&nd,
    // O_CREAT | FWRITE | O_NOFOLLOW | O_NONBLOCK, S_IRUSR | S_IWUSR); then the regular-file,
    // link-count, mode and owner checks (VOP_GETATTR), the truncation (VOP_SETATTR), ACORE,
    // vn_close and coredump_elf(p, &io), which sets ACORE: the filesystems (see the module's
    // deviations).
    let error = unported!("coredump: vn_open of the core file (vfs)");

    // out:
    drop(cred);
    Err(error)
}

/// `coredump_write`: writes `len` bytes at `data` (kernel or user space, as `segflg` says)
/// to the core file, in `MAXPHYS` chunks, giving up when `SIGKILL` arrives.
pub fn coredump_write(
    io: &mut CoredumpIostate<'_>,
    _segflg: i32,
    _data: *const u8,
    len: usize,
    isvnode: bool,
) -> Result<(), Errno> {
    let mut coffset = 0usize;
    let mut csize = len;

    loop {
        if sigmask(SIGKILL)
            & (io.io_proc.p_siglist.load(Ordering::Relaxed)
                | io.io_proc.process().ps_siglist.load(Ordering::Relaxed))
            != 0
        {
            return Err(Errno::EINTR);
        }

        // Rest of the loop sleeps with lock held, so...
        r#yield();

        let chunk = csize.min(MAXPHYS);
        // vn_rdwr(UIO_WRITE, io->io_vp, data + coffset, chunk, io->io_offset + coffset,
        // segflg, IO_UNIT, io->io_cred, NULL, io->io_proc): the filesystems.
        let error = unported!("coredump_write: vn_rdwr (vfs)");
        if error != Errno::EFAULT || !isvnode {
            let pr = io.io_proc.process();

            if error == Errno::ENOSPC {
                log(
                    LOG_ERR,
                    format_args!(
                        "coredump of {}({}) failed, filesystem full\n",
                        Str(&pr.comm()),
                        pr.ps_pid
                    ),
                );
            } else {
                log(
                    LOG_ERR,
                    format_args!(
                        "coredump of {}({}), write failed: errno {}\n",
                        Str(&pr.comm()),
                        pr.ps_pid,
                        error as i32
                    ),
                );
            }
            return Err(error);
        }

        coffset += chunk;
        csize -= chunk;
        if csize == 0 {
            break;
        }
    }

    let _ = coffset;
    io.io_offset += len as i64;
    Ok(())
}

/// `coredump_unmap`: unmaps what `coredump_elf` is done with.
pub fn coredump_unmap(io: &CoredumpIostate<'_>, start: usize, end: usize) {
    uvm_unmap(&io.io_proc.vmspace().vm_map, start, end);
}

/// `sigio_del`: frees the sigios on `rmlist`.
pub fn sigio_del(rmlist: &Sigiolst) {
    while let Some(sigio) = rmlist.pop_head() {
        // The sigio's reference to its credentials, dropped here.
        drop(sigio.sio_ucred.take());
        free(
            NonNull::from(sigio).cast::<u8>(),
            M_SIGIO,
            size_of::<Sigio>(),
        );
    }
}

/// `sigio_unlink`: detaches the sigio of `sir` from its owner and moves it to `rmlist`.
pub fn sigio_unlink(sir: &SigioRef, rmlist: &Sigiolst) {
    mutex_assert_locked(&SIGIO_LOCK, "sigio_unlink");

    // SAFETY: a registered sigio stays allocated until `sigio_del`, after this.
    if let Some(sigio) = unsafe { sir.sir_sigio.get().as_ref() } {
        kassert!(ptr::eq(sigio.sio_myref.get(), sir));
        sir.sir_sigio.set(ptr::null());

        // SAFETY: a registered sigio is on its owner's list, under `sigio_lock`, and the
        // owner is alive while it is: `exit1` and `pgdelete` empty the list
        // (`sigio_freelist`) before the process or group is freed; the sign of `sio_pgid`
        // says which of `sio_proc`/`sio_pgrp` names the owner (the C's union).
        let owner: &Sigiolst = unsafe {
            if sigio.sio_pgid.get() > 0 {
                &(&*sigio.sio_proc.get()).ps_sigiolst
            } else {
                &(&*sigio.sio_pgrp.get()).pg_sigiolst
            }
        };
        if sigio.sio_pgid.get() > 0 {
            sigio.sio_proc.set(ptr::null());
        } else {
            sigio.sio_pgrp.set(ptr::null());
        }
        // It moves to `rmlist`, which outlives it until `sigio_del`.
        owner.remove(sigio);
        rmlist.insert_head(sigio);
    }
}

/// `sigio_free`: unregisters `sir`.
pub fn sigio_free(sir: &SigioRef) {
    if sir.sir_sigio.get().is_null() {
        return;
    }

    let rmlist = Sigiolst::new();

    mtx_enter(&SIGIO_LOCK);
    sigio_unlink(sir, &rmlist);
    mtx_leave(&SIGIO_LOCK);

    sigio_del(&rmlist);
}

/// `sigio_freelist`: unregisters every sigio on a process's or group's list.
pub fn sigio_freelist(sigiolst: &Sigiolst) {
    if sigiolst.is_empty() {
        return;
    }

    let rmlist = Sigiolst::new();

    mtx_enter(&SIGIO_LOCK);
    while let Some(sigio) = sigiolst.first() {
        // SAFETY: a sigio's `sio_myref` names the registration that holds it.
        sigio_unlink(unsafe { &*sigio.sio_myref.get() }, &rmlist);
    }
    mtx_leave(&SIGIO_LOCK);

    sigio_del(&rmlist);
}

/// `sigio_setown`: registers the process (`*data` > 0) or process group (< 0, or
/// `TIOCSPGRP`) to receive `SIGIO`/`SIGURG` through `sir`; 0 unregisters.
pub fn sigio_setown(sir: &SigioRef, cmd: u64, data: &i32) -> Result<(), Errno> {
    let Some(p) = curproc() else {
        return Err(Errno::ESRCH);
    };
    let mut pgid: Pid = *data;

    if pgid == 0 {
        sigio_free(sir);
        return Ok(());
    }

    if cmd == TIOCSPGRP {
        if pgid < 0 {
            return Err(Errno::EINVAL);
        }
        pgid = -pgid;
    }

    let Some(mem) = malloc(size_of::<Sigio>(), M_SIGIO, M_WAITOK) else {
        panic(format_args!("sigio_setown: malloc"));
    };
    let new = mem.cast::<Sigio>();
    // SAFETY: a fresh allocation of `size_of::<Sigio>()` bytes, aligned by malloc(9),
    // written once before anything else sees it.
    unsafe { new.as_ptr().write(Sigio::new()) };
    // SAFETY: as above; it lives until `sigio_del` (or the failure path below) frees it.
    let sigio: &'static Sigio = unsafe { new.as_ref() };
    sigio.sio_pgid.set(pgid);
    sigio.sio_ucred.set(p.cred_ref());
    sigio.sio_myref.set(sir);

    let rmlist = Sigiolst::new();

    // The kernel lock, and not sleeping between prfind()/pgfind() and linking of the sigio
    // ensure that the process or process group does not disappear unexpectedly.
    kernel_lock(); // KERNEL_LOCK()
    mtx_enter(&SIGIO_LOCK);

    let result = 'fail: {
        let r = klock::reader();
        if pgid > 0 {
            let Some(pr) = prfind(&r, pgid) else {
                break 'fail Err(Errno::ESRCH);
            };

            // Policy - Don't allow a process to FSETOWN a process in another session.
            //
            // Remove this test to allow maximum flexibility or restrict FSETOWN to the
            // current process or process group for maximum safety.
            if session_addr(&r, pr) != session_addr(&r, p.process()) {
                break 'fail Err(Errno::EPERM);
            }

            if pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING != 0 {
                break 'fail Err(Errno::ESRCH);
            }
            sigio.sio_proc.set(pr);
            // A new sigio on no list; it stays until unlinked by `sigio_unlink`.
            pr.ps_sigiolst.insert_head(sigio);
        } else {
            // if (pgid < 0)
            let Some(pgrp) = pgfind(&r, -pgid) else {
                break 'fail Err(Errno::ESRCH);
            };

            // Policy - Don't allow a process to FSETOWN a process in another session.
            //
            // Remove this test to allow maximum flexibility or restrict FSETOWN to the
            // current process or process group for maximum safety.
            let pgsess = pgrp.session(&r).map_or(0, |s| ptr::from_ref(s).addr());
            if pgsess != session_addr(&r, p.process()) {
                break 'fail Err(Errno::EPERM);
            }
            sigio.sio_pgrp.set(pgrp);
            pgrp.pg_sigiolst.insert_head(sigio);
        }
        Ok(())
    };

    if let Err(error) = result {
        // fail:
        mtx_leave(&SIGIO_LOCK);
        kernel_unlock(); // KERNEL_UNLOCK()

        // The reference taken above.
        drop(sigio.sio_ucred.take());
        free(new.cast::<u8>(), M_SIGIO, size_of::<Sigio>());

        return Err(error);
    }

    sigio_unlink(sir, &rmlist);
    sir.sir_sigio.set(sigio);

    mtx_leave(&SIGIO_LOCK);
    kernel_unlock(); // KERNEL_UNLOCK()

    sigio_del(&rmlist);

    Ok(())
}

/// `sigio_getown`: the pgid registered through `sir` (negated for `TIOCGPGRP`).
pub fn sigio_getown(sir: &SigioRef, cmd: u64, data: &mut i32) {
    let mut pgid: Pid = 0;

    mtx_enter(&SIGIO_LOCK);
    // SAFETY: a registered sigio stays allocated while `sigio_lock` is held.
    if let Some(sigio) = unsafe { sir.sir_sigio.get().as_ref() } {
        pgid = sigio.sio_pgid.get();
    }
    mtx_leave(&SIGIO_LOCK);

    if cmd == TIOCGPGRP {
        pgid = -pgid;
    }

    *data = pgid;
}

/// `sigio_copy`: registers the owner of `src` in `dst` too.
pub fn sigio_copy(dst: &SigioRef, src: &SigioRef) {
    sigio_free(dst);

    if src.sir_sigio.get().is_null() {
        return;
    }

    let Some(mem) = malloc(size_of::<Sigio>(), M_SIGIO, M_WAITOK) else {
        panic(format_args!("sigio_copy: malloc"));
    };
    let new = mem.cast::<Sigio>();
    // SAFETY: a fresh allocation, written once before anything else sees it.
    unsafe { new.as_ptr().write(Sigio::new()) };
    // SAFETY: as above.
    let newsigio: &'static Sigio = unsafe { new.as_ref() };
    let rmlist = Sigiolst::new();

    mtx_enter(&SIGIO_LOCK);

    // SAFETY: a registered sigio stays allocated while `sigio_lock` is held.
    let Some(sigio) = (unsafe { src.sir_sigio.get().as_ref() }) else {
        mtx_leave(&SIGIO_LOCK);
        free(new.cast::<u8>(), M_SIGIO, size_of::<Sigio>());
        return;
    };

    newsigio.sio_pgid.set(sigio.sio_pgid.get());
    // The source holds a reference to its credentials: one more for the copy.
    if let Some(cr) = sigio.sio_ucred.cred() {
        newsigio.sio_ucred.set(cr);
    }
    newsigio.sio_myref.set(dst);
    if newsigio.sio_pgid.get() > 0 {
        newsigio.sio_proc.set(sigio.sio_proc.get());
        // SAFETY: the owner is alive (its sigio is registered); the new sigio is on no
        // list.
        unsafe {
            (&*newsigio.sio_proc.get())
                .ps_sigiolst
                .insert_head(newsigio)
        };
    } else {
        newsigio.sio_pgrp.set(sigio.sio_pgrp.get());
        // SAFETY: as above, for the process group.
        unsafe {
            (&*newsigio.sio_pgrp.get())
                .pg_sigiolst
                .insert_head(newsigio)
        };
    }

    sigio_unlink(dst, &rmlist);
    dst.sir_sigio.set(newsigio);

    mtx_leave(&SIGIO_LOCK);

    sigio_del(&rmlist);
}
/* </CODE> */
