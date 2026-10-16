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
//! `bsd`'s side of the ABI module `abi-openbsd.ko` (decision 22) until the subsystems it
//! stands for move: the `Legacy` system call path (LZ's `sysent[]` and `mi_syscall`, what
//! both arch traps called themselves), the `KernelApi` table `bsd` fills, the platform the
//! module loader maps the module through (the kernel's pmap and the boot hand-over), and
//! the boot steps that register the fallback and load the module before `init(8)` runs.
//!
//! The arch traps call [`syscall`], which goes through `hal`'s dispatch point: to the
//! module's handler once it registered (every entry `Legacy` at N2.4, so back here), to
//! [`legacy_syscall`] otherwise. Either way a system call reaches `mi_syscall` with the same
//! thread, number, arguments and return registers as before, and comes back with the same
//! `Result`; only the route changed.
//!
//! No `unsafe`: every call here is a safe function of `bsd`, `hal` or `kernel`. Deleted at
//! N9 with `bsd`.

use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use hal::module::{ModulePlatform, Prot};
use kernel::api::KernelApi;

use crate::kern::init_sysent::SYSENT;
use crate::kern::subr_prf::panic;
use crate::machine::copy::{copyin, copyout};
use crate::machine::cpu::curproc;
use crate::machine::pmap::{pmap_enter, pmap_extract, pmap_kernel, pmap_update};
use crate::sys::errno::Errno;
use crate::sys::mman::{PROT_EXEC, PROT_READ, PROT_WRITE};
use crate::sys::syscall::SYS_MAXSYSCALL;
use crate::sys::syscall_mi::mi_syscall;
use crate::sys::systm::SysArgs;
use crate::sys::types::{Register, Vaddr};
use crate::uvm::uvm_km::{KD_WAITOK, KP_NONE, KP_ZERO, KV_ANY, km_alloc, km_free};
use crate::uvm::uvm_param::round_page;
use crate::uvm::uvm_pmap::PMAP_WIRED;

/// The `KernelApi` `bsd` hands the module: the `Legacy` path and the user copy are `bsd`'s;
/// pipe(2) is the kernel crate's, entered through `bsd`'s system call prologue.
static KERNEL_API: KernelApi = kernel::api::table(
    legacy_syscall,
    api_copyin,
    api_copyout,
    crate::kern::pipe_glue::native_pipe2,
);

/// The module file the boot loader handed over (`set_image`): its kernel virtual address.
static IMAGE_VA: AtomicUsize = AtomicUsize::new(0);
/// Its length; 0 while there is none.
static IMAGE_LEN: AtomicUsize = AtomicUsize::new(0);

/// The loader's platform over the kernel's pmap (`hal::module::ModulePlatform`).
struct BsdPlatform;

impl ModulePlatform for BsdPlatform {
    fn image(&self) -> Option<(usize, usize)> {
        let len = IMAGE_LEN.load(Ordering::Acquire);
        (len != 0).then(|| (IMAGE_VA.load(Ordering::Relaxed), len))
    }

    fn alloc(&self, len: usize) -> Option<usize> {
        km_alloc(round_page(len), &KV_ANY, &KP_ZERO, &KD_WAITOK)
            .map(|p: NonNull<u8>| p.as_ptr().expose_provenance())
    }

    fn reserve(&self, len: usize) -> Option<usize> {
        km_alloc(round_page(len), &KV_ANY, &KP_NONE, &KD_WAITOK).map(|p| p.as_ptr().addr())
    }

    fn enter(&self, va: usize, rw: usize, prot: Prot) -> bool {
        let Some(pa) = pmap_extract(pmap_kernel(), Vaddr::new(rw)) else {
            return false;
        };
        let prot = match prot {
            Prot::Read => PROT_READ,
            Prot::ReadWrite => PROT_READ | PROT_WRITE,
            Prot::ReadExec => PROT_READ | PROT_EXEC,
        };
        let entered = pmap_enter(pmap_kernel(), Vaddr::new(va), pa, prot, prot | PMAP_WIRED);
        pmap_update(pmap_kernel());
        entered.is_ok()
    }

    fn release(&self, rw: usize, len: usize) {
        // `kp_none`: km_free gives back the virtual range only (uvm_unmap removes its
        // mappings); the pages stay, held by the module's mappings, never freed.
        if let Some(v) = NonNull::new(core::ptr::with_exposed_provenance_mut::<u8>(rw)) {
            km_free(v, round_page(len), &KV_ANY, &KP_NONE);
        }
    }
}

/// The system call path both arch traps call where they chose the `sysent[]` entry:
/// `hal`'s dispatch point, its errno value back as an [`Errno`].
pub fn syscall(code: Register, args: &SysArgs, rval: &mut [Register; 2]) -> Result<(), Errno> {
    hal::syscall::dispatch(code, args, rval).map_err(errno_of)
}

/// The `Legacy` fallback: what the traps did before (`ENOSYS` for a number outside
/// `sysent[]`, else `mi_syscall` on the entry), for the thread on this CPU.
pub fn legacy_syscall(code: Register, args: &SysArgs, rval: &mut [Register; 2]) -> Result<(), i32> {
    let Some(p) = curproc() else {
        panic(format_args!("syscall: no curproc"));
    };
    let error = if code <= 0 || code as usize >= SYS_MAXSYSCALL {
        Err(Errno::ENOSYS)
    } else {
        mi_syscall(p, code, &SYSENT[code as usize], args, rval)
    };
    error.map_err(Errno::as_i32)
}

/// The first boot step, from the boot glue before `main`: registers the `Legacy` fallback
/// (so a system call is answered from the first one on, as before) and records the module
/// file the boot loader handed over, if any: its kernel virtual address and length
/// (Limine's module, or boot(8)'s through the direct map).
pub fn abi_module_boot(image: Option<(usize, usize)>) {
    let _ = hal::syscall::set_legacy(legacy_syscall);
    let _ = hal::bridges::register_module_platform(&BsdPlatform);
    if let Some((va, len)) = image {
        IMAGE_VA.store(va, Ordering::Relaxed);
        IMAGE_LEN.store(len, Ordering::Release);
    }
}

/// The second boot step, from `main` before `init(8)` may exec: loads the module, which
/// registers its handler; prints the one line that says whether it did.
pub fn abi_module_attach() {
    match hal::module::load(&KERNEL_API) {
        Ok(loaded) => crate::kprintf!("{loaded}\n"),
        Err(e) => crate::kprintf!("{e}\n"),
    }
}

/// `KernelApi::copyin`.
fn api_copyin(uaddr: usize, kbuf: &mut [u8]) -> Result<(), i32> {
    copyin(uaddr, kbuf).map_err(Errno::as_i32)
}

/// `KernelApi::copyout`.
fn api_copyout(kbuf: &[u8], uaddr: usize) -> Result<(), i32> {
    copyout(kbuf, uaddr).map_err(Errno::as_i32)
}

/// An errno value from `hal` back as the [`Errno`] it was made from.
fn errno_of(raw: i32) -> Errno {
    match Errno::from_raw(raw) {
        Some(e) => e,
        None => panic(format_args!("syscall: errno {raw} out of range")),
    }
}
/* </CODE> */
