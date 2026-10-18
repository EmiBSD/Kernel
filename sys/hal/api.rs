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
//! and its siblings, `kbind(2)`, `pinsyscalls(2)`, `break`), one typed entry each.

use core::mem::{offset_of, size_of};

use crate::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

/// The version of [`KernelApi`] this tree defines; the module's header must name it.
pub const KERNEL_API_VERSION: u32 = 3;

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
    ) -> Self {
        Self {
            prefix: ApiPrefix::CURRENT,
            register_syscall_handler,
            syscall_legacy,
            copyin,
            copyout,
            pipe2,
            vm,
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
    // Version 3 only adds: the version 2 fields keep their places, `vm` comes last.
    assert!(offset_of!(KernelApi, vm) == size_of::<KernelApi>() - size_of::<VmEntries>());
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

    static API: KernelApi = KernelApi::new(register, legacy, copyin, copyout, pipe2, VM);

    #[test]
    fn the_table_names_its_version_and_size() {
        assert_eq!(API.prefix, ApiPrefix::CURRENT);
        assert_eq!(API.prefix.version, KERNEL_API_VERSION);
        assert_eq!(KERNEL_API_VERSION, 3);
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
