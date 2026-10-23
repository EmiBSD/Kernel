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
//! The type of `KernelApi`, the vtable of safe functions the kernel hands the ABI module
//! when `hal::module` loads it (decision 22, docs/ZERO_UNSAFE.md 0.1), and the header the
//! module carries. They are declared here, below both sides, so that the module (`abi`)
//! names them with no dependency on the `kernel` crate's code, and the loader checks the
//! module against the very type it hands over. `kernel::api` fills the table and keeps the
//! module's registration; this file holds no function a table entry points at.
//!
//! The table crosses a binary boundary (the kernel and `abi_openbsd.ko` are linked apart),
//! so it is `#[repr(C)]` and starts with [`ApiPrefix`] (its version and size); both sides
//! are built by the one pinned toolchain from the one tree, and its fields are Rust-ABI
//! `fn` pointers over slices and `Result`s, laid out the same on both sides. The loader
//! refuses a table whose prefix is not this version's, and a module whose header names
//! another version or size.
//!
//! Version 1 (N2.4): the system call handler's registration, the `Legacy` fallback, and the
//! raw user copy the module's own `UserPtr` work needs. Version 2 (N2.6): `pipe2`, the
//! kernel's entry for pipe(2) and pipe2(2), the first `Native` system calls. Version 3
//! (N3.5): [`VmEntries`], the fifteen system calls of the virtual memory system (`mmap(2)`
//! and its siblings, `kbind(2)`, `pinsyscalls(2)`, `break`), one typed entry each. Version 4
//! (N4.3): [`FdEntries`], the twenty system calls of the descriptor tables and their I/O
//! (`read(2)`, `close(2)`, `dup(2)`, `fcntl(2)`, `ioctl(2)`, ...), one typed entry each.
//! Version 5 (N4.4): [`EventEntries`], the seven of kqueue and select/poll (`kqueue(2)`,
//! `kqueue1(2)`, `kevent(2)`, `select(2)`, `pselect(2)`, `poll(2)`, `ppoll(2)`). Version 6
//! (N4.7a): `sendsyslog`, `sendsyslog(2)`'s entry (`kernel::log`). Version 7 (N4.6c):
//! `futex`, the kernel's entry for futex(2) ([`FutexEntry`]). Version 8 (N4.6a):
//! [`ProcEntries`], the forty-six of the processes (fork, exit, wait, the limits, the ids
//! and credentials, accounting, the TCB, the thread names, the routing table).

use core::mem::{offset_of, size_of};

use crate::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

/// The version of [`KernelApi`] this tree defines; the module's header must name it.
pub const KERNEL_API_VERSION: u32 = 8;

/// The module's system call handler: the table, the number, the arguments, the return
/// registers; `Ok(())` or an errno value, as the trap expects ([`SysResult`]).
pub type SyscallHandler = fn(&'static KernelApi, Register, &SysArgs, &mut Rval) -> SysResult;

/// The module's entry point, which `hal::module` calls once, before `init(8)` starts.
pub type AbiInit = fn(&'static KernelApi);

/// What every table and header starts with, read before anything else.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPrefix {
    /// [`KERNEL_API_VERSION`].
    pub version: u32,
    /// `size_of::<KernelApi>()`.
    pub size: u32,
}

impl ApiPrefix {
    /// This tree's prefix.
    pub const CURRENT: Self = Self {
        version: KERNEL_API_VERSION,
        size: size_of::<KernelApi>() as u32,
    };
}

/// `mmap(2)` and `mquery(2)`: `(nr, addr, len, prot, flags, fd, pos)`, the address in the
/// `Ok`.
pub type MmapEntry = fn(Register, usize, usize, i32, i32, i32, i64) -> Result<usize, i32>;
/// A range call, `(nr, addr, len)`: `munmap`, `mimmutable`, `mlock`, `munlock`.
pub type RangeEntry = fn(Register, usize, usize) -> SysResult;
/// A range call with an `int`, `(nr, addr, len, arg)`: `mprotect` (`prot`), `madvise`
/// (`behav`), `minherit` (`inherit`), `msync` (`flags`).
pub type RangeIntEntry = fn(Register, usize, usize, i32) -> SysResult;

/// The virtual memory system's entries (version 3): each takes the system call's number
/// first (the prologue still `bsd`'s checks it: pledge, the pinned syscalls), then the
/// arguments as `<sys/syscallargs.h>` types them.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VmEntries {
    /// 17 = `break(char *nsize)`: `(nr, nsize)`.
    pub obreak: fn(Register, usize) -> SysResult,
    /// 49 = `mmap`.
    pub mmap: MmapEntry,
    /// 73 = `munmap`.
    pub munmap: RangeEntry,
    /// 74 = `mprotect`.
    pub mprotect: RangeIntEntry,
    /// 75 = `madvise`.
    pub madvise: RangeIntEntry,
    /// 78 = `mquery`.
    pub mquery: MmapEntry,
    /// 86 = `kbind(const struct __kbind *param, size_t psize, int64_t proc_cookie)`:
    /// `(nr, param, psize, proc_cookie)`.
    pub kbind: fn(Register, usize, usize, u64) -> SysResult,
    /// 158 = `pinsyscalls(void *base, size_t len, u_int *pins, int npins)`: `(nr, base, len,
    /// pins, npins)`.
    pub pinsyscalls: fn(Register, usize, usize, usize, i32) -> SysResult,
    /// 159 = `mimmutable`.
    pub mimmutable: RangeEntry,
    /// 203 = `mlock`.
    pub mlock: RangeEntry,
    /// 204 = `munlock`.
    pub munlock: RangeEntry,
    /// 250 = `minherit`.
    pub minherit: RangeIntEntry,
    /// 256 = `msync`.
    pub msync: RangeIntEntry,
    /// 271 = `mlockall(int flags)`: `(nr, flags)`.
    pub mlockall: fn(Register, i32) -> SysResult,
    /// 272 = `munlockall(void)`: `(nr)`.
    pub munlockall: fn(Register) -> SysResult,
}

/// A descriptor call, `(nr, fd, rval)`: `close`, `dup`, `closefrom`.
pub type FdEntry = fn(Register, i32, &mut Rval) -> SysResult;
/// A descriptor call with an `int`, `(nr, fd, arg, rval)`: `dup2` (`to`), `flock` (`how`),
/// `fpathconf` (`name`).
pub type FdIntEntry = fn(Register, i32, i32, &mut Rval) -> SysResult;
/// One buffer, `(nr, fd, buf, nbyte, rval)`: `read`, `write`.
pub type IoEntry = fn(Register, i32, usize, usize, &mut Rval) -> SysResult;
/// An iovec array, `(nr, fd, iovp, iovcnt, rval)`: `readv`, `writev`.
pub type IovEntry = fn(Register, i32, usize, i32, &mut Rval) -> SysResult;
/// One buffer at an offset, `(nr, fd, buf, nbyte, offset, rval)`: `pread`, `pwrite`.
pub type PioEntry = fn(Register, i32, usize, usize, i64, &mut Rval) -> SysResult;
/// An iovec array at an offset, `(nr, fd, iovp, iovcnt, offset, rval)`: `preadv`,
/// `pwritev`.
pub type PiovEntry = fn(Register, i32, usize, i32, i64, &mut Rval) -> SysResult;

/// The descriptor system calls' entries (version 4): each takes the system call's number
/// first (the prologue still `bsd`'s checks it: pledge, the pinned syscalls), then the
/// arguments as `<sys/syscallargs.h>` types them, and last the trap's return registers,
/// which the call writes as the C's `retval` (the count of `read(2)`, the new descriptor
/// of `dup(2)`, ...) and leaves alone where the C does.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FdEntries {
    /// 3 = `read`.
    pub read: IoEntry,
    /// 4 = `write`.
    pub write: IoEntry,
    /// 6 = `close`.
    pub close: FdEntry,
    /// 18 = `getdtablecount(void)`: `(nr, rval)`.
    pub getdtablecount: fn(Register, &mut Rval) -> SysResult,
    /// 41 = `dup`.
    pub dup: FdEntry,
    /// 53 = `fstat(int fd, struct stat *sb)`: `(nr, fd, sb, rval)`.
    pub fstat: fn(Register, i32, usize, &mut Rval) -> SysResult,
    /// 54 = `ioctl(int fd, u_long com, void *data)`: `(nr, fd, com, data, rval)`.
    pub ioctl: fn(Register, i32, u64, usize, &mut Rval) -> SysResult,
    /// 90 = `dup2`.
    pub dup2: FdIntEntry,
    /// 92 = `fcntl(int fd, int cmd, void *arg)`: `(nr, fd, cmd, arg, rval)`.
    pub fcntl: fn(Register, i32, i32, usize, &mut Rval) -> SysResult,
    /// 102 = `dup3(int from, int to, int flags)`: `(nr, from, to, flags, rval)`.
    pub dup3: fn(Register, i32, i32, i32, &mut Rval) -> SysResult,
    /// 120 = `readv`.
    pub readv: IovEntry,
    /// 121 = `writev`.
    pub writev: IovEntry,
    /// 131 = `flock`.
    pub flock: FdIntEntry,
    /// 169 = `pread`.
    pub pread: PioEntry,
    /// 170 = `pwrite`.
    pub pwrite: PioEntry,
    /// 171 = `preadv`.
    pub preadv: PiovEntry,
    /// 172 = `pwritev`.
    pub pwritev: PiovEntry,
    /// 192 = `fpathconf`.
    pub fpathconf: FdIntEntry,
    /// 209 = `utrace(const char *label, const void *addr, size_t len)`: `(nr, label, addr,
    /// len, rval)`.
    pub utrace: fn(Register, usize, usize, usize, &mut Rval) -> SysResult,
    /// 287 = `closefrom`.
    pub closefrom: FdEntry,
}

/// A user array of `kevent(2)` and its count: `(changelist, nchanges)`, `(eventlist,
/// nevents)`.
pub type KeventList = (usize, i32);
/// `kevent(2)`'s entry: `(nr, fd, changes, events, timeout, rval)`.
pub type KeventEntry = fn(Register, i32, KeventList, KeventList, usize, &mut Rval) -> SysResult;

/// The event system calls' entries (version 5): each takes the system call's number first
/// (the prologue still `bsd`'s checks it: pledge, the pinned syscalls), then the arguments
/// as `<sys/syscallargs.h>` types them, the three `fd_set` pointers of `select(2)` and
/// `pselect(2)` and the (list, count) pairs of `kevent(2)` grouped, and last the trap's
/// return registers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventEntries {
    /// 71 = `select(int nd, fd_set *in, fd_set *ou, fd_set *ex, struct timeval *tv)`:
    /// `(nr, nd, [in, ou, ex], tv, rval)`.
    pub select: fn(Register, i32, [usize; 3], usize, &mut Rval) -> SysResult,
    /// 72 = `kevent(int fd, const struct kevent *changelist, int nchanges, struct kevent
    /// *eventlist, int nevents, const struct timespec *timeout)`: `(nr, fd, (changelist,
    /// nchanges), (eventlist, nevents), timeout, rval)`.
    pub kevent: KeventEntry,
    /// 109 = `ppoll(struct pollfd *fds, u_int nfds, const struct timespec *ts, const
    /// sigset_t *mask)`: `(nr, fds, nfds, ts, mask, rval)`.
    pub ppoll: fn(Register, usize, u32, usize, usize, &mut Rval) -> SysResult,
    /// 110 = `pselect(int nd, fd_set *in, fd_set *ou, fd_set *ex, const struct timespec
    /// *ts, const sigset_t *mask)`: `(nr, nd, [in, ou, ex], ts, mask, rval)`.
    pub pselect: fn(Register, i32, [usize; 3], usize, usize, &mut Rval) -> SysResult,
    /// 252 = `poll(struct pollfd *fds, u_int nfds, int timeout)`: `(nr, fds, nfds, timeout,
    /// rval)`.
    pub poll: fn(Register, usize, u32, i32, &mut Rval) -> SysResult,
    /// 269 = `kqueue(void)`: `(nr, rval)`.
    pub kqueue: fn(Register, &mut Rval) -> SysResult,
    /// 270 = `kqueue1(int flags)`: `(nr, flags, rval)`.
    pub kqueue1: fn(Register, i32, &mut Rval) -> SysResult,
}

/// 83 = `futex(uint32_t *f, int op, int val, const struct timespec *timeout, uint32_t *g)`:
/// `(nr, f, op, val, timeout, g, rval)`, the addresses as integers (`timeout` is also
/// `FUTEX_REQUEUE`'s count), the number woken in `rval[0]`.
pub type FutexEntry = fn(Register, usize, i32, i32, usize, usize, &mut Rval) -> SysResult;

/// The process system calls' entries (version 8, N4.6a): `fork(2)` and `vfork(2)`,
/// `__tfork(2)`, `exit(2)` and `__threxit(2)`, `wait4(2)` and `waitid(2)`, the limits and
/// usage (`getrusage`, `getrlimit`, `setrlimit`, `getpriority`, `setpriority`), `acct(2)`, the
/// ids and credentials of kern_prot.c, the TCB, the thread names and the routing table: one
/// typed entry each, its arguments as the registers carry them, the return registers written
/// by the entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcEntries {
    /// `fork(void)`.
    pub fork: fn(Register, &mut Rval) -> SysResult,
    /// `vfork(void)`.
    pub vfork: fn(Register, &mut Rval) -> SysResult,
    /// `__tfork(const struct __tfork *param, size_t psize)`.
    pub tfork: fn(Register, usize, usize, &mut Rval) -> SysResult,
    /// `exit(int rval)`.
    pub exit: fn(Register, i32, &mut Rval) -> SysResult,
    /// `__threxit(pid_t *notdead)`.
    pub threxit: fn(Register, usize, &mut Rval) -> SysResult,
    /// `wait4(pid_t pid, int *status, int options, struct rusage *rusage)`.
    pub wait4: fn(Register, i32, usize, i32, usize, &mut Rval) -> SysResult,
    /// `waitid(int idtype, id_t id, siginfo_t *info, int options)`.
    pub waitid: fn(Register, i32, u32, usize, i32, &mut Rval) -> SysResult,
    /// `getrusage(int who, struct rusage *rusage)`.
    pub getrusage: fn(Register, i32, usize, &mut Rval) -> SysResult,
    /// `getrlimit(int which, struct rlimit *rlp)`.
    pub getrlimit: fn(Register, i32, usize, &mut Rval) -> SysResult,
    /// `setrlimit(int which, const struct rlimit *rlp)`.
    pub setrlimit: fn(Register, i32, usize, &mut Rval) -> SysResult,
    /// `getpriority(int which, id_t who)`.
    pub getpriority: fn(Register, i32, u32, &mut Rval) -> SysResult,
    /// `setpriority(int which, id_t who, int prio)`.
    pub setpriority: fn(Register, i32, u32, i32, &mut Rval) -> SysResult,
    /// `acct(const char *path)`.
    pub acct: fn(Register, usize, &mut Rval) -> SysResult,
    /// `getpid(void)`.
    pub getpid: fn(Register, &mut Rval) -> SysResult,
    /// `getppid(void)`.
    pub getppid: fn(Register, &mut Rval) -> SysResult,
    /// `getuid(void)`.
    pub getuid: fn(Register, &mut Rval) -> SysResult,
    /// `geteuid(void)`.
    pub geteuid: fn(Register, &mut Rval) -> SysResult,
    /// `getgid(void)`.
    pub getgid: fn(Register, &mut Rval) -> SysResult,
    /// `getegid(void)`.
    pub getegid: fn(Register, &mut Rval) -> SysResult,
    /// `getgroups(int gidsetsize, gid_t *gidset)`.
    pub getgroups: fn(Register, i32, usize, &mut Rval) -> SysResult,
    /// `setgroups(int gidsetsize, const gid_t *gidset)`.
    pub setgroups: fn(Register, i32, usize, &mut Rval) -> SysResult,
    /// `getpgrp(void)`.
    pub getpgrp: fn(Register, &mut Rval) -> SysResult,
    /// `getpgid(pid_t pid)`.
    pub getpgid: fn(Register, i32, &mut Rval) -> SysResult,
    /// `getsid(pid_t pid)`.
    pub getsid: fn(Register, i32, &mut Rval) -> SysResult,
    /// `setpgid(pid_t pid, pid_t pgid)`.
    pub setpgid: fn(Register, i32, i32, &mut Rval) -> SysResult,
    /// `setsid(void)`.
    pub setsid: fn(Register, &mut Rval) -> SysResult,
    /// `setuid(uid_t uid)`.
    pub setuid: fn(Register, u32, &mut Rval) -> SysResult,
    /// `seteuid(uid_t euid)`.
    pub seteuid: fn(Register, u32, &mut Rval) -> SysResult,
    /// `setgid(gid_t gid)`.
    pub setgid: fn(Register, u32, &mut Rval) -> SysResult,
    /// `setegid(gid_t egid)`.
    pub setegid: fn(Register, u32, &mut Rval) -> SysResult,
    /// `setreuid(uid_t ruid, uid_t euid)`.
    pub setreuid: fn(Register, u32, u32, &mut Rval) -> SysResult,
    /// `setregid(gid_t rgid, gid_t egid)`.
    pub setregid: fn(Register, u32, u32, &mut Rval) -> SysResult,
    /// `setresuid(uid_t ruid, uid_t euid, uid_t suid)`.
    pub setresuid: fn(Register, u32, u32, u32, &mut Rval) -> SysResult,
    /// `setresgid(gid_t rgid, gid_t egid, gid_t sgid)`.
    pub setresgid: fn(Register, u32, u32, u32, &mut Rval) -> SysResult,
    /// `getresuid(uid_t *ruid, uid_t *euid, uid_t *suid)`.
    pub getresuid: fn(Register, usize, usize, usize, &mut Rval) -> SysResult,
    /// `getresgid(gid_t *rgid, gid_t *egid, gid_t *sgid)`.
    pub getresgid: fn(Register, usize, usize, usize, &mut Rval) -> SysResult,
    /// `issetugid(void)`.
    pub issetugid: fn(Register, &mut Rval) -> SysResult,
    /// `getlogin_r(char *namebuf, size_t namelen)`.
    pub getlogin_r: fn(Register, usize, usize, &mut Rval) -> SysResult,
    /// `setlogin(const char *namebuf)`.
    pub setlogin: fn(Register, usize, &mut Rval) -> SysResult,
    /// `__set_tcb(void *tcb)`.
    pub set_tcb: fn(Register, usize, &mut Rval) -> SysResult,
    /// `__get_tcb(void)`.
    pub get_tcb: fn(Register, &mut Rval) -> SysResult,
    /// `getthrid(void)`.
    pub getthrid: fn(Register, &mut Rval) -> SysResult,
    /// `getthrname(pid_t tid, char *name, size_t len)`.
    pub getthrname: fn(Register, i32, usize, usize, &mut Rval) -> SysResult,
    /// `setthrname(pid_t tid, const char *name)`.
    pub setthrname: fn(Register, i32, usize, &mut Rval) -> SysResult,
    /// `getrtable(void)`.
    pub getrtable: fn(Register, &mut Rval) -> SysResult,
    /// `setrtable(int rtableid)`.
    pub setrtable: fn(Register, i32, &mut Rval) -> SysResult,
}

/// The vtable handed to the module.
#[repr(C)]
pub struct KernelApi {
    /// [`ApiPrefix::CURRENT`].
    pub prefix: ApiPrefix,
    /// Registers the module's system call handler and what it says about itself: from
    /// here on every system call trap reaches it.
    pub register_syscall_handler: fn(&'static KernelApi, SyscallHandler, HandlerInfo),
    /// The `Legacy` path: what a `Legacy` entry of the module's table calls (the LZ
    /// `sysent[]` and `mi_syscall`, which `bsd` registers).
    pub syscall_legacy: fn(Register, &SysArgs, &mut Rval) -> SysResult,
    /// `copyin(uaddr, kbuf)`: user bytes at `uaddr` into `kbuf`; an errno value on a fault.
    pub copyin: fn(usize, &mut [u8]) -> Result<(), i32>,
    /// `copyout(kbuf, uaddr)`: `kbuf` to user memory at `uaddr`; an errno value on a fault.
    pub copyout: fn(&[u8], usize) -> Result<(), i32>,
    /// pipe(2) and pipe2(2): `(nr, fdp, flags)`, the system call's number (`SYS_pipe` or
    /// `SYS_pipe2`, which the system call prologue checks: pledge, the pinned syscalls), the
    /// user address of the two descriptors and `pipe2`'s flags (0 for `pipe`).
    pub pipe2: fn(Register, usize, Register) -> SysResult,
    /// The virtual memory system's system calls (version 3).
    pub vm: VmEntries,
    /// The descriptor system calls (version 4).
    pub fd: FdEntries,
    /// The event system calls (version 5).
    pub event: EventEntries,
    /// 112 = `sendsyslog(const char *buf, size_t nbyte, int flags)` (version 6): `(nr, buf,
    /// nbyte, flags, rval)`.
    pub sendsyslog: fn(Register, usize, usize, i32, &mut Rval) -> SysResult,
    /// futex(2) (version 7).
    pub futex: FutexEntry,
    /// The process system calls (version 8).
    pub proc: ProcEntries,
}

impl KernelApi {
    /// The table of this version over the given functions.
    #[allow(
        clippy::too_many_arguments,
        reason = "one argument per member of the table, as its layout"
    )]
    pub const fn new(
        register_syscall_handler: fn(&'static KernelApi, SyscallHandler, HandlerInfo),
        syscall_legacy: fn(Register, &SysArgs, &mut Rval) -> SysResult,
        copyin: fn(usize, &mut [u8]) -> Result<(), i32>,
        copyout: fn(&[u8], usize) -> Result<(), i32>,
        pipe2: fn(Register, usize, Register) -> SysResult,
        vm: VmEntries,
        fd: FdEntries,
        event: EventEntries,
        sendsyslog: fn(Register, usize, usize, i32, &mut Rval) -> SysResult,
        futex: FutexEntry,
        proc: ProcEntries,
    ) -> Self {
        Self {
            prefix: ApiPrefix::CURRENT,
            register_syscall_handler,
            syscall_legacy,
            copyin,
            copyout,
            pipe2,
            vm,
            fd,
            event,
            sendsyslog,
            futex,
            proc,
        }
    }
}

/// The header the module carries at its ELF entry address (its linker script puts it
/// there), which `hal::module` checks before it calls `init`.
#[repr(C)]
pub struct AbiModuleHeader {
    /// [`MODULE_MAGIC`].
    pub magic: [u8; 8],
    /// The [`ApiPrefix`] of the table the module was built against.
    pub api: ApiPrefix,
    /// The module's entry point.
    pub init: AbiInit,
}

impl AbiModuleHeader {
    /// The header of a module whose entry point is `init`, built against this table.
    pub const fn new(init: AbiInit) -> Self {
        Self {
            magic: MODULE_MAGIC,
            api: ApiPrefix::CURRENT,
            init,
        }
    }
}

/// The first eight bytes of the module's header.
pub const MODULE_MAGIC: [u8; 8] = *b"EMIABIKO";

/// Offset of the header's magic.
pub const HEADER_MAGIC: usize = offset_of!(AbiModuleHeader, magic);
/// Offset of the version of the table the module was built against (`u32`).
pub const HEADER_API_VERSION: usize = offset_of!(AbiModuleHeader, api);
/// Offset of the size of that table (`u32`).
pub const HEADER_API_SIZE: usize = offset_of!(AbiModuleHeader, api) + 4;
/// Offset of the entry point (a pointer, relocated).
pub const HEADER_INIT: usize = offset_of!(AbiModuleHeader, init);
/// The header's size.
pub const HEADER_SIZE: usize = size_of::<AbiModuleHeader>();

// The layout the loader reads as bytes before it calls anything.
const _: () = {
    assert!(offset_of!(ApiPrefix, version) == 0);
    assert!(offset_of!(ApiPrefix, size) == 4);
    assert!(offset_of!(KernelApi, prefix) == 0);
    // Version 8 only adds: the version 7 fields keep their places, `proc` comes last.
    assert!(offset_of!(KernelApi, proc) == size_of::<KernelApi>() - size_of::<ProcEntries>());
    assert!(offset_of!(KernelApi, proc) == offset_of!(KernelApi, futex) + size_of::<FutexEntry>());
    assert!(size_of::<ProcEntries>() == 46 * size_of::<fn()>());
    // Version 7 added `futex` after `sendsyslog`.
    assert!(offset_of!(KernelApi, futex) == offset_of!(KernelApi, sendsyslog) + size_of::<fn()>());
    // Version 6 added `sendsyslog` after the version 5 fields.
    assert!(
        offset_of!(KernelApi, sendsyslog)
            == offset_of!(KernelApi, event) + size_of::<EventEntries>()
    );
    // Version 5 added: the version 4 fields keep their places, `event` follows them.
    assert!(offset_of!(KernelApi, event) == offset_of!(KernelApi, fd) + size_of::<FdEntries>());
    assert!(size_of::<EventEntries>() == 7 * size_of::<fn()>());
    assert!(offset_of!(KernelApi, fd) == offset_of!(KernelApi, vm) + size_of::<VmEntries>());
    assert!(size_of::<FdEntries>() == 20 * size_of::<fn()>());
    assert!(size_of::<VmEntries>() == 15 * size_of::<fn()>());
    // Version 2's fields, the prefix and five entries, keep their offsets: `vm` follows them.
    assert!(offset_of!(KernelApi, vm) == size_of::<ApiPrefix>() + 5 * size_of::<fn()>());
    assert!(HEADER_MAGIC == 0 && HEADER_API_VERSION == 8 && HEADER_INIT == 16);
    assert!(HEADER_SIZE == 24);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn register(_: &'static KernelApi, _: SyscallHandler, _: HandlerInfo) {}

    fn legacy(code: Register, _: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = code;
        Ok(())
    }

    fn copyin(_: usize, buf: &mut [u8]) -> Result<(), i32> {
        buf.fill(1);
        Ok(())
    }

    fn copyout(_: &[u8], _: usize) -> Result<(), i32> {
        Err(14) // EFAULT
    }

    fn pipe2(nr: Register, fdp: usize, flags: Register) -> SysResult {
        if nr as usize + fdp + flags as usize == 0 {
            Ok(())
        } else {
            Err(22) // EINVAL
        }
    }

    fn nr(nr: Register) -> SysResult {
        Err(nr as i32)
    }
    fn nr_int(nr: Register, v: i32) -> SysResult {
        Err(nr as i32 + v)
    }
    fn obreak(nr: Register, n: usize) -> SysResult {
        Err(nr as i32 + n as i32)
    }
    fn range(nr: Register, a: usize, l: usize) -> SysResult {
        Err(nr as i32 + (a + l) as i32)
    }
    fn range_int(nr: Register, a: usize, l: usize, v: i32) -> SysResult {
        Err(nr as i32 + (a + l) as i32 + v)
    }
    fn mmap(
        nr: Register,
        a: usize,
        l: usize,
        p: i32,
        f: i32,
        fd: i32,
        o: i64,
    ) -> Result<usize, i32> {
        Ok(nr as usize + a + l + (p + f + fd) as usize + o as usize)
    }
    fn kbind(nr: Register, p: usize, s: usize, c: u64) -> SysResult {
        Err(nr as i32 + (p + s) as i32 + c as i32)
    }
    fn pins(nr: Register, b: usize, l: usize, p: usize, n: i32) -> SysResult {
        Err(nr as i32 + (b + l + p) as i32 + n)
    }

    const VM: VmEntries = VmEntries {
        obreak,
        mmap,
        munmap: range,
        mprotect: range_int,
        madvise: range_int,
        mquery: mmap,
        kbind,
        pinsyscalls: pins,
        mimmutable: range,
        mlock: range,
        munlock: range,
        minherit: range_int,
        msync: range_int,
        mlockall: nr_int,
        munlockall: nr,
    };

    // The fd entries of the test table: the number plus the arguments in the error, the
    // number in rval[0].
    fn fd1(nr: Register, fd: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd)
    }
    fn fd2(nr: Register, fd: i32, v: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + v)
    }
    fn io(nr: Register, fd: i32, b: usize, n: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + (b + n) as i32)
    }
    fn iov(nr: Register, fd: i32, b: usize, n: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + b as i32 + n)
    }
    fn pio(nr: Register, fd: i32, b: usize, n: usize, o: i64, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + (b + n) as i32 + o as i32)
    }
    fn piov(nr: Register, fd: i32, b: usize, n: i32, o: i64, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + b as i32 + n + o as i32)
    }
    fn count(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32)
    }
    fn fstat(nr: Register, fd: i32, sb: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + sb as i32)
    }
    fn ioctl(nr: Register, fd: i32, c: u64, d: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + c as i32 + d as i32)
    }
    fn fcntl(nr: Register, fd: i32, c: i32, a: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + c + a as i32)
    }
    fn dup3(nr: Register, f: i32, t: i32, fl: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + f + t + fl)
    }
    fn utrace(nr: Register, l: usize, a: usize, n: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + (l + a + n) as i32)
    }

    const FD: FdEntries = FdEntries {
        read: io,
        write: io,
        close: fd1,
        getdtablecount: count,
        dup: fd1,
        fstat,
        ioctl,
        dup2: fd2,
        fcntl,
        dup3,
        readv: iov,
        writev: iov,
        flock: fd2,
        pread: pio,
        pwrite: pio,
        preadv: piov,
        pwritev: piov,
        fpathconf: fd2,
        utrace,
        closefrom: fd1,
    };

    fn select(nr: Register, nd: i32, s: [usize; 3], tv: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + nd + (s[0] + s[1] + s[2] + tv) as i32)
    }
    fn kevent(
        nr: Register,
        fd: i32,
        c: (usize, i32),
        e: (usize, i32),
        t: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + fd + c.0 as i32 + c.1 + e.0 as i32 + e.1 + t as i32)
    }
    fn ppoll(nr: Register, f: usize, n: u32, t: usize, m: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + (f + n as usize + t + m) as i32)
    }
    fn pselect(
        nr: Register,
        nd: i32,
        s: [usize; 3],
        t: usize,
        m: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + nd + (s[0] + s[1] + s[2] + t + m) as i32)
    }
    fn poll(nr: Register, f: usize, n: u32, t: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + f as i32 + n as i32 + t)
    }

    fn sendsyslog(nr: Register, b: usize, n: usize, f: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(b as i32 * 100 + n as i32 * 10 + f)
    }

    const EVENT: EventEntries = EventEntries {
        select,
        kevent,
        ppoll,
        pselect,
        poll,
        kqueue: count,
        kqueue1: fd1,
    };

    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_fork(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_vfork(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_tfork(nr: Register, param: usize, psize: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((param as i64) * 10 + (psize as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_exit(nr: Register, status: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((status as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_threxit(nr: Register, notdead: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((notdead as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_wait4(
        nr: Register,
        pid: i32,
        status: usize,
        options: i32,
        rusage: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1000
            + (status as i64) * 100
            + (options as i64) * 10
            + (rusage as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_waitid(
        nr: Register,
        idtype: i32,
        id: u32,
        info: usize,
        options: i32,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(
            ((idtype as i64) * 1000 + (id as i64) * 100 + (info as i64) * 10 + (options as i64) * 1)
                as i32,
        )
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrusage(nr: Register, who: i32, rusage: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((who as i64) * 10 + (rusage as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrlimit(nr: Register, which: i32, rlp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (rlp as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setrlimit(nr: Register, which: i32, rlp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (rlp as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpriority(nr: Register, which: i32, who: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (who as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setpriority(nr: Register, which: i32, who: u32, prio: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 100 + (who as i64) * 10 + (prio as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_acct(nr: Register, path: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((path as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getppid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getuid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_geteuid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getgid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getegid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getgroups(nr: Register, gidsetsize: i32, gidset: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gidsetsize as i64) * 10 + (gidset as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setgroups(nr: Register, gidsetsize: i32, gidset: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gidsetsize as i64) * 10 + (gidset as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpgrp(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpgid(nr: Register, pid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getsid(nr: Register, pid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setpgid(nr: Register, pid: i32, pgid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 10 + (pgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setsid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setuid(nr: Register, uid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((uid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_seteuid(nr: Register, euid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((euid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setgid(nr: Register, gid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setegid(nr: Register, egid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((egid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setreuid(nr: Register, ruid: u32, euid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 10 + (euid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setregid(nr: Register, rgid: u32, egid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 10 + (egid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setresuid(nr: Register, ruid: u32, euid: u32, suid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 100 + (euid as i64) * 10 + (suid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setresgid(nr: Register, rgid: u32, egid: u32, sgid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 100 + (egid as i64) * 10 + (sgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getresuid(
        nr: Register,
        ruid: usize,
        euid: usize,
        suid: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 100 + (euid as i64) * 10 + (suid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getresgid(
        nr: Register,
        rgid: usize,
        egid: usize,
        sgid: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 100 + (egid as i64) * 10 + (sgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_issetugid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getlogin_r(nr: Register, namebuf: usize, namelen: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((namebuf as i64) * 10 + (namelen as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setlogin(nr: Register, namebuf: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((namebuf as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_set_tcb(nr: Register, tcb: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tcb as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_get_tcb(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getthrid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getthrname(nr: Register, tid: i32, name: usize, len: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tid as i64) * 100 + (name as i64) * 10 + (len as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setthrname(nr: Register, tid: i32, name: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tid as i64) * 10 + (name as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrtable(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setrtable(nr: Register, rtableid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rtableid as i64) * 1) as i32)
    }

    const PROC: ProcEntries = ProcEntries {
        fork: p_fork,
        vfork: p_vfork,
        tfork: p_tfork,
        exit: p_exit,
        threxit: p_threxit,
        wait4: p_wait4,
        waitid: p_waitid,
        getrusage: p_getrusage,
        getrlimit: p_getrlimit,
        setrlimit: p_setrlimit,
        getpriority: p_getpriority,
        setpriority: p_setpriority,
        acct: p_acct,
        getpid: p_getpid,
        getppid: p_getppid,
        getuid: p_getuid,
        geteuid: p_geteuid,
        getgid: p_getgid,
        getegid: p_getegid,
        getgroups: p_getgroups,
        setgroups: p_setgroups,
        getpgrp: p_getpgrp,
        getpgid: p_getpgid,
        getsid: p_getsid,
        setpgid: p_setpgid,
        setsid: p_setsid,
        setuid: p_setuid,
        seteuid: p_seteuid,
        setgid: p_setgid,
        setegid: p_setegid,
        setreuid: p_setreuid,
        setregid: p_setregid,
        setresuid: p_setresuid,
        setresgid: p_setresgid,
        getresuid: p_getresuid,
        getresgid: p_getresgid,
        issetugid: p_issetugid,
        getlogin_r: p_getlogin_r,
        setlogin: p_setlogin,
        set_tcb: p_set_tcb,
        get_tcb: p_get_tcb,
        getthrid: p_getthrid,
        getthrname: p_getthrname,
        setthrname: p_setthrname,
        getrtable: p_getrtable,
        setrtable: p_setrtable,
    };

    fn futex(
        nr: Register,
        f: usize,
        op: i32,
        val: i32,
        t: usize,
        g: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(nr as i32 + f as i32 + op + val + t as i32 + g as i32)
    }

    static API: KernelApi = KernelApi::new(
        register, legacy, copyin, copyout, pipe2, VM, FD, EVENT, sendsyslog, futex, PROC,
    );

    #[test]
    fn the_table_names_its_version_and_size() {
        assert_eq!(API.prefix, ApiPrefix::CURRENT);
        assert_eq!(API.prefix.version, KERNEL_API_VERSION);
        assert_eq!(KERNEL_API_VERSION, 8);
        assert_eq!(API.prefix.size as usize, size_of::<KernelApi>());
    }

    #[test]
    fn the_sendsyslog_entry_is_the_callers() {
        let mut rval = [0; 2];
        assert_eq!((API.sendsyslog)(112, 1, 2, 3, &mut rval), Err(123));
        assert_eq!(rval[0], 112);
    }

    #[test]
    fn the_vm_entries_are_the_callers() {
        assert_eq!((API.vm.mmap)(49, 1, 2, 3, 4, 5, 6), Ok(70));
        assert_eq!((API.vm.mquery)(78, 1, 2, 3, 4, 5, 6), Ok(99));
        assert_eq!((API.vm.munmap)(73, 1, 2), Err(76));
        assert_eq!((API.vm.msync)(256, 1, 2, 3), Err(262));
        assert_eq!((API.vm.kbind)(86, 1, 2, 3), Err(92));
        assert_eq!((API.vm.pinsyscalls)(158, 1, 2, 3, 4), Err(168));
        assert_eq!((API.vm.obreak)(17, 1), Err(18));
        assert_eq!((API.vm.mlockall)(271, 1), Err(272));
        assert_eq!((API.vm.munlockall)(272), Err(272));
    }

    #[test]
    fn the_fd_entries_are_the_callers_and_write_the_return_registers() {
        let mut rval = [0; 2];
        assert_eq!((API.fd.read)(3, 1, 2, 3, &mut rval), Err(9));
        assert_eq!(rval, [3, 0]);
        assert_eq!((API.fd.pwritev)(172, 1, 2, 3, 4, &mut rval), Err(182));
        assert_eq!((API.fd.getdtablecount)(18, &mut rval), Err(18));
        assert_eq!((API.fd.ioctl)(54, 1, 2, 3, &mut rval), Err(60));
        assert_eq!((API.fd.closefrom)(287, 1, &mut rval), Err(288));
        assert_eq!(rval, [287, 0]);
    }

    #[test]
    fn the_event_entries_are_the_callers_and_write_the_return_registers() {
        let mut rval = [0; 2];
        assert_eq!((API.event.select)(71, 1, [2, 3, 4], 5, &mut rval), Err(86));
        assert_eq!(rval, [71, 0]);
        assert_eq!(
            (API.event.kevent)(72, 1, (2, 3), (4, 5), 6, &mut rval),
            Err(93)
        );
        assert_eq!((API.event.ppoll)(109, 1, 2, 3, 4, &mut rval), Err(119));
        assert_eq!(
            (API.event.pselect)(110, 1, [2, 3, 4], 5, 6, &mut rval),
            Err(131)
        );
        assert_eq!((API.event.poll)(252, 1, 2, 3, &mut rval), Err(258));
        assert_eq!((API.event.kqueue)(269, &mut rval), Err(269));
        assert_eq!((API.event.kqueue1)(270, 1, &mut rval), Err(271));
        assert_eq!(rval, [270, 0]);
    }

    #[test]
    fn the_futex_entry_is_the_caller_and_writes_the_return_registers() {
        let mut rval = [0; 2];
        assert_eq!((API.futex)(83, 1, 2, 3, 4, 5, &mut rval), Err(98));
        assert_eq!(rval, [83, 0]);
    }

    #[test]
    fn the_entries_are_the_callers() {
        let mut rval = [0; 2];
        assert_eq!((API.syscall_legacy)(9, &[0; 6], &mut rval), Ok(()));
        assert_eq!(rval[0], 9);
        let mut buf = [0u8; 3];
        assert_eq!((API.copyin)(0x1000, &mut buf), Ok(()));
        assert_eq!(buf, [1, 1, 1]);
        assert_eq!((API.copyout)(&buf, 0x1000), Err(14));
        assert_eq!((API.pipe2)(0, 0, 0), Ok(()));
        assert_eq!((API.pipe2)(101, 0x1000, 0), Err(22));
    }

    fn init(_: &'static KernelApi) {}

    #[test]
    fn the_header_names_this_table() {
        let h = AbiModuleHeader::new(init);
        assert_eq!(h.magic, MODULE_MAGIC);
        assert_eq!(h.api, ApiPrefix::CURRENT);
    }
}
/* </TESTS> */
