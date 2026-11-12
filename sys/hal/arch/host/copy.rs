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
//! The host double of the machine's copies (`copy.S`, `copystr.S`; `hal::user`'s backend on
//! the host), the one the host tests of every crate copy through since the `UserCopy` bridge
//! went (N4.8b; before, `hal`'s and `kernel`'s tests registered a double each).
//!
//! - User memory: one address space of [`USER_LEN`] bytes at [`USER_BASE`]; anything outside
//!   it is `EFAULT`, as an unmapped user page is. The tests read and write it through
//!   [`user_bytes`] and `hal::user::UserPtr`. It is one array for the whole test process, so
//!   tests that run at once keep to ranges of their own (offsets from `USER_BASE`):
//!   - `0x0_0000..0x1_0000`: the 64 KiB the tests lay their arguments out in (paths, buffers,
//!     `struct stat`s, the `bsd` tests' `user_buf` at `0x8000`); `0x7000..0x8000` is the
//!     routing socket's `net.route` answers (`kernel::net::rtsock`'s tests, N6.3b2), which
//!     no other kernel test writes.
//!   - `0x1_0000..0x3_0000`: the memory file system's image (`kernel::fs::ufs::mfs`'s host
//!     fixture, N5.5b3), 128 KiB at most, written and zeroed by the fixture alone. An image
//!     smaller than 64 KiB cannot mount (the super-block search's first slot, 64 KiB in,
//!     lies past its end and that read faults, as the C's copy of a negative length does);
//!     one of 64 KiB or more mounts from FFS1's slot at 8 KiB.
//!   - `0x3_0000..0x4_0000`: the end of the memory, for the copies that must fault past it:
//!     `kern.msgbuf`'s copy (`kernel::log`'s tests: the 40-byte header and 256-byte chunks
//!     from `0x3_0000`, so its last whole chunk ends 216 bytes before the end and the next
//!     faults) and, in those last 216 bytes, which that copy never writes, `uio`'s nine
//!     iovecs at `USER_LEN - 144` and `pipe`'s word at `USER_LEN - 4`.
//! - Foreign memory (a `hal::user::ForeignWindow`'s pages): one run of [`FOREIGN_LEN`] bytes at
//!   [`FOREIGN_BASE`] ([`foreign_bytes`], [`set_foreign_bytes`]). The doubles' addresses lie
//!   below 4 GiB ([`DOUBLE_LIMIT`]), where no host memory is (macOS keeps the low 4 GiB of a
//!   64-bit process unmapped, `__PAGEZERO`): a kernel copy there outside the foreign run faults
//!   (`EFAULT`), as a window onto a page that went away does.
//! - A kernel address at or above [`DOUBLE_LIMIT`] in the lower half is the host's own
//!   memory: the `hal::user::KcopyToken`'s holder (`bsd`'s `uiomove`, whose host tests build
//!   `UIO_SYSSPACE` uios over their own buffers) vouches for it, as on the machine.
//! - A kernel address in the upper half is a kernel-map address (N5.9t1: the kernel tests'
//!   `kernel_map` lies there, `kernel::testutil::KM_VA_BASE`): a kernel copy reaches it a page
//!   at a time through the frame the kernel pmap maps there (host memory through the identity
//!   direct map), and a page the pmap does not map yet is faulted in through the hook
//!   [`set_kernel_fault_hook`] installs (`uvm_fault(kernel_map)`, the host's data abort), once,
//!   before the copy is `EFAULT`: tmpfs's window over its aobj (`kernel::vm::km::ObjectWindow`)
//!   is copied through, as on the machine, where `kdata_abort` serves the fault before
//!   `pcb_onfault`. [`copy_raw`] is the machine's raw legs, over a bounce of its own.

// The host double alone links std outside the tests (rust-kernel.md: the host doubles).
extern crate std;

use std::sync::{Mutex, MutexGuard, OnceLock};
use std::vec::Vec;

use super::param::{PAGE_MASK, PAGE_SIZE};
use crate::sys::errno::Errno;
use crate::sys::types::{Paddr, Vaddr};
use crate::user::RawCopy;

/// Where the double's user address space starts.
pub const USER_BASE: usize = 0x1000_0000;
/// Its size: 256 KiB since N5.5b3 (64 KiB before), the three ranges of the module's docs.
pub const USER_LEN: usize = 256 * 1024;
/// Where the double's foreign kernel memory starts (a `ForeignWindow`'s side of `kcopy`).
pub const FOREIGN_BASE: usize = 0x3000_0000;
/// Its size.
pub const FOREIGN_LEN: usize = 64 * 1024;

/// The user address space of the double.
static USER: Mutex<[u8; USER_LEN]> = Mutex::new([0; USER_LEN]);
/// The double's foreign kernel memory.
static FOREIGN: Mutex<[u8; FOREIGN_LEN]> = Mutex::new([0; FOREIGN_LEN]);

/// A double's memory, a panicking test's poison ignored.
fn lock<T>(m: &'static Mutex<T>) -> MutexGuard<'static, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The range `[addr - base, addr - base + len)` of a region of `size` bytes at `base`,
/// `EFAULT` outside it.
fn range(
    base: usize,
    size: usize,
    addr: usize,
    len: usize,
) -> Result<core::ops::Range<usize>, Errno> {
    let start = addr.checked_sub(base).ok_or(Errno::EFAULT)?;
    let end = start.checked_add(len).ok_or(Errno::EFAULT)?;
    if end > size {
        return Err(Errno::EFAULT);
    }
    Ok(start..end)
}

/// The range of the user address space at `uaddr`, `EFAULT` outside it.
fn user_range(uaddr: usize, len: usize) -> Result<core::ops::Range<usize>, Errno> {
    range(USER_BASE, USER_LEN, uaddr, len)
}

/// Below this the addresses are the doubles' (user and foreign memory, and the faults around
/// them); host memory lies above it.
pub const DOUBLE_LIMIT: usize = 1 << 32;

/// Whether `kva` is one of the doubles' addresses (a kernel copy there is the double's, an
/// `EFAULT` outside the foreign run).
fn is_foreign(kva: usize) -> bool {
    kva < DOUBLE_LIMIT
}

/// `copyin`: `EFAULT` outside the user address space.
pub(crate) fn copyin(uaddr: usize, kbuf: &mut [u8]) -> Result<(), Errno> {
    let r = user_range(uaddr, kbuf.len())?;
    kbuf.copy_from_slice(&lock(&USER)[r]);
    Ok(())
}

/// `copyout`.
pub(crate) fn copyout(kbuf: &[u8], uaddr: usize) -> Result<(), Errno> {
    let r = user_range(uaddr, kbuf.len())?;
    lock(&USER)[r].copy_from_slice(kbuf);
    Ok(())
}

/// `copyin32`: the aligned word, `EFAULT` when misaligned or outside.
pub(crate) fn copyin32(uaddr: usize) -> Result<u32, Errno> {
    if !uaddr.is_multiple_of(4) {
        return Err(Errno::EFAULT);
    }
    let r = user_range(uaddr, 4)?;
    let mut w = [0u8; 4];
    w.copy_from_slice(&lock(&USER)[r]);
    Ok(u32::from_ne_bytes(w))
}

/// `copyinstr`: up to the NUL (copied), at most `kbuf.len()` bytes, `ENAMETOOLONG` without
/// one; `EFAULT` past the address space.
pub(crate) fn copyinstr(uaddr: usize, kbuf: &mut [u8]) -> Result<usize, Errno> {
    let user = lock(&USER);
    for (i, b) in kbuf.iter_mut().enumerate() {
        let r = user_range(uaddr.wrapping_add(i), 1)?;
        *b = user[r.start];
        if *b == 0 {
            return Ok(i + 1);
        }
    }
    Err(Errno::ENAMETOOLONG)
}

/// `copyoutstr`: `kbuf` up to its NUL (copied), at most `kbuf.len()` bytes, `ENAMETOOLONG`
/// without one; `EFAULT` past the address space.
pub(crate) fn copyoutstr(kbuf: &[u8], uaddr: usize) -> Result<usize, Errno> {
    let mut user = lock(&USER);
    for (i, &b) in kbuf.iter().enumerate() {
        let r = user_range(uaddr.wrapping_add(i), 1)?;
        user[r.start] = b;
        if b == 0 {
            return Ok(i + 1);
        }
    }
    Err(Errno::ENAMETOOLONG)
}

/// The host's data abort for a kernel-map address ([`set_kernel_fault_hook`]): `uvm_fault`
/// on `kernel_map` at the address, for a write or not; `true` when the page is now mapped.
static KERNEL_FAULT: OnceLock<fn(usize, bool) -> bool> = OnceLock::new();

/// Installs the host's data abort for kernel-map addresses (`kernel::testutil::install`'s
/// `uvm_fault(kernel_map(), va, ..)`, once per process; a later call keeps the first): a
/// kernel copy that meets a kernel-map page the kernel pmap does not map yet calls it once
/// and retries, `EFAULT` when it fails, as `kdata_abort` serves the fault before
/// `pcb_onfault`.
pub fn set_kernel_fault_hook(hook: fn(usize, bool) -> bool) {
    let _ = KERNEL_FAULT.set(hook);
}

/// Whether `kva` is a kernel-map address: the upper half (the host's own memory, a process's,
/// lies in the lower one; the doubles' below [`DOUBLE_LIMIT`]).
fn is_kernel_map(kva: usize) -> bool {
    kva >> 63 == 1
}

/// The host bytes behind the kernel-map address `va`: the frame the kernel pmap maps at its
/// page (a frame is host memory through the identity direct map, as the kernel tests that
/// write their pages install it, `kernel::testutil::pages_install`), the page faulted in once
/// through the hook on a miss; `EFAULT` when it stays unmapped.
fn kernel_map_bytes(va: usize, write: bool) -> Result<usize, Errno> {
    let pmap = crate::pmap::kernel();
    if let Some(pa) = pmap.extract(Vaddr::new(va)) {
        return Ok(pa.as_usize());
    }
    let hook = KERNEL_FAULT.get().ok_or(Errno::EFAULT)?;
    if !hook(va & !PAGE_MASK, write) {
        return Err(Errno::EFAULT);
    }
    pmap.extract(Vaddr::new(va))
        .map(Paddr::as_usize)
        .ok_or(Errno::EFAULT)
}

/// The host address of the first bytes of the kernel range `[kva, kva + len)` and how many
/// of them lie contiguous there: within its page for a kernel-map address, the whole range
/// for the host's own memory. `EFAULT` for a range that wraps.
fn host_run(kva: usize, len: usize, write: bool) -> Result<(usize, usize), Errno> {
    if kva.checked_add(len).is_none() {
        return Err(Errno::EFAULT);
    }
    if is_kernel_map(kva) {
        let n = len.min(PAGE_SIZE - (kva & PAGE_MASK));
        return Ok((kernel_map_bytes(kva, write)?, n));
    }
    Ok((kva, len))
}

/// `kcopy(kva, buf, buf.len())`: the foreign region's bytes (`EFAULT` past its end), a
/// kernel-map range's frames page by page (faulted in through the hook), or the host's own
/// memory at `kva`.
///
/// # Safety
///
/// As the machine's (`arch/arm64/copy.rs`): `[kva, kva + buf.len())` outside the foreign
/// region is the caller's memory to read, holding no Rust object written during the copy
/// (its bytes may change meanwhile through another mapping: no reference is made).
pub(crate) unsafe fn kcopy_in(kva: usize, buf: &mut [u8]) -> Result<(), Errno> {
    if is_foreign(kva) {
        let r = range(FOREIGN_BASE, FOREIGN_LEN, kva, buf.len())?;
        buf.copy_from_slice(&lock(&FOREIGN)[r]);
        return Ok(());
    }
    let mut done = 0;
    while done < buf.len() {
        let (at, n) = host_run(kva + done, buf.len() - done, false)?;
        let src = core::ptr::with_exposed_provenance::<u8>(at);
        let dst = &mut buf[done..done + n];
        // SAFETY: the caller's guarantee: the range is readable memory holding no Rust object
        // written during the copy (another mapping may change its bytes meanwhile, which a
        // copy through a raw pointer tolerates: no reference is made), the token holder's own
        // buffer (on the host its test's) or the
        // pages of a window the kernel map holds, whose `n` bytes from `at` lie in the one
        // frame the kernel pmap maps there (`host_run` stops at the page's end), host memory
        // through the identity direct map; `dst` is a distinct, exclusively borrowed slice of
        // `n` bytes.
        unsafe { core::ptr::copy(src, dst.as_mut_ptr(), n) };
        done += n;
    }
    Ok(())
}

/// `kcopy(buf, kva, buf.len())`: into the foreign region (`EFAULT` past its end), a
/// kernel-map range's frames page by page (faulted in through the hook), or the host's own
/// memory at `kva`.
///
/// # Safety
///
/// As the machine's: `[kva, kva + buf.len())` outside the foreign region is the caller's
/// memory to write, no Rust object referencing it during the copy.
pub(crate) unsafe fn kcopy_out(buf: &[u8], kva: usize) -> Result<(), Errno> {
    if is_foreign(kva) {
        let r = range(FOREIGN_BASE, FOREIGN_LEN, kva, buf.len())?;
        lock(&FOREIGN)[r].copy_from_slice(buf);
        return Ok(());
    }
    let mut done = 0;
    while done < buf.len() {
        let (at, n) = host_run(kva + done, buf.len() - done, true)?;
        let dst = core::ptr::with_exposed_provenance_mut::<u8>(at);
        let src = &buf[done..done + n];
        // SAFETY: the caller's guarantee: the range is writable memory no Rust reference
        // covers during the copy (as in `kcopy_in`, a kernel-map range is copied a frame at a
        // time, `host_run` keeping each run inside one page); `src` is only read, for `n`
        // bytes.
        unsafe { core::ptr::copy(src.as_ptr(), dst, n) };
        done += n;
    }
    Ok(())
}

/// `copyout`, `copyin` or `kcopy` between raw addresses, as `kind` says (the machine's
/// `copy_raw`, `arch/arm64/copy.rs`): a `hal::user::ForeignWindow`'s legs to and from a
/// uio's iovecs, through a bounce of the double's own (512 bytes at a time: the double's
/// cost, not the machine's, which copies once), each side as [`kcopy_in`], [`kcopy_out`],
/// [`copyin`] and [`copyout`] reach it.
///
/// # Safety
///
/// As the machine's: each kernel side is `len` bytes holding no Rust object referenced during
/// the copy (a window's pages, a `UIO_SYSSPACE` iovec's buffer); a user side may be any
/// address.
pub(crate) unsafe fn copy_raw(
    kind: RawCopy,
    src: usize,
    dst: usize,
    len: usize,
) -> Result<(), Errno> {
    let mut chunk = [0u8; 512];
    let mut done = 0;
    while done < len {
        let n = (len - done).min(chunk.len());
        let c = &mut chunk[..n];
        let s = src.checked_add(done).ok_or(Errno::EFAULT)?;
        let d = dst.checked_add(done).ok_or(Errno::EFAULT)?;
        // SAFETY: `kcopy_in` reads and `kcopy_out` writes `n` bytes of a kernel side of this
        // call, inside the `len` bytes the caller guarantees hold no Rust object referenced
        // during the copy (this function's contract, theirs); `c` is a local buffer of `n`
        // bytes, borrowed for each step alone.
        unsafe {
            match kind {
                RawCopy::Out => {
                    kcopy_in(s, c)?;
                    copyout(c, d)?;
                }
                RawCopy::In => {
                    copyin(s, c)?;
                    kcopy_out(c, d)?;
                }
                RawCopy::Kernel => {
                    kcopy_in(s, c)?;
                    kcopy_out(c, d)?;
                }
            }
        }
        done += n;
    }
    Ok(())
}

/// `len` bytes of the double's user memory at `uaddr`, `None` outside it.
pub fn user_bytes(uaddr: usize, len: usize) -> Option<Vec<u8>> {
    let r = user_range(uaddr, len).ok()?;
    Some(lock(&USER)[r].to_vec())
}

/// `len` bytes of the double's foreign kernel memory at `kva`, `None` outside it.
pub fn foreign_bytes(kva: usize, len: usize) -> Option<Vec<u8>> {
    let r = range(FOREIGN_BASE, FOREIGN_LEN, kva, len).ok()?;
    Some(lock(&FOREIGN)[r].to_vec())
}

/// Writes `b` into the double's foreign kernel memory at `kva`; `false` outside it.
pub fn set_foreign_bytes(kva: usize, b: &[u8]) -> bool {
    match range(FOREIGN_BASE, FOREIGN_LEN, kva, b.len()) {
        Ok(r) => {
            lock(&FOREIGN)[r].copy_from_slice(b);
            true
        }
        Err(_) => false,
    }
}
/* </CODE> */
