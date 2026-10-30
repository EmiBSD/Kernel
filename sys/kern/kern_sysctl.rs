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

/*-
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Mike Karels at Berkeley Software Design, Inc.
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
 *	@(#)kern_sysctl.c	8.4 (Berkeley) 4/14/94
 */
/* </LICENSES> */

/* <CODE> */
//! The `sysctl(2)` system call's `bsd` half: `kern/kern_sysctl.c` minus what moved into the
//! kernel crate (`kernel::sysctl`: the dispatcher of `sys_sysctl`, `kern_sysctl` and
//! `hw_sysctl`, the vslock, the nodes whose state is the kernel's, `kern.proc` and its
//! siblings, the disk exports, the hostname). What stays here is what reaches subsystems
//! still in `bsd`: `kern.file` (`kinfo_file` over the descriptors, vnodes, sockets and
//! control blocks: N5, N6), the helpers every `bsd` node is written with (`sysctl_int`,
//! `sysctl_rdint`, `sysctl_string`, `sysctl_rdstruct`, `sysctl_bounded_arr`, ...), and the
//! registration of `bsd`'s nodes with the kernel's dispatcher
//! ([`sysctl_register_legacy`]): the `net`, `vfs`, `machdep` and `ddb` trees, the `kern` and
//! `hw` leaves and directories whose state is `bsd`'s (`kern.ostype`, `kern.clockrate`,
//! `kern.mbstat`, `kern.cp_time`, `kern.tty`, `kern.timecounter`, `hw.sensors`,
//! `hw.machine`, ...), their bounded integers, the disk list's export and `physmem`.
//!
//! Upstream: sys/kern/kern_sysctl.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_sysctl.rs@3c62ede68802
//!
//! The system identifies itself as EmiBSD 8.0 (`kern.ostype`, `kern.osrelease`,
//! `kern.version`, `kern.osversion` come from `conf/vers.rs`); `kern.osrevision` stays the
//! `OpenBSD` API date of `<sys/param.h>`, which is what programs test.
//!
//! ## Deviations
//! - Names are slices (`&[i32]`), `oldp`/`newp` user addresses as `usize` (0 is NULL),
//!   `oldlenp` a `&mut usize`, and errors `Result<(), Errno>`. A string is `&[u8]` up to its
//!   first NUL, a writable string `&mut [u8]` whose length is the C's `maxlen`; a structure is
//!   its bytes ([`SysctlPlain`]). `int *valp` is `&AtomicI32`, the C's atomic operations on it
//!   are the atomic's; a C local passed by address is an `AtomicI32` read back with
//!   `into_inner`.
//! - The helpers here copy through `bsd`'s `copyin`/`copyout` for the nodes still in `bsd`;
//!   `kernel::sysctl` has the same helpers over `hal::user::UserPtr` for its own nodes (the
//!   same machine routines through the `UserCopy` bridge). Each `bsd` node moves to the
//!   kernel's with its subsystem.
//! - `resettodr` after a new `kern.utc_offset` is reported and skipped. (`kern.clockintr`,
//!   LZ's `unported!`, is the kernel's `kernel::clockintr::ClockintrNode` since N4.8a, and
//!   `kern.timecounter` is `kernel::tc::TcNode`: both registered from [`KERN_NODES`].)
//! - `kern.file`: `fill_file` fills the `AF_INET` and `AF_INET6` (feature `inet6`) control
//!   blocks and the TCP members (`fill_file_tcpcb`, zero for a control block without a
//!   `tcpcb`). `KERN_FILE_BYFILE` of sockets walks `tcbtable`, `udbtable`, `rawcbtable`,
//!   `divbtable` (`NPF`) and, with `INET6`, `tcb6table`, `udb6table`, `rawin6pcbtable` and
//!   `divb6table`. `ps_tracevp` does not exist
//!   (`KTRACE`), so no `KERN_FILE_TRACE` entry is made. The C's `FILLIT` macros are the
//!   methods of a private `FileWalk` (the C's `kf`, `dp`, `buflen`, `elem_count`,
//!   `needed`); `kf` lives in it instead of an `M_TEMP` allocation. A `copyout` error ends
//!   the walk and is returned once the references taken are dropped (the C's `break` only
//!   left the macro's `do { } while (0)`, so its walk went on and a later `copyout` could
//!   overwrite the error). `kinfo_file` is copied out as bytes, its two C padding holes
//!   being members (`hal::sys::sysctl`).
//! - The morally-const values `sysctl_bounded_arr` reports (`openbsd`, `maxpartitions`,
//!   `raw_part`) are `AtomicI32` statics like the C's `static int`s.
//! - `KERN_CPTIME` with no CPU online (which only the host double can produce) leaves the
//!   sums at zero instead of dividing by zero.
//! - `hw.sensors.<dev>.<type>.<numt>` with a `type` outside `enum sensor_type` finds no
//!   sensor (`ENOENT`, after the device lookup's own errors), as the C's comparison would.
//!
//! ## Redesign
//! - N4.6b: the dispatch (`sys_sysctl`, `kern_sysctl`, `hw_sysctl`), `sysctl_vslock`, the
//!   nodes whose state is the kernel's and `kern.proc` with `fill_kproc` moved into
//!   `kernel::sysctl` (its `## Redesign` has the shape). What stays here registers `bsd`'s
//!   nodes with that dispatcher ([`sysctl_register_legacy`], called from the bridges'
//!   registration): the `CTL_*` trees, the `kern` and `hw` leaves and directories as
//!   `SysctlNode`s (`Node`, a function over a name and a `SysctlReq`; `ProcNode`, one that
//!   wants the calling thread), the `kern_vars[]`/`hw_vars[]` entries whose variables are
//!   `bsd`'s, `hw.machine` and `physmem`.
//! - `BsdDiskExport` is the `DiskExport` bridge (`bridges.toml`): the disk list walked
//!   under its lock, each disk's name, DUID and `struct diskstats` handed to the kernel's
//!   `sysctl_diskinit` as bytes.
//! - `native_sysctl` is `sysctl(2)`'s entry in the `KernelApi` table (`abi_openbsd.ko`):
//!   the kernel's `sys_sysctl` over `bsd`'s glue.
//! - `sysctl_vslock` returns the kernel's `VsLock` guard (its drop is `sysctl_vsunlock`);
//!   `sysctl_lock` is the kernel's `SYSCTL_LOCK`, the one lock of both halves.
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicI32, Ordering};

use libkern::{strlcpy, strnlen};

use crate::conf::param::{FSCALE, MAXFILES, MAXPROCESS, MAXTHREAD, NMBCLUST, UTC_OFFSET};
use crate::conf::vers::{OSRELEASE, OSTYPE, OSVERSION, VERSION};
use crate::dev::audio::{AUDIO_KBDCONTROL_ENABLE, AUDIO_RECORD_ENABLE};
use crate::dev::cons::cn_tab;
use crate::dev::usb::ucom::sysctl_ucominit;
use crate::kern::init_main::{NCPUS, NCPUSFOUND};
use crate::kern::kern_clock::sysctl_clockrate;
use crate::kern::kern_descrip::{BsdFileData, NUMFILES, fd_getfile, fd_iterfile};
use crate::kern::kern_event::kqueue_kinfo;
use crate::kern::kern_lock::{mtx_enter, mtx_leave, pc_cons_enter, pc_cons_leave};
use crate::kern::kern_malloc::sysctl_malloc;
use crate::kern::kern_prot::suser;
use crate::kern::kern_sched::{cpu_is_online, sysctl_hwblockcpu, sysctl_hwsmt};
use crate::kern::kern_sensors::{sensor_find, sensordev_get};
use crate::kern::kern_sig::NOSUIDCOREDUMP;
use crate::kern::kern_tc::{microboottime, nanotime, tc_setrealtimeclock};
use crate::kern::kern_watchdog::sysctl_wdog;
use crate::kern::pipe_glue::pipe_kinfo;
use crate::kern::proc_glue::{BsdProcGlue, proc_tables, process_ref};
use crate::kern::subr_autoconf::AUTOCONF_SERIAL;
use crate::kern::subr_disk::{DISK_CHANGE, DISK_COUNT, DISKLIST, duid_format, duid_iszero};
use crate::kern::subr_evcount::evcount_sysctl;
use crate::kern::subr_pool::{POOL_DEBUG, pool_reclaim_all, sysctl_dopool};
use crate::kern::subr_prf::{SPLASSERT_CTL, panic};
use crate::kern::sys_socket::fp_socket;
use crate::kern::uipc_mbuf::{MBS_NCOUNTERS, mbstat, nmbclust_update};
use crate::kern::uipc_socket::{somaxconn, sominconn};
use crate::kern::uipc_socket2::{soassertlocked, solock_shared, sounlock_shared};
use crate::kern::vfs_cache::NCHSTATS;
use crate::kern::vfs_lockf::MAXLOCKSPERUID;
use crate::kern::vfs_vops::VOP_GETATTR;
use crate::machine::Machine;
use crate::machine::copy::{copyin, copyout};
use crate::machine::cpu::{Cpu, CpuInfo, cpu_info_foreach, curproc};
use crate::machine::param::MachineInfo;
use crate::netinet::in_::IPPROTO_TCP;
use crate::netinet::in_pcb::{
    Inpcb, InpcbIterator, Inpcbtable, in_pcb_iterator, in_pcb_iterator_abort, in_pcbsolock,
    in_pcbsounlock, sotoinpcb,
};
use crate::netinet::ip_divert::DIVBTABLE;
use crate::netinet::raw_ip::RAWCBTABLE;
use crate::netinet::tcp_usrreq::TCBTABLE;
use crate::netinet::tcp_var::intotcpcb;
use crate::netinet::udp_usrreq::UDBTABLE;
use crate::sys::buf::{BUFHIGHPAGES, bufadjust};
use crate::sys::disk::{DS_DISKNAMELEN, Disk, Diskstats};
use crate::sys::errno::Errno;
use crate::sys::fcntl::{FREAD, FWRITE};
use crate::sys::file::FileExt;
use crate::sys::file::{DTYPE_KQUEUE, DTYPE_PIPE, DTYPE_SOCKET, DTYPE_VNODE, File, FileRef, frele};
use crate::sys::filedesc::{Filedesc, fdplock, fdpunlock};
use crate::sys::limits::SHRT_MAX;
use crate::sys::mbuf::{MT_NTYPES, Mbstat, MbstatCounters, mtod};
use crate::sys::param::{NODEV, OpenBSD};
use crate::sys::proc::{PS_EMBRYO, PS_EXITING, PS_SYSTEM, Proc, Process, ProcessRef};
use crate::sys::queue::SlistHead;
use crate::sys::sched::{CPUSTATES, CPUSTATS_ONLINE, Cpustats};
use crate::sys::sensors::{Sensor, SensorType, Sensordev};
use crate::sys::socket::{AF_INET, AF_INET6, AF_UNIX, SOCK_RAW};
use crate::sys::socketvar::{Socket, isspliced, issplicedback};
use crate::sys::syscallargs::SysSysctlArgs;
use crate::sys::sysctl::*;
use crate::sys::systm::{
    SysArgs, kernel_lock, kernel_unlock, net_lock_shared, net_unlock_shared, sysargs,
};
use crate::sys::types::Register;
use crate::sys::un::SockaddrUn;
use crate::sys::unpcb::{UnpRefs, sotounpcb};
use crate::sys::vnode::NUMVNODES;
use crate::sys::vnode::{Vattr, Vnode, makeimode};
use crate::unported;
#[cfg(feature = "inet6")]
use crate::{
    netinet::tcp_usrreq::TCB6TABLE, netinet::udp_usrreq::UDB6TABLE,
    netinet6::ip6_divert::DIVB6TABLE, netinet6::raw_ip6::RAWIN6PCBTABLE,
};
use hal::klock;
use hal::user::UserPtr;
use kernel::sysctl::{RdString, SysctlNode, SysctlReq};
use kernel::vm::km::NO_CONSTRAINT;
use kernel::vm::page::uvm_pagecount;

/// `MAXPARTITIONS` (`<machine/disklabel.h>`, 16 on amd64 and arm64): number of partitions.
/// The disklabel headers are not ported.
const MAXPARTITIONS_C: i32 = 16;
/// `RAW_PART` (`<sys/disklabel.h>`): the 'c' partition.
const RAW_PART_C: i32 = 2;

pub use kernel::sysctl::{
    ALLOWKMEM, ALLOWPOWERDOWN, HOSTID, SECURELEVEL, SYSCTL_LOCK, hw_battery_chargemode,
    hw_battery_chargestart, hw_battery_chargestop, hw_power, hw_prod, hw_serial, hw_uuid,
    hw_vendor, hw_ver,
};

/// `openbsd`: morally const, reported by `sysctl_bounded_arr`.
static OPENBSD: AtomicI32 = AtomicI32::new(OpenBSD as i32);
/// `maxpartitions`.
static MAXPARTITIONS: AtomicI32 = AtomicI32::new(MAXPARTITIONS_C);
/// `raw_part`.
static RAW_PART: AtomicI32 = AtomicI32::new(RAW_PART_C);

/// `kern_vars[]`, the entries whose variables are `bsd`'s (the kernel has the others):
/// registered with the kernel's dispatcher, which searches its own entries first.
static KERN_VARS: [SysctlBoundedArgs; 15] = [
    SysctlBoundedArgs::readonly(KERN_OSREV, &OPENBSD),
    SysctlBoundedArgs::new(
        KERN_MAXVNODES,
        &crate::kern::kern_descrip::VFS_TABLES.maxvnodes,
        0,
        i32::MAX,
    ),
    SysctlBoundedArgs::new(KERN_MAXPROC, &MAXPROCESS, 0, i32::MAX),
    SysctlBoundedArgs::new(KERN_MAXFILES, &MAXFILES, 0, i32::MAX),
    SysctlBoundedArgs::readonly(KERN_NFILES, &NUMFILES),
    SysctlBoundedArgs::readonly(KERN_MAXPARTITIONS, &MAXPARTITIONS),
    SysctlBoundedArgs::readonly(KERN_RAWPARTITION, &RAW_PART),
    SysctlBoundedArgs::new(KERN_MAXTHREAD, &MAXTHREAD, 0, i32::MAX),
    SysctlBoundedArgs::new(KERN_SOMAXCONN, &somaxconn, 0, SHRT_MAX as i32),
    SysctlBoundedArgs::new(KERN_SOMINCONN, &sominconn, 0, SHRT_MAX as i32),
    SysctlBoundedArgs::new(KERN_NOSUIDCOREDUMP, &NOSUIDCOREDUMP, 0, 3),
    SysctlBoundedArgs::readonly(KERN_FSCALE, &FSCALE),
    SysctlBoundedArgs::new(KERN_SPLASSERT, &SPLASSERT_CTL, 0, 3),
    SysctlBoundedArgs::new(KERN_MAXLOCKSPERUID, &MAXLOCKSPERUID, 0, i32::MAX),
    SysctlBoundedArgs::readonly(KERN_AUTOCONF_SERIAL, &AUTOCONF_SERIAL),
];

/// `hw_vars[]`, the entries whose variables are `bsd`'s.
static HW_VARS: [SysctlBoundedArgs; 3] = [
    SysctlBoundedArgs::readonly(HW_NCPU, &NCPUS),
    SysctlBoundedArgs::readonly(HW_NCPUFOUND, &NCPUSFOUND),
    SysctlBoundedArgs::readonly(HW_DISKCOUNT, &DISK_COUNT),
];

/// `sysctl_vslock(addr, len)` for `curproc` (the network's protocol handlers that are not
/// `PR_MPSYSCTL`): the kernel's [`kernel::sysctl::VsLock`], whose drop is `sysctl_vsunlock`.
pub fn sysctl_vslock(addr: usize, len: usize) -> Result<kernel::sysctl::VsLock, Errno> {
    let Some(p) = curproc() else {
        panic(format_args!("sysctl_vslock: no curproc"));
    };
    kernel::sysctl::sysctl_vslock(p, addr, len)
}

/// `KernelApi::sysctl`: [`sys_sysctl`]'s row through the system call prologue (`mi_syscall`:
/// pledge, the pinned syscalls, the kernel-lock rule; the row is `SY_NOLOCK`, as the C's).
#[allow(
    clippy::too_many_arguments,
    reason = "the entry's shape: the number, six arguments, the return registers"
)]
pub fn native_sysctl(
    nr: Register,
    name: usize,
    namelen: u32,
    old: usize,
    oldlenp: usize,
    new: usize,
    newlen: usize,
    rval: &mut [Register; 2],
) -> hal::syscall::SysResult {
    let args = [
        name as Register,
        namelen as Register,
        old as Register,
        oldlenp as Register,
        new as Register,
        newlen as Register,
    ];
    crate::kern::kern_descrip::native(nr, crate::sys::syscall::SYS_sysctl, args, rval)
}

/// `sysctl(2)`'s `sysent[]` row: the kernel's [`kernel::sysctl::sys_sysctl`] with the
/// system call's arguments.
pub fn sys_sysctl(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSysctlArgs = sysargs(v);
    kernel::sysctl::sys_sysctl::<BsdProcGlue>(
        p,
        UserPtr::new(uap.name.get() as usize),
        uap.namelen.get(),
        uap.old.get() as usize,
        UserPtr::new(uap.oldlenp.get() as usize),
        uap.new.get() as usize,
        uap.newlen.get(),
    )
}

/// `sysctl_int_lower`: reads, or writes that lower the value.
pub fn sysctl_int_lower(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    valp: &AtomicI32,
) -> Result<(), Errno> {
    if oldp != 0 && *oldlenp < size_of::<i32>() {
        return Err(Errno::ENOMEM);
    }
    if newp != 0 && newlen != size_of::<i32>() {
        return Err(Errno::EINVAL);
    }
    *oldlenp = size_of::<i32>();

    if newp != 0 {
        let mut b = [0u8; 4];
        copyin(newp, &mut b)?;
        let newval = i32::from_ne_bytes(b);
        let mut oldval = valp.load(Ordering::Relaxed);
        loop {
            if (oldval as u32) < (newval as u32) {
                return Err(Errno::EPERM); // do not allow raising
            }
            match valp.compare_exchange(oldval, newval, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(current) => oldval = current,
            }
        }

        if oldp != 0 {
            // new value has been set although user gets error
            copyout(&oldval.to_ne_bytes(), oldp)?;
        }
    } else if oldp != 0 {
        let oldval = valp.load(Ordering::Relaxed);

        copyout(&oldval.to_ne_bytes(), oldp)?;
    }

    Ok(())
}

/// `sysctl_int`: validates parameters and gets the old / sets the new value of an
/// integer-valued sysctl.
pub fn sysctl_int(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    valp: &AtomicI32,
) -> Result<(), Errno> {
    sysctl_int_bounded(oldp, oldlenp, newp, newlen, valp, i32::MIN, i32::MAX)
}

/// `sysctl_rdint`: as [`sysctl_int`], but read-only.
pub fn sysctl_rdint(oldp: usize, oldlenp: &mut usize, newp: usize, val: i32) -> Result<(), Errno> {
    if oldp != 0 && *oldlenp < size_of::<i32>() {
        return Err(Errno::ENOMEM);
    }
    if newp != 0 {
        return Err(Errno::EPERM);
    }
    *oldlenp = size_of::<i32>();
    if oldp != 0 {
        copyout(&val.to_ne_bytes(), oldp)?;
    }
    Ok(())
}

/// `sysctl_securelevel_int`: [`sysctl_rdint`] or [`sysctl_int`] according to
/// `securelevel`.
pub fn sysctl_securelevel_int(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    valp: &AtomicI32,
) -> Result<(), Errno> {
    if SECURELEVEL.load(Ordering::Relaxed) > 0 {
        return sysctl_rdint(oldp, oldlenp, newp, valp.load(Ordering::Relaxed));
    }
    sysctl_int(oldp, oldlenp, newp, newlen, valp)
}

/// `sysctl_int_bounded`: read-only or bounded integer values. `minimum > maximum` makes the
/// value read-only; both bounds are inclusive.
pub fn sysctl_int_bounded(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    valp: &AtomicI32,
    minimum: i32,
    maximum: i32,
) -> Result<(), Errno> {
    // read only
    if newp != 0 && minimum > maximum {
        return Err(Errno::EPERM);
    }

    if oldp != 0 && *oldlenp < size_of::<i32>() {
        return Err(Errno::ENOMEM);
    }
    if newp != 0 && newlen != size_of::<i32>() {
        return Err(Errno::EINVAL);
    }
    *oldlenp = size_of::<i32>();

    // copyin() may sleep, call it first
    let mut newval = 0;
    if newp != 0 {
        let mut b = [0u8; 4];
        copyin(newp, &mut b)?;
        newval = i32::from_ne_bytes(b);
        // outside limits
        if newval < minimum || maximum < newval {
            return Err(Errno::EINVAL);
        }
    }
    if oldp != 0 {
        let oldval = if newp != 0 {
            valp.swap(newval, Ordering::Relaxed)
        } else {
            valp.load(Ordering::Relaxed)
        };
        // new value has been set although user gets error
        copyout(&oldval.to_ne_bytes(), oldp)?;
    } else if newp != 0 {
        valp.store(newval, Ordering::Relaxed);
    }

    Ok(())
}

/// `sysctl_bounded_arr`: an array of read-only or bounded integer values, looked up by the
/// one remaining name component.
pub fn sysctl_bounded_arr(
    valpp: &[SysctlBoundedArgs],
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let [mib] = name else {
        return Err(Errno::ENOTDIR);
    };
    match valpp.iter().find(|v| v.mib == *mib) {
        Some(v) => sysctl_int_bounded(oldp, oldlenp, newp, newlen, v.var, v.minimum, v.maximum),
        None => Err(Errno::EOPNOTSUPP),
    }
}

/// `sysctl_rdquad`: validates parameters and gets the old value of a 64-bit sysctl.
pub fn sysctl_rdquad(oldp: usize, oldlenp: &mut usize, newp: usize, val: i64) -> Result<(), Errno> {
    if oldp != 0 && *oldlenp < size_of::<i64>() {
        return Err(Errno::ENOMEM);
    }
    if newp != 0 {
        return Err(Errno::EPERM);
    }
    *oldlenp = size_of::<i64>();
    if oldp != 0 {
        copyout(&val.to_ne_bytes(), oldp)?;
    }
    Ok(())
}

/// `sysctl_string`: validates parameters and gets the old / sets the new value of a
/// string-valued sysctl. `str` is the variable's whole buffer (`maxlen` in C).
pub fn sysctl_string(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    str: &mut [u8],
) -> Result<(), Errno> {
    sysctl__string(oldp, oldlenp, newp, newlen, str, false)
}

/// `sysctl_tstring`: as [`sysctl_string`], but a short `old` buffer gets a truncated,
/// NUL-terminated copy instead of `ENOMEM`.
pub fn sysctl_tstring(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    str: &mut [u8],
) -> Result<(), Errno> {
    sysctl__string(oldp, oldlenp, newp, newlen, str, true)
}

/// `sysctl__string`: the body of [`sysctl_string`] and [`sysctl_tstring`].
#[allow(non_snake_case)] // the C's name, with its double underscore
pub fn sysctl__string(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    str: &mut [u8],
    trunc: bool,
) -> Result<(), Errno> {
    let maxlen = str.len();
    let slen = strnlen(str, maxlen);
    let mut len = slen + 1;
    if oldp != 0 && *oldlenp < len && (!trunc || *oldlenp == 0) {
        return Err(Errno::ENOMEM);
    }
    if newp != 0 && newlen >= maxlen {
        return Err(Errno::EINVAL);
    }
    let mut error = Ok(());
    if oldp != 0 {
        if trunc && *oldlenp < len {
            len = *oldlenp;
            error = copyout(&str[..len - 1], oldp);
            if error.is_ok() {
                error = copyout(&[0], oldp + len - 1);
            }
        } else {
            error = copyout_cstr(&str[..slen], oldp);
        }
    }
    *oldlenp = len;
    if error.is_ok() && newp != 0 {
        error = copyin(newp, &mut str[..newlen]);
        str[newlen] = 0;
    }
    error
}

/// `copyout(str, oldp, strlen(str) + 1)`: the bytes, then the NUL.
fn copyout_cstr(s: &[u8], uaddr: usize) -> Result<(), Errno> {
    copyout(s, uaddr)?;
    copyout(&[0], uaddr + s.len())
}

/// `sysctl_rdstring`: as [`sysctl_string`], but read-only. `str` ends at its first NUL or
/// at its end.
pub fn sysctl_rdstring(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    str: &[u8],
) -> Result<(), Errno> {
    let slen = strnlen(str, str.len());
    let len = slen + 1;
    if oldp != 0 && *oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if newp != 0 {
        return Err(Errno::EPERM);
    }
    *oldlenp = len;
    if oldp != 0 {
        copyout_cstr(&str[..slen], oldp)?;
    }
    Ok(())
}

/// `sysctl_struct`: validates parameters and gets the old / sets the new value of a
/// structure-valued sysctl. `sp` is the structure's bytes.
pub fn sysctl_struct(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    sp: &mut [u8],
) -> Result<(), Errno> {
    let len = sp.len();

    if oldp != 0 && *oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if newp != 0 && newlen > len {
        return Err(Errno::EINVAL);
    }
    let mut error = Ok(());
    if oldp != 0 {
        *oldlenp = len;
        error = copyout(sp, oldp);
    }
    if error.is_ok() && newp != 0 {
        error = copyin(newp, sp);
    }
    error
}

/// `sysctl_rdstruct`: validates parameters and gets the old value of a structure-valued
/// sysctl.
pub fn sysctl_rdstruct(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    sp: &[u8],
) -> Result<(), Errno> {
    let len = sp.len();

    if oldp != 0 && *oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if newp != 0 {
        return Err(Errno::EPERM);
    }
    *oldlenp = len;
    if oldp != 0 {
        copyout(sp, oldp)?;
    }
    Ok(())
}

/// `PTRTOINT64(vp->v_un.vu_socket)`: whatever pointer the vnode's union holds.
fn vnode_un_ptr(vp: &Vnode) -> u64 {
    // `v_socket` is bsd's member of the vnode (`VnodeG`), the other members the kernel's
    // `v_un`: one of them is set, as the C's union.
    match vp.g.v_socket.get() {
        Some(so) => ptrtoint64(so),
        None => vp.un_addr() as u64,
    }
}

/// The `memcpy(kf->unp_path, un->sun_path, un->sun_len - offsetof(...))` of `fill_file`:
/// `addr` is the bytes of a bound `struct sockaddr_un` (its mbuf's `m_len`). The copy is
/// bounded by both buffers.
fn unp_path_copy(dst: &mut [u8; KI_UNPPATHLEN], addr: &[u8]) {
    let off = SockaddrUn::PATH_OFFSET;
    let Some(&sun_len) = addr.first() else {
        return;
    };
    let len = usize::from(sun_len)
        .saturating_sub(off)
        .min(addr.len().saturating_sub(off))
        .min(KI_UNPPATHLEN);
    dst[..len].copy_from_slice(&addr[off..off + len]);
}

/// `fill_file`: one `kinfo_file` for the file `fp`, or (without a file) for the vnode `vp`
/// a process uses as its text, current or root directory (`fd` is then `KERN_FILE_TEXT`,
/// ...), or for the socket `so` of a protocol control block table (locked by the caller).
/// `fdp` and `pr` are the descriptor table and the process the descriptor is found through,
/// for `KERN_FILE_BYPID`/`KERN_FILE_BYUID`.
#[allow(clippy::too_many_arguments)] // the C's arguments
pub fn fill_file(
    kf: &mut KinfoFile,
    fp: Option<&FileRef>,
    fdp: Option<&Filedesc>,
    fd: i32,
    vp: Option<&'static Vnode>,
    pr: Option<&Process>,
    p: &Proc,
    so: Option<&Socket>,
    show_pointers: bool,
) {
    *kf = KinfoFile::zeroed();

    kf.fd_fd = fd; // might not really be an fd

    if let Some(fp) = fp {
        // An open file holds its credentials from `fnew` until its item goes, and the caller
        // holds a reference to the file.
        let cred = fp.cred();
        if show_pointers {
            kf.f_fileaddr = fp.addr() as u64;
        }
        kf.f_flag = fp.f_flag.load(Ordering::Relaxed);
        kf.f_iflags = fp.f_iflags.load(Ordering::Relaxed);
        kf.f_type = fp.f_type() as u32;
        kf.f_count = fp.count();
        if show_pointers {
            kf.f_ucred = ptrtoint64(fp.f_cred_ptr());
        }
        kf.f_uid = cred.cr_uid.get();
        kf.f_gid = cred.cr_gid.get();
        if show_pointers {
            kf.f_ops = fp.installed().map_or(0, |i| ptrtoint64(i.ops));
        }
        if show_pointers {
            kf.f_data = file_data_addr(fp);
        }
        kf.f_usecount = 0;

        if suser(p).is_ok() || p.ucred().cr_uid.get() == cred.cr_uid.get() {
            {
                let st = fp.stats();
                kf.f_offset = fp.offset() as u64;
                kf.f_rxfer = st.f_rxfer;
                kf.f_rwfer = st.f_wxfer;
                kf.f_seek = st.f_seek;
                kf.f_rbytes = st.f_rbytes;
                kf.f_wbytes = st.f_wbytes;
            }
        } else {
            kf.f_offset = -1i64 as u64;
        }
    } else if vp.is_some() {
        // fake it
        kf.f_type = DTYPE_VNODE as u32;
        kf.f_flag = FREAD as u32;
        if fd == KERN_FILE_TRACE {
            kf.f_flag |= FWRITE as u32;
        }
    } else if so.is_some() {
        // fake it
        kf.f_type = DTYPE_SOCKET as u32;
    }

    // information about the object associated with this file
    match (kf.f_type as i32, fp) {
        (DTYPE_VNODE, _) => {
            let vp = match (fp, vp) {
                (Some(fp), _) => Some(fp.vnode()),
                (None, vp) => vp,
            };
            if let Some(vp) = vp {
                fill_file_vnode(kf, vp, p, show_pointers);
            }
        }
        (DTYPE_SOCKET, _) => {
            // if so is passed as parameter it is already locked
            let (so, locked) = match (so, fp) {
                (Some(so), _) => (Some(so), false),
                (None, Some(fp)) => {
                    let so = fp_socket(fp);
                    solock_shared(so);
                    (Some(so), true)
                }
                (None, None) => (None, false),
            };
            if let Some(so) = so {
                fill_file_socket(kf, so, show_pointers);
                if locked {
                    sounlock_shared(so);
                }
            }
        }
        (DTYPE_PIPE, Some(fp)) => {
            // The pipe's own address is `f_data`'s column (its `f_data` stays null).
            if let Some(pipe) = pipe_kinfo(fp) {
                if show_pointers {
                    kf.f_data = pipe.addr as u64;
                    kf.pipe_peer = pipe.peer as u64;
                }
                kf.pipe_state = pipe.state;
            }
        }
        (DTYPE_KQUEUE, Some(fp)) => {
            (kf.kq_count, kf.kq_state) = kqueue_kinfo(fp);
        }
        _ => {}
    }

    // per-process information for KERN_FILE_BY[PU]ID
    if let Some(pr) = pr {
        let cred = pr.ucred();
        kf.p_pid = pr.ps_pid as u32;
        kf.p_uid = cred.cr_uid.get();
        kf.p_gid = cred.cr_gid.get();
        kf.p_tid = -1i32 as u32;
        strlcpy(&mut kf.p_comm, &pr.comm());
    }
    if let Some(fdp) = fdp {
        let g = fdplock(fdp);
        kf.fd_ofileflags = u32::from(g.ofileflags(fd));
        fdpunlock(g);
    }
}

/// `fp->f_data`'s address, as `kinfo_file` prints it: the vnode, the socket or the kqueue (a
/// pipe's own address is its own column, set by the caller; a file with no object prints 0).
fn file_data_addr(fp: &File) -> u64 {
    match fp.data() {
        Some(BsdFileData::Vnode(v)) => v.get().map_or(0, |vp| ptrtoint64(vp)),
        Some(BsdFileData::Socket(word)) => ptrtoint64(word.get()),
        Some(BsdFileData::Kqueue(kq)) => kq.addr() as u64,
        _ => 0,
    }
}

/// The `DTYPE_VNODE` case of `fill_file`.
fn fill_file_vnode(kf: &mut KinfoFile, vp: &'static Vnode, p: &Proc, show_pointers: bool) {
    if show_pointers {
        kf.v_un = vnode_un_ptr(vp);
    }
    kf.v_type = vp.vtype() as u32;
    kf.v_tag = vp.tag() as u32;
    kf.v_flag = vp.flag();
    if show_pointers {
        kf.v_data = ptrtoint64(vp.data());
    }
    if show_pointers {
        kf.v_mount = vp.mount().map_or(0, |mp| ptrtoint64(mp));
    }
    if let Some(mp) = vp.mount() {
        let (name, len) = mp.mntonname();
        strlcpy(&mut kf.f_mntonname, &name[..len]);
    }

    let mut va = Vattr::new();
    if VOP_GETATTR(vp, &mut va, p.ucred_ptr(), p).is_ok() {
        kf.va_fileid = va.va_fileid;
        kf.va_mode = makeimode(va.va_type, va.va_mode);
        kf.va_size = va.va_size;
        kf.va_rdev = va.va_rdev as u32;
        kf.va_fsid = (va.va_fsid & 0xffff_ffff) as u32;
        kf.va_nlink = va.va_nlink;
    }
}

/// The `DTYPE_SOCKET` case of `fill_file`, the socket locked.
fn fill_file_socket(kf: &mut KinfoFile, so: &Socket, show_pointers: bool) {
    kf.so_type = so.so_type.get() as u32;
    kf.so_state = so.so_state.get() | so.so_snd.sb_state.get() | so.so_rcv.sb_state.get();
    kf.so_pcb = if show_pointers {
        ptrtoint64(so.so_pcb.get())
    } else {
        -1i64 as u64
    };
    kf.so_protocol = so.so_proto.pr_protocol as u32;
    kf.so_family = so.dom_family() as u32;
    kf.so_rcv_cc = so.so_rcv.sb_cc.get();
    kf.so_snd_cc = so.so_snd.sb_cc.get();
    if isspliced(so) {
        if let Some(sp) = so.so_sp.get() {
            if show_pointers {
                kf.so_splice = sp.ssp_socket.get().map_or(0, |s| ptrtoint64(s));
            }
            kf.so_splicelen = sp.ssp_len.get();
        }
    } else if issplicedback(so) {
        kf.so_splicelen = -1;
    }
    if so.so_pcb.get().is_null() {
        return;
    }
    let family = so.dom_family();
    if family == i32::from(AF_INET) {
        let Some(inpcb) = sotoinpcb(so) else {
            return;
        };
        soassertlocked(so);
        if show_pointers {
            kf.inp_ppcb = ptrtoint64(inpcb.inp_ppcb.get());
        }
        kf.inp_lport = u32::from(inpcb.inp_lport.get());
        kf.inp_laddru[0] = inpcb.inp_laddr.get().s_addr;
        kf.inp_fport = u32::from(inpcb.inp_fport.get());
        kf.inp_faddru[0] = inpcb.inp_faddr.get().s_addr;
        kf.inp_rtableid = inpcb.inp_rtableid.get();
        if so.so_type.get() == SOCK_RAW {
            kf.inp_proto = u32::from(inpcb.inp_ip.get().ip_p);
        }
        if i32::from(so.so_proto.pr_protocol) == IPPROTO_TCP {
            fill_file_tcpcb(kf, inpcb);
        }
    } else if family == i32::from(AF_INET6) {
        let Some(inpcb) = sotoinpcb(so) else {
            return;
        };
        soassertlocked(so);
        if show_pointers {
            kf.inp_ppcb = ptrtoint64(inpcb.inp_ppcb.get());
        }
        kf.inp_lport = u32::from(inpcb.inp_lport.get());
        let laddr6 = inpcb.inp_laddr6.get();
        for (i, w) in kf.inp_laddru.iter_mut().enumerate() {
            *w = laddr6.s6_addr32(i);
        }
        kf.inp_fport = u32::from(inpcb.inp_fport.get());
        let faddr6 = inpcb.inp_faddr6.get();
        for (i, w) in kf.inp_faddru.iter_mut().enumerate() {
            *w = faddr6.s6_addr32(i);
        }
        kf.inp_rtableid = inpcb.inp_rtableid.get();
        if so.so_type.get() == SOCK_RAW {
            kf.inp_proto = u32::from(inpcb.inp_ipv6.get().ip6_nxt);
        }
        if i32::from(so.so_proto.pr_protocol) == IPPROTO_TCP {
            fill_file_tcpcb(kf, inpcb);
        }
    } else if family == i32::from(AF_UNIX) {
        let Some(unpcb) = sotounpcb(so) else {
            return;
        };
        kf.f_msgcount = unpcb.unp_msgcount.get() as u32;
        if show_pointers {
            kf.unp_conn = unpcb.unp_conn.get().map_or(0, |u| ptrtoint64(u));
            kf.unp_refs = unpcb.unp_refs.first().map_or(0, |u| ptrtoint64(u));
            kf.unp_nextref = SlistHead::<UnpRefs>::next(unpcb).map_or(0, |u| ptrtoint64(u));
            kf.v_un = unpcb.unp_vnode.get().map_or(0, |v| ptrtoint64(v));
            kf.unp_addr = unpcb.unp_addr.get().map_or(0, |m| ptrtoint64(m));
        }
        if let Some(m) = unpcb.unp_addr.get() {
            // SAFETY: a bound address mbuf holds `m_len` bytes of `struct sockaddr_un` and
            // lives while the control block is bound; the caller holds the socket lock.
            let addr = unsafe {
                core::slice::from_raw_parts(mtod::<u8>(m).cast_const(), m.m_len().get() as usize)
            };
            unp_path_copy(&mut kf.unp_path, addr);
        }
    }
}

/// The TCP members of `kinfo_file` (`fill_file`'s `intotcpcb(inpcb)` block, shared by the
/// `AF_INET` and `AF_INET6` cases); a control block without a `tcpcb` (the C would
/// dereference NULL) leaves them zero.
fn fill_file_tcpcb(kf: &mut KinfoFile, inpcb: &Inpcb) {
    let Some(tcpcb) = intotcpcb(inpcb) else {
        return;
    };
    kf.t_rcv_wnd = tcpcb.rcv_wnd.get();
    kf.t_snd_wnd = tcpcb.snd_wnd.get();
    kf.t_snd_cwnd = tcpcb.snd_cwnd.get();
    kf.t_state = tcpcb.t_state.get() as u32;
}

/// The output of a `sysctl_file` walk: the C's `kf`, `dp`, `buflen`, `elem_count` and
/// `needed`, which its `FILLIT` and `FILLINPTABLE` macros advance.
struct FileWalk<'a> {
    kf: KinfoFile,
    p: &'a Proc,
    show_pointers: bool,
    dp: usize,
    buflen: usize,
    elem_size: usize,
    elem_count: usize,
    outsize: usize,
    needed: usize,
}

impl FileWalk<'_> {
    /// Whether the buffer has room for one more element.
    fn room(&self) -> bool {
        self.buflen >= self.elem_size && self.elem_count > 0
    }

    /// Copies the filled `kf` out and advances past its element.
    fn put(&mut self) -> Result<(), Errno> {
        copyout(&self.kf.as_bytes()[..self.outsize], self.dp)?;
        self.dp += self.elem_size;
        self.buflen -= self.elem_size;
        self.elem_count -= 1;
        Ok(())
    }

    /// `FILLIT(fp, fdp, i, vp, pr)`.
    fn fillit(
        &mut self,
        fp: Option<&FileRef>,
        fdp: Option<&Filedesc>,
        i: i32,
        vp: Option<&'static Vnode>,
        pr: Option<&Process>,
    ) -> Result<(), Errno> {
        if self.room() {
            fill_file(
                &mut self.kf,
                fp,
                fdp,
                i,
                vp,
                pr,
                self.p,
                None,
                self.show_pointers,
            );
            self.put()?;
        }
        self.needed += self.elem_size;
        Ok(())
    }

    /// `FILLINPTABLE(table)`: the sockets of a protocol control block table, which also
    /// holds the closed connections no file refers to any more.
    fn fillinptable(&mut self, table: &Inpcbtable) -> Result<(), Errno> {
        let iter = InpcbIterator::new();
        let mut inp: Option<&'static Inpcb> = None;
        let mut error = Ok(());

        mtx_enter(&table.inpt_mtx);
        loop {
            // SAFETY: the table mutex is held; `iter` stays in place on this frame and serves
            // this walk only, until the iterator returns None or the walk is aborted below.
            inp = unsafe { in_pcb_iterator(table, inp, &iter) };
            let Some(i) = inp else {
                break;
            };
            if self.room() {
                mtx_leave(&table.inpt_mtx);
                net_lock_shared();
                let Some(so) = in_pcbsolock(i) else {
                    net_unlock_shared();
                    mtx_enter(&table.inpt_mtx);
                    continue;
                };
                fill_file(
                    &mut self.kf,
                    None,
                    None,
                    0,
                    None,
                    None,
                    self.p,
                    Some(so),
                    self.show_pointers,
                );
                in_pcbsounlock(Some(i), Some(so));
                net_unlock_shared();
                let r = copyout(&self.kf.as_bytes()[..self.outsize], self.dp);
                mtx_enter(&table.inpt_mtx);
                if let Err(e) = r {
                    // SAFETY: the mutex is held and `iter` is the iterator of this walk,
                    // whose last answer was `i`.
                    unsafe { in_pcb_iterator_abort(table, Some(i), &iter) };
                    error = Err(e);
                    break;
                }
                self.dp += self.elem_size;
                self.buflen -= self.elem_size;
                self.elem_count -= 1;
            }
            self.needed += self.elem_size;
        }
        mtx_leave(&table.inpt_mtx);
        error
    }

    /// The `KERN_FILE_BYPID`/`KERN_FILE_BYUID` body for one process: its text vnode (when
    /// `text`), current and root directories, then every open descriptor.
    fn fill_process(&mut self, pr: &Process, text: bool) -> Result<(), Errno> {
        let fdp = pr.fd();
        if text && let Some(vp) = pr.ps_textvp.get() {
            self.fillit(None, None, KERN_FILE_TEXT, Some(vp), Some(pr))?;
        }
        if let Some(vp) = fdp.cdir() {
            self.fillit(None, None, KERN_FILE_CDIR, Some(vp), Some(pr))?;
        }
        if let Some(vp) = fdp.rdir() {
            self.fillit(None, None, KERN_FILE_RDIR, Some(vp), Some(pr))?;
        }
        // ps_tracevp (KERN_FILE_TRACE) does not exist: KTRACE is not configured.
        let mut i = 0;
        while i < fdp.nfiles() as i32 {
            if let Some(fp) = fd_getfile(fdp, i) {
                let r = self.fillit(Some(&fp), Some(fdp), i, None, Some(pr));
                let _ = frele(fp, self.p);
                r?;
            }
            i += 1;
        }
        Ok(())
    }
}

/// Whether `sysctl_file`'s process walks skip `pr`: system, exiting, embryonic and undead
/// processes.
fn file_skips(pr: &Process) -> bool {
    pr.ps_flags.load(Ordering::Relaxed) & (PS_SYSTEM | PS_EMBRYO | PS_EXITING) != 0
}

/// `LIST_FOREACH(pr, &allprocess, ps_list)` with `refcnt_take(&pr->ps_refcnt)` around the
/// work on a process, for a holder of the kernel lock: `pick` chooses under the lock's
/// reader (it must not sleep), and takes the process's refcnt there, as the C does after
/// its skip test; `work` runs without the reader (it may sleep) while the refcnt keeps the
/// process from `exit1`'s unlink, so the walk goes on from it. `Break` ends the walk.
fn allprocess_foreach(
    mut pick: impl FnMut(&Process) -> bool,
    mut work: impl FnMut(&Process) -> ControlFlow<Result<(), Errno>>,
) -> Result<(), Errno> {
    let mut resume: Option<ProcessRef> = None;
    loop {
        let next = {
            let r = klock::reader();
            let t = proc_tables();
            let mut cur = match &resume {
                None => t.allprocess.first(&r),
                Some(pr) => t.allprocess.next(&r, pr),
            };
            loop {
                match cur {
                    None => break None,
                    Some(pr) if pick(pr) => {
                        pr.ps_refcnt.take();
                        break Some(process_ref(pr));
                    }
                    Some(pr) => cur = t.allprocess.next(&r, pr),
                }
            }
        };
        if let Some(done) = resume.take() {
            done.ps_refcnt.rele_wake();
        }
        let Some(pr) = next else {
            return Ok(());
        };
        if let ControlFlow::Break(r) = work(&pr) {
            pr.ps_refcnt.rele_wake();
            return r;
        }
        resume = Some(pr);
    }
}

/// `sysctl_file`: get file structures (`kern.file`; `name` is op, arg, element size,
/// element count).
pub fn sysctl_file(name: &[i32], where_: usize, sizep: &mut usize, p: &Proc) -> Result<(), Errno> {
    if name.len() > 4 {
        return Err(Errno::ENOTDIR);
    }
    if name.len() < 4 || name[2] < 0 || name[2] as usize > size_of::<KinfoFile>() {
        return Err(Errno::EINVAL);
    }

    let op = name[0];
    let arg = name[1];
    let elem_size = name[2] as usize;
    // As the C's size_t: a negative count is no limit.
    let elem_count = name[3] as usize;

    if elem_size < 1 {
        return Err(Errno::EINVAL);
    }

    let mut w = FileWalk {
        kf: KinfoFile::zeroed(),
        p,
        show_pointers: curproc().is_some_and(|cp| suser(cp).is_ok()),
        dp: where_,
        buflen: if where_ != 0 { *sizep } else { 0 },
        elem_size,
        elem_count,
        outsize: size_of::<KinfoFile>().min(elem_size),
        needed: 0,
    };

    match op {
        KERN_FILE_BYFILE => {
            // use the inp-tables to pick up closed connections, too
            if arg == DTYPE_SOCKET {
                w.fillinptable(&TCBTABLE)?;
                #[cfg(feature = "inet6")]
                w.fillinptable(&TCB6TABLE)?;
                w.fillinptable(&UDBTABLE)?;
                #[cfg(feature = "inet6")]
                w.fillinptable(&UDB6TABLE)?;
                w.fillinptable(&RAWCBTABLE)?;
                #[cfg(feature = "inet6")]
                w.fillinptable(&RAWIN6PCBTABLE)?;
                // NPF > 0
                w.fillinptable(&DIVBTABLE)?;
                #[cfg(feature = "inet6")]
                w.fillinptable(&DIVB6TABLE)?;
            }
            let mut fp: Option<FileRef> = None;
            loop {
                fp = fd_iterfile(fp.take(), p);
                let Some(f) = fp.as_ref() else {
                    break;
                };
                if arg != 0 && f.f_type() != arg {
                    continue;
                }
                let skip = arg == DTYPE_SOCKET && {
                    let af = fp_socket(f).dom_family();
                    af == i32::from(AF_INET) || af == i32::from(AF_INET6)
                };
                if !skip {
                    kernel_lock();
                    let r = w.fillit(Some(f), None, 0, None, None);
                    kernel_unlock();
                    if let Err(e) = r {
                        if let Some(f) = fp.take() {
                            let _ = frele(f, p);
                        }
                        return Err(e);
                    }
                }
            }
        }
        KERN_FILE_BYPID => {
            // A arg of -1 indicates all processes
            if arg < -1 {
                return Err(Errno::EINVAL);
            }
            let mut matched = false;
            kernel_lock();
            let r = allprocess_foreach(
                // not the pid we are looking for: skipped
                |pr| !file_skips(pr) && (arg < 0 || pr.ps_pid == arg),
                |pr| {
                    matched = true;
                    if let Err(e) = w.fill_process(pr, true) {
                        return ControlFlow::Break(Err(e));
                    }
                    // pid is unique, stop searching
                    if arg >= 0 {
                        return ControlFlow::Break(Ok(()));
                    }
                    ControlFlow::Continue(())
                },
            );
            kernel_unlock();
            r?;
            if !matched {
                return Err(Errno::ESRCH);
            }
        }
        KERN_FILE_BYUID => {
            kernel_lock();
            let r = allprocess_foreach(
                // not the uid we are looking for: skipped
                |pr| !file_skips(pr) && (arg < 0 || pr.ucred().cr_uid.get() == arg as u32),
                |pr| match w.fill_process(pr, false) {
                    Err(e) => ControlFlow::Break(Err(e)),
                    Ok(()) => ControlFlow::Continue(()),
                },
            );
            kernel_unlock();
            r?;
        }
        _ => return Err(Errno::EINVAL),
    }

    let mut needed = w.needed;
    let mut error = Ok(());
    if where_ == 0 {
        needed += KERN_FILESLOP * elem_size;
    } else if *sizep < needed {
        error = Err(Errno::ENOMEM);
    }
    *sizep = needed;
    error
}

/// `PTRTOINT64` (`<sys/sysctl.h>`).
fn ptrtoint64<T>(x: *const T) -> u64 {
    x as usize as u64
}

/// The per-disk copy of `sysctl_diskinit`: `dk`'s name and statistics into `sdk`.
fn disk_stats_copy(sdk: &mut Diskstats, dk: &Disk) {
    strlcpy(&mut sdk.ds_name, &dk.dk_name.get());
    mtx_enter(&dk.dk_mtx);
    sdk.ds_busy = dk.dk_busy.get();
    sdk.ds_rxfer = dk.dk_rxfer.get();
    sdk.ds_wxfer = dk.dk_wxfer.get();
    sdk.ds_seek = dk.dk_seek.get();
    sdk.ds_rbytes = dk.dk_rbytes.get();
    sdk.ds_wbytes = dk.dk_wbytes.get();
    sdk.ds_attachtime = dk.dk_attachtime.get();
    sdk.ds_timestamp = dk.dk_timestamp.get();
    sdk.ds_time = dk.dk_time.get();
    mtx_leave(&dk.dk_mtx);
}

/// The disk list (`subr_disk.c`'s `disklist`, `disk_count`, `disk_change`) as the kernel's
/// `sysctl_diskinit` reads it (`kernel::sysctl::DiskExport`).
struct BsdDiskExport;

impl kernel::sysctl::DiskExport for BsdDiskExport {
    fn take_change(&self) -> bool {
        DISK_CHANGE.swap(0, Ordering::Relaxed) != 0
    }

    fn count(&self) -> usize {
        DISK_COUNT.load(Ordering::Relaxed).max(0) as usize
    }

    fn for_each(&self, f: &mut kernel::sysctl::DiskVisit<'_>) {
        for dk in DISKLIST.0.iter() {
            let label_uid = dk.label().map(|lp| lp.d_uid).filter(|u| !duid_iszero(u));
            let duid = label_uid.map(|u| duid_format(&u));
            let name = dk.dk_name.get();
            let mut sdk = Diskstats::default();
            disk_stats_copy(&mut sdk, dk);
            f(
                &name[..strnlen(&name, DS_DISKNAMELEN)],
                duid.as_ref().map(|d| &d[..]),
                sdk.as_bytes(),
            );
        }
    }
}

static BSD_DISK_EXPORT: BsdDiskExport = BsdDiskExport;

const _: () = assert!(size_of::<Diskstats>() == kernel::sysctl::DISKSTATS_SIZE);

/// A node of a subsystem still in `bsd` (`kernel::sysctl::SysctlNode`): its function over
/// the name below it and the request.
struct Node(fn(&[i32], &mut SysctlReq<'_>) -> Result<(), Errno>);

impl SysctlNode for Node {
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        (self.0)(name, req)
    }
}

/// A `sysctlfn` of a subsystem still in `bsd` that takes the calling thread (`net_sysctl`,
/// `vfs_sysctl`, `cpu_sysctl`, `ddb_sysctl`, `sysctl_file`): `curproc`.
struct ProcNode(Sysctlfn);

impl SysctlNode for ProcNode {
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        let Some(p) = curproc() else {
            panic(format_args!("sysctl: no curproc"));
        };
        (self.0)(name, req.oldp, req.oldlenp, req.newp, req.newlen, p)
    }
}

/// `kern.clockrate`.
fn kern_clockrate(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_clockrate(req.oldp, req.oldlenp, req.newp)
}

/// `kern.numvnodes` (XXX numvnodes is a long).
fn kern_numvnodes(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_rdint(
        req.oldp,
        req.oldlenp,
        req.newp,
        NUMVNODES.load(Ordering::Relaxed) as i32,
    )
}

/// `kern.boottime`.
fn kern_boottime(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let bt = microboottime();
    sysctl_rdstruct(req.oldp, req.oldlenp, req.newp, bt.as_bytes())
}

/// `kern.maxclusters`: a change is `nmbclust_update`'s, under `sysctl_lock`.
fn kern_maxclusters(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let oldval = NMBCLUST.load(Ordering::Relaxed) as i32;
    let newval = AtomicI32::new(oldval);
    let mut error = sysctl_int(req.oldp, req.oldlenp, req.newp, req.newlen, &newval);
    let newval = newval.into_inner();

    if error.is_ok() && oldval != newval {
        let _w = SYSCTL_LOCK.write();
        error = nmbclust_update(i64::from(newval));
    }

    error
}

/// `kern.mbstat`.
fn kern_mbstat(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let mut counters = [0u64; MBS_NCOUNTERS];
    crate::kern::subr_percpu::counters_read(mbstat(), &mut counters, MBS_NCOUNTERS, None);
    let mut mbs = Mbstat::default();
    mbs.m_mtypes.copy_from_slice(&counters[..MT_NTYPES]);
    mbs.m_drops = counters[MbstatCounters::MbsDrops as usize];
    mbs.m_wait = counters[MbstatCounters::MbsWait as usize];
    mbs.m_drain = counters[MbstatCounters::MbsDrain as usize];
    mbs.m_defrag_alloc = counters[MbstatCounters::MbsDefragAlloc as usize];
    mbs.m_prepend_alloc = counters[MbstatCounters::MbsPrependAlloc as usize];
    mbs.m_pullup_alloc = counters[MbstatCounters::MbsPullupAlloc as usize];
    mbs.m_pullup_copy = counters[MbstatCounters::MbsPullupCopy as usize];
    mbs.m_pulldown_alloc = counters[MbstatCounters::MbsPulldownAlloc as usize];
    mbs.m_pulldown_copy = counters[MbstatCounters::MbsPulldownCopy as usize];
    sysctl_rdstruct(req.oldp, req.oldlenp, req.newp, mbs.as_bytes())
}

/// `kern.cp_time`: the online CPUs' `spc_cp_time`, averaged.
fn kern_cptime(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let mut cp_time = [0i64; CPUSTATES];
    let mut n = 0i64;

    cpu_info_foreach(&mut |ci| {
        if !cpu_is_online(ci) {
            return;
        }

        n += 1;
        let ci_cp_time = sysctl_ci_cp_time(ci);
        for (sum, t) in cp_time.iter_mut().zip(ci_cp_time) {
            *sum += t as i64;
        }
    });

    if n > 0 {
        for t in &mut cp_time {
            *t /= n;
        }
    }

    sysctl_rdstruct(req.oldp, req.oldlenp, req.newp, cp_time.as_bytes())
}

/// `kern.pool_debug`: a change reclaims every pool.
fn kern_pool_debug(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let oldval = POOL_DEBUG.load(Ordering::Relaxed);
    let newval = AtomicI32::new(oldval);

    let error = sysctl_int(req.oldp, req.oldlenp, req.newp, req.newlen, &newval);
    let newval = newval.into_inner();
    if error.is_ok()
        && oldval != newval
        && POOL_DEBUG
            .compare_exchange(oldval, newval, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        pool_reclaim_all();
    }

    error
}

/// `kern.nchstats`.
fn kern_nchstats(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let mut bytes = [0u8; 12 * size_of::<u64>()];
    for (chunk, v) in bytes.chunks_mut(size_of::<u64>()).zip(NCHSTATS.snapshot()) {
        chunk.copy_from_slice(&v.to_ne_bytes());
    }
    sysctl_rdstruct(req.oldp, req.oldlenp, req.newp, &bytes)
}

/// `kern.stackgap_random`: a power of two, at least `ALIGNBYTES` (or 0), under `MAXSSIZ`.
fn kern_stackgaprandom(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    use crate::kern::kern_exec::stackgap_random;

    let stackgap = AtomicI32::new(stackgap_random.load(Ordering::Relaxed));
    sysctl_int(req.oldp, req.oldlenp, req.newp, req.newlen, &stackgap)?;
    let stackgap = stackgap.into_inner();
    // Safety harness.
    let alignbytes = <crate::machine::Machine as crate::machine::param::MachineParam>::ALIGNBYTES;
    let maxssiz = <crate::machine::Machine as crate::machine::VmParam>::MAXSSIZ;
    if (stackgap < alignbytes as i32 && stackgap != 0)
        || (stackgap.wrapping_sub(1) & stackgap) != 0
        || stackgap as i64 >= maxssiz as i64
    {
        return Err(Errno::EINVAL);
    }
    stackgap_random.store(stackgap, Ordering::Relaxed);
    Ok(())
}

/// `kern.bufcachepercent`: between 5 and 90; a change adjusts the buffer cache.
fn kern_cachepct(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    use crate::conf::param::{bufcachepercent, bufpages};

    let opct = bufcachepercent.load(Ordering::Relaxed);
    sysctl_int(
        req.oldp,
        req.oldlenp,
        req.newp,
        req.newlen,
        &bufcachepercent,
    )?;
    let pct = bufcachepercent.load(Ordering::Relaxed);
    if !(5..=90).contains(&pct) {
        bufcachepercent.store(opct, Ordering::Relaxed);
        return Err(Errno::EINVAL);
    }
    let pages = uvm_pagecount(&NO_CONSTRAINT) as u64;
    if pct != opct {
        let pgs = (pct as u64 * pages / 100) as i32;
        bufadjust(i64::from(pgs)); // adjust bufpages
        // set high water mark
        BUFHIGHPAGES.store(bufpages.load(Ordering::Relaxed), Ordering::Relaxed);
    }
    Ok(())
}

/// `kern.pfstatus` (`NPF`).
fn kern_pfstatus(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    crate::net::pf_ioctl::pf_sysctl(req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `kern.consdev`.
fn kern_consdev(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let dev = cn_tab().map_or(NODEV, |cn| cn.cn_dev.get());
    sysctl_rdstruct(req.oldp, req.oldlenp, req.newp, dev.as_bytes())
}

/// `kern.utc_offset`.
fn kern_utc_offset(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_utc_offset(req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `kern.malloc`.
fn kern_malloc(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_malloc(name, req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `kern.cp_time2`.
fn kern_cptime2(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_cptime2(name, req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `kern.pool`.
fn kern_pool(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_dopool(name, req.oldp, req.oldlenp)
}

/// `kern.cpustats`.
fn kern_cpustats(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_cpustats(name, req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `kern.audio`.
fn kern_audio(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_audio(name, req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `kern.intrcnt`.
fn kern_intrcnt(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_intrcnt(name, req.oldp, req.oldlenp)
}

/// `kern.watchdog`.
fn kern_watchdog(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_wdog(name, req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `kern.evcount`.
fn kern_evcount(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    evcount_sysctl(name, req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `hw.sensors`.
fn hw_sensors(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_sensors(name, req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `hw.ucomnames` (`NUCOM > 0`).
fn hw_ucomnames(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let names = sysctl_ucominit();
    sysctl_rdstring(req.oldp, req.oldlenp, req.newp, names.as_bytes())
}

/// `hw.smt`.
fn hw_smt(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_hwsmt(req.oldp, req.oldlenp, req.newp, req.newlen)
}

/// `hw.blockcpu`.
fn hw_blockcpu(_name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    sysctl_hwblockcpu(req.oldp, req.oldlenp, req.newp, req.newlen)
}

static OSTYPE_NODE: RdString = RdString(OSTYPE.as_bytes());
static OSRELEASE_NODE: RdString = RdString(OSRELEASE.as_bytes());
static OSVERSION_NODE: RdString = RdString(OSVERSION.as_bytes());
static VERSION_NODE: RdString = RdString(VERSION.as_bytes());
static MACHINE_NODE: RdString = RdString(<Machine as MachineInfo>::MACHINE.as_bytes());

/// The `kern` nodes of `bsd`'s subsystems, by id.
static KERN_NODES: [(i32, &dyn SysctlNode); 29] = [
    (KERN_OSTYPE, &OSTYPE_NODE),
    (KERN_OSRELEASE, &OSRELEASE_NODE),
    (KERN_OSVERSION, &OSVERSION_NODE),
    (KERN_VERSION, &VERSION_NODE),
    (KERN_CLOCKRATE, &Node(kern_clockrate)),
    (KERN_NUMVNODES, &Node(kern_numvnodes)),
    (KERN_BOOTTIME, &Node(kern_boottime)),
    (KERN_MAXCLUSTERS, &Node(kern_maxclusters)),
    (KERN_MBSTAT, &Node(kern_mbstat)),
    (KERN_CPTIME, &Node(kern_cptime)),
    (KERN_POOL_DEBUG, &Node(kern_pool_debug)),
    (KERN_NCHSTATS, &Node(kern_nchstats)),
    (KERN_STACKGAPRANDOM, &Node(kern_stackgaprandom)),
    (KERN_CACHEPCT, &Node(kern_cachepct)),
    (KERN_PFSTATUS, &Node(kern_pfstatus)),
    (KERN_CONSDEV, &Node(kern_consdev)),
    (KERN_UTC_OFFSET, &Node(kern_utc_offset)),
    (KERN_FILE, &ProcNode(sysctl_file_node)),
    (KERN_MALLOCSTATS, &Node(kern_malloc)),
    (KERN_CPTIME2, &Node(kern_cptime2)),
    (KERN_POOL, &Node(kern_pool)),
    (KERN_CPUSTATS, &Node(kern_cpustats)),
    (KERN_AUDIO, &Node(kern_audio)),
    (KERN_INTRCNT, &Node(kern_intrcnt)),
    (KERN_WATCHDOG, &Node(kern_watchdog)),
    (KERN_EVCOUNT, &Node(kern_evcount)),
    // `kern.clockintr` and `kern.timecounter`: the kernel's nodes (N4.8a).
    (KERN_CLOCKINTR, &kernel::clockintr::ClockintrNode),
    // `kern.tty`: the kernel's node (N4.7b), over this glue's terminals.
    (KERN_TTY, &kernel::tty::TtyNode::<BsdProcGlue>::NODE),
    (KERN_TIMECOUNTER, &kernel::tc::TcNode),
];

/// `sysctl_intrcnt`: `kern.intrcnt`, served by `evcount_sysctl`.
pub fn sysctl_intrcnt(name: &[i32], oldp: usize, oldlenp: &mut usize) -> Result<(), Errno> {
    evcount_sysctl(name, oldp, oldlenp, 0, 0)
}

/// `sysctl_sensors`: `hw.sensors.<dev>` (a `struct sensordev`) and
/// `hw.sensors.<dev>.<type>.<numt>` (a `struct sensor`), copies without the kernel pointers.
pub fn sysctl_sensors(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let _ = newlen;

    if name.len() != 1 && name.len() != 3 {
        return Err(Errno::ENOTDIR);
    }

    let dev = name[0];
    if name.len() == 1 {
        kernel_lock();
        let ksd = match sensordev_get(dev) {
            Ok(ksd) => ksd,
            Err(e) => {
                kernel_unlock();
                return Err(e);
            }
        };

        // Grab a copy, to clear the kernel pointers
        let mut usd = Sensordev {
            num: ksd.num.get(),
            maxnumt: ksd.maxnumt.get(),
            sensors_count: ksd.sensors_count.get(),
            ..Sensordev::default()
        };
        strlcpy(&mut usd.xname, &ksd.xname.get());
        kernel_unlock();

        return sysctl_rdstruct(oldp, oldlenp, newp, usd.as_bytes());
    }

    // `(enum sensor_type)name[1]`: a value outside the enum matches no sensor.
    let r#type = SensorType::from_i32(name[1]);
    let numt = name[2];

    kernel_lock();
    let ks = match r#type {
        Some(t) => sensor_find(dev, t, numt),
        None => sensordev_get(dev).and(Err(Errno::ENOENT)),
    };
    let ks = match ks {
        Ok(ks) => ks,
        Err(e) => {
            kernel_unlock();
            return Err(e);
        }
    };

    // Grab a copy, to clear the kernel pointers
    let us = Sensor {
        desc: ks.desc.get(),
        tv: ks.tv.get(),
        value: ks.value.get(),
        r#type: ks.r#type.get() as i32,
        status: ks.status.get() as i32,
        numt: ks.numt.get(),
        flags: ks.flags.get(),
    };
    kernel_unlock();

    sysctl_rdstruct(oldp, oldlenp, newp, us.as_bytes())
}

/// `sysctl_cpustats`: `kern.cpustats.<cpu>`.
pub fn sysctl_cpustats(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let _ = newlen;
    let [unit] = *name else {
        return Err(Errno::ENOTDIR);
    };

    let Some(ci) = cpu_by_unit(unit) else {
        return Err(Errno::ENOENT);
    };

    let mut cs = Cpustats {
        cs_time: sysctl_ci_cp_time(ci),
        cs_flags: 0,
    };
    if cpu_is_online(ci) {
        cs.cs_flags |= CPUSTATS_ONLINE;
    }

    sysctl_rdstruct(oldp, oldlenp, newp, cs.as_bytes())
}

/// The `CPU_INFO_FOREACH` search of `sysctl_cpustats` and `sysctl_cptime2`.
fn cpu_by_unit(unit: i32) -> Option<&'static CpuInfo> {
    let mut found = None;
    cpu_info_foreach(&mut |ci| {
        if found.is_none() && i64::from(unit) == i64::from(Machine::cpu_info_unit(ci)) {
            found = Some(ci);
        }
    });
    found
}

/// `sysctl_ci_cp_time`: a consistent copy of a CPU's `spc_cp_time`.
fn sysctl_ci_cp_time(ci: &CpuInfo) -> [u64; CPUSTATES] {
    let spc = Machine::ci_schedstate(ci);
    let mut generation = 0;
    let mut cp_time = [0u64; CPUSTATES];

    pc_cons_enter(&spc.spc_cp_time_lock, &mut generation);
    loop {
        for (t, c) in cp_time.iter_mut().zip(&spc.spc_cp_time) {
            *t = c.load(Ordering::Relaxed);
        }
        if !pc_cons_leave(&spc.spc_cp_time_lock, &mut generation) {
            break;
        }
    }
    cp_time
}

/// `sysctl_cptime2`: `kern.cp_time2.<cpu>`.
pub fn sysctl_cptime2(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let _ = newlen;
    let [unit] = *name else {
        return Err(Errno::ENOTDIR);
    };

    let Some(ci) = cpu_by_unit(unit) else {
        return Err(Errno::ENOENT);
    };

    let cp_time = sysctl_ci_cp_time(ci);

    sysctl_rdstruct(oldp, oldlenp, newp, cp_time.as_bytes())
}

/// `sysctl_audio`: `kern.audio.record` and `kern.audio.kbdcontrol` (audio(4)'s
/// `audio_record_enable` and `audio_kbdcontrol_enable`; the latter with `NWSKBD > 0`, as in
/// GENERIC).
pub fn sysctl_audio(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let [n] = *name else {
        return Err(Errno::ENOTDIR);
    };
    let intptr = match n {
        KERN_AUDIO_RECORD => &AUDIO_RECORD_ENABLE,
        KERN_AUDIO_KBDCONTROL => &AUDIO_KBDCONTROL_ENABLE,
        _ => return Err(Errno::ENOENT),
    };
    sysctl_int(oldp, oldlenp, newp, newlen, intptr)
}

/// `sysctl_utc_offset`: `kern.utc_offset`, in minutes; a change steps the real-time clock
/// by the difference.
pub fn sysctl_utc_offset(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let old_offset_minutes = UTC_OFFSET.load(Ordering::Relaxed) / 60; // seconds -> minutes
    let new_offset_minutes = AtomicI32::new(old_offset_minutes);
    sysctl_securelevel_int(oldp, oldlenp, newp, newlen, &new_offset_minutes)?;
    let new_offset_minutes = new_offset_minutes.into_inner();
    if !(-24 * 60..=24 * 60).contains(&new_offset_minutes) {
        return Err(Errno::EINVAL);
    }
    if new_offset_minutes == old_offset_minutes {
        return Ok(());
    }

    UTC_OFFSET.store(new_offset_minutes * 60, Ordering::Relaxed); // minutes -> seconds
    let adjustment_seconds = (new_offset_minutes - old_offset_minutes) * 60;

    let now = nanotime();
    let mut adjusted = now;
    adjusted.tv_sec -= i64::from(adjustment_seconds);
    tc_setrealtimeclock(&adjusted);
    let _ = unported!("kern.utc_offset: resettodr (kern_time.c)");

    Ok(())
}

/// `kern.file` as a `sysctlfn` (it reads no new value).
fn sysctl_file_node(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    _newp: usize,
    _newlen: usize,
    p: &Proc,
) -> Result<(), Errno> {
    sysctl_file(name, oldp, oldlenp, p)
}

/// The `hw` nodes of `bsd`'s subsystems, by id.
static HW_NODES: [(i32, &dyn SysctlNode); 5] = [
    (HW_MACHINE, &MACHINE_NODE),
    (HW_SENSORS, &Node(hw_sensors)),
    (HW_UCOMNAMES, &Node(hw_ucomnames)),
    (HW_SMT, &Node(hw_smt)),
    (HW_BLOCKCPU, &Node(hw_blockcpu)),
];

/// The top-level trees of `bsd`'s subsystems, by id (`ddb_sysctl` with `DDB`).
static TOP_NODES: [(i32, &dyn SysctlNode); 4] = [
    (CTL_NET, &ProcNode(crate::kern::uipc_domain::net_sysctl)),
    (
        CTL_VFS,
        &kernel::fs::subr::VfsSysctl::<crate::kern::proc_glue::BsdProcGlue>::NODE,
    ),
    (CTL_MACHDEP, &ProcNode(crate::machine::cpu::cpu_sysctl)),
    (CTL_DDB, &ProcNode(crate::ddb::db_usrreq::ddb_sysctl)),
];

/// Registers `bsd`'s sysctl nodes, bounded variables, disk list and `physmem` with the
/// kernel's dispatcher (`kernel::sysctl`), once, before the first `sysctl(2)` (`main`, with
/// the other bridges); a second call changes nothing.
pub fn sysctl_register_legacy() {
    for &(ctl, n) in &TOP_NODES {
        let _ = kernel::sysctl::register_top(ctl, n);
    }
    for &(id, n) in &KERN_NODES {
        let _ = kernel::sysctl::register_kern(id, n);
    }
    for &(id, n) in &HW_NODES {
        let _ = kernel::sysctl::register_hw(id, n);
    }
    let _ = kernel::sysctl::register_kern_vars(&KERN_VARS);
    let _ = kernel::sysctl::register_hw_vars(&HW_VARS);
    let _ = kernel::sysctl::install_disk_export(&BSD_DISK_EXPORT);
    let _ = kernel::sysctl::register_physmem(&crate::sys::systm::PHYSMEM);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `kern_sysctl.rs`: the helpers' C semantics on old and new buffers (sizes,
    // `ENOMEM`/`EPERM`/`EINVAL`, truncation, bounds), the bounded tables, the identity nodes,
    // `hw` nodes that need no hardware, `kern.proc` sizing and `fill_kproc` over a process built
    // by hand, `fill_file`/`kern.file` checks and sizing, the `hw.disk*` nodes over a disk
    // put on the list by hand, and `sysctl(2)` itself through its argument registers.

    use std::assert_eq;
    use std::boxed::Box;
    use std::sync::MutexGuard;

    use super::*;
    use crate::kern::proc_glue::{procinit, test_process, test_setsid, test_thread};
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::time::{Clockinfo, Timeval};
    use crate::sys::ucred::CredRef;
    use crate::sys::ucred::Ucred;

    /// A user address for a buffer of the test.
    fn ua<T>(b: &mut T) -> usize {
        b as *mut T as usize
    }

    /// A user address of a read-only value.
    fn ra<T>(b: &T) -> usize {
        b as *const T as usize
    }

    #[test]
    fn rdint_sizes_reads_and_refuses_writes() {
        let mut len = 0;
        assert_eq!(sysctl_rdint(0, &mut len, 0, 42), Ok(()));
        assert_eq!(len, 4);

        let mut out = 0i32;
        let mut len = 3;
        assert_eq!(
            sysctl_rdint(ua(&mut out), &mut len, 0, 42),
            Err(Errno::ENOMEM)
        );
        assert_eq!(len, 3);

        let mut len = 4;
        assert_eq!(sysctl_rdint(ua(&mut out), &mut len, 0, 42), Ok(()));
        assert_eq!(out, 42);

        let new = 7i32;
        assert_eq!(sysctl_rdint(0, &mut len, ra(&new), 42), Err(Errno::EPERM));
    }

    #[test]
    fn int_swaps_the_old_value_out() {
        let var = AtomicI32::new(5);
        let mut out = 0i32;
        let mut len = 4;
        let new = -9i32;
        assert_eq!(
            sysctl_int(ua(&mut out), &mut len, ra(&new), 4, &var),
            Ok(())
        );
        assert_eq!((out, var.load(Ordering::Relaxed), len), (5, -9, 4));

        // A new value of the wrong size is EINVAL, and nothing changes.
        assert_eq!(
            sysctl_int(0, &mut len, ra(&new), 2, &var),
            Err(Errno::EINVAL)
        );
        // No old buffer: the value is just stored.
        let new = 11i32;
        assert_eq!(sysctl_int(0, &mut len, ra(&new), 4, &var), Ok(()));
        assert_eq!(var.load(Ordering::Relaxed), 11);
    }

    #[test]
    fn int_bounded_checks_both_bounds_and_read_only() {
        let var = AtomicI32::new(50);
        let mut len = 4;
        let too_big = 101i32;
        let too_small = -1i32;
        let fine = 100i32;
        assert_eq!(
            sysctl_int_bounded(0, &mut len, ra(&too_big), 4, &var, 0, 100),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_int_bounded(0, &mut len, ra(&too_small), 4, &var, 0, 100),
            Err(Errno::EINVAL)
        );
        assert_eq!(var.load(Ordering::Relaxed), 50);
        assert_eq!(
            sysctl_int_bounded(0, &mut len, ra(&fine), 4, &var, 0, 100),
            Ok(())
        );
        assert_eq!(var.load(Ordering::Relaxed), 100);

        // minimum > maximum: read-only, refused before anything is looked at.
        let (min, max) = SYSCTL_INT_READONLY;
        let mut len = 0;
        assert_eq!(
            sysctl_int_bounded(0, &mut len, ra(&fine), 4, &var, min, max),
            Err(Errno::EPERM)
        );
        let mut out = 0i32;
        let mut len = 4;
        assert_eq!(
            sysctl_int_bounded(ua(&mut out), &mut len, 0, 0, &var, min, max),
            Ok(())
        );
        assert_eq!(out, 100);
    }

    #[test]
    fn int_lower_never_raises() {
        let var = AtomicI32::new(3);
        let mut len = 4;
        let higher = 4i32;
        assert_eq!(
            sysctl_int_lower(0, &mut len, ra(&higher), 4, &var),
            Err(Errno::EPERM)
        );
        let lower = 1i32;
        let mut out = 0i32;
        assert_eq!(
            sysctl_int_lower(ua(&mut out), &mut len, ra(&lower), 4, &var),
            Ok(())
        );
        assert_eq!((out, var.load(Ordering::Relaxed)), (3, 1));
    }

    #[test]
    fn rdquad_is_eight_bytes() {
        let mut out = 0i64;
        let mut len = 7;
        assert_eq!(
            sysctl_rdquad(ua(&mut out), &mut len, 0, 1 << 40),
            Err(Errno::ENOMEM)
        );
        let mut len = 8;
        assert_eq!(sysctl_rdquad(ua(&mut out), &mut len, 0, 1 << 40), Ok(()));
        assert_eq!(out, 1 << 40);
    }

    #[test]
    fn strings_count_the_nul_and_refuse_short_buffers() {
        let mut out = [0xffu8; 16];
        let mut len = 0;
        assert_eq!(sysctl_rdstring(0, &mut len, 0, b"EmiBSD"), Ok(()));
        assert_eq!(len, 7);

        let mut len = 6;
        assert_eq!(
            sysctl_rdstring(ua(&mut out), &mut len, 0, b"EmiBSD"),
            Err(Errno::ENOMEM)
        );
        let mut len = out.len();
        assert_eq!(
            sysctl_rdstring(ua(&mut out), &mut len, 0, b"EmiBSD\0junk"),
            Ok(())
        );
        assert_eq!((len, &out[..8]), (7, &b"EmiBSD\0\xff"[..]));

        let new = *b"x\0";
        assert_eq!(
            sysctl_rdstring(0, &mut len, ra(&new), b"EmiBSD"),
            Err(Errno::EPERM)
        );
    }

    #[test]
    fn string_reads_then_writes_and_terminates() {
        let mut var = [0u8; 8];
        var[..3].copy_from_slice(b"old");
        let new = *b"newname";
        let mut out = [0xffu8; 8];
        let mut len = out.len();
        assert_eq!(
            sysctl_string(ua(&mut out), &mut len, ra(&new), 4, &mut var),
            Ok(())
        );
        assert_eq!((len, &out[..4]), (4, &b"old\0"[..]));
        assert_eq!(&var[..5], b"newn\0");

        // newlen must leave room for the NUL.
        assert_eq!(
            sysctl_string(0, &mut len, ra(&new), 8, &mut var),
            Err(Errno::EINVAL)
        );
        // A short old buffer is ENOMEM for sysctl_string...
        let mut len = 2;
        assert_eq!(
            sysctl_string(ua(&mut out), &mut len, 0, 0, &mut var),
            Err(Errno::ENOMEM)
        );
    }

    #[test]
    fn tstring_truncates_with_a_nul() {
        let mut var = [0u8; 16];
        var[..8].copy_from_slice(b"hostname");
        let mut out = [0xffu8; 8];
        let mut len = 5;
        assert_eq!(
            sysctl_tstring(ua(&mut out), &mut len, 0, 0, &mut var),
            Ok(())
        );
        assert_eq!((len, &out[..6]), (5, &b"host\0\xff"[..]));

        let mut len = 0;
        assert_eq!(
            sysctl_tstring(ua(&mut out), &mut len, 0, 0, &mut var),
            Err(Errno::ENOMEM)
        );
    }

    #[test]
    fn structs_copy_whole_and_rdstruct_is_read_only() {
        let tv = Timeval {
            tv_sec: 1,
            tv_usec: 2,
        };
        let mut out = Timeval::default();
        let mut len = 15;
        assert_eq!(
            sysctl_rdstruct(ua(&mut out), &mut len, 0, tv.as_bytes()),
            Err(Errno::ENOMEM)
        );
        let mut len = 16;
        assert_eq!(
            sysctl_rdstruct(ua(&mut out), &mut len, 0, tv.as_bytes()),
            Ok(())
        );
        assert_eq!(out, tv);
        assert_eq!(
            sysctl_rdstruct(0, &mut len, ra(&tv), tv.as_bytes()),
            Err(Errno::EPERM)
        );

        let mut var = Clockinfo::default();
        let new = Clockinfo {
            hz: 100,
            tick: 10_000,
            stathz: 128,
            profhz: 1024,
        };
        let mut len = 0;
        assert_eq!(
            sysctl_struct(0, &mut len, ra(&new), 17, var.as_bytes_mut()),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_struct(0, &mut len, ra(&new), 16, var.as_bytes_mut()),
            Ok(())
        );
        assert_eq!(var, new);
    }

    #[test]
    fn bounded_arr_finds_the_mib() {
        static A: AtomicI32 = AtomicI32::new(7);
        let table = [SysctlBoundedArgs::readonly(3, &A)];
        let mut out = 0i32;
        let mut len = 4;
        assert_eq!(
            sysctl_bounded_arr(&table, &[3, 1], 0, &mut len, 0, 0),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            sysctl_bounded_arr(&table, &[4], 0, &mut len, 0, 0),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            sysctl_bounded_arr(&table, &[3], ua(&mut out), &mut len, 0, 0),
            Ok(())
        );
        assert_eq!(out, 7);
    }

    /// A thread of a fresh process with credentials `uid`, in its own session and process
    /// group, as `fork1` would leave it.
    fn thread(uid: u32) -> &'static Proc {
        let mut v = Ucred::new();
        v.cr_uid.set(uid);
        v.cr_ruid.set(uid);
        let cr = CredRef::new(v);
        let pr = test_process(42);
        let p = test_thread(pr);
        drop(p.set_cred(cr.clone()));
        drop(pr.set_ucred(cr));
        pr.set_main_static(hal::pin::pin_static(p));
        // The group (pgid 42) and the session it leads: the test's references are leaked
        // with the process.
        core::mem::forget(test_setsid(pr));
        pr.set_comm(b"test");
        p
    }

    fn setup() -> MutexGuard<'static, ()> {
        let guard = setup_real_memory();
        crate::machine::cons::consinit();
        procinit();
        guard
    }

    #[test]
    fn fill_file_reports_the_file_and_its_process() {
        let _g = setup();
        let p = thread(1000);
        let pr = p.process();

        let mut v = Ucred::new();
        v.cr_uid.set(1000);
        v.cr_gid.set(20);
        crate::kern::kern_descrip::filedesc_init();
        let opener = test_thread(test_process(43));
        drop(opener.set_cred(CredRef::new(v)));
        let fp = crate::kern::kern_descrip::fnew(opener).unwrap();
        // Never closed: the file and its second reference (a table's) are leaked with the
        // test.
        fp.install(
            crate::sys::file::DTYPE_DMABUF,
            &crate::kern::vfs_vnops::VNOPS,
            BsdFileData::None,
        );
        fp.f_flag.store((FREAD | FWRITE) as u32, Ordering::Relaxed);
        let other_ref = fp.clone();
        fp.set_offset(77);

        let mut kf = KinfoFile::zeroed();
        fill_file(&mut kf, Some(&fp), None, 3, None, Some(pr), p, None, false);
        assert_eq!((kf.fd_fd, kf.f_type, kf.f_count), (3, 5, 2));
        assert_eq!((kf.f_flag, kf.f_uid, kf.f_gid), (3, 1000, 20));
        // The owner sees the offset; no pointers without root.
        assert_eq!((kf.f_offset, kf.f_fileaddr, kf.f_data), (77, 0, 0));
        assert_eq!((kf.p_pid, kf.p_uid, kf.p_tid), (42, 1000, u32::MAX));
        assert_eq!(&kf.p_comm[..5], b"test\0");

        // Another user does not.
        let other = thread(1001);
        fill_file(&mut kf, Some(&fp), None, 3, None, None, other, None, true);
        assert_eq!(kf.f_offset, u64::MAX);
        assert_eq!(kf.f_fileaddr, fp.addr() as u64);
        assert_eq!(kf.p_pid, 0);
        core::mem::forget(other_ref);
        core::mem::forget(fp);
    }

    #[test]
    fn sysctl_file_checks_and_sizes() {
        let _g = setup();
        let p = thread(0);
        let size = size_of::<KinfoFile>() as i32;

        let mut len = 0;
        assert_eq!(
            sysctl_file(&[KERN_FILE_BYPID, -1, size], 0, &mut len, p),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_file(&[KERN_FILE_BYPID, -1, size, 1, 0], 0, &mut len, p),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            sysctl_file(&[KERN_FILE_BYPID, -1, size + 1, 1], 0, &mut len, p),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_file(&[KERN_FILE_BYPID, -1, 0, 1], 0, &mut len, p),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_file(&[KERN_FILE_BYPID, -2, size, 1], 0, &mut len, p),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_file(&[9, 0, size, 1], 0, &mut len, p),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sysctl_file(&[KERN_FILE_BYPID, 999_999, size, 1], 0, &mut len, p),
            Err(Errno::ESRCH)
        );
        // Nobody has this uid: only the slop is asked for.
        assert_eq!(
            sysctl_file(&[KERN_FILE_BYUID, 54_321, size, 10], 0, &mut len, p),
            Ok(())
        );
        assert_eq!(len, KERN_FILESLOP * size as usize);
    }

    #[test]
    fn unp_path_copy_honours_sun_len() {
        let mut addr = [0u8; 16];
        addr[0] = 2 + 5;
        addr[1] = 1;
        addr[2..7].copy_from_slice(b"/sock");
        addr[7] = b'X';
        let mut path = [0u8; KI_UNPPATHLEN];
        unp_path_copy(&mut path, &addr);
        assert_eq!(&path[..6], b"/sock\0");

        // A sun_len past the mbuf's bytes is cut at the mbuf.
        addr[0] = 200;
        let mut path = [0u8; KI_UNPPATHLEN];
        unp_path_copy(&mut path, &addr);
        assert_eq!(&path[..15], b"/sockX\0\0\0\0\0\0\0\0\0");
        unp_path_copy(&mut path, &[]);
    }

    /// This module's range of the kernel's user-copy double (the kernel dispatcher's copies).
    const UB: usize = kernel::testutil::USER_BASE + 0x4000;

    /// `n` bytes at the double's user address `at`.
    fn user(at: usize, n: usize) -> std::vec::Vec<u8> {
        kernel::testutil::user_bytes(at, n).unwrap()
    }

    /// The identity nodes `bsd` registers with the kernel's dispatcher: EmiBSD 8.0, and the
    /// bounded integers of `bsd`'s table after the kernel's own.
    #[test]
    fn the_registered_nodes_report_emibsd_8_0() {
        let _g = setup();
        sysctl_register_legacy();
        let p = thread(1000);
        let out = UB;

        let mut len = 32;
        let mut req = SysctlReq {
            oldp: out,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            kernel::sysctl::kern_sysctl(p, &[KERN_OSTYPE], &mut req),
            Ok(())
        );
        assert_eq!(user(out, len), b"EmiBSD\0".to_vec());

        let mut len = 32;
        let mut req = SysctlReq {
            oldp: out,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            kernel::sysctl::kern_sysctl(p, &[KERN_OSRELEASE], &mut req),
            Ok(())
        );
        assert_eq!(user(out, len), b"8.0\0".to_vec());

        let mut len = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            kernel::sysctl::kern_sysctl(p, &[KERN_VERSION], &mut req),
            Ok(())
        );
        assert_eq!(len, VERSION.len() + 1);

        // A read-only node refuses a new value; kern.osrevision is bsd's table's.
        let mut len = 4;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: out,
            newlen: 8,
        };
        assert_eq!(
            kernel::sysctl::kern_sysctl(p, &[KERN_OSTYPE], &mut req),
            Err(Errno::EPERM)
        );
        let mut len = 4;
        let mut req = SysctlReq {
            oldp: out,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            kernel::sysctl::kern_sysctl(p, &[KERN_OSREV], &mut req),
            Ok(())
        );
        assert_eq!(user(out, 4), (OpenBSD as i32).to_ne_bytes().to_vec());
        // PTRACE is not configured: kern.global_ptrace is nobody's.
        assert_eq!(
            kernel::sysctl::kern_sysctl(p, &[KERN_GLOBAL_PTRACE], &mut req),
            Err(Errno::EOPNOTSUPP)
        );
        // hw.machine and hw.ncpu are bsd's.
        let mut len = 16;
        let mut req = SysctlReq {
            oldp: out,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            kernel::sysctl::hw_sysctl(p, &[HW_MACHINE], &mut req),
            Ok(())
        );
        assert_eq!(user(out, len), b"host\0".to_vec());
        let mut len = 4;
        let mut req = SysctlReq {
            oldp: out,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(kernel::sysctl::hw_sysctl(p, &[HW_NCPU], &mut req), Ok(()));
        assert_eq!(
            user(out, 4),
            NCPUS.load(Ordering::Relaxed).to_ne_bytes().to_vec()
        );
    }

    /// `sysctl(2)` through its `sysent[]` row, the kernel's dispatcher behind it.
    #[test]
    fn sysctl_2_reads_kern_ostype() {
        let _g = setup();
        sysctl_register_legacy();
        let p = thread(1000);

        let name = UB + 0x100;
        let out = UB + 0x140;
        let oldlenp = UB + 0x180;
        let put = |at: usize, b: &[u8]| UserPtr::<u8>::new(at).copyout_slice(b).unwrap();
        let mut ids = [0u8; 8];
        ids[..4].copy_from_slice(&CTL_KERN.to_ne_bytes());
        ids[4..].copy_from_slice(&KERN_OSTYPE.to_ne_bytes());
        put(name, &ids);
        put(oldlenp, &16usize.to_ne_bytes());
        let mut retval = [0; 2];
        let args = [
            name as Register,
            2,
            out as Register,
            oldlenp as Register,
            0,
            0,
        ];
        assert_eq!(sys_sysctl(p, &args, &mut retval), Ok(()));
        assert_eq!(user(oldlenp, 8), 7usize.to_ne_bytes().to_vec());
        assert_eq!(user(out, 7), b"EmiBSD\0".to_vec());

        // One component is not enough; a new value needs root.
        let args = [name as Register, 1, 0, 0, 0, 0];
        assert_eq!(sys_sysctl(p, &args, &mut retval), Err(Errno::EINVAL));
        let args = [name as Register, 2, 0, 0, out as Register, 4];
        assert_eq!(sys_sysctl(p, &args, &mut retval), Err(Errno::EPERM));
        let mut ids = [0u8; 8];
        ids[..4].copy_from_slice(&CTL_DEBUG.to_ne_bytes());
        put(name, &ids);
        let args = [name as Register, 2, 0, 0, 0, 0];
        assert_eq!(sys_sysctl(p, &args, &mut retval), Err(Errno::EOPNOTSUPP));
    }

    #[test]
    fn the_disk_export_follows_the_disklist() {
        use kernel::sysctl::DiskExport;

        let _g = setup();

        // Other tests may have left their disks on the list: this one is the last.
        let dk: &'static Disk = Box::leak(Box::new(Disk::new()));
        let mut name = [0u8; DS_DISKNAMELEN];
        name[..4].copy_from_slice(b"tst9");
        dk.dk_name.set(name);
        dk.dk_rxfer.set(5);
        dk.dk_rbytes.set(4096);
        if DISKLIST.0.is_empty() {
            DISKLIST.0.init();
        }
        // SAFETY: the disk is leaked, in no list; the setup guard serialises the tests.
        unsafe { DISKLIST.0.insert_tail(dk) };
        DISK_COUNT.fetch_add(1, Ordering::Relaxed);
        DISK_CHANGE.store(1, Ordering::Relaxed);

        assert!(BSD_DISK_EXPORT.take_change());
        assert!(!BSD_DISK_EXPORT.take_change());
        assert_eq!(
            BSD_DISK_EXPORT.count(),
            DISK_COUNT.load(Ordering::Relaxed) as usize
        );
        dk.dk_wxfer.set(9);
        let mut last = None;
        BSD_DISK_EXPORT.for_each(&mut |name, duid, ds| {
            let mut sdk = Diskstats::default();
            sdk.as_bytes_mut().copy_from_slice(ds);
            last = Some((name.to_vec(), duid.map(<[u8]>::to_vec), sdk));
        });
        let (name, duid, sdk) = last.unwrap();
        assert_eq!((&name[..], duid), (&b"tst9"[..], None));
        assert_eq!(&sdk.ds_name[..5], b"tst9\0");
        assert_eq!((sdk.ds_rxfer, sdk.ds_wxfer, sdk.ds_rbytes), (5, 9, 4096));

        // SAFETY: inserted above; the guard is still held.
        unsafe { DISKLIST.0.remove(dk) };
        DISK_COUNT.fetch_sub(1, Ordering::Relaxed);
        DISK_CHANGE.store(1, Ordering::Relaxed);
    }

    #[test]
    fn kern_proc_readers_give_back_only_the_reference_they_took() {
        // kernel::sysctl's fill_kproc and sysctl_proc_args read a live process's vmspace
        // under a reference of their own (the C's uvmspace_addref ... uvmspace_free,
        // kern_sysctl.c:2073-2074 and 2124, 2208-2209 and 2362): the process's vm_refcnt is
        // the same after the read, and its map keeps its entries. Without the addref the free
        // dropped the process's own reference, and `ps` purged every address space it
        // listed, its own first (smoke-diag's `uvm_map_pageable: ... has no entry`; main's
        // 2536a3e6 fixed bsd's copies, this is the kernel's).
        use crate::sys::param::PAGE_SIZE;
        use kernel::sysctl::{SysctlReq, fill_kproc, sysctl_proc_args};

        let _g = setup();
        // The map pools (the pmap's are up: setup_real_memory's pmap_bootstrap_lazy).
        kernel::vm::map::uvm_map_init();
        // A pooled process (what prfind counts), in its own group and session.
        let p = crate::kern::proc_glue::test_proc(43, CredRef::new(Ucred::new()));
        let pr = p.process();
        core::mem::forget(test_setsid(pr));
        let vm = kernel::vm::map::uvmspace_alloc(0x1000, 0x4000_0000, true, false);
        let rw = crate::sys::mman::PROT_READ | crate::sys::mman::PROT_WRITE;
        let flags = kernel::vm::types::uvm_mapflag(
            rw,
            rw,
            hal::sys::mman::MAP_INHERIT_COPY,
            hal::sys::mman::MADV_NORMAL,
            kernel::vm::types::UVM_FLAG_FIXED | kernel::vm::types::UVM_FLAG_COPYONW,
        );
        let mut at = 0x10_0000;
        assert_eq!(
            kernel::vm::map::uvm_mapanon(&vm.vm_map, &mut at, PAGE_SIZE, 0, flags),
            Ok(())
        );
        assert!(pr.set_vmspace(Some(vm.clone())).is_none());
        assert_eq!(vm.vm_refcnt.load(Ordering::Relaxed), 1);

        let mut ki = KinfoProc::zeroed();
        for _ in 0..2 {
            kernel_lock();
            let r = hal::klock::reader();
            let g = pr.ps_mtx.lock();
            let held = fill_kproc(&r, pr, &mut ki, None, false, &g);
            drop((g, r));
            // The fill's reference outlives ps_mtx: the caller gives it back after it.
            assert_eq!(vm.vm_refcnt.load(Ordering::Relaxed), 2);
            drop(held);
            kernel_unlock();
        }
        assert_eq!(vm.vm_refcnt.load(Ordering::Relaxed), 1);

        // kern.procargs.<pid>.nargv: the victim found on the pid hash, its ps_strings (0,
        // never mapped) read through its map; the answer is an error, the reference is
        // given back all the same.
        kernel_lock();
        {
            let mut w = hal::klock::witness();
            let t = crate::kern::proc_glue::proc_tables();
            t.pidhash(pr.ps_pid)
                .insert_head(&mut w, core::pin::Pin::static_ref(pr));
        }
        let mut len = size_of::<i32>();
        let mut req = SysctlReq {
            oldp: UB + 0x200,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert!(sysctl_proc_args(p, &[pr.ps_pid, KERN_PROC_NARGV], &mut req).is_err());
        {
            let mut w = hal::klock::witness();
            let t = crate::kern::proc_glue::proc_tables();
            t.pidhash(pr.ps_pid).remove(&mut w, pr);
        }
        kernel_unlock();
        assert_eq!(vm.vm_refcnt.load(Ordering::Relaxed), 1);

        // The map still describes the page mapped above.
        let g = vm.vm_map.lock_read();
        let e = g.lookup(at);
        assert!(e.is_some_and(|e| e.start() == at && e.end() == at + PAGE_SIZE));
        drop(g);
        drop(pr.set_vmspace(None));
    }
}
/* </TESTS> */
