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
 *	@(#)kern_descrip.c	8.6 (Berkeley) 4/19/94
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
 *	@(#)file.h	8.2 (Berkeley) 8/20/94
 */
/* </LICENSES> */

/* <CODE> */
//! `bsd`'s descriptors over the kernel's (`kernel::file`, `kernel::descrip`): the glue the
//! kernel's open files and tables are instantiated with ([`BsdFileGlue`]: the objects a
//! file holds, the hooks into the vnode layer, the sockets, kqueue, signals and pledge), the
//! `sysent[]` rows of the descriptor system calls, and the `/dev/fd` driver.
//!
//! Upstream: sys/kern/kern_descrip.c @ 3ce1f3f79392
//! Upstream: sys/sys/file.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_descrip.rs@3c62ede68802
//! LZ: sys/sys/file.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.3: `kern_descrip.c`'s descriptor management moved into `kernel::descrip` and
//!   `struct file` into `kernel::file`; this module is what stays `bsd`'s until the
//!   subsystems a file points at move: [`BsdFileData`] (`f_data` by kind: a vnode link, a
//!   socket's address word, a pipe's end, a kqueue link), [`BsdFdKq`] (a table's
//!   `fd_kqlist` and `fd_nuserevents`, kqueue's until N4.4), the hooks of
//!   [`BsdFileGlue`] (a `bridges.toml` row, `FileGlue`), the open files' tables, the
//!   `sysent[]` rows (`sys_dup` ... `sys_getdtablecount`, which call the kernel's typed
//!   entries), `filedescopen`, and the twenty `Native` entries of `KernelApi::fd`
//!   ([`FD_ENTRIES`]: each runs `mi_syscall` on its own `sysent[]` row, so the prologue,
//!   pledge and the pinned syscalls stay on the path as for the vm calls). The module keeps LZ's path, which the frozen amd64
//!   `conf.rs` imports `filedescopen` from (decision 44).

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hal::link::{Link, PtrWord};
use kernel::api::{FdEntries, SysResult};
use kernel::file::{FileGlue, FileTables, LockOwner};

pub use kernel::descrip::{
    dupfdopen, fd_checkclosed, fd_getfile, fd_getfile_flags, fd_iterfile, fdalloc, fdcopy,
    fdexpand, fdfree, fdinit, fdinsert, fdprepforexec, fdrelease, fdremove, fdshare, finishdup,
};
pub use kernel::file::fdrop;

use crate::conf::param::MAXFILES;
use crate::kern::init_sysent::SYSENT;
use crate::kern::kern_event::knote_fdclose;
use crate::kern::kern_pledge::{pledge_fcntl, pledge_flock, pledge_ioctl};
use crate::kern::kern_rwlock::{rw_enter_read, rw_exit_read};
use crate::kern::kern_sig::ptsignal;
use crate::kern::pipe_glue::PipeFile;
use crate::kern::subr_prf::{panic, tablefull};
use crate::kern::vfs_subr::{VFS_STALL_LOCK, vref, vrele};
use crate::kern::vfs_vnops::vn_lock;
use crate::kern::vfs_vops::{VOP_ADVLOCK, VOP_PATHCONF, VOP_UNLOCK};
use crate::machine::copy::copyout;
use crate::machine::cpu::curproc;
#[cfg(not(test))]
use crate::machine::intr::IPL_MPFLOOR;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::eventvar::{KqList, Kqueue};
use crate::sys::fcntl::Flock;
use crate::sys::file::{File, FileRef, Fileops};
use crate::sys::filedesc::FdGuard;
use crate::sys::lock::{LK_EXCLUSIVE, LK_RETRY};
use crate::sys::proc::Proc;
use crate::sys::signal::SIGPIPE;
use crate::sys::signalvar::SignalType;
use crate::sys::socketvar::{SS_DNS, Socket};
use crate::sys::stat::Stat;
use crate::sys::syscall::{
    SYS_close, SYS_closefrom, SYS_dup, SYS_dup2, SYS_dup3, SYS_fcntl, SYS_flock, SYS_fpathconf,
    SYS_fstat, SYS_getdtablecount, SYS_ioctl, SYS_pread, SYS_preadv, SYS_pwrite, SYS_pwritev,
    SYS_read, SYS_readv, SYS_utrace, SYS_write, SYS_writev,
};
use crate::sys::syscall_mi::mi_syscall;
use crate::sys::syscallargs::{
    SysCloseArgs, SysClosefromArgs, SysDup2Args, SysDup3Args, SysDupArgs, SysFcntlArgs,
    SysFlockArgs, SysFpathconfArgs, SysFstatArgs,
};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::{Dev, Off, Register, minor};
use crate::sys::vnode::{VCHR, VFIFO, VISTTY, Vnode};

/// `f_data`: what a `bsd` file is open on.
pub enum BsdFileData {
    /// `DTYPE_VNODE`: the vnode the file holds a use count on.
    Vnode(Link<Vnode>),
    /// `DTYPE_SOCKET`: the socket, as an address (`Socket` is not `Sync`; `fp_socket` keeps
    /// its deref until the sockets move, N6), cleared by `soo_close`.
    Socket(PtrWord<Socket>),
    /// `DTYPE_PIPE`: the pipe this file reads, and the opener's ids for `pipe_stat`.
    Pipe(PipeFile),
    /// `DTYPE_KQUEUE`: the kqueue, cleared by `kqueue_close`.
    Kqueue(Link<Kqueue>),
    /// A file with no object (the console stand-in, `DTYPE_CONSFILE`).
    None,
}

impl BsdFileData {
    /// A vnode file's object.
    pub fn vnode(vp: &'static Vnode) -> Self {
        Self::Vnode(Link::to(vp))
    }

    /// A socket file's object.
    pub fn socket(so: &Socket) -> Self {
        let word = PtrWord::new();
        word.set(core::ptr::from_ref(so));
        Self::Socket(word)
    }
}

/// A table's kqueue members (kqueue's, N4.4): `fd_kqlist` and `fd_nuserevents`.
#[derive(Default)]
pub struct BsdFdKq {
    /// \[f\] `fd_kqlist`: kqueues attached to this filedesc.
    pub fd_kqlist: hal::list::List<KqList>,
    /// \[a\] `fd_nuserevents`: number of kqueue user events.
    pub fd_nuserevents: AtomicU32,
}

/// `bsd`'s instantiation of `kernel::file` and `kernel::descrip` (a `bridges.toml` row while
/// `bsd` implements it).
pub struct BsdFileGlue;

kernel::file_glue_links!(BsdFileGlue);

/// `numfiles`: actual number of open files (`kern.nfiles`).
pub static NUMFILES: AtomicI32 = AtomicI32::new(0);

/// The open files and the pools of `bsd`'s descriptors.
#[cfg(not(test))]
static FILE_TABLES: FileTables<BsdFileGlue> = FileTables::new(IPL_MPFLOOR, &NUMFILES);

#[cfg(test)]
std::thread_local! {
    /// Each host test thread's own tables ([`filedesc_init`] makes fresh ones).
    static TEST_TABLES: core::cell::Cell<&'static FileTables<BsdFileGlue>> =
        core::cell::Cell::new(std::boxed::Box::leak(std::boxed::Box::new(FileTables::new(0, &NUMFILES))));
}

/// The tables of `bsd`'s open files.
pub fn file_tables() -> &'static FileTables<BsdFileGlue> {
    #[cfg(not(test))]
    {
        &FILE_TABLES
    }
    #[cfg(test)]
    {
        TEST_TABLES.with(core::cell::Cell::get)
    }
}

/// The vnode of a vnode file's object.
fn vnode_of(d: &BsdFileData) -> &'static Vnode {
    match d {
        BsdFileData::Vnode(v) => match v.get() {
            Some(vp) => vp,
            None => crate::kern::subr_prf::panic(format_args!("vnode file: no vnode")),
        },
        _ => crate::kern::subr_prf::panic(format_args!("not a vnode file")),
    }
}

/// A record lock owner as `VOP_ADVLOCK`'s `id` (compared, never followed).
fn owner_id(owner: LockOwner) -> *const core::ffi::c_void {
    match owner {
        LockOwner::Table(a) | LockOwner::File(a) => core::ptr::without_provenance(a),
    }
}

impl FileGlue for BsdFileGlue {
    type Proc = crate::kern::proc_glue::BsdProcGlue;
    type Ops = Fileops;
    type Data = BsdFileData;
    type Knote = Knote;
    type Stat = Stat;
    type Vnode = Vnode;
    type FileList = FileListLink;
    type FdKq = BsdFdKq;

    fn tables() -> &'static FileTables<Self> {
        file_tables()
    }

    fn maxfiles() -> i32 {
        MAXFILES.load(Ordering::Relaxed)
    }

    fn tablefull(tab: &str) {
        tablefull(tab);
    }

    fn stall_wait() {
        rw_enter_read(&VFS_STALL_LOCK);
        rw_exit_read(&VFS_STALL_LOCK);
    }

    fn vref(v: &'static Vnode) {
        vref(v);
    }

    fn vrele(v: &'static Vnode) {
        vrele(v);
    }

    fn advlock(
        d: &BsdFileData,
        owner: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        VOP_ADVLOCK(vnode_of(d), owner_id(owner), op, fl, flags)
    }

    fn pathconf(d: &BsdFileData, name: i32) -> Result<Register, Errno> {
        let vp = vnode_of(d);
        let mut retval: Register = 0;
        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
        let error = VOP_PATHCONF(vp, name, &mut retval);
        let _ = VOP_UNLOCK(vp);
        error.map(|()| retval)
    }

    fn isatty(d: &BsdFileData) -> bool {
        vnode_of(d).v_flag.get() & VISTTY != 0
    }

    fn position_check(d: &BsdFileData, offset: Off) -> Result<(), Errno> {
        let vp = vnode_of(d);
        if vp.v_type.get() == VFIFO || vp.v_flag.get() & VISTTY != 0 {
            return Err(Errno::ESPIPE);
        }
        if offset < 0 && vp.v_type.get() != VCHR {
            return Err(Errno::EINVAL);
        }
        Ok(())
    }

    fn dns_socket(d: &BsdFileData) -> bool {
        match d {
            BsdFileData::Socket(so) => {
                crate::kern::sys_socket::socket_of(so).is_some_and(|so| so.has_state(SS_DNS))
            }
            _ => false,
        }
    }

    fn knote_fdclose(p: &Proc, fd: i32) {
        knote_fdclose(p, fd);
    }

    fn sigpipe(p: &Proc) {
        ptsignal(p, SIGPIPE, SignalType::STHREAD);
    }

    fn pledge_fcntl(p: &Proc, cmd: i32) -> Result<(), Errno> {
        pledge_fcntl(p, cmd)
    }

    fn pledge_flock(p: &Proc) -> Result<(), Errno> {
        pledge_flock(p)
    }

    fn pledge_ioctl(p: &Proc, com: u64, fp: &File) -> Result<(), Errno> {
        pledge_ioctl(p, com, fp)
    }

    fn stat_hide_gen(st: &mut Stat) {
        st.st_gen = 0;
    }

    fn stat_copyout(st: &Stat, sb: usize) -> Result<(), Errno> {
        copyout(&st.to_bytes(), sb)
    }

    fn fdkq_free(kq: &BsdFdKq) {
        crate::kassert!(kq.fd_nuserevents.load(Ordering::SeqCst) == 0);
    }
}

/// `filedesc_init`: the pools of open files and descriptor tables. A host test calls it once
/// per test: its thread gets fresh tables first.
pub fn filedesc_init() {
    #[cfg(test)]
    TEST_TABLES.with(|t| {
        t.set(std::boxed::Box::leak(std::boxed::Box::new(
            FileTables::new(0, &NUMFILES),
        )))
    });
    if kernel::descrip::filedesc_init::<BsdFileGlue>().is_err() {
        crate::kern::subr_prf::panic(format_args!("filedesc_init: cannot make the pools"));
    }
}

/// `fp->f_count` of a file a holder names without a reference (`unp_gc`'s `unp_file`, a
/// message's word), read as the C reads it (uipc_usrreq.c's `unp_gc`): no reference is
/// taken, so the read can never be the last holder whose drop closes the socket under
/// `unp_gc_lock`; 0 once the last is gone.
pub fn file_count(fp: &File) -> u32 {
    fp.f_count()
}

/// `falloc(p)` with the table locked: a new file (the caller's reference) and its descriptor.
pub fn falloc(p: &Proc, g: &mut FdGuard<'_>) -> Result<(FileRef, i32), Errno> {
    kernel::descrip::falloc(p, g)
}

/// `fnew(p)`: a new open file with one reference and the thread's credentials.
pub fn fnew(p: &Proc) -> Option<FileRef> {
    kernel::file::fnew(p)
}

/// `closef(fp, p)`: the reference a slot or a message held, given back after the table's
/// POSIX locks are released (a thread's close).
pub fn closef<'a>(fp: FileRef, p: impl Into<Option<&'a Proc>>) -> Result<(), Errno> {
    kernel::descrip::closef(fp, p.into())
}

/// `fd_getfile_mode(fdp, fd, mode)` of `p`'s table.
pub fn fd_getfile_mode(p: &Proc, fd: i32, mode: i32) -> Option<FileRef> {
    kernel::descrip::fd_getfile_mode(p, fd, mode)
}

/// Duplicate a file descriptor.
pub fn sys_dup(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysDupArgs = sysargs(v);
    retval[0] = kernel::descrip::sys_dup(p, uap.fd.get())? as Register;
    Ok(())
}

/// Duplicate a file descriptor to a particular value.
pub fn sys_dup2(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysDup2Args = sysargs(v);
    retval[0] = kernel::descrip::sys_dup2(p, uap.from.get(), uap.to.get())? as Register;
    Ok(())
}

/// `dup3(2)`: `dup2` with `O_CLOEXEC`/`O_CLOFORK`, refusing `from == to`.
pub fn sys_dup3(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysDup3Args = sysargs(v);
    retval[0] =
        kernel::descrip::sys_dup3(p, uap.from.get(), uap.to.get(), uap.flags.get())? as Register;
    Ok(())
}

/// The file control system call.
pub fn sys_fcntl(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFcntlArgs = sysargs(v);
    retval[0] = kernel::descrip::sys_fcntl(p, uap.fd.get(), uap.cmd.get(), uap.arg.get() as usize)?;
    Ok(())
}

/// Close a file descriptor.
pub fn sys_close(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysCloseArgs = sysargs(v);
    kernel::descrip::sys_close(p, uap.fd.get())
}

/// Return status information about a file descriptor.
pub fn sys_fstat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFstatArgs = sysargs(v);
    kernel::descrip::sys_fstat(p, uap.fd.get(), uap.sb.get() as usize)
}

/// Return pathconf information about a file descriptor.
pub fn sys_fpathconf(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFpathconfArgs = sysargs(v);
    retval[0] = kernel::descrip::sys_fpathconf(p, uap.fd.get(), uap.name.get())?;
    Ok(())
}

/// Apply an advisory lock on a file descriptor.
pub fn sys_flock(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFlockArgs = sysargs(v);
    kernel::descrip::sys_flock(p, uap.fd.get(), uap.how.get())
}

/// File Descriptor pseudo-device driver (`/dev/fd/`).
///
/// Opening minor device N dup()s the file (if any) connected to file descriptor N belonging
/// to the calling process. Note that this driver consists of only the ``open()'' routine,
/// because all subsequent references to this file will be direct to the other driver.
pub fn filedescopen(dev: Dev, _mode: i32, _type: i32, p: &Proc) -> Result<(), Errno> {
    // XXX Kludge: set curproc->p_dupfd to contain the value of the the file descriptor being
    // sought for duplication. The error return ensures that the vnode for this device will
    // be released by vn_open. Open will detect this special error and take the actions in
    // dupfdopen below. Other callers of vn_open or VOP_OPEN will simply report the error.
    p.p_dupfd.set(minor(dev) as i32);
    Err(Errno::ENODEV)
}

/// `closefrom(2)`: closes every descriptor from `fd` on.
pub fn sys_closefrom(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysClosefromArgs = sysargs(v);
    kernel::descrip::sys_closefrom(p, uap.fd.get())
}

/// `getdtablecount(2)`: the number of descriptors open.
pub fn sys_getdtablecount(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::descrip::sys_getdtablecount(p) as Register;
    Ok(())
}

/// `KernelApi::fd`: the `Native` entries of the descriptor system calls.
pub const FD_ENTRIES: FdEntries = FdEntries {
    read: native_read,
    write: native_write,
    close: native_close,
    getdtablecount: native_getdtablecount,
    dup: native_dup,
    fstat: native_fstat,
    ioctl: native_ioctl,
    dup2: native_dup2,
    fcntl: native_fcntl,
    dup3: native_dup3,
    readv: native_readv,
    writev: native_writev,
    flock: native_flock,
    pread: native_pread,
    pwrite: native_pwrite,
    preadv: native_preadv,
    pwritev: native_pwritev,
    fpathconf: native_fpathconf,
    utrace: native_utrace,
    closefrom: native_closefrom,
};

/// A `Native` descriptor entry: the thread on this CPU runs `mi_syscall` on row `nr` of
/// `sysent[]` (which must be `want`'s, else `ENOSYS`) with `args` and the trap's return
/// registers: the prologue (pledge, the pinned syscalls, the kernel lock) and the row above,
/// as on the `Legacy` path.
fn native(nr: Register, want: i32, args: SysArgs, rval: &mut [Register; 2]) -> SysResult {
    if nr != want as Register {
        return Err(Errno::ENOSYS.as_i32());
    }
    let Some(p) = curproc() else {
        panic(format_args!("native fd syscall: no curproc"));
    };
    mi_syscall(p, nr, &SYSENT[nr as usize], &args, rval).map_err(Errno::as_i32)
}

/// `KernelApi::fd.read`.
pub fn native_read(
    nr: Register,
    fd: i32,
    buf: usize,
    nbyte: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [fd as Register, buf as Register, nbyte as Register, 0, 0, 0];
    native(nr, SYS_read, args, rval)
}

/// `KernelApi::fd.write`.
pub fn native_write(
    nr: Register,
    fd: i32,
    buf: usize,
    nbyte: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [fd as Register, buf as Register, nbyte as Register, 0, 0, 0];
    native(nr, SYS_write, args, rval)
}

/// `KernelApi::fd.close`.
pub fn native_close(nr: Register, fd: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_close, [fd as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::fd.getdtablecount`.
pub fn native_getdtablecount(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_getdtablecount, [0; 6], rval)
}

/// `KernelApi::fd.dup`.
pub fn native_dup(nr: Register, fd: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_dup, [fd as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::fd.fstat`.
pub fn native_fstat(nr: Register, fd: i32, sb: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fstat,
        [fd as Register, sb as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::fd.ioctl`.
pub fn native_ioctl(
    nr: Register,
    fd: i32,
    com: u64,
    data: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [fd as Register, com as Register, data as Register, 0, 0, 0];
    native(nr, SYS_ioctl, args, rval)
}

/// `KernelApi::fd.dup2`.
pub fn native_dup2(nr: Register, from: i32, to: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_dup2,
        [from as Register, to as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::fd.fcntl`.
pub fn native_fcntl(
    nr: Register,
    fd: i32,
    cmd: i32,
    arg: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [fd as Register, cmd as Register, arg as Register, 0, 0, 0];
    native(nr, SYS_fcntl, args, rval)
}

/// `KernelApi::fd.dup3`.
pub fn native_dup3(
    nr: Register,
    from: i32,
    to: i32,
    flags: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [from as Register, to as Register, flags as Register, 0, 0, 0];
    native(nr, SYS_dup3, args, rval)
}

/// `KernelApi::fd.readv`.
pub fn native_readv(
    nr: Register,
    fd: i32,
    iovp: usize,
    iovcnt: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fd as Register,
        iovp as Register,
        iovcnt as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_readv, args, rval)
}

/// `KernelApi::fd.writev`.
pub fn native_writev(
    nr: Register,
    fd: i32,
    iovp: usize,
    iovcnt: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fd as Register,
        iovp as Register,
        iovcnt as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_writev, args, rval)
}

/// `KernelApi::fd.flock`.
pub fn native_flock(nr: Register, fd: i32, how: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_flock,
        [fd as Register, how as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::fd.pread`.
pub fn native_pread(
    nr: Register,
    fd: i32,
    buf: usize,
    nbyte: usize,
    offset: i64,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fd as Register,
        buf as Register,
        nbyte as Register,
        offset as Register,
        0,
        0,
    ];
    native(nr, SYS_pread, args, rval)
}

/// `KernelApi::fd.pwrite`.
pub fn native_pwrite(
    nr: Register,
    fd: i32,
    buf: usize,
    nbyte: usize,
    offset: i64,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fd as Register,
        buf as Register,
        nbyte as Register,
        offset as Register,
        0,
        0,
    ];
    native(nr, SYS_pwrite, args, rval)
}

/// `KernelApi::fd.preadv`.
pub fn native_preadv(
    nr: Register,
    fd: i32,
    iovp: usize,
    iovcnt: i32,
    offset: i64,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fd as Register,
        iovp as Register,
        iovcnt as Register,
        offset as Register,
        0,
        0,
    ];
    native(nr, SYS_preadv, args, rval)
}

/// `KernelApi::fd.pwritev`.
pub fn native_pwritev(
    nr: Register,
    fd: i32,
    iovp: usize,
    iovcnt: i32,
    offset: i64,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        fd as Register,
        iovp as Register,
        iovcnt as Register,
        offset as Register,
        0,
        0,
    ];
    native(nr, SYS_pwritev, args, rval)
}

/// `KernelApi::fd.fpathconf`.
pub fn native_fpathconf(nr: Register, fd: i32, name: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fpathconf,
        [fd as Register, name as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::fd.utrace`.
pub fn native_utrace(
    nr: Register,
    label: usize,
    addr: usize,
    len: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    let args = [
        label as Register,
        addr as Register,
        len as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_utrace, args, rval)
}

/// `KernelApi::fd.closefrom`.
pub fn native_closefrom(nr: Register, fd: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_closefrom, [fd as Register, 0, 0, 0, 0, 0], rval)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::kern::kern_event::tests::{setup, thread};
    use crate::sys::fcntl::{F_GETFD, F_SETFD, FD_CLOEXEC, FREAD, FWRITE};
    use crate::sys::file::{DTYPE_PIPE, frele};
    use crate::sys::filedesc::fdplock;
    use crate::sys::uio::Uio;

    fn ok_rw(_: &File, _: &mut Uio<'_>, _: i32) -> Result<(), Errno> {
        Ok(())
    }
    fn no_ioctl(_: &File, _: u64, _: &mut [u8], _: &Proc) -> Result<(), Errno> {
        Err(Errno::ENOTTY)
    }
    fn no_kq(_: &File, _: &Knote) -> Result<(), Errno> {
        Err(Errno::EINVAL)
    }
    fn ok_stat(_: &File, _: &mut Stat, _: &Proc) -> Result<(), Errno> {
        Ok(())
    }
    fn ok_close(_: &File, _: Option<&Proc>) -> Result<(), Errno> {
        Ok(())
    }

    /// A file of no object for the rows' tests.
    static TESTOPS: Fileops = Fileops {
        fo_read: ok_rw,
        fo_write: ok_rw,
        fo_ioctl: no_ioctl,
        fo_kqfilter: no_kq,
        fo_stat: ok_stat,
        fo_close: ok_close,
        fo_seek: None,
    };

    fn args(a: &[i64]) -> SysArgs {
        let mut v: SysArgs = [0; 6];
        for (slot, &x) in v.iter_mut().zip(a) {
            *slot = x as Register;
        }
        v
    }

    /// A file at the lowest free descriptor of `p`'s table.
    fn open(p: &Proc) -> i32 {
        let mut g = fdplock(p.fd());
        let (fp, fd) = falloc(p, &mut g).unwrap();
        fp.f_flag.store((FREAD | FWRITE) as u32, Ordering::Relaxed);
        fp.install(DTYPE_PIPE, &TESTOPS, BsdFileData::None);
        fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        frele(fp, p).unwrap();
        fd
    }

    #[test]
    fn the_sysent_rows_reach_the_kernel_s_descriptors() {
        let _g = setup();
        let p = thread();
        crate::kern::proc_glue::test_set_limits(p, 64);
        let fd = open(p);
        let mut retval = [0; 2];
        assert_eq!(sys_dup(p, &args(&[i64::from(fd)]), &mut retval), Ok(()));
        let dup = retval[0] as i32;
        assert_eq!(dup, fd + 1);
        let fp = fd_getfile(p.fd(), dup).unwrap();
        // The two slots and ours; file_count reads them without its own.
        assert_eq!(fp.count(), 3);
        assert_eq!(file_count(&fp), 3);
        drop(fp);
        assert_eq!(
            sys_fcntl(
                p,
                &args(&[i64::from(dup), i64::from(F_SETFD), i64::from(FD_CLOEXEC)]),
                &mut retval
            ),
            Ok(())
        );
        assert_eq!(
            sys_fcntl(p, &args(&[i64::from(dup), i64::from(F_GETFD)]), &mut retval),
            Ok(())
        );
        assert_eq!(retval[0], FD_CLOEXEC as Register);
        assert_eq!(sys_getdtablecount(p, &args(&[]), &mut retval), Ok(()));
        assert_eq!(retval[0], 2);
        assert_eq!(sys_close(p, &args(&[i64::from(dup)]), &mut retval), Ok(()));
        assert_eq!(sys_closefrom(p, &args(&[0]), &mut retval), Ok(()));
        assert_eq!(sys_getdtablecount(p, &args(&[]), &mut retval), Ok(()));
        assert_eq!(retval[0], 0);
        assert_eq!(
            sys_close(p, &args(&[i64::from(fd)]), &mut retval),
            Err(Errno::EBADF)
        );
    }

    /// An entry of `KernelApi::fd` reached with another call's number answers `ENOSYS`
    /// before it looks for a thread, and leaves the return registers alone.
    #[test]
    fn a_native_entry_refuses_another_number() {
        let enosys = Err(Errno::ENOSYS.as_i32());
        let mut rval: [Register; 2] = [-7, -7];
        let nr = crate::sys::syscall::SYS_dup as Register;
        assert_eq!(native_close(nr, 3, &mut rval), enosys);
        assert_eq!(native_read(nr, 3, 0, 0, &mut rval), enosys);
        assert_eq!(native_pwritev(nr, 3, 0, 0, 0, &mut rval), enosys);
        assert_eq!(native_getdtablecount(nr, &mut rval), enosys);
        assert_eq!(
            (FD_ENTRIES.dup)(SYS_close as Register, 3, &mut rval),
            enosys
        );
        assert_eq!(rval, [-7, -7]);
    }

    #[test]
    fn a_socket_file_s_object_is_its_socket_s_word() {
        use crate::kern::uipc_usrreq::tests::{close, setup as unix_setup, socketpair, teardown};
        let (_g, p) = unix_setup();
        let [a, b] = socketpair(p);
        let fp = fd_getfile(p.fd(), a).unwrap();
        let Some(BsdFileData::Socket(word)) = fp.data() else {
            panic!("a socket's object");
        };
        let so = crate::kern::sys_socket::socket_of(word).unwrap();
        assert!(core::ptr::eq(so, crate::kern::sys_socket::fp_socket(&fp)));
        assert!(!BsdFileGlue::dns_socket(fp.data().unwrap()));
        drop(fp);
        close(p, a);
        close(p, b);
        teardown();
    }
}
/* </TESTS> */
