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
//! The kernel's side of `KernelApi`, the vtable handed to the ABI module when `hal` loads
//! it (decision 22, docs/ZERO_UNSAFE.md 0.1): the module's registration, which the kernel
//! owns, and the table's construction. The type itself is `hal::api`'s, so the module
//! names it without depending on this crate.
//!
//! Who fills it: `bsd` builds the table today with [`table`] (its `main` owns the functions
//! that answer: the LZ system call path, `copyin`/`copyout`); an entry moves to a `kernel`
//! function when its subsystem does, and at N9 the kernel builds the table alone.

pub use hal::api::{
    AbiInit, AbiModuleHeader, ApiPrefix, KERNEL_API_VERSION, KernelApi, SyscallHandler,
};
use hal::module::once::Once;
pub use hal::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

/// The registered module: the table it was given and its handler.
struct Module {
    api: &'static KernelApi,
    handler: SyscallHandler,
}

/// The module's registration, once `abi_init` made it.
static MODULE: Once<Module> = Once::new();

/// The table of this version, with the kernel's registration and the functions the caller
/// owns.
pub const fn table(
    syscall_legacy: fn(Register, &SysArgs, &mut Rval) -> SysResult,
    copyin: fn(usize, &mut [u8]) -> Result<(), i32>,
    copyout: fn(&[u8], usize) -> Result<(), i32>,
) -> KernelApi {
    KernelApi::new(register_syscall_handler, syscall_legacy, copyin, copyout)
}

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
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn legacy(code: Register, _: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = code;
        Ok(())
    }

    fn copyin(_: usize, _: &mut [u8]) -> Result<(), i32> {
        Ok(())
    }

    fn copyout(_: &[u8], _: usize) -> Result<(), i32> {
        Ok(())
    }

    static API: KernelApi = table(legacy, copyin, copyout);

    #[test]
    fn the_table_is_this_version_with_the_kernels_registration() {
        assert_eq!(API.prefix, ApiPrefix::CURRENT);
        let register: fn(&'static KernelApi, SyscallHandler, HandlerInfo) =
            register_syscall_handler;
        assert!(core::ptr::fn_addr_eq(
            API.register_syscall_handler,
            register
        ));
        let mut rval = [0; 2];
        assert_eq!((API.syscall_legacy)(9, &[0; 6], &mut rval), Ok(()));
        assert_eq!(rval[0], 9);
    }
}
/* </TESTS> */
