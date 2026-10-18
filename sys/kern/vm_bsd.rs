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
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993 The Regents of the University of California.
 * Copyright (c) 1988 University of Utah.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
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
 * from: Utah $Hdr: vm_mmap.c 1.6 91/10/21$
 *      @(#)vm_mmap.c   8.5 (Berkeley) 5/19/94
 * from: Id: uvm_mmap.c,v 1.1.2.14 1998/01/05 21:04:26 chuck Exp
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993 The Regents of the University of California.
 * Copyright (c) 1988 University of Utah.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
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
 * from: Utah $Hdr: vm_unix.c 1.1 89/11/07$
 *      @(#)vm_unix.c   8.1 (Berkeley) 6/11/93
 * from: Id: uvm_unix.c,v 1.1.2.2 1997/08/25 18:52:30 chuck Exp
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993, The Regents of the University of California.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * The Mach Operating System project at Carnegie-Mellon University.
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
 *	@(#)vm_glue.c	8.6 (Berkeley) 1/5/94
 * from: Id: uvm_glue.c,v 1.1.2.8 1998/02/07 01:16:54 chs Exp
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
 *
 * Permission to use, copy, modify and distribute this software and
 * its documentation is hereby granted, provided that both the copyright
 * notice and this permission notice appear in all copies of the
 * software, derivative works or modified versions, and any portions
 * thereof, and that both notices appear in supporting documentation.
 *
 * CARNEGIE MELLON ALLOWS FREE USE OF THIS SOFTWARE IN ITS "AS IS"
 * CONDITION.  CARNEGIE MELLON DISCLAIMS ANY LIABILITY OF ANY KIND
 * FOR ANY DAMAGES WHATSOEVER RESULTING FROM THE USE OF THIS SOFTWARE.
 *
 * Carnegie Mellon requests users of this software to return to
 *
 *  Software Distribution Coordinator  or  Software.Distribution@CS.CMU.EDU
 *  School of Computer Science
 *  Carnegie Mellon University
 *  Pittsburgh PA 15213-3890
 *
 * any improvements or extensions that they make and grant Carnegie the
 * rights to redistribute these changes.
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1982, 1986, 1989, 1993
 *      The Regents of the University of California.
 *
 * All rights reserved.
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
 *      @(#)vm_meter.c  8.4 (Berkeley) 1/4/94
 * from: Id: uvm_meter.c,v 1.1.2.1 1997/08/14 19:10:35 chuck Exp
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm_init.c,v 1.1.2.3 1998/02/06 05:15:27 chs Exp
 */
/* </LICENSES> */

/* <CODE> */
//! `bsd`'s side of the kernel's virtual memory system (`kernel::vm`, N3.5): what still
//! belongs to `struct proc`, the system call table, the sysctl helpers and the boot order
//! until those move (N4), each a thin call into the kernel or one call of `bsd`'s.
//!
//! Upstream: sys/uvm/uvm_mmap.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_unix.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_glue.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_meter.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_init.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_mmap.rs@3c62ede68802
//! LZ: sys/uvm/uvm_unix.rs@3c62ede68802
//! LZ: sys/uvm/uvm_glue.rs@3c62ede68802
//! LZ: sys/uvm/uvm_meter.rs@3c62ede68802
//! LZ: sys/uvm/uvm_init.rs@3c62ede68802
//!
//! - The bridges the kernel's vm reaches `bsd` through: [`BsdVmProc`] (`kernel::vm::proc::
//!   VmProc`, curproc's facts) and [`BsdVmSysctl`] (`kernel::vm::meter::VmSysctl`, the
//!   sysctl helpers, `nkmempages`, `averunnable`, `allproc`), one call of `bsd`'s each.
//! - The fifteen `sysent[]` entries of the vm system calls (`sys_mmap`, ..., `sys_obreak`):
//!   the `sysargs` unpacked and handed to `kernel::vm::mmap`/`unix` with the process's
//!   address space; and their `Native` entries for the ABI module (`native_mmap`, ...,
//!   [`VM_ENTRIES`], `KernelApi::vm`), which run `mi_syscall` on the same row (pipe's
//!   precedent, `pipe_glue.rs`), so the prologue (`refreshcreds`, the pinned syscalls,
//!   pledge, the kernel lock rule) is the `Legacy` path's.
//! - `uvm_init` in the C's order around the kernel's two steps, `uvm_purge`, `uvm_exit` and
//!   `uvm_init_limits` (operations on `struct proc`, `struct process` and `struct plimit`).
//!
//! ## Redesign
//! - N3.5 S1: written dead (nothing registers the bridges or names these entries until the
//!   flip, S2, which also deletes `sys/uvm/` and moves the rest of its `bsd` remainders here:
//!   the `VnodeOps` impl and the `uvn_attach`/`uvm_vnp_*` wrappers of `uvm_vnode.rs`,
//!   `BsdBufCacheStats`, `uvm_pagewait`, `uvm_pagealloc_multi`, `uvm_shutdown`,
//!   `paddr_is_dma_reachable`, `vm_physmem`, the `kp_dma*` modes). Three helpers name what
//!   the flip changes in `struct proc`: [`vmspace_of`] (`p->p_vmspace` as the kernel's
//!   `Vmspace`), [`vmspace_take`] (the process's owning `VmspaceRef`, G5) and
//!   [`pin_install`] (the owned `pinsyscalls(2)` table, G10). No `unsafe`: every call here is
//!   a safe function of `bsd`, `hal` or `kernel`.

use core::fmt;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::malloc::MArray;
use hal::syscall::SysResult;
use hal::user::UserPtr;
use kernel::api::VmEntries;
use kernel::vm::init::VmMachine;
use kernel::vm::map::{Vmspace as KVmspace, VmspaceRef, uvmspace_free, uvmspace_purge};
use kernel::vm::meter::{ProcState, SysctlReq, VmSysctl};
use kernel::vm::mmap::MmapArgs;
use kernel::vm::proc::{KbindSlot, VmProc};
use kernel::vm::types::VmProt;
use kernel::vm::vnode::VnodeOps;

use crate::kern::init_sysent::SYSENT;
use crate::kern::kern_descrip::fd_getfile;
use crate::kern::kern_lock::{mtx_enter, mtx_leave};
use crate::kern::kern_malloc::nkmempages;
use crate::kern::kern_pledge::pledge_protexec;
use crate::kern::kern_proc::ALLPROC;
use crate::kern::kern_sig::{sigexit, trapsignal};
use crate::kern::kern_sysctl::{sysctl_int, sysctl_rdint, sysctl_rdstruct, sysctl_string};
use crate::kern::sched_bsd::averunnable;
use crate::kern::subr_prf::{log, panic};
use crate::machine::cpu::{Cpu, curproc};
use crate::machine::exec::MachineExec;
use crate::machine::param::MachineParam;
use crate::machine::{Machine, Pmap};
use crate::sys::acct::AMAP;
use crate::sys::errno::Errno;
use crate::sys::file::{DTYPE_VNODE, frele};
use crate::sys::mount::MNT_WXALLOWED;
use crate::sys::param::{USPACE, USPACE_ALIGN};
use crate::sys::proc::{PSI_WXNEEDED, Proc, Process, p_hassibling};
use crate::sys::resource::{RLIMIT_DATA, RLIMIT_RSS, RLIMIT_STACK};
use crate::sys::resourcevar::{Plimit, lim_cur};
use crate::sys::siginfo::{SEGV_ACCERR, Sigval};
use crate::sys::signal::SIGSEGV;
use crate::sys::syscall::{
    SYS_break, SYS_kbind, SYS_madvise, SYS_mimmutable, SYS_minherit, SYS_mlock, SYS_mlockall,
    SYS_mmap, SYS_mprotect, SYS_mquery, SYS_msync, SYS_munlock, SYS_munlockall, SYS_munmap,
    SYS_pinsyscalls,
};
use crate::sys::syscall_mi::mi_syscall;
use crate::sys::syscallargs::{
    SysKbindArgs, SysMadviseArgs, SysMimmutableArgs, SysMinheritArgs, SysMlockArgs,
    SysMlockallArgs, SysMmapArgs, SysMprotectArgs, SysMqueryArgs, SysMsyncArgs, SysMunlockArgs,
    SysMunmapArgs, SysObreakArgs, SysPinsyscallsArgs,
};
use crate::sys::sysctl::SysctlPlain;
use crate::sys::systm::{SysArgs, kernel_assert_unlocked, sysargs};
use crate::sys::types::{Register, Rlim};

/// `curproc` as `kernel::vm` asks about it (`kernel::vm::proc::VmProc`).
pub struct BsdVmProc;

/// `CTL_VM`'s reach into `bsd` (`kernel::vm::meter::VmSysctl`).
pub struct BsdVmSysctl;

/// The registered process bridge.
pub static BSD_VM_PROC: BsdVmProc = BsdVmProc;

/// The registered sysctl bridge.
pub static BSD_VM_SYSCTL: BsdVmSysctl = BsdVmSysctl;

/// The machine's constants `kernel::vm` reads (`kernel::vm::init::VmMachine`), handed down
/// by [`uvm_init`].
pub static VM_MACHINE: VmMachine = VmMachine {
    ldpgsz: <Machine as MachineExec>::LDPGSZ,
    uspace: USPACE,
    uspace_align: USPACE_ALIGN,
    have_uspace_guard: <Machine as MachineParam>::HAVE_USPACE_GUARD,
    dma_constraint: <Machine as Pmap>::DMA_CONSTRAINT,
};

/// `KernelApi::vm`: the `Native` entries of the vm system calls.
pub const VM_ENTRIES: VmEntries = VmEntries {
    obreak: native_obreak,
    mmap: native_mmap,
    munmap: native_munmap,
    mprotect: native_mprotect,
    madvise: native_madvise,
    mquery: native_mquery,
    kbind: native_kbind,
    pinsyscalls: native_pinsyscalls,
    mimmutable: native_mimmutable,
    mlock: native_mlock,
    munlock: native_munlock,
    minherit: native_minherit,
    msync: native_msync,
    mlockall: native_mlockall,
    munlockall: native_munlockall,
};

/// The thread on this CPU; a system call or a fault always has one.
fn curp(who: &str) -> &'static Proc {
    match curproc() {
        Some(p) => p,
        None => panic(format_args!("{who}: no curproc")),
    }
}

/// `p->p_vmspace`, the kernel's address space (the flip, S2, types the slot so).
fn vmspace_of(p: &Proc) -> &'static KVmspace {
    p.vmspace()
}

/// `pr->ps_vmspace` taken out of the process (`NULL` left), the owning handle with it (G5:
/// the raw slot beside its `VmspaceRef`, S2).
fn vmspace_take(pr: &Process) -> Option<VmspaceRef> {
    pr.take_vmspace()
}

/// `pr->ps_libcpin = { base, end, pins, npins }` with the table owned (G10, S2).
fn pin_install(pr: &Process, base: usize, end: usize, pins: MArray<u32>) {
    pr.ps_libcpin.install(base, end, pins);
}

impl VmProc for BsdVmProc {
    fn pledge_protexec(&self, prot: VmProt) -> Result<(), Errno> {
        pledge_protexec(curp("pledge_protexec"), prot)
    }

    fn wx_allowed(&self) -> bool {
        let pr = curp("uvm_wxcheck").process();
        let wxallowed = pr
            .ps_textvp
            .get()
            .and_then(|vp| vp.v_mount.get())
            .is_some_and(|mp| mp.mnt_flag.get() & MNT_WXALLOWED != 0);
        wxallowed && pr.ps_iflags.get() & PSI_WXNEEDED != 0
    }

    fn wxcounter_inc(&self) -> u32 {
        let pr = curp("uvm_wxcheck").process();
        let n = pr.ps_wxcounter.get();
        pr.ps_wxcounter.set(n + 1);
        n as u32
    }

    fn sigexit(&self, sig: i32) -> ! {
        sigexit(curp("sigexit"), sig)
    }

    fn trapsignal_segv(&self, pc: usize) {
        trapsignal(
            curp("trapsignal"),
            SIGSEGV,
            0,
            SEGV_ACCERR,
            Sigval::from_ptr(pc),
        );
    }

    fn pid(&self) -> i32 {
        curp("pid").process().ps_pid.get()
    }

    fn tid(&self) -> i32 {
        curp("tid").p_tid.get()
    }

    fn comm(&self, out: &mut [u8]) -> usize {
        let comm = curp("comm").process().comm();
        let n = comm.len().min(out.len());
        out[..n].copy_from_slice(&comm[..n]);
        n
    }

    fn pc(&self) -> usize {
        <Machine as Cpu>::proc_pc(curp("PROC_PC"))
    }

    fn set_acflag_amap(&self) {
        let pr = curp("ps_acflag").process();
        pr.ps_acflag.set(pr.ps_acflag.get() | AMAP);
    }

    fn lim_cur(&self, which: i32) -> u64 {
        lim_cur(which as usize)
    }

    fn has_sibling(&self) -> bool {
        p_hassibling(curp("P_HASSIBLING"))
    }

    fn ps_strings(&self) -> usize {
        curp("ps_strings").process().ps_strings.get()
    }

    fn ps_sigcode(&self) -> usize {
        curp("ps_sigcode").process().ps_sigcode.get()
    }

    fn ps_timekeep(&self) -> usize {
        curp("ps_timekeep").process().ps_timekeep.get()
    }

    fn with_fd_vnode(
        &self,
        fd: i32,
        f: &mut dyn FnMut(Option<&'static dyn VnodeOps>, u32) -> Result<(), Errno>,
    ) -> Result<(), Errno> {
        let p = curp("fd_getfile");
        let Some(fp) = fd_getfile(p.fd(), fd) else {
            return Err(Errno::EBADF);
        };
        let vp: Option<&'static dyn VnodeOps> = if fp.f_type.get() == DTYPE_VNODE {
            Some(fp.vnode())
        } else {
            None
        };
        let r = f(vp, fp.flag() as u32);
        let _ = frele(fp, p);
        r
    }

    fn with_kbind(&self, f: &mut dyn FnMut(&mut KbindSlot)) {
        let pr = curp("kbind").process();
        mtx_enter(&pr.ps_mtx);
        let mut slot = KbindSlot {
            addr: pr.ps_kbind_addr.get(),
            cookie: pr.ps_kbind_cookie.get() as u64,
        };
        f(&mut slot);
        pr.ps_kbind_addr.set(slot.addr);
        pr.ps_kbind_cookie.set(slot.cookie as i64);
        mtx_leave(&pr.ps_mtx);
    }

    fn pin_table_set(&self, base: usize, end: usize, pins: MArray<u32>) {
        pin_install(curp("pinsyscalls").process(), base, end, pins);
    }

    fn ru_minflt_inc(&self) {
        if let Some(p) = curproc() {
            p.p_ru.ru_minflt.set(p.p_ru.ru_minflt.get() + 1);
        }
    }

    fn ru_majflt_inc(&self) {
        if let Some(p) = curproc() {
            p.p_ru.ru_majflt.set(p.p_ru.ru_majflt.get() + 1);
        }
    }

    fn ru_maxrss(&self, kb: usize) {
        if let Some(p) = curproc() {
            let kb = i64::try_from(kb).unwrap_or(i64::MAX);
            if p.p_ru.ru_maxrss.get() < kb {
                p.p_ru.ru_maxrss.set(kb);
            }
        }
    }

    fn log(&self, level: i32, args: fmt::Arguments<'_>) {
        log(level, args);
    }
}

impl VmSysctl for BsdVmSysctl {
    fn rdint(&self, req: &mut SysctlReq<'_>, v: i32) -> Result<(), Errno> {
        sysctl_rdint(req.oldp, req.oldlenp, req.newp, v)
    }

    fn int(&self, req: &mut SysctlReq<'_>, v: &mut i32) -> Result<(), Errno> {
        let t = AtomicI32::new(*v);
        let r = sysctl_int(req.oldp, req.oldlenp, req.newp, req.newlen, &t);
        *v = t.into_inner();
        r
    }

    fn rdstruct(&self, req: &mut SysctlReq<'_>, bytes: &[u8]) -> Result<(), Errno> {
        sysctl_rdstruct(req.oldp, req.oldlenp, req.newp, bytes)
    }

    fn rdstruct_uvmexp(
        &self,
        req: &mut SysctlReq<'_>,
        uexp: &hal::vm::uvmexp::UvmexpCopy,
    ) -> Result<(), Errno> {
        sysctl_rdstruct(req.oldp, req.oldlenp, req.newp, uexp.as_bytes())
    }

    fn string(&self, req: &mut SysctlReq<'_>, buf: &mut [u8]) -> Result<(), Errno> {
        sysctl_string(req.oldp, req.oldlenp, req.newp, req.newlen, buf)
    }

    fn nkmempages(&self) -> i32 {
        nkmempages() as i32
    }

    fn averunnable(&self) -> ([u32; 3], i64) {
        let avg = averunnable();
        (avg.ldavg, avg.fscale)
    }

    fn for_each_proc(&self, f: &mut dyn FnMut(ProcState)) {
        for p in ALLPROC.0.iter() {
            let idle = p
                .cpu()
                .is_some_and(|ci| core::ptr::eq(Machine::ci_schedstate(ci).spc_idleproc.get(), p));
            f(ProcState {
                stat: i32::from(p.p_stat.get()),
                idle,
            });
        }
    }
}

/// `sysent[]`'s `mquery(2)`.
pub fn sys_mquery(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMqueryArgs = sysargs(v);
    let a = MmapArgs {
        addr: uap.addr.get() as usize,
        len: uap.len.get(),
        prot: uap.prot.get(),
        flags: uap.flags.get(),
        fd: uap.fd.get(),
        pos: uap.pos.get(),
    };
    retval[0] = kernel::vm::mmap::sys_mquery(vmspace_of(p), a)? as Register;
    Ok(())
}

/// `sysent[]`'s `mmap(2)`.
pub fn sys_mmap(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMmapArgs = sysargs(v);
    let a = MmapArgs {
        addr: uap.addr.get() as usize,
        len: uap.len.get(),
        prot: uap.prot.get(),
        flags: uap.flags.get(),
        fd: uap.fd.get(),
        pos: uap.pos.get(),
    };
    retval[0] = kernel::vm::mmap::sys_mmap(vmspace_of(p), a)? as Register;
    Ok(())
}

/// `sysent[]`'s `msync(2)`.
pub fn sys_msync(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMsyncArgs = sysargs(v);
    kernel::vm::mmap::sys_msync(
        vmspace_of(p),
        uap.addr.get() as usize,
        uap.len.get(),
        uap.flags.get(),
    )
}

/// `sysent[]`'s `munmap(2)`.
pub fn sys_munmap(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMunmapArgs = sysargs(v);
    kernel::vm::mmap::sys_munmap(vmspace_of(p), uap.addr.get() as usize, uap.len.get())
}

/// `sysent[]`'s `mprotect(2)`.
pub fn sys_mprotect(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMprotectArgs = sysargs(v);
    kernel::vm::mmap::sys_mprotect(
        vmspace_of(p),
        uap.addr.get() as usize,
        uap.len.get(),
        uap.prot.get(),
    )
}

/// `sysent[]`'s `pinsyscalls(2)`.
pub fn sys_pinsyscalls(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPinsyscallsArgs = sysargs(v);
    kernel::vm::mmap::sys_pinsyscalls(
        vmspace_of(p),
        uap.base.get() as usize,
        uap.len.get(),
        UserPtr::new(uap.pins.get() as usize),
        uap.npins.get(),
    )
}

/// `sysent[]`'s `mimmutable(2)`.
pub fn sys_mimmutable(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMimmutableArgs = sysargs(v);
    kernel::vm::mmap::sys_mimmutable(vmspace_of(p), uap.addr.get() as usize, uap.len.get())
}

/// `sysent[]`'s `minherit(2)`.
pub fn sys_minherit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMinheritArgs = sysargs(v);
    kernel::vm::mmap::sys_minherit(
        vmspace_of(p),
        uap.addr.get() as usize,
        uap.len.get(),
        uap.inherit.get(),
    )
}

/// `sysent[]`'s `madvise(2)`.
pub fn sys_madvise(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMadviseArgs = sysargs(v);
    kernel::vm::mmap::sys_madvise(
        vmspace_of(p),
        uap.addr.get() as usize,
        uap.len.get(),
        uap.behav.get(),
    )
}

/// `sysent[]`'s `mlock(2)`.
pub fn sys_mlock(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMlockArgs = sysargs(v);
    kernel::vm::mmap::sys_mlock(vmspace_of(p), uap.addr.get() as usize, uap.len.get())
}

/// `sysent[]`'s `munlock(2)`.
pub fn sys_munlock(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMunlockArgs = sysargs(v);
    kernel::vm::mmap::sys_munlock(vmspace_of(p), uap.addr.get() as usize, uap.len.get())
}

/// `sysent[]`'s `mlockall(2)`.
pub fn sys_mlockall(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMlockallArgs = sysargs(v);
    kernel::vm::mmap::sys_mlockall(vmspace_of(p), uap.flags.get())
}

/// `sysent[]`'s `munlockall(2)`.
pub fn sys_munlockall(p: &Proc, _v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    kernel::vm::mmap::sys_munlockall(vmspace_of(p))
}

/// `sysent[]`'s `kbind(2)`.
pub fn sys_kbind(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysKbindArgs = sysargs(v);
    kernel::vm::mmap::sys_kbind(
        vmspace_of(p),
        UserPtr::new(uap.param.get() as usize),
        uap.psize.get(),
        uap.proc_cookie.get() as u64,
    )
}

/// `sysent[]`'s `break` (`sys_obreak`).
pub fn sys_obreak(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysObreakArgs = sysargs(v);
    kernel::vm::unix::sys_obreak(vmspace_of(p), uap.nsize.get() as usize)
}

/// A `Native` vm entry: the thread on this CPU runs `mi_syscall` on row `nr` of `sysent[]`
/// (which must be `want`'s, else `ENOSYS`) with `args`: the prologue and the row's adapter
/// above, as on the `Legacy` path. `rval[0]` back in the `Ok`.
fn native(nr: Register, want: i32, args: SysArgs) -> Result<Register, i32> {
    let p = curp("native vm syscall");
    if nr != want as Register {
        return Err(Errno::ENOSYS.as_i32());
    }
    let mut rval: [Register; 2] = [0, 0];
    mi_syscall(p, nr, &SYSENT[nr as usize], &args, &mut rval).map_err(Errno::as_i32)?;
    Ok(rval[0])
}

/// `KernelApi::vm.obreak`.
pub fn native_obreak(nr: Register, nsize: usize) -> SysResult {
    native(nr, SYS_break, [nsize as Register, 0, 0, 0, 0, 0]).map(|_| ())
}

/// `KernelApi::vm.mmap`.
pub fn native_mmap(
    nr: Register,
    addr: usize,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    pos: i64,
) -> Result<usize, i32> {
    let args = [
        addr as Register,
        len as Register,
        prot as Register,
        flags as Register,
        fd as Register,
        pos as Register,
    ];
    native(nr, SYS_mmap, args).map(|a| a as usize)
}

/// `KernelApi::vm.mquery`.
pub fn native_mquery(
    nr: Register,
    addr: usize,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    pos: i64,
) -> Result<usize, i32> {
    let args = [
        addr as Register,
        len as Register,
        prot as Register,
        flags as Register,
        fd as Register,
        pos as Register,
    ];
    native(nr, SYS_mquery, args).map(|a| a as usize)
}

/// A range system call's `Native` entry: `(addr, len)` on row `want`.
fn native_range(nr: Register, want: i32, addr: usize, len: usize) -> SysResult {
    native(nr, want, [addr as Register, len as Register, 0, 0, 0, 0]).map(|_| ())
}

/// A range system call's `Native` entry with an `int`: `(addr, len, v)` on row `want`.
fn native_range_int(nr: Register, want: i32, addr: usize, len: usize, v: i32) -> SysResult {
    let args = [addr as Register, len as Register, v as Register, 0, 0, 0];
    native(nr, want, args).map(|_| ())
}

/// `KernelApi::vm.munmap`.
pub fn native_munmap(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_munmap, addr, len)
}

/// `KernelApi::vm.mprotect`.
pub fn native_mprotect(nr: Register, addr: usize, len: usize, prot: i32) -> SysResult {
    native_range_int(nr, SYS_mprotect, addr, len, prot)
}

/// `KernelApi::vm.madvise`.
pub fn native_madvise(nr: Register, addr: usize, len: usize, behav: i32) -> SysResult {
    native_range_int(nr, SYS_madvise, addr, len, behav)
}

/// `KernelApi::vm.kbind`.
pub fn native_kbind(nr: Register, param: usize, psize: usize, cookie: u64) -> SysResult {
    let args = [
        param as Register,
        psize as Register,
        cookie as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_kbind, args).map(|_| ())
}

/// `KernelApi::vm.pinsyscalls`.
pub fn native_pinsyscalls(
    nr: Register,
    base: usize,
    len: usize,
    pins: usize,
    npins: i32,
) -> SysResult {
    let args = [
        base as Register,
        len as Register,
        pins as Register,
        npins as Register,
        0,
        0,
    ];
    native(nr, SYS_pinsyscalls, args).map(|_| ())
}

/// `KernelApi::vm.mimmutable`.
pub fn native_mimmutable(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_mimmutable, addr, len)
}

/// `KernelApi::vm.mlock`.
pub fn native_mlock(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_mlock, addr, len)
}

/// `KernelApi::vm.munlock`.
pub fn native_munlock(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_munlock, addr, len)
}

/// `KernelApi::vm.minherit`.
pub fn native_minherit(nr: Register, addr: usize, len: usize, inherit: i32) -> SysResult {
    native_range_int(nr, SYS_minherit, addr, len, inherit)
}

/// `KernelApi::vm.msync`.
pub fn native_msync(nr: Register, addr: usize, len: usize, flags: i32) -> SysResult {
    native_range_int(nr, SYS_msync, addr, len, flags)
}

/// `KernelApi::vm.mlockall`.
pub fn native_mlockall(nr: Register, flags: i32) -> SysResult {
    native(nr, SYS_mlockall, [flags as Register, 0, 0, 0, 0, 0]).map(|_| ())
}

/// `KernelApi::vm.munlockall`.
pub fn native_munlockall(nr: Register) -> SysResult {
    native(nr, SYS_munlockall, [0; 6]).map(|_| ())
}

/// `uvm_init`: init the VM system. Called from `kern/init_main.c`: the bridges and the
/// machine's constants first, the page system over the machine's constraints, the kernel's
/// first step, `pmap_init`, the kernel memory allocators, the kernel's last step (the C's
/// order).
pub fn uvm_init() {
    // The kernel's objects and vnode pager reach the kernel lock, the net lock, the clock
    // and the shutdown state through bridges (N3.4e); the map, the system calls and the
    // sysctls reach curproc and the sysctl helpers (N3.5): registered before the first use.
    crate::kern::hal_bridges::hal_bridges_register_vm();
    crate::uvm::uvm_vnode::uvn_install_sysinfo();
    let _ = kernel::vm::proc::install(&BSD_VM_PROC);
    let _ = kernel::vm::meter::install(&BSD_VM_SYSCTL);
    kernel::vm::init::uvm_machine_init(&VM_MACHINE);

    // Init the page sub-system. This includes allocating the vm_page structures, and setting
    // up all the page queues (and locks). Available memory will be put in the "free" queue,
    // kvm_start and kvm_end will be set to the area of kernel virtual memory which is
    // available for general use.
    let mut kvm_start = crate::sys::types::Vaddr::new(0);
    let mut kvm_end = crate::sys::types::Vaddr::new(0);
    crate::uvm::uvm_page::uvm_page_init(&mut kvm_start, &mut kvm_end);

    // The map sub-system, kernel_map and the kernel object, the fault's tuning.
    let base = crate::uvm::uvm_init::VM_MIN_KERNEL_ADDRESS.load(Ordering::Relaxed);
    kernel::vm::init::uvm_init_early(crate::sys::types::Vaddr::new(base), (kvm_start, kvm_end));

    // Init the pmap module. The pmap module is free to allocate memory for its private use
    // (e.g. pvlists).
    crate::machine::pmap::pmap_init();

    // step 6: init uvm_km_page allocator memory.
    let _ = crate::unported!("uvm_km_page_init");

    // Make kernel memory allocators ready for use. After this call the malloc memory allocator
    // can be used.
    crate::kern::kern_malloc::kmeminit();
    crate::kern::hal_bridges::hal_bridges_heap_ready();

    // step 7.5: init the dma allocator, which is backed by pools.
    crate::kern::dma_alloc::dma_alloc_init();

    // The pagers, the amaps, the kernel's swap object, the anons, the best-fit switches.
    kernel::vm::init::uvm_init_late(crate::machine::intr::IPL_MPFLOOR);
}

/// `uvm_purge`: teardown a virtual address space. If multi-threaded, must be called by the
/// last thread of a process.
pub fn uvm_purge() {
    let p = curp("uvm_purge");
    let vm = vmspace_of(p);

    kernel_assert_unlocked();

    // __HAVE_PMAP_PURGE: arm64's; nothing on a machine without it.
    crate::machine::pmap::pmap_purge(p);
    uvmspace_purge(vm);
}

/// `uvm_exit`: exit a virtual address space.
pub fn uvm_exit(pr: &Process) {
    if let Some(vm) = vmspace_take(pr) {
        uvmspace_free(vm);
    }
}

/// `uvm_init_limits`: init per-process VM limits, the kernel's numbers
/// (`kernel::vm::glue::default_limits`) on `limit0`.
pub fn uvm_init_limits(limit0: &Plimit) {
    let l = kernel::vm::glue::default_limits();
    let set = |which: usize, cur: Rlim, max: Option<Rlim>| {
        let mut r = limit0.pl_rlimit[which].get();
        r.rlim_cur = cur;
        if let Some(m) = max {
            r.rlim_max = m;
        }
        limit0.pl_rlimit[which].set(r);
    };
    set(RLIMIT_STACK, l.stack.0 as Rlim, Some(l.stack.1 as Rlim));
    set(RLIMIT_DATA, l.data.0 as Rlim, Some(l.data.1 as Rlim));
    set(RLIMIT_RSS, l.rss_cur as Rlim, None);
}
/* </CODE> */
