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
//! `bsd`'s side of the ABI module `abi_openbsd.ko` (decision 22) until the subsystems it
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
//! Where the module runs: the machine's module window when it has one
//! (`machine::pmap_module_window`; amd64, whose `kernel_map` is never executable, keeps one
//! in the kernel image's slot), entered with `pmap_module_enter`; `kernel_map` otherwise
//! (arm64), entered with `pmap_enter`. Either way the loader's writable copy is a
//! `kernel_map` range, released before the module's entry point is called, and a page of
//! the module is executable only when its segment is.
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
use crate::machine::pmap::{
    pmap_enter, pmap_extract, pmap_kernel, pmap_module_enter, pmap_module_window, pmap_update,
};
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
/// The bytes of the machine's module window `reserve` handed out (`window_take`).
static WINDOW_USED: AtomicUsize = AtomicUsize::new(0);

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
        let len = round_page(len);
        match pmap_module_window() {
            // The machine's module window (amd64: outside kernel_map, which never executes
            // there); refused when it has no room left, never placed elsewhere.
            Some((base, size)) => window_take(&WINDOW_USED, base.as_usize(), size, len),
            None => km_alloc(len, &KV_ANY, &KP_NONE, &KD_WAITOK).map(|p| p.as_ptr().addr()),
        }
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
        let entered = if pmap_module_window().is_some() {
            pmap_module_enter(Vaddr::new(va), pa, prot)
        } else {
            pmap_enter(pmap_kernel(), Vaddr::new(va), pa, prot, prot | PMAP_WIRED).is_ok()
        };
        pmap_update(pmap_kernel());
        entered
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
    };
}

/// `KernelApi::copyin`.
fn api_copyin(uaddr: usize, kbuf: &mut [u8]) -> Result<(), i32> {
    copyin(uaddr, kbuf).map_err(Errno::as_i32)
}

/// `KernelApi::copyout`.
fn api_copyout(kbuf: &[u8], uaddr: usize) -> Result<(), i32> {
    copyout(kbuf, uaddr).map_err(Errno::as_i32)
}

/// The next `len` bytes of the window `[base, base + size)`, `used` bytes of which were
/// handed out: each range once, for the kernel's lifetime; `None` when the window has no
/// room left for `len`.
fn window_take(used: &AtomicUsize, base: usize, size: usize, len: usize) -> Option<usize> {
    let off = used
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |u| {
            u.checked_add(len).filter(|&end| end <= size)
        })
        .ok()?;
    base.checked_add(off)
}

/// An errno value from `hal` back as the [`Errno`] it was made from.
fn errno_of(raw: i32) -> Errno {
    match Errno::from_raw(raw) {
        Some(e) => e,
        None => panic(format_args!("syscall: errno {raw} out of range")),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_eq;

    /// The window's bump: ranges in order, each once, refused past the end without moving.
    #[test]
    fn window_take_hands_out_each_range_once() {
        let used = AtomicUsize::new(0);
        let (base, size) = (0x10_0000, 0x4000);
        assert_eq!(window_take(&used, base, size, 0x1000), Some(0x10_0000));
        assert_eq!(window_take(&used, base, size, 0x2000), Some(0x10_1000));
        assert_eq!(window_take(&used, base, size, 0x2000), None);
        assert_eq!(used.load(Ordering::Relaxed), 0x3000);
        assert_eq!(window_take(&used, base, size, 0x1000), Some(0x10_3000));
        assert_eq!(window_take(&used, base, size, 0x1000), None);
    }

    /// An empty window (amd64 when `pmap_bootstrap` could not link it) refuses every
    /// request; a length that would overflow is refused, not wrapped.
    #[test]
    fn window_take_refuses_empty_and_overflow() {
        let used = AtomicUsize::new(0);
        assert_eq!(window_take(&used, 0, 0, 0x1000), None);
        let used = AtomicUsize::new(0x1000);
        assert_eq!(window_take(&used, 0x1000, 0x4000, usize::MAX), None);
        assert_eq!(used.load(Ordering::Relaxed), 0x1000);
    }
}
/* </TESTS> */
