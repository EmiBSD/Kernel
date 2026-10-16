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
//! The table crosses a binary boundary (the kernel and `abi-openbsd.ko` are linked apart),
//! so it is `#[repr(C)]` and starts with [`ApiPrefix`] (its version and size); both sides
//! are built by the one pinned toolchain from the one tree, and its fields are Rust-ABI
//! `fn` pointers over slices and `Result`s, laid out the same on both sides. The loader
//! refuses a table whose prefix is not this version's, and a module whose header names
//! another version or size.
//!
//! Version 1 (N2.4): the system call handler's registration, the `Legacy` fallback, and the
//! raw user copy the module's own `UserPtr` work needs. pipe(2) adds its entries at N2.6,
//! with version 2.

use core::mem::{offset_of, size_of};

use crate::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

/// The version of [`KernelApi`] this tree defines; the module's header must name it.
pub const KERNEL_API_VERSION: u32 = 1;

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
}

impl KernelApi {
    /// The table of this version over the given functions.
    pub const fn new(
        register_syscall_handler: fn(&'static KernelApi, SyscallHandler, HandlerInfo),
        syscall_legacy: fn(Register, &SysArgs, &mut Rval) -> SysResult,
        copyin: fn(usize, &mut [u8]) -> Result<(), i32>,
        copyout: fn(&[u8], usize) -> Result<(), i32>,
    ) -> Self {
        Self {
            prefix: ApiPrefix::CURRENT,
            register_syscall_handler,
            syscall_legacy,
            copyin,
            copyout,
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

    static API: KernelApi = KernelApi::new(register, legacy, copyin, copyout);

    #[test]
    fn the_table_names_its_version_and_size() {
        assert_eq!(API.prefix, ApiPrefix::CURRENT);
        assert_eq!(API.prefix.version, KERNEL_API_VERSION);
        assert_eq!(API.prefix.size as usize, size_of::<KernelApi>());
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
