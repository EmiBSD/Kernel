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
//! `KernelApi`: the vtable of safe functions the kernel hands the ABI module when `hal`
//! loads it (decision 22, docs/ZERO_UNSAFE.md 0.1): the only face of the kernel the module
//! sees. The module links no kernel code and no `hal` state: every call it makes into the
//! kernel goes through a field of this table, and the table reaches it as the argument of
//! its `abi_init` and of every system call.
//!
//! The table crosses a binary boundary (the kernel and `abi-openbsd.ko` are linked apart),
//! so it is `#[repr(C)]`, starts with its version and its size, and both sides are built by
//! the one pinned toolchain from the one tree: the fields are Rust-ABI `fn` pointers over
//! slices and `Result`s, whose layout is the compiler's and the same on both sides. The
//! loader refuses a module whose header names another version or another size.
//!
//! Who fills it: `bsd` builds the table today (its `main` owns the functions that answer:
//! the LZ system call path, `copyin`/`copyout`), with [`KernelApi::new`]; an entry moves to a
//! `kernel` function when its subsystem does, and at N9 the kernel builds the table alone.
//!
//! Version 1 (N2.4): the system call handler's registration, the `Legacy` fallback, and the
//! raw user copy the module's own `UserPtr` work needs. pipe(2) adds its entries at N2.6,
//! with version 2.

use core::mem::{offset_of, size_of};

use hal::module::once::Once;
pub use hal::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

/// The version of [`KernelApi`] this kernel hands out; the module's header must name it.
pub const KERNEL_API_VERSION: u32 = 1;

/// The module's system call handler: the table, the number, the arguments, the return
/// registers; `Ok(())` or an errno value, as the trap expects ([`SysResult`]).
pub type SyscallHandler = fn(&'static KernelApi, Register, &SysArgs, &mut Rval) -> SysResult;

/// The module's entry point, which `hal::module` calls once, before `init(8)` starts.
pub type AbiInit = fn(&'static KernelApi);

/// The vtable handed to the module.
#[repr(C)]
pub struct KernelApi {
    /// [`KERNEL_API_VERSION`].
    pub version: u32,
    /// `size_of::<KernelApi>()`, so a module built against another layout is refused.
    pub size: u32,
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
    /// The table of this version, with the registration this module provides and the
    /// functions the caller owns.
    pub const fn new(
        syscall_legacy: fn(Register, &SysArgs, &mut Rval) -> SysResult,
        copyin: fn(usize, &mut [u8]) -> Result<(), i32>,
        copyout: fn(&[u8], usize) -> Result<(), i32>,
    ) -> Self {
        Self {
            version: KERNEL_API_VERSION,
            size: size_of::<Self>() as u32,
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
    /// [`hal::module::MODULE_MAGIC`].
    pub magic: [u8; 8],
    /// The [`KERNEL_API_VERSION`] the module was built against.
    pub api_version: u32,
    /// The `size_of::<KernelApi>()` the module was built against.
    pub api_size: u32,
    /// The module's entry point.
    pub init: AbiInit,
}

impl AbiModuleHeader {
    /// The header of a module whose entry point is `init`, built against this table.
    pub const fn new(init: AbiInit) -> Self {
        Self {
            magic: hal::module::MODULE_MAGIC,
            api_version: KERNEL_API_VERSION,
            api_size: size_of::<KernelApi>() as u32,
            init,
        }
    }
}

/// The registered module: the table it was given and its handler.
struct Module {
    api: &'static KernelApi,
    handler: SyscallHandler,
}

/// The module's registration, once `abi_init` made it.
static MODULE: Once<Module> = Once::new();

/// `KernelApi::register_syscall_handler`: keeps the module's handler with the table it was
/// given and registers in `hal` the trampoline that calls it. Once: a second registration
/// is ignored, so the handler never changes under a running system call.
pub fn register_syscall_handler(
    api: &'static KernelApi,
    handler: SyscallHandler,
    info: HandlerInfo,
) {
    if MODULE.set(Module { api, handler }).is_ok() {
        let _ = hal::syscall::set_handler(module_syscall, info);
    }
}

/// What `hal`'s dispatch point calls once the module registered: the module's handler with
/// its table.
fn module_syscall(code: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
    match MODULE.get() {
        Some(m) => (m.handler)(m.api, code, args, rval),
        None => hal::syscall::legacy(code, args, rval),
    }
}

// The header's layout is what `hal::module` reads before it knows the type.
const _: () = {
    assert!(offset_of!(AbiModuleHeader, magic) == hal::module::HEADER_MAGIC);
    assert!(offset_of!(AbiModuleHeader, api_version) == hal::module::HEADER_API_VERSION);
    assert!(offset_of!(AbiModuleHeader, api_size) == hal::module::HEADER_API_SIZE);
    assert!(offset_of!(AbiModuleHeader, init) == hal::module::HEADER_INIT);
    assert!(size_of::<AbiModuleHeader>() == hal::module::HEADER_SIZE);
    assert!(offset_of!(KernelApi, version) == 0);
    assert!(offset_of!(KernelApi, size) == 4);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

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

    static API: KernelApi = KernelApi::new(legacy, copyin, copyout);

    #[test]
    fn the_table_names_its_version_and_size() {
        assert_eq!(API.version, KERNEL_API_VERSION);
        assert_eq!(API.size as usize, size_of::<KernelApi>());
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
        assert_eq!(h.magic, hal::module::MODULE_MAGIC);
        assert_eq!(h.api_version, KERNEL_API_VERSION);
        assert_eq!(h.api_size as usize, size_of::<KernelApi>());
    }
}
/* </TESTS> */
