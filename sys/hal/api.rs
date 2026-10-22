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

use core::mem::{offset_of, size_of};

use crate::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

/// The version of [`KernelApi`] this tree defines; the module's header must name it.
pub const KERNEL_API_VERSION: u32 = 4;

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
}

impl KernelApi {
    /// The table of this version over the given functions.
    pub const fn new(
        register_syscall_handler: fn(&'static KernelApi, SyscallHandler, HandlerInfo),
        syscall_legacy: fn(Register, &SysArgs, &mut Rval) -> SysResult,
        copyin: fn(usize, &mut [u8]) -> Result<(), i32>,
        copyout: fn(&[u8], usize) -> Result<(), i32>,
        pipe2: fn(Register, usize, Register) -> SysResult,
        vm: VmEntries,
        fd: FdEntries,
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
    // Version 4 only adds: the version 3 fields keep their places, `fd` comes last.
    assert!(offset_of!(KernelApi, fd) == size_of::<KernelApi>() - size_of::<FdEntries>());
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

    static API: KernelApi = KernelApi::new(register, legacy, copyin, copyout, pipe2, VM, FD);

    #[test]
    fn the_table_names_its_version_and_size() {
        assert_eq!(API.prefix, ApiPrefix::CURRENT);
        assert_eq!(API.prefix.version, KERNEL_API_VERSION);
        assert_eq!(KERNEL_API_VERSION, 4);
        assert_eq!(API.prefix.size as usize, size_of::<KernelApi>());
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
